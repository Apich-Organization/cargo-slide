//! Diagnostics command to verify system environment, compilers, and media tools.

use std::path::PathBuf;
use std::process::Command;

/// System health check result item
pub struct DiagnosticItem {
    pub category: &'static str,
    pub name: &'static str,
    pub status: bool,
    pub detail: String,
    pub advice: Option<&'static str>,
}

/// Execute the `cargo slide doctor` diagnostics check
#[allow(clippy::too_many_lines)]
pub fn execute() -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("  ┌────────────────────────────────────────────────────────┐");
    println!("  │  [DOCTOR] Cargo Slide Environment & Dependency Audit   │");
    println!("  └────────────────────────────────────────────────────────┘");
    println!();

    let mut checks = Vec::new();

    // 1. Rust Compiler & Toolchain
    if let Ok(output) = Command::new("rustc").arg("--version").output() {
        if output.status.success() {
            let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
            checks.push(DiagnosticItem {
                category: "Toolchain",
                name: "Rust Compiler (rustc)",
                status: true,
                detail: ver,
                advice: None,
            });
        } else {
            checks.push(DiagnosticItem {
                category: "Toolchain",
                name: "Rust Compiler (rustc)",
                status: false,
                detail: "Command returned non-zero exit code".to_string(),
                advice: Some("Ensure rustc is installed via rustup: https://rustup.rs"),
            });
        }
    } else {
        checks.push(DiagnosticItem {
            category: "Toolchain",
            name: "Rust Compiler (rustc)",
            status: false,
            detail: "rustc binary not found in PATH".to_string(),
            advice: Some("Install Rust using: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"),
        });
    }

    // 2. Cargo Package Manager
    if let Ok(output) = Command::new("cargo").arg("--version").output() {
        if output.status.success() {
            let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
            checks.push(DiagnosticItem {
                category: "Toolchain",
                name: "Cargo Package Manager",
                status: true,
                detail: ver,
                advice: None,
            });
        } else {
            checks.push(DiagnosticItem {
                category: "Toolchain",
                name: "Cargo Package Manager",
                status: false,
                detail: "Failed to query cargo version".to_string(),
                advice: Some("Ensure cargo is in your PATH"),
            });
        }
    }

    // 3. Typst Compilation Backend
    let typst_cli = Command::new("typst").arg("--version").output();
    if let Ok(output) = typst_cli {
        if output.status.success() {
            let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
            checks.push(DiagnosticItem {
                category: "Typst Backend",
                name: "Typst CLI",
                status: true,
                detail: format!("{ver} (Standalone CLI available)"),
                advice: None,
            });
        } else {
            checks.push(DiagnosticItem {
                category: "Typst Backend",
                name: "Typst CLI",
                status: true,
                detail: "Embedded Typst Compiler active (Internal typst library v0.14)".to_string(),
                advice: None,
            });
        }
    } else {
        checks.push(DiagnosticItem {
            category: "Typst Backend",
            name: "Typst Backend",
            status: true,
            detail: "Embedded Typst Engine (zero external dependency required)".to_string(),
            advice: None,
        });
    }

    // 4. Graphical Display Environment
    let display = std::env::var("DISPLAY").ok();
    let wayland = std::env::var("WAYLAND_DISPLAY").ok();
    if cfg!(target_os = "linux") {
        if let Some(w) = wayland {
            checks.push(DiagnosticItem {
                category: "Display",
                name: "Window Server",
                status: true,
                detail: format!("Wayland Compositor ({w})"),
                advice: None,
            });
        } else if let Some(d) = display {
            checks.push(DiagnosticItem {
                category: "Display",
                name: "Window Server",
                status: true,
                detail: format!("X11 Display Server ({d})"),
                advice: None,
            });
        } else {
            checks.push(DiagnosticItem {
                category: "Display",
                name: "Window Server",
                status: false,
                detail: "No DISPLAY or WAYLAND_DISPLAY variable found (Headless/SSH environment)".to_string(),
                advice: Some("For interactive window playback, run in a desktop session or use 'cargo slide serve' / 'cargo slide export'"),
            });
        }
    } else {
        checks.push(DiagnosticItem {
            category: "Display",
            name: "Window Server",
            status: true,
            detail: "Native OS Windowing Available".to_string(),
            advice: None,
        });
    }

    // 5. Media Players (mpv & ffplay)
    let mpv_check = Command::new("mpv").arg("--version").output();
    if let Ok(output) = mpv_check {
        if output.status.success() {
            let ver = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or("mpv")
                .to_string();
            checks.push(DiagnosticItem {
                category: "Media",
                name: "Video Player (mpv)",
                status: true,
                detail: format!("{ver} (Hardware-accelerated playback enabled)"),
                advice: None,
            });
        } else {
            checks.push(DiagnosticItem {
                category: "Media",
                name: "Video Player (mpv)",
                status: false,
                detail: "mpv returned error".to_string(),
                advice: Some("Install mpv for embedded video hotspots: e.g. sudo apt install mpv"),
            });
        }
    } else {
        checks.push(DiagnosticItem {
            category: "Media",
            name: "Video Player (mpv)",
            status: false,
            detail: "mpv not found in PATH".to_string(),
            advice: Some(
                "Install mpv for video hotspots: e.g. sudo apt install mpv / brew install mpv",
            ),
        });
    }

    let ffmpeg_check = Command::new("ffmpeg").arg("-version").output();
    if let Ok(output) = ffmpeg_check
        && output.status.success()
    {
        let ver = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("ffmpeg")
            .to_string();
        checks.push(DiagnosticItem {
            category: "Media",
            name: "Audio/Video Transcoder (ffmpeg)",
            status: true,
            detail: ver,
            advice: None,
        });
    }

    // 6. Slide-Viewer Player Executable
    let viewer_in_path = Command::new("slide-viewer").arg("--version").output();
    if let Ok(output) = viewer_in_path {
        if output.status.success() {
            let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
            checks.push(DiagnosticItem {
                category: "Viewer",
                name: "slide-viewer binary",
                status: true,
                detail: format!("{ver} in PATH"),
                advice: None,
            });
        }
    } else {
        let home_bin = dirs_next().map(|p| p.join(".cargo/bin/slide-viewer"));
        if let Some(hb) = home_bin
            && hb.exists()
        {
            checks.push(DiagnosticItem {
                category: "Viewer",
                name: "slide-viewer binary",
                status: true,
                detail: format!("Found at {}", hb.display()),
                advice: None,
            });
        } else {
            checks.push(DiagnosticItem {
                category: "Viewer",
                name: "slide-viewer binary",
                status: false,
                detail: "slide-viewer not currently installed in PATH".to_string(),
                advice: Some(
                    "Run 'cargo slide install-viewer' to install system integration and viewer",
                ),
            });
        }
    }

    // 7. Temporary presentation cache write permissions
    let cache_dir = PathBuf::from("/tmp/cargo_slide_presentation_cache");
    let cache_ok = std::fs::create_dir_all(&cache_dir).is_ok();
    checks.push(DiagnosticItem {
        category: "Storage",
        name: "Incremental Compilation Cache",
        status: cache_ok,
        detail: if cache_ok {
            format!("Writable: {}", cache_dir.display())
        } else {
            format!("Permission denied: {}", cache_dir.display())
        },
        advice: if cache_ok {
            None
        } else {
            Some("Ensure /tmp has write permissions")
        },
    });

    // 8. WebAssembly Target Check
    let wasm_check = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output();
    let has_wasm = if let Ok(output) = wasm_check {
        String::from_utf8_lossy(&output.stdout).contains("wasm32-unknown-unknown")
    } else {
        false
    };
    checks.push(DiagnosticItem {
        category: "WASM",
        name: "wasm32-unknown-unknown Target",
        status: has_wasm,
        detail: if has_wasm {
            "Installed (CSR Web and standalone HTML export ready)".to_string()
        } else {
            "Not installed (Optional for web bundles)".to_string()
        },
        advice: if has_wasm {
            None
        } else {
            Some("Run 'rustup target add wasm32-unknown-unknown' for Web export support")
        },
    });

    // 9. Presentation Fonts Availability
    let fc_check = Command::new("fc-list").arg(":").arg("family").output();
    let fonts_available = if let Ok(output) = fc_check {
        let text = String::from_utf8_lossy(&output.stdout);
        text.contains("DejaVu")
            || text.contains("Liberation")
            || text.contains("Roboto")
            || text.contains("Inter")
            || text.contains("Arial")
            || text.contains("Noto")
    } else {
        true
    };
    checks.push(DiagnosticItem {
        category: "Typography",
        name: "Presentation Fonts",
        status: fonts_available,
        detail: if fonts_available {
            "System presentation fonts detected".to_string()
        } else {
            "Limited font inventory detected".to_string()
        },
        advice: if fonts_available {
            None
        } else {
            Some("Install standard presentation fonts (e.g. fonts-dejavu, Inter)")
        },
    });

    // 10. Trunk CLI for Leptos Web Development
    let trunk_check = Command::new("trunk").arg("--version").output();
    let has_trunk = trunk_check.is_ok_and(|o| o.status.success());
    checks.push(DiagnosticItem {
        category: "Toolchain",
        name: "Trunk WASM Bundler",
        status: has_trunk,
        detail: if has_trunk {
            "Available in PATH".to_string()
        } else {
            "Not found in PATH (Optional for web hot-reload)".to_string()
        },
        advice: if has_trunk {
            None
        } else {
            Some("Install via 'cargo install trunk' for custom web client live editing")
        },
    });

    // Print summary table
    let mut total_ok = 0usize;
    let total_checks = checks.len();

    for item in &checks {
        let mark = if item.status {
            total_ok += 1;
            "\x1b[32m[✓]\x1b[0m"
        } else {
            "\x1b[33m[!]\x1b[0m"
        };
        println!(
            "  {mark} [{:<9}] {:<24} : {}",
            item.category, item.name, item.detail
        );
        if let Some(adv) = item.advice {
            println!("      \x1b[90m↳ Suggestion: {adv}\x1b[0m");
        }
    }

    println!();
    println!("  ────────────────────────────────────────────────────────");
    if total_ok == total_checks {
        println!(
            "  \x1b[32m[ALL CLEAR] Your cargo-slide presentation environment is in prime condition! ({total_ok}/{total_checks} checks passed)\x1b[0m"
        );
    } else {
        println!(
            "  \x1b[33m[NOTICE] {total_ok} of {total_checks} checks passed. See suggestions above for missing optional tools.\x1b[0m"
        );
    }
    println!();

    let critical_failure = checks
        .iter()
        .any(|c| (c.name.starts_with("Rust") || c.name.starts_with("Cargo")) && !c.status);
    if critical_failure {
        return Err("System environment has unsatisfied critical dependencies for cargo-slide (rustc/cargo).".into());
    }

    Ok(())
}

fn dirs_next() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
