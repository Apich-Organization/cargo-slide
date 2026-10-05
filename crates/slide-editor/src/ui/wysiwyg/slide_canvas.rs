//! 16:9 True WYSIWYG Live Preview Canvas for Typst presentations.
//!
//! Renders presentation slides with Typora-style fine-grained block fidelity:
//! - All unedited slide elements are displayed as 100% native vector compiled graphics.
//! - Fine-grained element chips allow activating any individual element in-place.
//! - When active, an in-place editor reveals only that element's Typst source for editing.
//! - When finished, the slide immediately recompiles and displays the crisp vector presentation.

use crate::app::Message;
use crate::model::ast_engine::TypstDocumentEngine;
use crate::model::ast_engine::WysiwygBlock;
use crate::model::ast_engine::WysiwygSlide;
use crate::ui::theme::AppTheme;
use crate::ui::theme::{
    self,
};
use crate::ui::typst_highlighter::TypstHighlightSettings;
use crate::ui::typst_highlighter::TypstHighlighter;
use crate::ui::typst_highlighter::to_typst_format;
use crate::ui::wysiwyg::block_view::InsertBlockKind;
use crate::ui::wysiwyg::block_view::clean_input_style;
use crate::ui::wysiwyg::block_view::view_block;
use crate::ui::wysiwyg::block_view::view_insert_bar;
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
use iced::widget::scrollable;
use iced::widget::text;
use iced::widget::text_editor;
use iced::widget::text_input;
use std::collections::HashMap;
use std::collections::HashSet;

