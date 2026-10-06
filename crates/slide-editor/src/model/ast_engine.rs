//! Typst Abstract Syntax Tree & Document Engine.
//!
//! Powered directly by official `typst-syntax`.
//! Provides zero-loss, zero-wildcard AST classification and bidirectional range splicing.

use std::ops::Range;
use typst_syntax::LinkedNode;
use typst_syntax::SyntaxKind;
use typst_syntax::ast::AstNode;
use typst_syntax::ast::{
    self,
};

/// Semantic block within a Typora-style visual presentation page
#[derive(Debug, Clone, PartialEq)]
pub enum WysiwygBlock {
    /// Section Heading (= Title, == Subtitle, etc.)
    Heading {
        id: String,
        level: usize,
        title: String,
        range: Range<usize>,
        raw: String,
    },
    /// Bullet list item (- Item)
    ListItem {
        id: String,
        body: String,
        range: Range<usize>,
        raw: String,
    },
    /// Numbered enumeration item (+ Item or 1. Item)
    EnumItem {
        id: String,
        number: Option<usize>,
        body: String,
        range: Range<usize>,
        raw: String,
    },
    /// Term item (/ Term: Description)
    TermItem {
        id: String,
        term: String,
        description: String,
        range: Range<usize>,
        raw: String,
    },
    /// Code block (```lang ... ```)
    CodeBlock {
        id: String,
        language: String,
        code: String,
        range: Range<usize>,
        raw: String,
    },
    /// Standalone or inline mathematical equation ($ ... $)
    Equation {
        id: String,
        formula: String,
        display: bool,
        range: Range<usize>,
        raw: String,
    },
    /// Regular text paragraph
    Paragraph {
        id: String,
        text: String,
        range: Range<usize>,
        raw: String,
    },
    /// Set rule (#set ...)
    SetRule {
        id: String,
        target: String,
        range: Range<usize>,
        raw: String,
    },
    /// Show rule (#show ...)
    ShowRule {
        id: String,
        target: Option<String>,
        range: Range<usize>,
        raw: String,
    },
    /// Let binding (#let ...)
    LetBinding {
        id: String,
        name: String,
        range: Range<usize>,
        raw: String,
    },
    /// Module import or include (#import ..., #include ...)
    Module {
        id: String,
        path: String,
        range: Range<usize>,
        raw: String,
    },
    /// Macro or function call (#align, #grid, #video, etc.)
    FuncCall {
        id: String,
        callee: String,
        args: String,
        range: Range<usize>,
        raw: String,
    },
    /// Pure code block ({ let x = 1; ... })
    PureCode {
        id: String,
        body: String,
        range: Range<usize>,
        raw: String,
    },
    /// Content block ([ ... ])
    ContentBlock {
        id: String,
        body: String,
        range: Range<usize>,
        raw: String,
    },
    /// Control flow (#if, #for, #while, etc.)
    ControlFlow {
        id: String,
        kind: String,
        range: Range<usize>,
        raw: String,
    },
    /// Other expressions (literals, variables, operators)
    Expression {
        id: String,
        kind: String,
        range: Range<usize>,
        raw: String,
    },
    /// Dedicated speaker note for presenter display (#speaker-note[...] or // [note]: ...)
    SpeakerNote {
        id: String,
        range: Range<usize>,
        raw: String,
        content: String,
    },
    /// General code comment (// comment or /* comment */)
    Comment {
        id: String,
        range: Range<usize>,
        raw: String,
        content: String,
        is_block: bool,
    },
}

impl WysiwygBlock {
    /// Return unique block identifier
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            | Self::Heading { id, .. }
            | Self::ListItem { id, .. }
            | Self::EnumItem { id, .. }
            | Self::TermItem { id, .. }
            | Self::CodeBlock { id, .. }
            | Self::Equation { id, .. }
            | Self::Paragraph { id, .. }
            | Self::SetRule { id, .. }
            | Self::ShowRule { id, .. }
            | Self::LetBinding { id, .. }
            | Self::Module { id, .. }
            | Self::FuncCall { id, .. }
            | Self::PureCode { id, .. }
            | Self::ContentBlock { id, .. }
            | Self::ControlFlow { id, .. }
            | Self::Expression { id, .. }
            | Self::SpeakerNote { id, .. }
            | Self::Comment { id, .. } => id,
        }
    }

    /// Return exact byte range of this block in source text
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        match self {
            | Self::Heading { range, .. }
            | Self::ListItem { range, .. }
            | Self::EnumItem { range, .. }
            | Self::TermItem { range, .. }
            | Self::CodeBlock { range, .. }
            | Self::Equation { range, .. }
            | Self::Paragraph { range, .. }
            | Self::SetRule { range, .. }
            | Self::ShowRule { range, .. }
            | Self::LetBinding { range, .. }
            | Self::Module { range, .. }
            | Self::FuncCall { range, .. }
            | Self::PureCode { range, .. }
            | Self::ContentBlock { range, .. }
            | Self::ControlFlow { range, .. }
            | Self::Expression { range, .. }
            | Self::SpeakerNote { range, .. }
            | Self::Comment { range, .. } => range.clone(),
        }
    }

    /// Return raw source text of this block
    #[must_use]
    pub fn raw(&self) -> &str {
        match self {
            | Self::Heading { raw, .. }
            | Self::ListItem { raw, .. }
            | Self::EnumItem { raw, .. }
            | Self::TermItem { raw, .. }
            | Self::CodeBlock { raw, .. }
            | Self::Equation { raw, .. }
            | Self::Paragraph { raw, .. }
            | Self::SetRule { raw, .. }
            | Self::ShowRule { raw, .. }
            | Self::LetBinding { raw, .. }
            | Self::Module { raw, .. }
            | Self::FuncCall { raw, .. }
            | Self::PureCode { raw, .. }
            | Self::ContentBlock { raw, .. }
            | Self::ControlFlow { raw, .. }
            | Self::Expression { raw, .. }
            | Self::SpeakerNote { raw, .. }
            | Self::Comment { raw, .. } => raw,
        }
    }

    /// Return whether this block is a user-visible content element on a presentation slide
    #[must_use]
    pub fn is_content_element(&self) -> bool {
        match self {
            | Self::Heading { .. }
            | Self::ListItem { .. }
            | Self::EnumItem { .. }
            | Self::TermItem { .. }
            | Self::CodeBlock { .. }
            | Self::Equation { .. }
            | Self::Paragraph { .. }
            | Self::ContentBlock { .. }
            | Self::SpeakerNote { .. }
            | Self::Comment { .. } => true,
            | Self::FuncCall { callee, .. } => {
                callee != "v" && callee != "h" && callee != "pagebreak" && callee != "colbreak"
            },
            | Self::SetRule { .. }
            | Self::ShowRule { .. }
            | Self::LetBinding { .. }
            | Self::Module { .. }
            | Self::PureCode { .. }
            | Self::ControlFlow { .. }
            | Self::Expression { .. } => false,
        }
    }

    /// Return (badge, short_preview) for UI element chip representation
    #[must_use]
    pub fn chip_info(&self) -> (String, String) {
        match self {
            | Self::Heading { level, title, .. } => {
                let badge = format!("H{level}");
                let preview = if title.chars().count() > 22 {
                    format!("{}…", title.chars().take(22).collect::<String>())
                } else {
                    title.clone()
                };
                (badge, preview)
            },
            | Self::ListItem { body, .. } => {
                let preview = if body.chars().count() > 22 {
                    format!("{}…", body.chars().take(22).collect::<String>())
                } else {
                    body.clone()
                };
                ("List".to_string(), preview)
            },
            | Self::EnumItem { number, body, .. } => {
                let badge = format!("{}.", number.unwrap_or(1));
                let preview = if body.chars().count() > 22 {
                    format!("{}…", body.chars().take(22).collect::<String>())
                } else {
                    body.clone()
                };
                (badge, preview)
            },
            | Self::TermItem { term, .. } => {
                let preview = if term.chars().count() > 22 {
                    format!("{}…", term.chars().take(22).collect::<String>())
                } else {
                    term.clone()
                };
                ("Def".to_string(), preview)
            },
            | Self::CodeBlock { language, .. } => {
                let badge = if language.is_empty() {
                    "Code".to_string()
                } else {
                    language.clone()
                };
                (badge, "Snippet".to_string())
            },
            | Self::Equation { formula, .. } => {
                let clean = formula.trim().trim_matches('$').trim();
                let preview = if clean.chars().count() > 20 {
                    format!("{}…", clean.chars().take(20).collect::<String>())
                } else {
                    clean.to_string()
                };
                ("Math".to_string(), preview)
            },
            | Self::Paragraph { text, .. } => {
                let preview = if text.chars().count() > 22 {
                    format!("{}…", text.chars().take(22).collect::<String>())
                } else {
                    text.clone()
                };
                ("Text".to_string(), preview)
            },
            | Self::FuncCall { callee, args, .. } => {
                let badge = match callee.as_str() {
                    | "title-slide" => "Title".to_string(),
                    | "table" => "Table".to_string(),
                    | "grid" => "Grid".to_string(),
                    | "cols" | "columns" => "Cols".to_string(),
                    | "callout" | "note" | "tip" | "warning" | "info" | "alert" | "quote" => {
                        callee.clone()
                    },
                    | "box" | "block" | "rect" => "Box".to_string(),
                    | "link" => "Link".to_string(),
                    | "chart" | "chart-bar" | "chart-pie" | "chart-line" | "plot" => {
                        "Chart".to_string()
                    },
                    | _ => format!("#{callee}"),
                };
                let preview = if callee == "title-slide" {
                    extract_arg_str(args, "title").unwrap_or_else(|| "Title Slide".to_string())
                } else {
                    let clean = args.trim().trim_matches(|c| {
                        c == '(' || c == ')' || c == '"' || c == '\'' || c == '[' || c == ']'
                    });
                    if clean.is_empty() {
                        callee.clone()
                    } else if clean.chars().count() > 20 {
                        format!("{}…", clean.chars().take(20).collect::<String>())
                    } else {
                        clean.to_string()
                    }
                };
                (badge, preview)
            },
            | Self::ContentBlock { body, .. } => {
                let preview = if body.chars().count() > 20 {
                    format!("{}…", body.chars().take(20).collect::<String>())
                } else {
                    body.clone()
                };
                ("Block".to_string(), preview)
            },
            | Self::SpeakerNote { content, .. } => {
                let preview = if content.chars().count() > 22 {
                    format!("{}…", content.chars().take(22).collect::<String>())
                } else {
                    content.clone()
                };
                ("Note".to_string(), preview)
            },
            | Self::Comment { content, .. } => {
                let preview = if content.chars().count() > 22 {
                    format!("{}…", content.chars().take(22).collect::<String>())
                } else {
                    content.clone()
                };
                ("Comment".to_string(), preview)
            },
            | _ => ("Directive".to_string(), String::new()),
        }
    }
}

