use crate::error::Result;
use crate::error::SlideError;
use crate::model::SlideDeck;
use serde::Deserialize;
use serde::Serialize;
use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;
use std::io::Write;
use std::path::Path;

/// Metadata header stored inside the `.slide` standalone package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlidePackageMetadata {
    pub format_version: u32,
    pub title: String,
    pub total_slides: usize,
    pub default_animation: String,
    pub compression: String,
    pub generator: String,
    #[serde(default)]
    pub is_editable: bool,
    #[serde(default)]
    pub entrypoint: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub aspect_ratio: Option<String>,
    #[serde(default)]
    pub has_notes: bool,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Default for SlidePackageMetadata {
    fn default() -> Self {
        Self {
            format_version: 1,
            title: "Presentation".to_string(),
            total_slides: 0,
            default_animation: "fade".to_string(),
            compression: "lzma2-max".to_string(),
            generator: format!("cargo-slide {}", env!("CARGO_PKG_VERSION")),
            is_editable: false,
            entrypoint: None,
            created_at: None,
            author: None,
            aspect_ratio: Some("16:9".to_string()),
            has_notes: false,
            tags: Vec::new(),
        }
    }
}

/// Package a `SlideDeck` along with its optional asset directory into a standalone `.slide` file.
pub fn pack_deck_with_assets(
    deck: &SlideDeck,
    assets_dir: Option<&Path>,
    output_path: &Path,
) -> Result<()> {
    pack_deck_with_source_and_assets(deck, assets_dir, output_path, false, None)
}

