//! Command to audit presentation health, check formatting, density, and missing assets.

use slide_core::compiler::HealthSeverity;
use slide_core::compiler::SlideCompiler;
use slide_core::compiler::check_presentation_health;
use std::path::Path;

/// Run presentation health check and report issues
#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
pub fn execute(
    file: &Path,
    target_minutes: Option<u64>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let resolved_path = slide_core::compiler::resolve_presentation_target(file)?;
    let file = resolved_path.as_path();

    if json {
        slide_core::logger::set_silent(true);
    }

    let source_text = std::fs::read_to_string(file)?;
    let assets_dir = file.parent().map(|p| p.join("assets"));

    let compiler = SlideCompiler::new()?;
    let deck = match compiler.compile_file(file) {
        | Ok(d) => d,
        | Err(e) => {
            if json {
                eprintln!(
                    "{}",
                    serde_json::json!({
                        "status": "error",
                        "error": e.to_string(),
                    })
                );
            } else {
                eprintln!("  [ERR] Compilation Error: {e}");
            }
            return Err(e.into());
        },
    };

    let mut issues = check_presentation_health(&deck, &source_text, assets_dir.as_deref());

    let total_slides = deck.total_slides();
    let total_sec = deck.total_speaking_seconds();
    let total_notes_words: usize = deck
        .slides
        .iter()
        .map(slide_core::model::Slide::notes_word_count)
        .sum();

    // Audit against target duration if requested
    if let Some(target_mins) = target_minutes {
        let target_sec = target_mins.saturating_mul(60) as usize;
        if total_sec > target_sec {
            let over_mins = (total_sec.saturating_sub(target_sec)) as f64 / 60.0;
            issues.push(slide_core::compiler::PresentationHealthIssue {
                severity: HealthSeverity::Warning,
                message: format!(
                    "Presentation duration (~{:.1} min) exceeds target talk limit ({} min) by ~{:.1} min",
                    total_sec as f64 / 60.0,
                    target_mins,
                    over_mins
                ),
                slide_index: None,
                suggestion: Some(
                    "Trim content or reduce words in slides to fit the allotted time slot".to_string(),
                ),
            });
        }
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
    }

    if json {
        let report = serde_json::json!({
            "title": deck.title,
            "total_slides": total_slides,
            "total_speaking_seconds": total_sec,
            "estimated_minutes": total_sec as f64 / 60.0,
            "notes_words": total_notes_words,
            "target_minutes": target_minutes,
            "errors_count": errors_count,
            "warnings_count": warnings_count,
            "issues": issues.iter().map(|i| {
                serde_json::json!({
                    "severity": format!("{:?}", i.severity),
                    "message": i.message,
                    "slide_index": i.slide_index.map(|idx| idx + 1),
                    "suggestion": i.suggestion,
                })
            }).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&report)?);
        if errors_count > 0 {
            return Err(format!(
                "Health audit detected {errors_count} critical presentation issue(s)"
            )
            .into());
        }
        return Ok(());
    }

    println!();
    println!("  ┌────────────────────────────────────────────────────────┐");
    println!("  │  [AUDIT] Cargo Slide Presentation Health Audit         │");
    println!("  └────────────────────────────────────────────────────────┘");
    println!();

    println!("  Presentation: {}", deck.title);
    println!("  Slides:       {total_slides} slides");
    println!("  Est. Talk:    ~{:.1} min pacing", total_sec as f64 / 60.0);
    if let Some(target_mins) = target_minutes {
        println!("  Target Talk:  {target_mins} min allotted limit");
    }
    println!("  Notes Words:  {total_notes_words} words");
    println!();

    if issues.is_empty() {
        println!("  [OK] Excellent! No presentation health issues detected.");
        println!("  [OK] Media paths, slide titles, imports, and element density look clean.");
        println!();
        return Ok(());
    }

    for issue in &issues {
        let prefix = match issue.severity {
            | HealthSeverity::Error => "[ERROR]",
            | HealthSeverity::Warning => "[WARN]",
            | HealthSeverity::Info => "[INFO]",
        };

        let slide_label = issue.slide_index.map_or_else(String::new, |idx| {
            format!(" [Slide {}]", idx.saturating_add(1))
        });

        println!("  {} {}{}", prefix, issue.message, slide_label);
        if let Some(ref sug) = issue.suggestion {
            println!("     Suggestion: {sug}");
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
