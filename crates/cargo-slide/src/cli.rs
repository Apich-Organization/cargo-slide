use clap::Parser;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "cargo-slide",
    bin_name = "cargo-slide",
    version,
    about = "Modern code-driven presentation system in Rust + Typst"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to typst file when run directly (default: slides.typ)
    #[arg(global = true)]
    pub file: Option<PathBuf>,

    /// Log format: human (default) or json
    #[arg(long, global = true, default_value = "human")]
    pub log_format: String,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new presentation project in current or specified directory
    Init {
        /// Target directory path (default: current directory ".")
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Include Rust Cargo package with main.rs for custom trait extensions
        #[arg(long)]
        rust: bool,

        /// Starter presentation template (minimal, geek, academic, pitch, business)
        #[arg(short, long, default_value = "minimal")]
        template: String,

        /// Slide aspect ratio: 16:9 (default), 4:3, 16:10
        #[arg(long, default_value = "16:9")]
        aspect: String,
    },
    /// Create a new presentation project directory
    New {
        /// Name of the presentation project
        name: String,

        /// Include Rust Cargo package with main.rs for custom trait extensions
        #[arg(long)]
        rust: bool,

        /// Starter presentation template (minimal, geek, academic, pitch, business)
        #[arg(short, long, default_value = "minimal")]
        template: String,

        /// Slide aspect ratio: 16:9 (default), 4:3, 16:10
        #[arg(long, default_value = "16:9")]
        aspect: String,
    },
    /// Run presentation with interactive GUI player
    Run {
        /// Path to .typ slide file (default: slides.typ)
        #[arg(default_value = "slides.typ")]
        file: PathBuf,

        /// Default transition animation (fade, cut, slide-left, slide-right, particles, zoom)
        #[arg(short, long, default_value = "fade")]
        animation: String,

        /// Watch file for changes and re-render
        #[arg(short, long)]
        watch: bool,

        /// Start directly in fullscreen mode
        #[arg(long)]
        fullscreen: bool,

        /// Target talk duration in minutes for pacing alert clock
        #[arg(short = 't', long)]
        target: Option<u64>,

        /// Kiosk unattended loop interval in seconds (e.g. --kiosk 10)
        #[arg(long)]
        kiosk: Option<u64>,

        /// Start presentation player with light HUD theme
        #[arg(long)]
        light: bool,
    },
    /// Alias for run --watch
    Dev {
        /// Path to .typ slide file (default: slides.typ)
        #[arg(default_value = "slides.typ")]
        file: PathBuf,

        /// Default transition animation
        #[arg(short, long, default_value = "fade")]
        animation: String,

        /// Start directly in fullscreen mode
        #[arg(long)]
        fullscreen: bool,

        /// Target talk duration in minutes for pacing alert clock
        #[arg(short = 't', long)]
        target: Option<u64>,

        /// Kiosk unattended loop interval in seconds (e.g. --kiosk 10)
        #[arg(long)]
        kiosk: Option<u64>,

        /// Start presentation player with light HUD theme
        #[arg(long)]
        light: bool,
    },
    /// Launch the Typora-style WYSIWYG desktop editor for presentations and documents
    Edit {
        /// Path to .typ slide file or .slide package (optional)
        file: Option<PathBuf>,

        /// Start directly in dark mode
        #[arg(long)]
        dark: bool,
    },
    /// Build presentation into a standalone executable, a .slide package, or a WASM CSR web bundle
    Build {
        /// Path to .typ slide file (default: slides.typ)
        #[arg(default_value = "slides.typ")]
        file: PathBuf,

        /// Output path (default: `<filename>-presentation`, `<filename>.slide`, or `dist/`)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format: binary (default standalone executable), slide (.slide package), wasm (Leptos CSR web bundle)
        #[arg(short, long, default_value = "binary")]
        format: String,

        /// Default transition animation
        #[arg(short, long, default_value = "fade")]
        animation: String,

        /// Cross-compile for a different Rust target triple (e.g.
        /// `x86_64-pc-windows-msvc`, `aarch64-unknown-linux-musl`) instead of the host's own
        /// platform. The target must already be installed (`rustup target add <triple>`) and,
        /// for a non-host target, have a working linker configured (see `.cargo/config.toml`).
        #[arg(long)]
        target: Option<String>,

        /// Include original editable Typst source files and templates in the package
        #[arg(long, alias = "origin")]
        source: bool,
    },
    /// Pack presentation into a standalone compressed .slide package (LZMA2)
    Pack {
        /// Path to .typ slide file (default: slides.typ)
        #[arg(default_value = "slides.typ")]
        file: PathBuf,

        /// Output .slide package file path (default: `<filename>.slide`)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Default transition animation
        #[arg(short, long, default_value = "fade")]
        animation: String,

        /// Include original editable Typst source files and templates in the package
        #[arg(long, alias = "origin")]
        source: bool,
    },
    /// Unpack a .slide package into an editable project directory
    Unpack {
        /// Path to .slide package file
        file: PathBuf,

        /// Output directory to unpack the presentation files (default: directory named after package)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Serve presentation via built-in static web server
    Serve {
        /// Path to .typ slide file, .slide package, or CSR directory (default: slides.typ)
        #[arg(default_value = "slides.typ")]
        file: PathBuf,

        /// Port to bind the static server to (default: 8080)
        #[arg(short, long, default_value_t = 8080)]
        port: u16,

        /// IP address to bind the static server to (default: 127.0.0.1)
        #[arg(long, default_value = "127.0.0.1")]
        ip: String,

        /// Optional directory of an already built Leptos CSR package to serve directly
        #[arg(short, long)]
        dir: Option<PathBuf>,

        /// Open presentation in default web browser
        #[arg(long)]
        open: bool,

        /// Watch source Typst file and assets, hot-recompiling bundle on change
        #[arg(short, long)]
        watch: bool,
    },
    /// Inspect presentation package or Typst file metadata, slide count, and integrity
    Info {
        /// Path to .slide package or .typ file (default: slides.typ)
        #[arg(default_value = "slides.typ")]
        file: PathBuf,
    },
    /// Audit presentation health, element density, slide titles, and media assets
    Check {
        /// Path to .typ slide file (default: slides.typ)
        #[arg(default_value = "slides.typ")]
        file: PathBuf,

        /// Target talk duration in minutes to verify deck pacing against
        #[arg(short = 't', long)]
        target: Option<u64>,

        /// Output report in JSON format
        #[arg(long)]
        json: bool,
    },
    /// Calculate comprehensive slide deck statistics, speaking duration, and notes coverage
    Stats {
        /// Path to .typ slide file or .slide package (default: slides.typ)
        #[arg(default_value = "slides.typ")]
        file: PathBuf,

        /// Output stats in JSON format for automated analysis
        #[arg(long)]
        json: bool,

        /// Words per minute speaking pace calibration (default: 130)
        #[arg(long, default_value_t = 130)]
        wpm: u32,
    },
    /// Export presentation to PDF, SVG, PNG, .slide, or WASM CSR
    Export {
        /// Path to .typ slide file (default: slides.typ)
        #[arg(default_value = "slides.typ")]
        file: PathBuf,

        /// Export format (pdf, svg, png, slide, wasm)
        #[arg(short, long, default_value = "pdf")]
        format: String,

        /// Output file or directory path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Export selected slide pages or ranges (e.g. "1,3,5-8", 1-indexed)
        #[arg(short, long)]
        pages: Option<String>,

        /// Render resolution scale factor for PNG export (e.g. 1.0, 2.0 for Retina/4K, default: 2.0)
        #[arg(long, default_value_t = 2.0)]
        scale: f32,
    },
    /// Install universal slide-viewer player and desktop integration to system
    InstallViewer,
}