/// Package a `SlideDeck` along with its optional asset directory and optional editable source files into a standalone `.slide` file.
pub fn pack_deck_with_source_and_assets(
    deck: &SlideDeck,
    assets_dir: Option<&Path>,
    output_path: &Path,
    include_source: bool,
    source_entrypoint: Option<&Path>,
) -> Result<()> {
    if let Some(parent) = output_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }

    let entry_name = source_entrypoint
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .map(|s| s.to_string());

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default();

    let metadata = SlidePackageMetadata {
        format_version: 1,
        title: deck.title.clone(),
        total_slides: deck.total_slides(),
        default_animation: deck.default_animation.clone(),
        compression: "lzma2-max".to_string(),
        generator: format!("cargo-slide {}", env!("CARGO_PKG_VERSION")),
        is_editable: include_source,
        entrypoint: entry_name,
        created_at: Some(timestamp),
        author: None,
        aspect_ratio: Some("16:9".to_string()),
        has_notes: deck.has_any_notes(),
        tags: Vec::new(),
    };

    let metadata_json = serde_json::to_vec_pretty(&metadata)?;
    let deck_json = serde_json::to_vec(deck)?;

    // Create an in-memory TAR archive
    let mut tar_builder = tar::Builder::new(Vec::new());

    // Add manifest.json
    let mut meta_header = tar::Header::new_gnu();
    meta_header.set_size(metadata_json.len() as u64);
    meta_header.set_mode(0o644);
    meta_header.set_cksum();
    tar_builder.append_data(&mut meta_header, "manifest.json", &metadata_json[..])?;

    // Add deck.json
    let mut deck_header = tar::Header::new_gnu();
    deck_header.set_size(deck_json.len() as u64);
    deck_header.set_mode(0o644);
    deck_header.set_cksum();
    tar_builder.append_data(&mut deck_header, "deck.json", &deck_json[..])?;

    // Helper to add a file to TAR archive
    let add_file_to_tar = |builder: &mut tar::Builder<Vec<u8>>,
                           paths: &mut std::collections::HashSet<String>,
                           rel_path: &str,
                           abs_path: &Path|
     -> Result<()> {
        let clean_rel = rel_path
            .replace('\\', "/")
            .trim_start_matches('/')
            .to_string();
        if clean_rel.is_empty() {
            return Ok(());
        }
        if paths.insert(clean_rel.clone())
            && let Ok(data) = std::fs::read(abs_path)
        {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, clean_rel, &data[..])?;
        }
        Ok(())
    };

    let mut added_paths = std::collections::HashSet::<String>::new();
    added_paths.insert("manifest.json".to_string());
    added_paths.insert("deck.json".to_string());

    if include_source {
        if let Some(src_path) = source_entrypoint
            && let Ok(src_data) = std::fs::read(src_path)
        {
            // Store standard entrypoint source.typ
            if added_paths.insert("source.typ".to_string()) {
                let mut header = tar::Header::new_gnu();
                header.set_size(src_data.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                let _ = tar_builder.append_data(&mut header, "source.typ", &src_data[..]);
            }
            // Store original filename as well if different
            if let Some(name) = src_path.file_name().and_then(|s| s.to_str())
                && name != "source.typ"
                && added_paths.insert(name.to_string())
            {
                let mut header = tar::Header::new_gnu();
                header.set_size(src_data.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                let _ = tar_builder.append_data(&mut header, name, &src_data[..]);
            }
        }

        // Recursively bundle all .typ files under assets_dir
        if let Some(base) = assets_dir {
            for entry in walkdir::WalkDir::new(base).into_iter().flatten() {
                let path = entry.path();
                if path.is_file()
                    && path.extension().and_then(|e| e.to_str()) == Some("typ")
                    && let Ok(rel) = path.strip_prefix(base)
                {
                    let rel_str = rel.to_string_lossy().replace('\\', "/");
                    add_file_to_tar(&mut tar_builder, &mut added_paths, &rel_str, path)?;
                }
            }
        }

        // Bundle referenced system fonts into fonts/ directory in package
        let mut referenced_fonts = std::collections::HashSet::new();
        if let Some(src_path) = source_entrypoint
            && let Ok(src_content) = std::fs::read_to_string(src_path)
        {
            for f in crate::font::extract_font_families_from_typst(&src_content) {
                referenced_fonts.insert(f);
            }
        }
        if let Some(base) = assets_dir {
            for entry in walkdir::WalkDir::new(base)
                .max_depth(3)
                .into_iter()
                .flatten()
            {
                let p = entry.path();
                if p.is_file()
                    && p.extension().and_then(|e| e.to_str()) == Some("typ")
                    && let Ok(c) = std::fs::read_to_string(p)
                {
                    for f in crate::font::extract_font_families_from_typst(&c) {
                        referenced_fonts.insert(f);
                    }
                }
            }
        }

        for font_family in referenced_fonts {
            let font_files = crate::font::find_font_files_for_family(&font_family);
            for f_path in font_files {
                if let Some(f_name) = f_path.file_name().and_then(|s| s.to_str()) {
                    let rel_dest = format!("fonts/{f_name}");
                    add_file_to_tar(&mut tar_builder, &mut added_paths, &rel_dest, &f_path)?;
                }
            }
        }
    }

    if let Some(base) = assets_dir {
        // 1. Recursively bundle standard asset & media directories
        const ASSET_DIRS: &[&str] = &[
            "assets",
            "images",
            "img",
            "media",
            "static",
            "data",
            "fonts",
            "figures",
            "resources",
            "vendor",
        ];
        for dir_name in ASSET_DIRS {
            let folder = base.join(dir_name);
            if folder.is_dir() {
                for entry in walkdir::WalkDir::new(&folder).into_iter().flatten() {
                    if entry.file_type().is_file() {
                        let file_path = entry.path();
                        if let Ok(rel) = file_path.strip_prefix(base) {
                            let rel_str = rel.to_string_lossy().replace('\\', "/");
                            add_file_to_tar(
                                &mut tar_builder,
                                &mut added_paths,
                                &rel_str,
                                file_path,
                            )?;
                        }
                    }
                }
            }
        }

        // 2. Scan top-level files in base directory for common asset extensions
        const ASSET_EXTS: &[&str] = &[
            "png", "jpg", "jpeg", "svg", "webp", "gif", "bmp", "ico", "tiff", "mp3", "mp4", "wav",
            "ogg", "webm", "m4a", "mov", "csv", "json", "toml", "yaml", "yml", "txt", "sqlite",
            "db", "tsv", "pdf", "ttf", "otf", "woff", "woff2",
        ];
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file()
                    && let Some(ext) = path.extension().and_then(|e| e.to_str())
                    && ASSET_EXTS.iter().any(|&e| e.eq_ignore_ascii_case(ext))
                    && let Some(name) = path.file_name().and_then(|s| s.to_str())
                {
                    add_file_to_tar(&mut tar_builder, &mut added_paths, name, &path)?;
                }
            }
        }

        // 3. Scan .typ files for explicitly referenced relative assets (e.g. #image("..."), #read("..."), #csv("..."))
        let mut typ_files = Vec::new();
        if let Some(src) = source_entrypoint {
            typ_files.push(src.to_path_buf());
        }
        for entry in walkdir::WalkDir::new(base)
            .max_depth(3)
            .into_iter()
            .flatten()
        {
            let p = entry.path();
            if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("typ") {
                typ_files.push(p.to_path_buf());
            }
        }
        for typ_file in typ_files {
            if let Ok(content) = std::fs::read_to_string(&typ_file) {
                for candidate in extract_typst_asset_references(&content) {
                    let rel_clean = candidate.replace('\\', "/");
                    let file_path = base.join(&rel_clean);
                    if file_path.is_file() {
                        add_file_to_tar(
                            &mut tar_builder,
                            &mut added_paths,
                            &rel_clean,
                            &file_path,
                        )?;
                    }
                }
            }
        }

        // 4. Scan hotspots across all slides for local relative file links, audio, and video
        for slide in &deck.slides {
            for hotspot in &slide.hotspots {
                let candidate_path = match hotspot {
                    | crate::model::Hotspot::Link { target, .. } => {
                        let clean = target.strip_prefix('#').unwrap_or(target);
                        if clean.starts_with("chart:")
                            || clean.starts_with("video:")
                            || clean.starts_with("audio:")
                            || clean.starts_with("step:")
                            || clean.starts_with("transition:")
                            || clean.starts_with("mailto:")
                            || clean.contains("://")
                            || target.starts_with('#')
                        {
                            None
                        } else {
                            Some(target.strip_prefix("file://").unwrap_or(target))
                        }
                    },
                    | crate::model::Hotspot::Video { source, .. }
                    | crate::model::Hotspot::Audio { source, .. } => {
                        if source.contains("://") || source.starts_with('#') {
                            None
                        } else {
                            Some(source.strip_prefix("file://").unwrap_or(source))
                        }
                    },
                    | _ => None,
                };

                if let Some(raw_rel) = candidate_path {
                    let rel_clean = raw_rel.replace('\\', "/");
                    let file_path = base.join(&rel_clean);
                    if file_path.is_file() {
                        add_file_to_tar(
                            &mut tar_builder,
                            &mut added_paths,
                            &rel_clean,
                            &file_path,
                        )?;
                    }
                }
            }
        }
    }

    let uncompressed_tar = tar_builder.into_inner()?;

    // Compress with LZMA2 at maximum compression ratio (Level 9 + Extreme preset)
    let file = File::create(output_path)?;
    let writer = BufWriter::new(file);

    // XZ container format with LZMA2 filter preset 9 | Extreme flag
    const LZMA_PRESET_EXTREME: u32 = 1 << 31;
    let mut encoder =
        xz2::write::XzEncoder::new_stream(
            writer,
            xz2::stream::Stream::new_easy_encoder(
                9 | LZMA_PRESET_EXTREME,
                xz2::stream::Check::Crc64,
            )
            .map_err(|e| {
                SlideError::Compilation(format!("Failed to initialize LZMA2 encoder: {e}"))
            })?,
        );

    encoder.write_all(&uncompressed_tar)?;
    let mut writer = encoder
        .finish()
        .map_err(|e| SlideError::Compilation(format!("LZMA2 compression error: {e}")))?;
    writer.flush()?;

    Ok(())
}

