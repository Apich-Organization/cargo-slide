//! Software-based encryption mode using dual-key keystores (P-256 ECDH recipient key +
//! P-256 ECDSA signer key) protected with AES-256-GCM and PBKDF2-HMAC-SHA512 key derivation.
//! Supports both enforced-strength passphrases and BIP-39 12/24-word recovery mnemonics.

use super::bip39_words::BIP39_WORDLIST;
use super::cert::PivPublicKey;
use super::cert::PivSlot;
use super::cert::derive_self_signed_piv_certificate;
use super::crypto::aes_256_gcm_decrypt;
use super::crypto::aes_256_gcm_encrypt;
use super::crypto::p256_ecdh_decrypt;
use super::crypto::p256_ecdsa_sign;
use super::piv::PivCard;
use crate::error::Result;
use crate::error::SlideError;
use p256::SecretKey;
use p256::elliptic_curve::Generate;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest as _;
use sha2::Sha256;
use sha2::Sha512;
use zeroize::Zeroize;
use zeroize::Zeroizing;

/// Authentication mode for slide package encryption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    /// Hardware token authentication via YubiKey PIV smartcard.
    #[default]
    Piv,
    /// Software dual-key keystore protected by an enforced-strength passphrase.
    Passphrase,
    /// Software dual-key keystore protected by a BIP-39 recovery mnemonic.
    Mnemonic,
}

impl std::fmt::Display for AuthMode {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            | Self::Piv => write!(f, "piv"),
            | Self::Passphrase => write!(f, "passphrase"),
            | Self::Mnemonic => write!(f, "mnemonic"),
        }
    }
}

impl std::str::FromStr for AuthMode {
    type Err = SlideError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            | "piv" | "yubikey" | "smartcard" | "hardware" => Ok(Self::Piv),
            | "passphrase" | "password" | "pass" => Ok(Self::Passphrase),
            | "mnemonic" | "bip39" | "words" => Ok(Self::Mnemonic),
            | other => {
                Err(SlideError::Crypto(format!(
                    "Invalid auth mode '{other}'. Supported modes: 'piv', 'passphrase', 'mnemonic'"
                )))
            },
        }
    }
}

/// Minimum required passphrase length.
pub const MIN_PASSPHRASE_LENGTH: usize = 10;
/// Default iterations for PBKDF2-HMAC-SHA512.
pub const PBKDF2_RECOMMENDED_ITERATIONS: u32 = 600_000;

/// Enforce strong passphrase requirements:
/// - Minimum 10 characters
/// - At least 3 of 4 character classes (uppercase, lowercase, numbers, symbols)
/// - Reject common trivial sequences and repeat patterns
pub fn validate_password_strength(password: &str) -> Result<()> {
    if password.len() < MIN_PASSPHRASE_LENGTH {
        return Err(SlideError::Crypto(format!(
            "Passphrase is too short: {} characters provided, minimum {} required.",
            password.len(),
            MIN_PASSPHRASE_LENGTH
        )));
    }

    let mut has_lower = false;
    let mut has_upper = false;
    let mut has_digit = false;
    let mut has_symbol = false;

    for ch in password.chars() {
        if ch.is_ascii_lowercase() {
            has_lower = true;
        } else if ch.is_ascii_uppercase() {
            has_upper = true;
        } else if ch.is_ascii_digit() {
            has_digit = true;
        } else if ch.is_ascii_punctuation() || !ch.is_ascii_alphanumeric() {
            has_symbol = true;
        }
    }

    let class_count = usize::from(has_lower)
        .saturating_add(usize::from(has_upper))
        .saturating_add(usize::from(has_digit))
        .saturating_add(usize::from(has_symbol));

    if class_count < 3 {
        return Err(SlideError::Crypto(
            "Passphrase does not meet complexity requirements. It must include at least 3 of the following 4 categories: \
             uppercase letters (A-Z), lowercase letters (a-z), digits (0-9), and symbols/punctuation."
                .to_string(),
        ));
    }

    // Check against common trivial sequences
    let lower = password.to_ascii_lowercase();
    const TRIVIAL_PATTERNS: &[&str] = &[
        "password",
        "12345678",
        "qwerty",
        "admin123",
        "cargo-slide",
        "letmein",
        "welcome1",
    ];

    for pat in TRIVIAL_PATTERNS {
        if lower.contains(pat) {
            return Err(SlideError::Crypto(format!(
                "Passphrase contains a common or trivial pattern ('{pat}'). Please choose a stronger passphrase."
            )));
        }
    }

    // Reject repetition of identical characters for >= 4 consecutive characters (case-insensitive)
    let lower_chars: Vec<char> = lower.chars().collect();
    if lower_chars
        .windows(4)
        .any(|w| w[0] == w[1] && w[1] == w[2] && w[2] == w[3])
    {
        return Err(SlideError::Crypto(
            "Passphrase contains excessive character repetitions. Please choose a more varied passphrase."
                .to_string(),
        ));
    }

    Ok(())
}

