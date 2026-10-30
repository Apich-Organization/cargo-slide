//! Cross-platform multi-architecture compilation engine for cargo-slide.
//!
//! Provides first-class support for compiling presentations, workspace tools,
//! and standalone binaries across desktop operating systems (Linux, Windows)
//! and architectures (`x86_64`, `aarch64`) using Zig (`cargo-zigbuild`) or Cross.

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

/// Standard desktop cross-compilation targets supported by cargo-slide.
pub const SUPPORTED_CROSS_TARGETS: &[(&str, &str)] = &[
    ("x86_64-unknown-linux-gnu", "linux-x86_64"),
    ("aarch64-unknown-linux-gnu", "linux-aarch64"),
    ("x86_64-pc-windows-gnu", "windows-x86_64"),
    ("aarch64-pc-windows-gnullvm", "windows-aarch64"),
];

/// Specification of a cross-compilation target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetSpec {
    /// Full Rust target triple (e.g. `x86_64-pc-windows-gnu`)
    pub triple: String,
    /// Human-friendly target platform label (e.g. `windows-x86_64`)
    pub label: String,
}

/// Normalize target aliases (e.g. 'windows-x86_64', 'win64') to Rust triples.
#[must_use]
pub fn normalize_target_triple(target: &str) -> &str {
    match target.to_lowercase().as_str() {
        | "windows-x86_64" | "win64" | "windows64" | "x86_64-windows" => "x86_64-pc-windows-gnu",
        | "windows-aarch64" | "windows-arm64" | "win-arm64" | "aarch64-windows" => {
            "aarch64-pc-windows-gnullvm"
        },
        | "linux-x86_64" | "linux64" | "x86_64-linux" => "x86_64-unknown-linux-gnu",
        | "linux-aarch64" | "linux-arm64" | "aarch64-linux" => "aarch64-unknown-linux-gnu",
        | "linux-x86_64-musl" | "musl64" => "x86_64-unknown-linux-musl",
        | "linux-aarch64-musl" | "musl-arm64" => "aarch64-unknown-linux-musl",
        | _ => target,
    }
}

/// Resolve user-supplied target argument into a list of [`TargetSpec`].
#[must_use]
pub fn resolve_targets(target_req: &str) -> Vec<TargetSpec> {
    match target_req.to_lowercase().as_str() {
        | "all" => {
            SUPPORTED_CROSS_TARGETS
                .iter()
                .map(|&(triple, label)| {
                    TargetSpec {
                        triple: triple.to_string(),
                        label: label.to_string(),
                    }
                })
                .collect()
        },
        | "windows-x86_64" | "win64" | "windows64" | "x86_64-windows" => {
            vec![TargetSpec {
                triple: "x86_64-pc-windows-gnu".to_string(),
                label: "windows-x86_64".to_string(),
            }]
        },
        | "windows-aarch64" | "windows-arm64" | "win-arm64" | "aarch64-windows" => {
            vec![TargetSpec {
                triple: "aarch64-pc-windows-gnullvm".to_string(),
                label: "windows-aarch64".to_string(),
            }]
        },
        | "linux-x86_64" | "linux64" | "x86_64-linux" => {
            vec![TargetSpec {
                triple: "x86_64-unknown-linux-gnu".to_string(),
                label: "linux-x86_64".to_string(),
            }]
        },
        | "linux-aarch64" | "linux-arm64" | "aarch64-linux" => {
            vec![TargetSpec {
                triple: "aarch64-unknown-linux-gnu".to_string(),
                label: "linux-aarch64".to_string(),
            }]
        },
        | "linux-x86_64-musl" | "musl64" => {
            vec![TargetSpec {
                triple: "x86_64-unknown-linux-musl".to_string(),
                label: "linux-x86_64-musl".to_string(),
            }]
        },
        | "linux-aarch64-musl" | "musl-arm64" => {
            vec![TargetSpec {
                triple: "aarch64-unknown-linux-musl".to_string(),
                label: "linux-aarch64-musl".to_string(),
            }]
        },
        | other => {
            for &(triple, label) in SUPPORTED_CROSS_TARGETS {
                if other == triple {
                    return vec![TargetSpec {
                        triple: triple.to_string(),
                        label: label.to_string(),
                    }];
                }
            }
            let label = other.replace(['/', '\\'], "-");
            vec![TargetSpec {
                triple: other.to_string(),
                label,
            }]
        },
    }
}

