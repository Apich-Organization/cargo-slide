use slide_core::error::SlideError;
use slide_core::model::Rect;
use slide_core::model::Slide;
use slide_core::model::SlideDeck;
use slide_core::security::MemoryAssetStore;
use slide_core::security::cert::PivSlot;
use slide_core::security::container::EncryptedSlideContainer;
use slide_core::security::crypto::aes_256_gcm_decrypt;
use slide_core::security::crypto::aes_256_gcm_encrypt;
use slide_core::security::crypto::compute_sha3_512;
use slide_core::security::crypto::generate_aes_key;
use slide_core::security::piv::PivManager;

#[test]
fn test_aes_256_gcm_encryption_decryption() {
    let key = generate_aes_key();
    let plaintext = b"Confidential Presentation Slide Content: Apich Financial Report 2026";

    let (ciphertext, nonce, tag) = aes_256_gcm_encrypt(&key, plaintext).unwrap();
    assert_ne!(ciphertext, plaintext);
    assert_eq!(nonce.len(), 12);
    assert_eq!(tag.len(), 16);

    let decrypted = aes_256_gcm_decrypt(&key, &nonce, &tag, &ciphertext).unwrap();
    assert_eq!(decrypted.as_slice(), plaintext);

    // Tamper test: 1 bit modified in ciphertext should fail authentication
    let mut tampered = ciphertext.clone();
    tampered[0] ^= 0x01;
    let err = aes_256_gcm_decrypt(&key, &nonce, &tag, &tampered);
    assert!(
        err.is_err(),
        "Decryption must fail when ciphertext is modified"
    );
}

#[test]
fn test_sha3_512_digest() {
    let data = b"SlideDeck Integrity Content";
    let digest1 = compute_sha3_512(data);
    let digest2 = compute_sha3_512(data);
    assert_eq!(digest1, digest2);
    assert_eq!(digest1.len(), 64);

    let diff_data = b"SlideDeck Integrity Content Modified";
    let diff_digest = compute_sha3_512(diff_data);
    assert_ne!(digest1, diff_digest);
}

#[test]
fn test_piv_hardware_interface_and_crypto() {
    unsafe {
        std::env::set_var("CARGO_SLIDE_MOCK_PIV", "1");
    }
    let mut card = PivManager::acquire_card(Some("123456")).unwrap();
    assert!(card.is_authenticated());

    let slots = card.list_slots();
    assert!(slots.contains(&PivSlot::Slot9D), "Slot 9D must be present");
    assert!(slots.contains(&PivSlot::Slot9C), "Slot 9C must be present");

    // 1. Slot 9D Key Management (Decryption / Decapsulation)
    let pub_9d = card.get_public_key(PivSlot::Slot9D).unwrap();
    let session_key = generate_aes_key();

    let ciphertext = match pub_9d {
        | slide_core::security::PivPublicKey::Ecc(ref ecc_pub) => {
            slide_core::security::crypto::p256_ecdh_encrypt(ecc_pub, &session_key).unwrap()
        },
        | slide_core::security::PivPublicKey::Rsa(ref rsa_pub) => {
            slide_core::security::crypto::rsa_oaep_encrypt(rsa_pub, session_key.as_ref()).unwrap()
        },
    };

    let decrypted = card.decrypt(PivSlot::Slot9D, &ciphertext).unwrap();
    assert_eq!(decrypted.as_slice(), session_key.as_ref());

    // 2. Slot 9C Digital Signature over SHA3-512
    let digest = compute_sha3_512(b"Authentic slide deck payload hash");
    let signature = card.sign(PivSlot::Slot9C, &digest).unwrap();

    let pub_9c = card.get_public_key(PivSlot::Slot9C).unwrap();
    match pub_9c {
        | slide_core::security::PivPublicKey::Rsa(ref rsa_pub) => {
            slide_core::security::crypto::rsa_verify_sha3_512(rsa_pub, &digest, &signature)
                .unwrap();
        },
        | slide_core::security::PivPublicKey::Ecc(ref ecc_pub) => {
            slide_core::security::crypto::p256_ecdsa_verify(ecc_pub, &digest, &signature).unwrap();
        },
    }
}

#[test]
fn test_piv_slot9c_prompt_pin_for_slot() {
    unsafe {
        std::env::set_var("CARGO_SLIDE_MOCK_PIV", "1");
    }
    let mut card = PivManager::acquire_card(None).unwrap();
    // Verify prompt_pin_for_slot authenticates for Slot 9C
    slide_core::security::prompt_pin_for_slot(
        card.as_mut(),
        PivSlot::Slot9C,
        Some("123456"),
        false,
    )
    .unwrap();
    assert!(card.is_authenticated());

    let digest = compute_sha3_512(b"Test Slot 9C Signing Payload");
    let sig = card.sign(PivSlot::Slot9C, &digest).unwrap();
    assert!(!sig.is_empty());
}

