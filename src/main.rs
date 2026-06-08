use image::{GenericImageView, ImageBuffer, Rgba, RgbaImage};
use rand::prelude::*;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, WeightedIndex};
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;
use std::cmp::Ordering;
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::env;
use std::error::Error;
use std::fmt;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use js_sys::{Object, Reflect, Uint32Array, Uint8Array};

#[derive(Debug, Clone)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct Config {
    pub k_colors: usize,
    pub pixel_size_override: Option<f64>,
    pub prefilter_mode: u32,
    pub palette_source: u32,
    pub dither_mode: u32,
    pub color_space: u32,
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
            dither_mode: 0,
            color_space: 1,
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

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
struct ProcessedImage {
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
    pub dither_mode: u32,
    pub color_space: u32,
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
            dither_mode: config.dither_mode,
            color_space: config.color_space,
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
            dither_mode: config.dither_mode,
            color_space: config.color_space,
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
    col_cuts: Vec<usize>,
    row_cuts: Vec<usize>,
    step_x: f64,
    step_y: f64,
    input_width: u32,
    input_height: u32,
}

fn process_image_bytes_common(input_bytes: &[u8], config: Option<Config>) -> Result<Vec<u8>> {
    process_image_common(input_bytes, config).map(|out| out.output_bytes)
}

fn process_image_common(input_bytes: &[u8], config: Option<Config>) -> Result<ProcessedImage> {
    let config = config.unwrap_or_default();
    let out = process_image_bytes_debug_common(input_bytes, Some(config.clone()))?;
    Ok(ProcessedImage {
        output_bytes: out.output_bytes,
        pixel_size: out.step_x,
        pixel_size_override: config.pixel_size_override.is_some(),
        output_width: (out.col_cuts.len() - 1) as u32,
        output_height: (out.row_cuts.len() - 1) as u32,
    })
}