/// An element transition / fragment step configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementTransition {
    pub order: usize,
    pub effect: String,
    pub wrapper_range: Range<usize>,
}

/// A visual slide page holding structured blocks
#[derive(Debug, Clone, PartialEq)]
pub struct WysiwygSlide {
    pub page_number: usize,
    pub title: String,
    pub blocks: Vec<WysiwygBlock>,
    pub range: Range<usize>,
    pub transition: Option<String>,
    pub element_transitions: std::collections::HashMap<String, ElementTransition>,
}

/// Main document engine backed by official `typst_syntax`
#[derive(Debug, Clone)]
pub struct TypstDocumentEngine {
    pub source_text: String,
    pub slides: Vec<WysiwygSlide>,
}

impl TypstDocumentEngine {
    /// Parse source text into structured slides and blocks
    #[must_use]
    pub fn from_source(source: String) -> Self {
        let mut engine = Self {
            source_text: source,
            slides: Vec::new(),
        };
        engine.reparse();
        engine
    }

    /// Re-parse source code using official `typst_syntax::parse` and `LinkedNode`
    pub fn reparse(&mut self) {
        let root = typst_syntax::parse(&self.source_text);
        let linked_root = LinkedNode::new(&root);

        let has_slide_macros = linked_root.children().any(|child| {
            if let Some(fc) = child.get().cast::<ast::FuncCall>() {
                let callee = fc.callee().to_untyped().full_text();
                is_slide_macro(callee.as_str())
            } else {
                false
            }
        });

        let mut slides = Vec::new();
        let mut block_counter = 0usize;

        if has_slide_macros {
            self.reparse_macro_slides(&linked_root, &mut slides, &mut block_counter);
        } else {
            self.reparse_pagebreak_slides(&linked_root, &mut slides, &mut block_counter);
        }

        if slides.is_empty() {
            if !self.slides.is_empty()
                && (self.source_text.contains("#slide(")
                    || self.source_text.contains("#title-slide(")
                    || self.source_text.contains("#pagebreak"))
            {
                // Prevent temporary syntax errors during typing from collapsing the presentation to slide 1
                return;
            }
            let mut fallback_blocks = Vec::new();
            let mut fallback_transitions = std::collections::HashMap::new();
            extract_semantic_blocks_from_nodes(
                linked_root.children(),
                &self.source_text,
                &mut block_counter,
                &mut fallback_blocks,
                &mut fallback_transitions,
            );
            slides.push(WysiwygSlide {
                page_number: 1,
                title: "Slide 1".to_string(),
                blocks: fallback_blocks,
                range: 0..self.source_text.len(),
                transition: None,
                element_transitions: fallback_transitions,
            });
        }

        self.slides = slides;
    }

    fn reparse_macro_slides(
        &self,
        linked_root: &LinkedNode,
        slides: &mut Vec<WysiwygSlide>,
        block_counter: &mut usize,
    ) {
        let mut last_hash_start: Option<usize> = None;
        let mut pending_preamble_blocks: Vec<WysiwygBlock> = Vec::new();

        for child in linked_root.children() {
            let kind = child.kind();
            if kind == SyntaxKind::Space || kind == SyntaxKind::Parbreak {
                continue;
            }

            if kind == SyntaxKind::Hash {
                if last_hash_start.is_none() {
                    last_hash_start = Some(child.range().start);
                }
                continue;
            }

            let start = last_hash_start.take().unwrap_or(child.range().start);

            if let Some(fc) = child.get().cast::<ast::FuncCall>() {
                let callee = fc.callee().to_untyped().full_text();
                let callee_str = callee.as_str().trim();

                if is_slide_macro(callee_str) {
                    let slide_num = slides.len() + 1;
                    let mut slide_blocks = Vec::new();
                    let mut slide_transitions = std::collections::HashMap::new();

                    if slides.is_empty() && !pending_preamble_blocks.is_empty() {
                        slide_blocks.append(&mut pending_preamble_blocks);
                    }

                    let mut slide_title = None;
                    let mut slide_transition = None;
                    for arg in fc.args().items() {
                        if let ast::Arg::Named(n) = arg {
                            let arg_name = n.name().as_str();
                            let arg_val = n.expr().to_untyped().full_text();
                            let clean_val = arg_val.as_str().trim().trim_matches('"').to_string();

                            if arg_name == "title" {
                                slide_title = Some(clean_val.clone());
                            } else if arg_name == "transition"
                                && !clean_val.is_empty()
                                && clean_val != "none"
                                && clean_val != "cut"
                            {
                                slide_transition = Some(clean_val.clone());
                            }
                        }
                    }

                    let slide_range = start..child.range().end;
                    if slide_transition.is_none() {
                        let slide_slice = self.source_text.get(slide_range.clone()).unwrap_or("");
                        slide_transition = extract_transition_from_raw(slide_slice);
                    }

                    if callee_str == "title-slide" {
                        let full_range = start..child.range().end;
                        let raw_text = self
                            .source_text
                            .get(full_range.clone())
                            .unwrap_or("")
                            .to_string();
                        if let Some(block) =
                            classify_linked_node(&child, full_range, raw_text, block_counter)
                        {
                            slide_blocks.push(block);
                        }
                    }

                    // Extract all inner blocks from the slide body / ContentBlocks
                    for cb in find_content_blocks_in_funccall(&child) {
                        extract_semantic_blocks_from_nodes(
                            cb.children(),
                            &self.source_text,
                            block_counter,
                            &mut slide_blocks,
                            &mut slide_transitions,
                        );
                    }

                    let title = slide_title.unwrap_or_else(|| {
                        if callee_str == "title-slide" {
                            "Title Slide".to_string()
                        } else {
                            format!("Slide {slide_num}")
                        }
                    });

                    slides.push(WysiwygSlide {
                        page_number: slide_num,
                        title,
                        blocks: slide_blocks,
                        range: slide_range,
                        transition: slide_transition,
                        element_transitions: slide_transitions,
                    });
                    continue;
                }
            }

            let full_range = start..child.range().end;
            let raw_text = self
                .source_text
                .get(full_range.clone())
                .unwrap_or("")
                .to_string();
            if let Some(block) = classify_linked_node(&child, full_range, raw_text, block_counter) {
                if let Some(last_slide) = slides.last_mut() {
                    last_slide.range.end = child.range().end;
                    last_slide.blocks.push(block);
                } else {
                    pending_preamble_blocks.push(block);
                }
            }
        }
    }

    fn reparse_pagebreak_slides(
        &self,
        linked_root: &LinkedNode,
        slides: &mut Vec<WysiwygSlide>,
        block_counter: &mut usize,
    ) {
        let mut slide_nodes = Vec::new();
        let mut slide_start_offset = 0;
        let mut last_hash_start: Option<usize> = None;
        let mut has_seen_heading = false;

        let has_explicit_pagebreaks = linked_root.children().any(|child| {
            let full_range = child.range();
            let raw_text = self.source_text.get(full_range).unwrap_or("");
            is_slide_boundary(&child, raw_text)
        });

        for child in linked_root.children() {
            if child.kind() == SyntaxKind::Hash {
                if last_hash_start.is_none() {
                    last_hash_start = Some(child.range().start);
                }
                slide_nodes.push(child);
                continue;
            }

            let start = last_hash_start.take().unwrap_or(child.range().start);
            let full_range = start..child.range().end;
            let raw_text = self.source_text.get(full_range).unwrap_or("").to_string();

            let is_h1 = !has_explicit_pagebreaks
                && child.kind() == SyntaxKind::Heading
                && child
                    .get()
                    .cast::<ast::Heading>()
                    .is_some_and(|h| h.depth().get() == 1);

            if is_slide_boundary(&child, &raw_text) {
                let slide_num = slides.len() + 1;
                let mut current_blocks = Vec::new();
                let mut current_transitions = std::collections::HashMap::new();
                extract_semantic_blocks_from_nodes(
                    slide_nodes.drain(..),
                    &self.source_text,
                    block_counter,
                    &mut current_blocks,
                    &mut current_transitions,
                );

                let slide_title = current_blocks
                    .iter()
                    .find_map(|b| {
                        match b {
                            | WysiwygBlock::Heading { title, .. } => Some(title.clone()),
                            | WysiwygBlock::FuncCall { callee, args, .. }
                                if callee == "title-slide" =>
                            {
                                extract_arg_str(args, "title")
                            },
                            | _ => None,
                        }
                    })
                    .unwrap_or_else(|| format!("Slide {slide_num}"));

                let slide_range = slide_start_offset..start;
                let slide_raw = self.source_text.get(slide_range.clone()).unwrap_or("");
                let slide_transition = extract_transition_from_raw(slide_raw);

                slides.push(WysiwygSlide {
                    page_number: slide_num,
                    title: slide_title,
                    blocks: current_blocks,
                    range: slide_range,
                    transition: slide_transition,
                    element_transitions: current_transitions,
                });
                slide_start_offset = child.range().end;
                has_seen_heading = false;
                continue;
            } else if is_h1 {
                if has_seen_heading && !slide_nodes.is_empty() {
                    let slide_num = slides.len() + 1;
                    let mut current_blocks = Vec::new();
                    let mut current_transitions = std::collections::HashMap::new();
                    extract_semantic_blocks_from_nodes(
                        slide_nodes.drain(..),
                        &self.source_text,
                        block_counter,
                        &mut current_blocks,
                        &mut current_transitions,
                    );

                    let slide_title = current_blocks
                        .iter()
                        .find_map(|b| {
                            match b {
                                | WysiwygBlock::Heading { title, .. } => Some(title.clone()),
                                | WysiwygBlock::FuncCall { callee, args, .. }
                                    if callee == "title-slide" =>
                                {
                                    extract_arg_str(args, "title")
                                },
                                | _ => None,
                            }
                        })
                        .unwrap_or_else(|| format!("Slide {slide_num}"));

                    let slide_range = slide_start_offset..start;
                    let slide_raw = self.source_text.get(slide_range.clone()).unwrap_or("");
                    let slide_transition = extract_transition_from_raw(slide_raw);

                    slides.push(WysiwygSlide {
                        page_number: slide_num,
                        title: slide_title,
                        blocks: current_blocks,
                        range: slide_range,
                        transition: slide_transition,
                        element_transitions: current_transitions,
                    });
                    slide_start_offset = start;
                }
                has_seen_heading = true;
            }

            slide_nodes.push(child);
        }

        // Seal final slide
        let slide_num = slides.len() + 1;
        let mut current_blocks = Vec::new();
        let mut current_transitions = std::collections::HashMap::new();
        extract_semantic_blocks_from_nodes(
            slide_nodes.drain(..),
            &self.source_text,
            block_counter,
            &mut current_blocks,
            &mut current_transitions,
        );

        let slide_title = current_blocks
            .iter()
            .find_map(|b| {
                match b {
                    | WysiwygBlock::Heading { title, .. } => Some(title.clone()),
                    | WysiwygBlock::FuncCall { callee, args, .. } if callee == "title-slide" => {
                        extract_arg_str(args, "title")
                    },
                    | _ => None,
                }
            })
            .unwrap_or_else(|| format!("Slide {slide_num}"));

        let slide_range = slide_start_offset..self.source_text.len();
        let slide_raw = self.source_text.get(slide_range.clone()).unwrap_or("");
        let slide_transition = extract_transition_from_raw(slide_raw);

        slides.push(WysiwygSlide {
            page_number: slide_num,
            title: slide_title,
            blocks: current_blocks,
            range: slide_range,
            transition: slide_transition,
            element_transitions: current_transitions,
        });
    }

