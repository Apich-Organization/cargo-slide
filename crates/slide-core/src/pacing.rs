//! Advanced Multi-Modal Presentation Pacing & Speech Timing Model.
//!
//! Provides high-precision speaking duration estimation for presentations, distinguishing:
//! - Bilingual / Multi-lingual speech dynamics: English/Latin WPM (~135 WPM) vs CJK CPM (~230 CPM)
//! - Speaker notes vs visual slide complexity (bullet points, code listings, math equations, charts)
//! - Step reveals (#step) audience cognitive pauses
//! - Slide-by-slide health categorization (Brisk, Optimal, Dense, Overloaded)
//! - Target speech timing comparison with buffer analysis

use crate::model::Slide;
use crate::model::SlideDeck;
use serde::Deserialize;
use serde::Serialize;

/// Average speech rate constants
pub const WPM_ENGLISH_NORMAL: f64 = 135.0; // Words Per Minute
pub const CPM_CJK_NORMAL: f64 = 230.0; // Characters Per Minute (Chinese/Japanese/Korean)
pub const PAUSE_PER_LINE_SECS: f64 = 1.2; // Natural speech pause between thoughts
pub const PAUSE_PER_STEP_SECS: f64 = 6.0; // Pause for audience orientation on #step reveal
pub const CODE_EXPLANATION_SEC_PER_LINE: f64 = 2.5; // Average time to explain a code line
pub const CHART_EXPLANATION_SECS: f64 = 28.0; // Time to explain axes, data trends, insights
pub const MATH_EXPLANATION_SECS: f64 = 14.0; // Time to parse and explain a formula

/// Counts English/Latin words and CJK characters separately with high precision
#[must_use]
pub fn count_words_and_cjk(text: &str) -> (usize, usize) {
    let mut cjk_count: usize = 0;
    let mut in_ascii_word = false;
    let mut word_count: usize = 0;

    for ch in text.chars() {
        if is_cjk(ch) {
            cjk_count = cjk_count.saturating_add(1);
            if in_ascii_word {
                in_ascii_word = false;
            }
        } else if ch.is_alphanumeric() {
            if !in_ascii_word {
                in_ascii_word = true;
                word_count = word_count.saturating_add(1);
            }
        } else {
            in_ascii_word = false;
        }
    }

    (word_count, cjk_count)
}

/// Check if a character belongs to CJK Unicode blocks
#[must_use]
pub fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}'   // CJK Unified Ideographs
        | '\u{3400}'..='\u{4DBF}' // CJK Extension A
        | '\u{20000}'..='\u{2A6DF}' // CJK Extension B
        | '\u{F900}'..='\u{FAFF}' // CJK Compatibility Ideographs
        | '\u{3040}'..='\u{309F}' // Hiragana
        | '\u{30A0}'..='\u{30FF}' // Katakana
        | '\u{AC00}'..='\u{D7AF}' // Hangul Syllables
    )
}

/// Slide pacing health categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlidePacingStatus {
    /// Under 20s - very brief, check if content is sufficient
    Brisk,
    /// 20s - 90s - ideal, well-balanced pacing window
    Optimal,
    /// 90s - 150s - rich content, ensure key points are highlighted
    Dense,
    /// Over 150s (2.5 min) - overtime risk, consider splitting slide
    Overloaded,
}

impl SlidePacingStatus {
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            | Self::Brisk => "Brisk",
            | Self::Optimal => "Optimal",
            | Self::Dense => "Dense",
            | Self::Overloaded => "Overloaded",
        }
    }
}

/// Detailed pacing metrics for an individual presentation slide
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlidePacingInfo {
    pub slide_index: usize,
    pub title: String,
    pub has_speaker_notes: bool,
    pub notes_words: usize,
    pub notes_cjk_chars: usize,
    pub visual_words: usize,
    pub visual_cjk_chars: usize,
    pub code_lines: usize,
    pub math_formulas: usize,
    pub charts_and_tables: usize,
    pub step_count: usize,
    pub estimated_seconds: usize,
    pub status: SlidePacingStatus,
    pub recommendation: String,
}

