//! Interactive Typora-style visual block component.
//!
//! Renders each AST block either as a rich visual element (Typora typography, bullet points,
//! math equations, code boxes) or in-place interactive editor when focused.

use crate::app::Message;
use crate::model::ast_engine::ElementTransition;
use crate::model::ast_engine::WysiwygBlock;
use crate::ui::theme::AppTheme;
use crate::ui::theme::{
    self,
};
use crate::ui::typst_highlighter::TypstHighlightSettings;
use crate::ui::typst_highlighter::TypstHighlighter;
use crate::ui::typst_highlighter::to_typst_format;
use iced::Alignment;
use iced::Background;
use iced::Color;
use iced::Element;
use iced::Length;
use iced::border::Border;
use iced::border::{
    self,
};
use iced::widget::Space;
use iced::widget::button;
use iced::widget::column;
use iced::widget::container;
use iced::widget::image;
use iced::widget::mouse_area;
use iced::widget::rich_text;
use iced::widget::row;
use iced::widget::span;
use iced::widget::text;
use iced::widget::text_editor;
use iced::widget::text_input;
use std::ops::Range;

/// Action kinds for inserting new Typst blocks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertBlockKind {
    Heading1,
    Heading2,
    Heading3,
    Heading4,
    TitleSlide,
    Bullet,
    Numbered,
    Paragraph,
    Table,
    Grid,
    Callout,
    BoxBlock,
    Link,
    Chart,
    CodeBlock,
    Equation,
    Video,
    Audio,
}

impl InsertBlockKind {
    /// Return Typst snippet for this block kind
    #[must_use]
    pub const fn template(&self) -> &'static str {
        match self {
            | Self::Heading1 => "= New Section Title\n",
            | Self::Heading2 => "== Subtitle / Key Topic\n",
            | Self::Heading3 => "=== Section Subheading\n",
            | Self::Heading4 => "==== Detail Topic\n",
            | Self::TitleSlide => {
                "#title-slide(\n  title: \"Presentation Title\",\n  subtitle: \"Subtitle or Tagline\",\n  author: \"Presenter Name\",\n  date: \"2026\",\n)\n"
            },
            | Self::Bullet => "- Key takeaway or bullet point\n",
            | Self::Numbered => "+ Ordered step or procedure\n",
            | Self::Paragraph => "Enter paragraph description here.\n",
            | Self::Table => {
                "#table(\n  columns: (1fr, 1fr),\n  [Header 1], [Header 2],\n  [Item A], [Item B],\n)\n"
            },
            | Self::Grid => {
                "#grid(\n  columns: (1fr, 1fr),\n  gutter: 12pt,\n  [\n    *Left Column*\n    Content for left column.\n  ],\n  [\n    *Right Column*\n    Content for right column.\n  ],\n)\n"
            },
            | Self::Callout => {
                "#callout(title: \"Key Takeaway\")[\n  Important summary callout here.\n]\n"
            },
            | Self::BoxBlock => {
                "#box(stroke: 1pt + rgb(\"3b82f6\"), inset: 8pt, radius: 4pt)[\n  Box container content here.\n]\n"
            },
            | Self::Link => "#link(\"https://cargo-slide.dev\")[Cargo Slide Documentation]\n",
            | Self::Chart => {
                "#chart-bar(\n  title: \"Performance Metrics\",\n  (\"Speed\", 85),\n  (\"Safety\", 95),\n  (\"Simplicity\", 80),\n)\n"
            },
            | Self::CodeBlock => {
                "```rust\nfn main() {\n    println!(\"Hello, Cargo Slide!\");\n}\n```\n"
            },
            | Self::Equation => "$ f(x) = sum_(i=1)^n x_i $\n",
            | Self::Video => "#video(\"assets/demo.mp4\", caption: \"Hardware Video Playback\")\n",
            | Self::Audio => {
                "#audio-player(\"assets/soundtrack.mp3\", title: \"Background Music\")\n"
            },
        }
    }

    /// Short label
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            | Self::Heading1 => "H1",
            | Self::Heading2 => "H2",
            | Self::Heading3 => "H3",
            | Self::Heading4 => "H4",
            | Self::TitleSlide => "Title Slide",
            | Self::Bullet => "List",
            | Self::Numbered => "1. Number",
            | Self::Paragraph => "Text",
            | Self::Table => "Table",
            | Self::Grid => "Grid",
            | Self::Callout => "Callout",
            | Self::BoxBlock => "Box",
            | Self::Link => "Link",
            | Self::Chart => "Chart",
            | Self::CodeBlock => "Code",
            | Self::Equation => "Math",
            | Self::Video => "Video",
            | Self::Audio => "Audio",
        }
    }
}

/// Text formatting actions for active block
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatAction {
    Bold,
    Italic,
    Monospace,
    Equation,
}

/// Helper to parse Typst inline markup into iced Rich text Spans:
/// Supports:
/// - `*bold text*`
/// - `_italic text_`
/// - `` `code` ``
/// - `$math$`
#[must_use]
pub fn render_rich_text<'a>(
    raw_text: &'a str,
    base_size: f32,
    base_color: Color,
    theme: AppTheme,
) -> Element<'a, Message> {
    let mut spans = Vec::new();
    let mut segments = Vec::new();

    parse_typst_segments(raw_text, false, false, &mut segments);

    for (seg_text, bold, italic, code, math) in segments {
        if seg_text.is_empty() {
            continue;
        }
        let (font, color) = if code {
            let code_color = if theme.is_dark() {
                Color::from_rgb(0.95, 0.75, 0.40)
            } else {
                Color::from_rgb(0.70, 0.25, 0.10)
            };
            (iced::Font::MONOSPACE, code_color)
        } else if math {
            (
                iced::Font {
                    style: iced::font::Style::Italic,
                    ..iced::Font::DEFAULT
                },
                theme.accent(),
            )
        } else {
            let weight = if bold {
                iced::font::Weight::Bold
            } else {
                iced::font::Weight::Normal
            };
            let style = if italic {
                iced::font::Style::Italic
            } else {
                iced::font::Style::Normal
            };
            (
                iced::Font {
                    weight,
                    style,
                    ..iced::Font::DEFAULT
                },
                base_color,
            )
        };
        spans.push(span(seg_text).font(font).color(color));
    }

    if spans.is_empty() {
        text(raw_text)
            .size(base_size)
            .wrapping(iced::widget::text::Wrapping::Word)
            .color(base_color)
            .into()
    } else {
        rich_text(spans)
            .size(base_size)
            .wrapping(iced::widget::text::Wrapping::Word)
            .on_link_click(iced::never)
            .into()
    }
}

fn parse_typst_segments<'a>(
    input: &'a str,
    is_bold: bool,
    is_italic: bool,
    out: &mut Vec<(&'a str, bool, bool, bool, bool)>,
) {
    if input.is_empty() {
        return;
    }

    let mut earliest_delim: Option<(usize, char, usize, usize)> = None;

    // Check inline `...` (single backtick, not part of ``` code fences)
    let bytes = input.as_bytes();
    let mut search_from = 0;
    while let Some(rel_pos) = input[search_from..].find('`') {
        let pos = search_from + rel_pos;
        let is_fence =
            (pos > 0 && bytes.get(pos - 1) == Some(&b'`')) || bytes.get(pos + 1) == Some(&b'`');
        if !is_fence {
            let after = &input[pos + 1..];
            let after_bytes = after.as_bytes();
            let mut close_from = 0;
            while let Some(rel_close) = after[close_from..].find('`') {
                let close_pos = close_from + rel_close;
                let close_is_fence = (close_pos > 0
                    && after_bytes.get(close_pos - 1) == Some(&b'`'))
                    || after_bytes.get(close_pos + 1) == Some(&b'`');
                if !close_is_fence {
                    let end_pos = pos + 1 + close_pos;
                    if earliest_delim.is_none() || pos < earliest_delim.unwrap().0 {
                        earliest_delim = Some((pos, '`', pos + 1, end_pos));
                    }
                    break;
                }
                close_from = close_pos + 1;
            }
            break;
        }
        search_from = pos + 1;
    }

    // Check $...$
    if let Some(pos) = input.find('$')
        && let Some(end_rel) = input[pos + 1..].find('$')
    {
        let end_pos = pos + 1 + end_rel;
        if earliest_delim.is_none() || pos < earliest_delim.unwrap().0 {
            earliest_delim = Some((pos, '$', pos + 1, end_pos));
        }
    }

    // Check *...*
    if !is_bold {
        let mut search_from = 0;
        while let Some(rel_pos) = input[search_from..].find('*') {
            let pos = search_from + rel_pos;
            let after = &input[pos + 1..];
            if !after.starts_with(' ')
                && !after.starts_with('\t')
                && !after.starts_with('\n')
                && let Some(end_rel) = after.find('*')
            {
                let end_pos = pos + 1 + end_rel;
                let inner = &input[pos + 1..end_pos];
                if !inner.is_empty()
                    && !inner.ends_with(' ')
                    && !inner.ends_with('\t')
                    && !inner.ends_with('\n')
                {
                    if earliest_delim.is_none() || pos < earliest_delim.unwrap().0 {
                        earliest_delim = Some((pos, '*', pos + 1, end_pos));
                    }
                    break;
                }
            }
            search_from = pos + 1;
        }
    }

    // Check _..._
    if !is_italic {
        let mut search_from = 0;
        while let Some(rel_pos) = input[search_from..].find('_') {
            let pos = search_from + rel_pos;
            let after = &input[pos + 1..];
            if !after.starts_with(' ')
                && !after.starts_with('\t')
                && !after.starts_with('\n')
                && let Some(end_rel) = after.find('_')
            {
                let end_pos = pos + 1 + end_rel;
                let inner = &input[pos + 1..end_pos];
                if !inner.is_empty()
                    && !inner.ends_with(' ')
                    && !inner.ends_with('\t')
                    && !inner.ends_with('\n')
                {
                    if earliest_delim.is_none() || pos < earliest_delim.unwrap().0 {
                        earliest_delim = Some((pos, '_', pos + 1, end_pos));
                    }
                    break;
                }
            }
            search_from = pos + 1;
        }
    }

    if let Some((start_pos, delim, inner_start, inner_end)) = earliest_delim {
        if start_pos > 0 {
            out.push((&input[..start_pos], is_bold, is_italic, false, false));
        }

        let inner_text = &input[inner_start..inner_end];
        match delim {
            | '`' => out.push((inner_text, false, false, true, false)),
            | '$' => out.push((inner_text, false, false, false, true)),
            | '*' => parse_typst_segments(inner_text, true, is_italic, out),
            | '_' => parse_typst_segments(inner_text, is_bold, true, out),
            | _ => out.push((inner_text, is_bold, is_italic, false, false)),
        }

        let next_start = inner_end + 1;
        if next_start < input.len() {
            parse_typst_segments(&input[next_start..], is_bold, is_italic, out);
        }
    } else {
        out.push((input, is_bold, is_italic, false, false));
    }
}

/// Render a single WYSIWYG block (visual or inline active edit)
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn view_block<'a>(
    theme: AppTheme,
    slide_idx: usize,
    block_idx: usize,
    total_blocks: usize,
    block: &'a WysiwygBlock,
    is_active: bool,
    is_raw_code: bool,
    active_content: Option<&'a text_editor::Content>,
    scale: f32,
    equation_image: Option<&'a image::Handle>,
    code_image: Option<&'a image::Handle>,
    is_dragging: bool,
    spacing_pt: Option<f32>,
    transition: Option<&'a ElementTransition>,
) -> Element<'a, Message> {
    let is_complex = match block {
        | WysiwygBlock::FuncCall { callee, .. } => {
            matches!(
                callee.as_str(),
                "title-slide"
                    | "table"
                    | "grid"
                    | "cols"
                    | "columns"
                    | "chart"
                    | "chart-bar"
                    | "chart-pie"
                    | "chart-line"
                    | "plot"
                    | "callout"
                    | "note"
                    | "tip"
                    | "warning"
                    | "info"
                    | "alert"
                    | "quote"
                    | "box"
                    | "block"
                    | "rect"
                    | "badge"
                    | "pill"
                    | "video"
                    | "audio"
                    | "audio-player"
            )
        },
        | _ => false,
    };

    if is_complex {
        if is_raw_code {
            view_active_block_editor(
                theme,
                slide_idx,
                block_idx,
                block,
                active_content,
                scale,
                transition,
            )
        } else {
            view_visual_block(
                theme,
                slide_idx,
                block_idx,
                total_blocks,
                block,
                is_active,
                scale,
                equation_image,
                code_image,
                is_dragging,
                spacing_pt,
                transition,
            )
        }
    } else if is_active {
        view_active_block_editor(
            theme,
            slide_idx,
            block_idx,
            block,
            active_content,
            scale,
            transition,
        )
    } else {
        view_visual_block(
            theme,
            slide_idx,
            block_idx,
            total_blocks,
            block,
            is_active,
            scale,
            equation_image,
            code_image,
            is_dragging,
            spacing_pt,
            transition,
        )
    }
}

/// Render active in-place block editor (Typora-style seamless inline text editing right where the block is)
#[allow(clippy::too_many_arguments)]
pub fn view_active_block_editor<'a>(
    theme: AppTheme,
    slide_idx: usize,
    block_idx: usize,
    block: &'a WysiwygBlock,
    active_content: Option<&'a text_editor::Content>,
    scale: f32,
    transition: Option<&'a ElementTransition>,
) -> Element<'a, Message> {
    let (font_size, top_margin, bottom_margin) = match block {
        | WysiwygBlock::Heading { level, .. } => {
            match level {
                | 1 => ((24.0 * scale).max(18.0), 8.0 * scale, 6.0 * scale),
                | 2 => ((19.0 * scale).max(15.0), 6.0 * scale, 4.0 * scale),
                | 3 => ((15.0 * scale).max(13.0), 4.0 * scale, 3.0 * scale),
                | _ => ((13.0 * scale).max(11.0), 2.0 * scale, 2.0 * scale),
            }
        },
        | WysiwygBlock::ListItem { .. } | WysiwygBlock::EnumItem { .. } => {
            ((14.0 * scale).max(12.0), 2.0 * scale, 2.0 * scale)
        },
        | WysiwygBlock::CodeBlock { .. } => ((12.0 * scale).max(10.0), 4.0 * scale, 4.0 * scale),
        | WysiwygBlock::Equation { .. } => ((14.0 * scale).max(12.0), 4.0 * scale, 4.0 * scale),
        | WysiwygBlock::Paragraph { .. } => ((14.0 * scale).max(12.0), 2.0 * scale, 2.0 * scale),
        | _ => ((13.0 * scale).max(11.0), 2.0 * scale, 2.0 * scale),
    };

    let editor_widget: Element<'a, Message> = if let Some(content) = active_content {
        text_editor(content)
            .id(iced::widget::Id::new("active_block_editor"))
            .placeholder("Typst markup...")
            .wrapping(iced::widget::text::Wrapping::Word)
            .size(font_size)
            .height(Length::Shrink)
            .min_height(font_size * 1.35)
            .padding([2.0, 4.0])
            .style(move |_t, _s| {
                text_editor::Style {
                    background: Background::Color(if theme.is_dark() {
                        Color::from_rgba(0.10, 0.13, 0.20, 0.95)
                    } else {
                        Color::from_rgba(0.97, 0.98, 1.0, 0.98)
                    }),
                    border: Border {
                        color: theme.accent(),
                        width: 1.0,
                        radius: border::Radius::from(4.0),
                    },
                    placeholder: theme.text_muted(),
                    value: theme.text_primary(),
                    selection: Color::from_rgba(0.2, 0.5, 0.9, 0.25),
                }
            })
            .highlight_with::<TypstHighlighter>(
                TypstHighlightSettings {
                    is_dark: theme.is_dark(),
                },
                to_typst_format,
            )
            .on_action(Message::ActiveBlockAction)
            .into()
    } else {
        container(text("Loading...").size(font_size).color(theme.text_muted())).into()
    };

    let btn_step = button(
        text(if let Some(tr) = transition {
            format!("⚡ Step {} • {}", tr.order, tr.effect)
        } else {
            "⚡ Step Anim".to_string()
        })
        .size(9),
    )
    .padding([1, 6])
    .style(move |_t, _s| {
        if transition.is_some() {
            theme::primary_button_style(theme)
        } else {
            theme::subtle_button_style(theme, false)
        }
    })
    .on_press(Message::OpenElementTransitionModal { slide_idx, block_idx });

    let top_bar = row![
        text(format!("</> Code: {}", block.chip_info().0))
            .size(10)
            .color(theme.accent()),
        Space::new().width(Length::Fill),
        btn_step,
        button(text("Visual").size(9))
            .padding([1, 5])
            .style(move |_t, _s| theme::subtle_button_style(theme, false))
            .on_press(Message::ToggleBlockRawCode(block.id().to_string())),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .padding([2, 4]);

    column![
        Space::new().height(Length::Fixed(top_margin)),
        top_bar,
        editor_widget,
        Space::new().height(Length::Fixed(bottom_margin)),
    ]
    .into()
}

/// Extract all top-level bracketed contents `[...]` from an arguments string.
/// Correctly handles nested brackets `[[nested]]` or `[*bold*]`, and ignores
/// brackets inside ```code fences``` and string quotes.
#[must_use]
pub fn extract_bracket_contents(args: &str) -> Vec<&str> {
    let mut results = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut in_fence = false;
    let mut in_quote = false;
    let mut escape = false;

    let bytes = args.as_bytes();
    let len = bytes.len();
    let mut idx = 0;

    while idx < len {
        let b = bytes[idx];

        if escape {
            escape = false;
            idx += 1;
            continue;
        }

        if b == b'\\' {
            escape = true;
            idx += 1;
            continue;
        }

        // Check for 3+ backticks (code fence)
        if b == b'`' && idx + 2 < len && bytes[idx + 1] == b'`' && bytes[idx + 2] == b'`' {
            in_fence = !in_fence;
            idx += 3;
            while idx < len && bytes[idx] == b'`' {
                idx += 1;
            }
            continue;
        }

        if in_fence {
            idx += 1;
            continue;
        }

        if b == b'"' {
            in_quote = !in_quote;
            idx += 1;
            continue;
        }

        if in_quote {
            idx += 1;
            continue;
        }

        if b == b'[' {
            if depth == 0 {
                start = idx + 1;
            }
            depth += 1;
        } else if b == b']' && depth > 0 {
            depth -= 1;
            if depth == 0 {
                let trimmed = args[start..idx].trim();
                results.push(trimmed);
            }
        }

        idx += 1;
    }
    results
}

/// Helper to split comma-separated items outside of parens/brackets/braces
#[must_use]
pub fn split_args(s: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut start = 0usize;

    for (byte_idx, c) in s.char_indices() {
        match c {
            | '(' => depth_paren += 1,
            | ')' => depth_paren = depth_paren.saturating_sub(1),
            | '[' => depth_bracket += 1,
            | ']' => depth_bracket = depth_bracket.saturating_sub(1),
            | '{' => depth_brace += 1,
            | '}' => depth_brace = depth_brace.saturating_sub(1),
            | ',' if depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 => {
                let trimmed = s[start..byte_idx].trim();
                if !trimmed.is_empty() {
                    items.push(trimmed);
                }
                start = byte_idx + 1;
            },
            | _ => {},
        }
    }
    let trimmed = s[start..].trim();
    if !trimmed.is_empty() {
        items.push(trimmed);
    }
    items
}

#[must_use]
pub fn is_named_arg(part: &str) -> bool {
    if let Some((name, _)) = part.split_once(':') {
        let name = name.trim();
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    } else {
        false
    }
}

/// Extract cells from `#table(...)` arguments
#[must_use]
pub fn extract_table_cells(args: &str) -> Vec<&str> {
    let bracketed = extract_bracket_contents(args);
    if !bracketed.is_empty() {
        return bracketed;
    }

    let mut cells = Vec::new();
    let inner = args
        .trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .trim();
    for part in split_args(inner) {
        let part = part.trim();
        if part.is_empty() || is_named_arg(part) {
            continue;
        }
        let clean = part.trim_matches('"').trim_matches('\'').trim();
        if !clean.is_empty() {
            cells.push(clean);
        }
    }
    cells
}

/// Top-level column item parsed from `#cols(...)` or `#grid(...)`
#[derive(Debug, Clone, PartialEq)]
pub struct GridColItem<'a> {
    pub raw: &'a str,
    pub display_text: &'a str,
    pub range_in_raw: Range<usize>,
    pub portion: u16,
}

