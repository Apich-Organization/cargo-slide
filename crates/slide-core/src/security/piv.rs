//! Real YubiKey PIV smartcard driver over PC/SC and NIST SP 800-73-4 standard.
//!
//! Provides direct hardware communication with genuine YubiKeys:
//! - Slots: 9A (Authentication), 9C (Digital Signature), 9D (Key Management), 9E (Card Authentication)
//! - Cryptography on Hardware: On-chip RSA-OAEP & ECC-P256 decipher on Slot 9D;
//!   On-chip RSA & ECDSA-P256 digital signature over SHA3-512 on Slot 9C.
//! - Non-destructive: Strictly works in conjunction with Yubico's official tools (`ykman` / `yubico-piv-tool`).
//!   Never overwrites user PINs, management keys, or existing slot certificates.

use super::cert::PivPublicKey;
use super::cert::PivSlot;
use super::cert::parse_certificate_public_key;
use super::crypto::p256_ecdh_finish_decrypt;
use super::crypto::pkcs1v15_pad_sha3_512;
use super::crypto::rsa_oaep_unpad_sha256;
use super::pcsc::PcscCard;
use super::pcsc::PcscContext;
use crate::error::Result;
use crate::error::SlideError;
use rsa::traits::PublicKeyParts as _;
use std::io::IsTerminal as _;
use zeroize::Zeroize;
use zeroize::Zeroizing;

/// Status of PIN verification on the smartcard (ISO 7816-4 empty VERIFY query).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinStatus {
    /// PIN is already verified in this session.
    Verified,
    /// PIN is not verified, with given number of retry attempts remaining.
    NotVerified { tries_remaining: u8 },
    /// PIN is blocked (0 tries left, PUK required).
    Blocked,
    /// Status could not be determined.
    Unknown,
}

/// Common interface for YubiKey PIV hardware smartcards.
pub trait PivCard: Send + Sync {
    /// Human-readable token name and reader description.
    fn name(&self) -> String;

    /// Verify User PIN (typically 6-8 digits) against the physical card.
    fn verify_pin(
        &mut self,
        pin: &str,
    ) -> Result<()>;

    /// Check if PIN has been verified in current card session.
    fn is_authenticated(&self) -> bool;

    /// Check if the card has a cached PIN ready for NIST SP 800-73 re-verification.
    fn has_cached_pin(&self) -> bool {
        true
    }

    /// Query PIN verification status without consuming a retry attempt (ISO 7816-4 empty VERIFY).
    fn check_pin_status(&self) -> Result<PinStatus> {
        if self.is_authenticated() {
            Ok(PinStatus::Verified)
        } else {
            Ok(PinStatus::NotVerified { tries_remaining: 3 })
        }
    }

    /// Retrieve X.509 certificate DER bytes from the given PIV slot.
    fn get_certificate(
        &self,
        slot: PivSlot,
    ) -> Result<Vec<u8>>;

    /// Retrieve public key associated with the given PIV slot.
    fn get_public_key(
        &self,
        slot: PivSlot,
    ) -> Result<PivPublicKey>;

