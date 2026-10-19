use std::process::Command;
use tempfile::tempdir;

fn slide_cmd(bin: &str) -> Command {
    let mut cmd = Command::new(bin);
    cmd.env("CARGO_SLIDE_MOCK_PIV", "1");
    cmd
}

#[test]
fn test_piv_management_commands() {
    let bin = env!("CARGO_BIN_EXE_cargo-slide");
    let dir = tempdir().expect("tempdir");

    // 1. Inspect PIV info
    let info_res = slide_cmd(bin)
        .arg("piv")
        .arg("info")
        .output()
        .expect("cargo-slide piv info");
    assert!(
        info_res.status.success(),
        "piv info failed: {:?}",
        String::from_utf8_lossy(&info_res.stderr)
    );
    let info_out = String::from_utf8_lossy(&info_res.stdout);
    assert!(info_out.contains("Cargo Slide YubiKey PIV Smartcard Inspector"));
    assert!(info_out.contains("Slot 9d") || info_out.contains("Configured PIV Slots"));

    // 2. Export slot 9d certificate to PEM
    let cert_out = dir.path().join("slot9d_cert.pem");
    let export_res = slide_cmd(bin)
        .arg("piv")
        .arg("export-cert")
        .arg("--slot")
        .arg("9d")
        .arg("--output")
        .arg(&cert_out)
        .output()
        .expect("cargo-slide piv export-cert");
    assert!(
        export_res.status.success(),
        "export-cert failed: {:?}",
        String::from_utf8_lossy(&export_res.stderr)
    );
    assert!(cert_out.exists());
    let pem_text = std::fs::read_to_string(&cert_out).unwrap();
    assert!(pem_text.contains("-----BEGIN CERTIFICATE-----"));

    // 3. Sign an arbitrary payload with SHA3-512 on slot 9c
    let test_file = dir.path().join("payload.txt");
    std::fs::write(&test_file, b"Sensitive presentation integrity test payload").unwrap();
    let sig_file = dir.path().join("payload.sig");

    let sign_res = slide_cmd(bin)
        .arg("piv")
        .arg("sign")
        .arg(&test_file)
        .arg("--slot")
        .arg("9c")
        .arg("--output")
        .arg(&sig_file)
        .output()
        .expect("cargo-slide piv sign");
    assert!(
        sign_res.status.success(),
        "sign failed: {:?}",
        String::from_utf8_lossy(&sign_res.stderr)
    );
    assert!(sig_file.exists());

    // 4. Verify SHA3-512 signature
    let verify_res = slide_cmd(bin)
        .arg("piv")
        .arg("verify")
        .arg(&test_file)
        .arg("--slot")
        .arg("9c")
        .arg("--sig")
        .arg(&sig_file)
        .output()
        .expect("cargo-slide piv verify");
    assert!(
        verify_res.status.success(),
        "verify failed: {:?}",
        String::from_utf8_lossy(&verify_res.stderr)
    );
    let verify_out = String::from_utf8_lossy(&verify_res.stdout);
    assert!(verify_out.contains("SHA3-512 signature verified successfully"));

    // 5. Run self-test
    let test_cmd_res = slide_cmd(bin)
        .arg("piv")
        .arg("test")
        .output()
        .expect("cargo-slide piv test");
    assert!(
        test_cmd_res.status.success(),
        "test failed: {:?}",
        String::from_utf8_lossy(&test_cmd_res.stderr)
    );
    let test_out = String::from_utf8_lossy(&test_cmd_res.stdout);
    assert!(test_out.contains("All Cryptographic and YubiKey PIV Self-Tests PASSED"));
}