/// Extract top-level column items from `#cols(...)` or `#grid(...)`
#[must_use]
pub fn extract_column_items<'a>(raw: &'a str) -> Vec<GridColItem<'a>> {
    let mut items = Vec::new();
    let trimmed = raw.trim();
    let raw_offset = raw.find(trimmed).unwrap_or(0);

    // 1. Find the primary function call parentheses if present
    let open_paren = trimmed.find('(');
    let close_paren = if let Some(open) = open_paren {
        let mut depth = 0usize;
        let mut found = None;
        for (idx, c) in trimmed[open..].char_indices() {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    found = Some(open + idx);
                    break;
                }
            }
        }
        found
    } else {
        None
    };

    // 2. Extract column ratios if present (e.g. ratio: (1fr, 2fr, 1fr) or columns: (1fr, 1fr, 1fr))
    let mut portions: Vec<u16> = Vec::new();
    if let (Some(open), Some(close)) = (open_paren, close_paren) {
        let inner_call = &trimmed[open + 1..close];
        for key in &["ratio:", "columns:"] {
            if let Some(pos) = inner_call.find(key) {
                let after = &inner_call[pos + key.len()..].trim_start();
                if after.starts_with('(')
                    && let Some(end) = after.find(')')
                {
                    for part in after[1..end].split(',') {
                        let p_trimmed = part.trim();
                        if let Some(first_non_digit) = p_trimmed.find(|c: char| !c.is_ascii_digit())
                            && let Ok(val) = p_trimmed[..first_non_digit].parse::<u16>()
                        {
                            portions.push(val.max(1));
                            continue;
                        }
                        if !p_trimmed.is_empty() {
                            portions.push(1);
                        }
                    }
                    if !portions.is_empty() {
                        break;
                    }
                }
            }
        }
    }

    // 3. Check if trailing content blocks follow the closing parenthesis:
    // e.g. #cols(2)[\n Left\n][\n Right\n] or #cols()[...][...]
    if let Some(close) = close_paren {
        let after_call = &trimmed[close + 1..];
        if after_call.trim_start().starts_with('[') {
            let offset_in_trimmed = close + 1;
            let spans = extract_bracket_spans(after_call);
            for (col_idx, span) in spans.into_iter().enumerate() {
                let abs_start = raw_offset + offset_in_trimmed + span.start;
                let abs_end = raw_offset + offset_in_trimmed + span.end;
                let item_raw = &raw[abs_start..abs_end];
                let display_text = if item_raw.starts_with('[')
                    && item_raw.ends_with(']')
                    && item_raw.len() >= 2
                {
                    item_raw[1..item_raw.len() - 1].trim()
                } else {
                    item_raw.trim()
                };
                let portion = portions.get(col_idx).copied().unwrap_or(1);
                items.push(GridColItem {
                    raw: item_raw,
                    display_text,
                    range_in_raw: abs_start..abs_end,
                    portion,
                });
            }
            if !items.is_empty() {
                return items;
            }
        }
    }

    // 4. Otherwise, the column blocks are arguments inside the parentheses
    let (arg_start, arg_end) = if let (Some(open), Some(close)) = (open_paren, close_paren) {
        (open + 1, close)
    } else if let Some(last_paren) = trimmed.rfind(')') {
        if let Some(open) = open_paren {
            (open + 1, last_paren)
        } else {
            (0, trimmed.len())
        }
    } else {
        (0, trimmed.len())
    };

    let inner = &trimmed[arg_start..arg_end];
    let abs_base = raw_offset + arg_start;

    // Split inner by comma outside parentheses, brackets, braces, and quotes
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_quote = false;
    let mut escape = false;
    let mut start = 0usize;

    let mut raw_args = Vec::new();

    for (byte_idx, c) in inner.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' {
            escape = true;
            continue;
        }
        if c == '"' {
            in_quote = !in_quote;
            continue;
        }
        if in_quote {
            continue;
        }

        match c {
            | '(' => depth_paren = depth_paren.saturating_add(1),
            | ')' => depth_paren = depth_paren.saturating_sub(1),
            | '[' => depth_bracket = depth_bracket.saturating_add(1),
            | ']' => depth_bracket = depth_bracket.saturating_sub(1),
            | '{' => depth_brace = depth_brace.saturating_add(1),
            | '}' => depth_brace = depth_brace.saturating_sub(1),
            | ',' if depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 => {
                let slice = &inner[start..byte_idx];
                raw_args.push((abs_base + start, abs_base + byte_idx, slice));
                start = byte_idx + 1;
            },
            | _ => {},
        }
    }

    if start < inner.len() {
        let slice = &inner[start..];
        raw_args.push((abs_base + start, abs_base + inner.len(), slice));
    }

    // Identify positional column arguments (filter out named args like `ratio: ...` or pure integers like `2`)
    let mut col_idx = 0usize;
    for (abs_start, abs_end, arg_str) in raw_args {
        let trimmed_arg = arg_str.trim();
        if trimmed_arg.is_empty() {
            continue;
        }

        // Check if named arg (e.g. `ratio: ...`, `columns: ...`, `gutter: ...`)
        if is_named_arg(trimmed_arg) {
            continue;
        }

        // Check if pure integer (e.g. `2` or `3` column count specifier)
        if trimmed_arg.parse::<usize>().is_ok() {
            continue;
        }

        let leading_spaces = arg_str.len() - arg_str.trim_start().len();
        let trailing_spaces = arg_str.len() - arg_str.trim_end().len();
        let real_start = abs_start + leading_spaces;
        let real_end = abs_end.saturating_sub(trailing_spaces);

        let portion = portions.get(col_idx).copied().unwrap_or(1);

        // Compute display text as direct zero-copy slice of raw
        let display_text: &'a str =
            if trimmed_arg.starts_with('[') && trimmed_arg.ends_with(']') && trimmed_arg.len() >= 2
            {
                trimmed_arg[1..trimmed_arg.len() - 1].trim()
            } else if let Some(bracket_start) = trimmed_arg.find('[')
                && let Some(bracket_end) = trimmed_arg.rfind(']')
                && bracket_start < bracket_end
            {
                trimmed_arg[bracket_start + 1..bracket_end].trim()
            } else {
                trimmed_arg
            };

        items.push(GridColItem {
            raw: trimmed_arg,
            display_text,
            range_in_raw: real_start..real_end,
            portion,
        });

        col_idx += 1;
    }

    if items.is_empty() {
        // Fallback: search for any top-level bracket spans in raw
        let spans = extract_bracket_spans(raw);
        for (idx, span) in spans.into_iter().enumerate() {
            let item_raw = &raw[span.clone()];
            let display_text =
                if item_raw.starts_with('[') && item_raw.ends_with(']') && item_raw.len() >= 2 {
                    item_raw[1..item_raw.len() - 1].trim()
                } else {
                    item_raw.trim()
                };
            let portion = portions.get(idx).copied().unwrap_or(1);
            items.push(GridColItem {
                raw: item_raw,
                display_text,
                range_in_raw: span,
                portion,
            });
        }
    }

    items
}

/// Parse column count from arguments (e.g. `columns: 3`, `ratio: (1fr, 1fr, 1fr)`, or positional count)
#[must_use]
pub fn parse_column_count(args: &str) -> usize {
    if let Some(col_idx) = args.find("columns:") {
        let after = &args[col_idx + 8..].trim_start();
        if after.starts_with('(') {
            if let Some(end_paren) = after.find(')') {
                let inner = &after[1..end_paren];
                let count = inner.split(',').filter(|s| !s.trim().is_empty()).count();
                if count > 0 {
                    return count;
                }
            }
        } else if let Some(first_non_digit) = after.find(|c: char| !c.is_ascii_digit()) {
            if let Ok(n) = after[..first_non_digit].trim().parse::<usize>()
                && n > 0
            {
                return n;
            }
        } else if let Ok(n) = after.parse::<usize>()
            && n > 0
        {
            return n;
        }
    }

    if let Some(ratio_idx) = args.find("ratio:") {
        let after = &args[ratio_idx + 6..].trim_start();
        if after.starts_with('(')
            && let Some(end_paren) = after.find(')')
        {
            let inner = &after[1..end_paren];
            let count = inner.split(',').filter(|s| !s.trim().is_empty()).count();
            if count > 0 {
                return count;
            }
        }
    }

    // Check positional integer count e.g. cols(2) or 3
    let trimmed = args.trim_start();
    let search_str = if let Some(open) = trimmed.find('(') {
        trimmed[open + 1..].trim_start()
    } else {
        trimmed
    };
    if let Some(digit_len) = search_str.find(|c: char| !c.is_ascii_digit()) {
        if digit_len > 0
            && let Ok(n) = search_str[..digit_len].parse::<usize>()
            && n > 0
            && (search_str[digit_len..].starts_with(',')
                || search_str[digit_len..].starts_with(')')
                || search_str[digit_len..].trim_start().starts_with('[')
                || search_str[digit_len..].is_empty())
        {
            return n;
        }
    } else if let Ok(n) = search_str.parse::<usize>()
        && n > 0
    {
        return n;
    }

    let items = extract_column_items(args);
    if !items.is_empty() {
        return items.len();
    }

    2
}

/// Extract named string literal or bracket content
#[must_use]
pub fn extract_named_string<'a>(
    args: &'a str,
    key: &str,
) -> Option<&'a str> {
    let pattern = format!("{key}:");
    let idx = args.find(&pattern)?;
    let after = &args[idx + pattern.len()..].trim_start();
    if let Some(stripped) = after.strip_prefix('"') {
        let end_quote = stripped.find('"')?;
        Some(&stripped[..end_quote])
    } else if let Some(stripped) = after.strip_prefix('[') {
        let end_bracket = stripped.find(']')?;
        Some(&stripped[..end_bracket])
    } else {
        None
    }
}

/// Add an empty row of cells to `#table(...)`
#[must_use]
pub fn add_row_to_table(
    raw: &str,
    cols: usize,
) -> String {
    if let Some(last_paren) = raw.rfind(')') {
        let new_cells = (0..cols.max(1))
            .map(|_| "[ ]")
            .collect::<Vec<_>>()
            .join(", ");
        let (before, after) = raw.split_at(last_paren);
        let separator = if before.trim_end().ends_with(',') {
            "\n  "
        } else {
            ",\n  "
        };
        format!("{before}{separator}{new_cells}\n{after}")
    } else {
        format!("{raw}\n  [ ], [ ]")
    }
}

/// Add a column to `#table(...)`
#[must_use]
pub fn add_col_to_table(
    raw: &str,
    current_cols: usize,
) -> String {
    let mut updated = raw.to_string();
    let new_cols = current_cols + 1;

    if let Some(col_pos) = updated.find("columns:") {
        let after = &updated[col_pos + 8..];
        let leading_spaces = after.len() - after.trim_start().len();
        let val_start = col_pos + 8 + leading_spaces;
        if updated[val_start..].starts_with('(') {
            if let Some(end_paren) = updated[val_start..].find(')') {
                let paren_pos = val_start + end_paren;
                updated.insert_str(paren_pos, ", 1fr");
            }
        } else if let Some(digit_end) = updated[val_start..].find(|c: char| !c.is_ascii_digit())
            && updated[val_start..val_start + digit_end]
                .parse::<usize>()
                .is_ok()
        {
            updated.replace_range(val_start..val_start + digit_end, &new_cols.to_string());
        }
    }

    if let Some(last_paren) = updated.rfind(')') {
        let (before, after) = updated.split_at(last_paren);
        let separator = if before.trim_end().ends_with(',') {
            "\n  "
        } else {
            ",\n  "
        };
        format!("{before}{separator}[ ]\n{after}")
    } else {
        updated
    }
}

/// Find byte ranges of each table cell inside `#table(...)`
#[must_use]
pub fn extract_table_cell_spans(raw: &str) -> Vec<Range<usize>> {
    let bracketed = extract_bracket_spans(raw);
    if !bracketed.is_empty() {
        return bracketed;
    }

    let mut spans = Vec::new();
    let Some(first_paren) = raw.find('(') else {
        return spans;
    };
    let Some(last_paren) = raw.rfind(')') else {
        return spans;
    };
    if first_paren >= last_paren {
        return spans;
    }

    let inner = &raw[first_paren + 1..last_paren];
    let offset = first_paren + 1;

    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut arg_start = None;

    for (byte_idx, c) in inner.char_indices() {
        if arg_start.is_none() && !c.is_whitespace() && c != ',' {
            arg_start = Some(byte_idx);
        }

        match c {
            | '(' => depth_paren += 1,
            | ')' => depth_paren = depth_paren.saturating_sub(1),
            | '[' => depth_bracket += 1,
            | ']' => depth_bracket = depth_bracket.saturating_sub(1),
            | ',' if depth_paren == 0 && depth_bracket == 0 => {
                if let Some(start) = arg_start {
                    let candidate = inner[start..byte_idx].trim();
                    if !candidate.is_empty() && !is_named_arg(candidate) {
                        let rel_start = start
                            + (inner[start..byte_idx].len()
                                - inner[start..byte_idx].trim_start().len());
                        let rel_end = start + inner[start..byte_idx].trim_end().len();
                        spans.push((offset + rel_start)..(offset + rel_end));
                    }
                    arg_start = None;
                }
            },
            | _ => {},
        }
    }

    if let Some(start) = arg_start {
        let candidate = inner[start..].trim();
        if !candidate.is_empty() && !is_named_arg(candidate) {
            let rel_start = start + (inner[start..].len() - inner[start..].trim_start().len());
            let rel_end = start + inner[start..].trim_end().len();
            spans.push((offset + rel_start)..(offset + rel_end));
        }
    }

    spans
}

/// Update a table cell at `cell_idx` with `new_text`
#[must_use]
pub fn update_table_cell(
    raw: &str,
    cell_idx: usize,
    new_text: &str,
) -> String {
    let spans = extract_table_cell_spans(raw);
    if let Some(span) = spans.get(cell_idx) {
        let mut res = raw.to_string();
        res.replace_range(span.clone(), &format!("[{new_text}]"));
        res
    } else {
        raw.to_string()
    }
}

/// Delete the last row of `#table(...)`
#[must_use]
pub fn delete_row_from_table(
    raw: &str,
    cols: usize,
) -> String {
    let spans = extract_table_cell_spans(raw);
    let n = spans.len();
    let to_remove = cols.max(1);
    if n <= to_remove {
        return raw.to_string();
    }

    let first_remove = &spans[n - to_remove];
    let last_remove = &spans[n - 1];

    let before = &raw[..first_remove.start];
    let trim_start = before.rfind(',').unwrap_or(first_remove.start);

    let mut res = raw.to_string();
    res.replace_range(trim_start..last_remove.end, "");
    res
}

/// Delete a column from `#table(...)`
#[must_use]
pub fn delete_col_from_table(
    raw: &str,
    current_cols: usize,
) -> String {
    if current_cols <= 1 {
        return raw.to_string();
    }
    let mut updated = raw.to_string();

    // 1. Decrement columns: count
    let new_cols = current_cols - 1;
    if let Some(col_pos) = updated.find("columns:") {
        let after = &updated[col_pos + 8..];
        let leading_spaces = after.len() - after.trim_start().len();
        let val_start = col_pos + 8 + leading_spaces;
        if updated[val_start..].starts_with('(') {
            if let Some(last_comma) = updated[val_start..].rfind(',')
                && let Some(end_paren) = updated[val_start + last_comma..].find(')')
            {
                let range_to_del = (val_start + last_comma)..(val_start + last_comma + end_paren);
                updated.replace_range(range_to_del, "");
            }
        } else if let Some(digit_end) = updated[val_start..].find(|c: char| !c.is_ascii_digit())
            && updated[val_start..val_start + digit_end]
                .parse::<usize>()
                .is_ok()
        {
            updated.replace_range(val_start..val_start + digit_end, &new_cols.to_string());
        }
    }

    // 2. Remove the last cell of each row in reverse order
    let spans = extract_table_cell_spans(&updated);
    let mut indices_to_remove = Vec::new();
    let mut r = 0;
    while (r + 1) * current_cols <= spans.len() {
        indices_to_remove.push((r + 1) * current_cols - 1);
        r += 1;
    }
    for &idx in indices_to_remove.iter().rev() {
        let spans_now = extract_table_cell_spans(&updated);
        if let Some(span) = spans_now.get(idx) {
            let before = &updated[..span.start];
            let trim_start = before.rfind(',').unwrap_or(span.start);
            updated.replace_range(trim_start..span.end, "");
        }
    }

    updated
}

/// Extract top-level bracketed content ranges from `raw`
#[must_use]
pub fn extract_bracket_spans(raw: &str) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut in_fence = false;
    let mut in_quote = false;
    let mut escape = false;

    let bytes = raw.as_bytes();
    let len = bytes.len();
    let mut idx = 0;

    while idx < len {
        let b = bytes[idx];

        if escape {
            escape = false;
            idx += 1;
            continue;
        }

        if b == b'\\' {
            escape = true;
            idx += 1;
            continue;
        }

        // Check for 3+ backticks (code fence)
        if b == b'`' && idx + 2 < len && bytes[idx + 1] == b'`' && bytes[idx + 2] == b'`' {
            in_fence = !in_fence;
            idx += 3;
            while idx < len && bytes[idx] == b'`' {
                idx += 1;
            }
            continue;
        }

        if in_fence {
            idx += 1;
            continue;
        }

        if b == b'"' {
            in_quote = !in_quote;
            idx += 1;
            continue;
        }

        if in_quote {
            idx += 1;
            continue;
        }

        if b == b'[' {
            if depth == 0 {
                start = idx;
            }
            depth += 1;
        } else if b == b']' && depth > 0 {
            depth -= 1;
            if depth == 0 {
                spans.push(start..(idx + 1));
            }
        }

        idx += 1;
    }
    spans
}

/// Update column content in `#grid(...)` or `#cols(...)`
#[must_use]
pub fn update_grid_col(
    raw: &str,
    col_idx: usize,
    new_text: &str,
) -> String {
    let col_items = extract_column_items(raw);
    if let Some(item) = col_items.get(col_idx) {
        let mut res = raw.to_string();
        let replacement: String = if item.raw.starts_with("callout") {
            if let Some(start) = item.raw.find('[')
                && let Some(end) = item.raw.rfind(']')
                && start < end
            {
                let mut c_raw = item.raw.to_string();
                c_raw.replace_range((start + 1)..end, &format!("\n  {new_text}\n"));
                c_raw
            } else {
                format!("[\n  {new_text}\n]")
            }
        } else {
            format!("[\n  {new_text}\n]")
        };
        res.replace_range(item.range_in_raw.clone(), &replacement);
        res
    } else {
        let spans = extract_bracket_spans(raw);
        if let Some(span) = spans.get(col_idx) {
            let mut res = raw.to_string();
            res.replace_range(span.clone(), &format!("[\n  {new_text}\n]"));
            res
        } else {
            raw.to_string()
        }
    }
}

/// Delete a column from `#grid(...)` or `#cols(...)`
#[must_use]
pub fn grid_delete_col(
    raw: &str,
    col_idx: usize,
) -> String {
    let col_items = extract_column_items(raw);
    if col_items.len() <= 1 || col_idx >= col_items.len() {
        return raw.to_string();
    }

    let mut updated = raw.to_string();

    // 1. If ratio: (1fr, 1fr, 1fr) is present, remove col_idx-th entry
    if let Some(pos) = updated.find("ratio:") {
        let after_idx = pos + 6;
        let after = &updated[after_idx..];
        let leading_spaces = after.len() - after.trim_start().len();
        let val_start = after_idx + leading_spaces;
        if updated[val_start..].starts_with('(')
            && let Some(end_paren) = updated[val_start..].find(')')
        {
            let inner = &updated[val_start + 1..val_start + end_paren];
            let parts: Vec<&str> = inner.split(',').collect();
            if parts.len() > 1 && col_idx < parts.len() {
                let mut new_parts = parts;
                new_parts.remove(col_idx);
                let new_inner = new_parts.join(",");
                let range_to_replace = (val_start + 1)..(val_start + end_paren);
                updated.replace_range(range_to_replace, &new_inner);
            }
        }
    }

    // 2. Decrement columns if present
    if let Some(col_pos) = updated.find("columns:") {
        let after = &updated[col_pos + 8..];
        let leading_spaces = after.len() - after.trim_start().len();
        let val_start = col_pos + 8 + leading_spaces;
        if updated[val_start..].starts_with('(')
            && let Some(end_paren) = updated[val_start..].find(')')
        {
            let inner = &updated[val_start + 1..val_start + end_paren];
            let parts: Vec<&str> = inner.split(',').collect();
            if parts.len() > 1 && col_idx < parts.len() {
                let mut new_parts = parts;
                new_parts.remove(col_idx);
                let new_inner = new_parts.join(",");
                let range_to_replace = (val_start + 1)..(val_start + end_paren);
                updated.replace_range(range_to_replace, &new_inner);
            }
        } else if let Some(digit_end) = updated[val_start..].find(|c: char| !c.is_ascii_digit())
            && let Ok(n) = updated[val_start..val_start + digit_end].parse::<usize>()
            && n > 1
        {
            let new_n = n - 1;
            updated.replace_range(val_start..val_start + digit_end, &new_n.to_string());
        }
    }

    // 3. Decrement cols(N) if present
    if let Some(pos) = updated.find("cols(") {
        let after_idx = pos + 5;
        let after = &updated[after_idx..];
        let leading_spaces = after.len() - after.trim_start().len();
        let val_start = after_idx + leading_spaces;
        if let Some(digit_end) = updated[val_start..].find(|c: char| !c.is_ascii_digit())
            && digit_end > 0
            && let Ok(n) = updated[val_start..val_start + digit_end].parse::<usize>()
            && n > 1
        {
            let new_n = n - 1;
            updated.replace_range(val_start..val_start + digit_end, &new_n.to_string());
        }
    }

    // 4. Remove the column argument or bracket block
    let re_items = extract_column_items(&updated);
    if let Some(target) = re_items.get(col_idx) {
        let span = &target.range_in_raw;
        let before = &updated[..span.start];
        let before_trimmed = before.trim_end();
        if before_trimmed.ends_with(',') {
            let comma_idx = before_trimmed.len() - 1;
            updated.replace_range(comma_idx..span.end, "");
        } else {
            let after = &updated[span.end..];
            let after_trimmed = after.trim_start();
            if after_trimmed.starts_with(',') {
                let comma_offset = after.len() - after_trimmed.len();
                updated.replace_range(span.start..(span.end + comma_offset + 1), "");
            } else {
                updated.replace_range(span.start..span.end, "");
            }
        }
    }

    updated
}

