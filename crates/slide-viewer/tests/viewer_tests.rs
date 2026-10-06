use slide_core::model::Slide;
use slide_core::model::SlideDeck;
use slide_core::package::pack_deck_to_file;
use slide_core::package::read_package_metadata;
use slide_core::package::unpack_deck_from_file;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_viewer_help_and_version() {
    let bin = env!("CARGO_BIN_EXE_slide-viewer");

    let output = Command::new(bin)
        .arg("--help")
        .output()
        .expect("slide-viewer --help");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Universal standalone presentation player"));
    assert!(stdout.contains("--animation"));
    assert!(stdout.contains("--fullscreen"));

    let ver_output = Command::new(bin)
        .arg("--version")
        .output()
        .expect("slide-viewer --version");
    assert!(ver_output.status.success());
}

#[test]
fn test_viewer_lzma2_package_roundtrip() {
    let dir = tempdir().expect("tempdir");
    let package_path = dir.path().join("presentation.slide");

    let mut deck = SlideDeck::new("Viewer Test Deck");
    deck.slides.push(Slide {
        page_number: 1,
        svg_data:
            r##"<svg viewBox="0 0 100 100"><circle cx="50" cy="50" r="40" fill="cyan"/></svg>"##
                .to_string(),
        view_box: slide_core::model::Rect::new(0.0, 0.0, 100.0, 100.0),
        hotspots: vec![],
        animation: Some("fade".to_string()),
        steps: vec![],
        notes: None,
    });

    // Pack using LZMA2 extreme
    pack_deck_to_file(&deck, &package_path).expect("Pack deck");
    assert!(package_path.exists());

    // Read metadata
    let meta = read_package_metadata(&package_path).expect("Read metadata");
    assert_eq!(meta.title, "Viewer Test Deck");
    assert_eq!(meta.total_slides, 1);
    assert_eq!(meta.compression, "lzma2-max");

    // Unpack deck
    let unpacked = unpack_deck_from_file(&package_path).expect("Unpack deck");
    assert_eq!(unpacked.title, "Viewer Test Deck");
    assert_eq!(unpacked.slides.len(), 1);
    assert!(unpacked.slides[0].svg_data.contains("cyan"));
}

#[test]
fn test_viewer_cli_args_light_and_demo() {
    let bin = env!("CARGO_BIN_EXE_slide-viewer");

    let output = Command::new(bin)
        .arg("--help")
        .output()
        .expect("slide-viewer --help");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--light"),
        "Must support --light for light HUD theme"
    );
    assert!(
        stdout.contains("--demo"),
        "Must support --demo for showcase launch"
    );
}

#[test]
fn test_compiler_auto_resolves_slide_typ_in_isolated_directory() {
    let dir = tempdir().expect("tempdir");
    let typ_path = dir.path().join("isolated_presentation.typ");
    let content = r#"#import "slide.typ": *

#title-slide(
  title: "Auto Macro Test",
  subtitle: "Testing Slide.typ In Root Dir",
)

#slide(title: "First Slide")[
  - Testing item
]
"#;
    std::fs::write(&typ_path, content).expect("Write typ file");

    // Initially slide.typ does NOT exist in dir
    let macro_path = dir.path().join("slide.typ");
    assert!(!macro_path.exists());

    // Compile file using SlideCompiler
    let compiler = slide_core::compiler::SlideCompiler::new().expect("SlideCompiler");
    let deck = compiler.compile_file(&typ_path);
    assert!(
        deck.is_ok(),
        "Compilation must succeed without slide.typ missing error: {:?}",
        deck.err()
    );

    // Verify slide.typ was automatically generated in dir
    assert!(
        macro_path.exists(),
        "slide.typ should be auto-created in document root dir"
    );
    let macro_content = std::fs::read_to_string(&macro_path).unwrap();
    assert!(macro_content.contains("title-slide"));
}
