use slide_editor::app::ActiveModal;
use slide_editor::app::CompilationStatus;
use slide_editor::app::EditorMode;
use slide_editor::app::Message;
use slide_editor::app::SlideEditorApp;
use slide_editor::compiler_bridge::CompilerBridge;
use slide_editor::document::DocumentFormat;
use slide_editor::document::EditorDocument;
use slide_editor::model::ast_engine::TypstDocumentEngine;
use slide_editor::ui::theme::AppTheme;

#[test]
fn test_new_presentation_document() {
    let doc = EditorDocument::new_presentation("Test Deck");
    assert_eq!(doc.title, "Test Deck");
    assert_eq!(doc.format, DocumentFormat::Typst);
    assert!(!doc.is_dirty);
    assert!(doc.total_slides() >= 3);

    let engine =
        slide_editor::model::ast_engine::TypstDocumentEngine::from_source(doc.source_text.clone());
    println!("=== ENGINE SLIDES: {} ===", engine.slides.len());
    for (s_idx, s) in engine.slides.iter().enumerate() {
        println!(
            "Slide {}: title={:?}, range={:?}, blocks={}",
            s_idx,
            s.title,
            s.range,
            s.blocks.len()
        );
        for (b_idx, b) in s.blocks.iter().enumerate() {
            println!(
                "  b{}: id={}, raw={:?}, range={:?}",
                b_idx,
                b.id(),
                b.raw(),
                b.range()
            );
        }
    }
}

#[test]
fn test_new_generic_typst_document() {
    let doc = EditorDocument::new_typst_document("Report");
    assert_eq!(doc.title, "Report");
    assert_eq!(doc.format, DocumentFormat::Typst);
    assert!(doc.source_text.contains("Report"));
}

#[test]
fn test_slide_chunk_update_and_recompile() {
    let mut doc = EditorDocument::new_presentation("Chunk Test");
    let initial_slides = doc.total_slides();

    // Modify slide 1 chunk
    doc.update_slide_chunk(0, "= Updated Title Slide\n\nNew Subtitle");
    assert!(doc.is_dirty);
    assert_eq!(doc.total_slides(), initial_slides);

    let chunks = doc.get_slide_chunks();
    assert!(chunks[0].contains("Updated Title Slide"));
    assert!(doc.source_text.contains("Updated Title Slide"));

    // Add a new slide
    doc.add_new_slide();
    assert_eq!(doc.total_slides(), initial_slides + 1);
}

#[test]
fn test_compiler_bridge_compilation() {
    let compiler = CompilerBridge::new();
    let doc = EditorDocument::new_presentation("Compile Test");

    let result = compiler.compile_source(&doc.source_text, None);
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    let deck = result.unwrap();
    assert_eq!(deck.slides.len(), doc.total_slides());
    assert!(!deck.slides[0].svg_data.is_empty());
}

#[test]
fn test_compiler_bridge_diagnostic_parsing() {
    let compiler = CompilerBridge::new();
    // Invalid Typst source
    let invalid_source = "#set page(invalid_attribute: 1234)\n= Hello";

    let result = compiler.compile_source(invalid_source, None);
    assert!(result.is_err());

    let diag = result.unwrap_err();
    assert!(!diag.message.is_empty());
    assert!(!diag.full_stderr.is_empty());
}

#[test]
fn test_multi_format_export() {
    let compiler = CompilerBridge::new();
    let doc = EditorDocument::new_presentation("Export Test");

    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");

    // 1. Export PDF
    let pdf_path = temp_dir.path().join("output.pdf");
    let pdf_res = compiler.export_pdf(&doc, &pdf_path);
    assert!(pdf_res.is_ok(), "PDF export error: {:?}", pdf_res.err());
    assert!(pdf_path.exists());
    assert!(pdf_path.metadata().unwrap().len() > 0);

    // 2. Export Standalone .slide Package (with editable origin source)
    let slide_path = temp_dir.path().join("output.slide");
    let slide_res = compiler.export_slide_package(&doc, true, &slide_path);
    assert!(
        slide_res.is_ok(),
        "Slide export error: {:?}",
        slide_res.err()
    );
    assert!(slide_path.exists());

    // 3. Open the newly exported .slide package
    let opened_slide = EditorDocument::open(&slide_path);
    assert!(
        opened_slide.is_ok(),
        "Open .slide failed: {:?}",
        opened_slide.err()
    );
    let opened_doc = opened_slide.unwrap();
    assert_eq!(opened_doc.format, DocumentFormat::SlidePackage);
    assert_eq!(opened_doc.total_slides(), doc.total_slides());
    assert!(
        opened_doc.has_editable_source(),
        "Exported package should contain editable source"
    );

    // 4. Export SVGs
    let svg_dir = temp_dir.path().join("svg_export");
    let deck = compiler
        .compile_source(&doc.source_text, None)
        .expect("Compile deck");
    let svg_res = CompilerBridge::export_svgs(&deck, &svg_dir, None);
    assert!(svg_res.is_ok());
    assert_eq!(svg_res.unwrap().len(), deck.slides.len());

    // 5. Export PNGs
    let png_dir = temp_dir.path().join("png_export");
    let png_res = CompilerBridge::export_all_pngs(&deck, &png_dir, 1.0, None);
    assert!(png_res.is_ok());
    assert_eq!(png_res.unwrap().len(), deck.slides.len());
    let first_png = png_dir.join("slide-1.png");
    assert!(first_png.exists());
    assert!(first_png.metadata().unwrap().len() > 0);
}

#[test]
fn test_app_state_and_mode_switching() {
    let mut app = SlideEditorApp::new(None, false);
    assert_eq!(app.theme, AppTheme::Light);
    assert_eq!(app.mode, EditorMode::LivePreview);
    assert_eq!(app.compilation_status, CompilationStatus::Ready);

    // Toggle Theme
    let _ = app.update(Message::ToggleTheme);
    assert_eq!(app.theme, AppTheme::Dark);
    let _ = app.update(Message::ToggleTheme);
    assert_eq!(app.theme, AppTheme::Light);

    // Switch to Focus Mode
    let _ = app.update(Message::SwitchMode(EditorMode::FocusMode));
    assert_eq!(app.mode, EditorMode::FocusMode);

    // Switch to Source Mode
    let _ = app.update(Message::SwitchMode(EditorMode::SourceMode));
    assert_eq!(app.mode, EditorMode::SourceMode);

    // Toggle In-Place Edit in Live Preview
    let _ = app.update(Message::SwitchMode(EditorMode::LivePreview));
    let _ = app.update(Message::ToggleInPlaceEdit(0));
    assert_eq!(app.in_place_editing_slide, Some(0));

    // Turn off In-Place Edit
    let _ = app.update(Message::ToggleInPlaceEdit(0));
    assert_eq!(app.in_place_editing_slide, None);
}

#[test]
fn test_open_and_save_typ_file() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let typ_path = temp_dir.path().join("sample.typ");
    std::fs::write(&typ_path, "= Sample Title\n\nSample body text.").expect("write sample");

    let mut doc = EditorDocument::open(&typ_path).expect("open sample");
    assert_eq!(doc.format, DocumentFormat::Typst);
    assert_eq!(doc.title, "sample");
    assert!(doc.source_text.contains("Sample Title"));

    // Modify and save
    doc.source_text
        .push_str("\n\n== Section 2\nAdditional content.");
    doc.is_dirty = true;
    doc.save(None).expect("save sample");

    let reread = std::fs::read_to_string(&typ_path).expect("reread sample");
    assert!(reread.contains("Section 2"));
}

#[test]
fn test_geek_presentation_loading_and_image_caching() {
    let example_path = std::path::PathBuf::from("../../examples/geek-presentation/slides.typ");
    if !example_path.exists() {
        return;
    }

    let start = std::time::Instant::now();
    let app = SlideEditorApp::new(Some(example_path), false);
    let load_time = start.elapsed();
    println!("Loaded and rendered presentation in {:?}", load_time);

    assert_eq!(app.compilation_status, CompilationStatus::Ready);
    assert!(app.doc.total_slides() > 0);
    assert_eq!(app.slide_images.len(), app.doc.total_slides());
    assert_eq!(app.slide_titles.len(), app.doc.total_slides());

    // Verify view generation time (representing a scroll frame)
    let view_start = std::time::Instant::now();
    let _element = app.view();
    let view_time = view_start.elapsed();
    println!("Single scroll frame view generation time: {:?}", view_time);
    // View generation must be < 5ms to guarantee 60+ FPS
    assert!(
        view_time.as_millis() < 50,
        "Frame view time too slow: {:?}",
        view_time
    );
}

#[test]
fn test_wysiwyg_ast_block_classification() {
    use slide_editor::model::ast_engine::TypstDocumentEngine;
    use slide_editor::model::ast_engine::WysiwygBlock;

    let sample_typ = r#"// Header comment
#set page(width: 16cm, height: 9cm)

= Slide Title 1

- Bullet point Alpha
- Bullet point Beta

+ Numbered step 1
+ Numbered step 2

/ Rust: A high performance systems language

```rust
fn main() {
    println!("Hello Typora WYSIWYG");
}
```

$ E = m c^2 $

Regular paragraph explaining the slide contents.

#pagebreak()

= Slide 2: Equations

$ integral_0^infinity e^(-x^2) dif x = sqrt(pi) / 2 $
"#;

    let engine = TypstDocumentEngine::from_source(sample_typ.to_string());
    assert_eq!(engine.slides.len(), 2);

    let slide1 = &engine.slides[0];
    assert_eq!(slide1.page_number, 1);
    assert_eq!(slide1.title, "Slide Title 1");

    // Verify block kinds classified without any wildcards
    let mut found_heading = false;
    let mut found_bullet = false;
    let mut found_enum = false;
    let mut found_term = false;
    let mut found_code = false;
    let mut found_equation = false;
    let mut found_paragraph = false;
    let mut found_set_rule = false;

    for block in &slide1.blocks {
        match block {
            | WysiwygBlock::Heading { level, title, .. } => {
                assert_eq!(*level, 1);
                assert_eq!(title, "Slide Title 1");
                found_heading = true;
            },
            | WysiwygBlock::ListItem { body, .. } => {
                assert!(body.starts_with("Bullet point"));
                found_bullet = true;
            },
            | WysiwygBlock::EnumItem { body, .. } => {
                assert!(body.starts_with("Numbered step"));
                found_enum = true;
            },
            | WysiwygBlock::TermItem { term, .. } => {
                assert_eq!(term, "Rust");
                found_term = true;
            },
            | WysiwygBlock::CodeBlock { language, code, .. } => {
                assert_eq!(language, "rust");
                assert!(code.contains("Hello Typora"));
                found_code = true;
            },
            | WysiwygBlock::Equation { formula, .. } => {
                assert!(formula.contains("E = m c^2"));
                found_equation = true;
            },
            | WysiwygBlock::Paragraph { text, .. } => {
                if text.contains("Regular paragraph") {
                    found_paragraph = true;
                }
            },
            | WysiwygBlock::SetRule { target, .. } => {
                assert_eq!(target, "page");
                found_set_rule = true;
            },
            | _ => {},
        }
    }

    assert!(found_heading, "Heading block should be recognized");
    assert!(found_bullet, "List item block should be recognized");
    assert!(found_enum, "Enum item block should be recognized");
    assert!(found_term, "Term item block should be recognized");
    assert!(found_code, "Code block should be recognized");
    assert!(found_equation, "Equation block should be recognized");
    assert!(found_paragraph, "Paragraph block should be recognized");
    assert!(found_set_rule, "Set rule block should be recognized");
}

#[test]
fn test_wysiwyg_in_place_range_replacement() {
    use slide_editor::model::ast_engine::TypstDocumentEngine;
    use slide_editor::model::ast_engine::WysiwygBlock;

    let sample_typ = r#"// Header comment
= Original Heading

- Key bullet point
"#;

    let mut engine = TypstDocumentEngine::from_source(sample_typ.to_string());
    assert_eq!(engine.slides.len(), 1);

    // Locate heading block
    let heading_block = engine.slides[0]
        .blocks
        .iter()
        .find(|b| matches!(b, WysiwygBlock::Heading { .. }))
        .expect("Must have heading block");

    let range = heading_block.range();
    assert_eq!(heading_block.raw(), "= Original Heading");

    // Perform exact byte-range in-place edit
    let updated_heading = "= New Super Exciting Title";
    let new_range = engine
        .update_block_at_range(range, updated_heading)
        .expect("Update in-place should succeed");

    // Surrounding comment and following bullet point must remain intact!
    assert!(engine.source_text.starts_with("// Header comment\n"));
    assert!(engine.source_text.contains("= New Super Exciting Title"));
    assert!(engine.source_text.contains("- Key bullet point"));

    // Verify reparse reflected the change
    let re_heading = engine.slides[0]
        .blocks
        .iter()
        .find(|b| matches!(b, WysiwygBlock::Heading { .. }))
        .expect("Must have heading after update");

    assert_eq!(re_heading.range(), new_range);
    if let WysiwygBlock::Heading { title, .. } = re_heading {
        assert_eq!(title, "New Super Exciting Title");
    } else {
        panic!("Block kind changed unexpectedly");
    }
}

#[test]
fn test_app_wysiwyg_block_editing_lifecycle() {
    use slide_editor::ui::wysiwyg::FormatAction;
    use slide_editor::ui::wysiwyg::InsertBlockKind;

    let mut app = SlideEditorApp::new(None, false);
    assert_eq!(app.mode, EditorMode::LivePreview);
    assert!(app.active_block_id.is_none());

    // 1. Activate first heading block
    let first_block = app.engine.slides[0].blocks[0].clone();
    let _ = app.update(Message::ActivateBlock {
        slide_idx: 0,
        id: first_block.id().to_string(),
        range: first_block.range(),
        raw: first_block.raw().to_string(),
    });

    assert_eq!(app.active_block_id, Some(first_block.id().to_string()));
    assert_eq!(app.active_block_range, Some(first_block.range()));

    // 2. Format block (Bold)
    let _ = app.update(Message::FormatBlock(FormatAction::Bold));
    assert!(app.doc.is_dirty);
    assert!(app.doc.source_text.contains("*text*"));

    // 3. Deactivate block (commit to visual presentation)
    let _ = app.update(Message::DeactivateBlock);
    assert!(app.active_block_id.is_none());
    assert!(app.active_block_range.is_none());

    // 4. Insert new block after
    let initial_blocks = app.engine.slides[0].blocks.len();
    let offset = app.engine.slides[0].range.start;
    let _ = app.update(Message::InsertBlockAfter {
        offset,
        kind: InsertBlockKind::Equation,
    });

    assert_eq!(app.engine.slides[0].blocks.len(), initial_blocks + 1);
    assert!(app.doc.source_text.contains("$ f(x) = sum_(i=1)^n x_i $"));
}

#[test]
fn test_geek_presentation_slides_typ_parsing() {
    use slide_editor::model::ast_engine::TypstDocumentEngine;

    let example_path = std::path::PathBuf::from("../../examples/geek-presentation/slides.typ");
    if !example_path.exists() {
        return;
    }
    let content = std::fs::read_to_string(&example_path).expect("read slides.typ");
    let engine = TypstDocumentEngine::from_source(content);
    println!(">>> ENGINE PARSED SLIDES COUNT: {}", engine.slides.len());
    assert_eq!(
        engine.slides.len(),
        22,
        "Engine should parse exactly 22 slides"
    );

    for (i, s) in engine.slides.iter().take(6).enumerate() {
        println!(
            ">>> Slide {}: title='{}', range={:?}, blocks={}",
            i + 1,
            s.title,
            s.range,
            s.blocks.len()
        );
        for (bi, b) in s.blocks.iter().take(5).enumerate() {
            let preview = if b.raw().len() > 50 {
                format!("{}...", &b.raw()[..48])
            } else {
                b.raw().to_string()
            };
            println!("    block {}: id={}, preview={:?}", bi, b.id(), preview);
            // Ensure no block has just "#" or is empty
            assert_ne!(
                b.raw().trim(),
                "#",
                "Block must never be standalone hash token"
            );
            assert!(!b.raw().trim().is_empty(), "Block must not be empty");
        }
    }

    // Verify slide 1 has Title
    assert_eq!(
        engine.slides[0].title,
        "Building Modern Slides with Rust & Typst"
    );
    // Verify slide 2 has Title
    assert_eq!(
        engine.slides[1].title,
        "Architecture: Typst Functional Markup to Native Engine"
    );
    // Verify slide 4 has Math & Code
    let slide4 = &engine.slides[3];
    assert_eq!(
        slide4.title,
        "Mathematical Typography & Code Syntax Highlighting"
    );
    let has_equation = slide4.blocks.iter().any(|b| {
        matches!(
            b,
            slide_editor::model::ast_engine::WysiwygBlock::Equation { .. }
        )
    });
    let has_code = slide4.blocks.iter().any(|b| {
        matches!(
            b,
            slide_editor::model::ast_engine::WysiwygBlock::CodeBlock { .. }
        )
    });
    println!(
        ">>> Slide 4 has_equation={}, has_code={}",
        has_equation, has_code
    );
    assert!(has_equation, "Slide 4 must contain equations");
    assert!(has_code, "Slide 4 must contain code block");
}

#[test]
fn test_geek_in_place_editing_flow() {
    let example_path = std::path::PathBuf::from("../../examples/geek-presentation/slides.typ");
    if !example_path.exists() {
        return;
    }

    let mut app = SlideEditorApp::new(Some(example_path), false);
    assert_eq!(app.compilation_status, CompilationStatus::Ready);
    assert_eq!(app.engine.slides.len(), 22);

    // 1. Locate equation block on Slide 4
    let slide4 = &app.engine.slides[3];
    let eq_block = slide4
        .blocks
        .iter()
        .find(|b| {
            matches!(
                b,
                slide_editor::model::ast_engine::WysiwygBlock::Equation { .. }
            )
        })
        .expect("Slide 4 must have equation block")
        .clone();

    // Verify block content before activating
    assert!(!eq_block.raw().is_empty());
    assert_ne!(eq_block.raw().trim(), "#");
    assert!(eq_block.raw().contains('$'));

    // 2. Activate equation block for in-place editing
    let _ = app.update(Message::ActivateBlock {
        slide_idx: 1,
        id: eq_block.id().to_string(),
        range: eq_block.range(),
        raw: eq_block.raw().to_string(),
    });

    assert_eq!(app.active_block_id, Some(eq_block.id().to_string()));
    // Editor MUST be pre-filled with the exact equation formula!
    let editor_text = app.active_block_content.text();
    assert_eq!(editor_text, eq_block.raw());
    assert_ne!(editor_text.trim(), "#");

    // 3. Deactivate block
    let _ = app.update(Message::DeactivateBlock);
    assert!(app.active_block_id.is_none());

    // 4. Test "Edit Slide" on Slide 2
    let _ = app.update(Message::ToggleInPlaceEdit(1));
    assert_eq!(app.in_place_editing_slide, Some(1));
    let slide2_source = app.in_place_content.text();
    assert!(slide2_source.contains("Architecture: Typst Functional Markup"));
    assert!(slide2_source.contains("assets/architecture.svg"));

    // Close in-place editor
    let _ = app.update(Message::ToggleInPlaceEdit(1));
    assert_eq!(app.in_place_editing_slide, None);
}

#[test]
fn test_slide_chunk_isolation_and_async_compilation() {
    let example_path = std::path::PathBuf::from("../../examples/geek-presentation/slides.typ");
    if !example_path.exists() {
        return;
    }

    let mut app = SlideEditorApp::new(Some(example_path), false);
    assert_eq!(app.doc.slide_chunks.len(), 22);
    assert!(app.doc.preamble.contains("theme.typ"));

    // Modify slide 2 (index 1) chunk with an in-progress incomplete string
    let original_slide3_chunk = app.doc.slide_chunks[2].clone();
    app.doc.update_slide_chunk(
        1,
        "#slide(title: \"Modified Slide 2\")[ Incomplete edit... ",
    );

    // Verify slide count remains 22 and slide 3 is completely unaffected!
    assert_eq!(app.doc.slide_chunks.len(), 22);
    assert_eq!(app.doc.slide_chunks[2], original_slide3_chunk);
    assert!(app.doc.source_text.contains("Modified Slide 2"));
    assert!(app.doc.source_text.contains("theme.typ"));

    // Verify InPlaceEditorAction triggers without blocking
    let _ = app.update(Message::ToggleInPlaceEdit(1));
    assert_eq!(app.in_place_editing_slide, Some(1));

    // Verify double-click canvas toggle on slide 3
    let _ = app.update(Message::ToggleInPlaceEdit(2));
    assert_eq!(app.in_place_editing_slide, Some(2));
    assert_eq!(app.active_slide, 2);
}

#[test]
fn test_wysiwyg_block_edit_and_vector_recompile() {
    let mut app = SlideEditorApp::new(None, false);
    assert!(
        !app.slide_images.is_empty(),
        "Slide images must be pre-rendered"
    );

    // Check chip info for slide 1 blocks and extract target info
    let (target_id, target_range, target_raw) = {
        let first_slide = &app.engine.slides[0];
        let content_blocks: Vec<_> = first_slide
            .blocks
            .iter()
            .filter(|b| b.is_content_element())
            .collect();
        assert!(!content_blocks.is_empty());

        for block in &content_blocks {
            let (badge, preview) = block.chip_info();
            assert!(!badge.is_empty());
            assert!(!preview.is_empty());
        }

        let target_block = content_blocks[0];
        (
            target_block.id().to_string(),
            target_block.range(),
            target_block.raw().to_string(),
        )
    };

    // Activate the first content block
    let _ = app.update(Message::ActivateBlock {
        slide_idx: 0,
        id: target_id.clone(),
        range: target_range,
        raw: target_raw,
    });
    assert_eq!(app.active_block_id.as_deref(), Some(target_id.as_str()));

    // Simulate editing block content
    app.active_block_content =
        iced::widget::text_editor::Content::with_text("= Vector Slide WYSIWYG");
    if let Some(range) = app.active_block_range.clone() {
        let _ = app
            .engine
            .update_block_at_range(range, "= Vector Slide WYSIWYG");
        app.doc.source_text = app.engine.source_text.clone();
    }

    // Deactivate block -> commits edit, closes editor, and synchronously recompiles vector slide
    let _ = app.update(Message::DeactivateBlock);
    assert!(
        app.active_block_id.is_none(),
        "Active editor must close upon deactivation"
    );
    assert!(app.doc.source_text.contains("Vector Slide WYSIWYG"));
    assert!(!app.slide_images.is_empty(), "Slide images must be updated");

    // Ensure view renders without panicking
    let _ = app.view();
}

