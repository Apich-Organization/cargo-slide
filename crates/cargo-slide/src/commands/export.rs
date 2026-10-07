use slide_core::compiler::SlideCompiler;
use slide_player::SvgRenderer;
use std::collections::BTreeSet;
use std::fs::create_dir_all;
use std::fs::write;
use std::path::Path;
use std::path::PathBuf;

/// Parse page selection expression like "1,3,5-8", returning sorted 1-indexed page numbers.
pub fn parse_page_selection(
    pages_str: Option<&str>,
    total_pages: usize,
) -> Result<Vec<usize>, String> {
    let Some(raw) = pages_str else {
        return Ok((1..=total_pages).collect());
    };

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok((1..=total_pages).collect());
    }

    let mut selected = BTreeSet::new();
    for part in trimmed.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((start_s, end_s)) = part.split_once('-') {
            let start = start_s
                .trim()
                .parse::<usize>()
                .map_err(|_| format!("Invalid start page number in range '{part}': '{start_s}'"))?;
            let end = end_s
                .trim()
                .parse::<usize>()
                .map_err(|_| format!("Invalid end page number in range '{part}': '{end_s}'"))?;
            if start == 0 || end == 0 {
                return Err("Page numbers must be 1-based (>= 1)".to_string());
            }
            if start > end {
                return Err(format!(
                    "Invalid range '{part}': start ({start}) cannot exceed end ({end})"
                ));
            }
            for p in start..=end {
                if p > total_pages {
                    return Err(format!(
                        "Requested page {p} exceeds total slides ({total_pages})"
                    ));
                }
                selected.insert(p);
            }
        } else {
            let p = part
                .parse::<usize>()
                .map_err(|_| format!("Invalid page number: '{part}'"))?;
            if p == 0 {
                return Err("Page numbers must be 1-based (>= 1)".to_string());
            }
            if p > total_pages {
                return Err(format!(
                    "Requested page {p} exceeds total slides ({total_pages})"
                ));
            }
            selected.insert(p);
        }
    }

    if selected.is_empty() {
        return Err("No valid pages selected".to_string());
    }

    Ok(selected.into_iter().collect())
}

