use nasty_retropixel::{process_batch_with_reporter, process_image_bytes_with_config, BatchConfig, BatchEvent};
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
            "Usage: nasty-retropixel-cli <input> <output> [k_colors] [--pixel-size <n>] [--denoise off|box3] [--palette-source pixels|cells] [--dither off|fs] [--color-space srgb|linear]\n\nNotes:\n- Use a file input + file output for single image.\n- Use a directory input + directory output for batch processing.".to_string(),
        ));
    }

    let input_path = args[1].clone();
    let output_path = args[2].clone();

    let mut k_colors: Option<usize> = None;
    let mut pixel_size_override: Option<f64> = None;
    let mut denoise: Option<u32> = None;
    let mut palette_source: Option<u32> = None;
    let mut dither: Option<u32> = None;
    let mut color_space: Option<u32> = None;

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
    if let Some(v) = dither {
        config.dither_mode = v;
    }
    if let Some(v) = color_space {
        config.color_space = v;
    }

    let input = Path::new(&input_path);
    let output = Path::new(&output_path);

    if input.is_dir() {
        let batch = BatchConfig {
            input_dir: PathBuf::from(input),
            output_dir: PathBuf::from(output),
            k_colors: config.k_colors,
            pixel_size_override: config.pixel_size_override,
            prefilter_mode: config.prefilter_mode,
            palette_source: config.palette_source,
            dither_mode: config.dither_mode,
            color_space: config.color_space,
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

    let img_bytes = std::fs::read(&input_path).map_err(|e| {
        nasty_retropixel::PixelSnapperError::ProcessingError(format!(
            "Failed to read input file: {}",
            e
        ))
    })?;

    let output_bytes = process_image_bytes_with_config(&img_bytes, config)?;

    std::fs::write(&actual_output_path, output_bytes).map_err(|e| {
        nasty_retropixel::PixelSnapperError::ProcessingError(format!(
            "Failed to write output file: {}",
            e
        ))
    })?;

    println!("Saved to: {}", actual_output_path.display());
    Ok(())
}