/// Generate a cryptographically secure BIP-39 mnemonic (12 or 24 words).
pub fn generate_bip39_mnemonic(word_count: usize) -> Result<Zeroizing<String>> {
    let entropy_bytes_len = match word_count {
        | 12 => 16,
        | 24 => 32,
        | other => {
            return Err(SlideError::Crypto(format!(
                "Unsupported BIP-39 word count {other}. Only 12 or 24 words are supported."
            )));
        },
    };

    let mut entropy = vec![0u8; entropy_bytes_len];
    rand::fill(&mut entropy[..]);

    let mnemonic = entropy_to_mnemonic(&entropy)?;
    entropy.zeroize();
    Ok(mnemonic)
}

/// Convert raw entropy bytes into a BIP-39 mnemonic string.
fn entropy_to_mnemonic(entropy: &[u8]) -> Result<Zeroizing<String>> {
    let entropy_len = entropy.len();
    let checksum_bits_len = entropy_len / 4; // 16 bytes -> 4 bits; 32 bytes -> 8 bits
    let total_bits = (entropy_len * 8).saturating_add(checksum_bits_len);
    let word_count = total_bits / 11;

    let hash = Sha256::digest(entropy);

    // Build bitstream: entropy bits followed by checksum bits
    let mut bits = Vec::with_capacity(total_bits);
    for &b in entropy {
        for bit_idx in (0..8).rev() {
            bits.push((b >> bit_idx) & 1);
        }
    }
    for bit_idx in (0..checksum_bits_len).rev() {
        let first_byte = hash.first().copied().unwrap_or(0);
        bits.push((first_byte >> (7 - (checksum_bits_len - 1 - bit_idx))) & 1);
    }

    let mut words = Vec::with_capacity(word_count);
    for chunk in bits.chunks(11) {
        let mut index = 0usize;
        for &bit in chunk {
            index = (index << 1) | (bit as usize);
        }
        let word = BIP39_WORDLIST
            .get(index)
            .ok_or_else(|| SlideError::Crypto(format!("BIP-39 word index {index} out of range")))?;
        words.push(*word);
    }

    Ok(Zeroizing::new(words.join(" ")))
}

/// Validate a BIP-39 mnemonic phrase and extract the original entropy bytes.
pub fn validate_bip39_mnemonic(mnemonic: &str) -> Result<Zeroizing<Vec<u8>>> {
    let words: Vec<&str> = mnemonic.split_whitespace().collect();
    let word_count = words.len();

    let entropy_bytes_len = match word_count {
        | 12 => 16,
        | 24 => 32,
        | other => {
            return Err(SlideError::Crypto(format!(
                "Invalid BIP-39 mnemonic length: {other} words provided. Expected 12 or 24 words."
            )));
        },
    };

    let checksum_bits_len = entropy_bytes_len / 4;
    let mut bits = Vec::with_capacity(word_count * 11);

    for w in words {
        let lower = w.to_ascii_lowercase();
        // Binary search in sorted BIP-39 English wordlist
        let index = BIP39_WORDLIST.binary_search(&lower.as_str()).map_err(|_| {
            SlideError::Crypto(format!(
                "Word '{lower}' is not a valid BIP-39 dictionary word. Please check spelling."
            ))
        })?;

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
            return Err(SlideError::Crypto(
                "BIP-39 mnemonic checksum validation failed. One or more words may be incorrect or out of order."
                    .to_string(),
            ));
        }
    }

    Ok(Zeroizing::new(entropy))
}

/// Derive a 256-bit AES encryption key from a user passphrase using PBKDF2-HMAC-SHA512 + HKDF-SHA256.
pub fn derive_key_from_passphrase(
    passphrase: &str,
    salt: &[u8; 32],
    iterations: u32,
) -> Result<Zeroizing<[u8; 32]>> {
    let mut intermediate_master = [0u8; 64];
    pbkdf2::pbkdf2_hmac::<Sha512>(
        passphrase.as_bytes(),
        salt,
        iterations,
        &mut intermediate_master,
    );

    let hk = hkdf::Hkdf::<Sha256>::new(Some(salt), &intermediate_master);
    let mut aes_key = [0u8; 32];
    hk.expand(b"cargo-slide-passphrase-aes256", &mut aes_key)
        .map_err(|e| SlideError::Crypto(format!("HKDF key expansion failed: {e}")))?;

    intermediate_master.zeroize();
    Ok(Zeroizing::new(aes_key))
}