/// Add a column to `#grid(...)` or `#cols(...)`
#[must_use]
pub fn add_col_to_grid(raw: &str) -> String {
    let mut updated = raw.to_string();

    // 1. If ratio: (1fr, 1fr) is present, append , 1fr
    if let Some(pos) = updated.find("ratio:") {
        let after_idx = pos + 6;
        let after = &updated[after_idx..];
        let leading_spaces = after.len() - after.trim_start().len();
        let val_start = after_idx + leading_spaces;
        if updated[val_start..].starts_with('(')
            && let Some(end_paren) = updated[val_start..].find(')')
        {
            let paren_pos = val_start + end_paren;
            let prefix = if updated[val_start..paren_pos].trim_end().ends_with(',') {
                " 1fr"
            } else {
                ", 1fr"
            };
            updated.insert_str(paren_pos, prefix);
        }
    }

    // 2. If columns: (...) is present, append , 1fr; or columns: N, increment N
    if let Some(col_pos) = updated.find("columns:") {
        let after = &updated[col_pos + 8..];
        let leading_spaces = after.len() - after.trim_start().len();
        let val_start = col_pos + 8 + leading_spaces;
        if updated[val_start..].starts_with('(') {
            if let Some(end_paren) = updated[val_start..].find(')') {
                let paren_pos = val_start + end_paren;
                let prefix = if updated[val_start..paren_pos].trim_end().ends_with(',') {
                    " 1fr"
                } else {
                    ", 1fr"
                };
                updated.insert_str(paren_pos, prefix);
            }
        } else if let Some(digit_end) = updated[val_start..].find(|c: char| !c.is_ascii_digit())
            && let Ok(n) = updated[val_start..val_start + digit_end].parse::<usize>()
        {
            let new_n = n + 1;
            updated.replace_range(val_start..val_start + digit_end, &new_n.to_string());
        }
    }

    // 3. If cols(N) is present, increment N
    if let Some(pos) = updated.find("cols(") {
        let after_idx = pos + 5;
        let after = &updated[after_idx..];
        let leading_spaces = after.len() - after.trim_start().len();
        let val_start = after_idx + leading_spaces;
        if let Some(digit_end) = updated[val_start..].find(|c: char| !c.is_ascii_digit())
            && digit_end > 0
            && let Ok(n) = updated[val_start..val_start + digit_end].parse::<usize>()
        {
            let new_n = n + 1;
            updated.replace_range(val_start..val_start + digit_end, &new_n.to_string());
        }
    }

    // 4. Check if trailing bracket blocks syntax is used (e.g. #cols(2)[...][...])
    let has_trailing_brackets = if let Some(open) = updated.find('(') {
        let mut depth = 0usize;
        let mut first_close = None;
        for (idx, c) in updated[open..].char_indices() {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    first_close = Some(open + idx);
                    break;
                }
            }
        }
        if let Some(close) = first_close {
            updated[close + 1..].trim_start().starts_with('[')
        } else {
            false
        }
    } else {
        false
    };

    if has_trailing_brackets {
        format!("{updated}[\n  *New Column*\n  Content here\n]")
    } else if let Some(last_paren) = updated.rfind(')') {
        let (before, after) = updated.split_at(last_paren);
        let separator = if before.trim_end().ends_with(',') {
            "\n  "
        } else {
            ",\n  "
        };
        format!("{before}{separator}[\n    *New Column*\n    Content here\n  ]\n{after}")
    } else {
        format!("{updated}\n[\n  *New Column*\n  Content here\n]")
    }
}

/// Switch callout kind (e.g. #note -> #tip)
#[must_use]
pub fn switch_callout_kind(
    raw: &str,
    old_callee: &str,
    new_callee: &str,
) -> String {
    let target = format!("#{old_callee}");
    if let Some(pos) = raw.find(&target) {
        let mut res = raw.to_string();
        res.replace_range(pos..pos + target.len(), &format!("#{new_callee}"));
        res
    } else {
        raw.to_string()
    }
}

/// Add data point to `#chart(...)`
#[must_use]
pub fn add_data_point_to_chart(raw: &str) -> String {
    if let Some(last_paren) = raw.rfind(')') {
        let (before, after) = raw.split_at(last_paren);
        let separator = if before.trim_end().ends_with(',') {
            "\n  "
        } else {
            ",\n  "
        };
        format!("{before}{separator}(\"New\", 50)\n{after}")
    } else {
        format!("{raw}, (\"New\", 50)")
    }
}

/// Switch chart kind
#[must_use]
pub fn switch_chart_kind(
    raw: &str,
    new_macro: &str,
) -> String {
    for candidate in &["#chart-bar", "#chart-pie", "#chart-line", "#chart", "#plot"] {
        if let Some(pos) = raw.find(candidate) {
            let mut res = raw.to_string();
            res.replace_range(pos..pos + candidate.len(), new_macro);
            return res;
        }
    }
    raw.to_string()
}

/// Update chart title
#[must_use]
pub fn update_chart_title(
    raw: &str,
    new_title: &str,
) -> String {
    if let Some(title_pos) = raw.find("title:") {
        let after = &raw[title_pos + 6..];
        let leading = after.len() - after.trim_start().len();
        let val_start = title_pos + 6 + leading;
        if let Some(stripped) = raw[val_start..].strip_prefix('"')
            && let Some(end_quote) = stripped.find('"')
        {
            let mut res = raw.to_string();
            res.replace_range(
                val_start..val_start + end_quote + 2,
                &format!("\"{new_title}\""),
            );
            return res;
        }
    } else if let Some(first_paren) = raw.find('(') {
        let mut res = raw.to_string();
        res.insert_str(first_paren + 1, &format!("title: \"{new_title}\", "));
        return res;
    }
    raw.to_string()
}

/// Find the byte spans of data tuple items in `#chart(...)`, e.g. `("Label", 50)`
#[must_use]
pub fn extract_chart_item_spans(raw: &str) -> Vec<(Range<usize>, Range<usize>, Range<usize>)> {
    let mut spans = Vec::new();
    let Some(first_paren) = raw.find('(') else {
        return spans;
    };
    let Some(last_paren) = raw.rfind(')') else {
        return spans;
    };
    if first_paren >= last_paren {
        return spans;
    }

    let inner = &raw[first_paren + 1..last_paren];
    let offset = first_paren + 1;

    let mut cursor = 0;
    while let Some(start_paren) = inner[cursor..].find('(') {
        let abs_start = cursor + start_paren;
        if let Some(end_paren) = inner[abs_start..].find(')') {
            let abs_end = abs_start + end_paren;
            let tuple_content = &inner[abs_start + 1..abs_end];
            if let Some((lbl_part, val_part)) = tuple_content.split_once(',')
                && !is_named_arg(lbl_part)
            {
                let full_span = (offset + abs_start)..(offset + abs_end + 1);

                let lbl_trim_start = lbl_part.len() - lbl_part.trim_start().len();
                let lbl_len = lbl_part.trim().len();
                let lbl_abs_start = offset + abs_start + 1 + lbl_trim_start;
                let lbl_span = lbl_abs_start..(lbl_abs_start + lbl_len);

                let val_offset = abs_start + 1 + lbl_part.len() + 1;
                let val_trim_start = val_part.len() - val_part.trim_start().len();
                let val_len = val_part.trim().len();
                let val_abs_start = offset + val_offset + val_trim_start;
                let val_span = val_abs_start..(val_abs_start + val_len);

                spans.push((full_span, lbl_span, val_span));
            }
            cursor = abs_end + 1;
        } else {
            break;
        }
    }
    spans
}

/// Update chart item label
#[must_use]
pub fn update_chart_item_label(
    raw: &str,
    item_idx: usize,
    new_label: &str,
) -> String {
    let spans = extract_chart_item_spans(raw);
    if let Some((_, lbl_span, _)) = spans.get(item_idx) {
        let mut res = raw.to_string();
        res.replace_range(lbl_span.clone(), &format!("\"{new_label}\""));
        res
    } else {
        raw.to_string()
    }
}

/// Update chart item value
#[must_use]
pub fn update_chart_item_value(
    raw: &str,
    item_idx: usize,
    new_val: f32,
) -> String {
    let spans = extract_chart_item_spans(raw);
    if let Some((_, _, val_span)) = spans.get(item_idx) {
        let mut res = raw.to_string();
        res.replace_range(val_span.clone(), &format!("{new_val}"));
        res
    } else {
        raw.to_string()
    }
}

/// Delete chart item
#[must_use]
pub fn chart_delete_item(
    raw: &str,
    item_idx: usize,
) -> String {
    let spans = extract_chart_item_spans(raw);
    if spans.len() <= 1 || item_idx >= spans.len() {
        return raw.to_string();
    }
    if let Some((full_span, _, _)) = spans.get(item_idx) {
        let before = &raw[..full_span.start];
        let trim_start = before.rfind(',').unwrap_or(full_span.start);
        let mut res = raw.to_string();
        res.replace_range(trim_start..full_span.end, "");
        res
    } else {
        raw.to_string()
    }
}

/// Extract the inner content of a bracketed block `[...]`, correctly ignoring
/// any brackets occurring inside ```code fences```, inline `code`, quotes, or comments.
#[must_use]
pub fn extract_inner_content_block(s: &str) -> &str {
    let trimmed = s.trim();
    let Some(first_bracket) = trimmed.find('[') else {
        return trimmed;
    };

    let slice_from_bracket = &trimmed[first_bracket..];
    let mut depth = 0usize;
    let mut in_fence = false;
    let mut in_inline_code = false;
    let mut in_quote = false;
    let mut escape = false;

    let bytes = slice_from_bracket.as_bytes();
    let len = bytes.len();
    let mut idx = 0;
    let mut close_bracket = None;

    while idx < len {
        let b = bytes[idx];

        if escape {
            escape = false;
            idx += 1;
            continue;
        }

        if b == b'\\' {
            escape = true;
            idx += 1;
            continue;
        }

        // Check for 3+ backticks (code fence)
        if b == b'`' && idx + 2 < len && bytes[idx + 1] == b'`' && bytes[idx + 2] == b'`' {
            in_fence = !in_fence;
            idx += 3;
            while idx < len && bytes[idx] == b'`' {
                idx += 1;
            }
            continue;
        }

        if in_fence {
            idx += 1;
            continue;
        }

        // Check for inline backtick
        if b == b'`' {
            in_inline_code = !in_inline_code;
            idx += 1;
            continue;
        }

        if in_inline_code {
            idx += 1;
            continue;
        }

        if b == b'"' {
            in_quote = !in_quote;
            idx += 1;
            continue;
        }

        if in_quote {
            idx += 1;
            continue;
        }

        if b == b'[' {
            depth += 1;
        } else if b == b']' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                close_bracket = Some(first_bracket + idx);
                break;
            }
        }

        idx += 1;
    }

    if let Some(close_idx) = close_bracket {
        trimmed[first_bracket + 1..close_idx].trim()
    } else if let Some(last_bracket) = trimmed.rfind(']') {
        if first_bracket < last_bracket {
            trimmed[first_bracket + 1..last_bracket].trim()
        } else {
            trimmed
        }
    } else {
        trimmed
    }
}

/// Update callout title
#[must_use]
pub fn update_callout_title(
    raw: &str,
    new_title: &str,
) -> String {
    if let Some(pos) = raw.find("title:") {
        let after = &raw[pos + 6..];
        let leading = after.len() - after.trim_start().len();
        let val_start = pos + 6 + leading;
        if let Some(stripped) = raw[val_start..].strip_prefix('"')
            && let Some(end_quote) = stripped.find('"')
        {
            let mut res = raw.to_string();
            res.replace_range(
                val_start..val_start + end_quote + 2,
                &format!("\"{new_title}\""),
            );
            return res;
        } else if let Some(stripped) = raw[val_start..].strip_prefix('[')
            && let Some(end_bracket) = stripped.find(']')
        {
            let mut res = raw.to_string();
            res.replace_range(
                val_start..val_start + end_bracket + 2,
                &format!("\"{new_title}\""),
            );
            return res;
        }
    } else if let Some(first_paren) = raw.find('(') {
        let mut res = raw.to_string();
        res.insert_str(first_paren + 1, &format!("title: \"{new_title}\", "));
        return res;
    } else if let Some(bracket_pos) = raw.find('[') {
        let mut res = raw.to_string();
        res.insert_str(bracket_pos, &format!("(title: \"{new_title}\")"));
        return res;
    }
    raw.to_string()
}

/// Update callout body
#[must_use]
pub fn update_callout_body(
    raw: &str,
    new_body: &str,
) -> String {
    if let Some(open) = raw.find('(') {
        let mut depth = 0usize;
        let mut close_paren = None;
        let mut in_str = false;
        let mut esc = false;
        for (idx, c) in raw[open..].char_indices() {
            if esc {
                esc = false;
                continue;
            }
            if c == '\\' {
                esc = true;
                continue;
            }
            if c == '"' {
                in_str = !in_str;
                continue;
            }
            if in_str {
                continue;
            }
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    close_paren = Some(open + idx);
                    break;
                }
            }
        }
        if let Some(cp) = close_paren {
            let after_offset = cp + 1;
            let spans = extract_bracket_spans(&raw[after_offset..]);
            if let Some(first) = spans.first() {
                let abs_span = (after_offset + first.start)..(after_offset + first.end);
                let mut res = raw.to_string();
                res.replace_range(abs_span, &format!("[\n  {new_body}\n]"));
                return res;
            }
        }
    }

    let spans = extract_bracket_spans(raw);
    if let Some(first) = spans.first() {
        let mut res = raw.to_string();
        res.replace_range(first.clone(), &format!("[\n  {new_body}\n]"));
        res
    } else {
        format!("{raw}[\n  {new_body}\n]")
    }
}

/// Update box content
#[must_use]
pub fn update_box_content(
    raw: &str,
    new_content: &str,
) -> String {
    let spans = extract_bracket_spans(raw);
    if let Some(first) = spans.first() {
        let mut res = raw.to_string();
        res.replace_range(first.clone(), &format!("[\n  {new_content}\n]"));
        res
    } else {
        raw.to_string()
    }
}

/// Parsed data for callouts
#[derive(Debug, Clone)]
pub struct CalloutData<'a> {
    pub kind: &'a str,
    pub title: &'a str,
    pub body: &'a str,
    pub attribution: Option<&'a str>,
    pub stroke_color: Option<&'a str>,
}

#[must_use]
pub fn parse_callout_data<'a>(
    callee: &'a str,
    args: &'a str,
) -> CalloutData<'a> {
    let title = extract_named_string(args, "title").unwrap_or("");
    let attribution =
        extract_named_string(args, "attribution").or_else(|| extract_named_string(args, "author"));
    let stroke_color = extract_named_string(args, "stroke-color")
        .or_else(|| extract_named_string(args, "stroke"))
        .or_else(|| extract_named_string(args, "color"))
        .or_else(|| {
            if let Some(pos) = args.find("stroke-color:") {
                let after = args[pos + 13..].trim_start();
                let mut depth = 0usize;
                let mut in_str = false;
                let mut esc = false;
                let mut end_pos = after.len();
                for (i, c) in after.char_indices() {
                    if esc {
                        esc = false;
                        continue;
                    }
                    if c == '\\' {
                        esc = true;
                        continue;
                    }
                    if c == '"' {
                        in_str = !in_str;
                        continue;
                    }
                    if in_str {
                        continue;
                    }
                    if c == '(' {
                        depth += 1;
                    } else if c == ')' {
                        if depth == 0 {
                            end_pos = i;
                            break;
                        }
                        depth -= 1;
                    } else if (c == ',' || c == '\n') && depth == 0 {
                        end_pos = i;
                        break;
                    }
                }
                let val = after[..end_pos].trim();
                if !val.is_empty() {
                    Some(val)
                } else {
                    None
                }
            } else {
                None
            }
        });

    let body = if let Some(open) = args.find('(') {
        let mut depth = 0usize;
        let mut close_paren = None;
        let mut in_str = false;
        let mut esc = false;
        for (idx, c) in args[open..].char_indices() {
            if esc {
                esc = false;
                continue;
            }
            if c == '\\' {
                esc = true;
                continue;
            }
            if c == '"' {
                in_str = !in_str;
                continue;
            }
            if in_str {
                continue;
            }
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    close_paren = Some(open + idx);
                    break;
                }
            }
        }
        if let Some(cp) = close_paren {
            let after = args[cp + 1..].trim_start();
            if after.starts_with('[') {
                extract_inner_content_block(after)
            } else {
                let bracketed = extract_bracket_contents(args);
                bracketed
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| args[open + 1..cp].trim())
            }
        } else {
            let bracketed = extract_bracket_contents(args);
            bracketed.into_iter().next().unwrap_or_else(|| {
                args.trim()
                    .trim_start_matches('(')
                    .trim_end_matches(')')
                    .trim()
            })
        }
    } else {
        let bracketed = extract_bracket_contents(args);
        bracketed.into_iter().next().unwrap_or_else(|| {
            args.trim()
                .trim_start_matches('(')
                .trim_end_matches(')')
                .trim()
        })
    };

    CalloutData {
        kind: callee,
        title,
        body,
        attribution,
        stroke_color,
    }
}

/// Parsed data for charts
#[derive(Debug, Clone)]
pub struct ChartData<'a> {
    pub title: &'a str,
    pub items: Vec<(&'a str, f32)>,
}

#[must_use]
pub fn parse_chart_data<'a>(args: &'a str) -> ChartData<'a> {
    let title = extract_named_string(args, "title").unwrap_or("Data Chart");

    let mut items = Vec::new();
    let trimmed = args.trim();
    let inner_content = if trimmed.starts_with('(') && trimmed.ends_with(')') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };

    let mut cursor = 0;
    while let Some(start_paren) = inner_content[cursor..].find('(') {
        let abs_start = cursor + start_paren;
        if let Some(end_paren) = inner_content[abs_start..].find(')') {
            let abs_end = abs_start + end_paren;
            let inner = inner_content[abs_start + 1..abs_end].trim();
            if let Some((label_raw, val_raw)) = inner.split_once(',') {
                let label = label_raw.trim().trim_matches('"').trim_matches('\'').trim();
                let val_str = val_raw
                    .trim()
                    .trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
                if let Ok(val) = val_str.parse::<f32>()
                    && !label.is_empty()
                    && !is_named_arg(label_raw)
                {
                    items.push((label, val));
                }
            }
            cursor = abs_end + 1;
        } else {
            break;
        }
    }

    if items.is_empty() {
        let parts = split_args(inner_content);
        let non_named: Vec<&str> = parts
            .into_iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && !is_named_arg(s))
            .collect();
        for chunk in non_named.as_chunks::<2>().0 {
            let label = chunk[0]
                .trim_matches('[')
                .trim_matches(']')
                .trim_matches('"')
                .trim_matches('\'')
                .trim();
            let val_str = chunk[1].trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
            if let Ok(val) = val_str.parse::<f32>()
                && !label.is_empty()
            {
                items.push((label, val));
            }
        }
    }

    if items.is_empty() {
        items = vec![("Item A", 40.0), ("Item B", 75.0), ("Item C", 55.0)];
    }

    ChartData { title, items }
}

#[must_use]
pub fn parse_box_data(raw_or_args: &str) -> &str {
    if let Some(first_bracket) = raw_or_args.find('[')
        && let Some(last_bracket) = raw_or_args.rfind(']')
        && first_bracket < last_bracket
    {
        raw_or_args[first_bracket + 1..last_bracket].trim()
    } else {
        let bracketed = extract_bracket_contents(raw_or_args);
        if let Some(first) = bracketed.into_iter().next() {
            first
        } else {
            raw_or_args
                .trim()
                .trim_start_matches('(')
                .trim_end_matches(')')
                .trim()
        }
    }
}

#[must_use]
pub fn clean_input_style(
    theme: AppTheme,
    text_color: Color,
) -> text_input::Style {
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        icon: theme.text_muted(),
        placeholder: theme.text_muted(),
        value: text_color,
        selection: Color::from_rgba(0.2, 0.5, 0.9, 0.25),
    }
}

