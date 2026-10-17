//! X.509 certificate management, parsing, and derivation for YubiKey PIV slots.
//!
//! Provides built-in generation of self-signed X.509 PIV certificates, extraction
//! of public keys, and SHA3-512 certificate fingerprinting without external system dependencies.

use crate::error::Result;
use crate::error::SlideError;
use rsa::RsaPublicKey;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;

/// Standard NIST SP 800-73-4 PIV card slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PivSlot {
    /// Slot 9A: PIV Authentication
    #[serde(rename = "9a")]
    Slot9A,
    /// Slot 9C: Digital Signature
    #[serde(rename = "9c")]
    Slot9C,
    /// Slot 9D: Key Management (Decryption)
    #[serde(rename = "9d")]
    Slot9D,
    /// Slot 9E: Card Authentication
    #[serde(rename = "9e")]
    Slot9E,
}

impl PivSlot {
    /// Parse PIV slot from string (e.g. "9a", "9c", "9d", "9e").
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let clean = s.trim().to_lowercase();
        match clean.as_str() {
            | "9a" | "0x9a" | "auth" => Some(Self::Slot9A),
            | "9c" | "0x9c" | "sig" | "sign" => Some(Self::Slot9C),
            | "9d" | "0x9d" | "key" | "decrypt" | "mgmt" => Some(Self::Slot9D),
            | "9e" | "0x9e" | "card" => Some(Self::Slot9E),
            | _ => None,
        }
    }

    /// Two-character standard PIV slot identifier.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            | Self::Slot9A => "9a",
            | Self::Slot9C => "9c",
            | Self::Slot9D => "9d",
            | Self::Slot9E => "9e",
        }
    }

    /// Human-readable slot name according to NIST SP 800-73-4.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            | Self::Slot9A => "PIV Authentication",
            | Self::Slot9C => "Digital Signature",
            | Self::Slot9D => "Key Management",
            | Self::Slot9E => "Card Authentication",
        }
    }

    /// 3-byte NIST PIV container object tag for certificate.
    #[must_use]
    pub const fn object_tag(&self) -> [u8; 3] {
        match self {
            | Self::Slot9A => [0x5F, 0xC1, 0x05],
            | Self::Slot9C => [0x5F, 0xC1, 0x0A],
            | Self::Slot9D => [0x5F, 0xC1, 0x0B],
            | Self::Slot9E => [0x5F, 0xC1, 0x01],
        }
    }

    /// Standard 1-byte PIV key reference byte.
    #[must_use]
    pub const fn reference_byte(&self) -> u8 {
        match self {
            | Self::Slot9A => 0x9A,
            | Self::Slot9C => 0x9C,
            | Self::Slot9D => 0x9D,
            | Self::Slot9E => 0x9E,
        }
    }
}

impl fmt::Display for PivSlot {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "Slot {} ({})", self.as_str(), self.name())
    }
}

/// Supported PIV cryptographic key types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PivKeyAlgorithm {
    /// RSA 2048-bit
    Rsa2048,
    /// RSA 3072-bit
    Rsa3072,
    /// RSA 4096-bit
    Rsa4096,
    /// NIST P-256 (secp256r1)
    EccP256,
}

impl PivKeyAlgorithm {
    /// Parse key algorithm from string.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let clean = s.trim().to_lowercase();
        match clean.as_str() {
            | "rsa2048" | "rsa-2048" | "2048" => Some(Self::Rsa2048),
            | "rsa3072" | "rsa-3072" | "3072" => Some(Self::Rsa3072),
            | "rsa4096" | "rsa-4096" | "4096" => Some(Self::Rsa4096),
            | "eccp256" | "p256" | "p-256" | "secp256r1" | "ecc" => Some(Self::EccP256),
            | _ => None,
        }
    }

    /// String name of the algorithm.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            | Self::Rsa2048 => "RSA-2048",
            | Self::Rsa3072 => "RSA-3072",
            | Self::Rsa4096 => "RSA-4096",
            | Self::EccP256 => "ECCP256",
        }
    }
}

/// Generic PIV public key (RSA or NIST P-256).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PivPublicKey {
    /// RSA public key
    Rsa(RsaPublicKey),
    /// NIST P-256 public key
    Ecc(p256::PublicKey),
}

