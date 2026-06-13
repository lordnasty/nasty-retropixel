use nasty_retropixel::{
    apply_named_preset, process_batch_with_reporter, process_file_with_debug_exports,
    process_image_debug_with_config, process_image_debug_with_palette_image_config,
    recommend_variant_for_image_bytes, suggest_setup_for_image_bytes, BatchConfig, BatchEvent,
    Config, DebugExportOptions, DebugReport,
};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
struct BatchSummaryRow {
    rank: usize,
    source_relative_path: String,
    relative_path: String,
    output_relative_path: String,
    review_priority: String,
    recommended_profile: String,
    recommended_actions: String,
    recommendation_reason: String,
    k_colors: usize,
    pixel_size_override: Option<f64>,
    quality_overall: f64,
    diff_score: f64,
    diff_area: f64,
    grid_regularity: f64,
    palette_compactness: f64,
    coverage_ratio: f64,
    palette_count: usize,
    output_width: u32,
    output_height: u32,
    step_x: f64,
    step_y: f64,
    prefilter_mode: String,
    palette_source: String,
    palette_cleanup_mode: String,
    cell_color_mode: String,
    dither_mode: String,
    color_space: String,
    cleanup_mode: String,
    repair_mode: String,
}

#[derive(Debug)]
struct BatchSummaryArtifacts {
    json_path: PathBuf,
    csv_path: PathBuf,
    rows: Vec<BatchSummaryRow>,
}

#[derive(Debug, Clone, Serialize)]
struct RetryAlgoOverrides {
    denoise: u32,
    palette_source: u32,
    palette_cleanup_mode: u32,
    cell_color_mode: u32,
    dither_mode: u32,
    color_space: u32,
    cleanup_mode: u32,
    repair_mode: u32,
}

#[derive(Debug, Clone, Serialize)]
struct RetryCandidatePlan {
    key: String,
    label: String,
    reason: String,
    algo_overrides: RetryAlgoOverrides,
}