fn process_image_bytes_debug_common(input_bytes: &[u8], config: Option<Config>) -> Result<DebugOutput> {
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

    let quantized_for_profile = quantize_image(&rgba_prefiltered, &profile_config)?;
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

    let output_img = match config.palette_source {
        1 => resample_cells(&rgba_prefiltered, &col_cuts, &row_cuts, &config)?,
        _ => {
            let quantized_img = quantize_image(&rgba_prefiltered, &config)?;
            resample_mode(&quantized_img, &col_cuts, &row_cuts)?
        }
    };

    // Returns bytes for both implementations
    let mut output_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut output_bytes);
    output_img
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| PixelSnapperError::ImageError(e))?;

    Ok(DebugOutput {
        output_bytes,
        col_cuts,
        row_cuts,
        step_x,
        step_y,
        input_width: width,
        input_height: height,
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

/// WASM entry point
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn process_image(
    input_bytes: &[u8],
    k_colors: Option<u32>,
    pixel_size_override: Option<f64>,
    prefilter_mode: Option<u32>,
    palette_source: Option<u32>,
    dither_mode: Option<u32>,
    color_space: Option<u32>,
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
    if let Some(v) = dither_mode {
        config.dither_mode = v;
    }
    if let Some(v) = color_space {
        config.color_space = v;
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
    dither_mode: Option<u32>,
    color_space: Option<u32>,
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
    if let Some(v) = dither_mode {
        config.dither_mode = v;
    }
    if let Some(v) = color_space {
        config.color_space = v;
    }

    let out = process_image_bytes_debug_common(input_bytes, Some(config))
        .map_err(|e| wasm_bindgen::JsValue::from(e))?;

    let obj = Object::new();

    let bytes = Uint8Array::from(out.output_bytes.as_slice());
    let cols = Uint32Array::new_with_length(out.col_cuts.len() as u32);
    for (i, v) in out.col_cuts.iter().enumerate() {
        cols.set_index(i as u32, *v as u32);
    }
    let rows = Uint32Array::new_with_length(out.row_cuts.len() as u32);
    for (i, v) in out.row_cuts.iter().enumerate() {
        rows.set_index(i as u32, *v as u32);
    }

    Reflect::set(&obj, &JsValue::from_str("bytes"), &bytes.into())?;
    Reflect::set(&obj, &JsValue::from_str("col_cuts"), &cols.into())?;
    Reflect::set(&obj, &JsValue::from_str("row_cuts"), &rows.into())?;
    Reflect::set(&obj, &JsValue::from_str("step_x"), &JsValue::from_f64(out.step_x))?;
    Reflect::set(&obj, &JsValue::from_str("step_y"), &JsValue::from_f64(out.step_y))?;
    Reflect::set(
        &obj,
        &JsValue::from_str("input_width"),
        &JsValue::from_f64(out.input_width as f64),
    )?;
    Reflect::set(
        &obj,
        &JsValue::from_str("input_height"),
        &JsValue::from_f64(out.input_height as f64),
    )?;

    Ok(obj.into())
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
        .map(|input| Ok((input.clone(), get_output_path(output_dir, input)?)))
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
            let result = process_file(input, output, &item_config).map(|_| ());
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
    let img_bytes = std::fs::read(input_path).map_err(|e| {
        PixelSnapperError::ProcessingError(format!(
            "Failed to read input file '{}': {}",
            input_path.display(),
            e
        ))
    })?;

    let processed = process_image_common(&img_bytes, Some(config.clone()))?;

    std::fs::write(output_path, &processed.output_bytes).map_err(|e| {
        PixelSnapperError::ProcessingError(format!(
            "Failed to write output file '{}': {}",
            output_path.display(),
            e
        ))
    })?;

    Ok(processed)
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
fn collect_batch_inputs(input_dir: &Path) -> Result<Vec<PathBuf>> {
    let entries = std::fs::read_dir(input_dir).map_err(|e| {
        PixelSnapperError::ProcessingError(format!(
            "Failed to read input directory '{}': {}",
            input_dir.display(),
            e
        ))
    })?;

    let mut inputs = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| {
            PixelSnapperError::ProcessingError(format!(
                "Failed to read an entry from '{}': {}",
                input_dir.display(),
                e
            ))
        })?;
        let path = entry.path();
        if path.is_file() && is_supported_image_path(&path) {
            inputs.push(path);
        }
    }

    Ok(inputs)
}

#[cfg(not(target_arch = "wasm32"))]
fn is_supported_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg"))
        .unwrap_or(false)
}

#[cfg(not(target_arch = "wasm32"))]
fn get_output_path(output_dir: &Path, input_path: &Path) -> Result<PathBuf> {
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

    Ok(output_dir.join(format!("{}.png", stem)))
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

fn quantize_image(img: &RgbaImage, config: &Config) -> Result<RgbaImage> {
    if config.k_colors == 0 {
        return Err(PixelSnapperError::InvalidInput(
            "Number of colors must be greater than 0".to_string(),
        ));
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

fn resample_cells(img: &RgbaImage, cols: &[usize], rows: &[usize], config: &Config) -> Result<RgbaImage> {
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
                    }
                }
            }

            let idx = y_i * out_w + x_i;
            if sum_a > 0.0 {
                let mean_lin = [sum_lin[0] / sum_a, sum_lin[1] / sum_a, sum_lin[2] / sum_a];
                let space = match config.color_space {
                    1 => mean_lin,
                    _ => [
                        linear_255_to_srgb_u8(mean_lin[0]) as f32,
                        linear_255_to_srgb_u8(mean_lin[1]) as f32,
                        linear_255_to_srgb_u8(mean_lin[2]) as f32,
                    ],
                };
                cell_colors[idx] = space;
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

    let palette = kmeans_palette(
        &palette_input,
        config.k_colors.max(1),
        config.k_seed,
        config.max_kmeans_iterations,
    );

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
