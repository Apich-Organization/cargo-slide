//! Command to launch the interactive presentation player.

use slide_core::compiler::SlideCompiler;
use slide_core::package::is_encrypted_package_file;
use slide_core::package::unpack_deck_and_assets;
use slide_core::package::unpack_encrypted_deck_and_assets_in_memory;
use slide_core::security::piv::PivManager;
use slide_player::PlayerConfig;
use slide_player::SlidePlayer;
use std::path::Path;

/// Launch the presentation viewer for .typ, .slide, or deck.json files.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub fn execute(
    file: &Path,
    animation: &str,
    watch: bool,
    fullscreen: bool,
    target_minutes: Option<u64>,
    light: bool,
    kiosk: Option<u64>,
    pin: Option<&str>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let resolved_path = slide_core::compiler::resolve_presentation_target(file)?;
    let file = resolved_path.as_path();

    let is_slide_pkg = file.extension().and_then(|e| e.to_str()) == Some("slide");
    let is_json_deck = file.extension().and_then(|e| e.to_str()) == Some("json");

    let is_encrypted = is_slide_pkg && is_encrypted_package_file(file);
    let mut in_memory_asset_store = None;

    let (deck, source_dir) = if is_encrypted {
        slide_core::logger::log_event(
            "info",
            &format!(
                "[SECURITY] Decrypting PIV-protected presentation in memory: {}",
                file.display()
            ),
            Some(serde_json::json!({
                "stage": "package_decrypt_start",
                "file": file.display().to_string(),
            })),
        );

        let mut card = PivManager::acquire_card(pin)?;
        if !card.is_authenticated() && pin.is_none() {
            if let Some(gui_pin) = slide_player::prompt_pin_gui(
                "Cargo Slide - YubiKey PIV Authentication",
                "Enter 6-8 digit User PIN to decrypt presentation:",
            ) {
                card.verify_pin(&gui_pin)?;
            }
        }
        slide_core::security::prompt_pin_if_needed(card.as_mut(), pin)?;

        let (d, store) = unpack_encrypted_deck_and_assets_in_memory(file, card.as_mut(), None)?;
        in_memory_asset_store = Some(store);

        slide_core::logger::log_event(
            "success",
            "[OK] SHA3-512 integrity verified & deck decrypted directly in memory (zero-disk dump prevention)",
            Some(serde_json::json!({
                "stage": "package_decrypt_success",
                "total_slides": d.total_slides(),
            })),
        );

        (d, None)
    } else if is_slide_pkg {
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

    let hud_theme = if light {
        slide_player::hud::HudTheme::Light
    } else {
        slide_player::hud::HudTheme::Dark
    };

    let config = PlayerConfig {
        title: deck.title.clone(),
        default_animation: animation.to_string(),
        fullscreen,
        watch: watch && !is_slide_pkg,
        hud_theme,
        target_duration_secs: target_minutes.map(|m| m.saturating_mul(60)),
        kiosk_interval_secs: kiosk,
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
        .with_asset_store(in_memory_asset_store)
        .with_watch(watch && !is_slide_pkg)
        .run()?;

    Ok(())
}
