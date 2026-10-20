//! Command to launch the interactive presentation player.

use slide_core::compiler::SlideCompiler;
use slide_core::package::is_encrypted_package_file;
use slide_core::package::unpack_deck_and_assets;
use slide_core::security::piv::PivManager;
use slide_player::PlayerConfig;
use slide_player::SlidePlayer;
use std::io::IsTerminal;
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
    passphrase: Option<&str>,
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
                "[SECURITY] Decrypting protected presentation in memory: {}",
                file.display()
            ),
            Some(serde_json::json!({
                "stage": "package_decrypt_start",
                "file": file.display().to_string(),
            })),
        );

        let data = std::fs::read(file)?;
        let container = slide_core::security::EncryptedSlideContainer::parse(&data)?;

        let (d, store) = match container.header.auth_mode {
            | slide_core::security::AuthMode::Passphrase
            | slide_core::security::AuthMode::Mnemonic => {
                let max_attempts = 3;
                let mut last_err = None;
                let mut decrypted = None;

                let mut initial_secret = passphrase
                    .map(|p| slide_core::security::Zeroizing::new(p.to_string()))
                    .or_else(|| {
                        std::env::var("SLIDE_PASSPHRASE")
                            .or_else(|_| std::env::var("SLIDE_SECRET"))
                            .ok()
                            .map(slide_core::security::Zeroizing::new)
                    });

                for attempt in 0..max_attempts {
                    let secret: slide_core::security::Zeroizing<String> = if let Some(s) =
                        initial_secret.take()
                    {
                        s
                    } else {
                        let attempts_left = max_attempts - attempt;
                        let is_mnemonic =
                            container.header.auth_mode == slide_core::security::AuthMode::Mnemonic;
                        let title = if attempt == 0 {
                            "Cargo Slide - Presentation Decryption"
                        } else {
                            "Authentication Failed - Please Re-enter"
                        };
                        let gui_subtitle = if attempt == 0 {
                            if is_mnemonic {
                                "Enter 12/24-word BIP-39 recovery mnemonic:".to_string()
                            } else {
                                "Enter passphrase to decrypt presentation:".to_string()
                            }
                        } else if is_mnemonic {
                            format!(
                                "Incorrect mnemonic ({attempts_left} attempts remaining). Please re-enter:"
                            )
                        } else {
                            format!(
                                "Incorrect passphrase ({attempts_left} attempts remaining). Please re-enter:"
                            )
                        };

                        if let Some(gui_secret) = slide_player::prompt_pin_gui(title, &gui_subtitle)
                        {
                            gui_secret
                        } else if std::io::stdin().is_terminal() {
                            let term_prompt = if attempt == 0 {
                                if is_mnemonic {
                                    "Enter 12/24-word recovery mnemonic: ".to_string()
                                } else {
                                    "Enter passphrase: ".to_string()
                                }
                            } else if is_mnemonic {
                                format!(
                                    "\x1b[33mIncorrect mnemonic ({attempts_left} attempts remaining). Please re-enter:\x1b[0m "
                                )
                            } else {
                                format!(
                                    "\x1b[33mIncorrect passphrase ({attempts_left} attempts remaining). Please re-enter:\x1b[0m "
                                )
                            };
                            slide_core::security::read_masked_secret(&term_prompt)?
                        } else {
                            break;
                        }
                    };

                    match container.decrypt_with_secret(&secret) {
                        | Ok(res) => {
                            decrypted = Some(res);
                            break;
                        },
                        | Err(e) => {
                            last_err = Some(e);
                            if attempt + 1 < max_attempts {
                                eprintln!(
                                    "[WARNING] Authentication failed. Please try again ({} attempts remaining).",
                                    max_attempts - 1 - attempt
                                );
                            }
                        },
                    }
                }

                decrypted.ok_or_else(|| {
                    last_err.unwrap_or_else(|| {
                        slide_core::error::SlideError::Crypto(
                            "Authentication canceled or retries exhausted".to_string(),
                        )
                    })
                })?
            },
            | slide_core::security::AuthMode::Piv => {
                let mut card = PivManager::acquire_card(pin)?;
                if !card.is_authenticated() && pin.is_none() {
                    let max_attempts = 3;
                    for attempt in 0..max_attempts {
                        let attempts_left = match card.check_pin_status() {
                            | Ok(slide_core::security::piv::PinStatus::NotVerified {
                                tries_remaining,
                            }) => (tries_remaining as usize).min(max_attempts - attempt),
                            | _ => max_attempts - attempt,
                        };
                        let title = if attempt == 0 {
                            "Cargo Slide - YubiKey PIV Authentication"
                        } else {
                            "PIN Verification Failed - Please Re-enter"
                        };
                        let subtitle = if attempt == 0 {
                            format!(
                                "Enter 6-8 digit User PIN to decrypt presentation ({attempts_left} attempts remaining):"
                            )
                        } else {
                            format!(
                                "Incorrect PIN ({attempts_left} attempts remaining). Please re-enter:"
                            )
                        };

                        if let Some(gui_pin) = slide_player::prompt_pin_gui(title, &subtitle) {
                            match card.verify_pin(&gui_pin) {
                                | Ok(()) => break,
                                | Err(e) => {
                                    if attempt + 1 == max_attempts {
                                        return Err(e.into());
                                    }
                                    eprintln!("[WARNING] Incorrect PIN. Please try again.");
                                },
                            }
                        } else {
                            break;
                        }
                    }
                }
                slide_core::security::prompt_pin_if_needed(card.as_mut(), pin)?;

                container.decrypt_in_memory(card.as_mut(), None)?
            },
        };

        in_memory_asset_store = Some(store);

        slide_core::logger::log_event(
            "success",
            "[OK] SHA3-512 integrity verified & deck decrypted directly in memory",
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