    /// In-place update a specific block using exact byte-range splice
    pub fn update_block_at_range(
        &mut self,
        target_range: Range<usize>,
        new_text: &str,
    ) -> Option<Range<usize>> {
        if target_range.start > self.source_text.len() || target_range.end > self.source_text.len()
        {
            return None;
        }

        // Exact byte-range splice in the source string
        self.source_text
            .replace_range(target_range.clone(), new_text);

        let new_range = target_range.start..(target_range.start + new_text.len());
        self.reparse();
        Some(new_range)
    }

    /// In-place update a block text without reparsing AST immediately (for active keystroke typing)
    pub fn update_block_at_range_unparsed(
        &mut self,
        target_range: Range<usize>,
        new_text: &str,
    ) -> Option<Range<usize>> {
        if target_range.start > self.source_text.len() || target_range.end > self.source_text.len()
        {
            return None;
        }

        self.source_text
            .replace_range(target_range.clone(), new_text);

        let new_range = target_range.start..(target_range.start + new_text.len());
        Some(new_range)
    }

    /// Calculate safe insertion offset inside slide content boundaries
    #[must_use]
    pub fn get_slide_content_insert_offset(
        &self,
        slide_idx: usize,
    ) -> usize {
        if let Some(slide) = self.slides.get(slide_idx) {
            // If slide already has content blocks, insert right after the last content block
            if let Some(last_block) = slide.blocks.iter().rfind(|b| b.is_content_element()) {
                return last_block.range().end;
            }
            // For macro slide #slide(...)[\n...], find the last closing ']' before slide.range.end
            let slice_end = slide.range.end.min(self.source_text.len());
            if slide.range.start < slice_end {
                let slide_slice = &self.source_text[slide.range.start..slice_end];
                if let Some(bracket_idx) = slide_slice.rfind(']') {
                    return slide.range.start + bracket_idx;
                }
            }
            return slide.range.end;
        }
        self.source_text.len()
    }

    /// Insert a new block after a specific offset
    pub fn insert_block_after(
        &mut self,
        after_offset: usize,
        block_text: &str,
    ) {
        let insert_pos = after_offset.min(self.source_text.len());
        let to_insert = format!("\n\n{block_text}\n");
        self.source_text.insert_str(insert_pos, &to_insert);
        self.reparse();
    }

    /// Add a brand new slide at the end
    pub fn add_new_slide(&mut self) {
        let new_num = self.slides.len() + 1;
        let has_macros = self.slides.iter().any(|s| {
            self.source_text
                .get(s.range.clone())
                .is_some_and(|t| t.contains("#slide(") || t.contains("#title-slide("))
        });
        let template = if has_macros {
            format!("\n\n#slide(title: \"Slide {new_num}\")[\n  - Key point or takeaway\n]\n")
        } else {
            format!("\n\n#pagebreak()\n\n= Slide {new_num}\n\n- Key point or takeaway\n")
        };
        self.source_text.push_str(&template);
        self.reparse();
    }

    /// Delete a slide by index
    pub fn delete_slide(
        &mut self,
        idx: usize,
    ) {
        if self.slides.len() <= 1 || idx >= self.slides.len() {
            return;
        }
        let slide_range = self.slides[idx].range.clone();
        if slide_range.end <= self.source_text.len() {
            self.source_text.drain(slide_range);
            self.reparse();
        }
    }

    /// Move a semantic block from one slide to another
    pub fn move_block_to_slide(
        &mut self,
        from_slide_idx: usize,
        block_range: Range<usize>,
        to_slide_idx: usize,
    ) -> bool {
        if from_slide_idx == to_slide_idx
            || from_slide_idx >= self.slides.len()
            || to_slide_idx >= self.slides.len()
            || block_range.start >= self.source_text.len()
            || block_range.end > self.source_text.len()
            || block_range.start >= block_range.end
        {
            return false;
        }

        let block_text = self.source_text[block_range.clone()].trim().to_string();
        if block_text.is_empty() {
            return false;
        }

        // Cut block from current slide
        self.source_text.replace_range(block_range, "");
        self.reparse();

        if to_slide_idx >= self.slides.len() {
            return false;
        }

        let insert_pos = self.get_slide_content_insert_offset(to_slide_idx);
        let to_insert = format!("\n  {block_text}\n");
        self.source_text.insert_str(insert_pos, &to_insert);
        self.reparse();
        true
    }

    /// Insert a new slide at the specified index position
    pub fn insert_slide_at(
        &mut self,
        target_idx: usize,
    ) {
        if target_idx >= self.slides.len() {
            self.add_new_slide();
            return;
        }
        let new_num = target_idx + 1;
        let has_macros = self.slides.iter().any(|s| {
            self.source_text
                .get(s.range.clone())
                .is_some_and(|t| t.contains("#slide(") || t.contains("#title-slide("))
        });
        let insert_pos = self.slides[target_idx].range.start;
        let template = if has_macros {
            format!("#slide(title: \"Slide {new_num}\")[\n  - Key point or takeaway\n]\n\n")
        } else {
            format!("= Slide {new_num}\n\n- Key point or takeaway\n\n#pagebreak()\n\n")
        };
        self.source_text.insert_str(insert_pos, &template);
        self.reparse();
    }

    /// Duplicate a slide at the specified index
    pub fn duplicate_slide(
        &mut self,
        idx: usize,
    ) {
        if idx >= self.slides.len() {
            return;
        }
        let has_macros = self.slides.iter().any(|s| {
            self.source_text
                .get(s.range.clone())
                .is_some_and(|t| t.contains("#slide(") || t.contains("#title-slide("))
        });
        let slide_range = self.slides[idx].range.clone();
        if let Some(text) = self.source_text.get(slide_range.clone()) {
            let slide_text = text.trim().to_string();
            let insert_pos = slide_range.end;
            let dup = if has_macros {
                format!("\n\n{slide_text}\n")
            } else {
                format!("\n\n#pagebreak()\n\n{slide_text}\n")
            };
            self.source_text.insert_str(insert_pos, &dup);
            self.reparse();
        }
    }

    /// Get the vertical spacing in points immediately following content block at `block_idx` on `slide_idx`.
    #[must_use]
    pub fn get_block_spacing_pt(
        &self,
        slide_idx: usize,
        block_idx: usize,
    ) -> Option<f32> {
        let slide = self.slides.get(slide_idx)?;
        let content_blocks: Vec<&WysiwygBlock> = slide
            .blocks
            .iter()
            .filter(|b| b.is_content_element())
            .collect();
        let curr_block = content_blocks.get(block_idx)?;
        let curr_end = curr_block.range().end;
        let next_start = if block_idx + 1 < content_blocks.len() {
            content_blocks[block_idx + 1].range().start
        } else {
            slide.range.end
        };
        if curr_end <= next_start && next_start <= self.source_text.len() {
            let gap = &self.source_text[curr_end..next_start];
            extract_v_spacing_points(gap)
        } else {
            None
        }
    }
}

/// Convert a Typst length string (e.g. "12pt", "1.5em", "0.5cm", "10mm") into points (pt)
#[must_use]
pub fn parse_length_to_pt(val: &str) -> Option<f32> {
    let s = val.trim();
    if let Some(num) = s.strip_suffix("pt") {
        num.trim().parse::<f32>().ok()
    } else if let Some(num) = s.strip_suffix("em") {
        num.trim().parse::<f32>().ok().map(|em| em * 14.0)
    } else if let Some(num) = s.strip_suffix("cm") {
        num.trim().parse::<f32>().ok().map(|cm| cm * 28.346_457)
    } else if let Some(num) = s.strip_suffix("mm") {
        num.trim().parse::<f32>().ok().map(|mm| mm * 2.834_646)
    } else if let Some(num) = s.strip_suffix("in") {
        num.trim().parse::<f32>().ok().map(|inch| inch * 72.0)
    } else {
        s.parse::<f32>().ok()
    }
}

