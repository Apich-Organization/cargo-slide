//! Command to package a presentation into a standalone compressed .slide archive.

use slide_core::compiler::SlideCompiler;
use slide_core::package::pack_deck_with_source_and_assets;
use slide_core::security::cert::PivPublicKey;
use slide_core::security::cert::PivSlot;
use slide_core::security::piv::PivManager;
use std::path::Path;
use std::path::PathBuf;

/// Execute the `pack` command.
///
/// Compiles the presentation slides and archives them into a standalone `.slide` file.
/// When `encrypt` is true, encrypts with AES-256 bound to a PIV slot public key,
/// hashes with SHA3-512, and signs with the signer's PIV slot.
#[allow(
    clippy::pedantic,
    clippy::nursery,
    clippy::cast_precision_loss,
    clippy::too_many_arguments
)]
pub fn execute(
    file: &Path,
    output: Option<PathBuf>,
    animation: &str,
    source: bool,
    encrypt: bool,
    auth_mode_str: &str,
    passphrase: Option<&str>,
    mnemonic: Option<&str>,
    mnemonic_words: usize,
    piv_slot_str: &str,
    sign_slot_str: &str,
    cert_path: Option<&Path>,
    pin: Option<&str>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let resolved_file = slide_core::compiler::resolve_presentation_target(file)?;
    let target = resolved_file.as_path();

    let stem = target
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("presentation");
    let out_file = output.unwrap_or_else(|| PathBuf::from(format!("{stem}.slide")));

    slide_core::logger::log_event(
        "info",
        &format!(
            "Packaging presentation into .slide archive (source: {}, encrypted: {}): {}",
            source,
            encrypt,
            target.display()
        ),
        Some(serde_json::json!({
            "stage": "pack_start",
            "source_file": target.display().to_string(),
            "output_file": out_file.display().to_string(),
            "include_source": source,
            "encrypted": encrypt,
        })),
    );

    let compiler = SlideCompiler::new()?;
    let mut deck = compiler.compile_file(target)?;

    if !animation.is_empty() {
        deck.default_animation = animation.to_string();
    }

    let base_dir = target.parent();

    if encrypt {
        let auth_mode: slide_core::security::AuthMode = auth_mode_str.parse()?;

        match auth_mode {
            | slide_core::security::AuthMode::Passphrase => {
                let pass = if let Some(p) = passphrase {
                    p.to_string()
                } else if let Ok(env_pass) = std::env::var("SLIDE_PASSPHRASE") {
                    env_pass
                } else {
                    let input = slide_core::security::read_masked_secret(
                        "Enter passphrase for presentation encryption: ",
                    )?;
                    input.to_string()
                };

                slide_core::security::validate_password_strength(&pass)?;

                let (pkg_bytes, _) =
                    slide_core::security::EncryptedSlideContainer::pack_software_with_source(
                        &deck,
                        base_dir,
                        &pass,
                        slide_core::security::AuthMode::Passphrase,
                        None,
                        source,
                    )?;
                std::fs::write(&out_file, pkg_bytes)?;

                let size_bytes = std::fs::metadata(&out_file).map(|m| m.len()).unwrap_or(0);
                let size_kb = (size_bytes as f64) / 1024.0;

                slide_core::logger::log_event(
                    "success",
                    &format!(
                        "[OK] Packaged & encrypted {} slides into {} ({:.1} KB, AES-256-GCM + Passphrase KDF + SHA3-512)",
                        deck.total_slides(),
                        out_file.display(),
                        size_kb
                    ),
                    Some(serde_json::json!({
                        "stage": "pack_success",
                        "total_slides": deck.total_slides(),
                        "output_file": out_file.display().to_string(),
                        "size_bytes": size_bytes,
                        "encrypted": true,
                        "auth_mode": "passphrase",
                    })),
                );
            },
            | slide_core::security::AuthMode::Mnemonic => {
                let mnem = mnemonic.unwrap_or("");
                let (pkg_bytes, gen_mnem) =
                    slide_core::security::EncryptedSlideContainer::pack_software_with_source(
                        &deck,
                        base_dir,
                        mnem,
                        slide_core::security::AuthMode::Mnemonic,
                        Some(mnemonic_words),
                        source,
                    )?;
                std::fs::write(&out_file, pkg_bytes)?;

                if let Some(generated) = gen_mnem {
                    let word_count = generated.split_whitespace().count();
                    eprintln!();
                    eprintln!(
                        "================================================================================"
                    );
                    eprintln!("[SECURITY] Generated {word_count}-Word BIP-39 Recovery Mnemonic:");
                    eprintln!();
                    eprintln!("    {}", generated.as_str());
                    eprintln!();
                    eprintln!(
                        "[IMPORTANT] Write down these {word_count} words in order and store them in a secure location."
                    );
                    eprintln!(
                        "            Anyone with this mnemonic can decrypt and view this presentation."
                    );
                    eprintln!(
                        "================================================================================"
                    );
                    eprintln!();
                }

                let size_bytes = std::fs::metadata(&out_file).map(|m| m.len()).unwrap_or(0);
                let size_kb = (size_bytes as f64) / 1024.0;

                slide_core::logger::log_event(
                    "success",
                    &format!(
                        "[OK] Packaged & encrypted {} slides into {} ({:.1} KB, AES-256-GCM + BIP-39 Mnemonic + SHA3-512)",
                        deck.total_slides(),
                        out_file.display(),
                        size_kb
                    ),
                    Some(serde_json::json!({
                        "stage": "pack_success",
                        "total_slides": deck.total_slides(),
                        "output_file": out_file.display().to_string(),
                        "size_bytes": size_bytes,
                        "encrypted": true,
                        "auth_mode": "mnemonic",
                    })),
                );
            },
            | slide_core::security::AuthMode::Piv => {
                let recipient_slot = PivSlot::parse(piv_slot_str).ok_or_else(|| {
                    format!("Invalid recipient PIV slot: '{piv_slot_str}'. Supported slots: 9a, 9c, 9d, 9e")
                })?;
                let signer_slot = PivSlot::parse(sign_slot_str).ok_or_else(|| {
                    format!("Invalid signer PIV slot: '{sign_slot_str}'. Supported slots: 9a, 9c, 9d, 9e")
                })?;

                let mut card = PivManager::acquire_card(pin).map_err(|e| {
                    format!(
                        "{e}\n\n[TIP] To encrypt without a hardware YubiKey, use software passphrase mode:\n  cargo slide pack slides.typ --encrypt --auth passphrase\nor BIP-39 recovery mnemonic mode:\n  cargo slide pack slides.typ --encrypt --auth mnemonic"
                    )
                })?;
                slide_core::security::prompt_pin_for_slot(card.as_mut(), signer_slot, pin, false)?;

                let recipient_pub = if let Some(cp) = cert_path {
                    let cert_data = std::fs::read(cp)?;
                    PivPublicKey::from_pem_or_der(&cert_data)?
                } else {
                    card.get_public_key(recipient_slot)?
                };

                slide_core::package::pack_encrypted_deck_to_file_with_source(
                    &deck,
                    base_dir,
                    &out_file,
                    &recipient_pub,
                    recipient_slot,
                    card.as_mut(),
                    signer_slot,
                    source,
                )?;

                let size_bytes = std::fs::metadata(&out_file).map(|m| m.len()).unwrap_or(0);
                let size_kb = (size_bytes as f64) / 1024.0;

                slide_core::logger::log_event(
                    "success",
                    &format!(
                        "[OK] Packaged & encrypted {} slides into {} ({:.1} KB, AES-256-GCM + PIV slot {} + SHA3-512 signature)",
                        deck.total_slides(),
                        out_file.display(),
                        size_kb,
                        recipient_slot.as_str()
                    ),
                    Some(serde_json::json!({
                        "stage": "pack_success",
                        "total_slides": deck.total_slides(),
                        "output_file": out_file.display().to_string(),
                        "size_bytes": size_bytes,
                        "encrypted": true,
                        "piv_slot": recipient_slot.as_str(),
                        "sign_slot": signer_slot.as_str(),
                    })),
                );
            },
        }
    } else {
        pack_deck_with_source_and_assets(&deck, base_dir, &out_file, source, Some(target))?;

        let size_bytes = std::fs::metadata(&out_file).map(|m| m.len()).unwrap_or(0);
        let size_kb = (size_bytes as f64) / 1024.0;

        slide_core::logger::log_event(
            "success",
            &format!(
                "[OK] Packaged {} slides into {} ({:.1} KB, LZMA2)",
                deck.total_slides(),
                out_file.display(),
                size_kb
            ),
            Some(serde_json::json!({
                "stage": "pack_success",
                "total_slides": deck.total_slides(),
                "output_file": out_file.display().to_string(),
                "size_bytes": size_bytes,
            })),
        );
    }

    Ok(())
}