/// Comparison delta against a target talk duration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PacingTargetDelta {
    pub target_minutes: usize,
    pub delta_seconds: i64,
    pub formatted_delta: String,
    pub on_track: bool,
    pub message: String,
}

/// Presentation-wide pacing and delivery report
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeckPacingReport {
    pub total_slides: usize,
    pub total_estimated_seconds: usize,
    pub formatted_duration: String,
    pub duration_range: String,
    pub avg_seconds_per_slide: usize,
    pub total_notes_words: usize,
    pub total_notes_cjk: usize,
    pub total_visual_words: usize,
    pub total_visual_cjk: usize,
    pub notes_coverage_percent: usize,
    pub optimal_count: usize,
    pub dense_count: usize,
    pub overloaded_count: usize,
    pub brisk_count: usize,
    pub slides: Vec<SlidePacingInfo>,
}

impl DeckPacingReport {
    /// Compare estimated talk time against a target time in minutes
    #[must_use]
    pub fn compare_target(
        &self,
        target_minutes: usize,
    ) -> PacingTargetDelta {
        let target_secs = (target_minutes as i64).saturating_mul(60);
        let actual_secs = self.total_estimated_seconds as i64;
        let delta = actual_secs.saturating_sub(target_secs);

        let formatted_delta = if delta == 0 {
            "Exact match".to_string()
        } else if delta > 0 {
            let m = delta / 60;
            let s = delta % 60;
            format!("+{m}m {s}s over target")
        } else {
            let abs_d = delta.abs();
            let m = abs_d / 60;
            let s = abs_d % 60;
            format!("-{m}m {s}s buffer available")
        };

        let (on_track, message) = if delta.abs() <= 120 {
            (
                true,
                "Pacing is well aligned with the presentation schedule.".to_string(),
            )
        } else if delta > 120 {
            (
                false,
                format!(
                    "Over time by {}m. Consider condensing overloaded slides or speeding up speaking rate.",
                    delta / 60
                ),
            )
        } else {
            (
                true,
                format!(
                    "Under target with {}m reserve buffer. Allows ample time for audience Q&A.",
                    delta.abs() / 60
                ),
            )
        };

        PacingTargetDelta {
            target_minutes,
            delta_seconds: delta,
            formatted_delta,
            on_track,
            message,
        }
    }
}

