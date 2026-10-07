//! Launch the Typora-style WYSIWYG desktop editor for presentations and documents.

use std::path::Path;
use std::process::Command;

/// Launch the slide-editor desktop app
pub fn execute(
    file: Option<&Path>,
    dark: bool,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    slide_core::logger::log_event(
        "info",
        "[EDITOR] Launching Slide Editor (Typora-style WYSIWYG)...",
        Some(serde_json::json!({
            "stage": "editor_launch",
            "file": file.map(|f| f.display().to_string()),
            "dark_mode": dark,
        })),
    );

    // 1. Try launching `slide-editor` directly from PATH or current exe's directory
    let current_exe = std::env::current_exe().ok();
    let sibling_editor = current_exe.as_ref().and_then(|p| p.parent()).map(|dir| {
        dir.join(if cfg!(windows) {
            "slide-editor.exe"
        } else {
            "slide-editor"
        })
    });

    let mut cmd = sibling_editor.filter(|p| p.is_file()).map_or_else(
        || {
            if Command::new("slide-editor")
                .arg("--version")
                .output()
                .is_ok_and(|o| o.status.success())
            {
                Command::new("slide-editor")
            } else {
                let mut c = Command::new("cargo");
                c.args(["run", "--quiet", "-p", "slide-editor", "--"]);
                c
            }
        },
        Command::new,
    );

    let resolved_path =
        file.and_then(|f| slide_core::compiler::resolve_presentation_target(f).ok());
    let target = resolved_path.as_deref().or(file);

    if let Some(f) = target {
        cmd.arg(f);
    }

    if dark {
        cmd.arg("--dark");
    }

    let status = cmd.status()?;
    if !status.success() {
        return Err(format!("slide-editor exited with status {status}").into());
    }

    Ok(())
}