/// Render the sequence of true WYSIWYG live presentation slides
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn view_slide_canvas<'a>(
    theme: AppTheme,
    slides: &'a [WysiwygSlide],
    slide_images: &'a [image::Handle],
    equation_images: &'a HashMap<String, image::Handle>,
    code_images: &'a HashMap<String, image::Handle>,
    active_slide_idx: usize,
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
    let scale = zoom_percent as f32 / 100.0;
    let card_width = 820.0 * scale;
    let card_height = 461.25 * scale; // 16:9 presentation ratio

    let mut slides_col = column![]
        .spacing(32)
        .align_x(Alignment::Center)
        .width(Length::Fill);

    for (idx, slide) in slides.iter().enumerate() {
        let is_slide_active = idx == active_slide_idx;
        let is_in_place_editing_this_slide = in_place_editing_slide == Some(idx);
        let slide_num = slide.page_number;

        // 1. Header Bar: Page Badge, Slide Title, Directive Badge, Quick Actions, and Delete Button
        let page_badge = container(
            text(format!("Page {slide_num}"))
                .size(11)
                .color(theme.text_secondary()),
        )
        .padding([2, 8])
        .style(move |_| {
            container::Style {
                background: Some(iced::Background::Color(theme.bg_subtle())),
                border: iced::border::rounded(4.0),
                ..container::Style::default()
            }
        });

        let title_color = if is_slide_active {
            theme.accent()
        } else {
            theme.text_primary()
        };
        let title_input = text_input("Slide Title...", &slide.title)
            .size(13)
            .padding([2, 6])
            .style(move |_t, _s| clean_input_style(theme, title_color))
            .on_input(move |new_title| {
                Message::UpdateSlideTitle {
                    slide_idx: idx,
                    new_title,
                }
            })
            .width(Length::Fill);

        let directive_count = slide
            .blocks
            .iter()
            .filter(|b| !b.is_content_element())
            .count();
        let directive_badge: Option<Element<'a, Message>> = if directive_count > 0 {
            Some(
                container(
                    text(format!("{directive_count} directives"))
                        .size(10)
                        .color(theme.text_muted()),
                )
                .padding([2, 6])
                .style(move |_| {
                    container::Style {
                        background: Some(iced::Background::Color(theme.bg_subtle())),
                        border: iced::border::rounded(4.0),
                        ..container::Style::default()
                    }
                })
                .into(),
            )
        } else {
            None
        };

        let has_trans = slide.transition.is_some();
        let trans_text = match slide.transition.as_deref() {
            | Some(t) if !t.is_empty() => format!("Transition: {t}"),
            | _ => "Transition: Cut".to_string(),
        };
        let trans_btn = button(text(trans_text).size(11).color(if has_trans {
            theme.accent()
        } else {
            theme.text_secondary()
        }))
        .style(move |_t, _s| theme::subtle_button_style(theme, has_trans))
        .padding([3, 8])
        .on_press(Message::OpenTransitionModal(idx));

        let is_vector_mode = slide_view_vector.contains(&idx);
        let view_toggle_btn = button(
            text(if is_vector_mode {
                "Flow Edit"
            } else {
                "Vector View"
            })
            .size(11),
        )
        .style(move |_t, _s| theme::subtle_button_style(theme, is_vector_mode))
        .padding([3, 8])
        .on_press(Message::ToggleSlideViewMode(idx));

        let delete_btn = button(text("Delete").size(11))
            .style(move |_t, _s| theme::danger_button_style(theme))
            .padding([3, 8])
            .on_press(Message::DeleteSlide(idx));

        let mut header_items = row![
            page_badge,
            Space::new().width(Length::Fixed(8.0)),
            title_input,
            trans_btn,
            view_toggle_btn,
            Space::new().width(Length::Fixed(6.0)),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Length::Fill);

        if let Some(badge) = directive_badge {
            header_items = header_items.push(badge);
        }

        header_items = header_items.push(delete_btn);

        let slide_header = mouse_area(container(header_items).padding([4, 6]).width(Length::Fill))
            .on_press(Message::SelectSlide(idx));

        let content_blocks: Vec<&WysiwygBlock> = slide
            .blocks
            .iter()
            .filter(|b| b.is_content_element())
            .collect();

        // 2. Main 16:9 Presentation Canvas Surface (Compiled Vector Layer)
        let img_widget: Element<'a, Message> = if let Some(handle) = slide_images.get(idx) {
            image(handle.clone())
                .width(Length::Fixed(card_width))
                .height(Length::Fixed(card_height))
                .content_fit(iced::ContentFit::Contain)
                .filter_method(image::FilterMethod::Nearest)
                .into()
        } else {
            container(
                text(format!("Compiling slide {slide_num} vector preview..."))
                    .size(13)
                    .color(theme.text_secondary()),
            )
            .width(Length::Fixed(card_width))
            .height(Length::Fixed(card_height))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(move |_| {
                container::Style {
                    background: Some(iced::Background::Color(theme.bg_subtle())),
                    ..container::Style::default()
                }
            })
            .into()
        };

        // 3. Real-Time Typora-Style Block Presentation Surface
        // Directly renders each semantic block in the presentation flow.
        // When inactive: view_visual_block renders the styled Typora presentation block.
        // When active: view_active_block_editor renders the in-place Typst editor AT THAT EXACT LOCATION.
        // Zero drift, zero ghost text from a background image, seamless in-place editing!
        let slide_surface: Element<'a, Message> = if is_vector_mode {
            let vector_view = button(img_widget)
                .style(move |_t, _s| {
                    button::Style {
                        background: None,
                        border: iced::border::Border::default(),
                        ..button::Style::default()
                    }
                })
                .padding(0)
                .on_press(Message::ToggleSlideViewMode(idx));
            vector_view.into()
        } else if content_blocks.is_empty() {
            let empty_prompt = button(
                container(
                    column![
                        text("+ Click here to add slide content")
                            .size(14)
                            .color(theme.accent()),
                        text("Direct in-place visual presentation editing")
                            .size(11)
                            .color(theme.text_muted()),
                    ]
                    .spacing(6)
                    .align_x(Alignment::Center),
                )
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .width(Length::Fill)
                .height(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_t, _s| {
                button::Style {
                    background: Some(iced::Background::Color(iced::Color {
                        a: 0.04,
                        ..theme.accent()
                    })),
                    border: iced::border::Border {
                        color: theme.accent().scale_alpha(0.25),
                        width: 1.0,
                        radius: iced::border::Radius::from(6.0),
                    },
                    ..button::Style::default()
                }
            })
            .on_press(Message::InsertBlockAfter {
                offset: engine.get_slide_content_insert_offset(idx),
                kind: InsertBlockKind::Heading1,
            });

            let ins_bar = view_insert_bar(theme, engine.get_slide_content_insert_offset(idx));
            column![empty_prompt, ins_bar].spacing(6).into()
        } else {
            let pad_v = (12.0 * scale).max(6.0);
            let pad_h = (24.0 * scale).max(12.0);
            let space_v = (4.0 * scale).max(2.0);
            let mut flow_col = column![]
                .spacing(space_v)
                .padding([pad_v, pad_h])
                .width(Length::Fill);

            // Typst-fidelity Title Banner inside the 16:9 canvas
            if !slide.title.trim().is_empty() {
                let banner = container(column![
                    text(&slide.title)
                        .size((18.0 * scale).max(13.5))
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..iced::Font::DEFAULT
                        })
                        .color(theme.accent()),
                    Space::new().height(Length::Fixed((3.0 * scale).max(2.0))),
                    container(Space::new().height(Length::Fixed(1.0)))
                        .width(Length::Fill)
                        .style(move |_| {
                            container::Style {
                                background: Some(iced::Background::Color(theme.border_subtle())),
                                ..container::Style::default()
                            }
                        }),
                    Space::new().height(Length::Fixed((4.0 * scale).max(2.0))),
                ])
                .width(Length::Fill);
                flow_col = flow_col.push(banner);
            }

            for (b_idx, block) in content_blocks.iter().enumerate() {
                let is_this_block_active = is_slide_active && active_block_id == Some(block.id());
                let is_raw_code = raw_code_blocks.contains(block.id());
                let eq_img = equation_images.get(block.id()).or_else(|| {
                    match block {
                        | WysiwygBlock::Equation { formula, .. } => equation_images.get(formula),
                        | _ => None,
                    }
                });
                let cd_img = code_images.get(block.id()).or_else(|| {
                    match block {
                        | WysiwygBlock::CodeBlock { code, .. } => code_images.get(code),
                        | _ => None,
                    }
                });
                let is_block_dragging = dragging_block == Some((idx, b_idx));
                let spacing_pt = engine.get_block_spacing_pt(idx, b_idx);
                let element_trans = engine
                    .slides
                    .get(idx)
                    .and_then(|s| s.element_transitions.get(block.id()));

                let block_widget = view_block(
                    theme,
                    idx,
                    b_idx,
                    content_blocks.len(),
                    block,
                    is_this_block_active,
                    is_raw_code,
                    active_block_content,
                    scale,
                    eq_img,
                    cd_img,
                    is_block_dragging,
                    spacing_pt,
                    element_trans,
                );
                flow_col = flow_col.push(block_widget);

                if let Some(pt) = spacing_pt
                    && pt > 0.0
                {
                    let extra_px = (pt * scale).max(2.0);
                    flow_col = flow_col.push(Space::new().height(Length::Fixed(extra_px)));
                }
            }

            if is_slide_active {
                let ins_bar = view_insert_bar(theme, engine.get_slide_content_insert_offset(idx));
                flow_col = flow_col.push(ins_bar);
            }

            // Clicking on empty space below blocks selects this slide and commits/deactivates active block
            let empty_space = mouse_area(
                container(Space::new())
                    .width(Length::Fill)
                    .height(Length::Fixed(30.0)),
            )
            .on_press(Message::SelectSlide(idx));

            flow_col = flow_col.push(empty_space);

            let flow_scroll = scrollable(flow_col)
                .direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::new().width(4).scroller_width(4),
                ))
                .width(Length::Fixed(card_width))
                .height(Length::Fill);

            let header_footer =
                crate::model::ast_engine::extract_header_footer_from_source(&engine.source_text);

            let header_bar = if header_footer.header_enabled {
                let hl = if header_footer.header_left.is_empty() {
                    " ".to_string()
                } else {
                    header_footer.header_left.clone()
                };
                let hr = if header_footer.header_right.is_empty() {
                    " ".to_string()
                } else {
                    header_footer.header_right.clone()
                };
                let h_btn = button(
                    row![
                        text(hl)
                            .size((8.5 * scale).max(7.0))
                            .color(theme.text_muted().scale_alpha(0.7)),
                        Space::new().width(Length::Fill),
                        text(hr)
                            .size((8.5 * scale).max(7.0))
                            .color(theme.text_muted().scale_alpha(0.7)),
                    ]
                    .align_y(Alignment::Center)
                    .padding([2, 12]),
                )
                .style(move |_t, _s| {
                    button::Style {
                        background: None,
                        border: iced::border::Border::default(),
                        ..button::Style::default()
                    }
                })
                .on_press(Message::OpenHeaderFooterModal);

                Some(container(h_btn).width(Length::Fixed(card_width)))
            } else {
                None
            };

            let footer_left_label = if !header_footer.footer_left.is_empty() {
                header_footer.footer_left.clone()
            } else {
                String::new()
            };

            let footer_right_label =
                if !header_footer.footer_enabled || header_footer.footer_right_mode == 2 {
                    String::new()
                } else if header_footer.footer_right_mode == 1 {
                    header_footer.footer_right_custom.clone()
                } else {
                    format!("{} / {}", slide_num, slides.len())
                };

            let footer_btn = button(
                row![
                    text(footer_left_label)
                        .size((9.0 * scale).max(7.5))
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..iced::Font::DEFAULT
                        })
                        .color(theme.text_muted().scale_alpha(0.7)),
                    Space::new().width(Length::Fill),
                    text(footer_right_label)
                        .size((9.0 * scale).max(7.5))
                        .color(theme.text_muted().scale_alpha(0.7)),
                ]
                .align_y(Alignment::Center)
                .padding([2, 12]),
            )
            .style(move |_t, _s| {
                button::Style {
                    background: None,
                    border: iced::border::Border::default(),
                    ..button::Style::default()
                }
            })
            .on_press(Message::OpenHeaderFooterModal);

            let footer_bar = container(footer_btn).width(Length::Fixed(card_width));

            let mut canvas_col = column![]
                .width(Length::Fixed(card_width))
                .height(Length::Fixed(card_height));
            if let Some(h) = header_bar {
                canvas_col = canvas_col.push(h);
            }
            canvas_col = canvas_col.push(flow_scroll).push(footer_bar);
            let canvas_content = canvas_col;

            container(canvas_content)
                .width(Length::Fixed(card_width))
                .height(Length::Fixed(card_height))
                .clip(true)
                .into()
        };

        let slide_canvas_container = container(slide_surface)
            .width(Length::Fixed(card_width))
            .height(Length::Fixed(card_height))
            .clip(true)
            .style(move |_| theme::slide_presentation_surface_style(theme, is_slide_active));

        // 4. Optional Slide Source Drawer (when explicitly toggled)
        let slide_markup_drawer: Option<Element<'a, Message>> = if is_in_place_editing_this_slide {
            let ed: Element<'a, Message> = if let Some(content) = in_place_content {
                text_editor(content)
                    .id(iced::widget::Id::new("slide_markup_drawer_editor"))
                    .placeholder("Slide Typst source...")
                    .wrapping(iced::widget::text::Wrapping::Word)
                    .style(move |_t, _s| theme::editor_style(theme))
                    .highlight_with::<TypstHighlighter>(
                        TypstHighlightSettings {
                            is_dark: theme.is_dark(),
                        },
                        to_typst_format,
                    )
                    .on_action(Message::InPlaceEditorAction)
                    .into()
            } else {
                container(
                    text("Loading slide source...")
                        .size(11)
                        .color(theme.text_muted()),
                )
                .into()
            };

            let done_btn = button(text("Close").size(10))
                .style(move |_t, _s| theme::primary_button_style(theme))
                .padding([2, 8])
                .on_press(Message::ToggleInPlaceEdit(idx));

            let bar = row![
                text("Slide Typst Source").size(11).color(theme.accent()),
                Space::new().width(Length::Fill),
                done_btn,
            ]
            .padding([2, 4])
            .align_y(Alignment::Center);

            let box_cont = container(ed)
                .height(Length::Fixed(120.0 * scale.max(1.0)))
                .padding(4);

            Some(
                container(column![bar, box_cont].spacing(4))
                    .width(Length::Fixed(card_width))
                    .style(move |_| theme::card_style(theme))
                    .padding(6)
                    .into(),
            )
        } else {
            None
        };

        let mut card_content = column![slide_header].spacing(6);
        card_content = card_content.push(slide_canvas_container);

        if let Some(slide_ed) = slide_markup_drawer {
            card_content = card_content.push(slide_ed);
        }

        let slide_card = container(card_content)
            .width(Length::Fixed(card_width + 24.0))
            .style(move |_| theme::slide_card_style(theme, is_slide_active))
            .padding(10);

        let slide_element: Element<'a, Message> = if !is_slide_active {
            mouse_area(slide_card)
                .on_press(Message::SelectSlide(idx))
                .into()
        } else {
            slide_card.into()
        };

        slides_col = slides_col.push(slide_element);
    }

    // Add slide prompt at bottom of live preview
    let add_slide_card = button(
        row![
            text("+").size(16).color(theme.accent()),
            text("Add New Slide").size(13).color(theme.accent()),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([12, 24])
    .style(move |_t, _s| theme::subtle_button_style(theme, false))
    .on_press(Message::InsertSlide);

    slides_col = slides_col.push(add_slide_card);

    let scroll = scrollable(slides_col.padding([24, 40]))
        .id(iced::widget::Id::new("live_preview_scrollable"))
        .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::new()))
        .width(Length::Fill)
        .height(Length::Fill);

    container(scroll)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| {
            container::Style {
                background: Some(iced::Background::Color(theme.bg_canvas())),
                ..container::Style::default()
            }
        })
        .into()
}