#[derive(Debug, Clone, Serialize)]
struct RetryCandidateEvaluation {
    key: String,
    label: String,
    reason: String,
    algo_overrides: RetryAlgoOverrides,
    quality_overall: Option<f64>,
    diff_score: Option<f64>,
    diff_area: Option<f64>,
    delta_quality: Option<f64>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct RetryReviewRow {
    relative_path: String,
    source_relative_path: String,
    retry_status: String,
    baseline_quality: f64,
    recommended_profile: String,
    recommended_actions: String,
    recommendation_reason: String,
    best_retry_candidate: Option<RetryCandidateEvaluation>,
    selected_candidate: Option<RetryCandidateEvaluation>,
    exported_output_relative_path: Option<String>,
    candidates: Vec<RetryCandidateEvaluation>,
}

#[derive(Debug, Clone)]
struct RetrySelection {
    evaluation: RetryCandidateEvaluation,
    output_bytes: Vec<u8>,
    overlay_bytes: Vec<u8>,
    heatmap_bytes: Vec<u8>,
    report: DebugReport,
}

fn build_retry_plan_candidates(row: &BatchSummaryRow) -> Vec<serde_json::Value> {
    let mut plans = Vec::new();
    plans.push(serde_json::json!({
        "key": "recommended-profile",
        "label": format!("Profilo {}", row.recommended_profile),
        "reason": row.recommendation_reason,
        "actions": row.recommended_actions,
    }));
    plans.push(serde_json::json!({
        "key": "stability-pass",
        "label": "Stability Pass",
        "reason": "rinforza griglia, palette dalle celle e repair medio",
        "actions": "attiva denoise box3 | usa palette dalle celle | attiva/alza repair smart",
    }));
    plans.push(serde_json::json!({
        "key": "recovery-pass",
        "label": "Recovery Pass",
        "reason": "pass aggressivo per casi molto degradati",
        "actions": "attiva denoise box3 | palette cleanup strict | prova repair ultra",
    }));
    plans
}

fn mode_prefilter(value: &str) -> u32 {
    match value {
        "box3" => 1,
        _ => 0,
    }
}

fn mode_palette_source(value: &str) -> u32 {
    match value {
        "pixels" => 0,
        _ => 1,
    }
}

fn mode_palette_cleanup(value: &str) -> u32 {
    match value {
        "off" => 0,
        "strict" => 2,
        _ => 1,
    }
}

fn mode_cell_color(value: &str) -> u32 {
    match value {
        "mean" => 0,
        "medoid" => 2,
        _ => 1,
    }
}

fn mode_dither(value: &str) -> u32 {
    match value {
        "fs" => 1,
        _ => 0,
    }
}

fn mode_color_space(value: &str) -> u32 {
    match value {
        "srgb" => 0,
        _ => 1,
    }
}

fn mode_cleanup(value: &str) -> u32 {
    match value {
        "off" => 0,
        _ => 1,
    }
}

fn mode_repair(value: &str) -> u32 {
    match value {
        "off" => 0,
        "basic" => 1,
        "ultra" => 3,
        _ => 2,
    }
}

fn config_from_summary_row(row: &BatchSummaryRow) -> Config {
    let mut config = Config::default();
    config.k_colors = row.k_colors.max(1);
    config.pixel_size_override = row.pixel_size_override;
    config.prefilter_mode = mode_prefilter(&row.prefilter_mode);
    config.palette_source = mode_palette_source(&row.palette_source);
    config.palette_cleanup_mode = mode_palette_cleanup(&row.palette_cleanup_mode);
    config.cell_color_mode = mode_cell_color(&row.cell_color_mode);
    config.dither_mode = mode_dither(&row.dither_mode);
    config.color_space = mode_color_space(&row.color_space);
    config.cleanup_mode = mode_cleanup(&row.cleanup_mode);
    config.repair_mode = mode_repair(&row.repair_mode);
    config
}

fn algo_overrides_from_config(config: &Config) -> RetryAlgoOverrides {
    RetryAlgoOverrides {
        denoise: config.prefilter_mode,
        palette_source: config.palette_source,
        palette_cleanup_mode: config.palette_cleanup_mode,
        cell_color_mode: config.cell_color_mode,
        dither_mode: config.dither_mode,
        color_space: config.color_space,
        cleanup_mode: config.cleanup_mode,
        repair_mode: config.repair_mode,
    }
}

fn dedupe_retry_candidate_plans(plans: Vec<RetryCandidatePlan>) -> Vec<RetryCandidatePlan> {
    let mut seen = HashSet::new();
    plans.into_iter()
        .filter(|plan| {
            let key = format!(
                "{}:{}:{}:{}:{}:{}:{}:{}",
                plan.algo_overrides.denoise,
                plan.algo_overrides.palette_source,
                plan.algo_overrides.palette_cleanup_mode,
                plan.algo_overrides.cell_color_mode,
                plan.algo_overrides.dither_mode,
                plan.algo_overrides.color_space,
                plan.algo_overrides.cleanup_mode,
                plan.algo_overrides.repair_mode
            );
            seen.insert(key)
        })
        .collect()
}

fn build_retry_candidate_plans_for_cli(
    row: &BatchSummaryRow,
) -> nasty_retropixel::Result<Vec<RetryCandidatePlan>> {
    let base = config_from_summary_row(row);
    let mut recommended = base.clone();
    let keep_k = recommended.k_colors;
    let keep_px = recommended.pixel_size_override;
    apply_named_preset(&mut recommended, &row.recommended_profile)?;
    recommended.k_colors = keep_k;
    recommended.pixel_size_override = keep_px;

    let palette_count = row.palette_count;
    let base_overrides = algo_overrides_from_config(&base);
    let recommended_overrides = algo_overrides_from_config(&recommended);
    Ok(dedupe_retry_candidate_plans(vec![
        RetryCandidatePlan {
            key: "recommended-profile".to_string(),
            label: format!("Profilo {}", row.recommended_profile),
            reason: row.recommendation_reason.clone(),
            algo_overrides: recommended_overrides,
        },
        RetryCandidatePlan {
            key: "stability-pass".to_string(),
            label: "Stability Pass".to_string(),
            reason: "rinforza griglia, palette dalle celle e repair medio".to_string(),
            algo_overrides: RetryAlgoOverrides {
                denoise: base_overrides.denoise.max(1),
                palette_source: 1,
                palette_cleanup_mode: base_overrides
                    .palette_cleanup_mode
                    .max(if palette_count > 24 { 2 } else { 1 }),
                cell_color_mode: base_overrides.cell_color_mode.max(1),
                dither_mode: base_overrides.dither_mode,
                color_space: base_overrides.color_space,
                cleanup_mode: base_overrides.cleanup_mode.max(1),
                repair_mode: base_overrides.repair_mode.max(2),
            },
        },
        RetryCandidatePlan {
            key: "recovery-pass".to_string(),
            label: "Recovery Pass".to_string(),
            reason: "pass aggressivo per casi molto degradati o con grande area differente"
                .to_string(),
            algo_overrides: RetryAlgoOverrides {
                denoise: 1,
                palette_source: 1,
                palette_cleanup_mode: 2,
                cell_color_mode: 2,
                dither_mode: base_overrides.dither_mode,
                color_space: base_overrides.color_space,
                cleanup_mode: 1,
                repair_mode: 3,
            },
        },
    ]))
}

fn apply_retry_algo_overrides(base: &Config, overrides: &RetryAlgoOverrides) -> Config {
    let mut config = base.clone();
    config.prefilter_mode = overrides.denoise;
    config.palette_source = overrides.palette_source;
    config.palette_cleanup_mode = overrides.palette_cleanup_mode;
    config.cell_color_mode = overrides.cell_color_mode;
    config.dither_mode = overrides.dither_mode;
    config.color_space = overrides.color_space;
    config.cleanup_mode = overrides.cleanup_mode;
    config.repair_mode = overrides.repair_mode;
    config
}

fn summarize_recommendation(row: &BatchSummaryRow) -> (String, String, String, String) {
    let priority = if row.quality_overall < 0.38 || row.diff_area > 0.32 {
        "critical"
    } else if row.quality_overall < 0.55 {
        "high"
    } else if row.quality_overall < 0.72 {
        "medium"
    } else {
        "low"
    }
    .to_string();

    let weakest = if row.grid_regularity < 0.48 {
        "griglia instabile"
    } else if row.palette_compactness < 0.60 {
        "palette dispersa"
    } else if row.diff_area > 0.22 {
        "molta area differente"
    } else if row.diff_score > 24.0 {
        "fedelta' bassa"
    } else {
        "rifinitura generale"
    };

    let recommended_profile = if row.grid_regularity < 0.42 {
        "tileset-cleanup"
    } else if row.diff_area > 0.26 || row.coverage_ratio < 0.45 {
        "character-cleanup"
    } else if row.palette_compactness < 0.55 || row.palette_count > 24 {
        "strict-retro"
    } else if row.diff_score > 24.0 {
        "ultra-cleanup"
    } else {
        "ai-sprite"
    }
    .to_string();

    let mut actions = Vec::new();
    if row.prefilter_mode == "off" && (row.diff_score > 22.0 || row.grid_regularity < 0.55) {
        actions.push("attiva denoise box3".to_string());
    }
    if row.palette_source != "cells" && (row.grid_regularity < 0.58 || row.palette_compactness < 0.68) {
        actions.push("usa palette dalle celle".to_string());
    }
    if row.palette_cleanup_mode != "strict" && (row.palette_compactness < 0.62 || row.palette_count > 24) {
        actions.push("porta palette cleanup a strict".to_string());
    }
    if row.cleanup_mode == "off" && row.diff_area > 0.14 {
        actions.push("attiva cleanup base".to_string());
    }
    if row.repair_mode == "off" {
        actions.push("attiva repair smart".to_string());
    } else if row.repair_mode == "basic" && row.diff_area > 0.18 {
        actions.push("alza repair a smart".to_string());
    } else if row.repair_mode != "ultra" && (row.diff_area > 0.26 || row.quality_overall < 0.38) {
        actions.push("prova repair ultra".to_string());
    }
    if actions.is_empty() {
        actions.push("mantieni setup attuale e verifica manualmente".to_string());
    }

    (
        priority,
        recommended_profile,
        actions.join(" | "),
        format!("focus principale: {}", weakest),
    )
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> nasty_retropixel::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
            "Usage: nasty-retropixel-cli <input> <output> [k_colors] [--preset ai-sprite|strict-retro|tileset-cleanup|character-cleanup|icon-cleanup|ultra-cleanup|auto] [--pixel-size <n>] [--denoise off|box3] [--palette-source pixels|cells] [--palette-lock <palette.png>] [--palette-cleanup off|basic|strict] [--cell-color mean|dominant|medoid] [--dither off|fs] [--color-space srgb|linear] [--cleanup off|basic] [--repair off|basic|smart|ultra] [--recommend-variant] [--debug-json] [--debug-overlay] [--debug-heatmap] [--debug-dir <path>] [--review-pack-top <n>] [--retry-review-top <n>]\n\nNotes:\n- Use a file input + file output for single image.\n- Use a directory input + directory output for batch processing.".to_string(),
        ));
    }

    let input_path = args[1].clone();
    let output_path = args[2].clone();

    let mut k_colors: Option<usize> = None;
    let mut preset_name: Option<String> = None;
    let mut pixel_size_override: Option<f64> = None;
    let mut denoise: Option<u32> = None;
    let mut palette_source: Option<u32> = None;
    let mut palette_lock_path: Option<PathBuf> = None;
    let mut palette_cleanup_mode: Option<u32> = None;
    let mut cell_color_mode: Option<u32> = None;
    let mut dither: Option<u32> = None;
    let mut color_space: Option<u32> = None;
    let mut cleanup_mode: Option<u32> = None;
    let mut repair_mode: Option<u32> = None;
    let mut debug_json = false;
    let mut debug_overlay = false;
    let mut debug_heatmap = false;
    let mut debug_dir: Option<PathBuf> = None;
    let mut review_pack_top: Option<usize> = None;
    let mut retry_review_top: Option<usize> = None;
    let mut recommend_variant = false;

    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--pixel-size" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
                        "--pixel-size requires a value".to_string(),
                    ));
                };

                match val.parse::<f64>() {
                    Ok(px) if px.is_finite() && px > 0.0 => pixel_size_override = Some(px),
                    _ => {
                        return Err(nasty_retropixel::PixelSnapperError::InvalidInput(format!(
                            "invalid --pixel-size '{}'",
                            val
                        )))
                    }
                }
                i += 2;
            }
            "--preset" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
                        "--preset requires a value".to_string(),
                    ));
                };
                if val.trim().is_empty() {
                    return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
                        "--preset cannot be empty".to_string(),
                    ));
                }
                preset_name = Some(val.to_string());
                i += 2;
            }
            "--denoise" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
                        "--denoise requires a value".to_string(),
                    ));
                };
                denoise = Some(match val.as_str() {
                    "off" => 0,
                    "box3" => 1,
                    _ => {
                        return Err(nasty_retropixel::PixelSnapperError::InvalidInput(format!(
                            "invalid --denoise '{}'",
                            val
                        )))
                    }
                });
                i += 2;
            }
            "--palette-source" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
                        "--palette-source requires a value".to_string(),
                    ));
                };
                palette_source = Some(match val.as_str() {
                    "pixels" => 0,
                    "cells" => 1,
                    _ => {
                        return Err(nasty_retropixel::PixelSnapperError::InvalidInput(format!(
                            "invalid --palette-source '{}'",
                            val
                        )))
                    }
                });
                i += 2;
            }
            "--palette-lock" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
                        "--palette-lock requires a value".to_string(),
                    ));
                };
                if val.trim().is_empty() {
                    return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
                        "--palette-lock cannot be empty".to_string(),
                    ));
                }
                palette_lock_path = Some(PathBuf::from(val));
                i += 2;
            }
            "--palette-cleanup" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
                        "--palette-cleanup requires a value".to_string(),
                    ));
                };
                palette_cleanup_mode = Some(match val.as_str() {
                    "off" => 0,
                    "basic" => 1,
                    "strict" => 2,
                    _ => {
                        return Err(nasty_retropixel::PixelSnapperError::InvalidInput(format!(
                            "invalid --palette-cleanup '{}'",
                            val
                        )))
                    }
                });
                i += 2;
            }
            "--cell-color" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
                        "--cell-color requires a value".to_string(),
                    ));
                };
                cell_color_mode = Some(match val.as_str() {
                    "mean" => 0,
                    "dominant" => 1,
                    "medoid" => 2,
                    _ => {
                        return Err(nasty_retropixel::PixelSnapperError::InvalidInput(format!(
                            "invalid --cell-color '{}'",
                            val
                        )))
                    }
                });
                i += 2;
            }
            "--dither" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
                        "--dither requires a value".to_string(),
                    ));
                };
                dither = Some(match val.as_str() {
                    "off" => 0,
                    "fs" => 1,
                    _ => {
                        return Err(nasty_retropixel::PixelSnapperError::InvalidInput(format!(
                            "invalid --dither '{}'",
                            val
                        )))
                    }
                });
                i += 2;
            }
            "--color-space" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
                        "--color-space requires a value".to_string(),
                    ));
                };
                color_space = Some(match val.as_str() {
                    "srgb" => 0,
                    "linear" => 1,
                    _ => {
                        return Err(nasty_retropixel::PixelSnapperError::InvalidInput(format!(
                            "invalid --color-space '{}'",
                            val
                        )))
                    }
                });
                i += 2;
            }
            "--cleanup" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
                        "--cleanup requires a value".to_string(),
                    ));
                };
                cleanup_mode = Some(match val.as_str() {
                    "off" => 0,
                    "basic" => 1,
                    _ => {
                        return Err(nasty_retropixel::PixelSnapperError::InvalidInput(format!(
                            "invalid --cleanup '{}'",
                            val
                        )))
                    }
                });
                i += 2;
            }
            "--repair" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
                        "--repair requires a value".to_string(),
                    ));
                };
                repair_mode = Some(match val.as_str() {
                    "off" => 0,
                    "basic" => 1,
                    "smart" => 2,
                    "ultra" => 3,
                    _ => {
                        return Err(nasty_retropixel::PixelSnapperError::InvalidInput(format!(
                            "invalid --repair '{}'",
                            val
                        )))
                    }
                });
                i += 2;
            }
            "--debug-json" => {
                debug_json = true;
                i += 1;
            }
            "--debug-overlay" => {
                debug_overlay = true;
                i += 1;
            }
            "--debug-heatmap" => {
                debug_heatmap = true;
                i += 1;
            }
            "--debug-dir" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
                        "--debug-dir requires a value".to_string(),
                    ));
                };
                if val.trim().is_empty() {
                    return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
                        "--debug-dir cannot be empty".to_string(),
                    ));
                }
                debug_dir = Some(PathBuf::from(val));
                i += 2;
            }
            "--review-pack-top" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
                        "--review-pack-top requires a value".to_string(),
                    ));
                };
                match val.parse::<usize>() {
                    Ok(v) if v > 0 => review_pack_top = Some(v),
                    _ => {
                        return Err(nasty_retropixel::PixelSnapperError::InvalidInput(format!(
                            "invalid --review-pack-top '{}'",
                            val
                        )))
                    }
                }
                i += 2;
            }
            "--retry-review-top" => {
                let Some(val) = args.get(i + 1) else {
                    return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
                        "--retry-review-top requires a value".to_string(),
                    ));
                };
                match val.parse::<usize>() {
                    Ok(v) if v > 0 => retry_review_top = Some(v),
                    _ => {
                        return Err(nasty_retropixel::PixelSnapperError::InvalidInput(format!(
                            "invalid --retry-review-top '{}'",
                            val
                        )))
                    }
                }
                i += 2;
            }
            "--recommend-variant" => {
                recommend_variant = true;
                i += 1;
            }
            arg if arg.starts_with("--") => {
                return Err(nasty_retropixel::PixelSnapperError::InvalidInput(format!(
                    "unknown argument '{}'",
                    arg
                )))
            }
            k_arg => {
                match k_arg.parse::<usize>() {
                    Ok(k) if k > 0 => k_colors = Some(k),
                    _ => {
                        return Err(nasty_retropixel::PixelSnapperError::InvalidInput(format!(
                            "invalid k_colors '{}'",
                            k_arg
                        )))
                    }
                }
                i += 1;
            }
        }
    }

    let mut config = nasty_retropixel::Config::default();
    if let Some(preset_name) = &preset_name {
        if preset_name != "auto" {
            apply_named_preset(&mut config, preset_name)?;
        }
    }
    if let Some(k) = k_colors {
        config.k_colors = k;
    }
    config.pixel_size_override = pixel_size_override;
    if let Some(v) = denoise {
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
    if let Some(v) = dither {
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
    if review_pack_top.is_some() || retry_review_top.is_some() {
        debug_json = true;
        debug_overlay = true;
        debug_heatmap = true;
    }
    let debug = DebugExportOptions {
        write_json: debug_json,
        write_overlay: debug_overlay,
        write_heatmap: debug_heatmap,
        output_dir: debug_dir,
    };
    let palette_lock_bytes = if let Some(path) = &palette_lock_path {
        Some(std::fs::read(path).map_err(|e| {
            nasty_retropixel::PixelSnapperError::ProcessingError(format!(
                "Failed to read palette lock file '{}': {}",
                path.display(),
                e
            ))
        })?)
    } else {
        None
    };

    let input = Path::new(&input_path);
    let output = Path::new(&output_path);

    if preset_name.as_deref() == Some("auto") && input.is_file() {
        let input_bytes = std::fs::read(input).map_err(|e| {
            nasty_retropixel::PixelSnapperError::ProcessingError(format!(
                "Failed to read input file for auto preset '{}': {}",
                input.display(),
                e
            ))
        })?;
        let suggestion = suggest_setup_for_image_bytes(&input_bytes)?;
        println!(
            "Auto preset: {} ({})",
            suggestion.preset_key, suggestion.reason
        );
        apply_named_preset(&mut config, &suggestion.preset_key)?;
        config.prefilter_mode = suggestion.recommended_prefilter_mode;
        config.palette_source = suggestion.recommended_palette_source;
        config.palette_cleanup_mode = suggestion.recommended_palette_cleanup_mode;
        config.cell_color_mode = suggestion.recommended_cell_color_mode;
        config.cleanup_mode = suggestion.recommended_cleanup_mode;
        config.repair_mode = suggestion.recommended_repair_mode;
        println!(
            "Auto setup: denoise={} | palette={} | palette-fix={} | cell={} | cleanup={} | repair={} | trim={}",
            suggestion.recommended_prefilter_label,
            suggestion.recommended_palette_source_label,
            suggestion.recommended_palette_cleanup_label,
            suggestion.recommended_cell_color_label,
            suggestion.recommended_cleanup_label,
            suggestion.recommended_repair_label,
            if suggestion.recommended_trim_transparent {
                "on"
            } else {
                "off"
            }
        );
        println!("Auto setup note: {}", suggestion.recommendation_reason);
        if let Some(k) = k_colors {
            config.k_colors = k;
        }
        config.pixel_size_override = pixel_size_override;
        if let Some(v) = denoise {
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
        if let Some(v) = dither {
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
    }

    if recommend_variant {
        if !input.is_file() {
            return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
                "--recommend-variant is supported only for single-file input".to_string(),
            ));
        }
        let input_bytes = std::fs::read(input).map_err(|e| {
            nasty_retropixel::PixelSnapperError::ProcessingError(format!(
                "Failed to read input file for variant recommendation '{}': {}",
                input.display(),
                e
            ))
        })?;
        let report = recommend_variant_for_image_bytes(&input_bytes, palette_lock_bytes.as_deref())?;
        println!(
            "Variant consigliata: {} ({})",
            report.recommendation.label, report.recommendation.reason
        );
        for m in report.metrics {
            println!(
                "  - {}: diff {:.1} | area {}% | palette {} | aggress {}%",
                m.label,
                m.diff_score,
                (m.diff_area * 100.0).round() as i64,
                m.palette_count,
                (m.aggressiveness * 100.0).round() as i64
            );
        }
    }

    if input.is_dir() {
        if preset_name.as_deref() == Some("auto") {
            return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
                "--preset auto is currently supported only for single-file input".to_string(),
            ));
        }
        let input_root = PathBuf::from(input);
        let output_root = PathBuf::from(output);
        let batch = BatchConfig {
            input_dir: input_root.clone(),
            output_dir: output_root.clone(),
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
            palette_lock_bytes: palette_lock_bytes.clone(),
            debug: debug.clone(),
        };

        let output_to_input_rel = Mutex::new(HashMap::<String, String>::new());
        let batch_result = process_batch_with_reporter(&batch, |event| match event {
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
                if let (Ok(input_rel), Ok(output_rel), Ok(mut map)) = (
                    input.strip_prefix(&input_root),
                    output.strip_prefix(&output_root),
                    output_to_input_rel.lock(),
                ) {
                    map.insert(
                        output_rel.to_string_lossy().replace('\\', "/"),
                        input_rel.to_string_lossy().replace('\\', "/"),
                    );
                }
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
        });

        if debug.write_json {
            let debug_root = debug
                .output_dir
                .clone()
                .unwrap_or_else(|| PathBuf::from(output));
            let source_map = output_to_input_rel
                .into_inner()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            match write_batch_summary_reports(&debug_root, Some(&source_map)) {
                Ok(Some(artifacts)) => {
                    let count = artifacts.rows.len();
                    println!(
                        "Batch quality summary: {} item{} -> {} | {}",
                        count,
                        if count == 1 { "" } else { "s" },
                        artifacts.json_path.display(),
                        artifacts.csv_path.display()
                    );
                    if let Some(top_n) = review_pack_top {
                        let review_root = write_review_pack(
                            &debug_root,
                            output,
                            &artifacts.rows,
                            top_n,
                        )?;
                        let exported = artifacts.rows.len().min(top_n);
                        println!(
                            "Review pack: top {} worst case{} -> {}",
                            exported,
                            if exported == 1 { "" } else { "s" },
                            review_root.display()
                        );
                    }
                    if let Some(top_n) = retry_review_top {
                        let retry_root = write_retry_review_pack(
                            &input_root,
                            &output_root,
                            &debug_root,
                            palette_lock_bytes.as_deref(),
                            &artifacts.rows,
                            top_n,
                        )?;
                        let exported = artifacts.rows.len().min(top_n);
                        println!(
                            "Retry review: analizzati top {} worst case{} -> {}",
                            exported,
                            if exported == 1 { "" } else { "s" },
                            retry_root.display()
                        );
                    }
                }
                Ok(None) => {}
                Err(err) => {
                    if batch_result.is_ok() {
                        return Err(err);
                    }
                    eprintln!("Batch summary export failed: {}", err);
                }
            }
        }

        return batch_result;
    }

    if review_pack_top.is_some() || retry_review_top.is_some() {
        return Err(nasty_retropixel::PixelSnapperError::InvalidInput(
            "--review-pack-top e --retry-review-top sono supportati solo con input directory batch"
                .to_string(),
        ));
    }

    let actual_output_path = if output.is_dir() {
        let stem = input.file_stem().and_then(|s| s.to_str()).filter(|s| !s.is_empty());
        let Some(stem) = stem else {
            return Err(nasty_retropixel::PixelSnapperError::InvalidInput(format!(
                "Input path has no file stem: {}",
                input.display()
            )));
        };
        output.join(format!("{}.png", stem))
    } else {
        PathBuf::from(output)
    };

    println!("Processing: {}", input_path);

    process_file_with_debug_exports(
        input,
        &actual_output_path,
        &config,
        &debug,
        palette_lock_bytes.as_deref(),
    )?;

    println!("Saved to: {}", actual_output_path.display());
    if debug.write_json || debug.write_overlay || debug.write_heatmap {
        let debug_root = debug
            .output_dir
            .clone()
            .unwrap_or_else(|| actual_output_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf());
        println!("Debug artifacts: {}", debug_root.display());
    }
    Ok(())
}