fn bordered_input_style(
    theme: AppTheme,
    text_color: Color,
) -> text_input::Style {
    text_input::Style {
        background: Background::Color(Color::from_rgba(0.5, 0.5, 0.5, 0.06)),
        border: Border {
            color: Color::from_rgba(0.5, 0.5, 0.5, 0.15),
            width: 0.5,
            radius: border::Radius::from(3.0),
        },
        icon: theme.text_muted(),
        placeholder: theme.text_muted(),
        value: text_color,
        selection: Color::from_rgba(0.2, 0.5, 0.9, 0.25),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_table_block<'a>(
    theme: AppTheme,
    slide_idx: usize,
    block_idx: usize,
    block: &'a WysiwygBlock,
    args: &'a str,
    raw: &'a str,
    scale: f32,
    transition: Option<&'a ElementTransition>,
) -> Element<'a, Message> {
    let cols = parse_column_count(args);
    let cells = extract_table_cells(args);

    let rows: Vec<Vec<&'a str>> = if cells.is_empty() {
        vec![vec!["Header 1", "Header 2"], vec!["Data A", "Data B"]]
    } else {
        cells
            .chunks(cols.max(1))
            .map(|chunk| chunk.to_vec())
            .collect()
    };

    let row_count = rows.len();
    let col_count = cols;

    let badge = row![
        text("Table")
            .size((11.0 * scale).max(10.0))
            .color(theme.accent()),
        text(format!("({row_count}×{col_count})"))
            .size((10.0 * scale).max(9.0))
            .color(theme.text_muted()),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let range_for_row = block.range();
    let raw_for_row = raw.to_string();
    let btn_add_row = button(text("+ Row").size(9))
        .padding([1, 5])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::UpdateBlockRange {
            slide_idx: Some(slide_idx),
            range: range_for_row,
            new_text: add_row_to_table(&raw_for_row, col_count),
        });

    let btn_del_row = if row_count > 1 {
        let r = block.range();
        let raw_del = raw.to_string();
        button(text("- Row").size(9))
            .padding([1, 5])
            .style(move |_t, _s| theme::subtle_button_style(theme, false))
            .on_press(Message::UpdateBlockRange {
                slide_idx: Some(slide_idx),
                range: r,
                new_text: delete_row_from_table(&raw_del, col_count),
            })
    } else {
        button(text("- Row").size(9))
            .padding([1, 5])
            .style(move |_t, _s| theme::subtle_button_style(theme, false))
    };

    let range_for_col = block.range();
    let raw_for_col = raw.to_string();
    let btn_add_col = button(text("+ Col").size(9))
        .padding([1, 5])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::UpdateBlockRange {
            slide_idx: Some(slide_idx),
            range: range_for_col,
            new_text: add_col_to_table(&raw_for_col, col_count),
        });

    let btn_del_col = if col_count > 1 {
        let r = block.range();
        let raw_del = raw.to_string();
        button(text("- Col").size(9))
            .padding([1, 5])
            .style(move |_t, _s| theme::subtle_button_style(theme, false))
            .on_press(Message::UpdateBlockRange {
                slide_idx: Some(slide_idx),
                range: r,
                new_text: delete_col_from_table(&raw_del, col_count),
            })
    } else {
        button(text("- Col").size(9))
            .padding([1, 5])
            .style(move |_t, _s| theme::subtle_button_style(theme, false))
    };

    let r_modal = block.range();
    let raw_modal = raw.to_string();
    let id_modal = block.id().to_string();
    let btn_modal = button(text("Edit").size(9))
        .padding([1, 6])
        .style(move |_t, _s| theme::primary_button_style(theme))
        .on_press(Message::OpenComplexModal {
            slide_idx,
            block_id: id_modal,
            range: r_modal,
            raw: raw_modal,
            callee: "table".to_string(),
        });

    let btn_step = button(
        text(if let Some(tr) = transition {
            format!("⚡ Step {} • {}", tr.order, tr.effect)
        } else {
            "⚡ Step".to_string()
        })
        .size(9),
    )
    .padding([1, 6])
    .style(move |_t, _s| {
        if transition.is_some() {
            theme::primary_button_style(theme)
        } else {
            theme::subtle_button_style(theme, false)
        }
    })
    .on_press(Message::OpenElementTransitionModal { slide_idx, block_idx });

    let btn_code = button(text("Code").size(9))
        .padding([1, 5])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::ToggleBlockRawCode(block.id().to_string()));

    let top_bar = row![
        badge,
        Space::new().width(Length::Fill),
        btn_step,
        btn_modal,
        btn_add_row,
        btn_del_row,
        btn_add_col,
        btn_del_col,
        btn_code,
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .padding([2, 4]);

    let mut table_col = column![].spacing(1).width(Length::Fill);

    for (r_idx, row_cells) in rows.into_iter().enumerate() {
        let is_header = r_idx == 0;
        let mut row_widget = row![]
            .spacing(1)
            .width(Length::Fill)
            .align_y(Alignment::Center);

        for cell_text in row_cells {
            let cell_text_display = cell_text.trim();
            let cell_content = text(if cell_text_display.is_empty() {
                " "
            } else {
                cell_text_display
            })
            .size((11.0 * scale).max(10.0))
            .color(if is_header {
                theme.accent()
            } else {
                theme.text_primary()
            });

            let cell_box = container(cell_content)
                .width(Length::FillPortion(1))
                .padding([4, 6])
                .style(move |_| {
                    if is_header {
                        container::Style {
                            background: Some(Background::Color(theme.bg_subtle())),
                            border: Border {
                                color: theme.border_color(),
                                width: 0.5,
                                radius: border::Radius::from(theme::RADIUS_XS),
                            },
                            ..container::Style::default()
                        }
                    } else {
                        container::Style {
                            background: Some(Background::Color(if r_idx % 2 == 1 {
                                theme.bg_subtle().scale_alpha(0.5)
                            } else {
                                Color::TRANSPARENT
                            })),
                            border: Border {
                                color: theme.border_subtle(),
                                width: 0.5,
                                radius: border::Radius::from(theme::RADIUS_XS),
                            },
                            ..container::Style::default()
                        }
                    }
                });

            row_widget = row_widget.push(cell_box);
        }

        table_col = table_col.push(row_widget);
    }

    container(column![top_bar, table_col].spacing(4))
        .width(Length::Fill)
        .padding([6, 8])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.bg_card())),
                border: Border {
                    color: theme.border_color(),
                    width: 1.0,
                    radius: border::Radius::from(theme::RADIUS_MD),
                },
                shadow: theme::elevation_subtle(theme),
                ..container::Style::default()
            }
        })
        .into()
}

#[allow(clippy::too_many_arguments)]
fn render_grid_block<'a>(
    theme: AppTheme,
    slide_idx: usize,
    block_idx: usize,
    block: &'a WysiwygBlock,
    callee: &'a str,
    args: &'a str,
    raw: &'a str,
    scale: f32,
    transition: Option<&'a ElementTransition>,
) -> Element<'a, Message> {
    let col_items = extract_column_items(raw);
    let items: Vec<(&'a str, u16)> = if col_items.is_empty() {
        let brackets = extract_bracket_contents(args);
        if brackets.is_empty() {
            vec![("*Column 1*\nContent", 1), ("*Column 2*\nContent", 1)]
        } else {
            brackets.into_iter().map(|b| (b, 1)).collect()
        }
    } else {
        col_items
            .into_iter()
            .map(|item| (item.display_text, item.portion))
            .collect()
    };

    let col_count = items.len().max(1);
    let badge_name = if callee.contains("col") {
        "Cols"
    } else {
        "Grid"
    };

    let badge = row![
        text(badge_name)
            .size((11.0 * scale).max(10.0))
            .color(theme.accent()),
        text(format!("({col_count} cols)"))
            .size((10.0 * scale).max(9.0))
            .color(theme.text_muted()),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let range_for_col = block.range();
    let raw_for_col = raw.to_string();
    let btn_add_col = button(text("+ Col").size(9))
        .padding([1, 5])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::UpdateBlockRange {
            slide_idx: Some(slide_idx),
            range: range_for_col,
            new_text: add_col_to_grid(&raw_for_col),
        });

    let btn_del_col = if col_count > 1 {
        let r = block.range();
        let raw_del = raw.to_string();
        button(text("- Col").size(9))
            .padding([1, 5])
            .style(move |_t, _s| theme::subtle_button_style(theme, false))
            .on_press(Message::UpdateBlockRange {
                slide_idx: Some(slide_idx),
                range: r,
                new_text: grid_delete_col(&raw_del, col_count.saturating_sub(1)),
            })
    } else {
        button(text("- Col").size(9))
            .padding([1, 5])
            .style(move |_t, _s| theme::subtle_button_style(theme, false))
    };

    let r_modal = block.range();
    let raw_modal = raw.to_string();
    let id_modal = block.id().to_string();
    let modal_callee = if callee.contains("col") {
        "cols".to_string()
    } else {
        "grid".to_string()
    };
    let btn_modal = button(text("Edit Columns").size(9))
        .padding([1, 8])
        .style(move |_t, _s| theme::primary_button_style(theme))
        .on_press(Message::OpenComplexModal {
            slide_idx,
            block_id: id_modal,
            range: r_modal,
            raw: raw_modal,
            callee: modal_callee,
        });

    let btn_step = button(
        text(if let Some(tr) = transition {
            format!("⚡ Step {} • {}", tr.order, tr.effect)
        } else {
            "⚡ Step".to_string()
        })
        .size(9),
    )
    .padding([1, 6])
    .style(move |_t, _s| {
        if transition.is_some() {
            theme::primary_button_style(theme)
        } else {
            theme::subtle_button_style(theme, false)
        }
    })
    .on_press(Message::OpenElementTransitionModal { slide_idx, block_idx });

    let btn_code = button(text("Code").size(9))
        .padding([1, 5])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::ToggleBlockRawCode(block.id().to_string()));

    let top_bar = row![
        badge,
        Space::new().width(Length::Fill),
        btn_step,
        btn_modal,
        btn_add_col,
        btn_del_col,
        btn_code,
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .padding([2, 4]);

    let mut cols_row = row![]
        .spacing((8.0 * scale).max(5.0))
        .width(Length::Fill)
        .align_y(Alignment::Start);

    for (c_idx, (item_text, portion)) in items.into_iter().enumerate() {
        let col_badge = text(format!("Col {}", c_idx + 1))
            .size((9.0 * scale).max(8.0))
            .color(theme.accent());

        let trimmed_item = item_text.trim();
        let col_content: Element<'a, Message> = if trimmed_item.starts_with("#callout")
            || trimmed_item.starts_with("callout(")
            || trimmed_item.starts_with("#note")
            || trimmed_item.starts_with("#tip")
            || trimmed_item.starts_with("#warning")
            || trimmed_item.starts_with("#info")
            || trimmed_item.starts_with("#alert")
            || trimmed_item.starts_with("#quote")
        {
            let callee_end = trimmed_item
                .find(|c: char| c == '(' || c == '[' || c.is_whitespace())
                .unwrap_or(trimmed_item.len());
            let item_callee = trimmed_item[..callee_end].trim_start_matches('#');
            let item_args = &trimmed_item[callee_end..];
            render_callout_block(
                theme,
                slide_idx,
                block_idx,
                block,
                item_callee,
                item_args,
                trimmed_item,
                scale,
                None,
            )
        } else {
            container(render_rich_text(
                item_text,
                (11.0 * scale).max(9.5),
                theme.text_primary(),
                theme,
            ))
            .width(Length::Fill)
            .padding([2, 4])
            .into()
        };

        let mut card_top =
            row![col_badge, Space::new().width(Length::Fill)].align_y(Alignment::Center);
        if col_count > 1 {
            let r_del = block.range();
            let raw_del = raw.to_string();
            let btn_del_this = button(text("×").size(8))
                .padding([0, 3])
                .style(move |_t, _s| theme::subtle_button_style(theme, false))
                .on_press(Message::UpdateBlockRange {
                    slide_idx: Some(slide_idx),
                    range: r_del,
                    new_text: grid_delete_col(&raw_del, c_idx),
                });
            card_top = card_top.push(btn_del_this);
        }

        let col_card = container(column![card_top, col_content].spacing(3))
            .width(Length::FillPortion(portion.max(1)))
            .padding([6, 8])
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(Color::from_rgba(0.5, 0.5, 0.5, 0.04))),
                    border: Border {
                        color: Color::from_rgba(0.5, 0.5, 0.5, 0.15),
                        width: 0.5,
                        radius: border::Radius::from(6.0),
                    },
                    ..container::Style::default()
                }
            });

        cols_row = cols_row.push(col_card);
    }

    container(column![top_bar, cols_row].spacing(4))
        .width(Length::Fill)
        .padding([6, 8])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.bg_subtle())),
                border: Border {
                    color: theme.border_color(),
                    width: 1.0,
                    radius: border::Radius::from(6.0),
                },
                ..container::Style::default()
            }
        })
        .into()
}

/// Parse a color from hex string "#rrggbb" or "rrggbb"
#[must_use]
pub fn parse_hex_color(hex: &str) -> Option<Color> {
    let clean = hex.trim().trim_start_matches('#').trim_matches('"');
    if clean.len() == 6 {
        let r = u8::from_str_radix(&clean[0..2], 16).ok()? as f32 / 255.0;
        let g = u8::from_str_radix(&clean[2..4], 16).ok()? as f32 / 255.0;
        let b = u8::from_str_radix(&clean[4..6], 16).ok()? as f32 / 255.0;
        Some(Color::from_rgb(r, g, b))
    } else if clean.len() == 8 {
        let r = u8::from_str_radix(&clean[0..2], 16).ok()? as f32 / 255.0;
        let g = u8::from_str_radix(&clean[2..4], 16).ok()? as f32 / 255.0;
        let b = u8::from_str_radix(&clean[4..6], 16).ok()? as f32 / 255.0;
        let a = u8::from_str_radix(&clean[6..8], 16).ok()? as f32 / 255.0;
        Some(Color::from_rgba(r, g, b, a))
    } else if clean.len() == 3 {
        let r = clean.get(0..1)?;
        let g = clean.get(1..2)?;
        let b = clean.get(2..3)?;
        let r_val = u8::from_str_radix(&format!("{r}{r}"), 16).ok()? as f32 / 255.0;
        let g_val = u8::from_str_radix(&format!("{g}{g}"), 16).ok()? as f32 / 255.0;
        let b_val = u8::from_str_radix(&format!("{b}{b}"), 16).ok()? as f32 / 255.0;
        Some(Color::from_rgb(r_val, g_val, b_val))
    } else {
        None
    }
}

/// Parse stroke or accent color spec into an Iced Color
#[must_use]
pub fn parse_color_spec(
    spec: &str,
    theme: AppTheme,
) -> Option<Color> {
    let trimmed = spec.trim().trim_matches('"');
    if trimmed.contains("accent-cyan") {
        return Some(Color::from_rgb(0.06, 0.72, 0.50));
    }
    if trimmed.contains("accent-orange") {
        return Some(Color::from_rgb(0.96, 0.62, 0.05));
    }
    if trimmed.contains("accent-red") {
        return Some(Color::from_rgb(0.94, 0.27, 0.27));
    }
    if trimmed.contains("accent-purple") {
        return Some(Color::from_rgb(0.65, 0.40, 0.95));
    }
    if trimmed.contains("accent-yellow") {
        return Some(Color::from_rgb(0.95, 0.75, 0.15));
    }
    if trimmed.contains("accent-dark") {
        return Some(Color::from_rgb(0.12, 0.43, 0.92));
    }
    if trimmed.contains("accent") {
        return Some(theme.accent());
    }

    if let Some(pos) = trimmed.find("rgb(") {
        let inside = trimmed[pos + 4..].trim_end_matches(')').trim();
        if inside.starts_with('"') || inside.starts_with('#') {
            return parse_hex_color(inside);
        }
        let parts: Vec<&str> = inside.split(',').collect();
        if parts.len() >= 3 {
            let r = parts[0].trim().parse::<f32>().ok()? / 255.0;
            let g = parts[1].trim().parse::<f32>().ok()? / 255.0;
            let b = parts[2].trim().parse::<f32>().ok()? / 255.0;
            return Some(Color::from_rgb(
                r.clamp(0.0, 1.0),
                g.clamp(0.0, 1.0),
                b.clamp(0.0, 1.0),
            ));
        }
    }

    if trimmed.starts_with('#') {
        return parse_hex_color(trimmed);
    }

    None
}

///// Structured elements parsed from a callout body
#[derive(Debug)]
pub enum CalloutSegment<'a> {
    CodeBlock { lang: &'a str, code: &'a str },
    CodeWindow { title: &'a str, code: &'a str },
    Bullet { text: &'a str },
    Numbered { num: &'a str, text: &'a str },
    Paragraph(&'a str),
}

/// Parse lines of callout body into structured segments
#[must_use]
pub fn parse_callout_segments<'a>(body: &'a str) -> Vec<CalloutSegment<'a>> {
    let mut segments = Vec::new();
    let mut line_spans: Vec<(usize, usize)> = Vec::new();
    let mut start = 0;
    for (idx, b) in body.bytes().enumerate() {
        if b == b'\n' {
            let line_end = if idx > start && body.as_bytes()[idx - 1] == b'\r' {
                idx - 1
            } else {
                idx
            };
            line_spans.push((start, line_end));
            start = idx + 1;
        }
    }
    if start <= body.len() {
        line_spans.push((start, body.len()));
    }

    let mut i = 0;
    let mut para_start: Option<usize> = None;
    let mut para_end: Option<usize> = None;

    let flush_para = |p_start: &mut Option<usize>,
                      p_end: &mut Option<usize>,
                      segs: &mut Vec<CalloutSegment<'a>>| {
        if let (Some(s), Some(e)) = (p_start.take(), p_end.take()) {
            let p_text = body[line_spans[s].0..line_spans[e].1].trim();
            if !p_text.is_empty() {
                segs.push(CalloutSegment::Paragraph(p_text));
            }
        }
    };

    while i < line_spans.len() {
        let line = &body[line_spans[i].0..line_spans[i].1];
        let trimmed_line = line.trim();

        // 1. Check for fenced code block ```lang
        if trimmed_line.starts_with("```") {
            flush_para(&mut para_start, &mut para_end, &mut segments);
            let lang = trimmed_line.trim_start_matches('`').trim();
            let code_start_line = i + 1;
            i += 1;
            let mut code_end_line = i;
            while i < line_spans.len() {
                let check_line = body[line_spans[i].0..line_spans[i].1].trim();
                if check_line.starts_with("```") {
                    code_end_line = i;
                    i += 1;
                    break;
                }
                i += 1;
                code_end_line = i;
            }
            let code = if code_end_line > code_start_line {
                &body[line_spans[code_start_line].0..line_spans[code_end_line - 1].1]
            } else {
                ""
            };
            segments.push(CalloutSegment::CodeBlock { lang, code });
            continue;
        }

        // 2. Check for #code-window(title: "...")
        if trimmed_line.starts_with("#code-window") || trimmed_line.starts_with("code-window") {
            flush_para(&mut para_start, &mut para_end, &mut segments);
            let win_start = line_spans[i].0;
            let mut win_end = line_spans[i].1;
            while i < line_spans.len() {
                let check_line = body[line_spans[i].0..line_spans[i].1].trim();
                win_end = line_spans[i].1;
                i += 1;
                if check_line.ends_with(']') {
                    break;
                }
            }
            let full_window = &body[win_start..win_end];
            let title = extract_named_string(full_window, "title").unwrap_or("code");
            let code = extract_inner_content_block(full_window);
            segments.push(CalloutSegment::CodeWindow { title, code });
            continue;
        }

        // 3. Check for bullet list item: - item or * item
        if trimmed_line.starts_with("- ") || trimmed_line.starts_with("* ") {
            flush_para(&mut para_start, &mut para_end, &mut segments);
            let leading_spaces = line.len() - line.trim_start().len();
            let item_start = line_spans[i].0 + leading_spaces + 2;
            let mut item_end = line_spans[i].1;
            i += 1;
            while i < line_spans.len() {
                let next_line = &body[line_spans[i].0..line_spans[i].1];
                let next_trimmed = next_line.trim();
                if (next_line.starts_with("  ") || next_line.starts_with('\t'))
                    && !next_trimmed.starts_with("- ")
                    && !next_trimmed.starts_with("* ")
                    && !next_trimmed.starts_with("+ ")
                    && !next_trimmed.starts_with("```")
                {
                    item_end = line_spans[i].1;
                    i += 1;
                } else {
                    break;
                }
            }
            let text = body[item_start..item_end].trim();
            segments.push(CalloutSegment::Bullet { text });
            continue;
        }

        // 4. Check for numbered list item: + item or 1. item
        let is_numbered = if trimmed_line.starts_with("+ ") {
            Some(("+", 2))
        } else if let Some(dot_pos) = trimmed_line.find(". ") {
            let prefix = &trimmed_line[..dot_pos];
            if !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit()) {
                Some((prefix, dot_pos + 2))
            } else {
                None
            }
        } else {
            None
        };

        if let Some((num_str, skip_len)) = is_numbered {
            flush_para(&mut para_start, &mut para_end, &mut segments);
            let leading_spaces = line.len() - line.trim_start().len();
            let item_start = line_spans[i].0 + leading_spaces + skip_len;
            let mut item_end = line_spans[i].1;
            i += 1;
            while i < line_spans.len() {
                let next_line = &body[line_spans[i].0..line_spans[i].1];
                let next_trimmed = next_line.trim();
                let next_is_num = next_trimmed.starts_with("+ ")
                    || next_trimmed.find(". ").is_some_and(|dp| {
                        let p = &next_trimmed[..dp];
                        !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())
                    });
                if (next_line.starts_with("  ") || next_line.starts_with('\t'))
                    && !next_trimmed.starts_with("- ")
                    && !next_trimmed.starts_with("* ")
                    && !next_is_num
                    && !next_trimmed.starts_with("```")
                {
                    item_end = line_spans[i].1;
                    i += 1;
                } else {
                    break;
                }
            }
            let text = body[item_start..item_end].trim();
            segments.push(CalloutSegment::Numbered { num: num_str, text });
            continue;
        }

        // 5. Blank line: flushes paragraph
        if trimmed_line.is_empty() {
            flush_para(&mut para_start, &mut para_end, &mut segments);
            i += 1;
            continue;
        }

        // 6. Normal text line: record into pending paragraph
        if para_start.is_none() {
            para_start = Some(i);
        }
        para_end = Some(i);
        i += 1;
    }

    flush_para(&mut para_start, &mut para_end, &mut segments);
    segments
}

