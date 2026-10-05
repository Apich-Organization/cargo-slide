//! Custom Typst Syntax Highlighter for Iced text_editor widget.
//!
//! Powered by official `typst-syntax`, providing accurate line-by-line syntax
//! highlighting for Typst documents, slides, commands, math, and code blocks.

use iced::Color;
use iced::Font;
use iced::advanced::text::Highlighter;
use iced::advanced::text::highlighter::Format;
use iced::advanced::text::highlighter::{
    self,
};
use std::ops::Range;
use typst_syntax::LinkedNode;
use typst_syntax::SyntaxKind;

/// Configuration settings for the Typst syntax highlighter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TypstHighlightSettings {
    pub is_dark: bool,
}

/// Token classification for Typst syntax highlighting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypstTokenType {
    Heading,
    Command,
    Keyword,
    Function,
    Math,
    ListMarker,
    String,
    Comment,
    RawCode,
    Number,
    Punctuation,
    Normal,
}

/// State-tracking syntax highlighter for Iced text editor
pub struct TypstHighlighter {
    _settings: TypstHighlightSettings,
    current_line: usize,
}

impl Highlighter for TypstHighlighter {
    type Highlight = TypstTokenType;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Self::Highlight)>;
    type Settings = TypstHighlightSettings;

    fn new(settings: &Self::Settings) -> Self {
        Self {
            _settings: *settings,
            current_line: 0,
        }
    }

    fn update(
        &mut self,
        new_settings: &Self::Settings,
    ) {
        self._settings = *new_settings;
    }

    fn change_line(
        &mut self,
        line: usize,
    ) {
        self.current_line = line;
    }

    fn highlight_line(
        &mut self,
        line: &str,
    ) -> Self::Iterator<'_> {
        let tokens = tokenize_line(line);
        tokens.into_iter()
    }

    fn current_line(&self) -> usize {
        self.current_line
    }
}

/// Tokenize a single line of Typst text using `typst_syntax`
fn tokenize_line(line: &str) -> Vec<(Range<usize>, TypstTokenType)> {
    if line.trim().is_empty() {
        return Vec::new();
    }

    let root = typst_syntax::parse(line);
    let linked = LinkedNode::new(&root);
    let mut tokens = Vec::new();

    collect_tokens(&linked, &mut tokens);
    tokens
}

fn collect_tokens(
    node: &LinkedNode,
    tokens: &mut Vec<(Range<usize>, TypstTokenType)>,
) {
    let kind = node.kind();
    let range = node.range();

    if let Some(token_type) = classify_syntax_kind(kind, node) {
        if range.start < range.end {
            tokens.push((range, token_type));
        }
        return;
    }

    for child in node.children() {
        collect_tokens(&child, tokens);
    }
}

/// Classify a syntax node into highlighting token
fn classify_syntax_kind(
    kind: SyntaxKind,
    node: &LinkedNode,
) -> Option<TypstTokenType> {
    match kind {
        | SyntaxKind::LineComment | SyntaxKind::BlockComment => Some(TypstTokenType::Comment),
        | SyntaxKind::Heading | SyntaxKind::HeadingMarker => Some(TypstTokenType::Heading),
        | SyntaxKind::Equation | SyntaxKind::Math | SyntaxKind::Dollar => {
            Some(TypstTokenType::Math)
        },
        | SyntaxKind::Raw | SyntaxKind::RawLang | SyntaxKind::RawDelim | SyntaxKind::RawTrimmed => {
            Some(TypstTokenType::RawCode)
        },
        | SyntaxKind::ListMarker | SyntaxKind::EnumMarker | SyntaxKind::TermMarker => {
            Some(TypstTokenType::ListMarker)
        },
        | SyntaxKind::Str => Some(TypstTokenType::String),
        | SyntaxKind::Int | SyntaxKind::Float | SyntaxKind::Numeric => Some(TypstTokenType::Number),
        | SyntaxKind::Let
        | SyntaxKind::Set
        | SyntaxKind::Show
        | SyntaxKind::Import
        | SyntaxKind::Include
        | SyntaxKind::If
        | SyntaxKind::Else
        | SyntaxKind::For
        | SyntaxKind::While
        | SyntaxKind::Return
        | SyntaxKind::Context
        | SyntaxKind::Break
        | SyntaxKind::Continue => Some(TypstTokenType::Keyword),
        | SyntaxKind::Hash => Some(TypstTokenType::Command),
        | SyntaxKind::Ident => {
            if let Some(prev) = node.prev_leaf()
                && prev.kind() == SyntaxKind::Hash
            {
                Some(TypstTokenType::Command)
            } else if let Some(parent) = node.parent()
                && (parent.kind() == SyntaxKind::FuncCall
                    || parent.kind() == SyntaxKind::FieldAccess)
            {
                Some(TypstTokenType::Function)
            } else {
                None
            }
        },
        | SyntaxKind::LeftBrace
        | SyntaxKind::RightBrace
        | SyntaxKind::LeftBracket
        | SyntaxKind::RightBracket
        | SyntaxKind::LeftParen
        | SyntaxKind::RightParen => Some(TypstTokenType::Punctuation),
        | _ => None,
    }
}

/// Convert a syntax token into an Iced formatting format
pub fn to_typst_format(
    token: &TypstTokenType,
    _theme: &iced::Theme,
) -> highlighter::Format<Font> {
    let color = match token {
        | TypstTokenType::Heading => Color::from_rgb(0.28, 0.60, 1.0),
        | TypstTokenType::Command => Color::from_rgb(0.75, 0.45, 0.98),
        | TypstTokenType::Keyword => Color::from_rgb(0.88, 0.38, 0.80),
        | TypstTokenType::Function => Color::from_rgb(0.35, 0.72, 0.98),
        | TypstTokenType::Math => Color::from_rgb(0.96, 0.72, 0.22),
        | TypstTokenType::ListMarker => Color::from_rgb(0.96, 0.54, 0.22),
        | TypstTokenType::String => Color::from_rgb(0.38, 0.82, 0.48),
        | TypstTokenType::Comment => Color::from_rgb(0.50, 0.56, 0.66),
        | TypstTokenType::RawCode => Color::from_rgb(0.38, 0.85, 0.78),
        | TypstTokenType::Number => Color::from_rgb(0.96, 0.60, 0.40),
        | TypstTokenType::Punctuation => Color::from_rgb(0.65, 0.70, 0.82),
        | TypstTokenType::Normal => Color::from_rgb(0.85, 0.88, 0.94),
    };

    Format {
        color: Some(color),
        font: None,
    }
}
