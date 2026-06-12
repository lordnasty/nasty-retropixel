use image::{GenericImageView, ImageBuffer, Rgba, RgbaImage};
use rand::prelude::*;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, WeightedIndex};
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;
use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};
#[cfg(not(target_arch = "wasm32"))]
use std::env;
use std::error::Error;
use std::fmt;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use js_sys::{Array, Object, Reflect, Uint32Array, Uint8Array};

#[derive(Debug, Clone)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct Config {
    pub k_colors: usize,
    pub pixel_size_override: Option<f64>,
    pub prefilter_mode: u32,
    pub palette_source: u32,
    pub palette_cleanup_mode: u32,
    pub cell_color_mode: u32,
    pub dither_mode: u32,
    pub color_space: u32,
    pub cleanup_mode: u32,
    pub repair_mode: u32,
    k_seed: u64,
    /// Input image path only used for CLI use
    #[allow(dead_code)]
    input_path: String,
    /// Output image path only used for CLI use
    #[allow(dead_code)]
    output_path: String,
    max_kmeans_iterations: usize,
    peak_threshold_multiplier: f64,
    peak_distance_filter: usize,
    walker_search_window_ratio: f64,
    walker_min_search_window: f64,
    walker_strength_threshold: f64,
    min_cuts_per_axis: usize,
    fallback_target_segments: usize,
    max_step_ratio: f64,
    autocorr_max_lag: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            k_colors: 16,
            k_seed: 42,
            input_path: "samples/2/skeleton.png".to_string(),
            output_path: "samples/2/skeleton_fixed_clean2.png".to_string(),
            max_kmeans_iterations: 15,
            peak_threshold_multiplier: 0.2,
            peak_distance_filter: 4,
            walker_search_window_ratio: 0.35,
            walker_min_search_window: 2.0,
            walker_strength_threshold: 0.5,
            min_cuts_per_axis: 4,
            fallback_target_segments: 64,
            max_step_ratio: 1.8, // Lowered from 3.0 to catch more skew cases
            pixel_size_override: None,
            prefilter_mode: 0,
            palette_source: 1,
            palette_cleanup_mode: 1,
            cell_color_mode: 1,
            dither_mode: 0,
            color_space: 1,
            cleanup_mode: 1,
            repair_mode: 2,
            autocorr_max_lag: 256,
        }
    }
}

#[derive(Debug)]
pub enum PixelSnapperError {
    ImageError(image::ImageError),
    InvalidInput(String),
    ProcessingError(String),
}

impl fmt::Display for PixelSnapperError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PixelSnapperError::ImageError(e) => write!(f, "Image error: {}", e),
            PixelSnapperError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            PixelSnapperError::ProcessingError(msg) => write!(f, "Processing error: {}", msg),
        }
    }
}

impl Error for PixelSnapperError {}

impl From<image::ImageError> for PixelSnapperError {
    fn from(error: image::ImageError) -> Self {
        PixelSnapperError::ImageError(error)
    }
}

