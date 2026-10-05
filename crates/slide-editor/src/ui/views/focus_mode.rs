use crate::app::Message;
use crate::document::EditorDocument;
use crate::model::ast_engine::WysiwygBlock;
use crate::model::ast_engine::WysiwygSlide;
use crate::ui::theme::AppTheme;
use crate::ui::theme::{
    self,
};
use crate::ui::typst_highlighter::TypstHighlightSettings;
use crate::ui::typst_highlighter::TypstHighlighter;
use crate::ui::typst_highlighter::to_typst_format;
use iced::Alignment;
use iced::Element;
use iced::Length;
use iced::widget::Space;
use iced::widget::button;
use iced::widget::column;
use iced::widget::container;
use iced::widget::image;
use iced::widget::mouse_area;
use iced::widget::row;
use iced::widget::stack;
use iced::widget::text;
use iced::widget::text_editor;

/// Render the focused slide editor view
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn view_focus_mode<'a>(
    theme: AppTheme,
    doc: &'a EditorDocument,
    slide_images: &'a [image::Handle],
    active_slide: usize,
    slide_content: &'a text_editor::Content,
    zoom_percent: u32,
    engine_slides: &'a [WysiwygSlide],
    hover_formula: Option<&'a str>,
    equation_images: &'a std::collections::HashMap<String, image::Handle>,
) -> Element<'a, Message> {
    let total = doc.total_slides();
    let slide_num = active_slide.saturating_add(1);

    // Navigation bar for switching slides
    let prev_btn = button(text("< Previous").size(12))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([4, 10])
        .on_press_maybe(if active_slide > 0 {
            Some(Message::SelectSlide(active_slide.saturating_sub(1)))
        } else {
            None
        });

    let slide_indicator = text(format!("Slide {slide_num} of {total}"))
        .size(13)
        .color(theme.text_primary());

    let next_btn = button(text("Next >").size(12))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([4, 10])
        .on_press_maybe(if slide_num < total {
            Some(Message::SelectSlide(active_slide.saturating_add(1)))
        } else {
            None
        });

    let add_btn = button(text("+ Add Slide").size(12))
        .style(move |_theme, _status| theme::primary_button_style(theme))
        .padding([4, 10])
        .on_press(Message::InsertSlide);

    let delete_btn = button(text("Delete").size(12))
        .style(move |_theme, _status| theme::danger_button_style(theme))
        .padding([4, 8])
        .on_press(Message::DeleteSlide(active_slide));

    let nav_row = row![
        prev_btn,
        slide_indicator,
        next_btn,
        Space::new().width(Length::Fill),
        add_btn,
        delete_btn
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding([8, 16]);

    let nav_container = container(nav_row).width(Length::Fill).style(move |_| {
        container::Style {
            background: Some(iced::Background::Color(theme.bg_surface())),
            border: iced::border::Border {
                color: theme.border_color(),
                width: 1.0,
                radius: iced::border::Radius::from(0.0),
            },
            ..container::Style::default()
        }
    });

    // Left pane: Syntax-highlighted Editor for current slide
    let editor_header = row![
        text(format!("EDITING SLIDE {slide_num}"))
            .size(11)
            .color(theme.text_muted()),
        Space::new().width(Length::Fill),
        text("Typst Source").size(11).color(theme.text_secondary())
    ]
    .padding([8, 12]);

    let editor = text_editor(slide_content)
        .id(iced::widget::Id::new("focus_mode_editor"))
        .placeholder("Slide Typst markup...")
        .wrapping(iced::widget::text::Wrapping::Word)
        .style(move |_theme, _status| theme::editor_style(theme))
        .highlight_with::<TypstHighlighter>(
            TypstHighlightSettings {
                is_dark: theme.is_dark(),
            },
            to_typst_format,
        )
        .on_action(Message::FocusSlideEditorAction);

    // Live math formula hover preview card
    let math_preview_card: Option<Element<'a, Message>> = if let Some(formula) = hover_formula {
        let clean = formula
            .trim()
            .trim_start_matches('$')
            .trim_end_matches('$')
            .trim();
        let eq_img = equation_images
            .get(formula)
            .or_else(|| equation_images.get(clean));

        let header_row = row![
            text("MATH FORMULA PREVIEW")
                .size(10)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..iced::Font::DEFAULT
                })
                .color(theme.accent()),
            Space::new().width(Length::Fill),
            text(formula).size(10).color(theme.text_muted()),
        ]
        .align_y(Alignment::Center);

        let img_element: Element<'a, Message> = if let Some(handle) = eq_img {
            image(handle.clone())
                .width(Length::Shrink)
                .height(Length::Shrink)
                .into()
        } else {
            text("Rendering formula...")
                .size(11)
                .color(theme.text_secondary())
                .into()
        };

        let card = container(
            column![
                header_row,
                container(img_element)
                    .padding([4, 8])
                    .align_x(Alignment::Center)
            ]
            .spacing(6),
        )
        .padding(10)
        .style(move |_| {
            container::Style {
                background: Some(iced::Background::Color(theme.bg_surface())),
                border: iced::border::Border {
                    color: theme.accent().scale_alpha(0.7),
                    width: 1.5,
                    radius: iced::border::Radius::from(6.0),
                },
                shadow: iced::Shadow {
                    color: iced::Color::BLACK.scale_alpha(0.35),
                    offset: iced::Vector::new(0.0, 3.0),
                    blur_radius: 8.0,
                },
                ..container::Style::default()
            }
        });

        Some(
            container(card)
                .padding(iced::Padding {
                    top: 0.0,
                    right: 16.0,
                    bottom: 16.0,
                    left: 0.0,
                })
                .into(),
        )
    } else {
        None
    };

    let editor_pane: Element<'a, Message> = if let Some(math_hud) = math_preview_card {
        stack![
            container(editor)
                .padding(iced::Padding {
                    top: 0.0,
                    right: 10.0,
                    bottom: 10.0,
                    left: 10.0
                })
                .width(Length::Fill)
                .height(Length::Fill),
            container(math_hud)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::End)
                .align_y(Alignment::End),
        ]
        .into()
    } else {
        container(editor)
            .padding(iced::Padding {
                top: 0.0,
                right: 10.0,
                bottom: 10.0,
                left: 10.0,
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    };

    let editor_col = column![editor_header, editor_pane]
        .width(Length::FillPortion(1))
        .height(Length::Fill);

    // 1. Check for compiler-generated vector hotspots (#link("slide-loc:{line}")[...])
    let current_slide_opt = doc.deck.as_ref().and_then(|d| d.slides.get(active_slide));

    let mut loc_hotspots: Vec<(usize, slide_core::model::Rect)> = Vec::new();
    let mut view_box = slide_core::model::Rect {
        x: 0.0,
        y: 0.0,
        width: 841.89,
        height: 595.28,
    };

    if let Some(slide) = current_slide_opt {
        if slide.view_box.width > 0.0 && slide.view_box.height > 0.0 {
            view_box = slide.view_box;
        }
        for hs in &slide.hotspots {
            if let slide_core::model::Hotspot::Link { target, rect } = hs
                && let Some(rest) = target.strip_prefix("slide-loc:")
                && let Ok(line) = rest.parse::<usize>()
            {
                loc_hotspots.push((line, *rect));
            }
        }
    }

    // Right pane: Real-time SVG preview for current slide with click-to-jump navigation
    let preview_header = row![
        text(format!("LIVE PREVIEW (SLIDE {slide_num})"))
            .size(11)
            .color(theme.text_muted()),
        Space::new().width(Length::Fill),
        text("Vector-accurate click to jump")
            .size(11)
            .color(theme.accent())
    ]
    .padding([8, 12]);

    let scale = zoom_percent as f32 / 100.0;
    let preview_w = 640.0 * scale;
    let preview_h = 360.0 * scale;

    let preview_widget: Element<'a, Message> = if let Some(handle) = slide_images.get(active_slide)
    {
        image(handle.clone())
            .width(Length::Fixed(preview_w))
            .height(Length::Fixed(preview_h))
            .content_fit(iced::ContentFit::Contain)
            .filter_method(image::FilterMethod::Nearest)
            .into()
    } else {
        container(
            text("Compiling slide preview...")
                .size(13)
                .color(theme.text_muted()),
        )
        .width(Length::Fixed(preview_w))
        .height(Length::Fixed(preview_h))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_| {
            container::Style {
                background: Some(iced::Background::Color(theme.bg_subtle())),
                border: iced::border::rounded(6.0),
                ..container::Style::default()
            }
        })
        .into()
    };

    let interactive_preview: Element<'a, Message> = {
        let vb_w = view_box.width;
        let vb_h = view_box.height;
        let aspect = vb_w / vb_h;
        let target_aspect = preview_w / preview_h;

        let (offset_x, offset_y, ratio, actual_w, actual_h) = if aspect >= target_aspect {
            let actual_h = preview_w / aspect;
            let off_x = 0.0;
            let off_y = (preview_h - actual_h) / 2.0;
            let r = preview_w / vb_w;
            (off_x, off_y, r, preview_w, actual_h)
        } else {
            let actual_w = preview_h * aspect;
            let off_x = (preview_w - actual_w) / 2.0;
            let off_y = 0.0;
            let r = preview_h / vb_h;
            (off_x, off_y, r, actual_w, preview_h)
        };

        // Base layer: preview image wrapped in mouse_area for background click fallback
        let mut preview_stack = stack![
            mouse_area(preview_widget)
                .on_move(move |p: iced::Point| {
                    let r = if actual_h > 0.0 {
                        ((p.y - offset_y) / actual_h).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    Message::PreviewHoverRatio(r)
                })
                .on_press(Message::JumpToCodeFromPreview)
        ]
        .width(Length::Fixed(preview_w))
        .height(Length::Fixed(preview_h));

        // Overlay layer: one button per compiler-generated hotspot rect.
        // These rects come from #link("slide-loc:{line}") injected by instrument_source_for_focus,
        // so their coordinates are pixel-accurate from the Typst compiler — no manual calculation.
        if !loc_hotspots.is_empty() {
            let slide_range_start = engine_slides
                .get(active_slide)
                .map(|s| s.range.start)
                .unwrap_or(0);
            let slide_text = doc.source_text.get(slide_range_start..).unwrap_or("");

            let content_blocks: Vec<(usize, &WysiwygBlock)> = engine_slides
                .get(active_slide)
                .map(|s| {
                    s.blocks
                        .iter()
                        .enumerate()
                        .filter(|(_, b)| b.is_content_element())
                        .collect()
                })
                .unwrap_or_default();

            for &(hs_line, ref hs_rect) in &loc_hotspots {
                // Ignore full-slide background rects or gigantic container rects
                if hs_rect.width >= view_box.width * 0.88
                    && hs_rect.height >= view_box.height * 0.75
                {
                    continue;
                }

                // Map hotspot line number → AST block index
                let block_idx = content_blocks.iter().find_map(|&(b_idx, block)| {
                    let b_start = block.range().start.saturating_sub(slide_range_start);
                    let b_end = block.range().end.saturating_sub(slide_range_start);
                    let b_start_line = slide_text[..b_start.min(slide_text.len())]
                        .bytes()
                        .filter(|&b| b == b'\n')
                        .count()
                        + 1;
                    let b_end_line = slide_text[..b_end.min(slide_text.len())]
                        .bytes()
                        .filter(|&b| b == b'\n')
                        .count()
                        + 1;
                    if hs_line >= b_start_line && hs_line <= b_end_line {
                        Some(b_idx)
                    } else {
                        None
                    }
                });

                let Some(target_block_idx) = block_idx else {
                    continue;
                };

                // Transform viewBox coordinates → screen pixel coordinates
                let sx = (offset_x + (hs_rect.x - view_box.x) * ratio).max(offset_x);
                let sy = (offset_y + (hs_rect.y - view_box.y) * ratio).max(offset_y);
                let max_w = (offset_x + actual_w - sx).max(36.0);
                let max_h = (offset_y + actual_h - sy).max(16.0);
                let sw = (hs_rect.width * ratio).min(max_w).max(36.0);
                let sh = (hs_rect.height * ratio).min(max_h).max(16.0);

                let btn = button(Space::new().width(Length::Fill).height(Length::Fill))
                    .width(Length::Fixed(sw))
                    .height(Length::Fixed(sh))
                    .style(move |_t, status| {
                        match status {
                            | button::Status::Hovered | button::Status::Pressed => {
                                button::Style {
                                    background: Some(iced::Background::Color(iced::Color {
                                        a: 0.12,
                                        ..theme.accent()
                                    })),
                                    border: iced::border::Border {
                                        color: theme.accent().scale_alpha(0.65),
                                        width: 1.5,
                                        radius: iced::border::Radius::from(4.0),
                                    },
                                    ..button::Style::default()
                                }
                            },
                            | _ => {
                                button::Style {
                                    background: None,
                                    border: iced::border::Border::default(),
                                    ..button::Style::default()
                                }
                            },
                        }
                    })
                    .on_press(Message::JumpToFocusBlock {
                        slide_idx: active_slide,
                        block_idx: target_block_idx,
                    });

                let placed_btn = container(btn)
                    .padding(iced::Padding {
                        top: sy.max(0.0),
                        left: sx.max(0.0),
                        right: 0.0,
                        bottom: 0.0,
                    })
                    .width(Length::Fixed(preview_w))
                    .height(Length::Fixed(preview_h));

                preview_stack = preview_stack.push(placed_btn);
            }
        }

        preview_stack.into()
    };

    let preview_card = container(interactive_preview)
        .padding(8)
        .style(move |_| theme::slide_card_style(theme, true));

    let preview_container = container(preview_card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .padding(20)
        .style(move |_| {
            container::Style {
                background: Some(iced::Background::Color(theme.bg_canvas())),
                ..container::Style::default()
            }
        });

    let preview_col = column![preview_header, preview_container]
        .width(Length::FillPortion(1))
        .height(Length::Fill);

    let split_row = row![
        container(editor_col).style(move |_| theme::sidebar_container_style(theme)),
        preview_col
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    column![nav_container, split_row]
        .height(Length::Fill)
        .into()
}