/// Renders structured callout body content, including fenced code blocks (```lang...```),
/// nested #code-window mockups, bullet lists (- item), and formatted paragraphs.
#[must_use]
pub fn render_callout_body<'a>(
    body: &'a str,
    theme: AppTheme,
    accent_color: Color,
    scale: f32,
) -> Element<'a, Message> {
    let segments = parse_callout_segments(body);
    let mut col = column![]
        .spacing((5.0 * scale).max(3.0))
        .width(Length::Fill);
    let mut num_counter = 1usize;

    for seg in segments {
        match seg {
            | CalloutSegment::CodeBlock { lang, code } => {
                let lang_display = if lang.is_empty() {
                    "CODE".to_string()
                } else {
                    lang.to_uppercase()
                };

                let lang_badge = container(
                    text(lang_display)
                        .size(8.5)
                        .font(iced::Font::MONOSPACE)
                        .color(theme.text_muted()),
                )
                .padding([1, 5])
                .style(move |_| {
                    container::Style {
                        background: Some(Background::Color(theme.bg_subtle())),
                        border: border::rounded(theme::RADIUS_XS),
                        ..container::Style::default()
                    }
                });

                let code_header = row![Space::new().width(Length::Fill), lang_badge,]
                    .align_y(Alignment::Center)
                    .padding([2, 6]);

                let code_text = text(code)
                    .size((11.0 * scale).max(9.5))
                    .font(iced::Font::MONOSPACE)
                    .color(if theme.is_dark() {
                        Color::from_rgb(0.92, 0.94, 0.98)
                    } else {
                        Color::from_rgb(0.12, 0.14, 0.18)
                    });

                let code_content = container(code_text).padding([4, 8]).width(Length::Fill);

                let code_card = container(column![code_header, code_content].spacing(1))
                    .width(Length::Fill)
                    .style(move |_| {
                        container::Style {
                            background: Some(Background::Color(if theme.is_dark() {
                                Color::from_rgb(0.07, 0.09, 0.12)
                            } else {
                                Color::from_rgb(0.95, 0.96, 0.98)
                            })),
                            border: Border {
                                color: theme.border_subtle(),
                                width: 0.5,
                                radius: border::Radius::from(theme::RADIUS_SM),
                            },
                            ..container::Style::default()
                        }
                    });

                col = col.push(code_card);
            },

            | CalloutSegment::CodeWindow { title, code } => {
                let dots = row![
                    container(
                        Space::new()
                            .width(Length::Fixed(5.0))
                            .height(Length::Fixed(5.0))
                    )
                    .style(|_| {
                        container::Style {
                            background: Some(Background::Color(Color::from_rgb(1.0, 0.37, 0.34))),
                            border: border::rounded(theme::RADIUS_FULL),
                            ..container::Style::default()
                        }
                    }),
                    container(
                        Space::new()
                            .width(Length::Fixed(5.0))
                            .height(Length::Fixed(5.0))
                    )
                    .style(|_| {
                        container::Style {
                            background: Some(Background::Color(Color::from_rgb(1.0, 0.74, 0.18))),
                            border: border::rounded(theme::RADIUS_FULL),
                            ..container::Style::default()
                        }
                    }),
                    container(
                        Space::new()
                            .width(Length::Fixed(5.0))
                            .height(Length::Fixed(5.0))
                    )
                    .style(|_| {
                        container::Style {
                            background: Some(Background::Color(Color::from_rgb(0.15, 0.79, 0.25))),
                            border: border::rounded(theme::RADIUS_FULL),
                            ..container::Style::default()
                        }
                    }),
                ]
                .spacing(3)
                .align_y(Alignment::Center);

                let win_header = row![
                    dots,
                    text(title)
                        .size(9.0)
                        .color(theme.text_secondary())
                        .font(iced::Font::MONOSPACE),
                    Space::new().width(Length::Fill),
                ]
                .spacing(6)
                .align_y(Alignment::Center)
                .padding([3, 8]);

                let code_text = text(code)
                    .size((11.0 * scale).max(9.5))
                    .font(iced::Font::MONOSPACE)
                    .color(theme.text_primary());

                let code_content = container(code_text).padding([4, 8]).width(Length::Fill);

                let win_card = container(column![win_header, code_content].spacing(1))
                    .width(Length::Fill)
                    .style(move |_| theme::code_block_container_style(theme));

                col = col.push(win_card);
            },

            | CalloutSegment::Bullet { text: item_text } => {
                num_counter = 1;
                let bullet_dot = text("•").size((13.0 * scale).max(10.5)).color(accent_color);
                let text_elem = container(render_rich_text(
                    item_text,
                    (12.0 * scale).max(10.0),
                    theme.text_primary(),
                    theme,
                ))
                .width(Length::Fill);
                let bullet_row = row![bullet_dot, text_elem]
                    .spacing(6)
                    .width(Length::Fill)
                    .align_y(Alignment::Start);
                col = col.push(bullet_row);
            },

            | CalloutSegment::Numbered { num, text: item_text } => {
                let num_str = if num == "+" {
                    let s = format!("{num_counter}.");
                    num_counter += 1;
                    s
                } else if num.ends_with('.') {
                    num.to_string()
                } else {
                    format!("{num}.")
                };
                let num_label = text(num_str)
                    .size((11.5 * scale).max(9.5))
                    .color(accent_color);
                let text_elem = container(render_rich_text(
                    item_text,
                    (12.0 * scale).max(10.0),
                    theme.text_primary(),
                    theme,
                ))
                .width(Length::Fill);
                let num_row = row![num_label, text_elem]
                    .spacing(6)
                    .width(Length::Fill)
                    .align_y(Alignment::Start);
                col = col.push(num_row);
            },

            | CalloutSegment::Paragraph(para_text) => {
                num_counter = 1;
                let para_elem = render_rich_text(
                    para_text,
                    (12.0 * scale).max(10.0),
                    theme.text_primary(),
                    theme,
                );
                col = col.push(container(para_elem).width(Length::Fill));
            },
        }
    }

    col.into()
}

#[allow(clippy::too_many_arguments)]
fn render_callout_block<'a>(
    theme: AppTheme,
    slide_idx: usize,
    block_idx: usize,
    block: &'a WysiwygBlock,
    callee: &'a str,
    args: &'a str,
    raw: &'a str,
    scale: f32,
    transition: Option<&'a ElementTransition>,
) -> Element<'a, Message> {
    let data = parse_callout_data(callee, args);

    let default_accent = match data.kind {
        | "tip" | "hint" => Color::from_rgb(0.06, 0.72, 0.50),
        | "note" | "info" => Color::from_rgb(0.12, 0.55, 0.95),
        | "warning" | "caution" => Color::from_rgb(0.96, 0.62, 0.05),
        | "alert" | "danger" => Color::from_rgb(0.94, 0.27, 0.27),
        | "quote" => Color::from_rgb(0.65, 0.40, 0.95),
        | _ => theme.accent(),
    };

    let accent_color = data
        .stroke_color
        .and_then(|sc| parse_color_spec(sc, theme))
        .unwrap_or(default_accent);

    let label_str = match data.kind {
        | "tip" | "hint" => "TIP",
        | "note" | "info" => "NOTE",
        | "warning" | "caution" => "WARNING",
        | "alert" | "danger" => "ALERT",
        | "quote" => "QUOTE",
        | _ => "CALLOUT",
    };

    let pill_bar = container(
        Space::new()
            .width(Length::Fixed(3.5))
            .height(Length::Fixed(14.0)),
    )
    .style(move |_| {
        container::Style {
            background: Some(Background::Color(accent_color)),
            border: border::rounded(theme::RADIUS_FULL),
            ..container::Style::default()
        }
    });

    let badge = container(
        text(label_str)
            .size((9.0 * scale).max(8.0))
            .color(Color::WHITE)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::DEFAULT
            }),
    )
    .padding([1.5, 6.0])
    .style(move |_| {
        container::Style {
            background: Some(Background::Color(accent_color)),
            border: border::rounded(theme::RADIUS_XS),
            ..container::Style::default()
        }
    });

    let title_widget = if data.title.is_empty() {
        row![pill_bar, badge].spacing(6).align_y(Alignment::Center)
    } else {
        row![
            pill_bar,
            badge,
            text(data.title)
                .size((11.5 * scale).max(10.0))
                .color(theme.text_primary())
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..iced::Font::DEFAULT
                }),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
    };

    let r_modal = block.range();
    let raw_modal = raw.to_string();
    let id_modal = block.id().to_string();
    let curr_callee_modal = callee.to_string();
    let btn_modal = button(text("Edit").size(8))
        .padding([1, 6])
        .style(move |_t, _s| theme::primary_button_style(theme))
        .on_press(Message::OpenComplexModal {
            slide_idx,
            block_id: id_modal,
            range: r_modal,
            raw: raw_modal,
            callee: curr_callee_modal,
        });

    let btn_step = button(
        text(if let Some(tr) = transition {
            format!("⚡ Step {} • {}", tr.order, tr.effect)
        } else {
            "⚡ Step".to_string()
        })
        .size(8),
    )
    .padding([1, 6])
    .style(move |_t, _s| {
        if transition.is_some() {
            theme::primary_button_style(theme)
        } else {
            theme::subtle_button_style(theme, false)
        }
    })
    .on_press(Message::OpenElementTransitionModal { slide_idx, block_idx });

    let btn_code = button(text("</> Code").size(9))
        .padding([1, 5])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::ToggleBlockRawCode(block.id().to_string()));

    let top_bar = row![
        title_widget,
        Space::new().width(Length::Fill),
        btn_step,
        btn_modal,
        btn_code,
    ]
    .spacing(4)
    .width(Length::Fill)
    .align_y(Alignment::Center);

    let body_display: Element<'a, Message> =
        container(render_callout_body(data.body, theme, accent_color, scale))
            .width(Length::Fill)
            .padding([3, 4])
            .into();

    let mut content_col = column![top_bar, body_display]
        .spacing(6)
        .width(Length::Fill);

    if let Some(author) = data.attribution {
        let attr_row = row![
            Space::new().width(Length::Fill),
            text(format!("— {author}"))
                .size(10)
                .color(theme.text_muted()),
        ]
        .width(Length::Fill);
        content_col = content_col.push(attr_row);
    }

    container(content_col)
        .width(Length::Fill)
        .padding([8, 12])
        .style(move |_| theme::callout_container_style(theme, accent_color))
        .into()
}

#[allow(clippy::too_many_arguments)]
fn render_box_block<'a>(
    theme: AppTheme,
    slide_idx: usize,
    block_idx: usize,
    block: &'a WysiwygBlock,
    callee: &'a str,
    args: &'a str,
    raw: &'a str,
    scale: f32,
    transition: Option<&'a ElementTransition>,
) -> Element<'a, Message> {
    let body_text = parse_box_data(raw);

    let badge = text(callee.to_string()).size(10).color(theme.accent());

    let r_modal = block.range();
    let raw_modal = raw.to_string();
    let id_modal = block.id().to_string();
    let btn_modal = button(text("Edit Box").size(9))
        .padding([1, 6])
        .style(move |_t, _s| theme::primary_button_style(theme))
        .on_press(Message::OpenComplexModal {
            slide_idx,
            block_id: id_modal,
            range: r_modal,
            raw: raw_modal,
            callee: callee.to_string(),
        });

    let btn_step = button(
        text(if let Some(tr) = transition {
            format!("⚡ Step {} • {}", tr.order, tr.effect)
        } else {
            "⚡ Step".to_string()
        })
        .size(9),
    )
    .padding([1, 6])
    .style(move |_t, _s| {
        if transition.is_some() {
            theme::primary_button_style(theme)
        } else {
            theme::subtle_button_style(theme, false)
        }
    })
    .on_press(Message::OpenElementTransitionModal { slide_idx, block_idx });

    let btn_code = button(text("</> Code").size(9))
        .padding([1, 5])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::ToggleBlockRawCode(block.id().to_string()));

    let style_info = if args.contains("blue") || args.contains("3b82f6") {
        Some("Blue")
    } else if args.contains("green") || args.contains("10b981") {
        Some("Green")
    } else if args.contains("amber") || args.contains("f59e0b") {
        Some("Amber")
    } else if args.contains("red") || args.contains("ef4444") {
        Some("Red")
    } else {
        None
    };

    let mut top_bar = row![badge]
        .spacing(4)
        .align_y(Alignment::Center)
        .padding([2, 4]);
    if let Some(style_name) = style_info {
        top_bar = top_bar.push(
            text(format!("• {style_name}"))
                .size(9)
                .color(theme.text_muted()),
        );
    }
    top_bar = top_bar.push(Space::new().width(Length::Fill));
    top_bar = top_bar.push(btn_step);
    top_bar = top_bar.push(btn_modal);
    top_bar = top_bar.push(btn_code);

    let body_display: Element<'a, Message> = container(render_rich_text(
        body_text,
        (12.0 * scale).max(10.0),
        theme.text_primary(),
        theme,
    ))
    .width(Length::Fill)
    .padding([4, 8])
    .into();

    let (bg_color, border_color) =
        if args.contains("blue") || args.contains("3b82f6") || args.contains("eff6ff") {
            (
                Color::from_rgba(0.2, 0.5, 0.9, 0.08),
                Color::from_rgba(0.2, 0.5, 0.9, 0.35),
            )
        } else if args.contains("green") || args.contains("10b981") || args.contains("f0fdf4") {
            (
                Color::from_rgba(0.1, 0.7, 0.3, 0.08),
                Color::from_rgba(0.1, 0.7, 0.3, 0.35),
            )
        } else if args.contains("amber") || args.contains("f59e0b") || args.contains("fffbeb") {
            (
                Color::from_rgba(0.9, 0.6, 0.1, 0.08),
                Color::from_rgba(0.9, 0.6, 0.1, 0.35),
            )
        } else if args.contains("red") || args.contains("ef4444") || args.contains("fef2f2") {
            (
                Color::from_rgba(0.9, 0.2, 0.2, 0.08),
                Color::from_rgba(0.9, 0.2, 0.2, 0.35),
            )
        } else {
            (theme.bg_subtle(), theme.border_color())
        };

    container(column![top_bar, body_display].spacing(4))
        .width(Length::Fill)
        .padding([6, 10])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(bg_color)),
                border: Border {
                    color: border_color,
                    width: 1.0,
                    radius: border::Radius::from(6.0),
                },
                ..container::Style::default()
            }
        })
        .into()
}

#[allow(clippy::too_many_arguments)]
fn render_link_block<'a>(
    theme: AppTheme,
    slide_idx: usize,
    block_idx: usize,
    block: &'a WysiwygBlock,
    args: &'a str,
    raw: &'a str,
    scale: f32,
    transition: Option<&'a ElementTransition>,
) -> Element<'a, Message> {
    let url: &'a str = if let Some(s_idx) = args.find('"') {
        let after = &args[s_idx + 1..];
        if let Some(e_idx) = after.find('"') {
            &after[..e_idx]
        } else {
            args.trim()
                .trim_matches(|c| c == '(' || c == ')' || c == '"' || c == '\'')
        }
    } else {
        args.trim()
            .trim_matches(|c| c == '(' || c == ')' || c == '"' || c == '\'')
    };

    let label: &'a str = if let Some(first_bracket) = raw.find('[')
        && let Some(last_bracket) = raw.rfind(']')
        && first_bracket < last_bracket
    {
        raw[first_bracket + 1..last_bracket].trim()
    } else {
        url
    };

    let badge = text("Link")
        .size((11.0 * scale).max(10.0))
        .color(theme.accent());

    let r_modal = block.range();
    let raw_modal = raw.to_string();
    let id_modal = block.id().to_string();
    let btn_modal = button(text("Edit Link").size(9))
        .padding([1, 6])
        .style(move |_t, _s| theme::primary_button_style(theme))
        .on_press(Message::OpenComplexModal {
            slide_idx,
            block_id: id_modal,
            range: r_modal,
            raw: raw_modal,
            callee: "link".to_string(),
        });

    let btn_step = button(
        text(if let Some(tr) = transition {
            format!("⚡ Step {} • {}", tr.order, tr.effect)
        } else {
            "⚡ Step".to_string()
        })
        .size(9),
    )
    .padding([1, 6])
    .style(move |_t, _s| {
        if transition.is_some() {
            theme::primary_button_style(theme)
        } else {
            theme::subtle_button_style(theme, false)
        }
    })
    .on_press(Message::OpenElementTransitionModal { slide_idx, block_idx });

    let btn_open = button(text("Open").size(9))
        .padding([1, 6])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::OpenUrl(url.to_string()));

    let btn_code = button(text("</> Code").size(9))
        .padding([1, 5])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::ToggleBlockRawCode(block.id().to_string()));

    let top_bar = row![
        badge,
        Space::new().width(Length::Fill),
        btn_step,
        btn_open,
        btn_modal,
        btn_code,
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .padding([2, 4]);

    let display_title = if label.is_empty() {
        url
    } else {
        label
    };
    let body_display = row![
        column![
            text(display_title)
                .size((13.0 * scale).max(11.0))
                .color(theme.text_primary()),
            text(url)
                .size((10.0 * scale).max(9.0))
                .font(iced::Font::MONOSPACE)
                .color(theme.accent().scale_alpha(0.85)),
        ]
        .spacing(2)
        .width(Length::Fill),
    ]
    .align_y(Alignment::Center)
    .padding([2, 6]);

    container(column![top_bar, body_display].spacing(4))
        .width(Length::Fill)
        .padding([6, 10])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(Color::from_rgba(0.2, 0.5, 0.9, 0.05))),
                border: Border {
                    color: Color::from_rgba(0.2, 0.5, 0.9, 0.25),
                    width: 1.0,
                    radius: border::Radius::from(6.0),
                },
                ..container::Style::default()
            }
        })
        .into()
}

fn extract_badge_label(args: &str) -> String {
    if let Some(s) = args.find('"') {
        let after = &args[s + 1..];
        if let Some(e) = after.find('"') {
            return after[..e].to_string();
        }
    }
    if let Some(s) = args.find('\'') {
        let after = &args[s + 1..];
        if let Some(e) = after.find('\'') {
            return after[..e].to_string();
        }
    }
    crate::model::ast_engine::extract_arg_str(args, "label").unwrap_or_else(|| "Badge".to_string())
}

#[allow(clippy::too_many_arguments)]
fn render_badge_block<'a>(
    theme: AppTheme,
    slide_idx: usize,
    block_idx: usize,
    block: &'a WysiwygBlock,
    callee: &'a str,
    args: &'a str,
    raw: &'a str,
    scale: f32,
    transition: Option<&'a ElementTransition>,
) -> Element<'a, Message> {
    let label = extract_badge_label(args);
    let fill = crate::model::ast_engine::extract_arg_str(args, "fill")
        .unwrap_or_else(|| "slide-colors.accent".to_string());

    let pill_color = if fill.contains("cyan") {
        Color::from_rgb(0.0, 0.75, 0.85)
    } else if fill.contains("purple") {
        Color::from_rgb(0.68, 0.42, 0.98)
    } else if fill.contains("orange") {
        Color::from_rgb(0.98, 0.55, 0.20)
    } else if fill.contains("green") {
        Color::from_rgb(0.20, 0.80, 0.45)
    } else if fill.contains("red") {
        Color::from_rgb(0.95, 0.28, 0.35)
    } else {
        theme.accent()
    };

    let badge_type_label = if callee == "pill" {
        "Pill"
    } else {
        "Badge"
    };
    let badge_chip = container(
        text(label)
            .size((12.0 * scale).max(10.0))
            .color(Color::WHITE),
    )
    .padding([3, 12])
    .style(move |_| {
        container::Style {
            background: Some(Background::Color(pill_color)),
            border: Border {
                color: pill_color,
                width: 1.0,
                radius: border::Radius::from(theme::RADIUS_FULL),
            },
            shadow: iced::Shadow {
                color: pill_color.scale_alpha(0.35),
                offset: iced::Vector::new(0.0, 1.5),
                blur_radius: 6.0,
            },
            ..container::Style::default()
        }
    });

    let btn_modal = button(text("Edit Badge").size(9))
        .padding([1, 6])
        .style(move |_t, _s| theme::primary_button_style(theme))
        .on_press(Message::OpenComplexModal {
            slide_idx,
            block_id: block.id().to_string(),
            range: block.range(),
            raw: raw.to_string(),
            callee: callee.to_string(),
        });

    let btn_step = button(
        text(if let Some(tr) = transition {
            format!("⚡ Step {} • {}", tr.order, tr.effect)
        } else {
            "⚡ Step".to_string()
        })
        .size(9),
    )
    .padding([1, 6])
    .style(move |_t, _s| {
        if transition.is_some() {
            theme::primary_button_style(theme)
        } else {
            theme::subtle_button_style(theme, false)
        }
    })
    .on_press(Message::OpenElementTransitionModal { slide_idx, block_idx });

    let btn_code = button(text("</> Code").size(9))
        .padding([1, 5])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::ToggleBlockRawCode(block.id().to_string()));

    let top_bar = row![
        text(format!("[{badge_type_label}]"))
            .size((10.0 * scale).max(9.0))
            .color(theme.accent()),
        Space::new().width(Length::Fill),
        btn_step,
        btn_modal,
        btn_code,
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .padding([2, 4]);

    let body_display = row![
        badge_chip,
        Space::new().width(Length::Fixed(8.0)),
        text(format!("Fill: {fill}"))
            .size((11.0 * scale).max(9.0))
            .color(theme.text_muted()),
    ]
    .align_y(Alignment::Center)
    .padding([4, 6]);

    container(column![top_bar, body_display].spacing(4))
        .width(Length::Fill)
        .padding([6, 10])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(Color::from_rgba(0.2, 0.5, 0.9, 0.05))),
                border: Border {
                    color: Color::from_rgba(0.2, 0.5, 0.9, 0.25),
                    width: 1.0,
                    radius: border::Radius::from(6.0),
                },
                ..container::Style::default()
            }
        })
        .into()
}