/// Extract points from any `#v(...)` found in a text slice
#[must_use]
pub fn extract_v_spacing_points(gap: &str) -> Option<f32> {
    let start_idx = gap.find("#v(")?;
    let end_idx = gap[start_idx..].find(')')?;
    let inner = gap[start_idx + 3..start_idx + end_idx].trim();
    parse_length_to_pt(inner)
}


/// Helper to extract string value of a named argument from an argument list or raw func call
#[must_use]
pub fn extract_arg_str(
    args: &str,
    key: &str,
) -> Option<String> {
    let pattern = format!("{key}:");
    let idx = args.find(&pattern)?;
    let after = args[idx + pattern.len()..].trim_start();
    if let Some(stripped) = after.strip_prefix('"') {
        let end_quote = stripped.find('"')?;
        Some(stripped[..end_quote].to_string())
    } else {
        let end = after.find([',', '\n', ')']).unwrap_or(after.len());
        let val = after[..end].trim().trim_matches('"');
        if val.is_empty() || val == "none" {
            None
        } else {
            Some(val.to_string())
        }
    }
}

/// Helper to extract transition name from raw slide text
#[must_use]
pub fn extract_transition_from_raw(raw: &str) -> Option<String> {
    // 1. Check named transition: "..." in slide macro or func
    if let Some(val) = extract_arg_str(raw, "transition") {
        let clean = val.trim().trim_matches('"');
        if !clean.is_empty() && clean != "none" && clean != "cut" {
            return Some(clean.to_string());
        }
    }
    // 2. Check link("transition:...") or link("transition:name")
    if let Some(pos) = raw.find("transition:") {
        let after = &raw[pos + 11..].trim_start();
        if let Some(stripped) = after.strip_prefix('"') {
            if let Some(end) = stripped.find('"') {
                let name = stripped[..end].trim();
                if !name.is_empty() && name != "none" && name != "cut" {
                    return Some(name.to_string());
                }
            }
        } else {
            let end = after
                .find(['"', ')', ']', ',', '\n', ' '])
                .unwrap_or(after.len());
            let name = after[..end].trim().trim_matches('"');
            if !name.is_empty() && name != "none" && name != "cut" {
                return Some(name.to_string());
            }
        }
    }
    None
}

/// Helper to find matching closing parenthesis for function calls
#[must_use]
pub fn find_matching_paren(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_quote = false;
    for (idx, b) in s.bytes().enumerate() {
        if b == b'"' {
            in_quote = !in_quote;
        } else if !in_quote {
            if b == b'(' {
                depth += 1;
            } else if b == b')' {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(idx);
                }
            }
        }
    }
    None
}

/// Update or remove a slide transition for a given slide range in the source text.
#[must_use]
pub fn update_slide_transition(
    source: &str,
    slide_range: Range<usize>,
    new_transition: Option<&str>,
) -> String {
    if slide_range.start > source.len()
        || slide_range.end > source.len()
        || slide_range.start > slide_range.end
    {
        return source.to_string();
    }
    let slide_chunk = &source[slide_range.clone()];
    let clean_new = new_transition
        .map(str::trim)
        .filter(|t| !t.is_empty() && *t != "cut" && *t != "none");

    // Case A: Slide has #slide(...) or slide(...)
    if let Some(slide_idx) = slide_chunk
        .find("#slide(")
        .or_else(|| slide_chunk.find("slide("))
    {
        let after_slide = &slide_chunk[slide_idx..];
        if let Some(paren_close) = find_matching_paren(after_slide) {
            let header = &after_slide[..=paren_close];
            let mut new_header = header.to_string();

            if let Some(t_idx) = new_header.find("transition:") {
                // Already has transition argument: replace or remove it
                let after_t = &new_header[t_idx + 11..];
                // Find end of this argument: comma or closing paren
                let arg_len = after_t.find([',', ')']).unwrap_or(after_t.len());
                let full_arg_len = 11 + arg_len;
                if let Some(trans) = clean_new {
                    new_header.replace_range(
                        t_idx..t_idx + full_arg_len,
                        &format!("transition: \"{trans}\""),
                    );
                } else {
                    // Remove transition argument cleanly
                    let mut remove_start = t_idx;
                    let mut remove_end = t_idx + full_arg_len;
                    if new_header[remove_end..].starts_with(',') {
                        remove_end += 1;
                        while remove_end < new_header.len()
                            && new_header.as_bytes()[remove_end].is_ascii_whitespace()
                        {
                            remove_end += 1;
                        }
                    } else if remove_start > 0
                        && new_header[..remove_start].trim_end().ends_with(',')
                    {
                        let trimmed_len = new_header[..remove_start].trim_end().len();
                        remove_start = trimmed_len.saturating_sub(1);
                    }
                    new_header.replace_range(remove_start..remove_end, "");
                }
            } else if let Some(trans) = clean_new {
                // Add transition argument into the header
                let insert_pos = new_header.find('(').map(|p| p + 1).unwrap_or(1);
                let inner = new_header[insert_pos..new_header.len() - 1].trim();
                if inner.is_empty() {
                    new_header =
                        format!("{}(transition: \"{trans}\")", &new_header[..insert_pos - 1]);
                } else {
                    new_header.insert_str(insert_pos, &format!("transition: \"{trans}\", "));
                }
            }

            if new_header != header {
                let mut updated_slide = slide_chunk.to_string();
                updated_slide.replace_range(slide_idx..slide_idx + header.len(), &new_header);
                let mut full = source.to_string();
                full.replace_range(slide_range, &updated_slide);
                return full;
            }
        }
    }

    // Case B: Non-macro slide or slide with #link("transition:...")
    let mut updated_slide = slide_chunk.to_string();
    if let Some(link_pos) = updated_slide
        .find("#link(\"transition:")
        .or_else(|| updated_slide.find("link(\"transition:"))
    {
        // Find closing bracket
        if let Some(end_bracket) = updated_slide[link_pos..].find(']') {
            let full_link_len = end_bracket + 1;
            let absorb_nl = if updated_slide[link_pos + full_link_len..].starts_with('\n') {
                1
            } else {
                0
            };
            if let Some(trans) = clean_new {
                let new_link = format!("#link(\"transition:{trans}\")[]");
                updated_slide.replace_range(link_pos..link_pos + full_link_len, &new_link);
            } else {
                updated_slide.replace_range(link_pos..link_pos + full_link_len + absorb_nl, "");
            }
            let mut full = source.to_string();
            full.replace_range(slide_range, &updated_slide);
            return full;
        }
    }

    // Case C: Slide has no transition link and no #slide macro, but user wants to set a transition
    if let Some(trans) = clean_new {
        let tag = format!("#link(\"transition:{trans}\")[]\n");
        let insert_offset = if let Some(pb_pos) = updated_slide.find("#pagebreak()") {
            pb_pos
                + 12
                + if updated_slide[pb_pos + 12..].starts_with('\n') {
                    1
                } else {
                    0
                }
        } else {
            0
        };
        updated_slide.insert_str(insert_offset, &tag);
        let mut full = source.to_string();
        full.replace_range(slide_range, &updated_slide);
        return full;
    }

    source.to_string()
}

/// Update transition across all slides in the document.
#[must_use]
pub fn update_all_slides_transition(
    source: &str,
    engine: &TypstDocumentEngine,
    new_transition: Option<&str>,
) -> String {
    let mut updated = source.to_string();
    let mut slide_ranges: Vec<Range<usize>> =
        engine.slides.iter().map(|s| s.range.clone()).collect();
    slide_ranges.reverse();
    for range in slide_ranges {
        updated = update_slide_transition(&updated, range, new_transition);
    }
    updated
}

/// Helper to test if a function name indicates a slide macro
fn is_slide_macro(callee: &str) -> bool {
    let c = callee.trim();
    c == "slide"
        || c == "title-slide"
        || c == "centered-slide"
        || c == "focus-slide"
        || c.ends_with("-slide")
        || c.starts_with("slide-")
}

/// Check if a node corresponds to an explicit pagebreak separator
fn is_slide_boundary(
    node: &LinkedNode,
    raw_text: &str,
) -> bool {
    let trimmed = raw_text.trim();
    if trimmed == "#pagebreak()"
        || trimmed.starts_with("#pagebreak(")
        || trimmed == "pagebreak()"
        || trimmed.starts_with("pagebreak(")
        || trimmed == "---"
    {
        return true;
    }

    if let Some(fc) = node.get().cast::<ast::FuncCall>() {
        let callee = fc.callee().to_untyped().full_text();
        let callee = callee.as_str().trim();
        if callee == "pagebreak" {
            return true;
        }
    }

    false
}

/// Helper to check if a function call is a transparent layout container
fn is_layout_container(callee: &str) -> bool {
    let c = callee.trim();
    c == "align" || c == "code-window" || c == "step" || c == "cols"
}

/// Helper to check if a function call is an inline expression that should not break paragraphs
fn is_inline_func_call(callee: &str) -> bool {
    let c = callee.trim();
    matches!(
        c,
        "link"
            | "text"
            | "strong"
            | "emph"
            | "sub"
            | "super"
            | "underline"
            | "strike"
            | "highlight"
            | "badge"
            | "label"
            | "ref"
            | "emoji"
            | "symbol"
    )
}

fn extract_first_int_arg(args: &str) -> Option<usize> {
    let trimmed = args.trim().trim_start_matches('(').trim_end_matches(')');
    let first = trimmed.split(',').next()?.trim();
    first.parse::<usize>().ok()
}

