use crate::error::Result;
use crate::error::SlideError;
use crate::model::Slide;
use crate::model::SlideDeck;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

/// Compute a 64-bit deterministic hash of a byte slice
#[must_use]
pub fn compute_source_hash(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// Metadata stored in the persistent rendering cache for a compiled deck
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RenderCacheManifest {
    pub source_hash: u64,
    pub source_path: String,
    pub deck: SlideDeck,
    pub slide_hashes: Vec<u64>,
}

/// Determine deterministic rendering cache directory for a given typst file
#[must_use]
pub fn get_render_cache_dir(typ_file: &Path) -> PathBuf {
    let root = typ_file.parent().unwrap_or_else(|| Path::new("."));
    let mut cur = root;
    while let Some(parent) = cur.parent() {
        if cur.join("target").is_dir() || cur.join("Cargo.toml").is_file() {
            let p = cur.join("target/.slide_cache");
            let _ = std::fs::create_dir_all(&p);
            return p;
        }
        cur = parent;
    }
    let local = root.join(".slide_cache");
    if std::fs::create_dir_all(&local).is_ok() {
        return local;
    }
    let temp = std::env::temp_dir().join("cargo_slide_cache");
    let _ = std::fs::create_dir_all(&temp);
    temp
}

/// Compiler for Typst presentation files
pub struct SlideCompiler {
    typst_path: PathBuf,
}

impl Default for SlideCompiler {
    fn default() -> Self {
        Self::new().expect("Failed to locate Typst executable")
    }
}

impl SlideCompiler {
    /// Locate typst compiler on the system
    pub fn new() -> Result<Self> {
        let typst_path = Self::find_typst().ok_or_else(|| {
            SlideError::Compilation(
                "Typst executable not found. Please install Typst or ensure it is on your PATH, or set the TYPST_PATH environment variable."
                    .to_string(),
            )
        })?;
        Ok(Self { typst_path })
    }

    /// Construct with a specific typst executable path
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            typst_path: path.into(),
        }
    }

    /// Cross-platform detection of Typst executable (Linux, macOS, Windows)
    fn find_typst() -> Option<PathBuf> {
        // 1. Check explicit environment variable TYPST_PATH
        if let Some(env_path) = std::env::var_os("TYPST_PATH") {
            let p = PathBuf::from(env_path);
            if p.exists() {
                return Some(p);
            }
        }

        // 2. Test if `typst` is directly callable on PATH (works on Linux, macOS, Windows)
        if let Ok(output) = Command::new("typst").arg("--version").output()
            && output.status.success()
        {
            return Some(PathBuf::from("typst"));
        }

        // 3. Platform-specific fallbacks
        #[cfg(windows)]
        {
            // Try `where.exe typst` on Windows
            if let Ok(output) = Command::new("where").arg("typst").output() {
                if output.status.success() {
                    let path_str = String::from_utf8_lossy(&output.stdout);
                    if let Some(first_line) = path_str.lines().next() {
                        let trimmed = first_line.trim();
                        if !trimmed.is_empty() {
                            return Some(PathBuf::from(trimmed));
                        }
                    }
                }
            }

            // Check %USERPROFILE%\.cargo\bin\typst.exe
            if let Some(user_profile) = std::env::var_os("USERPROFILE") {
                let cargo_bin = PathBuf::from(user_profile).join(".cargo\\bin\\typst.exe");
                if cargo_bin.exists() {
                    return Some(cargo_bin);
                }
            }

            // Check common local app data
            if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
                let scoop_bin = PathBuf::from(&local_app_data).join("Programs\\typst\\typst.exe");
                if scoop_bin.exists() {
                    return Some(scoop_bin);
                }
            }
        }

        #[cfg(not(windows))]
        {
            // Try `which typst` on Unix / macOS
            if let Ok(output) = Command::new("which").arg("typst").output()
                && output.status.success()
            {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    return Some(PathBuf::from(path_str));
                }
            }

            // Check ~/.cargo/bin/typst
            if let Some(home) = std::env::var_os("HOME") {
                let cargo_bin = PathBuf::from(&home).join(".cargo/bin/typst");
                if cargo_bin.exists() {
                    return Some(cargo_bin);
                }

                // Check macOS Homebrew paths
                let homebrew_arm = PathBuf::from("/opt/homebrew/bin/typst");
                if homebrew_arm.exists() {
                    return Some(homebrew_arm);
                }

                let homebrew_intel = PathBuf::from("/usr/local/bin/typst");
                if homebrew_intel.exists() {
                    return Some(homebrew_intel);
                }
            }
        }

        None
    }

    /// Compile a Typst presentation file into a complete `SlideDeck` using the incremental rendering cache.
    pub fn compile_file(
        &self,
        typ_file: &Path,
    ) -> Result<SlideDeck> {
        self.compile_file_with_options(typ_file, true)
    }

    /// Compile a Typst presentation file with explicit control over incremental rendering caching.
    pub fn compile_file_with_options(
        &self,
        typ_file: &Path,
        use_cache: bool,
    ) -> Result<SlideDeck> {
        if !typ_file.exists() {
            return Err(SlideError::Compilation(format!(
                "File does not exist: {}",
                typ_file.display()
            )));
        }

        // Safe root directory determination: handle "slides.typ" vs "path/to/slides.typ"
        let root_dir = typ_file
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map_or_else(
                || std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                std::path::Path::to_path_buf,
            );

        // Ensure slide.typ exists in root_dir so `#import "slide.typ": *` always resolves
        let macro_path = root_dir.join("slide.typ");
        if !macro_path.exists() {
            let _ = std::fs::write(&macro_path, slide_theme::SLIDE_MACROS);
        }

        // Preprocess any SQLite database and JSON chart queries so Typst can load cached CSV
        preprocess_charts(typ_file, &root_dir);

        let source_bytes = std::fs::read(typ_file).map_err(|e| {
            SlideError::Compilation(format!("Failed to read {}: {e}", typ_file.display()))
        })?;
        let current_source_hash = compute_source_hash(&source_bytes);
        let source_str = String::from_utf8_lossy(&source_bytes);
        let extracted_notes = extract_speaker_notes_by_slide(&source_str);

        let cache_dir = get_render_cache_dir(typ_file);
        let canonical_path_str = typ_file
            .canonicalize()
            .unwrap_or_else(|_| typ_file.to_path_buf())
            .to_string_lossy()
            .to_string();
        let file_key = format!(
            "{:016x}",
            compute_source_hash(canonical_path_str.as_bytes())
        );
        let manifest_path = cache_dir.join(format!("manifest_{file_key}.json"));

        let previous_manifest: Option<RenderCacheManifest> = if use_cache && manifest_path.is_file()
        {
            std::fs::read_to_string(&manifest_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
        } else {
            None
        };

        // Level 1 Cache Hit: If source hash is completely identical and use_cache is enabled
        if let Some(ref m) = previous_manifest
            && m.source_hash == current_source_hash
            && !m.deck.slides.is_empty()
        {
            crate::logger::log_event(
                "info",
                &format!(
                    "[CACHE] Incremental rendering cache hit for {}: loaded {} slides in <1ms",
                    typ_file.display(),
                    m.deck.total_slides()
                ),
                Some(serde_json::json!({
                    "stage": "render_cache_hit",
                    "file": typ_file.display().to_string(),
                    "total_slides": m.deck.total_slides(),
                })),
            );
            return Ok(m.deck.clone());
        }

        // Cache Miss / Partial Miss: Execute Typst compile
        let temp_dir = tempdir()?;
        let output_template = temp_dir.path().join("slide-{p}.svg");
        let output_str = output_template.to_string_lossy().to_string();

        let mut cmd = Command::new(&self.typst_path);
        cmd.arg("compile").arg(typ_file).arg(&output_str);

        // Auto-detect project font directories and TYPST_FONT_PATHS for cross-platform deterministic rendering
        let fonts_dir = root_dir.join("fonts");
        if fonts_dir.is_dir() {
            cmd.arg("--font-path").arg(&fonts_dir);
        }
        let assets_fonts_dir = root_dir.join("assets").join("fonts");
        if assets_fonts_dir.is_dir() {
            cmd.arg("--font-path").arg(&assets_fonts_dir);
        }
        if let Ok(extra_fonts) = std::env::var("TYPST_FONT_PATHS") {
            for p in std::env::split_paths(&extra_fonts) {
                cmd.arg("--font-path").arg(p);
            }
        }

        cmd.arg("--root").arg(&root_dir);

        let output = cmd.output().map_err(|e| {
            SlideError::Compilation(format!(
                "Failed to execute typst at {:?}: {}",
                self.typst_path, e
            ))
        })?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        if !output.status.success() {
            let full_err = if stderr.trim().is_empty() {
                stdout.trim()
            } else {
                stderr.trim()
            };
            crate::logger::log_event(
                "error",
                &format!("Typst compilation failed:\n{full_err}"),
                Some(serde_json::json!({
                    "stage": "typst_compile",
                    "status": "failed",
                    "error": full_err,
                    "file": typ_file.display().to_string(),
                })),
            );
            return Err(SlideError::Compilation(format!(
                "Typst compilation failed:\n{full_err}"
            )));
        } else if !stderr.trim().is_empty() {
            let (warn_text, has_font_warning, missing_fonts) = format_typst_warnings(&stderr);
            crate::logger::log_event(
                "warn",
                &warn_text,
                Some(serde_json::json!({
                    "stage": "typst_compile",
                    "status": "warning",
                    "warning": stderr.trim(),
                    "has_font_warning": has_font_warning,
                    "missing_fonts": missing_fonts,
                    "file": typ_file.display().to_string(),
                })),
            );
        }

        // Collect all generated SVGs in numerical page order with Level 2 per-slide incremental caching
        let mut slides = Vec::new();
        let mut slide_hashes = Vec::new();
        let mut page_num: usize = 1;
        let mut reused_slide_count = 0;
        let mut parsed_slide_count = 0;

        loop {
            let svg_file = temp_dir.path().join(format!("slide-{page_num}.svg"));
            if !svg_file.exists() {
                break;
            }

            let raw_svg = std::fs::read_to_string(&svg_file)?;
            let raw_svg_hash = compute_source_hash(raw_svg.as_bytes());
            slide_hashes.push(raw_svg_hash);

            // Level 2 Per-Slide Cache: Check if this slide's SVG matches previously parsed slide
            let cached_slide = previous_manifest.as_ref().and_then(|prev| {
                if prev.slide_hashes.get(page_num.saturating_sub(1)) == Some(&raw_svg_hash) {
                    prev.deck.slides.get(page_num.saturating_sub(1)).cloned()
                } else {
                    None
                }
            });

            if let Some(mut s) = cached_slide {
                s.page_number = page_num;
                if s.notes.is_none() {
                    s.notes = extracted_notes
                        .get(page_num.saturating_sub(1))
                        .cloned()
                        .flatten();
                }
                slides.push(s);
                reused_slide_count += 1;
            } else {
                let svg_content = crate::svg::sanitize_svg(&raw_svg);
                let svg_info =
                    crate::svg::parse_svg_slide_with_root(&svg_content, Some(&root_dir))?;

                let note = extracted_notes
                    .get(page_num.saturating_sub(1))
                    .cloned()
                    .flatten();

                slides.push(Slide {
                    page_number: page_num,
                    svg_data: svg_content,
                    view_box: svg_info.view_box,
                    hotspots: svg_info.hotspots,
                    animation: svg_info.transition,
                    steps: svg_info.steps,
                    notes: note,
                });
                parsed_slide_count += 1;
            }

            page_num += 1;
        }

        if slides.is_empty() {
            return Err(SlideError::Compilation(
                "No slides were generated from the Typst document.".to_string(),
            ));
        }

        if reused_slide_count > 0 {
            crate::logger::log_event(
                "info",
                &format!(
                    "[CACHE] Incremental slide render: {reused_slide_count} slides reused from cache, {parsed_slide_count} slides updated",
                ),
                Some(serde_json::json!({
                    "stage": "incremental_slide_reuse",
                    "reused": reused_slide_count,
                    "updated": parsed_slide_count,
                    "total": slides.len(),
                })),
            );
        }

        // Check for Typst slide content overflow
        check_slide_overflow(typ_file, &root_dir, &slides)?;

        let title = typ_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Slide Deck")
            .to_string();

        let mut deck = SlideDeck::new(title);
        deck.slides = slides;

        // Persist to incremental rendering cache
        if use_cache {
            let manifest = RenderCacheManifest {
                source_hash: current_source_hash,
                source_path: canonical_path_str,
                deck: deck.clone(),
                slide_hashes,
            };
            if let Ok(manifest_json) = serde_json::to_string(&manifest) {
                let _ = std::fs::write(&manifest_path, manifest_json);
            }
        }

        Ok(deck)
    }

    /// Clear the persistent rendering cache for a given typst file
    pub fn clear_render_cache(typ_file: &Path) {
        let cache_dir = get_render_cache_dir(typ_file);
        let canonical_path_str = typ_file
            .canonicalize()
            .unwrap_or_else(|_| typ_file.to_path_buf())
            .to_string_lossy()
            .to_string();
        let file_key = format!(
            "{:016x}",
            compute_source_hash(canonical_path_str.as_bytes())
        );
        let manifest_path = cache_dir.join(format!("manifest_{file_key}.json"));
        let _ = std::fs::remove_file(manifest_path);
    }

    /// Compile a Typst presentation directly to PDF
    pub fn compile_to_pdf(
        &self,
        typ_file: &Path,
        output_pdf: &Path,
    ) -> Result<()> {
        let mut cmd = Command::new(&self.typst_path);
        cmd.arg("compile").arg(typ_file).arg(output_pdf);

        let root_dir = typ_file
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map_or_else(
                || std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                std::path::Path::to_path_buf,
            );

        let fonts_dir = root_dir.join("fonts");
        if fonts_dir.is_dir() {
            cmd.arg("--font-path").arg(&fonts_dir);
        }
        let assets_fonts_dir = root_dir.join("assets").join("fonts");
        if assets_fonts_dir.is_dir() {
            cmd.arg("--font-path").arg(&assets_fonts_dir);
        }
        if let Ok(extra_fonts) = std::env::var("TYPST_FONT_PATHS") {
            for p in std::env::split_paths(&extra_fonts) {
                cmd.arg("--font-path").arg(p);
            }
        }

        cmd.arg("--root").arg(&root_dir);

        let output = cmd
            .output()
            .map_err(|e| SlideError::Compilation(format!("Failed to execute typst: {e}")))?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            return Err(SlideError::Compilation(format!(
                "Typst compilation failed:\n{stderr}"
            )));
        } else if !stderr.trim().is_empty() {
            let (warn_text, has_font_warning, missing_fonts) = format_typst_warnings(&stderr);
            crate::logger::log_event(
                "warn",
                &warn_text,
                Some(serde_json::json!({
                    "stage": "typst_compile_pdf",
                    "status": "warning",
                    "warning": stderr.trim(),
                    "has_font_warning": has_font_warning,
                    "missing_fonts": missing_fonts,
                    "file": typ_file.display().to_string(),
                })),
            );
        }

        Ok(())
    }
}