/// Export presentation to PDF, SVG, PNG, .slide, or WASM CSR formats.
#[allow(clippy::too_many_lines)]
pub fn execute(
    file: &Path,
    format: &str,
    output: Option<PathBuf>,
    pages: Option<&str>,
    scale: f32,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let resolved_file = slide_core::compiler::resolve_presentation_target(file)?;
    let target_file = resolved_file.as_path();

    let stem = target_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("presentation");
    let compiler = SlideCompiler::new()?;

    match format.to_lowercase().as_str() {
        | "pdf" => {
            let out_pdf = output.unwrap_or_else(|| PathBuf::from(format!("{stem}.pdf")));
            slide_core::logger::log_event(
                "info",
                &format!("[EXPORT] Exporting to PDF: {}", out_pdf.display()),
                Some(serde_json::json!({
                    "stage": "export_pdf_start",
                    "output": out_pdf.display().to_string(),
                })),
            );
            compiler.compile_to_pdf(target_file, &out_pdf)?;
            slide_core::logger::log_event(
                "success",
                &format!("[OK] PDF exported successfully: {}", out_pdf.display()),
                Some(serde_json::json!({
                    "stage": "export_pdf_success",
                    "output": out_pdf.display().to_string(),
                })),
            );
        },
        | "svg" => {
            let out_dir = output.unwrap_or_else(|| PathBuf::from(format!("{stem}-svgs")));
            create_dir_all(&out_dir)?;
            let deck = compiler.compile_file(target_file)?;
            let selected_pages = parse_page_selection(pages, deck.total_slides())?;

            slide_core::logger::log_event(
                "info",
                &format!(
                    "[EXPORT] Exporting {} SVG slides to directory: {}",
                    selected_pages.len(),
                    out_dir.display()
                ),
                Some(serde_json::json!({
                    "stage": "export_svg_start",
                    "output_dir": out_dir.display().to_string(),
                    "selected_count": selected_pages.len(),
                })),
            );

            for slide in &deck.slides {
                if selected_pages.contains(&slide.page_number) {
                    let page_num = slide.page_number;
                    let svg_file = out_dir.join(format!("page-{page_num}.svg"));
                    write(&svg_file, &slide.svg_data)?;
                }
            }

            slide_core::logger::log_event(
                "success",
                &format!(
                    "[OK] {} SVG slides exported successfully to {}",
                    selected_pages.len(),
                    out_dir.display()
                ),
                Some(serde_json::json!({
                    "stage": "export_svg_success",
                    "exported_count": selected_pages.len(),
                    "output_dir": out_dir.display().to_string(),
                })),
            );
        },
        | "png" => {
            let deck = compiler.compile_file(target_file)?;
            let selected_pages = parse_page_selection(pages, deck.total_slides())?;
            let renderer = SvgRenderer::new();
            let effective_scale = if scale <= 0.0 { 1.0 } else { scale };
            let target_width = (1920.0 * effective_scale).round() as usize;
            let target_height = (1080.0 * effective_scale).round() as usize;

            if selected_pages.len() == 1
                && output.as_ref().is_some_and(|p| {
                    p.extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
                })
            {
                let out_file = output.unwrap();
                if let Some(parent) = out_file.parent() {
                    create_dir_all(parent)?;
                }
                let page_idx = selected_pages[0];
                let slide = deck
                    .slides
                    .iter()
                    .find(|s| s.page_number == page_idx)
                    .ok_or_else(|| format!("Slide page {page_idx} not found"))?;

                let (surface, _) =
                    renderer.render_svg(&slide.svg_data, target_width, target_height)?;
                let mut rgba = Vec::with_capacity(surface.width * surface.height * 4);
                for &px in &surface.pixels {
                    rgba.push(((px >> 16) & 0xFF) as u8);
                    rgba.push(((px >> 8) & 0xFF) as u8);
                    rgba.push((px & 0xFF) as u8);
                    rgba.push(((px >> 24) & 0xFF) as u8);
                }
                image::save_buffer(
                    &out_file,
                    &rgba,
                    surface.width as u32,
                    surface.height as u32,
                    image::ExtendedColorType::Rgba8,
                )?;

                slide_core::logger::log_event(
                    "success",
                    &format!(
                        "[OK] PNG slide exported successfully to {}",
                        out_file.display()
                    ),
                    Some(serde_json::json!({
                        "stage": "export_png_success",
                        "output": out_file.display().to_string(),
                        "scale": effective_scale,
                    })),
                );
            } else {
                let out_dir = output.unwrap_or_else(|| PathBuf::from(format!("{stem}-pngs")));
                create_dir_all(&out_dir)?;

                slide_core::logger::log_event(
                    "info",
                    &format!(
                        "[EXPORT] Exporting {} PNG slides (scale: {:.1}x, {}x{}) to directory: {}",
                        selected_pages.len(),
                        effective_scale,
                        target_width,
                        target_height,
                        out_dir.display()
                    ),
                    Some(serde_json::json!({
                        "stage": "export_png_start",
                        "output_dir": out_dir.display().to_string(),
                        "scale": effective_scale,
                        "selected_count": selected_pages.len(),
                    })),
                );

                for slide in &deck.slides {
                    if selected_pages.contains(&slide.page_number) {
                        let page_num = slide.page_number;
                        let png_file = out_dir.join(format!("page-{page_num}.png"));
                        let (surface, _) =
                            renderer.render_svg(&slide.svg_data, target_width, target_height)?;
                        let mut rgba = Vec::with_capacity(surface.width * surface.height * 4);
                        for &px in &surface.pixels {
                            rgba.push(((px >> 16) & 0xFF) as u8);
                            rgba.push(((px >> 8) & 0xFF) as u8);
                            rgba.push((px & 0xFF) as u8);
                            rgba.push(((px >> 24) & 0xFF) as u8);
                        }
                        image::save_buffer(
                            &png_file,
                            &rgba,
                            surface.width as u32,
                            surface.height as u32,
                            image::ExtendedColorType::Rgba8,
                        )?;
                    }
                }

                slide_core::logger::log_event(
                    "success",
                    &format!(
                        "[OK] {} PNG slides exported successfully to {}",
                        selected_pages.len(),
                        out_dir.display()
                    ),
                    Some(serde_json::json!({
                        "stage": "export_png_success",
                        "exported_count": selected_pages.len(),
                        "output_dir": out_dir.display().to_string(),
                    })),
                );
            }
        },
        | "slide" | "package" => {
            crate::commands::pack::execute(target_file, output, "fade", false)?;
        },
        | "wasm" | "web" | "csr" => {
            crate::commands::build::execute_wasm(target_file, output, "fade")?;
        },
        | "markdown" | "md" => {
            let out_file = output.unwrap_or_else(|| PathBuf::from(format!("{stem}-handout.md")));
            let deck = compiler.compile_file(target_file)?;
            let selected_pages = parse_page_selection(pages, deck.total_slides())?;
            export_markdown(&deck, &selected_pages, &out_file)?;
        },
        | "json" => {
            let out_file = output.unwrap_or_else(|| PathBuf::from(format!("{stem}.json")));
            let deck = compiler.compile_file(target_file)?;
            let selected_pages = parse_page_selection(pages, deck.total_slides())?;
            export_json(&deck, &selected_pages, &out_file)?;
        },
        | other => {
            return Err(format!(
                "Unsupported export format: {other}. Supported formats: pdf, svg, png, markdown (md), json, slide, wasm"
            )
            .into());
        },
    }

    Ok(())
}

