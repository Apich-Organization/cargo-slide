//! Encrypted slide container format with SHA3-512 integrity verification,
//! PIV slot key binding, and in-memory zeroized unpacking.

use super::asset_store::MemoryAssetStore;
use super::cert::PivPublicKey;
use super::cert::PivSlot;
use super::cert::parse_certificate_public_key;
use super::crypto::aes_256_gcm_decrypt;
use super::crypto::aes_256_gcm_encrypt;
use super::crypto::compute_sha3_512;
use super::crypto::generate_aes_key;
use super::crypto::p256_ecdh_encrypt;
use super::crypto::p256_ecdsa_verify;
use super::crypto::rsa_oaep_encrypt;
use super::crypto::rsa_verify_sha3_512;
use super::piv::PivCard;
use crate::error::Result;
use crate::error::SlideError;
use crate::model::SlideDeck;
use serde::Deserialize;
use serde::Serialize;
use std::io::Read as _;
use std::path::Path;
use std::sync::Arc;
use zeroize::Zeroize;

/// Container magic bytes identifying an encrypted cargo-slide presentation.
pub const ENCRYPTED_CONTAINER_MAGIC: &[u8] = b"SLIDECRYPT\x01";
const PAYLOAD_DELIMITER: &[u8] = b"\n---ENCRYPTED-PAYLOAD---\n";

/// Header metadata stored within an encrypted slide package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedContainerHeader {
    pub format_version: u32,
    pub cipher: String,
    #[serde(default)]
    pub auth_mode: super::passphrase::AuthMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_keystore: Option<super::passphrase::EncryptedKeystore>,
    pub recipient_slot: PivSlot,
    pub encrypted_session_key: String, // base64
    pub nonce: String,                 // base64 (12 bytes)
    pub tag: String,                   // base64 (16 bytes)
    pub signer_slot: PivSlot,
    pub signer_cert: Option<String>, // base64 DER
    pub integrity_algorithm: String, // "SHA3-512"
    pub sha3_512_digest: String,     // hex (128 chars)
    pub signature: String,           // base64
    pub title: String,
    pub total_slides: usize,
    pub uncompressed_size: u64,
}

/// Encrypted slide presentation container with built-in integrity and in-memory unpacking.
#[derive(Debug, Clone)]
pub struct EncryptedSlideContainer {
    pub header: EncryptedContainerHeader,
    pub ciphertext: Vec<u8>,
}

impl EncryptedSlideContainer {
    /// Check if a byte slice starts with the encrypted container magic bytes.
    #[must_use]
    pub fn is_encrypted_package(data: &[u8]) -> bool {
        let magic = crate::obf_bytes!(b"SLIDECRYPT\x01");
        data.starts_with(&magic)
    }

    /// Pack and encrypt a presentation deck and its assets into an encrypted `.slide` byte buffer using hardware PIV.
    pub fn pack(
        deck: &SlideDeck,
        assets_dir: Option<&Path>,
        recipient_pub_key: &PivPublicKey,
        recipient_slot: PivSlot,
        signer_card: &mut dyn PivCard,
        signer_slot: PivSlot,
    ) -> Result<Vec<u8>> {
        Self::pack_with_source(
            deck,
            assets_dir,
            recipient_pub_key,
            recipient_slot,
            signer_card,
            signer_slot,
            true,
        )
    }

    /// Pack and encrypt a presentation deck and its assets into an encrypted `.slide` byte buffer using hardware PIV with explicit `include_source` control.
    pub fn pack_with_source(
        deck: &SlideDeck,
        assets_dir: Option<&Path>,
        recipient_pub_key: &PivPublicKey,
        recipient_slot: PivSlot,
        signer_card: &mut dyn PivCard,
        signer_slot: PivSlot,
        include_source: bool,
    ) -> Result<Vec<u8>> {
        Self::pack_internal(
            deck,
            assets_dir,
            recipient_pub_key,
            recipient_slot,
            signer_card,
            signer_slot,
            super::passphrase::AuthMode::Piv,
            None,
            include_source,
        )
    }

