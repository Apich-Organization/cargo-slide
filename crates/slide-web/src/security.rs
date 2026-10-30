//! Web client-side presentation security and token authentication engine.
//! Decrypts protected presentations in pure Rust WebAssembly and manages session tokens.

use crate::bip39_words::BIP39_WORDLIST;
use crate::model::SlideDeck;
use aes_gcm::Aes256Gcm;
use aes_gcm::Nonce;
use aes_gcm::aead::Aead;
use aes_gcm::aead::KeyInit;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest as _;
use sha2::Sha256;
use sha2::Sha512;
use zeroize::Zeroize;
use zeroize::Zeroizing;

/// JSON envelope for protected web presentations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedDeckEnvelope {
    /// Marker indicating that the deck is protected
    pub cargo_slide_protected: bool,
    /// Authentication mode: "passphrase" or "mnemonic"
    pub auth_mode: String,
    /// Token lifetime in seconds
    pub token_ttl_seconds: u64,
    /// PBKDF2 iterations used during encryption
    pub iterations: u32,
    /// Base64 encoded 32-byte salt
    pub salt: String,
    /// Base64 encoded 12-byte AES-GCM nonce
    pub nonce: String,
    /// Base64 encoded AES-256-GCM ciphertext of deck.json
    pub ciphertext: String,
    /// Optional presentation title for the auth modal
    #[serde(default)]
    pub title: Option<String>,
}

/// Key storage keys in browser Web Storage
const TOKEN_KEY: &str = "cargo_slide_web_token";
const EXPIRY_KEY: &str = "cargo_slide_web_expiry";

/// Retrieve non-expired token from sessionStorage or localStorage if available
pub fn get_saved_token() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.session_storage().ok()??;
    let token = storage.get_item(TOKEN_KEY).ok()??;
    let expiry_str = storage.get_item(EXPIRY_KEY).ok()??;
    let expiry: f64 = expiry_str.parse().ok()?;

    let now_sec = js_sys::Date::now() / 1000.0;
    if now_sec < expiry {
        Some(token)
    } else {
        let _ = storage.remove_item(TOKEN_KEY);
        let _ = storage.remove_item(EXPIRY_KEY);
        None
    }
}

/// Save token with TTL into sessionStorage
pub fn save_token(
    token: &str,
    ttl_seconds: u64,
) {
    if let Some(window) = web_sys::window()
        && let Ok(Some(storage)) = window.session_storage()
    {
        let expiry = (js_sys::Date::now() / 1000.0) + (ttl_seconds as f64);
        let _ = storage.set_item(TOKEN_KEY, token);
        let _ = storage.set_item(EXPIRY_KEY, &expiry.to_string());
    }
}

/// Clear saved token from sessionStorage
pub fn clear_token() {
    if let Some(window) = web_sys::window()
        && let Ok(Some(storage)) = window.session_storage()
    {
        let _ = storage.remove_item(TOKEN_KEY);
        let _ = storage.remove_item(EXPIRY_KEY);
    }
}

