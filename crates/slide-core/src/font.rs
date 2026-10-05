//! System font discovery, query, and package font bundling.
//!
//! Automatically detects installed system fonts, provides smart sans-serif fallbacks,
//! and bundles fonts into origin packages so presentations render identically anywhere.

use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

/// Information about a detected system font family
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFamilyInfo {
    /// Family name (e.g. `Nimbus Sans`, `DejaVu Sans`, `Arial`)
    pub name: String,
    /// Absolute paths to font files (.otf, .ttf) associated with this family
    pub file_paths: Vec<PathBuf>,
}

static SYSTEM_FONTS_CACHE: OnceLock<HashMap<String, FontFamilyInfo>> = OnceLock::new();

/// Detect all available system fonts and their file locations.
pub fn get_system_fonts() -> &'static HashMap<String, FontFamilyInfo> {
    SYSTEM_FONTS_CACHE.get_or_init(|| {
        let mut map = HashMap::new();

        // 1. Try querying `typst fonts --variants` which provides Typst's exact view of fonts
        if let Ok(output) = Command::new("typst").args(["fonts", "--variants"]).output()
            && output.status.success()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut current_family: Option<String> = None;

            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // A non-indented line is a font family header
                if !line.starts_with(' ') && !line.starts_with('\t') {
                    let name = trimmed.to_string();
                    map.entry(name.clone()).or_insert_with(|| {
                        FontFamilyInfo {
                            name: name.clone(),
                            file_paths: Vec::new(),
                        }
                    });
                    current_family = Some(name);
                } else if let Some(ref family) = current_family {
                    // Lines with paths typically contain "├ " or "└ " followed by absolute path
                    if let Some(pos) = line.find('/') {
                        let path_candidate = line[pos..].trim();
                        // Strip any trailing parentheses or comments like " (Variable)"
                        let clean_path = path_candidate
                            .split_whitespace()
                            .next()
                            .unwrap_or(path_candidate);
                        let p = PathBuf::from(clean_path);
                        if p.is_file()
                            && let Some(info) = map.get_mut(family)
                            && !info.file_paths.contains(&p)
                        {
                            info.file_paths.push(p);
                        }
                    }
                }
            }
        }

        // 2. Scan standard system font directories if map is empty or to complement
        let search_dirs = get_system_font_directories();
        for dir in search_dirs {
            if dir.is_dir() {
                for entry in walkdir::WalkDir::new(&dir)
                    .max_depth(4)
                    .into_iter()
                    .flatten()
                {
                    let p = entry.path();
                    if p.is_file()
                        && let Some(ext) = p.extension().and_then(|e| e.to_str())
                    {
                        let ext_lower = ext.to_lowercase();
                        if (ext_lower == "otf"
                            || ext_lower == "ttf"
                            || ext_lower == "woff2"
                            || ext_lower == "ttc")
                            && let Some(stem) = p.file_stem().and_then(|s| s.to_str())
                        {
                            // Normalize basic names
                            let clean_name = stem
                                .replace(['-', '_'], " ")
                                .split("Regular")
                                .next()
                                .unwrap_or(stem)
                                .trim()
                                .to_string();
                            if !clean_name.is_empty() {
                                let entry = map.entry(clean_name.clone()).or_insert_with(|| {
                                    FontFamilyInfo {
                                        name: clean_name,
                                        file_paths: Vec::new(),
                                    }
                                });
                                if !entry.file_paths.contains(&p.to_path_buf()) {
                                    entry.file_paths.push(p.to_path_buf());
                                }
                            }
                        }
                    }
                }
            }
        }

        map
    })
}