/// Parse and format Typst compiler stderr output, segregating font fallback notices from critical warnings
#[must_use]
pub fn format_typst_warnings(stderr: &str) -> (String, bool, Vec<String>) {
    let mut missing_fonts = Vec::new();
    let mut other_blocks = Vec::new();
    let mut current_block = Vec::new();
    let mut is_font_block = false;
    let mut font_name = String::new();

    for line in stderr.lines() {
        if line.starts_with("warning:") || line.starts_with("error:") {
            if !current_block.is_empty() {
                if is_font_block && !font_name.is_empty() {
                    if !missing_fonts.contains(&font_name) {
                        missing_fonts.push(font_name.clone());
                    }
                } else {
                    other_blocks.push(current_block.join("\n"));
                }
                current_block.clear();
            }
            if let Some(rest) = line.strip_prefix("warning: unknown font family:") {
                is_font_block = true;
                font_name = rest.trim().to_string();
            } else {
                is_font_block = false;
                font_name.clear();
            }
        }
        current_block.push(line);
    }

    if !current_block.is_empty() {
        if is_font_block && !font_name.is_empty() {
            if !missing_fonts.contains(&font_name) {
                missing_fonts.push(font_name);
            }
        } else {
            other_blocks.push(current_block.join("\n"));
        }
    }

    let mut output = String::new();
    if !other_blocks.is_empty() {
        output.push_str(&format!(
            "[WARN] Typst compiler warning:\n{}\n",
            other_blocks.join("\n\n")
        ));
    }

    let has_font_warnings = !missing_fonts.is_empty();
    if has_font_warnings {
        if !other_blocks.is_empty() {
            output.push('\n');
        }
        output.push_str(&format!(
            "[INFO] Typst font fallback notice:\n   The following font families were not found: {}\n   Typst automatically falls back to available system fonts.\n[TIP] Missing fonts will fall back to system defaults. You can bundle custom fonts by placing .ttf or .otf files into the 'fonts/' or 'assets/fonts/' directory of your project.",
            missing_fonts.join(", ")
        ));
    }

    (output.trim().to_string(), has_font_warnings, missing_fonts)
}