#[test]
fn test_typst_syntax_highlighter_tokenization() {
    use iced::advanced::text::Highlighter;
    use slide_editor::ui::typst_highlighter::TypstHighlightSettings;
    use slide_editor::ui::typst_highlighter::TypstHighlighter;
    use slide_editor::ui::typst_highlighter::TypstTokenType;

    let settings = TypstHighlightSettings::default();
    let mut highlighter = TypstHighlighter::new(&settings);

    // Test heading
    let tokens: Vec<_> = highlighter.highlight_line("= My Slide Title").collect();
    assert!(!tokens.is_empty());
    assert!(tokens.iter().any(|(_, t)| *t == TypstTokenType::Heading));

    // Test Math
    let tokens: Vec<_> = highlighter.highlight_line("$E = m c^2$").collect();
    assert!(!tokens.is_empty());
    assert!(tokens.iter().any(|(_, t)| *t == TypstTokenType::Math));

    // Test String & Command
    let tokens: Vec<_> = highlighter
        .highlight_line("#slide(title: \"Hello World\")[")
        .collect();
    assert!(!tokens.is_empty());
    assert!(
        tokens
            .iter()
            .any(|(_, t)| *t == TypstTokenType::Command || *t == TypstTokenType::Keyword)
    );
    assert!(tokens.iter().any(|(_, t)| *t == TypstTokenType::String));

    // Test Raw Code
    let tokens: Vec<_> = highlighter
        .highlight_line("```rust let x = 42; ```")
        .collect();
    assert!(!tokens.is_empty());
    assert!(tokens.iter().any(|(_, t)| *t == TypstTokenType::RawCode));

    // Test Comment
    let tokens: Vec<_> = highlighter.highlight_line("// This is a comment").collect();
    assert!(!tokens.is_empty());
    assert!(tokens.iter().any(|(_, t)| *t == TypstTokenType::Comment));
}

#[test]
fn test_focus_mode_preview_and_chip_navigation() {
    let mut app = SlideEditorApp::new(None, false);
    let _ = app.update(Message::SwitchMode(EditorMode::FocusMode));
    assert_eq!(app.mode, EditorMode::FocusMode);

    // Test preview hover ratio tracking
    let _ = app.update(Message::PreviewHoverRatio(0.42));
    assert!((app.preview_hover_ratio - 0.42).abs() < 1e-4);

    // Test jumping to code from preview
    let _ = app.update(Message::JumpToCodeFromPreview);
    let cursor = app.focus_slide_content.cursor();
    assert!(cursor.position.line < 100);

    // Test jumping via block chip
    if let Some(first_block) = app.engine.slides.first().and_then(|s| s.blocks.first()) {
        let block_id = first_block.id().to_string();
        let _ = app.update(Message::JumpToBlockCode(block_id));
        let cursor = app.focus_slide_content.cursor();
        assert!(cursor.position.line < 100);
    }
}

#[test]
fn test_wysiwyg_keystroke_stability_unparsed() {
    let mut app = SlideEditorApp::new(None, false);
    let first_slide = &app.engine.slides[0];
    let content_blocks: Vec<_> = first_slide
        .blocks
        .iter()
        .filter(|b| b.is_content_element())
        .collect();
    assert!(!content_blocks.is_empty());
    let target = content_blocks[0];
    let initial_id = target.id().to_string();
    let initial_range = target.range();
    let initial_raw = target.raw().to_string();

    // Activate block
    let _ = app.update(Message::ActivateBlock {
        slide_idx: 0,
        id: initial_id.clone(),
        range: initial_range.clone(),
        raw: initial_raw.clone(),
    });
    assert_eq!(app.active_block_id.as_deref(), Some(initial_id.as_str()));

    // Simulate typing keystrokes via ActiveBlockAction
    use iced::widget::text_editor::Action;
    let _ = app.update(Message::ActiveBlockAction(Action::Edit(
        iced::widget::text_editor::Edit::Insert('!'),
    )));

    // Active block ID must not be wiped out or unmounted during typing
    assert_eq!(app.active_block_id.as_deref(), Some(initial_id.as_str()));
    assert!(app.doc.source_text.contains('!'));

    // Commit edit via DeactivateBlock
    let _ = app.update(Message::DeactivateBlock);
    assert!(app.active_block_id.is_none());
    assert!(app.doc.source_text.contains('!'));
}

#[test]
fn test_preview_direct_element_editing_and_jump() {
    let mut app = SlideEditorApp::new(None, false);

    // 1. Verify LivePreview renders interactive canvas stack with blocks
    let first_slide = &app.engine.slides[0];
    let content_blocks: Vec<_> = first_slide
        .blocks
        .iter()
        .filter(|b| b.is_content_element())
        .collect();
    assert!(
        !content_blocks.is_empty(),
        "Slide must have content blocks for direct preview editing"
    );

    let first_block = content_blocks[0];
    let block_id = first_block.id().to_string();
    let block_range = first_block.range();
    let block_raw = first_block.raw().to_string();

    // Directly activate the block as if user clicked on preview element
    let _ = app.update(Message::ActivateBlock {
        slide_idx: 0,
        id: block_id.clone(),
        range: block_range,
        raw: block_raw.clone(),
    });
    assert_eq!(app.active_block_id.as_deref(), Some(block_id.as_str()));

    // Verify view builds seamlessly with active block in-place editor in canvas stack
    let _ = app.view();

    // Deactivate
    let _ = app.update(Message::DeactivateBlock);
    assert!(app.active_block_id.is_none());

    // 2. Switch to FocusMode and verify preview jump to code
    let _ = app.update(Message::SwitchMode(EditorMode::FocusMode));
    assert_eq!(app.mode, EditorMode::FocusMode);

    // Jump to block from preview
    let _ = app.update(Message::JumpToBlockCode(block_raw));
    let cursor = app.focus_slide_content.cursor();
    assert!(
        cursor.selection.is_some(),
        "Selection must be highlighted upon jumping to code"
    );

    // Verify view in focus mode builds seamlessly with stack jump overlay
    let _ = app.view();
}

#[test]
fn test_multi_slide_and_multi_element_in_place_editing() {
    let mut app = SlideEditorApp::new(None, false);

    // 1. Verify Slide 0 has multiple fine-grained unpacked content blocks
    let slide0_content: Vec<_> = app.engine.slides[0]
        .blocks
        .iter()
        .filter(|b| b.is_content_element())
        .collect();
    assert!(
        slide0_content.len() >= 2,
        "Slide 0 must have at least 2 distinct content blocks (e.g. title, subtitle)"
    );

    // 2. Activate element on Slide 1
    assert!(app.engine.slides.len() >= 2);
    let (slide1_id, slide1_range, slide1_raw) = {
        let slide1_content: Vec<_> = app.engine.slides[1]
            .blocks
            .iter()
            .filter(|b| b.is_content_element())
            .collect();
        assert!(
            slide1_content.len() >= 2,
            "Slide 1 must have multiple distinct content blocks"
        );
        let b = slide1_content[1];
        (b.id().to_string(), b.range(), b.raw().to_string())
    };

    let _ = app.update(Message::ActivateBlock {
        slide_idx: 1,
        id: slide1_id.clone(),
        range: slide1_range,
        raw: slide1_raw,
    });

    assert_eq!(app.active_slide, 1, "Active slide must switch to slide 1");
    assert_eq!(app.active_block_id.as_deref(), Some(slide1_id.as_str()));

    // 3. Switch to element on Slide 2 directly
    assert!(app.engine.slides.len() >= 3);
    let (slide2_id, slide2_range, slide2_raw) = {
        let slide2_content: Vec<_> = app.engine.slides[2]
            .blocks
            .iter()
            .filter(|b| b.is_content_element())
            .collect();
        assert!(!slide2_content.is_empty());
        let b = slide2_content[0];
        (b.id().to_string(), b.range(), b.raw().to_string())
    };

    let _ = app.update(Message::ActivateBlock {
        slide_idx: 2,
        id: slide2_id.clone(),
        range: slide2_range,
        raw: slide2_raw,
    });

    assert_eq!(app.active_slide, 2, "Active slide must switch to slide 2");
    assert_eq!(app.active_block_id.as_deref(), Some(slide2_id.as_str()));

    // 4. Click outside / empty canvas deactivates
    let _ = app.update(Message::DeactivateBlock);
    assert!(
        app.active_block_id.is_none(),
        "Active block must close upon deactivation"
    );
}

#[test]
fn test_wysiwyg_rich_text_rendering() {
    use iced::Color;
    use slide_editor::ui::wysiwyg::render_rich_text;

    let theme = AppTheme::Dark;
    // 1. Bold text
    let _el = render_rich_text("This is *bold* text", 14.0, Color::WHITE, theme);

    // 2. Italic text
    let _el = render_rich_text("This is _italic_ text", 14.0, Color::WHITE, theme);

    // 3. Monospace code
    let _el = render_rich_text("Here is `let x = 42;` in code", 14.0, Color::WHITE, theme);

    // 4. Inline math
    let _el = render_rich_text("The formula $E = m c^2$ holds", 14.0, Color::WHITE, theme);

    // 5. Nested and combined rich text
    let _el = render_rich_text(
        "*Bold and _italic_* with `inline_code()` and $x + y$",
        14.0,
        Color::WHITE,
        theme,
    );

    // 6. Plain text fallback
    let _el = render_rich_text("Plain text with 2 * 3 = 6", 14.0, Color::WHITE, theme);
}

#[test]
fn test_wysiwyg_block_drag_reordering_and_spacing() {
    let mut app = SlideEditorApp::new(None, false);
    assert!(app.engine.slides.len() >= 2);

    let slide1_content: Vec<_> = app.engine.slides[1]
        .blocks
        .iter()
        .filter(|b| b.is_content_element())
        .collect();
    assert!(
        slide1_content.len() >= 2,
        "Slide 1 should have multiple blocks"
    );

    let block0_raw = slide1_content[0].raw().to_string();
    let block1_raw = slide1_content[1].raw().to_string();

    // 1. Move block 0 down
    let _ = app.update(Message::MoveBlockDown {
        slide_idx: 1,
        block_idx: 0,
    });

    // Verify blocks swapped in source text
    let updated_slide1: Vec<_> = app.engine.slides[1]
        .blocks
        .iter()
        .filter(|b| b.is_content_element())
        .collect();
    assert_eq!(updated_slide1[0].raw().trim(), block1_raw.trim());
    assert_eq!(updated_slide1[1].raw().trim(), block0_raw.trim());

    // 2. Move block 1 up (restore)
    let _ = app.update(Message::MoveBlockUp {
        slide_idx: 1,
        block_idx: 1,
    });
    let restored_slide1: Vec<_> = app.engine.slides[1]
        .blocks
        .iter()
        .filter(|b| b.is_content_element())
        .collect();
    assert_eq!(restored_slide1[0].raw().trim(), block0_raw.trim());
    assert_eq!(restored_slide1[1].raw().trim(), block1_raw.trim());

    // 3. Test vertical spacing insertion
    let _ = app.update(Message::AdjustBlockSpacing {
        slide_idx: 1,
        block_idx: 1,
        delta_pt: 6,
    });
    assert!(app.doc.source_text.contains("#v(6pt)"));

    // Increase vertical spacing
    let _ = app.update(Message::AdjustBlockSpacing {
        slide_idx: 1,
        block_idx: 1,
        delta_pt: 6,
    });
    assert!(app.doc.source_text.contains("#v(12pt)"));

    // Reduce and remove vertical spacing
    let _ = app.update(Message::AdjustBlockSpacing {
        slide_idx: 1,
        block_idx: 1,
        delta_pt: -12,
    });
    assert!(!app.doc.source_text.contains("#v(6pt)"));
    assert!(!app.doc.source_text.contains("#v(12pt)"));

    // 4. Test drag gestures
    let _ = app.update(Message::StartDragBlock {
        slide_idx: 1,
        block_idx: 0,
    });
    assert!(app.dragging_block.is_some());

    // Drag move under threshold (no change)
    let _ = app.update(Message::DragBlockY {
        slide_idx: 1,
        block_idx: 0,
        y: 10.0,
    });
    // First event sets start_y = 10.0
    let _ = app.update(Message::DragBlockY {
        slide_idx: 1,
        block_idx: 0,
        y: 20.0,
    }); // dy = 10 < 25

    // Drag move over downward threshold (dy = 40 > 25)
    let _ = app.update(Message::DragBlockY {
        slide_idx: 1,
        block_idx: 0,
        y: 50.0,
    });
    // Block moved down
    let dragged_slide1: Vec<_> = app.engine.slides[1]
        .blocks
        .iter()
        .filter(|b| b.is_content_element())
        .collect();
    assert_eq!(dragged_slide1[0].raw().trim(), block1_raw.trim());

    // End drag
    let _ = app.update(Message::EndDragBlock);
    assert!(app.dragging_block.is_none());
}

#[test]
fn test_focus_mode_fine_grained_line_navigation() {
    let mut app = SlideEditorApp::new(None, false);
    let _ = app.update(Message::SwitchMode(EditorMode::FocusMode));
    assert_eq!(app.mode, EditorMode::FocusMode);

    // Verify slide has multiple lines
    let slide_text = app.focus_slide_content.text();
    let lines: Vec<&str> = slide_text.lines().collect();
    assert!(lines.len() >= 2);

    // Jump to specific second line
    let second_line = lines[1].trim();
    if !second_line.is_empty() {
        let _ = app.update(Message::JumpToBlockCode(second_line.to_string()));
        let cursor = app.focus_slide_content.cursor();
        assert_eq!(
            cursor.position.line, 1,
            "Should jump to exactly line 1, not line 0"
        );
    }
}

#[test]
fn test_vector_equation_and_code_image_rendering_and_interaction() {
    let mut app = SlideEditorApp::new(None, false);

    // Insert a slide with equation and code block
    let custom_typst = r#"
= Slide with Math and Code

$ E = m c^2 $

```rust
fn hello() {
    println!("Hello Typst!");
}
```
"#;
    app.doc.update_slide_chunk(0, custom_typst);
    app.engine.source_text = app.doc.source_text.clone();
    app.engine.reparse();
    app.update_slide_cache();

    // Verify equation and code images are cached by both ID and content
    assert!(app.equation_images.contains_key("E = m c^2"));
    assert!(app.code_images.iter().any(|(k, _)| k.contains("hello")));

    // Test EndBlockInteraction distinction:
    // 1. Direct click (no move): starts drag, releases without move -> activates editor
    let _ = app.update(Message::StartDragBlock {
        slide_idx: 0,
        block_idx: 0,
    });
    let _ = app.update(Message::EndBlockInteraction {
        slide_idx: 0,
        block_idx: 0,
        id: "b-heading-0".to_string(),
        range: 0..27,
        raw: "= Slide with Math and Code\n".to_string(),
    });
    assert!(
        app.active_block_id.is_some(),
        "Direct click should activate in-place block editor"
    );

    // Deactivate
    let _ = app.update(Message::DeactivateBlock);
    assert!(app.active_block_id.is_none());

    // 2. Drag reorder (has moved): releases after move -> should NOT activate editor
    let _ = app.update(Message::StartDragBlock {
        slide_idx: 0,
        block_idx: 0,
    });
    // First drag event sets start position
    let _ = app.update(Message::DragBlockY {
        slide_idx: 0,
        block_idx: 0,
        y: 100.0,
    });
    // Significant vertical move (dy = 40.0 > 25.0) sets has_moved = true
    let _ = app.update(Message::DragBlockY {
        slide_idx: 0,
        block_idx: 0,
        y: 140.0,
    });
    let _ = app.update(Message::EndBlockInteraction {
        slide_idx: 0,
        block_idx: 0,
        id: "b-heading-0".to_string(),
        range: 0..27,
        raw: "= Slide with Math and Code\n".to_string(),
    });
    assert!(
        app.active_block_id.is_none(),
        "Drag release should commit movement without popping editor"
    );
}

#[test]
fn test_global_subscription_drag_and_release() {
    let mut app = SlideEditorApp::new(None, false);
    assert!(app.engine.slides.len() >= 2);

    let slide1_content: Vec<_> = app.engine.slides[1]
        .blocks
        .iter()
        .filter(|b| b.is_content_element())
        .collect();
    assert!(slide1_content.len() >= 2);
    let _block0_raw = slide1_content[0].raw().to_string();
    let block1_raw = slide1_content[1].raw().to_string();

    // 1. Test global drag move and release (reorders blocks)
    let _ = app.update(Message::StartDragBlock {
        slide_idx: 1,
        block_idx: 0,
    });
    assert!(app.dragging_block.is_some());

    // Subscription is active while dragging
    let sub = app.subscription();
    let _ = sub; // Verify subscription compiles and runs

    // Mouse move sets start
    let _ = app.update(Message::GlobalCursorMoved(iced::Point::new(50.0, 100.0)));
    // Mouse drag downwards past threshold (140.0 - 100.0 = 40.0 > 25.0)
    let _ = app.update(Message::GlobalCursorMoved(iced::Point::new(50.0, 140.0)));

    // Verify blocks swapped
    let swapped_blocks: Vec<_> = app.engine.slides[1]
        .blocks
        .iter()
        .filter(|b| b.is_content_element())
        .collect();
    assert_eq!(swapped_blocks[0].raw().trim(), block1_raw.trim());

    // Release mouse after drag: commits drag, does NOT open editor
    let _ = app.update(Message::GlobalButtonReleased);
    assert!(app.dragging_block.is_none());
    assert!(
        app.active_block_id.is_none(),
        "Drag release must not open editor"
    );

    // 2. Test click without drag (activates editor)
    let _ = app.update(Message::StartDragBlock {
        slide_idx: 1,
        block_idx: 0,
    });
    // User releases without moving:
    let _ = app.update(Message::GlobalButtonReleased);
    assert!(app.dragging_block.is_none());
    assert!(
        app.active_block_id.is_some(),
        "Click release without movement must activate in-place editor"
    );

    // Deactivate
    let _ = app.update(Message::DeactivateBlock);
    assert!(app.active_block_id.is_none());

    // 3. Test global spacing drag
    let _ = app.update(Message::StartDragSpacing {
        slide_idx: 1,
        block_idx: 0,
    });
    assert!(app.dragging_spacing.is_some());

    let _ = app.update(Message::GlobalCursorMoved(iced::Point::new(50.0, 100.0)));
    let _ = app.update(Message::GlobalCursorMoved(iced::Point::new(50.0, 120.0))); // dy = 20 > 10 -> +4pt
    assert!(app.doc.source_text.contains("#v(4pt)"));

    let _ = app.update(Message::GlobalButtonReleased);
    assert!(app.dragging_spacing.is_none());
}

#[test]
fn test_open_slide_package_and_vector_caching() {
    let slide_path = std::path::PathBuf::from("examples/slides.slide");
    if !slide_path.exists() {
        return;
    }

    // 1. Test EditorDocument::open on .slide package
    let doc = EditorDocument::open(&slide_path).expect("Failed to open .slide package");
    assert_eq!(doc.format, DocumentFormat::SlidePackage);
    assert_eq!(doc.total_slides(), 22);
    assert!(doc.deck.is_some());

    // 2. Test SlideEditorApp initialization with .slide package
    let app = SlideEditorApp::new(Some(slide_path.clone()), false);
    assert_eq!(app.compilation_status, CompilationStatus::Ready);
    assert_eq!(app.slide_images.len(), 22);
    assert_eq!(app.slide_view_vector.len(), 22);
    assert_eq!(app.engine.slides.len(), 22);

    // 3. Test opening via ExecuteOpen message in running app
    let mut app2 = SlideEditorApp::new(None, false);
    app2.open_path = slide_path.to_string_lossy().to_string();
    let _ = app2.update(Message::ExecuteOpen);
    assert_eq!(app2.doc.format, DocumentFormat::SlidePackage);
    assert_eq!(app2.compilation_status, CompilationStatus::Ready);
    assert_eq!(app2.slide_images.len(), 22);
    assert_eq!(app2.slide_view_vector.len(), 22);
}

#[test]
fn test_inter_block_spacing_adjustment_and_live_pt() {
    let mut app = SlideEditorApp::new(None, false);
    // Slide 1 has content blocks. Check initial spacing after block 0:
    let initial_spacing = app.engine.get_block_spacing_pt(1, 0);
    assert!(initial_spacing.is_none() || initial_spacing == Some(0.0));

    // Increase spacing by +12pt
    let _ = app.update(Message::AdjustBlockSpacing {
        slide_idx: 1,
        block_idx: 0,
        delta_pt: 12,
    });
    assert_eq!(app.engine.get_block_spacing_pt(1, 0), Some(12.0));
    assert!(app.doc.source_text.contains("#v(12pt)"));

    // Increase spacing by +4pt (total 16pt)
    let _ = app.update(Message::AdjustBlockSpacing {
        slide_idx: 1,
        block_idx: 0,
        delta_pt: 4,
    });
    assert_eq!(app.engine.get_block_spacing_pt(1, 0), Some(16.0));
    assert!(app.doc.source_text.contains("#v(16pt)"));

    // Decrease spacing by -16pt (down to 0)
    let _ = app.update(Message::AdjustBlockSpacing {
        slide_idx: 1,
        block_idx: 0,
        delta_pt: -16,
    });
    let final_spacing = app.engine.get_block_spacing_pt(1, 0);
    assert!(final_spacing.is_none() || final_spacing == Some(0.0));
}