/// Return standard operating system font directories
fn get_system_font_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    #[cfg(target_os = "linux")]
    {
        dirs.push(PathBuf::from("/usr/share/fonts"));
        dirs.push(PathBuf::from("/usr/local/share/fonts"));
        if let Some(home) = std::env::var_os("HOME") {
            let h = PathBuf::from(home);
            dirs.push(h.join(".fonts"));
            dirs.push(h.join(".local/share/fonts"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        dirs.push(PathBuf::from("/System/Library/Fonts"));
        dirs.push(PathBuf::from("/Library/Fonts"));
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(home).join("Library/Fonts"));
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(windir) = std::env::var_os("WINDIR") {
            dirs.push(PathBuf::from(windir).join("Fonts"));
        }
        if let Some(appdata) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(PathBuf::from(appdata).join("Microsoft\\Windows\\Fonts"));
        }
    }

    dirs
}

/// Return a sorted, deduplicated list of all available system font family names
#[must_use]
pub fn list_system_font_families() -> Vec<String> {
    let fonts = get_system_fonts();
    let mut names: Vec<String> = fonts.keys().cloned().collect();
    names.sort_by_key(|a| a.to_lowercase());
    names.dedup();
    names
}

/// Detect the best installed sans-serif font family to prevent unknown font warnings
#[must_use]
pub fn detect_default_sans_font() -> String {
    // Priority list of widely respected sans-serif fonts
    const CANDIDATES: &[&str] = &[
        "Liberation Sans",
        "Nimbus Sans",
        "DejaVu Sans",
        "FreeSans",
        "Cantarell",
        "Adwaita Sans",
        "Arial",
        "Helvetica",
        "Noto Sans",
        "Roboto",
        "Inter",
        "Segoe UI",
        "SF Pro Display",
    ];

    let fonts = get_system_fonts();

    for candidate in CANDIDATES {
        if fonts.contains_key(*candidate) {
            return (*candidate).to_string();
        }
    }

    // Secondary scan: check if any font has "Sans" or "Arial" in its name
    for key in fonts.keys() {
        if key.contains("Sans") || key.contains("Arial") || key.contains("Helvetica") {
            return key.clone();
        }
    }

    // Default fallback
    "Liberation Sans".to_string()
}

/// Look up file paths for a font family name
#[must_use]
pub fn find_font_files_for_family(family: &str) -> Vec<PathBuf> {
    let fonts = get_system_fonts();
    if let Some(info) = fonts.get(family) {
        return info.file_paths.clone();
    }

    // Case-insensitive fallback lookup
    let lower = family.to_lowercase();
    for (name, info) in fonts {
        if name.to_lowercase() == lower {
            return info.file_paths.clone();
        }
    }

    Vec::new()
}

/// Extract font family names referenced in a Typst document source code
#[must_use]
pub fn extract_font_families_from_typst(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut seen = HashSet::new();

    // Scan for font: "Family Name" or font: ("Family1", "Family2")
    for line in source.lines() {
        if let Some(font_idx) = line.find("font:") {
            let rest = &line[font_idx + 5..];
            let mut in_quote = false;
            let mut quote_char = '"';
            let mut cur_name = String::new();

            for ch in rest.chars() {
                if !in_quote && (ch == '"' || ch == '\'') {
                    in_quote = true;
                    quote_char = ch;
                    cur_name.clear();
                } else if in_quote && ch == quote_char {
                    in_quote = false;
                    let clean = cur_name.trim().to_string();
                    if !clean.is_empty() && seen.insert(clean.clone()) {
                        found.push(clean);
                    }
                } else if in_quote {
                    cur_name.push(ch);
                } else if ch == ')' || ch == ']' || ch == '\n' {
                    break;
                }
            }
        }
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_system_fonts() {
        let families = list_system_font_families();
        // On Linux/CI, at least some system fonts should be discovered
        assert_ne!(families, [] as [std::string::String; 0]);
    }

    #[test]
    fn test_detect_default_sans_font() {
        let sans = detect_default_sans_font();
        assert_ne!(sans, "");
    }

    #[test]
    fn test_extract_font_families() {
        let source = r#"
            #set text(font: "Liberation Sans", size: 14pt)
            #set text(font: ("Nimbus Sans", "Arial"))
        "#;
        let fonts = extract_font_families_from_typst(source);
        assert_eq!(fonts, vec!["Liberation Sans", "Nimbus Sans", "Arial"]);
    }
}