    /// Perform on-chip private-key decipher on Slot 9D (RSA-OAEP or ECC P-256 ECDH).
    /// The private key never leaves the YubiKey hardware secure element.
    fn decrypt(
        &mut self,
        slot: PivSlot,
        ciphertext: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>>;

    /// Perform on-chip digital signature on Slot 9C over a SHA3-512 digest.
    /// The private key never leaves the YubiKey hardware secure element.
    fn sign(
        &mut self,
        slot: PivSlot,
        digest_sha3_512: &[u8; 64],
    ) -> Result<Vec<u8>>;

    /// List all configured PIV slots with valid certificates on this card.
    fn list_slots(&self) -> Vec<PivSlot>;

    /// Check if this token is an emulated mock token.
    fn is_software(&self) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// BER-TLV Encoding and Decoding Helpers (NIST SP 800-73-4)
// ---------------------------------------------------------------------------

fn find_tlv(
    data: &[u8],
    target_tag: u8,
) -> Option<&[u8]> {
    let mut i = 0;
    while i < data.len() {
        let tag = *data.get(i)?;
        i += 1;
        if i >= data.len() {
            break;
        }

        let first_len = *data.get(i)? as usize;
        i += 1;

        let len = if first_len == 0x81 {
            let l = *data.get(i)? as usize;
            i += 1;
            l
        } else if first_len == 0x82 {
            let b1 = *data.get(i)? as usize;
            let b2 = *data.get(i + 1)? as usize;
            i += 2;
            (b1 << 8) | b2
        } else if first_len == 0x83 {
            let b1 = *data.get(i)? as usize;
            let b2 = *data.get(i + 1)? as usize;
            let b3 = *data.get(i + 2)? as usize;
            i += 3;
            (b1 << 16) | (b2 << 8) | b3
        } else {
            first_len
        };

        if i + len > data.len() {
            break;
        }

        if tag == target_tag {
            return Some(&data[i..i + len]);
        }

        i += len;
    }
    None
}

/// Unpack standard X.509 DER certificate bytes from NIST SP 800-73-4 Tag 0x53 and Tag 0x70.
fn extract_certificate_from_piv_data(data: &[u8]) -> Result<Vec<u8>> {
    // 1. If wrapped in Tag 0x53 (PIV Data Object wrapper), unwrap it
    let content = find_tlv(data, 0x53).unwrap_or(data);

    // 2. Locate Tag 0x70 (Certificate Data Object)
    let cert_raw = if let Some(c) = find_tlv(content, 0x70) {
        c
    } else if let Some(c) = find_tlv(data, 0x70) {
        c
    } else {
        content
    };

    if cert_raw.is_empty() {
        return Err(SlideError::Piv(
            "Empty certificate data in PIV slot".to_string(),
        ));
    }

    // 3. Handle GZIP compression (NIST SP 800-73-4 Table 9 / gzip magic 1F 8B)
    if cert_raw.len() >= 2 && cert_raw[0] == 0x1F && cert_raw[1] == 0x8B {
        use std::io::Read as _;
        let mut decoder = flate2::read::GzDecoder::new(cert_raw);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).map_err(|e| {
            SlideError::Crypto(format!("Failed to decompress gzip PIV certificate: {e}"))
        })?;
        return Ok(decompressed);
    }

    // 4. If it starts with 0x30 (ASN.1 SEQUENCE), it's raw DER
    if cert_raw[0] == 0x30 {
        return Ok(cert_raw.to_vec());
    }

    // 5. Fallback: locate 0x30 if preceded by padding
    if let Some(pos) = cert_raw.iter().position(|&b| b == 0x30) {
        return Ok(cert_raw[pos..].to_vec());
    }

    Ok(cert_raw.to_vec())
}

fn encode_tlv(
    tag: u8,
    val: &[u8],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(val.len() + 4);
    out.push(tag);
    if val.len() < 128 {
        out.push(val.len() as u8);
    } else if val.len() < 256 {
        out.push(0x81);
        out.push(val.len() as u8);
    } else {
        out.push(0x82);
        out.push((val.len() >> 8) as u8);
        out.push((val.len() & 0xFF) as u8);
    }
    out.extend_from_slice(val);
    out
}

// ---------------------------------------------------------------------------
// Physical Hardware PIV Smartcard via PC/SC (PcscPivCard)
// ---------------------------------------------------------------------------

/// Real YubiKey PIV Smartcard connected through native PC/SC subsystem.
pub struct PcscPivCard {
    card: PcscCard,
    name: String,
    authenticated: bool,
    cached_pin: Option<Zeroizing<String>>,
}

impl PcscPivCard {
    /// Connect to a detected PC/SC card and select NIST PIV application.
    pub fn connect(card: PcscCard) -> Result<Self> {
        let name = format!("YubiKey PIV [{}]", card.reader_name);

        // Select PIV Applet: AID A0 00 00 03 08 00 00 10 00
        let piv_aid = [0xA0, 0x00, 0x00, 0x03, 0x08, 0x00, 0x00, 0x10, 0x00];
        let mut select_apdu = vec![0x00, 0xA4, 0x04, 0x00, piv_aid.len() as u8];
        select_apdu.extend_from_slice(&piv_aid);

        card.transmit(&select_apdu).map_err(|e| {
            SlideError::Piv(format!(
                "Failed to select PIV applet on {}: {e}",
                card.reader_name
            ))
        })?;

        Ok(Self {
            card,
            name,
            authenticated: false,
            cached_pin: None,
        })
    }

    /// Re-verify cached PIN immediately before sensitive private key operations.
    /// This is strictly required for slots provisioned with NIST SP 800-73-4 PIN_POLICY = ALWAYS.
    fn ensure_pin_verified(&mut self) -> Result<()> {
        if let Some(ref pin) = self.cached_pin {
            let raw = pin.as_bytes();
            let mut padded = [0xFFu8; 8];
            padded[..raw.len()].copy_from_slice(raw);

            let mut apdu = vec![0x00, 0x20, 0x00, 0x80, 0x08];
            apdu.extend_from_slice(&padded);

            let res = self.card.transmit(&apdu);
            padded.zeroize();
            res.map(|_| ())
        } else {
            Ok(())
        }
    }
}

impl PivCard for PcscPivCard {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn verify_pin(
        &mut self,
        pin: &str,
    ) -> Result<()> {
        let raw = pin.as_bytes();
        if raw.len() > 8 {
            return Err(SlideError::Piv(
                "PIV PIN length cannot exceed 8 characters".to_string(),
            ));
        }

        // PIV PIN padded to 8 bytes with 0xFF
        let mut padded = [0xFFu8; 8];
        padded[..raw.len()].copy_from_slice(raw);

        // APDU: 00 20 00 80 08 [pin_bytes]
        let mut apdu = vec![0x00, 0x20, 0x00, 0x80, 0x08];
        apdu.extend_from_slice(&padded);

        let res = self.card.transmit(&apdu);
        padded.zeroize();

        match res {
            | Ok(_) => {
                self.authenticated = true;
                self.cached_pin = Some(Zeroizing::new(pin.to_string()));
                Ok(())
            },
            | Err(e) => {
                self.authenticated = false;
                self.cached_pin = None;
                let err_str = e.to_string();
                if err_str.contains("0x63C") || err_str.contains("SW=0x63C") {
                    let retries = err_str.chars().last().unwrap_or('?');
                    Err(SlideError::Piv(format!(
                        "YubiKey PIV PIN verification failed: Incorrect PIN ({retries} retries remaining)"
                    )))
                } else if err_str.contains("0x6983") || err_str.contains("SW=0x6983") {
                    Err(SlideError::Piv(
                        "YubiKey PIV PIN is blocked (0 retries remaining). Please unblock using 'ykman piv access unblock-pin'".to_string()
                    ))
                } else {
                    Err(SlideError::Piv(format!(
                        "YubiKey PIV PIN verification failed: {e}"
                    )))
                }
            },
        }
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    fn has_cached_pin(&self) -> bool {
        self.cached_pin.is_some()
    }

    fn check_pin_status(&self) -> Result<PinStatus> {
        // ISO/IEC 7816-4 empty VERIFY APDU: 00 20 00 80 00
        let apdu = [0x00, 0x20, 0x00, 0x80, 0x00];
        match self.card.transmit(&apdu) {
            | Ok(_) => Ok(PinStatus::Verified),
            | Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("0x63C") || err_str.contains("SW=0x63C") {
                    let retries_char = err_str.chars().last().unwrap_or('3');
                    let tries = retries_char.to_digit(10).unwrap_or(3) as u8;
                    Ok(PinStatus::NotVerified {
                        tries_remaining: tries,
                    })
                } else if err_str.contains("0x6983") || err_str.contains("SW=0x6983") {
                    Ok(PinStatus::Blocked)
                } else if err_str.contains("0x6982") || err_str.contains("SW=0x6982") {
                    Ok(PinStatus::NotVerified { tries_remaining: 3 })
                } else {
                    Ok(PinStatus::Unknown)
                }
            },
        }
    }

