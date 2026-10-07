//! Command to calculate comprehensive slide deck statistics, speaking pace, and notes coverage.

use serde::Serialize;
use slide_core::compiler::SlideCompiler;
use slide_core::model::SlideDeck;
use slide_core::package::read_package_metadata;
use slide_core::package::unpack_deck_from_file;
use std::path::Path;

/// Pacing breakdown for an individual slide
#[derive(Debug, Clone, Serialize)]
pub struct SlideStatEntry {
    pub slide_index: usize,
    pub title: String,
    pub word_count: usize,
    pub char_count: usize,
    pub estimated_seconds: u64,
    pub has_notes: bool,
    pub notes_word_count: usize,
}

/// Aggregated presentation deck statistics
#[derive(Debug, Clone, Serialize)]
pub struct DeckStatsReport {
    pub title: String,
    pub total_slides: usize,
    pub total_words: usize,
    pub total_chars: usize,
    pub notes_coverage_percent: f32,
    pub slides_with_notes: usize,
    pub slides_without_notes: usize,
    pub total_notes_words: usize,
    pub calibrated_wpm: u32,
    pub est_duration_slow_mins: f32,
    pub est_duration_target_mins: f32,
    pub est_duration_brisk_mins: f32,
    pub rhythm_score: u32,
    pub rhythm_description: String,
    pub pace_variance_seconds: f32,
    pub checkpoints: Vec<String>,
    pub rebalance_report: Option<slide_core::pacing::DeckRebalanceReport>,
    pub slides: Vec<SlideStatEntry>,
}

/// Analyze a `SlideDeck` and generate a `DeckStatsReport`
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
pub fn analyze_deck_stats(
    deck: &SlideDeck,
    wpm: u32,
    rebalance_mins: Option<u64>,
) -> DeckStatsReport {
    let calibrated_wpm = if wpm == 0 { 130 } else { wpm };
    let mut slides_stats = Vec::with_capacity(deck.slides.len());
    let mut total_words = 0usize;
    let mut total_chars = 0usize;
    let mut total_notes_words = 0usize;
    let mut slides_with_notes = 0usize;

    for (idx, slide) in deck.slides.iter().enumerate() {
        let clean_text = slide.extract_plain_text();
        let words = clean_text.split_whitespace().count();
        let chars = clean_text.chars().filter(|c| !c.is_whitespace()).count();
        total_words = total_words.saturating_add(words);
        total_chars = total_chars.saturating_add(chars);

        let notes_wc = slide.notes_word_count();
        let has_notes = slide.notes.is_some() && notes_wc > 0;
        if has_notes {
            slides_with_notes = slides_with_notes.saturating_add(1);
            total_notes_words = total_notes_words.saturating_add(notes_wc);
        }

        let words_for_pace = if has_notes {
            notes_wc
        } else {
            words.max(25)
        };
        let est_sec = ((words_for_pace as f64 / f64::from(calibrated_wpm)) * 60.0).round() as u64;

        slides_stats.push(SlideStatEntry {
            slide_index: idx + 1,
            title: slide.extract_title().chars().take(40).collect(),
            word_count: words,
            char_count: chars,
            estimated_seconds: est_sec.max(10),
            has_notes,
            notes_word_count: notes_wc,
        });
    }

    let total_slides = deck.slides.len();
    let slides_without_notes = total_slides.saturating_sub(slides_with_notes);
    let notes_coverage_percent = if total_slides > 0 {
        (slides_with_notes as f32 / total_slides as f32) * 100.0
    } else {
        0.0
    };

    let total_pace_words = if total_notes_words > 0 {
        total_notes_words
    } else {
        total_words.max(total_slides * 30)
    };

    let est_duration_slow_mins = (total_pace_words as f32 / 100.0).max(1.0);
    let est_duration_target_mins = (total_pace_words as f32 / calibrated_wpm as f32).max(1.0);
    let est_duration_brisk_mins = (total_pace_words as f32 / 160.0).max(1.0);

    let pacing_report = slide_core::pacing::calculate_deck_pacing(deck, &[]);
    let rebalance_report =
        rebalance_mins.map(|mins| slide_core::pacing::rebalance_deck_pacing(deck, mins as usize));

    DeckStatsReport {
        title: deck.title.clone(),
        total_slides,
        total_words,
        total_chars,
        notes_coverage_percent,
        slides_with_notes,
        slides_without_notes,
        total_notes_words,
        calibrated_wpm,
        est_duration_slow_mins,
        est_duration_target_mins,
        est_duration_brisk_mins,
        rhythm_score: pacing_report.rhythm_score,
        rhythm_description: pacing_report.rhythm_description,
        pace_variance_seconds: pacing_report.pace_variance_seconds as f32,
        checkpoints: pacing_report
            .checkpoint_recommendations
            .iter()
            .map(|idx| format!("Slide #{idx}"))
            .collect(),
        rebalance_report,
        slides: slides_stats,
    }
}

