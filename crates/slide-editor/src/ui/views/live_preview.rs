//! Flagship Typora-style Live Preview canvas with true Typst layout and in-place editing overlays.

use crate::app::Message;
use crate::model::ast_engine::TypstDocumentEngine;
use crate::model::ast_engine::WysiwygSlide;
use crate::ui::theme::AppTheme;
use crate::ui::wysiwyg::slide_canvas::view_slide_canvas;
use iced::Element;
use iced::widget::image;
use iced::widget::text_editor;
use std::collections::HashMap;
use std::collections::HashSet;

/// Render the Typora-style Live Preview mode with interactive AST blocks
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn view_live_preview<'a>(
    theme: AppTheme,
    slides: &'a [WysiwygSlide],
    slide_images: &'a [image::Handle],
    equation_images: &'a HashMap<String, image::Handle>,
    code_images: &'a HashMap<String, image::Handle>,
    active_slide: usize,
    active_block_id: Option<&'a str>,
    active_block_content: Option<&'a text_editor::Content>,
    in_place_editing_slide: Option<usize>,
    in_place_content: Option<&'a text_editor::Content>,
    zoom_percent: u32,
    slide_view_vector: &'a HashSet<usize>,
    dragging_block: Option<(usize, usize)>,
    raw_code_blocks: &'a HashSet<String>,
    engine: &'a TypstDocumentEngine,
) -> Element<'a, Message> {
    view_slide_canvas(
        theme,
        slides,
        slide_images,
        equation_images,
        code_images,
        active_slide,
        active_block_id,
        active_block_content,
        in_place_editing_slide,
        in_place_content,
        zoom_percent,
        slide_view_vector,
        dragging_block,
        raw_code_blocks,
        engine,
    )
}
