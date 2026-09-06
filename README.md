# cargo-slide

> **Code-driven presentation engine combining Typst typesetting with a native Rust viewer**  
> *Native 60 FPS Software Rendering · Standalone Single-Binary Output · Hardware-Accelerated Media · Interactive SQL Charts · Extensible Animation Traits*

[English](README.md) | [简体中文](README_zh.md)

---

## Overview

`cargo-slide` is a command-line tool and runtime for creating, presenting, and distributing presentations using code. It bridges two complementary technologies:

- **[Typst](https://typst.app/) for typesetting**: Fast compilation, clean markup syntax, first-class mathematical equation typesetting, and modular macros compiled directly to high-precision vector graphics (SVG).
- **Rust for the native presentation viewer**: A standalone runtime built on [`tiny-skia`](https://github.com/RazrFalcon/tiny-skia) that renders slides at a steady 60 FPS without web browser or Electron dependencies, featuring presenter tools, audio DSP mixing, interactive charts, and single-binary packaging.

---

## Key Features

- **Typst-Based Authoring**: Presentations are authored entirely in Typst markup (`slides.typ`). Rust is only required when implementing custom low-level transition or fragment animation traits.
- **Minimal Project Scaffolding**: `cargo slide init` creates a standard 5-file project structure without unnecessary boilerplate.
- **Standalone Single-Binary Distribution**: `cargo slide build` packages presentation content, vectorized slide assets, layout metadata, and the runtime player into a single portable binary (~14–15 MB). Target machines run the binary directly without requiring Typst, Rust, Node.js, or browser engines.
- **Cross-Platform Window & Fullscreen Modes**: Seamlessly toggle between native fullscreen (`F11` / `F` / Dock `FULL`) and resizable windowed mode across Linux (EWMH), Windows, and macOS. Outer letterbox margins automatically sample and extend the slide background color to prevent visual seams on non-standard displays. Native aspect ratio presets include `16-9`, `16-10`, `3-2`, and `4-3`.
- **13 Built-in Slide Transitions**: `fade`, `cut`, `slide-left`, `slide-right`, `slide-up`, `slide-down`, `zoom`, `wipe-left`, `wipe-right`, `iris`, `glitch`, `cube`, and `particles`.
- **In-Slide Fragment Steps**: Control sequential content reveals with `#step(order, effect: "...")` supporting `fade-in`, `slide-up`, `glitch`, and `typewriter`. Advance with `Space` / Left-Click and reverse with `Backspace` / Right-Click.
- **Interactive Data Charts & HUD Data Inspector**:
  - Read tabular data directly from `.csv`, `.json`, or `.db` (SQLite) files.
  - Execute in-memory SQL (`SELECT ... FROM data WHERE ...`) or pipeline DSL queries (`source -> filter() -> select()`).
  - Hover crosshairs with multi-series indicators during presentation.
  - Interactive HUD Data Inspector: click any chart or table during presentation to switch chart types (Bar, Line, Area, Scatter), apply transform presets (`TOP 5`, `SORT ▼`, `SORT ▲`, `CUM`, `% SHARE`, `MA3`), filter rows by text or condition (`>50`, `<=100`), inspect tabular data, sort columns, or export to CSV.
- **Presenter Tools**:
  - **Laser Pointer**: Smooth on-screen pointer with a simulated decay trail (`L` key or Dock `LSR`).
  - **Whiteboard Pen & Palette**: Freehand slide annotation (`P` key) with a 7-color palette (`K` key, numbers `1`–`7`), smooth Bézier interpolation, and per-slide persistence (`C` / `X` to clear).
  - **Audio Engine**: Background music playback with looping, track mixing, on-screen toasts, and volume adjustments (`+`, `-`, mouse scroll wheel, interactive Dock slider, `M` to mute) safely clamped to [0%, 100%].
  - **Video Playback**: Clickable video placeholders delegate playback to external media players (`ffplay`, `mpv`, or default system player) in an independent background window with auto-exit.
- **Slide Overflow Protection**: Checks compiled vector pages against declared `#slide(...)` declarations during compilation. If content exceeds vertical canvas bounds, compilation reports the specific slide title and source line number.
- **Vectorized Typography & Multilingual Support**: Typst resolves text (Latin, CJK Chinese/Japanese/Korean, math equations, and Unicode emojis) into vector Bézier `<path>` outlines. Presentations render identically across systems without requiring font installations. The HUD Data Inspector includes a built-in bitmap font with glyphs for currencies, arrows, sorting indicators, and presentation symbols.
- **Accessibility & Touch Support**: Full keyboard navigation across all features; floating bottom dock for mouse or touch screen operation; high-contrast tabular data view in the inspector.
- **Structured Output**: Human-readable terminal output by default, or single-line JSON (`--log-format json` or `CARGO_SLIDE_LOG_FORMAT=json`) for automated CI/CD scripts.

---

## Getting Started

### Prerequisites

1. **Rust Toolchain**: Rust 1.80+ (recommended: latest stable). Install via [rustup.rs](https://rustup.rs/).
2. **Typst CLI**: `typst` executable available in your `PATH`. Install via package manager (`cargo install --locked typst-cli`, `brew install typst`, or distro packages).
3. *(Optional)* **External Media Player**: `mpv` or `ffplay` for video playback cards.

### Step 1: Install `cargo-slide`

Install from local source:
```bash
cargo install --path crates/cargo-slide
```

Verify installation:
```bash
cargo slide --help
```

### Step 2: Initialize a New Presentation

Navigate to where you want your presentation, and run:
```bash
# Initialize in current directory
cargo slide init

# Or initialize in a new directory
cargo slide init my-talk
cd my-talk
```

The generated project contains exactly 5 files:
```text
my-talk/
├── slides.typ          # Main presentation content in Typst markup
├── theme.typ           # Theme styling, color palette, and canvas aspect ratio
├── slide.typ           # Built-in component macros (#slide, #step, #chart, etc.)
├── assets/
│   └── data.csv        # Sample dataset for interactive charts
└── .gitignore          # Excludes build caches, PDFs, and compiled binaries
```

*(Note: If you wish to implement custom Rust animation traits, add the `--rust` flag: `cargo slide init my-talk --rust`. This will additionally generate `Cargo.toml` and `src/main.rs`.)*

### Step 3: Author Your Slides

Edit `slides.typ` with any text editor:
```typst
#import "theme.typ": *

#show: slide-theme.with(
  aspect-ratio: "16-9", // "16-9", "16-10" (MacBook / Dell XPS / ThinkPad), "3-2" (Surface / Framework), or "4-3"
  theme: "dark"
)

#title-slide(
  title: "Building Modern Systems",
  subtitle: "Native Code-Driven Presentation",
  author: "Jane Doe",
  date: "2026",
)

#slide(title: "Architecture & Core Concepts", transition: "fade")[
  #cols(
    [
      === High-Level Overview
      - Pure Typst functional markup
      - Native 60 FPS vector renderer
      - Single-binary distribution
    ],
    [
      === Key Equation
      $ cal(H) |psi(t) chevron.r = i ħ dif / (dif t) |psi(t) chevron.r $

      #v(0.3cm)
      #badge("Quantum Engine", fill: slide-colors.accent)
    ]
  )
]
```

### Step 4: Run the Presentation

Launch the native 60 FPS presentation viewer:
```bash
cargo slide run
```

For rapid development, start live hot-reloading:
```bash
cargo slide dev
```
Whenever you edit and save `slides.typ`, the player re-compiles and updates the view while keeping your current slide position.

### Step 5: Build a Standalone Executable

When you are ready to distribute your presentation:
```bash
cargo slide build
```
This produces an executable (`slides-presentation` or `slides-presentation.exe`). Copy this single file to any computer and run it directly—no Typst, Rust, or browser required.

### Step 6: Export to PDF or SVG

Export static documents for handouts or archiving:
```bash
# Export to PDF
cargo slide export slides.typ --format pdf -o presentation.pdf

# Export each slide as an SVG vector image
cargo slide export slides.typ --format svg -o exported-svgs/
```

---

## Frequently Asked Questions (FAQ)

### Q1: Is Rust knowledge required to use `cargo-slide`?
**No.** Ordinary presentations are authored 100% in Typst markup (`slides.typ`). You never need to touch Rust unless you want to write custom raster shaders or physics-based transition algorithms via Rust traits.

### Q2: What files are generated by `cargo slide init`?
In standard mode, exactly 5 files are created:
1. `slides.typ`: Your slide deck content.
2. `theme.typ`: Dimensions (16:9), color schemes, and typographical tokens.
3. `slide.typ`: Reusable layout and widget macros (`#slide`, `#step`, `#chart`, `#video`, `#audio`, `#callout`).
4. `assets/data.csv`: A sample tabular dataset.
5. `.gitignore`: Ignores `.build_tmp/`, `*.cache.csv`, `target/`, and output binaries.

If `--rust` is supplied, `Cargo.toml` and `src/main.rs` are added for trait extensions.

### Q3: What assets are bundled into the binary vs carried in `assets/`?
- **Embedded inside the binary (~14–15 MB standalone executable)**:
  - All slide vector paths, layout geometry, glyph curves, and LaTeX math formulas.
  - Interactive hotspot definitions, fragment steps, and transition metadata.
  - Processed CSV, JSON, and SQLite chart tables.
  - The `tiny-skia` software rendering engine, transition routines, laser pointer, whiteboard pen, audio mixer, and HUD Data Inspector.
- **Carried alongside in `assets/`**:
  - Video files (`.mp4`, `.webm`, `.mkv`) and external audio recordings.
  - *Rationale*: Storing large video files directly inside the binary executable creates excessive binary bloat and unnecessary memory footprint. By referencing media via relative paths, `cargo-slide` keeps the binary compact while delegating video decoding to dedicated media players.

### Q4: How does video playback work?
Typst generates a styled video placeholder card with an embedded hotspot bounding box. During presentation, clicking the video card launches an external video player (`ffplay`, `mpv`, or the operating system's default media player) in a detached background thread. The video opens in an independent native window with standard controls and auto-exits on completion, ensuring smooth playback without blocking the slide presentation event loop.

### Q5: How do custom fonts and cross-platform font rendering work?
- **Vectorized Glyph Output**: Typst vectorizes text glyphs into SVG `<path>` outlines during compilation. The presentation player rasterizes these mathematical Bézier curves directly through `tiny-skia`, ensuring that presentations look identical on all machines without requiring target systems to have fonts installed.
- **Custom Fonts**: Authors can customize fonts in three flexible ways:
  1. **Theme configuration**: Pass `font: "Inter"` (or an array `font: ("Inter", "PingFang SC")`) and `code-font: "Fira Code"` directly into `#show: slide-theme.with(...)`.
  2. **Typst primitives**: Use `#set text(font: "...")` anywhere in `slides.typ`.
  3. **Bundling local font files**: Drop `.ttf` or `.otf` font files into a `fonts/` or `assets/fonts/` directory in your presentation project. `cargo-slide` automatically detects them and passes `--font-path` to Typst. You can also specify the `TYPST_FONT_PATHS` environment variable.
- **Missing Font Warnings**: If a requested font family cannot be found on your system or in bundled font directories, Typst emits an `unknown font family` warning. `cargo-slide` catches this warning and displays an actionable notice with bundling tips while safely falling back to available system fonts.

### Q6: How does slide overflow protection work?
In Typst, when content exceeds the 16:9 vertical page height (15.75 cm), Typst inserts an implicit pagebreak, creating an orphan spillover page that disrupts slide numbering.  
`cargo-slide` inspects the compiled page count against the declared `#slide` / `#title-slide` count. If a discrepancy is detected, it pinpoints the offending slide title and line number:
```text
Typst slide content overflow detected!
The presentation source declares 17 slide(s), but Typst compiled 18 pages (1 extra spillover page(s)).

Overflow location:
  Slide 4 ("Mathematical Typography & Code Syntax Highlighting", line 64) exceeded the vertical 16:9 canvas bounds.
  Spillover content pushed onto compiled page 5.
```
Authors can resolve the issue by adjusting heights, spacing, or setting `CARGO_SLIDE_ALLOW_OVERFLOW=1` to bypass.

### Q7: How does `cargo-slide` compare to Marp, Slidev, and Beamer?

| Feature | `cargo-slide` | Marp / Slidev | LaTeX Beamer |
| :--- | :--- | :--- | :--- |
| **Engine** | Native Rust (`tiny-skia`) | Web / Electron / Node.js | TeX / PDF reader |
| **Output** | Standalone ~12 MB binary | HTML bundle / PDF / App | Static PDF |
| **Typography** | Typst native vector paths | Browser CSS / Web fonts | LaTeX native fonts |
| **Math Quality** | LaTeX-grade equations | KaTeX / MathJax | Native TeX math |
| **Frame Rate** | Locked 60 FPS | Variable (DOM/browser engine) | Static (no transitions) |
| **Presenter Tools** | Laser trail, pen, palette, audio mixer | Limited browser plugins | PDF reader dependent |
| **External Dependencies** | None for output binary | Browser or Node runtime | PDF viewer |
| **Trade-offs** | Requires Typst CLI for editing | Large bundle, browser inconsistencies | Slow compile, steep syntax |

### Q8: Can `cargo-slide` be used in headless CI/CD pipelines?
Yes. Both `cargo slide build` and `cargo slide export` can run in headless environments. Use `--log-format json` to receive structured machine-readable events for build status, slide counts, and warnings.

---

## CLI Command Reference

Global option: `--log-format <human|json>` (default: `human`) or environment variable `CARGO_SLIDE_LOG_FORMAT=json`.

### `cargo slide init`
```bash
cargo slide init [PATH] [--rust] [--log-format <human|json>]
```
Initialize a presentation project in the specified path (defaults to current directory `.`). Use `--rust` to include Cargo scaffolding for custom Rust animation traits.

### `cargo slide new`
```bash
cargo slide new <NAME> [--rust] [--log-format <human|json>]
```
Create a new directory `<NAME>` and initialize a presentation project inside it.

### `cargo slide run`
```bash
cargo slide run [FILE] [--animation <NAME>] [--fullscreen]
```
Launch the native presentation player. Defaults to `slides.typ` with the `fade` transition.

### `cargo slide dev`
```bash
cargo slide dev [FILE] [--animation <NAME>]
```
Launch the presentation player with file-system watching. Automatically re-compiles on save and refreshes the current view.

### `cargo slide build`
```bash
cargo slide build [FILE] [-o <OUTPUT>] [--animation <NAME>]
```
Compile the presentation into a standalone self-contained release binary.

### `cargo slide export`
```bash
cargo slide export [FILE] --format <pdf|svg> -o <OUTPUT>
```
Export slides to a multi-page PDF document or individual SVG vector image files.

---

## Presenter Controls & Shortcuts

| Key / Action | Function |
| :--- | :--- |
| **Space** / **Enter** / **Right** / **Down** / **Left Click** | Advance to next step (or next slide if steps completed) |
| **Backspace** / **Left** / **Up** / **Right Click** | Step back to previous step (or previous slide) |
| **PageDown** / **PageUp** | Advance / reverse whole slide directly |
| **Home** / **End** | Jump to first / last slide |
| **Digit(s) + Enter** | Jump directly to specific slide number |
| **F11** / **F** / Dock `FULL` | Toggle native fullscreen and windowed modes |
| **L** / Dock `LSR` | Toggle laser pointer with decay trail |
| **P** / Dock `PEN` | Toggle whiteboard annotation pen |
| **K** / Dock `COL` | Toggle 7-color floating palette |
| **Keys 1 .. 7** | Select pen & laser color (Cyan, Red, Green, Yellow, Purple, White, Orange) |
| **C** / **X** / Dock `CLR` | Clear annotations on current slide |
| **Mouse Wheel** / **`+` / `-`** | Adjust master audio volume (0% to 100%) |
| **Dock Volume Slider** | Click or drag directly on the slider track to adjust volume |
| **M** / Dock `VOL` | Mute / unmute audio |
| **H** / **?** / Dock `HELP` | Toggle shortcut help overlay |
| **Click on Chart / Table** | Open interactive HUD Data Inspector modal |
| **Click on Video Card** | Launch external video player in detached window |
| **Click on Hyperlink** | Open external URL or local file in background |
| **Esc** / **Q** | Close active modal / exit presentation |

---

## Interactive Charts & HUD Data Inspector

### 1. CSV Data Source
```typst
#chart(
  data: "assets/data.csv",
  type: "bar",
  title: "Runtime Memory Benchmarks",
  width: 100%,
  height: 6cm
)
```

### 2. In-Memory SQL Queries
Query CSV, JSON, or SQLite datasets using standard SQL:
```typst
#chart(
  data: "assets/telemetry.db",
  sql: "SELECT service, p99_latency FROM traces WHERE p99_latency > 50 ORDER BY p99_latency DESC",
  type: "bar",
  title: "High-Latency Microservices"
)
```

### 3. Pipeline DSL
For quick transformations without full SQL syntax:
```typst
#chart(
  data: "assets/metrics.json",
  dsl: "source -> filter(fps >= 30) -> select(Framework, FPS)",
  type: "line",
  title: "Frame Rate Comparison"
)
```

### 4. Interactive HUD Data Inspector
Clicking any chart or data card during presentation opens the HUD Data Inspector modal:
- **Type Switcher**: Dynamically switch visualizations between Bar, Line, Area, and Scatter representations.
- **Transform Presets**: Apply instant calculations (`ORIG`, `TOP 5`, `SORT ▼` desc, `SORT ▲` asc, `CUM` cumulative, `% SHARE` 100% share, `MA3` 3-period moving average).
- **Series Chips**: Toggle individual series on or off to isolate specific metrics.
- **KPI Summary Cards**: Real-time stats strip showing Total Sum, Mean/Avg, Minimum, Maximum, Median, and Standard Deviation across visible data points.
- **Tabular Data View**: Inspect raw data records; click table headers (`Category` or series names) to sort; scroll vertically via scroll wheel or drag.
- **Search & Condition Filtering**: Filter rows by text query or numeric condition expressions (e.g. `>50`, `<=100`, `!=0`).
- **Number Formatting**: Cycle display formats (`ORIG`, `INT`, `DEC1`, `DEC2`, `CURR`, `PCT`).
- **CSV Export**: Click `EXPORT CSV` to save the active filtered data to a timestamped CSV file on disk.
- **Dismiss**: Press `Esc` or click `CLOSE [X]` to return to the presentation.

---

## Extending with Custom Rust Traits

For developers wishing to implement custom rendering algorithms:

```rust
use slide_core::animation::{RenderContext, SlideAnimation, SlideSurface};
use slide_player::SlideApp;
use std::time::Duration;

/// Custom vertical curtain split transition
pub struct CurtainSplitTransition;

impl SlideAnimation for CurtainSplitTransition {
    fn name(&self) -> &str {
        "curtain-split"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(500)
    }

    fn render(
        &self,
        ctx: &mut RenderContext,
        from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    ) {
        let t = progress.clamp(0.0, 1.0);
        let (w, h) = (ctx.width, ctx.height);
        let split_y = (h as f32 * t * 0.5) as usize;

        for y in 0..h {
            let row = y * w;
            for x in 0..w {
                let pixel = if y < split_y || y >= (h - split_y) {
                    to.get_pixel(x, y)
                } else if let Some(f) = from {
                    f.get_pixel(x, y)
                } else {
                    0
                };
                ctx.buffer[row + x] = pixel;
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    SlideApp::new("slides.typ")
        .default_animation("curtain-split")
        .register_animation(CurtainSplitTransition)
        .run()?;
    Ok(())
}
```

---

## Project Structure

```text
cargo-slide/
├── Cargo.toml                      # Workspace configuration
├── crates/
│   ├── slide-core/                 # Core data models, Typst compiler bridge, SVG parser, charts/SQL, traits, logging
│   ├── slide-player/               # Native player: tiny-skia blitter, windowing, audio mixer, presenter tools, HUD
│   ├── slide-theme/                # Built-in themes, default templates, and Typst macros (#slide, #step, #chart)
│   └── cargo-slide/                # CLI tool implementing init, new, run, dev, build, export
└── examples/
    └── geek-presentation/          # 17-slide reference presentation demonstrating all features
```

---

## License

This project is licensed under the [GNU Affero General Public License v3.0 or later](LICENSE) (`AGPL-3.0-or-later`).