#[test]
fn test_pack_and_unpack_encrypted_slide_package() {
    let bin = env!("CARGO_BIN_EXE_cargo-slide");
    let dir = tempdir().expect("tempdir");

    // Initialize presentation
    let init_res = slide_cmd(bin)
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("cargo-slide init");
    assert!(init_res.status.success());

    // Pack encrypted presentation bound to PIV slots
    let out_slide = dir.path().join("confidential.slide");
    let pack_res = slide_cmd(bin)
        .arg("pack")
        .arg("slides.typ")
        .arg("--encrypt")
        .arg("--piv-slot")
        .arg("9d")
        .arg("--sign-slot")
        .arg("9c")
        .arg("--output")
        .arg(&out_slide)
        .current_dir(dir.path())
        .output()
        .expect("cargo-slide pack --encrypt");
    assert!(
        pack_res.status.success(),
        "Pack failed: {:?}",
        String::from_utf8_lossy(&pack_res.stderr)
    );
    assert!(out_slide.exists());

    // Verify magic header
    let file_bytes = std::fs::read(&out_slide).unwrap();
    assert!(file_bytes.starts_with(b"SLIDECRYPT\x01"));

    // Verify file is recognized as encrypted package
    assert!(slide_core::package::is_encrypted_package_file(&out_slide));

    // Verify unpack in-memory directly with PivCard
    unsafe {
        std::env::set_var("CARGO_SLIDE_MOCK_PIV", "1");
    }
    let mut card = slide_core::security::piv::PivManager::acquire_card(Some("123456")).unwrap();
    let (deck, asset_store) = slide_core::package::unpack_encrypted_deck_and_assets_in_memory(
        &out_slide,
        card.as_mut(),
        None,
    )
    .unwrap();
    assert!(deck.total_slides() > 0);
    assert!(asset_store.contains("assets/data.csv"));

    // Tamper test: flip 1 byte in ciphertext and verify integrity check fails
    let mut tampered_bytes = file_bytes.clone();
    let len = tampered_bytes.len();
    tampered_bytes[len - 20] ^= 0x55;
    let tampered_slide = dir.path().join("tampered.slide");
    std::fs::write(&tampered_slide, tampered_bytes).unwrap();

    let unpack_err = slide_core::package::unpack_encrypted_deck_and_assets_in_memory(
        &tampered_slide,
        card.as_mut(),
        None,
    );
    assert!(unpack_err.is_err());
    let err_msg = unpack_err.err().unwrap().to_string();
    assert!(
        err_msg.contains("Integrity check failed")
            || err_msg.contains("Crypto")
            || err_msg.contains("AES-GCM decryption failed")
    );
}

#[test]
fn test_build_encrypted_standalone_binary_embed() {
    let bin = env!("CARGO_BIN_EXE_cargo-slide");
    let dir = tempdir().expect("tempdir");

    // Initialize presentation
    let _ = slide_cmd(bin).arg("init").current_dir(dir.path()).output();

    let out_bin = dir.path().join("encrypted_presentation_app");
    let build_res = slide_cmd(bin)
        .arg("build")
        .arg("slides.typ")
        .arg("--format")
        .arg("binary")
        .arg("--encrypt")
        .arg("--piv-slot")
        .arg("9d")
        .arg("--sign-slot")
        .arg("9c")
        .arg("--output")
        .arg(&out_bin)
        .current_dir(dir.path())
        .output()
        .expect("cargo-slide build --encrypt");
    assert!(
        build_res.status.success(),
        "Build failed: {:?}",
        String::from_utf8_lossy(&build_res.stderr)
    );
    assert!(out_bin.exists());

    // Test running binary with --help
    let help_res = Command::new(&out_bin)
        .arg("--help")
        .output()
        .expect("Run standalone binary --help");
    assert!(help_res.status.success());
    let help_txt = String::from_utf8_lossy(&help_res.stdout);
    assert!(help_txt.contains("Encrypted Standalone Slide Presentation"));
    assert!(help_txt.contains("--pin"));
    assert!(help_txt.contains("Connected YubiKey PIV smartcard"));
}