#[cfg(target_arch = "wasm32")]
impl From<PixelSnapperError> for wasm_bindgen::JsValue {
    fn from(err: PixelSnapperError) -> wasm_bindgen::JsValue {
        wasm_bindgen::JsValue::from_str(&err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, PixelSnapperError>;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
struct PresetSpec {
    key: &'static str,
    label: &'static str,
    description: &'static str,
    k_colors: usize,
    prefilter_mode: u32,
    palette_source: u32,
    palette_cleanup_mode: u32,
    cell_color_mode: u32,
    dither_mode: u32,
    color_space: u32,
    cleanup_mode: u32,
    repair_mode: u32,
}

const PRESET_AI_SPRITE: PresetSpec = PresetSpec {
    key: "ai-sprite",
    label: "AI Sprite Cleanup",
    description: "Riduce artefatti AI, consolida palette e preserva masse cromatiche leggibili.",
    k_colors: 24,
    prefilter_mode: 1,
    palette_source: 1,
    palette_cleanup_mode: 2,
    cell_color_mode: 2,
    dither_mode: 0,
    color_space: 1,
    cleanup_mode: 1,
    repair_mode: 2,
};

const PRESET_STRICT_RETRO: PresetSpec = PresetSpec {
    key: "strict-retro",
    label: "Strict Retro",
    description: "Riduce al minimo il rumore e privilegia blocchi netti con palette controllata.",
    k_colors: 16,
    prefilter_mode: 1,
    palette_source: 1,
    palette_cleanup_mode: 2,
    cell_color_mode: 1,
    dither_mode: 0,
    color_space: 1,
    cleanup_mode: 1,
    repair_mode: 2,
};

const PRESET_TILESET: PresetSpec = PresetSpec {
    key: "tileset-cleanup",
    label: "Tileset Cleanup",
    description: "Favorisce coerenza di celle, palette stabile e piccoli repair strutturali.",
    k_colors: 32,
    prefilter_mode: 1,
    palette_source: 1,
    palette_cleanup_mode: 1,
    cell_color_mode: 1,
    dither_mode: 0,
    color_space: 1,
    cleanup_mode: 1,
    repair_mode: 2,
};

const PRESET_CHARACTER: PresetSpec = PresetSpec {
    key: "character-cleanup",
    label: "Character Cleanup",
    description: "Pensato per sprite e personaggi con silhouette, volume e contorni da ripulire.",
    k_colors: 20,
    prefilter_mode: 1,
    palette_source: 1,
    palette_cleanup_mode: 2,
    cell_color_mode: 2,
    dither_mode: 0,
    color_space: 1,
    cleanup_mode: 1,
    repair_mode: 2,
};

const PRESET_ICON: PresetSpec = PresetSpec {
    key: "icon-cleanup",
    label: "Icon Cleanup",
    description: "Favorisce chiarezza grafica, contrasti netti e palette compatta per icone e UI.",
    k_colors: 12,
    prefilter_mode: 1,
    palette_source: 1,
    palette_cleanup_mode: 2,
    cell_color_mode: 1,
    dither_mode: 0,
    color_space: 1,
    cleanup_mode: 1,
    repair_mode: 2,
};

const PRESET_ULTRA: PresetSpec = PresetSpec {
    key: "ultra-cleanup",
    label: "Ultra Cleanup",
    description: "Repair aggressivo per output AI molto sporchi: chiude gap, rimuove micro-isole e compatta diagonali.",
    k_colors: 24,
    prefilter_mode: 1,
    palette_source: 1,
    palette_cleanup_mode: 2,
    cell_color_mode: 2,
    dither_mode: 0,
    color_space: 1,
    cleanup_mode: 1,
    repair_mode: 3,
};

#[allow(dead_code)]
const ALL_PRESETS: [PresetSpec; 6] = [
    PRESET_AI_SPRITE,
    PRESET_STRICT_RETRO,
    PRESET_TILESET,
    PRESET_CHARACTER,
    PRESET_ICON,
    PRESET_ULTRA,
];

fn preset_by_name(name: &str) -> Option<PresetSpec> {
    match name {
        "ai-sprite" => Some(PRESET_AI_SPRITE),
        "strict-retro" => Some(PRESET_STRICT_RETRO),
        "tileset-cleanup" => Some(PRESET_TILESET),
        "character-cleanup" => Some(PRESET_CHARACTER),
        "icon-cleanup" => Some(PRESET_ICON),
        "ultra-cleanup" => Some(PRESET_ULTRA),
        _ => None,
    }
}

pub fn apply_named_preset(config: &mut Config, preset_name: &str) -> Result<()> {
    let preset = preset_by_name(preset_name).ok_or_else(|| {
        PixelSnapperError::InvalidInput(format!(
            "unknown preset '{}'. Available presets: ai-sprite, strict-retro, tileset-cleanup, character-cleanup, icon-cleanup, ultra-cleanup",
            preset_name
        ))
    })?;

    config.k_colors = preset.k_colors;
    config.prefilter_mode = preset.prefilter_mode;
    config.palette_source = preset.palette_source;
    config.palette_cleanup_mode = preset.palette_cleanup_mode;
    config.cell_color_mode = preset.cell_color_mode;
    config.dither_mode = preset.dither_mode;
    config.color_space = preset.color_space;
    config.cleanup_mode = preset.cleanup_mode;
    config.repair_mode = preset.repair_mode;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct ImageAnalysis {
    opaque_ratio: f64,
    unique_colors: usize,
    edge_density: f64,
    dominant_alpha: bool,
    preset_key: &'static str,
    reason: &'static str,
}

fn analyze_image_for_suggestion(input_bytes: &[u8]) -> Result<ImageAnalysis> {
    let img = image::load_from_memory(input_bytes)?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    validate_image_dimensions(width, height)?;

    let total_pixels = (width as usize).saturating_mul(height as usize).max(1);
    let mut opaque_pixels = 0usize;
    let mut alpha_pixels = 0usize;
    let mut unique_colors: HashMap<[u8; 3], usize> = HashMap::new();
    let mut edge_sum = 0.0f64;

    for y in 0..height {
        for x in 0..width {
            let p = rgba.get_pixel(x, y).0;
            if p[3] > 0 {
                opaque_pixels += 1;
                if p[3] < 255 {
                    alpha_pixels += 1;
                }
                *unique_colors.entry([p[0], p[1], p[2]]).or_insert(0) += 1;
            }

            if x + 1 < width {
                let q = rgba.get_pixel(x + 1, y).0;
                edge_sum += ((p[0] as f64 - q[0] as f64).abs()
                    + (p[1] as f64 - q[1] as f64).abs()
                    + (p[2] as f64 - q[2] as f64).abs())
                    / 3.0;
            }
            if y + 1 < height {
                let q = rgba.get_pixel(x, y + 1).0;
                edge_sum += ((p[0] as f64 - q[0] as f64).abs()
                    + (p[1] as f64 - q[1] as f64).abs()
                    + (p[2] as f64 - q[2] as f64).abs())
                    / 3.0;
            }
        }
    }

    let opaque_ratio = opaque_pixels as f64 / total_pixels as f64;
    let dominant_alpha = alpha_pixels > 0 && (alpha_pixels as f64 / opaque_pixels.max(1) as f64) > 0.05;
    let edge_density = edge_sum / total_pixels as f64;
    let unique_color_count = unique_colors.len();
    let aspect = width as f64 / height.max(1) as f64;

    let (preset_key, reason) = if width <= 96
        && height <= 96
        && unique_color_count <= 24
        && opaque_ratio > 0.45
    {
        (
            PRESET_ICON.key,
            "Immagine compatta con palette contenuta: profilo adatto a icone e UI.",
        )
    } else if (aspect > 1.6 || aspect < 0.7)
        && unique_color_count <= 48
        && edge_density > 12.0
    {
        (
            PRESET_CHARACTER.key,
            "Forma allungata e dettagli leggibili: profilo consigliato per personaggi/sprite.",
        )
    } else if width >= 96
        && height >= 96
        && unique_color_count >= 28
        && opaque_ratio > 0.55
    {
        (
            PRESET_TILESET.key,
            "Canvas ampio e palette piu' ricca: profilo utile per tileset e texture grid-based.",
        )
    } else if dominant_alpha || unique_color_count <= 18 {
        (
            PRESET_STRICT_RETRO.key,
            "Pochi colori o alpha evidente: profilo retro piu' rigido e controllato.",
        )
    } else {
        (
            PRESET_AI_SPRITE.key,
            "Caso generale per cleanup di output AI con palette e griglia da ricostruire.",
        )
    };

    Ok(ImageAnalysis {
        opaque_ratio,
        unique_colors: unique_color_count,
        edge_density,
        dominant_alpha,
        preset_key,
        reason,
    })
}

fn preset_suggestion_from_analysis(analysis: ImageAnalysis) -> PresetSuggestion {
    PresetSuggestion {
        preset_key: analysis.preset_key.to_string(),
        reason: analysis.reason.to_string(),
        opaque_ratio: analysis.opaque_ratio,
        unique_colors: analysis.unique_colors,
        edge_density: analysis.edge_density,
        dominant_alpha: analysis.dominant_alpha,
    }
}

pub fn suggest_preset_for_image_bytes(input_bytes: &[u8]) -> Result<PresetSuggestion> {
    let analysis = analyze_image_for_suggestion(input_bytes)?;
    Ok(preset_suggestion_from_analysis(analysis))
}

pub fn suggest_setup_for_image_bytes(input_bytes: &[u8]) -> Result<AutoSetupSuggestion> {
    let analysis = analyze_image_for_suggestion(input_bytes)?;
    let preset = preset_by_name(analysis.preset_key).unwrap_or(PRESET_AI_SPRITE);
    let mut prefilter_mode = preset.prefilter_mode;
    let mut palette_source = preset.palette_source;
    let mut palette_cleanup_mode = preset.palette_cleanup_mode;
    let mut cell_color_mode = preset.cell_color_mode;
    let mut cleanup_mode = preset.cleanup_mode;
    let mut repair_mode = preset.repair_mode;
    let mut trim_transparent = analysis.dominant_alpha || analysis.opaque_ratio < 0.30;
    let mut notes: Vec<&'static str> = Vec::new();

    if analysis.unique_colors >= 56 {
        palette_cleanup_mode = 2;
        cell_color_mode = 2;
        repair_mode = repair_mode.max(3);
        notes.push("molti colori rilevati: cleanup palette rigoroso e repair piu' forte");
    } else if analysis.unique_colors >= 36 {
        palette_cleanup_mode = palette_cleanup_mode.max(2);
        notes.push("palette abbastanza ricca: cleanup strict consigliato");
    } else if analysis.unique_colors <= 18 {
        notes.push("palette abbastanza contenuta: evita correzioni cromatiche inutilmente pesanti");
    }

    if analysis.edge_density >= 22.0 {
        prefilter_mode = 1;
        repair_mode = repair_mode.max(3);
        cleanup_mode = cleanup_mode.max(1);
        notes.push("bordi molto densi: denoise box3 e repair ultra aiutano a stabilizzare la griglia");
    } else if analysis.edge_density >= 16.0 {
        prefilter_mode = 1;
        repair_mode = repair_mode.max(2);
        notes.push("bordi abbastanza rumorosi: denoise box3 e repair smart consigliati");
    }

    if analysis.dominant_alpha || analysis.opaque_ratio < 0.28 {
        trim_transparent = true;
        notes.push("trasparenza o area utile ridotta: trim trasparenza utile per ripulire il canvas");
    }

    if analysis.preset_key == PRESET_TILESET.key {
        palette_source = 1;
        cell_color_mode = 1;
        notes.push("profilo tileset: palette dalle celle e colore dominante per tenere i tile coerenti");
    } else if analysis.preset_key == PRESET_CHARACTER.key {
        cell_color_mode = 2;
        repair_mode = repair_mode.max(2);
        notes.push("profilo character: medoid e repair smart aiutano silhouette e contorni");
    } else if analysis.preset_key == PRESET_ICON.key {
        trim_transparent = true;
        palette_cleanup_mode = 2;
        notes.push("profilo icon: trim e palette strict favoriscono asset compatti e leggibili");
    } else if analysis.preset_key == PRESET_STRICT_RETRO.key {
        palette_cleanup_mode = palette_cleanup_mode.max(2);
        notes.push("profilo retro: blocchi netti e palette controllata come base di partenza");
    }

    if notes.is_empty() {
        notes.push("il preset suggerito copre gia' bene il caso generale");
    }

    Ok(AutoSetupSuggestion {
        preset_key: analysis.preset_key.to_string(),
        reason: analysis.reason.to_string(),
        opaque_ratio: analysis.opaque_ratio,
        unique_colors: analysis.unique_colors,
        edge_density: analysis.edge_density,
        dominant_alpha: analysis.dominant_alpha,
        recommended_prefilter_mode: prefilter_mode,
        recommended_prefilter_label: prefilter_mode_label(prefilter_mode).to_string(),
        recommended_palette_source: palette_source,
        recommended_palette_source_label: palette_source_label(palette_source).to_string(),
        recommended_palette_cleanup_mode: palette_cleanup_mode,
        recommended_palette_cleanup_label: palette_cleanup_mode_label(palette_cleanup_mode)
            .to_string(),
        recommended_cell_color_mode: cell_color_mode,
        recommended_cell_color_label: cell_color_mode_label(cell_color_mode).to_string(),
        recommended_cleanup_mode: cleanup_mode,
        recommended_cleanup_label: cleanup_mode_label(cleanup_mode).to_string(),
        recommended_repair_mode: repair_mode,
        recommended_repair_label: repair_mode_label(repair_mode).to_string(),
        recommended_trim_transparent: trim_transparent,
        recommendation_reason: notes.join("; "),
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct PresetSuggestion {
    pub preset_key: String,
    pub reason: String,
    pub opaque_ratio: f64,
    pub unique_colors: usize,
    pub edge_density: f64,
    pub dominant_alpha: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AutoSetupSuggestion {
    pub preset_key: String,
    pub reason: String,
    pub opaque_ratio: f64,
    pub unique_colors: usize,
    pub edge_density: f64,
    pub dominant_alpha: bool,
    pub recommended_prefilter_mode: u32,
    pub recommended_prefilter_label: String,
    pub recommended_palette_source: u32,
    pub recommended_palette_source_label: String,
    pub recommended_palette_cleanup_mode: u32,
    pub recommended_palette_cleanup_label: String,
    pub recommended_cell_color_mode: u32,
    pub recommended_cell_color_label: String,
    pub recommended_cleanup_mode: u32,
    pub recommended_cleanup_label: String,
    pub recommended_repair_mode: u32,
    pub recommended_repair_label: String,
    pub recommended_trim_transparent: bool,
    pub recommendation_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantMetrics {
    pub key: String,
    pub label: String,
    pub diff_score: f64,
    pub diff_area: f64,
    pub palette_count: usize,
    pub aggressiveness: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct VariantRecommendation {
    pub key: String,
    pub label: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VariantRecommendationReport {
    pub recommendation: VariantRecommendation,
    pub metrics: Vec<VariantMetrics>,
}

fn clamp01(v: f64) -> f64 {
    if v.is_nan() {
        0.0
    } else if v < 0.0 {
        0.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    }
}

fn normalize(v: f64, min: f64, max: f64) -> f64 {
    if !v.is_finite() || !min.is_finite() || !max.is_finite() || max <= min {
        0.5
    } else {
        (v - min) / (max - min)
    }
}

pub fn recommend_variant_from_metrics(metrics: &[VariantMetrics]) -> Result<VariantRecommendation> {
    if metrics.is_empty() {
        return Err(PixelSnapperError::InvalidInput(
            "variant metrics cannot be empty".to_string(),
        ));
    }

    let min_diff = metrics
        .iter()
        .map(|m| m.diff_score)
        .fold(f64::INFINITY, f64::min);
    let max_diff = metrics
        .iter()
        .map(|m| m.diff_score)
        .fold(f64::NEG_INFINITY, f64::max);

    let min_area = metrics
        .iter()
        .map(|m| m.diff_area)
        .fold(f64::INFINITY, f64::min);
    let max_area = metrics
        .iter()
        .map(|m| m.diff_area)
        .fold(f64::NEG_INFINITY, f64::max);

    let min_palette = metrics.iter().map(|m| m.palette_count).min().unwrap_or(0) as f64;
    let max_palette = metrics.iter().map(|m| m.palette_count).max().unwrap_or(0) as f64;

    let fidelity_winner_key = metrics
        .iter()
        .min_by(|a, b| a.diff_score.total_cmp(&b.diff_score))
        .map(|m| m.key.as_str())
        .unwrap_or("");
    let clean_winner_key = metrics
        .iter()
        .min_by(|a, b| a.palette_count.cmp(&b.palette_count))
        .map(|m| m.key.as_str())
        .unwrap_or("");
    let aggressive_winner_key = metrics
        .iter()
        .max_by(|a, b| a.aggressiveness.total_cmp(&b.aggressiveness))
        .map(|m| m.key.as_str())
        .unwrap_or("");

    let mut best_idx = 0usize;
    let mut best_total = f64::NEG_INFINITY;
    for (idx, m) in metrics.iter().enumerate() {
        let diff_norm = normalize(m.diff_score, min_diff, max_diff);
        let area_norm = normalize(m.diff_area, min_area, max_area);
        let palette_norm = normalize(m.palette_count as f64, min_palette, max_palette);

        let fidelity = 1.0 - (diff_norm * 0.65 + area_norm * 0.35);
        let clean = (1.0 - palette_norm) * 0.55 + (1.0 - diff_norm) * 0.2 + (1.0 - area_norm) * 0.25;
        let aggressive = clamp01(m.aggressiveness);

        let total = fidelity * 0.48 + clean * 0.37 + (1.0 - aggressive) * 0.15;
        if total > best_total {
            best_total = total;
            best_idx = idx;
        }
    }

    let best = &metrics[best_idx];
    let mut reasons: Vec<&'static str> = Vec::new();
    if best.key == fidelity_winner_key {
        reasons.push("mantiene meglio la struttura originale");
    }
    if best.key == clean_winner_key {
        reasons.push("ripulisce meglio la palette");
    }
    if best.key != aggressive_winner_key && best.aggressiveness < 0.60 {
        reasons.push("resta meno invasiva delle opzioni piu' spinte");
    }
    if reasons.is_empty() {
        reasons.push("offre il compromesso migliore tra fedelta', pulizia e controllo");
    }

    Ok(VariantRecommendation {
        key: best.key.clone(),
        label: best.label.clone(),
        reason: reasons.join("; "),
    })
}

fn compute_diff_metrics_centered(input_bytes: &[u8], output_png_bytes: &[u8]) -> Result<(f64, f64)> {
    let input_img = image::load_from_memory(input_bytes)?.to_rgba8();
    let out_img = image::load_from_memory(output_png_bytes)?.to_rgba8();
    let (_, diff_score, diff_area) = build_diff_heatmap_centered(&input_img, &out_img)?;
    Ok((diff_score, diff_area))
}

fn build_diff_heatmap_centered(
    input_img: &RgbaImage,
    out_img: &RgbaImage,
) -> Result<(RgbaImage, f64, f64)> {
    let (w_in, h_in) = input_img.dimensions();
    let (w_out, h_out) = out_img.dimensions();
    validate_image_dimensions(w_in, h_in)?;
    validate_image_dimensions(w_out, h_out)?;

    let width = w_in.max(w_out).max(1);
    let height = h_in.max(h_out).max(1);

    let ox_in = (width as i64 - w_in as i64) / 2;
    let oy_in = (height as i64 - h_in as i64) / 2;
    let ox_out = (width as i64 - w_out as i64) / 2;
    let oy_out = (height as i64 - h_out as i64) / 2;

    let mut heatmap = RgbaImage::new(width, height);
    let mut diff_sum = 0.0f64;
    let mut active = 0usize;
    let total = (width as usize).saturating_mul(height as usize).max(1);

    for y in 0..height as i64 {
        for x in 0..width as i64 {
            let ax = x - ox_in;
            let ay = y - oy_in;
            let bx = x - ox_out;
            let by = y - oy_out;

            let a = if ax >= 0 && ay >= 0 && (ax as u32) < w_in && (ay as u32) < h_in {
                input_img.get_pixel(ax as u32, ay as u32).0
            } else {
                [0u8, 0u8, 0u8, 0u8]
            };
            let b = if bx >= 0 && by >= 0 && (bx as u32) < w_out && (by as u32) < h_out {
                out_img.get_pixel(bx as u32, by as u32).0
            } else {
                [0u8, 0u8, 0u8, 0u8]
            };

            let dr = (a[0] as f64 - b[0] as f64).abs();
            let dg = (a[1] as f64 - b[1] as f64).abs();
            let db = (a[2] as f64 - b[2] as f64).abs();
            let da = (a[3] as f64 - b[3] as f64).abs();
            let intensity = ((dr + dg + db) / 3.0) + da * 0.35;
            diff_sum += intensity;
            if intensity > 12.0 {
                active += 1;
            }

            let norm = (intensity / 255.0).clamp(0.0, 1.0);
            let alpha = if norm < 0.045 {
                0
            } else {
                (norm * 220.0 + 35.0).round().clamp(0.0, 255.0) as u8
            };
            let red = (norm * 255.0).round().clamp(0.0, 255.0) as u8;
            let green = (norm * norm * 180.0).round().clamp(0.0, 255.0) as u8;
            let blue = ((1.0 - norm) * 90.0).round().clamp(0.0, 255.0) as u8;
            heatmap.put_pixel(x as u32, y as u32, Rgba([red, green, blue, alpha]));
        }
    }

    Ok((heatmap, diff_sum / total as f64, active as f64 / total as f64))
}

fn aggressiveness_from_config(config: &Config) -> f64 {
    let repair = (config.repair_mode as f64 / 3.0) * 0.55;
    let palette_cleanup = (config.palette_cleanup_mode.min(2) as f64 / 2.0) * 0.25;
    let cleanup = (config.cleanup_mode.min(1) as f64 / 1.0) * 0.10;
    let denoise = (config.prefilter_mode.min(1) as f64 / 1.0) * 0.05;
    clamp01(repair + palette_cleanup + cleanup + denoise)
}

fn compute_grid_regularity(cuts: &[usize], step: f64) -> f64 {
    if cuts.len() < 2 || !step.is_finite() || step <= 0.0 {
        return 0.0;
    }
    let mut deltas = Vec::with_capacity(cuts.len().saturating_sub(1));
    for pair in cuts.windows(2) {
        let delta = pair[1].saturating_sub(pair[0]) as f64;
        if delta > 0.0 {
            deltas.push(delta);
        }
    }
    if deltas.is_empty() {
        return 0.0;
    }
    let mean = deltas.iter().sum::<f64>() / deltas.len() as f64;
    if mean <= 0.0 {
        return 0.0;
    }
    let variance = deltas
        .iter()
        .map(|d| {
            let v = *d - mean;
            v * v
        })
        .sum::<f64>()
        / deltas.len() as f64;
    let stddev = variance.sqrt();
    let cv = stddev / step.max(mean).max(1.0);
    clamp01(1.0 - cv.min(1.0))
}

fn compute_coverage_ratio(img: &RgbaImage) -> f64 {
    let total = (img.width() as usize).saturating_mul(img.height() as usize).max(1);
    let opaque = img.pixels().filter(|p| p.0[3] > 0).count();
    opaque as f64 / total as f64
}

fn compute_palette_compactness(palette_len: usize, target_colors: usize) -> f64 {
    if palette_len == 0 {
        return 1.0;
    }
    if target_colors == 0 {
        return 0.0;
    }
    if palette_len <= target_colors {
        return 1.0;
    }
    clamp01(target_colors as f64 / palette_len as f64)
}

pub fn recommend_variant_for_image_bytes(
    input_bytes: &[u8],
    palette_lock_image_bytes: Option<&[u8]>,
) -> Result<VariantRecommendationReport> {
    let setup = suggest_setup_for_image_bytes(input_bytes)?;
    let base_preset = preset_by_name(&setup.preset_key).unwrap_or(PRESET_AI_SPRITE);

    let mut balanced = Config::default();
    apply_named_preset(&mut balanced, &setup.preset_key)?;
    balanced.prefilter_mode = setup.recommended_prefilter_mode;
    balanced.palette_source = setup.recommended_palette_source;
    balanced.palette_cleanup_mode = setup.recommended_palette_cleanup_mode;
    balanced.cell_color_mode = setup.recommended_cell_color_mode;
    balanced.cleanup_mode = setup.recommended_cleanup_mode;
    balanced.repair_mode = setup.recommended_repair_mode;
    balanced.k_colors = base_preset.k_colors;
    balanced.pixel_size_override = None;

    let mut aggressive = balanced.clone();
    aggressive.prefilter_mode = aggressive.prefilter_mode.max(1);
    aggressive.palette_cleanup_mode = aggressive.palette_cleanup_mode.max(2);
    aggressive.cell_color_mode = 2;
    aggressive.cleanup_mode = aggressive.cleanup_mode.max(1);
    aggressive.repair_mode = aggressive.repair_mode.max(3);

    let mut ultra = Config::default();
    apply_named_preset(&mut ultra, PRESET_ULTRA.key)?;
    ultra.repair_mode = 3;
    ultra.pixel_size_override = None;

    let variants: [(&str, &str, Config); 3] = [
        ("balanced", "Bilanciato", balanced),
        ("aggressive", "Aggressivo", aggressive),
        ("ultra", "Ultra", ultra),
    ];

    let mut metrics: Vec<VariantMetrics> = Vec::new();
    for (key, label, cfg) in variants {
        let out = process_image_bytes_debug_common(input_bytes, Some(cfg.clone()), palette_lock_image_bytes)?;
        let (diff_score, diff_area) = compute_diff_metrics_centered(input_bytes, &out.output_bytes)?;
        let palette_count = out.report.palette.len();
        let aggressiveness = aggressiveness_from_config(&cfg);
        metrics.push(VariantMetrics {
            key: key.to_string(),
            label: label.to_string(),
            diff_score,
            diff_area,
            palette_count,
            aggressiveness,
        });
    }

    let recommendation = recommend_variant_from_metrics(&metrics)?;
    Ok(VariantRecommendationReport {
        recommendation,
        metrics,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct EffectiveConfigReport {
    pub k_colors: usize,
    pub pixel_size_override: Option<f64>,
    pub pixel_size_override_used: bool,
    pub palette_lock_used: bool,
    pub palette_lock_size: usize,
    pub prefilter_mode: &'static str,
    pub palette_source: &'static str,
    pub palette_cleanup_mode: &'static str,
    pub cell_color_mode: &'static str,
    pub dither_mode: &'static str,
    pub color_space: &'static str,
    pub cleanup_mode: &'static str,
    pub repair_mode: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebugPaletteEntry {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
    pub count: u32,
    pub hex: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebugQualityReport {
    pub diff_score: f64,
    pub diff_area: f64,
    pub grid_regularity: f64,
    pub palette_compactness: f64,
    pub coverage_ratio: f64,
    pub overall_score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebugReport {
    pub input_width: u32,
    pub input_height: u32,
    pub output_width: u32,
    pub output_height: u32,
    pub step_x: f64,
    pub step_y: f64,
    pub col_cuts: Vec<usize>,
    pub row_cuts: Vec<usize>,
    pub palette: Vec<DebugPaletteEntry>,
    pub quality: DebugQualityReport,
    pub config: EffectiveConfigReport,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Default)]
pub struct DebugExportOptions {
    pub write_json: bool,
    pub write_overlay: bool,
    pub write_heatmap: bool,
    pub output_dir: Option<PathBuf>,
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub struct ProcessedImage {
    output_bytes: Vec<u8>,
    pixel_size: f64,
    pixel_size_override: bool,
    output_width: u32,
    output_height: u32,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub k_colors: usize,
    pub pixel_size_override: Option<f64>,
    pub prefilter_mode: u32,
    pub palette_source: u32,
    pub palette_cleanup_mode: u32,
    pub cell_color_mode: u32,
    pub dither_mode: u32,
    pub color_space: u32,
    pub cleanup_mode: u32,
    pub repair_mode: u32,
    pub palette_lock_bytes: Option<Vec<u8>>,
    pub debug: DebugExportOptions,
}

#[cfg(not(target_arch = "wasm32"))]
impl From<&Config> for BatchConfig {
    fn from(config: &Config) -> Self {
        Self {
            input_dir: PathBuf::from(&config.input_path),
            output_dir: PathBuf::from(&config.output_path),
            k_colors: config.k_colors,
            pixel_size_override: config.pixel_size_override,
            prefilter_mode: config.prefilter_mode,
            palette_source: config.palette_source,
            palette_cleanup_mode: config.palette_cleanup_mode,
            cell_color_mode: config.cell_color_mode,
            dither_mode: config.dither_mode,
            color_space: config.color_space,
            cleanup_mode: config.cleanup_mode,
            repair_mode: config.repair_mode,
            palette_lock_bytes: None,
            debug: DebugExportOptions::default(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<&BatchConfig> for Config {
    fn from(config: &BatchConfig) -> Self {
        Self {
            k_colors: config.k_colors,
            pixel_size_override: config.pixel_size_override,
            prefilter_mode: config.prefilter_mode,
            palette_source: config.palette_source,
            palette_cleanup_mode: config.palette_cleanup_mode,
            cell_color_mode: config.cell_color_mode,
            dither_mode: config.dither_mode,
            color_space: config.color_space,
            cleanup_mode: config.cleanup_mode,
            repair_mode: config.repair_mode,
            ..Default::default()
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub enum BatchEvent {
    BatchStarted {
        input_dir: PathBuf,
        total: usize,
    },
    Started {
        input: PathBuf,
        index: usize,
        total: usize,
    },
    Finished {
        input: PathBuf,
        output: PathBuf,
        index: usize,
        total: usize,
    },
    Failed {
        input: PathBuf,
        output: PathBuf,
        error: String,
        index: usize,
        total: usize,
    },
    BatchFinished {
        input_dir: PathBuf,
        total: usize,
    },
}

#[allow(dead_code)]
struct DebugOutput {
    output_bytes: Vec<u8>,
    overlay_bytes: Vec<u8>,
    heatmap_bytes: Vec<u8>,
    report: DebugReport,
}

fn process_image_bytes_common(input_bytes: &[u8], config: Option<Config>) -> Result<Vec<u8>> {
    process_image_common(input_bytes, config).map(|out| out.output_bytes)
}

fn process_image_common(input_bytes: &[u8], config: Option<Config>) -> Result<ProcessedImage> {
    let config = config.unwrap_or_default();
    let out = process_image_bytes_debug_common(input_bytes, Some(config.clone()), None)?;
    Ok(ProcessedImage {
        output_bytes: out.output_bytes,
        pixel_size: out.report.step_x,
        pixel_size_override: config.pixel_size_override.is_some(),
        output_width: out.report.output_width,
        output_height: out.report.output_height,
    })
}

fn process_image_bytes_debug_common(
    input_bytes: &[u8],
    config: Option<Config>,
    palette_lock_image_bytes: Option<&[u8]>,
) -> Result<DebugOutput> {
    let config = config.unwrap_or_default();

    let img = image::load_from_memory(input_bytes)?;
    let (width, height) = img.dimensions();

    validate_image_dimensions(width, height)?;

    if let Some(px) = config.pixel_size_override {
        if !px.is_finite() || px < 1.0 || px > (width.min(height) as f64 / 2.0) {
            return Err(PixelSnapperError::InvalidInput(format!(
                "pixel_size_override {:.1} is out of valid range [1, {}]",
                px,
                width.min(height) / 2
            )));
        }
    }

    let rgba_img = img.to_rgba8();
    let rgba_prefiltered = match config.prefilter_mode {
        0 => rgba_img.clone(),
        1 => prefilter_box3_alpha_aware(&rgba_img),
        _ => rgba_img.clone(),
    };

    let mut profile_config = config.clone();
    profile_config.k_colors = profile_config.k_colors.max(16).min(64);

    let quantized_for_profile = quantize_image(&rgba_prefiltered, &profile_config, None)?;
    let (profile_x, profile_y) = compute_profiles(&quantized_for_profile)?;

    // Estimate step sizes
    let step_x_opt = estimate_step_size(&profile_x, &config);
    let step_y_opt = estimate_step_size(&profile_y, &config);

    // Resolve step sizes. Some instabilities so use sibling axis if one fails, or fallback if both fail
    let (step_x, step_y) = resolve_step_sizes(step_x_opt, step_y_opt, width, height, &config);

    #[cfg(not(target_arch = "wasm32"))]
    println!(
        "Pixel size: {:.1}px ({})",
        step_x,
        if config.pixel_size_override.is_some() {
            "override"
        } else {
            "auto-detected"
        }
    );

    let raw_col_cuts = walk(&profile_x, step_x, width as usize, &config)?;
    let raw_row_cuts = walk(&profile_y, step_y, height as usize, &config)?;

    // Two-pass stabilization: first pass with raw cuts, then cross-validate
    let (col_cuts, row_cuts) = stabilize_both_axes(
        &profile_x,
        &profile_y,
        raw_col_cuts,
        raw_row_cuts,
        width as usize,
        height as usize,
        &config,
    );

    #[cfg(not(target_arch = "wasm32"))]
    println!("Output size: {}x{}", col_cuts.len() - 1, row_cuts.len() - 1);

    let locked_palette_rgb = palette_lock_image_bytes
        .map(parse_palette_lock_image_bytes)
        .transpose()?;
    let locked_palette = locked_palette_rgb
        .as_ref()
        .map(|rgb| LockedPalette::from_rgb(rgb, config.color_space));

    let output_img = match config.palette_source {
        1 => resample_cells(&rgba_prefiltered, &col_cuts, &row_cuts, &config, locked_palette.as_ref())?,
        _ => {
            let quantized_img = quantize_image(&rgba_prefiltered, &config, locked_palette.as_ref())?;
            resample_mode(&quantized_img, &col_cuts, &row_cuts)?
        }
    };
    let output_img = if locked_palette.is_some() {
        output_img
    } else {
        apply_palette_cleanup_mode(output_img, &config)
    };
    let output_img = apply_cleanup_mode(output_img, &config);
    let output_img = apply_repair_mode(output_img, &config);

    // Returns bytes for both implementations
    let mut output_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut output_bytes);
    output_img
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| PixelSnapperError::ImageError(e))?;

    let overlay_img = render_debug_overlay(&rgba_img, &col_cuts, &row_cuts);
    let mut overlay_bytes = Vec::new();
    let mut overlay_cursor = std::io::Cursor::new(&mut overlay_bytes);
    overlay_img
        .write_to(&mut overlay_cursor, image::ImageFormat::Png)
        .map_err(PixelSnapperError::ImageError)?;
    let (heatmap_img, diff_score, diff_area) = build_diff_heatmap_centered(&rgba_img, &output_img)?;
    let mut heatmap_bytes = Vec::new();
    let mut heatmap_cursor = std::io::Cursor::new(&mut heatmap_bytes);
    heatmap_img
        .write_to(&mut heatmap_cursor, image::ImageFormat::Png)
        .map_err(PixelSnapperError::ImageError)?;

    Ok(DebugOutput {
        output_bytes,
        overlay_bytes,
        heatmap_bytes,
        report: build_debug_report(
            &config,
            locked_palette_rgb.as_ref().map(|v| v.len()).unwrap_or(0),
            width,
            height,
            col_cuts,
            row_cuts,
            step_x,
            step_y,
            &output_img,
            diff_score,
            diff_area,
        ),
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn process_image_bytes(
    input_bytes: &[u8],
    k_colors: Option<usize>,
    pixel_size_override: Option<f64>,
) -> Result<Vec<u8>> {
    let mut config = Config::default();

    if let Some(k) = k_colors {
        if k == 0 {
            return Err(PixelSnapperError::InvalidInput(
                "k_colors must be greater than 0".to_string(),
            ));
        }
        config.k_colors = k;
    }

    config.pixel_size_override = pixel_size_override;
    process_image_bytes_common(input_bytes, Some(config))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn process_image_bytes_with_config(input_bytes: &[u8], config: Config) -> Result<Vec<u8>> {
    process_image_bytes_common(input_bytes, Some(config))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn process_image_debug_with_config(
    input_bytes: &[u8],
    config: Config,
) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>, DebugReport)> {
    let out = process_image_bytes_debug_common(input_bytes, Some(config), None)?;
    Ok((out.output_bytes, out.overlay_bytes, out.heatmap_bytes, out.report))
}

#[cfg(target_arch = "wasm32")]
fn debug_output_to_js(out: DebugOutput) -> std::result::Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue> {
    let obj = Object::new();
    let palette = Array::new();
    let config_obj = Object::new();
    let quality_obj = Object::new();

    let bytes = Uint8Array::from(out.output_bytes.as_slice());
    let overlay = Uint8Array::from(out.overlay_bytes.as_slice());
    let heatmap = Uint8Array::from(out.heatmap_bytes.as_slice());
    let cols = Uint32Array::new_with_length(out.report.col_cuts.len() as u32);
    for (i, v) in out.report.col_cuts.iter().enumerate() {
        cols.set_index(i as u32, *v as u32);
    }
    let rows = Uint32Array::new_with_length(out.report.row_cuts.len() as u32);
    for (i, v) in out.report.row_cuts.iter().enumerate() {
        rows.set_index(i as u32, *v as u32);
    }
    for entry in &out.report.palette {
        let palette_entry = Object::new();
        Reflect::set(&palette_entry, &JsValue::from_str("r"), &JsValue::from_f64(entry.r as f64))?;
        Reflect::set(&palette_entry, &JsValue::from_str("g"), &JsValue::from_f64(entry.g as f64))?;
        Reflect::set(&palette_entry, &JsValue::from_str("b"), &JsValue::from_f64(entry.b as f64))?;
        Reflect::set(&palette_entry, &JsValue::from_str("a"), &JsValue::from_f64(entry.a as f64))?;
        Reflect::set(
            &palette_entry,
            &JsValue::from_str("count"),
            &JsValue::from_f64(entry.count as f64),
        )?;
        Reflect::set(
            &palette_entry,
            &JsValue::from_str("hex"),
            &JsValue::from_str(&entry.hex),
        )?;
        palette.push(&palette_entry);
    }

    Reflect::set(
        &config_obj,
        &JsValue::from_str("k_colors"),
        &JsValue::from_f64(out.report.config.k_colors as f64),
    )?;
    if let Some(pixel_size_override) = out.report.config.pixel_size_override {
        Reflect::set(
            &config_obj,
            &JsValue::from_str("pixel_size_override"),
            &JsValue::from_f64(pixel_size_override),
        )?;
    }
    Reflect::set(
        &config_obj,
        &JsValue::from_str("pixel_size_override_used"),
        &JsValue::from_bool(out.report.config.pixel_size_override_used),
    )?;
    Reflect::set(
        &config_obj,
        &JsValue::from_str("palette_lock_used"),
        &JsValue::from_bool(out.report.config.palette_lock_used),
    )?;
    Reflect::set(
        &config_obj,
        &JsValue::from_str("palette_lock_size"),
        &JsValue::from_f64(out.report.config.palette_lock_size as f64),
    )?;
    Reflect::set(
        &config_obj,
        &JsValue::from_str("prefilter_mode"),
        &JsValue::from_str(out.report.config.prefilter_mode),
    )?;
    Reflect::set(
        &config_obj,
        &JsValue::from_str("palette_source"),
        &JsValue::from_str(out.report.config.palette_source),
    )?;
    Reflect::set(
        &config_obj,
        &JsValue::from_str("palette_cleanup_mode"),
        &JsValue::from_str(out.report.config.palette_cleanup_mode),
    )?;
    Reflect::set(
        &config_obj,
        &JsValue::from_str("cell_color_mode"),
        &JsValue::from_str(out.report.config.cell_color_mode),
    )?;
    Reflect::set(
        &config_obj,
        &JsValue::from_str("dither_mode"),
        &JsValue::from_str(out.report.config.dither_mode),
    )?;
    Reflect::set(
        &config_obj,
        &JsValue::from_str("color_space"),
        &JsValue::from_str(out.report.config.color_space),
    )?;
    Reflect::set(
        &config_obj,
        &JsValue::from_str("cleanup_mode"),
        &JsValue::from_str(out.report.config.cleanup_mode),
    )?;
    Reflect::set(
        &config_obj,
        &JsValue::from_str("repair_mode"),
        &JsValue::from_str(out.report.config.repair_mode),
    )?;

    Reflect::set(
        &quality_obj,
        &JsValue::from_str("diff_score"),
        &JsValue::from_f64(out.report.quality.diff_score),
    )?;
    Reflect::set(
        &quality_obj,
        &JsValue::from_str("diff_area"),
        &JsValue::from_f64(out.report.quality.diff_area),
    )?;
    Reflect::set(
        &quality_obj,
        &JsValue::from_str("grid_regularity"),
        &JsValue::from_f64(out.report.quality.grid_regularity),
    )?;
    Reflect::set(
        &quality_obj,
        &JsValue::from_str("palette_compactness"),
        &JsValue::from_f64(out.report.quality.palette_compactness),
    )?;
    Reflect::set(
        &quality_obj,
        &JsValue::from_str("coverage_ratio"),
        &JsValue::from_f64(out.report.quality.coverage_ratio),
    )?;
    Reflect::set(
        &quality_obj,
        &JsValue::from_str("overall_score"),
        &JsValue::from_f64(out.report.quality.overall_score),
    )?;

    Reflect::set(&obj, &JsValue::from_str("bytes"), &bytes.into())?;
    Reflect::set(&obj, &JsValue::from_str("overlay_bytes"), &overlay.into())?;
    Reflect::set(&obj, &JsValue::from_str("heatmap_bytes"), &heatmap.into())?;
    Reflect::set(&obj, &JsValue::from_str("col_cuts"), &cols.into())?;
    Reflect::set(&obj, &JsValue::from_str("row_cuts"), &rows.into())?;
    Reflect::set(
        &obj,
        &JsValue::from_str("step_x"),
        &JsValue::from_f64(out.report.step_x),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("step_y"),
        &JsValue::from_f64(out.report.step_y),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("input_width"),
        &JsValue::from_f64(out.report.input_width as f64),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("input_height"),
        &JsValue::from_f64(out.report.input_height as f64),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("output_width"),
        &JsValue::from_f64(out.report.output_width as f64),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("output_height"),
        &JsValue::from_f64(out.report.output_height as f64),
    )?;
    Reflect::set(&obj, &JsValue::from_str("palette"), &palette.into())?;
    Reflect::set(
        &obj,
        &JsValue::from_str("palette_count"),
        &JsValue::from_f64(out.report.palette.len() as f64),
    )?;
    Reflect::set(&obj, &JsValue::from_str("config"), &config_obj.into())?;
    Reflect::set(&obj, &JsValue::from_str("quality"), &quality_obj.into())?;

    Ok(obj.into())
}

#[cfg(target_arch = "wasm32")]
fn preset_to_js_object(preset: PresetSpec) -> std::result::Result<Object, JsValue> {
    let obj = Object::new();
    Reflect::set(&obj, &JsValue::from_str("key"), &JsValue::from_str(preset.key))?;
    Reflect::set(&obj, &JsValue::from_str("label"), &JsValue::from_str(preset.label))?;
    Reflect::set(
        &obj,
        &JsValue::from_str("description"),
        &JsValue::from_str(preset.description),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("k_colors"),
        &JsValue::from_f64(preset.k_colors as f64),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("prefilter_mode"),
        &JsValue::from_f64(preset.prefilter_mode as f64),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("palette_source"),
        &JsValue::from_f64(preset.palette_source as f64),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("palette_cleanup_mode"),
        &JsValue::from_f64(preset.palette_cleanup_mode as f64),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("cell_color_mode"),
        &JsValue::from_f64(preset.cell_color_mode as f64),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("dither_mode"),
        &JsValue::from_f64(preset.dither_mode as f64),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("color_space"),
        &JsValue::from_f64(preset.color_space as f64),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("cleanup_mode"),
        &JsValue::from_f64(preset.cleanup_mode as f64),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("repair_mode"),
        &JsValue::from_f64(preset.repair_mode as f64),
    )?;
    Ok(obj)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_preset_config(preset_name: &str) -> std::result::Result<JsValue, JsValue> {
    let preset = preset_by_name(preset_name).ok_or_else(|| {
        JsValue::from_str(
            "Unknown preset. Available presets: ai-sprite, strict-retro, tileset-cleanup, character-cleanup, icon-cleanup",
        )
    })?;
    Ok(preset_to_js_object(preset)?.into())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn list_presets() -> std::result::Result<JsValue, JsValue> {
    let arr = Array::new();
    for preset in ALL_PRESETS {
        arr.push(&preset_to_js_object(preset)?.into());
    }
    Ok(arr.into())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn suggest_preset(input_bytes: &[u8]) -> std::result::Result<JsValue, JsValue> {
    let suggestion = suggest_preset_for_image_bytes(input_bytes).map_err(JsValue::from)?;
    let obj = Object::new();
    Reflect::set(
        &obj,
        &JsValue::from_str("preset_key"),
        &JsValue::from_str(&suggestion.preset_key),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("reason"),
        &JsValue::from_str(&suggestion.reason),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("opaque_ratio"),
        &JsValue::from_f64(suggestion.opaque_ratio),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("unique_colors"),
        &JsValue::from_f64(suggestion.unique_colors as f64),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("edge_density"),
        &JsValue::from_f64(suggestion.edge_density),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("dominant_alpha"),
        &JsValue::from_bool(suggestion.dominant_alpha),
    )?;
    Ok(obj.into())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn suggest_setup(input_bytes: &[u8]) -> std::result::Result<JsValue, JsValue> {
    let suggestion = suggest_setup_for_image_bytes(input_bytes).map_err(JsValue::from)?;
    let obj = Object::new();
    Reflect::set(
        &obj,
        &JsValue::from_str("preset_key"),
        &JsValue::from_str(&suggestion.preset_key),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("reason"),
        &JsValue::from_str(&suggestion.reason),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("opaque_ratio"),
        &JsValue::from_f64(suggestion.opaque_ratio),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("unique_colors"),
        &JsValue::from_f64(suggestion.unique_colors as f64),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("edge_density"),
        &JsValue::from_f64(suggestion.edge_density),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("dominant_alpha"),
        &JsValue::from_bool(suggestion.dominant_alpha),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("recommended_prefilter_mode"),
        &JsValue::from_f64(suggestion.recommended_prefilter_mode as f64),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("recommended_prefilter_label"),
        &JsValue::from_str(&suggestion.recommended_prefilter_label),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("recommended_palette_source"),
        &JsValue::from_f64(suggestion.recommended_palette_source as f64),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("recommended_palette_source_label"),
        &JsValue::from_str(&suggestion.recommended_palette_source_label),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("recommended_palette_cleanup_mode"),
        &JsValue::from_f64(suggestion.recommended_palette_cleanup_mode as f64),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("recommended_palette_cleanup_label"),
        &JsValue::from_str(&suggestion.recommended_palette_cleanup_label),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("recommended_cell_color_mode"),
        &JsValue::from_f64(suggestion.recommended_cell_color_mode as f64),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("recommended_cell_color_label"),
        &JsValue::from_str(&suggestion.recommended_cell_color_label),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("recommended_cleanup_mode"),
        &JsValue::from_f64(suggestion.recommended_cleanup_mode as f64),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("recommended_cleanup_label"),
        &JsValue::from_str(&suggestion.recommended_cleanup_label),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("recommended_repair_mode"),
        &JsValue::from_f64(suggestion.recommended_repair_mode as f64),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("recommended_repair_label"),
        &JsValue::from_str(&suggestion.recommended_repair_label),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("recommended_trim_transparent"),
        &JsValue::from_bool(suggestion.recommended_trim_transparent),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("recommendation_reason"),
        &JsValue::from_str(&suggestion.recommendation_reason),
    )?;
    Ok(obj.into())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn recommend_variant_from_metrics_json(
    metrics_json: &str,
) -> std::result::Result<JsValue, JsValue> {
    let metrics: Vec<VariantMetrics> =
        serde_json::from_str(metrics_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let reco = recommend_variant_from_metrics(&metrics).map_err(JsValue::from)?;
    let obj = Object::new();
    Reflect::set(
        &obj,
        &JsValue::from_str("key"),
        &JsValue::from_str(&reco.key),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("label"),
        &JsValue::from_str(&reco.label),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("reason"),
        &JsValue::from_str(&reco.reason),
    )?;
    Ok(obj.into())
}

/// WASM entry point
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn process_image(
    input_bytes: &[u8],
    k_colors: Option<u32>,
    pixel_size_override: Option<f64>,
    prefilter_mode: Option<u32>,
    palette_source: Option<u32>,
    palette_cleanup_mode: Option<u32>,
    cell_color_mode: Option<u32>,
    dither_mode: Option<u32>,
    color_space: Option<u32>,
    cleanup_mode: Option<u32>,
    repair_mode: Option<u32>,
) -> std::result::Result<Vec<u8>, wasm_bindgen::JsValue> {
    let mut config = Config::default();
    if let Some(k) = k_colors {
        if k == 0 {
            return Err(wasm_bindgen::JsValue::from_str(
                "k_colors must be greater than 0",
            ));
        }
        config.k_colors = k as usize;
    }

    config.pixel_size_override = pixel_size_override;
    if let Some(v) = prefilter_mode {
        config.prefilter_mode = v;
    }
    if let Some(v) = palette_source {
        config.palette_source = v;
    }
    if let Some(v) = palette_cleanup_mode {
        config.palette_cleanup_mode = v;
    }
    if let Some(v) = cell_color_mode {
        config.cell_color_mode = v;
    }
    if let Some(v) = dither_mode {
        config.dither_mode = v;
    }
    if let Some(v) = color_space {
        config.color_space = v;
    }
    if let Some(v) = cleanup_mode {
        config.cleanup_mode = v;
    }
    if let Some(v) = repair_mode {
        config.repair_mode = v;
    }

    process_image_bytes_common(input_bytes, Some(config))
        .map_err(|e| wasm_bindgen::JsValue::from(e))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn process_image_debug(
    input_bytes: &[u8],
    k_colors: Option<u32>,
    pixel_size_override: Option<f64>,
    prefilter_mode: Option<u32>,
    palette_source: Option<u32>,
    palette_cleanup_mode: Option<u32>,
    cell_color_mode: Option<u32>,
    dither_mode: Option<u32>,
    color_space: Option<u32>,
    cleanup_mode: Option<u32>,
    repair_mode: Option<u32>,
) -> std::result::Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue> {
    let mut config = Config::default();
    if let Some(k) = k_colors {
        if k == 0 {
            return Err(wasm_bindgen::JsValue::from_str(
                "k_colors must be greater than 0",
            ));
        }
        config.k_colors = k as usize;
    }

    config.pixel_size_override = pixel_size_override;
    if let Some(v) = prefilter_mode {
        config.prefilter_mode = v;
    }
    if let Some(v) = palette_source {
        config.palette_source = v;
    }
    if let Some(v) = palette_cleanup_mode {
        config.palette_cleanup_mode = v;
    }
    if let Some(v) = cell_color_mode {
        config.cell_color_mode = v;
    }
    if let Some(v) = dither_mode {
        config.dither_mode = v;
    }
    if let Some(v) = color_space {
        config.color_space = v;
    }
    if let Some(v) = cleanup_mode {
        config.cleanup_mode = v;
    }
    if let Some(v) = repair_mode {
        config.repair_mode = v;
    }

    let out = process_image_bytes_debug_common(input_bytes, Some(config), None)
        .map_err(|e| wasm_bindgen::JsValue::from(e))?;
    debug_output_to_js(out)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn process_image_with_palette_image(
    input_bytes: &[u8],
    palette_image_bytes: &[u8],
    k_colors: Option<u32>,
    pixel_size_override: Option<f64>,
    prefilter_mode: Option<u32>,
    palette_source: Option<u32>,
    palette_cleanup_mode: Option<u32>,
    cell_color_mode: Option<u32>,
    dither_mode: Option<u32>,
    color_space: Option<u32>,
    cleanup_mode: Option<u32>,
    repair_mode: Option<u32>,
) -> std::result::Result<Vec<u8>, wasm_bindgen::JsValue> {
    let mut config = Config::default();
    if let Some(k) = k_colors {
        if k == 0 {
            return Err(wasm_bindgen::JsValue::from_str("k_colors must be greater than 0"));
        }
        config.k_colors = k as usize;
    }

    config.pixel_size_override = pixel_size_override;
    if let Some(v) = prefilter_mode {
        config.prefilter_mode = v;
    }
    if let Some(v) = palette_source {
        config.palette_source = v;
    }
    if let Some(v) = palette_cleanup_mode {
        config.palette_cleanup_mode = v;
    }
    if let Some(v) = cell_color_mode {
        config.cell_color_mode = v;
    }
    if let Some(v) = dither_mode {
        config.dither_mode = v;
    }
    if let Some(v) = color_space {
        config.color_space = v;
    }
    if let Some(v) = cleanup_mode {
        config.cleanup_mode = v;
    }
    if let Some(v) = repair_mode {
        config.repair_mode = v;
    }

    let out = process_image_bytes_debug_common(input_bytes, Some(config), Some(palette_image_bytes))
        .map_err(|e| wasm_bindgen::JsValue::from(e))?;
    Ok(out.output_bytes)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn process_image_debug_with_palette_image(
    input_bytes: &[u8],
    palette_image_bytes: &[u8],
    k_colors: Option<u32>,
    pixel_size_override: Option<f64>,
    prefilter_mode: Option<u32>,
    palette_source: Option<u32>,
    palette_cleanup_mode: Option<u32>,
    cell_color_mode: Option<u32>,
    dither_mode: Option<u32>,
    color_space: Option<u32>,
    cleanup_mode: Option<u32>,
    repair_mode: Option<u32>,
) -> std::result::Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue> {
    let mut config = Config::default();
    if let Some(k) = k_colors {
        if k == 0 {
            return Err(wasm_bindgen::JsValue::from_str("k_colors must be greater than 0"));
        }
        config.k_colors = k as usize;
    }

    config.pixel_size_override = pixel_size_override;
    if let Some(v) = prefilter_mode {
        config.prefilter_mode = v;
    }
    if let Some(v) = palette_source {
        config.palette_source = v;
    }
    if let Some(v) = palette_cleanup_mode {
        config.palette_cleanup_mode = v;
    }
    if let Some(v) = cell_color_mode {
        config.cell_color_mode = v;
    }
    if let Some(v) = dither_mode {
        config.dither_mode = v;
    }
    if let Some(v) = color_space {
        config.color_space = v;
    }
    if let Some(v) = cleanup_mode {
        config.cleanup_mode = v;
    }
    if let Some(v) = repair_mode {
        config.repair_mode = v;
    }

    let out = process_image_bytes_debug_common(input_bytes, Some(config), Some(palette_image_bytes))
        .map_err(|e| wasm_bindgen::JsValue::from(e))?;
    debug_output_to_js(out)
}

}

fn build_debug_report(
    config: &Config,
    palette_lock_size: usize,
    input_width: u32,
    input_height: u32,
    col_cuts: Vec<usize>,
    row_cuts: Vec<usize>,
    step_x: f64,
    step_y: f64,
    output_img: &RgbaImage,
    diff_score: f64,
    diff_area: f64,
) -> DebugReport {
    let output_width = col_cuts.len().saturating_sub(1) as u32;
    let output_height = row_cuts.len().saturating_sub(1) as u32;
    let palette = extract_palette(output_img);
    let grid_regularity = (
        compute_grid_regularity(&col_cuts, step_x) + compute_grid_regularity(&row_cuts, step_y)
    ) * 0.5;
    let palette_compactness = compute_palette_compactness(palette.len(), config.k_colors);
    let coverage_ratio = compute_coverage_ratio(output_img);
    let fidelity_score = 1.0
        - ((diff_score / 96.0).clamp(0.0, 1.0) * 0.7 + (diff_area / 0.45).clamp(0.0, 1.0) * 0.3);
    let overall_score = clamp01(
        grid_regularity * 0.35
            + palette_compactness * 0.20
            + coverage_ratio * 0.10
            + fidelity_score * 0.35,
    );

    DebugReport {
        input_width,
        input_height,
        output_width,
        output_height,
        step_x,
        step_y,
        col_cuts,
        row_cuts,
        palette,
        quality: DebugQualityReport {
            diff_score,
            diff_area,
            grid_regularity,
            palette_compactness,
            coverage_ratio,
            overall_score,
        },
        config: EffectiveConfigReport {
            k_colors: config.k_colors,
            pixel_size_override: config.pixel_size_override,
            pixel_size_override_used: config.pixel_size_override.is_some(),
            palette_lock_used: palette_lock_size > 0,
            palette_lock_size,
            prefilter_mode: prefilter_mode_label(config.prefilter_mode),
            palette_source: palette_source_label(config.palette_source),
            palette_cleanup_mode: palette_cleanup_mode_label(config.palette_cleanup_mode),
            cell_color_mode: cell_color_mode_label(config.cell_color_mode),
            dither_mode: dither_mode_label(config.dither_mode),
            color_space: color_space_label(config.color_space),
            cleanup_mode: cleanup_mode_label(config.cleanup_mode),
            repair_mode: repair_mode_label(config.repair_mode),
        },
    }
}

fn extract_palette(img: &RgbaImage) -> Vec<DebugPaletteEntry> {
    let mut counts: HashMap<[u8; 4], u32> = HashMap::new();
    for pixel in img.pixels() {
        let rgba = pixel.0;
        if rgba[3] == 0 {
            continue;
        }
        *counts.entry(rgba).or_insert(0) += 1;
    }

    let mut palette: Vec<DebugPaletteEntry> = counts
        .into_iter()
        .map(|(rgba, count)| DebugPaletteEntry {
            r: rgba[0],
            g: rgba[1],
            b: rgba[2],
            a: rgba[3],
            count,
            hex: if rgba[3] == 255 {
                format!("#{:02X}{:02X}{:02X}", rgba[0], rgba[1], rgba[2])
            } else {
                format!("#{:02X}{:02X}{:02X}{:02X}", rgba[0], rgba[1], rgba[2], rgba[3])
            },
        })
        .collect();

    palette.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.hex.cmp(&b.hex))
    });
    palette
}

fn render_debug_overlay(img: &RgbaImage, col_cuts: &[usize], row_cuts: &[usize]) -> RgbaImage {
    let mut overlay = img.clone();
    let guide = [75u8, 227u8, 255u8, 196u8];

    for &x in col_cuts {
        if x >= overlay.width() as usize {
            continue;
        }
        let x = x as u32;
        for y in 0..overlay.height() {
            blend_rgba_pixel(overlay.get_pixel_mut(x, y), guide);
        }
    }

    for &y in row_cuts {
        if y >= overlay.height() as usize {
            continue;
        }
        let y = y as u32;
        for x in 0..overlay.width() {
            blend_rgba_pixel(overlay.get_pixel_mut(x, y), guide);
        }
    }

    overlay
}

fn blend_rgba_pixel(dst: &mut Rgba<u8>, src: [u8; 4]) {
    let alpha = src[3] as f32 / 255.0;
    let inv_alpha = 1.0 - alpha;
    for i in 0..3 {
        dst.0[i] = (dst.0[i] as f32 * inv_alpha + src[i] as f32 * alpha).round() as u8;
    }
    dst.0[3] = 255;
}

fn prefilter_mode_label(mode: u32) -> &'static str {
    match mode {
        1 => "box3",
        _ => "off",
    }
}

fn palette_source_label(mode: u32) -> &'static str {
    match mode {
        1 => "cells",
        _ => "pixels",
    }
}

fn palette_cleanup_mode_label(mode: u32) -> &'static str {
    match mode {
        2 => "strict",
        1 => "basic",
        _ => "off",
    }
}

fn cell_color_mode_label(mode: u32) -> &'static str {
    match mode {
        2 => "medoid",
        1 => "dominant",
        _ => "mean",
    }
}

fn dither_mode_label(mode: u32) -> &'static str {
    match mode {
        1 => "fs",
        _ => "off",
    }
}

fn color_space_label(mode: u32) -> &'static str {
    match mode {
        1 => "linear",
        _ => "srgb",
    }
}

fn cleanup_mode_label(mode: u32) -> &'static str {
    match mode {
        1 => "basic",
        _ => "off",
    }
}

fn repair_mode_label(mode: u32) -> &'static str {
    match mode {
        3 => "ultra",
        2 => "smart",
        1 => "basic",
        _ => "off",
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
fn main() -> Result<()> {
    let config = parse_args().unwrap_or_default();
    process(&config)
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
fn parse_args() -> Option<Config> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        return None;
    }

    let mut config = Config {
        input_path: args[1].clone(),
        output_path: args[2].clone(),
        ..Default::default()
    };

    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--pixel-size" => {
                let Some(val) = args.get(i + 1) else {
                    eprintln!("Warning: --pixel-size requires a value");
                    break;
                };

                match val.parse::<f64>() {
                    Ok(px) if px.is_finite() && px > 0.0 => config.pixel_size_override = Some(px),
                    _ => eprintln!("Warning: invalid --pixel-size '{}', ignoring", val),
                }
                i += 2;
            }
            arg if arg.starts_with("--") => {
                eprintln!("Warning: unknown argument '{}', ignoring", arg);
                i += 1;
            }
            k_arg => {
                match k_arg.parse::<usize>() {
                    Ok(k) if k > 0 => config.k_colors = k,
                    _ => eprintln!(
                        "Warning: invalid k_colors '{}', falling back to default ({})",
                        k_arg, config.k_colors
                    ),
                }
                i += 1;
            }
        }
    }

    Some(config)
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
fn process(config: &Config) -> Result<()> {
    let input_path = Path::new(&config.input_path);
    if input_path.is_dir() {
        process_batch(config)
    } else {
        process_single(config)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
fn process_single(config: &Config) -> Result<()> {
    let input_path = Path::new(&config.input_path);
    let output_path = Path::new(&config.output_path);
    let processed = process_file(input_path, output_path, config)?;
    println!("Processing: {}", config.input_path);
    print_processed_image(
        processed.pixel_size,
        processed.pixel_size_override,
        processed.output_width,
        processed.output_height,
    );
    println!("Saved to: {}", config.output_path);
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
fn process_batch(config: &Config) -> Result<()> {
    process_batch_with_reporter(&BatchConfig::from(config), |event| match event {
        BatchEvent::BatchStarted { input_dir, total } => {
            println!(
                "Batch processing {} image{} from: {}",
                total,
                if total == 1 { "" } else { "s" },
                input_dir.display()
            );
        }
        BatchEvent::Started {
            input,
            index,
            total,
        } => {
            println!("Processing {}/{}: {}", index + 1, total, input.display());
        }
        BatchEvent::Finished {
            input,
            output,
            index,
            total,
        } => {
            println!(
                "Done {}/{}: {} -> {}",
                index + 1,
                total,
                input.display(),
                output.display()
            );
        }
        BatchEvent::Failed {
            input,
            output,
            error,
            index,
            total,
        } => {
            eprintln!(
                "Failed {}/{}: {} -> {} ({})",
                index + 1,
                total,
                input.display(),
                output.display(),
                error
            );
        }
        BatchEvent::BatchFinished { input_dir, total } => {
            println!(
                "Processed {} image{} in: {}",
                total,
                if total == 1 { "" } else { "s" },
                input_dir.display()
            );
        }
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn process_batch_with_reporter<F>(config: &BatchConfig, reporter: F) -> Result<()>
where
    F: Fn(BatchEvent) + Send + Sync,
{
    let input_dir = &config.input_dir;
    let output_dir = &config.output_dir;

    if input_dir == output_dir {
        return Err(PixelSnapperError::InvalidInput(
            "Batch output directory must be different from the input directory".to_string(),
        ));
    }

    if output_dir.exists() && !output_dir.is_dir() {
        return Err(PixelSnapperError::InvalidInput(format!(
            "Batch output path must be a directory: {}",
            output_dir.display()
        )));
    }

    std::fs::create_dir_all(output_dir).map_err(|e| {
        PixelSnapperError::ProcessingError(format!(
            "Failed to create output directory '{}': {}",
            output_dir.display(),
            e
        ))
    })?;

    let mut inputs = collect_batch_inputs(input_dir)?;
    inputs.sort();

    if inputs.is_empty() {
        return Err(PixelSnapperError::InvalidInput(format!(
            "No supported images found in '{}'",
            input_dir.display()
        )));
    }

    let items: Vec<(PathBuf, PathBuf)> = inputs
        .iter()
        .map(|input| Ok((input.clone(), get_output_path(output_dir, input_dir, input)?)))
        .collect::<Result<_>>()?;

    reporter(BatchEvent::BatchStarted {
        input_dir: input_dir.clone(),
        total: items.len(),
    });

    let results: Vec<(PathBuf, Result<()>)> = items
        .par_iter()
        .enumerate()
        .map(|(index, (input, output))| {
            reporter(BatchEvent::Started {
                input: input.clone(),
                index,
                total: items.len(),
            });
            let item_config = Config::from(config);
            let debug_relative_dir = input
                .strip_prefix(input_dir)
                .ok()
                .and_then(|rel| rel.parent())
                .filter(|p| !p.as_os_str().is_empty())
                .map(Path::to_path_buf);
            let result = process_file_with_debug_exports_internal(
                input,
                output,
                &item_config,
                &config.debug,
                config.palette_lock_bytes.as_deref(),
                debug_relative_dir.as_deref(),
            )
            .map(|_| ());
            match &result {
                Ok(()) => reporter(BatchEvent::Finished {
                    input: input.clone(),
                    output: output.clone(),
                    index,
                    total: items.len(),
                }),
                Err(err) => reporter(BatchEvent::Failed {
                    input: input.clone(),
                    output: output.clone(),
                    error: err.to_string(),
                    index,
                    total: items.len(),
                }),
            }
            (input.clone(), result)
        })
        .collect();

    let mut failures = Vec::new();
    for (input, result) in results {
        match result {
            Ok(()) => {}
            Err(err) => failures.push(format!("{} ({})", input.display(), err)),
        }
    }

    if failures.is_empty() {
        reporter(BatchEvent::BatchFinished {
            input_dir: input_dir.clone(),
            total: items.len(),
        });
        Ok(())
    } else {
        Err(PixelSnapperError::ProcessingError(format!(
            "Batch completed with {} failure{}: {}",
            failures.len(),
            if failures.len() == 1 { "" } else { "s" },
            failures.join("; ")
        )))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn process_file(input_path: &Path, output_path: &Path, config: &Config) -> Result<ProcessedImage> {
    process_file_with_debug_exports(
        input_path,
        output_path,
        config,
        &DebugExportOptions::default(),
        None,
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub fn process_file_with_debug_exports(
    input_path: &Path,
    output_path: &Path,
    config: &Config,
    debug: &DebugExportOptions,
    palette_lock_image_bytes: Option<&[u8]>,
) -> Result<ProcessedImage> {
    process_file_with_debug_exports_internal(
        input_path,
        output_path,
        config,
        debug,
        palette_lock_image_bytes,
        None,
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn process_file_with_debug_exports_internal(
    input_path: &Path,
    output_path: &Path,
    config: &Config,
    debug: &DebugExportOptions,
    palette_lock_image_bytes: Option<&[u8]>,
    debug_relative_dir: Option<&Path>,
) -> Result<ProcessedImage> {
    let img_bytes = std::fs::read(input_path).map_err(|e| {
        PixelSnapperError::ProcessingError(format!(
            "Failed to read input file '{}': {}",
            input_path.display(),
            e
        ))
    })?;

    let out = process_image_bytes_debug_common(&img_bytes, Some(config.clone()), palette_lock_image_bytes)?;
    let output_bytes = out.output_bytes;
    let overlay_bytes = out.overlay_bytes;
    let heatmap_bytes = out.heatmap_bytes;
    let report = out.report;

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            PixelSnapperError::ProcessingError(format!(
                "Failed to create output directory '{}': {}",
                parent.display(),
                e
            ))
        })?;
    }

    std::fs::write(output_path, &output_bytes).map_err(|e| {
        PixelSnapperError::ProcessingError(format!(
            "Failed to write output file '{}': {}",
            output_path.display(),
            e
        ))
    })?;

    write_debug_exports(
        input_path,
        output_path,
        debug,
        debug_relative_dir,
        &report,
        &overlay_bytes,
        &heatmap_bytes,
    )?;

    Ok(ProcessedImage {
        output_bytes,
        pixel_size: report.step_x,
        pixel_size_override: config.pixel_size_override.is_some(),
        output_width: report.output_width,
        output_height: report.output_height,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn print_processed_image(
    pixel_size: f64,
    pixel_size_override: bool,
    output_width: u32,
    output_height: u32,
) {
    println!(
        "Pixel size: {:.1}px ({})",
        pixel_size,
        if pixel_size_override {
            "override"
        } else {
            "auto-detected"
        }
    );
    println!("Output size: {}x{}", output_width, output_height);
}

#[cfg(not(target_arch = "wasm32"))]
fn write_debug_exports(
    input_path: &Path,
    output_path: &Path,
    debug: &DebugExportOptions,
    debug_relative_dir: Option<&Path>,
    report: &DebugReport,
    overlay_bytes: &[u8],
    heatmap_bytes: &[u8],
) -> Result<()> {
    if !debug.write_json && !debug.write_overlay && !debug.write_heatmap {
        return Ok(());
    }

    let mut base_dir = debug
        .output_dir
        .clone()
        .unwrap_or_else(|| output_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf());
    if debug.output_dir.is_some() {
        if let Some(rel) = debug_relative_dir {
            base_dir = base_dir.join(rel);
        }
    }
    std::fs::create_dir_all(&base_dir).map_err(|e| {
        PixelSnapperError::ProcessingError(format!(
            "Failed to create debug directory '{}': {}",
            base_dir.display(),
            e
        ))
    })?;

    let stem = output_path
        .file_stem()
        .or_else(|| input_path.file_stem())
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .ok_or_else(|| {
            PixelSnapperError::InvalidInput(format!(
                "Unable to derive debug file stem from '{}'",
                output_path.display()
            ))
        })?;

    if debug.write_json {
        let json_path = base_dir.join(format!("{}.debug.json", stem));
        let json = serde_json::to_vec_pretty(report).map_err(|e| {
            PixelSnapperError::ProcessingError(format!(
                "Failed to serialize debug report for '{}': {}",
                json_path.display(),
                e
            ))
        })?;
        std::fs::write(&json_path, json).map_err(|e| {
            PixelSnapperError::ProcessingError(format!(
                "Failed to write debug report '{}': {}",
                json_path.display(),
                e
            ))
        })?;
    }

    if debug.write_overlay {
        let overlay_path = base_dir.join(format!("{}.overlay.png", stem));
        std::fs::write(&overlay_path, overlay_bytes).map_err(|e| {
            PixelSnapperError::ProcessingError(format!(
                "Failed to write debug overlay '{}': {}",
                overlay_path.display(),
                e
            ))
        })?;
    }

    if debug.write_heatmap {
        let heatmap_path = base_dir.join(format!("{}.heatmap.png", stem));
        std::fs::write(&heatmap_path, heatmap_bytes).map_err(|e| {
            PixelSnapperError::ProcessingError(format!(
                "Failed to write debug heatmap '{}': {}",
                heatmap_path.display(),
                e
            ))
        })?;
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_batch_inputs(input_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut inputs = Vec::new();
    collect_batch_inputs_recursive(input_dir, input_dir, &mut inputs)?;
    Ok(inputs)
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_batch_inputs_recursive(root: &Path, current: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries = std::fs::read_dir(current).map_err(|e| {
        PixelSnapperError::ProcessingError(format!(
            "Failed to read input directory '{}': {}",
            current.display(),
            e
        ))
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            PixelSnapperError::ProcessingError(format!(
                "Failed to read an entry from '{}': {}",
                current.display(),
                e
            ))
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_batch_inputs_recursive(root, &path, out)?;
            continue;
        }
        if path.is_file() && is_supported_image_path(&path) {
            if path.strip_prefix(root).is_ok() {
                out.push(path);
            }
        }
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn is_supported_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg"))
        .unwrap_or(false)
}

#[cfg(not(target_arch = "wasm32"))]
fn get_output_path(output_dir: &Path, input_dir: &Path, input_path: &Path) -> Result<PathBuf> {
    let rel = input_path.strip_prefix(input_dir).map_err(|_| {
        PixelSnapperError::InvalidInput(format!(
            "Input path '{}' is not under input directory '{}'",
            input_path.display(),
            input_dir.display()
        ))
    })?;
    let rel_parent = rel.parent().unwrap_or_else(|| Path::new(""));
    let stem = input_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .ok_or_else(|| {
            PixelSnapperError::InvalidInput(format!(
                "Input path has no file stem: {}",
                input_path.display()
            ))
        })?;

    Ok(output_dir.join(rel_parent).join(format!("{}.png", stem)))
}


fn validate_image_dimensions(width: u32, height: u32) -> Result<()> {
    if width == 0 || height == 0 {
        return Err(PixelSnapperError::InvalidInput(
            "Image dimensions cannot be zero".to_string(),
        ));
    }
    if width > 10000 || height > 10000 {
        return Err(PixelSnapperError::InvalidInput(
            "Image dimensions too large (max 10000x10000)".to_string(),
        ));
    }
    Ok(())
}

fn srgb_u8_to_linear_255(v: u8) -> f32 {
    let s = v as f32 / 255.0;
    (s.powf(2.2) * 255.0).clamp(0.0, 255.0)
}

fn linear_255_to_srgb_u8(v: f32) -> u8 {
    let l = (v / 255.0).clamp(0.0, 1.0);
    (l.powf(1.0 / 2.2) * 255.0).round().clamp(0.0, 255.0) as u8
}

fn rgb_to_space_255(rgb: [u8; 3], color_space: u32) -> [f32; 3] {
    match color_space {
        1 => [
            srgb_u8_to_linear_255(rgb[0]),
            srgb_u8_to_linear_255(rgb[1]),
            srgb_u8_to_linear_255(rgb[2]),
        ],
        _ => [rgb[0] as f32, rgb[1] as f32, rgb[2] as f32],
    }
}

fn space_255_to_rgb(space: [f32; 3], color_space: u32) -> [u8; 3] {
    match color_space {
        1 => [
            linear_255_to_srgb_u8(space[0]),
            linear_255_to_srgb_u8(space[1]),
            linear_255_to_srgb_u8(space[2]),
        ],
        _ => [
            space[0].round().clamp(0.0, 255.0) as u8,
            space[1].round().clamp(0.0, 255.0) as u8,
            space[2].round().clamp(0.0, 255.0) as u8,
        ],
    }
}

#[derive(Debug, Clone)]
struct LockedPalette {
    colors_space: Vec<[f32; 3]>,
}

impl LockedPalette {
    fn from_rgb(colors_rgb: &[[u8; 3]], color_space: u32) -> Self {
        let colors_space = colors_rgb
            .iter()
            .map(|rgb| rgb_to_space_255(*rgb, color_space))
            .collect();
        Self { colors_space }
    }
}

fn parse_palette_lock_image_bytes(palette_image_bytes: &[u8]) -> Result<Vec<[u8; 3]>> {
    let img = image::load_from_memory(palette_image_bytes)?;
    let rgba = img.to_rgba8();
    let mut counts: HashMap<[u8; 4], u32> = HashMap::new();
    for p in rgba.pixels() {
        let rgba = p.0;
        if rgba[3] == 0 {
            continue;
        }
        *counts.entry(rgba).or_insert(0) += 1;
    }

    if counts.is_empty() {
        return Err(PixelSnapperError::InvalidInput(
            "palette lock image has no opaque pixels".to_string(),
        ));
    }

    let mut colors: Vec<([u8; 4], u32)> = counts.into_iter().collect();
    colors.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut out = Vec::new();
    let mut seen: HashMap<[u8; 3], ()> = HashMap::new();
    for (rgba, _) in colors {
        let rgb = [rgba[0], rgba[1], rgba[2]];
        if seen.contains_key(&rgb) {
            continue;
        }
        seen.insert(rgb, ());
        out.push(rgb);
        if out.len() >= 256 {
            break;
        }
    }
    Ok(out)
}

fn quantize_image(img: &RgbaImage, config: &Config, locked_palette: Option<&LockedPalette>) -> Result<RgbaImage> {
    if config.k_colors == 0 {
        return Err(PixelSnapperError::InvalidInput(
            "Number of colors must be greater than 0".to_string(),
        ));
    }

    if let Some(locked_palette) = locked_palette {
        if locked_palette.colors_space.is_empty() {
            return Ok(img.clone());
        }

        fn dist_sq(p: &[f32; 3], c: &[f32; 3]) -> f32 {
            let dr = p[0] - c[0];
            let dg = p[1] - c[1];
            let db = p[2] - c[2];
            dr * dr + dg * dg + db * db
        }

        let mut new_img = RgbaImage::new(img.width(), img.height());
        for (x, y, pixel) in img.enumerate_pixels() {
            if pixel[3] == 0 {
                new_img.put_pixel(x, y, *pixel);
                continue;
            }
            let p = rgb_to_space_255([pixel[0], pixel[1], pixel[2]], config.color_space);
            let mut min_dist = f32::MAX;
            let mut best = locked_palette.colors_space[0];
            for c in &locked_palette.colors_space {
                let d = dist_sq(&p, c);
                if d < min_dist {
                    min_dist = d;
                    best = *c;
                }
            }
            let rgb = space_255_to_rgb(best, config.color_space);
            new_img.put_pixel(x, y, Rgba([rgb[0], rgb[1], rgb[2], pixel[3]]));
        }
        return Ok(new_img);
    }

    let opaque_pixels: Vec<[f32; 3]> = img
        .pixels()
        .filter_map(|p| {
            if p[3] == 0 {
                None
            } else {
                Some(rgb_to_space_255([p[0], p[1], p[2]], config.color_space))
            }
        })
        .collect();
    let n_pixels = opaque_pixels.len();
    if n_pixels == 0 {
        return Ok(img.clone());
    }

    let mut rng = ChaCha8Rng::seed_from_u64(config.k_seed);
    let k = config.k_colors.min(n_pixels);

    fn sample_index(rng: &mut ChaCha8Rng, upper: usize) -> usize {
        debug_assert!(upper > 0);
        let upper = upper as u64;
        rng.gen_range(0..upper) as usize
    }

    fn dist_sq(p: &[f32; 3], c: &[f32; 3]) -> f32 {
        let dr = p[0] - c[0];
        let dg = p[1] - c[1];
        let db = p[2] - c[2];
        dr * dr + dg * dg + db * db
    }

    let mut centroids: Vec<[f32; 3]> = Vec::with_capacity(k);
    let first_idx = sample_index(&mut rng, n_pixels);
    centroids.push(opaque_pixels[first_idx]);
    let mut distances = vec![f32::MAX; n_pixels];

    // Maybe try a faster algorithm for this? like https://crates.io/crates/kmeans_colors
    for _ in 1..k {
        let last_c = centroids.last().unwrap();
        let mut sum_sq_dist = 0.0;

        for (i, p) in opaque_pixels.iter().enumerate() {
            let d_sq = dist_sq(p, last_c);
            if d_sq < distances[i] {
                distances[i] = d_sq;
            }
            sum_sq_dist += distances[i];
        }

        if sum_sq_dist <= 0.0 {
            let idx = sample_index(&mut rng, n_pixels);
            centroids.push(opaque_pixels[idx]);
        } else {
            let dist = WeightedIndex::new(&distances).map_err(|e| {
                PixelSnapperError::ProcessingError(format!("Failed to sample new centroid: {}", e))
            })?;
            let idx = dist.sample(&mut rng);
            centroids.push(opaque_pixels[idx]);
        }
    }

    let mut prev_centroids = centroids.clone();
    for iteration in 0..config.max_kmeans_iterations {
        let mut sums = vec![[0.0f32; 3]; k];
        let mut counts = vec![0usize; k];

        for p in &opaque_pixels {
            let mut min_dist = f32::MAX;
            let mut best_k = 0;

            for (i, c) in centroids.iter().enumerate() {
                let d = dist_sq(p, c);
                if d < min_dist {
                    min_dist = d;
                    best_k = i;
                }
            }
            sums[best_k][0] += p[0];
            sums[best_k][1] += p[1];
            sums[best_k][2] += p[2];
            counts[best_k] += 1;
        }

        for i in 0..k {
            if counts[i] > 0 {
                let fcount = counts[i] as f32;
                centroids[i] = [
                    sums[i][0] / fcount,
                    sums[i][1] / fcount,
                    sums[i][2] / fcount,
                ];
            }
        }

        if iteration > 0 {
            let mut max_movement = 0.0f32;
            for (new_c, old_c) in centroids.iter().zip(prev_centroids.iter()) {
                let movement = dist_sq(new_c, old_c);
                if movement > max_movement {
                    max_movement = movement;
                }
            }

            if max_movement < 0.01 {
                break;
            }
        }

        prev_centroids.copy_from_slice(&centroids);
    }

    let mut new_img = RgbaImage::new(img.width(), img.height());
    for (x, y, pixel) in img.enumerate_pixels() {
        if pixel[3] == 0 {
            new_img.put_pixel(x, y, *pixel);
            continue;
        }
        let p = rgb_to_space_255([pixel[0], pixel[1], pixel[2]], config.color_space);
        let mut min_dist = f32::MAX;
        let mut best_c = [pixel[0], pixel[1], pixel[2]];

        for c in &centroids {
            let d = dist_sq(&p, c);
            if d < min_dist {
                min_dist = d;
                best_c = space_255_to_rgb(*c, config.color_space);
            }
        }
        new_img.put_pixel(x, y, Rgba([best_c[0], best_c[1], best_c[2], pixel[3]]));
    }
    Ok(new_img)
}

fn compute_profiles(img: &RgbaImage) -> Result<(Vec<f64>, Vec<f64>)> {
    let (w, h) = img.dimensions();

    if w < 3 || h < 3 {
        return Err(PixelSnapperError::InvalidInput(
            "Image too small (minimum 3x3)".to_string(),
        ));
    }

    let mut col_proj = vec![0.0; w as usize];
    let mut row_proj = vec![0.0; h as usize];

    let gray = |x, y| {
        let p = img.get_pixel(x, y);
        if p[3] == 0 {
            0.0
        } else {
            0.299 * p[0] as f64 + 0.587 * p[1] as f64 + 0.114 * p[2] as f64
        }
    };

    // kernels: [-1, 0, 1]
    for y in 0..h {
        for x in 1..w - 1 {
            let left = gray(x - 1, y);
            let right = gray(x + 1, y);
            let grad = (right - left).abs();
            col_proj[x as usize] += grad;
        }
    }
    for x in 0..w {
        for y in 1..h - 1 {
            let top = gray(x, y - 1);
            let bottom = gray(x, y + 1);
            let grad = (bottom - top).abs();
            row_proj[y as usize] += grad;
        }
    }

    Ok((col_proj, row_proj))
}

fn estimate_step_size(profile: &[f64], config: &Config) -> Option<f64> {
    if profile.is_empty() {
        return None;
    }

    let max_val = profile.iter().cloned().fold(0.0 / 0.0, f64::max);
    if max_val == 0.0 {
        return estimate_step_size_autocorr(profile, config);
    }
    let threshold = max_val * config.peak_threshold_multiplier;

    let mut peaks = Vec::new();
    for i in 1..profile.len() - 1 {
        if profile[i] > threshold && profile[i] > profile[i - 1] && profile[i] > profile[i + 1] {
            peaks.push(i);
        }
    }

    if peaks.len() < 2 {
        return estimate_step_size_autocorr(profile, config);
    }

    let mut clean_peaks = vec![peaks[0]];
    for &p in peaks.iter().skip(1) {
        if p - clean_peaks.last().unwrap() > (config.peak_distance_filter - 1) {
            clean_peaks.push(p);
        }
    }

    if clean_peaks.len() < 2 {
        return estimate_step_size_autocorr(profile, config);
    }

    // Compute diffs
    let mut diffs: Vec<f64> = clean_peaks
        .windows(2)
        .map(|w| (w[1] - w[0]) as f64)
        .collect();

    // Median
    diffs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    Some(diffs[diffs.len() / 2])
}

fn estimate_step_size_autocorr(profile: &[f64], config: &Config) -> Option<f64> {
    if profile.len() < 4 {
        return None;
    }

    let mean = profile.iter().sum::<f64>() / profile.len() as f64;
    let mut var = 0.0;
    for &v in profile {
        let d = v - mean;
        var += d * d;
    }
    if var <= 0.0 {
        return None;
    }

    let max_lag = config
        .autocorr_max_lag
        .min(profile.len().saturating_sub(1))
        .min(profile.len() / 2)
        .max(2);

    let mut best_lag = None;
    let mut best_score = f64::MIN;
    for lag in 2..=max_lag {
        let mut score = 0.0;
        for i in 0..profile.len() - lag {
            score += (profile[i] - mean) * (profile[i + lag] - mean);
        }
        if score > best_score {
            best_score = score;
            best_lag = Some(lag);
        }
    }

    best_lag.map(|l| l as f64)
}

fn resolve_step_sizes(
    step_x_opt: Option<f64>,
    step_y_opt: Option<f64>,
    width: u32,
    height: u32,
    config: &Config,
) -> (f64, f64) {
    if let Some(px) = config.pixel_size_override {
        return (px, px);
    }

    match (step_x_opt, step_y_opt) {
        (Some(sx), Some(sy)) => {
            let ratio = if sx > sy { sx / sy } else { sy / sx };
            if ratio > config.max_step_ratio {
                let smaller = sx.min(sy);
                (smaller, smaller)
            } else {
                let avg = (sx + sy) / 2.0;
                (avg, avg)
            }
        }

        (Some(sx), None) => (sx, sx),

        (None, Some(sy)) => (sy, sy),

        (None, None) => {
            let fallback_step =
                ((width.min(height) as f64) / config.fallback_target_segments as f64).max(1.0);
            (fallback_step, fallback_step)
        }
    }
}

fn stabilize_both_axes(
    profile_x: &[f64],
    profile_y: &[f64],
    raw_col_cuts: Vec<usize>,
    raw_row_cuts: Vec<usize>,
    width: usize,
    height: usize,
    config: &Config,
) -> (Vec<usize>, Vec<usize>) {
    let col_cuts_pass1 = stabilize_cuts(
        profile_x,
        raw_col_cuts.clone(),
        width,
        &raw_row_cuts,
        height,
        config,
    );
    let row_cuts_pass1 = stabilize_cuts(
        profile_y,
        raw_row_cuts.clone(),
        height,
        &raw_col_cuts,
        width,
        config,
    );

    // Check if the results are coherent
    let col_cells = col_cuts_pass1.len().saturating_sub(1).max(1);
    let row_cells = row_cuts_pass1.len().saturating_sub(1).max(1);
    let col_step = width as f64 / col_cells as f64;
    let row_step = height as f64 / row_cells as f64;

    let step_ratio = if col_step > row_step {
        col_step / row_step
    } else {
        row_step / col_step
    };

    if step_ratio > config.max_step_ratio {
        let target_step = col_step.min(row_step);

        let final_col_cuts = if col_step > target_step * 1.2 {
            snap_uniform_cuts(
                profile_x,
                width,
                target_step,
                config,
                config.min_cuts_per_axis,
            )
        } else {
            col_cuts_pass1
        };

        let final_row_cuts = if row_step > target_step * 1.2 {
            snap_uniform_cuts(
                profile_y,
                height,
                target_step,
                config,
                config.min_cuts_per_axis,
            )
        } else {
            row_cuts_pass1
        };

        (final_col_cuts, final_row_cuts)
    } else {
        (col_cuts_pass1, row_cuts_pass1)
    }
}

// Tried uniform grid instead of an elastic-ish walker, but the result was a bit worse.
// Keeping the walker for now. But some distortions might happen...
fn walk(profile: &[f64], step_size: f64, limit: usize, config: &Config) -> Result<Vec<usize>> {
    if profile.is_empty() {
        return Err(PixelSnapperError::ProcessingError(
            "Cannot walk on empty profile".to_string(),
        ));
    }

    let mut cuts = vec![0];
    let mut current_pos = 0.0;
    let search_window =
        (step_size * config.walker_search_window_ratio).max(config.walker_min_search_window);
    let mean_val: f64 = profile.iter().sum::<f64>() / profile.len() as f64;

    while current_pos < limit as f64 {
        let target = current_pos + step_size;
        if target >= limit as f64 {
            cuts.push(limit);
            break;
        }

        let start_search = ((target - search_window) as usize).max((current_pos + 1.0) as usize);
        let end_search = ((target + search_window) as usize).min(limit);

        if end_search <= start_search {
            current_pos = target;
            continue;
        }

        let mut max_val = -1.0;
        let mut max_idx = start_search;
        for i in start_search..end_search {
            if profile[i] > max_val {
                max_val = profile[i];
                max_idx = i;
            }
        }

        if max_val > mean_val * config.walker_strength_threshold {
            cuts.push(max_idx);
            current_pos = max_idx as f64;
        } else {
            cuts.push(target as usize);
            current_pos = target;
        }
    }
    Ok(cuts)
}

fn stabilize_cuts(
    profile: &[f64],
    cuts: Vec<usize>,
    limit: usize,
    sibling_cuts: &[usize],
    sibling_limit: usize,
    config: &Config,
) -> Vec<usize> {
    if limit == 0 {
        return vec![0];
    }

    let cuts = sanitize_cuts(cuts, limit);
    let min_required = config.min_cuts_per_axis.max(2).min(limit.saturating_add(1));
    let axis_cells = cuts.len().saturating_sub(1);
    let sibling_cells = sibling_cuts.len().saturating_sub(1);
    let sibling_has_grid =
        sibling_limit > 0 && sibling_cells >= min_required.saturating_sub(1) && sibling_cells > 0;
    let steps_skewed = sibling_has_grid && axis_cells > 0 && {
        let axis_step = limit as f64 / axis_cells as f64;
        let sibling_step = sibling_limit as f64 / sibling_cells as f64;
        let step_ratio = axis_step / sibling_step;
        step_ratio > config.max_step_ratio || step_ratio < 1.0 / config.max_step_ratio
    };
    let has_enough = cuts.len() >= min_required;

    if has_enough && !steps_skewed {
        return cuts;
    }

    let mut target_step = if sibling_has_grid {
        sibling_limit as f64 / sibling_cells as f64
    } else if config.fallback_target_segments > 1 {
        limit as f64 / config.fallback_target_segments as f64
    } else if axis_cells > 0 {
        limit as f64 / axis_cells as f64
    } else {
        limit as f64
    };
    if !target_step.is_finite() || target_step <= 0.0 {
        target_step = 1.0;
    }

    snap_uniform_cuts(profile, limit, target_step, config, min_required)
}

fn sanitize_cuts(mut cuts: Vec<usize>, limit: usize) -> Vec<usize> {
    if limit == 0 {
        return vec![0];
    }

    let mut has_zero = false;
    let mut has_limit = false;

    for value in cuts.iter_mut() {
        if *value == 0 {
            has_zero = true;
        }
        if *value >= limit {
            *value = limit;
        }
        if *value == limit {
            has_limit = true;
        }
    }

    if !has_zero {
        cuts.push(0);
    }
    if !has_limit {
        cuts.push(limit);
    }

    cuts.sort_unstable();
    cuts.dedup();
    cuts
}

fn snap_uniform_cuts(
    profile: &[f64],
    limit: usize,
    target_step: f64,
    config: &Config,
    min_required: usize,
) -> Vec<usize> {
    if limit == 0 {
        return vec![0];
    }
    if limit == 1 {
        return vec![0, 1];
    }

    // Get desired cells
    let mut desired_cells = if target_step.is_finite() && target_step > 0.0 {
        (limit as f64 / target_step).round() as usize
    } else {
        0
    };
    desired_cells = desired_cells
        .max(min_required.saturating_sub(1))
        .max(1)
        .min(limit);

    let cell_width = limit as f64 / desired_cells as f64;
    let search_window =
        (cell_width * config.walker_search_window_ratio).max(config.walker_min_search_window);
    let mean_val = if profile.is_empty() {
        0.0
    } else {
        profile.iter().sum::<f64>() / profile.len() as f64
    };

    let mut cuts = Vec::with_capacity(desired_cells + 1);
    cuts.push(0);
    for idx in 1..desired_cells {
        let target = cell_width * idx as f64;
        let prev = *cuts.last().unwrap();
        if prev + 1 >= limit {
            break;
        }
        let mut start = ((target - search_window).floor() as isize)
            .max(prev as isize + 1)
            .max(0);
        let mut end = ((target + search_window).ceil() as isize).min(limit as isize - 1);
        if end < start {
            start = prev as isize + 1;
            end = start;
        }
        let start = start as usize;
        let end = end as usize;
        let mut best_idx = start.min(profile.len().saturating_sub(1));
        let mut best_val = -1.0;
        for i in start..=end.min(profile.len().saturating_sub(1)) {
            let v = profile.get(i).copied().unwrap_or(0.0);
            if v > best_val {
                best_val = v;
                best_idx = i;
            }
        }
        let strength_threshold = mean_val * config.walker_strength_threshold;
        if best_val < strength_threshold {
            let mut fallback_idx = target.round() as isize;
            if fallback_idx <= prev as isize {
                fallback_idx = prev as isize + 1;
            }
            if fallback_idx >= limit as isize {
                fallback_idx = (limit as isize - 1).max(prev as isize + 1);
            }
            best_idx = fallback_idx as usize;
        }
        cuts.push(best_idx);
    }
    if *cuts.last().unwrap() != limit {
        cuts.push(limit);
    }
    cuts = sanitize_cuts(cuts, limit);
    cuts
}

fn resample_mode(img: &RgbaImage, cols: &[usize], rows: &[usize]) -> Result<RgbaImage> {
    if cols.len() < 2 || rows.len() < 2 {
        return Err(PixelSnapperError::ProcessingError(
            "Insufficient grid cuts for resampling".to_string(),
        ));
    }

    let out_w = (cols.len().max(1) - 1) as u32;
    let out_h = (rows.len().max(1) - 1) as u32;
    let mut final_img: RgbaImage = ImageBuffer::new(out_w, out_h);

    for (y_i, w_y) in rows.windows(2).enumerate() {
        for (x_i, w_x) in cols.windows(2).enumerate() {
            let ys = w_y[0];
            let ye = w_y[1];
            let xs = w_x[0];
            let xe = w_x[1];

            if xe <= xs || ye <= ys {
                continue;
            }

            let mut counts: HashMap<[u8; 4], usize> = HashMap::new();

            for y in ys..ye {
                for x in xs..xe {
                    if x < img.width() as usize && y < img.height() as usize {
                        let p = img.get_pixel(x as u32, y as u32).0;
                        *counts.entry(p).or_insert(0) += 1;
                    }
                }
            }

            let mut best_pixel = [0, 0, 0, 0];

            let mut candidates: Vec<([u8; 4], usize)> = counts.into_iter().collect();
            candidates.sort_by(|a, b| {
                let count_cmp = b.1.cmp(&a.1);
                if count_cmp == Ordering::Equal {
                    a.0.cmp(&b.0)
                } else {
                    count_cmp
                }
            });

            if let Some(winner) = candidates.first() {
                best_pixel = winner.0;
            }

            final_img.put_pixel(x_i as u32, y_i as u32, Rgba(best_pixel));
        }
    }
    Ok(final_img)
}

fn prefilter_box3_alpha_aware(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let mut out = RgbaImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            let mut sum = [0u32; 4];
            let mut n = 0u32;

            let y0 = y.saturating_sub(1);
            let y1 = (y + 1).min(h.saturating_sub(1));
            let x0 = x.saturating_sub(1);
            let x1 = (x + 1).min(w.saturating_sub(1));

            for yy in y0..=y1 {
                for xx in x0..=x1 {
                    let p = img.get_pixel(xx, yy).0;
                    if p[3] == 0 {
                        continue;
                    }
                    sum[0] += p[0] as u32;
                    sum[1] += p[1] as u32;
                    sum[2] += p[2] as u32;
                    sum[3] += p[3] as u32;
                    n += 1;
                }
            }

            if n == 0 {
                out.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            } else {
                out.put_pixel(
                    x,
                    y,
                    Rgba([
                        (sum[0] / n) as u8,
                        (sum[1] / n) as u8,
                        (sum[2] / n) as u8,
                        (sum[3] / n) as u8,
                    ]),
                );
            }
        }
    }

    out
}

fn kmeans_palette(colors: &[[f32; 3]], k_colors: usize, seed: u64, max_iters: usize) -> Vec<[f32; 3]> {
    if colors.is_empty() {
        return vec![[0.0, 0.0, 0.0]];
    }

    let k = k_colors.min(colors.len()).max(1);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    fn sample_index(rng: &mut ChaCha8Rng, upper: usize) -> usize {
        let upper = upper as u64;
        rng.gen_range(0..upper) as usize
    }

    fn dist_sq(p: &[f32; 3], c: &[f32; 3]) -> f32 {
        let dr = p[0] - c[0];
        let dg = p[1] - c[1];
        let db = p[2] - c[2];
        dr * dr + dg * dg + db * db
    }

    let mut centroids: Vec<[f32; 3]> = Vec::with_capacity(k);
    let first_idx = sample_index(&mut rng, colors.len());
    centroids.push(colors[first_idx]);
    let mut distances = vec![f32::MAX; colors.len()];

    for _ in 1..k {
        let last_c = centroids.last().unwrap();
        let mut sum_sq_dist = 0.0;

        for (i, p) in colors.iter().enumerate() {
            let d_sq = dist_sq(p, last_c);
            if d_sq < distances[i] {
                distances[i] = d_sq;
            }
            sum_sq_dist += distances[i];
        }

        if sum_sq_dist <= 0.0 {
            let idx = sample_index(&mut rng, colors.len());
            centroids.push(colors[idx]);
        } else {
            match WeightedIndex::new(&distances) {
                Ok(dist) => {
                    let idx = dist.sample(&mut rng);
                    centroids.push(colors[idx]);
                }
                Err(_) => {
                    let idx = sample_index(&mut rng, colors.len());
                    centroids.push(colors[idx]);
                }
            }
        }
    }

    let mut prev_centroids = centroids.clone();
    for iteration in 0..max_iters {
        let mut sums = vec![[0.0f32; 3]; k];
        let mut counts = vec![0usize; k];

        for p in colors {
            let mut min_dist = f32::MAX;
            let mut best_k = 0;

            for (i, c) in centroids.iter().enumerate() {
                let d = dist_sq(p, c);
                if d < min_dist {
                    min_dist = d;
                    best_k = i;
                }
            }
            sums[best_k][0] += p[0];
            sums[best_k][1] += p[1];
            sums[best_k][2] += p[2];
            counts[best_k] += 1;
        }

        for i in 0..k {
            if counts[i] > 0 {
                let fcount = counts[i] as f32;
                centroids[i] = [
                    sums[i][0] / fcount,
                    sums[i][1] / fcount,
                    sums[i][2] / fcount,
                ];
            }
        }

        if iteration > 0 {
            let mut max_movement = 0.0f32;
            for (new_c, old_c) in centroids.iter().zip(prev_centroids.iter()) {
                let movement = dist_sq(new_c, old_c);
                if movement > max_movement {
                    max_movement = movement;
                }
            }

            if max_movement < 0.01 {
                break;
            }
        }

        prev_centroids.copy_from_slice(&centroids);
    }

    centroids
}

fn resample_cells(
    img: &RgbaImage,
    cols: &[usize],
    rows: &[usize],
    config: &Config,
    locked_palette: Option<&LockedPalette>,
) -> Result<RgbaImage> {
    if cols.len() < 2 || rows.len() < 2 {
        return Err(PixelSnapperError::ProcessingError(
            "Insufficient grid cuts for resampling".to_string(),
        ));
    }

    let out_w = (cols.len().max(1) - 1) as usize;
    let out_h = (rows.len().max(1) - 1) as usize;

    let mut cell_colors = vec![[0.0f32; 3]; out_w * out_h];
    let mut cell_has = vec![false; out_w * out_h];
    let mut cell_alpha = vec![0u8; out_w * out_h];

    for (y_i, w_y) in rows.windows(2).enumerate() {
        for (x_i, w_x) in cols.windows(2).enumerate() {
            let ys = w_y[0];
            let ye = w_y[1];
            let xs = w_x[0];
            let xe = w_x[1];

            if xe <= xs || ye <= ys {
                continue;
            }

            let mut sum_lin = [0.0f32; 3];
            let mut sum_a = 0.0f32;
            let mut max_a = 0u8;
            let mut rgba_counts: HashMap<[u8; 4], usize> = HashMap::new();

            for y in ys..ye {
                for x in xs..xe {
                    if x < img.width() as usize && y < img.height() as usize {
                        let p = img.get_pixel(x as u32, y as u32).0;
                        if p[3] == 0 {
                            continue;
                        }
                        let a = p[3] as f32 / 255.0;
                        sum_lin[0] += srgb_u8_to_linear_255(p[0]) * a;
                        sum_lin[1] += srgb_u8_to_linear_255(p[1]) * a;
                        sum_lin[2] += srgb_u8_to_linear_255(p[2]) * a;
                        sum_a += a;
                        max_a = max_a.max(p[3]);
                        *rgba_counts.entry(p).or_insert(0) += 1;
                    }
                }
            }

            let idx = y_i * out_w + x_i;
            if sum_a > 0.0 {
                let mean_lin = [sum_lin[0] / sum_a, sum_lin[1] / sum_a, sum_lin[2] / sum_a];
                cell_colors[idx] =
                    choose_cell_color_space(&rgba_counts, mean_lin, config.color_space, config.cell_color_mode);
                cell_has[idx] = true;
                cell_alpha[idx] = max_a;
            }
        }
    }

    let palette_input: Vec<[f32; 3]> = cell_colors
        .iter()
        .zip(cell_has.iter())
        .filter_map(|(c, has)| if *has { Some(*c) } else { None })
        .collect();

    let palette = if let Some(locked_palette) = locked_palette {
        locked_palette.colors_space.clone()
    } else {
        kmeans_palette(
            &palette_input,
            config.k_colors.max(1),
            config.k_seed,
            config.max_kmeans_iterations,
        )
    };

    fn dist_sq(p: &[f32; 3], c: &[f32; 3]) -> f32 {
        let dr = p[0] - c[0];
        let dg = p[1] - c[1];
        let db = p[2] - c[2];
        dr * dr + dg * dg + db * db
    }

    let mut out_space = vec![[0.0f32; 3]; out_w * out_h];
    if config.dither_mode == 1 {
        let mut work = cell_colors.clone();
        for y in 0..out_h {
            for x in 0..out_w {
                let idx = y * out_w + x;
                if !cell_has[idx] {
                    continue;
                }
                let p = work[idx];
                let mut best = palette[0];
                let mut best_d = f32::MAX;
                for c in &palette {
                    let d = dist_sq(&p, c);
                    if d < best_d {
                        best_d = d;
                        best = *c;
                    }
                }
                out_space[idx] = best;
                let err = [p[0] - best[0], p[1] - best[1], p[2] - best[2]];

                let add = |work: &mut [[f32; 3]], xi: isize, yi: isize, e: [f32; 3], w: f32, out_w: usize, out_h: usize| {
                    if xi < 0 || yi < 0 {
                        return;
                    }
                    let xi = xi as usize;
                    let yi = yi as usize;
                    if xi >= out_w || yi >= out_h {
                        return;
                    }
                    let j = yi * out_w + xi;
                    if !cell_has[j] {
                        return;
                    }
                    work[j][0] = (work[j][0] + e[0] * w).clamp(0.0, 255.0);
                    work[j][1] = (work[j][1] + e[1] * w).clamp(0.0, 255.0);
                    work[j][2] = (work[j][2] + e[2] * w).clamp(0.0, 255.0);
                };

                add(&mut work, x as isize + 1, y as isize, err, 7.0 / 16.0, out_w, out_h);
                add(&mut work, x as isize - 1, y as isize + 1, err, 3.0 / 16.0, out_w, out_h);
                add(&mut work, x as isize, y as isize + 1, err, 5.0 / 16.0, out_w, out_h);
                add(&mut work, x as isize + 1, y as isize + 1, err, 1.0 / 16.0, out_w, out_h);
            }
        }
    } else {
        for i in 0..cell_colors.len() {
            if !cell_has[i] {
                continue;
            }
            let p = cell_colors[i];
            let mut best = palette[0];
            let mut best_d = f32::MAX;
            for c in &palette {
                let d = dist_sq(&p, c);
                if d < best_d {
                    best_d = d;
                    best = *c;
                }
            }
            out_space[i] = best;
        }
    }

    let mut out_img: RgbaImage = ImageBuffer::new(out_w as u32, out_h as u32);
    for y in 0..out_h {
        for x in 0..out_w {
            let idx = y * out_w + x;
            if !cell_has[idx] {
                out_img.put_pixel(x as u32, y as u32, Rgba([0, 0, 0, 0]));
                continue;
            }
            let rgb = space_255_to_rgb(out_space[idx], config.color_space);
            out_img.put_pixel(x as u32, y as u32, Rgba([rgb[0], rgb[1], rgb[2], cell_alpha[idx]]));
        }
    }

    Ok(out_img)
}

fn choose_cell_color_space(
    rgba_counts: &HashMap<[u8; 4], usize>,
    mean_lin: [f32; 3],
    color_space: u32,
    cell_color_mode: u32,
) -> [f32; 3] {
    if rgba_counts.is_empty() {
        return match color_space {
            1 => mean_lin,
            _ => [
                linear_255_to_srgb_u8(mean_lin[0]) as f32,
                linear_255_to_srgb_u8(mean_lin[1]) as f32,
                linear_255_to_srgb_u8(mean_lin[2]) as f32,
            ],
        };
    }

    match cell_color_mode {
        2 => choose_cell_medoid(rgba_counts, color_space),
        1 => choose_cell_dominant(rgba_counts, color_space),
        _ => match color_space {
            1 => mean_lin,
            _ => [
                linear_255_to_srgb_u8(mean_lin[0]) as f32,
                linear_255_to_srgb_u8(mean_lin[1]) as f32,
                linear_255_to_srgb_u8(mean_lin[2]) as f32,
            ],
        },
    }
}

fn choose_cell_dominant(rgba_counts: &HashMap<[u8; 4], usize>, color_space: u32) -> [f32; 3] {
    let rgba = rgba_counts
        .iter()
        .max_by(|(a_rgba, a_count), (b_rgba, b_count)| {
            a_count
                .cmp(b_count)
                .then_with(|| a_rgba[3].cmp(&b_rgba[3]))
                .then_with(|| a_rgba.cmp(b_rgba))
        })
        .map(|(rgba, _)| *rgba)
        .unwrap_or([0, 0, 0, 0]);
    rgba_to_space(rgba, color_space)
}

fn choose_cell_medoid(rgba_counts: &HashMap<[u8; 4], usize>, color_space: u32) -> [f32; 3] {
    let mut best_rgba = [0u8, 0u8, 0u8, 0u8];
    let mut best_score = f32::MAX;
    let mut best_count = 0usize;

    for (candidate_rgba, candidate_count) in rgba_counts {
        let candidate = rgba_to_space(*candidate_rgba, color_space);
        let mut score = 0.0f32;
        for (other_rgba, other_count) in rgba_counts {
            let other = rgba_to_space(*other_rgba, color_space);
            let dr = candidate[0] - other[0];
            let dg = candidate[1] - other[1];
            let db = candidate[2] - other[2];
            score += (dr * dr + dg * dg + db * db) * *other_count as f32;
        }

        if score < best_score
            || (score == best_score
                && (*candidate_count > best_count
                    || (*candidate_count == best_count && *candidate_rgba > best_rgba)))
        {
            best_score = score;
            best_count = *candidate_count;
            best_rgba = *candidate_rgba;
        }
    }

    rgba_to_space(best_rgba, color_space)
}

fn rgba_to_space(rgba: [u8; 4], color_space: u32) -> [f32; 3] {
    match color_space {
        1 => [
            srgb_u8_to_linear_255(rgba[0]),
            srgb_u8_to_linear_255(rgba[1]),
            srgb_u8_to_linear_255(rgba[2]),
        ],
        _ => [rgba[0] as f32, rgba[1] as f32, rgba[2] as f32],
    }
}

fn apply_palette_cleanup_mode(img: RgbaImage, config: &Config) -> RgbaImage {
    match config.palette_cleanup_mode {
        2 => remap_palette_near_duplicates(&img, 32.0),
        1 => remap_palette_near_duplicates(&img, 18.0),
        _ => img,
    }
}

fn apply_cleanup_mode(img: RgbaImage, config: &Config) -> RgbaImage {
    match config.cleanup_mode {
        1 => cleanup_pixel_art_basic(&img, 2),
        _ => img,
    }
}

fn apply_repair_mode(img: RgbaImage, config: &Config) -> RgbaImage {
    match config.repair_mode {
        3 => repair_pixel_art_ultra(&img),
        2 => repair_pixel_art_smart(&img, 2),
        1 => repair_outline_bridges(&img, 2),
        _ => img,
    }
}

fn remap_palette_near_duplicates(img: &RgbaImage, threshold: f32) -> RgbaImage {
    let mut counts: HashMap<[u8; 4], usize> = HashMap::new();
    for pixel in img.pixels() {
        let rgba = pixel.0;
        if rgba[3] == 0 {
            continue;
        }
        *counts.entry(rgba).or_insert(0) += 1;
    }

    if counts.len() <= 1 {
        return img.clone();
    }

    let mut colors: Vec<([u8; 4], usize)> = counts.into_iter().collect();
    colors.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let threshold_sq = threshold * threshold;
    let mut canonical_colors: Vec<[u8; 4]> = Vec::new();
    let mut remap: HashMap<[u8; 4], [u8; 4]> = HashMap::new();

    for (color, _) in colors {
        let mut mapped = None;
        for canonical in &canonical_colors {
            if canonical[3] != color[3] {
                continue;
            }
            if color_distance_sq(color, *canonical) <= threshold_sq {
                mapped = Some(*canonical);
                break;
            }
        }

        let target = mapped.unwrap_or_else(|| {
            canonical_colors.push(color);
            color
        });
        remap.insert(color, target);
    }

    let mut out = img.clone();
    for pixel in out.pixels_mut() {
        if pixel.0[3] == 0 {
            continue;
        }
        if let Some(mapped) = remap.get(&pixel.0) {
            pixel.0 = *mapped;
        }
    }
    out
}

fn cleanup_pixel_art_basic(img: &RgbaImage, passes: usize) -> RgbaImage {
    let mut current = img.clone();
    for _ in 0..passes {
        let mut next = current.clone();
        let width = current.width();
        let height = current.height();

        for y in 0..height {
            for x in 0..width {
                let current_px = current.get_pixel(x, y).0;
                let mut counts: HashMap<[u8; 4], usize> = HashMap::new();
                let mut opaque_neighbors = 0usize;

                for (nx, ny) in pixel_neighbors4(x, y, width, height) {
                    let neighbor = current.get_pixel(nx, ny).0;
                    if neighbor[3] == 0 {
                        continue;
                    }
                    opaque_neighbors += 1;
                    *counts.entry(neighbor).or_insert(0) += 1;
                }

                if counts.is_empty() {
                    continue;
                }

                let best_neighbor = counts
                    .iter()
                    .max_by(|(a_rgba, a_count), (b_rgba, b_count)| {
                        a_count
                            .cmp(b_count)
                            .then_with(|| a_rgba[3].cmp(&b_rgba[3]))
                            .then_with(|| a_rgba.cmp(b_rgba))
                    })
                    .map(|(rgba, count)| (*rgba, *count));

                let Some((replacement, replacement_count)) = best_neighbor else {
                    continue;
                };

                let current_same_neighbors = counts.get(&current_px).copied().unwrap_or(0);
                let should_replace = if current_px[3] == 0 {
                    replacement_count >= 3
                } else {
                    opaque_neighbors >= 3 && replacement_count >= 3 && current_same_neighbors == 0
                };

                if should_replace {
                    next.put_pixel(x, y, Rgba(replacement));
                }
            }
        }

        current = next;
    }
    current
}

fn repair_outline_bridges(img: &RgbaImage, passes: usize) -> RgbaImage {
    let mut current = img.clone();
    for _ in 0..passes {
        let mut next = current.clone();
        let width = current.width();
        let height = current.height();

        for y in 0..height {
            for x in 0..width {
                let current_px = current.get_pixel(x, y).0;

                let horizontal = if x > 0 && x + 1 < width {
                    let left = current.get_pixel(x - 1, y).0;
                    let right = current.get_pixel(x + 1, y).0;
                    if left[3] > 0 && left == right && left != current_px {
                        Some(left)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let vertical = if y > 0 && y + 1 < height {
                    let up = current.get_pixel(x, y - 1).0;
                    let down = current.get_pixel(x, y + 1).0;
                    if up[3] > 0 && up == down && up != current_px {
                        Some(up)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let replacement = match (horizontal, vertical) {
                    (Some(h), Some(v)) if h == v => Some(h),
                    (Some(h), None) => Some(h),
                    (None, Some(v)) => Some(v),
                    _ => None,
                };

                if let Some(replacement) = replacement {
                    next.put_pixel(x, y, Rgba(replacement));
                }
            }
        }

        current = next;
    }
    current
}

fn repair_pixel_art_smart(img: &RgbaImage, passes: usize) -> RgbaImage {
    let mut current = img.clone();
    for _ in 0..passes {
        current = repair_outline_bridges(&current, 1);
        current = repair_outline_gaps(&current, 1);
        current = cleanup_tiny_islands(&current, 2);
        current = consolidate_diagonals(&current, 1);
    }
    current
}

fn repair_pixel_art_ultra(img: &RgbaImage) -> RgbaImage {
    let mut current = img.clone();
    for _ in 0..3 {
        current = repair_outline_bridges(&current, 1);
        current = repair_outline_gaps(&current, 2);
        current = cleanup_tiny_islands(&current, 3);
        current = cleanup_tiny_islands(&current, 6);
        current = consolidate_diagonals(&current, 2);
    }
    current
}

fn repair_outline_gaps(img: &RgbaImage, passes: usize) -> RgbaImage {
    let mut current = img.clone();
    for _ in 0..passes {
        let mut next = current.clone();
        let width = current.width();
        let height = current.height();

        for y in 0..height {
            for x in 0..width {
                let current_px = current.get_pixel(x, y).0;
                let mut counts: HashMap<[u8; 4], usize> = HashMap::new();
                for (nx, ny) in pixel_neighbors8(x, y, width, height) {
                    let neighbor = current.get_pixel(nx, ny).0;
                    if neighbor[3] == 0 {
                        continue;
                    }
                    *counts.entry(neighbor).or_insert(0) += 1;
                }

                let Some((best_color, best_count)) = counts
                    .iter()
                    .max_by(|(a_rgba, a_count), (b_rgba, b_count)| {
                        a_count
                            .cmp(b_count)
                            .then_with(|| a_rgba[3].cmp(&b_rgba[3]))
                            .then_with(|| a_rgba.cmp(b_rgba))
                    })
                    .map(|(rgba, count)| (*rgba, *count))
                else {
                    continue;
                };

                if best_color == current_px {
                    continue;
                }

                let left = get_pixel_checked(&current, x as i32 - 1, y as i32);
                let right = get_pixel_checked(&current, x as i32 + 1, y as i32);
                let up = get_pixel_checked(&current, x as i32, y as i32 - 1);
                let down = get_pixel_checked(&current, x as i32, y as i32 + 1);
                let nw = get_pixel_checked(&current, x as i32 - 1, y as i32 - 1);
                let ne = get_pixel_checked(&current, x as i32 + 1, y as i32 - 1);
                let sw = get_pixel_checked(&current, x as i32 - 1, y as i32 + 1);
                let se = get_pixel_checked(&current, x as i32 + 1, y as i32 + 1);

                let horizontal_bridge = left == Some(best_color) && right == Some(best_color);
                let vertical_bridge = up == Some(best_color) && down == Some(best_color);
                let diagonal_bridge_a = nw == Some(best_color)
                    && se == Some(best_color)
                    && (up == Some(best_color)
                        || down == Some(best_color)
                        || left == Some(best_color)
                        || right == Some(best_color));
                let diagonal_bridge_b = ne == Some(best_color)
                    && sw == Some(best_color)
                    && (up == Some(best_color)
                        || down == Some(best_color)
                        || left == Some(best_color)
                        || right == Some(best_color));
                let enclosed_hole = current_px[3] == 0 && best_count >= 5;

                if horizontal_bridge
                    || vertical_bridge
                    || diagonal_bridge_a
                    || diagonal_bridge_b
                    || enclosed_hole
                {
                    next.put_pixel(x, y, Rgba(best_color));
                }
            }
        }

        current = next;
    }
    current
}

fn cleanup_tiny_islands(img: &RgbaImage, max_component_size: usize) -> RgbaImage {
    let width = img.width() as usize;
    let height = img.height() as usize;
    if width == 0 || height == 0 {
        return img.clone();
    }

    let mut out = img.clone();
    let mut visited = vec![false; width * height];

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            if visited[idx] {
                continue;
            }

            let target = img.get_pixel(x as u32, y as u32).0;
            let mut queue = VecDeque::new();
            let mut component = Vec::new();
            let mut boundary_counts: HashMap<[u8; 4], usize> = HashMap::new();

            visited[idx] = true;
            queue.push_back((x as u32, y as u32));

            while let Some((cx, cy)) = queue.pop_front() {
                component.push((cx, cy));
                for (nx, ny) in pixel_neighbors4(cx, cy, img.width(), img.height()) {
                    let nidx = ny as usize * width + nx as usize;
                    let neighbor = img.get_pixel(nx, ny).0;
                    if neighbor == target {
                        if !visited[nidx] {
                            visited[nidx] = true;
                            queue.push_back((nx, ny));
                        }
                    } else if neighbor[3] > 0 {
                        *boundary_counts.entry(neighbor).or_insert(0) += 1;
                    }
                }
            }

            if component.len() > max_component_size || boundary_counts.is_empty() {
                continue;
            }

            let Some((replacement, boundary_support)) = boundary_counts
                .iter()
                .max_by(|(a_rgba, a_count), (b_rgba, b_count)| {
                    a_count
                        .cmp(b_count)
                        .then_with(|| a_rgba[3].cmp(&b_rgba[3]))
                        .then_with(|| a_rgba.cmp(b_rgba))
                })
                .map(|(rgba, count)| (*rgba, *count))
            else {
                continue;
            };

            let min_support = if target[3] == 0 {
                component.len().saturating_mul(2)
            } else {
                component.len().saturating_add(1)
            };
            if boundary_support < min_support {
                continue;
            }

            for (cx, cy) in component {
                out.put_pixel(cx, cy, Rgba(replacement));
            }
        }
    }

    out
}

fn consolidate_diagonals(img: &RgbaImage, passes: usize) -> RgbaImage {
    let mut current = img.clone();
    for _ in 0..passes {
        let mut next = current.clone();
        let width = current.width();
        let height = current.height();
        if width < 2 || height < 2 {
            return current;
        }

        for y in 0..height - 1 {
            for x in 0..width - 1 {
                let tl = current.get_pixel(x, y).0;
                let tr = current.get_pixel(x + 1, y).0;
                let bl = current.get_pixel(x, y + 1).0;
                let br = current.get_pixel(x + 1, y + 1).0;

                let checker_a = tl[3] > 0 && tl == br && tr[3] > 0 && tr == bl && tl != tr;
                if !checker_a {
                    continue;
                }

                let a_support_tr = color_support_8(&current, x + 1, y, tl);
                let a_support_bl = color_support_8(&current, x, y + 1, tl);
                let b_support_tl = color_support_8(&current, x, y, tr);
                let b_support_br = color_support_8(&current, x + 1, y + 1, tr);

                let best_a = a_support_tr.max(a_support_bl);
                let best_b = b_support_tl.max(b_support_br);

                if best_a >= best_b && best_a >= 3 {
                    if a_support_tr >= a_support_bl {
                        next.put_pixel(x + 1, y, Rgba(tl));
                    } else {
                        next.put_pixel(x, y + 1, Rgba(tl));
                    }
                } else if best_b > best_a && best_b >= 3 {
                    if b_support_tl >= b_support_br {
                        next.put_pixel(x, y, Rgba(tr));
                    } else {
                        next.put_pixel(x + 1, y + 1, Rgba(tr));
                    }
                }
            }
        }

        current = next;
    }
    current
}

fn pixel_neighbors4(x: u32, y: u32, width: u32, height: u32) -> Vec<(u32, u32)> {
    let mut neighbors = Vec::with_capacity(4);
    if x > 0 {
        neighbors.push((x - 1, y));
    }
    if x + 1 < width {
        neighbors.push((x + 1, y));
    }
    if y > 0 {
        neighbors.push((x, y - 1));
    }
    if y + 1 < height {
        neighbors.push((x, y + 1));
    }
    neighbors
}

fn pixel_neighbors8(x: u32, y: u32, width: u32, height: u32) -> Vec<(u32, u32)> {
    let mut neighbors = Vec::with_capacity(8);
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            neighbors.push((nx as u32, ny as u32));
        }
    }
    neighbors
}

fn get_pixel_checked(img: &RgbaImage, x: i32, y: i32) -> Option<[u8; 4]> {
    if x < 0 || y < 0 || x >= img.width() as i32 || y >= img.height() as i32 {
        None
    } else {
        Some(img.get_pixel(x as u32, y as u32).0)
    }
}

fn color_support_8(img: &RgbaImage, x: u32, y: u32, color: [u8; 4]) -> usize {
    pixel_neighbors8(x, y, img.width(), img.height())
        .into_iter()
        .filter(|(nx, ny)| img.get_pixel(*nx, *ny).0 == color)
        .count()
}

fn color_distance_sq(a: [u8; 4], b: [u8; 4]) -> f32 {
    let dr = a[0] as f32 - b[0] as f32;
    let dg = a[1] as f32 - b[1] as f32;
    let db = a[2] as f32 - b[2] as f32;
    dr * dr + dg * dg + db * db
}
