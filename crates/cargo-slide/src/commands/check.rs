//! Command to audit presentation health, check formatting, density, and missing assets.

use slide_core::compiler::HealthSeverity;
use slide_core::compiler::SlideCompiler;
use slide_core::compiler::check_presentation_health;
use std::path::Path;

/// Run presentation health check and report issues
#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
pub fn execute(file: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !file.exists() {
        return Err(format!("File does not exist: {}", file.display()).into());
    }

    println!();
    println!("  ┌────────────────────────────────────────────────────────┐");
    println!("  │  🩺 Cargo Slide Presentation Health Audit              │");
    println!("  └────────────────────────────────────────────────────────┘");
    println!();

    let source_text = std::fs::read_to_string(file)?;
    let assets_dir = file.parent().map(|p| p.join("assets"));

    let compiler = SlideCompiler::new()?;
    let deck = match compiler.compile_file(file) {
        | Ok(d) => d,
        | Err(e) => {
            eprintln!("  ✕ Compilation Error: {e}");
            return Err(e.into());
        },
    };

    let issues = check_presentation_health(&deck, &source_text, assets_dir.as_deref());

    let total_slides = deck.total_slides();
    let total_sec = deck.total_speaking_seconds();
    let total_notes_words: usize = deck
        .slides
        .iter()
        .map(slide_core::model::Slide::notes_word_count)
        .sum();

    println!("  Presentation: {}", deck.title);
    println!("  Slides:       {total_slides} slides");
    println!("  Est. Talk:    ~{:.1} min pacing", total_sec as f64 / 60.0);
    println!("  Notes Words:  {total_notes_words} words");
    println!();

    if issues.is_empty() {
        println!("  ✓ Excellent! No presentation health issues detected.");
        println!("  ✓ Media paths, slide titles, imports, and element density look clean.");
        println!();
        return Ok(());
    }

    let mut warnings_count = 0usize;
    let mut errors_count = 0usize;

    for issue in &issues {
        match issue.severity {
            | HealthSeverity::Error => errors_count = errors_count.saturating_add(1),
            | HealthSeverity::Warning | HealthSeverity::Info => {
                warnings_count = warnings_count.saturating_add(1);
            },
        }

        let (icon, prefix) = match issue.severity {
            | HealthSeverity::Error => ("✕", "ERROR"),
            | HealthSeverity::Warning => ("⚠️", "WARN "),
            | HealthSeverity::Info => ("ℹ️", "INFO "),
        };

        let slide_label = issue.slide_index.map_or_else(String::new, |idx| {
            format!(" [Slide {}]", idx.saturating_add(1))
        });

        println!("  {} {} {}{}", icon, prefix, issue.message, slide_label);
        if let Some(ref sug) = issue.suggestion {
            println!("     👉 Suggestion: {sug}");
        }
        println!();
    }

    println!(
        "  Health Summary: {errors_count} error(s), {warnings_count} warning(s) across {total_slides} slides"
    );
    println!();

    if errors_count > 0 {
        return Err(
            format!("Health audit detected {errors_count} critical presentation issue(s)").into(),
        );
    }

    Ok(())
}
