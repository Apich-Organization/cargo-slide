//! Built-in static web server for hosting Leptos CSR presentation bundles.

use slide_core::compiler::SlideCompiler;
use slide_core::model::SlideDeck;
use std::fs::File;
use std::fs::create_dir_all;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use tiny_http::Header;
use tiny_http::Response;
use tiny_http::Server;
use tiny_http::StatusCode;

// Embedded prebuilt Leptos CSR web application assets bundled in cargo-slide's own pkg/ directory
pub const INDEX_HTML: &str = include_str!("../../pkg/index.html");
pub const BOOTSTRAP_JS: &str = include_str!("../../pkg/bootstrap.js");
pub const STYLE_CSS: &str = include_str!("../../pkg/style.css");
pub const SLIDE_WEB_JS: &str = include_str!("../../pkg/slide_web.js");
pub const SLIDE_WEB_WASM: &[u8] = include_bytes!("../../pkg/slide_web_bg.wasm");

/// Configuration options for preparing Leptos CSR web bundles, including presentation protection.
#[derive(Debug, Clone, Default)]
pub struct CsrBundleOptions<'a> {
    /// Deploy web presentation in encrypted protection mode
    pub encrypt: bool,
    /// Authentication mode: "passphrase" (default) or "mnemonic"
    pub auth_mode: &'a str,
    /// Explicit passphrase for encryption
    pub passphrase: Option<&'a str>,
    /// Explicit BIP-39 mnemonic for encryption
    pub mnemonic: Option<&'a str>,
    /// Number of BIP-39 words to generate (12 or 24, default: 12)
    pub mnemonic_words: usize,
    /// Session token validity duration in hours (default: 24)
    pub token_ttl_hours: u64,
}

/// Prepare a standalone CSR distribution directory with all web player assets and `deck.json`.
#[allow(clippy::pedantic, clippy::nursery)]
pub fn prepare_csr_bundle(
    file: &Path,
    dest_dir: &Path,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    prepare_csr_bundle_with_options(file, dest_dir, &CsrBundleOptions::default())
}

