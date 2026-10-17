//! Cryptographic protection, YubiKey PIV slot key binding, SHA3-512 integrity verification,
//! and in-memory zeroization for cargo-slide presentations.

pub mod asset_store;
pub mod cert;
pub mod container;
pub mod crypto;
pub mod pcsc;
pub mod piv;

pub use asset_store::MemoryAssetStore;
pub use cert::PivCertInfo;
pub use cert::PivKeyAlgorithm;
pub use cert::PivPublicKey;
pub use cert::PivSlot;
pub use cert::cert_der_to_pem;
pub use cert::derive_self_signed_piv_certificate;
pub use cert::parse_certificate_info;
pub use cert::parse_certificate_public_key;
pub use container::ENCRYPTED_CONTAINER_MAGIC;
pub use container::EncryptedContainerHeader;
pub use container::EncryptedSlideContainer;
pub use crypto::aes_256_gcm_decrypt;
pub use crypto::aes_256_gcm_encrypt;
pub use crypto::compute_sha3_512;
pub use crypto::generate_aes_key;
pub use pcsc::PcscCard;
pub use pcsc::PcscContext;
pub use piv::PcscPivCard;
pub use piv::PivCard;
pub use piv::PivManager;
pub use piv::prompt_pin_if_needed;
pub use zeroize::Zeroize;
pub use zeroize::Zeroizing;
