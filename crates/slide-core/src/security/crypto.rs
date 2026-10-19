//! Core cryptographic primitives for cargo-slide:
//! - Pure Rust AES-256-GCM symmetric encryption with memory zeroization
//! - SHA3-512 cryptographic hashing for integrity validation
//! - RSA-OAEP (2048/3072/4096) and ECC NIST P-256 ECDH for PIV slot key encapsulation
//! - RSA-PKCS#1v15 and ECDSA signatures over SHA3-512 digests for PIV slot signing

use crate::error::Result;
use crate::error::SlideError;
use aes_gcm::Aes256Gcm;
use aes_gcm::Nonce;
use aes_gcm::aead::Aead;
use aes_gcm::aead::KeyInit;
use p256::ecdsa::Signature as EcdsaSignature;
use p256::ecdsa::SigningKey as EcdsaSigningKey;
use p256::ecdsa::VerifyingKey as EcdsaVerifyingKey;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::signature::Verifier;
use rsa::RsaPrivateKey;
use rsa::RsaPublicKey;
use sha3::Digest as _;
use sha3::Sha3_512;
use zeroize::Zeroize;
use zeroize::Zeroizing;

/// Generate a cryptographically secure random 256-bit AES symmetric key.
#[must_use]
pub fn generate_aes_key() -> Zeroizing<[u8; 32]> {
    let mut key = [0u8; 32];
    rand::fill(&mut key);
    Zeroizing::new(key)
}

/// Compute the SHA3-512 hash of the given data.
#[must_use]
pub fn compute_sha3_512(data: &[u8]) -> [u8; 64] {
    let mut hasher = Sha3_512::new();
    hasher.update(data);
    let output = hasher.finalize();
    let mut result = [0u8; 64];
    result.copy_from_slice(&output);
    result
}

/// Encrypt data using AES-256-GCM with a random 96-bit nonce.
/// Returns `(ciphertext, nonce, tag)`.
pub fn aes_256_gcm_encrypt(
    key: &[u8; 32],
    plaintext: &[u8],
) -> Result<(Vec<u8>, [u8; 12], [u8; 16])> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| SlideError::Crypto(format!("Failed to initialize AES-256-GCM cipher: {e}")))?;

    let mut nonce_bytes = [0u8; 12];
    rand::fill(&mut nonce_bytes);
    let nonce = Nonce::from(nonce_bytes);

    let ciphertext_and_tag = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| SlideError::Crypto(format!("AES-256-GCM encryption failed: {e}")))?;

    if ciphertext_and_tag.len() < 16 {
        return Err(SlideError::Crypto(
            "AES-256-GCM output shorter than 16 bytes authentication tag".to_string(),
        ));
    }

    let split_pos = ciphertext_and_tag.len().saturating_sub(16);
    let ciphertext = ciphertext_and_tag
        .get(..split_pos)
        .unwrap_or_default()
        .to_vec();
    let mut tag = [0u8; 16];
    if let Some(tag_slice) = ciphertext_and_tag.get(split_pos..) {
        tag.copy_from_slice(tag_slice);
    }

    Ok((ciphertext, nonce_bytes, tag))
}

/// Decrypt and authenticate data using AES-256-GCM.
/// Returns a zeroized memory buffer to protect against heap inspection.
pub fn aes_256_gcm_decrypt(
    key: &[u8; 32],
    nonce: &[u8; 12],
    tag: &[u8; 16],
    ciphertext: &[u8],
) -> Result<Zeroizing<Vec<u8>>> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| SlideError::Crypto(format!("Failed to initialize AES-256-GCM: {e}")))?;

    let mut combined = Vec::with_capacity(ciphertext.len().saturating_add(16));
    combined.extend_from_slice(ciphertext);
    combined.extend_from_slice(tag);

    let nonce_ref = Nonce::from(*nonce);
    let plaintext = cipher.decrypt(&nonce_ref, combined.as_ref()).map_err(|e| {
        SlideError::Crypto(format!("AES-256-GCM decryption failed / tag mismatch: {e}"))
    })?;

    Ok(Zeroizing::new(plaintext))
}

