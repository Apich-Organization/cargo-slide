use slide_core::compiler::SlideCompiler;
use std::fs::create_dir_all;
use std::fs::write;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

pub use crate::commands::cross::SUPPORTED_CROSS_TARGETS;

/// Dispatch presentation build by format: binary, slide, or wasm.
#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::fn_params_excessive_bools
)]
pub fn execute(
    file: &Path,
    output: Option<PathBuf>,
    format: &str,
    animation: &str,
    target: Option<&str>,
    cross: bool,
    cross_tool: &str,
    all_targets: bool,
    source: bool,
    encrypt: bool,
    auth_mode_str: &str,
    passphrase: Option<&str>,
    mnemonic: Option<&str>,
    mnemonic_words: usize,
    token_ttl: u64,
    piv_slot_str: &str,
    sign_slot_str: &str,
    cert_path: Option<&Path>,
    pin: Option<&str>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let resolved_file = slide_core::compiler::resolve_presentation_target(file)?;
    let target_file = resolved_file.as_path();

    match format.to_lowercase().as_str() {
        | "slide" | "package" => {
            crate::commands::pack::execute(
                target_file,
                output,
                animation,
                source,
                encrypt,
                auth_mode_str,
                passphrase,
                mnemonic,
                mnemonic_words,
                piv_slot_str,
                sign_slot_str,
                cert_path,
                pin,
            )
        },
        | "wasm" | "web" | "csr" => {
            execute_wasm(
                target_file,
                output,
                animation,
                encrypt,
                auth_mode_str,
                passphrase,
                mnemonic,
                mnemonic_words,
                token_ttl,
            )
        },
        | "binary" | "exe" | "elf" | "" => {
            if all_targets || target == Some("all") {
                let file_stem = target_file
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("presentation");
                eprintln!(
                    "[CROSS] Building presentation for all supported desktop targets (x86_64/ARM64 Linux & Windows)..."
                );
                for &(triple, label) in SUPPORTED_CROSS_TARGETS {
                    crate::commands::cross::ensure_target_installed(triple);
                    let is_win = triple.contains("windows");
                    let ext = if is_win { ".exe" } else { "" };
                    let target_out = output.as_ref().map_or_else(
                        || PathBuf::from(format!("{file_stem}-{label}{ext}")),
                        |out| {
                            if out.is_dir() {
                                out.join(format!("{file_stem}-{label}{ext}"))
                            } else {
                                let parent = out.parent().unwrap_or_else(|| Path::new("."));
                                let stem = out
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or(file_stem);
                                parent.join(format!("{stem}-{label}{ext}"))
                            }
                        },
                    );

                    eprintln!(
                        ">>> Cross-compiling target [{triple}] -> {}",
                        target_out.display()
                    );
                    execute_binary(
                        target_file,
                        Some(target_out),
                        animation,
                        Some(triple),
                        true,
                        cross_tool,
                        encrypt,
                        auth_mode_str,
                        passphrase,
                        mnemonic,
                        mnemonic_words,
                        piv_slot_str,
                        sign_slot_str,
                        cert_path,
                        pin,
                    )?;
                }
                eprintln!("[OK] Cross-platform compilation finished for all desktop targets!");
                Ok(())
            } else {
                let normalized_target = target.map(crate::commands::cross::normalize_target_triple);
                if let Some(triple) = normalized_target {
                    crate::commands::cross::ensure_target_installed(triple);
                }
                execute_binary(
                    target_file,
                    output,
                    animation,
                    normalized_target,
                    cross,
                    cross_tool,
                    encrypt,
                    auth_mode_str,
                    passphrase,
                    mnemonic,
                    mnemonic_words,
                    piv_slot_str,
                    sign_slot_str,
                    cert_path,
                    pin,
                )
            }
        },
        | other => {
            Err(
                format!("Unknown build format: '{other}'. Supported formats: binary, slide, wasm")
                    .into(),
            )
        },
    }
}