/// Package a `SlideDeck` into a standalone `.slide` file using LZMA2 maximum compression.
pub fn pack_deck_to_file(
    deck: &SlideDeck,
    output_path: &Path,
) -> Result<()> {
    pack_deck_with_assets(deck, None, output_path)
}

/// Helper to compute deterministic temporary cache directory for a package
pub fn get_package_cache_dir(input_path: &Path) -> std::path::PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hash;
    use std::hash::Hasher;
    let mut hasher = DefaultHasher::new();
    input_path.hash(&mut hasher);
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("deck");
    let hash = hasher.finish();
    std::env::temp_dir()
        .join("cargo_slide_cache")
        .join(format!("{stem}_{hash:016x}"))
}

/// Unpack a `SlideDeck` and its assets from a standalone `.slide` file.
/// Returns the `SlideDeck` and the directory containing extracted assets.
pub fn unpack_deck_and_assets(input_path: &Path) -> Result<(SlideDeck, std::path::PathBuf)> {
    let file = File::open(input_path)?;
    let mut reader = BufReader::new(file);

    // Read initial bytes to check magic
    let mut magic = [0u8; 6];
    let n = reader.read(&mut magic)?;
    let file = File::open(input_path)?;

    let cache_dir = get_package_cache_dir(input_path);
    let _ = std::fs::create_dir_all(&cache_dir);

    // Check if XZ (standard LZMA2 container starts with \xFD7zXZ\x00)
    if n >= 6 && magic == [0xFD, b'7', b'z', b'X', b'Z', 0x00] {
        let decompressor = xz2::read::XzDecoder::new(file);
        let mut tar_archive = tar::Archive::new(decompressor);

        let mut deck_data = None;
        for entry in tar_archive.entries()? {
            let mut entry = entry?;
            let path_str = entry.path()?.to_string_lossy().to_string();
            if path_str == "deck.json" || path_str.ends_with("/deck.json") {
                let mut content = Vec::new();
                entry.read_to_end(&mut content)?;
                deck_data = Some(content);
            } else if path_str != "manifest.json" && !path_str.ends_with("/manifest.json") {
                let dest = cache_dir.join(&path_str);
                if let Some(parent) = dest.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let mut out = File::create(&dest)?;
                std::io::copy(&mut entry, &mut out)?;
            }
        }

        if let Some(data) = deck_data {
            let deck: SlideDeck = serde_json::from_slice(&data)?;
            return Ok((deck, cache_dir));
        }
        return Err(SlideError::Compilation(
            "No deck.json found in .slide archive".to_string(),
        ));
    }

    // Check if ZIP archive (starts with PK\x03\x04)
    if n >= 4 && magic[0..4] == [0x50, 0x4B, 0x03, 0x04] {
        let mut zip_archive = zip::ZipArchive::new(file)?;
        let mut deck_data = None;
        for i in 0..zip_archive.len() {
            let mut entry = zip_archive.by_index(i)?;
            let name = entry.name().to_string();
            if name == "deck.json" || name.ends_with("/deck.json") {
                let mut content = Vec::new();
                entry.read_to_end(&mut content)?;
                deck_data = Some(content);
            } else if name != "manifest.json"
                && !name.ends_with("/manifest.json")
                && !entry.is_dir()
            {
                let dest = cache_dir.join(&name);
                if let Some(parent) = dest.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let mut out = File::create(&dest)?;
                std::io::copy(&mut entry, &mut out)?;
            }
        }
        if let Some(data) = deck_data {
            let deck: SlideDeck = serde_json::from_slice(&data)?;
            return Ok((deck, cache_dir));
        }
        return Err(SlideError::Compilation(
            "No deck.json found in zip package".to_string(),
        ));
    }

    // Try pure Rust lzma_rs decompressor as fallback
    let mut uncompressed = Vec::new();
    let mut buf_file = BufReader::new(file);
    if lzma_rs::xz_decompress(&mut buf_file, &mut uncompressed).is_ok() || {
        let mut retry_file = BufReader::new(File::open(input_path)?);
        uncompressed.clear();
        lzma_rs::lzma2_decompress(&mut retry_file, &mut uncompressed).is_ok()
    } {
        let mut tar_archive = tar::Archive::new(&uncompressed[..]);
        let mut deck_data = None;
        if let Ok(entries) = tar_archive.entries() {
            for mut entry in entries.flatten() {
                let path_str = entry
                    .path()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                if path_str == "deck.json" || path_str.ends_with("/deck.json") {
                    let mut content = Vec::new();
                    entry.read_to_end(&mut content)?;
                    deck_data = Some(content);
                } else if path_str != "manifest.json" && !path_str.ends_with("/manifest.json") {
                    let dest = cache_dir.join(&path_str);
                    if let Some(parent) = dest.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Ok(mut out) = File::create(&dest) {
                        let _ = std::io::copy(&mut entry, &mut out);
                    }
                }
            }
        }
        if let Some(data) = deck_data {
            let deck: SlideDeck = serde_json::from_slice(&data)?;
            return Ok((deck, cache_dir));
        }
        if let Ok(deck) = serde_json::from_slice::<SlideDeck>(&uncompressed) {
            return Ok((deck, cache_dir));
        }
    }

    Err(SlideError::Compilation(format!(
        "Unsupported or corrupted slide package: {}",
        input_path.display()
    )))
}