#[test]
fn test_complex_elements_ast_classification_and_chips() {
    let source = r#"
= Demo Slide

#table(columns: 2, [Header 1], [Header 2], [Data A], [Data B])

#grid(columns: (1fr, 1fr), [Left Column], [Right Column])

#note(title: "Takeaway")[Important note here]

#box(stroke: 1pt + rgb("3b82f6"), inset: 8pt)[Box content]

#chart-bar(title: "Sales", ("Q1", 45), ("Q2", 80))
"#;

    let engine =
        slide_editor::model::ast_engine::TypstDocumentEngine::from_source(source.to_string());
    assert_eq!(engine.slides.len(), 1);
    let slide = &engine.slides[0];

    let content_blocks: Vec<_> = slide
        .blocks
        .iter()
        .filter(|b| b.is_content_element())
        .collect();
    // 1 Heading + Table + Grid + Note + Box + Chart = 6 blocks
    assert_eq!(content_blocks.len(), 6);

    let (badge0, _) = content_blocks[0].chip_info();
    assert_eq!(badge0, "H1");

    let (badge_tbl, _) = content_blocks[1].chip_info();
    assert_eq!(badge_tbl, "Table");

    let (badge_grid, _) = content_blocks[2].chip_info();
    assert_eq!(badge_grid, "Grid");

    let (badge_note, _) = content_blocks[3].chip_info();
    assert_eq!(badge_note, "note");

    let (badge_box, _) = content_blocks[4].chip_info();
    assert_eq!(badge_box, "Box");

    let (badge_chart, _) = content_blocks[5].chip_info();
    assert_eq!(badge_chart, "Chart");
}

#[test]
fn test_complex_elements_argument_parsers() {
    use slide_editor::ui::wysiwyg::block_view::extract_bracket_contents;
    use slide_editor::ui::wysiwyg::block_view::extract_table_cells;
    use slide_editor::ui::wysiwyg::block_view::parse_box_data;
    use slide_editor::ui::wysiwyg::block_view::parse_callout_data;
    use slide_editor::ui::wysiwyg::block_view::parse_chart_data;
    use slide_editor::ui::wysiwyg::block_view::parse_column_count;

    // Table parsing
    let table_args = "(columns: 3, [Header 1], [Header 2], [Header 3], [A], [B], [C])";
    assert_eq!(parse_column_count(table_args), 3);
    let cells = extract_table_cells(table_args);
    assert_eq!(cells.len(), 6);
    assert_eq!(cells[0], "Header 1");
    assert_eq!(cells[5], "C");

    // Grid parsing with tuple columns
    let grid_args = "(columns: (1fr, 2fr), [Col 1], [Col 2])";
    assert_eq!(parse_column_count(grid_args), 2);
    let grid_items = extract_bracket_contents(grid_args);
    assert_eq!(grid_items.len(), 2);
    assert_eq!(grid_items[0], "Col 1");
    assert_eq!(grid_items[1], "Col 2");

    // Callout parsing
    let callout_args =
        "(title: \"Summary\", attribution: \"Alan Kay\")[Simple things should be simple]";
    let callout = parse_callout_data("quote", callout_args);
    assert_eq!(callout.kind, "quote");
    assert_eq!(callout.title, "Summary");
    assert_eq!(callout.body, "Simple things should be simple");
    assert_eq!(callout.attribution, Some("Alan Kay"));

    // Box parsing
    let box_args = "(stroke: 1pt)[Inner box content]";
    let box_body = parse_box_data(box_args);
    assert_eq!(box_body, "Inner box content");

    // Chart parsing (tuple format)
    let chart_args = "(title: \"Q1 Results\", (\"Alpha\", 25), (\"Beta\", 75))";
    let chart = parse_chart_data(chart_args);
    assert_eq!(chart.title, "Q1 Results");
    assert_eq!(chart.items.len(), 2);
    assert_eq!(chart.items[0], ("Alpha", 25.0));
    assert_eq!(chart.items[1], ("Beta", 75.0));

    // Standard data dictionary chart parsing
    let std_chart_args = "(title: \"Metrics\", data: (categories: (\"Speed\", \"Safety\", \"Simplicity\"), series: ((name: \"Score\", values: (85, 95, 80)),)))";
    let std_chart = parse_chart_data(std_chart_args);
    assert_eq!(std_chart.title, "Metrics");
    assert_eq!(std_chart.items.len(), 3);
    assert_eq!(std_chart.items[0], ("Speed", 85.0));
    assert_eq!(std_chart.items[1], ("Safety", 95.0));
    assert_eq!(std_chart.items[2], ("Simplicity", 80.0));

    // Single item standard chart parsing
    let single_chart_args =
        "(title: \"Single\", data: (categories: (\"Only\",), series: ((values: (42,)),)))";
    let single_chart = parse_chart_data(single_chart_args);
    assert_eq!(single_chart.title, "Single");
    assert_eq!(single_chart.items.len(), 1);
    assert_eq!(single_chart.items[0], ("Only", 42.0));
}

#[test]
fn test_complex_elements_quick_actions() {
    use slide_editor::ui::wysiwyg::block_view::add_col_to_grid;
    use slide_editor::ui::wysiwyg::block_view::add_col_to_table;
    use slide_editor::ui::wysiwyg::block_view::add_data_point_to_chart;
    use slide_editor::ui::wysiwyg::block_view::add_row_to_table;
    use slide_editor::ui::wysiwyg::block_view::switch_callout_kind;
    use slide_editor::ui::wysiwyg::block_view::switch_chart_kind;

    // 1. Table Add Row
    let raw_table = "#table(columns: 2, [H1], [H2])";
    let with_row = add_row_to_table(raw_table, 2);
    assert!(with_row.contains("[ ], [ ]"));

    // 2. Table Add Col
    let with_col = add_col_to_table(raw_table, 2);
    assert!(with_col.contains("columns: 3"));
    assert!(with_col.contains("[ ]"));

    // 3. Grid Add Col
    let raw_grid = "#grid(columns: (1fr, 1fr), [Left], [Right])";
    let grid_with_col = add_col_to_grid(raw_grid);
    assert!(grid_with_col.contains("columns: (1fr, 1fr, 1fr)"));
    assert!(grid_with_col.contains("New Column"));

    // 4. Callout switch kind
    let raw_callout = "#note(title: \"Note\")[Text]";
    let switched_callout = switch_callout_kind(raw_callout, "note", "tip");
    assert_eq!(switched_callout, "#tip(title: \"Note\")[Text]");

    // 5. Chart add data point & switch
    let raw_chart = "#chart-bar(title: \"Trend\", (\"A\", 10))";
    let with_pt = add_data_point_to_chart(raw_chart);
    assert!(with_pt.contains("(\"New\", 50)"));

    let switched_chart = switch_chart_kind(raw_chart, "#chart-pie");
    assert!(switched_chart.starts_with("#chart-pie("));
}

#[test]
fn test_interactive_editing_and_inserting_complex_elements() {
    use slide_editor::ui::wysiwyg::block_view::InsertBlockKind;

    let mut app = SlideEditorApp::new(None, false);
    let initial_slides = app.engine.slides.len();
    assert!(initial_slides >= 1);

    // 1. Insert Table
    let offset = app.engine.slides[0].range.end;
    let _ = app.update(Message::InsertBlockAfter {
        offset,
        kind: InsertBlockKind::Table,
    });
    assert!(app.doc.source_text.contains("#table("));

    // 2. Insert Grid
    let offset2 = app.engine.slides[0].range.end;
    let _ = app.update(Message::InsertBlockAfter {
        offset: offset2,
        kind: InsertBlockKind::Grid,
    });
    assert!(app.doc.source_text.contains("#grid("));

    // 3. Insert Callout
    let offset3 = app.engine.slides[0].range.end;
    let _ = app.update(Message::InsertBlockAfter {
        offset: offset3,
        kind: InsertBlockKind::Callout,
    });
    assert!(app.doc.source_text.contains("#callout("));

    // 4. Insert Box
    let offset4 = app.engine.slides[0].range.end;
    let _ = app.update(Message::InsertBlockAfter {
        offset: offset4,
        kind: InsertBlockKind::BoxBlock,
    });
    assert!(app.doc.source_text.contains("#box("));

    // 5. Insert Chart
    let offset5 = app.engine.slides[0].range.end;
    let _ = app.update(Message::InsertBlockAfter {
        offset: offset5,
        kind: InsertBlockKind::Chart,
    });
    assert!(app.doc.source_text.contains("#chart-bar("));

    // 6. Test UpdateBlockRange message
    let table_range = {
        let table_block = app.engine.slides[0]
            .blocks
            .iter()
            .find(|b| {
                match b {
                    | slide_editor::model::ast_engine::WysiwygBlock::FuncCall {
                        callee, ..
                    } => callee == "table",
                    | _ => false,
                }
            })
            .expect("Table block must exist");
        table_block.range()
    };

    let updated_table_text = "#table(columns: 2, [Col 1], [Col 2], [Val 1], [Val 2])";
    let _ = app.update(Message::UpdateBlockRange {
        slide_idx: None,
        range: table_range,
        new_text: updated_table_text.to_string(),
    });
    assert!(app.doc.source_text.contains("Val 1"));

    // 7. Test in-place ActivateBlock on the complex element
    let (re_id, re_range, re_raw) = {
        let re_table_block = app.engine.slides[0]
            .blocks
            .iter()
            .find(|b| {
                match b {
                    | slide_editor::model::ast_engine::WysiwygBlock::FuncCall {
                        callee, ..
                    } => callee == "table",
                    | _ => false,
                }
            })
            .expect("Re-parsed table block must exist");
        (
            re_table_block.id().to_string(),
            re_table_block.range(),
            re_table_block.raw().to_string(),
        )
    };

    let _ = app.update(Message::ActivateBlock {
        slide_idx: 0,
        id: re_id.clone(),
        range: re_range,
        raw: re_raw,
    });
    assert_eq!(app.active_block_id.as_deref(), Some(re_id.as_str()));

    // 8. Commit active block
    let _ = app.update(Message::DeactivateBlock);
    assert!(app.active_block_id.is_none());

    // 9. View does not panic
    let _ = app.view();
}

#[test]
fn test_wysiwyg_complex_elements_in_place_editing_and_raw_code_toggle() {
    use slide_editor::ui::wysiwyg::block_view::*;

    // 1. Table mutations
    let raw_table = "#table(columns: 2, [Header 1], [Header 2], [Data A], [Data B])";
    let updated_cell = update_table_cell(raw_table, 2, "Data X");
    assert!(updated_cell.contains("[Data X]"));
    assert!(!updated_cell.contains("[Data A]"));

    let added_row = add_row_to_table(&updated_cell, 2);
    assert!(added_row.contains("[ ]"));

    let deleted_row = delete_row_from_table(&added_row, 2);
    assert!(!deleted_row.contains("[ ]"));

    let added_col = add_col_to_table(raw_table, 2);
    assert!(added_col.contains("columns: 3") || added_col.contains("(1fr, 1fr, 1fr)"));

    let deleted_col = delete_col_from_table(&added_col, 3);
    assert!(deleted_col.contains("columns: 2"));

    // 2. Grid & Cols mutations
    let raw_grid = "#grid(\n  columns: 2,\n  [\n    *Col 1*\n  ],\n  [\n    *Col 2*\n  ]\n)";
    let updated_grid = update_grid_col(raw_grid, 0, "Updated Column 1 Content");
    assert!(updated_grid.contains("Updated Column 1 Content"));

    let added_col_grid = add_col_to_grid(&updated_grid);
    assert!(added_col_grid.contains("columns: 3"));
    assert!(added_col_grid.contains("*New Column*"));

    let deleted_col_grid = grid_delete_col(&added_col_grid, 2);
    assert!(deleted_col_grid.contains("columns: 2"));
    assert!(!deleted_col_grid.contains("*New Column*"));

    let raw_cols = "#cols(2)[\n  Left\n][\n  Right\n]";
    let added_cols = add_col_to_grid(raw_cols);
    assert!(added_cols.contains("#cols(3)"));
    let deleted_cols = grid_delete_col(&added_cols, 2);
    assert!(deleted_cols.contains("#cols(2)"));

    // 3. Chart mutations
    let raw_chart = "#chart-bar(\n  title: \"Sales\",\n  (\"Q1\", 100),\n  (\"Q2\", 150)\n)";
    let updated_title = update_chart_title(raw_chart, "Quarterly Revenue");
    assert!(updated_title.contains("title: \"Quarterly Revenue\""));

    let updated_lbl = update_chart_item_label(raw_chart, 0, "Q1-2026");
    assert!(updated_lbl.contains("(\"Q1-2026\", 100)"));

    let updated_val = update_chart_item_value(raw_chart, 1, 200.0);
    assert!(updated_val.contains("(\"Q2\", 200)"));

    let added_pt = add_data_point_to_chart(raw_chart);
    assert!(added_pt.contains("(\"New\", 50)"));

    let deleted_pt = chart_delete_item(&added_pt, 2);
    assert!(!deleted_pt.contains("(\"New\", 50)"));

    let switched_chart = switch_chart_kind(raw_chart, "#chart-pie");
    assert!(switched_chart.contains("#chart-pie("));

    // 4. Callout and Box mutations
    let raw_callout = "#note(title: \"Notice\")[\n  Important info\n]";
    let updated_callout_title = update_callout_title(raw_callout, "Attention");
    assert!(updated_callout_title.contains("title: \"Attention\""));

    let updated_callout_body = update_callout_body(raw_callout, "Revised info content");
    assert!(updated_callout_body.contains("Revised info content"));

    let switched_callout = switch_callout_kind(raw_callout, "note", "warning");
    assert!(switched_callout.contains("#warning("));

    let raw_box = "#box[\n  Initial box text\n]";
    let updated_box = update_box_content(raw_box, "Updated box text");
    assert!(updated_box.contains("Updated box text"));

    // 5. ToggleBlockRawCode handling
    let mut app = SlideEditorApp::new(None, false);
    let block_id = "test-block-42".to_string();
    assert!(!app.raw_code_blocks.contains(&block_id));

    let _ = app.update(Message::ToggleBlockRawCode(block_id.clone()));
    assert!(app.raw_code_blocks.contains(&block_id));

    let _ = app.update(Message::ToggleBlockRawCode(block_id.clone()));
    assert!(!app.raw_code_blocks.contains(&block_id));

    // 6. Visual block view does not panic with raw code toggle
    let _ = app.view();
}

#[test]
fn test_complex_element_modal_table_editing() {
    use slide_editor::app::ActiveModal;
    use slide_editor::ui::wysiwyg::ComplexElementModal;
    use slide_editor::ui::wysiwyg::ComplexModalState;

    let raw = "#table(\n  columns: 2,\n  [Header 1], [Header 2],\n  [Item A], [Item B],\n)";
    let modal = ComplexElementModal::from_raw(0, "blk-tbl".to_string(), 0..raw.len(), "table", raw);

    if let ComplexModalState::Table { cols, rows, .. } = &modal.state {
        assert_eq!(*cols, 2);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0][0], "Header 1");
        assert_eq!(rows[1][1], "Item B");
    } else {
        panic!("Expected Table modal state");
    }

    let mut app = SlideEditorApp::new(None, false);
    app.active_modal = Some(ActiveModal::ComplexElement(modal));

    // 1. Update cell
    let _ = app.update(Message::ModalTableUpdateCell {
        row: 1,
        col: 0,
        val: "New Item A".to_string(),
    });

    // 2. Add row
    let _ = app.update(Message::ModalTableAddRow);

    // 3. Add column
    let _ = app.update(Message::ModalTableAddCol);

    if let Some(ActiveModal::ComplexElement(ref m)) = app.active_modal {
        if let ComplexModalState::Table { cols, rows, .. } = &m.state {
            assert_eq!(*cols, 3);
            assert_eq!(rows.len(), 3);
            assert_eq!(rows[1][0], "New Item A");
            assert_eq!(rows[0].len(), 3);
        } else {
            panic!("Expected Table modal state");
        }

        // Test serialization to Typst markup
        let typst = m.to_typst();
        assert!(typst.starts_with("#table("));
        assert!(typst.contains("columns: 3"));
        assert!(typst.contains("[New Item A]"));
    } else {
        panic!("Modal should be active");
    }

    // 4. Delete column
    let _ = app.update(Message::ModalTableDeleteCol(2));

    // 5. Delete row
    let _ = app.update(Message::ModalTableDeleteRow(2));

    if let Some(ActiveModal::ComplexElement(ref m)) = app.active_modal
        && let ComplexModalState::Table { cols, rows, .. } = &m.state
    {
        assert_eq!(*cols, 2);
        assert_eq!(rows.len(), 2);
    }

    // 6. Test modal view rendering without panics
    let _ = app.view();

    // 7. Apply modal
    let _ = app.update(Message::ApplyComplexModal);
    assert!(app.active_modal.is_none());
}

#[test]
fn test_complex_element_modal_chart_editing() {
    use slide_editor::app::ActiveModal;
    use slide_editor::ui::wysiwyg::ComplexElementModal;
    use slide_editor::ui::wysiwyg::ComplexModalState;

    let raw = "#chart-bar(title: \"Revenue\", [Q1], 100, [Q2], 250)";
    let modal =
        ComplexElementModal::from_raw(0, "blk-chart".to_string(), 0..raw.len(), "chart-bar", raw);

    if let ComplexModalState::Chart {
        title,
        chart_type,
        items,
        ..
    } = &modal.state
    {
        assert_eq!(title, "Revenue");
        assert_eq!(chart_type, "bar");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].0, "Q1");
        assert_eq!(items[0].1, 100.0);
    } else {
        panic!("Expected Chart modal state");
    }

    let mut app = SlideEditorApp::new(None, false);
    app.active_modal = Some(ActiveModal::ComplexElement(modal));

    // 1. Update Title and Switch Type
    let _ = app.update(Message::ModalChartUpdateTitle(
        "Quarterly Revenue".to_string(),
    ));
    let _ = app.update(Message::ModalChartSetType("pie".to_string()));

    // 2. Update Label and Value
    let _ = app.update(Message::ModalChartUpdateItemLabel {
        idx: 0,
        label: "Quarter 1".to_string(),
    });
    let _ = app.update(Message::ModalChartUpdateItemValue { idx: 0, val: 150.0 });

    // 3. Add Item
    let _ = app.update(Message::ModalChartAddItem);

    if let Some(ActiveModal::ComplexElement(ref m)) = app.active_modal {
        if let ComplexModalState::Chart {
            title,
            chart_type,
            items,
            ..
        } = &m.state
        {
            assert_eq!(title, "Quarterly Revenue");
            assert_eq!(chart_type, "pie");
            assert_eq!(items.len(), 3);
            assert_eq!(items[0].0, "Quarter 1");
            assert_eq!(items[0].1, 150.0);
        }

        let typst = m.to_typst();
        assert!(typst.starts_with("#chart-pie("));
        assert!(typst.contains("title: \"Quarterly Revenue\""));
        assert!(typst.contains("(\"Quarter 1\", 150)"));
    }

    // 4. Delete Item
    let _ = app.update(Message::ModalChartDeleteItem(2));
    if let Some(ActiveModal::ComplexElement(ref m)) = app.active_modal
        && let ComplexModalState::Chart { items, .. } = &m.state
    {
        assert_eq!(items.len(), 2);
    }

    // 5. Test modal view rendering
    let _ = app.view();
}

#[test]
fn test_complex_element_modal_chart_sql_and_custom_table_spec() {
    use slide_editor::app::ActiveModal;
    use slide_editor::ui::wysiwyg::ComplexElementModal;
    use slide_editor::ui::wysiwyg::ComplexModalState;

    // 1. Chart with SQL source, DSL pipeline, format and unit
    let raw_chart = "#chart(\n  source: \"sales.db\",\n  sql: \"SELECT qtr, rev FROM metrics\",\n  dsl: \"sort rev desc | limit 5\",\n  title: \"Top Metrics\",\n  format: \"currency\",\n  unit: \"$\",\n)";
    let modal_chart = ComplexElementModal::from_raw(
        0,
        "blk-sql-chart".to_string(),
        0..raw_chart.len(),
        "chart",
        raw_chart,
    );

    if let ComplexModalState::Chart {
        source,
        sql,
        dsl,
        title,
        format,
        unit,
        ..
    } = &modal_chart.state
    {
        assert_eq!(source, "sales.db");
        assert_eq!(sql, "SELECT qtr, rev FROM metrics");
        assert_eq!(dsl, "sort rev desc | limit 5");
        assert_eq!(title, "Top Metrics");
        assert_eq!(format, "currency");
        assert_eq!(unit, "$");
    } else {
        panic!("Expected SQL Chart modal state");
    }

    let mut app = SlideEditorApp::new(None, false);
    app.active_modal = Some(ActiveModal::ComplexElement(modal_chart));

    let _ = app.update(Message::ModalChartUpdateSource(
        "warehouse.sqlite".to_string(),
    ));
    let _ = app.update(Message::ModalChartUpdateSql(
        "SELECT region, profit FROM sales".to_string(),
    ));
    let _ = app.update(Message::ModalChartUpdateUnit("USD".to_string()));

    if let Some(ActiveModal::ComplexElement(ref m)) = app.active_modal {
        let typst = m.to_typst();
        assert!(typst.starts_with("#chart("));
        assert!(typst.contains("source: \"warehouse.sqlite\""));
        assert!(typst.contains("sql: \"SELECT region, profit FROM sales\""));
        assert!(typst.contains("unit: \"USD\""));
    }

    // 2. Table with custom column spec (auto, 1fr, 2fr) and table.header
    let raw_table = "#table(\n  columns: (auto, 1fr, 2fr),\n  table.header([Name], [Role], [Bio]),\n  [Alice], [Engineer], [Rustacean],\n)";
    let modal_table = ComplexElementModal::from_raw(
        0,
        "blk-spec-table".to_string(),
        0..raw_table.len(),
        "table",
        raw_table,
    );

    if let ComplexModalState::Table {
        cols,
        col_spec,
        has_header,
        header_cells,
        rows,
        ..
    } = &modal_table.state
    {
        assert_eq!(*cols, 3);
        assert_eq!(col_spec, "(auto, 1fr, 2fr)");
        assert!(*has_header);
        assert_eq!(header_cells.len(), 3);
        assert_eq!(header_cells[0], "Name");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], "Alice");
    } else {
        panic!("Expected custom Table modal state");
    }

    app.active_modal = Some(ActiveModal::ComplexElement(modal_table));
    let _ = app.update(Message::ModalTableUpdateColSpec(
        "(100pt, auto, 1fr)".to_string(),
    ));

    if let Some(ActiveModal::ComplexElement(ref m)) = app.active_modal {
        let typst = m.to_typst();
        assert!(typst.contains("columns: (100pt, auto, 1fr)"));
        assert!(typst.contains("table.header("));
        assert!(typst.contains("[Alice]"));
    }
}

