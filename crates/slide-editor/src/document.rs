//! Document model supporting both general Typst documents (.typ) and Cargo Slide packages (.slide).

use slide_core::error::Result;
use slide_core::error::SlideError;
use slide_core::model::SlideDeck;
use slide_core::package::SlidePackageMetadata;
use slide_core::package::pack_deck_with_assets;
use slide_core::package::unpack_deck_and_assets;
use std::path::Path;
use std::path::PathBuf;

/// The format / kind of document currently loaded
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentFormat {
    /// Standard Typst source file (.typ)
    Typst,
    /// Standalone Cargo Slide LZMA2 compressed package (.slide)
    SlidePackage,
}

impl DocumentFormat {
    /// Human readable name
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            | Self::Typst => "Typst Document (.typ)",
            | Self::SlidePackage => "Slide Package (.slide)",
        }
    }

    /// File extension
    #[must_use]
    pub const fn extension(&self) -> &'static str {
        match self {
            | Self::Typst => "typ",
            | Self::SlidePackage => "slide",
        }
    }
}

/// Represents an editable presentation or document
#[derive(Debug, Clone)]
pub struct EditorDocument {
    /// Document title
    pub title: String,
    /// Absolute or relative path to the file on disk (if saved)
    pub file_path: Option<PathBuf>,
    /// Format of the document
    pub format: DocumentFormat,
    /// The full Typst source code of the document
    pub source_text: String,
    /// Whether the document has unsaved modifications
    pub is_dirty: bool,
    /// The compiled or unpacked slide deck
    pub deck: Option<SlideDeck>,
    /// Package metadata (if opened from .slide)
    pub package_metadata: Option<SlidePackageMetadata>,
    /// Directory containing extracted assets for .slide package or project root
    pub assets_dir: Option<PathBuf>,
    /// Default transition animation (fade, zoom, slide-left, slide-right, particles, cut)
    pub default_animation: String,
    /// Document preamble (imports, show/set rules)
    pub preamble: String,
    /// Individual slide chunks (isolated source code per slide)
    pub slide_chunks: Vec<String>,
}

impl Default for EditorDocument {
    fn default() -> Self {
        Self::new_presentation("Untitled Presentation")
    }
}

impl EditorDocument {
    /// Create a new blank presentation document
    #[must_use]
    pub fn new_presentation(title: &str) -> Self {
        let font_family = slide_core::font::detect_default_sans_font();
        let default_source = format!(
            "// Cargo Slide Presentation\n\
             #set page(width: 16cm, height: 9cm, margin: (x: 1.2cm, y: 1cm))\n\
             #import \"slide.typ\": *\n\
             #set text(font: \"{font_family}\", size: 14pt, fill: rgb(\"1e293b\"))\n\n\
             // --- Slide 1: Title ---\n\
             #align(center + horizon)[\n\
               = {title}\n\n\
               #v(0.3cm)\n\
               == A modern code-driven presentation\n\n\
               #v(0.5cm)\n\
               Created with Slide Editor\n\
             ]\n\n\
             #pagebreak()\n\n\
             // --- Slide 2: Features ---\n\
             = Key Capabilities\n\n\
             - *Typora-style Live Preview*: Seamless in-place editing\n\
             - *Pure Native GUI*: Fast Rust desktop app\n\
             - *Multi-Format Export*: PDF, PNG, SVG, and `.slide`\n\n\
             #v(0.2cm)\n\
             $ E = m c^2 quad \"and\" quad sum_(i=1)^n i = (n(n+1))/2 $\n\n\
             #pagebreak()\n\n\
             // --- Slide 3: Code & Data ---\n\
             = Code Integration\n\n\
             ```rust\n\
             fn main() {{\n\
                 println!(\"Hello, Cargo Slide WYSIWYG!\");\n\
             }}\n\
             ```\n\n\
             - Edit directly in *Live Preview* or *Source Mode*\n\
             - Instant visual feedback on every keystroke\n"
        );

        let mut doc = Self {
            title: title.to_string(),
            file_path: None,
            format: DocumentFormat::Typst,
            source_text: default_source,
            is_dirty: false,
            deck: None,
            package_metadata: None,
            assets_dir: None,
            default_animation: "fade".to_string(),
            preamble: String::new(),
            slide_chunks: Vec::new(),
        };
        doc.sync_chunks_from_source();
        doc
    }