/// Encrypt a 32-byte session key using an RSA public key with OAEP (SHA-256).
pub fn rsa_oaep_encrypt(
    pub_key: &RsaPublicKey,
    plaintext: &[u8],
) -> Result<Vec<u8>> {
    let mut rng = rsa::rand_core::OsRng;
    let padding = rsa::Oaep::new::<rsa::sha2::Sha256>();
    pub_key
        .encrypt(&mut rng, padding, plaintext)
        .map_err(|e| SlideError::Crypto(format!("RSA-OAEP encryption failed: {e}")))
}

/// Decrypt an RSA-OAEP ciphertext using an RSA private key.
pub fn rsa_oaep_decrypt(
    priv_key: &RsaPrivateKey,
    ciphertext: &[u8],
) -> Result<Zeroizing<Vec<u8>>> {
    let padding = rsa::Oaep::new::<rsa::sha2::Sha256>();
    let decrypted = priv_key
        .decrypt(padding, ciphertext)
        .map_err(|e| SlideError::Crypto(format!("RSA-OAEP decryption failed: {e}")))?;
    Ok(Zeroizing::new(decrypted))
}

/// Unpad an RSA-OAEP encoded message block (using SHA-256 and MGF1-SHA256) returned by hardware smartcard.
pub fn rsa_oaep_unpad_sha256(em: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    use sha2::Digest as _;
    let h_len = 32;
    if em.len() < 2 * h_len + 2 {
        return Err(SlideError::Crypto("RSA-OAEP block too short".to_string()));
    }

    let y = em[0];
    let masked_seed = &em[1..1 + h_len];
    let masked_db = &em[1 + h_len..];

    fn mgf1(
        seed: &[u8],
        mask_len: usize,
    ) -> Vec<u8> {
        let mut mask = Vec::with_capacity(mask_len);
        let mut counter: u32 = 0;
        while mask.len() < mask_len {
            let mut hasher = sha2::Sha256::new();
            hasher.update(seed);
            hasher.update(counter.to_be_bytes());
            let digest = hasher.finalize();
            let take = std::cmp::min(digest.len(), mask_len.saturating_sub(mask.len()));
            mask.extend_from_slice(&digest[..take]);
            counter = counter.saturating_add(1);
        }
        mask
    }

    let seed_mask = mgf1(masked_db, h_len);
    let mut seed = [0u8; 32];
    for i in 0..h_len {
        seed[i] = masked_seed[i] ^ seed_mask[i];
    }

    let db_mask = mgf1(&seed, masked_db.len());
    let mut db = vec![0u8; masked_db.len()];
    for i in 0..masked_db.len() {
        db[i] = masked_db[i] ^ db_mask[i];
    }

    let p_hash = sha2::Sha256::digest(b"");
    if &db[..h_len] != p_hash.as_slice() || y != 0 {
        return Err(SlideError::Crypto(
            "RSA-OAEP decoding failed: hash mismatch or invalid padding".to_string(),
        ));
    }

    let mut start_idx = None;
    for (offset, &b) in db[h_len..].iter().enumerate() {
        if b == 0x01 {
            start_idx = Some(h_len.saturating_add(offset).saturating_add(1));
            break;
        } else if b != 0x00 {
            return Err(SlideError::Crypto(
                "RSA-OAEP decoding failed: non-zero byte in padding".to_string(),
            ));
        }
    }

    let start = start_idx.ok_or_else(|| {
        SlideError::Crypto("RSA-OAEP decoding failed: 0x01 separator not found".to_string())
    })?;
    Ok(Zeroizing::new(db[start..].to_vec()))
}