fn write_batch_summary_reports(
    debug_root: &Path,
    output_to_input_rel: Option<&HashMap<String, String>>,
) -> nasty_retropixel::Result<Option<BatchSummaryArtifacts>> {
    let mut debug_files = Vec::new();
    collect_debug_json_files(debug_root, &mut debug_files)?;
    debug_files.sort();
    if debug_files.is_empty() {
        return Ok(None);
    }

    let mut rows = Vec::new();
    for file in debug_files {
        let raw = std::fs::read(&file).map_err(|e| {
            nasty_retropixel::PixelSnapperError::ProcessingError(format!(
                "Failed to read debug report '{}': {}",
                file.display(),
                e
            ))
        })?;
        let value: serde_json::Value = serde_json::from_slice(&raw).map_err(|e| {
            nasty_retropixel::PixelSnapperError::ProcessingError(format!(
                "Failed to parse debug report '{}': {}",
                file.display(),
                e
            ))
        })?;

        let rel_path = file
            .strip_prefix(debug_root)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        let output_relative_path = rel_path
            .strip_suffix(".debug.json")
            .map(|s| format!("{}.png", s))
            .unwrap_or_else(|| rel_path.clone());
        let mut row = BatchSummaryRow {
            rank: 0,
            source_relative_path: output_to_input_rel
                .and_then(|map| map.get(&output_relative_path))
                .cloned()
                .unwrap_or_default(),
            relative_path: rel_path,
            output_relative_path,
            review_priority: String::new(),
            recommended_profile: String::new(),
            recommended_actions: String::new(),
            recommendation_reason: String::new(),
            k_colors: json_u64(&value, &["config", "k_colors"]).unwrap_or(16) as usize,
            pixel_size_override: json_optional_f64(&value, &["config", "pixel_size_override"]),
            quality_overall: json_f64(&value, &["quality", "overall_score"]),
            diff_score: json_f64(&value, &["quality", "diff_score"]),
            diff_area: json_f64(&value, &["quality", "diff_area"]),
            grid_regularity: json_f64(&value, &["quality", "grid_regularity"]),
            palette_compactness: json_f64(&value, &["quality", "palette_compactness"]),
            coverage_ratio: json_f64(&value, &["quality", "coverage_ratio"]),
            palette_count: json_u64(&value, &["palette"])
                .or_else(|| json_u64(&value, &["palette_count"]))
                .unwrap_or(0) as usize,
            output_width: json_u64(&value, &["output_width"]).unwrap_or(0) as u32,
            output_height: json_u64(&value, &["output_height"]).unwrap_or(0) as u32,
            step_x: json_f64(&value, &["step_x"]),
            step_y: json_f64(&value, &["step_y"]),
            prefilter_mode: json_str(&value, &["config", "prefilter_mode"]),
            palette_source: json_str(&value, &["config", "palette_source"]),
            palette_cleanup_mode: json_str(&value, &["config", "palette_cleanup_mode"]),
            cell_color_mode: json_str(&value, &["config", "cell_color_mode"]),
            dither_mode: json_str(&value, &["config", "dither_mode"]),
            color_space: json_str(&value, &["config", "color_space"]),
            cleanup_mode: json_str(&value, &["config", "cleanup_mode"]),
            repair_mode: json_str(&value, &["config", "repair_mode"]),
        };
        let (priority, profile, actions, reason) = summarize_recommendation(&row);
        row.review_priority = priority;
        row.recommended_profile = profile;
        row.recommended_actions = actions;
        row.recommendation_reason = reason;
        rows.push(row);
    }

    rows.sort_by(|a, b| {
        a.quality_overall
            .total_cmp(&b.quality_overall)
            .then_with(|| a.diff_score.total_cmp(&b.diff_score))
            .then_with(|| a.relative_path.cmp(&b.relative_path))
    });
    for (idx, row) in rows.iter_mut().enumerate() {
        row.rank = idx + 1;
    }

    let avg_quality = if rows.is_empty() {
        0.0
    } else {
        rows.iter().map(|r| r.quality_overall).sum::<f64>() / rows.len() as f64
    };
    let avg_diff_score = if rows.is_empty() {
        0.0
    } else {
        rows.iter().map(|r| r.diff_score).sum::<f64>() / rows.len() as f64
    };
    let avg_diff_area = if rows.is_empty() {
        0.0
    } else {
        rows.iter().map(|r| r.diff_area).sum::<f64>() / rows.len() as f64
    };

    let json_path = debug_root.join("nasty-retropixel.batch-summary.json");
    let csv_path = debug_root.join("nasty-retropixel.batch-summary.csv");
    let json_payload = serde_json::json!({
        "count": rows.len(),
        "sorted_by": "quality_overall_asc",
        "focus": "lowest_quality_first",
        "averages": {
            "quality_overall": avg_quality,
            "diff_score": avg_diff_score,
            "diff_area": avg_diff_area
        },
        "worst": rows.first(),
        "best": rows.last(),
        "rows": rows
    });
    std::fs::write(
        &json_path,
        serde_json::to_vec_pretty(&json_payload).map_err(|e| {
            nasty_retropixel::PixelSnapperError::ProcessingError(format!(
                "Failed to serialize batch summary '{}': {}",
                json_path.display(),
                e
            ))
        })?,
    )
    .map_err(|e| {
        nasty_retropixel::PixelSnapperError::ProcessingError(format!(
            "Failed to write batch summary '{}': {}",
            json_path.display(),
            e
        ))
    })?;

    std::fs::write(&csv_path, build_batch_summary_csv(&rows)).map_err(|e| {
        nasty_retropixel::PixelSnapperError::ProcessingError(format!(
            "Failed to write batch summary CSV '{}': {}",
            csv_path.display(),
            e
        ))
    })?;

    Ok(Some(BatchSummaryArtifacts {
        json_path,
        csv_path,
        rows,
    }))
}