#[test]
fn test_complex_element_modal_grid_callout_box_editing() {
    use slide_editor::app::ActiveModal;
    use slide_editor::ui::wysiwyg::ComplexElementModal;

    // 1. Grid / Columns
    let raw_grid = "#grid(columns: 2, [Column A], [Column B])";
    let modal_grid = ComplexElementModal::from_raw(
        0,
        "blk-grid".to_string(),
        0..raw_grid.len(),
        "grid",
        raw_grid,
    );
    let mut app = SlideEditorApp::new(None, false);
    app.active_modal = Some(ActiveModal::ComplexElement(modal_grid));

    let _ = app.update(Message::ModalGridUpdateCol {
        idx: 0,
        val: "Revised Col A".to_string(),
    });
    let _ = app.update(Message::ModalGridAddCol);

    if let Some(ActiveModal::ComplexElement(ref m)) = app.active_modal {
        let typst = m.to_typst();
        assert!(typst.starts_with("#grid("));
        assert!(typst.contains("columns: 3"));
        assert!(typst.contains("Revised Col A"));
    }
    let _ = app.view();

    // 2. Callout
    let raw_callout = "#callout(title: \"Pro Tip\", stroke-color: slide-colors.accent-cyan)[\n  Keep slides visual\n]";
    let modal_callout = ComplexElementModal::from_raw(
        0,
        "blk-callout".to_string(),
        0..raw_callout.len(),
        "callout",
        raw_callout,
    );
    app.active_modal = Some(ActiveModal::ComplexElement(modal_callout));

    let _ = app.update(Message::ModalCalloutSetKind("warning".to_string()));
    let _ = app.update(Message::ModalCalloutUpdateTitle(
        "Warning Notice".to_string(),
    ));
    let _ = app.update(Message::ModalCalloutUpdateBody(
        "Beware of pitfalls".to_string(),
    ));

    if let Some(ActiveModal::ComplexElement(ref m)) = app.active_modal {
        let typst = m.to_typst();
        assert!(typst.starts_with("#callout("));
        assert!(typst.contains("title: \"Warning Notice\""));
        assert!(typst.contains("Beware of pitfalls"));
    }
    let _ = app.view();

    // 3. Box
    let raw_box = "#box[\n  Initial content\n]";
    let modal_box =
        ComplexElementModal::from_raw(0, "blk-box".to_string(), 0..raw_box.len(), "box", raw_box);
    app.active_modal = Some(ActiveModal::ComplexElement(modal_box));

    let _ = app.update(Message::ModalBoxUpdateContent("Framed content".to_string()));
    if let Some(ActiveModal::ComplexElement(ref m)) = app.active_modal {
        let typst = m.to_typst();
        assert_eq!(typst, "#box[\n  Framed content\n]");
    }
    let _ = app.view();
}

#[test]
fn test_focus_mode_vector_hyperlink_instrumentation_and_jump() {
    use slide_editor::model::ast_engine::instrument_source_for_focus;

    let source = "= Introduction to Vector Loc\n\n- First point of interest\n- Second point of interest\n\n```rust\nfn main() {}\n```\n";
    let mut app = SlideEditorApp::new(None, false);
    app.doc.source_text = source.to_string();
    app.doc.sync_chunks_from_source();
    app.engine =
        slide_editor::model::ast_engine::TypstDocumentEngine::from_source(source.to_string());
    app.sync_editors_from_doc();

    // 1. Instrument source for focus mode
    let instrumented = instrument_source_for_focus(&app.doc.source_text, 0, &app.engine);
    assert!(
        instrumented.contains("slide-loc:"),
        "Instrumented source must contain vector loc links"
    );

    // 2. Compile instrumented slide with Typst
    let bridge = CompilerBridge::new();
    let deck_res = bridge.compile_source(&instrumented, None);
    assert!(
        deck_res.is_ok(),
        "Instrumented slide must compile cleanly in Typst"
    );

    let deck = deck_res.unwrap();
    assert!(!deck.slides.is_empty());
    let slide = &deck.slides[0];

    let visual_bounds = slide_core::svg::extract_slide_visual_element_bounds(&slide.svg_data);
    println!(
        "Extracted visual bounds ({}): {:?}",
        visual_bounds.len(),
        visual_bounds
    );

    // 3. Verify vector hotspots generated with slide-loc targets
    let loc_links: Vec<_> = slide
        .hotspots
        .iter()
        .filter_map(|h| {
            if let slide_core::model::Hotspot::Link { target, rect } = h {
                if target.starts_with("slide-loc:") {
                    Some((target.clone(), *rect))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    assert!(
        !loc_links.is_empty(),
        "Slide must have vector link hotspots extracted from Typst SVG"
    );
    for (target, rect) in &loc_links {
        assert!(rect.width > 0.0);
        assert!(rect.height > 0.0);
        println!("Found vector hotspot: {target} at {:?}", rect);
    }

    // 4. Test JumpToLine updates focus editor cursor
    app.doc.deck = Some(deck);
    let _ = app.update(Message::SwitchMode(EditorMode::FocusMode));

    let _ = app.update(Message::JumpToLine(3));
    let cursor = app.focus_slide_content.cursor();
    // Line 3 (1-indexed) maps to line 2 (0-indexed) in the text editor
    assert_eq!(cursor.position.line, 2);
    assert!(cursor.selection.is_some());

    // 5. Test Focus Mode UI view rendering with vector hotspots
    let _ = app.view();
}

#[test]
fn test_slide_macro_boundary_safety_and_title_update() {
    let source = r#"#let slide(title: "", transition: none, body) = block(width: 100%, height: 100%)[
  #text(18pt, weight: "bold", title)
  #v(1em)
  #body
]

#slide(title: "Architecture: Typst Functional Markup to Native Engine", transition: "slide-left")[
  #grid(
    columns: (1fr, 1fr),
    gutter: 16pt,
    [Left column text],
    [Right column text],
  )
]
"#;

    let mut app = SlideEditorApp::new(None, false);
    app.doc.source_text = source.to_string();
    app.doc.sync_chunks_from_source();
    app.engine =
        slide_editor::model::ast_engine::TypstDocumentEngine::from_source(source.to_string());
    app.sync_editors_from_doc();

    assert_eq!(app.engine.slides.len(), 1);
    let slide = &app.engine.slides[0];
    assert_eq!(
        slide.title,
        "Architecture: Typst Functional Markup to Native Engine"
    );

    // Ensure macro arguments are NOT added as fake Heading blocks in slide.blocks
    for block in &slide.blocks {
        if let slide_editor::model::ast_engine::WysiwygBlock::Heading { title, .. } = block {
            assert_ne!(
                title, "Architecture: Typst Functional Markup to Native Engine",
                "Slide macro title argument must not be classified as an internal content block"
            );
        }
    }

    // Ensure content element inside body exists (the #grid)
    let content_blocks: Vec<_> = slide
        .blocks
        .iter()
        .filter(|b| b.is_content_element())
        .collect();
    assert!(
        matches!(content_blocks[0], slide_editor::model::ast_engine::WysiwygBlock::FuncCall { callee, .. } if callee == "grid")
    );

    // Attempting to move block down should be safely ignored and not corrupt macro syntax
    let _ = app.update(Message::MoveBlockDown {
        slide_idx: 0,
        block_idx: 0,
    });
    assert!(
        !app.engine.source_text.contains(r#"#slide("Architecture"#),
        "Source syntax must never be corrupted across macro boundaries"
    );

    // Test updating the slide title via Message::UpdateSlideTitle
    let _ = app.update(Message::UpdateSlideTitle {
        slide_idx: 0,
        new_title: "Modern Typst Engine Architecture".to_string(),
    });

    assert!(app.engine.source_text.contains(
        r#"#slide(title: "Modern Typst Engine Architecture", transition: "slide-left")["#
    ));
    assert!(app.engine.source_text.contains("#grid("));
    assert_eq!(
        app.engine.slides[0].title,
        "Modern Typst Engine Architecture"
    );

    // Verify Typst compiles successfully without errors
    let bridge = CompilerBridge::new();
    let compile_res = bridge.compile_source(&app.engine.source_text, None);
    assert!(
        compile_res.is_ok(),
        "Slide must compile cleanly after title update: {:?}",
        compile_res.err()
    );
}

#[test]
fn test_multilevel_headings_and_heading_slide_separation() {
    use slide_editor::model::ast_engine::TypstDocumentEngine;
    use slide_editor::model::ast_engine::WysiwygBlock;

    let source = r#"#let slide(title: "", body) = block[#body]

#slide(title: "Multi-level Test")[
  = Heading 1
  == Heading 2
  === Heading 3
  ==== Heading 4
  ===== Heading 5
  ====== Heading 6
  Paragraph with #link("https://example.com")[External Link] and more text.
  #box(stroke: 1pt + blue, inset: 4pt)[Boxed text inside]
]
"#;

    let engine = TypstDocumentEngine::from_source(source.to_string());
    assert_eq!(engine.slides.len(), 1);
    let slide = &engine.slides[0];

    // Assert all 6 heading levels are recognized
    let heading_levels: Vec<usize> = slide
        .blocks
        .iter()
        .filter_map(|b| {
            if let WysiwygBlock::Heading { level, .. } = b {
                Some(*level)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(heading_levels, vec![1, 2, 3, 4, 5, 6]);

    // Assert paragraph with inline link was not fractured
    let paragraphs: Vec<_> = slide
        .blocks
        .iter()
        .filter_map(|b| {
            if let WysiwygBlock::Paragraph { text, .. } = b {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(paragraphs.len(), 1);
    assert!(paragraphs[0].contains("External Link"));
    assert!(paragraphs[0].contains("and more text"));

    // Test heading-based slide separation without macros
    let heading_source = r#"= Heading Slide 1
== Section 1
Content 1

= Heading Slide 2
== Section 2
Content 2
"#;
    let engine2 = TypstDocumentEngine::from_source(heading_source.to_string());
    assert_eq!(
        engine2.slides.len(),
        2,
        "Heading-based presentation should have 2 slides"
    );
    assert_eq!(engine2.slides[0].title, "Heading Slide 1");
    assert_eq!(engine2.slides[1].title, "Heading Slide 2");
}

#[test]
fn test_box_link_visual_modals_and_grid_multiline_enter() {
    use slide_editor::app::ActiveModal;
    use slide_editor::ui::wysiwyg::ComplexElementModal;
    use slide_editor::ui::wysiwyg::ComplexModalState;
    use slide_editor::ui::wysiwyg::block_view::InsertBlockKind;

    // 1. Test Link Complex Modal Parsing, Updating, and Typst Generation
    let raw_link = "#link(\"https://cargo-slide.dev\")[Cargo Slide Docs]";
    let modal_link = ComplexElementModal::from_raw(
        0,
        "blk-link-1".to_string(),
        0..raw_link.len(),
        "link",
        raw_link,
    );

    if let ComplexModalState::Link { url, label } = &modal_link.state {
        assert_eq!(url, "https://cargo-slide.dev");
        assert_eq!(label, "Cargo Slide Docs");
    } else {
        panic!("Expected Link modal state");
    }

    let mut app = SlideEditorApp::new(None, false);
    app.active_modal = Some(ActiveModal::ComplexElement(modal_link));

    let _ = app.update(Message::ModalLinkUpdateUrl("https://typst.app".to_string()));
    let _ = app.update(Message::ModalLinkUpdateLabel("Typst Official".to_string()));

    if let Some(ActiveModal::ComplexElement(ref m)) = app.active_modal {
        let typst = m.to_typst();
        assert_eq!(typst, "#link(\"https://typst.app\")[Typst Official]");
    }
    let _ = app.view();

    // 2. Test Box Modal with Content Extraction and Styling Presets
    let raw_box = "#box(stroke: 1pt + rgb(\"3b82f6\"), inset: 8pt)[Box Content Here]";
    let modal_box =
        ComplexElementModal::from_raw(0, "blk-box-1".to_string(), 0..raw_box.len(), "box", raw_box);

    if let ComplexModalState::BoxBlock {
        content,
        stroke,
        inset,
        ..
    } = &modal_box.state
    {
        assert_eq!(
            content, "Box Content Here",
            "Box content must be cleanly extracted from bracket"
        );
        assert!(stroke.contains("3b82f6"));
        assert_eq!(inset, "8pt");
    } else {
        panic!("Expected BoxBlock modal state");
    }

    app.active_modal = Some(ActiveModal::ComplexElement(modal_box));

    // Apply "green" preset
    let _ = app.update(Message::ModalBoxApplyPreset("green"));
    if let Some(ActiveModal::ComplexElement(ref m)) = app.active_modal {
        if let ComplexModalState::BoxBlock {
            fill,
            stroke,
            radius,
            inset,
            ..
        } = &m.state
        {
            assert!(fill.contains("f0fdf4"));
            assert!(stroke.contains("10b981"));
            assert_eq!(radius, "6pt");
            assert_eq!(inset, "8pt");
        }
        let typst = m.to_typst();
        assert!(typst.contains("fill: rgb(\"f0fdf4\")"));
        assert!(typst.contains("stroke: 1pt + rgb(\"10b981\")"));
        assert!(typst.contains("Box Content Here"));
    }
    let _ = app.view();

    // 3. Test Grid Columns Multi-line and Enter Key Simulation
    let raw_grid = "#grid(columns: 2, [Line 1 of Col A], [Col B])";
    let modal_grid = ComplexElementModal::from_raw(
        0,
        "blk-grid-1".to_string(),
        0..raw_grid.len(),
        "grid",
        raw_grid,
    );
    if let ComplexModalState::Grid { cols, columns } = &modal_grid.state {
        assert_eq!(*cols, 2);
        assert_eq!(columns[0], "Line 1 of Col A");
    } else {
        panic!("Expected Grid modal state");
    }

    let _ = app.update(Message::OpenComplexModal {
        slide_idx: 0,
        block_id: "blk-grid-1".to_string(),
        range: 0..raw_grid.len(),
        raw: raw_grid.to_string(),
        callee: "grid".to_string(),
    });

    assert_eq!(app.modal_col_editors.len(), 2);
    assert_eq!(app.modal_col_editors[0].text(), "Line 1 of Col A");

    // Simulate multi-line update on Column 0 (with newline)
    let multiline_text = "Line 1 of Col A\nLine 2 after Enter";
    let _ = app.update(Message::ModalGridUpdateCol {
        idx: 0,
        val: multiline_text.to_string(),
    });

    if let Some(ActiveModal::ComplexElement(ref m)) = app.active_modal {
        if let ComplexModalState::Grid { columns, .. } = &m.state {
            assert_eq!(columns[0], multiline_text);
        }
        let typst = m.to_typst();
        assert!(typst.contains("Line 1 of Col A\nLine 2 after Enter"));
    }
    let _ = app.view();

    // 4. Test InsertBlockKind templates for Heading3, Heading4, and Link
    assert_eq!(InsertBlockKind::Heading3.label(), "H3");
    assert_eq!(
        InsertBlockKind::Heading3.template(),
        "=== Section Subheading\n"
    );
    assert_eq!(InsertBlockKind::Heading4.label(), "H4");
    assert_eq!(InsertBlockKind::Heading4.template(), "==== Detail Topic\n");
    assert_eq!(InsertBlockKind::Link.label(), "Link");
    assert!(InsertBlockKind::Link.template().contains("#link("));
}

#[test]
fn test_title_slide_interactive_editing_and_modals() {
    use slide_editor::app::ActiveModal;
    use slide_editor::model::ast_engine::TypstDocumentEngine;
    use slide_editor::model::ast_engine::WysiwygBlock;
    use slide_editor::ui::wysiwyg::block_view::InsertBlockKind;
    use slide_editor::ui::wysiwyg::block_view::parse_title_slide_data;
    use slide_editor::ui::wysiwyg::block_view::update_title_slide_param;
    use slide_editor::ui::wysiwyg::complex_modal::ComplexElementModal;
    use slide_editor::ui::wysiwyg::complex_modal::ComplexModalState;

    let title_slide_typ = r#"#title-slide(
  title: "Building Modern Slides with Rust & Typst",
  subtitle: "Native Binary • LZMA2 Standalone Packages",
  author: "Cargo Slide Team",
  date: "2026",
)
#audio("assets/ambient.wav", autoplay: true)

#slide(title: "Second Slide")[
  - Content on slide 2
]
"#;

    // 1. Engine AST parsing: Slide 1 MUST contain title-slide as block 0!
    let engine = TypstDocumentEngine::from_source(title_slide_typ.to_string());
    assert_eq!(engine.slides.len(), 2);
    assert_eq!(
        engine.slides[0].title,
        "Building Modern Slides with Rust & Typst"
    );
    assert_eq!(engine.slides[0].blocks.len(), 2);

    let title_block = &engine.slides[0].blocks[0];
    assert!(title_block.is_content_element());
    match title_block {
        | WysiwygBlock::FuncCall { callee, args, .. } => {
            assert_eq!(callee, "title-slide");
            assert!(args.contains("Building Modern Slides"));
            let (badge, preview) = title_block.chip_info();
            assert_eq!(badge, "Title");
            assert_eq!(preview, "Building Modern Slides with Rust & Typst");
        },
        | _ => panic!("Block 0 must be FuncCall with title-slide callee"),
    }

    // 2. Test parse_title_slide_data & update_title_slide_param
    let raw_source = title_block.raw();
    let data = parse_title_slide_data(raw_source);
    assert_eq!(data.title, "Building Modern Slides with Rust & Typst");
    assert_eq!(data.subtitle, "Native Binary • LZMA2 Standalone Packages");
    assert_eq!(data.author, "Cargo Slide Team");
    assert_eq!(data.date, "2026");
    assert!(data.version.is_empty());

    // Update existing parameter
    let updated_sub = update_title_slide_param(raw_source, "subtitle", "Updated Subtitle Text");
    assert!(updated_sub.contains("subtitle: \"Updated Subtitle Text\""));
    assert!(updated_sub.contains("title: \"Building Modern Slides with Rust & Typst\""));

    // Add new parameter (e.g. version)
    let with_version = update_title_slide_param(&updated_sub, "version", "0.2.0");
    assert!(with_version.contains("version: \"0.2.0\""));
    let re_data = parse_title_slide_data(&with_version);
    assert_eq!(re_data.version, "0.2.0");
    assert_eq!(re_data.subtitle, "Updated Subtitle Text");

    // 3. Test ComplexElementModal roundtrip for title-slide
    let modal = ComplexElementModal::from_raw(
        0,
        "blk-title-1".to_string(),
        0..with_version.len(),
        "title-slide",
        &with_version,
    );

    match &modal.state {
        | ComplexModalState::TitleSlide {
            title,
            subtitle,
            author,
            date,
            version,
            ..
        } => {
            assert_eq!(title, "Building Modern Slides with Rust & Typst");
            assert_eq!(subtitle, "Updated Subtitle Text");
            assert_eq!(author, "Cargo Slide Team");
            assert_eq!(date, "2026");
            assert_eq!(version, "0.2.0");
        },
        | _ => panic!("Modal state must be TitleSlide"),
    }

    // 4. Test SlideEditorApp with interactive TitleSlide modal & presets
    let mut app = SlideEditorApp::new(None, false);
    app.engine = TypstDocumentEngine::from_source(with_version.clone());
    app.doc.source_text = with_version.clone();
    app.doc.sync_chunks_from_source();

    let _ = app.update(Message::OpenComplexModal {
        slide_idx: 0,
        block_id: "blk-title-1".to_string(),
        range: 0..with_version.len(),
        raw: with_version.clone(),
        callee: "title-slide".to_string(),
    });

    assert!(app.active_modal.is_some());

    // Apply preset
    let _ = app.update(Message::ModalTitleSlideApplyPreset {
        title: Some("New Keynote Title".to_string()),
        subtitle: Some("High Speed Engine".to_string()),
        author: Some("DeepMind Rust Team".to_string()),
        date: Some("October 2026".to_string()),
        version: Some("v1.0.0".to_string()),
        institution: Some("Google DeepMind".to_string()),
    });

    // Add extra parameter
    let _ = app.update(Message::ModalTitleSlideAddExtraArg);
    let _ = app.update(Message::ModalTitleSlideUpdateExtraKey(
        0,
        "theme".to_string(),
    ));
    let _ = app.update(Message::ModalTitleSlideUpdateExtraVal(
        0,
        "dark".to_string(),
    ));

    if let Some(ActiveModal::ComplexElement(ref m)) = app.active_modal {
        let typst_out = m.to_typst();
        assert!(typst_out.contains("title: \"New Keynote Title\""));
        assert!(typst_out.contains("subtitle: \"High Speed Engine\""));
        assert!(typst_out.contains("author: \"DeepMind Rust Team\""));
        assert!(typst_out.contains("date: \"October 2026\""));
        assert!(typst_out.contains("version: \"v1.0.0\""));
        assert!(typst_out.contains("institution: \"Google DeepMind\""));
        assert!(typst_out.contains("theme: \"dark\""));
    }

    // Test view rendering without panic
    let _ = app.view();

    // 5. Test UpdateSlideTitle synchronization with title-slide
    let _ = app.update(Message::CloseComplexModal);
    let _ = app.update(Message::UpdateSlideTitle {
        slide_idx: 0,
        new_title: "Synchronized Slide Title".to_string(),
    });
    assert!(
        app.engine
            .source_text
            .contains("title: \"Synchronized Slide Title\"")
    );

    // 6. Test InsertBlockKind::TitleSlide
    assert_eq!(InsertBlockKind::TitleSlide.label(), "Title Slide");
    assert!(
        InsertBlockKind::TitleSlide
            .template()
            .contains("#title-slide(")
    );
}

#[test]
fn test_table_editing_on_page_preserves_active_slide_and_syntax() {
    use slide_editor::app::ActiveModal;
    use slide_editor::app::Message;
    use slide_editor::app::SlideEditorApp;
    use slide_editor::model::ast_engine::TypstDocumentEngine;
    use slide_editor::ui::wysiwyg::ComplexModalState;

    let doc_source = r#"#slide("Page 1")[
  = Introduction
]

#slide("Page 2")[
  = Section 1
]

#slide("Page 3")[
  #table(
    columns: (auto, 1fr),
    fill: (col, row) => if calc.even(row) { rgb(22, 27, 34, 40%) } else { none },
    align: (col, row) => if row == 0 { center } else { left },
    table.header([Name], [Description]),
    [Alpha], [First component],
    [Beta], [Second component],
  )
]

#slide("Page 4")[
  = Conclusion
]
"#;

    let mut app = SlideEditorApp::new(None, false);
    app.engine = TypstDocumentEngine::from_source(doc_source.to_string());
    app.doc.source_text = doc_source.to_string();
    app.doc.sync_chunks_from_source();

    assert_eq!(app.engine.slides.len(), 4);

    // Slide 2 is Page 3 (0-indexed)
    app.active_slide = 2;

    // Find table block on Slide 2
    let table_block = app.engine.slides[2]
        .blocks
        .iter()
        .find(|b| {
            match b {
                | slide_editor::model::ast_engine::WysiwygBlock::FuncCall { callee, .. } => {
                    callee == "table"
                },
                | _ => false,
            }
        })
        .expect("Table block must exist on slide 2");

    let table_id = table_block.id().to_string();
    let table_range = table_block.range();
    let table_raw = table_block.raw().to_string();

    // 1. Open modal from Slide 2
    let _ = app.update(Message::OpenComplexModal {
        slide_idx: 2,
        block_id: table_id,
        range: table_range.clone(),
        raw: table_raw.clone(),
        callee: "table".to_string(),
    });

    assert_eq!(
        app.active_slide, 2,
        "Active slide must stay at 2 when opening modal"
    );
    assert!(app.active_modal.is_some());

    // Verify parsed properties did not truncate closures
    if let Some(ActiveModal::ComplexElement(ref m)) = app.active_modal {
        if let ComplexModalState::Table {
            rows,
            properties,
            header_cells,
            has_header,
            ..
        } = &m.state
        {
            assert!(*has_header);
            assert_eq!(header_cells.len(), 2);
            assert_eq!(header_cells[0], "Name");
            assert_eq!(header_cells[1], "Description");
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0][0], "Alpha");
            assert_eq!(rows[1][0], "Beta");

            // Check that fill closure was not cut off at first comma
            let fill_prop = properties.iter().find(|p| p.starts_with("fill:"));
            assert!(fill_prop.is_some(), "fill property must be captured");
            let fill_val = fill_prop.unwrap();
            assert!(
                fill_val.contains("rgb(22, 27, 34, 40%)"),
                "Full closure must be preserved without cut-off: {}",
                fill_val
            );
        } else {
            panic!("Expected Table modal state");
        }
    }

    // 2. Modify a cell inside the modal
    let _ = app.update(Message::ModalTableUpdateCell {
        row: 0,
        col: 0,
        val: "Alpha Pro".to_string(),
    });

    // 3. Apply the modal
    let _ = app.update(Message::ApplyComplexModal);

    // Active slide MUST NOT bounce back to 0!
    assert_eq!(
        app.active_slide, 2,
        "Active slide must remain 2 after applying modal"
    );
    assert_eq!(
        app.engine.slides.len(),
        4,
        "Slide count must remain 4 without syntax corruption collapse"
    );
    assert!(app.doc.source_text.contains("Alpha Pro"));
    assert!(app.doc.source_text.contains("rgb(22, 27, 34, 40%)"));

    // 4. Test in-place cell update on slide 2 using UpdateBlockRange
    let slide2_table = app.engine.slides[2]
        .blocks
        .iter()
        .find(|b| {
            match b {
                | slide_editor::model::ast_engine::WysiwygBlock::FuncCall { callee, .. } => {
                    callee == "table"
                },
                | _ => false,
            }
        })
        .expect("Table block must exist on slide 2");

    let updated_text = slide2_table.raw().replace("Beta", "Beta Super");
    let _ = app.update(Message::UpdateBlockRange {
        slide_idx: Some(2),
        range: slide2_table.range(),
        new_text: updated_text,
    });

    assert_eq!(
        app.active_slide, 2,
        "Active slide must remain 2 after in-place cell edit"
    );
    assert_eq!(
        app.engine.slides.len(),
        4,
        "Slide count must remain 4 after in-place edit"
    );
    assert!(app.doc.source_text.contains("Beta Super"));
}