    /// Create a new generic Typst document
    #[must_use]
    pub fn new_typst_document(title: &str) -> Self {
        let default_source = format!(
            "= {title}\n\n\
             == Introduction\n\n\
             This is a Typst document edited in *Slide Editor*, featuring Typora-style live preview and in-place editing.\n\n\
             == Mathematics & Equations\n\n\
             Typst provides clean native math formatting:\n\n\
             $ integral_0^infinity e^(-x^2) dif x = sqrt(pi) / 2 $\n\n\
             == Code Listings\n\n\
             ```rust\n\
             // Embedded high-performance code snippet\n\
             pub fn greet(name: &str) -> String {{\n\
                 format!(\"Hello, {{name}}!\")\n\
             }}\n\
             ```\n\n\
             == Structured Lists\n\n\
             + Fast compilation with native Typst bridge\n\
             + Clean Light and Dark modes\n\
             + Direct export to PDF and vector SVGs\n"
        );

        let mut doc = Self {
            title: title.to_string(),
            file_path: None,
            format: DocumentFormat::Typst,
            source_text: default_source,
            is_dirty: false,
            deck: None,
            package_metadata: None,
            assets_dir: None,
            default_animation: "fade".to_string(),
            preamble: String::new(),
            slide_chunks: Vec::new(),
        };
        doc.sync_chunks_from_source();
        doc
    }