/// Build standalone Leptos CSR web presentation bundle.
#[allow(clippy::too_many_arguments)]
pub fn execute_wasm(
    file: &Path,
    output: Option<PathBuf>,
    _animation: &str,
    encrypt: bool,
    auth_mode: &str,
    passphrase: Option<&str>,
    mnemonic: Option<&str>,
    mnemonic_words: usize,
    token_ttl: u64,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    if !file.exists() {
        return Err(format!("File does not exist: {}", file.display()).into());
    }

    let stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("presentation");
    let out_dir = output.unwrap_or_else(|| PathBuf::from(format!("{stem}-web")));

    slide_core::logger::log_event(
        "info",
        &format!(
            "[BUILD] Building Leptos CSR web presentation: {}",
            out_dir.display()
        ),
        Some(serde_json::json!({
            "stage": "wasm_build_start",
            "source_file": file.display().to_string(),
            "output_dir": out_dir.display().to_string(),
            "encrypted": encrypt,
            "auth_mode": auth_mode,
        })),
    );

    let opts = crate::commands::serve::CsrBundleOptions {
        encrypt,
        auth_mode,
        passphrase,
        mnemonic,
        mnemonic_words,
        token_ttl_hours: token_ttl,
    };
    crate::commands::serve::prepare_csr_bundle_with_options(file, &out_dir, &opts)?;

    slide_core::logger::log_event(
        "success",
        &format!(
            "[OK] Leptos CSR web bundle created successfully at: {}",
            out_dir.display()
        ),
        Some(serde_json::json!({
            "stage": "wasm_build_success",
            "output_dir": out_dir.display().to_string(),
        })),
    );

    Ok(())
}

