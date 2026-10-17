#![allow(unsafe_code)]
//! In-memory mapped asset store with memory zeroization protection against heap dumps.
//!
//! When presentations are encrypted, assets (images, audio, video, fonts, templates)
//! are held in RAM using `Zeroizing<Vec<u8>>` buffers and resolved directly in memory,
//! preventing unencrypted files from ever touching the disk.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use zeroize::Zeroizing;

/// Thread-safe in-memory asset store mapping relative paths to zeroized byte buffers.
#[derive(Debug, Clone, Default)]
pub struct MemoryAssetStore {
    assets: Arc<RwLock<HashMap<String, Zeroizing<Vec<u8>>>>>,
}

impl MemoryAssetStore {
    /// Create a new empty `MemoryAssetStore`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            assets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Normalize an asset path for consistent lookup across platforms.
    #[must_use]
    pub fn normalize_key(path: &str) -> String {
        let clean = path
            .strip_prefix("file://")
            .unwrap_or(path)
            .replace('\\', "/");
        clean.trim_start_matches('/').to_string()
    }

    /// Insert an asset into memory with automatic zeroization on drop.
    pub fn insert(
        &self,
        path: impl AsRef<str>,
        data: Vec<u8>,
    ) {
        let key = Self::normalize_key(path.as_ref());
        if let Ok(mut map) = self.assets.write() {
            map.insert(key, Zeroizing::new(data));
        }
    }

    /// Retrieve a clone of an asset's data if present.
    #[must_use]
    pub fn get(
        &self,
        path: &str,
    ) -> Option<Vec<u8>> {
        let key = Self::normalize_key(path);
        let map = self.assets.read().ok()?;
        if let Some(val) = map.get(&key) {
            return Some(val.to_vec());
        }

        // Try fuzzy lookup (e.g. filename only if uniquely matched)
        let file_name = std::path::Path::new(&key)
            .file_name()
            .and_then(|s| s.to_str())?;
        for (k, v) in map.iter() {
            if k == file_name || k.ends_with(&format!("/{file_name}")) {
                return Some(v.to_vec());
            }
        }

        None
    }

    /// Check if the store contains an asset at the given path.
    #[must_use]
    pub fn contains(
        &self,
        path: &str,
    ) -> bool {
        let key = Self::normalize_key(path);
        let Ok(map) = self.assets.read() else {
            return false;
        };
        if map.contains_key(&key) {
            return true;
        }

        let Some(file_name) = std::path::Path::new(&key)
            .file_name()
            .and_then(|s| s.to_str())
        else {
            return false;
        };
        map.keys()
            .any(|k| k == file_name || k.ends_with(&format!("/{file_name}")))
    }

    /// Total number of assets held in memory.
    #[must_use]
    pub fn len(&self) -> usize {
        self.assets.read().map(|m| m.len()).unwrap_or(0)
    }

    /// Whether the store is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// List of all asset paths registered in memory.
    #[must_use]
    pub fn keys(&self) -> Vec<String> {
        self.assets
            .read()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Clear all assets from memory, triggering memory zeroization of all buffers.
    pub fn clear(&self) {
        if let Ok(mut map) = self.assets.write() {
            map.clear();
        }
    }

    /// Resolve an image MIME type from file extension.
    #[must_use]
    pub fn detect_mime(path: &str) -> &'static str {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            | "png" => "image/png",
            | "jpg" | "jpeg" => "image/jpeg",
            | "svg" => "image/svg+xml",
            | "webp" => "image/webp",
            | "gif" => "image/gif",
            | "mp3" => "audio/mpeg",
            | "wav" => "audio/wav",
            | "mp4" => "video/mp4",
            | _ => "application/octet-stream",
        }
    }

    /// Inline all local image references in an SVG string with Base64 data-URIs
    /// resolved directly from this in-memory store.
    #[must_use]
    pub fn inline_svg_images(
        &self,
        svg_content: &str,
    ) -> String {
        let mut result = svg_content.to_string();
        for key in self.keys() {
            let mime = Self::detect_mime(&key);
            if !mime.starts_with("image/") {
                continue;
            }
            if let Some(data) = self.get(&key) {
                use base64::Engine as _;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                let data_uri = format!("data:{mime};base64,{b64}");

                // Replace common SVG attribute patterns:
                // href="assets/foo.png" or xlink:href="assets/foo.png"
                let candidates = [
                    format!("href=\"{key}\""),
                    format!("href='{key}'"),
                    format!("xlink:href=\"{key}\""),
                    format!("xlink:href='{key}'"),
                    format!("href=\"./{key}\""),
                    format!("href='./{key}'"),
                ];

                for cand in candidates {
                    if result.contains(&cand) {
                        let attr_prefix = if cand.starts_with("xlink:") {
                            "xlink:href=\""
                        } else {
                            "href=\""
                        };
                        let replacement = format!("{attr_prefix}{data_uri}\"");
                        result = result.replace(&cand, &replacement);
                    }
                }

                // Also check by simple basename if path had subdirectories
                if let Some(file_name) = std::path::Path::new(&key)
                    .file_name()
                    .and_then(|s| s.to_str())
                {
                    let base_cands = [
                        format!("href=\"{file_name}\""),
                        format!("href='{file_name}'"),
                        format!("xlink:href=\"{file_name}\""),
                        format!("xlink:href='{file_name}'"),
                    ];
                    for cand in base_cands {
                        if result.contains(&cand) {
                            let attr_prefix = if cand.starts_with("xlink:") {
                                "xlink:href=\""
                            } else {
                                "href=\""
                            };
                            let replacement = format!("{attr_prefix}{data_uri}\"");
                            result = result.replace(&cand, &replacement);
                        }
                    }
                }
            }
        }
        result
    }

    /// On Linux systems, create an anonymous in-memory file descriptor (memfd)
    /// for video/audio players (e.g. ffplay, mpv) that strictly require a file descriptor or path,
    /// without ever writing unencrypted data to physical disk.
    #[must_use]
    pub fn create_memfd_path(
        &self,
        path: &str,
    ) -> Option<std::path::PathBuf> {
        let data = self.get(path)?;
        #[cfg(target_os = "linux")]
        {
            use std::io::Write as _;
            use std::os::unix::io::AsRawFd as _;
            // memfd_create is supported on Linux kernels >= 3.17
            let name = std::ffi::CString::new("slide_memfd_asset").ok()?;
            let fd = unsafe { libc::syscall(libc::SYS_memfd_create, name.as_ptr(), 0) };
            if fd >= 0 {
                use std::os::unix::io::FromRawFd;
                let mut file =
                    unsafe { std::fs::File::from_raw_fd(fd as std::os::unix::io::RawFd) };
                if file.write_all(&data).is_ok() {
                    let path =
                        std::path::PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()));
                    // Intentionally leak or hold fd so external player can read it
                    std::mem::forget(file);
                    return Some(path);
                }
            }
        }
        let _ = data;
        None
    }
}