#[derive(Debug, Clone)]
pub struct DeclaredSlide {
    pub line_number: usize,
    pub title: String,
}

#[must_use]
pub fn parse_declared_slides(
    typ_file: &Path,
    _root_dir: &Path,
) -> Vec<DeclaredSlide> {
    let content = match std::fs::read_to_string(typ_file) {
        | Ok(c) => c,
        | Err(_) => return Vec::new(),
    };

    let mut declared = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }

        // Title slide declaration
        if trimmed.starts_with("#title-slide") && !trimmed.starts_with("#let title-slide") {
            let mut title = "Title Slide".to_string();
            for l in &lines[idx..lines.len().min(idx + 10)] {
                if let Some(t_idx) = l.find("title:") {
                    let rest = &l[t_idx + 6..];
                    if let Some(q1) = rest.find('"')
                        && let Some(q2) = rest[q1 + 1..].find('"')
                    {
                        title = rest[q1 + 1..q1 + 1 + q2].to_string();
                        break;
                    }
                }
            }
            declared.push(DeclaredSlide {
                line_number: idx + 1,
                title,
            });
        } else if (trimmed.starts_with("#slide")
            || trimmed.starts_with("#centered-slide")
            || trimmed.starts_with("#focus-slide")
            || (trimmed.starts_with('#')
                && trimmed[1..].contains("-slide")
                && !trimmed.contains("title-slide")))
            && !trimmed.starts_with("#let ")
        {
            let mut title = format!("Slide {}", declared.len() + 1);
            for l in &lines[idx..lines.len().min(idx + 10)] {
                if let Some(t_idx) = l.find("title:") {
                    let rest = &l[t_idx + 6..];
                    if let Some(q1) = rest.find('"')
                        && let Some(q2) = rest[q1 + 1..].find('"')
                    {
                        title = rest[q1 + 1..q1 + 1 + q2].to_string();
                        break;
                    }
                }
            }
            declared.push(DeclaredSlide {
                line_number: idx + 1,
                title,
            });
        }
    }

    declared
}