/// Extract semantic presentation blocks from an iterator of LinkedNodes,
/// preserving full code syntax, grouping paragraphs across tokens, and unpacking layout containers.
fn extract_semantic_blocks_from_nodes<'a, I>(
    nodes: I,
    source_text: &str,
    counter: &mut usize,
    blocks: &mut Vec<WysiwygBlock>,
    element_transitions: &mut std::collections::HashMap<String, ElementTransition>,
) where
    I: IntoIterator<Item = LinkedNode<'a>>,
{
    let mut pending_inline_start: Option<usize> = None;
    let mut pending_inline_end: Option<usize> = None;
    let mut pending_inline_text = String::new();
    let mut last_hash_start: Option<usize> = None;

    let flush_pending_paragraph =
        |pending_start: &mut Option<usize>,
         pending_end: &mut Option<usize>,
         pending_text: &mut String,
         counter: &mut usize,
         blocks: &mut Vec<WysiwygBlock>| {
            if let Some(start) = pending_start.take() {
                let end = pending_end.take().unwrap_or(start);
                if start < end && end <= source_text.len() {
                    let raw = source_text[start..end].to_string();
                    let trimmed = raw.trim();
                    if !trimmed.is_empty() {
                        *counter += 1;
                        let id = format!("block-{}", *counter);
                        let display_text = if !pending_text.trim().is_empty() {
                            pending_text.trim().to_string()
                        } else {
                            trimmed.to_string()
                        };
                        blocks.push(WysiwygBlock::Paragraph {
                            id,
                            text: display_text,
                            range: start..end,
                            raw,
                        });
                    }
                }
                pending_text.clear();
            }
        };

    for child in nodes {
        let kind = child.kind();

        // 1. Parbreaks flush pending paragraphs
        if kind == SyntaxKind::Parbreak {
            flush_pending_paragraph(
                &mut pending_inline_start,
                &mut pending_inline_end,
                &mut pending_inline_text,
                counter,
                blocks,
            );
            continue;
        }

        // 1b. Comments flush pending paragraphs and are parsed as SpeakerNote or Comment
        if kind == SyntaxKind::LineComment || kind == SyntaxKind::BlockComment {
            flush_pending_paragraph(
                &mut pending_inline_start,
                &mut pending_inline_end,
                &mut pending_inline_text,
                counter,
                blocks,
            );

            let range = child.range();
            let raw = source_text.get(range.clone()).unwrap_or("").to_string();
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                *counter += 1;
                let id = format!("block-{}", *counter);
                blocks.push(classify_comment_node(kind, id, range, raw));
            }
            continue;
        }

        // 2. Hash token notes the start of an expression like #call(...) or #let
        if kind == SyntaxKind::Hash {
            if last_hash_start.is_none() {
                last_hash_start = Some(child.range().start);
            }
            continue;
        }

        // 3. Skip delimiter tokens
        if kind == SyntaxKind::LeftBracket
            || kind == SyntaxKind::RightBracket
            || kind == SyntaxKind::LeftBrace
            || kind == SyntaxKind::RightBrace
            || kind == SyntaxKind::Comma
            || kind == SyntaxKind::Semicolon
        {
            continue;
        }

        // 4. Pure whitespace
        if kind == SyntaxKind::Space {
            if pending_inline_start.is_some() {
                pending_inline_end = Some(child.range().end);
                pending_inline_text.push(' ');
            }
            continue;
        }

        if kind == SyntaxKind::Linebreak {
            if pending_inline_start.is_some() {
                pending_inline_end = Some(child.range().end);
                pending_inline_text.push('\n');
            }
            continue;
        }

        // 5. Container unpack: ONLY pure layout containers (Markup, ContentBlock, #align, #grid, #cols, #block)
        if kind == SyntaxKind::Markup || kind == SyntaxKind::ContentBlock {
            flush_pending_paragraph(
                &mut pending_inline_start,
                &mut pending_inline_end,
                &mut pending_inline_text,
                counter,
                blocks,
            );
            extract_semantic_blocks_from_nodes(
                child.children(),
                source_text,
                counter,
                blocks,
                element_transitions,
            );
            continue;
        }

        if kind == SyntaxKind::FuncCall {
            let callee_name = if let Some(fc) = child.get().cast::<ast::FuncCall>() {
                fc.callee()
                    .to_untyped()
                    .full_text()
                    .as_str()
                    .trim()
                    .to_string()
            } else {
                String::new()
            };

            // Skip layout spacer trivia
            if callee_name == "v" || callee_name == "h" {
                last_hash_start = None;
                continue;
            }

            if is_layout_container(&callee_name) {
                let cbs = find_content_blocks_in_funccall(&child);
                if !cbs.is_empty() {
                    flush_pending_paragraph(
                        &mut pending_inline_start,
                        &mut pending_inline_end,
                        &mut pending_inline_text,
                        counter,
                        blocks,
                    );
                    let before = blocks.len();
                    for cb in cbs {
                        extract_semantic_blocks_from_nodes(
                            cb.children(),
                            source_text,
                            counter,
                            blocks,
                            element_transitions,
                        );
                    }
                    if callee_name == "step" {
                        let step_args = child
                            .children()
                            .find(|c| c.kind() == SyntaxKind::Args)
                            .and_then(|a| source_text.get(a.range()))
                            .unwrap_or("");
                        let order = extract_first_int_arg(step_args).unwrap_or(1);
                        let effect = extract_arg_str(step_args, "effect")
                            .unwrap_or_else(|| "fade-in".to_string());
                        let step_range =
                            last_hash_start.unwrap_or(child.range().start)..child.range().end;
                        for blk in &blocks[before..] {
                            element_transitions.insert(
                                blk.id().to_string(),
                                ElementTransition {
                                    order,
                                    effect: effect.clone(),
                                    wrapper_range: step_range.clone(),
                                },
                            );
                        }
                    }
                    if blocks.len() > before {
                        continue;
                    }
                }
            }

            // If we are currently inside an inline paragraph and this is an inline function (like #link, #text, #badge),
            // keep it inside the paragraph instead of fracturing into separate raw-code blocks!
            if pending_inline_start.is_some() && is_inline_func_call(&callee_name) {
                let start = last_hash_start.take().unwrap_or(child.range().start);
                let func_str = source_text.get(start..child.range().end).unwrap_or("");
                pending_inline_end = Some(child.range().end);
                pending_inline_text.push_str(func_str);
                continue;
            }
        }

        // 6. Block-level constructs
        let is_block_node = matches!(
            kind,
            SyntaxKind::Heading
                | SyntaxKind::ListItem
                | SyntaxKind::EnumItem
                | SyntaxKind::TermItem
                | SyntaxKind::SetRule
                | SyntaxKind::ShowRule
                | SyntaxKind::LetBinding
                | SyntaxKind::ModuleImport
                | SyntaxKind::ModuleInclude
                | SyntaxKind::FuncCall
                | SyntaxKind::CodeBlock
        ) || (kind == SyntaxKind::Raw
            && child.get().cast::<ast::Raw>().is_some_and(|r| r.block()))
            || (kind == SyntaxKind::Equation
                && child
                    .get()
                    .cast::<ast::Equation>()
                    .is_some_and(|eq| eq.block()));

        if is_block_node {
            flush_pending_paragraph(
                &mut pending_inline_start,
                &mut pending_inline_end,
                &mut pending_inline_text,
                counter,
                blocks,
            );
            let start = last_hash_start.take().unwrap_or(child.range().start);
            let full_range = start..child.range().end;
            let raw = source_text
                .get(full_range.clone())
                .unwrap_or("")
                .to_string();

            if let Some(block) = classify_linked_node(&child, full_range, raw, counter) {
                blocks.push(block);
            }
            continue;
        }

        // 7. Otherwise, it is inline markup (Text, Strong, Emph, Link, Label, Ref, inline math/raw, etc.)
        let child_str = source_text.get(child.range()).unwrap_or("");
        if pending_inline_start.is_none() {
            pending_inline_start = Some(child.range().start);
            pending_inline_text.clear();
        }
        pending_inline_end = Some(child.range().end);
        pending_inline_text.push_str(child_str);
    }

    // Flush any trailing paragraph
    flush_pending_paragraph(
        &mut pending_inline_start,
        &mut pending_inline_end,
        &mut pending_inline_text,
        counter,
        blocks,
    );
}

/// Helper to find ContentBlock arguments inside a FuncCall node
fn find_content_blocks_in_funccall<'a>(fc_node: &'a LinkedNode<'a>) -> Vec<LinkedNode<'a>> {
    let mut cbs = Vec::new();
    for sub in fc_node.children() {
        if sub.kind() == SyntaxKind::Args {
            for arg_child in sub.children() {
                if arg_child.kind() == SyntaxKind::ContentBlock {
                    cbs.push(arg_child);
                }
            }
        } else if sub.kind() == SyntaxKind::ContentBlock {
            cbs.push(sub);
        }
    }
    cbs
}

/// Classify a LineComment or BlockComment node as SpeakerNote or Comment
fn classify_comment_node(
    kind: SyntaxKind,
    id: String,
    range: Range<usize>,
    raw: String,
) -> WysiwygBlock {
    let trimmed = raw.trim();
    let is_speaker_note = trimmed.starts_with("// [note]:")
        || trimmed.starts_with("// [notes]:")
        || trimmed.starts_with("// [note]")
        || trimmed.starts_with("// [notes]")
        || trimmed.starts_with("// Note:")
        || trimmed.starts_with("// note:")
        || trimmed.starts_with("// Notes:")
        || trimmed.starts_with("// notes:")
        || trimmed.starts_with("// Speaker:")
        || trimmed.starts_with("// speaker:")
        || trimmed.starts_with("// [speaker]:")
        || trimmed.starts_with("// [speaker]")
        || (trimmed.starts_with("/* [note]:") && trimmed.ends_with("*/"));

    if is_speaker_note {
        let content = if let Some(rest) = trimmed
            .strip_prefix("// [note]:")
            .or_else(|| trimmed.strip_prefix("// [notes]:"))
            .or_else(|| trimmed.strip_prefix("// [note]"))
            .or_else(|| trimmed.strip_prefix("// [notes]"))
            .or_else(|| trimmed.strip_prefix("// Note:"))
            .or_else(|| trimmed.strip_prefix("// note:"))
            .or_else(|| trimmed.strip_prefix("// Notes:"))
            .or_else(|| trimmed.strip_prefix("// notes:"))
            .or_else(|| trimmed.strip_prefix("// Speaker:"))
            .or_else(|| trimmed.strip_prefix("// speaker:"))
            .or_else(|| trimmed.strip_prefix("// [speaker]:"))
            .or_else(|| trimmed.strip_prefix("// [speaker]"))
        {
            rest.trim().to_string()
        } else if trimmed.starts_with("/* [note]:") && trimmed.ends_with("*/") {
            trimmed
                .trim_start_matches("/* [note]:")
                .trim_end_matches("*/")
                .trim()
                .to_string()
        } else {
            trimmed.to_string()
        };

        WysiwygBlock::SpeakerNote {
            id,
            range,
            raw,
            content,
        }
    } else {
        let mut content = if kind == SyntaxKind::LineComment {
            trimmed.trim_start_matches("//").trim().to_string()
        } else {
            trimmed
                .trim_start_matches("/*")
                .trim_end_matches("*/")
                .trim()
                .to_string()
        };
        if let Some(rest) = content
            .strip_prefix("Comment:")
            .or_else(|| content.strip_prefix("comment:"))
            .or_else(|| content.strip_prefix("[comment]:"))
            .or_else(|| content.strip_prefix("[comment]"))
        {
            content = rest.trim().to_string();
        }
        WysiwygBlock::Comment {
            id,
            range,
            raw,
            content,
            is_block: kind == SyntaxKind::BlockComment,
        }
    }
}