    /// Open a document from a path (.typ or .slide)
    pub fn open(path: &Path) -> Result<Self> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "slide" {
            // Unpack .slide archive
            let (deck, cache_dir) = unpack_deck_and_assets(path)?;
            let title = deck.title.clone();
            let default_anim = deck.default_animation.clone();

            let meta = slide_core::package::read_package_metadata(path).ok();

            // Check if unpacked archive has entrypoint or source.typ, otherwise reconstruct
            let entrypoint_file = meta
                .as_ref()
                .and_then(|m| m.entrypoint.as_ref())
                .map(|ep| cache_dir.join(ep))
                .filter(|p| p.is_file());

            let source_file = entrypoint_file.unwrap_or_else(|| cache_dir.join("source.typ"));
            let reconstructed_source = if source_file.is_file() {
                std::fs::read_to_string(&source_file)
                    .unwrap_or_else(|_| Self::deck_to_typst_source(&deck))
            } else {
                Self::deck_to_typst_source(&deck)
            };

            let mut doc = Self {
                title,
                file_path: Some(path.to_path_buf()),
                format: DocumentFormat::SlidePackage,
                source_text: reconstructed_source,
                is_dirty: false,
                deck: Some(deck),
                package_metadata: meta,
                assets_dir: Some(cache_dir),
                default_animation: default_anim,
                preamble: String::new(),
                slide_chunks: Vec::new(),
            };
            doc.sync_chunks_from_source();
            Ok(doc)
        } else {
            // Read .typ source file
            let content = std::fs::read_to_string(path)?;
            let title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled")
                .to_string();

            let parent_dir = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map(Path::to_path_buf);

            let mut doc = Self {
                title,
                file_path: Some(path.to_path_buf()),
                format: DocumentFormat::Typst,
                source_text: content,
                is_dirty: false,
                deck: None,
                package_metadata: None,
                assets_dir: parent_dir,
                default_animation: "fade".to_string(),
                preamble: String::new(),
                slide_chunks: Vec::new(),
            };
            doc.sync_chunks_from_source();
            Ok(doc)
        }
    }

    /// Save document to its existing path or specified path
    pub fn save(
        &mut self,
        target_path: Option<&Path>,
    ) -> Result<PathBuf> {
        let path = target_path
            .map(Path::to_path_buf)
            .or_else(|| self.file_path.clone())
            .ok_or_else(|| SlideError::Format("No file path specified for saving".to_string()))?;

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "slide" {
            // If we have a deck, pack it directly with assets
            if let Some(ref deck) = self.deck {
                if let Some(ref adir) = self.assets_dir {
                    let _ = std::fs::write(adir.join("source.typ"), &self.source_text);
                }
                pack_deck_with_assets(deck, self.assets_dir.as_deref(), &path)?;
            } else {
                // Otherwise compile temp typ file and pack
                let temp_dir = tempfile::tempdir()?;
                let _ =
                    std::fs::write(temp_dir.path().join("slide.typ"), slide_theme::SLIDE_MACROS);
                let (proc_src, _) =
                    crate::compiler_bridge::CompilerBridge::prepare_typst_source_with_macros(
                        &self.source_text,
                    );
                let temp_typ = temp_dir.path().join("slides.typ");
                std::fs::write(&temp_typ, &proc_src)?;

                let compiler = slide_core::compiler::SlideCompiler::new()?;
                let compiled_deck = compiler.compile_file(&temp_typ)?;
                if let Some(ref adir) = self.assets_dir {
                    let _ = std::fs::write(adir.join("source.typ"), &self.source_text);
                }
                pack_deck_with_assets(&compiled_deck, self.assets_dir.as_deref(), &path)?;
                self.deck = Some(compiled_deck);
            }
            self.format = DocumentFormat::SlidePackage;
        } else {
            // Save as .typ
            if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, &self.source_text)?;
            self.format = DocumentFormat::Typst;
        }

        self.file_path = Some(path.clone());
        self.is_dirty = false;
        Ok(path)
    }

    /// Helper to convert a SlideDeck into editable Typst source
    fn deck_to_typst_source(deck: &SlideDeck) -> String {
        let font_family = slide_core::font::detect_default_sans_font();
        let mut source = String::new();
        source.push_str("// Imported from .slide package\n");
        source.push_str("#import \"slide.typ\": *\n");
        source.push_str("#set page(width: 16cm, height: 9cm, margin: 1cm)\n");
        source.push_str(&format!(
            "#set text(font: \"{font_family}\", size: 18pt)\n\n"
        ));

        if deck.slides.is_empty() {
            source.push_str(&format!("= {}\n\nBlank presentation.", deck.title));
            return source;
        }

        for (idx, slide) in deck.slides.iter().enumerate() {
            if idx > 0 {
                source.push_str("\n#pagebreak()\n\n");
            }
            source.push_str(&format!("// --- Slide {} ---\n", slide.page_number));

            // Extract any text hints or markers from SVG if possible
            let mut extracted_title = None;
            for line in slide.svg_data.lines() {
                if let Some(start) = line.find("<text")
                    && let Some(content_start) = line[start..].find('>')
                {
                    let text_part = &line[start + content_start + 1..];
                    if let Some(content_end) = text_part.find("</text>") {
                        let raw_text = text_part[..content_end].trim();
                        if !raw_text.is_empty() && raw_text.len() < 80 {
                            extracted_title = Some(raw_text.to_string());
                            break;
                        }
                    }
                }
            }

            if let Some(stitle) = extracted_title {
                source.push_str(&format!("= {stitle}\n\n"));
            } else if idx == 0 {
                source.push_str(&format!("= {}\n\n", deck.title));
            } else {
                source.push_str(&format!("= Slide {}\n\n", slide.page_number));
            }

            source.push_str(&format!(
                "// Slide {} content (from package)\n",
                slide.page_number
            ));
        }

        source
    }

    /// Sync preamble and slide chunks from source_text
    pub fn sync_chunks_from_source(&mut self) {
        let engine =
            crate::model::ast_engine::TypstDocumentEngine::from_source(self.source_text.clone());
        if !engine.slides.is_empty() {
            let first_start = engine.slides[0].range.start;
            self.preamble = self.source_text[..first_start].to_string();
            self.slide_chunks = engine
                .slides
                .iter()
                .map(|s| {
                    self.source_text
                        .get(s.range.clone())
                        .unwrap_or("")
                        .to_string()
                })
                .collect();
        } else {
            self.preamble.clear();
            self.slide_chunks = vec![self.source_text.clone()];
        }
    }

    /// Rebuild full source_text from preamble and slide_chunks
    pub fn rebuild_source_from_chunks(&mut self) {
        let is_macro = self.source_text.contains("#slide(")
            || self.source_text.contains("#title-slide(")
            || self.preamble.contains("#slide");
        if is_macro {
            if self.preamble.trim().is_empty() {
                self.source_text = self.slide_chunks.join("\n\n");
            } else {
                self.source_text = format!(
                    "{}\n\n{}",
                    self.preamble.trim(),
                    self.slide_chunks.join("\n\n")
                );
            }
        } else if self.preamble.trim().is_empty() {
            self.source_text = self.slide_chunks.join("\n\n#pagebreak()\n\n");
        } else {
            self.source_text = format!(
                "{}\n\n{}",
                self.preamble.trim(),
                self.slide_chunks.join("\n\n#pagebreak()\n\n")
            );
        }
    }

    /// Get slide sections split by slide macro or pagebreak
    #[must_use]
    pub fn get_slide_chunks(&self) -> Vec<String> {
        if !self.slide_chunks.is_empty() {
            self.slide_chunks.clone()
        } else {
            vec![self.source_text.clone()]
        }
    }

    /// Update a specific slide's text and reassemble full source_text
    pub fn update_slide_chunk(
        &mut self,
        slide_idx: usize,
        new_chunk: &str,
    ) {
        if slide_idx < self.slide_chunks.len() {
            self.slide_chunks[slide_idx] = new_chunk.to_string();
            self.rebuild_source_from_chunks();
            self.is_dirty = true;
            return;
        }

        let engine =
            crate::model::ast_engine::TypstDocumentEngine::from_source(self.source_text.clone());
        if let Some(slide) = engine.slides.get(slide_idx) {
            let range = slide.range.clone();
            if range.start <= self.source_text.len() && range.end <= self.source_text.len() {
                self.source_text.replace_range(range, new_chunk);
                self.sync_chunks_from_source();
                self.is_dirty = true;
                return;
            }
        }

        if !self.source_text.is_empty() {
            self.source_text.push_str("\n\n");
            self.source_text.push_str(new_chunk);
        } else {
            self.source_text = new_chunk.to_string();
        }
        self.sync_chunks_from_source();
        self.is_dirty = true;
    }

    /// Delete a specific slide
    pub fn delete_slide(
        &mut self,
        slide_idx: usize,
    ) {
        if slide_idx < self.slide_chunks.len() && self.slide_chunks.len() > 1 {
            self.slide_chunks.remove(slide_idx);
            self.rebuild_source_from_chunks();
            self.is_dirty = true;
        }
    }

    /// Add a new blank slide at the end
    pub fn add_new_slide(&mut self) {
        self.insert_slide_at(self.slide_chunks.len());
    }

    /// Insert a new blank slide at a specific position
    pub fn insert_slide_at(
        &mut self,
        target_idx: usize,
    ) {
        let is_macro = self.source_text.contains("#slide(")
            || self.source_text.contains("#title-slide(")
            || self.preamble.contains("#slide");
        let idx = target_idx.min(self.slide_chunks.len());
        let new_chunk = if is_macro {
            format!(
                "#slide(title: \"Slide {}\")[\n  = New Topic\n\n  - Enter bullet point here\n]",
                idx + 1
            )
        } else {
            format!("= Slide {}\n\n- Enter bullet point here\n", idx + 1)
        };
        self.slide_chunks.insert(idx, new_chunk);
        self.rebuild_source_from_chunks();
        self.is_dirty = true;
    }

    /// Duplicate a slide at index
    pub fn duplicate_slide(
        &mut self,
        slide_idx: usize,
    ) {
        if slide_idx < self.slide_chunks.len() {
            let chunk = self.slide_chunks[slide_idx].clone();
            self.slide_chunks.insert(slide_idx + 1, chunk);
            self.rebuild_source_from_chunks();
            self.is_dirty = true;
        }
    }

    /// Move a slide from one index to another
    pub fn move_slide(
        &mut self,
        from_idx: usize,
        to_idx: usize,
    ) {
        if from_idx == to_idx
            || from_idx >= self.slide_chunks.len()
            || to_idx >= self.slide_chunks.len()
        {
            return;
        }
        let chunk = self.slide_chunks.remove(from_idx);
        self.slide_chunks.insert(to_idx, chunk);
        self.rebuild_source_from_chunks();
        self.is_dirty = true;
    }

    /// Total number of slides (from compiled deck or parsed chunks)
    #[must_use]
    pub fn total_slides(&self) -> usize {
        if !self.slide_chunks.is_empty() {
            return self.slide_chunks.len();
        }
        if let Some(ref d) = self.deck
            && !d.slides.is_empty()
        {
            return d.slides.len();
        }
        1
    }

    /// Check whether the document contains editable Typst source code
    #[must_use]
    pub fn has_editable_source(&self) -> bool {
        if self.format == DocumentFormat::Typst {
            return true;
        }
        if let Some(ref meta) = self.package_metadata
            && meta.is_editable
        {
            return true;
        }
        !self.source_text.trim().is_empty()
    }
}
