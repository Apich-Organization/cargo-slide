//! Left outline sidebar showing slide list, thumbnails preview, and document hierarchy.

use crate::app::Message;
use crate::app::SidebarViewMode;
use crate::document::EditorDocument;
use crate::model::ast_engine::WysiwygSlide;
use crate::ui::theme::AppTheme;
use crate::ui::theme::RADIUS_FULL;
use crate::ui::theme::RADIUS_MD;
use crate::ui::theme::RADIUS_SM;
use crate::ui::theme::RADIUS_XS;
use crate::ui::theme::{
    self,
};
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

/// Render the outline sidebar with support for both Outline list and 16:9 Rendered Page Thumbnails preview
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn view_sidebar<'a>(
    theme: AppTheme,
    doc: &'a EditorDocument,
    slide_titles: &'a [String],
    slides: &'a [WysiwygSlide],
    slide_images: &'a [iced::widget::image::Handle],
    active_slide: usize,
    view_mode: SidebarViewMode,
    window_width: f32,
) -> Element<'a, Message> {
    let count_badge = container(
        text(format!(
            "{}/{}",
            active_slide.saturating_add(1),
            doc.total_slides().max(1)
        ))
        .size(10)
        .color(theme.text_secondary()),
    )
    .padding([2, 7])
    .style(move |_| {
        container::Style {
            background: Some(iced::Background::Color(theme.bg_subtle())),
            border: iced::border::rounded(RADIUS_FULL),
            ..container::Style::default()
        }
    });

    let mode_toggle_btn = button(
        text(if view_mode == SidebarViewMode::Thumbnails {
            "Preview"
        } else {
            "List"
        })
        .size(10)
        .color(theme.accent()),
    )
    .padding([2, 8])
    .style(move |_theme, _status| theme::subtle_button_style(theme, false))
    .on_press(Message::ToggleSidebarViewMode);

    let header = row![
        text("SLIDES OUTLINE").size(11).color(theme.text_muted()),
        Space::new().width(Length::Fill),
        mode_toggle_btn,
        Space::new().width(Length::Fixed(4.0)),
        count_badge,
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .padding(iced::Padding {
        top: 12.0,
        right: 12.0,
        bottom: 8.0,
        left: 12.0,
    });

    let insert_divider = |insert_idx: usize| {
        let add_btn = button(text("+").size(10).color(theme.accent()))
            .padding([1, 6])
            .style(move |_theme, status| {
                let is_hovered = matches!(status, iced::widget::button::Status::Hovered);
                iced::widget::button::Style {
                    background: Some(iced::Background::Color(if is_hovered {
                        theme.accent()
                    } else {
                        theme.bg_subtle()
                    })),
                    text_color: if is_hovered {
                        iced::Color::WHITE
                    } else {
                        theme.accent()
                    },
                    border: iced::border::rounded(RADIUS_FULL),
                    ..Default::default()
                }
            })
            .on_press(Message::InsertSlideAt(insert_idx));

        let div_line_left = container(Space::new().height(Length::Fixed(1.0)))
            .width(Length::Fill)
            .style(move |_| {
                container::Style {
                    background: Some(iced::Background::Color(theme.border_subtle())),
                    ..Default::default()
                }
            });
        let div_line_right = container(Space::new().height(Length::Fixed(1.0)))
            .width(Length::Fill)
            .style(move |_| {
                container::Style {
                    background: Some(iced::Background::Color(theme.border_subtle())),
                    ..Default::default()
                }
            });

        row![div_line_left, add_btn, div_line_right]
            .spacing(4)
            .align_y(Alignment::Center)
            .padding([1, 4])
    };

    let mut items = column![].spacing(6);
    let total = doc.total_slides();

    for idx in 0..total {
        let is_selected = idx == active_slide;
        let slide_num = idx.saturating_add(1);

        // Precomputed title from slide_titles cache
        let title = slide_titles
            .get(idx)
            .cloned()
            .unwrap_or_else(|| format!("Slide {slide_num}"));

        let slide_trans = slides.get(idx).and_then(|s| s.transition.as_deref());

        // Quick insert button above this slide
        items = items.push(insert_divider(idx));

        let item_element: Element<'a, Message> = if view_mode == SidebarViewMode::Thumbnails {
            // Rendered Page Thumbnail Card (16:9 ratio)
            let header_overlay = row![
                container(
                    text(format!("{slide_num:02}"))
                        .size(10)
                        .color(if is_selected {
                            iced::Color::WHITE
                        } else {
                            theme.text_primary()
                        }),
                )
                .padding([2, 6])
                .style(move |_| {
                    container::Style {
                        background: Some(iced::Background::Color(if is_selected {
                            theme.accent()
                        } else {
                            theme.bg_subtle()
                        })),
                        border: iced::border::rounded(RADIUS_FULL),
                        ..Default::default()
                    }
                }),
                Space::new().width(Length::Fill),
                if let Some(trans) = slide_trans {
                    container(
                        text(trans.to_uppercase())
                            .size(8)
                            .color(theme.accent_cyan()),
                    )
                    .padding([1, 4])
                    .style(move |_| {
                        container::Style {
                            background: Some(iced::Background::Color(theme.bg_subtle())),
                            border: iced::border::rounded(RADIUS_XS),
                            ..Default::default()
                        }
                    })
                } else {
                    container(Space::new().width(Length::Shrink))
                }
            ]
            .align_y(Alignment::Center)
            .padding([4, 6]);

            let title_footer = container(
                text(title)
                    .size(11)
                    .color(if is_selected {
                        theme.accent()
                    } else {
                        theme.text_primary()
                    })
                    .width(Length::Fill),
            )
            .padding([3, 6])
            .style(move |_| {
                container::Style {
                    background: Some(iced::Background::Color(theme.bg_surface_solid())),
                    border: iced::border::rounded(RADIUS_SM),
                    ..Default::default()
                }
            });

            let card_preview: Element<'a, Message> = if let Some(img_handle) = slide_images.get(idx)
            {
                container(
                    image(img_handle.clone())
                        .width(Length::Fixed(206.0))
                        .height(Length::Fixed(116.0)),
                )
                .align_x(Alignment::Center)
                .into()
            } else {
                let line1 = container(Space::new().height(Length::Fixed(6.0)))
                    .width(Length::Fixed(80.0))
                    .style(move |_| {
                        container::Style {
                            background: Some(iced::Background::Color(theme.border_subtle())),
                            border: iced::border::rounded(RADIUS_FULL),
                            ..Default::default()
                        }
                    });
                let line2 = container(Space::new().height(Length::Fixed(5.0)))
                    .width(Length::Fixed(120.0))
                    .style(move |_| {
                        container::Style {
                            background: Some(iced::Background::Color(theme.border_subtle())),
                            border: iced::border::rounded(RADIUS_FULL),
                            ..Default::default()
                        }
                    });
                let line3 = container(Space::new().height(Length::Fixed(5.0)))
                    .width(Length::Fixed(100.0))
                    .style(move |_| {
                        container::Style {
                            background: Some(iced::Background::Color(theme.border_subtle())),
                            border: iced::border::rounded(RADIUS_FULL),
                            ..Default::default()
                        }
                    });

                container(
                    column![
                        Space::new().height(Length::Fill),
                        column![line1, line2, line3].spacing(6).padding([0, 12]),
                        Space::new().height(Length::Fill),
                    ]
                    .height(Length::Fixed(116.0)),
                )
                .width(Length::Fill)
                .style(move |_| {
                    container::Style {
                        background: Some(iced::Background::Color(if theme.is_dark() {
                            iced::Color::from_rgb8(18, 22, 30)
                        } else {
                            iced::Color::from_rgb8(245, 247, 250)
                        })),
                        border: iced::border::rounded(RADIUS_SM),
                        ..Default::default()
                    }
                })
                .into()
            };

            let card_inner =
                container(column![header_overlay, card_preview, title_footer,]).width(Length::Fill);

            let card_btn = button(card_inner)
                .width(Length::Fill)
                .padding(0)
                .style(move |_theme, _status| {
                    let (brd_col, brd_w) = if is_selected {
                        (theme.accent(), 2.0)
                    } else {
                        (theme.border_color(), 1.0)
                    };
                    iced::widget::button::Style {
                        background: Some(iced::Background::Color(theme.bg_surface_solid())),
                        border: iced::border::Border {
                            color: brd_col,
                            width: brd_w,
                            radius: iced::border::Radius::from(RADIUS_MD),
                        },
                        shadow: if is_selected {
                            theme::elevation_subtle(theme)
                        } else {
                            iced::Shadow::default()
                        },
                        ..Default::default()
                    }
                })
                .on_press(Message::SelectSlide(idx));

            mouse_area(card_btn)
                .on_right_press(Message::OpenSlideContextMenu(idx))
                .into()
        } else {
            // Outline list item
            let active_bar = container(
                Space::new()
                    .width(Length::Fixed(3.0))
                    .height(Length::Fixed(22.0)),
            )
            .style(move |_| {
                container::Style {
                    background: Some(iced::Background::Color(if is_selected {
                        theme.accent()
                    } else {
                        iced::Color::TRANSPARENT
                    })),
                    border: iced::border::rounded(RADIUS_FULL),
                    ..container::Style::default()
                }
            });

            let num_str = format!("{slide_num:02}");
            let num_badge = container(text(num_str).size(10).color(if is_selected {
                iced::Color::WHITE
            } else {
                theme.text_secondary()
            }))
            .padding([2, 6])
            .style(move |_| {
                container::Style {
                    background: Some(iced::Background::Color(if is_selected {
                        theme.accent()
                    } else {
                        theme.bg_subtle()
                    })),
                    border: iced::border::rounded(RADIUS_FULL),
                    ..container::Style::default()
                }
            });

            let title_label = text(title).size(12).color(if is_selected {
                theme.accent()
            } else {
                theme.text_primary()
            });

            let trans_tag: Option<Element<'a, Message>> = slide_trans.map(|t| {
                container(text(t.to_uppercase()).size(9).color(theme.accent_cyan()))
                    .padding([1, 5])
                    .style(move |_| {
                        container::Style {
                            background: Some(iced::Background::Color(theme.bg_subtle())),
                            border: iced::border::rounded(RADIUS_XS),
                            ..container::Style::default()
                        }
                    })
                    .into()
            });

            let mut right_info = row![title_label].spacing(6).align_y(Alignment::Center);
            if let Some(tag) = trans_tag {
                right_info = right_info.push(tag);
            }

            let item_inner = row![
                active_bar,
                Space::new().width(Length::Fixed(2.0)),
                num_badge,
                right_info,
            ]
            .spacing(6)
            .align_y(Alignment::Center);

            let item_btn = button(item_inner)
                .width(Length::Fill)
                .padding([6, 8])
                .style(move |_theme, _status| {
                    let (bg, brd, shadow) = if is_selected {
                        (
                            if theme.is_dark() {
                                iced::Color::from_rgba(0.39, 0.40, 0.95, 0.15)
                            } else {
                                iced::Color::from_rgba(0.31, 0.27, 0.90, 0.10)
                            },
                            theme.border_active(),
                            theme::elevation_subtle(theme),
                        )
                    } else {
                        (
                            theme.bg_surface_solid(),
                            theme.border_color(),
                            iced::Shadow::default(),
                        )
                    };
                    iced::widget::button::Style {
                        background: Some(iced::Background::Color(bg)),
                        text_color: theme.text_primary(),
                        border: iced::border::Border {
                            color: brd,
                            width: if is_selected { 1.5 } else { 1.0 },
                            radius: iced::border::Radius::from(RADIUS_MD),
                        },
                        shadow,
                        snap: true,
                    }
                })
                .on_press(Message::SelectSlide(idx));

            mouse_area(item_btn)
                .on_right_press(Message::OpenSlideContextMenu(idx))
                .into()
        };

        items = items.push(item_element);
    }

    // Insert divider at the very end
    items = items.push(insert_divider(total));

    let add_slide_btn = button(
        row![
            text("+").size(14).color(theme.accent()),
            text("New Slide").size(12).color(theme.accent())
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([8, 12])
    .style(move |_theme, _status| theme::subtle_button_style(theme, false))
    .on_press(Message::InsertSlide);

    let content = column![
        header,
        scrollable(items.padding([4, 10]))
            .id(iced::widget::Id::new("sidebar_scrollable"))
            .height(Length::Fill),
        container(add_slide_btn).padding([8, 10])
    ];

    let base_width = if view_mode == SidebarViewMode::Thumbnails {
        260.0
    } else {
        240.0
    };
    // Proportional sidebar: shrink at narrow widths, never exceed base, never go below 160px
    let sidebar_width = (window_width * 0.2).clamp(160.0, base_width);

    container(content)
        .width(Length::Fixed(sidebar_width))
        .height(Length::Fill)
        .style(move |_| theme::sidebar_container_style(theme))
        .into()
}
