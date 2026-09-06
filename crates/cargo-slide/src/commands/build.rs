use slide_core::compiler::SlideCompiler;
use std::fs::create_dir_all;
use std::fs::write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

/// Compile the presentation into a standalone release binary
#[allow(clippy::too_many_lines)]
pub fn execute(
    file: &Path,
    output: Option<PathBuf>,
    animation: &str,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    if !file.exists() {
        return Err(format!("File does not exist: {}", file.display()).into());
    }

    slide_core::logger::log_event(
        "info",
        &format!(
            "📦 Packaging presentation into single binary: {}",
            file.display()
        ),
        Some(serde_json::json!({
            "stage": "package_start",
            "source_file": file.display().to_string(),
        })),
    );
    let compiler = SlideCompiler::new()?;
    let deck = compiler.compile_file(file)?;

    let stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("presentation");
    let default_name = if cfg!(windows) {
        format!("{stem}-presentation.exe")
    } else {
        format!("{stem}-presentation")
    };
    let target_binary = output.unwrap_or_else(|| PathBuf::from(default_name));

    slide_core::logger::log_event(
        "info",
        &format!(
            "⏳ Generating self-contained Rust bundle with {} slides...",
            deck.total_slides()
        ),
        Some(serde_json::json!({
            "stage": "bundle_generate",
            "total_slides": deck.total_slides(),
        })),
    );

    // Discover repository root for workspace dependencies
    let current_exe = std::env::current_exe().unwrap_or_default();
    let repo_root = find_repo_root(&current_exe)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    // Create temporary build project in target/.build_tmp if possible to avoid tmpfs size exhaustion
    let build_dir_parent = if repo_root.join("target").exists() {
        let p = repo_root.join("target/.build_tmp");
        let _ = create_dir_all(&p);
        p
    } else {
        std::env::temp_dir()
    };
    let build_dir = tempfile::Builder::new()
        .prefix("slide_build_")
        .tempdir_in(&build_dir_parent)
        .or_else(|_| tempdir())?;
    let build_path = build_dir.path();
    let src_dir = build_path.join("src");
    create_dir_all(&src_dir)?;

    // Serialize deck to JSON to be embedded via include_str!
    let deck_json = serde_json::to_string(&deck)?;
    write(build_path.join("deck.json"), deck_json)?;

    let core_path = repo_root.join("crates/slide-core");
    let player_path = repo_root.join("crates/slide-player");

    // Format paths with forward slashes for cross-platform TOML compatibility (avoids backslash escaping on Windows)
    let core_str = core_path.to_string_lossy().replace('\\', "/");
    let player_str = player_path.to_string_lossy().replace('\\', "/");

    let cargo_toml = format!(
        r#"[package]
name = "slide-standalone-runner"
version = "0.1.0"
edition = "2024"

[workspace]

[dependencies]
slide-core = {{ path = "{core_str}", version = "0.1.0" }}
slide-player = {{ path = "{player_str}", version = "0.1.0" }}
serde_json = "1.0"
"#
    );
    write(build_path.join("Cargo.toml"), cargo_toml)?;

    let runner_main = format!(
        r#"use slide_player::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let deck_json = include_str!("../deck.json");
    let deck: SlideDeck = serde_json::from_str(deck_json)?;

    SlideApp::from_deck(deck)
        .default_animation("{animation}")
        .run()?;

    Ok(())
}}
"#
    );
    write(src_dir.join("main.rs"), runner_main)?;

    slide_core::logger::log_event(
        "info",
        "🔨 Compiling release binary with Cargo...",
        Some(serde_json::json!({
            "stage": "cargo_build_release",
        })),
    );
    let mut cargo_cmd = Command::new("cargo");
    let target_dir = repo_root.join("target/standalone_target");
    cargo_cmd
        .arg("build")
        .arg("--release")
        .arg("--target-dir")
        .arg(&target_dir)
        .current_dir(build_path);

    let build_res = cargo_cmd.status()?;
    if !build_res.success() {
        return Err("Cargo compilation failed for single binary output".into());
    }

    let bin_filename = if cfg!(windows) {
        "slide-standalone-runner.exe"
    } else {
        "slide-standalone-runner"
    };
    let built_bin = target_dir.join("release").join(bin_filename);
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

    slide_core::logger::log_event(
        "success",
        &format!(
            "✅ Single binary generated successfully!\n\
             📍 Output: {} ({:.2} MB)\n\
             💡 You can now distribute and run this standalone presentation on any machine!",
            target_binary.display(),
            size_mb
        ),
        Some(serde_json::json!({
            "stage": "package_complete",
            "status": "success",
            "output": target_binary.display().to_string(),
            "size_mb": size_mb,
            "total_slides": deck.total_slides(),
        })),
    );

    Ok(())
}

fn find_repo_root(exe: &Path) -> Option<PathBuf> {
    // 1. Search upwards from current working directory
    if let Ok(cwd) = std::env::current_dir() {
        let mut cur = Some(cwd.as_path());
        while let Some(dir) = cur {
            if dir.join("crates").join("slide-core").exists() {
                return Some(dir.to_path_buf());
            }
            cur = dir.parent();
        }
    }

    // 2. Search upwards from executable location
    let mut cur = exe.parent();
    while let Some(dir) = cur {
        if dir.join("crates").join("slide-core").exists() {
            return Some(dir.to_path_buf());
        }
        cur = dir.parent();
    }

    None
}
