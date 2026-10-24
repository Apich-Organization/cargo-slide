//! Compile-time string and byte slice obfuscation with memory-zeroized runtime deobfuscation.
//!
//! Prevents sensitive magic headers, payload delimiters, error messages, and internal
//! identifiers from appearing in plaintext within binary `.rodata` or the output of `strings`.

use zeroize::Zeroizing;

/// Linear congruential generator parameters for compile-time and runtime key streaming.
const LCG_MULTIPLIER: u64 = 6364136223846793005;
const LCG_ADDEND: u64 = 1442695040888963407;

/// Deobfuscate an XOR-masked byte slice with a single-byte key into a zeroized String.
/// Preserved for backward compatibility.
#[must_use]
pub fn deobfuscate_str(
    encrypted: &[u8],
    key: u8,
) -> Zeroizing<String> {
    let mut decrypted = Vec::with_capacity(encrypted.len());
    for &b in encrypted {
        decrypted.push(b ^ key);
    }
    let s = String::from_utf8(decrypted).unwrap_or_default();
    Zeroizing::new(s)
}

/// Deobfuscate an LCG-masked byte slice into a zeroized String.
#[must_use]
pub fn deobfuscate_stream(
    encrypted: &[u8],
    seed: u64,
) -> Zeroizing<String> {
    let mut decrypted = Vec::with_capacity(encrypted.len());
    let mut state = seed;
    for &b in encrypted {
        state = state.wrapping_mul(LCG_MULTIPLIER).wrapping_add(LCG_ADDEND);
        let k = ((state >> 33) ^ state) as u8;
        decrypted.push(b ^ k);
    }
    let s = String::from_utf8(decrypted).unwrap_or_default();
    Zeroizing::new(s)
}

/// Deobfuscate an LCG-masked byte slice into a zeroized byte buffer.
#[must_use]
pub fn deobfuscate_bytes_stream(
    encrypted: &[u8],
    seed: u64,
) -> Zeroizing<Vec<u8>> {
    let mut decrypted = Vec::with_capacity(encrypted.len());
    let mut state = seed;
    for &b in encrypted {
        state = state.wrapping_mul(LCG_MULTIPLIER).wrapping_add(LCG_ADDEND);
        let k = ((state >> 33) ^ state) as u8;
        decrypted.push(b ^ k);
    }
    Zeroizing::new(decrypted)
}

/// Macro for compile-time string obfuscation with rolling multi-byte XOR key stream.
#[macro_export]
macro_rules! obf_str {
    ($s:expr) => {{
        const S: &str = $s;
        const LEN: usize = S.len();
        const SEED: u64 = 0x517c_c1b7_2722_0a95 ^ (LEN as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        const ENCRYPTED: [u8; LEN] = {
            let bytes = S.as_bytes();
            let mut arr = [0u8; LEN];
            let mut i = 0;
            let mut state = SEED;
            while i < LEN {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let k = ((state >> 33) ^ state) as u8;
                arr[i] = bytes[i] ^ k;
                i += 1;
            }
            arr
        };
        $crate::security::obfuscation::deobfuscate_stream(&ENCRYPTED, SEED)
    }};
}

/// Macro for compile-time byte slice obfuscation with rolling multi-byte XOR key stream.
#[macro_export]
macro_rules! obf_bytes {
    ($b:expr) => {{
        const B: &[u8] = $b;
        const LEN: usize = B.len();
        const SEED: u64 = 0x85eb_ca6b_3b84_5eab ^ (LEN as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        const ENCRYPTED: [u8; LEN] = {
            let mut arr = [0u8; LEN];
            let mut i = 0;
            let mut state = SEED;
            while i < LEN {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let k = ((state >> 33) ^ state) as u8;
                arr[i] = B[i] ^ k;
                i += 1;
            }
            arr
        };
        $crate::security::obfuscation::deobfuscate_bytes_stream(&ENCRYPTED, SEED)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obf_str() {
        let dec = obf_str!("ApichPresentationSecretHeader");
        assert_eq!(*dec, "ApichPresentationSecretHeader");
    }

    #[test]
    fn test_obf_bytes() {
        let dec = obf_bytes!(b"SLIDECRYPT\x01");
        assert_eq!(&dec[..], b"SLIDECRYPT\x01");
    }

    #[test]
    fn test_deobfuscate_str_legacy() {
        let enc = [b'A' ^ 0x6B, b'B' ^ 0x6B];
        let dec = deobfuscate_str(&enc, 0x6B);
        assert_eq!(*dec, "AB");
    }
}