/// Standard NIST SP 800-73-4 / RFC 8017 DigestInfo prefix for SHA3-512 (OID: 2.16.840.1.101.3.4.2.10).
pub const SHA3_512_DIGEST_INFO_PREFIX: [u8; 19] = [
    0x30, 0x51, 0x30, 0x0D, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x0A, 0x05,
    0x00, 0x04, 0x40,
];

/// Pad a 64-byte SHA3-512 digest into standard EMSA-PKCS1-v1_5 format for on-chip RSA hardware signing.
/// Standard legacy NIST SHA-512 DigestInfo prefix (OID: 2.16.840.1.101.3.4.2.3).
pub const SHA512_LEGACY_DIGEST_INFO_PREFIX: [u8; 19] = [
    0x30, 0x51, 0x30, 0x0D, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03, 0x05,
    0x00, 0x04, 0x40,
];

/// Pad a 64-byte digest into standard EMSA-PKCS1-v1_5 format with the specified DigestInfo prefix.
pub fn pkcs1v15_pad_with_prefix(
    digest: &[u8; 64],
    prefix: &[u8],
    key_len: usize,
) -> Result<Vec<u8>> {
    let digest_info_len = prefix.len().saturating_add(digest.len());
    if key_len < digest_info_len.saturating_add(3) {
        return Err(SlideError::Crypto(format!(
            "RSA modulus length {key_len} too small for signature digest"
        )));
    }
    let pad_len = key_len - digest_info_len - 3;
    let mut block = Vec::with_capacity(key_len);
    block.push(0x00);
    block.push(0x01);
    block.resize(2 + pad_len, 0xFF);
    block.push(0x00);
    block.extend_from_slice(prefix);
    block.extend_from_slice(digest);
    Ok(block)
}

/// Pad a 64-byte SHA3-512 digest into standard EMSA-PKCS1-v1_5 format for on-chip RSA hardware signing.
pub fn pkcs1v15_pad_sha3_512(
    digest: &[u8; 64],
    key_len: usize,
) -> Result<Vec<u8>> {
    pkcs1v15_pad_with_prefix(digest, &SHA3_512_DIGEST_INFO_PREFIX, key_len)
}

/// Sign a SHA3-512 digest using an RSA private key with PKCS#1 v1.5 padding.
pub fn rsa_sign_sha3_512(
    priv_key: &RsaPrivateKey,
    digest_sha3_512: &[u8; 64],
) -> Result<Vec<u8>> {
    use rsa::BigUint;
    use rsa::traits::PrivateKeyParts as _;
    use rsa::traits::PublicKeyParts as _;

    let k = priv_key.size();
    let padded = pkcs1v15_pad_sha3_512(digest_sha3_512, k)?;
    let m = BigUint::from_bytes_be(&padded);
    let s = m.modpow(priv_key.d(), priv_key.n());
    let s_bytes = s.to_bytes_be();
    let mut sig = vec![0u8; k];
    let offset = k.saturating_sub(s_bytes.len());
    sig[offset..].copy_from_slice(&s_bytes);
    Ok(sig)
}