    fn get_certificate(
        &self,
        slot: PivSlot,
    ) -> Result<Vec<u8>> {
        let oid = slot.object_tag();

        // APDU: 00 CB 3F FF [len] 5C [tag_len] [oid] 00
        let mut apdu = vec![
            0x00,
            0xCB,
            0x3F,
            0xFF,
            (oid.len() + 2) as u8,
            0x5C,
            oid.len() as u8,
        ];
        apdu.extend_from_slice(&oid);
        apdu.push(0x00);

        let data = self.card.transmit(&apdu)?;
        extract_certificate_from_piv_data(&data)
    }

    fn get_public_key(
        &self,
        slot: PivSlot,
    ) -> Result<PivPublicKey> {
        let cert_der = self.get_certificate(slot)?;
        parse_certificate_public_key(&cert_der)
    }

    fn decrypt(
        &mut self,
        slot: PivSlot,
        ciphertext: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>> {
        if !self.authenticated {
            return Err(SlideError::Piv(
                "YubiKey PIV operation failed: User PIN has not been verified".to_string(),
            ));
        }

        let pub_key = self.get_public_key(slot)?;
        let slot_ref = slot.reference_byte();

        // For slots with PIN_POLICY = ALWAYS, ensure PIN is verified immediately prior to INS 0x87
        self.ensure_pin_verified()?;

        use std::io::Write as _;
        eprintln!("\n[PIV] Touch YubiKey sensor to authorize {slot}...");
        let _ = std::io::stderr().flush();

        match pub_key {
            | PivPublicKey::Rsa(_) => {
                // RSA on-chip raw decipher
                let alg_byte = pub_key.piv_algorithm_byte();
                let item_82 = [0x82, 0x00];
                let item_81 = encode_tlv(0x81, ciphertext);
                let mut tmpl_val = Vec::with_capacity(item_82.len() + item_81.len());
                tmpl_val.extend_from_slice(&item_82);
                tmpl_val.extend_from_slice(&item_81);
                let tmpl = encode_tlv(0x7C, &tmpl_val);

                let resp = self
                    .card
                    .send_apdu(0x00, 0x87, alg_byte, slot_ref, &tmpl)
                    .map_err(|e| {
                        let s = e.to_string();
                        if s.contains("0x6982") || s.contains("SW=0x6982") {
                            SlideError::Piv(format!(
                                "YubiKey PIV decryption failed on {slot}: Security condition not satisfied (SW=0x6982). \
                                 Please touch the YubiKey sensor promptly when prompted."
                            ))
                        } else {
                            e
                        }
                    })?;
                let tmpl_resp = find_tlv(&resp, 0x7C).unwrap_or(&resp);
                let deciphered_raw = find_tlv(tmpl_resp, 0x82).ok_or_else(|| {
                    SlideError::Piv("YubiKey decipher response missing Tag 82".to_string())
                })?;

                rsa_oaep_unpad_sha256(deciphered_raw)
            },
            | PivPublicKey::Ecc(_) => {
                // ECC P-256 on-chip ECDH
                if ciphertext.len() < 65 {
                    return Err(SlideError::Crypto(
                        "Invalid ECC ciphertext: missing 65-byte ephemeral public key".to_string(),
                    ));
                }
                let (eph_pub, wrapped_key) = ciphertext.split_at(65);

                let alg_byte = pub_key.piv_algorithm_byte();
                let item_82 = [0x82, 0x00];
                let item_85 = encode_tlv(0x85, eph_pub);
                let mut tmpl_val = Vec::with_capacity(item_82.len() + item_85.len());
                tmpl_val.extend_from_slice(&item_82);
                tmpl_val.extend_from_slice(&item_85);
                let tmpl = encode_tlv(0x7C, &tmpl_val);

                let resp = self
                    .card
                    .send_apdu(0x00, 0x87, alg_byte, slot_ref, &tmpl)
                    .map_err(|e| {
                        let s = e.to_string();
                        if s.contains("0x6982") || s.contains("SW=0x6982") {
                            SlideError::Piv(format!(
                                "YubiKey PIV decryption failed on {slot}: Security condition not satisfied (SW=0x6982). \
                                 Please touch the YubiKey sensor promptly when prompted."
                            ))
                        } else {
                            e
                        }
                    })?;
                let tmpl_resp = find_tlv(&resp, 0x7C).unwrap_or(&resp);
                let shared_secret = find_tlv(tmpl_resp, 0x82).ok_or_else(|| {
                    SlideError::Piv(
                        "YubiKey ECDH response missing Tag 82 shared secret".to_string(),
                    )
                })?;

                let key_array = p256_ecdh_finish_decrypt(shared_secret, wrapped_key)?;
                Ok(Zeroizing::new(key_array.to_vec()))
            },
        }
    }

    fn sign(
        &mut self,
        slot: PivSlot,
        digest_sha3_512: &[u8; 64],
    ) -> Result<Vec<u8>> {
        if !self.authenticated || (slot == PivSlot::Slot9C && self.cached_pin.is_none()) {
            prompt_pin_for_slot(self, slot, None, false)?;
        }

        let pub_key = self.get_public_key(slot)?;
        let slot_ref = slot.reference_byte();

        // For slots with PIN_POLICY = ALWAYS, ensure PIN is verified immediately prior to INS 0x87
        self.ensure_pin_verified()?;

        use std::io::Write as _;
        eprintln!("\n[PIV] Touch YubiKey sensor to authorize {slot}...");
        let _ = std::io::stderr().flush();

        match pub_key {
            | PivPublicKey::Rsa(ref rsa_pub) => {
                let modulus_len = rsa_pub.size();
                let padded = pkcs1v15_pad_sha3_512(digest_sha3_512, modulus_len)?;
                let alg_byte = pub_key.piv_algorithm_byte();

                let item_82 = [0x82, 0x00];
                let item_81 = encode_tlv(0x81, &padded);
                let mut tmpl_val = Vec::with_capacity(item_82.len() + item_81.len());
                tmpl_val.extend_from_slice(&item_82);
                tmpl_val.extend_from_slice(&item_81);
                let tmpl = encode_tlv(0x7C, &tmpl_val);

                let resp = match self.card.send_apdu(0x00, 0x87, alg_byte, slot_ref, &tmpl) {
                    | Ok(r) => r,
                    | Err(e) => {
                        let s = e.to_string();
                        if s.contains("0x6982") || s.contains("SW=0x6982") {
                            prompt_pin_for_slot(self, slot, None, false)?;
                            self.ensure_pin_verified()?;
                            self.card.send_apdu(0x00, 0x87, alg_byte, slot_ref, &tmpl).map_err(|_e2| {
                                SlideError::Piv(format!(
                                    "YubiKey PIV signing failed on {slot}: Security condition not satisfied (SW=0x6982). \
                                     Please touch the YubiKey sensor promptly when prompted."
                                ))
                            })?
                        } else {
                            return Err(e);
                        }
                    },
                };
                let tmpl_resp = find_tlv(&resp, 0x7C).unwrap_or(&resp);
                let sig = find_tlv(tmpl_resp, 0x82).ok_or_else(|| {
                    SlideError::Piv("YubiKey signing response missing Tag 82".to_string())
                })?;
                Ok(sig.to_vec())
            },
            | PivPublicKey::Ecc(_) => {
                // ECC P-256 signs the 32-byte truncated hash
                let truncated_hash = &digest_sha3_512[..32];
                let alg_byte = pub_key.piv_algorithm_byte();

                let item_82 = [0x82, 0x00];
                let item_81 = encode_tlv(0x81, truncated_hash);
                let mut tmpl_val = Vec::with_capacity(item_82.len() + item_81.len());
                tmpl_val.extend_from_slice(&item_82);
                tmpl_val.extend_from_slice(&item_81);
                let tmpl = encode_tlv(0x7C, &tmpl_val);

                let resp = match self.card.send_apdu(0x00, 0x87, alg_byte, slot_ref, &tmpl) {
                    | Ok(r) => r,
                    | Err(e) => {
                        let s = e.to_string();
                        if s.contains("0x6982") || s.contains("SW=0x6982") {
                            prompt_pin_for_slot(self, slot, None, false)?;
                            self.ensure_pin_verified()?;
                            self.card.send_apdu(0x00, 0x87, alg_byte, slot_ref, &tmpl).map_err(|_e2| {
                                SlideError::Piv(format!(
                                    "YubiKey PIV signing failed on {slot}: Security condition not satisfied (SW=0x6982). \
                                     Please touch the YubiKey sensor promptly when prompted."
                                ))
                            })?
                        } else {
                            return Err(e);
                        }
                    },
                };
                let tmpl_resp = find_tlv(&resp, 0x7C).unwrap_or(&resp);
                let sig = find_tlv(tmpl_resp, 0x82).ok_or_else(|| {
                    SlideError::Piv("YubiKey ECDSA response missing Tag 82".to_string())
                })?;
                Ok(sig.to_vec())
            },
        }
    }

    fn list_slots(&self) -> Vec<PivSlot> {
        let candidates = [
            PivSlot::Slot9A,
            PivSlot::Slot9C,
            PivSlot::Slot9D,
            PivSlot::Slot9E,
        ];
        candidates
            .into_iter()
            .filter(|&slot| self.get_certificate(slot).is_ok())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Mock/Test Card for Headless CI / Offline Tests
// ---------------------------------------------------------------------------

/// In-memory mock PIV card strictly used when physical YubiKey is unavailable in automated tests.
#[cfg(any(test, feature = "test-mock", debug_assertions))]
pub struct MockPivCard {
    authenticated: bool,
    pin: String,
    tries_remaining: u8,
    rsa_priv: rsa::RsaPrivateKey,
    ecc_priv: p256::SecretKey,
    slot_certs: std::collections::HashMap<PivSlot, Vec<u8>>,
}

#[cfg(any(test, feature = "test-mock", debug_assertions))]
impl MockPivCard {
    /// Create a self-contained in-memory mock card for automated tests.
    pub fn new(pin: &str) -> Result<Self> {
        use p256::pkcs8::EncodePrivateKey as _;
        use rsa::pkcs8::EncodePrivateKey as _;

        struct MockRng(u64);
        impl rsa::rand_core::RngCore for MockRng {
            fn next_u32(&mut self) -> u32 {
                self.next_u64() as u32
            }

            fn next_u64(&mut self) -> u64 {
                self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
                self.0
            }

            fn fill_bytes(
                &mut self,
                dest: &mut [u8],
            ) {
                for chunk in dest.chunks_mut(8) {
                    let val = self.next_u64().to_le_bytes();
                    let take = chunk.len();
                    chunk.copy_from_slice(&val[..take]);
                }
            }

            fn try_fill_bytes(
                &mut self,
                dest: &mut [u8],
            ) -> std::result::Result<(), rsa::rand_core::Error> {
                self.fill_bytes(dest);
                Ok(())
            }
        }
        impl rsa::rand_core::CryptoRng for MockRng {}

        let mut rng = MockRng(0x0005_11DE_91B1);
        let rsa_priv = rsa::RsaPrivateKey::new(&mut rng, 2048)
            .map_err(|e| SlideError::Crypto(format!("RSA keygen failed: {e}")))?;

        let ecc_bytes = [
            0x45, 0x12, 0x89, 0xfe, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa,
            0xbb, 0xcc, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90, 0xa0, 0xb0, 0xc0,
            0xd0, 0xe0, 0xf0, 0x01,
        ];
        let ecc_priv = p256::SecretKey::from_slice(&ecc_bytes)
            .map_err(|e| SlideError::Crypto(format!("ECC keygen failed: {e}")))?;

        let mut slot_certs = std::collections::HashMap::new();

        // Slot 9D: ECC key management cert
        let pkcs8_ecc = ecc_priv
            .to_pkcs8_der()
            .map_err(|e| SlideError::Crypto(e.to_string()))?;
        let kp_ecc = rcgen::KeyPair::try_from(pkcs8_ecc.as_bytes())
            .map_err(|e| SlideError::Crypto(e.to_string()))?;
        let (ecc_der, _) = super::cert::derive_self_signed_piv_certificate(
            PivSlot::Slot9D,
            "YubiKey Mock 9D",
            None,
            365,
            &kp_ecc,
        )?;
        slot_certs.insert(PivSlot::Slot9D, ecc_der);

        // Slot 9C: RSA signature cert
        let pkcs8_rsa = rsa_priv
            .to_pkcs8_der()
            .map_err(|e| SlideError::Crypto(e.to_string()))?;
        let kp_rsa = rcgen::KeyPair::try_from(pkcs8_rsa.as_bytes())
            .map_err(|e| SlideError::Crypto(e.to_string()))?;
        let (rsa_der, _) = super::cert::derive_self_signed_piv_certificate(
            PivSlot::Slot9C,
            "YubiKey Mock 9C",
            None,
            365,
            &kp_rsa,
        )?;
        slot_certs.insert(PivSlot::Slot9C, rsa_der);

        Ok(Self {
            authenticated: false,
            pin: pin.to_string(),
            tries_remaining: 3,
            rsa_priv,
            ecc_priv,
            slot_certs,
        })
    }
}

#[cfg(any(test, feature = "test-mock", debug_assertions))]
impl PivCard for MockPivCard {
    fn name(&self) -> String {
        "YubiKey Mock PIV Test Device".to_string()
    }

    fn check_pin_status(&self) -> Result<PinStatus> {
        if self.is_authenticated() {
            Ok(PinStatus::Verified)
        } else if self.tries_remaining == 0 {
            Ok(PinStatus::Blocked)
        } else {
            Ok(PinStatus::NotVerified {
                tries_remaining: self.tries_remaining,
            })
        }
    }

    fn verify_pin(
        &mut self,
        pin: &str,
    ) -> Result<()> {
        if self.tries_remaining == 0 {
            return Err(SlideError::Piv(
                "PIN blocked: 0 attempts remaining".to_string(),
            ));
        }
        if pin == self.pin {
            self.authenticated = true;
            self.tries_remaining = 3;
            Ok(())
        } else {
            self.authenticated = false;
            self.tries_remaining = self.tries_remaining.saturating_sub(1);
            Err(SlideError::Piv("PIN verification failed".to_string()))
        }
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    fn get_certificate(
        &self,
        slot: PivSlot,
    ) -> Result<Vec<u8>> {
        self.slot_certs
            .get(&slot)
            .cloned()
            .ok_or_else(|| SlideError::Piv(format!("Slot {slot} has no certificate")))
    }

    fn get_public_key(
        &self,
        slot: PivSlot,
    ) -> Result<PivPublicKey> {
        let cert = self.get_certificate(slot)?;
        parse_certificate_public_key(&cert)
    }

    fn decrypt(
        &mut self,
        slot: PivSlot,
        ciphertext: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>> {
        if !self.authenticated {
            return Err(SlideError::Piv("PIN authentication required".to_string()));
        }
        match slot {
            | PivSlot::Slot9D => {
                let decrypted = super::crypto::p256_ecdh_decrypt(&self.ecc_priv, ciphertext)?;
                Ok(Zeroizing::new(decrypted.to_vec()))
            },
            | _ => super::crypto::rsa_oaep_decrypt(&self.rsa_priv, ciphertext),
        }
    }

    fn sign(
        &mut self,
        slot: PivSlot,
        digest: &[u8; 64],
    ) -> Result<Vec<u8>> {
        if !self.authenticated {
            return Err(SlideError::Piv("PIN authentication required".to_string()));
        }
        match slot {
            | PivSlot::Slot9C => super::crypto::rsa_sign_sha3_512(&self.rsa_priv, digest),
            | _ => super::crypto::p256_ecdsa_sign(&self.ecc_priv, digest),
        }
    }

    fn list_slots(&self) -> Vec<PivSlot> {
        let mut list: Vec<PivSlot> = self.slot_certs.keys().copied().collect();
        list.sort_by_key(PivSlot::as_str);
        list
    }

    fn is_software(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// Global PIV Card Manager (PivManager)
// ---------------------------------------------------------------------------

/// PIV hardware detection and management.
pub struct PivManager;

impl PivManager {
    fn connect_hardware_card(pin: Option<&str>) -> Option<Result<Box<dyn PivCard>>> {
        let context = match PcscContext::establish() {
            | Ok(ctx) => ctx,
            | Err(_) => return None,
        };
        let readers = match context.list_readers() {
            | Ok(r) if !r.is_empty() => r,
            | _ => return None,
        };

        // Prioritize YubiKey reader names, followed by any other connected readers
        let mut sorted_readers = readers;
        sorted_readers.sort_by_key(|r| {
            let lower = r.to_lowercase();
            if lower.contains("yubico") || lower.contains("yubikey") {
                0
            } else {
                1
            }
        });

        for reader in &sorted_readers {
            if let Ok(pcsc_card) = context.connect(reader)
                && let Ok(mut piv_card) = PcscPivCard::connect(pcsc_card)
            {
                if let Some(pin_code) = pin
                    && let Err(e) = piv_card.verify_pin(pin_code)
                {
                    return Some(Err(e));
                }
                return Some(Ok(Box::new(piv_card)));
            }
        }

        None
    }

    /// Connect to a real physical YubiKey or PIV card via PC/SC.
    ///
    /// If a real card is present, authenticates with `pin` if provided.
    /// If no physical smartcard reader is connected and run within automated tests,
    /// falls back to `MockPivCard` to ensure CI testing remains functional.
    pub fn acquire_card(pin: Option<&str>) -> Result<Box<dyn PivCard>> {
        // Explicit mock override for automated tests and CI environments
        #[cfg(any(test, feature = "test-mock", debug_assertions))]
        if std::env::var("CARGO_SLIDE_MOCK_PIV")
            .map(|v| v == "1" || v == "true")
            .unwrap_or(false)
            || (std::env::var("CARGO_TARGET_TMPDIR").is_ok()
                && std::env::var("CARGO_SLIDE_HARDWARE_PIV").is_err())
        {
            let effective_pin = pin.unwrap_or("123456");
            let mut mock = MockPivCard::new(effective_pin)?;
            if pin.is_some() {
                mock.verify_pin(effective_pin)?;
            }
            return Ok(Box::new(mock));
        }

        // 1. Attempt hardware connection over PC/SC
        if let Some(res) = Self::connect_hardware_card(pin) {
            return res;
        }

        // 2. Headless test environment fallback
        #[cfg(any(test, feature = "test-mock", debug_assertions))]
        {
            let effective_pin = pin.unwrap_or("123456");
            let mut mock = MockPivCard::new(effective_pin)?;
            if pin.is_some() {
                mock.verify_pin(effective_pin)?;
            }
            Ok(Box::new(mock))
        }

        #[cfg(not(any(test, feature = "test-mock", debug_assertions)))]
        {
            let service_hint = if cfg!(target_os = "macos") {
                "2. macOS SmartCardServices are running (check 'system_profiler SPSmartCardsDataType' or reset with 'sudo killall com.apple.ctkpcscd').\n"
            } else if cfg!(target_os = "windows") {
                "2. Windows Smart Card service is running ('sc query SCardSvr' or 'Start-Service SCardSvr').\n"
            } else {
                "2. PC/SC smartcard service is running ('systemctl status pcscd' on Linux).\n"
            };
            Err(SlideError::Piv(format!(
                "No YubiKey or PIV smartcard detected.\n\
                 Please ensure:\n\
                 1. Your YubiKey is connected to a USB port.\n\
                 {service_hint}\
                 3. Your PIV slots are provisioned with certificates using Yubico's official tool ('ykman piv info')."
            )))
        }
    }
}

/// Helper to prompt and verify User PIN for a specific PIV slot (e.g. Slot 9C for Digital Signature).
///
/// Order of precedence:
/// 1. Already authenticated session with cached PIN (or non-9c slot).
/// 2. Explicit `pin` parameter if provided.
/// 3. Environment variable `PIV_PIN`.
/// 4. In mock/software mode, auto-authenticates with default test PIN.
/// 5. Graphical desktop dialog popup (prioritized if `prefer_gui` is true or if running non-interactive).
/// 6. Interactive masked input on terminal.
/// 7. Graphical desktop dialog popup fallback if terminal is unavailable.
pub fn prompt_pin_for_slot(
    card: &mut dyn PivCard,
    slot: PivSlot,
    pin: Option<&str>,
    prefer_gui: bool,
) -> Result<()> {
    let is_signing_slot = slot == PivSlot::Slot9C;

    // 1. If already authenticated and card has cached PIN ready for NIST SP 800-73 re-verification
    if card.is_authenticated() && (card.has_cached_pin() || !is_signing_slot) {
        return Ok(());
    }

    match card.check_pin_status() {
        | Ok(PinStatus::Verified) if card.has_cached_pin() || !is_signing_slot => return Ok(()),
        | Ok(PinStatus::Blocked) => {
            return Err(SlideError::Piv(
                "YubiKey PIV PIN is blocked (0 retries remaining). Please unblock using 'ykman piv access unblock-pin'".to_string(),
            ));
        },
        | _ => {},
    }

    // 2. Explicit argument
    if let Some(p) = pin {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            return card.verify_pin(trimmed);
        }
    }

    // 3. Environment variable
    if let Ok(env_pin) = std::env::var("PIV_PIN") {
        let trimmed = env_pin.trim();
        if !trimmed.is_empty() {
            return card.verify_pin(trimmed);
        }
    }

    // 4. Mock test card fallback (for headless CI / automated tests)
    if card.is_software() {
        return card.verify_pin("123456");
    }

    let base_title = if is_signing_slot {
        "YubiKey PIV - Slot 9C Digital Signature"
    } else {
        "Cargo Slide - YubiKey PIV Authentication"
    };

    let base_prompt_desc = if is_signing_slot {
        "authorize Slot 9C Digital Signature"
    } else {
        "decrypt presentation"
    };

    let has_gui_desktop = std::env::var("DISPLAY").is_ok()
        || std::env::var("WAYLAND_DISPLAY").is_ok()
        || cfg!(target_os = "macos")
        || cfg!(target_os = "windows");

    let should_try_gui = prefer_gui || (has_gui_desktop && !std::io::stdin().is_terminal());

    // 5. Native OS graphical dialog prompt (zenity / kdialog / osascript / PowerShell)
    if should_try_gui {
        for attempt in 0..3 {
            let status = card.check_pin_status();
            let attempts_left = match status {
                | Ok(PinStatus::NotVerified { tries_remaining }) => {
                    (tries_remaining as usize).min(3 - attempt)
                },
                | _ => 3 - attempt,
            };
            let attempts_str = format!(" ({attempts_left} attempts remaining)");
            let title = if attempt == 0 {
                base_title
            } else {
                "PIN Verification Failed - Please Re-enter"
            };
            let subtitle = if attempt == 0 {
                format!("Enter 6-8 digit User PIN to {base_prompt_desc}{attempts_str}:")
            } else {
                format!("Incorrect PIN. Please re-enter{attempts_str}:")
            };

            if let Some(pin_val) = try_native_desktop_dialog(title, &subtitle) {
                match card.verify_pin(&pin_val) {
                    | Ok(()) => return Ok(()),
                    | Err(e) => {
                        if attempt == 2 {
                            return Err(e);
                        }
                    },
                }
            } else {
                break;
            }
        }
    }

    // 6. Interactive prompt on terminal with masked input, retry loop and zeroization
    if std::io::stdin().is_terminal() {
        for attempt in 0..3 {
            let status = card.check_pin_status();
            let attempts_left = match status {
                | Ok(PinStatus::NotVerified { tries_remaining }) => {
                    (tries_remaining as usize).min(3 - attempt)
                },
                | _ => 3 - attempt,
            };
            let attempts_str = format!(" ({attempts_left} attempts remaining)");
            let prompt = if attempt == 0 {
                format!("Enter YubiKey PIV PIN to {base_prompt_desc}{attempts_str}: ")
            } else {
                format!("\x1b[33mIncorrect PIN. Please re-enter{attempts_str}:\x1b[0m ")
            };
            if let Ok(pin_val) = super::masked_input::read_masked_secret(&prompt) {
                match card.verify_pin(&pin_val) {
                    | Ok(()) => return Ok(()),
                    | Err(e) => {
                        if attempt == 2 {
                            return Err(e);
                        }
                    },
                }
            } else {
                break;
            }
        }
    } else if !should_try_gui {
        // Fallback graphical dialog prompt if running non-interactive without terminal
        for attempt in 0..3 {
            let attempts_left = match card.check_pin_status() {
                | Ok(PinStatus::NotVerified { tries_remaining }) => {
                    (tries_remaining as usize).min(3 - attempt)
                },
                | _ => 3 - attempt,
            };
            let attempts_str = format!(" ({attempts_left} attempts remaining)");
            let title = if attempt == 0 {
                base_title
            } else {
                "PIN Verification Failed - Please Re-enter"
            };
            let subtitle = if attempt == 0 {
                format!("Enter 6-8 digit User PIN to {base_prompt_desc}{attempts_str}:")
            } else {
                format!("Incorrect PIN. Please re-enter{attempts_str}:")
            };
            if let Some(pin_val) = try_native_desktop_dialog(title, &subtitle) {
                match card.verify_pin(&pin_val) {
                    | Ok(()) => return Ok(()),
                    | Err(e) => {
                        if attempt == 2 {
                            return Err(e);
                        }
                    },
                }
            } else {
                break;
            }
        }
    }

    Err(SlideError::Piv(format!(
        "YubiKey PIV PIN required to {base_prompt_desc}. Please specify --pin <PIN> or set PIV_PIN environment variable."
    )))
}

/// Helper to ensure a PIV card is authenticated with User PIN before private key operations.
pub fn prompt_pin_if_needed(
    card: &mut dyn PivCard,
    pin: Option<&str>,
) -> Result<()> {
    prompt_pin_for_slot(card, PivSlot::Slot9D, pin, false)
}

/// Prompt the user for a secret (PIN, passphrase, or mnemonic) via a native OS graphical dialog
/// (zenity / kdialog on Linux, osascript on macOS, PowerShell on Windows) with masked input.
pub fn prompt_secret_gui(
    title: &str,
    prompt: &str,
) -> Option<Zeroizing<String>> {
    try_native_desktop_dialog(title, prompt)
}

fn try_native_desktop_dialog(
    title: &str,
    prompt: &str,
) -> Option<Zeroizing<String>> {
    #[cfg(target_os = "windows")]
    {
        use base64::Engine as _;
        let script = "Add-Type -AssemblyName System.Windows.Forms; \
             Add-Type -AssemblyName System.Drawing; \
             $form = New-Object System.Windows.Forms.Form; \
             $form.Text = [Environment]::GetEnvironmentVariable('PIN_TITLE'); \
             $form.Size = New-Object System.Drawing.Size(440, 190); \
             $form.StartPosition = 'CenterScreen'; \
             $form.FormBorderStyle = 'FixedDialog'; \
             $form.MaximizeBox = $false; \
             $form.MinimizeBox = $false; \
             $form.TopMost = $true; \
             $lbl = New-Object System.Windows.Forms.Label; \
             $lbl.Location = New-Object System.Drawing.Point(20, 16); \
             $lbl.Size = New-Object System.Drawing.Size(390, 42); \
             $lbl.Text = [Environment]::GetEnvironmentVariable('PIN_PROMPT'); \
             $form.Controls.Add($lbl); \
             $txt = New-Object System.Windows.Forms.TextBox; \
             $txt.Location = New-Object System.Drawing.Point(20, 66); \
             $txt.Size = New-Object System.Drawing.Size(385, 25); \
             $txt.UseSystemPasswordChar = $true; \
             $form.Controls.Add($txt); \
             $btnOk = New-Object System.Windows.Forms.Button; \
             $btnOk.Location = New-Object System.Drawing.Point(225, 106); \
             $btnOk.Size = New-Object System.Drawing.Size(85, 28); \
             $btnOk.Text = 'OK'; \
             $btnOk.DialogResult = [System.Windows.Forms.DialogResult]::OK; \
             $form.AcceptButton = $btnOk; \
             $form.Controls.Add($btnOk); \
             $btnCancel = New-Object System.Windows.Forms.Button; \
             $btnCancel.Location = New-Object System.Drawing.Point(320, 106); \
             $btnCancel.Size = New-Object System.Drawing.Size(85, 28); \
             $btnCancel.Text = 'Cancel'; \
             $btnCancel.DialogResult = [System.Windows.Forms.DialogResult]::Cancel; \
             $form.CancelButton = $btnCancel; \
             $form.Controls.Add($btnCancel); \
             $form.Add_Shown({ $form.Activate(); $txt.Focus() }); \
             if ($form.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { \
                 [Console]::Out.Write($txt.Text) \
             }";

        let utf16_bytes: Vec<u8> = script
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        let encoded_cmd = base64::engine::general_purpose::STANDARD.encode(&utf16_bytes);

        if let Ok(mut out) = std::process::Command::new("powershell")
            .env("PIN_TITLE", title)
            .env("PIN_PROMPT", prompt)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-EncodedCommand",
                &encoded_cmd,
            ])
            .output()
            && out.status.success()
        {
            let raw_str = String::from_utf8_lossy(&out.stdout);
            let trimmed = raw_str.trim_matches(|c| c == '\r' || c == '\n');
            let result = if !trimmed.is_empty() {
                Some(Zeroizing::new(trimmed.to_string()))
            } else {
                None
            };
            out.stdout.zeroize();
            if result.is_some() {
                return result;
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Use "text returned of (display dialog ...)" directly via Standard Additions.
        // Do NOT wrap in "tell application System Events" — that triggers macOS TCC
        // Automation permission popup (Mojave+) which users may deny, breaking the dialog.
        let script = "text returned of (display dialog (system attribute \"PIN_PROMPT\") default answer \"\" with hidden answer with title (system attribute \"PIN_TITLE\"))";
        if let Ok(mut out) = std::process::Command::new("osascript")
            .env("PIN_TITLE", title)
            .env("PIN_PROMPT", prompt)
            .args(["-e", &script])
            .output()
        {
            if out.status.success() {
                // With "text returned of (...)", stdout is the raw secret text followed by a newline.
                let mut pin = String::from_utf8_lossy(&out.stdout)
                    .trim_end_matches('\n')
                    .to_string();
                if !pin.is_empty() {
                    let res = Zeroizing::new(pin.clone());
                    pin.zeroize();
                    out.stdout.zeroize();
                    return Some(res);
                }
            }
            out.stdout.zeroize();
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok() {
            let zenity_title = format!("--title={title}");
            let zenity_text = format!("--text={prompt}");
            if let Ok(mut out) = std::process::Command::new("zenity")
                .args(["--entry", "--hide-text", &zenity_title, &zenity_text])
                .output()
            {
                if out.status.success() {
                    let mut s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !s.is_empty() {
                        let res = Zeroizing::new(s.clone());
                        s.zeroize();
                        out.stdout.zeroize();
                        return Some(res);
                    }
                }
                out.stdout.zeroize();
            } else if let Ok(mut out) = std::process::Command::new("zenity")
                .args(["--password", &zenity_title])
                .output()
            {
                if out.status.success() {
                    let mut s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !s.is_empty() {
                        let res = Zeroizing::new(s.clone());
                        s.zeroize();
                        out.stdout.zeroize();
                        return Some(res);
                    }
                }
                out.stdout.zeroize();
            }

            if let Ok(mut out) = std::process::Command::new("kdialog")
                .args(["--password", prompt, "--title", title])
                .output()
            {
                if out.status.success() {
                    let mut s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !s.is_empty() {
                        let res = Zeroizing::new(s.clone());
                        s.zeroize();
                        out.stdout.zeroize();
                        return Some(res);
                    }
                }
                out.stdout.zeroize();
            }
        }
    }

    None
}