fn write_review_pack(
    debug_root: &Path,
    output_root: &Path,
    rows: &[BatchSummaryRow],
    top_n: usize,
) -> nasty_retropixel::Result<PathBuf> {
    let selected = rows.iter().take(top_n).collect::<Vec<_>>();
    let review_root = debug_root.join("nasty-retropixel.review-pack");
    std::fs::create_dir_all(&review_root).map_err(|e| {
        nasty_retropixel::PixelSnapperError::ProcessingError(format!(
            "Failed to create review pack directory '{}': {}",
            review_root.display(),
            e
        ))
    })?;

    let payload = serde_json::json!({
        "count": selected.len(),
        "source_count": rows.len(),
        "focus": "top_worst_only",
        "rows": selected,
        "retry_plan_candidates": selected.iter().map(|row| serde_json::json!({
            "relative_path": row.relative_path,
            "baseline_quality": row.quality_overall,
            "review_priority": row.review_priority,
            "recommended_profile": row.recommended_profile,
            "recommended_actions": row.recommended_actions,
            "recommendation_reason": row.recommendation_reason,
            "candidates": build_retry_plan_candidates(row),
        })).collect::<Vec<_>>(),
    });
    let manifest_json = review_root.join("nasty-retropixel.review-pack.json");
    std::fs::write(
        &manifest_json,
        serde_json::to_vec_pretty(&payload).map_err(|e| {
            nasty_retropixel::PixelSnapperError::ProcessingError(format!(
                "Failed to serialize review pack manifest '{}': {}",
                manifest_json.display(),
                e
            ))
        })?,
    )
    .map_err(|e| {
        nasty_retropixel::PixelSnapperError::ProcessingError(format!(
            "Failed to write review pack manifest '{}': {}",
            manifest_json.display(),
            e
        ))
    })?;

    let manifest_csv = review_root.join("nasty-retropixel.review-pack.csv");
    std::fs::write(
        &manifest_csv,
        build_batch_summary_csv(
            &selected.iter().map(|row| (*row).clone()).collect::<Vec<BatchSummaryRow>>(),
        ),
    )
    .map_err(|e| {
        nasty_retropixel::PixelSnapperError::ProcessingError(format!(
            "Failed to write review pack CSV '{}': {}",
            manifest_csv.display(),
            e
        ))
    })?;

    for row in selected {
        let output_src = output_root.join(&row.output_relative_path);
        let output_dst = review_root.join("outputs").join(&row.output_relative_path);
        copy_if_exists(&output_src, &output_dst)?;

        let debug_json_src = debug_root.join(&row.relative_path);
        let debug_json_dst = review_root.join("debug").join(&row.relative_path);
        copy_if_exists(&debug_json_src, &debug_json_dst)?;

        let overlay_rel = row.relative_path.replace(".debug.json", ".overlay.png");
        let overlay_src = debug_root.join(&overlay_rel);
        let overlay_dst = review_root.join("debug").join(&overlay_rel);
        copy_if_exists(&overlay_src, &overlay_dst)?;

        let heatmap_rel = row.relative_path.replace(".debug.json", ".heatmap.png");
        let heatmap_src = debug_root.join(&heatmap_rel);
        let heatmap_dst = review_root.join("debug").join(&heatmap_rel);
        copy_if_exists(&heatmap_src, &heatmap_dst)?;
    }

    Ok(review_root)
}

