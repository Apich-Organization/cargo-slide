# `slide-editor`

**Typora-style WYSIWYG live-preview native desktop editor for Typst presentations and documents.**

Part of the [Cargo Slide](https://github.com/Apich-Organization/cargo-slide) presentation suite.

---

## 🌟 Highlights

- **Typora-Style Live Preview (所见即所得实时预览)**:
  - Not just a clumsy dual-pane split view.
  - The document flows naturally as rich, rendered vector pages/slides.
  - **In-Place Slide Editing**: Click "✏️ In-Place Edit" on any slide to edit its Typst markup directly inside the document stream, with instant debounced live recompilation and visual updates.
- **Pure Native Rust GUI**:
  - Built with [iced](https://github.com/iced-rs/iced) (native desktop GUI toolkit).
  - No Electron, no Tauri, no WebView, no Chromium overhead.
  - Software & vector accelerated rendering via `tiny-skia` and `resvg` for high reliability across Linux (X11 & Wayland), macOS, and Windows.
- **Dual Format Support**:
  - **Typst Documents (`.typ`)**: Edit general Typst documents (articles, reports, notes) and presentations.
  - **Slide Packages (`.slide`)**: Open, inspect, edit, and save dedicated standalone presentation bundles compressed with LZMA2 extreme presets.
- **Multi-Format Export**:
  - 📄 **PDF**: Vector PDF generation via Typst compiler bridge.
  - 🖼️ **PNG**: High-resolution raster image rendering (1x, 2x 1080p, 3x 4K).
  - 📐 **SVG**: Clean vector SVG extraction for all slides.
  - 📦 **.slide Bundle**: Standalone presentation archive with embedded assets.
- **Modern Typora Aesthetics**:
  - **☀️ Day Mode (Light Theme)**: Typora Classic / GitHub Light palette (`#ffffff`, `#f1f5f9`, `#2673f2`).
  - **🌙 Night Mode (Dark Theme)**: Typora Night / GitHub Dark palette (`#0f141c`, `#171c26`, `#4099ff`).
  - Collapsible Outline sidebar with slide jumping.
  - Quick formatting bar for 1-click headings, math formulas, code blocks, tables, and slide pagebreaks.
  - Non-intrusive compiler diagnostic inspector with line/column pointers.

---

## 🖥️ Three View Modes

| Mode | Description |
|---|---|
| **👁️ Live Preview** | Flagship Typora-style visual canvas showing live vector slides. Clicking a slide allows inline in-place markup editing right within the document flow. |
| **🎯 Focus Mode** | Focused view dedicated to the active slide, featuring an integrated synchronized editor alongside real-time live SVG preview. |
| **📝 Source Code** | Full-document Typst editor (Typora's classic `Ctrl+/` mode) with line counts and syntax insertion tools. |

---

## 🚀 Getting Started

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

## 🧪 Testing
```bash
cargo test -p slide-editor
```
All integration tests cover document creation, `.typ` reading/writing, `.slide` LZMA2 archive unpacking/repacking, debounced compiler diagnostics, and PDF/PNG/SVG exports.