/// Verify an RSA PKCS#1 v1.5 signature against a SHA3-512 digest (RSAVP1).
/// Validates modular exponentiation s^e mod n using strictly constant-time canonical
/// comparison (`subtle::ConstantTimeEq`) to eliminate Bleichenbacher and Marvin timing side-channels.
pub fn rsa_verify_sha3_512(
    pub_key: &RsaPublicKey,
    digest_sha3_512: &[u8; 64],
    signature_bytes: &[u8],
) -> Result<()> {
    use rsa::BigUint;
    use rsa::traits::PublicKeyParts as _;
    use subtle::ConstantTimeEq as _;

    let k = pub_key.size();
    if signature_bytes.len() != k {
        return Err(SlideError::IntegrityCheckFailed(format!(
            "RSA signature length mismatch: expected {k} bytes, got {}",
            signature_bytes.len()
        )));
    }

    let s = BigUint::from_bytes_be(signature_bytes);
    let n = pub_key.n();
    let e = pub_key.e();
    if s >= *n {
        return Err(SlideError::IntegrityCheckFailed(
            "RSA signature value out of range (s >= n)".to_string(),
        ));
    }

    let m = s.modpow(e, n);
    let m_bytes = m.to_bytes_be();
    let mut em = vec![0u8; k];
    let offset = k.saturating_sub(m_bytes.len());
    em[offset..].copy_from_slice(&m_bytes);

    // Precompute the expected canonical EMSA-PKCS1-v1_5 blocks
    let expected_sha3 = pkcs1v15_pad_sha3_512(digest_sha3_512, k)?;
    let expected_legacy =
        pkcs1v15_pad_with_prefix(digest_sha3_512, &SHA512_LEGACY_DIGEST_INFO_PREFIX, k)?;

    // Strict constant-time comparison against canonical padded representations
    let valid_sha3 = em.ct_eq(&expected_sha3);
    let valid_legacy = em.ct_eq(&expected_legacy);
    let is_valid: bool = (valid_sha3 | valid_legacy).into();

    if !is_valid {
        return Err(SlideError::IntegrityCheckFailed(
            "RSA SHA3-512 signature verification failed: signature invalid or digest mismatch"
                .to_string(),
        ));
    }

    Ok(())
}


/// Encapsulate a 32-byte AES key using NIST P-256 ECDH and HKDF-SHA256.
/// Format of returned bytes: `[33-byte ephemeral compressed pubkey] + [32-byte wrapped key]`.
pub fn p256_ecdh_encrypt(
    recipient_pub: &p256::PublicKey,
    session_key: &[u8; 32],
) -> Result<Vec<u8>> {
    use p256::elliptic_curve::Generate;
    use p256::elliptic_curve::sec1::ToSec1Point;
    let ephemeral_secret = p256::SecretKey::generate();
    let ephemeral_pub = ephemeral_secret.public_key();

    let shared_secret = p256::ecdh::diffie_hellman(
        ephemeral_secret.to_nonzero_scalar(),
        recipient_pub.as_affine(),
    );

    let hkdf = shared_secret.extract::<sha2::Sha256>(Some(b"cargo-slide-piv-kdf-salt"));
    let mut wrapping_key = [0u8; 32];
    hkdf.expand(b"piv-aes256-wrap", &mut wrapping_key)
        .map_err(|e| SlideError::Crypto(format!("HKDF key derivation failed: {e}")))?;

    // XOR or AES wrap key
    let mut wrapped = [0u8; 32];
    for i in 0..32 {
        if let (Some(w), Some(&s), Some(&k)) =
            (wrapped.get_mut(i), session_key.get(i), wrapping_key.get(i))
        {
            *w = s ^ k;
        }
    }
    wrapping_key.zeroize();

    let uncompressed_point = ephemeral_pub.to_sec1_point(false);
    let ephemeral_pub_bytes = uncompressed_point.as_bytes();
    let mut result = Vec::with_capacity(ephemeral_pub_bytes.len().saturating_add(32));
    result.extend_from_slice(ephemeral_pub_bytes);
    result.extend_from_slice(&wrapped);

    Ok(result)
}

/// Finish P-256 ECDH decapsulation given the 32-byte shared secret Z and wrapped AES key bytes.
pub fn p256_ecdh_finish_decrypt(
    shared_secret_z: &[u8],
    wrapped: &[u8],
) -> Result<Zeroizing<[u8; 32]>> {
    if shared_secret_z.len() != 32 || wrapped.len() != 32 {
        return Err(SlideError::Crypto(
            "Invalid P-256 ECDH shared secret or wrapped key length".to_string(),
        ));
    }
    let hk = hkdf::Hkdf::<sha2::Sha256>::new(Some(b"cargo-slide-piv-kdf-salt"), shared_secret_z);
    let mut wrapping_key = [0u8; 32];
    hk.expand(b"piv-aes256-wrap", &mut wrapping_key)
        .map_err(|e| SlideError::Crypto(format!("HKDF key derivation failed: {e}")))?;

    let mut session_key = [0u8; 32];
    for i in 0..32 {
        if let (Some(s), Some(&w), Some(&k)) =
            (session_key.get_mut(i), wrapped.get(i), wrapping_key.get(i))
        {
            *s = w ^ k;
        }
    }
    wrapping_key.zeroize();

    Ok(Zeroizing::new(session_key))
}