    /// Pack and encrypt presentation using software dual-key keystore protected by passphrase or mnemonic.
    /// Supports specifying 12 or 24 words for BIP-39 mnemonic generation.
    /// Returns `(package_bytes, generated_mnemonic_if_any)`.
    pub fn pack_software(
        deck: &SlideDeck,
        assets_dir: Option<&Path>,
        secret: &str,
        auth_mode: super::passphrase::AuthMode,
        mnemonic_words: Option<usize>,
    ) -> Result<(Vec<u8>, Option<zeroize::Zeroizing<String>>)> {
        Self::pack_software_with_source(deck, assets_dir, secret, auth_mode, mnemonic_words, true)
    }

    /// Pack and encrypt presentation using software dual-key keystore protected by passphrase or mnemonic with explicit `include_source` control.
    pub fn pack_software_with_source(
        deck: &SlideDeck,
        assets_dir: Option<&Path>,
        secret: &str,
        auth_mode: super::passphrase::AuthMode,
        mnemonic_words: Option<usize>,
        include_source: bool,
    ) -> Result<(Vec<u8>, Option<zeroize::Zeroizing<String>>)> {
        let (actual_secret, generated_mnemonic) = if auth_mode
            == super::passphrase::AuthMode::Mnemonic
            && (secret.trim().is_empty() || secret.trim() == "12" || secret.trim() == "24")
        {
            let count = if secret.trim() == "24" {
                24
            } else if secret.trim() == "12" {
                12
            } else {
                mnemonic_words.unwrap_or(12)
            };
            if count != 12 && count != 24 {
                return Err(SlideError::Crypto(format!(
                    "Invalid BIP-39 mnemonic word count: {count}. Only 12 or 24 words are supported."
                )));
            }
            let m = super::passphrase::generate_bip39_mnemonic(count)?;
            (m.clone(), Some(m))
        } else {
            (zeroize::Zeroizing::new(secret.to_string()), None)
        };

        let keystore = super::passphrase::SoftwareKeystore::generate(auth_mode);
        let encrypted_keystore = keystore.encrypt(&actual_secret)?;
        let recipient_pub = keystore.recipient_public_key();
        let mut card: Box<dyn PivCard> = Box::new(keystore);

        let pkg = Self::pack_internal(
            deck,
            assets_dir,
            &recipient_pub,
            PivSlot::Slot9D,
            card.as_mut(),
            PivSlot::Slot9C,
            auth_mode,
            Some(encrypted_keystore),
            include_source,
        )?;

        Ok((pkg, generated_mnemonic))
    }