/// Validate a BIP-39 mnemonic and extract entropy bytes.
pub fn validate_bip39_mnemonic(mnemonic: &str) -> Result<Zeroizing<Vec<u8>>, String> {
    let words: Vec<&str> = mnemonic.split_whitespace().collect();
    let word_count = words.len();

    let entropy_bytes_len = match word_count {
        | 12 => 16,
        | 24 => 32,
        | other => {
            return Err(format!(
                "Invalid BIP-39 mnemonic length: {other} words provided. Expected 12 or 24 words."
            ));
        },
    };

    let checksum_bits_len = entropy_bytes_len / 4;
    let mut bits = Vec::with_capacity(word_count * 11);

    for w in words {
        let lower = w.to_ascii_lowercase();
        let index = BIP39_WORDLIST
            .binary_search(&lower.as_str())
            .map_err(|_| format!("Word '{lower}' is not a valid BIP-39 dictionary word."))?;

        for bit_idx in (0..11).rev() {
            bits.push(((index >> bit_idx) & 1) as u8);
        }
    }

    let entropy_bits_len = entropy_bytes_len * 8;
    let mut entropy = vec![0u8; entropy_bytes_len];

    for (i, byte) in entropy.iter_mut().enumerate() {
        let mut val = 0u8;
        for bit_idx in 0..8 {
            let overall_bit = i * 8 + bit_idx;
            if let Some(&b) = bits.get(overall_bit) {
                val = (val << 1) | b;
            }
        }
        *byte = val;
    }

    // Verify checksum
    let hash = Sha256::digest(&entropy);
    let first_byte = hash.first().copied().unwrap_or(0);

    for bit_idx in 0..checksum_bits_len {
        let expected_bit = (first_byte >> (7 - bit_idx)) & 1;
        let actual_bit = bits.get(entropy_bits_len + bit_idx).copied().unwrap_or(0);
        if expected_bit != actual_bit {
            return Err("BIP-39 mnemonic checksum validation failed.".to_string());
        }
    }

    Ok(Zeroizing::new(entropy))
}

/// Derive 256-bit AES key from passphrase
pub fn derive_key_from_passphrase(
    passphrase: &str,
    salt: &[u8; 32],
    iterations: u32,
) -> Result<Zeroizing<[u8; 32]>, String> {
    if !(10_000..=5_000_000).contains(&iterations) {
        return Err(format!(
            "Invalid KDF iteration count: {iterations} (must be between 10,000 and 5,000,000)"
        ));
    }
    let mut intermediate = [0u8; 64];
    pbkdf2::pbkdf2_hmac::<Sha512>(passphrase.as_bytes(), salt, iterations, &mut intermediate);

    let hk = hkdf::Hkdf::<Sha256>::new(Some(salt), &intermediate);
    let mut aes_key = [0u8; 32];
    hk.expand(b"cargo-slide-passphrase-aes256", &mut aes_key)
        .map_err(|e| format!("HKDF expansion failed: {e}"))?;

    intermediate.zeroize();
    Ok(Zeroizing::new(aes_key))
}

/// Derive 256-bit AES key from BIP-39 mnemonic
pub fn derive_key_from_mnemonic(
    mnemonic: &str,
    salt: &[u8; 32],
    iterations: u32,
) -> Result<Zeroizing<[u8; 32]>, String> {
    if !(10_000..=5_000_000).contains(&iterations) {
        return Err(format!(
            "Invalid KDF iteration count: {iterations} (must be between 10,000 and 5,000,000)"
        ));
    }
    let entropy = validate_bip39_mnemonic(mnemonic)?;
    let mut intermediate = [0u8; 64];
    pbkdf2::pbkdf2_hmac::<Sha512>(&entropy, salt, iterations, &mut intermediate);

    let hk = hkdf::Hkdf::<Sha256>::new(Some(salt), &intermediate);
    let mut aes_key = [0u8; 32];
    hk.expand(b"cargo-slide-mnemonic-aes256", &mut aes_key)
        .map_err(|e| format!("HKDF expansion failed: {e}"))?;

    intermediate.zeroize();
    Ok(Zeroizing::new(aes_key))
}

/// Decrypt ciphertext with derived AES key and parse into SlideDeck
pub fn decrypt_deck(
    key: &[u8; 32],
    nonce_bytes: &[u8],
    ciphertext_bytes: &[u8],
) -> Result<SlideDeck, String> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| format!("Cipher initialization failed: {e}"))?;
    let nonce = Nonce::try_from(nonce_bytes).map_err(|e| format!("Invalid nonce format: {e}"))?;
    let plaintext = cipher
        .decrypt(&nonce, ciphertext_bytes)
        .map_err(|_| "Decryption failed. Incorrect passphrase or mnemonic phrase.".to_string())?;

    let deck: SlideDeck = serde_json::from_slice(&plaintext)
        .map_err(|e| format!("Corrupted presentation deck JSON: {e}"))?;
    Ok(deck)
}