fn export_markdown(
    deck: &slide_core::model::SlideDeck,
    selected_pages: &[usize],
    out_file: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fmt::Write as _;
    let mut md = String::new();
    let _ = writeln!(md, "# {}\n", deck.title);
    let _ = writeln!(
        md,
        "*Total Slides: {} | Exported: {}*\n\n---\n",
        deck.total_slides(),
        selected_pages.len()
    );

    for slide in &deck.slides {
        if !selected_pages.contains(&slide.page_number) {
            continue;
        }
        let title = slide.extract_title();
        let plain_text = slide.extract_plain_text();
        let _ = writeln!(md, "## Slide {}: {}\n", slide.page_number, title);
        if !plain_text.is_empty() {
            md.push_str(&plain_text);
            md.push_str("\n\n");
        }
        if let Some(notes) = &slide.notes
            && !notes.trim().is_empty()
        {
            md.push_str("> **Speaker Notes:**\n");
            for line in notes.lines() {
                let _ = writeln!(md, "> {line}");
            }
            md.push('\n');
        }
        md.push_str("---\n\n");
    }

    if let Some(parent) = out_file.parent() {
        create_dir_all(parent)?;
    }
    write(out_file, md)?;

    slide_core::logger::log_event(
        "success",
        &format!(
            "[OK] Markdown handout ({} slides) exported successfully to {}",
            selected_pages.len(),
            out_file.display()
        ),
        Some(serde_json::json!({
            "stage": "export_markdown_success",
            "output": out_file.display().to_string(),
            "count": selected_pages.len(),
        })),
    );
    Ok(())
}

#[derive(serde::Serialize)]
pub struct SlideExportItem<'a> {
    pub page_number: usize,
    pub title: String,
    pub step_count: usize,
    pub notes: Option<&'a str>,
    pub plain_text: String,
    pub estimated_seconds: u64,
}

#[derive(serde::Serialize)]
pub struct PresentationExportData<'a> {
    pub title: &'a str,
    pub total_slides: usize,
    pub exported_slides: usize,
    pub aspect_ratio: f32,
    pub slides: Vec<SlideExportItem<'a>>,
}