#[test]
fn test_encrypted_container_pack_and_decrypt_in_memory() {
    unsafe {
        std::env::set_var("CARGO_SLIDE_MOCK_PIV", "1");
    }
    let mut card = PivManager::acquire_card(Some("123456")).unwrap();
    let recipient_pub = card.get_public_key(PivSlot::Slot9D).unwrap();

    // Construct a sample slide deck
    let mut deck = SlideDeck::new("Encrypted Secret Presentation");
    deck.slides.push(Slide {
        page_number: 1,
        svg_data: r#"<svg xmlns="http://www.w3.org/2000/svg" width="1280" height="720"><text x="100" y="100">Top Secret Slide</text><image href="assets/logo.png" x="10" y="10" width="100" height="100"/></svg>"#.to_string(),
        view_box: Rect::new(0.0, 0.0, 1280.0, 720.0),
        hotspots: Vec::new(),
        animation: Some("fade".to_string()),
        steps: Vec::new(),
        notes: Some("Secret speaking notes".to_string()),
    });

    // Create a temporary directory with an asset file
    let tmp = tempfile::tempdir().unwrap();
    let asset_dir = tmp.path().join("assets");
    std::fs::create_dir_all(&asset_dir).unwrap();
    std::fs::write(asset_dir.join("logo.png"), b"FAKE_PNG_BINARY_BYTES").unwrap();

    // Pack into encrypted container
    let encrypted_bytes = EncryptedSlideContainer::pack(
        &deck,
        Some(tmp.path()),
        &recipient_pub,
        PivSlot::Slot9D,
        card.as_mut(),
        PivSlot::Slot9C,
    )
    .unwrap();

    assert!(EncryptedSlideContainer::is_encrypted_package(
        &encrypted_bytes
    ));

    // Parse container
    let container = EncryptedSlideContainer::parse(&encrypted_bytes).unwrap();
    assert_eq!(container.header.title, "Encrypted Secret Presentation");
    assert_eq!(container.header.total_slides, 1);
    assert_eq!(container.header.integrity_algorithm, "SHA3-512");

    // Verify SHA3-512 integrity
    container.verify_integrity(None).unwrap();

    // Decrypt directly into memory
    let (decrypted_deck, asset_store) = container.decrypt_in_memory(card.as_mut(), None).unwrap();
    assert_eq!(decrypted_deck.title, "Encrypted Secret Presentation");
    assert_eq!(decrypted_deck.total_slides(), 1);

    // Verify in-memory asset store contains the asset
    assert!(asset_store.contains("assets/logo.png") || asset_store.contains("logo.png"));
    let logo_data = asset_store.get("assets/logo.png").unwrap();
    assert_eq!(logo_data, b"FAKE_PNG_BINARY_BYTES");

    // Verify image inlining in SVG
    assert!(
        decrypted_deck.slides[0]
            .svg_data
            .contains("data:image/png;base64,"),
        "SVG image reference should be inlined as data-URI"
    );
}

#[test]
fn test_integrity_tamper_detection_triggers_error() {
    unsafe {
        std::env::set_var("CARGO_SLIDE_MOCK_PIV", "1");
    }
    let mut card = PivManager::acquire_card(Some("123456")).unwrap();
    let recipient_pub = card.get_public_key(PivSlot::Slot9D).unwrap();

    let deck = SlideDeck::new("Tamper Verification Presentation");
    let encrypted_bytes = EncryptedSlideContainer::pack(
        &deck,
        None,
        &recipient_pub,
        PivSlot::Slot9D,
        card.as_mut(),
        PivSlot::Slot9C,
    )
    .unwrap();

    let mut container = EncryptedSlideContainer::parse(&encrypted_bytes).unwrap();

    // Tamper with 1 byte of the ciphertext
    let len = container.ciphertext.len();
    container.ciphertext[len / 2] ^= 0x55;

    // Integrity check must fail with SlideError::IntegrityCheckFailed
    let err = container.verify_integrity(None);
    assert!(
        matches!(err, Err(SlideError::IntegrityCheckFailed(_))),
        "Must detect ciphertext tampering via SHA3-512 and reject"
    );

    // Decryption must also fail immediately without attempting key decipher
    let decrypt_err = container.decrypt_in_memory(card.as_mut(), None);
    assert!(
        matches!(decrypt_err, Err(SlideError::IntegrityCheckFailed(_))),
        "Decryption must abort if SHA3-512 integrity verification fails"
    );
}

#[test]
fn test_memory_asset_store_zeroization() {
    let store = MemoryAssetStore::new();
    store.insert("confidential/report.pdf", b"SECRET_PDF_BYTES".to_vec());
    store.insert("logo.png", b"LOGO_BYTES".to_vec());

    assert_eq!(store.len(), 2);
    assert!(store.contains("confidential/report.pdf"));
    assert!(store.contains("logo.png"));

    let data = store.get("confidential/report.pdf").unwrap();
    assert_eq!(data, b"SECRET_PDF_BYTES");

    store.clear();
    assert_eq!(store.len(), 0);
    assert!(store.is_empty());
    assert!(store.get("confidential/report.pdf").is_none());
}