/// Platform-agnostic command lookup in PATH (handles Windows .exe, .cmd, .bat as well as Unix binaries).
#[must_use]
pub fn which_cmd(cmd: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        for p in std::env::split_paths(&path) {
            if p.join(cmd).is_file() {
                return true;
            }
            if p.join(format!("{cmd}.exe")).is_file() {
                return true;
            }
            if p.join(format!("{cmd}.cmd")).is_file() {
                return true;
            }
            if p.join(format!("{cmd}.bat")).is_file() {
                return true;
            }
        }
    }
    false
}

/// Determines whether a specified target triple represents a cross-compilation from this host.
#[must_use]
pub fn is_cross_target(target_triple: &str) -> bool {
    let host_arch = std::env::consts::ARCH;
    let host_os = std::env::consts::OS;
    let os_pattern = if host_os == "macos" {
        "darwin"
    } else {
        host_os
    };

    if !target_triple.starts_with(host_arch) {
        return true;
    }
    if !target_triple.contains(os_pattern) {
        return true;
    }
    #[cfg(target_env = "msvc")]
    if !target_triple.contains("msvc") {
        return true;
    }
    #[cfg(target_env = "gnu")]
    if !target_triple.contains("gnu") {
        return true;
    }
    #[cfg(target_env = "musl")]
    if !target_triple.contains("musl") {
        return true;
    }
    false
}

/// Ensure a Rust compilation target is installed via rustup.
pub fn ensure_target_installed(triple: &str) {
    if which_cmd("rustup")
        && let Ok(output) = Command::new("rustup")
            .args(["target", "list", "--installed"])
            .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.lines().any(|l| l.trim() == triple) {
            slide_core::logger::log_event(
                "info",
                &format!("[SETUP] Adding missing Rust target via rustup: {triple}"),
                None,
            );
            let _ = Command::new("rustup")
                .args(["target", "add", triple])
                .status();
        }
    }
}

/// Determine the compilation backend tool to use (`zig`, `cross`, or `cargo`).
#[must_use]
pub fn select_backend(
    triple: &str,
    requested_tool: &str,
) -> &'static str {
    match requested_tool {
        | "zig" | "cargo-zigbuild" => "zig",
        | "cross" => "cross",
        | "cargo" | "native" => "cargo",
        | _ => {
            let host_is_windows = cfg!(target_os = "windows");
            if host_is_windows || triple.contains("windows") {
                if which_cmd("cargo-zigbuild") || which_cmd("zig") {
                    "zig"
                } else if which_cmd("cross") {
                    "cross"
                } else {
                    "cargo"
                }
            } else if which_cmd("cross") {
                "cross"
            } else if which_cmd("cargo-zigbuild") || which_cmd("zig") {
                "zig"
            } else {
                "cargo"
            }
        },
    }
}