#[allow(dead_code)]
fn strip_svg_markup(svg: &str) -> String {
    slide_core::svg::extract_text_from_svg(svg)
}

/// Execute the stats calculation command
pub fn execute(
    file: &Path,
    json: bool,
    wpm: u32,
    rebalance: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let resolved_file = slide_core::compiler::resolve_presentation_target(file)?;
    let target = resolved_file.as_path();

    if json {
        slide_core::logger::set_silent(true);
    }

    let deck = if target.extension().and_then(|e| e.to_str()) == Some("slide") {
        let _ = read_package_metadata(target)?;
        unpack_deck_from_file(target)?
    } else {
        let compiler = SlideCompiler::new()?;
        compiler.compile_file(target)?
    };

    let report = analyze_deck_stats(&deck, wpm, rebalance);

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    print_report_console(&report, wpm);
    Ok(())
}

#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
fn print_report_console(
    report: &DeckStatsReport,
    wpm: u32,
) {
    println!();
    println!("  ┌────────────────────────────────────────────────────────┐");
    println!("  │  [STATS] Cargo Slide Deck Analytics & Pacing Report    │");
    println!("  └────────────────────────────────────────────────────────┘");
    println!();
    println!("  Presentation:          {}", report.title);
    println!("  Total Slides:          {} slides", report.total_slides);
    println!(
        "  Total Slide Words:     {} words ({} characters)",
        report.total_words, report.total_chars
    );
    println!(
        "  Speaker Notes Words:   {} words",
        report.total_notes_words
    );
    println!(
        "  Notes Coverage:        {:.1}% ({} of {} slides have notes)",
        report.notes_coverage_percent, report.slides_with_notes, report.total_slides
    );
    println!();
    println!("  ── Estimated Presentation Pacing ({wpm} WPM baseline) ──");
    println!(
        "  • Deliberate / Technical (100 WPM)   : ~{:.1} minutes",
        report.est_duration_slow_mins
    );
    println!(
        "  • Target Speaking Pace   ({wpm} WPM)   : ~{:.1} minutes",
        report.est_duration_target_mins
    );
    println!(
        "  • Brisk / Keynote Pace   (160 WPM)   : ~{:.1} minutes",
        report.est_duration_brisk_mins
    );
    println!();
    println!("  ── Delivery Dynamics & Rhythm ────────────────────────────");
    println!(
        "  • Rhythm Score:        {}/100 ({})",
        report.rhythm_score, report.rhythm_description
    );
    println!(
        "  • Pace Variance:       ±{:.1}s per slide",
        report.pace_variance_seconds
    );
    if !report.checkpoints.is_empty() {
        println!(
            "  • Suggested Checkpoints: {}",
            report.checkpoints.join(", ")
        );
    }
    println!();

    if !report.slides.is_empty() {
        println!("  ── Per-Slide Pacing Breakdown ────────────────────────────");
        println!("  Slide  Words  Duration Timeline       Notes  Status");
        println!("  ─────  ─────  ─────────────────────   ─────  ──────");
        let max_sec = report
            .slides
            .iter()
            .map(|s| s.estimated_seconds)
            .max()
            .unwrap_or(60)
            .max(30);
        for s in &report.slides {
            let notes_badge = if s.has_notes { "YES" } else { "---" };
            let status = if s.estimated_seconds > 180 {
                "Pacing bottleneck (>3 min)"
            } else if !s.has_notes && s.word_count < 5 {
                "Minimal content"
            } else {
                "Healthy"
            };
            let filled = ((s.estimated_seconds.saturating_mul(10)) / max_sec).clamp(1, 10) as usize;
            let bar = format!(
                "[{}{}] ~{}s",
                "█".repeat(filled),
                "░".repeat(10 - filled),
                s.estimated_seconds
            );
            println!(
                "   #{:<3}  {:<5}  {:<22}  {:<5}  {}",
                s.slide_index, s.word_count, bar, notes_badge, status
            );
        }
        println!();
    }

    if let Some(ref reb) = report.rebalance_report {
        println!("  ── Intelligent Pacing Rebalance Timetable ────────────────");
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
}