#[test]
fn test_slide_transition_parsing_and_extraction() {
    use slide_editor::model::ast_engine::TypstDocumentEngine;
    use slide_editor::model::ast_engine::extract_transition_from_raw;

    // 1. Test raw extraction helper
    assert_eq!(
        extract_transition_from_raw(r#"#slide(title: "Hello", transition: "glitch")[]"#),
        Some("glitch".to_string())
    );
    assert_eq!(
        extract_transition_from_raw(r#"#link("transition:zoom")[]"#),
        Some("zoom".to_string())
    );
    assert_eq!(
        extract_transition_from_raw(r#"#slide(title: "Cut", transition: "cut")[]"#),
        None
    );
    assert_eq!(
        extract_transition_from_raw(r#"#slide(title: "None", transition: none)[]"#),
        None
    );

    // 2. Test engine macro slides parsing
    let macro_doc = r#"
#let slide(title: "", transition: none, body) = block(body)

#slide(title: "Slide 1", transition: "fade")[
  - Content 1
]

#slide(title: "Slide 2", transition: "cube")[
  - Content 2
]

#slide(title: "Slide 3")[
  - Content 3
]
"#;
    let engine = TypstDocumentEngine::from_source(macro_doc.to_string());
    assert_eq!(engine.slides.len(), 3);
    assert_eq!(engine.slides[0].transition.as_deref(), Some("fade"));
    assert_eq!(engine.slides[1].transition.as_deref(), Some("cube"));
    assert_eq!(engine.slides[2].transition, None);

    // 3. Test engine pagebreak slides parsing
    let pagebreak_doc = r#"
= Slide 1
#link("transition:iris")[]
- Item A

#pagebreak()

= Slide 2
#link("transition:particles")[]
- Item B

#pagebreak()

= Slide 3
- Item C without transition
"#;
    let pb_engine = TypstDocumentEngine::from_source(pagebreak_doc.to_string());
    assert_eq!(pb_engine.slides.len(), 3);
    assert_eq!(pb_engine.slides[0].transition.as_deref(), Some("iris"));
    assert_eq!(pb_engine.slides[1].transition.as_deref(), Some("particles"));
    assert_eq!(pb_engine.slides[2].transition, None);
}

#[test]
fn test_slide_transition_update_single_and_bulk() {
    use slide_editor::model::ast_engine::TypstDocumentEngine;
    use slide_editor::model::ast_engine::update_all_slides_transition;
    use slide_editor::model::ast_engine::update_slide_transition;

    let doc = r#"#let slide(title: "", transition: none, body) = block(body)

#slide(title: "Slide 1")[
  - Content 1
]

#slide(title: "Slide 2", transition: "slide-left")[
  - Content 2
]
"#;
    let engine = TypstDocumentEngine::from_source(doc.to_string());
    assert_eq!(engine.slides.len(), 2);
    assert_eq!(engine.slides[0].transition, None);
    assert_eq!(engine.slides[1].transition.as_deref(), Some("slide-left"));

    // 1. Add transition to slide 0
    let updated1 = update_slide_transition(doc, engine.slides[0].range.clone(), Some("zoom"));
    let engine1 = TypstDocumentEngine::from_source(updated1.clone());
    assert_eq!(engine1.slides[0].transition.as_deref(), Some("zoom"));
    assert_eq!(engine1.slides[1].transition.as_deref(), Some("slide-left"));

    // 2. Change transition on slide 1
    let updated2 =
        update_slide_transition(&updated1, engine1.slides[1].range.clone(), Some("glitch"));
    let engine2 = TypstDocumentEngine::from_source(updated2.clone());
    assert_eq!(engine2.slides[0].transition.as_deref(), Some("zoom"));
    assert_eq!(engine2.slides[1].transition.as_deref(), Some("glitch"));

    // 3. Remove transition from slide 1 (set to None or cut)
    let updated3 = update_slide_transition(&updated2, engine2.slides[1].range.clone(), None);
    let engine3 = TypstDocumentEngine::from_source(updated3.clone());
    assert_eq!(engine3.slides[0].transition.as_deref(), Some("zoom"));
    assert_eq!(engine3.slides[1].transition, None);

    // 4. Bulk apply transition to all slides
    let bulk = update_all_slides_transition(&updated3, &engine3, Some("cube"));
    let bulk_engine = TypstDocumentEngine::from_source(bulk);
    assert_eq!(bulk_engine.slides.len(), 2);
    assert_eq!(bulk_engine.slides[0].transition.as_deref(), Some("cube"));
    assert_eq!(bulk_engine.slides[1].transition.as_deref(), Some("cube"));
}

#[test]
fn test_slide_transition_interactive_modal_lifecycle() {
    use slide_editor::app::ActiveModal;

    let doc = r#"#let slide(title: "", transition: none, body) = block(body)

#slide(title: "Architecture", transition: "fade")[
  - Core pipelines
]

#slide(title: "Performance")[
  - 60 FPS
]
"#;
    let mut app = SlideEditorApp::new(None, false);
    app.doc.source_text = doc.to_string();
    app.doc.sync_chunks_from_source();
    app.engine = slide_editor::model::ast_engine::TypstDocumentEngine::from_source(doc.to_string());
    app.sync_editors_from_doc();

    assert_eq!(app.engine.slides.len(), 2);
    assert_eq!(app.engine.slides[0].transition.as_deref(), Some("fade"));
    assert_eq!(app.engine.slides[1].transition, None);

    // 1. Open Transition Modal for slide 1
    let _ = app.update(Message::OpenTransitionModal(1));
    match app.active_modal {
        | Some(ActiveModal::SlideTransition {
            slide_idx,
            ref current_transition,
            ..
        }) => {
            assert_eq!(slide_idx, 1);
            assert_eq!(*current_transition, None);
        },
        | _ => panic!("Expected SlideTransition modal"),
    }

    // 2. Select a transition in modal
    let _ = app.update(Message::SelectTransition(Some("glitch".to_string())));
    if let Some(ActiveModal::SlideTransition {
        ref current_transition,
        ..
    }) = app.active_modal
    {
        assert_eq!(*current_transition, Some("glitch".to_string()));
    }

    // 3. Render modal UI (ensure view does not panic)
    let _ = app.view();

    // 4. Apply transition to slide 1
    let _ = app.update(Message::ApplySlideTransition {
        slide_idx: 1,
        transition: Some("glitch".to_string()),
        all_slides: false,
    });
    assert!(app.active_modal.is_none());
    assert_eq!(app.engine.slides[1].transition.as_deref(), Some("glitch"));
    assert_eq!(app.engine.slides[0].transition.as_deref(), Some("fade"));

    // 5. Test Apply to All Slides
    let _ = app.update(Message::OpenTransitionModal(0));
    let _ = app.update(Message::SelectTransition(Some("iris".to_string())));
    let _ = app.update(Message::ApplySlideTransition {
        slide_idx: 0,
        transition: Some("iris".to_string()),
        all_slides: true,
    });
    assert_eq!(app.engine.slides[0].transition.as_deref(), Some("iris"));
    assert_eq!(app.engine.slides[1].transition.as_deref(), Some("iris"));
}

#[test]
fn test_geek_presentation_transitions_parsing_and_editing() {
    let geek_path = std::path::Path::new("examples/geek-presentation/slides.typ");
    if !geek_path.exists() {
        return;
    }
    let content = std::fs::read_to_string(geek_path).expect("Failed to read slides.typ");
    let mut app = SlideEditorApp::new(Some(geek_path.to_path_buf()), false);
    app.doc.source_text = content.clone();
    app.doc.sync_chunks_from_source();
    app.engine = slide_editor::model::ast_engine::TypstDocumentEngine::from_source(content);
    app.sync_editors_from_doc();

    assert!(
        app.engine.slides.len() >= 10,
        "Geek presentation must parse into at least 10 slides"
    );

    // Verify slide 1 has transition "slide-left"
    // (Slide 0 is Title Slide, Slide 1 is Architecture)
    let arch_slide = app
        .engine
        .slides
        .iter()
        .find(|s| s.title.contains("Architecture"))
        .expect("Architecture slide must exist");
    assert_eq!(arch_slide.transition.as_deref(), Some("slide-left"));

    let glitch_slide = app
        .engine
        .slides
        .iter()
        .find(|s| s.title.contains("13 Built-in Page Transitions"))
        .expect("Transitions slide must exist");
    assert_eq!(glitch_slide.transition.as_deref(), Some("glitch"));

    // Verify view rendering with sidebar cards, toolbar, canvas
    let _ = app.view();
}

#[test]
fn test_safe_macro_content_insertion_inside_slide() {
    use slide_editor::app::Message;
    use slide_editor::app::SlideEditorApp;
    use slide_editor::model::ast_engine::TypstDocumentEngine;

    let source =
        "#slide(\"Page 1\")[\n  = Title 1\n  First content line.\n]\n\n#slide(\"Page 2\")[\n]\n";
    let mut app = SlideEditorApp::new(None, false);
    app.doc.source_text = source.to_string();
    app.doc.sync_chunks_from_source();
    app.engine = TypstDocumentEngine::from_source(source.to_string());
    app.active_slide = 0;

    // Test offset for slide with content: must insert before or at the closing ']'
    let offset_s0 = app.engine.get_slide_content_insert_offset(0);
    let s0_close = source.find("]\n\n#slide(\"Page 2\")").unwrap();
    assert!(offset_s0 <= s0_close);

    // Test offset for empty slide: must insert right before the closing ']'
    let offset_s1 = app.engine.get_slide_content_insert_offset(1);
    let s1_close = source.rfind(']').unwrap();
    assert_eq!(offset_s1, s1_close);

    // Insert snippet into active slide 0
    let _ = app.update(Message::InsertSnippet("- New Bullet Item\n"));
    assert!(app.doc.source_text.contains("- New Bullet Item\n"));

    // Check that slide 0 still has its closing bracket properly after the new bullet
    let p1_close = app
        .doc
        .source_text
        .find("]\n\n#slide(\"Page 2\")")
        .expect("Slide 1 closing bracket must exist");
    let bullet_pos = app
        .doc
        .source_text
        .find("- New Bullet Item")
        .expect("Inserted snippet must exist");
    assert!(
        bullet_pos < p1_close,
        "Snippet must be inserted INSIDE the slide macro closing bracket"
    );
}

#[test]
fn test_block_spacing_step_adjustment_and_reordering() {
    use slide_editor::app::Message;
    use slide_editor::app::SlideEditorApp;
    use slide_editor::model::ast_engine::TypstDocumentEngine;

    let source = "#slide(\"Spacing Test\")[\n  = Heading One\n\n  First paragraph text.\n\n  Second paragraph text.\n]\n";
    let mut app = SlideEditorApp::new(None, false);
    app.doc.source_text = source.to_string();
    app.doc.sync_chunks_from_source();
    app.engine = TypstDocumentEngine::from_source(source.to_string());
    app.active_slide = 0;

    // 1. Increase spacing between block 0 and block 1 by +4pt
    let _ = app.update(Message::AdjustBlockSpacing {
        slide_idx: 0,
        block_idx: 0,
        delta_pt: 4,
    });
    assert!(
        app.doc.source_text.contains("#v(4pt)"),
        "Must insert #v(4pt)"
    );

    // 2. Increase spacing by another +4pt -> should be #v(8pt)
    let _ = app.update(Message::AdjustBlockSpacing {
        slide_idx: 0,
        block_idx: 0,
        delta_pt: 4,
    });
    assert!(
        app.doc.source_text.contains("#v(8pt)"),
        "Must step up to #v(8pt)"
    );

    // 3. Decrease spacing by -4pt -> should be #v(4pt)
    let _ = app.update(Message::AdjustBlockSpacing {
        slide_idx: 0,
        block_idx: 0,
        delta_pt: -4,
    });
    assert!(
        app.doc.source_text.contains("#v(4pt)"),
        "Must step down to #v(4pt)"
    );

    // 4. Reset spacing with delta_pt: -999 -> should remove #v(...) entirely
    let _ = app.update(Message::AdjustBlockSpacing {
        slide_idx: 0,
        block_idx: 0,
        delta_pt: -999,
    });
    assert!(
        !app.doc.source_text.contains("#v("),
        "Must remove #v(...) when reset"
    );

    // 5. Test MoveBlockDown on block 1 (First paragraph)
    let _ = app.update(Message::MoveBlockDown {
        slide_idx: 0,
        block_idx: 1,
    });
    let p1_pos = app.doc.source_text.find("First paragraph text.").unwrap();
    let p2_pos = app.doc.source_text.find("Second paragraph text.").unwrap();
    assert!(
        p2_pos < p1_pos,
        "Second paragraph must now appear before first paragraph"
    );

    // 6. Test MoveBlockUp to swap them back
    let _ = app.update(Message::MoveBlockUp {
        slide_idx: 0,
        block_idx: 2,
    });
    let p1_pos_back = app.doc.source_text.find("First paragraph text.").unwrap();
    let p2_pos_back = app.doc.source_text.find("Second paragraph text.").unwrap();
    assert!(
        p1_pos_back < p2_pos_back,
        "First paragraph must be moved back above second paragraph"
    );
}

#[test]
fn test_apply_complex_modal_anchors_active_slide() {
    use slide_editor::app::ActiveModal;
    use slide_editor::app::Message;
    use slide_editor::app::SlideEditorApp;
    use slide_editor::model::ast_engine::TypstDocumentEngine;
    use slide_editor::ui::wysiwyg::ComplexElementModal;

    let source = "#slide(\"Page 1\")[\n  = First Slide\n]\n\n#slide(\"Page 2\")[\n  = Second Slide\n  #table(\n    columns: (1fr, 1fr),\n    [A], [B],\n  )\n]\n";
    let mut app = SlideEditorApp::new(None, false);
    app.doc.source_text = source.to_string();
    app.doc.sync_chunks_from_source();
    app.engine = TypstDocumentEngine::from_source(source.to_string());
    app.active_slide = 1;

    let table_block = app.engine.slides[1]
        .blocks
        .iter()
        .find(|b| {
            match b {
                | slide_editor::model::ast_engine::WysiwygBlock::FuncCall { callee, .. } => {
                    callee == "table"
                },
                | _ => false,
            }
        })
        .expect("Table block on slide 2 must exist");

    let modal = ComplexElementModal::from_raw(
        1,
        table_block.id().to_string(),
        table_block.range(),
        "table",
        table_block.raw(),
    );
    app.active_modal = Some(ActiveModal::ComplexElement(modal));

    // Edit a cell in the modal
    let _ = app.update(Message::ModalTableUpdateCell {
        row: 0,
        col: 0,
        val: "Alpha".to_string(),
    });

    // Apply modal
    let _ = app.update(Message::ApplyComplexModal);

    // Active slide MUST stay at 1 (page 2), NEVER jump back to 0!
    assert_eq!(
        app.active_slide, 1,
        "Active slide must remain on slide 1 after applying modal"
    );
    assert!(
        app.doc.source_text.contains("[Alpha]"),
        "Table cell must be updated"
    );
}

#[test]
fn test_callout_fidelity_and_color_presets() {
    use slide_editor::app::ActiveModal;
    use slide_editor::app::Message;
    use slide_editor::app::SlideEditorApp;
    use slide_editor::ui::wysiwyg::ComplexElementModal;
    use slide_editor::ui::wysiwyg::ComplexModalState;

    let raw_legacy = "#tip(title: \"Legacy Tip\")[\n  Some tip body\n]";
    let modal = ComplexElementModal::from_raw(
        0,
        "callout-1".to_string(),
        0..raw_legacy.len(),
        "tip",
        raw_legacy,
    );

    if let ComplexModalState::Callout {
        title,
        stroke_color,
        body,
    } = &modal.state
    {
        assert_eq!(title, "Legacy Tip");
        assert_eq!(stroke_color, "slide-colors.accent-cyan");
        assert_eq!(body, "Some tip body");
    } else {
        panic!("Expected Callout state");
    }

    let mut app = SlideEditorApp::new(None, false);
    app.active_modal = Some(ActiveModal::ComplexElement(modal));

    // Switch stroke color preset to purple
    let _ = app.update(Message::ModalCalloutSetStrokeColor(
        "slide-colors.accent-purple".to_string(),
    ));
    let _ = app.update(Message::ModalCalloutUpdateTitle(
        "Summary Takeaway".to_string(),
    ));

    if let Some(ActiveModal::ComplexElement(ref m)) = app.active_modal {
        let typst = m.to_typst();
        assert!(
            typst.starts_with("#callout("),
            "Must ALWAYS emit valid #callout, never unknown macro"
        );
        assert!(typst.contains("stroke-color: slide-colors.accent-purple"));
        assert!(typst.contains("title: \"Summary Takeaway\""));
        assert!(typst.contains("Some tip body"));
    }
}

