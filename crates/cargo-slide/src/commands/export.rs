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
pub fn execute(
    file: &Path,
    format: &str,
    output: Option<PathBuf>,
    pages: Option<&str>,
    scale: f32,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    if !file.exists() {
        return Err(format!("File does not exist: {}", file.display()).into());
    }

    let stem = file
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
            compiler.compile_to_pdf(file, &out_pdf)?;
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
            let deck = compiler.compile_file(file)?;
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
            let deck = compiler.compile_file(file)?;
            let selected_pages = parse_page_selection(pages, deck.total_slides())?;
            let renderer = SvgRenderer::new();
            let effective_scale = if scale <= 0.0 { 1.0 } else { scale };
            let target_width = (1920.0 * effective_scale).round() as usize;
            let target_height = (1080.0 * effective_scale).round() as usize;

            if selected_pages.len() == 1
                && output.as_ref().map_or(false, |p| {
                    p.extension()
                        .map_or(false, |ext| ext.eq_ignore_ascii_case("png"))
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
            crate::commands::pack::execute(file, output, "fade", false)?;
        },
        | "wasm" | "web" | "csr" => {
            crate::commands::build::execute_wasm(file, output, "fade")?;
        },
        | other => {
            return Err(format!(
                "Unsupported export format: {other}. Supported formats: pdf, svg, png, slide, wasm"
            )
            .into());
        },
    }

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
}