    #[allow(clippy::too_many_arguments)]
    fn pack_internal(
        deck: &SlideDeck,
        assets_dir: Option<&Path>,
        recipient_pub_key: &PivPublicKey,
        recipient_slot: PivSlot,
        signer_card: &mut dyn PivCard,
        signer_slot: PivSlot,
        auth_mode: super::passphrase::AuthMode,
        encrypted_keystore: Option<super::passphrase::EncryptedKeystore>,
        include_source: bool,
    ) -> Result<Vec<u8>> {
        use base64::Engine as _;
        let b64 = &base64::engine::general_purpose::STANDARD;

        // 1. Build uncompressed TAR archive in memory containing deck.json, manifest, and assets
        let mut tar_builder = tar::Builder::new(Vec::new());

        let deck_json = serde_json::to_vec_pretty(deck)
            .map_err(|e| SlideError::Crypto(format!("Failed to serialize deck: {e}")))?;

        let mut deck_header = tar::Header::new_gnu();
        deck_header.set_size(deck_json.len() as u64);
        deck_header.set_mode(0o644);
        deck_header.set_cksum();
        tar_builder.append_data(&mut deck_header, "deck.json", &deck_json[..])?;

        // Determine manifest metadata to preserve package structure and entrypoints
        let manifest_bytes = if let Some(base) = assets_dir
            && let Ok(existing_manifest) = std::fs::read(base.join("manifest.json"))
        {
            if !include_source {
                if let Ok(mut meta) = serde_json::from_slice::<crate::package::SlidePackageMetadata>(
                    &existing_manifest,
                ) {
                    meta.is_editable = false;
                    meta.entrypoint = None;
                    serde_json::to_vec_pretty(&meta).unwrap_or(existing_manifest)
                } else {
                    existing_manifest
                }
            } else {
                if let Ok(mut meta) = serde_json::from_slice::<crate::package::SlidePackageMetadata>(
                    &existing_manifest,
                ) {
                    meta.is_editable = true;
                    if meta.entrypoint.is_none() {
                        let candidate_entry = if base.join("slides.typ").is_file() {
                            Some("slides.typ".to_string())
                        } else if base.join("source.typ").is_file() {
                            Some("source.typ".to_string())
                        } else if base.join("presentation.typ").is_file() {
                            Some("presentation.typ".to_string())
                        } else {
                            None
                        };
                        meta.entrypoint = candidate_entry;
                    }
                    serde_json::to_vec_pretty(&meta).unwrap_or(existing_manifest)
                } else {
                    existing_manifest
                }
            }
        } else {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs().to_string())
                .unwrap_or_default();
            let entrypoint = if include_source && let Some(base) = assets_dir {
                if base.join("slides.typ").is_file() {
                    Some("slides.typ".to_string())
                } else if base.join("source.typ").is_file() {
                    Some("source.typ".to_string())
                } else if base.join("presentation.typ").is_file() {
                    Some("presentation.typ".to_string())
                } else {
                    None
                }
            } else {
                None
            };
            let is_editable = entrypoint.is_some() && include_source;
            let meta = crate::package::SlidePackageMetadata {
                format_version: 1,
                title: deck.title.clone(),
                total_slides: deck.total_slides(),
                default_animation: deck.default_animation.clone(),
                compression: "lzma2-max".to_string(),
                generator: format!("cargo-slide {}", env!("CARGO_PKG_VERSION")),
                is_editable,
                entrypoint,
                created_at: Some(timestamp),
                author: None,
                aspect_ratio: Some("16:9".to_string()),
                has_notes: deck.has_any_notes(),
                tags: vec!["encrypted".to_string()],
                checksum: None,
                uncompressed_size: Some(deck_json.len() as u64),
                default_theme: deck
                    .default_theme
                    .clone()
                    .or_else(|| deck.extract_default_theme()),
            };
            serde_json::to_vec_pretty(&meta).unwrap_or_default()
        };

        if !manifest_bytes.is_empty() {
            let mut meta_header = tar::Header::new_gnu();
            meta_header.set_size(manifest_bytes.len() as u64);
            meta_header.set_mode(0o644);
            meta_header.set_cksum();
            tar_builder.append_data(&mut meta_header, "manifest.json", &manifest_bytes[..])?;
        }