fn retry_overlay_relative_path(relative_debug_path: &str) -> String {
    relative_debug_path.replace(".debug.json", ".overlay.png")
}

fn retry_heatmap_relative_path(relative_debug_path: &str) -> String {
    relative_debug_path.replace(".debug.json", ".heatmap.png")
}

fn write_retry_output_file(path: &Path, bytes: &[u8]) -> nasty_retropixel::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            nasty_retropixel::PixelSnapperError::ProcessingError(format!(
                "Failed to create retry review directory '{}': {}",
                parent.display(),
                e
            ))
        })?;
    }
    std::fs::write(path, bytes).map_err(|e| {
        nasty_retropixel::PixelSnapperError::ProcessingError(format!(
            "Failed to write retry review file '{}': {}",
            path.display(),
            e
        ))
    })?;
    Ok(())
}

fn write_retry_debug_report(path: &Path, report: &DebugReport) -> nasty_retropixel::Result<()> {
    let payload = serde_json::to_vec_pretty(report).map_err(|e| {
        nasty_retropixel::PixelSnapperError::ProcessingError(format!(
            "Failed to serialize retry debug report '{}': {}",
            path.display(),
            e
        ))
    })?;
    write_retry_output_file(path, &payload)
}

fn build_retry_review_csv(rows: &[RetryReviewRow]) -> String {
    let mut csv = String::from(
        "relative_path,source_relative_path,retry_status,baseline_quality,best_retry_candidate,best_retry_quality,best_retry_delta_quality,selected_candidate,selected_quality,selected_delta_quality,exported_output_relative_path,recommended_profile,recommended_actions,recommendation_reason\n",
    );
    for row in rows {
        let best = row.best_retry_candidate.as_ref();
        let selected = row.selected_candidate.as_ref();
        csv.push_str(&format!(
            "{},{},{},{:.4},{},{},{},{},{},{},{},{},{},{}\n",
            csv_escape(&row.relative_path),
            csv_escape(&row.source_relative_path),
            csv_escape(&row.retry_status),
            row.baseline_quality,
            csv_escape(best.map(|item| item.label.as_str()).unwrap_or("baseline")),
            csv_escape(&best.and_then(|item| item.quality_overall.map(|v| format!("{:.4}", v))).unwrap_or_default()),
            csv_escape(&best.and_then(|item| item.delta_quality.map(|v| format!("{:.4}", v))).unwrap_or_default()),
            csv_escape(selected.map(|item| item.label.as_str()).unwrap_or("")),
            csv_escape(&selected.and_then(|item| item.quality_overall.map(|v| format!("{:.4}", v))).unwrap_or_default()),
            csv_escape(&selected.and_then(|item| item.delta_quality.map(|v| format!("{:.4}", v))).unwrap_or_default()),
            csv_escape(row.exported_output_relative_path.as_deref().unwrap_or("")),
            csv_escape(&row.recommended_profile),
            csv_escape(&row.recommended_actions),
            csv_escape(&row.recommendation_reason),
        ));
    }
    csv
}