#[derive(Debug, Clone, PartialEq, Default)]
pub struct TitleSlideData {
    pub title: String,
    pub subtitle: String,
    pub author: String,
    pub date: String,
    pub version: String,
    pub institution: String,
    pub extra_args: Vec<(String, String)>,
    pub body: String,
}

#[must_use]
pub fn parse_title_slide_data(raw: &str) -> TitleSlideData {
    use crate::model::ast_engine::extract_arg_str;

    let title = extract_arg_str(raw, "title").unwrap_or_default();
    let subtitle = extract_arg_str(raw, "subtitle").unwrap_or_default();
    let author = extract_arg_str(raw, "author").unwrap_or_default();
    let date = extract_arg_str(raw, "date").unwrap_or_default();
    let version = extract_arg_str(raw, "version").unwrap_or_default();
    let institution = extract_arg_str(raw, "institution")
        .or_else(|| extract_arg_str(raw, "affiliation"))
        .or_else(|| extract_arg_str(raw, "org"))
        .unwrap_or_default();

    let mut extra_args = Vec::new();
    let args_slice = if let Some(first_paren) = raw.find('(')
        && let Some(last_paren) = raw.rfind(')')
        && last_paren > first_paren
    {
        &raw[first_paren + 1..last_paren]
    } else {
        ""
    };

    for part in args_slice.split(',') {
        let trimmed = part.trim();
        if let Some((k, v)) = trimmed.split_once(':') {
            let k = k.trim();
            let v = v.trim().trim_matches('"').trim_matches('\'');
            if !k.is_empty()
                && !matches!(
                    k,
                    "title"
                        | "subtitle"
                        | "author"
                        | "date"
                        | "version"
                        | "institution"
                        | "affiliation"
                        | "org"
                )
            {
                extra_args.push((k.to_string(), v.to_string()));
            }
        }
    }

    let body = if let Some(first_bracket) = raw.find('[')
        && let Some(last_bracket) = raw.rfind(']')
        && last_bracket > first_bracket
    {
        raw[first_bracket + 1..last_bracket].trim().to_string()
    } else {
        String::new()
    };

    TitleSlideData {
        title,
        subtitle,
        author,
        date,
        version,
        institution,
        extra_args,
        body,
    }
}

#[must_use]
pub fn update_title_slide_param(
    raw: &str,
    key: &str,
    new_val: &str,
) -> String {
    let pattern = format!("{key}:");
    if let Some(pos) = raw.find(&pattern) {
        let after_key = pos + pattern.len();
        let rest = &raw[after_key..];
        let trimmed_rest = rest.trim_start();
        let leading_spaces = rest.len() - trimmed_rest.len();
        let val_start = after_key + leading_spaces;

        if let Some(stripped) = trimmed_rest.strip_prefix('"') {
            // Quoted string
            if let Some(end_quote_idx) = stripped.find('"') {
                let quote_content_start = val_start + 1;
                let quote_content_end = val_start + 1 + end_quote_idx;
                let mut updated = raw.to_string();
                updated.replace_range(quote_content_start..quote_content_end, new_val);
                return updated;
            }
        } else {
            // Unquoted string / ident / none
            let end_val = trimmed_rest
                .find([',', '\n', ')'])
                .unwrap_or(trimmed_rest.len());
            let val_end = val_start + end_val;
            let mut updated = raw.to_string();
            updated.replace_range(val_start..val_end, &format!("\"{new_val}\""));
            return updated;
        }
    }

    // Key not found in raw!
    if new_val.trim().is_empty() {
        return raw.to_string();
    }

    // Insert before the matching closing parenthesis of the function call
    if let Some(last_paren) = raw.rfind(')') {
        let before_paren = &raw[..last_paren];
        let after_paren = &raw[last_paren..];

        let mut updated = String::new();
        updated.push_str(before_paren);

        let trimmed_before = before_paren.trim_end();
        let needs_comma = !trimmed_before.ends_with(',') && !trimmed_before.ends_with('(');

        if before_paren.contains('\n') {
            if needs_comma {
                updated.push(',');
            }
            updated.push('\n');
            updated.push_str(&format!("  {key}: \"{new_val}\",\n"));
        } else {
            if needs_comma {
                updated.push_str(", ");
            }
            updated.push_str(&format!("{key}: \"{new_val}\""));
        }
        updated.push_str(after_paren);
        return updated;
    }

    raw.to_string()
}

#[allow(clippy::too_many_arguments)]
fn render_title_slide_block<'a>(
    theme: AppTheme,
    slide_idx: usize,
    block_idx: usize,
    block: &'a WysiwygBlock,
    callee: &'a str,
    _args: &'a str,
    raw: &'a str,
    scale: f32,
    transition: Option<&'a ElementTransition>,
) -> Element<'a, Message> {
    let data = parse_title_slide_data(raw);

    let badge = text(format!("#{callee}"))
        .size((11.0 * scale).max(10.0))
        .color(theme.accent());

    let r_modal = block.range();
    let raw_modal = raw.to_string();
    let id_modal = block.id().to_string();
    let btn_modal = button(text("Edit Title Slide").size(9))
        .padding([2, 8])
        .style(move |_t, _s| theme::primary_button_style(theme))
        .on_press(Message::OpenComplexModal {
            slide_idx,
            block_id: id_modal,
            range: r_modal,
            raw: raw_modal,
            callee: callee.to_string(),
        });

    let btn_step = button(
        text(if let Some(tr) = transition {
            format!("⚡ Step {} • {}", tr.order, tr.effect)
        } else {
            "⚡ Step".to_string()
        })
        .size(9),
    )
    .padding([2, 8])
    .style(move |_t, _s| {
        if transition.is_some() {
            theme::primary_button_style(theme)
        } else {
            theme::subtle_button_style(theme, false)
        }
    })
    .on_press(Message::OpenElementTransitionModal { slide_idx, block_idx });

    let btn_code = button(text("</> Code").size(9))
        .padding([2, 6])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::ToggleBlockRawCode(block.id().to_string()));

    let mut top_bar = row![badge]
        .spacing(6)
        .align_y(Alignment::Center)
        .padding([2, 4]);
    if !data.version.is_empty() {
        top_bar = top_bar.push(
            container(
                text(format!("v{}", data.version))
                    .size(9)
                    .color(theme.text_secondary()),
            )
            .padding([1, 5])
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.bg_subtle())),
                    border: border::rounded(3.0),
                    ..container::Style::default()
                }
            }),
        );
    }
    top_bar = top_bar.push(Space::new().width(Length::Fill));
    top_bar = top_bar.push(btn_step);
    top_bar = top_bar.push(btn_modal);
    top_bar = top_bar.push(btn_code);

    let title_str = if data.title.is_empty() {
        "Untitled Presentation".to_string()
    } else {
        data.title
    };
    let title_display = text(title_str)
        .size((22.0 * scale).max(16.0))
        .color(theme.accent());

    let subtitle_display = if !data.subtitle.is_empty() {
        Some(
            text(data.subtitle)
                .size((13.5 * scale).max(11.0))
                .color(theme.text_secondary()),
        )
    } else {
        None
    };

    let divider = container(Space::new().height(1.0))
        .width(Length::Fill)
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.border_color())),
                ..container::Style::default()
            }
        });

    let mut meta_row = row![].spacing(14).align_y(Alignment::Center);

    if !data.author.is_empty() {
        meta_row = meta_row.push(
            row![
                text("Speaker:")
                    .size((10.0 * scale).max(9.0))
                    .color(theme.text_secondary()),
                text(data.author)
                    .size((11.5 * scale).max(9.5))
                    .color(theme.text_primary()),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        );
    }

    if !data.date.is_empty() {
        meta_row = meta_row.push(
            row![
                text("Date:")
                    .size((10.0 * scale).max(9.0))
                    .color(theme.text_secondary()),
                text(data.date)
                    .size((11.5 * scale).max(9.5))
                    .color(theme.text_secondary()),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        );
    }

    if !data.version.is_empty() {
        meta_row = meta_row.push(
            row![
                text("Ver:")
                    .size((10.0 * scale).max(9.0))
                    .color(theme.text_secondary()),
                text(data.version)
                    .size((11.0 * scale).max(9.0))
                    .color(theme.text_secondary()),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        );
    }

    if !data.institution.is_empty() {
        meta_row = meta_row.push(
            row![
                text("Org:")
                    .size((10.0 * scale).max(9.0))
                    .color(theme.text_secondary()),
                text(data.institution)
                    .size((11.0 * scale).max(9.0))
                    .color(theme.text_secondary()),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        );
    }

    let mut hero_col = column![title_display]
        .spacing((6.0 * scale).max(4.0))
        .width(Length::Fill);

    if let Some(sub) = subtitle_display {
        hero_col = hero_col.push(sub);
    }
    hero_col = hero_col.push(divider);
    hero_col = hero_col.push(meta_row);

    if !data.extra_args.is_empty() {
        let mut extra_row = row![].spacing(6).align_y(Alignment::Center);
        for (k, v) in &data.extra_args {
            extra_row = extra_row.push(
                container(text(format!("{k}: {v}")).size(9).color(theme.text_muted()))
                    .padding([1, 4])
                    .style(move |_| {
                        container::Style {
                            background: Some(Background::Color(theme.bg_subtle())),
                            border: border::rounded(3.0),
                            ..container::Style::default()
                        }
                    }),
            );
        }
        hero_col = hero_col.push(extra_row);
    }

    let body_slice: Option<&'a str> = if let Some(first_bracket) = raw.find('[')
        && let Some(last_bracket) = raw.rfind(']')
        && last_bracket > first_bracket
    {
        let slice = raw[first_bracket + 1..last_bracket].trim();
        if !slice.is_empty() {
            Some(slice)
        } else {
            None
        }
    } else {
        None
    };

    if let Some(body_text) = body_slice {
        let body_display = render_rich_text(
            body_text,
            (12.0 * scale).max(10.0),
            theme.text_primary(),
            theme,
        );
        hero_col = hero_col.push(container(body_display).padding([4, 6]));
    }

    let hero_card = container(hero_col)
        .width(Length::Fill)
        .padding([(10.0 * scale).max(8.0), (14.0 * scale).max(10.0)])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.bg_subtle())),
                border: Border {
                    color: theme.accent().scale_alpha(0.35),
                    width: 1.0,
                    radius: border::Radius::from(6.0),
                },
                ..container::Style::default()
            }
        });

    container(column![top_bar, hero_card].spacing(4))
        .width(Length::Fill)
        .padding([4, 6])
        .into()
}

#[allow(clippy::too_many_arguments)]
fn render_chart_block<'a>(
    theme: AppTheme,
    slide_idx: usize,
    block_idx: usize,
    block: &'a WysiwygBlock,
    callee: &'a str,
    args: &'a str,
    raw: &'a str,
    scale: f32,
    transition: Option<&'a ElementTransition>,
) -> Element<'a, Message> {
    let data = parse_chart_data(args);
    let is_pie = callee.contains("pie");
    let is_line = callee.contains("line") || callee == "plot";

    let title_label = if is_pie {
        "Pie"
    } else if is_line {
        "Line"
    } else {
        "Bar"
    };

    let r_title = block.range();
    let raw_title = raw.to_string();
    let title_input = text_input("Chart Title", data.title)
        .size((11.0 * scale).max(10.0))
        .padding([2, 5])
        .width(Length::Fixed(120.0 * scale.max(1.0)))
        .style(move |_t, _s| bordered_input_style(theme, theme.text_primary()))
        .on_input(move |new_title| {
            Message::UpdateBlockRange {
                slide_idx: Some(slide_idx),
                range: r_title.clone(),
                new_text: update_chart_title(&raw_title, &new_title),
            }
        });

    let title_row = row![
        text(title_label).size(10).color(theme.accent()),
        title_input,
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let make_switch_btn = |new_macro: &'static str, label: &'static str| {
        let r = block.range();
        let raw_str = raw.to_string();
        button(text(label).size(8))
            .padding([1, 4])
            .style(move |_t, _s| theme::subtle_button_style(theme, false))
            .on_press(Message::UpdateBlockRange {
                slide_idx: Some(slide_idx),
                range: r,
                new_text: switch_chart_kind(&raw_str, new_macro),
            })
    };

    let range_add = block.range();
    let raw_add = raw.to_string();
    let btn_add_pt = button(text("+ Item").size(9))
        .padding([1, 5])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::UpdateBlockRange {
            slide_idx: Some(slide_idx),
            range: range_add,
            new_text: add_data_point_to_chart(&raw_add),
        });

    let r_modal = block.range();
    let raw_modal = raw.to_string();
    let id_modal = block.id().to_string();
    let curr_callee_modal = callee.to_string();
    let btn_modal = button(text("Edit").size(9))
        .padding([1, 6])
        .style(move |_t, _s| theme::primary_button_style(theme))
        .on_press(Message::OpenComplexModal {
            slide_idx,
            block_id: id_modal,
            range: r_modal,
            raw: raw_modal,
            callee: curr_callee_modal,
        });

    let btn_step = button(
        text(if let Some(tr) = transition {
            format!("⚡ Step {} • {}", tr.order, tr.effect)
        } else {
            "⚡ Step".to_string()
        })
        .size(9),
    )
    .padding([1, 6])
    .style(move |_t, _s| {
        if transition.is_some() {
            theme::primary_button_style(theme)
        } else {
            theme::subtle_button_style(theme, false)
        }
    })
    .on_press(Message::OpenElementTransitionModal { slide_idx, block_idx });

    let btn_code = button(text("</> Code").size(9))
        .padding([1, 5])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::ToggleBlockRawCode(block.id().to_string()));

    let top_bar = row![
        title_row,
        Space::new().width(Length::Fill),
        btn_step,
        btn_modal,
        make_switch_btn("#chart-bar", "Bar"),
        make_switch_btn("#chart-line", "Line"),
        make_switch_btn("#chart-pie", "Pie"),
        btn_add_pt,
        btn_code,
    ]
    .spacing(3)
    .align_y(Alignment::Center);

    let palette = [
        Color::from_rgb(0.23, 0.51, 0.96),
        Color::from_rgb(0.06, 0.73, 0.83),
        Color::from_rgb(0.55, 0.36, 0.96),
        Color::from_rgb(0.96, 0.62, 0.05),
        Color::from_rgb(0.06, 0.72, 0.50),
        Color::from_rgb(0.96, 0.25, 0.37),
    ];

    // 1. Chart Graphic (Pie or Bar)
    let chart_graphic: Element<'a, Message> = if is_pie {
        let total: f32 = data.items.iter().map(|(_, v)| *v).sum();
        let total = if total == 0.0 { 1.0 } else { total };

        let mut bar_row = row![]
            .spacing(2)
            .width(Length::Fill)
            .height(Length::Fixed(16.0));
        let mut legend_row = row![]
            .spacing(8)
            .width(Length::Fill)
            .align_y(Alignment::Center);

        for (i, (lbl, val)) in data.items.iter().enumerate() {
            let color = palette[i % palette.len()];
            let portion = ((*val / total) * 100.0).max(1.0) as u16;

            let segment = container(Space::new())
                .width(Length::FillPortion(portion))
                .height(Length::Fill)
                .style(move |_| {
                    container::Style {
                        background: Some(Background::Color(color)),
                        border: Border {
                            radius: border::Radius::from(2.0),
                            ..Border::default()
                        },
                        ..container::Style::default()
                    }
                });
            bar_row = bar_row.push(segment);

            let pct = (*val / total) * 100.0;
            let leg_item = row![
                container(Space::new())
                    .width(Length::Fixed(8.0))
                    .height(Length::Fixed(8.0))
                    .style(move |_| {
                        container::Style {
                            background: Some(Background::Color(color)),
                            border: Border {
                                radius: border::Radius::from(4.0),
                                ..Border::default()
                            },
                            ..container::Style::default()
                        }
                    }),
                text(format!("{lbl} ({pct:.0}%)"))
                    .size(9)
                    .color(theme.text_secondary()),
            ]
            .spacing(4)
            .align_y(Alignment::Center);

            legend_row = legend_row.push(leg_item);
        }

        column![bar_row, legend_row].spacing(6).into()
    } else {
        let max_val: f32 = data.items.iter().map(|(_, v)| *v).fold(0.0, f32::max);
        let max_val = if max_val == 0.0 {
            1.0
        } else {
            max_val
        };

        let mut bars_col = column![].spacing(4).width(Length::Fill);

        for (i, (lbl, val)) in data.items.iter().enumerate() {
            let color = palette[i % palette.len()];
            let ratio = (*val / max_val).clamp(0.04, 1.0);
            let bar_portion = (ratio * 100.0) as u16;
            let remain_portion = (((1.0 - ratio) * 100.0).max(1.0)) as u16;

            let lbl_widget = container(text(*lbl).size(10).color(theme.text_secondary()))
                .width(Length::Fixed(60.0));

            let bar_fill = container(Space::new())
                .width(Length::FillPortion(bar_portion))
                .height(Length::Fixed(12.0))
                .style(move |_| {
                    container::Style {
                        background: Some(Background::Color(color)),
                        border: Border {
                            radius: border::Radius::from(3.0),
                            ..Border::default()
                        },
                        ..container::Style::default()
                    }
                });

            let bar_empty = Space::new().width(Length::FillPortion(remain_portion));
            let bar_container = row![bar_fill, bar_empty]
                .width(Length::Fill)
                .align_y(Alignment::Center);

            let val_pill = container(text(format!("{val:.0}")).size(9).color(color))
                .padding([1, 4])
                .style(move |_| {
                    container::Style {
                        background: Some(Background::Color(Color { a: 0.12, ..color })),
                        border: Border {
                            radius: border::Radius::from(3.0),
                            ..Border::default()
                        },
                        ..container::Style::default()
                    }
                });

            let item_row = row![lbl_widget, bar_container, val_pill]
                .spacing(6)
                .width(Length::Fill)
                .align_y(Alignment::Center);

            bars_col = bars_col.push(item_row);
        }

        bars_col.into()
    };

    // 2. Interactive In-Place Data Editor List
    let mut data_edit_col = column![].spacing(3).width(Length::Fill);
    for (i, (lbl, val)) in data.items.iter().enumerate() {
        let color = palette[i % palette.len()];

        let dot = container(Space::new())
            .width(Length::Fixed(8.0))
            .height(Length::Fixed(8.0))
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(color)),
                    border: Border {
                        radius: border::Radius::from(4.0),
                        ..Border::default()
                    },
                    ..container::Style::default()
                }
            });

        let r_lbl = block.range();
        let raw_lbl = raw.to_string();
        let lbl_input = text_input("Label", lbl)
            .size(9)
            .padding([2, 4])
            .width(Length::Fixed(80.0))
            .style(move |_t, _s| bordered_input_style(theme, theme.text_primary()))
            .on_input(move |new_lbl| {
                Message::UpdateBlockRange {
                    slide_idx: Some(slide_idx),
                    range: r_lbl.clone(),
                    new_text: update_chart_item_label(&raw_lbl, i, &new_lbl),
                }
            });

        let r_val = block.range();
        let raw_val = raw.to_string();
        let val_input = text_input("Val", &format!("{val:.0}"))
            .size(9)
            .padding([2, 4])
            .width(Length::Fixed(50.0))
            .style(move |_t, _s| bordered_input_style(theme, color))
            .on_input(move |new_v| {
                if let Ok(num) = new_v.parse::<f32>() {
                    Message::UpdateBlockRange {
                        slide_idx: Some(slide_idx),
                        range: r_val.clone(),
                        new_text: update_chart_item_value(&raw_val, i, num),
                    }
                } else {
                    Message::UpdateBlockRange {
                        slide_idx: Some(slide_idx),
                        range: r_val.clone(),
                        new_text: raw_val.clone(),
                    }
                }
            });

        let mut row_item = row![dot, lbl_input, val_input]
            .spacing(4)
            .align_y(Alignment::Center);

        if data.items.len() > 1 {
            let r_del = block.range();
            let raw_del = raw.to_string();
            let btn_del = button(text("x").size(8))
                .padding([1, 4])
                .style(move |_t, _s| theme::subtle_button_style(theme, false))
                .on_press(Message::UpdateBlockRange {
                    slide_idx: Some(slide_idx),
                    range: r_del,
                    new_text: chart_delete_item(&raw_del, i),
                });
            row_item = row_item.push(btn_del);
        }

        data_edit_col = data_edit_col.push(row_item);
    }

    container(column![top_bar, chart_graphic, data_edit_col].spacing(6))
        .width(Length::Fill)
        .padding([6, 10])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.bg_subtle())),
                border: Border {
                    color: theme.border_color(),
                    width: 1.0,
                    radius: border::Radius::from(6.0),
                },
                ..container::Style::default()
            }
        })
        .into()
}