#[test]
fn test_software_passphrase_container_pack_and_decrypt() {
    let mut deck = SlideDeck::new("Secret Passphrase Presentation");
    deck.slides.push(Slide {
        page_number: 1,
        svg_data: "<svg><text>Encrypted by Passphrase</text></svg>".to_string(),
        view_box: Rect::new(0.0, 0.0, 1920.0, 1080.0),
        hotspots: Vec::new(),
        animation: None,
        steps: Vec::new(),
        notes: None,
    });

    let pass = "StrongP@ssphrase42!";
    let (pkg_bytes, _) = EncryptedSlideContainer::pack_software(
        &deck,
        None,
        pass,
        slide_core::security::AuthMode::Passphrase,
        None,
    )
    .expect("Software pack with passphrase");

    assert!(EncryptedSlideContainer::is_encrypted_package(&pkg_bytes));

    let container = EncryptedSlideContainer::parse(&pkg_bytes).expect("parse container");
    assert_eq!(
        container.header.auth_mode,
        slide_core::security::AuthMode::Passphrase
    );

    // Verify integrity succeeds
    assert!(container.verify_integrity(None).is_ok());

    // Decrypt with correct passphrase
    let (decrypted_deck, _assets) = container
        .decrypt_with_secret(pass)
        .expect("decrypt with correct passphrase");
    assert_eq!(decrypted_deck.title, "Secret Passphrase Presentation");
    assert_eq!(decrypted_deck.slides.len(), 1);

    // Decrypt with incorrect passphrase must fail
    assert!(container.decrypt_with_secret("WrongP@ssword99!").is_err());
}

#[test]
fn test_software_bip39_mnemonic_container_pack_and_decrypt() {
    let mut deck = SlideDeck::new("Secret Mnemonic Presentation");
    deck.slides.push(Slide {
        page_number: 1,
        svg_data: "<svg><text>Encrypted by BIP-39 Mnemonic</text></svg>".to_string(),
        view_box: Rect::new(0.0, 0.0, 1920.0, 1080.0),
        hotspots: Vec::new(),
        animation: None,
        steps: Vec::new(),
        notes: None,
    });

    // Auto-generate 12-word recovery mnemonic
    let (pkg_bytes, generated_mnemonic) = EncryptedSlideContainer::pack_software(
        &deck,
        None,
        "",
        slide_core::security::AuthMode::Mnemonic,
        Some(12),
    )
    .expect("Software pack with auto-generated mnemonic");

    let mnemonic = generated_mnemonic.expect("generated mnemonic present");
    assert_eq!(mnemonic.split_whitespace().count(), 12);

    let container = EncryptedSlideContainer::parse(&pkg_bytes).expect("parse container");
    assert_eq!(
        container.header.auth_mode,
        slide_core::security::AuthMode::Mnemonic
    );

    // Decrypt with generated mnemonic
    let (decrypted_deck, _) = container
        .decrypt_with_secret(&mnemonic)
        .expect("decrypt with generated mnemonic");
    assert_eq!(decrypted_deck.title, "Secret Mnemonic Presentation");

    // Decrypt with corrupted mnemonic must fail
    assert!(
        container
            .decrypt_with_secret(
                "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon"
            )
            .is_err()
    );

    // Auto-generate 24-word recovery mnemonic
    let (pkg_bytes_24, generated_24) = EncryptedSlideContainer::pack_software(
        &deck,
        None,
        "24",
        slide_core::security::AuthMode::Mnemonic,
        Some(24),
    )
    .expect("Software pack with 24-word auto-generated mnemonic");

    let mnemonic_24 = generated_24.expect("generated 24-word mnemonic present");
    assert_eq!(mnemonic_24.split_whitespace().count(), 24);

    let container_24 = EncryptedSlideContainer::parse(&pkg_bytes_24).expect("parse container 24");
    let (decrypted_24, _) = container_24
        .decrypt_with_secret(&mnemonic_24)
        .expect("decrypt with 24-word mnemonic");
    assert_eq!(decrypted_24.title, "Secret Mnemonic Presentation");
}

#[test]
fn test_piv_retry_counter_decrement() {
    use slide_core::security::piv::MockPivCard;
    use slide_core::security::piv::PinStatus;
    use slide_core::security::piv::PivCard;

    let mut card = MockPivCard::new("123456").expect("create MockPivCard");
    assert_eq!(
        card.check_pin_status().unwrap(),
        PinStatus::NotVerified { tries_remaining: 3 }
    );

    // Attempt 1: wrong PIN
    assert!(card.verify_pin("000000").is_err());
    assert_eq!(
        card.check_pin_status().unwrap(),
        PinStatus::NotVerified { tries_remaining: 2 }
    );

    // Attempt 2: wrong PIN
    assert!(card.verify_pin("111111").is_err());
    assert_eq!(
        card.check_pin_status().unwrap(),
        PinStatus::NotVerified { tries_remaining: 1 }
    );

    // Attempt 3: wrong PIN
    assert!(card.verify_pin("222222").is_err());
    assert_eq!(card.check_pin_status().unwrap(), PinStatus::Blocked);

    // Blocked card rejects further attempts
    assert!(card.verify_pin("123456").is_err());
}