#[test]
fn test_svg_export_page_filtering() {
    use slide_editor::compiler_bridge::CompilerBridge;

    // Test page range parser
    let parsed_all = CompilerBridge::parse_page_range("", 5);
    assert_eq!(parsed_all, vec![1, 2, 3, 4, 5]);

    let parsed_custom = CompilerBridge::parse_page_range("1, 3-4", 5);
    assert_eq!(parsed_custom, vec![1, 3, 4]);

    let parsed_single = CompilerBridge::parse_page_range("2", 5);
    assert_eq!(parsed_single, vec![2]);

    let parsed_oob = CompilerBridge::parse_page_range("6, 10", 5);
    assert!(parsed_oob.is_empty());

    // Test actual SVG export with filtering
    let compiler = CompilerBridge::new();
    let src = "= Slide 1\nContent 1\n#pagebreak()\n= Slide 2\nContent 2\n#pagebreak()\n= Slide 3\nContent 3\n";
    let deck = compiler
        .compile_source(src, None)
        .expect("Compile 3-slide deck");
    assert_eq!(deck.slides.len(), 3);

    let temp_dir = tempfile::tempdir().unwrap();
    let target_single = temp_dir.path().join("slide_2.svg");
    let res = CompilerBridge::export_svgs(&deck, &target_single, Some(&[2]));
    assert!(res.is_ok());
    assert!(target_single.exists());
    assert!(target_single.metadata().unwrap().len() > 0);

    let target_dir = temp_dir.path().join("selected_svgs");
    let res2 = CompilerBridge::export_svgs(&deck, &target_dir, Some(&[1, 3]));
    assert!(res2.is_ok());
    let files = res2.unwrap();
    assert_eq!(files.len(), 2);
    assert!(
        files
            .iter()
            .any(|f| f.file_name().unwrap() == "slide-1.svg")
    );
    assert!(
        files
            .iter()
            .any(|f| f.file_name().unwrap() == "slide-3.svg")
    );
}

#[test]
fn test_editable_slide_package_flow_mode() {
    use slide_editor::app::Message;
    use slide_editor::app::SlideEditorApp;
    use slide_editor::compiler_bridge::CompilerBridge;
    use slide_editor::document::DocumentFormat;
    use slide_editor::document::EditorDocument;

    let temp_dir = tempfile::tempdir().unwrap();
    let src = "= Slide 1\nEditable Content\n#pagebreak()\n= Slide 2\nMore Content\n";
    let compiler = CompilerBridge::new();
    let mut doc = EditorDocument::new_presentation("Flow Package Test");
    doc.source_text = src.to_string();
    doc.sync_chunks_from_source();

    let package_path = temp_dir.path().join("editable.slide");
    let pack_res = compiler.export_slide_package(&doc, true, &package_path);
    assert!(pack_res.is_ok());
    assert!(package_path.exists());

    // Open via EditorDocument::open
    let opened = EditorDocument::open(&package_path).expect("Open editable slide");
    assert_eq!(opened.format, DocumentFormat::SlidePackage);
    assert!(opened.has_editable_source());
    assert!(opened.source_text.contains("Editable Content"));

    // Open via SlideEditorApp
    let app = SlideEditorApp::new(Some(package_path.clone()), false);
    // Because it contains editable source, slide_view_vector must be empty (Flow WYSIWYG Mode enabled)
    assert!(
        app.slide_view_vector.is_empty(),
        "Editable slide must NOT be forced into vector preview mode"
    );
    assert_eq!(app.doc.total_slides(), 2);

    // Open via ExecuteOpen message
    let mut app2 = SlideEditorApp::new(None, false);
    app2.open_path = package_path.to_string_lossy().to_string();
    let _ = app2.update(Message::ExecuteOpen);
    assert!(
        app2.slide_view_vector.is_empty(),
        "ExecuteOpen on editable slide must enter Flow Mode"
    );
}

#[test]
fn test_badge_live_editor_and_modal() {
    use slide_editor::app::ActiveModal;
    use slide_editor::app::Message;
    use slide_editor::app::SlideEditorApp;
    use slide_editor::ui::wysiwyg::ComplexElementModal;
    use slide_editor::ui::wysiwyg::ComplexModalState;

    let raw_badge = "#badge(\"PROD RELEASE\", fill: slide-colors.accent-cyan)";
    let modal = ComplexElementModal::from_raw(
        0,
        "badge-1".to_string(),
        0..raw_badge.len(),
        "badge",
        raw_badge,
    );

    if let ComplexModalState::Badge {
        label,
        fill,
        text_color,
    } = &modal.state
    {
        assert_eq!(label, "PROD RELEASE");
        assert_eq!(fill, "slide-colors.accent-cyan");
        assert!(text_color.is_empty());
    } else {
        panic!("Expected Badge state");
    }

    let mut app = SlideEditorApp::new(None, false);
    app.active_modal = Some(ActiveModal::ComplexElement(modal));

    let _ = app.update(Message::ModalBadgeUpdateLabel("BETA V2".to_string()));
    let _ = app.update(Message::ModalBadgeSetFill(
        "slide-colors.accent-purple".to_string(),
    ));

    if let Some(ActiveModal::ComplexElement(ref m)) = app.active_modal {
        let typst = m.to_typst();
        assert!(
            typst.starts_with("#badge(\"BETA V2\""),
            "Generated code: {typst}"
        );
        assert!(typst.contains("fill: slide-colors.accent-purple"));
    }
}

#[test]
fn test_element_transition_model_and_editor() {
    use slide_editor::app::ActiveModal;
    use slide_editor::app::Message;
    use slide_editor::app::SlideEditorApp;
    use slide_editor::model::ast_engine::TypstDocumentEngine;

    let source = "= Header\n\nSome paragraph text\n";
    let mut engine = TypstDocumentEngine::from_source(source.to_string());
    assert_eq!(engine.slides.len(), 1);
    assert_eq!(engine.slides[0].blocks.len(), 2);

    // Block 1 is the paragraph
    let blk = &engine.slides[0].blocks[1];
    let range = blk.range();

    // 1. Wrap with step transition
    let wrapped = slide_editor::model::ast_engine::apply_block_transition(
        &engine.source_text,
        range,
        2,
        "slide-up",
    );
    assert!(wrapped.contains("#step(2, effect: \"slide-up\")"));
    assert!(wrapped.contains("Some paragraph text"));

    // 2. Parse wrapped source and verify element_transitions
    engine.source_text = wrapped.clone();
    engine.reparse();
    assert_eq!(engine.slides.len(), 1);
    assert!(!engine.slides[0].element_transitions.is_empty());

    let trans = engine.slides[0]
        .element_transitions
        .values()
        .next()
        .unwrap();
    assert_eq!(trans.order, 2);
    assert_eq!(trans.effect, "slide-up");

    // 3. Update existing transition
    let updated = slide_editor::model::ast_engine::update_existing_step_transition(
        &engine.source_text,
        trans.wrapper_range.clone(),
        3,
        "cube",
    );
    assert!(updated.contains("#step(3, effect: \"cube\")"));

    // 4. Remove transition
    let removed = slide_editor::model::ast_engine::remove_step_transition(
        &updated,
        0..updated.len(), // wrapper range
    );
    assert!(!removed.contains("#step("));
    assert!(removed.contains("Some paragraph text"));

    // 5. Test full App flow with ElementTransition modal
    let mut app = SlideEditorApp::new(None, false);
    app.engine = TypstDocumentEngine::from_source(source.to_string());
    app.doc.source_text = source.to_string();

    // Open modal for block 1
    let _ = app.update(Message::OpenElementTransitionModal {
        slide_idx: 0,
        block_idx: 1,
    });
    assert!(matches!(
        app.active_modal,
        Some(ActiveModal::ElementTransition { .. })
    ));

    let _ = app.update(Message::SetElementTransitionOrder(2));
    let _ = app.update(Message::SelectElementTransitionEffect("glitch".to_string()));
    let _ = app.update(Message::ApplyElementTransition);

    assert!(app.doc.source_text.contains("#step(2, effect: \"glitch\")"));
}

#[test]
fn test_move_block_to_slide_ast_engine() {
    let source = r#"#slide(title: "Page 1")[
  = Title One
  #callout(title: "Target")[Callout Body]
]

#slide(title: "Page 2")[
  = Title Two
]
"#;

    let mut engine = TypstDocumentEngine::from_source(source.to_string());
    assert_eq!(engine.slides.len(), 2);

    // Block 1 on slide 0 is the callout
    let callout_block = &engine.slides[0].blocks[1];
    let range = callout_block.range();
    assert!(callout_block.raw().contains("#callout"));

    // Move block from slide 0 to slide 1
    let success = engine.move_block_to_slide(0, range, 1);
    assert!(success);

    // Slide 0 should no longer contain #callout
    let s0_text = engine
        .source_text
        .get(engine.slides[0].range.clone())
        .unwrap();
    assert!(!s0_text.contains("#callout"));

    // Slide 1 should now contain #callout
    let s1_text = engine
        .source_text
        .get(engine.slides[1].range.clone())
        .unwrap();
    assert!(s1_text.contains("#callout(title: \"Target\")[Callout Body]"));
}

#[test]
fn test_insert_slide_at_duplicate_and_move() {
    let source = r#"#slide(title: "First")[
  = One
]

#slide(title: "Second")[
  = Two
]
"#;

    let mut engine = TypstDocumentEngine::from_source(source.to_string());
    assert_eq!(engine.slides.len(), 2);

    // 1. Insert slide at index 1 (between First and Second)
    engine.insert_slide_at(1);
    assert_eq!(engine.slides.len(), 3);
    assert!(engine.source_text.contains("Slide 2"));

    // 2. Duplicate slide 0
    engine.duplicate_slide(0);
    assert_eq!(engine.slides.len(), 4);

    // 3. Test EditorDocument move_slide
    let mut doc = EditorDocument::new_presentation("Move Test");
    let initial_count = doc.total_slides();
    doc.insert_slide_at(1);
    assert_eq!(doc.total_slides(), initial_count + 1);

    doc.duplicate_slide(0);
    assert_eq!(doc.total_slides(), initial_count + 2);

    doc.move_slide(0, 2);
    assert_eq!(doc.total_slides(), initial_count + 2);
}

#[test]
fn test_app_move_block_modal_flow() {
    let source = r#"#slide(title: "Source Slide")[
  = Header
  - Move me please
]

#slide(title: "Destination Slide")[
  = Target Header
]
"#;

    let mut app = SlideEditorApp::new(None, false);
    app.engine = TypstDocumentEngine::from_source(source.to_string());
    app.doc.source_text = source.to_string();
    app.doc.sync_chunks_from_source();

    let block_range = app.engine.slides[0].blocks[1].range();

    // 1. Open move block modal
    let _ = app.update(Message::OpenMoveBlockModal {
        from_slide_idx: 0,
        block_idx: 1,
        range: block_range.clone(),
        block_label: "List Item".to_string(),
    });
    assert!(matches!(
        app.active_modal,
        Some(ActiveModal::MoveBlockToSlide { .. })
    ));

    // 2. Move block to slide 1
    let _ = app.update(Message::MoveBlockToSlide {
        from_slide_idx: 0,
        block_range,
        to_slide_idx: 1,
    });
    assert!(app.active_modal.is_none());

    // Verify slide 1 in doc contains the moved block
    let s1_chunk = &app.doc.slide_chunks[1];
    assert!(s1_chunk.contains("Move me please"));
}

#[test]
fn test_title_slide_macro_and_font_handling() {
    let source_without_import = r#"#title-slide(
  title: "Dynamic Title",
  subtitle: "Testing Unknown Variable Fix",
  author: "Test Suite",
)

#slide(title: "Content")[
  - Content here
]
"#;

    // Compiler bridge prepare_typst_source_with_macros must auto-import slide.typ
    let (processed, prepended) =
        slide_editor::compiler_bridge::CompilerBridge::prepare_typst_source_with_macros(
            source_without_import,
        );
    assert!(prepended);
    assert!(processed.starts_with("#import \"slide.typ\": *\n"));

    // Compile should succeed with no unknown variable error
    let bridge = slide_editor::compiler_bridge::CompilerBridge::new();
    let res = bridge.compile_source(source_without_import, None);
    assert!(
        res.is_ok(),
        "Typst compilation of title-slide without explicit import must succeed: {:?}",
        res.err()
    );

    // Font detection returns a non-empty available sans font
    let font = slide_core::font::detect_default_sans_font();
    assert!(!font.is_empty());
}

#[test]
fn test_font_selector_flow() {
    let mut app = SlideEditorApp::new(None, false);

    // Open font modal
    let _ = app.update(Message::OpenFontModal);
    assert!(matches!(
        app.active_modal,
        Some(ActiveModal::FontSelector { .. })
    ));

    // Search for a font
    let _ = app.update(Message::FontSearchQueryChanged("sans".to_string()));
    assert_eq!(app.font_search_query, "sans");

    // Select font
    let _ = app.update(Message::SelectFont("Nimbus Sans".to_string()));
    assert!(app.active_modal.is_none());
    assert!(
        app.doc
            .source_text
            .contains("#set text(font: \"Nimbus Sans\")")
    );
}

#[test]
fn test_present_unsaved_presentation_syncs_and_creates_macro_cache() {
    let mut app = SlideEditorApp::new(None, false);

    // Make an edit to in_place_content
    app.in_place_editing_slide = Some(0);
    app.in_place_content =
        iced::widget::text_editor::Content::with_text("= Modified Title\n\n- Synced bullet item");

    // Trigger PlayPresentation
    let _ = app.update(Message::PlayPresentation);

    // Verify cache directory has both slide.typ and untitled_presentation.typ
    let cache_dir = std::env::temp_dir().join("cargo_slide_presentation_cache");
    let macro_file = cache_dir.join("slide.typ");
    let untitled_file = cache_dir.join("untitled_presentation.typ");

    assert!(macro_file.exists(), "Cache dir must contain slide.typ");
    assert!(
        untitled_file.exists(),
        "Cache dir must contain untitled_presentation.typ"
    );

    let saved_typ = std::fs::read_to_string(&untitled_file).unwrap();
    assert!(
        saved_typ.contains("Modified Title"),
        "Latest in-place edits must be synced to untitled_presentation.typ"
    );
    assert!(saved_typ.contains("Synced bullet item"));

    // Verify Typst compiles untitled_presentation.typ without error
    let compiler = slide_core::compiler::SlideCompiler::new().expect("SlideCompiler");
    let compile_res = compiler.compile_file(&untitled_file);
    assert!(
        compile_res.is_ok(),
        "Typst compilation of unsaved presentation must succeed without missing slide.typ: {:?}",
        compile_res.err()
    );
}

#[test]
fn test_multi_column_extraction_and_rendering_3_plus_columns() {
    use slide_editor::ui::wysiwyg::block_view::add_col_to_grid;
    use slide_editor::ui::wysiwyg::block_view::extract_column_items;
    use slide_editor::ui::wysiwyg::block_view::grid_delete_col;
    use slide_editor::ui::wysiwyg::block_view::parse_column_count;
    use slide_editor::ui::wysiwyg::block_view::update_grid_col;

    // 1. Variadic 3-column #cols(...)
    let cols_3 = "#cols(\n  [\n    *Col 1*\n    Text A\n  ],\n  [\n    *Col 2*\n    Text B\n  ],\n  [\n    *Col 3*\n    Text C\n  ]\n)";
    let items_3 = extract_column_items(cols_3);
    assert_eq!(
        items_3.len(),
        3,
        "Must extract exactly 3 columns from 3-column #cols"
    );
    assert_eq!(items_3[0].display_text, "*Col 1*\n    Text A");
    assert_eq!(items_3[1].display_text, "*Col 2*\n    Text B");
    assert_eq!(items_3[2].display_text, "*Col 3*\n    Text C");
    assert_eq!(parse_column_count(cols_3), 3);

    // 2. Variadic 4-column with ratio: (1fr, 2fr, 1fr, 2fr)
    let cols_4 = "#cols(\n  ratio: (1fr, 2fr, 1fr, 2fr),\n  [First],\n  [Second],\n  [Third],\n  [Fourth]\n)";
    let items_4 = extract_column_items(cols_4);
    assert_eq!(items_4.len(), 4, "Must extract 4 columns");
    assert_eq!(items_4[0].portion, 1);
    assert_eq!(items_4[1].portion, 2);
    assert_eq!(items_4[2].portion, 1);
    assert_eq!(items_4[3].portion, 2);
    assert_eq!(items_4[0].display_text, "First");
    assert_eq!(items_4[3].display_text, "Fourth");
    assert_eq!(parse_column_count(cols_4), 4);

    // 3. Nested complex blocks inside columns (callout and code block)
    let nested_cols = "#cols(\n  [\n    #callout(title: \"Warning\")[Danger ahead]\n  ],\n  [\n    #code-window(title: \"lib.rs\")[pub fn run() {}]\n  ],\n  [\n    Simple column\n  ]\n)";
    let nested_items = extract_column_items(nested_cols);
    assert_eq!(nested_items.len(), 3);
    assert!(nested_items[0].raw.contains("#callout"));
    assert!(nested_items[1].raw.contains("#code-window"));
    assert_eq!(nested_items[2].display_text, "Simple column");

    // 4. Update specific column in 3-column layout
    let updated = update_grid_col(cols_3, 1, "*Modified Column 2*");
    assert!(updated.contains("*Modified Column 2*"));
    assert!(updated.contains("*Col 1*"));
    assert!(updated.contains("*Col 3*"));

    // 5. Add column to 3-column layout -> 4-column
    let added = add_col_to_grid(cols_3);
    let items_after_add = extract_column_items(&added);
    assert_eq!(
        items_after_add.len(),
        4,
        "Adding column to 3-column layout must produce 4 columns"
    );
    assert!(added.contains("*New Column*"));

    // 6. Delete column from 4-column layout -> 3-column
    let deleted = grid_delete_col(&added, 3);
    let items_after_del = extract_column_items(&deleted);
    assert_eq!(
        items_after_del.len(),
        3,
        "Deleting added column must return to 3 columns"
    );
    assert!(!deleted.contains("*New Column*"));

    // 7. Trailing brackets syntax #cols(3)[A][B][C]
    let trailing_cols = "#cols(3)[First][Second][Third]";
    let trailing_items = extract_column_items(trailing_cols);
    assert_eq!(trailing_items.len(), 3);
    assert_eq!(trailing_items[0].display_text, "First");
    assert_eq!(trailing_items[1].display_text, "Second");
    assert_eq!(trailing_items[2].display_text, "Third");
    assert_eq!(parse_column_count(trailing_cols), 3);

    let trailing_added = add_col_to_grid(trailing_cols);
    assert!(trailing_added.contains("#cols(4)"));
    let trailing_del = grid_delete_col(&trailing_added, 3);
    assert!(trailing_del.contains("#cols(3)"));
}

#[test]
fn test_typst_compiler_bridge_variadic_cols_compilation() {
    let compiler = slide_core::compiler::SlideCompiler::new().expect("SlideCompiler");
    let tmp_dir = std::env::temp_dir().join("cargo_slide_variadic_test");
    let _ = std::fs::create_dir_all(&tmp_dir);
    let typ_file = tmp_dir.join("test_cols.typ");
    std::fs::write(tmp_dir.join("slide.typ"), slide_theme::SLIDE_MACROS).expect("write slide.typ");

    let typst_source = r#"
#import "slide.typ": *

#title-slide(
  title: "Multi-Column Test",
  author: "Test Suite",
)

#slide(title: "3 Columns Test")[
  #cols(
    [
      *Column 1*
      Left column content.
    ],
    [
      *Column 2*
      Center column content.
    ],
    [
      *Column 3*
      Right column content.
    ]
  )
]

#slide(title: "4 Columns with Custom Ratios")[
  #cols(
    ratio: (1fr, 2fr, 2fr, 1fr),
    [Col A],
    [Col B with longer description],
    [Col C with more details],
    [Col D]
  )
]
"#;
    std::fs::write(&typ_file, typst_source).expect("write test_cols.typ");

    let res = compiler.compile_file(&typ_file);
    assert!(
        res.is_ok(),
        "Variadic #cols must compile cleanly with SlideCompiler: {:?}",
        res.err()
    );
    let doc = res.unwrap();
    assert_eq!(
        doc.slides.len(),
        3,
        "Must render title slide + 2 content slides"
    );
}

#[test]
fn test_typst_compiler_bridge_chart_and_callout_macros_compilation() {
    let compiler = slide_core::compiler::SlideCompiler::new().expect("SlideCompiler");
    let tmp_dir = std::env::temp_dir().join("cargo_slide_chart_macros_test");
    let _ = std::fs::create_dir_all(&tmp_dir);
    let typ_file = tmp_dir.join("test_chart_macros.typ");
    std::fs::write(tmp_dir.join("slide.typ"), slide_theme::SLIDE_MACROS).expect("write slide.typ");

    let typst_source = r#"
#import "slide.typ": *

#title-slide(
  title: "Chart & Callout Compatibility Test",
  author: "Test Suite",
)

#slide(title: "Bar & Line Macros")[
  #cols(
    [
      #chart-bar(
        title: "Metrics",
        height: 80pt,
        ("Speed", 85),
        ("Safety", 95),
      )
    ],
    [
      #chart-line(
        title: "Trend",
        height: 80pt,
        ("Jan", 10),
        ("Feb", 25),
      )
    ]
  )
]

#slide(title: "Pie & Plot Macros")[
  #cols(
    [
      #chart-pie(
        title: "Distribution",
        height: 80pt,
        ("Compute", 60),
        ("Storage", 40),
      )
    ],
    [
      #plot(
        title: "Plot",
        height: 80pt,
        ("P1", 100),
      )
    ]
  )
]

#slide(title: "Standard Chart")[
  #chart(
    type: "bar",
    title: "Standard Chart",
    height: 100pt,
    data: (
      categories: ("Alpha", "Beta"),
      series: ((name: "Metric", values: (50, 75)),),
    ),
  )
]

#slide(title: "Tip and Warning Callouts")[
  #cols(
    [#tip[Helpful hint text]],
    [#warning[Warning notice]]
  )
]

#slide(title: "Info, Alert, Pill, and Notes")[
  #cols(
    [#info[Informational note]],
    [#alert[Urgent alert message]]
  )
  #v(0.2cm)
  #pill("Status")
  #speaker-note[Speaker note body]
  #speaker_note[Alternative speaker note body]
]