/// Decapsulate a 32-byte AES key using NIST P-256 private key and HKDF-SHA256.
pub fn p256_ecdh_decrypt(
    recipient_priv: &p256::SecretKey,
    encapsulated: &[u8],
) -> Result<Zeroizing<[u8; 32]>> {
    if encapsulated.len() < 65 {
        return Err(SlideError::Crypto(
            "Encapsulated ECDH payload too short".to_string(),
        ));
    }

    let (pub_bytes, wrapped) = encapsulated.split_at(encapsulated.len().saturating_sub(32));
    let ephemeral_pub = p256::PublicKey::from_sec1_bytes(pub_bytes)
        .map_err(|e| SlideError::Crypto(format!("Invalid ephemeral ECDH public key: {e}")))?;

    let shared_secret = p256::ecdh::diffie_hellman(
        recipient_priv.to_nonzero_scalar(),
        ephemeral_pub.as_affine(),
    );

    let raw_secret = shared_secret.raw_secret_bytes();
    p256_ecdh_finish_decrypt(raw_secret.as_slice(), wrapped)
}

/// Sign a SHA3-512 digest using a NIST P-256 private key with ECDSA.
pub fn p256_ecdsa_sign(
    priv_key: &p256::SecretKey,
    digest_sha3_512: &[u8; 64],
) -> Result<Vec<u8>> {
    let signing_key = EcdsaSigningKey::from(priv_key);
    let signature: EcdsaSignature = signing_key.sign(digest_sha3_512);
    Ok(signature.to_bytes().to_vec())
}

/// Verify a NIST P-256 ECDSA signature against a SHA3-512 digest.
pub fn p256_ecdsa_verify(
    pub_key: &p256::PublicKey,
    digest_sha3_512: &[u8; 64],
    signature_bytes: &[u8],
) -> Result<()> {
    let verifying_key = EcdsaVerifyingKey::from(pub_key);
    let signature = EcdsaSignature::from_slice(signature_bytes)
        .map_err(|e| SlideError::IntegrityCheckFailed(format!("Invalid ECDSA signature: {e}")))?;

    verifying_key
        .verify(digest_sha3_512, &signature)
        .map_err(|e| {
            SlideError::IntegrityCheckFailed(format!(
                "ECDSA SHA3-512 signature verification failed: {e}"
            ))
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::RsaPrivateKey;

    #[test]
    fn test_rsa_sha3_512_sign_and_verify() {
        let mut rng = rsa::rand_core::OsRng;
        let priv_key = RsaPrivateKey::new(&mut rng, 2048).expect("failed to generate RSA-2048 key");
        let pub_key = priv_key.to_public_key();

        let data = b"Apich Cargo Slide SHA3-512 hardware verification test";
        let digest = compute_sha3_512(data);

        // Sign
        let sig = rsa_sign_sha3_512(&priv_key, &digest).expect("RSA signing failed");
        assert_eq!(sig.len(), 256);

        // Verify valid signature
        assert!(rsa_verify_sha3_512(&pub_key, &digest, &sig).is_ok());

        // Tamper with digest
        let mut corrupted_digest = digest;
        corrupted_digest[0] ^= 0x01;
        assert!(rsa_verify_sha3_512(&pub_key, &corrupted_digest, &sig).is_err());

        // Tamper with signature
        let mut corrupted_sig = sig.clone();
        corrupted_sig[10] ^= 0x01;
        assert!(rsa_verify_sha3_512(&pub_key, &digest, &corrupted_sig).is_err());
    }
}