/// Calculate detailed pacing metrics for an individual slide
#[must_use]
pub fn calculate_slide_pacing(
    slide: &Slide,
    slide_index: usize,
    source_chunk: Option<&str>,
) -> SlidePacingInfo {
    let title =
        slide
            .hotspots
            .iter()
            .find_map(|h| {
                match h {
                    | crate::model::Hotspot::Link { target, .. }
                        if target.starts_with("slide-meta:") =>
                    {
                        Some(target.strip_prefix("slide-meta:").unwrap_or("").to_string())
                    },
                    | _ => None,
                }
            })
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| format!("Slide {}", slide_index.saturating_add(1)));

    let has_speaker_notes = slide.has_notes();
    let (notes_words, notes_cjk_chars) = count_words_and_cjk(slide.notes_text());

    // Extract visual elements count from chunk or SVG
    let (visual_words, visual_cjk_chars, code_lines, math_formulas, charts_and_tables) =
        if let Some(src) = source_chunk {
            analyze_source_chunk_complexity(src)
        } else {
            analyze_svg_complexity(&slide.svg_data)
        };

    let step_count = slide.max_step();

    // 1. Calculate base speech duration
    let est_secs_f64 = if has_speaker_notes {
        let note_lines = slide.notes_text().lines().count().max(1);
        let note_speech = (notes_words as f64 * 60.0 / WPM_ENGLISH_NORMAL)
            + (notes_cjk_chars as f64 * 60.0 / CPM_CJK_NORMAL)
            + (note_lines as f64 * PAUSE_PER_LINE_SECS);

        // Visual orientation & pauses
        let visual_overhead = 4.0
            + (step_count as f64 * PAUSE_PER_STEP_SECS)
            + (charts_and_tables as f64 * 12.0)
            + (math_formulas as f64 * 6.0)
            + (code_lines as f64 * 1.0);

        note_speech + visual_overhead
    } else {
        // Visual content reading & conversational delivery
        let base_slide = if slide_index == 0 {
            20.0
        } else {
            28.0
        };
        let visual_speech = (visual_words as f64 * 60.0 / WPM_ENGLISH_NORMAL * 0.7)
            + (visual_cjk_chars as f64 * 60.0 / CPM_CJK_NORMAL * 0.7);

        let code_time = code_lines as f64 * CODE_EXPLANATION_SEC_PER_LINE;
        let chart_time = charts_and_tables as f64 * CHART_EXPLANATION_SECS;
        let math_time = math_formulas as f64 * MATH_EXPLANATION_SECS;
        let step_time = step_count as f64 * PAUSE_PER_STEP_SECS;

        base_slide + visual_speech + code_time + chart_time + math_time + step_time
    };

    let estimated_seconds = (est_secs_f64.round() as usize).clamp(12, 280);

    let (status, recommendation) = if estimated_seconds < 25 {
        (
            SlidePacingStatus::Brisk,
            "Quick slide. Good for transitions or brief statements.".to_string(),
        )
    } else if estimated_seconds <= 90 {
        (
            SlidePacingStatus::Optimal,
            "Optimal pacing. Content and speaking duration are well-balanced.".to_string(),
        )
    } else if estimated_seconds <= 150 {
        (
            SlidePacingStatus::Dense,
            "Dense slide. Ensure key points are clearly emphasized.".to_string(),
        )
    } else {
        (
            SlidePacingStatus::Overloaded,
            "High talk time (>2.5 min). Consider splitting into multiple slides for audience clarity.".to_string(),
        )
    };

    SlidePacingInfo {
        slide_index,
        title,
        has_speaker_notes,
        notes_words,
        notes_cjk_chars,
        visual_words,
        visual_cjk_chars,
        code_lines,
        math_formulas,
        charts_and_tables,
        step_count,
        estimated_seconds,
        status,
        recommendation,
    }
}

/// Calculate comprehensive pacing report for an entire presentation deck
#[must_use]
pub fn calculate_deck_pacing(
    deck: &SlideDeck,
    slide_chunks: &[String],
) -> DeckPacingReport {
    let mut slides_info = Vec::new();
    let mut total_secs = 0usize;
    let mut total_notes_words = 0usize;
    let mut total_notes_cjk = 0usize;
    let mut total_visual_words = 0usize;
    let mut total_visual_cjk = 0usize;
    let mut notes_count = 0usize;

    let mut optimal_count = 0usize;
    let mut dense_count = 0usize;
    let mut overloaded_count = 0usize;
    let mut brisk_count = 0usize;

    for (idx, slide) in deck.slides.iter().enumerate() {
        let chunk = slide_chunks.get(idx).map(|s| s.as_str());
        let info = calculate_slide_pacing(slide, idx, chunk);

        total_secs = total_secs.saturating_add(info.estimated_seconds);
        total_notes_words = total_notes_words.saturating_add(info.notes_words);
        total_notes_cjk = total_notes_cjk.saturating_add(info.notes_cjk_chars);
        total_visual_words = total_visual_words.saturating_add(info.visual_words);
        total_visual_cjk = total_visual_cjk.saturating_add(info.visual_cjk_chars);

        if info.has_speaker_notes {
            notes_count = notes_count.saturating_add(1);
        }

        match info.status {
            | SlidePacingStatus::Brisk => brisk_count = brisk_count.saturating_add(1),
            | SlidePacingStatus::Optimal => optimal_count = optimal_count.saturating_add(1),
            | SlidePacingStatus::Dense => dense_count = dense_count.saturating_add(1),
            | SlidePacingStatus::Overloaded => {
                overloaded_count = overloaded_count.saturating_add(1)
            },
        }

        slides_info.push(info);
    }

    let total_slides = deck.total_slides().max(1);
    let avg_seconds = total_secs.checked_div(total_slides).unwrap_or(0);
    let notes_coverage_percent = (notes_count.saturating_mul(100))
        .checked_div(total_slides)
        .unwrap_or(0);

    let total_mins = total_secs / 60;
    let rem_secs = total_secs % 60;
    let formatted_duration = if total_mins == 0 {
        format!("{rem_secs}s")
    } else {
        format!("{total_mins} min {rem_secs} s")
    };

    let min_range = (total_secs as f64 * 0.88 / 60.0).floor() as usize;
    let max_range = (total_secs as f64 * 1.15 / 60.0).ceil() as usize;
    let duration_range = format!("{min_range} – {max_range} min");

    DeckPacingReport {
        total_slides: deck.total_slides(),
        total_estimated_seconds: total_secs,
        formatted_duration,
        duration_range,
        avg_seconds_per_slide: avg_seconds,
        total_notes_words,
        total_notes_cjk,
        total_visual_words,
        total_visual_cjk,
        notes_coverage_percent,
        optimal_count,
        dense_count,
        overloaded_count,
        brisk_count,
        slides: slides_info,
    }
}

