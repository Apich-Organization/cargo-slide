//! Command to audit presentation health, check formatting, density, and missing assets.

use slide_core::compiler::HealthSeverity;
use slide_core::compiler::SlideCompiler;
use slide_core::compiler::auto_fix_source;
use slide_core::compiler::check_presentation_health;
use slide_core::pacing::rebalance_deck_pacing;
use std::path::Path;

/// Run presentation health check and report issues
#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
pub fn execute(
    file: &Path,
    target_minutes: Option<u64>,
    json: bool,
    fix: bool,
    rebalance: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let resolved_path = slide_core::compiler::resolve_presentation_target(file)?;
    let file = resolved_path.as_path();

    if json {
        slide_core::logger::set_silent(true);
    }

    let mut source_text = std::fs::read_to_string(file)?;
    let mut fixes_applied = Vec::new();

    if fix {
        let fix_res = auto_fix_source(&source_text);
        if !fix_res.repairs.is_empty() {
            std::fs::write(file, &fix_res.fixed_source)?;
            source_text = fix_res.fixed_source;
            fixes_applied = fix_res.repairs;
            if !json {
                println!(
                    "  [AUTO-FIX] Applied {} automated fix(es):",
                    fixes_applied.len()
                );
                for f in &fixes_applied {
                    println!("    • {f}");
                }
                println!();
            }
        } else if !json {
            println!("  [AUTO-FIX] Source syntax is already compliant. No changes needed.");
            println!();
        }
    }

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
                        "fixes_applied": fixes_applied,
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
    let target_mins = target_minutes.or(rebalance);
    if let Some(target) = target_mins {
        let target_sec = target.saturating_mul(60) as usize;
        if total_sec > target_sec {
            let over_mins = (total_sec.saturating_sub(target_sec)) as f64 / 60.0;
            issues.push(slide_core::compiler::PresentationHealthIssue {
                severity: HealthSeverity::Warning,
                message: format!(
                    "Presentation duration (~{:.1} min) exceeds target talk limit ({} min) by ~{:.1} min",
                    total_sec as f64 / 60.0,
                    target,
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

    let rebalance_report = rebalance.map(|mins| rebalance_deck_pacing(&deck, mins as usize));

    if json {
        let report = serde_json::json!({
            "title": deck.title,
            "total_slides": total_slides,
            "total_speaking_seconds": total_sec,
            "estimated_minutes": total_sec as f64 / 60.0,
            "notes_words": total_notes_words,
            "target_minutes": target_mins,
            "errors_count": errors_count,
            "warnings_count": warnings_count,
            "fixes_applied": fixes_applied,
            "rebalance_report": rebalance_report,
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
    if let Some(target) = target_mins {
        println!("  Target Talk:  {target} min allotted limit");
    }
    println!("  Notes Words:  {total_notes_words} words");
    println!();

    if let Some(ref reb) = rebalance_report {
        println!(
            "  ── Intelligent Pacing Rebalance (Target: {} min) ──────",
            reb.target_minutes
        );
        println!("  Slide  Target Sec  Target Min  Cognitive Weight  Status");
        println!("  ─────  ──────────  ──────────  ────────────────  ──────");
        for alloc in &reb.slide_allocations {
            let status = if alloc.is_bottleneck {
                "⚠️ Bottleneck slide"
            } else {
                "OK"
            };
            println!(
                "   #{:<3}  {:<10}  ~{:<8.1}  {:<16.2}  {}",
                alloc.slide_index,
                format!("{}s", alloc.budgeted_seconds),
                alloc.budgeted_seconds as f64 / 60.0,
                alloc.cognitive_weight,
                status
            );
        }
        println!();
        if !reb.checkpoints.is_empty() {
            println!("  Presentation Checkpoints:");
            for cp in &reb.checkpoints {
                println!(
                    "    • {:>3}% Checkpoint at Slide #{} (~{:.1} min mark)",
                    cp.milestone_percent, cp.slide_index, cp.target_minute_mark
                );
            }
            println!();
        }
    }

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