impl PivPublicKey {
    /// Return the algorithm name for this public key.
    #[must_use]
    pub const fn algorithm_name(&self) -> &'static str {
        match self {
            | Self::Rsa(_) => "RSA",
            | Self::Ecc(_) => "ECDSA/ECDH (P-256)",
        }
    }

    /// Return the NIST SP 800-73-4 / YubiKey algorithm identifier byte (P1 for INS 0x87).
    #[must_use]
    pub fn piv_algorithm_byte(&self) -> u8 {
        match self {
            | Self::Rsa(rsa_key) => {
                match rsa::traits::PublicKeyParts::size(rsa_key) {
                    | 128 => 0x06, // RSA 1024
                    | 256 => 0x07, // RSA 2048
                    | 384 => 0x05, // RSA 3072
                    | 512 => 0x16, // RSA 4096
                    | _ => 0x07,
                }
            },
            | Self::Ecc(_) => 0x11, // ECC P-256
        }
    }

    /// Encode public key to SubjectPublicKeyInfo DER bytes.
    pub fn to_der(&self) -> Result<Vec<u8>> {
        match self {
            | Self::Rsa(k) => {
                rsa::pkcs8::EncodePublicKey::to_public_key_der(k)
                    .map(|d| d.as_bytes().to_vec())
                    .map_err(|e| {
                        SlideError::Crypto(format!("Failed to encode RSA public key to DER: {e}"))
                    })
            },
            | Self::Ecc(k) => {
                p256::pkcs8::EncodePublicKey::to_public_key_der(k)
                    .map(|d| d.as_bytes().to_vec())
                    .map_err(|e| {
                        SlideError::Crypto(format!("Failed to encode ECC public key to DER: {e}"))
                    })
            },
        }
    }

    /// Parse public key from SubjectPublicKeyInfo DER bytes.
    pub fn from_der(der: &[u8]) -> Result<Self> {
        if let Ok(rsa_key) = <RsaPublicKey as rsa::pkcs8::DecodePublicKey>::from_public_key_der(der)
        {
            return Ok(Self::Rsa(rsa_key));
        }
        if let Ok(rsa_key) = <RsaPublicKey as rsa::pkcs1::DecodeRsaPublicKey>::from_pkcs1_der(der) {
            return Ok(Self::Rsa(rsa_key));
        }
        if let Ok(ecc_key) =
            <p256::PublicKey as p256::pkcs8::DecodePublicKey>::from_public_key_der(der)
        {
            return Ok(Self::Ecc(ecc_key));
        }
        if let Ok(ecc_key) = p256::PublicKey::from_sec1_bytes(der) {
            return Ok(Self::Ecc(ecc_key));
        }
        Err(SlideError::Crypto(
            "Failed to parse public key from DER (neither RSA nor P-256)".to_string(),
        ))
    }

    /// Encode public key to PEM string.
    pub fn to_pem(&self) -> Result<String> {
        let der = self.to_der()?;
        use base64::Engine as _;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&der);
        let mut pem = String::from("-----BEGIN PUBLIC KEY-----\n");
        for chunk in b64.as_bytes().chunks(64) {
            if let Ok(s) = std::str::from_utf8(chunk) {
                pem.push_str(s);
                pem.push('\n');
            }
        }
        pem.push_str("-----END PUBLIC KEY-----\n");
        Ok(pem)
    }

    /// Parse public key from PEM string or DER bytes.
    pub fn from_pem_or_der(data: &[u8]) -> Result<Self> {
        // Try DER first
        if let Ok(k) = Self::from_der(data) {
            return Ok(k);
        }

        // Try PEM string
        if let Ok(s) = std::str::from_utf8(data) {
            let lines: Vec<&str> = s
                .lines()
                .filter(|l| !l.starts_with("-----"))
                .map(str::trim)
                .collect();
            let b64 = lines.join("");
            use base64::Engine as _;
            if let Ok(der_bytes) = base64::engine::general_purpose::STANDARD.decode(b64)
                && let Ok(k) = Self::from_der(&der_bytes)
            {
                return Ok(k);
            }
        }

        // Also try parsing as X.509 certificate
        parse_certificate_public_key(data)
    }
}

/// Parsed X.509 Certificate Metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PivCertInfo {
    /// Subject Distinguished Name
    pub subject: String,
    /// Issuer Distinguished Name
    pub issuer: String,
    /// Serial Number (hex)
    pub serial: String,
    /// Not Before ISO string
    pub not_before: String,
    /// Not After ISO string
    pub not_after: String,
    /// SHA3-512 fingerprint in hex
    pub sha3_512_fingerprint: String,
    /// Algorithm name
    pub algorithm: String,
}