pub fn check_slide_overflow(
    typ_file: &Path,
    root_dir: &Path,
    slides: &[Slide],
) -> Result<()> {
    let declared = parse_declared_slides(typ_file, root_dir);
    if declared.is_empty() {
        return Ok(());
    }

    let compiled_count = slides.len();
    let declared_count = declared.len();

    if compiled_count > declared_count {
        let extra = compiled_count - declared_count;

        let mut problem_idx = declared_count.saturating_sub(1);
        let mut spillover_page = compiled_count;

        // Detect spillover pages using slide-meta markers embedded by slide.typ
        let has_any_meta = slides.iter().any(|s| s.svg_data.contains("slide-meta:"));
        if has_any_meta {
            let mut current_decl_idx: usize = 0;
            for (p_i, s) in slides.iter().enumerate() {
                if s.svg_data.contains("slide-meta:") {
                    current_decl_idx += 1;
                } else {
                    // This compiled page has no slide-meta declaration, so it is a spillover of the preceding slide
                    problem_idx = current_decl_idx.saturating_sub(1);
                    spillover_page = p_i + 1;
                    break;
                }
            }
        }

        let problem_slide = &declared[problem_idx.min(declared.len().saturating_sub(1))];

        let msg = format!(
            "Typst slide content overflow detected!\n\
             The presentation source declares {} slide(s), but Typst compiled {} pages ({} extra spillover page(s)).\n\n\
             Overflow location:\n  \
             Slide {} (\"{}\", line {}) exceeded the vertical 16:9 canvas bounds.\n  \
             Spillover content pushed onto compiled page {}.\n\n\
             Remediation suggestions:\n  \
             1. Reduce component heights (e.g. adjust chart height or image dimensions).\n  \
             2. Reduce vertical spacing (e.g. change #v(...) to a smaller distance) or font size.\n  \
             3. Split the content into an additional slide using #slide(title: \"...\").\n\n\
             (To bypass this error, set CARGO_SLIDE_ALLOW_OVERFLOW=1 in your environment)",
            declared_count,
            compiled_count,
            extra,
            problem_idx + 1,
            problem_slide.title,
            problem_slide.line_number,
            spillover_page
        );

        if let Ok(val) = std::env::var("CARGO_SLIDE_ALLOW_OVERFLOW")
            && (val == "1" || val.eq_ignore_ascii_case("true"))
        {
            crate::logger::log_event(
                "warn",
                &format!("\n[WARN] {msg}\n"),
                Some(serde_json::json!({
                    "event": "slide_overflow_warning",
                    "declared_slides": declared_count,
                    "compiled_pages": compiled_count,
                    "overflow_slide": problem_idx + 1,
                    "overflow_title": &problem_slide.title,
                    "line": problem_slide.line_number,
                })),
            );
            return Ok(());
        }

        crate::logger::log_event(
            "error",
            &msg,
            Some(serde_json::json!({
                "event": "slide_overflow_error",
                "declared_slides": declared_count,
                "compiled_pages": compiled_count,
                "overflow_slide": problem_idx + 1,
                "overflow_title": &problem_slide.title,
                "line": problem_slide.line_number,
                "spillover_page": spillover_page,
            })),
        );

        return Err(SlideError::Overflow(msg));
    }

    Ok(())
}