fn analyze_source_chunk_complexity(src: &str) -> (usize, usize, usize, usize, usize) {
    let mut code_lines = 0usize;
    let mut math_formulas = 0usize;
    let mut charts_and_tables = 0usize;
    let mut in_code_fence = false;
    let mut clean_visual_text = String::new();

    for line in src.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            continue;
        }

        if in_code_fence {
            if !trimmed.is_empty() {
                code_lines = code_lines.saturating_add(1);
            }
            continue;
        }

        if trimmed.starts_with("//") {
            continue; // Skip comments and notes
        }

        if trimmed.starts_with('$') && trimmed.ends_with('$') {
            math_formulas = math_formulas.saturating_add(1);
            continue;
        }

        if trimmed.contains("#chart(")
            || trimmed.contains("#table(")
            || trimmed.contains("chart.bar")
            || trimmed.contains("chart.line")
        {
            charts_and_tables = charts_and_tables.saturating_add(1);
        }

        // Strip known layout macros
        let mut clean = trimmed;
        if clean.starts_with('#') {
            if clean.starts_with("#slide") || clean.starts_with("#pagebreak") {
                continue;
            }
            if let Some(pos) = clean.find('[') {
                clean = &clean[pos..];
            }
        }
        clean_visual_text.push_str(clean);
        clean_visual_text.push(' ');
    }

    let (v_words, v_cjk) = count_words_and_cjk(&clean_visual_text);
    (v_words, v_cjk, code_lines, math_formulas, charts_and_tables)
}

fn analyze_svg_complexity(svg: &str) -> (usize, usize, usize, usize, usize) {
    let mut visual_text = String::new();
    let mut text_elements = 0usize;

    for line in svg.lines() {
        if line.contains("<text") {
            text_elements = text_elements.saturating_add(1);
            // Extract inner text
            if let Some(start) = line.find('>')
                && let Some(end) = line.rfind("</text>")
                && start < end
            {
                visual_text.push_str(&line[start.saturating_add(1)..end]);
                visual_text.push(' ');
            }
        }
    }

    let (v_words, v_cjk) = count_words_and_cjk(&visual_text);
    let code_lines = if text_elements > 40 { 8 } else { 0 };
    let math_formulas = if svg.contains("<path") && text_elements > 30 {
        1
    } else {
        0
    };
    let charts_and_tables = if svg.contains("rect") && text_elements > 25 {
        1
    } else {
        0
    };

    (v_words, v_cjk, code_lines, math_formulas, charts_and_tables)
}