/// Derive a 256-bit AES encryption key from a BIP-39 mnemonic phrase.
pub fn derive_key_from_mnemonic(
    mnemonic: &str,
    salt: &[u8; 32],
    iterations: u32,
) -> Result<Zeroizing<[u8; 32]>> {
    // Validate mnemonic integrity first
    let entropy = validate_bip39_mnemonic(mnemonic)?;

    // Derive seed using PBKDF2-HMAC-SHA512 with entropy + salt
    let mut intermediate_master = [0u8; 64];
    pbkdf2::pbkdf2_hmac::<Sha512>(&entropy, salt, iterations, &mut intermediate_master);

    let hk = hkdf::Hkdf::<Sha256>::new(Some(salt), &intermediate_master);
    let mut aes_key = [0u8; 32];
    hk.expand(b"cargo-slide-mnemonic-aes256", &mut aes_key)
        .map_err(|e| SlideError::Crypto(format!("HKDF key expansion failed: {e}")))?;

    intermediate_master.zeroize();
    Ok(Zeroizing::new(aes_key))
}

/// Serialized encrypted software keystore containing recipient and signer private keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedKeystore {
    /// PBKDF2 CSPRNG salt (base64 encoded, 32 bytes)
    pub kdf_salt: String,
    /// PBKDF2 iteration count
    pub kdf_iterations: u32,
    /// AES-256-GCM 96-bit nonce (base64 encoded, 12 bytes)
    pub nonce: String,
    /// AES-256-GCM 128-bit authentication tag (base64 encoded, 16 bytes)
    pub tag: String,
    /// AES-256-GCM ciphertext containing the dual private keys (base64 encoded)
    pub ciphertext: String,
    /// Auth mode: "passphrase" or "mnemonic"
    pub auth_mode: AuthMode,
}

/// Dual-key in-memory software keystore matching PIV slot architecture:
/// - Recipient private key: NIST P-256 SecretKey (ECDH key decapsulation)
/// - Signer private key: NIST P-256 SecretKey (ECDSA SHA3-512 digital signature)
pub struct SoftwareKeystore {
    pub recipient_key: SecretKey,
    pub signer_key: SecretKey,
    pub auth_mode: AuthMode,
}

#[derive(Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
struct PlaintextKeystorePayload {
    recipient_priv_bytes: Vec<u8>,
    signer_priv_bytes: Vec<u8>,
}

impl SoftwareKeystore {
    /// Generate a fresh random dual-key software keystore using OS CSPRNG.
    #[must_use]
    pub fn generate(auth_mode: AuthMode) -> Self {
        let recipient_key = SecretKey::generate();
        let signer_key = SecretKey::generate();
        Self {
            recipient_key,
            signer_key,
            auth_mode,
        }
    }

    /// Encrypt this keystore using a user passphrase or BIP-39 mnemonic.
    pub fn encrypt(
        &self,
        secret: &str,
    ) -> Result<EncryptedKeystore> {
        use base64::Engine as _;
        let b64 = &base64::engine::general_purpose::STANDARD;

        let mut salt = [0u8; 32];
        rand::fill(&mut salt);

        let aes_key = match self.auth_mode {
            | AuthMode::Passphrase => {
                validate_password_strength(secret)?;
                derive_key_from_passphrase(secret, &salt, PBKDF2_RECOMMENDED_ITERATIONS)?
            },
            | AuthMode::Mnemonic => {
                derive_key_from_mnemonic(secret, &salt, PBKDF2_RECOMMENDED_ITERATIONS)?
            },
            | AuthMode::Piv => {
                return Err(SlideError::Crypto(
                    "Cannot encrypt software keystore with PIV auth mode".to_string(),
                ));
            },
        };

        let payload = PlaintextKeystorePayload {
            recipient_priv_bytes: self.recipient_key.to_bytes().to_vec(),
            signer_priv_bytes: self.signer_key.to_bytes().to_vec(),
        };

        let mut plaintext = serde_json::to_vec(&payload).map_err(|e| {
            SlideError::Crypto(format!("Failed to serialize keystore payload: {e}"))
        })?;

        let (ciphertext, nonce, tag) = aes_256_gcm_encrypt(&aes_key, &plaintext)?;
        plaintext.zeroize();

        Ok(EncryptedKeystore {
            kdf_salt: b64.encode(salt),
            kdf_iterations: PBKDF2_RECOMMENDED_ITERATIONS,
            nonce: b64.encode(nonce),
            tag: b64.encode(tag),
            ciphertext: b64.encode(ciphertext),
            auth_mode: self.auth_mode,
        })
    }