/// Parsed data for #video(...)
#[derive(Debug, Clone, PartialEq)]
pub struct VideoData<'a> {
    pub source: &'a str,
    pub caption: Option<&'a str>,
    pub duration: Option<&'a str>,
    pub quality: &'a str,
    pub style: &'a str,
    pub width: &'a str,
}

#[must_use]
pub fn parse_video_data<'a>(args: &'a str) -> VideoData<'a> {
    let trimmed = args.trim().trim_start_matches('(').trim_end_matches(')');
    let source = if let Some(s_idx) = trimmed.find('"') {
        let after = &trimmed[s_idx + 1..];
        if let Some(e_idx) = after.find('"') {
            &after[..e_idx]
        } else {
            "assets/demo.mp4"
        }
    } else {
        let first = trimmed
            .split(',')
            .next()
            .unwrap_or("assets/demo.mp4")
            .trim()
            .trim_matches('"')
            .trim_matches('\'');
        if first.is_empty() || is_named_arg(first) {
            "assets/demo.mp4"
        } else {
            first
        }
    };

    let caption = extract_named_string(args, "caption");
    let duration = extract_named_string(args, "duration");
    let quality = extract_named_string(args, "quality").unwrap_or("4K 60FPS");
    let style = extract_named_string(args, "style").unwrap_or("glass");
    let width = extract_named_string(args, "width").unwrap_or("90%");

    VideoData {
        source,
        caption,
        duration,
        quality,
        style,
        width,
    }
}

/// Parsed data for #audio(...) and #audio-player(...)
#[derive(Debug, Clone, PartialEq)]
pub struct AudioData<'a> {
    pub source: &'a str,
    pub is_player: bool,
    pub title: &'a str,
    pub artist: &'a str,
    pub autoplay: bool,
    pub loop_playback: bool,
    pub volume: f32,
    pub width: &'a str,
}

#[must_use]
pub fn parse_audio_data<'a>(
    callee: &str,
    args: &'a str,
) -> AudioData<'a> {
    let is_player = callee == "audio-player";
    let trimmed = args.trim().trim_start_matches('(').trim_end_matches(')');
    let source = if let Some(s_idx) = trimmed.find('"') {
        let after = &trimmed[s_idx + 1..];
        if let Some(e_idx) = after.find('"') {
            &after[..e_idx]
        } else {
            "assets/soundtrack.mp3"
        }
    } else {
        let first = trimmed
            .split(',')
            .next()
            .unwrap_or("assets/soundtrack.mp3")
            .trim()
            .trim_matches('"')
            .trim_matches('\'');
        if first.is_empty() || is_named_arg(first) {
            "assets/soundtrack.mp3"
        } else {
            first
        }
    };

    let title = extract_named_string(args, "title").unwrap_or("Background Music");
    let artist = extract_named_string(args, "artist").unwrap_or("cargo-slide soundtrack");
    let autoplay = if let Some(pos) = args.find("autoplay:") {
        let after = args[pos + 9..].trim_start();
        after.starts_with("true")
    } else {
        !is_player
    };
    let loop_playback = if let Some(pos) = args.find("loop:") {
        let after = args[pos + 5..].trim_start();
        !after.starts_with("false")
    } else {
        true
    };
    let volume = if let Some(pos) = args.find("volume:") {
        let after = args[pos + 7..].trim_start();
        let end = after.find([',', ')', '\n']).unwrap_or(after.len());
        after[..end].trim().parse::<f32>().unwrap_or(0.8)
    } else {
        0.8
    };
    let width = extract_named_string(args, "width").unwrap_or("100%");

    AudioData {
        source,
        is_player,
        title,
        artist,
        autoplay,
        loop_playback,
        volume,
        width,
    }
}

#[allow(clippy::too_many_arguments)]
fn render_video_block<'a>(
    theme: AppTheme,
    slide_idx: usize,
    block_idx: usize,
    block: &'a WysiwygBlock,
    args: &'a str,
    raw: &'a str,
    scale: f32,
    transition: Option<&'a ElementTransition>,
) -> Element<'a, Message> {
    let data = parse_video_data(args);
    let display_title = data.caption.unwrap_or(data.source);

    let badge = row![
        text("Video")
            .size((11.0 * scale).max(10.0))
            .color(Color::from_rgb(0.98, 0.45, 0.09)),
        text(format!("({})", data.quality))
            .size((10.0 * scale).max(9.0))
            .color(theme.text_muted()),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let btn_modal = button(text("Edit Video").size(9))
        .padding([1, 6])
        .style(move |_t, _s| theme::primary_button_style(theme))
        .on_press(Message::OpenComplexModal {
            slide_idx,
            block_id: block.id().to_string(),
            range: block.range(),
            raw: raw.to_string(),
            callee: "video".to_string(),
        });

    let btn_step = button(
        text(if let Some(tr) = transition {
            format!("⚡ Step {} • {}", tr.order, tr.effect)
        } else {
            "⚡ Step".to_string()
        })
        .size(9),
    )
    .padding([1, 6])
    .style(move |_t, _s| {
        if transition.is_some() {
            theme::primary_button_style(theme)
        } else {
            theme::subtle_button_style(theme, false)
        }
    })
    .on_press(Message::OpenElementTransitionModal { slide_idx, block_idx });

    let btn_code = button(text("</> Code").size(9))
        .padding([1, 5])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::ToggleBlockRawCode(block.id().to_string()));

    let top_bar = row![
        badge,
        Space::new().width(Length::Fill),
        btn_step,
        btn_modal,
        btn_code,
    ]
    .spacing(4)
    .width(Length::Fill)
    .align_y(Alignment::Center);

    // Video visual card
    let is_cinema = data.style == "cinema";
    let is_minimal = data.style == "minimal";

    let card_content: Element<'a, Message> = if is_cinema {
        let rec = container(
            text("● REC")
                .size((9.0 * scale).max(8.0))
                .color(Color::from_rgb(0.95, 0.25, 0.25)),
        )
        .padding([2, 5])
        .style(|_| {
            container::Style {
                background: Some(Background::Color(Color::from_rgba(0.95, 0.25, 0.25, 0.2))),
                border: Border {
                    color: Color::from_rgb(0.95, 0.25, 0.25),
                    width: 1.0,
                    radius: border::Radius::from(3.0),
                },
                ..container::Style::default()
            }
        });

        let q_pill = container(
            text(data.quality)
                .size((9.0 * scale).max(8.0))
                .color(theme.text_secondary()),
        )
        .padding([2, 5])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.1))),
                border: border::rounded(3.0),
                ..container::Style::default()
            }
        });

        let mut meta_right = row![q_pill].spacing(4).align_y(Alignment::Center);
        if let Some(dur) = data.duration {
            let dur_pill = container(
                text(format!("⏱ {dur}"))
                    .size((9.0 * scale).max(8.0))
                    .color(theme.text_secondary()),
            )
            .padding([2, 5])
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.1))),
                    border: border::rounded(3.0),
                    ..container::Style::default()
                }
            });
            meta_right = meta_right.push(dur_pill);
        }

        let header =
            row![rec, Space::new().width(Length::Fill), meta_right].align_y(Alignment::Center);

        let play_btn = container(text("▶").size((18.0 * scale).max(14.0)).color(Color::WHITE))
            .width(Length::Fixed(36.0 * scale))
            .height(Length::Fixed(36.0 * scale))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(|_| {
                container::Style {
                    background: Some(Background::Color(Color::from_rgb(0.98, 0.45, 0.09))),
                    border: border::rounded(18.0),
                    ..container::Style::default()
                }
            });

        let center = column![
            play_btn,
            Space::new().height(Length::Fixed(3.0)),
            text(display_title)
                .size((13.0 * scale).max(11.0))
                .color(Color::WHITE),
            text(data.source)
                .size((9.0 * scale).max(8.0))
                .color(theme.text_muted()),
        ]
        .spacing(2)
        .align_x(Alignment::Center);

        container(column![header, Space::new().height(Length::Fixed(6.0)), center].spacing(4))
            .padding([8, 12])
            .width(Length::Fill)
            .style(|_| {
                container::Style {
                    background: Some(Background::Color(Color::from_rgb(0.03, 0.04, 0.06))),
                    border: Border {
                        color: Color::from_rgb(0.98, 0.45, 0.09),
                        width: 1.5,
                        radius: border::Radius::from(6.0),
                    },
                    ..container::Style::default()
                }
            })
            .into()
    } else if is_minimal {
        let play_btn = container(text("▶").size((12.0 * scale).max(10.0)).color(Color::WHITE))
            .width(Length::Fixed(24.0 * scale))
            .height(Length::Fixed(24.0 * scale))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.accent())),
                    border: border::rounded(12.0),
                    ..container::Style::default()
                }
            });

        let play_badge = container(
            text("PLAY")
                .size((9.0 * scale).max(8.0))
                .color(theme.accent()),
        )
        .padding([2, 6])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.bg_subtle())),
                border: border::rounded(3.0),
                ..container::Style::default()
            }
        });

        let body = row![
            play_btn,
            column![
                text(display_title)
                    .size((12.0 * scale).max(10.0))
                    .color(theme.text_primary()),
                text(data.source)
                    .size((9.0 * scale).max(8.0))
                    .color(theme.text_muted()),
            ]
            .spacing(2),
            Space::new().width(Length::Fill),
            play_badge,
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        container(body)
            .padding([6, 10])
            .width(Length::Fill)
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.bg_card())),
                    border: Border {
                        color: theme.border_color(),
                        width: 1.0,
                        radius: border::Radius::from(6.0),
                    },
                    ..container::Style::default()
                }
            })
            .into()
    } else {
        // Default "glass"
        let play_btn = container(text("▶").size((18.0 * scale).max(14.0)).color(Color::WHITE))
            .width(Length::Fixed(38.0 * scale))
            .height(Length::Fixed(38.0 * scale))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.accent())),
                    border: border::rounded(19.0),
                    ..container::Style::default()
                }
            });

        let q_badge = container(
            text(data.quality)
                .size((9.0 * scale).max(8.0))
                .color(theme.accent()),
        )
        .padding([2, 5])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.bg_subtle())),
                border: Border {
                    color: theme.border_active(),
                    width: 0.8,
                    radius: border::Radius::from(3.0),
                },
                ..container::Style::default()
            }
        });

        let mut badges = row![q_badge].spacing(4).align_y(Alignment::Center);
        if let Some(dur) = data.duration {
            let dur_pill = container(
                text(format!("⏱ {dur}"))
                    .size((9.0 * scale).max(8.0))
                    .color(theme.text_secondary()),
            )
            .padding([2, 5])
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
                    border: border::rounded(3.0),
                    ..container::Style::default()
                }
            });
            badges = badges.push(dur_pill);
        }

        let center = column![
            play_btn,
            Space::new().height(Length::Fixed(3.0)),
            text(display_title)
                .size((13.0 * scale).max(11.0))
                .color(theme.text_primary()),
            text(data.source)
                .size((9.0 * scale).max(8.0))
                .color(theme.text_muted()),
            Space::new().height(Length::Fixed(2.0)),
            badges,
        ]
        .spacing(2)
        .align_x(Alignment::Center);

        container(center)
            .padding([10, 14])
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.bg_card())),
                    border: Border {
                        color: theme.border_active(),
                        width: 1.2,
                        radius: border::Radius::from(6.0),
                    },
                    ..container::Style::default()
                }
            })
            .into()
    };

    container(column![top_bar, card_content].spacing(4))
        .width(Length::Fill)
        .padding([4, 6])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.bg_subtle().scale_alpha(0.5))),
                border: Border {
                    color: theme.border_subtle(),
                    width: 0.8,
                    radius: border::Radius::from(6.0),
                },
                ..container::Style::default()
            }
        })
        .into()
}

#[allow(clippy::too_many_arguments)]
fn render_audio_block<'a>(
    theme: AppTheme,
    slide_idx: usize,
    block_idx: usize,
    block: &'a WysiwygBlock,
    callee: &'a str,
    args: &'a str,
    raw: &'a str,
    scale: f32,
    transition: Option<&'a ElementTransition>,
) -> Element<'a, Message> {
    let data = parse_audio_data(callee, args);
    let vol_pct = (data.volume * 100.0).round() as usize;

    let badge = row![
        text(if data.is_player {
            "Audio Player"
        } else {
            "Audio Trigger"
        })
        .size((11.0 * scale).max(10.0))
        .color(Color::from_rgb(0.02, 0.71, 0.83)),
        text(format!("(vol: {vol_pct}%)"))
            .size((10.0 * scale).max(9.0))
            .color(theme.text_muted()),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let btn_modal = button(text("Edit Audio").size(9))
        .padding([1, 6])
        .style(move |_t, _s| theme::primary_button_style(theme))
        .on_press(Message::OpenComplexModal {
            slide_idx,
            block_id: block.id().to_string(),
            range: block.range(),
            raw: raw.to_string(),
            callee: callee.to_string(),
        });

    let btn_step = button(
        text(if let Some(tr) = transition {
            format!("⚡ Step {} • {}", tr.order, tr.effect)
        } else {
            "⚡ Step".to_string()
        })
        .size(9),
    )
    .padding([1, 6])
    .style(move |_t, _s| {
        if transition.is_some() {
            theme::primary_button_style(theme)
        } else {
            theme::subtle_button_style(theme, false)
        }
    })
    .on_press(Message::OpenElementTransitionModal { slide_idx, block_idx });

    let btn_code = button(text("</> Code").size(9))
        .padding([1, 5])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::ToggleBlockRawCode(block.id().to_string()));

    let top_bar = row![
        badge,
        Space::new().width(Length::Fill),
        btn_step,
        btn_modal,
        btn_code,
    ]
    .spacing(4)
    .width(Length::Fill)
    .align_y(Alignment::Center);

    let card_content: Element<'a, Message> = if data.is_player {
        let note = container(
            text("♫")
                .size((13.0 * scale).max(11.0))
                .color(Color::from_rgb(0.02, 0.71, 0.83)),
        )
        .width(Length::Fixed(28.0 * scale))
        .height(Length::Fixed(28.0 * scale))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(|_| {
            container::Style {
                background: Some(Background::Color(Color::from_rgba(0.02, 0.71, 0.83, 0.2))),
                border: Border {
                    color: Color::from_rgb(0.02, 0.71, 0.83),
                    width: 1.0,
                    radius: border::Radius::from(14.0),
                },
                ..container::Style::default()
            }
        });

        let bar = |h: f32| {
            container(Space::new())
                .width(Length::Fixed(2.5))
                .height(Length::Fixed(h))
                .style(|_| {
                    container::Style {
                        background: Some(Background::Color(Color::from_rgb(0.02, 0.71, 0.83))),
                        border: border::rounded(1.0),
                        ..container::Style::default()
                    }
                })
        };

        let eq = row![bar(6.0), bar(12.0), bar(8.0), bar(14.0), bar(5.0),]
            .spacing(2)
            .align_y(Alignment::End);

        let vol_badge = container(
            text(format!("🔊 {vol_pct}%"))
                .size((9.0 * scale).max(8.0))
                .color(Color::from_rgb(0.02, 0.71, 0.83)),
        )
        .padding([2, 5])
        .style(|_| {
            container::Style {
                background: Some(Background::Color(Color::from_rgba(0.02, 0.71, 0.83, 0.15))),
                border: border::rounded(3.0),
                ..container::Style::default()
            }
        });

        let auto_badge = container(
            text(if data.autoplay {
                "⚡ Auto"
            } else {
                "Manual"
            })
            .size((9.0 * scale).max(8.0))
            .color(theme.text_secondary()),
        )
        .padding([2, 4])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
                border: border::rounded(3.0),
                ..container::Style::default()
            }
        });

        let loop_badge = container(
            text(if data.loop_playback {
                "🔁 Loop"
            } else {
                "Once"
            })
            .size((9.0 * scale).max(8.0))
            .color(theme.text_secondary()),
        )
        .padding([2, 4])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
                border: border::rounded(3.0),
                ..container::Style::default()
            }
        });

        let right_items = row![eq, vol_badge, auto_badge, loop_badge]
            .spacing(5)
            .align_y(Alignment::Center);

        let row_body = row![
            note,
            column![
                text(data.title)
                    .size((12.0 * scale).max(10.0))
                    .color(theme.text_primary()),
                text(format!("{} • {}", data.artist, data.source))
                    .size((9.0 * scale).max(8.0))
                    .color(theme.text_muted()),
            ]
            .spacing(2),
            Space::new().width(Length::Fill),
            right_items,
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        container(row_body)
            .padding([8, 12])
            .width(Length::Fill)
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.bg_card())),
                    border: Border {
                        color: Color::from_rgb(0.02, 0.71, 0.83),
                        width: 1.2,
                        radius: border::Radius::from(6.0),
                    },
                    ..container::Style::default()
                }
            })
            .into()
    } else {
        let trigger_row = row![
            text("🔊").size((13.0 * scale).max(11.0)),
            column![
                text("Background Audio Trigger")
                    .size((11.0 * scale).max(9.5))
                    .color(theme.text_primary()),
                text(data.source)
                    .size((9.0 * scale).max(8.0))
                    .color(theme.text_muted()),
            ]
            .spacing(1),
            Space::new().width(Length::Fill),
            text(format!("Vol: {vol_pct}%"))
                .size((9.0 * scale).max(8.0))
                .color(Color::from_rgb(0.02, 0.71, 0.83)),
            text(if data.autoplay {
                "⚡ Auto"
            } else {
                "Manual"
            })
            .size((9.0 * scale).max(8.0))
            .color(theme.text_secondary()),
            text(if data.loop_playback {
                "🔁 Loop"
            } else {
                "Once"
            })
            .size((9.0 * scale).max(8.0))
            .color(theme.text_secondary()),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        container(trigger_row)
            .padding([6, 10])
            .width(Length::Fill)
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.bg_card())),
                    border: Border {
                        color: theme.border_color(),
                        width: 1.0,
                        radius: border::Radius::from(5.0),
                    },
                    ..container::Style::default()
                }
            })
            .into()
    };

    container(column![top_bar, card_content].spacing(4))
        .width(Length::Fill)
        .padding([4, 6])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.bg_subtle().scale_alpha(0.5))),
                border: Border {
                    color: theme.border_subtle(),
                    width: 0.8,
                    radius: border::Radius::from(6.0),
                },
                ..container::Style::default()
            }
        })
        .into()
}