/// Compile a single package or binary for the given target specification.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
fn compile_package_for_target(
    target: &TargetSpec,
    bin_name: &str,
    package_name: Option<&str>,
    tool: &str,
    output_dir: &Path,
    dev: bool,
    repo_root: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let is_windows = target.triple.contains("windows");
    let ext = if is_windows { ".exe" } else { "" };
    let profile = if dev { "debug" } else { "release" };

    slide_core::logger::log_event(
        "info",
        &format!(
            "[CROSS] Building [{bin_name}] for [{}] ({})",
            target.label, target.triple
        ),
        None,
    );

    ensure_target_installed(&target.triple);
    let selected_backend = select_backend(&target.triple, tool);

    let mut cmd = if selected_backend == "cross" {
        Command::new("cross")
    } else {
        Command::new("cargo")
    };

    if selected_backend == "zig" {
        cmd.arg("zigbuild");
    } else {
        cmd.arg("build");
    }

    if !dev {
        cmd.arg("--release");
    }

    cmd.arg("--target").arg(&target.triple);

    if let Some(pkg) = package_name {
        cmd.arg("-p").arg(pkg);
    } else {
        cmd.arg("--bin").arg(bin_name);
    }

    cmd.current_dir(repo_root);

    if selected_backend == "cross"
        && std::env::var("CROSS_CONTAINER_ENGINE").is_err()
        && which_cmd("podman")
        && !which_cmd("docker")
    {
        cmd.env("CROSS_CONTAINER_ENGINE", "podman");
    }

    let status = cmd.status()?;
    if !status.success() {
        return Err(format!(
            "Compilation failed for target {} ({}) with backend {selected_backend}",
            target.label, target.triple
        )
        .into());
    }

    let built_file = repo_root
        .join("target")
        .join(&target.triple)
        .join(profile)
        .join(format!("{bin_name}{ext}"));

    if !built_file.exists() {
        return Err(format!("Built artifact not found at: {}", built_file.display()).into());
    }

    let clean_bin_name = Path::new(bin_name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(bin_name);
    let dest_name = format!("{clean_bin_name}-{}{ext}", target.label);
    let dest_file = output_dir.join(dest_name);
    fs::copy(&built_file, &dest_file)?;

    let size_bytes = fs::metadata(&dest_file).map_or(0, |m| m.len());
    #[allow(clippy::cast_precision_loss)]
    let size_mb = size_bytes as f64 / 1_048_576.0;

    slide_core::logger::log_event(
        "success",
        &format!(
            "[OK] Successfully built: {} ({:.2} MB)",
            dest_file.display(),
            size_mb
        ),
        None,
    );

    Ok(dest_file)
}

/// Compile a presentation for the given target specification.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
fn compile_presentation_for_target(
    target: &TargetSpec,
    presentation_file: &Path,
    tool: &str,
    output_dir: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let is_windows = target.triple.contains("windows");
    let ext = if is_windows { ".exe" } else { "" };
    let stem = presentation_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("presentation");
    let dest_file = output_dir.join(format!("{stem}-{}{ext}", target.label));

    slide_core::logger::log_event(
        "info",
        &format!(
            "[CROSS] Building presentation for [{}] ({}) -> {}",
            target.label,
            target.triple,
            dest_file.display()
        ),
        None,
    );

    ensure_target_installed(&target.triple);

    crate::commands::build::execute_binary(
        presentation_file,
        Some(dest_file.clone()),
        "fade",
        Some(&target.triple),
        true,
        tool,
        false,
        "piv",
        None,
        None,
        12,
        "9d",
        "9c",
        None,
        None,
    )?;

    Ok(dest_file)
}

/// Execute the `cross` command.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub fn execute(
    target_req: &str,
    bin: Option<&str>,
    package: Option<&str>,
    file: Option<&Path>,
    tool: &str,
    output_dir: &Path,
    dev: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output_dir)?;
    let targets = resolve_targets(target_req);
    if targets.is_empty() {
        return Err(format!("No matching targets found for '{target_req}'").into());
    }

    let is_presentation = file.map_or_else(
        || bin.is_none() && package.is_none() && Path::new("slides.typ").exists(),
        Path::exists,
    );

    let mut built_artifacts = Vec::new();

    if is_presentation {
        let pres_path = file.unwrap_or_else(|| Path::new("slides.typ"));
        slide_core::logger::log_event(
            "info",
            &format!(
                "[CROSS] Starting cross-compilation for presentation: {} (targets: {})",
                pres_path.display(),
                targets.len()
            ),
            None,
        );

        for target in &targets {
            let artifact = compile_presentation_for_target(target, pres_path, tool, output_dir)?;
            built_artifacts.push(artifact);
        }
    } else {
        let current_exe = std::env::current_exe().unwrap_or_default();
        let repo_root = crate::commands::build::find_repo_root(&current_exe)
            .or_else(|| {
                std::env::current_dir()
                    .ok()
                    .filter(|cwd| cwd.join("Cargo.toml").exists())
            })
            .unwrap_or_else(|| PathBuf::from("."));

        let resolved_bin = bin.or(package).unwrap_or("cargo-slide");

        slide_core::logger::log_event(
            "info",
            &format!(
                "[CROSS] Starting cross-compilation for binary [{resolved_bin}] (targets: {})",
                targets.len()
            ),
            None,
        );

        for target in &targets {
            let artifact = compile_package_for_target(
                target,
                resolved_bin,
                package,
                tool,
                output_dir,
                dev,
                &repo_root,
            )?;
            built_artifacts.push(artifact);
        }
    }

    println!("\n================================================================================");
    println!("  All requested cross-platform builds completed successfully!");
    println!("  Output directory: {}", output_dir.display());
    for art in &built_artifacts {
        let size = fs::metadata(art).map_or(0, |m| m.len());
        #[allow(clippy::cast_precision_loss)]
        let size_mb = size as f64 / 1_048_576.0;
        println!("    - {} ({:.2} MB)", art.display(), size_mb);
    }
    println!("================================================================================");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_target_triples_and_extensions() {
        for target in resolve_targets("all") {
            let is_win = target.triple.contains("windows");
            let ext = if is_win { ".exe" } else { "" };
            if is_win {
                assert_eq!(ext, ".exe");
                assert!(target.label.starts_with("windows-"));
            } else {
                assert_eq!(ext, "");
                assert!(target.label.starts_with("linux-"));
            }
        }
    }

    #[test]
    fn test_normalize_target_triple_aliases() {
        assert_eq!(normalize_target_triple("win64"), "x86_64-pc-windows-gnu");
        assert_eq!(
            normalize_target_triple("windows-x86_64"),
            "x86_64-pc-windows-gnu"
        );
        assert_eq!(
            normalize_target_triple("windows-aarch64"),
            "aarch64-pc-windows-gnullvm"
        );
        assert_eq!(
            normalize_target_triple("win-arm64"),
            "aarch64-pc-windows-gnullvm"
        );
        assert_eq!(
            normalize_target_triple("linux-x86_64"),
            "x86_64-unknown-linux-gnu"
        );
        assert_eq!(
            normalize_target_triple("linux64"),
            "x86_64-unknown-linux-gnu"
        );
        assert_eq!(
            normalize_target_triple("linux-aarch64"),
            "aarch64-unknown-linux-gnu"
        );
        assert_eq!(
            normalize_target_triple("linux-arm64"),
            "aarch64-unknown-linux-gnu"
        );
    }

    #[test]
    fn test_resolve_targets_windows_and_linux() {
        let win_targets = resolve_targets("windows-x86_64");
        assert_eq!(win_targets.len(), 1);
        assert_eq!(win_targets[0].triple, "x86_64-pc-windows-gnu");
        assert!(win_targets[0].triple.contains("windows"));

        let linux_targets = resolve_targets("linux-x86_64");
        assert_eq!(linux_targets.len(), 1);
        assert_eq!(linux_targets[0].triple, "x86_64-unknown-linux-gnu");
        assert!(!linux_targets[0].triple.contains("windows"));

        let all = resolve_targets("all");
        assert_eq!(all.len(), 4);
        assert!(all.iter().any(|t| t.triple.contains("windows")));
        assert!(all.iter().any(|t| t.triple.contains("linux")));
    }
}