fn preprocess_charts(
    typ_file: &Path,
    root_dir: &Path,
) {
    use crate::chart::ChartData;
    use crate::chart::ChartType;

    let content = match std::fs::read_to_string(typ_file) {
        | Ok(c) => c,
        | Err(_) => return,
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(src_start) = trimmed.find("source:") {
            let rest = &trimmed[src_start + 7..];
            if let Some(quote_start) = rest.find('"')
                && let Some(quote_end) = rest[quote_start + 1..].find('"')
            {
                let source_val = &rest[quote_start + 1..quote_start + 1 + quote_end];

                let (db_path_str, query_opt) = match source_val.split_once('?') {
                    | Some((p, q)) => {
                        let query_str = q.strip_prefix("query=").unwrap_or(q);
                        (p, Some(query_str.replace('+', " ")))
                    },
                    | None => (source_val, None),
                };

                let file_path = if Path::new(db_path_str).is_absolute() {
                    PathBuf::from(db_path_str)
                } else {
                    root_dir.join(db_path_str)
                };

                if file_path.exists() {
                    let query = query_opt.unwrap_or_else(|| "SELECT * FROM data".to_string());
                    let chart_data_res = if db_path_str.ends_with(".db")
                        || db_path_str.ends_with(".sqlite")
                    {
                        ChartData::from_sqlite(&file_path, &query, ChartType::Bar, None)
                    } else if db_path_str.ends_with(".json") || db_path_str.ends_with(".jsonl") {
                        ChartData::from_json_file(&file_path, ChartType::Bar, None)
                    } else if db_path_str.ends_with(".csv") {
                        ChartData::from_csv_file(&file_path, ChartType::Bar, None)
                    } else {
                        continue;
                    };

                    if let Ok(data) = chart_data_res {
                        let new_csv = data.to_csv();
                        let cache_csv = format!("{}.cache.csv", file_path.display());
                        let should_write = std::fs::read_to_string(&cache_csv)
                            .map_or(true, |existing| existing != new_csv);
                        if should_write {
                            let _ = std::fs::write(&cache_csv, &new_csv);
                        }
                        if db_path_str.ends_with(".db") || db_path_str.ends_with(".sqlite") {
                            let alt_cache = file_path.with_extension("db.cache.csv");
                            let should_write_alt = std::fs::read_to_string(&alt_cache)
                                .map_or(true, |existing| existing != new_csv);
                            if should_write_alt {
                                let _ = std::fs::write(&alt_cache, &new_csv);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Extract speaker notes per slide from Typst source code
#[must_use]
pub fn extract_speaker_notes_by_slide(source: &str) -> Vec<Option<String>> {
    let mut results = Vec::new();
    let mut current_notes: Vec<String> = Vec::new();
    let mut has_slide_started = false;

    for line in source.lines() {
        let trimmed = line.trim();
        let is_pagebreak = trimmed.starts_with("#pagebreak()");
        let is_slide_macro = (trimmed.starts_with("#slide")
            || trimmed.starts_with("#title-slide")
            || trimmed.starts_with("#centered-slide")
            || trimmed.starts_with("#focus-slide"))
            && !trimmed.starts_with("#let ");

        if is_pagebreak {
            let note_text = if current_notes.is_empty() {
                None
            } else {
                Some(current_notes.join("\n").trim().to_string())
            };
            results.push(note_text);
            current_notes.clear();
            has_slide_started = true;
        } else if is_slide_macro {
            if has_slide_started {
                let note_text = if current_notes.is_empty() {
                    None
                } else {
                    Some(current_notes.join("\n").trim().to_string())
                };
                results.push(note_text);
                current_notes.clear();
            }
            has_slide_started = true;
        }

        // Check for note patterns (strictly distinct from general code comments)
        if let Some(rest) = trimmed
            .strip_prefix("// [note]:")
            .or_else(|| trimmed.strip_prefix("// [notes]:"))
            .or_else(|| trimmed.strip_prefix("// [note]"))
            .or_else(|| trimmed.strip_prefix("// [notes]"))
            .or_else(|| trimmed.strip_prefix("// [NOTE]:"))
            .or_else(|| trimmed.strip_prefix("// [NOTES]:"))
            .or_else(|| trimmed.strip_prefix("// [NOTE]"))
            .or_else(|| trimmed.strip_prefix("// [NOTES]"))
            .or_else(|| trimmed.strip_prefix("// Note:"))
            .or_else(|| trimmed.strip_prefix("// note:"))
            .or_else(|| trimmed.strip_prefix("// NOTE:"))
            .or_else(|| trimmed.strip_prefix("// Notes:"))
            .or_else(|| trimmed.strip_prefix("// notes:"))
            .or_else(|| trimmed.strip_prefix("// NOTES:"))
            .or_else(|| trimmed.strip_prefix("// Speaker:"))
            .or_else(|| trimmed.strip_prefix("// speaker:"))
            .or_else(|| trimmed.strip_prefix("// SPEAKER:"))
            .or_else(|| trimmed.strip_prefix("// Speaker Note:"))
            .or_else(|| trimmed.strip_prefix("// speaker note:"))
            .or_else(|| trimmed.strip_prefix("// SPEAKER NOTE:"))
            .or_else(|| trimmed.strip_prefix("// [speaker]:"))
            .or_else(|| trimmed.strip_prefix("// [speaker]"))
            .or_else(|| trimmed.strip_prefix("// [SPEAKER]:"))
            .or_else(|| trimmed.strip_prefix("// [SPEAKER]"))
            .or_else(|| trimmed.strip_prefix("// [speaker note]:"))
            .or_else(|| trimmed.strip_prefix("// [SPEAKER NOTE]:"))
            .or_else(|| trimmed.strip_prefix("// [speaker_note]:"))
            .or_else(|| trimmed.strip_prefix("// [SPEAKER_NOTE]:"))
        {
            let n = rest.trim();
            if !n.is_empty() {
                current_notes.push(n.to_string());
            }
        } else if (trimmed.starts_with("/* [note]:")
            || trimmed.starts_with("/* [NOTE]:")
            || trimmed.starts_with("/* NOTE:")
            || trimmed.starts_with("/* Note:"))
            && trimmed.ends_with("*/")
        {
            let inner = trimmed
                .trim_start_matches("/* [note]:")
                .trim_start_matches("/* [NOTE]:")
                .trim_start_matches("/* NOTE:")
                .trim_start_matches("/* Note:")
                .trim_end_matches("*/")
                .trim();
            if !inner.is_empty() {
                current_notes.push(inner.to_string());
            }
        } else if (trimmed.starts_with("<!-- note:")
            || trimmed.starts_with("<!-- notes:")
            || trimmed.starts_with("<!-- speaker note:"))
            && trimmed.ends_with("-->")
        {
            let inner = trimmed
                .trim_start_matches("<!-- note:")
                .trim_start_matches("<!-- notes:")
                .trim_start_matches("<!-- speaker note:")
                .trim_end_matches("-->")
                .trim();
            if !inner.is_empty() {
                current_notes.push(inner.to_string());
            }
        } else if (trimmed.starts_with("#note[") && trimmed.ends_with(']'))
            || (trimmed.starts_with("#speaker-note[") && trimmed.ends_with(']'))
            || (trimmed.starts_with("#speaker_note[") && trimmed.ends_with(']'))
        {
            let prefix_len =
                if trimmed.starts_with("#speaker-note[") || trimmed.starts_with("#speaker_note[") {
                    14
                } else {
                    6
                };
            let inner = trimmed
                .get(prefix_len..trimmed.len().saturating_sub(1))
                .unwrap_or("")
                .trim();
            if !inner.is_empty() {
                current_notes.push(inner.to_string());
            }
        } else if (trimmed.starts_with("#speaker-note(\"") && trimmed.ends_with("\")"))
            || (trimmed.starts_with("#speaker_note(\"") && trimmed.ends_with("\")"))
            || (trimmed.starts_with("#note(\"") && trimmed.ends_with("\")"))
        {
            let prefix_len = if trimmed.starts_with("#speaker-note(\"")
                || trimmed.starts_with("#speaker_note(\"")
            {
                15
            } else {
                7
            };
            let inner = trimmed
                .get(prefix_len..trimmed.len().saturating_sub(2))
                .unwrap_or("")
                .trim();
            if !inner.is_empty() {
                current_notes.push(inner.to_string());
            }
        }
    }

    let final_note = if current_notes.is_empty() {
        None
    } else {
        Some(current_notes.join("\n").trim().to_string())
    };
    results.push(final_note);

    results
}

/// Issue severity for presentation health checks
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HealthSeverity {
    /// Informational suggestion
    Info,
    /// Warning that may affect presentation quality
    Warning,
    /// Error preventing successful presentation
    Error,
}

/// A detected presentation issue or suggestion
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PresentationHealthIssue {
    /// Severity level
    pub severity: HealthSeverity,
    /// Slide index if issue is specific to a slide
    pub slide_index: Option<usize>,
    /// Human-readable diagnostic description
    pub message: String,
    /// Actionable suggestion to remediate
    pub suggestion: Option<String>,
}

/// Analyze a presentation deck and source code for common pitfalls and formatting issues.
#[must_use]
pub fn check_presentation_health(
    deck: &SlideDeck,
    source: &str,
    project_root: Option<&Path>,
) -> Vec<PresentationHealthIssue> {
    let mut issues = Vec::new();

    // 1. Check total slide count
    if deck.slides.is_empty() {
        issues.push(PresentationHealthIssue {
            severity: HealthSeverity::Error,
            slide_index: None,
            message: "Presentation has no slides.".to_string(),
            suggestion: Some("Add at least one slide with `= Title` or `#slide(...)`.".to_string()),
        });
    }

    // 2. Check each slide for text density and media assets
    for (idx, slide) in deck.slides.iter().enumerate() {
        let text_in_svg = slide
            .svg_data
            .lines()
            .filter(|l| l.contains("<text"))
            .count();
        if text_in_svg > 150 {
            issues.push(PresentationHealthIssue {
                severity: HealthSeverity::Warning,
                slide_index: Some(idx),
                message: format!(
                    "Slide {} has very high visual text density ({} text elements).",
                    idx.saturating_add(1),
                    text_in_svg
                ),
                suggestion: Some(
                    "Consider splitting this slide into two slides to improve audience readability."
                        .to_string(),
                ),
            });
        }

        // Check for empty / content-less slide
        if text_in_svg == 0 && slide.hotspots.is_empty() {
            issues.push(PresentationHealthIssue {
                severity: HealthSeverity::Warning,
                slide_index: Some(idx),
                message: format!(
                    "Slide {} appears to be blank or has no rendered text/hotspots.",
                    idx.saturating_add(1)
                ),
                suggestion: Some("Add slide content or remove unnecessary pagebreaks.".to_string()),
            });
        }

        // Pacing bottleneck check (slides > 4 minutes / 240 seconds)
        let est_sec = slide.estimated_speaking_seconds();
        if est_sec > 240 {
            let est_min = (est_sec as f32 / 60.0).round();
            issues.push(PresentationHealthIssue {
                severity: HealthSeverity::Warning,
                slide_index: Some(idx),
                message: format!(
                    "Slide {} has an estimated speaking duration of ~{est_min:.0} minutes (density: {} words/chars).",
                    idx.saturating_add(1),
                    slide.notes_word_count()
                ),
                suggestion: Some(
                    "Consider breaking this slide into multiple sequential slides to keep pacing dynamic."
                        .to_string(),
                ),
            });
        }

        // Check for media and chart file hotspots
        for hs in &slide.hotspots {
            match hs {
                | crate::model::Hotspot::Video {
                    source: media_src, ..
                }
                | crate::model::Hotspot::Audio {
                    source: media_src, ..
                } => {
                    if !media_src.contains("://") && !media_src.starts_with('#') {
                        let clean = media_src.strip_prefix("file://").unwrap_or(media_src);
                        let exists = if let Some(root) = project_root {
                            root.join(clean).exists() || Path::new(clean).exists()
                        } else {
                            Path::new(clean).exists()
                        };
                        if !exists {
                            issues.push(PresentationHealthIssue {
                                severity: HealthSeverity::Warning,
                                slide_index: Some(idx),
                                message: format!(
                                    "Slide {}: Media asset file not found: '{}'",
                                    idx.saturating_add(1),
                                    clean
                                ),
                                suggestion: Some(
                                    "Check the media file path or copy the file into the project directory."
                                        .to_string(),
                                ),
                            });
                        }
                    }
                },
                | crate::model::Hotspot::Chart { data, .. } => {
                    if data.categories.is_empty() && data.series.is_empty() {
                        issues.push(PresentationHealthIssue {
                            severity: HealthSeverity::Info,
                            slide_index: Some(idx),
                            message: format!(
                                "Slide {}: Interactive chart has empty data categories and series.",
                                idx.saturating_add(1)
                            ),
                            suggestion: Some(
                                "Provide a dataset (.csv, .json, or .db) or inline series values."
                                    .to_string(),
                            ),
                        });
                    }
                },
                | _ => {},
            }
        }
    }

    // 3. Speaker notes coverage check across entire deck
    if deck.slides.len() >= 3 && !deck.has_any_notes() {
        issues.push(PresentationHealthIssue {
            severity: HealthSeverity::Info,
            slide_index: None,
            message: format!(
                "Deck has {} slides, but no speaker notes were detected.",
                deck.slides.len()
            ),
            suggestion: Some(
                "Adding presenter talking notes (using '// Note: ...' or '#speaker-note[...]') helps ensure smooth timing."
                    .to_string(),
            ),
        });
    }

    // 4. Check source text for common Typst slide issues
    if !source.contains("#import \"slide.typ\"")
        && !source.contains("#import \"theme.typ\"")
        && source.contains("#slide(")
    {
        issues.push(PresentationHealthIssue {
            severity: HealthSeverity::Error,
            slide_index: None,
            message:
                "Missing slide macros import: source uses '#slide(' but does not import 'slide.typ'."
                    .to_string(),
            suggestion: Some(
                "Add `#import \"slide.typ\": *` at the top of your document.".to_string(),
            ),
        });
    }

    issues
}
