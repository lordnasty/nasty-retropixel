# Nasty RetroPixel

An AI pixel-art cleanup and grid recovery tool based on the Sprite Fusion Pixel Snapper algorithm.

<img src="./static/hero.png" alt="Pixel Snapper" style="width: 100%; image-rendering: pixelated;">

## Why

**Current AI image models can't reliably preserve grid-based pixel art.**

- Pixels are inconsistent in size and position.
- The grid resolution can drift over time.
- Colors are not tied to a strict palette.
- Noise, soft blending and mixed edges break readability.
- A "standard" snap is often not enough when the source image is already degraded.

**Nasty RetroPixel goes further than a standard snap:**

- Grid recovery stabilizes inconsistent pixel spacing.
- Palette quantization rebuilds a controlled color set.
- Cell-based resampling preserves cleaner tiles and sprites.
- Optional denoise helps recover noisy AI outputs before snapping.
- Linear/sRGB handling and optional dithering give more control over the final look.
- Batch processing makes the workflow practical on full asset packs.

## Core Goals

- Recover a readable pixel grid from AI-generated or degraded source images.
- Produce output that is more consistent than a simple resize or nearest-neighbor pass.
- Rebuild a palette-aware result suitable for games, spritesheets and tilesets.
- Keep enough detail to remain faithful to the source while making it production-friendly.

## Perfect for

- **AI generated pixel art** that needs cleanup before it is usable.
- **Spritesheets and tilesets** that need a stable grid and palette.
- **Procedural 2D art that doesn't fit a grid** like tilemaps or isometric maps.
- **2D game assets and 3D textures** that need to scale cleanly.

<img src="./static/details.png" alt="Details" style="width: 100%; image-rendering: pixelated;">

<p align="center"><em>Nasty RetroPixel preserves as much useful detail as possible while rebuilding grid consistency.</em></p>

<br>

## Build from source

Requires [Rust](https://www.rust-lang.org/) installed on your machine.

### 💻 CLI

```bash
git clone https://github.com/Hugo-Dz/spritefusion-pixel-snapper.git
cd spritefusion-pixel-snapper
```

```bash
cargo run --bin nasty-retropixel-cli -- input.png output.png
```

The command accepts an optional k-colors argument:

```bash
cargo run --bin nasty-retropixel-cli -- input.png output.png 16
```

Use a directory as the input path to process a batch:

```bash
cargo run -- input_dir output_dir 16
```

Useful CLI flags:

```bash
cargo run --bin nasty-retropixel-cli -- input.png output.png 16 --pixel-size 8
cargo run --bin nasty-retropixel-cli -- input.png output.png 16 --denoise box3 --palette-source cells
cargo run --bin nasty-retropixel-cli -- input.png output.png 16 --dither fs --color-space linear
```

Available options:

- `--pixel-size <n>`: override auto-detected pixel size
- `--denoise off|box3`: apply prefiltering before grid recovery
- `--palette-source pixels|cells`: choose how the output palette is reconstructed
- `--dither off|fs`: disable or enable Floyd-Steinberg dithering
- `--color-space srgb|linear`: choose the color space used during quantization

Batch processing is available when the input path is a directory and the output path is a different directory.

### 🌐 Web (WASM)

```bash
git clone https://github.com/Hugo-Dz/spritefusion-pixel-snapper.git
cd spritefusion-pixel-snapper
```

Build and run the included web UI:

```bash
./scripts/dev-web.ps1
```

Then open `http://localhost:8080/`.

The web UI includes:

- single image processing
- batch mode
- compare slider
- grid/debug overlay
- advanced palette / denoise / dithering controls
- offline ZIP download for multi-file output

If you only want to rebuild the WASM package:

```bash
./scripts/build-web.ps1
```

Use the generated WASM module in your own project:

```js
import init, { process_image, process_image_debug } from "./pkg/nasty_retropixel.js";

await init();

// process_image(inputBytes, kColors?, pixelSizeOverride?, prefilterMode?, paletteSource?, ditherMode?, colorSpace?)
const outputBytes = process_image(inputBytes, 16);
```

Pass `null` for any optional argument you want to leave on its default behavior.

## Acknowledgments

This project is based on the original [Sprite Fusion Pixel Snapper](https://spritefusion.com/pixel-snapper) idea and extends it with a custom CLI workflow, a local web GUI and extra restoration controls.

<img src="./static/spritefusion.webp" alt="Sprite Fusion" style="width: 100%;">

## License

MIT License [Hugo Duprez](https://www.hugoduprez.com/)