/// Unpack a `SlideDeck` directly from raw in-memory bytes and extract assets to cache.
pub fn unpack_deck_and_assets_from_bytes(
    bytes: &[u8],
    package_name: &str,
) -> Result<(SlideDeck, std::path::PathBuf)> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hash;
    use std::hash::Hasher;
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    let cache_dir = std::env::temp_dir()
        .join("cargo_slide_cache")
        .join(format!("{package_name}_{hash:016x}"));
    let _ = std::fs::create_dir_all(&cache_dir);

    // Decompress XZ stream
    let cursor = std::io::Cursor::new(bytes);
    let decompressor = xz2::read::XzDecoder::new(cursor);
    let mut tar_archive = tar::Archive::new(decompressor);

    let mut deck_data = None;
    for entry in tar_archive.entries()? {
        let mut entry = entry?;
        let path_str = entry.path()?.to_string_lossy().to_string();
        if path_str == "deck.json" || path_str.ends_with("/deck.json") {
            let mut content = Vec::new();
            entry.read_to_end(&mut content)?;
            deck_data = Some(content);
        } else if path_str != "manifest.json" && !path_str.ends_with("/manifest.json") {
            let dest = cache_dir.join(&path_str);
            if let Some(parent) = dest.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let mut out = File::create(&dest)?;
            std::io::copy(&mut entry, &mut out)?;
        }
    }

    if let Some(data) = deck_data {
        let deck: SlideDeck = serde_json::from_slice(&data)?;
        return Ok((deck, cache_dir));
    }

    Err(SlideError::Compilation(
        "No deck.json found in embedded slide package".to_string(),
    ))
}