/// Compile the presentation into a standalone release binary. `target`, when given, cross-compiles
/// for that Rust target triple (e.g. `x86_64-pc-windows-msvc`) instead of the host's own platform;
/// the triple must already be installed via `rustup target add` and have a working linker
/// configured for a non-host target.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub fn execute_binary(
    file: &Path,
    output: Option<PathBuf>,
    animation: &str,
    target: Option<&str>,
    cross: bool,
    cross_tool: &str,
    encrypt: bool,
    auth_mode_str: &str,
    passphrase: Option<&str>,
    mnemonic: Option<&str>,
    mnemonic_words: usize,
    piv_slot_str: &str,
    sign_slot_str: &str,
    cert_path: Option<&Path>,
    pin: Option<&str>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Validate animation name against injection attacks
    const VALID_ANIMATIONS: &[&str] = &[
        "fade",
        "cut",
        "slide-left",
        "slide-right",
        "particles",
        "zoom",
        "glitch",
        "wipe",
        "dissolve",
    ];
    let safe_animation = if VALID_ANIMATIONS.contains(&animation)
        || (!animation.is_empty()
            && animation
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
    {
        animation
    } else {
        return Err(format!(
            "Invalid animation identifier '{animation}'. Must be alphanumeric with hyphens/underscores."
        )
        .into());
    };

    // The target platform decides the binary's extension: check if an explicit Windows target
    // was specified, or if we are building natively on a Windows host without an explicit target.
    let targets_windows =
        target.is_some_and(|t| t.contains("windows")) || (target.is_none() && cfg!(windows));
    if !file.exists() {
        return Err(format!("File does not exist: {}", file.display()).into());
    }

    slide_core::logger::log_event(
        "info",
        &format!(
            "[PACKAGE] Packaging presentation into single binary (encrypted: {}): {}",
            encrypt,
            file.display()
        ),
        Some(serde_json::json!({
            "stage": "package_start",
            "source_file": file.display().to_string(),
            "encrypted": encrypt,
            "percent": 1,
        })),
    );
    let compiler = SlideCompiler::new()?;
    let deck = compiler.compile_file(file)?;

    let stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("presentation");
    let default_name = if targets_windows {
        format!("{stem}-presentation.exe")
    } else {
        format!("{stem}-presentation")
    };
    let mut target_binary = output.unwrap_or_else(|| PathBuf::from(default_name));
    if targets_windows && target_binary.extension().is_none() {
        target_binary.set_extension("exe");
    }

    slide_core::logger::log_event(
        "info",
        &format!(
            "[BUNDLE] Generating self-contained Rust bundle with {} slides...",
            deck.total_slides()
        ),
        Some(serde_json::json!({
            "stage": "bundle_generate",
            "total_slides": deck.total_slides(),
            "percent": 3,
        })),
    );

    // Discover repository root for workspace dependencies
    let current_exe = std::env::current_exe().unwrap_or_default();
    let repo_root = find_repo_root(&current_exe)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    // Create persistent build project in target/.build_tmp/slide_runner_project to enable Rust incremental compilation caching
    let build_dir_parent = if repo_root.join("target").exists() {
        let p = repo_root.join("target/.build_tmp");
        let _ = create_dir_all(&p);
        p
    } else {
        std::env::temp_dir()
    };
    let build_path = build_dir_parent.join("slide_runner_project");
    let src_dir = build_path.join("src");
    create_dir_all(&src_dir)?;

    let encrypted_bin_file = build_path.join("encrypted_deck.bin");
    let deck_file = build_path.join("deck.json");

    if encrypt {
        use slide_core::security::AuthMode;
        use slide_core::security::container::EncryptedSlideContainer;

        let auth_mode: AuthMode = auth_mode_str.parse()?;

        let enc_bytes = match auth_mode {
            | AuthMode::Passphrase => {
                let pass = if let Some(p) = passphrase {
                    p.to_string()
                } else if let Ok(env_pass) = std::env::var("SLIDE_PASSPHRASE") {
                    env_pass
                } else {
                    let input = slide_core::security::read_masked_secret(
                        "Enter passphrase for presentation encryption: ",
                    )?;
                    input.to_string()
                };

                slide_core::security::validate_password_strength(&pass)?;

                let (pkg, _) = EncryptedSlideContainer::pack_software_with_source(
                    &deck,
                    file.parent(),
                    &pass,
                    AuthMode::Passphrase,
                    None,
                    false,
                )?;
                pkg
            },
            | AuthMode::Mnemonic => {
                let mnem = mnemonic.unwrap_or("");
                let (pkg, gen_mnem) = EncryptedSlideContainer::pack_software_with_source(
                    &deck,
                    file.parent(),
                    mnem,
                    AuthMode::Mnemonic,
                    Some(mnemonic_words),
                    false,
                )?;
                if let Some(generated) = gen_mnem {
                    let word_count = generated.split_whitespace().count();
                    eprintln!();
                    eprintln!(
                        "================================================================================"
                    );
                    eprintln!("[SECURITY] Generated {word_count}-Word BIP-39 Recovery Mnemonic:");
                    eprintln!();
                    eprintln!("    {}", generated.as_str());
                    eprintln!();
                    eprintln!(
                        "[IMPORTANT] Write down these {word_count} words in order and store them in a secure location."
                    );
                    eprintln!(
                        "            Anyone with this mnemonic can decrypt and view this presentation."
                    );
                    eprintln!(
                        "================================================================================"
                    );
                    eprintln!();
                }
                pkg
            },
            | AuthMode::Piv => {
                use slide_core::security::cert::PivPublicKey;
                use slide_core::security::cert::PivSlot;
                use slide_core::security::cert::parse_certificate_public_key;
                use slide_core::security::piv::PivManager;

                let recipient_slot = PivSlot::parse(piv_slot_str)
                    .ok_or_else(|| format!("Invalid recipient PIV slot: {piv_slot_str}"))?;
                let signer_slot = PivSlot::parse(sign_slot_str)
                    .ok_or_else(|| format!("Invalid signer PIV slot: {sign_slot_str}"))?;

                let mut card = PivManager::acquire_card(pin).map_err(|e| {
                    format!(
                        "{e}\n\n[TIP] To compile without a hardware YubiKey, use software passphrase mode:\n  cargo slide build slides.typ --encrypt --auth passphrase\nor BIP-39 recovery mnemonic mode:\n  cargo slide build slides.typ --encrypt --auth mnemonic"
                    )
                })?;
                slide_core::security::prompt_pin_for_slot(card.as_mut(), signer_slot, pin, false)?;

                let recipient_pub = if let Some(cp) = cert_path {
                    let raw_data = std::fs::read(cp)?;
                    if let Ok(pubkey) = parse_certificate_public_key(&raw_data) {
                        pubkey
                    } else {
                        PivPublicKey::from_pem_or_der(&raw_data)?
                    }
                } else {
                    card.get_public_key(recipient_slot)?
                };

                let base_dir = file.parent();
                EncryptedSlideContainer::pack_with_source(
                    &deck,
                    base_dir,
                    &recipient_pub,
                    recipient_slot,
                    card.as_mut(),
                    signer_slot,
                    false,
                )?
            },
        };

        let should_write = std::fs::read(&encrypted_bin_file).map_or(true, |ex| ex != enc_bytes);
        if should_write {
            write(&encrypted_bin_file, &enc_bytes)?;
        }
        if deck_file.exists() {
            let _ = std::fs::remove_file(&deck_file);
        }
    } else {
        // Serialize deck to JSON to be embedded via include_str!
        let deck_json = serde_json::to_string(&deck)?;
        let should_write_deck =
            std::fs::read_to_string(&deck_file).map_or(true, |existing| existing != deck_json);
        if should_write_deck {
            write(&deck_file, &deck_json)?;
        }
        if encrypted_bin_file.exists() {
            let _ = std::fs::remove_file(&encrypted_bin_file);
        }
    }

    let core_path = repo_root.join("crates/slide-core");
    let player_path = repo_root.join("crates/slide-player");

    // Format paths with forward slashes for cross-platform TOML compatibility (avoids backslash escaping on Windows)
    let core_str = core_path.to_string_lossy().replace('\\', "/");
    let player_str = player_path.to_string_lossy().replace('\\', "/");

    let pkg_version = env!("CARGO_PKG_VERSION");
    let (core_dep, player_dep) = if core_path.exists() && player_path.exists() {
        (
            format!(r#"slide-core = {{ path = "{core_str}", version = "{pkg_version}" }}"#),
            format!(r#"slide-player = {{ path = "{player_str}", version = "{pkg_version}" }}"#),
        )
    } else {
        (
            format!(r#"slide-core = "{pkg_version}""#),
            format!(r#"slide-player = "{pkg_version}""#),
        )
    };

    let cargo_toml = format!(
        r#"[package]
name = "slide-standalone-runner"
version = "{pkg_version}"
edition = "2024"

[workspace]

[dependencies]
{core_dep}
{player_dep}
serde_json = "1.0"

[profile.release]
incremental = true
"#
    );
    let cargo_toml_file = build_path.join("Cargo.toml");
    let should_write_toml =
        std::fs::read_to_string(&cargo_toml_file).map_or(true, |existing| existing != cargo_toml);
    if should_write_toml {
        write(&cargo_toml_file, &cargo_toml)?;
    }

    let runner_main = if encrypt {
        format!(
            r#"use slide_player::prelude::*;
use slide_core::security::{{AuthMode, EncryptedSlideContainer, PivManager, prompt_pin_if_needed, read_masked_secret}};
use std::io::IsTerminal as _;

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let args: Vec<String> = std::env::args().collect();
    let mut pin: Option<String> = std::env::var("PIV_PIN").ok();
    let mut passphrase: Option<String> = std::env::var("SLIDE_PASSPHRASE")
        .or_else(|_| std::env::var("SLIDE_SECRET"))
        .ok();

    let mut i = 1;
    while i < args.len() {{
        match args[i].as_str() {{
            "--pin" if i + 1 < args.len() => {{
                pin = Some(args[i + 1].clone());
                i += 2;
            }}
            "--passphrase" | "--password" | "--secret" if i + 1 < args.len() => {{
                passphrase = Some(args[i + 1].clone());
                i += 2;
            }}
            "--help" | "-h" => {{
                println!("Encrypted Standalone Slide Presentation");
                println!("Usage: {{}} [--pin <PIN>] [--passphrase <PASSPHRASE>]", args[0]);
                println!("Hardware requirement: Connected YubiKey PIV smartcard (if PIV mode)");
                println!("Supported Authentication: Hardware YubiKey PIV smartcard OR Passphrase/BIP-39 Mnemonic");
                println!("Environment variables: PIV_PIN, SLIDE_PASSPHRASE, SLIDE_SECRET");
                return Ok(());
            }}
            _ => {{ i += 1; }}
        }}
    }}

    const ENCRYPTED_DATA: &[u8] = include_bytes!("../encrypted_deck.bin");

    let container = EncryptedSlideContainer::parse(ENCRYPTED_DATA)
        .map_err(|e| format!("Invalid presentation container: {{e}}"))?;

    let (deck, asset_store) = match container.header.auth_mode {{
        AuthMode::Passphrase | AuthMode::Mnemonic => {{
            let max_attempts = 3;
            let mut last_err = None;
            let mut decrypted = None;
            let mut initial_secret = passphrase.map(|p| slide_core::security::Zeroizing::new(p))
                .or_else(|| std::env::var("SLIDE_PASSPHRASE").or_else(|_| std::env::var("SLIDE_SECRET")).ok().map(slide_core::security::Zeroizing::new));

            for attempt in 0..max_attempts {{
                let secret: slide_core::security::Zeroizing<String> = if let Some(s) = initial_secret.take() {{
                    s
                }} else {{
                    let attempts_left = max_attempts - attempt;
                    let is_mnemonic = container.header.auth_mode == AuthMode::Mnemonic;
                    let title = if attempt == 0 {{
                        "Cargo Slide - Presentation Decryption"
                    }} else {{
                        "Authentication Failed - Please Re-enter"
                    }};
                    let gui_subtitle = if attempt == 0 {{
                        if is_mnemonic {{
                            "Enter 12/24-word BIP-39 recovery mnemonic:".to_string()
                        }} else {{
                            "Enter passphrase to decrypt presentation:".to_string()
                        }}
                    }} else if is_mnemonic {{
                        format!("Incorrect mnemonic ({{attempts_left}} attempts remaining). Please re-enter:")
                    }} else {{
                        format!("Incorrect passphrase ({{attempts_left}} attempts remaining). Please re-enter:")
                    }};

                    if let Some(gui_secret) = prompt_pin_gui(title, &gui_subtitle) {{
                        gui_secret
                    }} else if std::io::stdin().is_terminal() {{
                        let term_prompt = if attempt == 0 {{
                            if is_mnemonic {{
                                "Enter 12/24-word recovery mnemonic: ".to_string()
                            }} else {{
                                "Enter passphrase: ".to_string()
                            }}
                        }} else if is_mnemonic {{
                            format!("\x1b[33mIncorrect mnemonic ({{attempts_left}} attempts remaining). Please re-enter:\x1b[0m ")
                        }} else {{
                            format!("\x1b[33mIncorrect passphrase ({{attempts_left}} attempts remaining). Please re-enter:\x1b[0m ")
                        }};
                        read_masked_secret(&term_prompt)?
                    }} else {{
                        break;
                    }}
                }};

                match container.decrypt_with_secret(&secret) {{
                    Ok(res) => {{
                        decrypted = Some(res);
                        break;
                    }}
                    Err(e) => {{
                        last_err = Some(e);
                        if attempt + 1 < max_attempts {{
                            eprintln!("[WARNING] Authentication failed. Please try again ({{}} attempts remaining).", max_attempts - 1 - attempt);
                        }}
                    }}
                }}
            }}

            decrypted.ok_or_else(|| format!("Integrity check or decryption failed: {{}}", last_err.map(|e| e.to_string()).unwrap_or_else(|| "Retries exhausted".to_string())))?
        }}
        AuthMode::Piv => {{
            let mut card = PivManager::acquire_card(pin.as_deref())
                .map_err(|e| format!("PIV smartcard connection failed: {{e}}"))?;

            if !card.is_authenticated() && pin.is_none() {{
                let max_attempts = 3;
                for attempt in 0..max_attempts {{
                    let attempts_left = match card.check_pin_status() {{
                        Ok(slide_core::security::piv::PinStatus::NotVerified {{ tries_remaining }}) => (tries_remaining as usize).min(max_attempts - attempt),
                        _ => max_attempts - attempt,
                    }};
                    let title = if attempt == 0 {{
                        "Cargo Slide - YubiKey PIV Authentication"
                    }} else {{
                        "PIN Verification Failed - Please Re-enter"
                    }};
                    let subtitle = if attempt == 0 {{
                        format!("Enter 6-8 digit User PIN to decrypt presentation ({{attempts_left}} attempts remaining):")
                    }} else {{
                        format!("Incorrect PIN ({{attempts_left}} attempts remaining). Please re-enter:")
                    }};

                    if let Some(gui_pin) = prompt_pin_gui(title, &subtitle) {{
                        match card.verify_pin(&gui_pin) {{
                            Ok(()) => break,
                            Err(e) => {{
                                if attempt + 1 == max_attempts {{
                                    return Err(format!("PIV authentication failed: {{e}}").into());
                                }}
                                eprintln!("[WARNING] Incorrect PIN. Please try again.");
                            }}
                        }}
                    }} else {{
                        break;
                    }}
                }}
            }}

            prompt_pin_if_needed(card.as_mut(), pin.as_deref())
                .map_err(|e| format!("PIV authentication failed: {{e}}"))?;

            container.decrypt_in_memory(card.as_mut(), None)
                .map_err(|e| format!("Integrity check or decryption failed: {{e}}"))?
        }}
    }};

    SlideApp::from_deck(deck)
        .asset_store(asset_store)
        .default_animation("{safe_animation}")
        .run()?;

    Ok(())
}}
"#
        )
    } else {
        format!(
            r#"use slide_player::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let deck_json = include_str!("../deck.json");
    let deck: SlideDeck = serde_json::from_str(deck_json)?;

    SlideApp::from_deck(deck)
        .default_animation("{safe_animation}")
        .run()?;

    Ok(())
}}
"#
        )
    };
    let main_file = src_dir.join("main.rs");
    let should_write_main =
        std::fs::read_to_string(&main_file).map_or(true, |existing| existing != runner_main);
    if should_write_main {
        write(&main_file, &runner_main)?;
    }

    slide_core::logger::log_event(
        "info",
        "[CARGO] Compiling release binary with Cargo...",
        Some(serde_json::json!({
            "stage": "cargo_build_release",
            "percent": 5,
        })),
    );
    let target_dir = repo_root.join("target/standalone_target");
    run_cargo_build_with_progress(&build_path, &target_dir, target, cross, cross_tool)?;

    let bin_filename = if targets_windows {
        "slide-standalone-runner.exe"
    } else {
        "slide-standalone-runner"
    };
    // `cargo build --target <triple>` (even for the host's own triple) nests output under
    // `<target-dir>/<triple>/release/`, not `<target-dir>/release/` -- only the no-`--target`
    // invocation uses the flat layout.
    let release_dir = target.map_or_else(
        || target_dir.join("release"),
        |triple| target_dir.join(triple).join("release"),
    );
    let built_bin = release_dir.join(bin_filename);
    if !built_bin.exists() {
        return Err(format!("Compiled binary not found at {}", built_bin.display()).into());
    }

    // Copy to target location
    std::fs::copy(&built_bin, &target_binary)?;

    // Ensure executable permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(&target_binary) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(&target_binary, perms);
        }
    }

    let size_bytes = std::fs::metadata(&target_binary).map_or(0, |m| m.len());
    #[allow(clippy::cast_precision_loss)]
    let size_mb = size_bytes as f64 / (1024.0 * 1024.0);

    let enc_suffix = if encrypt {
        " (AES-256-GCM + YubiKey PIV + SHA3-512 integrity verified)"
    } else {
        ""
    };
    slide_core::logger::log_event(
        "success",
        &format!(
            "[OK] Single binary generated successfully!{enc_suffix}\n Output: {} ({:.2} MB)",
            target_binary.display(),
            size_mb
        ),
        Some(serde_json::json!({
            "stage": "package_complete",
            "status": "success",
            "output": target_binary.display().to_string(),
            "size_mb": size_mb,
            "encrypted": encrypt,
            "total_slides": deck.total_slides(),
            "percent": 100,
        })),
    );

    Ok(())
}