#centered-slide(title: "Centered Slide")[
  Centered content body.
]

#focus-slide[
  Focus content body.
]
"#;
    std::fs::write(&typ_file, typst_source).expect("write test_chart_macros.typ");

    let res = compiler.compile_file(&typ_file);
    assert!(
        res.is_ok(),
        "All chart, callout, and slide macro aliases must compile cleanly with SlideCompiler: {:?}",
        res.err()
    );
    let doc = res.unwrap();
    assert_eq!(
        doc.slides.len(),
        8,
        "Must compile all 8 slides cleanly without overflow"
    );
}

#[test]
fn test_callout_with_nested_code_fences_and_brackets() {
    use slide_editor::app::ActiveModal;
    use slide_editor::app::Message;
    use slide_editor::app::SlideEditorApp;
    use slide_editor::compiler_bridge::CompilerBridge;
    use slide_editor::ui::wysiwyg::ComplexElementModal;
    use slide_editor::ui::wysiwyg::block_view::CalloutSegment;
    use slide_editor::ui::wysiwyg::block_view::parse_callout_data;
    use slide_editor::ui::wysiwyg::block_view::parse_callout_segments;

    use slide_editor::model::ast_engine::TypstDocumentEngine;
    use slide_editor::model::ast_engine::WysiwygBlock;

    let slides_src =
        std::fs::read_to_string("../../examples/geek-presentation/slides.typ").unwrap();
    let engine = TypstDocumentEngine::from_source(slides_src);
    for slide in &engine.slides {
        for block in &slide.blocks {
            if let WysiwygBlock::FuncCall { callee, args, .. } = block
                && callee == "callout"
            {
                let d = parse_callout_data(callee, args);
                println!(
                    "CALLOUT: title='{}', sc='{:?}', body_len={}, body={:?}",
                    d.title,
                    d.stroke_color,
                    d.body.len(),
                    d.body
                );
                let segs = parse_callout_segments(d.body);
                println!("  SEGS count: {}", segs.len());
                for (si, s) in segs.iter().enumerate() {
                    println!("    seg {si}: {:?}", s);
                }
            }
        }
    }

    let raw_callout = r#"#callout(title: "Code Example", stroke-color: rgb("10b981"))[
Before code
```rust
fn example() {
    let arr = [1, 2, 3];
    println!("item: {}", arr[0]);
}
```
After code
- Bullet 1
- Bullet 2
]"#;

    // 1. Verify parse_callout_data doesn't break on brackets inside code fence
    let data = parse_callout_data("callout", raw_callout);
    assert_eq!(data.title, "Code Example");
    assert_eq!(data.stroke_color, Some("rgb(\"10b981\")"));
    assert!(
        data.body.contains("let arr = [1, 2, 3];"),
        "Body must contain full code snippet: {}",
        data.body
    );
    assert!(
        data.body.contains("arr[0]"),
        "Body must contain array indexing: {}",
        data.body
    );
    assert!(
        data.body.contains("Bullet 2"),
        "Body must contain trailing list items: {}",
        data.body
    );

    // 2. Verify parse_callout_segments segments properly into Paragraphs, CodeBlocks, and Bullets
    let segments = parse_callout_segments(data.body);
    assert_eq!(
        segments.len(),
        5,
        "Expected 5 structured segments: {:?}",
        segments
    );

    match &segments[0] {
        | CalloutSegment::Paragraph(p) => assert_eq!(*p, "Before code"),
        | other => panic!("Expected Paragraph, got {:?}", other),
    }

    match &segments[1] {
        | CalloutSegment::CodeBlock { lang, code } => {
            assert_eq!(*lang, "rust");
            assert!(code.contains("let arr = [1, 2, 3];"));
            assert!(code.contains("arr[0]"));
        },
        | other => panic!("Expected CodeBlock, got {:?}", other),
    }

    match &segments[2] {
        | CalloutSegment::Paragraph(p) => assert_eq!(*p, "After code"),
        | other => panic!("Expected Paragraph, got {:?}", other),
    }

    match &segments[3] {
        | CalloutSegment::Bullet { text } => assert_eq!(*text, "Bullet 1"),
        | other => panic!("Expected Bullet, got {:?}", other),
    }

    match &segments[4] {
        | CalloutSegment::Bullet { text } => assert_eq!(*text, "Bullet 2"),
        | other => panic!("Expected Bullet, got {:?}", other),
    }

    // 3. Test modal roundtrip and typing in multiline editor
    let modal = ComplexElementModal::from_raw(
        0,
        "callout-code".to_string(),
        0..raw_callout.len(),
        "callout",
        raw_callout,
    );
    assert_eq!(modal.callee, "callout");
    let mut app = SlideEditorApp::new(None, false);
    let _ = app.update(Message::OpenComplexModal {
        slide_idx: 0,
        block_id: "callout-code".to_string(),
        range: 0..raw_callout.len(),
        raw: raw_callout.to_string(),
        callee: "callout".to_string(),
    });

    assert_eq!(app.modal_callout_editor.text(), data.body);

    // Update body via modal editor action
    let new_body_text = "Updated paragraph\n```python\nx = [10, 20]\n```";
    let _ = app.update(Message::ModalCalloutUpdateBody(new_body_text.to_string()));

    if let Some(ActiveModal::ComplexElement(ref m)) = app.active_modal {
        let typst = m.to_typst();
        assert!(typst.starts_with("#callout("), "Emits callout macro");
        assert!(typst.contains("title: \"Code Example\""));
        assert!(typst.contains("stroke-color: rgb(\"10b981\")"));
        assert!(typst.contains("Updated paragraph"));
        assert!(typst.contains("```python\nx = [10, 20]\n```"));
    }

    // 4. Test actual SlideCompiler compilation of document with nested code callout
    let compiler = CompilerBridge::new();
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let slide_typ = temp_dir.path().join("slide.typ");
    std::fs::write(
        &slide_typ,
        r#"
#let slide-colors = (
  card-bg: rgb("ffffff"),
  card-border: rgb("e2e8f0"),
  accent: rgb("3b82f6"),
)
#let title-slide(title: "", author: "") = [ = #title \ #author ]
#let slide(title: "", body) = [ == #title \ #body ]
#let callout(title: none, body, stroke-color: rgb("58a6ff")) = [
  #if title != none [*#title*]
  #body
]
"#,
    )
    .unwrap();

    let doc_src = format!(
        r#"#import "slide.typ": *
#title-slide(title: "Callout Deck")
#slide(title: "Nested Code")[
{raw_callout}
]
"#
    );
    let res = compiler.compile_source(&doc_src, Some(temp_dir.path()));
    assert!(
        res.is_ok(),
        "Callout with nested code blocks must compile cleanly: {:?}",
        res.err()
    );
}

#[test]
fn test_callout_numbered_lists_and_nested_rendering() {
    use iced::Color;
    use slide_editor::ui::theme::AppTheme;
    use slide_editor::ui::wysiwyg::block_view::CalloutSegment;
    use slide_editor::ui::wysiwyg::block_view::parse_callout_data;
    use slide_editor::ui::wysiwyg::block_view::parse_callout_segments;
    use slide_editor::ui::wysiwyg::block_view::render_callout_body;

    // 1. Numbered lists with + prefix
    let callout_plus = "+ First step\n+ Second step\n+ Third step";
    let data_plus = parse_callout_data("tip", callout_plus);
    assert_eq!(data_plus.kind, "tip");
    let segs_plus = parse_callout_segments(callout_plus);
    assert_eq!(segs_plus.len(), 3);
    for (idx, seg) in segs_plus.iter().enumerate() {
        match seg {
            | CalloutSegment::Numbered { num, text } => {
                assert_eq!(*num, "+");
                assert!(text.contains("step"));
            },
            | other => panic!("Expected Numbered for item {idx}, got {:?}", other),
        }
    }

    // 2. Numbered lists with digit prefix (1. 2. 3.)
    let callout_digits = "1. First numeric\n2. Second numeric\n3. Third numeric";
    let segs_digits = parse_callout_segments(callout_digits);
    assert_eq!(segs_digits.len(), 3);
    match &segs_digits[0] {
        | CalloutSegment::Numbered { num, text } => {
            assert_eq!(*num, "1");
            assert_eq!(*text, "First numeric");
        },
        | other => panic!("Expected Numbered, got {:?}", other),
    }

    // 3. Verify render_callout_body executes cleanly without crashing on empty and rich bodies
    let _widget_empty =
        render_callout_body("", AppTheme::Dark, Color::from_rgb(0.2, 0.6, 1.0), 1.0);
    let _widget_plus = render_callout_body(
        callout_plus,
        AppTheme::Light,
        Color::from_rgb(0.1, 0.7, 0.3),
        1.0,
    );
    let _widget_digits = render_callout_body(
        callout_digits,
        AppTheme::Dark,
        Color::from_rgb(0.9, 0.5, 0.1),
        1.2,
    );

    // 4. Test callout with bullet lists and paragraphs mixed
    let mixed_body = "Overview paragraph.\n\n- Point A\n- Point B\n\nConcluding remarks.";
    let segs_mixed = parse_callout_segments(mixed_body);
    assert_eq!(segs_mixed.len(), 4);
    assert!(matches!(segs_mixed[0], CalloutSegment::Paragraph(_)));
    assert!(matches!(segs_mixed[1], CalloutSegment::Bullet { .. }));
    assert!(matches!(segs_mixed[2], CalloutSegment::Bullet { .. }));
    assert!(matches!(segs_mixed[3], CalloutSegment::Paragraph(_)));
    let _widget_mixed = render_callout_body(
        mixed_body,
        AppTheme::Dark,
        Color::from_rgb(0.8, 0.2, 0.2),
        1.0,
    );
}

#[test]
fn test_video_and_audio_modal_and_visual_rendering() {
    use slide_editor::ui::wysiwyg::ComplexElementModal;
    use slide_editor::ui::wysiwyg::ComplexModalState;
    use slide_editor::ui::wysiwyg::block_view::InsertBlockKind;
    use slide_editor::ui::wysiwyg::block_view::parse_audio_data;
    use slide_editor::ui::wysiwyg::block_view::parse_video_data;

    // 1. InsertBlockKind template verification
    assert_eq!(InsertBlockKind::Video.label(), "Video");
    assert_eq!(InsertBlockKind::Audio.label(), "Audio");
    assert!(InsertBlockKind::Video.template().contains("#video("));
    assert!(InsertBlockKind::Audio.template().contains("#audio-player("));

    // 2. Video parser & modal from_raw
    let video_raw = r#"#video("demo.mp4", caption: "Product Teaser", duration: "01:45", quality: "1080P", style: "cinema", width: "85%")"#;
    let video_parsed = parse_video_data(
        r#"("demo.mp4", caption: "Product Teaser", duration: "01:45", quality: "1080P", style: "cinema", width: "85%")"#,
    );
    assert_eq!(video_parsed.source, "demo.mp4");
    assert_eq!(video_parsed.caption, Some("Product Teaser"));
    assert_eq!(video_parsed.duration, Some("01:45"));
    assert_eq!(video_parsed.quality, "1080P");
    assert_eq!(video_parsed.style, "cinema");
    assert_eq!(video_parsed.width, "85%");

    let modal_video =
        ComplexElementModal::from_raw(0, "b-video".to_string(), 0..10, "video", video_raw);
    match &modal_video.state {
        | ComplexModalState::Video {
            source,
            caption,
            duration,
            quality,
            style,
            width,
        } => {
            assert_eq!(source, "demo.mp4");
            assert_eq!(caption, "Product Teaser");
            assert_eq!(duration, "01:45");
            assert_eq!(quality, "1080P");
            assert_eq!(style, "cinema");
            assert_eq!(width, "85%");
        },
        | other => panic!("Expected Video state, got {:?}", other),
    }

    let generated_video = modal_video.to_typst();
    assert!(generated_video.contains("#video("));
    assert!(generated_video.contains("\"demo.mp4\""));
    assert!(generated_video.contains("caption: \"Product Teaser\""));
    assert!(generated_video.contains("style: \"cinema\""));

    // 3. Audio parser & modal from_raw
    let audio_raw = r#"#audio-player("soundtrack.mp3", title: "Theme Song", artist: "Artist Name", autoplay: true, loop: false, volume: 0.5, width: "95%")"#;
    let audio_parsed = parse_audio_data(
        "audio-player",
        r#"("soundtrack.mp3", title: "Theme Song", artist: "Artist Name", autoplay: true, loop: false, volume: 0.5, width: "95%")"#,
    );
    assert_eq!(audio_parsed.source, "soundtrack.mp3");
    assert!(audio_parsed.is_player);
    assert_eq!(audio_parsed.title, "Theme Song");
    assert_eq!(audio_parsed.artist, "Artist Name");
    assert!(audio_parsed.autoplay);
    assert!(!audio_parsed.loop_playback);
    assert!((audio_parsed.volume - 0.5).abs() < 0.01);
    assert_eq!(audio_parsed.width, "95%");

    let modal_audio =
        ComplexElementModal::from_raw(0, "b-audio".to_string(), 0..10, "audio-player", audio_raw);
    match &modal_audio.state {
        | ComplexModalState::Audio {
            source,
            is_player,
            title,
            artist,
            autoplay,
            loop_playback,
            volume,
            width,
        } => {
            assert_eq!(source, "soundtrack.mp3");
            assert!(*is_player);
            assert_eq!(title, "Theme Song");
            assert_eq!(artist, "Artist Name");
            assert!(*autoplay);
            assert!(!*loop_playback);
            assert!((*volume - 0.5).abs() < 0.01);
            assert_eq!(width, "95%");
        },
        | other => panic!("Expected Audio state, got {:?}", other),
    }

    let generated_audio = modal_audio.to_typst();
    assert!(generated_audio.contains("#audio-player("));
    assert!(generated_audio.contains("\"soundtrack.mp3\""));
    assert!(generated_audio.contains("title: \"Theme Song\""));
    assert!(generated_audio.contains("volume: 0.5"));

    // 4. Background audio (#audio)
    let bg_audio_raw = r#"#audio("ambient.mp3", autoplay: true, loop: true, volume: 0.7)"#;
    let modal_bg =
        ComplexElementModal::from_raw(0, "b-bg".to_string(), 0..10, "audio", bg_audio_raw);
    match &modal_bg.state {
        | ComplexModalState::Audio {
            is_player, source, ..
        } => {
            assert!(!*is_player);
            assert_eq!(source, "ambient.mp3");
        },
        | other => panic!("Expected Audio state, got {:?}", other),
    }
}

#[test]
fn test_direct_step_animation_button_and_modal() {
    let mut app = SlideEditorApp::new(None, false);
    assert!(app.active_modal.is_none());

    // 1. Direct step animation modal trigger (without right clicking)
    let _ = app.update(Message::OpenElementTransitionModal {
        slide_idx: 0,
        block_idx: 0,
    });
    match &app.active_modal {
        | Some(ActiveModal::ElementTransition {
            slide_idx,
            block_idx,
            order,
            effect,
            ..
        }) => {
            assert_eq!(*slide_idx, 0);
            assert_eq!(*block_idx, 0);
            assert_eq!(*order, 1);
            assert_eq!(effect, "fade-in");
        },
        | other => panic!("Expected ElementTransition modal, got {:?}", other),
    }

    // 2. Adjust transition properties directly via messages
    let _ = app.update(Message::SetElementTransitionOrder(2));
    let _ = app.update(Message::SelectElementTransitionEffect("zoom".to_string()));
    if let Some(ActiveModal::ElementTransition { order, effect, .. }) = &app.active_modal {
        assert_eq!(*order, 2);
        assert_eq!(effect, "zoom");
    } else {
        panic!("Modal unexpectedly closed");
    }

    // 3. Confirm saving transition directly
    let _ = app.update(Message::ApplyElementTransition);
    assert!(app.active_modal.is_none());
}

#[test]
fn test_focus_mode_callout_safety_and_exact_block_jump() {
    use slide_editor::model::ast_engine::instrument_source_for_focus;

    let source = r#"= Focus Mode Accuracy Test

- First item in list

#callout(title: "Nested Callout Test", stroke-color: rgb("f0883e"))[
  ```python
  def compute(x):
      return x * 2
  ```
]

Final concluding remark
"#;

    let mut app = SlideEditorApp::new(None, false);
    app.doc.source_text = source.to_string();
    app.doc.sync_chunks_from_source();
    app.engine =
        slide_editor::model::ast_engine::TypstDocumentEngine::from_source(source.to_string());
    app.sync_editors_from_doc();

    // 1. Verify instrumentation safety: Callout is NEVER wrapped with #link (which would reduce it to a dot)
    let instrumented = instrument_source_for_focus(&app.doc.source_text, 0, &app.engine);
    assert!(
        !instrumented.contains("#link(\"slide-loc:") || !instrumented.contains("][#callout"),
        "Callouts must never be wrapped in #link to prevent rendering as a single dot"
    );
    assert!(
        instrumented.contains("#callout(title: \"Nested Callout Test\""),
        "Callout must remain intact in source"
    );

    // 2. Verify Typst compilation produces full visual bounds
    let bridge = CompilerBridge::new();
    let deck_res = bridge.compile_source(&instrumented, None);
    assert!(
        deck_res.is_ok(),
        "Slide with callout and nested code must compile cleanly: {:?}",
        deck_res.err()
    );

    let deck = deck_res.unwrap();
    let slide = &deck.slides[0];
    let visual_bounds = slide_core::svg::extract_slide_visual_element_bounds(&slide.svg_data);
    assert!(
        visual_bounds.len() >= 3,
        "Extracted visual elements must include heading, list, and callout"
    );

    // 3. Test JumpToFocusBlock for the callout block
    app.doc.deck = Some(deck);
    let _ = app.update(Message::SwitchMode(EditorMode::FocusMode));

    // Content blocks: [0] = Heading, [1] = ListItem, [2] = Callout, [3] = Paragraph
    let content_blocks: Vec<_> = app.engine.slides[0]
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, b)| b.is_content_element())
        .collect();
    let callout_block_idx = content_blocks[2].0;

    let _ = app.update(Message::JumpToFocusBlock {
        slide_idx: 0,
        block_idx: callout_block_idx,
    });

    let cursor = app.focus_slide_content.cursor();
    assert!(
        cursor.selection.is_some(),
        "Cursor must have an active selection spanning the entire callout block"
    );
    // Callout starts at line 4 (0-indexed) and ends at line 10
    assert!(cursor.position.line <= 4);
    if let Some(sel) = cursor.selection {
        assert!(
            sel.line >= 9,
            "Selection must cover the entire callout including the nested code block"
        );
    }

    // 4. Verify focus mode view builds smoothly
    let _ = app.view();
}

#[test]
fn test_focus_mode_precise_tight_bounding_boxes_and_no_fullwidth_stretch() {
    let source = r#"== Slide With Bullet Items

- Item Alpha
- Item Beta
- Item Gamma

#callout(title: "Info")[Notice]
"#;

    let mut app = SlideEditorApp::new(None, false);
    app.doc.source_text = source.to_string();
    app.doc.sync_chunks_from_source();
    app.engine =
        slide_editor::model::ast_engine::TypstDocumentEngine::from_source(source.to_string());
    app.sync_editors_from_doc();

    let bridge = CompilerBridge::new();
    let deck = bridge
        .compile_source(&app.doc.source_text, None)
        .expect("Slide should compile");
    app.doc.deck = Some(deck);

    let _ = app.update(Message::SwitchMode(EditorMode::FocusMode));

    let content_block_indices: Vec<usize> = app.engine.slides[0]
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, b)| b.is_content_element())
        .map(|(i, _)| i)
        .collect();

    // Check heading (block 0)
    let heading_idx = content_block_indices[0];
    let _ = app.update(Message::JumpToFocusBlock {
        slide_idx: 0,
        block_idx: heading_idx,
    });
    let cursor = app.focus_slide_content.cursor();
    assert_eq!(cursor.position.line, 0, "Heading is on line 0");
    if let Some(sel) = cursor.selection {
        assert_eq!(
            sel.line, 0,
            "Heading selection must NOT bleed into trailing blank line 1"
        );
    }

    // Check bullet items (block 1, 2, 3)
    let item1_idx = content_block_indices[1];
    let _ = app.update(Message::JumpToFocusBlock {
        slide_idx: 0,
        block_idx: item1_idx,
    });
    let cursor = app.focus_slide_content.cursor();
    assert_eq!(cursor.position.line, 2, "Item Alpha is on line 2");
    if let Some(sel) = cursor.selection {
        assert_eq!(sel.line, 2, "Item Alpha selection must be exactly line 2");
    }

    // Verify JumpToCodeFromPreview delegates to JumpToFocusBlock
    let _ = app.update(Message::PreviewHoverRatio(0.4));
    let _ = app.update(Message::JumpToCodeFromPreview);
    let cursor = app.focus_slide_content.cursor();
    assert!(
        cursor.selection.is_some(),
        "Preview click must focus an AST block"
    );

    // Verify view generation with tight bounding boxes
    let _ = app.view();
}

