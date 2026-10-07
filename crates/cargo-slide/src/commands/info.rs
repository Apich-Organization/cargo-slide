//! Command to inspect presentation packages and Typst presentation files.

use serde_json::json;
use slide_core::compiler::SlideCompiler;
use slide_core::package::read_package_metadata;
use slide_core::package::verify_package_integrity;
use std::path::Path;

/// Inspect and display presentation metadata and integrity
#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
pub fn execute(
    file: &Path,
    json_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let resolved_file = slide_core::compiler::resolve_presentation_target(file)?;
    let target = resolved_file.as_path();

    let is_slide_pkg = target.extension().and_then(|e| e.to_str()) == Some("slide");

    if is_slide_pkg {
        let meta = read_package_metadata(target)?;
        let file_size = std::fs::metadata(target).map_or(0, |m| m.len());
        let integrity = verify_package_integrity(target);

        if json_mode {
            let integrity_status = match &integrity {
                | Ok(rep) => {
                    json!({
                        "passed": true,
                        "verified_slides": rep.slide_count,
                        "embedded_assets": rep.asset_count,
                        "warnings": rep.warnings,
                    })
                },
                | Err(e) => {
                    json!({
                        "passed": false,
                        "error": e.to_string(),
                    })
                },
            };

            let out = json!({
                "type": "slide_package",
                "path": target.display().to_string(),
                "title": meta.title,
                "format_version": meta.format_version,
                "compression": meta.compression,
                "total_slides": meta.total_slides,
                "aspect_ratio": meta.aspect_ratio.as_deref().unwrap_or("16:9"),
                "author": meta.author,
                "created_at": meta.created_at,
                "has_notes": meta.has_notes,
                "tags": meta.tags,
                "file_size_bytes": file_size,
                "uncompressed_bytes": meta.uncompressed_size,
                "checksum": meta.checksum,
                "integrity": integrity_status,
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
            return Ok(());
        }

        println!();
        println!("  ┌────────────────────────────────────────────────────────┐");
        println!("  │  [INSPECTOR] Cargo Slide Presentation Inspector        │");
        println!("  └────────────────────────────────────────────────────────┘");
        println!();

        println!("  Package:          {}", target.display());
        println!("  Title:            {}", meta.title);
        println!(
            "  Format:           cargo-slide (v{}, {})",
            meta.format_version, meta.compression
        );
        println!("  Slide Count:      {} slides", meta.total_slides);
        println!(
            "  Aspect Ratio:     {}",
            meta.aspect_ratio.as_deref().unwrap_or("16:9")
        );
        println!(
            "  Author:           {}",
            meta.author.as_deref().unwrap_or("Unknown")
        );
        println!(
            "  Created At:       {}",
            meta.created_at.as_deref().unwrap_or("Not specified")
        );
        println!(
            "  Speaker Notes:    {}",
            if meta.has_notes {
                "Included"
            } else {
                "None"
            }
        );
        if !meta.tags.is_empty() {
            println!("  Tags:             {}", meta.tags.join(", "));
        }
        println!(
            "  File Size:        {:.2} KB ({} bytes)",
            file_size as f64 / 1024.0,
            file_size
        );
        if let Some(ref cs) = meta.checksum {
            println!("  Checksum:         {cs}");
        }
        if let Some(unc_size) = meta.uncompressed_size {
            let ratio = if unc_size > 0 {
                (1.0 - (file_size as f64 / unc_size as f64)) * 100.0
            } else {
                0.0
            };
            println!(
                "  Uncompressed:     {:.2} KB ({:.1}% compression savings)",
                unc_size as f64 / 1024.0,
                ratio.max(0.0)
            );
        }
        println!();

        println!("  Running deep integrity verification...");
        match integrity {
            | Ok(report) => {
                println!("  [OK] Integrity:      PASSED (Structural LZMA2 TAR is valid)");
                println!("  [OK] Verified Slides: {} slides", report.slide_count);
                println!("  [OK] Embedded Assets: {} files", report.asset_count);
                if !report.warnings.is_empty() {
                    println!("  [WARN] Warnings:");
                    for w in &report.warnings {
                        println!("     - {w}");
                    }
                }
            },
            | Err(e) => {
                println!("  [ERR] Integrity:      FAILED ({e})");
                return Err(format!("Package integrity check failed: {e}").into());
            },
        }
    } else {
        let compiler = SlideCompiler::new()?;
        let deck = compiler.compile_file(target)?;
        let file_size = std::fs::metadata(target).map_or(0, |m| m.len());

        let total_words: usize = deck
            .slides
            .iter()
            .map(slide_core::model::Slide::notes_word_count)
            .sum();
        let total_sec = deck.total_speaking_seconds();
        let total_hotspots: usize = deck.slides.iter().map(|s| s.hotspots.len()).sum();
        let total_steps: usize = deck.slides.iter().map(|s| s.steps.len()).sum();

        if json_mode {
            let out = json!({
                "type": "typst_source",
                "path": target.display().to_string(),
                "title": deck.title,
                "total_slides": deck.total_slides(),
                "default_animation": deck.default_animation,
                "has_notes": deck.has_any_notes(),
                "notes_words": total_words,
                "speaking_seconds": total_sec,
                "speaking_minutes": total_sec as f64 / 60.0,
                "total_hotspots": total_hotspots,
                "total_step_fragments": total_steps,
                "file_size_bytes": file_size,
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
            return Ok(());
        }

        println!();
        println!("  ┌────────────────────────────────────────────────────────┐");
        println!("  │  [INSPECTOR] Cargo Slide Presentation Inspector        │");
        println!("  └────────────────────────────────────────────────────────┘");
        println!();

        println!("  Typst Source:     {}", target.display());
        println!("  Title:            {}", deck.title);
        println!("  Slide Count:      {} slides", deck.total_slides());
        println!("  Animation:        {}", deck.default_animation);
        println!(
            "  Has Notes:        {}",
            if deck.has_any_notes() {
                "Yes"
            } else {
                "No"
            }
        );
        println!(
            "  Est. Talk:        ~{:.1} minutes ({:.0}s)",
            total_sec as f64 / 60.0,
            total_sec
        );
        println!("  Notes Words:      {total_words} words total");
        println!("  Media Hotspots:   {total_hotspots} interactive link/media targets");
        if total_steps > 0 {
            println!("  Step Fragments:   {total_steps} progressive reveals");
        }
        println!(
            "  Source File Size: {:.2} KB ({} bytes)",
            file_size as f64 / 1024.0,
            file_size
        );
    }
    println!();
    Ok(())
}