/// Prepare a standalone CSR distribution directory with options for encryption protection.
#[allow(clippy::pedantic, clippy::nursery, clippy::too_many_lines)]
pub fn prepare_csr_bundle_with_options(
    file: &Path,
    dest_dir: &Path,
    options: &CsrBundleOptions<'_>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    create_dir_all(dest_dir)?;

    // Write Leptos CSR static player files
    std::fs::write(dest_dir.join("index.html"), INDEX_HTML)?;
    std::fs::write(dest_dir.join("bootstrap.js"), BOOTSTRAP_JS)?;
    std::fs::write(dest_dir.join("style.css"), STYLE_CSS)?;
    std::fs::write(dest_dir.join("slide_web.js"), SLIDE_WEB_JS)?;
    std::fs::write(dest_dir.join("slide_web_bg.wasm"), SLIDE_WEB_WASM)?;

    // Generate deck.json from input file and copy assets
    let deck_json_path = dest_dir.join("deck.json");
    let assets_dest = dest_dir.join("assets");

    let deck: SlideDeck = if file.extension().and_then(|e| e.to_str()) == Some("slide") {
        if slide_core::security::is_encrypted_package_file(file) {
            let secret = if let Some(p) = options.passphrase {
                p.to_string()
            } else if let Some(m) = options.mnemonic {
                m.to_string()
            } else {
                slide_core::security::read_masked_secret(
                    "Enter passphrase to decrypt .slide package for web export: ",
                )?
                .to_string()
            };
            let (d, _asset_store) =
                slide_core::package::unpack_encrypted_deck_with_secret_in_memory(file, &secret)?;
            d
        } else {
            let (deck, cache_dir) = slide_core::package::unpack_deck_and_assets(file)?;
            copy_dir_all(&cache_dir, dest_dir);
            deck
        }
    } else if file.extension().and_then(|e| e.to_str()) == Some("json") {
        let content = std::fs::read_to_string(file)?;
        let d: SlideDeck = serde_json::from_str(&content)?;
        if let Some(parent) = file.parent() {
            let src_assets = parent.join("assets");
            if src_assets.is_dir() {
                copy_dir_all(&src_assets, &assets_dest);
            }
        }
        d
    } else {
        let compiler = SlideCompiler::new()?;
        let d = compiler.compile_file(file)?;
        if let Some(parent) = file.parent() {
            let src_assets = parent.join("assets");
            if src_assets.is_dir() {
                copy_dir_all(&src_assets, &assets_dest);
            }

            // Copy any linked local relative files or media into dest_dir
            for slide in &d.slides {
                for hotspot in &slide.hotspots {
                    let candidate = match hotspot {
                        | slide_core::model::Hotspot::Link { target, .. } => {
                            let clean = target.strip_prefix('#').unwrap_or(target);
                            if clean.starts_with("chart:")
                                || clean.starts_with("video:")
                                || clean.starts_with("audio:")
                                || clean.starts_with("step:")
                                || clean.starts_with("transition:")
                                || clean.starts_with("mailto:")
                                || clean.contains("://")
                                || target.starts_with('#')
                            {
                                None
                            } else {
                                Some(target.strip_prefix("file://").unwrap_or(target))
                            }
                        },
                        | slide_core::model::Hotspot::Video { source, .. }
                        | slide_core::model::Hotspot::Audio { source, .. } => {
                            if source.contains("://") || source.starts_with('#') {
                                None
                            } else {
                                Some(source.strip_prefix("file://").unwrap_or(source))
                            }
                        },
                        | _ => None,
                    };

                    if let Some(raw_rel) = candidate {
                        let rel_clean = raw_rel.replace('\\', "/");
                        let src_path = parent.join(&rel_clean);
                        if src_path.is_file() {
                            let dst_path = dest_dir.join(&rel_clean);
                            if let Some(p) = dst_path.parent() {
                                let _ = create_dir_all(p);
                            }
                            let _ = std::fs::copy(&src_path, dst_path);
                        }
                    }
                }
            }
        }
        d
    };

    if options.encrypt {
        use base64::Engine as _;
        let b64 = &base64::engine::general_purpose::STANDARD;
        let is_mnemonic = options.auth_mode.eq_ignore_ascii_case("mnemonic");

        let (derived_key, gen_mnem, salt) = if is_mnemonic {
            let mnem_val = options.mnemonic.unwrap_or("");
            let (actual_mnem, generated) =
                if mnem_val.trim().is_empty() || mnem_val.trim() == "12" || mnem_val.trim() == "24"
                {
                    let count = if mnem_val.trim() == "24" {
                        24
                    } else if mnem_val.trim() == "12" {
                        12
                    } else if options.mnemonic_words == 24 {
                        24
                    } else {
                        12
                    };
                    let m = slide_core::security::passphrase::generate_bip39_mnemonic(count)?;
                    (m.clone(), Some(m))
                } else {
                    (zeroize::Zeroizing::new(mnem_val.to_string()), None)
                };

            let mut salt = [0u8; 32];
            rand::fill(&mut salt);
            let k = slide_core::security::passphrase::derive_key_from_mnemonic(
                &actual_mnem,
                &salt,
                100_000,
            )?;
            (k, generated, salt)
        } else {
            let pass = if let Some(p) = options.passphrase {
                p.to_string()
            } else if let Ok(env_pass) =
                std::env::var("SLIDE_PASSPHRASE").or_else(|_| std::env::var("SLIDE_SECRET"))
            {
                env_pass
            } else {
                slide_core::security::read_masked_secret("Enter passphrase for Web presentation: ")?
                    .to_string()
            };
            slide_core::security::validate_password_strength(&pass)?;

            let mut salt = [0u8; 32];
            rand::fill(&mut salt);
            let k = slide_core::security::passphrase::derive_key_from_passphrase(
                &pass, &salt, 100_000,
            )?;
            (k, None, salt)
        };

        if let Some(generated) = gen_mnem {
            let count = generated.split_whitespace().count();
            eprintln!();
            eprintln!(
                "================================================================================"
            );
            eprintln!(
                "[SECURITY] Generated {count}-Word BIP-39 Recovery Mnemonic for Web Presentation:"
            );
            eprintln!();
            eprintln!("    {}", generated.as_str());
            eprintln!();
            eprintln!(
                "[IMPORTANT] Write down these {count} words in order. Visitors must enter this"
            );
            eprintln!("            mnemonic phrase to decrypt and view the presentation.");
            eprintln!(
                "================================================================================"
            );
            eprintln!();
        }

        let deck_bytes = serde_json::to_vec(&deck)?;
        let (mut ciphertext, nonce, tag) =
            slide_core::security::crypto::aes_256_gcm_encrypt(&derived_key, &deck_bytes)?;
        ciphertext.extend_from_slice(&tag);

        let ttl_hours = if options.token_ttl_hours == 0 {
            24
        } else {
            options.token_ttl_hours
        };
        let envelope = serde_json::json!({
            "cargo_slide_protected": true,
            "auth_mode": if is_mnemonic { "mnemonic" } else { "passphrase" },
            "token_ttl_seconds": ttl_hours * 3600,
            "iterations": 100_000,
            "salt": b64.encode(salt),
            "nonce": b64.encode(nonce),
            "ciphertext": b64.encode(&ciphertext),
            "title": deck.title,
        });

        std::fs::write(&deck_json_path, serde_json::to_string_pretty(&envelope)?)?;
    } else {
        let json = serde_json::to_string_pretty(&deck)?;
        std::fs::write(deck_json_path, json)?;
    }

    Ok(())
}

