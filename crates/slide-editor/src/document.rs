//! Document model supporting both general Typst documents (.typ) and Cargo Slide packages (.slide).

use slide_core::error::Result;
use slide_core::error::SlideError;
use slide_core::model::SlideDeck;
use slide_core::package::SlidePackageMetadata;
use slide_core::package::pack_deck_with_source_and_assets;
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
    /// Whether this document was opened from or will be saved to an encrypted package
    pub is_encrypted: bool,
    /// Encryption authentication mode (Passphrase, Mnemonic, or Piv)
    pub encryption_auth_mode: Option<slide_core::security::AuthMode>,
    /// Decryption secret cached in memory with zeroization for saving modifications
    pub encryption_secret: Option<slide_core::security::Zeroizing<String>>,
    /// YubiKey PIV recipient slot (if PIV encrypted)
    pub encryption_recipient_slot: Option<slide_core::security::PivSlot>,
    /// YubiKey PIV signer slot (if PIV encrypted)
    pub encryption_signer_slot: Option<slide_core::security::PivSlot>,
    /// Number of mnemonic words (12 or 24)
    pub encryption_mnemonic_words: Option<usize>,
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
             - Live visual feedback as you type\n"
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
            is_encrypted: false,
            encryption_auth_mode: None,
            encryption_secret: None,
            encryption_recipient_slot: None,
            encryption_signer_slot: None,
            encryption_mnemonic_words: None,
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
            is_encrypted: false,
            encryption_auth_mode: None,
            encryption_secret: None,
            encryption_recipient_slot: None,
            encryption_signer_slot: None,
            encryption_mnemonic_words: None,
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
            if slide_core::package::is_encrypted_package_file(path) {
                use slide_core::security::AuthMode;
                use slide_core::security::EncryptedSlideContainer;
                use slide_core::security::PivManager;
                use slide_core::security::Zeroizing;
                use slide_core::security::prompt_secret_gui;
                use slide_core::security::read_masked_secret;
                use std::io::IsTerminal as _;

                let bytes = std::fs::read(path)?;
                let container = EncryptedSlideContainer::parse(&bytes)?;
                let auth_mode = container.header.auth_mode;
                let title = container.header.title.clone();
                let recipient_slot = container.header.recipient_slot;
                let signer_slot = container.header.signer_slot;

                let mut cached_secret = None;
                let max_attempts = 3;

                let (deck, asset_store) = match auth_mode {
                    | AuthMode::Passphrase | AuthMode::Mnemonic => {
                        let mut last_err = None;
                        let mut decrypted = None;

                        let mut initial_secret = std::env::var("SLIDE_PASSPHRASE")
                            .or_else(|_| std::env::var("SLIDE_SECRET"))
                            .ok()
                            .map(Zeroizing::new);

                        for attempt in 0..max_attempts {
                            let secret: Zeroizing<String> = if let Some(s) = initial_secret.take() {
                                s
                            } else {
                                let attempts_left = max_attempts - attempt;
                                let is_mnemonic = auth_mode == AuthMode::Mnemonic;
                                let dialog_title = if attempt == 0 {
                                    "Cargo Slide Editor - Decrypt Presentation"
                                } else {
                                    "Authentication Failed - Please Re-enter"
                                };
                                let gui_subtitle = if attempt == 0 {
                                    if is_mnemonic {
                                        "Enter 12/24-word BIP-39 recovery mnemonic:".to_string()
                                    } else {
                                        "Enter passphrase to open encrypted presentation:"
                                            .to_string()
                                    }
                                } else if is_mnemonic {
                                    format!(
                                        "Incorrect mnemonic ({attempts_left} attempts remaining). Please re-enter:"
                                    )
                                } else {
                                    format!(
                                        "Incorrect passphrase ({attempts_left} attempts remaining). Please re-enter:"
                                    )
                                };

                                if let Some(gui_sec) =
                                    prompt_secret_gui(dialog_title, &gui_subtitle)
                                {
                                    gui_sec
                                } else if std::io::stdin().is_terminal() {
                                    let term_prompt = if attempt == 0 {
                                        if is_mnemonic {
                                            "Enter 12/24-word recovery mnemonic: ".to_string()
                                        } else {
                                            "Enter passphrase: ".to_string()
                                        }
                                    } else if is_mnemonic {
                                        format!(
                                            "\x1b[33mIncorrect mnemonic ({attempts_left} attempts remaining). Please re-enter:\x1b[0m "
                                        )
                                    } else {
                                        format!(
                                            "\x1b[33mIncorrect passphrase ({attempts_left} attempts remaining). Please re-enter:\x1b[0m "
                                        )
                                    };
                                    read_masked_secret(&term_prompt)?
                                } else {
                                    break;
                                }
                            };

                            match container.decrypt_with_secret(&secret) {
                                | Ok(res) => {
                                    cached_secret = Some(secret);
                                    decrypted = Some(res);
                                    break;
                                },
                                | Err(e) => {
                                    last_err = Some(e);
                                    if attempt + 1 < max_attempts {
                                        eprintln!(
                                            "[WARNING] Authentication failed. Please try again ({} attempts remaining).",
                                            max_attempts - 1 - attempt
                                        );
                                    }
                                },
                            }
                        }

                        decrypted.ok_or_else(|| {
                            last_err.unwrap_or_else(|| {
                                SlideError::Crypto(
                                    "Decryption canceled or retries exhausted".to_string(),
                                )
                            })
                        })?
                    },
                    | AuthMode::Piv => {
                        let env_pin = std::env::var("PIV_PIN").ok();
                        let mut card = PivManager::acquire_card(env_pin.as_deref())?;

                        if !card.is_authenticated() && env_pin.is_none() {
                            for attempt in 0..max_attempts {
                                let attempts_left = match card.check_pin_status() {
                                    | Ok(slide_core::security::piv::PinStatus::NotVerified {
                                        tries_remaining,
                                    }) => (tries_remaining as usize).min(max_attempts - attempt),
                                    | _ => max_attempts - attempt,
                                };
                                let dialog_title = if attempt == 0 {
                                    "Cargo Slide Editor - YubiKey PIV Authentication"
                                } else {
                                    "PIN Verification Failed - Please Re-enter"
                                };
                                let subtitle = if attempt == 0 {
                                    format!(
                                        "Enter 6-8 digit User PIN to open encrypted presentation ({attempts_left} attempts remaining):"
                                    )
                                } else {
                                    format!(
                                        "Incorrect PIN ({attempts_left} attempts remaining). Please re-enter:"
                                    )
                                };

                                if let Some(gui_pin) = prompt_secret_gui(dialog_title, &subtitle) {
                                    match card.verify_pin(&gui_pin) {
                                        | Ok(()) => break,
                                        | Err(e) => {
                                            if attempt + 1 == max_attempts {
                                                return Err(e);
                                            }
                                        },
                                    }
                                } else if std::io::stdin().is_terminal() {
                                    let term_prompt = format!(
                                        "Enter YubiKey PIV PIN ({attempts_left} attempts remaining): "
                                    );
                                    let pin_sec = read_masked_secret(&term_prompt)?;
                                    match card.verify_pin(&pin_sec) {
                                        | Ok(()) => break,
                                        | Err(e) => {
                                            if attempt + 1 == max_attempts {
                                                return Err(e);
                                            }
                                        },
                                    }
                                } else {
                                    break;
                                }
                            }
                        }

                        container.decrypt_in_memory(card.as_mut(), None)?
                    },
                };

                let default_anim = deck.default_animation.clone();
                let cache_dir = slide_core::package::get_package_cache_dir(path);
                let _ = std::fs::remove_dir_all(&cache_dir);
                let _ = std::fs::create_dir_all(&cache_dir);

                // Unpack assets to disk for editing preview
                for key in asset_store.keys() {
                    if let Some(val) = asset_store.get(&key)
                        && let Some(dest) =
                            slide_core::package::sanitize_archive_path(&cache_dir, &key)
                    {
                        if let Some(parent) = dest.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let _ = std::fs::write(&dest, val);
                    }
                }

                let meta: Option<slide_core::package::SlidePackageMetadata> =
                    std::fs::read(cache_dir.join("manifest.json"))
                        .ok()
                        .and_then(|bytes| serde_json::from_slice(&bytes).ok());

                let mut is_editable = meta.as_ref().is_some_and(|m| m.is_editable);

                let entrypoint_file = meta
                    .as_ref()
                    .and_then(|m| m.entrypoint.as_ref())
                    .map(|ep| cache_dir.join(ep))
                    .filter(|p| p.is_file());

                let mut source_file = entrypoint_file
                    .or_else(|| {
                        let s = cache_dir.join("slides.typ");
                        if s.is_file() { Some(s) } else { None }
                    })
                    .or_else(|| {
                        let s = cache_dir.join("source.typ");
                        if s.is_file() { Some(s) } else { None }
                    })
                    .or_else(|| {
                        let s = cache_dir.join("presentation.typ");
                        if s.is_file() { Some(s) } else { None }
                    })
                    .or_else(|| {
                        if let Ok(entries) = std::fs::read_dir(&cache_dir) {
                            for entry in entries.flatten() {
                                let p = entry.path();
                                if p.is_file()
                                    && p.extension().and_then(|e| e.to_str()) == Some("typ")
                                {
                                    let fname =
                                        p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                                    if fname != "slide.typ" && fname != "theme.typ" {
                                        return Some(p);
                                    }
                                }
                            }
                        }
                        None
                    });

                let parent_dir = path
                    .parent()
                    .map(|p| {
                        if p.as_os_str().is_empty() {
                            Path::new(".")
                        } else {
                            p
                        }
                    })
                    .unwrap_or(Path::new("."));
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("slides");
                if source_file.is_none() {
                    source_file = [
                        parent_dir.join(format!("{stem}.typ")),
                        parent_dir.join("slides.typ"),
                        parent_dir.join("presentation.typ"),
                    ]
                    .into_iter()
                    .find(|p| p.is_file());
                }

                if source_file.is_some() {
                    is_editable = true;
                }

                let reconstructed_source = if let Some(ref sf) = source_file {
                    std::fs::read_to_string(sf)
                        .unwrap_or_else(|_| Self::deck_to_typst_source(&deck))
                } else {
                    Self::deck_to_typst_source(&deck)
                };

                let mut final_meta = meta;
                if let Some(ref mut m) = final_meta {
                    m.is_editable = is_editable;
                }

                let mut doc = Self {
                    title: if title.is_empty() {
                        deck.title.clone()
                    } else {
                        title
                    },
                    file_path: Some(path.to_path_buf()),
                    format: DocumentFormat::SlidePackage,
                    source_text: reconstructed_source,
                    is_dirty: false,
                    deck: Some(deck),
                    package_metadata: final_meta,
                    assets_dir: Some(cache_dir),
                    default_animation: default_anim,
                    preamble: String::new(),
                    slide_chunks: Vec::new(),
                    is_encrypted: true,
                    encryption_auth_mode: Some(auth_mode),
                    encryption_secret: cached_secret,
                    encryption_recipient_slot: Some(recipient_slot),
                    encryption_signer_slot: Some(signer_slot),
                    encryption_mnemonic_words: None,
                };
                doc.sync_chunks_from_source();
                return Ok(doc);
            }

            // Unpack .slide archive
            let (deck, cache_dir) = unpack_deck_and_assets(path)?;
            let title = deck.title.clone();
            let default_anim = deck.default_animation.clone();

            let meta = slide_core::package::read_package_metadata(path).ok();
            let mut is_editable = meta.as_ref().is_some_and(|m| m.is_editable);

            let entrypoint_file = meta
                .as_ref()
                .and_then(|m| m.entrypoint.as_ref())
                .map(|ep| cache_dir.join(ep))
                .filter(|p| p.is_file());

            let mut source_file = entrypoint_file
                .or_else(|| {
                    let s = cache_dir.join("slides.typ");
                    if s.is_file() { Some(s) } else { None }
                })
                .or_else(|| {
                    let s = cache_dir.join("source.typ");
                    if s.is_file() { Some(s) } else { None }
                })
                .or_else(|| {
                    let s = cache_dir.join("presentation.typ");
                    if s.is_file() { Some(s) } else { None }
                });

            let parent_dir = path
                .parent()
                .map(|p| {
                    if p.as_os_str().is_empty() {
                        Path::new(".")
                    } else {
                        p
                    }
                })
                .unwrap_or(Path::new("."));
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("slides");
            if source_file.is_none() {
                source_file = [
                    parent_dir.join(format!("{stem}.typ")),
                    parent_dir.join("slides.typ"),
                    parent_dir.join("presentation.typ"),
                ]
                .into_iter()
                .find(|p| p.is_file());
            }

            if source_file.is_some() {
                is_editable = true;
            }

            let reconstructed_source = if let Some(ref sf) = source_file {
                std::fs::read_to_string(sf).unwrap_or_else(|_| Self::deck_to_typst_source(&deck))
            } else {
                Self::deck_to_typst_source(&deck)
            };

            let mut final_meta = meta;
            if let Some(ref mut m) = final_meta {
                m.is_editable = is_editable;
            }

            let mut doc = Self {
                title,
                file_path: Some(path.to_path_buf()),
                format: DocumentFormat::SlidePackage,
                source_text: reconstructed_source,
                is_dirty: false,
                deck: Some(deck),
                package_metadata: final_meta,
                assets_dir: Some(cache_dir),
                default_animation: default_anim,
                preamble: String::new(),
                slide_chunks: Vec::new(),
                is_encrypted: false,
                encryption_auth_mode: None,
                encryption_secret: None,
                encryption_recipient_slot: None,
                encryption_signer_slot: None,
                encryption_mnemonic_words: None,
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
                is_encrypted: false,
                encryption_auth_mode: None,
                encryption_secret: None,
                encryption_recipient_slot: None,
                encryption_signer_slot: None,
                encryption_mnemonic_words: None,
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
            // Ensure assets_dir exists
            let cache_dir = self.assets_dir.clone().unwrap_or_else(|| {
                let d = slide_core::package::get_package_cache_dir(&path);
                let _ = std::fs::create_dir_all(&d);
                d
            });
            self.assets_dir = Some(cache_dir.clone());

            // Write current source_text into cache_dir as slides.typ and source.typ
            let _ = std::fs::write(cache_dir.join("slides.typ"), &self.source_text);
            let _ = std::fs::write(cache_dir.join("source.typ"), &self.source_text);
            if let Some(ref ep) = self
                .package_metadata
                .as_ref()
                .and_then(|m| m.entrypoint.as_deref())
            {
                let _ = std::fs::write(cache_dir.join(ep), &self.source_text);
            }

            // Update or create manifest.json with is_editable = true
            let mut meta = self.package_metadata.clone().unwrap_or_else(|| {
                slide_core::package::SlidePackageMetadata {
                    format_version: 1,
                    title: self.title.clone(),
                    total_slides: self.total_slides(),
                    default_animation: self.default_animation.clone(),
                    compression: "lzma2-max".to_string(),
                    generator: format!("cargo-slide {}", env!("CARGO_PKG_VERSION")),
                    is_editable: true,
                    entrypoint: Some("slides.typ".to_string()),
                    created_at: None,
                    author: None,
                    aspect_ratio: Some("16:9".to_string()),
                    has_notes: false,
                    tags: Vec::new(),
                    checksum: None,
                    uncompressed_size: None,
                }
            });
            meta.is_editable = true;
            if meta.entrypoint.is_none() {
                meta.entrypoint = Some("slides.typ".to_string());
            }
            let _ = std::fs::write(
                cache_dir.join("manifest.json"),
                serde_json::to_vec_pretty(&meta).unwrap_or_default(),
            );
            self.package_metadata = Some(meta);

            // Ensure slide.typ and theme.typ exist in cache_dir
            let cache_slide_typ = cache_dir.join("slide.typ");
            if !cache_slide_typ.is_file() {
                let _ = std::fs::write(&cache_slide_typ, slide_theme::SLIDE_MACROS);
            }
            let cache_theme_typ = cache_dir.join("theme.typ");
            if !cache_theme_typ.is_file() {
                let _ = std::fs::write(&cache_theme_typ, slide_theme::DEFAULT_THEME);
            }

            // Recompile deck from source_text if possible, falling back to existing deck
            let deck = {
                let (proc_src, _) =
                    crate::compiler_bridge::CompilerBridge::prepare_typst_source_with_macros(
                        &self.source_text,
                    );
                let temp_dir = tempfile::tempdir().ok();
                let recompiled = temp_dir.as_ref().and_then(|td| {
                    let _ = std::fs::write(td.path().join("slide.typ"), slide_theme::SLIDE_MACROS);
                    let _ = std::fs::write(td.path().join("theme.typ"), slide_theme::DEFAULT_THEME);
                    if cache_theme_typ.is_file() {
                        let _ = std::fs::copy(&cache_theme_typ, td.path().join("theme.typ"));
                    }
                    let temp_typ = td.path().join("slides.typ");
                    std::fs::write(&temp_typ, &proc_src).ok()?;
                    let compiler = slide_core::compiler::SlideCompiler::new().ok()?;
                    compiler.compile_file(&temp_typ).ok()
                });
                if let Some(new_deck) = recompiled {
                    self.deck = Some(new_deck.clone());
                    new_deck
                } else if let Some(ref existing) = self.deck {
                    existing.clone()
                } else {
                    return Err(SlideError::Compilation(
                        "Failed to compile presentation deck for saving".to_string(),
                    ));
                }
            };

            if self.is_encrypted {
                use slide_core::security::AuthMode;
                use slide_core::security::EncryptedSlideContainer;
                use slide_core::security::PivManager;
                use slide_core::security::PivSlot;

                match self.encryption_auth_mode {
                    | Some(AuthMode::Passphrase) | Some(AuthMode::Mnemonic) => {
                        let auth_mode = self.encryption_auth_mode.unwrap();
                        let secret = self.encryption_secret.as_ref().ok_or_else(|| {
                            SlideError::Crypto(
                                "Cannot save encrypted package: encryption secret is missing from memory".to_string(),
                            )
                        })?;
                        let (bytes, _) = EncryptedSlideContainer::pack_software(
                            &deck,
                            Some(&cache_dir),
                            secret,
                            auth_mode,
                            self.encryption_mnemonic_words,
                        )?;
                        std::fs::write(&path, bytes)?;
                    },
                    | Some(AuthMode::Piv) => {
                        let mut card = PivManager::acquire_card(None)?;
                        let recipient_slot =
                            self.encryption_recipient_slot.unwrap_or(PivSlot::Slot9D);
                        let signer_slot = self.encryption_signer_slot.unwrap_or(PivSlot::Slot9C);
                        slide_core::security::prompt_pin_for_slot(
                            card.as_mut(),
                            signer_slot,
                            None,
                            true,
                        )?;
                        let recipient_pub = card.get_public_key(recipient_slot)?;
                        let bytes = EncryptedSlideContainer::pack_with_source(
                            &deck,
                            Some(&cache_dir),
                            &recipient_pub,
                            recipient_slot,
                            card.as_mut(),
                            signer_slot,
                            true,
                        )?;
                        std::fs::write(&path, bytes)?;
                    },
                    | None => {
                        let source_entry = cache_dir.join("slides.typ");
                        pack_deck_with_source_and_assets(
                            &deck,
                            Some(&cache_dir),
                            &path,
                            true,
                            Some(&source_entry),
                        )?;
                    },
                }
                self.format = DocumentFormat::SlidePackage;
            } else {
                let source_entry = cache_dir.join("slides.typ");
                pack_deck_with_source_and_assets(
                    &deck,
                    Some(&cache_dir),
                    &path,
                    true,
                    Some(&source_entry),
                )?;
                self.format = DocumentFormat::SlidePackage;
            }

            // Sync source to sibling .typ file on disk
            let parent_dir = path
                .parent()
                .map(|p| {
                    if p.as_os_str().is_empty() {
                        Path::new(".")
                    } else {
                        p
                    }
                })
                .unwrap_or(Path::new("."));

            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("slides");
            let entrypoint_name = self
                .package_metadata
                .as_ref()
                .and_then(|m| m.entrypoint.as_deref())
                .unwrap_or("slides.typ");

            let mut target_typ = None;
            for candidate in [
                parent_dir.join(entrypoint_name),
                parent_dir.join(format!("{stem}.typ")),
                parent_dir.join("slides.typ"),
                parent_dir.join("presentation.typ"),
            ] {
                if candidate.is_file() {
                    target_typ = Some(candidate);
                    break;
                }
            }

            let typ_to_write = target_typ.unwrap_or_else(|| {
                if stem != "slides" && !stem.is_empty() {
                    parent_dir.join(format!("{stem}.typ"))
                } else {
                    parent_dir.join(entrypoint_name)
                }
            });

            if let Some(parent) = typ_to_write.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&typ_to_write, &self.source_text);
        } else {
            // Save as .typ
            let parent_dir = path
                .parent()
                .map(|p| {
                    if p.as_os_str().is_empty() {
                        Path::new(".")
                    } else {
                        p
                    }
                })
                .unwrap_or(Path::new("."));
            let _ = std::fs::create_dir_all(parent_dir);
            std::fs::write(&path, &self.source_text)?;
            self.format = DocumentFormat::Typst;
        }

        self.file_path = Some(path.clone());
        self.is_dirty = false;
        let _ = self.clear_recovery_draft();
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
            let deck_title = deck
                .title
                .replace(['\r', '\n', '"', '$', '#', '[', ']', '(', ')'], " ");
            let dt = deck_title.trim();
            let title = if dt.is_empty() {
                "Untitled"
            } else {
                dt
            };
            source.push_str(&format!("= {title}\n\nBlank presentation."));
            return source;
        }

        for (idx, slide) in deck.slides.iter().enumerate() {
            if idx > 0 {
                source.push_str("\n#pagebreak()\n\n");
            }
            source.push_str(&format!("// --- Slide {} ---\n", slide.page_number));

            // Extract clean title from slide SVG and sanitize special Typst characters
            let raw_title = slide.extract_title();
            let sanitized_title = raw_title
                .replace(['\r', '\n', '"', '$', '#', '[', ']', '(', ')'], " ")
                .trim()
                .to_string();
            let title = if sanitized_title.is_empty() {
                format!("Slide {}", slide.page_number)
            } else {
                sanitized_title
            };

            if idx == 0 && (title.starts_with("Slide ") || title.is_empty()) {
                let deck_title = deck
                    .title
                    .replace(['\r', '\n', '"', '$', '#', '[', ']', '(', ')'], " ");
                let dt = deck_title.trim();
                let main_title = if dt.is_empty() {
                    "Presentation"
                } else {
                    dt
                };
                source.push_str(&format!("= {main_title}\n\n"));
            } else {
                source.push_str(&format!("= {title}\n\n"));
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
        if !self.has_editable_source()
            && let Some(ref d) = self.deck
            && !d.slides.is_empty()
        {
            self.preamble = "// Pre-compiled presentation package\n".to_string();
            self.slide_chunks = d
                .slides
                .iter()
                .enumerate()
                .map(|(idx, s)| {
                    let raw_title = s.extract_title();
                    let clean_title = raw_title
                        .replace(['\r', '\n', '"', '$', '#', '[', ']', '(', ')'], " ")
                        .trim()
                        .to_string();
                    let title = if clean_title.is_empty() {
                        format!("Slide {}", idx + 1)
                    } else {
                        clean_title
                    };
                    format!("= {title}\n\n// Pre-rendered slide vector content")
                })
                .collect();
            return;
        }

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

    /// Insert a custom slide chunk at a specific position
    pub fn insert_custom_slide_chunk_at(
        &mut self,
        target_idx: usize,
        chunk: &str,
    ) {
        let idx = target_idx.min(self.slide_chunks.len());
        self.slide_chunks.insert(idx, chunk.to_string());
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
        if !self.has_editable_source()
            && let Some(ref d) = self.deck
            && !d.slides.is_empty()
        {
            return d.slides.len();
        }
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
            && !self
                .source_text
                .starts_with("// Imported from .slide package")
    }

    /// Compute the path for auto-saved recovery draft (`.<filename>.draft.typ`)
    #[must_use]
    pub fn recovery_draft_path(&self) -> Option<PathBuf> {
        let path = self.file_path.as_ref()?;
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let filename = path.file_name()?.to_string_lossy();
        Some(parent.join(format!(".{filename}.draft.typ")))
    }

    /// Save an auto-recovery draft if the document is dirty
    pub fn save_recovery_draft(&self) -> Result<Option<PathBuf>> {
        if !self.is_dirty || self.source_text.trim().is_empty() {
            return Ok(None);
        }
        if let Some(draft_path) = self.recovery_draft_path() {
            if let Some(parent) = draft_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&draft_path, &self.source_text)?;
            Ok(Some(draft_path))
        } else {
            Ok(None)
        }
    }

    /// Remove the recovery draft after successful save or discard
    pub fn clear_recovery_draft(&self) -> Result<()> {
        if let Some(draft_path) = self.recovery_draft_path()
            && draft_path.exists()
        {
            let _ = std::fs::remove_file(draft_path);
        }
        Ok(())
    }

    /// Check if a recovery draft exists for a given original file path
    #[must_use]
    pub fn check_recovery_draft(original_path: &Path) -> Option<String> {
        let parent = original_path.parent().unwrap_or_else(|| Path::new("."));
        let filename = original_path.file_name()?.to_string_lossy();
        let draft_path = parent.join(format!(".{filename}.draft.typ"));
        if draft_path.is_file() {
            std::fs::read_to_string(draft_path).ok()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_document_slide_management() {
        let mut doc = EditorDocument::new_presentation("Test Title");
        let initial_count = doc.total_slides();
        assert!(initial_count >= 2);

        doc.duplicate_slide(0);
        assert_eq!(doc.total_slides(), initial_count + 1);

        doc.delete_slide(1);
        assert_eq!(doc.total_slides(), initial_count);
    }

    #[test]
    fn test_recovery_draft_lifecycle() {
        let temp_dir = tempfile::tempdir().unwrap();
        let orig_path = temp_dir.path().join("slides.typ");

        let mut doc = EditorDocument::new_presentation("Draft Test");
        doc.file_path = Some(orig_path.clone());
        doc.is_dirty = true;
        doc.source_text = "= Recovered Content".to_string();

        let draft_path = doc.save_recovery_draft().unwrap();
        assert!(draft_path.is_some());
        let draft_path = draft_path.unwrap();
        assert!(draft_path.exists());

        let recovered = EditorDocument::check_recovery_draft(&orig_path);
        assert_eq!(recovered.as_deref(), Some("= Recovered Content"));

        doc.clear_recovery_draft().unwrap();
        assert!(!draft_path.exists());
        assert_eq!(EditorDocument::check_recovery_draft(&orig_path), None);
    }
}