/// Unpack a `SlideDeck` from a standalone `.slide` file.
/// Supports LZMA2 tar archives and legacy/zip archives transparently.
pub fn unpack_deck_from_file(input_path: &Path) -> Result<SlideDeck> {
    unpack_deck_and_assets(input_path).map(|(deck, _)| deck)
}

/// Read only the package metadata without deserializing the whole deck.
pub fn read_package_metadata(input_path: &Path) -> Result<SlidePackageMetadata> {
    let file = File::open(input_path)?;
    let mut reader = BufReader::new(file);

    let mut magic = [0u8; 6];
    let n = reader.read(&mut magic)?;
    let file = File::open(input_path)?;

    if n >= 6 && magic == [0xFD, b'7', b'z', b'X', b'Z', 0x00] {
        let decompressor = xz2::read::XzDecoder::new(file);
        let mut tar_archive = tar::Archive::new(decompressor);
        for entry in tar_archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_string_lossy().to_string();
            if path == "manifest.json" || path.ends_with("/manifest.json") {
                let mut content = Vec::new();
                entry.read_to_end(&mut content)?;
                let meta: SlidePackageMetadata = serde_json::from_slice(&content)?;
                return Ok(meta);
            }
        }
    }

    // If no manifest found or different format, fallback to full unpack
    let deck = unpack_deck_from_file(input_path)?;
    Ok(SlidePackageMetadata {
        format_version: 1,
        title: deck.title.clone(),
        total_slides: deck.total_slides(),
        default_animation: deck.default_animation.clone(),
        compression: "lzma2".to_string(),
        generator: "cargo-slide".to_string(),
        is_editable: false,
        entrypoint: None,
        created_at: None,
        author: None,
        aspect_ratio: Some("16:9".to_string()),
        has_notes: deck.has_any_notes(),
        tags: Vec::new(),
    })
}

