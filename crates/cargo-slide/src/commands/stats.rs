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
    pub slides: Vec<SlideStatEntry>,
}

/// Analyze a `SlideDeck` and generate a `DeckStatsReport`
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
pub fn analyze_deck_stats(
    deck: &SlideDeck,
    wpm: u32,
) -> DeckStatsReport {
    let calibrated_wpm = if wpm == 0 { 130 } else { wpm };
    let mut slides_stats = Vec::with_capacity(deck.slides.len());
    let mut total_words = 0usize;
    let mut total_chars = 0usize;
    let mut total_notes_words = 0usize;
    let mut slides_with_notes = 0usize;

    for (idx, slide) in deck.slides.iter().enumerate() {
        // Strip SVG tags roughly to count plain words in slide
        let clean_text = strip_svg_markup(&slide.svg_data);
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
            title: slide
                .notes
                .as_deref()
                .and_then(|n| n.lines().next())
                .unwrap_or("Slide")
                .chars()
                .take(30)
                .collect(),
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
        slides: slides_stats,
    }
}

fn strip_svg_markup(svg: &str) -> String {
    let mut inside_tag = false;
    let mut result = String::with_capacity(svg.len() / 3);
    for ch in svg.chars() {
        if ch == '<' {
            inside_tag = true;
            result.push(' ');
        } else if ch == '>' {
            inside_tag = false;
        } else if !inside_tag {
            result.push(ch);
        }
    }
    result
}

/// Execute the stats calculation command
pub fn execute(
    file: &Path,
    json: bool,
    wpm: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    if !file.exists() {
        return Err(format!("File does not exist: {}", file.display()).into());
    }

    if json {
        slide_core::logger::set_silent(true);
    }

    let deck = if file.extension().and_then(|e| e.to_str()) == Some("slide") {
        let _ = read_package_metadata(file)?;
        unpack_deck_from_file(file)?
    } else {
        let compiler = SlideCompiler::new()?;
        compiler.compile_file(file)?
    };

    let report = analyze_deck_stats(&deck, wpm);

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

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

    if !report.slides.is_empty() {
        println!("  ── Per-Slide Pacing Breakdown ────────────────────────────");
        println!("  Slide  Words  Est. Sec  Notes  Status");
        println!("  ─────  ─────  ────────  ─────  ──────");
        for s in &report.slides {
            let notes_badge = if s.has_notes { "YES" } else { "---" };
            let status = if s.estimated_seconds > 180 {
                "Pacing bottleneck (>3 min)"
            } else if !s.has_notes && s.word_count < 5 {
                "Minimal content"
            } else {
                "Healthy"
            };
            println!(
                "   #{:<3}  {:<5}  {:<8}  {:<5}  {}",
                s.slide_index,
                s.word_count,
                format!("~{}s", s.estimated_seconds),
                notes_badge,
                status
            );
        }
        println!();
    }

    Ok(())
}
