//! Command to inspect presentation packages and Typst presentation files.

use slide_core::compiler::SlideCompiler;
use slide_core::package::read_package_metadata;
use slide_core::package::verify_package_integrity;
use std::path::Path;

/// Inspect and display presentation metadata and integrity
#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
pub fn execute(file: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !file.exists() {
        return Err(format!("File does not exist: {}", file.display()).into());
    }

    let is_slide_pkg = file.extension().and_then(|e| e.to_str()) == Some("slide");

    println!();
    println!("  ┌────────────────────────────────────────────────────────┐");
    println!("  │  🔍 Cargo Slide Presentation Inspector                 │");
    println!("  └────────────────────────────────────────────────────────┘");
    println!();

    if is_slide_pkg {
        let meta = read_package_metadata(file)?;
        let file_size = std::fs::metadata(file).map_or(0, |m| m.len());

        println!("  📄 Package:       {}", file.display());
        println!("  🏷️  Title:         {}", meta.title);
        println!(
            "  📦 Format:        cargo-slide (v{}, {})",
            meta.format_version, meta.compression
        );
        println!("  🔢 Slide Count:   {} slides", meta.total_slides);
        println!(
            "  📐 Aspect Ratio:  {}",
            meta.aspect_ratio.as_deref().unwrap_or("16:9")
        );
        println!(
            "  👤 Author:        {}",
            meta.author.as_deref().unwrap_or("Unknown")
        );
        println!(
            "  📅 Created At:    {}",
            meta.created_at.as_deref().unwrap_or("Not specified")
        );
        println!(
            "  📝 Speaker Notes: {}",
            if meta.has_notes {
                "Included"
            } else {
                "None"
            }
        );
        if !meta.tags.is_empty() {
            println!("  🏷️  Tags:          {}", meta.tags.join(", "));
        }
        println!(
            "  💾 File Size:     {:.2} KB ({} bytes)",
            file_size as f64 / 1024.0,
            file_size
        );
        println!();

        println!("  Running deep integrity verification...");
        match verify_package_integrity(file) {
            | Ok(report) => {
                println!("  ✓ Integrity:      PASSED (Structural LZMA2 TAR is valid)");
                println!("  ✓ Verified Slides: {} slides", report.slide_count);
                println!("  ✓ Embedded Assets: {} files", report.asset_count);
                if !report.warnings.is_empty() {
                    println!("  ⚠️  Warnings:");
                    for w in &report.warnings {
                        println!("     - {w}");
                    }
                }
            },
            | Err(e) => {
                println!("  ✕ Integrity:      FAILED ({e})");
                return Err(format!("Package integrity check failed: {e}").into());
            },
        }
    } else {
        println!("  📄 Typst Source:  {}", file.display());
        let compiler = SlideCompiler::new()?;
        let deck = compiler.compile_file(file)?;

        println!("  🏷️  Title:         {}", deck.title);
        println!("  🔢 Slide Count:   {} slides", deck.total_slides());
        println!("  🎬 Animation:     {}", deck.default_animation);
        println!(
            "  📝 Has Notes:     {}",
            if deck.has_any_notes() {
                "Yes"
            } else {
                "No"
            }
        );
        let total_sec = deck.total_speaking_seconds();
        println!(
            "  ⏱️  Est. Talk:     ~{:.1} minutes ({:.0}s)",
            total_sec as f64 / 60.0,
            total_sec
        );

        let total_words: usize = deck
            .slides
            .iter()
            .map(slide_core::model::Slide::notes_word_count)
            .sum();
        println!("  💬 Notes Words:   {total_words} words total");
    }
    println!();
    Ok(())
}
