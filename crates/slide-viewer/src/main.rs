//! Universal presentation player binary (`slide-viewer`) with a sleek interactive Iced GUI launcher.

use clap::Parser;
use slide_core::compiler::SlideCompiler;
use slide_core::model::SlideDeck;
use slide_core::package::read_package_metadata;
use slide_core::package::unpack_deck_and_assets;
use slide_core::package::unpack_deck_and_assets_from_bytes;
use slide_player::SlideApp;
use slide_player::hud::HudTheme;
use std::path::Path;
use std::path::PathBuf;

/// Embedded reference presentation package (`examples/geek-presentation`) with all assets (audio, charts, SVG)
const EMBEDDED_GEEK_DEMO: &[u8] = include_bytes!("../assets/geek_demo.slide");

pub mod gui;
mod installer;
pub mod theme;

use installer::install_viewer_to_system;

#[derive(Parser, Debug)]
#[command(
    name = "slide-viewer",
    bin_name = "slide-viewer",
    version,
    about = "Universal standalone presentation player and viewer for cargo-slide packages (.slide) and decks"
)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to .slide package file, .typ source file, or deck.json (optional)
    file: Option<PathBuf>,

    /// Default transition animation (fade, zoom, slide-left, slide-right, particles, cut)
    #[arg(short, long, default_value = "fade")]
    animation: String,

    /// Start directly in fullscreen mode
    #[arg(long)]
    fullscreen: bool,

    /// Start presentation player with light HUD theme
    #[arg(long)]
    light: bool,

    /// Run built-in Geek Demo showcase presentation
    #[arg(long)]
    demo: bool,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Install slide-viewer binary to system and register .slide file associations
    Install,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if let Some(Commands::Install) = args.command {
        match install_viewer_to_system() {
            | Ok(msg) => {
                println!("[OK] {msg}");
                return Ok(());
            },
            | Err(e) => {
                eprintln!("[ERR] Failed to install slide-viewer: {e}");
                std::process::exit(1);
            },
        }
    }

    let hud_theme = if args.light {
        HudTheme::Light
    } else {
        HudTheme::Dark
    };

    if args.demo {
        launch_builtin_demo(&args.animation, args.fullscreen, hud_theme)?;
        return Ok(());
    }

    if let Some(ref file_path) = args.file {
        if file_path.exists() {
            play_presentation_file(file_path, &args.animation, args.fullscreen, hud_theme)?;
            return Ok(());
        }
        eprintln!(
            "Error: Specified file does not exist: {}",
            file_path.display()
        );
    }

    // No file provided or file not found -> Launch the sleek Iced Viewer GUI Launcher
    run_launcher_gui(args.animation, args.fullscreen)?;

    Ok(())
}

/// Run the interactive Iced GUI launcher application
fn run_launcher_gui(
    animation: String,
    fullscreen: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    iced::application(
        move || gui::ViewerLauncherApp::new(animation.clone(), fullscreen),
        gui::ViewerLauncherApp::update,
        gui::ViewerLauncherApp::view,
    )
    .subscription(gui::ViewerLauncherApp::subscription)
    .title(|app: &gui::ViewerLauncherApp| format!("Slide Viewer - {}", app.window_title()))
    .theme(|app: &gui::ViewerLauncherApp| {
        match app.theme() {
            | theme::ViewerTheme::Light => iced::Theme::Light,
            | theme::ViewerTheme::Dark => iced::Theme::Dark,
        }
    })
    .window_size(iced::Size::new(1080.0, 720.0))
    .centered()
    .resizable(true)
    .run()?;

    Ok(())
}