fn write_retry_review_pack(
    input_root: &Path,
    output_root: &Path,
    debug_root: &Path,
    palette_lock_bytes: Option<&[u8]>,
    rows: &[BatchSummaryRow],
    top_n: usize,
) -> nasty_retropixel::Result<PathBuf> {
    let selected = rows.iter().take(top_n).collect::<Vec<_>>();
    let retry_root = debug_root.join("nasty-retropixel.retry-review");
    std::fs::create_dir_all(&retry_root).map_err(|e| {
        nasty_retropixel::PixelSnapperError::ProcessingError(format!(
            "Failed to create retry review directory '{}': {}",
            retry_root.display(),
            e
        ))
    })?;

    let mut manifest_rows = Vec::new();
    let mut improved_count = 0usize;

    for row in selected {
        let baseline_output_src = output_root.join(&row.output_relative_path);
        let baseline_output_dst = retry_root.join("baseline").join("outputs").join(&row.output_relative_path);
        copy_if_exists(&baseline_output_src, &baseline_output_dst)?;

        let baseline_debug_src = debug_root.join(&row.relative_path);
        let baseline_debug_dst = retry_root.join("baseline").join("debug").join(&row.relative_path);
        copy_if_exists(&baseline_debug_src, &baseline_debug_dst)?;

        let baseline_overlay_rel = retry_overlay_relative_path(&row.relative_path);
        let baseline_overlay_src = debug_root.join(&baseline_overlay_rel);
        let baseline_overlay_dst = retry_root.join("baseline").join("debug").join(&baseline_overlay_rel);
        copy_if_exists(&baseline_overlay_src, &baseline_overlay_dst)?;

        let baseline_heatmap_rel = retry_heatmap_relative_path(&row.relative_path);
        let baseline_heatmap_src = debug_root.join(&baseline_heatmap_rel);
        let baseline_heatmap_dst = retry_root.join("baseline").join("debug").join(&baseline_heatmap_rel);
        copy_if_exists(&baseline_heatmap_src, &baseline_heatmap_dst)?;

        let mut candidates = Vec::new();
        let mut best_retry: Option<RetrySelection> = None;
        let mut selected_retry: Option<RetrySelection> = None;
        let mut exported_output_relative_path = None;

        let retry_status = if !row.source_relative_path.is_empty() {
            let source_path = input_root.join(&row.source_relative_path);
            if source_path.exists() {
                let input_bytes = std::fs::read(&source_path).map_err(|e| {
                    nasty_retropixel::PixelSnapperError::ProcessingError(format!(
                        "Failed to read retry review source '{}': {}",
                        source_path.display(),
                        e
                    ))
                })?;
                let base_config = config_from_summary_row(row);
                for plan in build_retry_candidate_plans_for_cli(row)? {
                    let candidate_config = apply_retry_algo_overrides(&base_config, &plan.algo_overrides);
                    let processed = if let Some(palette_lock_bytes) = palette_lock_bytes {
                        process_image_debug_with_palette_image_config(
                            &input_bytes,
                            palette_lock_bytes,
                            candidate_config,
                        )
                    } else {
                        process_image_debug_with_config(&input_bytes, candidate_config)
                    };

                    match processed {
                        Ok((output_bytes, overlay_bytes, heatmap_bytes, report)) => {
                            let quality = report.quality.overall_score;
                            let delta = quality - row.quality_overall;
                            let evaluation = RetryCandidateEvaluation {
                                key: plan.key.clone(),
                                label: plan.label.clone(),
                                reason: plan.reason.clone(),
                                algo_overrides: plan.algo_overrides.clone(),
                                quality_overall: Some((quality * 10000.0).round() / 10000.0),
                                diff_score: Some((report.quality.diff_score * 10000.0).round() / 10000.0),
                                diff_area: Some((report.quality.diff_area * 10000.0).round() / 10000.0),
                                delta_quality: Some((delta * 10000.0).round() / 10000.0),
                                error: None,
                            };
                            candidates.push(evaluation.clone());
                            let selection = RetrySelection {
                                evaluation: evaluation.clone(),
                                output_bytes,
                                overlay_bytes,
                                heatmap_bytes,
                                report,
                            };
                            if best_retry
                                .as_ref()
                                .and_then(|item| item.evaluation.delta_quality)
                                .unwrap_or(f64::NEG_INFINITY)
                                < evaluation.delta_quality.unwrap_or(f64::NEG_INFINITY)
                            {
                                best_retry = Some(selection.clone());
                            }
                            if evaluation.delta_quality.unwrap_or(0.0) > 0.0
                                && selected_retry
                                    .as_ref()
                                    .and_then(|item| item.evaluation.delta_quality)
                                    .unwrap_or(f64::NEG_INFINITY)
                                    < evaluation.delta_quality.unwrap_or(f64::NEG_INFINITY)
                            {
                                selected_retry = Some(selection);
                            }
                        }
                        Err(err) => {
                            candidates.push(RetryCandidateEvaluation {
                                key: plan.key,
                                label: plan.label,
                                reason: plan.reason,
                                algo_overrides: plan.algo_overrides,
                                quality_overall: None,
                                diff_score: None,
                                diff_area: None,
                                delta_quality: None,
                                error: Some(err.to_string()),
                            });
                        }
                    }
                }
                if selected_retry.is_some() {
                    "improved".to_string()
                } else if best_retry.is_some() {
                    "no_gain".to_string()
                } else if !candidates.is_empty() {
                    "attempt_failed".to_string()
                } else {
                    "not_available".to_string()
                }
            } else {
                "missing_source".to_string()
            }
        } else {
            "missing_source".to_string()
        };

        if let Some(selection) = &selected_retry {
            let output_rel = Path::new("outputs").join(&row.output_relative_path);
            let debug_rel = Path::new("debug").join(&row.relative_path);
            let overlay_rel = Path::new("debug").join(retry_overlay_relative_path(&row.relative_path));
            let heatmap_rel = Path::new("debug").join(retry_heatmap_relative_path(&row.relative_path));

            write_retry_output_file(&retry_root.join(&output_rel), &selection.output_bytes)?;
            write_retry_debug_report(&retry_root.join(&debug_rel), &selection.report)?;
            write_retry_output_file(&retry_root.join(&overlay_rel), &selection.overlay_bytes)?;
            write_retry_output_file(&retry_root.join(&heatmap_rel), &selection.heatmap_bytes)?;
            exported_output_relative_path =
                Some(output_rel.to_string_lossy().replace('\\', "/"));
            improved_count += 1;
        }

        manifest_rows.push(RetryReviewRow {
            relative_path: row.relative_path.clone(),
            source_relative_path: row.source_relative_path.clone(),
            retry_status,
            baseline_quality: row.quality_overall,
            recommended_profile: row.recommended_profile.clone(),
            recommended_actions: row.recommended_actions.clone(),
            recommendation_reason: row.recommendation_reason.clone(),
            best_retry_candidate: best_retry.as_ref().map(|item| item.evaluation.clone()),
            selected_candidate: selected_retry.as_ref().map(|item| item.evaluation.clone()),
            exported_output_relative_path,
            candidates,
        });
    }

    let payload = serde_json::json!({
        "count": manifest_rows.len(),
        "source_count": rows.len(),
        "focus": "top_worst_retry_review",
        "requested_top_n": top_n,
        "improved_count": improved_count,
        "exported_count": improved_count,
        "rows": manifest_rows,
    });
    let manifest_json = retry_root.join("nasty-retropixel.retry-review.json");
    std::fs::write(
        &manifest_json,
        serde_json::to_vec_pretty(&payload).map_err(|e| {
            nasty_retropixel::PixelSnapperError::ProcessingError(format!(
                "Failed to serialize retry review manifest '{}': {}",
                manifest_json.display(),
                e
            ))
        })?,
    )
    .map_err(|e| {
        nasty_retropixel::PixelSnapperError::ProcessingError(format!(
            "Failed to write retry review manifest '{}': {}",
            manifest_json.display(),
            e
        ))
    })?;

    let manifest_csv = retry_root.join("nasty-retropixel.retry-review.csv");
    std::fs::write(&manifest_csv, build_retry_review_csv(&manifest_rows)).map_err(|e| {
        nasty_retropixel::PixelSnapperError::ProcessingError(format!(
            "Failed to write retry review CSV '{}': {}",
            manifest_csv.display(),
            e
        ))
    })?;

    Ok(retry_root)
}