/// Discover repository root for workspace dependencies
pub fn find_repo_root(exe: &Path) -> Option<PathBuf> {
    // 1. Explicit environment variable override
    if let Ok(env_root) = std::env::var("CARGO_SLIDE_REPO_ROOT") {
        let p = PathBuf::from(env_root);
        if p.join("crates").join("slide-core").exists() {
            return Some(p);
        }
    }

    // 2. Search upwards from current working directory
    if let Ok(cwd) = std::env::current_dir() {
        let mut cur = Some(cwd.as_path());
        while let Some(dir) = cur {
            if dir.join("crates").join("slide-core").exists() {
                return Some(dir.to_path_buf());
            }
            cur = dir.parent();
        }
    }

    // 3. Search upwards from executable location
    let mut cur = exe.parent();
    while let Some(dir) = cur {
        if dir.join("crates").join("slide-core").exists() {
            return Some(dir.to_path_buf());
        }
        cur = dir.parent();
    }

    // 4. Search upwards from compile-time manifest dir (if it actually exists on this host)
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    if manifest_dir.exists() {
        let mut cur = Some(manifest_dir);
        while let Some(dir) = cur {
            if dir.join("crates").join("slide-core").exists() {
                return Some(dir.to_path_buf());
            }
            cur = dir.parent();
        }
    }

    None
}