/// Comprehensive package integrity verification report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageVerificationReport {
    /// Whether package passed all structural and checksum checks
    pub is_valid: bool,
    /// Package metadata parsed from manifest
    pub metadata: SlidePackageMetadata,
    /// Total slides successfully verified in deck.json
    pub slide_count: usize,
    /// Number of embedded assets found in package
    pub asset_count: usize,
    /// Size of package file on disk in bytes
    pub file_size: u64,
    /// Any warnings encountered during inspection
    pub warnings: Vec<String>,
}

/// Verify that a `.slide` package is intact, readable, and structurally sound.
pub fn verify_package_integrity(package_path: &Path) -> Result<PackageVerificationReport> {
    let file_size = std::fs::metadata(package_path)
        .map(|m| m.len())
        .map_err(|e| SlideError::Package(format!("Failed to read package file size: {e}")))?;

    let meta = read_package_metadata(package_path)?;
    let (deck, temp_dir) = unpack_deck_and_assets(package_path)?;

    let mut asset_count = 0usize;
    let mut warnings = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&temp_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                asset_count = asset_count.saturating_add(1);
            } else if p.is_dir() {
                for sub in walkdir::WalkDir::new(&p).into_iter().flatten() {
                    if sub.file_type().is_file() {
                        asset_count = asset_count.saturating_add(1);
                    }
                }
            }
        }
    }

    if deck.slides.len() != meta.total_slides && meta.total_slides > 0 {
        warnings.push(format!(
            "Slide count mismatch: manifest states {}, but deck contains {}",
            meta.total_slides,
            deck.slides.len()
        ));
    }

    if deck.slides.is_empty() {
        warnings.push("Package contains zero slides.".to_string());
    }

    Ok(PackageVerificationReport {
        is_valid: warnings.is_empty(),
        metadata: meta,
        slide_count: deck.slides.len(),
        asset_count,
        file_size,
        warnings,
    })
}

/// Unpack a standalone `.slide` package into a directory to produce an editable presentation project.
/// Returns the path to the primary `.typ` presentation file (entrypoint).
pub fn unpack_package_to_dir(
    package_path: &Path,
    output_dir: &Path,
) -> Result<std::path::PathBuf> {
    std::fs::create_dir_all(output_dir)?;

    let file = File::open(package_path)?;
    let mut reader = BufReader::new(file);

    let mut magic = [0u8; 6];
    let n = reader.read(&mut magic)?;
    let file = File::open(package_path)?;

    let mut extracted_entrypoint = None;

    if n >= 6 && magic == [0xFD, b'7', b'z', b'X', b'Z', 0x00] {
        let decompressor = xz2::read::XzDecoder::new(file);
        let mut tar_archive = tar::Archive::new(decompressor);

        for entry in tar_archive.entries()? {
            let mut entry = entry?;
            let path_str = entry.path()?.to_string_lossy().to_string();
            let dest = output_dir.join(&path_str);
            if let Some(parent) = dest.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let mut out = File::create(&dest)?;
            std::io::copy(&mut entry, &mut out)?;

            if (path_str == "manifest.json" || path_str.ends_with("/manifest.json"))
                && let Ok(manifest_bytes) = std::fs::read(&dest)
                && let Ok(meta) = serde_json::from_slice::<SlidePackageMetadata>(&manifest_bytes)
                && let Some(ep) = meta.entrypoint
            {
                extracted_entrypoint = Some(ep);
            }
        }
    } else if n >= 4 && magic[0..4] == [0x50, 0x4B, 0x03, 0x04] {
        let mut zip_archive = zip::ZipArchive::new(file)?;
        for i in 0..zip_archive.len() {
            let mut entry = zip_archive.by_index(i)?;
            let name = entry.name().to_string();
            if entry.is_dir() {
                let _ = std::fs::create_dir_all(output_dir.join(&name));
            } else {
                let dest = output_dir.join(&name);
                if let Some(parent) = dest.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let mut out = File::create(&dest)?;
                std::io::copy(&mut entry, &mut out)?;
            }
        }
    } else {
        // Fallback using unpack_deck_and_assets
        let (deck, cache_dir) = unpack_deck_and_assets(package_path)?;
        for entry in walkdir::WalkDir::new(&cache_dir).into_iter().flatten() {
            if entry.file_type().is_file()
                && let Ok(rel) = entry.path().strip_prefix(&cache_dir)
            {
                let dest = output_dir.join(rel);
                if let Some(parent) = dest.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::copy(entry.path(), dest);
            }
        }
        let deck_json = serde_json::to_vec_pretty(&deck)?;
        let _ = std::fs::write(output_dir.join("deck.json"), deck_json);
    }

    // Determine entrypoint file
    if let Some(ref ep) = extracted_entrypoint {
        let p = output_dir.join(ep);
        if p.is_file() {
            return Ok(p);
        }
    }

    // Check common entrypoint file names in output directory
    let candidates = ["slides.typ", "source.typ", "presentation.typ", "main.typ"];
    for cand in candidates {
        let p = output_dir.join(cand);
        if p.is_file() {
            return Ok(p);
        }
    }

    // If no .typ file exists, check any .typ file in output directory
    if let Ok(entries) = std::fs::read_dir(output_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("typ") {
                return Ok(path);
            }
        }
    }

    // If only deck.json was found, synthesize slides.typ
    let deck_file = output_dir.join("deck.json");
    if deck_file.is_file()
        && let Ok(data) = std::fs::read(&deck_file)
        && let Ok(deck) = serde_json::from_slice::<SlideDeck>(&data)
    {
        let mut source = format!(
            "// Cargo Slide Presentation: {}\n#set page(width: 16cm, height: 9cm)\n\n",
            deck.title
        );
        for slide in &deck.slides {
            let title = format!("Slide {}", slide.page_number);
            source.push_str(&format!("= {title}\n\n"));
            if slide.page_number < deck.slides.len() {
                source.push_str("#pagebreak()\n\n");
            }
        }
        let out_typ = output_dir.join("slides.typ");
        std::fs::write(&out_typ, source)?;
        return Ok(out_typ);
    }

    Ok(output_dir.join("slides.typ"))
}

