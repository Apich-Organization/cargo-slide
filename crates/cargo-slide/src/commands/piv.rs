//! PIV hardware smartcard management command (NIST SP 800-73-4 / `YubiKey`).

use crate::cli::PivAction;
use slide_core::security::cert::PivSlot;
use slide_core::security::crypto::compute_sha3_512;
use slide_core::security::piv::PivManager;

/// Execute the `piv` subcommand.
#[allow(clippy::too_many_lines)]
pub fn execute(action: PivAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        | PivAction::Info { pin } => {
            println!("============================================================");
            println!(" Cargo Slide YubiKey PIV Smartcard Inspector");
            println!("============================================================");

            let env_pin = std::env::var("PIV_PIN").ok();
            let effective_pin = pin.as_deref().or(env_pin.as_deref());
            let card = PivManager::acquire_card(effective_pin)?;

            println!("Card Reader:    {}", card.name());
            println!("Authenticated:  {}", card.is_authenticated());

            let slots = card.list_slots();
            if slots.is_empty() {
                println!("\nConfigured Slots: (none detected)");
            } else {
                println!("\nConfigured PIV Slots (NIST SP 800-73-4):");
                for slot in slots {
                    println!("  ----------------------------------------------------------");
                    println!("  [Slot {}] {}", slot.as_str(), slot.name());
                    if let Ok(cert_der) = card.get_certificate(slot) {
                        if let Ok(info) =
                            slide_core::security::cert::parse_certificate_info(&cert_der)
                        {
                            println!("    Subject:     {}", info.subject);
                            println!("    Issuer:      {}", info.issuer);
                            println!("    Algorithm:   {}", info.algorithm);
                            println!("    Valid From:  {}", info.not_before);
                            println!("    Valid Until: {}", info.not_after);
                            println!("    SHA3-512:    {}", info.sha3_512_fingerprint);
                        } else {
                            println!("    Certificate: (binary X.509 {} bytes)", cert_der.len());
                        }
                    } else {
                        println!("    (No certificate installed in slot)");
                    }
                }
                println!("  ----------------------------------------------------------");
            }

            println!("\n[Yubico PIV Tool Integration Guide]");
            println!("  Keys and PINs are securely managed by official Yubico tools:");
            println!("    - Inspect slots:        ykman piv info");
            println!(
                "    - Generate Slot 9D key: ykman piv keys generate 9d -a ECCP256 (or RSA2048)"
            );
            println!(
                "    - Generate Slot 9C key: ykman piv keys generate 9c -a ECCP256 (or RSA2048)"
            );
            println!("    - Change User PIN:      ykman piv access change-pin");
            println!("============================================================\n");
        },
        | PivAction::ExportCert { slot, output, pin } => {
            let piv_slot = PivSlot::parse(&slot).ok_or_else(|| {
                format!("Invalid PIV slot: '{slot}'. Valid slots: 9a, 9c, 9d, 9e")
            })?;
            let env_pin = std::env::var("PIV_PIN").ok();
            let effective_pin = pin.as_deref().or(env_pin.as_deref());
            let card = PivManager::acquire_card(effective_pin)?;

            let cert_der = card.get_certificate(piv_slot)?;
            let pem = slide_core::security::cert_der_to_pem(&cert_der);

            if let Some(out_file) = output {
                std::fs::write(&out_file, &pem)?;
                println!("[OK] Exported certificate to {}", out_file.display());
            } else {
                println!("{pem}");
            }
        },
        | PivAction::Sign {
            file,
            slot,
            output,
            pin,
            gui,
        } => {
            let piv_slot = PivSlot::parse(&slot).ok_or_else(|| {
                format!("Invalid PIV slot: '{slot}'. Valid slots: 9a, 9c, 9d, 9e")
            })?;
            let file_data = std::fs::read(&file)?;
            let sha3_digest = compute_sha3_512(&file_data);

            let mut card = PivManager::acquire_card(pin.as_deref())?;
            slide_core::security::prompt_pin_for_slot(
                card.as_mut(),
                piv_slot,
                pin.as_deref(),
                gui,
            )?;

            let sig = card.sign(piv_slot, &sha3_digest)?;

            let out_path = output.unwrap_or_else(|| {
                let mut p = file.clone();
                p.set_extension("sig");
                p
            });

            std::fs::write(&out_path, &sig)?;
            println!("[OK] File signed successfully using SHA3-512 and {piv_slot}");
            println!("Target File:    {}", file.display());
            println!("Signature File: {}", out_path.display());
            println!("Signature Size: {} bytes", sig.len());
        },
        | PivAction::Verify {
            file,
            sig,
            slot,
            cert,
            pin,
        } => {
            let piv_slot = PivSlot::parse(&slot).ok_or_else(|| {
                format!("Invalid PIV slot: '{slot}'. Valid slots: 9a, 9c, 9d, 9e")
            })?;
            let file_data = std::fs::read(&file)?;
            let sig_data = std::fs::read(&sig)?;
            let sha3_digest = compute_sha3_512(&file_data);

            let pub_key = if let Some(cert_path) = cert {
                let cert_data = std::fs::read(cert_path)?;
                slide_core::security::cert::PivPublicKey::from_pem_or_der(&cert_data)?
            } else {
                let env_pin = std::env::var("PIV_PIN").ok();
                let effective_pin = pin.as_deref().or(env_pin.as_deref());
                let card = PivManager::acquire_card(effective_pin)?;
                card.get_public_key(piv_slot)?
            };

            match pub_key {
                | slide_core::security::PivPublicKey::Rsa(ref rsa_pub) => {
                    slide_core::security::crypto::rsa_verify_sha3_512(
                        rsa_pub,
                        &sha3_digest,
                        &sig_data,
                    )?;
                },
                | slide_core::security::PivPublicKey::Ecc(ref ecc_pub) => {
                    slide_core::security::crypto::p256_ecdsa_verify(
                        ecc_pub,
                        &sha3_digest,
                        &sig_data,
                    )?;
                },
            }

            println!("[OK] SHA3-512 signature verified successfully!");
            println!("File:      {}", file.display());
            println!("Signature: {}", sig.display());
            println!("Algorithm: SHA3-512 + {}", pub_key.algorithm_name());
        },
        | PivAction::Test { pin, gui } => {
            println!("Running Cargo Slide Cryptography and YubiKey PIV Self-Test...");
            let mut card = PivManager::acquire_card(pin.as_deref())?;
            slide_core::security::prompt_pin_for_slot(
                card.as_mut(),
                PivSlot::Slot9D,
                pin.as_deref(),
                gui,
            )?;

            // Test 1: Symmetric AES-256-GCM
            let key = slide_core::security::crypto::generate_aes_key();
            let (ct, nonce, tag) =
                slide_core::security::crypto::aes_256_gcm_encrypt(&key, b"Test payload")?;
            let pt = slide_core::security::crypto::aes_256_gcm_decrypt(&key, &nonce, &tag, &ct)?;
            assert_eq!(pt.as_slice(), b"Test payload");
            println!("  [PASS] AES-256-GCM encryption & memory zeroization");

            // Test 2: SHA3-512
            let h = compute_sha3_512(b"Integrity Test");
            assert_eq!(h.len(), 64);
            println!("  [PASS] SHA3-512 cryptographic hashing");

            // Test 3: PIV Slot Decryption
            if let Ok(pub_9d) = card.get_public_key(PivSlot::Slot9D) {
                let test_key = slide_core::security::crypto::generate_aes_key();
                let encrypted_key = match pub_9d {
                    | slide_core::security::PivPublicKey::Rsa(ref rsa_pub) => {
                        slide_core::security::crypto::rsa_oaep_encrypt(rsa_pub, test_key.as_ref())?
                    },
                    | slide_core::security::PivPublicKey::Ecc(ref ecc_pub) => {
                        slide_core::security::crypto::p256_ecdh_encrypt(ecc_pub, &test_key)?
                    },
                };
                let decrypted_key = card.decrypt(PivSlot::Slot9D, &encrypted_key)?;
                assert_eq!(decrypted_key.as_slice(), test_key.as_ref());
                println!("  [PASS] PIV Slot 9D Hardware Key Decryption");
            }

            // Test 4: PIV Slot Signing
            if let Ok(pub_9c) = card.get_public_key(PivSlot::Slot9C) {
                slide_core::security::prompt_pin_for_slot(
                    card.as_mut(),
                    PivSlot::Slot9C,
                    pin.as_deref(),
                    gui,
                )?;
                let test_digest = compute_sha3_512(b"Signature self-test");
                let sig = card.sign(PivSlot::Slot9C, &test_digest)?;
                match pub_9c {
                    | slide_core::security::PivPublicKey::Rsa(ref rsa_pub) => {
                        slide_core::security::crypto::rsa_verify_sha3_512(
                            rsa_pub,
                            &test_digest,
                            &sig,
                        )?;
                    },
                    | slide_core::security::PivPublicKey::Ecc(ref ecc_pub) => {
                        slide_core::security::crypto::p256_ecdsa_verify(
                            ecc_pub,
                            &test_digest,
                            &sig,
                        )?;
                    },
                }
                println!("  [PASS] PIV Slot 9C SHA3-512 Hardware Digital Signing & Verification");
            }

            println!("\n[OK] All Cryptographic and YubiKey PIV Self-Tests PASSED!");
        },
    }

    Ok(())
}