fn copy_dir_all(
    src: &Path,
    dst: &Path,
) {
    let _ = create_dir_all(dst);
    for entry in walkdir::WalkDir::new(src).into_iter().flatten() {
        if entry.file_type().is_file()
            && let Ok(rel) = entry.path().strip_prefix(src)
        {
            let target = dst.join(rel);
            if let Some(p) = target.parent() {
                let _ = create_dir_all(p);
            }
            let _ = std::fs::copy(entry.path(), target);
        }
    }
}

/// Execute the `serve` command to launch an HTTP static web server.
#[allow(
    clippy::pedantic,
    clippy::nursery,
    clippy::too_many_lines,
    clippy::too_many_arguments
)]
pub fn execute(
    file: &Path,
    port: u16,
    ip: &str,
    dir: Option<PathBuf>,
    open_browser: bool,
    watch: bool,
    encrypt: bool,
    auth_mode: &str,
    passphrase: Option<&str>,
    mnemonic: Option<&str>,
    mnemonic_words: usize,
    token_ttl_hours: u64,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let resolved_file = if dir.is_some() || file.is_dir() {
        file.to_path_buf()
    } else {
        slide_core::compiler::resolve_presentation_target(file)?
    };
    let target = resolved_file.as_path();

    let serve_dir = if let Some(custom_dir) = dir {
        if !custom_dir.exists() {
            return Err(
                format!("Custom directory does not exist: {}", custom_dir.display()).into(),
            );
        }
        custom_dir
    } else if target.is_dir() {
        target.to_path_buf()
    } else {
        let stem = target
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("presentation");
        let dist = PathBuf::from("target").join(".web_serve").join(stem);
        let opts = CsrBundleOptions {
            encrypt,
            auth_mode,
            passphrase,
            mnemonic,
            mnemonic_words,
            token_ttl_hours,
        };
        prepare_csr_bundle_with_options(target, &dist, &opts)?;
        dist
    };

    if watch && !target.is_dir() {
        let watch_file = target.to_path_buf();
        let target_dist = serve_dir.clone();
        std::thread::spawn(move || {
            use notify::RecursiveMode;
            use notify::Watcher;
            let (tx, rx) = std::sync::mpsc::channel();
            let Ok(mut watcher) = notify::recommended_watcher(tx) else {
                return;
            };
            let watch_path = watch_file.parent().unwrap_or_else(|| Path::new("."));
            let _ = watcher.watch(watch_path, RecursiveMode::Recursive);

            let mut last_recompile = std::time::Instant::now();
            while let Ok(event) = rx.recv() {
                if let Ok(ev) = event {
                    let is_relevant = ev.paths.iter().any(|p| {
                        p.extension().is_some_and(|ext| {
                            ext == "typ" || ext == "csv" || ext == "png" || ext == "svg"
                        })
                    });
                    if is_relevant
                        && last_recompile.elapsed() > std::time::Duration::from_millis(500)
                    {
                        last_recompile = std::time::Instant::now();
                        println!(
                            "  [CHANGE] Change detected, recompiling web presentation bundle..."
                        );
                        if let Err(e) = prepare_csr_bundle(&watch_file, &target_dist) {
                            eprintln!("  [ERR] Recompilation failed: {e}");
                        } else {
                            println!("  [OK] Web presentation recompiled successfully!");
                        }
                    }
                }
            }
        });
    }

    let mut server_instance = None;
    let mut bind_addr = format!("{ip}:{port}");

    for offset in 0..10 {
        let candidate_port = port.saturating_add(offset);
        let candidate_addr = format!("{ip}:{candidate_port}");
        match Server::http(&candidate_addr) {
            | Ok(srv) => {
                if offset > 0 {
                    println!(
                        "  [WARN] Port {port} is in use, auto-fallback to port {candidate_port}..."
                    );
                }
                bind_addr = candidate_addr;
                server_instance = Some(srv);
                break;
            },
            | Err(e) => {
                if offset == 9 {
                    return Err(format!(
                        "Failed to bind static server after 10 attempts (ports {port}..={candidate_port}): {e}"
                    )
                    .into());
                }
            },
        }
    }
    let server = server_instance.expect("Server must be bound");

    let base_url = format!("http://{bind_addr}/");
    slide_core::logger::log_event(
        "info",
        &format!("[SERVER] Cargo Slide Web Server running at {base_url}"),
        Some(serde_json::json!({
            "stage": "server_started",
            "url": base_url,
            "serve_dir": serve_dir.display().to_string(),
            "watch": watch,
        })),
    );

    println!();
    println!("  ┌────────────────────────────────────────────────────────┐");
    println!("  │  [SERVER] Cargo Slide Static Web Server (Leptos CSR)   │");
    println!("  │                                                        │");
    println!("  │  URL:        http://{:<31}│", bind_addr);
    println!("  │  Directory:  {:<42}│", serve_dir.display());
    println!("  │  Mode:       Pure Rust WASM + CSR (Zero Inline JS)     │");
    if watch {
        println!("  │  Live Watch: Active (auto-recompiling on change)       │");
    }
    println!("  │                                                        │");
    println!("  │  Press Ctrl+C to terminate the server                  │");
    println!("  └────────────────────────────────────────────────────────┘");
    println!();

    if open_browser {
        let _ = open::that(&base_url);
    }

    for request in server.incoming_requests() {
        handle_http_request(request, &serve_dir);
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn handle_http_request(
    request: tiny_http::Request,
    root_dir: &Path,
) {
    let raw_url = request.url();
    let url_path = raw_url.split('?').next().unwrap_or("/");
    let trimmed_path = url_path.trim_start_matches('/');

    let relative_path = if trimmed_path.is_empty() {
        "index.html"
    } else {
        trimmed_path
    };

    let target_path = root_dir.join(relative_path);

    // Prevent directory traversal
    if target_path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        let response = Response::from_string("403 Forbidden").with_status_code(StatusCode(403));
        let _ = request.respond(response);
        return;
    }

    if target_path.exists() && target_path.is_file() {
        let Ok(mut file) = File::open(&target_path) else {
            let resp = Response::from_string("500 Internal Server Error")
                .with_status_code(StatusCode(500));
            let _ = request.respond(resp);
            return;
        };

        let mut data = Vec::new();
        if file.read_to_end(&mut data).is_err() {
            let resp = Response::from_string("500 Internal Server Error")
                .with_status_code(StatusCode(500));
            let _ = request.respond(resp);
            return;
        }

        let mime = get_mime_type(&target_path);
        let Ok(content_type_header) = Header::from_bytes(&b"Content-Type"[..], mime.as_bytes())
        else {
            let resp = Response::from_string("500 Header Error").with_status_code(StatusCode(500));
            let _ = request.respond(resp);
            return;
        };

        let Ok(cors_header) = Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..])
        else {
            let resp = Response::from_string("500 Header Error").with_status_code(StatusCode(500));
            let _ = request.respond(resp);
            return;
        };

        let Ok(accept_ranges_header) = Header::from_bytes(&b"Accept-Ranges"[..], &b"bytes"[..])
        else {
            let resp = Response::from_string("500 Header Error").with_status_code(StatusCode(500));
            let _ = request.respond(resp);
            return;
        };

        let Ok(cache_header) = Header::from_bytes(
            &b"Cache-Control"[..],
            &b"no-cache, no-store, must-revalidate"[..],
        ) else {
            let resp = Response::from_string("500 Header Error").with_status_code(StatusCode(500));
            let _ = request.respond(resp);
            return;
        };

        // Support HTTP Range requests (206 Partial Content) for streaming media (audio/video)
        let range_opt = request
            .headers()
            .iter()
            .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case("range"))
            .map(|h| h.value.as_str().to_string());

        let total_len = data.len();

        if let Some(range_val) = range_opt
            && let Some(spec) = range_val.strip_prefix("bytes=")
        {
            let parts: Vec<&str> = spec.split('-').collect();
            let start = parts
                .first()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            let end = parts
                .get(1)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or_else(|| total_len.saturating_sub(1))
                .min(total_len.saturating_sub(1));

            if start <= end && start < total_len {
                let sliced = &data[start..=end];
                let content_range = format!("bytes {start}-{end}/{total_len}");
                if let Ok(cr_header) =
                    Header::from_bytes(&b"Content-Range"[..], content_range.as_bytes())
                {
                    let response = Response::from_data(sliced.to_vec())
                        .with_status_code(StatusCode(206))
                        .with_header(content_type_header)
                        .with_header(cors_header)
                        .with_header(accept_ranges_header)
                        .with_header(cr_header)
                        .with_header(cache_header);
                    let _ = request.respond(response);
                    return;
                }
            }
        }

        let response = Response::from_data(data)
            .with_header(content_type_header)
            .with_header(cors_header)
            .with_header(accept_ranges_header)
            .with_header(cache_header);

        let _ = request.respond(response);
    } else {
        let not_found = Response::from_string("404 Not Found").with_status_code(StatusCode(404));
        let _ = request.respond(not_found);
    }
}

fn get_mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        | Some("html" | "htm") => "text/html; charset=utf-8",
        | Some("js" | "mjs") => "application/javascript; charset=utf-8",
        | Some("wasm") => "application/wasm",
        | Some("css") => "text/css; charset=utf-8",
        | Some("json") => "application/json; charset=utf-8",
        | Some("svg") => "image/svg+xml",
        | Some("png") => "image/png",
        | Some("jpg" | "jpeg") => "image/jpeg",
        | Some("gif") => "image/gif",
        | Some("ico") => "image/x-icon",
        | Some("wav") => "audio/wav",
        | Some("mp3") => "audio/mpeg",
        | Some("ogg") => "audio/ogg",
        | Some("flac") => "audio/flac",
        | Some("mp4" | "m4v") => "video/mp4",
        | Some("webm") => "video/webm",
        | Some("csv") => "text/csv; charset=utf-8",
        | Some("md" | "markdown") => "text/markdown; charset=utf-8",
        | Some("txt" | "rs" | "typ" | "toml" | "yaml" | "yml" | "sql" | "py") => {
            "text/plain; charset=utf-8"
        },
        | Some("db" | "sqlite") => "application/x-sqlite3",
        | _ => "application/octet-stream",
    }
}