/// Extract relative asset file references inside Typst code (e.g. #image("..."), #read("..."), #csv("..."))
#[must_use]
pub fn extract_typst_asset_references(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let keywords = [
        "image(",
        "read(",
        "csv(",
        "json(",
        "yaml(",
        "toml(",
        "xml(",
        "sqlite(",
        "include \"",
        "import \"",
        "include '",
        "import '",
    ];
    for kw in &keywords {
        let mut start = 0;
        while let Some(pos) = text[start..].find(kw) {
            let abs_pos = start + pos + kw.len();
            let remainder = &text[abs_pos..];
            let quote_char = if kw.ends_with('"') {
                Some('"')
            } else if kw.ends_with('\'') {
                Some('\'')
            } else {
                remainder.chars().find(|c| *c == '"' || *c == '\'')
            };
            if let Some(qc) = quote_char {
                let quote_start = if kw.ends_with(qc) {
                    abs_pos
                } else if let Some(q_idx) = text[abs_pos..].find(qc) {
                    abs_pos + q_idx + 1
                } else {
                    start = abs_pos;
                    continue;
                };
                if let Some(end_q) = text[quote_start..].find(qc) {
                    let path = &text[quote_start..quote_start + end_q];
                    let clean = path.trim().trim_start_matches("file://");
                    if !clean.is_empty()
                        && !clean.contains("://")
                        && !clean.starts_with('#')
                        && !clean.starts_with('@')
                    {
                        found.push(clean.to_string());
                    }
                    start = quote_start + end_q + 1;
                } else {
                    start = abs_pos;
                }
            } else {
                start = abs_pos;
            }
        }
    }
    found
}

impl SlideDeck {
    /// Save this presentation deck to a `.slide` package using LZMA2 maximum compression.
    pub fn save_to_package(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<()> {
        pack_deck_to_file(self, path.as_ref())
    }

    /// Load a presentation deck from a `.slide` package.
    pub fn load_from_package(path: impl AsRef<Path>) -> Result<Self> {
        unpack_deck_from_file(path.as_ref())
    }
}
