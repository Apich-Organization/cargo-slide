//! Command to launch the interactive presentation player.

use slide_core::compiler::SlideCompiler;
use slide_core::package::unpack_deck_and_assets;
use slide_player::PlayerConfig;
use slide_player::SlidePlayer;
use std::path::Path;

/// Launch the presentation viewer for .typ, .slide, or deck.json files.
#[allow(clippy::too_many_lines)]
pub fn execute(
    file: &Path,
    animation: &str,
    watch: bool,
    fullscreen: bool,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    if !file.exists() {
        return Err(format!("File does not exist: {}", file.display()).into());
    }

    let is_slide_pkg = file.extension().and_then(|e| e.to_str()) == Some("slide");
    let is_json_deck = file.extension().and_then(|e| e.to_str()) == Some("json");

    let (deck, source_dir) = if is_slide_pkg {
        slide_core::logger::log_event(
            "info",
            &format!("[UNPACK] Unpacking slide package {}...", file.display()),
            Some(serde_json::json!({
                "stage": "package_unpack",
                "file": file.display().to_string(),
            })),
        );
        let (d, cache) = unpack_deck_and_assets(file)?;
        (d, Some(cache))
    } else if is_json_deck {
        let content = std::fs::read_to_string(file)?;
        let d: slide_core::model::SlideDeck = serde_json::from_str(&content)?;
        let parent = file.parent().map(Path::to_path_buf);
        (d, parent)
    } else {
        slide_core::logger::log_event(
            "info",
            &format!("[COMPILE] Compiling {}...", file.display()),
            Some(serde_json::json!({
                "stage": "compile_start",
                "file": file.display().to_string(),
            })),
        );
        let compiler = SlideCompiler::new()?;
        let d = compiler.compile_file(file)?;
        (d, file.parent().map(Path::to_path_buf))
    };

    slide_core::logger::log_event(
        "success",
        &format!(
            "[OK] Presentation loaded successfully! {} slides available.",
            deck.total_slides()
        ),
        Some(serde_json::json!({
            "stage": "compile_success",
            "total_slides": deck.total_slides(),
            "title": &deck.title,
        })),
    );

    let config = PlayerConfig {
        title: deck.title.clone(),
        default_animation: animation.to_string(),
        fullscreen,
        watch: watch && !is_slide_pkg,
        ..Default::default()
    };

    slide_core::logger::log_event(
        "info",
        "[RUN] Launching interactive GUI presentation player...",
        Some(serde_json::json!({
            "stage": "player_launch",
            "fullscreen": fullscreen,
            "animation": animation,
            "watch": watch && !is_slide_pkg,
        })),
    );
    let canonical_file = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());

    SlidePlayer::new(deck)
        .with_config(config)
        .with_source_file(source_dir.or(Some(canonical_file)))
        .with_watch(watch && !is_slide_pkg)
        .run()?;

    Ok(())
}