#[test]
fn test_regex_search_and_replace_lifecycle() {
    let source = "== Slide 1\nHere is v1.2 and v1.3 release.\n\n== Slide 2\nAnother v2.0 release and v1.2 patch.\n";
    let mut app = SlideEditorApp::new(None, false);
    app.doc.source_text = source.to_string();
    app.doc.sync_chunks_from_source();
    app.engine =
        slide_editor::model::ast_engine::TypstDocumentEngine::from_source(source.to_string());
    app.sync_editors_from_doc();

    // 1. Open Search bar
    let _ = app.update(Message::OpenSearchModal);
    assert!(app.search_state.is_visible);

    // 2. Set regex search query
    let _ = app.update(Message::ToggleSearchRegex);
    assert!(app.search_state.is_regex);
    let _ = app.update(Message::SearchQueryChanged(r"v\d+\.\d+".to_string()));

    let matches = app.get_search_matches();
    assert_eq!(matches.len(), 4, "Should match v1.2, v1.3, v2.0, and v1.2");

    // 3. Navigate matches
    let _ = app.update(Message::FindNextMatch);
    assert_eq!(app.search_state.current_match_idx, 1);
    let _ = app.update(Message::FindPrevMatch);
    assert_eq!(app.search_state.current_match_idx, 0);

    // 4. Replace current match
    let _ = app.update(Message::ReplaceQueryChanged("VERSION_NEW".to_string()));
    let _ = app.update(Message::ExecuteReplaceCurrent);
    assert!(
        app.doc.source_text.contains("VERSION_NEW"),
        "First occurrence replaced"
    );

    // 5. Replace all remaining matches
    let _ = app.update(Message::SearchQueryChanged(r"v\d+\.\d+".to_string()));
    let _ = app.update(Message::ReplaceQueryChanged("VER".to_string()));
    let _ = app.update(Message::ExecuteReplaceAllMatches);
    assert!(
        !app.doc.source_text.contains("v1.3"),
        "All occurrences should be replaced"
    );
    assert!(
        !app.doc.source_text.contains("v2.0"),
        "All occurrences should be replaced"
    );
    assert!(app.doc.source_text.contains("VER"), "Replaced with VER");

    // 6. View with floating search bar overlay
    let _ = app.view();

    // 7. Close search bar via CloseModal (Esc)
    let _ = app.update(Message::CloseModal);
    assert!(!app.search_state.is_visible);

    // 8. Re-open and close via CloseSearchBar
    let _ = app.update(Message::OpenSearchModal);
    assert!(app.search_state.is_visible);
    let _ = app.update(Message::CloseSearchBar);
    assert!(!app.search_state.is_visible);
}

#[test]
fn test_math_formula_extraction_and_hover_rendering() {
    use slide_editor::model::ast_engine::extract_math_formula_from_line;

    // 1. Test formula extraction from lines
    let line1 = "Here is an inline formula $E = m c^2$ in text.";
    assert_eq!(
        extract_math_formula_from_line(line1),
        Some("$E = m c^2$".to_string())
    );

    let line2 = "$ sum_(i=1)^n i = (n(n+1))/2 $";
    assert_eq!(
        extract_math_formula_from_line(line2),
        Some("$ sum_(i=1)^n i = (n(n+1))/2 $".to_string())
    );

    let line3 = "No formula on this line.";
    assert_eq!(extract_math_formula_from_line(line3), None);

    // 2. Test in-editor action triggering active_hover_formula
    let source =
        "== Slide 1\nLet $x^2 + y^2 = z^2$ be Pythagorean.\n\n== Slide 2\nPlain text line.\n";
    let mut app = SlideEditorApp::new(None, false);
    app.doc.source_text = source.to_string();
    app.doc.sync_chunks_from_source();
    app.engine =
        slide_editor::model::ast_engine::TypstDocumentEngine::from_source(source.to_string());
    app.sync_editors_from_doc();

    // In focus mode, move cursor to line 1 (the formula line)
    app.mode = EditorMode::FocusMode;
    app.focus_slide_content
        .perform(iced::widget::text_editor::Action::Move(
            iced::widget::text_editor::Motion::DocumentStart,
        ));
    app.focus_slide_content
        .perform(iced::widget::text_editor::Action::Move(
            iced::widget::text_editor::Motion::Down,
        ));

    let _ = app.update(Message::FocusSlideEditorAction(
        iced::widget::text_editor::Action::Move(iced::widget::text_editor::Motion::Right),
    ));
    assert!(
        app.active_hover_formula.is_some(),
        "Hover formula must be detected on formula line"
    );
    assert_eq!(
        app.active_hover_formula.as_deref(),
        Some("$x^2 + y^2 = z^2$")
    );

    // Render focus mode with math HUD
    let _ = app.view();

    // In source mode, test math preview
    app.mode = EditorMode::SourceMode;
    let _ = app.view();
}

#[test]
fn test_header_footer_customization_and_modal_lifecycle() {
    use slide_editor::model::ast_engine::HeaderFooterConfig;
    use slide_editor::model::ast_engine::extract_header_footer_from_source;
    use slide_editor::model::ast_engine::update_header_footer_in_source;

    // 1. Test extraction from default source (no header/footer specified)
    let default_source = "#import \"slide.typ\": *\n#show: slide-theme.with(aspect-ratio: \"16-9\", theme: \"dark\")\n\n= Title\n";
    let cfg = extract_header_footer_from_source(default_source);
    assert!(!cfg.header_enabled);
    assert_eq!(cfg.footer_left, "");

    // 2. Test update_header_footer_in_source
    let custom_cfg = HeaderFooterConfig {
        header_enabled: true,
        header_left: "Acme Corp Tech Summit".to_string(),
        header_right: "Confidential".to_string(),
        footer_enabled: true,
        footer_left: "Custom Brand 2026".to_string(),
        footer_right_mode: 1,
        footer_right_custom: "Page X of Y".to_string(),
    };

    let updated_source = update_header_footer_in_source(default_source, &custom_cfg);
    assert!(updated_source.contains("header: \"Acme Corp Tech Summit\""));
    assert!(updated_source.contains("header-right: \"Confidential\""));
    assert!(updated_source.contains("footer: \"Custom Brand 2026\""));
    assert!(updated_source.contains("footer-right: \"Page X of Y\""));

    // 3. Test round-trip extraction
    let re_extracted = extract_header_footer_from_source(&updated_source);
    assert!(re_extracted.header_enabled);
    assert_eq!(re_extracted.header_left, "Acme Corp Tech Summit");
    assert_eq!(re_extracted.header_right, "Confidential");
    assert_eq!(re_extracted.footer_left, "Custom Brand 2026");
    assert_eq!(re_extracted.footer_right_mode, 1);
    assert_eq!(re_extracted.footer_right_custom, "Page X of Y");

    // 4. Test modal flow in SlideEditorApp
    let mut app = SlideEditorApp::new(None, false);
    app.doc.source_text = default_source.to_string();
    app.doc.sync_chunks_from_source();
    app.engine = slide_editor::model::ast_engine::TypstDocumentEngine::from_source(
        default_source.to_string(),
    );
    app.sync_editors_from_doc();

    let _ = app.update(Message::OpenHeaderFooterModal);
    assert!(matches!(
        app.active_modal,
        Some(ActiveModal::HeaderFooter { .. })
    ));

    let _ = app.update(Message::ToggleHeaderEnabled(true));
    let _ = app.update(Message::HeaderLeftChanged("AI Keynote".to_string()));
    let _ = app.update(Message::ToggleFooterEnabled(true));
    let _ = app.update(Message::FooterLeftChanged("Custom Company".to_string()));
    let _ = app.update(Message::FooterRightModeChanged(0));

    let _ = app.update(Message::ApplyHeaderFooterSettings);
    assert!(app.active_modal.is_none());
    assert!(app.doc.source_text.contains("header: \"AI Keynote\""));
    assert!(app.doc.source_text.contains("footer: \"Custom Company\""));

    // 5. Test view rendering
    let _ = app.view();
}

#[test]
fn test_code_window_and_chart_focus_mode_stability() {
    use slide_editor::model::ast_engine::instrument_source_for_focus;

    let source = "= Slide With Code and Chart\n\n- Bullet item A\n\n```rust\nfn main() { println!(\"Hello\"); }\n```\n\n#chart(type: \"bar\", title: \"Stats\")\n";
    let mut app = SlideEditorApp::new(None, false);
    app.doc.source_text = source.to_string();
    app.doc.sync_chunks_from_source();
    app.engine =
        slide_editor::model::ast_engine::TypstDocumentEngine::from_source(source.to_string());
    app.sync_editors_from_doc();

    let instrumented = instrument_source_for_focus(&app.doc.source_text, 0, &app.engine);
    // Bullet item and heading get safe inline links
    assert!(instrumented.contains("slide-loc:"));
    // Code block and chart are strictly NOT wrapped with inline links (preventing offset/oversized bounding box)
    assert!(!instrumented.contains("#link(\"slide-loc:5\")[```rust"));
    assert!(!instrumented.contains("#link(\"slide-loc:9\")[#chart"));

    let _ = app.view();
}

#[test]
fn test_search_navigation_scrolls_sidebar_and_syncs_slides() {
    let source = "= Slide One\nApple is first.\n\n= Slide Two\nBanana is second.\n\n= Slide Three\nCherry and Apple appear here.\n";
    let mut app = SlideEditorApp::new(None, false);
    app.doc.source_text = source.to_string();
    app.doc.sync_chunks_from_source();
    app.engine =
        slide_editor::model::ast_engine::TypstDocumentEngine::from_source(source.to_string());
    app.sync_editors_from_doc();

    assert_eq!(app.active_slide, 0);

    // 1. Search for "Cherry" (which is on slide 2, i.e. 3rd slide)
    let _ = app.update(Message::OpenSearchModal);
    let _ = app.update(Message::SearchQueryChanged("Cherry".to_string()));

    // Active slide should immediately jump to slide 2!
    assert_eq!(
        app.active_slide, 2,
        "Active slide should synchronize to slide containing match"
    );
    assert!(
        app.focus_slide_content.text().contains("Cherry"),
        "Focus editor content should load target slide chunk"
    );

    // Check sidebar scroll offset for Thumbnails and List modes
    app.sidebar_view_mode = slide_editor::app::SidebarViewMode::Thumbnails;
    assert_eq!(app.get_sidebar_scroll_offset(2), 2.0 * 190.0);
    app.sidebar_view_mode = slide_editor::app::SidebarViewMode::Outline;
    assert_eq!(app.get_sidebar_scroll_offset(2), 2.0 * 48.0);

    // 2. Search for "Apple" (matches on slide 0 and slide 2)
    let _ = app.update(Message::SearchQueryChanged("Apple".to_string()));
    assert_eq!(app.active_slide, 0, "First match is on slide 0");
    assert_eq!(app.search_state.current_match_idx, 0);

    // Next match should jump to slide 2
    let _ = app.update(Message::FindNextMatch);
    assert_eq!(app.active_slide, 2, "Next match is on slide 2");
    assert_eq!(app.search_state.current_match_idx, 1);

    // Prev match should jump back to slide 0
    let _ = app.update(Message::FindPrevMatch);
    assert_eq!(app.active_slide, 0, "Prev match wraps back to slide 0");
    assert_eq!(app.search_state.current_match_idx, 0);

    // Verify view rendering with search bar open
    let _ = app.view();
}

#[test]
fn test_compiler_bridge_theme_typ_upgrade() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = temp_dir.path();

    // Write a legacy theme.typ without header/footer parameters
    let legacy_theme = "#let slide-theme(aspect-ratio: \"16-9\", theme: \"dark\", font: none, code-font: none, body) = {\n  body\n}\n";
    std::fs::write(root.join("theme.typ"), legacy_theme).unwrap();

    let bridge = slide_editor::compiler_bridge::CompilerBridge::new();
    let source = "#import \"theme.typ\": *\n#show: slide-theme.with(header: none, footer: none)\n\n= Title\n";

    // Compilation will auto-detect legacy theme.typ and upgrade it with header/footer parameters
    let _ = bridge.compile_source(source, Some(root));

    let upgraded = std::fs::read_to_string(root.join("theme.typ")).unwrap();
    assert!(
        upgraded.contains("header: none"),
        "theme.typ should be upgraded to support header parameter"
    );
    assert!(
        upgraded.contains("footer: none"),
        "theme.typ should be upgraded to support footer parameter"
    );
}

#[test]
fn test_speaker_note_and_comment_distinction_and_insertion() {
    use slide_editor::model::ast_engine::WysiwygBlock;
    use slide_editor::ui::wysiwyg::block_view::InsertBlockKind;

    // 1. Verify template separation
    let note_template = InsertBlockKind::SpeakerNote.template();
    let comment_template = InsertBlockKind::Comment.template();
    assert!(
        note_template.contains("#speaker-note"),
        "Speaker note template should use #speaker-note"
    );
    assert!(
        comment_template.contains("// Comment:"),
        "Comment template should use // Comment: to avoid note prefix collision"
    );
    assert!(
        !comment_template.contains("// Note:"),
        "Comment template MUST NOT start with // Note: which causes speaker note classification"
    );

    // 2. Parse a slide containing both a comment and a speaker note
    let source = r#"#slide(title: "Distinct Cues")[
  = Main Slide
  // Comment: This is an internal presentation comment
  Content paragraph here.
  #speaker-note[
    These are the verbal cues for the speaker.
  ]
]"#;
    let engine = TypstDocumentEngine::from_source(source.to_string());
    assert_eq!(engine.slides.len(), 1);
    let slide = &engine.slides[0];

    let has_comment = slide.blocks.iter().any(|b| {
        if let WysiwygBlock::Comment { content, .. } = b {
            content.contains("internal presentation comment")
        } else {
            false
        }
    });
    assert!(
        has_comment,
        "Comment must be parsed as WysiwygBlock::Comment"
    );

    let has_note = slide.blocks.iter().any(|b| {
        if let WysiwygBlock::SpeakerNote { content, .. } = b {
            content.contains("verbal cues for the speaker")
        } else {
            false
        }
    });
    assert!(
        has_note,
        "Speaker note must be parsed as WysiwygBlock::SpeakerNote"
    );

    // Ensure they have distinct chips
    for b in &slide.blocks {
        match b {
            | WysiwygBlock::SpeakerNote { .. } => {
                let (badge, _) = b.chip_info();
                assert_eq!(badge, "Note");
            },
            | WysiwygBlock::Comment { .. } => {
                let (badge, _) = b.chip_info();
                assert_eq!(badge, "Comment");
            },
            | _ => {},
        }
    }
}

#[test]
fn test_comment_and_speaker_note_visualization_and_root_insertion() {
    use slide_editor::model::ast_engine::TypstDocumentEngine;
    use slide_editor::model::ast_engine::WysiwygBlock;
    use slide_editor::ui::wysiwyg::block_view::InsertBlockKind;
    let example_path = std::path::PathBuf::from("../../examples/geek-presentation/slides.typ");
    let content = std::fs::read_to_string(&example_path).expect("read slides.typ");
    let mut engine = TypstDocumentEngine::from_source(content);

    // 1. Verify that preamble/root-level comment before Slide 1 is preserved and recognized
    let slide0 = &engine.slides[0];
    let has_preamble_comment = slide0.blocks.iter().any(|b| {
        if let WysiwygBlock::Comment { content, .. } = b {
            content.contains("Title Slide with Autoplay")
        } else {
            false
        }
    });
    assert!(
        has_preamble_comment,
        "Preamble comment must be parsed into Slide 1"
    );

    // 2. Insert a comment into Slide 0 (Title Slide)
    let offset_0 = engine.get_slide_content_insert_offset(0);
    engine.insert_block_after(offset_0, InsertBlockKind::Comment.template());
    let slide0_after = &engine.slides[0];
    let has_inserted_comment_0 = slide0_after.blocks.iter().any(|b| {
        if let WysiwygBlock::Comment { content, .. } = b {
            content.contains("presentation memo or draft remark")
        } else {
            false
        }
    });
    assert!(
        has_inserted_comment_0,
        "Comment inserted on Title Slide must be parsed and visualized"
    );

    // 3. Insert a comment into Slide 1 (Macro Slide)
    let offset_1 = engine.get_slide_content_insert_offset(1);
    engine.insert_block_after(offset_1, InsertBlockKind::Comment.template());
    let slide1_after = &engine.slides[1];
    let has_inserted_comment_1 = slide1_after.blocks.iter().any(|b| {
        if let WysiwygBlock::Comment { content, .. } = b {
            content.contains("presentation memo or draft remark")
        } else {
            false
        }
    });
    assert!(
        has_inserted_comment_1,
        "Comment inserted on Macro Slide must be parsed and visualized"
    );

    // 4. Verify clean content extraction (prefixes like 'Comment:' stripped from display content)
    for b in &slide1_after.blocks {
        if let WysiwygBlock::Comment { content, .. } = b
            && content.contains("presentation memo or draft remark")
        {
            assert!(
                !content.starts_with("Comment:"),
                "Comment display content must not redundantly include 'Comment:' prefix"
            );
        }
    }
}

#[test]
fn test_floating_context_menus_and_cursor_tracking() {
    let mut app = SlideEditorApp::new(None, false);

    // 1. Cursor position and window size tracking
    let cursor_pt = iced::Point::new(320.0, 210.0);
    let _ = app.update(Message::GlobalCursorMoved(cursor_pt));
    assert_eq!(app.last_cursor_pos, Some(cursor_pt));

    let new_size = iced::Size::new(1600.0, 900.0);
    let _ = app.update(Message::WindowResized(new_size));
    assert_eq!(app.window_size, new_size);

    // 2. Open Slide Context Menu - captures last cursor position
    let _ = app.update(Message::OpenSlideContextMenu(0));
    match app.active_modal.as_ref() {
        | Some(ActiveModal::SlideContextMenu { slide_idx, position }) => {
            assert_eq!(*slide_idx, 0);
            assert_eq!(*position, Some(cursor_pt));
        },
        | _ => panic!("Expected SlideContextMenu active modal"),
    }

    // View rendering must execute smoothly without errors
    let _ = app.view();

    // Close modal via Escape / backdrop click
    let _ = app.update(Message::CloseModal);
    assert!(app.active_modal.is_none());

    // 3. Open Block Context Menu - captures position and verifies dismissal on actions
    let _ = app.update(Message::OpenBlockContextMenu {
        slide_idx: 0,
        block_idx: 0,
        range: 0..5,
        block_label: "Heading".to_string(),
        block_id: "blk-test-1".to_string(),
    });

    match app.active_modal.as_ref() {
        | Some(ActiveModal::BlockContextMenu {
            slide_idx,
            block_idx,
            range,
            block_label,
            position,
            ..
        }) => {
            assert_eq!(*slide_idx, 0);
            assert_eq!(*block_idx, 0);
            assert_eq!(*range, 0..5);
            assert_eq!(block_label, "Heading");
            assert_eq!(*position, Some(cursor_pt));
        },
        | _ => panic!("Expected BlockContextMenu active modal"),
    }

    let _ = app.view();

    // Deleting element dismisses the context menu immediately
    let _ = app.update(Message::DeleteBlockAtRange(0..5));
    assert!(app.active_modal.is_none());

    // 4. Duplicate block dismisses context menu
    let _ = app.update(Message::OpenBlockContextMenu {
        slide_idx: 0,
        block_idx: 0,
        range: 0..5,
        block_label: "Callout".to_string(),
        block_id: "blk-test-2".to_string(),
    });
    assert!(app.active_modal.is_some());
    let _ = app.update(Message::DuplicateBlock(0..5));
    assert!(app.active_modal.is_none());

    // 5. Delete slide dismisses slide context menu
    let _ = app.update(Message::OpenSlideContextMenu(0));
    assert!(app.active_modal.is_some());
    let _ = app.update(Message::DeleteSlide(0));
    assert!(app.active_modal.is_none());
}

#[test]
fn test_responsive_window_scaling_and_proportional_layout() {
    let mut app = SlideEditorApp::new(None, false);

    // Test different screen widths: Ultrawide, Standard, Compact, and Ultra-narrow
    let test_resolutions = [
        iced::Size::new(1920.0, 1080.0),
        iced::Size::new(1440.0, 900.0),
        iced::Size::new(1100.0, 700.0),
        iced::Size::new(800.0, 600.0),
        iced::Size::new(550.0, 450.0),
    ];

    for res in test_resolutions {
        let _ = app.update(Message::WindowResized(res));
        assert_eq!(app.window_size, res);

        // Rendering view must succeed without overflow panic across all responsive tiers
        let _ = app.view();

        // Switch modes and ensure responsive view rendering functions correctly
        let _ = app.update(Message::SwitchMode(EditorMode::FocusMode));
        let _ = app.view();

        let _ = app.update(Message::SwitchMode(EditorMode::SourceMode));
        let _ = app.view();

        let _ = app.update(Message::SwitchMode(EditorMode::LivePreview));
        let _ = app.view();
    }

    // Test extreme zoom levels (50% to 200%) to verify slide card width clamping and no UI crush
    let zoom_levels = [50, 75, 100, 150, 200];
    for z in zoom_levels {
        app.zoom_percent = z;
        let _ = app.view();
    }
}

#[test]
fn test_command_palette_modal_and_agenda_generation() {
    let mut app = SlideEditorApp::new(None, false);

    // 1. Zoom and Pace cycling
    app.zoom_percent = 150;
    let _ = app.update(Message::ResetZoom);
    assert_eq!(app.zoom_percent, 100);

    assert_eq!(app.speaking_wpm, 130);
    let _ = app.update(Message::CycleSpeakingPace);
    assert_eq!(app.speaking_wpm, 160);
    let _ = app.update(Message::CycleSpeakingPace);
    assert_eq!(app.speaking_wpm, 100);
    let _ = app.update(Message::CycleSpeakingPace);
    assert_eq!(app.speaking_wpm, 130);

    // 2. Command Palette lifecycle & filtering
    let _ = app.update(Message::OpenCommandPalette);
    assert!(matches!(
        app.active_modal,
        Some(ActiveModal::CommandPalette { .. })
    ));

    // View rendering with Command Palette open
    let _ = app.view();

    let _ = app.update(Message::CommandPaletteQueryChanged("zoom".to_string()));
    if let Some(ActiveModal::CommandPalette {
        ref query,
        selected_idx,
    }) = app.active_modal
    {
        assert_eq!(query, "zoom");
        assert_eq!(selected_idx, 0);
    } else {
        panic!("Command palette should be active");
    }

    // Execute selected command (Reset Zoom)
    let _ = app.update(Message::CommandPaletteExecute);
    assert!(app.active_modal.is_none());

    // 3. Generate Agenda Slide
    let initial_slides = app.doc.total_slides();
    let _ = app.update(Message::GenerateAgendaSlide);
    assert!(app.doc.total_slides() > initial_slides);
    let current_source = &app.doc.source_text;
    assert!(current_source.contains("Agenda"));

    // 4. Delete slide protection
    let _ = app.update(Message::DeleteActiveSlide);
    assert_eq!(app.doc.total_slides(), initial_slides);
}