/// Load and play any presentation format (.slide package, .typ source, or deck.json)
pub fn play_presentation_file(
    path: &Path,
    animation: &str,
    fullscreen: bool,
    hud_theme: HudTheme,
) -> Result<(), Box<dyn std::error::Error>> {
    save_recent_viewer_file(path);
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let (deck, source_dir) = match extension.to_lowercase().as_str() {
        | "slide" => {
            let (d, cache) = unpack_deck_and_assets(path)?;
            (d, Some(cache))
        },
        | "json" => {
            let content = std::fs::read_to_string(path)?;
            let d: SlideDeck = serde_json::from_str(&content)?;
            let parent = path.parent().map(Path::to_path_buf);
            (d, parent)
        },
        | "typ" => {
            let compiler = SlideCompiler::new()?;
            let d = compiler.compile_file(path)?;
            let parent = path.parent().map(Path::to_path_buf);
            (d, parent)
        },
        | _ => {
            if let Ok((d, cache)) = unpack_deck_and_assets(path) {
                (d, Some(cache))
            } else {
                let compiler = SlideCompiler::new()?;
                let d = compiler.compile_file(path)?;
                let parent = path.parent().map(Path::to_path_buf);
                (d, parent)
            }
        },
    };

    let mut app = SlideApp::from_deck(deck)
        .default_animation(animation)
        .fullscreen(fullscreen)
        .hud_theme(hud_theme);

    if let Some(ref dir) = source_dir {
        app = app.source_file(dir);
    }

    app.run()?;
    Ok(())
}

/// Launch the built-in reference showcase presentation (`examples/geek-presentation`)
pub fn launch_builtin_demo(
    animation: &str,
    fullscreen: bool,
    hud_theme: HudTheme,
) -> Result<(), Box<dyn std::error::Error>> {
    let (deck, cache_dir) =
        unpack_deck_and_assets_from_bytes(EMBEDDED_GEEK_DEMO, "geek_showcase_demo")?;
    SlideApp::from_deck(deck)
        .source_file(&cache_dir)
        .default_animation(animation)
        .fullscreen(fullscreen)
        .hud_theme(hud_theme)
        .run()?;
    Ok(())
}

/// Discovered presentation entry in current directory or recent history
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SlideFileEntry {
    pub path: PathBuf,
    pub name: String,
    pub size_str: String,
    pub title: String,
    pub slides_count: usize,
    pub is_slide_pkg: bool,
    pub is_favorite: bool,
    pub aspect_ratio: Option<String>,
    pub has_notes: bool,
    pub author: Option<String>,
}

/// Helper to get cargo-slide viewer configuration directory
pub fn get_viewer_config_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(|h| PathBuf::from(h).join(".config").join("cargo-slide"))
}

/// Load recent presentations from configuration
pub fn load_recent_viewer_files() -> Vec<String> {
    if let Some(dir) = get_viewer_config_dir() {
        let path = dir.join("recent_viewer.json");
        if path.is_file()
            && let Ok(content) = std::fs::read_to_string(&path)
            && let Ok(list) = serde_json::from_str::<Vec<String>>(&content)
        {
            return list;
        }
    }
    Vec::new()
}

/// Save recent presentation to configuration
pub fn save_recent_viewer_file(file_path: &Path) {
    let mut recents = load_recent_viewer_files();
    let canonical = file_path.canonicalize().unwrap_or_else(|_| {
        if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_default().join(file_path)
        }
    });
    let s = canonical.to_string_lossy().to_string();
    recents.retain(|p| p != &s);
    recents.insert(0, s);
    if recents.len() > 20 {
        recents.truncate(20);
    }
    if let Some(dir) = get_viewer_config_dir() {
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("recent_viewer.json");
        if let Ok(json) = serde_json::to_string_pretty(&recents) {
            let _ = std::fs::write(path, json);
        }
    }
}

/// Load favorite presentation paths from configuration
pub fn load_favorite_files() -> Vec<String> {
    if let Some(dir) = get_viewer_config_dir() {
        let path = dir.join("favorites_viewer.json");
        if path.is_file()
            && let Ok(content) = std::fs::read_to_string(&path)
            && let Ok(list) = serde_json::from_str::<Vec<String>>(&content)
        {
            return list;
        }
    }
    Vec::new()
}

/// Toggle favorite status of a presentation file
pub fn toggle_favorite_file(file_path: &Path) -> bool {
    let mut favs = load_favorite_files();
    let s = file_path.to_string_lossy().to_string();
    let is_fav = if favs.contains(&s) {
        favs.retain(|p| p != &s);
        false
    } else {
        favs.push(s);
        true
    };
    if let Some(dir) = get_viewer_config_dir() {
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("favorites_viewer.json");
        if let Ok(json) = serde_json::to_string_pretty(&favs) {
            let _ = std::fs::write(path, json);
        }
    }
    is_fav
}

