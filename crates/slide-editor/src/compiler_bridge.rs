//! Typst compiler bridge and multi-format exporter (PDF, SVG, PNG, .slide).

use crate::document::EditorDocument;
use slide_core::compiler::SlideCompiler;
use slide_core::error::Result;
use slide_core::error::SlideError;
use slide_core::model::SlideDeck;
use slide_core::package::pack_deck_with_assets;
use std::path::Path;
use std::path::PathBuf;

/// An automated, 1-click quick-fix recommendation for a compiler diagnostic
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickFix {
    /// Human-friendly button label
    pub label: String,
    /// Code snippet to insert at document beginning if applicable
    pub prepend_text: Option<String>,
    /// Replacement target and string if applicable
    pub replace_pair: Option<(String, String)>,
}

/// Diagnostic error report from Typst compilation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerDiagnostic {
    /// Error summary or primary message
    pub message: String,
    /// Line number (if parsed from Typst stderr)
    pub line: Option<usize>,
    /// Column number (if parsed from Typst stderr)
    pub column: Option<usize>,
    /// Full stderr text
    pub full_stderr: String,
    /// Optional automated 1-click quick fix recommendation
    pub quick_fix: Option<QuickFix>,
}

impl std::fmt::Display for CompilerDiagnostic {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        if let Some(l) = self.line {
            write!(f, "Line {l}: {}", self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

/// Compilation bridge service
pub struct CompilerBridge {
    compiler: SlideCompiler,
}

impl Default for CompilerBridge {
    fn default() -> Self {
        Self::new()
    }
}

fn contains_macro_call(
    source: &str,
    macro_name: &str,
) -> bool {
    // 1. Check for `#macro_name`
    let needle = format!("#{macro_name}");
    let mut search_from = 0;
    while let Some(pos) = source[search_from..].find(&needle) {
        let abs_pos = search_from + pos;
        let after_idx = abs_pos + needle.len();
        let remaining = &source[after_idx..];
        let next_char = remaining.chars().next();
        match next_char {
            | None => return true,
            | Some(c) if c == '(' || c == '[' || c == '{' => return true,
            | Some(c) if c.is_whitespace() => {
                let trimmed = remaining.trim_start();
                if trimmed.starts_with('(') || trimmed.starts_with('[') || trimmed.starts_with('{')
                {
                    return true;
                }
            },
            | _ => {},
        }
        search_from = after_idx;
    }

    // 2. Check for `macro_name(` or `macro_name[` in code blocks
    let mut search_from = 0;
    while let Some(pos) = source[search_from..].find(macro_name) {
        let abs_pos = search_from + pos;
        let is_prev_ident = if abs_pos > 0 {
            let prev_char = source[..abs_pos].chars().next_back().unwrap_or(' ');
            prev_char.is_alphanumeric() || prev_char == '_' || prev_char == '-'
        } else {
            false
        };
        if !is_prev_ident {
            let after_idx = abs_pos + macro_name.len();
            let remaining = &source[after_idx..];
            let next_char = remaining.chars().next();
            match next_char {
                | Some(c) if c == '(' || c == '[' => return true,
                | Some(c) if c.is_whitespace() => {
                    let trimmed = remaining.trim_start();
                    if trimmed.starts_with('(') || trimmed.starts_with('[') {
                        return true;
                    }
                },
                | _ => {},
            }
        }
        search_from = abs_pos + macro_name.len();
    }

    false
}

impl CompilerBridge {
    /// Create new compiler bridge instance
    #[must_use]
    pub fn new() -> Self {
        let compiler = SlideCompiler::new()
            .unwrap_or_else(|_| SlideCompiler::with_path(PathBuf::from("typst")));
        Self { compiler }
    }

    /// Checks if source code references any cargo-slide macros (e.g. #slide, #title-slide, #step, etc.)
    #[must_use]
    pub fn has_slide_macros(source: &str) -> bool {
        const MACRO_NAMES: &[&str] = &[
            "title-slide",
            "slide",
            "step",
            "video",
            "audio",
            "audio-player",
            "chart",
            "cols",
            "badge",
            "callout",
            "code-window",
            "metric",
            "speaker-notes",
            "two-columns",
            "slide-theme",
            "centered-slide",
            "focus-slide",
        ];

        for &name in MACRO_NAMES {
            if contains_macro_call(source, name) {
                return true;
            }
        }
        false
    }

    /// Checks if source code explicitly defines or imports slide macros
    #[must_use]
    pub fn has_macro_definitions(source: &str) -> bool {
        source.contains("slide.typ")
            || source.contains("theme.typ")
            || source.contains("#let title-slide")
            || source.contains("#let slide(")
            || source.contains("#let slide =")
            || source.contains("#let slide[")
            || source.contains("@preview/polylux")
            || source.contains("@preview/touying")
            || source.contains("@preview/slydst")
    }

    /// Compile document source text into a SlideDeck
    /// Ensure Typst source has slide macros imported if macros are referenced
    #[must_use]
    pub fn prepare_typst_source_with_macros(source: &str) -> (String, bool) {
        let has_macro_ref = Self::has_slide_macros(source);
        let has_macro_def = Self::has_macro_definitions(source);

        if has_macro_ref && !has_macro_def {
            (format!("#import \"slide.typ\": *\n{source}"), true)
        } else {
            (source.to_string(), false)
        }
    }

    /// Compile document source text into a SlideDeck
    #[allow(clippy::result_large_err)]
    pub fn compile_source(
        &self,
        source_text: &str,
        project_root: Option<&Path>,
    ) -> std::result::Result<SlideDeck, CompilerDiagnostic> {
        enum Cleanup {
            File(PathBuf),
            #[allow(dead_code)]
            Dir(tempfile::TempDir),
        }
        impl Drop for Cleanup {
            fn drop(&mut self) {
                if let Self::File(p) = self {
                    let _ = std::fs::remove_file(p);
                }
            }
        }

        let is_user_slide_typ = if let Some(root) = project_root {
            let p = root.join("slide.typ");
            if p.is_file() {
                if let Ok(content) = std::fs::read_to_string(&p) {
                    !content.contains("#let slide-meta")
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        let (mut processed_source, prepended_import) =
            Self::prepare_typst_source_with_macros(source_text);

        if is_user_slide_typ {
            processed_source = processed_source
                .replace("#import \"slide.typ\"", "#import \".slide_macros.typ\"")
                .replace("#import 'slide.typ'", "#import \".slide_macros.typ\"");
        }

        static TMP_FILE_COUNTER: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(0);
        let (temp_file, _cleanup) = if let Some(root) = project_root.filter(|p| p.is_dir()) {
            let counter = TMP_FILE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let pid = std::process::id();
            let tmp = root.join(format!(".slide_preview_tmp_{pid}_{counter}.typ"));
            if prepended_import
                || processed_source.contains("slide.typ")
                || processed_source.contains(".slide_macros.typ")
                || processed_source.contains("theme.typ")
            {
                let dot_macro = root.join(".slide_macros.typ");
                if !dot_macro.exists() {
                    let _ = std::fs::write(&dot_macro, slide_theme::SLIDE_MACROS);
                }
                let slide_typ = root.join("slide.typ");
                if !slide_typ.exists() && !is_user_slide_typ {
                    let _ = std::fs::write(&slide_typ, slide_theme::SLIDE_MACROS);
                }
                let theme_typ = root.join("theme.typ");
                if !theme_typ.exists() {
                    let _ = std::fs::write(&theme_typ, slide_theme::DEFAULT_THEME);
                } else if let Ok(existing_theme) = std::fs::read_to_string(&theme_typ)
                    && existing_theme.contains("slide-theme(")
                    && !existing_theme.contains("header:")
                {
                    let _ = std::fs::write(&theme_typ, slide_theme::DEFAULT_THEME);
                }
            }
            if let Err(e) = std::fs::write(&tmp, &processed_source) {
                // If writing to root directory fails (e.g. read-only filesystem or permissions),
                // fall back to a safe temporary directory so viewing/compiling still succeeds.
                let temp_dir = tempfile::tempdir().map_err(|te| {
                    CompilerDiagnostic {
                        message: format!(
                            "Failed to write temporary typst file in {} ({e}) and tempdir creation failed: {te}",
                            root.display()
                        ),
                        line: None,
                        column: None,
                        full_stderr: String::new(),
                        quick_fix: None,
                    }
                })?;
                let fallback_tmp = temp_dir.path().join("document.typ");
                if prepended_import
                    || processed_source.contains("slide.typ")
                    || processed_source.contains(".slide_macros.typ")
                    || processed_source.contains("theme.typ")
                {
                    let _ = std::fs::write(
                        temp_dir.path().join("slide.typ"),
                        slide_theme::SLIDE_MACROS,
                    );
                    let _ = std::fs::write(
                        temp_dir.path().join(".slide_macros.typ"),
                        slide_theme::SLIDE_MACROS,
                    );
                    let _ = std::fs::write(
                        temp_dir.path().join("theme.typ"),
                        slide_theme::DEFAULT_THEME,
                    );
                }
                std::fs::write(&fallback_tmp, &processed_source).map_err(|te| {
                    CompilerDiagnostic {
                        message: format!("Failed to write fallback temporary typst file: {te}"),
                        line: None,
                        column: None,
                        full_stderr: String::new(),
                        quick_fix: None,
                    }
                })?;
                (fallback_tmp, Cleanup::Dir(temp_dir))
            } else {
                (tmp.clone(), Cleanup::File(tmp))
            }
        } else {
            let temp_dir = tempfile::tempdir().map_err(|e| {
                CompilerDiagnostic {
                    message: format!("Failed to create temporary directory: {e}"),
                    line: None,
                    column: None,
                    full_stderr: String::new(),
                    quick_fix: None,
                }
            })?;
            let slide_typ = temp_dir.path().join("slide.typ");
            let _ = std::fs::write(&slide_typ, slide_theme::SLIDE_MACROS);
            let theme_typ = temp_dir.path().join("theme.typ");
            let _ = std::fs::write(&theme_typ, slide_theme::DEFAULT_THEME);
            let tmp = temp_dir.path().join("document.typ");
            std::fs::write(&tmp, &processed_source).map_err(|e| {
                CompilerDiagnostic {
                    message: format!("Failed to write temporary typst file: {e}"),
                    line: None,
                    column: None,
                    full_stderr: String::new(),
                    quick_fix: None,
                }
            })?;
            (tmp, Cleanup::Dir(temp_dir))
        };

        // Invoke compiler
        match self.compiler.compile_file(&temp_file) {
            | Ok(deck) => Ok(deck),
            | Err(e) => {
                let err_str = e.to_string();
                let mut diagnostic = Self::parse_typst_diagnostic(&err_str, source_text);
                if prepended_import {
                    diagnostic.line = diagnostic.line.map(|l| l.saturating_sub(1).max(1));
                }
                Err(diagnostic)
            },
        }
    }

    /// Parse Typst CLI stderr output for file, line, and column details
    fn parse_typst_diagnostic(
        stderr: &str,
        source_text: &str,
    ) -> CompilerDiagnostic {
        let mut line_num = None;
        let mut col_num = None;
        let mut main_msg = String::new();

        for l in stderr.lines() {
            let trimmed = l.trim();
            // Look for standard Typst error pattern: error: ... at document.typ:12:5
            if let Some(pos) = trimmed.find(".typ:") {
                let suffix = &trimmed[pos + 5..];
                let parts: Vec<&str> = suffix.split(':').collect();
                if let Some(first) = parts.first()
                    && let Ok(num) = first.parse::<usize>()
                {
                    line_num = Some(num);
                }
                if let Some(second) = parts.get(1)
                    && let Ok(num) = second
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .parse::<usize>()
                {
                    col_num = Some(num);
                }
            }
            if (trimmed.starts_with("error:") || trimmed.starts_with("Typst compilation failed:"))
                && main_msg.is_empty()
            {
                main_msg = trimmed.to_string();
            }
        }

        if main_msg.is_empty() {
            main_msg = stderr
                .lines()
                .next()
                .unwrap_or("Typst compilation error")
                .to_string();
        }

        let mut quick_fix = None;
        let lower_err = stderr.to_lowercase();
        if lower_err.contains("unknown variable: slide")
            || lower_err.contains("unknown variable: title-slide")
            || lower_err.contains("unknown variable: cols")
            || lower_err.contains("unknown variable: callout")
            || lower_err.contains("unknown variable: metric")
        {
            quick_fix = Some(QuickFix {
                label: "Import slide macros (#import \"slide.typ\": *)".to_string(),
                prepend_text: Some("#import \"slide.typ\": *\n".to_string()),
                replace_pair: None,
            });
        } else if source_text.contains("#col(") && !source_text.contains("#cols(") {
            quick_fix = Some(QuickFix {
                label: "Fix typo: Replace '#col(' with '#cols('".to_string(),
                prepend_text: None,
                replace_pair: Some(("#col(".to_string(), "#cols(".to_string())),
            });
        } else if source_text.contains("#pagebrak(") {
            quick_fix = Some(QuickFix {
                label: "Fix typo: Replace '#pagebrak()' with '#pagebreak()' delayed".to_string(),
                prepend_text: None,
                replace_pair: Some(("#pagebrak()".to_string(), "#pagebreak()".to_string())),
            });
        } else if lower_err.contains("unclosed delimiter") || lower_err.contains("expected ']'") {
            quick_fix = Some(QuickFix {
                label: "Append missing closing bracket ']'".to_string(),
                prepend_text: None,
                replace_pair: None,
            });
        }

        CompilerDiagnostic {
            message: main_msg,
            line: line_num,
            column: col_num,
            full_stderr: stderr.to_string(),
            quick_fix,
        }
    }

    /// Export document directly to PDF
    pub fn export_pdf(
        &self,
        doc: &EditorDocument,
        output_pdf: &Path,
    ) -> Result<()> {
        enum Cleanup {
            File(PathBuf),
            #[allow(dead_code)]
            Dir(tempfile::TempDir),
        }
        impl Drop for Cleanup {
            fn drop(&mut self) {
                if let Self::File(p) = self {
                    let _ = std::fs::remove_file(p);
                }
            }
        }

        let project_root = doc
            .assets_dir
            .as_deref()
            .or_else(|| doc.file_path.as_deref().and_then(|p| p.parent()));

        let is_user_slide_typ = if let Some(root) = project_root {
            let p = root.join("slide.typ");
            if p.is_file() {
                if let Ok(content) = std::fs::read_to_string(&p) {
                    !content.contains("#let slide-meta")
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        let (mut processed_source, prepended_import) =
            Self::prepare_typst_source_with_macros(&doc.source_text);

        if is_user_slide_typ {
            processed_source = processed_source
                .replace("#import \"slide.typ\"", "#import \".slide_macros.typ\"")
                .replace("#import 'slide.typ'", "#import \".slide_macros.typ\"");
        }

        let (temp_file, _cleanup) = if let Some(root) = project_root.filter(|p| p.is_dir()) {
            let tmp = root.join(".slide_export_tmp.typ");
            if prepended_import
                || processed_source.contains("slide.typ")
                || processed_source.contains(".slide_macros.typ")
                || processed_source.contains("theme.typ")
            {
                let dot_macro = root.join(".slide_macros.typ");
                if !dot_macro.exists() {
                    let _ = std::fs::write(&dot_macro, slide_theme::SLIDE_MACROS);
                }
                let slide_typ = root.join("slide.typ");
                if !slide_typ.exists() && !is_user_slide_typ {
                    let _ = std::fs::write(&slide_typ, slide_theme::SLIDE_MACROS);
                }
                let theme_typ = root.join("theme.typ");
                if !theme_typ.exists() {
                    let _ = std::fs::write(&theme_typ, slide_theme::DEFAULT_THEME);
                } else if let Ok(existing_theme) = std::fs::read_to_string(&theme_typ)
                    && existing_theme.contains("slide-theme(")
                    && !existing_theme.contains("header:")
                {
                    let _ = std::fs::write(&theme_typ, slide_theme::DEFAULT_THEME);
                }
            }
            std::fs::write(&tmp, &processed_source).map_err(|e| {
                SlideError::Compilation(format!(
                    "Failed to write temporary typst file in {}: {e}",
                    root.display()
                ))
            })?;
            (tmp.clone(), Cleanup::File(tmp))
        } else {
            let temp_dir = tempfile::tempdir()?;
            let slide_typ = temp_dir.path().join("slide.typ");
            let _ = std::fs::write(&slide_typ, slide_theme::SLIDE_MACROS);
            let theme_typ = temp_dir.path().join("theme.typ");
            let _ = std::fs::write(&theme_typ, slide_theme::DEFAULT_THEME);
            let tmp = temp_dir.path().join("document.typ");
            std::fs::write(&tmp, &processed_source)?;
            (tmp, Cleanup::Dir(temp_dir))
        };

        let abs_output = if output_pdf.is_absolute() {
            output_pdf.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(output_pdf)
        };

        if let Some(parent) = abs_output.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }

        self.compiler.compile_to_pdf(&temp_file, &abs_output)?;
        Ok(())
    }

    /// Export slide SVGs to an output directory or single file with optional page filtering
    pub fn export_svgs(
        deck: &SlideDeck,
        output_path: &Path,
        pages: Option<&[usize]>,
    ) -> Result<Vec<PathBuf>> {
        let slides_to_export: Vec<_> = if let Some(selected) = pages {
            deck.slides
                .iter()
                .filter(|s| selected.contains(&s.page_number))
                .collect()
        } else {
            deck.slides.iter().collect()
        };

        if slides_to_export.is_empty() {
            return Err(SlideError::Compilation(
                "No slides match the selected page range".to_string(),
            ));
        }

        let mut exported = Vec::new();

        let is_svg_file = output_path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"));

        if slides_to_export.len() == 1 && is_svg_file && !output_path.is_dir() {
            if let Some(parent) = output_path.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(output_path, &slides_to_export[0].svg_data)?;
            exported.push(output_path.to_path_buf());
            return Ok(exported);
        }

        std::fs::create_dir_all(output_path)?;
        for slide in slides_to_export {
            let out_file = output_path.join(format!("slide-{}.svg", slide.page_number));
            std::fs::write(&out_file, &slide.svg_data)?;
            exported.push(out_file);
        }

        Ok(exported)
    }

    /// Export a specific slide SVG to high-resolution PNG using tiny-skia and resvg
    pub fn export_slide_png(
        slide_svg: &str,
        output_png: &Path,
        scale: f32,
    ) -> Result<()> {
        let opt = usvg::Options::default();
        let tree = usvg::Tree::from_str(slide_svg, &opt)
            .map_err(|e| SlideError::Format(format!("Failed to parse SVG for PNG export: {e}")))?;

        let size = tree.size();
        let width = ((size.width() * scale).round() as u32).max(1);
        let height = ((size.height() * scale).round() as u32).max(1);

        let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or_else(|| {
            SlideError::Format("Failed to allocate pixmap for PNG export".to_string())
        })?;

        // Fill background white by default
        pixmap.fill(tiny_skia::Color::WHITE);

        let transform = tiny_skia::Transform::from_scale(scale, scale);
        resvg::render(&tree, transform, &mut pixmap.as_mut());

        if let Some(parent) = output_png.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }

        pixmap
            .save_png(output_png)
            .map_err(|e| SlideError::Format(format!("Failed to save PNG file: {e}")))?;

        Ok(())
    }

    /// Export slides in deck to high-resolution PNGs with optional page filtering
    pub fn export_all_pngs(
        deck: &SlideDeck,
        output_path: &Path,
        scale: f32,
        pages: Option<&[usize]>,
    ) -> Result<Vec<PathBuf>> {
        let slides_to_export: Vec<_> = if let Some(selected) = pages {
            deck.slides
                .iter()
                .filter(|s| selected.contains(&s.page_number))
                .collect()
        } else {
            deck.slides.iter().collect()
        };

        if slides_to_export.is_empty() {
            return Err(SlideError::Compilation(
                "No slides match the selected page range".to_string(),
            ));
        }

        let mut exported = Vec::new();

        let is_png_file = output_path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("png"));

        if slides_to_export.len() == 1 && is_png_file && !output_path.is_dir() {
            Self::export_slide_png(&slides_to_export[0].svg_data, output_path, scale)?;
            exported.push(output_path.to_path_buf());
            return Ok(exported);
        }

        std::fs::create_dir_all(output_path)?;
        for slide in slides_to_export {
            let out_file = output_path.join(format!("slide-{}.png", slide.page_number));
            Self::export_slide_png(&slide.svg_data, &out_file, scale)?;
            exported.push(out_file);
        }

        Ok(exported)
    }

    /// Package document into a standalone .slide file with LZMA2 compression, optionally bundling origin sources
    pub fn export_slide_package(
        &self,
        doc: &EditorDocument,
        include_source: bool,
        output_slide: &Path,
    ) -> Result<()> {
        let deck = if let Some(ref d) = doc.deck {
            d.clone()
        } else {
            let project_root = doc
                .assets_dir
                .as_deref()
                .or_else(|| doc.file_path.as_deref().and_then(|p| p.parent()));
            self.compile_source(&doc.source_text, project_root)
                .map_err(|d| SlideError::Compilation(d.full_stderr))?
        };

        if include_source {
            let temp_entrypoint = if let Some(ref p) = doc.file_path {
                p.clone()
            } else {
                let base = doc.assets_dir.clone().unwrap_or_else(std::env::temp_dir);
                let tmp = base.join(".slide_temp_source.typ");
                let _ = std::fs::write(&tmp, &doc.source_text);
                tmp
            };

            let res = slide_core::package::pack_deck_with_source_and_assets(
                &deck,
                doc.assets_dir.as_deref(),
                output_slide,
                true,
                Some(&temp_entrypoint),
            );

            if doc.file_path.is_none() {
                let _ = std::fs::remove_file(&temp_entrypoint);
            }
            res
        } else {
            pack_deck_with_assets(&deck, doc.assets_dir.as_deref(), output_slide)
        }
    }

    /// Package document into an encrypted standalone .slide file with AES-256-GCM / PIV protection.
    /// Returns `Ok(Some(generated_mnemonic))` if an auto-generated recovery mnemonic was created,
    /// or `Ok(None)` otherwise.
    #[allow(clippy::too_many_arguments)]
    pub fn export_encrypted_slide_package(
        &self,
        doc: &EditorDocument,
        include_source: bool,
        output_slide: &Path,
        auth_mode: slide_core::security::AuthMode,
        secret: Option<&str>,
        mnemonic_words: Option<usize>,
        piv_pin: Option<&str>,
    ) -> Result<Option<String>> {
        use slide_core::security::AuthMode;
        use slide_core::security::EncryptedSlideContainer;
        use slide_core::security::PivManager;
        use slide_core::security::PivSlot;

        let deck = if let Some(ref d) = doc.deck {
            d.clone()
        } else {
            let project_root = doc
                .assets_dir
                .as_deref()
                .or_else(|| doc.file_path.as_deref().and_then(|p| p.parent()));
            self.compile_source(&doc.source_text, project_root)
                .map_err(|d| SlideError::Compilation(d.full_stderr))?
        };

        let temp_assets_dir = if doc.assets_dir.is_none() && include_source {
            let tmp = tempfile::tempdir().ok();
            if let Some(ref t) = tmp {
                let _ = std::fs::write(t.path().join("source.typ"), &doc.source_text);
            }
            tmp
        } else {
            if include_source && let Some(ref adir) = doc.assets_dir {
                let src_path = adir.join("source.typ");
                let _ = std::fs::write(&src_path, &doc.source_text);
            }
            None
        };
        let effective_assets_dir = doc
            .assets_dir
            .as_deref()
            .or_else(|| temp_assets_dir.as_ref().map(|t| t.path()));

        match auth_mode {
            | AuthMode::Passphrase => {
                let pass = secret
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| {
                        SlideError::Crypto("Encryption passphrase cannot be empty".to_string())
                    })?;
                slide_core::security::validate_password_strength(pass)?;
                let (bytes, _) = EncryptedSlideContainer::pack_software_with_source(
                    &deck,
                    effective_assets_dir,
                    pass,
                    AuthMode::Passphrase,
                    None,
                    include_source,
                )?;
                std::fs::write(output_slide, bytes)?;
                Ok(None)
            },
            | AuthMode::Mnemonic => {
                let sec = secret.map(str::trim).unwrap_or("");
                if !sec.is_empty() {
                    slide_core::security::validate_bip39_mnemonic(sec)?;
                }
                let (bytes, gen_mnemonic) = EncryptedSlideContainer::pack_software_with_source(
                    &deck,
                    effective_assets_dir,
                    sec,
                    AuthMode::Mnemonic,
                    mnemonic_words,
                    include_source,
                )?;
                std::fs::write(output_slide, bytes)?;
                Ok(gen_mnemonic.map(|m| (*m).clone()))
            },
            | AuthMode::Piv => {
                let pin_opt = piv_pin
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        std::env::var("PIV_PIN")
                            .ok()
                            .filter(|s| !s.trim().is_empty())
                    });

                let recipient_slot = PivSlot::Slot9D;
                let signer_slot = PivSlot::Slot9C;
                let mut card = PivManager::acquire_card(pin_opt.as_deref())?;
                slide_core::security::prompt_pin_for_slot(
                    card.as_mut(),
                    signer_slot,
                    pin_opt.as_deref(),
                    true,
                )?;
                let recipient_pub = card.get_public_key(recipient_slot)?;
                let bytes = EncryptedSlideContainer::pack_with_source(
                    &deck,
                    effective_assets_dir,
                    &recipient_pub,
                    recipient_slot,
                    card.as_mut(),
                    signer_slot,
                    include_source,
                )?;
                std::fs::write(output_slide, bytes)?;
                Ok(None)
            },
        }
    }

    /// Parse page range string (e.g. "1", "1-3", "1, 3, 5-7") into sorted, deduplicated 1-based page numbers
    #[must_use]
    pub fn parse_page_range(
        spec: &str,
        total_pages: usize,
    ) -> Vec<usize> {
        let trimmed_spec = spec.trim();
        if trimmed_spec.is_empty() || trimmed_spec.eq_ignore_ascii_case("all") {
            return (1..=total_pages).collect();
        }
        let mut pages = std::collections::BTreeSet::new();
        for part in trimmed_spec.split(',') {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some((start_s, end_s)) = trimmed.split_once('-') {
                let start = start_s.trim().parse::<usize>().unwrap_or(1);
                let end = end_s.trim().parse::<usize>().unwrap_or(total_pages);
                let (min, max) = (start.min(end), start.max(end));
                for p in min..=max {
                    if p >= 1 && p <= total_pages {
                        pages.insert(p);
                    }
                }
            } else if let Ok(p) = trimmed.parse::<usize>()
                && (1..=total_pages).contains(&p)
            {
                pages.insert(p);
            }
        }
        pages.into_iter().collect()
    }

    /// Render any SVG string to an image handle with optional background color (None for transparent)
    pub fn render_svg_to_image(
        svg_content: &str,
        scale: f32,
        bg_color: Option<tiny_skia::Color>,
    ) -> Option<iced::widget::image::Handle> {
        let opt = usvg::Options::default();
        let tree = usvg::Tree::from_str(svg_content, &opt).ok()?;

        let size = tree.size();
        let s = scale.clamp(0.2, 5.0);
        let width = ((size.width() * s).round() as u32).max(1);
        let height = ((size.height() * s).round() as u32).max(1);

        let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
        if let Some(bg) = bg_color {
            pixmap.fill(bg);
        }

        let transform = tiny_skia::Transform::from_scale(s, s);
        resvg::render(&tree, transform, &mut pixmap.as_mut());

        Some(iced::widget::image::Handle::from_rgba(
            width,
            height,
            pixmap.take(),
        ))
    }

    /// Pre-render a slide's SVG markup into an iced image Handle for cached raster rendering.
    pub fn render_slide_to_image(
        slide_svg: &str,
        target_width: f32,
    ) -> Option<iced::widget::image::Handle> {
        let opt = usvg::Options::default();
        let tree = usvg::Tree::from_str(slide_svg, &opt).ok()?;

        let size = tree.size();
        let scale = if size.width() > 0.0 {
            (target_width / size.width()).clamp(0.5, 3.0)
        } else {
            1.0
        };

        Self::render_svg_to_image(slide_svg, scale, Some(tiny_skia::Color::WHITE))
    }

    /// Render a Typst math equation snippet into a transparent vector image handle
    pub fn render_equation_to_image(
        &self,
        formula: &str,
        is_dark: bool,
    ) -> Option<iced::widget::image::Handle> {
        let text_color = if is_dark {
            "e2e8f0"
        } else {
            "1e293b"
        };
        let clean = formula
            .trim()
            .trim_start_matches('$')
            .trim_end_matches('$')
            .trim();
        if clean.is_empty() {
            return None;
        }

        let snippet = format!(
            "#set page(width: auto, height: auto, margin: (x: 8pt, y: 4pt), fill: none)\n#set text(size: 16pt, fill: rgb(\"{text_color}\"))\n$ {clean} $\n"
        );
        let temp_dir = tempfile::tempdir().ok()?;
        let temp_file = temp_dir.path().join("formula.typ");
        std::fs::write(&temp_file, snippet).ok()?;

        let deck = self.compiler.compile_file(&temp_file).ok()?;
        let first_slide = deck.slides.first()?;
        Self::render_svg_to_image(&first_slide.svg_data, 1.5, None)
    }

    /// Render a Typst code block into a transparent vector image handle
    pub fn render_code_to_image(
        &self,
        code: &str,
        language: &str,
        is_dark: bool,
    ) -> Option<iced::widget::image::Handle> {
        let text_color = if is_dark {
            "e2e8f0"
        } else {
            "1e293b"
        };
        let bg_color = if is_dark {
            "0f172a"
        } else {
            "f8fafc"
        };
        let clean_code = code.trim_end();
        if clean_code.is_empty() {
            return None;
        }
        let lang = if language.is_empty() {
            "rust"
        } else {
            language
        };

        let snippet = format!(
            "#set page(width: auto, height: auto, margin: (x: 10pt, y: 8pt), fill: rgb(\"{bg_color}\"))\n\
             #set text(size: 13pt, fill: rgb(\"{text_color}\"))\n\
             ```{lang}\n{clean_code}\n```\n"
        );
        let temp_dir = tempfile::tempdir().ok()?;
        let temp_file = temp_dir.path().join("code.typ");
        std::fs::write(&temp_file, snippet).ok()?;

        let deck = self.compiler.compile_file(&temp_file).ok()?;
        let first_slide = deck.slides.first()?;
        Self::render_svg_to_image(&first_slide.svg_data, 1.5, None)
    }

    /// Export slide deck to a standalone single-file HTML presentation player
    pub fn export_standalone_html(
        deck: &SlideDeck,
        target: &Path,
        pages: Option<&[usize]>,
    ) -> Result<PathBuf> {
        slide_core::export::export_standalone_html(deck, pages, target)
    }
}