#[test]
fn test_pack_and_unpack_software_passphrase_package() {
    let bin = env!("CARGO_BIN_EXE_cargo-slide");
    let dir = tempdir().expect("tempdir");

    // Initialize presentation
    let init_res = slide_cmd(bin)
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("cargo-slide init");
    assert!(init_res.status.success());

    let out_slide = dir.path().join("passphrase_protected.slide");
    let test_pass = "V3ry$Tr0ng&ComplexSecret!";

    // Pack with software passphrase mode
    let pack_res = slide_cmd(bin)
        .arg("pack")
        .arg("slides.typ")
        .arg("--encrypt")
        .arg("--auth")
        .arg("passphrase")
        .arg("--passphrase")
        .arg(test_pass)
        .arg("--output")
        .arg(&out_slide)
        .current_dir(dir.path())
        .output()
        .expect("cargo-slide pack --encrypt --auth passphrase");
    assert!(
        pack_res.status.success(),
        "Pack failed: {:?}",
        String::from_utf8_lossy(&pack_res.stderr)
    );
    assert!(out_slide.exists());

    // Verify unpack using unpack_encrypted_deck_with_secret_in_memory
    let (deck, asset_store) =
        slide_core::package::unpack_encrypted_deck_with_secret_in_memory(&out_slide, test_pass)
            .unwrap();
    assert!(deck.total_slides() > 0);
    assert!(asset_store.contains("assets/data.csv"));

    // Verify wrong passphrase fails
    let bad_unpack = slide_core::package::unpack_encrypted_deck_with_secret_in_memory(
        &out_slide,
        "WrongPassword#123",
    );
    assert!(bad_unpack.is_err());
}

#[test]
fn test_pack_and_unpack_software_mnemonic_package() {
    let bin = env!("CARGO_BIN_EXE_cargo-slide");
    let dir = tempdir().expect("tempdir");

    // Initialize presentation
    let init_res = slide_cmd(bin)
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("cargo-slide init");
    assert!(init_res.status.success());

    let out_slide = dir.path().join("mnemonic_protected.slide");
    let known_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    // Pack with specified BIP-39 mnemonic
    let pack_res = slide_cmd(bin)
        .arg("pack")
        .arg("slides.typ")
        .arg("--encrypt")
        .arg("--auth")
        .arg("mnemonic")
        .arg("--mnemonic")
        .arg(known_mnemonic)
        .arg("--output")
        .arg(&out_slide)
        .current_dir(dir.path())
        .output()
        .expect("cargo-slide pack --encrypt --auth mnemonic");
    assert!(
        pack_res.status.success(),
        "Pack failed: {:?}",
        String::from_utf8_lossy(&pack_res.stderr)
    );
    assert!(out_slide.exists());

    // Unpack with known mnemonic
    let (deck, _store) = slide_core::package::unpack_encrypted_deck_with_secret_in_memory(
        &out_slide,
        known_mnemonic,
    )
    .unwrap();
    assert!(deck.total_slides() > 0);

    // Unpack with wrong mnemonic fails
    let wrong_mnemonic =
        "legal winner thank year wave sausage worth useful legal winner thank yellow";
    let bad_unpack = slide_core::package::unpack_encrypted_deck_with_secret_in_memory(
        &out_slide,
        wrong_mnemonic,
    );
    assert!(bad_unpack.is_err());
}

#[test]
fn test_passphrase_complexity_enforcement() {
    let bin = env!("CARGO_BIN_EXE_cargo-slide");
    let dir = tempdir().expect("tempdir");

    let _ = slide_cmd(bin).arg("init").current_dir(dir.path()).output();

    let out_slide = dir.path().join("weak.slide");
    // Weak password: too short and simple
    let pack_res = slide_cmd(bin)
        .arg("pack")
        .arg("slides.typ")
        .arg("--encrypt")
        .arg("--auth")
        .arg("passphrase")
        .arg("--passphrase")
        .arg("12345")
        .arg("--output")
        .arg(&out_slide)
        .current_dir(dir.path())
        .output()
        .expect("cargo-slide pack weak passphrase");

    assert!(
        !pack_res.status.success(),
        "Weak passphrase should be rejected"
    );
    let stderr = String::from_utf8_lossy(&pack_res.stderr);
    assert!(
        stderr.contains("complexity")
            || stderr.contains("minimum 10 required")
            || stderr.contains("too short")
            || stderr.contains("Passphrase"),
        "Expected complexity error message, got: {stderr}"
    );
}