fn copy_if_exists(src: &Path, dst: &Path) -> nasty_retropixel::Result<()> {
    if !src.exists() {
        return Ok(());
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            nasty_retropixel::PixelSnapperError::ProcessingError(format!(
                "Failed to create directory '{}': {}",
                parent.display(),
                e
            ))
        })?;
    }
    std::fs::copy(src, dst).map_err(|e| {
        nasty_retropixel::PixelSnapperError::ProcessingError(format!(
            "Failed to copy '{}' -> '{}': {}",
            src.display(),
            dst.display(),
            e
        ))
    })?;
    Ok(())
}

fn collect_debug_json_files(root: &Path, out: &mut Vec<PathBuf>) -> nasty_retropixel::Result<()> {
    let entries = std::fs::read_dir(root).map_err(|e| {
        nasty_retropixel::PixelSnapperError::ProcessingError(format!(
            "Failed to read debug directory '{}': {}",
            root.display(),
            e
        ))
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            nasty_retropixel::PixelSnapperError::ProcessingError(format!(
                "Failed to read entry in debug directory '{}': {}",
                root.display(),
                e
            ))
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_debug_json_files(&path, out)?;
            continue;
        }
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with(".debug.json"))
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
    Ok(())
}

fn json_node<'a>(value: &'a serde_json::Value, path: &[&str]) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn json_f64(value: &serde_json::Value, path: &[&str]) -> f64 {
    json_node(value, path).and_then(|v| v.as_f64()).unwrap_or(0.0)
}