/// Resolved package-graph size for `build_path`'s generated standalone-runner project, used as a
/// rough denominator for build progress (see `run_cargo_build_with_progress`) -- not an exact
/// rustc-invocation count (a build-script or proc-macro crate can compile more than once), just
/// close enough that a caller polling `--log-format json`'s progress events sees a percentage
/// that moves at roughly the right pace instead of standing still for minutes then jumping to
/// 100%. Returns 0 (caller then treats every artifact as 1% until 100 are seen) if `cargo
/// metadata` itself fails for any reason, rather than aborting the whole build over a
/// progress-estimate step that was never essential to begin with.
fn estimate_total_units(
    build_path: &Path,
    target: Option<&str>,
) -> usize {
    let mut cmd = Command::new("cargo");
    cmd.arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(build_path);
    if let Some(triple) = target {
        cmd.arg("--filter-platform").arg(triple);
    }
    let Ok(output) = cmd.output() else {
        return 0;
    };
    if !output.status.success() {
        return 0;
    }
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return 0;
    };
    json.get("resolve")
        .and_then(|r| r.get("nodes"))
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len)
}

#[allow(clippy::too_many_lines)]
fn run_cargo_build_with_progress(
    build_path: &Path,
    target_dir: &Path,
    target: Option<&str>,
    cross: bool,
    cross_tool: &str,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let total = estimate_total_units(build_path, target);

    let explicit_cross = cross || cross_tool != "auto";
    let is_cross = target.is_some_and(crate::commands::cross::is_cross_target);

    let selected_backend = match cross_tool {
        | "zig" | "cargo-zigbuild" => "zig",
        | "cross" => "cross",
        | "cargo" | "native" => "cargo",
        | _ => {
            if is_cross || explicit_cross {
                crate::commands::cross::select_backend(target.unwrap_or(""), cross_tool)
            } else {
                "cargo"
            }
        },
    };

    slide_core::logger::log_event(
        "info",
        &format!(
            "[BUILD] Cross-platform compilation engine: {} (target: {})",
            match selected_backend {
                | "zig" => "cargo-zigbuild (Zig toolchain)",
                | "cross" => "cross (OCI container sysroot)",
                | _ => "cargo (native toolchain)",
            },
            target.unwrap_or("host")
        ),
        None,
    );

    let mut cargo_cmd = if selected_backend == "cross" {
        Command::new("cross")
    } else {
        Command::new("cargo")
    };

    if selected_backend == "zig" {
        cargo_cmd.arg("zigbuild");
    } else {
        cargo_cmd.arg("build");
    }

    cargo_cmd
        .arg("--release")
        .arg("--target-dir")
        .arg(target_dir)
        .arg("--message-format=json")
        .env("CARGO_INCREMENTAL", "1")
        .current_dir(build_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

    if let Some(triple) = target {
        cargo_cmd.arg("--target").arg(triple);
    }

    if selected_backend == "cross" {
        if std::env::var("CROSS_CONTAINER_ENGINE").is_err()
            && crate::commands::cross::which_cmd("podman")
            && !crate::commands::cross::which_cmd("docker")
        {
            cargo_cmd.env("CROSS_CONTAINER_ENGINE", "podman");
        }
        let oci_cross_config = build_path.join("Cross.toml");
        if !oci_cross_config.exists() {
            let _ = std::fs::write(
                &oci_cross_config,
                r#"[target.aarch64-unknown-linux-gnu]
pre-build = [
    "dpkg --add-architecture arm64",
    "apt-get update",
    "apt-get --assume-yes install libasound2-dev:arm64",
]
"#,
            );
        }
    }

    let mut child = cargo_cmd.spawn()?;
    let Some(stdout) = child.stdout.take() else {
        return Err("Failed to capture cargo build output".into());
    };
    let reader = BufReader::new(stdout);

    let mut compiled: usize = 0;
    let mut last_reported_percent: u8 = 0;
    for line in reader.lines() {
        let Ok(line) = line else { continue };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        let Some(reason) = value.get("reason").and_then(|r| r.as_str()) else {
            continue;
        };
        match reason {
            | "compiler-artifact" => {
                compiled = compiled.saturating_add(1);
                let denom = if total == 0 {
                    compiled.max(100)
                } else {
                    total
                };
                #[allow(clippy::cast_precision_loss)]
                let fraction = (compiled as f64 / denom as f64).min(1.0);
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let percent = fraction.mul_add(90.0, 5.0) as u8;
                if percent != last_reported_percent {
                    last_reported_percent = percent;
                    let pkg_name = value
                        .get("target")
                        .and_then(|t| t.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("dependency");
                    slide_core::logger::log_event(
                        "info",
                        &format!(
                            "[BUILD] Compiling {pkg_name} ({compiled}/{}, cross-target: {})",
                            if total == 0 {
                                "?".to_string()
                            } else {
                                total.to_string()
                            },
                            target.unwrap_or("host")
                        ),
                        Some(serde_json::json!({
                            "stage": "cargo_build_release",
                            "compiled": compiled,
                            "total": total,
                            "percent": percent,
                        })),
                    );
                }
            },
            | "compiler-message" => {
                let Some(message) = value.get("message") else {
                    continue;
                };
                let level = message.get("level").and_then(|l| l.as_str()).unwrap_or("");
                let Some(rendered) = message.get("rendered").and_then(|r| r.as_str()) else {
                    continue;
                };
                if level == "error" {
                    slide_core::logger::log_event("error", rendered, None);
                }
            },
            | _ => {},
        }
    }

    let status = child.wait()?;
    if !status.success() {
        return Err("Cargo compilation failed for single binary output".into());
    }
    Ok(())
}
