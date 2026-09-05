use slide_core::compiler::SlideCompiler;
use std::fs::write;
use tempfile::tempdir;

#[test]
fn test_compile_typst_deck() {
    let compiler = match SlideCompiler::new() {
        | Ok(c) => c,
        | Err(_) => {
            eprintln!("Typst not available, skipping test");
            return;
        },
    };

    let dir = tempdir().expect("tempdir");
    let typst_file = dir.path().join("test.typ");
    let content = r#"
#set page(width: 16cm, height: 9cm)
= Slide 1
Welcome to testing!
Visit #link("https://typst.app")[Typst].

#pagebreak()

= Slide 2
Second slide with math: $E = m c^2$.
"#;
    write(&typst_file, content).expect("write typst file");

    let deck = compiler
        .compile_file(&typst_file)
        .expect("Failed to compile typst deck");
    assert_eq!(deck.total_slides(), 2);
    assert_eq!(deck.slides[0].page_number, 1);
    assert_eq!(deck.slides[1].page_number, 2);
    assert!(!deck.slides[0].hotspots.is_empty());
}

#[test]
fn test_detect_slide_overflow() {
    use slide_core::error::SlideError;

    let compiler = match SlideCompiler::new() {
        | Ok(c) => c,
        | Err(_) => {
            eprintln!("Typst not available, skipping test");
            return;
        },
    };

    let dir = tempdir().expect("tempdir");
    let typst_file = dir.path().join("overflow.typ");
    // Define a 1-slide deck where huge content overflows the 5cm page
    let content = r#"
#set page(width: 10cm, height: 5cm)
#let title-slide(title: "") = {
  heading(title)
}
#title-slide(title: "Overflowing Slide")
#v(10cm)
This content definitely overflows the 5cm page bounds!
"#;
    write(&typst_file, content).expect("write overflow typst file");

    let res = compiler.compile_file(&typst_file);
    assert!(
        res.is_err(),
        "Expected compilation to fail with SlideError::Overflow"
    );
    match res.err().unwrap() {
        | SlideError::Overflow(msg) => {
            assert!(msg.contains("Typst slide content overflow detected"));
            assert!(msg.contains("Overflowing Slide"));
        },
        | other => panic!("Expected SlideError::Overflow, got {:?}", other),
    }
}

#[test]
fn test_multilingual_cjk_and_unicode_rendering() {
    let compiler = match SlideCompiler::new() {
        | Ok(c) => c,
        | Err(_) => {
            eprintln!("Typst not available, skipping test");
            return;
        },
    };

    let dir = tempdir().expect("tempdir");
    let typst_file = dir.path().join("cjk.typ");
    let content = r#"
#set page(width: 16cm, height: 9cm)
= 多语言与国际化排版测试
这是一个完全支持中文、日本語、한국어 的原生测试。
- 中文测试：你好，世界！向量图形渲染流畅无畸变。
- 日本語テスト：こんにちは世界！ルビ表記や約物。
- 한국어 테스트: 안녕하세요 세계! 유니코드 서체.
- Unicode & Emojis: 🚀 📊 🎨 ⚛️ ∑ ∇ ∫
- Latin Accents: Café, Über, façade, Español.
"#;
    write(&typst_file, content).expect("write typst file");

    let deck = compiler
        .compile_file(&typst_file)
        .expect("Failed to compile CJK typst deck");
    assert_eq!(deck.total_slides(), 1);
    assert!(!deck.slides[0].svg_data.is_empty());
    assert!(deck.slides[0].svg_data.contains("<svg"));

    let info = slide_core::svg::parse_svg_slide(&deck.slides[0].svg_data).expect("parse svg slide");
    assert!(info.view_box.width > 0.0);
    assert!(info.view_box.height > 0.0);
}