/// Parse X.509 certificate DER bytes to extract human-readable info.
pub fn parse_certificate_info(cert_der: &[u8]) -> Result<PivCertInfo> {
    let (_, cert) = x509_parser::parse_x509_certificate(cert_der)
        .map_err(|e| SlideError::Crypto(format!("Failed to parse X.509 certificate: {e}")))?;

    let subject = cert.subject().to_string();
    let issuer = cert.issuer().to_string();
    let serial = cert.raw_serial_as_string();
    let not_before = cert.validity().not_before.to_string();
    let not_after = cert.validity().not_after.to_string();

    let digest = crate::security::crypto::compute_sha3_512(cert_der);
    let mut sha3_512_fingerprint = String::with_capacity(128);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(sha3_512_fingerprint, "{b:02x}");
    }

    let alg = if *cert.public_key().algorithm.oid()
        == x509_parser::oid_registry::OID_PKCS1_RSAENCRYPTION
    {
        "RSA".to_string()
    } else {
        "ECC/ECDSA".to_string()
    };

    Ok(PivCertInfo {
        subject,
        issuer,
        serial,
        not_before,
        not_after,
        sha3_512_fingerprint,
        algorithm: alg,
    })
}

/// Extract `PivPublicKey` from X.509 certificate DER or PEM bytes.
pub fn parse_certificate_public_key(data: &[u8]) -> Result<PivPublicKey> {
    let der_bytes = if data.starts_with(b"-----BEGIN CERTIFICATE-----") {
        if let Ok(s) = std::str::from_utf8(data) {
            let lines: Vec<&str> = s
                .lines()
                .filter(|l| !l.starts_with("-----"))
                .map(str::trim)
                .collect();
            let b64 = lines.join("");
            use base64::Engine as _;
            base64::engine::general_purpose::STANDARD
                .decode(b64)
                .map_err(|e| SlideError::Crypto(format!("Invalid base64 in certificate: {e}")))?
        } else {
            return Err(SlideError::Crypto(
                "Invalid UTF-8 in certificate PEM".to_string(),
            ));
        }
    } else {
        data.to_vec()
    };

    let (_, cert) = x509_parser::parse_x509_certificate(&der_bytes)
        .map_err(|e| SlideError::Crypto(format!("Failed to parse X.509 certificate: {e}")))?;

    let spki_bytes = cert.public_key().raw;
    PivPublicKey::from_der(spki_bytes)
}

/// Generate a self-signed X.509 certificate for a PIV slot using `rcgen`.
/// Returns `(cert_der_bytes, cert_pem_string)`.
pub fn derive_self_signed_piv_certificate(
    slot: PivSlot,
    common_name: &str,
    org: Option<&str>,
    validity_days: u32,
    key_pair: &rcgen::KeyPair,
) -> Result<(Vec<u8>, String)> {
    let mut params = rcgen::CertificateParams::default();
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, common_name);
    if let Some(o) = org {
        params
            .distinguished_name
            .push(rcgen::DnType::OrganizationName, o);
    }
    params.distinguished_name.push(
        rcgen::DnType::OrganizationalUnitName,
        format!("PIV {}", slot.name()),
    );

    // Set validity duration
    let now = time::OffsetDateTime::now_utc();
    params.not_before = now;
    params.not_after = now + time::Duration::days(i64::from(validity_days));

    // Configure key usage according to PIV NIST SP 800-73-4 standard:
    match slot {
        | PivSlot::Slot9A => {
            // PIV Authentication
            params
                .key_usages
                .push(rcgen::KeyUsagePurpose::DigitalSignature);
            params
                .extended_key_usages
                .push(rcgen::ExtendedKeyUsagePurpose::ClientAuth);
        },
        | PivSlot::Slot9C => {
            // Digital Signature
            params
                .key_usages
                .push(rcgen::KeyUsagePurpose::DigitalSignature);
        },
        | PivSlot::Slot9D => {
            // Key Management (Decryption / Key Encipherment / Key Agreement)
            params
                .key_usages
                .push(rcgen::KeyUsagePurpose::KeyEncipherment);
            params.key_usages.push(rcgen::KeyUsagePurpose::KeyAgreement);
            params
                .key_usages
                .push(rcgen::KeyUsagePurpose::DataEncipherment);
        },
        | PivSlot::Slot9E => {
            // Card Authentication
            params
                .key_usages
                .push(rcgen::KeyUsagePurpose::DigitalSignature);
        },
    }

    let cert = params.self_signed(key_pair).map_err(|e| {
        SlideError::Crypto(format!("Failed to generate self-signed certificate: {e}"))
    })?;

    let der = cert.der().to_vec();
    let pem = cert.pem();

    Ok((der, pem))
}

/// Convert X.509 certificate DER bytes to PEM string format.
#[must_use]
pub fn cert_der_to_pem(cert_der: &[u8]) -> String {
    use base64::Engine as _;
    let b64 = base64::engine::general_purpose::STANDARD.encode(cert_der);
    let mut pem = String::from("-----BEGIN CERTIFICATE-----\n");
    for chunk in b64.as_bytes().chunks(64) {
        if let Ok(s) = std::str::from_utf8(chunk) {
            pem.push_str(s);
            pem.push('\n');
        }
    }
    pem.push_str("-----END CERTIFICATE-----\n");
    pem
}