/// Exhaustive AST Classifier: MATCHES ALL 61 VARIANTS OF `typst_syntax::ast::Expr`
///
/// Under `#[deny(clippy::wildcard_enum_match_arm)]`, this is compiler-guaranteed
/// to never use any wildcards and never skip any syntax.
#[deny(clippy::wildcard_enum_match_arm)]
fn classify_linked_node(
    linked: &LinkedNode,
    range: Range<usize>,
    raw: String,
    counter: &mut usize,
) -> Option<WysiwygBlock> {
    let untyped = linked.get();
    let kind = untyped.kind();

    // Line and block comments are classified into Comment or SpeakerNote
    if kind == SyntaxKind::LineComment || kind == SyntaxKind::BlockComment {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        *counter += 1;
        let id = format!("block-{}", *counter);
        return Some(classify_comment_node(kind, id, range, raw));
    }

    // Skip empty spaces, hash tokens, parbreaks, and brackets
    if kind == SyntaxKind::Space
        || kind == SyntaxKind::Hash
        || kind == SyntaxKind::Parbreak
        || kind == SyntaxKind::LeftBracket
        || kind == SyntaxKind::RightBracket
        || kind == SyntaxKind::LeftBrace
        || kind == SyntaxKind::RightBrace
        || kind == SyntaxKind::Comma
        || kind == SyntaxKind::Semicolon
    {
        return None;
    }

    *counter += 1;
    let id = format!("block-{}", *counter);

    // Try casting to typed Expr
    if let Some(expr) = untyped.cast::<ast::Expr>() {
        let block = match expr {
            // 1. Headings (= Title, == Subtitle, etc.)
            | ast::Expr::Heading(h) => {
                let full = h.body().to_untyped().full_text();
                let title = full.as_str().trim();
                let title = if !title.is_empty() {
                    title.to_string()
                } else {
                    raw.trim_start_matches('=').trim().to_string()
                };
                WysiwygBlock::Heading {
                    id,
                    level: h.depth().get(),
                    title,
                    range,
                    raw,
                }
            },

            // 2. Bullet list item (- Item)
            | ast::Expr::ListItem(li) => {
                let full = li.body().to_untyped().full_text();
                let body = full.as_str().trim();
                let body = if !body.is_empty() {
                    body.to_string()
                } else {
                    raw.trim_start_matches('-').trim().to_string()
                };
                WysiwygBlock::ListItem { id, body, range, raw }
            },

            // 3. Numbered enumeration item (+ Item or 1. Item)
            | ast::Expr::EnumItem(ei) => {
                let full = ei.body().to_untyped().full_text();
                let body = full.as_str().trim();
                let body = if !body.is_empty() {
                    body.to_string()
                } else {
                    raw.trim_start_matches('+').trim().to_string()
                };
                WysiwygBlock::EnumItem {
                    id,
                    number: ei.number().map(|n| n as usize),
                    body,
                    range,
                    raw,
                }
            },

            // 4. Term description item (/ Term: Description)
            | ast::Expr::TermItem(ti) => {
                let full_term = ti.term().to_untyped().full_text();
                let term = full_term.as_str().trim();
                let term = if !term.is_empty() {
                    term.to_string()
                } else {
                    raw.strip_prefix('/')
                        .and_then(|s| s.split_once(':'))
                        .map_or("Term", |(t, _)| t.trim())
                        .to_string()
                };
                let full_desc = ti.description().to_untyped().full_text();
                let desc = full_desc.as_str().trim().to_string();
                WysiwygBlock::TermItem {
                    id,
                    term,
                    description: desc,
                    range,
                    raw,
                }
            },

            // 5. Code block or inline raw text (```lang ... ```)
            | ast::Expr::Raw(r) => {
                let language = r.lang().map_or_else(String::new, |l| l.get().to_string());
                let mut code_lines = Vec::new();
                for l in r.lines() {
                    code_lines.push(l.get().to_string());
                }
                let code = if code_lines.is_empty() {
                    raw.clone()
                } else {
                    code_lines.join("\n")
                };
                WysiwygBlock::CodeBlock {
                    id,
                    language,
                    code,
                    range,
                    raw,
                }
            },

            // 6. Mathematical equations ($ ... $)
            | ast::Expr::Equation(eq) => {
                let formula = raw.trim_matches('$').trim().to_string();
                WysiwygBlock::Equation {
                    id,
                    formula,
                    display: eq.block(),
                    range,
                    raw,
                }
            },

            // 7. Math sub-nodes
            | ast::Expr::Math(_)
            | ast::Expr::MathText(_)
            | ast::Expr::MathIdent(_)
            | ast::Expr::MathFieldAccess(_)
            | ast::Expr::MathShorthand(_)
            | ast::Expr::MathAlignPoint(_)
            | ast::Expr::MathCall(_)
            | ast::Expr::MathDelimited(_)
            | ast::Expr::MathAttach(_)
            | ast::Expr::MathPrimes(_)
            | ast::Expr::MathFrac(_)
            | ast::Expr::MathRoot(_) => {
                WysiwygBlock::Equation {
                    id,
                    formula: raw.clone(),
                    display: false,
                    range,
                    raw,
                }
            },

            // 8. Set rule (#set ...)
            | ast::Expr::SetRule(sr) => {
                let full = sr.target().to_untyped().full_text();
                let target = full.as_str().trim();
                let target = if !target.is_empty() {
                    target.to_string()
                } else {
                    "rule".to_string()
                };
                WysiwygBlock::SetRule {
                    id,
                    target,
                    range,
                    raw,
                }
            },

            // 9. Show rule (#show ...)
            | ast::Expr::ShowRule(sr) => {
                let full = sr.transform().to_untyped().full_text();
                let target = full.as_str().trim();
                let target = if !target.is_empty() {
                    Some(target.to_string())
                } else {
                    None
                };
                WysiwygBlock::ShowRule {
                    id,
                    target,
                    range,
                    raw,
                }
            },

            // 10. Let binding (#let ...)
            | ast::Expr::LetBinding(lb) => {
                let name = lb
                    .kind()
                    .bindings()
                    .first()
                    .map_or("var", |i| i.get().as_str())
                    .to_string();
                WysiwygBlock::LetBinding { id, name, range, raw }
            },

            // 11. Module imports and includes (#import ..., #include ...)
            | ast::Expr::ModuleImport(mi) => {
                WysiwygBlock::Module {
                    id,
                    path: mi
                        .source()
                        .to_untyped()
                        .full_text()
                        .as_str()
                        .trim()
                        .to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::ModuleInclude(mi) => {
                WysiwygBlock::Module {
                    id,
                    path: mi
                        .source()
                        .to_untyped()
                        .full_text()
                        .as_str()
                        .trim()
                        .to_string(),
                    range,
                    raw,
                }
            },

            // 12. Function calls & Macros (#align, #grid, #video, etc.)
            | ast::Expr::FuncCall(fc) => {
                let callee = fc
                    .callee()
                    .to_untyped()
                    .full_text()
                    .as_str()
                    .trim()
                    .to_string();
                let args = fc
                    .args()
                    .to_untyped()
                    .full_text()
                    .as_str()
                    .trim()
                    .to_string();
                if callee == "speaker-note" || callee == "speaker_note" {
                    let content = args
                        .trim_matches(|c| c == '(' || c == ')' || c == '[' || c == ']' || c == '"')
                        .trim()
                        .to_string();
                    WysiwygBlock::SpeakerNote {
                        id,
                        range,
                        raw,
                        content,
                    }
                } else if callee == "note" && (args.starts_with('[') || args.starts_with("(\"")) {
                    let content = args
                        .trim_matches(|c| c == '(' || c == ')' || c == '[' || c == ']' || c == '"')
                        .trim()
                        .to_string();
                    WysiwygBlock::SpeakerNote {
                        id,
                        range,
                        raw,
                        content,
                    }
                } else {
                    WysiwygBlock::FuncCall {
                        id,
                        callee,
                        args,
                        range,
                        raw,
                    }
                }
            },

            // 13. Pure code blocks ({ let x = 1; ... })
            | ast::Expr::CodeBlock(cb) => {
                WysiwygBlock::PureCode {
                    id,
                    body: cb.to_untyped().full_text().as_str().trim().to_string(),
                    range,
                    raw,
                }
            },

            // 14. Content blocks ([ ... ])
            | ast::Expr::ContentBlock(cb) => {
                WysiwygBlock::ContentBlock {
                    id,
                    body: cb.to_untyped().full_text().as_str().trim().to_string(),
                    range,
                    raw,
                }
            },

            // 15. Control flow expressions
            | ast::Expr::Conditional(_) => {
                WysiwygBlock::ControlFlow {
                    id,
                    kind: "if-else".to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::WhileLoop(_) => {
                WysiwygBlock::ControlFlow {
                    id,
                    kind: "while".to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::ForLoop(_) => {
                WysiwygBlock::ControlFlow {
                    id,
                    kind: "for".to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::Contextual(_) => {
                WysiwygBlock::ControlFlow {
                    id,
                    kind: "context".to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::Closure(_) => {
                WysiwygBlock::ControlFlow {
                    id,
                    kind: "closure".to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::DestructAssignment(_) => {
                WysiwygBlock::ControlFlow {
                    id,
                    kind: "destruct".to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::LoopBreak(_) => {
                WysiwygBlock::ControlFlow {
                    id,
                    kind: "break".to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::LoopContinue(_) => {
                WysiwygBlock::ControlFlow {
                    id,
                    kind: "continue".to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::FuncReturn(_) => {
                WysiwygBlock::ControlFlow {
                    id,
                    kind: "return".to_string(),
                    range,
                    raw,
                }
            },

            // 16. Inline rich formatting and paragraphs
            | ast::Expr::Text(t) => {
                let text_val = t.get().to_string();
                let trimmed = text_val.trim();
                if trimmed.len() <= 1 && ":/.,;?!".contains(trimmed) {
                    return None;
                }
                WysiwygBlock::Paragraph {
                    id,
                    text: text_val,
                    range,
                    raw,
                }
            },
            | ast::Expr::Strong(st) => {
                WysiwygBlock::Paragraph {
                    id,
                    text: st
                        .body()
                        .to_untyped()
                        .full_text()
                        .as_str()
                        .trim()
                        .to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::Emph(em) => {
                WysiwygBlock::Paragraph {
                    id,
                    text: em
                        .body()
                        .to_untyped()
                        .full_text()
                        .as_str()
                        .trim()
                        .to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::Space(_)
            | ast::Expr::Linebreak(_)
            | ast::Expr::Parbreak(_)
            | ast::Expr::Escape(_)
            | ast::Expr::Shorthand(_)
            | ast::Expr::SmartQuote(_) => {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    return None;
                }
                WysiwygBlock::Paragraph {
                    id,
                    text: trimmed.to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::Link(l) => {
                WysiwygBlock::Paragraph {
                    id,
                    text: l.get().to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::Label(lb) => {
                WysiwygBlock::Paragraph {
                    id,
                    text: format!("<{}>", lb.get()),
                    range,
                    raw,
                }
            },
            | ast::Expr::Ref(r) => {
                WysiwygBlock::Paragraph {
                    id,
                    text: format!("@{}", r.target()),
                    range,
                    raw,
                }
            },

            // 17. Literals and data structures
            | ast::Expr::Ident(i) => {
                WysiwygBlock::Expression {
                    id,
                    kind: "ident".to_string(),
                    range,
                    raw: i.get().to_string(),
                }
            },
            | ast::Expr::None(_) => {
                WysiwygBlock::Expression {
                    id,
                    kind: "none".to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::Auto(_) => {
                WysiwygBlock::Expression {
                    id,
                    kind: "auto".to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::Bool(b) => {
                WysiwygBlock::Expression {
                    id,
                    kind: "bool".to_string(),
                    range,
                    raw: b.get().to_string(),
                }
            },
            | ast::Expr::Int(i) => {
                WysiwygBlock::Expression {
                    id,
                    kind: "int".to_string(),
                    range,
                    raw: i.get().to_string(),
                }
            },
            | ast::Expr::Float(f) => {
                WysiwygBlock::Expression {
                    id,
                    kind: "float".to_string(),
                    range,
                    raw: f.get().to_string(),
                }
            },
            | ast::Expr::Numeric(_) => {
                WysiwygBlock::Expression {
                    id,
                    kind: "numeric".to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::Str(s) => {
                WysiwygBlock::Expression {
                    id,
                    kind: "string".to_string(),
                    range,
                    raw: s.get().to_string(),
                }
            },
            | ast::Expr::Parenthesized(_) => {
                WysiwygBlock::Expression {
                    id,
                    kind: "paren".to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::Array(_) => {
                WysiwygBlock::Expression {
                    id,
                    kind: "array".to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::Dict(_) => {
                WysiwygBlock::Expression {
                    id,
                    kind: "dict".to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::Unary(_) => {
                WysiwygBlock::Expression {
                    id,
                    kind: "unary".to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::Binary(_) => {
                WysiwygBlock::Expression {
                    id,
                    kind: "binary".to_string(),
                    range,
                    raw,
                }
            },
            | ast::Expr::FieldAccess(_) => {
                WysiwygBlock::Expression {
                    id,
                    kind: "field".to_string(),
                    range,
                    raw,
                }
            },
        };
        Some(block)
    } else {
        // Fallback for non-expr trivia or raw markup node
        let trimmed = raw.trim();
        if trimmed.is_empty()
            || trimmed == "#"
            || trimmed == "{"
            || trimmed == "}"
            || trimmed == "["
            || trimmed == "]"
            || trimmed == "("
            || trimmed == ")"
            || trimmed == ","
            || trimmed == ";"
            || trimmed == ":"
        {
            None
        } else {
            Some(WysiwygBlock::Paragraph {
                id,
                text: trimmed.to_string(),
                range,
                raw,
            })
        }
    }
}

/// Instrument a slide chunk with Typst `#link("slide-loc:{line_idx}")[...]` tags
/// so that the compiled vector SVG embeds accurate `<a href="slide-loc:...">` tags.
#[must_use]
pub fn instrument_slide_chunk(chunk: &str) -> String {
    let lines: Vec<&str> = chunk.lines().collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0;
    let mut paren_depth = 0usize;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // 1. Code blocks: collect until closing ```
        if trimmed.starts_with("```") {
            let code_start = i;
            let mut code_lines = vec![line];
            i += 1;
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                code_lines.push(lines[i]);
                i += 1;
            }
            if i < lines.len() {
                code_lines.push(lines[i]);
            }
            let block_str = code_lines.join("\n");
            let indent_len = line.len() - line.trim_start().len();
            let indent = &line[..indent_len];
            out.push(format!(
                "{indent}#link(\"slide-loc:{code_start}\")[\n{block_str}\n{indent}]"
            ));
            i += 1;
            continue;
        }

        // Track parentheses depth across lines to detect code mode vs content mode
        for c in line.chars() {
            match c {
                | '(' => paren_depth += 1,
                | ')' => paren_depth = paren_depth.saturating_sub(1),
                | _ => {},
            }
        }

        // 2. Skip directives, comments, slide wrappers, and layout spacers
        if trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed.starts_with("#import")
            || trimmed.starts_with("#show")
            || trimmed.starts_with("#set")
            || trimmed.starts_with("#v(")
            || trimmed.starts_with("#h(")
            || trimmed.starts_with("#slide(")
            || trimmed.starts_with("#title-slide(")
            || trimmed == "]"
            || trimmed == ")"
            || trimmed == "}"
            || trimmed == "],"
            || trimmed == "),"
        {
            out.push(line.to_string());
            i += 1;
            continue;
        }

        let indent_len = line.len() - line.trim_start().len();
        let indent = &line[..indent_len];

        // 3. In content mode (paren_depth == 0)
        if paren_depth == 0 {
            if trimmed.starts_with('=')
                || trimmed.starts_with("- ")
                || trimmed.starts_with("+ ")
                || (trimmed.starts_with('$') && trimmed.ends_with('$') && trimmed.len() > 1)
                || trimmed.starts_with("#image(")
                || trimmed.starts_with("#table(")
                || trimmed.starts_with("#chart")
                || (trimmed.starts_with("#align(") && trimmed.contains("image("))
                || (!trimmed.starts_with('#')
                    && !trimmed.ends_with(':')
                    && !trimmed.ends_with(',')
                    && !trimmed.ends_with('('))
            {
                out.push(format!("{indent}#link(\"slide-loc:{i}\")[{trimmed}]"));
            } else {
                out.push(line.to_string());
            }
        } else {
            // Inside code mode (e.g. inside grid, cols, or function call argument list)
            if trimmed.starts_with("callout(") || trimmed.starts_with("#callout(") {
                let clean = trimmed.trim_start_matches('#');
                out.push(format!("{indent}link(\"slide-loc:{i}\")[#{clean}]"));
            } else {
                out.push(line.to_string());
            }
        }

        i += 1;
    }

    out.join("\n")
}

/// Instrument a full document source for the specified active slide
#[must_use]
pub fn instrument_source_for_focus(
    full_source: &str,
    active_slide: usize,
    engine: &TypstDocumentEngine,
) -> String {
    if let Some(slide) = engine.slides.get(active_slide)
        && let Some(chunk) = full_source.get(slide.range.clone())
    {
        let mut sorted_blocks: Vec<&WysiwygBlock> = slide
            .blocks
            .iter()
            .filter(|b| b.is_content_element())
            .collect();
        sorted_blocks.sort_by_key(|b| b.range().start);

        let mut non_overlapping: Vec<&WysiwygBlock> = Vec::new();
        let mut last_end = 0;
        for b in sorted_blocks {
            if b.range().start >= last_end {
                last_end = b.range().end;
                non_overlapping.push(b);
            }
        }

        if non_overlapping.is_empty() {
            return full_source.to_string();
        }

        let mut inst_chunk = chunk.to_string();
        non_overlapping.reverse();

        for b in non_overlapping {
            let rel_start = b.range().start.saturating_sub(slide.range.start);
            let rel_end = b.range().end.saturating_sub(slide.range.start);
            if rel_start >= rel_end || rel_end > inst_chunk.len() {
                continue;
            }

            let line_num = full_source[slide.range.start..b.range().start]
                .bytes()
                .filter(|&c| c == b'\n')
                .count()
                + 1;

            let raw = &inst_chunk[rel_start..rel_end];
            let trimmed = raw.trim();
            if trimmed.is_empty()
                || (trimmed.starts_with('"') && trimmed.ends_with('"'))
                || trimmed.starts_with('\'')
            {
                continue;
            }

            // Only wrap inline text elements that can be safely wrapped in inline #link(...) without layout corruption
            // Never wrap code blocks, code windows, callouts, containers, boxes, grids, tables, media, equations, or nested constructs!
            let is_safe_for_link = match b {
                | WysiwygBlock::Heading { .. }
                | WysiwygBlock::ListItem { .. }
                | WysiwygBlock::EnumItem { .. }
                | WysiwygBlock::TermItem { .. } => true,
                | WysiwygBlock::Paragraph { text, .. } => {
                    !text.contains('#')
                        && !text.contains("```")
                        && !text.contains("link(")
                        && !text.contains("step(")
                },
                | _ => false,
            };

            if !is_safe_for_link {
                continue;
            }

            // Skip blocks that already contain link(), step(), or pagebreak() to prevent nested link errors
            if trimmed.contains("link(")
                || trimmed.contains("step(")
                || trimmed.contains("pagebreak(")
            {
                continue;
            }

            let wrapped = format!("#link(\"slide-loc:{line_num}\")[{raw}]");

            inst_chunk.replace_range(rel_start..rel_end, &wrapped);
        }

        let mut res = full_source.to_string();
        res.replace_range(slide.range.clone(), &inst_chunk);
        return res;
    }
    full_source.to_string()
}

/// Wrap a block's source with #step(order, effect: "...")
#[must_use]
pub fn apply_block_transition(
    source: &str,
    block_range: Range<usize>,
    order: usize,
    effect: &str,
) -> String {
    if block_range.start > block_range.end || block_range.end > source.len() {
        return source.to_string();
    }
    let inner = &source[block_range.clone()];
    let wrapped = format!(
        "#step({order}, effect: \"{effect}\")[\n  {}\n]",
        inner.trim()
    );
    let mut out = String::with_capacity(source.len() + wrapped.len());
    out.push_str(&source[..block_range.start]);
    out.push_str(&wrapped);
    out.push_str(&source[block_range.end..]);
    out
}

/// Update an existing #step wrapper with new order and effect
#[must_use]
pub fn update_existing_step_transition(
    source: &str,
    wrapper_range: Range<usize>,
    order: usize,
    effect: &str,
) -> String {
    if wrapper_range.start > wrapper_range.end || wrapper_range.end > source.len() {
        return source.to_string();
    }
    let raw_step = &source[wrapper_range.clone()];
    let inner = if let Some(first_b) = raw_step.find('[')
        && let Some(last_b) = raw_step.rfind(']')
        && first_b < last_b
    {
        raw_step[first_b + 1..last_b].trim()
    } else {
        raw_step.trim()
    };
    let wrapped = format!("#step({order}, effect: \"{effect}\")[\n  {inner}\n]");
    let mut out = String::with_capacity(source.len() + wrapped.len());
    out.push_str(&source[..wrapper_range.start]);
    out.push_str(&wrapped);
    out.push_str(&source[wrapper_range.end..]);
    out
}

/// Remove an existing #step wrapper, restoring original inner content
#[must_use]
pub fn remove_step_transition(
    source: &str,
    wrapper_range: Range<usize>,
) -> String {
    if wrapper_range.start > wrapper_range.end || wrapper_range.end > source.len() {
        return source.to_string();
    }
    let raw_step = &source[wrapper_range.clone()];
    let inner = if let Some(first_b) = raw_step.find('[')
        && let Some(last_b) = raw_step.rfind(']')
        && first_b < last_b
    {
        raw_step[first_b + 1..last_b].trim()
    } else {
        raw_step.trim()
    };
    let mut out = String::with_capacity(source.len());
    out.push_str(&source[..wrapper_range.start]);
    out.push_str(inner);
    out.push_str(&source[wrapper_range.end..]);
    out
}

/// Configuration for presentation header and footer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderFooterConfig {
    pub header_enabled: bool,
    pub header_left: String,
    pub header_right: String,
    pub footer_enabled: bool,
    pub footer_left: String,
    pub footer_right_mode: usize, // 0 = page counter "1 / 1", 1 = custom text, 2 = hidden
    pub footer_right_custom: String,
}

impl Default for HeaderFooterConfig {
    fn default() -> Self {
        Self {
            header_enabled: false,
            header_left: String::new(),
            header_right: String::new(),
            footer_enabled: true,
            footer_left: String::new(),
            footer_right_mode: 0,
            footer_right_custom: String::new(),
        }
    }
}

/// Extract header and footer settings from document Typst source
#[must_use]
pub fn extract_header_footer_from_source(source: &str) -> HeaderFooterConfig {
    let mut config = HeaderFooterConfig::default();

    let Some(show_pos) = source.find("slide-theme.with(") else {
        return config;
    };

    let after_show = &source[show_pos + "slide-theme.with(".len()..];
    let Some(closing_paren) = after_show.find(')') else {
        return config;
    };
    let args_str = &after_show[..closing_paren];

    // Check header
    if let Some(pos) = args_str.find("header:") {
        let after = &args_str[pos + 7..].trim_start();
        if after.starts_with("none") || after.starts_with("\"\"") {
            config.header_enabled = false;
        } else if let Some(first_q) = after.find('"')
            && let Some(second_q) = after[first_q + 1..].find('"')
        {
            config.header_enabled = true;
            config.header_left = after[first_q + 1..first_q + 1 + second_q].to_string();
        }
    }

    // Check header-right
    if let Some(pos) = args_str.find("header-right:") {
        let after = &args_str[pos + 13..].trim_start();
        if let Some(first_q) = after.find('"')
            && let Some(second_q) = after[first_q + 1..].find('"')
        {
            config.header_right = after[first_q + 1..first_q + 1 + second_q].to_string();
        }
    }

    // Check footer
    if let Some(pos) = args_str.find("footer:") {
        let after = &args_str[pos + 7..].trim_start();
        if after.starts_with("none") {
            config.footer_enabled = false;
            config.footer_left.clear();
        } else if after.starts_with("\"\"") {
            config.footer_enabled = true;
            config.footer_left.clear();
        } else if let Some(first_q) = after.find('"')
            && let Some(second_q) = after[first_q + 1..].find('"')
        {
            config.footer_enabled = true;
            config.footer_left = after[first_q + 1..first_q + 1 + second_q].to_string();
        }
    }

    // Check footer-right
    if let Some(pos) = args_str.find("footer-right:") {
        let after = &args_str[pos + 13..].trim_start();
        if after.starts_with("none") || after.starts_with("\"\"") {
            config.footer_right_mode = 2; // hidden
        } else if after.starts_with("auto") {
            config.footer_right_mode = 0; // page number
        } else if let Some(first_q) = after.find('"')
            && let Some(second_q) = after[first_q + 1..].find('"')
        {
            config.footer_right_mode = 1; // custom
            config.footer_right_custom = after[first_q + 1..first_q + 1 + second_q].to_string();
        }
    }

    config
}

/// Update document Typst source with customized header and footer settings
#[must_use]
pub fn update_header_footer_in_source(
    source: &str,
    config: &HeaderFooterConfig,
) -> String {
    let mut header_args = Vec::new();

    if config.header_enabled {
        if !config.header_left.is_empty() {
            header_args.push(format!(
                "header: \"{}\"",
                config.header_left.replace('"', "\\\"")
            ));
        }
        if !config.header_right.is_empty() {
            header_args.push(format!(
                "header-right: \"{}\"",
                config.header_right.replace('"', "\\\"")
            ));
        }
    } else {
        header_args.push("header: none".to_string());
    }

    if config.footer_enabled {
        header_args.push(format!(
            "footer: \"{}\"",
            config.footer_left.replace('"', "\\\"")
        ));
        match config.footer_right_mode {
            | 0 => header_args.push("footer-right: auto".to_string()),
            | 1 => {
                header_args.push(format!(
                    "footer-right: \"{}\"",
                    config.footer_right_custom.replace('"', "\\\"")
                ))
            },
            | 2 => header_args.push("footer-right: none".to_string()),
            | _ => {},
        }
    } else {
        header_args.push("footer: none".to_string());
        header_args.push("footer-right: none".to_string());
    }

    if let Some(show_pos) = source.find("slide-theme.with(") {
        let after_show = &source[show_pos + "slide-theme.with(".len()..];
        if let Some(closing_paren) = after_show.find(')') {
            let full_end = show_pos + "slide-theme.with(".len() + closing_paren;
            let existing_args = &after_show[..closing_paren];

            // Retain existing aspect-ratio, theme, font, etc.
            let mut preserved_args = Vec::new();
            for part in existing_args.split(',') {
                let trimmed = part.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed.starts_with("header:")
                    || trimmed.starts_with("header-right:")
                    || trimmed.starts_with("footer:")
                    || trimmed.starts_with("footer-right:")
                {
                    continue;
                }
                preserved_args.push(trimmed.to_string());
            }

            preserved_args.extend(header_args);
            let new_call = format!("slide-theme.with({})", preserved_args.join(", "));
            let mut out = source.to_string();
            out.replace_range(show_pos..=full_end, &new_call);
            return out;
        }
    }

    // If slide-theme.with not found, insert at beginning
    let new_rule = format!("#show: slide-theme.with({})\n", header_args.join(", "));
    if source.starts_with("#import")
        && let Some(first_nl) = source.find('\n')
    {
        let mut out = source.to_string();
        out.insert_str(first_nl + 1, &new_rule);
        return out;
    }
    format!("{new_rule}{source}")
}

/// Extract any math equation from a text line for hover pre-rendering ($...$)
#[must_use]
pub fn extract_math_formula_from_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with('$') && trimmed.ends_with('$') && trimmed.len() >= 2 {
        return Some(trimmed.to_string());
    }
    // Search for $ ... $ inside line
    let mut in_dollar = false;
    let mut start = 0;
    for (i, c) in line.char_indices() {
        if c == '$' {
            if in_dollar {
                let candidate = &line[start..=i];
                if candidate.trim().len() > 2 {
                    return Some(candidate.trim().to_string());
                }
                in_dollar = false;
            } else {
                in_dollar = true;
                start = i;
            }
        }
    }
    None
}
