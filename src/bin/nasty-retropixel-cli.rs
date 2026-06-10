use nasty_retropixel::{
    apply_named_preset, process_batch_with_reporter, process_file_with_debug_exports,
    recommend_variant_for_image_bytes, suggest_setup_for_image_bytes, BatchConfig, BatchEvent,
    DebugExportOptions,
};
use std::path::{Path, PathBuf};

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
            "Usage: nasty-retropixel-cli <input> <output> [k_colors] [--preset ai-sprite|strict-retro|tileset-cleanup|character-cleanup|icon-cleanup|ultra-cleanup|auto] [--pixel-size <n>] [--denoise off|box3] [--palette-source pixels|cells] [--palette-lock <palette.png>] [--palette-cleanup off|basic|strict] [--cell-color mean|dominant|medoid] [--dither off|fs] [--color-space srgb|linear] [--cleanup off|basic] [--repair off|basic|smart|ultra] [--recommend-variant] [--debug-json] [--debug-overlay] [--debug-dir <path>]\n\nNotes:\n- Use a file input + file output for single image.\n- Use a directory input + directory output for batch processing.".to_string(),
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
    let mut debug_dir: Option<PathBuf> = None;
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
    let debug = DebugExportOptions {
        write_json: debug_json,
        write_overlay: debug_overlay,
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
        let batch = BatchConfig {
            input_dir: PathBuf::from(input),
            output_dir: PathBuf::from(output),
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

        return process_batch_with_reporter(&batch, |event| match event {
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
        });
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
    if debug.write_json || debug.write_overlay {
        let debug_root = debug
            .output_dir
            .clone()
            .unwrap_or_else(|| actual_output_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf());
        println!("Debug artifacts: {}", debug_root.display());
    }
    Ok(())
}