    /// Decrypt an encrypted keystore using a user passphrase or BIP-39 mnemonic.
    pub fn decrypt(
        encrypted: &EncryptedKeystore,
        secret: &str,
    ) -> Result<Self> {
        use base64::Engine as _;
        let b64 = &base64::engine::general_purpose::STANDARD;

        let salt_vec = b64
            .decode(&encrypted.kdf_salt)
            .map_err(|e| SlideError::Crypto(format!("Invalid salt base64: {e}")))?;
        if salt_vec.len() != 32 {
            return Err(SlideError::Crypto(
                "Invalid salt length in keystore".to_string(),
            ));
        }
        let mut salt = [0u8; 32];
        salt.copy_from_slice(&salt_vec);

        let nonce_vec = b64
            .decode(&encrypted.nonce)
            .map_err(|e| SlideError::Crypto(format!("Invalid nonce base64: {e}")))?;
        if nonce_vec.len() != 12 {
            return Err(SlideError::Crypto(
                "Invalid nonce length in keystore".to_string(),
            ));
        }
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&nonce_vec);

        let tag_vec = b64
            .decode(&encrypted.tag)
            .map_err(|e| SlideError::Crypto(format!("Invalid tag base64: {e}")))?;
        if tag_vec.len() != 16 {
            return Err(SlideError::Crypto(
                "Invalid tag length in keystore".to_string(),
            ));
        }
        let mut tag = [0u8; 16];
        tag.copy_from_slice(&tag_vec);

        let ciphertext = b64
            .decode(&encrypted.ciphertext)
            .map_err(|e| SlideError::Crypto(format!("Invalid ciphertext base64: {e}")))?;

        if !(10_000..=5_000_000).contains(&encrypted.kdf_iterations) {
            return Err(SlideError::Crypto(format!(
                "Invalid KDF iteration count: {} (must be between 10,000 and 5,000,000)",
                encrypted.kdf_iterations
            )));
        }

        let aes_key = match encrypted.auth_mode {
            | AuthMode::Passphrase => {
                derive_key_from_passphrase(secret, &salt, encrypted.kdf_iterations)?
            },
            | AuthMode::Mnemonic => {
                derive_key_from_mnemonic(secret, &salt, encrypted.kdf_iterations)?
            },
            | AuthMode::Piv => {
                return Err(SlideError::Crypto(
                    "Unexpected PIV auth mode in software keystore".to_string(),
                ));
            },
        };

        let mut decrypted_plaintext = aes_256_gcm_decrypt(&aes_key, &nonce, &tag, &ciphertext)
            .map_err(|_| {
                SlideError::IntegrityCheckFailed(
                    "Incorrect passphrase or recovery mnemonic. Decryption failed.".to_string(),
                )
            })?;

        let mut payload: PlaintextKeystorePayload = serde_json::from_slice(&decrypted_plaintext)
            .map_err(|e| SlideError::Crypto(format!("Failed to parse decrypted keystore: {e}")))?;
        decrypted_plaintext.zeroize();

        let recipient_key = SecretKey::from_slice(&payload.recipient_priv_bytes)
            .map_err(|e| SlideError::Crypto(format!("Invalid recipient private key bytes: {e}")))?;
        let signer_key = SecretKey::from_slice(&payload.signer_priv_bytes)
            .map_err(|e| SlideError::Crypto(format!("Invalid signer private key bytes: {e}")))?;
        payload.recipient_priv_bytes.zeroize();
        payload.signer_priv_bytes.zeroize();

        Ok(Self {
            recipient_key,
            signer_key,
            auth_mode: encrypted.auth_mode,
        })
    }

    /// Retrieve the recipient public key.
    #[must_use]
    pub fn recipient_public_key(&self) -> PivPublicKey {
        PivPublicKey::Ecc(self.recipient_key.public_key())
    }

    /// Retrieve the signer public key.
    #[must_use]
    pub fn signer_public_key(&self) -> PivPublicKey {
        PivPublicKey::Ecc(self.signer_key.public_key())
    }
}

impl PivCard for SoftwareKeystore {
    fn name(&self) -> String {
        format!("Software Dual-Key Keystore ({})", self.auth_mode)
    }

    fn verify_pin(
        &mut self,
        _pin: &str,
    ) -> Result<()> {
        Ok(())
    }

    fn is_authenticated(&self) -> bool {
        true
    }