fn export_json(
    deck: &slide_core::model::SlideDeck,
    selected_pages: &[usize],
    out_file: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut slides_data = Vec::new();
    for slide in &deck.slides {
        if !selected_pages.contains(&slide.page_number) {
            continue;
        }
        let title = slide.extract_title();
        let plain_text = slide.extract_plain_text();
        let words = plain_text.split_whitespace().count();
        let notes_wc = slide.notes_word_count();
        let pace_words = if notes_wc > 0 {
            notes_wc
        } else {
            words.max(25)
        };
        let est_sec = ((pace_words.saturating_mul(60)) / 130) as u64;

        slides_data.push(SlideExportItem {
            page_number: slide.page_number,
            title,
            step_count: slide.max_step(),
            notes: slide.notes.as_deref(),
            plain_text,
            estimated_seconds: est_sec.max(10),
        });
    }

    let aspect_ratio = deck.slides.first().map_or(16.0 / 9.0, |s| {
        if s.view_box.height > 0.0 {
            s.view_box.width / s.view_box.height
        } else {
            16.0 / 9.0
        }
    });

    let export_obj = PresentationExportData {
        title: &deck.title,
        total_slides: deck.total_slides(),
        exported_slides: slides_data.len(),
        aspect_ratio,
        slides: slides_data,
    };

    if let Some(parent) = out_file.parent() {
        create_dir_all(parent)?;
    }
    let json_str = serde_json::to_string_pretty(&export_obj)?;
    write(out_file, json_str)?;

    let count = export_obj.slides.len();
    slide_core::logger::log_event(
        "success",
        &format!(
            "[OK] Presentation JSON metadata ({count} slides) exported successfully to {}",
            out_file.display()
        ),
        Some(serde_json::json!({
            "stage": "export_json_success",
            "output": out_file.display().to_string(),
            "count": count,
        })),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_page_selection_all() {
        let pages = parse_page_selection(None, 5).unwrap();
        assert_eq!(pages, vec![1, 2, 3, 4, 5]);

        let pages_empty = parse_page_selection(Some(""), 5).unwrap();
        assert_eq!(pages_empty, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_parse_page_selection_ranges_and_singles() {
        let pages = parse_page_selection(Some("1, 3, 5-7"), 10).unwrap();
        assert_eq!(pages, vec![1, 3, 5, 6, 7]);

        let pages_dup = parse_page_selection(Some("2, 2, 3-4, 4"), 5).unwrap();
        assert_eq!(pages_dup, vec![2, 3, 4]);
    }

    #[test]
    fn test_parse_page_selection_out_of_bounds() {
        assert!(parse_page_selection(Some("6"), 5).is_err());
        assert!(parse_page_selection(Some("0"), 5).is_err());
        assert!(parse_page_selection(Some("3-1"), 5).is_err());
        assert!(parse_page_selection(Some("4-8"), 5).is_err());
    }

    #[test]
    fn test_export_markdown_and_json() {
        let deck = slide_core::model::SlideDeck {
            title: "Test Deck".to_string(),
            default_animation: "fade".to_string(),
            slides: vec![
                slide_core::model::Slide {
                    page_number: 1,
                    svg_data: "<svg><text>Slide 1 Intro</text></svg>".to_string(),
                    view_box: slide_core::model::Rect::new(0.0, 0.0, 1920.0, 1080.0),
                    notes: Some("Welcome everyone".to_string()),
                    hotspots: vec![],
                    animation: None,
                    steps: vec![],
                },
                slide_core::model::Slide {
                    page_number: 2,
                    svg_data: "<svg><text>Slide 2 Deep Dive</text></svg>".to_string(),
                    view_box: slide_core::model::Rect::new(0.0, 0.0, 1920.0, 1080.0),
                    notes: None,
                    hotspots: vec![],
                    animation: None,
                    steps: vec![],
                },
            ],
        };

        let temp_dir =
            std::env::temp_dir().join(format!("cargo_slide_export_test_{}", std::process::id()));
        let md_file = temp_dir.join("handout.md");
        let json_file = temp_dir.join("deck.json");

        export_markdown(&deck, &[1, 2], &md_file).unwrap();
        assert!(md_file.exists());
        let md_content = std::fs::read_to_string(&md_file).unwrap();
        assert!(md_content.contains("# Test Deck"));
        assert!(md_content.contains("Welcome everyone"));

        export_json(&deck, &[1], &json_file).unwrap();
        assert!(json_file.exists());
        let json_content = std::fs::read_to_string(&json_file).unwrap();
        assert!(json_content.contains("Test Deck"));
        assert!(json_content.contains("Welcome everyone"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
