# `slide-editor`

**Typora-style WYSIWYG live-preview native desktop editor for Typst presentations and documents.**

Part of the [Cargo Slide](https://github.com/Apich-Organization/cargo-slide) presentation suite.

---

## Highlights

- **Typora-Style Live Preview (所见即所得实时预览)**:
  - Document flows naturally as rendered vector pages.
  - **In-Place Slide Editing**: Click any slide to edit its Typst markup directly inside the visual canvas, with debounced live recompilation and visual updates.
- **Pure Native Rust GUI**:
  - Built with [iced](https://github.com/iced-rs/iced) (native desktop GUI toolkit).
  - No Electron, no Tauri, no WebView, no Chromium runtime dependencies.
  - Software & vector accelerated rendering via `tiny-skia` and `resvg` across Linux (X11 & Wayland), macOS, and Windows.
- **Dual Format Support**:
  - **Typst Documents (`.typ`)**: Edit general Typst documents (articles, reports, notes) and presentations.
  - **Slide Packages (`.slide`)**: Open, inspect, edit, and save dedicated standalone presentation bundles compressed with LZMA2.
- **Multi-Format Export**:
  - **PDF**: Vector PDF generation via Typst compiler bridge.
  - **PNG**: High-resolution raster image rendering (1x, 2x 1080p, 3x 4K).
  - **SVG**: Clean vector SVG extraction for all slides.
  - **.slide Bundle**: Standalone presentation archive with embedded assets.
- **Clean Desktop Interface**:
  - **Day Mode (Light Theme)**: Typora Classic / GitHub Light palette (`#ffffff`, `#f1f5f9`, `#2673f2`).
  - **Night Mode (Dark Theme)**: Typora Night / GitHub Dark palette (`#0f141c`, `#171c26`, `#4099ff`).
  - Collapsible Outline sidebar with thumbnail preview and slide jumping.
  - Quick insertion bar for headings, math formulas, code blocks, tables, speaker notes, and comments.
  - Non-intrusive compiler diagnostic inspector with line/column pointers.

---

## Three View Modes

| Mode | Description |
|---|---|
| **Live Preview** | Continuous visual canvas showing live vector slides. Clicking a slide allows inline in-place markup editing directly within the document flow. |
| **Focus Mode** | Focused view dedicated to the active slide, featuring an integrated synchronized editor alongside real-time live SVG preview. |
| **Source Code** | Full-document Typst editor with line counts and syntax insertion tools. |

---

## Getting Started

### Launching Standalone
```bash
# Launch with a new presentation
cargo run -p slide-editor

# Open a specific .typ file or .slide package
cargo run -p slide-editor -- path/to/slides.typ
cargo run -p slide-editor -- path/to/deck.slide

# Launch directly in Dark Mode
cargo run -p slide-editor -- path/to/slides.typ --dark
```

### Launching via Cargo Slide CLI
```bash
cargo slide edit [path/to/slides.typ] [--dark]
```

---

## Testing
```bash
cargo test -p slide-editor
```
All integration tests cover document creation, `.typ` reading/writing, `.slide` LZMA2 archive unpacking/repacking, debounced compiler diagnostics, and PDF/PNG/SVG exports.
