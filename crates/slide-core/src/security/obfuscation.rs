//! Compile-time string obfuscation and memory-zeroized runtime deobfuscation.
//!
//! Prevents sensitive magic headers, error messages, and internal identifiers from
//! appearing in plaintext within binary `.rodata` or the output of `strings`.

use zeroize::Zeroizing;

/// Deobfuscate an XOR-masked byte slice into a zeroized String.
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

/// Macro for compile-time string obfuscation.
#[macro_export]
macro_rules! obf_str {
    ($s:expr) => {{
        const S: &str = $s;
        const KEY: u8 = 0x6B;
        const LEN: usize = S.len();
        const ENCRYPTED: [u8; LEN] = {
            let bytes = S.as_bytes();
            let mut arr = [0u8; LEN];
            let mut i = 0;
            while i < LEN {
                arr[i] = bytes[i] ^ KEY;
                i += 1;
            }
            arr
        };
        $crate::security::obfuscation::deobfuscate_str(&ENCRYPTED, KEY)
    }};
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_obf_str() {
        let dec = obf_str!("ApichPresentationSecretHeader");
        assert_eq!(*dec, "ApichPresentationSecretHeader");
    }
}