/// Render visual presentation block (Typora typography, math, code, badges)
#[allow(clippy::too_many_arguments)]
pub fn view_visual_block<'a>(
    theme: AppTheme,
    slide_idx: usize,
    block_idx: usize,
    _total_blocks: usize,
    block: &'a WysiwygBlock,
    is_active: bool,
    scale: f32,
    equation_image: Option<&'a image::Handle>,
    code_image: Option<&'a image::Handle>,
    is_dragging: bool,
    spacing_pt: Option<f32>,
    transition: Option<&'a ElementTransition>,
) -> Element<'a, Message> {
    // Zero-wildcard AST block rendering!
    let inner_content: Element<'a, Message> = match block {
        | WysiwygBlock::Heading { level, title, .. } => {
            let (font_size, top_margin, bottom_margin, font_color) = match level {
                | 1 => {
                    (
                        (26.0 * scale).max(20.0),
                        10.0 * scale,
                        6.0 * scale,
                        theme.accent(),
                    )
                },
                | 2 => {
                    (
                        (21.0 * scale).max(17.0),
                        8.0 * scale,
                        5.0 * scale,
                        theme.text_primary(),
                    )
                },
                | 3 => {
                    (
                        (17.0 * scale).max(14.0),
                        6.0 * scale,
                        4.0 * scale,
                        theme.text_primary(),
                    )
                },
                | 4 => {
                    (
                        (14.5 * scale).max(12.5),
                        5.0 * scale,
                        3.0 * scale,
                        theme.text_secondary(),
                    )
                },
                | 5 => {
                    (
                        (13.0 * scale).max(11.5),
                        4.0 * scale,
                        2.0 * scale,
                        theme.text_secondary(),
                    )
                },
                | _ => {
                    (
                        (11.5 * scale).max(10.5),
                        3.0 * scale,
                        2.0 * scale,
                        theme.text_muted(),
                    )
                },
            };

            let heading_text = render_rich_text(title, font_size, font_color, theme);

            column![
                Space::new().height(Length::Fixed(top_margin)),
                heading_text,
                Space::new().height(Length::Fixed(bottom_margin)),
            ]
            .width(Length::Fill)
            .into()
        },

        | WysiwygBlock::ListItem { body, .. } => {
            let bullet_symbol = text("•")
                .size((14.0 * scale).max(11.0))
                .color(theme.accent());

            let item_text =
                render_rich_text(body, (14.0 * scale).max(12.0), theme.text_primary(), theme);

            row![bullet_symbol, container(item_text).width(Length::Fill)]
                .width(Length::Fill)
                .spacing(8)
                .align_y(Alignment::Start)
                .into()
        },

        | WysiwygBlock::EnumItem { number, body, .. } => {
            let num_str = format!("{}.", number.unwrap_or(1));
            let num_text = text(num_str)
                .size((14.0 * scale).max(12.0))
                .color(theme.accent());

            let item_text =
                render_rich_text(body, (14.0 * scale).max(12.0), theme.text_primary(), theme);

            row![num_text, container(item_text).width(Length::Fill)]
                .width(Length::Fill)
                .spacing(8)
                .align_y(Alignment::Start)
                .into()
        },

        | WysiwygBlock::TermItem {
            term, description, ..
        } => {
            let term_text = text(format!("{term}:"))
                .size((14.0 * scale).max(12.0))
                .color(theme.accent());

            let desc_text = render_rich_text(
                description,
                (14.0 * scale).max(12.0),
                theme.text_primary(),
                theme,
            );

            row![term_text, container(desc_text).width(Length::Fill)]
                .width(Length::Fill)
                .spacing(8)
                .align_y(Alignment::Start)
                .into()
        },

        | WysiwygBlock::CodeBlock { language, code, .. } => {
            if let Some(handle) = code_image {
                let img = image(handle.clone()).content_fit(iced::ContentFit::Contain);
                container(img).padding([2, 4]).width(Length::Fill).into()
            } else {
                let dot_red = container(
                    Space::new()
                        .width(Length::Fixed(6.0))
                        .height(Length::Fixed(6.0)),
                )
                .style(|_| {
                    container::Style {
                        background: Some(Background::Color(Color::from_rgb(1.0, 0.37, 0.34))),
                        border: border::rounded(theme::RADIUS_FULL),
                        ..container::Style::default()
                    }
                });
                let dot_yellow = container(
                    Space::new()
                        .width(Length::Fixed(6.0))
                        .height(Length::Fixed(6.0)),
                )
                .style(|_| {
                    container::Style {
                        background: Some(Background::Color(Color::from_rgb(1.0, 0.74, 0.18))),
                        border: border::rounded(theme::RADIUS_FULL),
                        ..container::Style::default()
                    }
                });
                let dot_green = container(
                    Space::new()
                        .width(Length::Fixed(6.0))
                        .height(Length::Fixed(6.0)),
                )
                .style(|_| {
                    container::Style {
                        background: Some(Background::Color(Color::from_rgb(0.15, 0.79, 0.25))),
                        border: border::rounded(theme::RADIUS_FULL),
                        ..container::Style::default()
                    }
                });
                let dots = row![dot_red, dot_yellow, dot_green]
                    .spacing(5)
                    .align_y(Alignment::Center);

                let lang_display = if language.is_empty() {
                    "CODE".to_string()
                } else {
                    language.to_uppercase()
                };

                let lang_badge = container(
                    text(lang_display)
                        .size(9)
                        .font(iced::Font::MONOSPACE)
                        .color(theme.text_muted()),
                )
                .padding([1, 6])
                .style(move |_| {
                    container::Style {
                        background: Some(Background::Color(theme.bg_subtle())),
                        border: border::rounded(theme::RADIUS_XS),
                        ..container::Style::default()
                    }
                });

                let window_header = row![dots, Space::new().width(Length::Fill), lang_badge,]
                    .align_y(Alignment::Center)
                    .padding([6, 10]);

                let code_display = text(code)
                    .size((12.0 * scale).max(10.0))
                    .font(iced::Font::MONOSPACE)
                    .color(theme.text_primary());

                let code_content = container(code_display).padding([6, 12]).width(Length::Fill);

                container(column![window_header, code_content].spacing(2))
                    .width(Length::Fill)
                    .style(move |_| theme::code_block_container_style(theme))
                    .into()
            }
        },

        | WysiwygBlock::Equation { formula, .. } => {
            if let Some(handle) = equation_image {
                let img = image(handle.clone()).content_fit(iced::ContentFit::Contain);
                container(img)
                    .padding([6, 12])
                    .align_x(Alignment::Center)
                    .width(Length::Fill)
                    .into()
            } else {
                let formula_text = text(format!("$ {formula} $"))
                    .size((16.0 * scale).max(13.0))
                    .font(iced::Font {
                        style: iced::font::Style::Italic,
                        ..iced::Font::DEFAULT
                    })
                    .color(theme.accent());

                container(formula_text)
                    .align_x(Alignment::Center)
                    .width(Length::Fill)
                    .padding([6, 12])
                    .into()
            }
        },

        | WysiwygBlock::Paragraph { text: body_text, .. } => {
            render_rich_text(
                body_text,
                (14.0 * scale).max(12.0),
                theme.text_primary(),
                theme,
            )
        },

        | WysiwygBlock::SetRule { target, .. } => {
            row![
                text("#set").size(10).color(theme.accent()),
                text(target).size(10).color(theme.text_muted()),
            ]
            .spacing(4)
            .into()
        },

        | WysiwygBlock::ShowRule { target, .. } => {
            let tgt = target.as_deref().unwrap_or("rule");
            row![
                text("#show").size(10).color(theme.accent()),
                text(tgt).size(10).color(theme.text_muted()),
            ]
            .spacing(4)
            .into()
        },

        | WysiwygBlock::LetBinding { name, .. } => {
            row![
                text("#let").size(10).color(theme.accent()),
                text(name).size(10).color(theme.text_muted()),
            ]
            .spacing(4)
            .into()
        },

        | WysiwygBlock::Module { path, .. } => {
            row![
                text("#import").size(10).color(theme.accent()),
                text(path).size(10).color(theme.text_muted()),
            ]
            .spacing(4)
            .into()
        },

        | WysiwygBlock::FuncCall { callee, args, .. } => {
            if callee == "image" {
                let clean_path = args
                    .trim()
                    .trim_matches(|c| c == '(' || c == ')' || c == '"' || c == '\'');
                row![
                    text("Image")
                        .size((10.0 * scale).max(9.0))
                        .color(theme.accent()),
                    text(clean_path)
                        .size((12.0 * scale).max(10.0))
                        .color(theme.text_secondary()),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .padding([4, 8])
                .into()
            } else if callee == "title-slide" {
                render_title_slide_block(
                    theme,
                    slide_idx,
                    block_idx,
                    block,
                    callee,
                    args,
                    block.raw(),
                    scale,
                    transition,
                )
            } else if callee == "link" {
                render_link_block(
                    theme,
                    slide_idx,
                    block_idx,
                    block,
                    args,
                    block.raw(),
                    scale,
                    transition,
                )
            } else if callee == "badge" || callee == "pill" {
                render_badge_block(
                    theme,
                    slide_idx,
                    block_idx,
                    block,
                    callee,
                    args,
                    block.raw(),
                    scale,
                    transition,
                )
            } else if callee == "table" {
                render_table_block(
                    theme,
                    slide_idx,
                    block_idx,
                    block,
                    args,
                    block.raw(),
                    scale,
                    transition,
                )
            } else if callee == "grid" || callee == "cols" || callee == "columns" {
                render_grid_block(
                    theme,
                    slide_idx,
                    block_idx,
                    block,
                    callee,
                    args,
                    block.raw(),
                    scale,
                    transition,
                )
            } else if callee == "callout"
                || callee == "note"
                || callee == "tip"
                || callee == "warning"
                || callee == "info"
                || callee == "alert"
                || callee == "quote"
            {
                render_callout_block(
                    theme,
                    slide_idx,
                    block_idx,
                    block,
                    callee,
                    args,
                    block.raw(),
                    scale,
                    transition,
                )
            } else if callee == "box" || callee == "block" || callee == "rect" {
                render_box_block(
                    theme,
                    slide_idx,
                    block_idx,
                    block,
                    callee,
                    args,
                    block.raw(),
                    scale,
                    transition,
                )
            } else if callee == "chart"
                || callee == "chart-bar"
                || callee == "chart-pie"
                || callee == "chart-line"
                || callee == "plot"
            {
                render_chart_block(
                    theme,
                    slide_idx,
                    block_idx,
                    block,
                    callee,
                    args,
                    block.raw(),
                    scale,
                    transition,
                )
            } else if callee == "video" {
                render_video_block(
                    theme,
                    slide_idx,
                    block_idx,
                    block,
                    args,
                    block.raw(),
                    scale,
                    transition,
                )
            } else if callee == "audio" || callee == "audio-player" {
                render_audio_block(
                    theme,
                    slide_idx,
                    block_idx,
                    block,
                    callee,
                    args,
                    block.raw(),
                    scale,
                    transition,
                )
            } else {
                row![
                    text(format!("#{callee}"))
                        .size((11.0 * scale).max(10.0))
                        .color(theme.accent()),
                    container(
                        text(args)
                            .size((11.0 * scale).max(9.0))
                            .color(theme.text_secondary())
                            .wrapping(iced::widget::text::Wrapping::Word)
                    )
                    .width(Length::Fill),
                ]
                .width(Length::Fill)
                .spacing(6)
                .align_y(Alignment::Center)
                .padding([2, 6])
                .into()
            }
        },

        | WysiwygBlock::PureCode { body, .. } => {
            text(format!("{{ {body} }}"))
                .size(11)
                .color(theme.text_muted())
                .into()
        },

        | WysiwygBlock::ContentBlock { body, .. } => {
            text(format!("[ {body} ]"))
                .size(12)
                .color(theme.text_primary())
                .into()
        },

        | WysiwygBlock::ControlFlow { kind, .. } => {
            text(format!("#{kind}"))
                .size(10)
                .color(theme.accent())
                .into()
        },

        | WysiwygBlock::Expression { raw, .. } => {
            text(raw).size(11).color(theme.text_muted()).into()
        },
    };

    // Styling container based on block type
    let is_code = matches!(block, WysiwygBlock::CodeBlock { .. });
    let is_math = matches!(block, WysiwygBlock::Equation { .. });
    let is_complex_element = match block {
        | WysiwygBlock::FuncCall { callee, .. } => {
            matches!(
                callee.as_str(),
                "title-slide"
                    | "table"
                    | "grid"
                    | "cols"
                    | "columns"
                    | "callout"
                    | "note"
                    | "tip"
                    | "warning"
                    | "info"
                    | "alert"
                    | "quote"
                    | "box"
                    | "block"
                    | "rect"
                    | "link"
                    | "chart"
                    | "chart-bar"
                    | "chart-pie"
                    | "chart-line"
                    | "plot"
                    | "badge"
                    | "pill"
                    | "video"
                    | "audio"
                    | "audio-player"
            )
        },
        | _ => false,
    };
    let is_directive = (matches!(
        block,
        WysiwygBlock::SetRule { .. }
            | WysiwygBlock::ShowRule { .. }
            | WysiwygBlock::LetBinding { .. }
            | WysiwygBlock::Module { .. }
            | WysiwygBlock::ControlFlow { .. }
    ) || matches!(block, WysiwygBlock::FuncCall { .. }))
        && !is_complex_element;

    let styled_container = container(inner_content)
        .width(Length::Fill)
        .padding(if is_code || is_math {
            [4, 6]
        } else if is_complex_element {
            [0, 0]
        } else {
            [2, 4]
        })
        .style(move |_| {
            if is_code {
                container::Style {
                    background: Some(Background::Color(theme.bg_subtle())),
                    border: Border {
                        color: theme.border_color(),
                        width: 1.0,
                        radius: border::Radius::from(6.0),
                    },
                    ..container::Style::default()
                }
            } else if is_math {
                container::Style {
                    background: Some(Background::Color(theme.bg_subtle())),
                    border: Border {
                        color: theme.border_active(),
                        width: 1.0,
                        radius: border::Radius::from(6.0),
                    },
                    ..container::Style::default()
                }
            } else if is_directive {
                container::Style {
                    background: Some(Background::Color(theme.bg_subtle())),
                    border: Border {
                        color: theme.border_color(),
                        width: 1.0,
                        radius: border::Radius::from(4.0),
                    },
                    ..container::Style::default()
                }
            } else {
                container::Style::default()
            }
        });

    let drag_handle = mouse_area(
        container(
            text("⠿")
                .size((13.0 * scale).max(11.0))
                .color(if is_dragging {
                    theme.accent()
                } else {
                    theme.text_muted().scale_alpha(0.7)
                }),
        )
        .padding([2, 4])
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_| {
            if is_dragging {
                container::Style {
                    background: Some(Background::Color(Color::from_rgba(0.18, 0.48, 0.94, 0.15))),
                    border: Border {
                        color: theme.accent(),
                        width: 1.0,
                        radius: border::Radius::from(4.0),
                    },
                    ..container::Style::default()
                }
            } else {
                container::Style {
                    background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.04))),
                    border: Border {
                        color: Color::from_rgba(0.5, 0.5, 0.5, 0.18),
                        width: 0.5,
                        radius: border::Radius::from(3.0),
                    },
                    ..container::Style::default()
                }
            }
        }),
    )
    .interaction(if is_dragging {
        iced::mouse::Interaction::Grabbing
    } else {
        iced::mouse::Interaction::Grab
    })
    .on_press(Message::StartDragBlock { slide_idx, block_idx });

    let block_body = mouse_area(
        container(styled_container)
            .padding(1)
            .width(Length::Fill)
            .style(move |_| {
                if is_dragging {
                    container::Style {
                        background: Some(Background::Color(Color::from_rgba(
                            0.18, 0.48, 0.94, 0.08,
                        ))),
                        border: Border {
                            color: theme.border_active(),
                            width: 1.0,
                            radius: border::Radius::from(6.0),
                        },
                        ..container::Style::default()
                    }
                } else {
                    container::Style::default()
                }
            }),
    )
    .interaction(if is_complex_element {
        iced::mouse::Interaction::Pointer
    } else {
        iced::mouse::Interaction::Text
    });

    let (chip_label, _) = block.chip_info();
    let open_context_menu_msg = Message::OpenBlockContextMenu {
        slide_idx,
        block_idx,
        range: block.range(),
        block_label: chip_label.clone(),
        block_id: block.id().to_string(),
    };

    let block_body: Element<'a, Message> = if is_complex_element {
        let (callee_str, raw_str) = match block {
            | WysiwygBlock::FuncCall { callee, raw, .. } => (callee.as_str(), raw.as_str()),
            | _ => ("", ""),
        };
        let open_modal_msg = Message::OpenComplexModal {
            slide_idx,
            block_id: block.id().to_string(),
            range: block.range(),
            raw: raw_str.to_string(),
            callee: callee_str.to_string(),
        };
        block_body
            .on_press(Message::SelectSlide(slide_idx))
            .on_double_click(open_modal_msg)
            .on_right_press(open_context_menu_msg)
            .into()
    } else {
        block_body
            .on_press(Message::ActivateBlock {
                slide_idx,
                id: block.id().to_string(),
                range: block.range(),
                raw: block.raw().to_string(),
            })
            .on_right_press(open_context_menu_msg)
            .into()
    };


    let trans_badge: Option<Element<'a, Message>> = if let Some(tr) = transition {
        Some(
            button(
                text(format!("⚡ Step {} • {}", tr.order, tr.effect)).size((9.0 * scale).max(8.0)),
            )
            .padding([1, 6])
            .style(move |_t, _s| theme::primary_button_style(theme))
            .on_press(Message::OpenElementTransitionModal { slide_idx, block_idx })
            .into(),
        )
    } else if is_active {
        Some(
            button(text("+ ⚡ Add Step Animation").size((9.0 * scale).max(8.0)))
                .padding([1, 6])
                .style(move |_t, _s| theme::subtle_button_style(theme, true))
                .on_press(Message::OpenElementTransitionModal { slide_idx, block_idx })
                .into(),
        )
    } else {
        None
    };

    let block_content_col = if let Some(badge) = trans_badge {
        column![
            row![badge, Space::new().width(Length::Fill)].align_y(Alignment::Center),
            Space::new().height(Length::Fixed(2.0)),
            block_body,
        ]
        .width(Length::Fill)
    } else {
        column![block_body].width(Length::Fill)
    };

    let spacing_val = spacing_pt.unwrap_or(0.0);
    let has_explicit_v = spacing_pt.is_some() && spacing_val > 0.0;
    let spacing_label = if has_explicit_v {
        format!("{:.0}pt", spacing_val)
    } else {
        "Auto".to_string()
    };

    let btn_dec_spacing = button(text("-").size(8))
        .padding([0, 4])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::AdjustBlockSpacing {
            slide_idx,
            block_idx,
            delta_pt: -4,
        });

    let btn_inc_spacing = button(text("+").size(8))
        .padding([0, 4])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::AdjustBlockSpacing {
            slide_idx,
            block_idx,
            delta_pt: 4,
        });

    let drag_label = mouse_area(
        container(
            text(format!("↕ {spacing_label}"))
                .size(9)
                .color(theme.text_muted().scale_alpha(0.7)),
        )
        .padding([1, 6])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(Color::from_rgba(0.5, 0.5, 0.5, 0.08))),
                border: Border {
                    color: Color::from_rgba(0.5, 0.5, 0.5, 0.18),
                    width: 0.5,
                    radius: border::Radius::from(3.0),
                },
                ..container::Style::default()
            }
        }),
    )
    .interaction(iced::mouse::Interaction::ResizingVertically)
    .on_press(Message::StartDragSpacing { slide_idx, block_idx });

    let block_row = row![
        drag_handle,
        Space::new().width(Length::Fixed(4.0)),
        block_content_col,
    ]
    .align_y(Alignment::Start)
    .width(Length::Fill);

    if has_explicit_v {
        let mut spacing_row = row![
            Space::new().width(Length::Fill),
            btn_dec_spacing,
            drag_label,
            btn_inc_spacing,
        ]
        .spacing(2)
        .align_y(Alignment::Center);

        let btn_reset_spacing = button(text("Auto").size(8))
            .padding([0, 4])
            .style(move |_t, _s| theme::subtle_button_style(theme, false))
            .on_press(Message::AdjustBlockSpacing {
                slide_idx,
                block_idx,
                delta_pt: -999,
            });
        spacing_row = spacing_row.push(btn_reset_spacing);
        spacing_row = spacing_row.push(Space::new().width(Length::Fill));

        let spacing_zone = container(spacing_row)
            .width(Length::Fill)
            .height(Length::Fixed(12.0));

        column![block_row, spacing_zone]
            .spacing(2)
            .width(Length::Fill)
            .into()
    } else {
        block_row.into()
    }
}

/// Render an insertion bar between blocks (+ Text, + Heading, + List, + Code, + Math)
#[must_use]
pub fn view_insert_bar<'a>(
    theme: AppTheme,
    after_offset: usize,
) -> Element<'a, Message> {
    let make_insert_btn = |kind: InsertBlockKind| {
        button(text(kind.label()).size(10))
            .style(move |_t, _s| theme::subtle_button_style(theme, false))
            .padding([2, 6])
            .on_press(Message::InsertBlockAfter {
                offset: after_offset,
                kind,
            })
    };

    let insert_row = row![
        text("+ Add:").size(10).color(theme.accent()),
        make_insert_btn(InsertBlockKind::Paragraph),
        make_insert_btn(InsertBlockKind::Heading1),
        make_insert_btn(InsertBlockKind::Heading2),
        make_insert_btn(InsertBlockKind::Heading3),
        make_insert_btn(InsertBlockKind::Heading4),
        make_insert_btn(InsertBlockKind::TitleSlide),
        make_insert_btn(InsertBlockKind::Bullet),
        make_insert_btn(InsertBlockKind::Numbered),
        make_insert_btn(InsertBlockKind::Table),
        make_insert_btn(InsertBlockKind::Grid),
        make_insert_btn(InsertBlockKind::Callout),
        make_insert_btn(InsertBlockKind::BoxBlock),
        make_insert_btn(InsertBlockKind::Link),
        make_insert_btn(InsertBlockKind::Chart),
        make_insert_btn(InsertBlockKind::CodeBlock),
        make_insert_btn(InsertBlockKind::Equation),
        make_insert_btn(InsertBlockKind::Video),
        make_insert_btn(InsertBlockKind::Audio),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .padding([2, 4]);

    container(insert_row)
        .width(Length::Fill)
        .padding([2, 6])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.bg_subtle().scale_alpha(0.5))),
                border: Border {
                    color: theme.border_subtle(),
                    width: 1.0,
                    radius: border::Radius::from(theme::RADIUS_MD),
                },
                ..container::Style::default()
            }
        })
        .into()
}