fn json_optional_f64(value: &serde_json::Value, path: &[&str]) -> Option<f64> {
    json_node(value, path).and_then(|v| v.as_f64())
}

fn json_u64(value: &serde_json::Value, path: &[&str]) -> Option<u64> {
    let node = json_node(value, path)?;
    node.as_u64().or_else(|| node.as_array().map(|a| a.len() as u64))
}

fn json_str(value: &serde_json::Value, path: &[&str]) -> String {
    json_node(value, path)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn build_batch_summary_csv(rows: &[BatchSummaryRow]) -> String {
    let mut csv = String::from(
        "rank,source_relative_path,relative_path,output_relative_path,review_priority,recommended_profile,recommended_actions,recommendation_reason,k_colors,pixel_size_override,quality_overall,diff_score,diff_area,grid_regularity,palette_compactness,coverage_ratio,palette_count,output_width,output_height,step_x,step_y,prefilter_mode,palette_source,palette_cleanup_mode,cell_color_mode,dither_mode,color_space,cleanup_mode,repair_mode\n",
    );
    for row in rows {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{},{},{},{:.4},{:.4},{},{},{},{},{},{},{},{}\n",
            row.rank,
            csv_escape(&row.source_relative_path),
            csv_escape(&row.relative_path),
            csv_escape(&row.output_relative_path),
            csv_escape(&row.review_priority),
            csv_escape(&row.recommended_profile),
            csv_escape(&row.recommended_actions),
            csv_escape(&row.recommendation_reason),
            row.k_colors,
            csv_escape(&row.pixel_size_override.map(|v| format!("{:.4}", v)).unwrap_or_default()),
            row.quality_overall,
            row.diff_score,
            row.diff_area,
            row.grid_regularity,
            row.palette_compactness,
            row.coverage_ratio,
            row.palette_count,
            row.output_width,
            row.output_height,
            row.step_x,
            row.step_y,
            csv_escape(&row.prefilter_mode),
            csv_escape(&row.palette_source),
            csv_escape(&row.palette_cleanup_mode),
            csv_escape(&row.cell_color_mode),
            csv_escape(&row.dither_mode),
            csv_escape(&row.color_space),
            csv_escape(&row.cleanup_mode),
            csv_escape(&row.repair_mode),
        ));
    }
    csv
}