    fn list_slots(&self) -> Vec<PivSlot> {
        vec![PivSlot::Slot9D, PivSlot::Slot9C]
    }

    fn get_certificate(
        &self,
        slot: PivSlot,
    ) -> Result<Vec<u8>> {
        use p256::pkcs8::EncodePrivateKey as _;
        let priv_key = match slot {
            | PivSlot::Slot9D => &self.recipient_key,
            | _ => &self.signer_key,
        };
        let pkcs8 = priv_key
            .to_pkcs8_der()
            .map_err(|e| SlideError::Crypto(e.to_string()))?;
        let kp = rcgen::KeyPair::try_from(pkcs8.as_bytes())
            .map_err(|e| SlideError::Crypto(e.to_string()))?;
        let (der, _) = derive_self_signed_piv_certificate(
            slot,
            &format!("Cargo Slide Software {}", slot.name()),
            None,
            365,
            &kp,
        )?;
        Ok(der)
    }

    fn get_public_key(
        &self,
        slot: PivSlot,
    ) -> Result<PivPublicKey> {
        match slot {
            | PivSlot::Slot9D => Ok(self.recipient_public_key()),
            | _ => Ok(self.signer_public_key()),
        }
    }

    fn decrypt(
        &mut self,
        slot: PivSlot,
        ciphertext: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>> {
        let priv_key = match slot {
            | PivSlot::Slot9D => &self.recipient_key,
            | _ => &self.signer_key,
        };
        let key_bytes = p256_ecdh_decrypt(priv_key, ciphertext)?;
        Ok(Zeroizing::new(key_bytes.to_vec()))
    }

    fn sign(
        &mut self,
        _slot: PivSlot,
        digest_sha3_512: &[u8; 64],
    ) -> Result<Vec<u8>> {
        p256_ecdsa_sign(&self.signer_key, digest_sha3_512)
    }

    fn is_software(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_strength_rules() {
        // Too short
        assert!(validate_password_strength("Short1!").is_err());
        // Missing classes (< 3)
        assert!(validate_password_strength("alllowercaseletters").is_err());
        assert!(validate_password_strength("ALLUPPERCASELETTERS").is_err());
        assert!(validate_password_strength("12345678901234").is_err());
        // Common pattern
        assert!(validate_password_strength("password123!A").is_err());
        // Repetitions
        assert!(validate_password_strength("Aaaa!1234567").is_err());
        // Valid strong passphrases
        assert!(validate_password_strength("Correct-Horse-Battery-Staple-99!").is_ok());
        assert!(validate_password_strength("P@ssw0rdSecure2026!").is_ok());
    }

    #[test]
    fn test_bip39_generation_and_validation() {
        // 12 words
        let m12 = generate_bip39_mnemonic(12).expect("12-word mnemonic generation");
        assert_eq!(m12.split_whitespace().count(), 12);
        let entropy12 = validate_bip39_mnemonic(&m12).expect("12-word validation");
        assert_eq!(entropy12.len(), 16);

        // 24 words
        let m24 = generate_bip39_mnemonic(24).expect("24-word mnemonic generation");
        assert_eq!(m24.split_whitespace().count(), 24);
        let entropy24 = validate_bip39_mnemonic(&m24).expect("24-word validation");
        assert_eq!(entropy24.len(), 32);

        // Invalid word
        assert!(validate_bip39_mnemonic("notarealbip39word abandon able about above absent absorb abstract absurd abuse access accident").is_err());

        // Official BIP-39 Test Vector 1 (16 zero bytes)
        let vector1 = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let entropy_vec1 = validate_bip39_mnemonic(vector1).expect("official vector 1");
        assert_eq!(*entropy_vec1, vec![0u8; 16]);

        // Known invalid checksum (12 'abandon's has checksum 0b0000 instead of 0b0011)
        let invalid_checksum = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon";
        assert!(validate_bip39_mnemonic(invalid_checksum).is_err());
    }

    #[test]
    fn test_software_keystore_encrypt_decrypt() {
        let ks = SoftwareKeystore::generate(AuthMode::Passphrase);
        let pass = "StrongP@ssphrase42!";
        let enc = ks.encrypt(pass).expect("keystore encryption");
        let dec = SoftwareKeystore::decrypt(&enc, pass).expect("keystore decryption");

        assert_eq!(ks.recipient_key.to_bytes(), dec.recipient_key.to_bytes());
        assert_eq!(ks.signer_key.to_bytes(), dec.signer_key.to_bytes());

        // Wrong password should fail
        assert!(SoftwareKeystore::decrypt(&enc, "WrongP@ssphrase42!").is_err());
    }
}