/// Scan current directory and recent history for presentation files
pub fn scan_local_presentations() -> Vec<SlideFileEntry> {
    let mut entries = Vec::new();
    let favorites = load_favorite_files();

    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(dir_entries) = std::fs::read_dir(&cwd) {
            for entry in dir_entries.flatten() {
                let p = entry.path();
                if p.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("slide"))
                {
                    let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                    let size_str = format!("{:.1} KB", size as f64 / 1024.0);
                    let (title, total_slides, aspect_ratio, has_notes, author) =
                        read_package_metadata(&p)
                            .map(|m| {
                                (
                                    m.title,
                                    m.total_slides,
                                    m.aspect_ratio,
                                    m.has_notes,
                                    m.author,
                                )
                            })
                            .unwrap_or_else(|_| {
                                (
                                    p.file_stem()
                                        .map(|s| s.to_string_lossy().to_string())
                                        .unwrap_or_default(),
                                    0,
                                    Some("16:9".to_string()),
                                    false,
                                    None,
                                )
                            });

                    let p_str = p.to_string_lossy().to_string();
                    let is_favorite = favorites.contains(&p_str);

                    entries.push(SlideFileEntry {
                        name: p
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string(),
                        path: p,
                        size_str,
                        title,
                        slides_count: total_slides,
                        is_slide_pkg: true,
                        is_favorite,
                        aspect_ratio,
                        has_notes,
                        author,
                    });
                }
            }
        }

        // Also check if `slides.typ` or other presentation typst files exist in current directory
        for typ_name in &["slides.typ", "main.typ", "presentation.typ"] {
            let typ_candidate = cwd.join(typ_name);
            if typ_candidate.exists() && !entries.iter().any(|e| e.path == typ_candidate) {
                let size = std::fs::metadata(&typ_candidate)
                    .map(|m| m.len())
                    .unwrap_or(0);
                let size_str = format!("{:.1} KB", size as f64 / 1024.0);
                let p_str = typ_candidate.to_string_lossy().to_string();
                let is_favorite = favorites.contains(&p_str);
                entries.push(SlideFileEntry {
                    name: (*typ_name).to_string(),
                    path: typ_candidate,
                    size_str,
                    title: format!("Local Typst Slides ({})", typ_name),
                    slides_count: 0,
                    is_slide_pkg: false,
                    is_favorite,
                    aspect_ratio: Some("16:9".to_string()),
                    has_notes: false,
                    author: None,
                });
            }
        }
    }

    // Also bring in recent presentations that exist on disk
    for recent_path_str in load_recent_viewer_files() {
        let p = PathBuf::from(&recent_path_str);
        if p.exists() && !entries.iter().any(|e| e.path == p) {
            let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            let size_str = format!("{:.1} KB", size as f64 / 1024.0);
            let is_favorite = favorites.contains(&recent_path_str);

            if p.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("slide"))
            {
                let (title, total_slides, aspect_ratio, has_notes, author) =
                    read_package_metadata(&p)
                        .map(|m| {
                            (
                                m.title,
                                m.total_slides,
                                m.aspect_ratio,
                                m.has_notes,
                                m.author,
                            )
                        })
                        .unwrap_or_else(|_| {
                            (
                                p.file_stem()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_default(),
                                0,
                                Some("16:9".to_string()),
                                false,
                                None,
                            )
                        });

                entries.push(SlideFileEntry {
                    name: p
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    path: p,
                    size_str,
                    title,
                    slides_count: total_slides,
                    is_slide_pkg: true,
                    is_favorite,
                    aspect_ratio,
                    has_notes,
                    author,
                });
            } else if p
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("typ"))
            {
                entries.push(SlideFileEntry {
                    name: p
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    path: p.clone(),
                    size_str,
                    title: format!(
                        "Recent Typst: {}",
                        p.file_stem().unwrap_or_default().to_string_lossy()
                    ),
                    slides_count: 0,
                    is_slide_pkg: false,
                    is_favorite,
                    aspect_ratio: Some("16:9".to_string()),
                    has_notes: false,
                    author: None,
                });
            }
        }
    }

    entries
}
