//! Command to unpack a .slide package into an editable project directory.

use slide_core::package::unpack_package_to_dir;
use std::path::Path;
use std::path::PathBuf;

/// Execute the `unpack` command.
///
/// Unpacks the compressed `.slide` package into a directory containing
/// the presentation slides, assets, and editable Typst source files.
#[allow(clippy::pedantic, clippy::nursery, clippy::cast_precision_loss)]
pub fn execute(
    file: &Path,
    output: Option<PathBuf>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    if !file.exists() {
        return Err(format!("Package file does not exist: {}", file.display()).into());
    }

    let stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("presentation_project");
    let out_dir = output.unwrap_or_else(|| PathBuf::from(stem));

    slide_core::logger::log_event(
        "info",
        &format!(
            "Unpacking .slide package into project directory: {}",
            out_dir.display()
        ),
        Some(serde_json::json!({
            "stage": "unpack_start",
            "package_file": file.display().to_string(),
            "output_dir": out_dir.display().to_string(),
        })),
    );

    let entrypoint = unpack_package_to_dir(file, &out_dir)?;

    slide_core::logger::log_event(
        "success",
        &format!(
            "Unpacked presentation project successfully to: {} (Entrypoint: {})",
            out_dir.display(),
            entrypoint.display()
        ),
        Some(serde_json::json!({
            "stage": "unpack_success",
            "output_dir": out_dir.display().to_string(),
            "entrypoint": entrypoint.display().to_string(),
        })),
    );

    println!(
        "Presentation unpacked to: {}\nPrimary presentation file: {}",
        out_dir.display(),
        entrypoint.display()
    );

    Ok(())
}