        // Bundle assets from directory if provided (matching package.rs asset rules)
        if let Some(raw_base) = assets_dir {
            let base = if raw_base.as_os_str().is_empty() {
                Path::new(".")
            } else {
                raw_base
            };

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
            const ASSET_EXTS: &[&str] = &[
                "png", "jpg", "jpeg", "svg", "webp", "gif", "bmp", "ico", "tiff", "mp3", "mp4",
                "wav", "ogg", "webm", "m4a", "mov", "csv", "json", "toml", "yaml", "yml", "txt",
                "sqlite", "db", "tsv", "pdf", "ttf", "otf", "woff", "woff2", "typ",
            ];

            let mut added_paths = std::collections::HashSet::new();

            // 1. Bundle standard asset subdirectories
            for dir_name in ASSET_DIRS {
                let folder = base.join(dir_name);
                if folder.is_dir() {
                    for entry in walkdir::WalkDir::new(&folder)
                        .into_iter()
                        .filter_entry(|e| {
                            let n = e.file_name().to_string_lossy();
                            n != "target" && n != ".git" && n != ".build_tmp" && n != ".slide_cache"
                        })
                        .flatten()
                    {
                        if entry.file_type().is_file() {
                            let file_path = entry.path();
                            if !include_source
                                && file_path
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .is_some_and(|e| e.eq_ignore_ascii_case("typ"))
                            {
                                continue;
                            }
                            if let Ok(rel) = file_path.strip_prefix(base) {
                                let rel_str = rel.to_string_lossy().replace('\\', "/");
                                if added_paths.insert(rel_str.clone())
                                    && let Ok(data) = std::fs::read(file_path)
                                {
                                    let mut header = tar::Header::new_gnu();
                                    header.set_size(data.len() as u64);
                                    header.set_mode(0o644);
                                    header.set_cksum();
                                    let _ =
                                        tar_builder.append_data(&mut header, rel_str, &data[..]);
                                }
                            }
                        }
                    }
                }
            }

            // 2. Bundle top-level asset files (excluding binary executables and caches)
            if let Ok(entries) = std::fs::read_dir(base) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file()
                        && let Some(ext) = path.extension().and_then(|e| e.to_str())
                        && (include_source || !ext.eq_ignore_ascii_case("typ"))
                        && ASSET_EXTS.iter().any(|&e| e.eq_ignore_ascii_case(ext))
                        && let Some(name) = path.file_name().and_then(|s| s.to_str())
                        && name != "deck.json"
                        && name != "manifest.json"
                        && !name.ends_with(".slide")
                        && !name.ends_with(".tmp")
                        && added_paths.insert(name.to_string())
                        && let Ok(data) = std::fs::read(&path)
                    {
                        let mut header = tar::Header::new_gnu();
                        header.set_size(data.len() as u64);
                        header.set_mode(0o644);
                        header.set_cksum();
                        let _ = tar_builder.append_data(&mut header, name, &data[..]);
                    }
                }
            }
        }

        let uncompressed_tar = tar_builder.into_inner()?;
        let uncompressed_size = uncompressed_tar.len() as u64;

        // Compress TAR archive using LZMA2/XZ container at maximum compression ratio (Level 9 + Extreme preset)
        const LZMA_PRESET_EXTREME: u32 = 1 << 31;
        let mut compressed_tar = Vec::new();
        {
            let mut encoder = xz2::write::XzEncoder::new_stream(
                &mut compressed_tar,
                xz2::stream::Stream::new_easy_encoder(
                    9 | LZMA_PRESET_EXTREME,
                    xz2::stream::Check::Crc64,
                )
                .map_err(|e| {
                    SlideError::Crypto(format!("Failed to initialize LZMA2 encoder: {e}"))
                })?,
            );
            use std::io::Write as _;
            encoder.write_all(&uncompressed_tar)?;
            encoder.finish()?;
        }

        // 2. Generate random 256-bit AES key with memory zeroization on drop
        let aes_key = generate_aes_key();

        // 3. Encrypt payload with AES-256-GCM
        let (ciphertext, nonce, tag) = aes_256_gcm_encrypt(&aes_key, &compressed_tar)?;

        // 4. Compute SHA3-512 hash over the ciphertext
        let sha3_digest = compute_sha3_512(&ciphertext);
        let mut sha3_hex = String::with_capacity(128);
        for b in sha3_digest {
            use std::fmt::Write as _;
            let _ = write!(sha3_hex, "{b:02x}");
        }

        // 5. Sign SHA3-512 digest using the private key on the signer's PIV slot
        let signature_bytes = signer_card.sign(signer_slot, &sha3_digest)?;
        let signature_b64 = b64.encode(&signature_bytes);

        // 6. Encrypt AES-256 session key using recipient PIV slot public key
        let encrypted_key_bytes = match recipient_pub_key {
            | PivPublicKey::Rsa(rsa_key) => rsa_oaep_encrypt(rsa_key, aes_key.as_ref())?,
            | PivPublicKey::Ecc(ecc_key) => p256_ecdh_encrypt(ecc_key, &aes_key)?,
        };
        let encrypted_key_b64 = b64.encode(&encrypted_key_bytes);

        // Retrieve signer certificate if available to embed in header
        let signer_cert_b64 = signer_card
            .get_certificate(signer_slot)
            .ok()
            .map(|cert| b64.encode(cert));

        // 7. Assemble header
        let header = EncryptedContainerHeader {
            format_version: 1,
            cipher: "AES-256-GCM".to_string(),
            auth_mode,
            encrypted_keystore,
            recipient_slot,
            encrypted_session_key: encrypted_key_b64,
            nonce: b64.encode(nonce),
            tag: b64.encode(tag),
            signer_slot,
            signer_cert: signer_cert_b64,
            integrity_algorithm: "SHA3-512".to_string(),
            sha3_512_digest: sha3_hex,
            signature: signature_b64,
            title: deck.title.clone(),
            total_slides: deck.total_slides(),
            uncompressed_size,
        };

        let header_json = serde_json::to_vec_pretty(&header)
            .map_err(|e| SlideError::Crypto(format!("Failed to serialize header: {e}")))?;

        // 8. Assemble final container bytes: MAGIC + HEADER_JSON + DELIMITER + CIPHERTEXT
        let mut output = Vec::with_capacity(
            ENCRYPTED_CONTAINER_MAGIC.len()
                + header_json.len()
                + PAYLOAD_DELIMITER.len()
                + ciphertext.len(),
        );
        output.extend_from_slice(ENCRYPTED_CONTAINER_MAGIC);
        output.extend_from_slice(&header_json);
        output.extend_from_slice(PAYLOAD_DELIMITER);
        output.extend_from_slice(&ciphertext);

        Ok(output)
    }

    /// Parse an encrypted slide container from raw file or memory bytes.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if !Self::is_encrypted_package(bytes) {
            return Err(SlideError::Crypto(
                "Provided data does not start with encrypted slide magic header".to_string(),
            ));
        }

        let after_magic = bytes
            .get(ENCRYPTED_CONTAINER_MAGIC.len()..)
            .ok_or_else(|| {
                SlideError::Crypto("Corrupted encrypted package: truncated magic".to_string())
            })?;

        // Locate PAYLOAD_DELIMITER with zeroized stream matching
        let delim = crate::obf_bytes!(b"\n---ENCRYPTED-PAYLOAD---\n");
        let delim_pos = after_magic
            .windows(delim.len())
            .position(|w| w == &delim[..])
            .ok_or_else(|| {
                SlideError::Crypto(
                    "Corrupted encrypted package: payload delimiter not found".to_string(),
                )
            })?;

        let header_bytes = after_magic
            .get(..delim_pos)
            .ok_or_else(|| SlideError::Crypto("Truncated package header".to_string()))?;
        let ciphertext = after_magic
            .get(delim_pos.saturating_add(delim.len())..)
            .ok_or_else(|| SlideError::Crypto("Truncated package ciphertext".to_string()))?
            .to_vec();

        let header: EncryptedContainerHeader = serde_json::from_slice(header_bytes)
            .map_err(|e| SlideError::Crypto(format!("Invalid encrypted package header: {e}")))?;

        Ok(Self { header, ciphertext })
    }

    /// Verify SHA3-512 integrity hash and PIV cryptographic signature.
    /// Aborts immediately with `SlideError::IntegrityCheckFailed` if any tampering is detected!
    pub fn verify_integrity(
        &self,
        signer_pub_key_override: Option<&PivPublicKey>,
    ) -> Result<()> {
        use base64::Engine as _;
        let b64 = &base64::engine::general_purpose::STANDARD;

        // 1. Recalculate SHA3-512 over the ciphertext
        let computed_digest = compute_sha3_512(&self.ciphertext);
        let mut computed_hex = String::with_capacity(128);
        for b in computed_digest {
            use std::fmt::Write as _;
            let _ = write!(computed_hex, "{b:02x}");
        }

        if !computed_hex.eq_ignore_ascii_case(&self.header.sha3_512_digest) {
            return Err(SlideError::IntegrityCheckFailed(format!(
                "SHA3-512 hash mismatch: expected {}, got {}",
                self.header.sha3_512_digest, computed_hex
            )));
        }

        // 2. Decode signature bytes
        let signature_bytes = b64.decode(&self.header.signature).map_err(|e| {
            SlideError::IntegrityCheckFailed(format!("Invalid signature base64: {e}"))
        })?;

        // 3. Resolve signer public key
        let signer_key = if let Some(k) = signer_pub_key_override {
            k.clone()
        } else if let Some(ref cert_b64) = self.header.signer_cert {
            eprintln!(
                "[SECURITY NOTICE] Verifying container signature using embedded certificate from package header without external root-of-trust pinning."
            );
            let cert_der = b64.decode(cert_b64).map_err(|e| {
                SlideError::IntegrityCheckFailed(format!("Invalid cert base64: {e}"))
            })?;
            parse_certificate_public_key(&cert_der)?
        } else {
            return Err(SlideError::IntegrityCheckFailed(
                "No signer public key or certificate available to verify SHA3-512 signature"
                    .to_string(),
            ));
        };

        // 4. Verify signature against SHA3-512 digest
        match signer_key {
            | PivPublicKey::Rsa(ref rsa_key) => {
                rsa_verify_sha3_512(rsa_key, &computed_digest, &signature_bytes)?;
            },
            | PivPublicKey::Ecc(ref ecc_key) => {
                p256_ecdsa_verify(ecc_key, &computed_digest, &signature_bytes)?;
            },
        }

        Ok(())
    }

    /// Decrypt the presentation deck and all assets directly into memory with zeroization on drop.
    /// Assets are loaded into a `MemoryAssetStore` and never written to physical disk!
    pub fn decrypt_in_memory(
        &self,
        recipient_card: &mut dyn PivCard,
        signer_pub_key_override: Option<&PivPublicKey>,
    ) -> Result<(SlideDeck, Arc<MemoryAssetStore>)> {
        use base64::Engine as _;
        let b64 = &base64::engine::general_purpose::STANDARD;

        // Step 0: Activate OS anti-dump protection, core dump suppression, and attach denial
        let _ = super::anti_dump::enable_anti_dump_protection();
        super::anti_dump::verify_environment_integrity()?;

        // Step 1: Enforce strict SHA3-512 integrity verification first
        self.verify_integrity(signer_pub_key_override)?;

        // Step 2: Decode encrypted session key, nonce, and tag
        let encrypted_session_key_bytes = b64
            .decode(&self.header.encrypted_session_key)
            .map_err(|e| SlideError::Crypto(format!("Invalid session key base64: {e}")))?;

        let nonce_vec = b64
            .decode(&self.header.nonce)
            .map_err(|e| SlideError::Crypto(format!("Invalid nonce base64: {e}")))?;
        if nonce_vec.len() != 12 {
            return Err(SlideError::Crypto(
                "Nonce must be exactly 12 bytes".to_string(),
            ));
        }
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&nonce_vec);

        let tag_vec = b64
            .decode(&self.header.tag)
            .map_err(|e| SlideError::Crypto(format!("Invalid tag base64: {e}")))?;
        if tag_vec.len() != 16 {
            return Err(SlideError::Crypto(
                "Tag must be exactly 16 bytes".to_string(),
            ));
        }
        let mut tag = [0u8; 16];
        tag.copy_from_slice(&tag_vec);

        // Step 3: Decrypt AES-256 session key using the recipient's PIV slot
        let decrypted_key_bytes =
            recipient_card.decrypt(self.header.recipient_slot, &encrypted_session_key_bytes)?;

        if decrypted_key_bytes.len() != 32 {
            return Err(SlideError::Crypto(format!(
                "Decrypted session key length must be 32 bytes, got {}",
                decrypted_key_bytes.len()
            )));
        }
        let mut aes_key = [0u8; 32];
        aes_key.copy_from_slice(&decrypted_key_bytes);

        // Step 4: Decrypt payload using AES-256-GCM into zeroized memory
        let mut decrypted_compressed_tar =
            aes_256_gcm_decrypt(&aes_key, &nonce, &tag, &self.ciphertext)?;
        aes_key.zeroize();

        // Step 5: Decompress XZ / LZMA2 container into uncompressed TAR bytes in memory
        let mut uncompressed_tar = Vec::new();
        {
            let mut decompressor = xz2::read::XzDecoder::new(&decrypted_compressed_tar[..]);
            let mut limited = std::io::Read::take(
                &mut decompressor,
                crate::package::MAX_DECOMPRESSED_FILE_BYTES.saturating_mul(4),
            );
            limited
                .read_to_end(&mut uncompressed_tar)
                .map_err(|e| SlideError::Crypto(format!("Failed to decompress archive: {e}")))?;
        }
        decrypted_compressed_tar.zeroize();

        // Lock decrypted memory pages into RAM to prevent swapping to disk
        let _ = super::anti_dump::mlock_slice(&uncompressed_tar);

        // Step 6: Unpack TAR entries purely in memory into SlideDeck and MemoryAssetStore
        let mut tar_archive = tar::Archive::new(&uncompressed_tar[..]);
        let asset_store = Arc::new(MemoryAssetStore::new());
        let mut deck_json = None;

        for entry in tar_archive.entries()? {
            let mut entry = entry?;
            let path_str = entry.path()?.to_string_lossy().to_string();
            let mut content = Vec::new();
            entry.read_to_end(&mut content)?;

            if path_str == "deck.json" || path_str.ends_with("/deck.json") {
                deck_json = Some(content);
            } else {
                asset_store.insert(path_str, content);
            }
        }

        // Memory unlock and zeroization for the uncompressed tar buffer
        super::anti_dump::munlock_slice(&uncompressed_tar);
        uncompressed_tar.zeroize();

        let Some(mut deck_bytes) = deck_json else {
            return Err(SlideError::Crypto(
                "No deck.json found inside encrypted presentation archive".to_string(),
            ));
        };

        let mut deck: SlideDeck = serde_json::from_slice(&deck_bytes)
            .map_err(|e| SlideError::Crypto(format!("Failed to parse decrypted SlideDeck: {e}")))?;
        deck_bytes.zeroize();

        // Inline images directly into slide SVGs so resvg can render them purely in RAM
        for slide in &mut deck.slides {
            slide.svg_data = asset_store.inline_svg_images(&slide.svg_data);
        }

        Ok((deck, asset_store))
    }

    /// Decrypt the presentation deck using a software passphrase or recovery mnemonic.
    pub fn decrypt_with_secret(
        &self,
        secret: &str,
    ) -> Result<(SlideDeck, Arc<MemoryAssetStore>)> {
        let Some(ref enc_keystore) = self.header.encrypted_keystore else {
            return Err(SlideError::Crypto(
                "This presentation was encrypted with hardware YubiKey PIV mode and requires a physical token.".to_string(),
            ));
        };

        let mut keystore = super::passphrase::SoftwareKeystore::decrypt(enc_keystore, secret)?;
        let signer_pub = keystore.signer_public_key();
        self.decrypt_in_memory(&mut keystore, Some(&signer_pub))
    }
}
