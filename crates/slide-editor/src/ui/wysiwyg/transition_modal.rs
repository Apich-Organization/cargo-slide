//! Dedicated visual Slide Transition & Animation Modal.
//!
//! Provides interactive visual selection and bulk application for all 13
//! hardware-accelerated 60 FPS slide transitions supported by cargo-slide.

use crate::app::Message;
use crate::ui::theme::AppTheme;
use crate::ui::theme::{
    self,
};
use iced::Alignment;
use iced::Background;
use iced::Border;
use iced::Color;
use iced::Element;
use iced::Length;
use iced::border;
use iced::widget::Space;
use iced::widget::button;
use iced::widget::column;
use iced::widget::container;
use iced::widget::row;
use iced::widget::scrollable;
use iced::widget::text;

/// Metadata definition for a built-in presentation transition animation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionItem {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub badge: &'static str,
    pub desc: &'static str,
}

/// Complete catalog of 13 official cargo-slide transitions
pub const TRANSITIONS: &[TransitionItem] = &[
    // Cuts & Fades
    TransitionItem {
        id: "cut",
        name: "Instant Cut",
        category: "Cuts & Fades",
        badge: "DEFAULT",
        desc: "Instant switch without animation. Recommended for clean, distraction-free presentations.",
    },
    TransitionItem {
        id: "fade",
        name: "Cross Dissolve",
        category: "Cuts & Fades",
        badge: "SMOOTH FADE",
        desc: "Silky 60 FPS alpha cross-fade dissolve between outgoing and incoming slides.",
    },
    TransitionItem {
        id: "particles",
        name: "Particle Dissolve",
        category: "Cuts & Fades",
        badge: "PARTICLES",
        desc: "Pixel-level sand disintegration particle scatter dissolve effect.",
    },
    // Directional
    TransitionItem {
        id: "slide-left",
        name: "Slide Left",
        category: "Directional Pushes",
        badge: "PUSH LEFT",
        desc: "Horizontal slide push translating outbound slide left while revealing inbound slide.",
    },
    TransitionItem {
        id: "slide-right",
        name: "Slide Right",
        category: "Directional Pushes",
        badge: "PUSH RIGHT",
        desc: "Horizontal slide push translating outbound slide right with smooth deceleration.",
    },
    TransitionItem {
        id: "slide-up",
        name: "Slide Up",
        category: "Directional Pushes",
        badge: "PUSH UP",
        desc: "Vertical upward slide push with physics-based cubic easing.",
    },
    TransitionItem {
        id: "slide-down",
        name: "Slide Down",
        category: "Directional Pushes",
        badge: "PUSH DOWN",
        desc: "Vertical downward slide push translation.",
    },
    // Wipes & Reveals
    TransitionItem {
        id: "zoom",
        name: "Zoom Wipe",
        category: "Wipes & Reveals",
        badge: "ZOOM",
        desc: "Radial scale expansion zoom reveal emanating smoothly from slide viewport center.",
    },
    TransitionItem {
        id: "iris",
        name: "Iris Aperture",
        category: "Wipes & Reveals",
        badge: "APERTURE",
        desc: "Circular camera lens aperture diaphragm opening reveal effect.",
    },
    TransitionItem {
        id: "wipe-left",
        name: "Wipe Left",
        category: "Wipes & Reveals",
        badge: "CURTAIN",
        desc: "Linear horizontal curtain sweep wipe moving from right to left.",
    },
    TransitionItem {
        id: "wipe-right",
        name: "Wipe Right",
        category: "Wipes & Reveals",
        badge: "CURTAIN",
        desc: "Linear horizontal curtain sweep wipe moving from left to right.",
    },
    // 3D & Advanced FX
    TransitionItem {
        id: "glitch",
        name: "Cyber Glitch",
        category: "3D & Advanced FX",
        badge: "CHROMATIC",
        desc: "RGB chromatic aberration displacement with horizontal scanline interference.",
    },
    TransitionItem {
        id: "cube",
        name: "3D Cube Rotation",
        category: "3D & Advanced FX",
        badge: "PERSPECTIVE 3D",
        desc: "Perspective 3D geometric cube rotation transforming slide faces in real-time.",
    },
];

/// Render the slide transition picker modal dialog
#[must_use]
pub fn view_slide_transition_modal<'a>(
    theme: AppTheme,
    slide_idx: usize,
    selected_transition: Option<&'a str>,
    total_slides: usize,
) -> Element<'a, Message> {
    let current_id = selected_transition.unwrap_or("cut");

    // 1. Header
    let badge = container(text("PAGE TRANSITION").size(10).color(theme.accent()))
        .padding([2, 8])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.bg_subtle())),
                border: border::rounded(4.0),
                ..container::Style::default()
            }
        });

    let target_pill = container(
        text(format!(
            "Target: Slide {} / {}",
            slide_idx + 1,
            total_slides.max(1)
        ))
        .size(11)
        .color(theme.text_secondary()),
    )
    .padding([2, 8])
    .style(move |_| {
        container::Style {
            background: Some(Background::Color(theme.bg_subtle())),
            border: border::rounded(4.0),
            ..container::Style::default()
        }
    });

    let title_row = row![
        badge,
        target_pill,
        text("Slide Transition & Visual Effects")
            .size(16)
            .color(theme.text_primary()),
        Space::new().width(Length::Fill),
        button(text("×").size(16))
            .style(move |_t, _s| theme::subtle_button_style(theme, false))
            .padding([2, 8])
            .on_press(Message::CloseModal),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let subtitle = text(
        "Select a GPU-independent 60 FPS slide transition. Changes compile directly to native Typst metadata.",
    )
    .size(12)
    .color(theme.text_muted());

    let header_col = column![title_row, subtitle].spacing(4);

    // 2. Group transitions by category
    let categories = [
        "Cuts & Fades",
        "Directional Pushes",
        "Wipes & Reveals",
        "3D & Advanced FX",
    ];

    let mut body_col = column![].spacing(16);

    for cat in categories {
        let cat_label = text(cat.to_uppercase()).size(11).color(theme.text_muted());

        let mut grid_col = column![].spacing(6);

        let items_in_cat: Vec<&TransitionItem> = TRANSITIONS
            .iter()
            .filter(|item| item.category == cat)
            .collect();

        for item in items_in_cat {
            let is_selected = current_id == item.id
                || (item.id == "cut"
                    && (selected_transition.is_none() || selected_transition == Some("")));

            let badge_bg = if is_selected {
                theme.accent()
            } else {
                theme.bg_subtle()
            };
            let badge_fg = if is_selected {
                Color::WHITE
            } else {
                theme.text_secondary()
            };

            let item_badge = container(text(item.badge).size(9).color(badge_fg))
                .padding([2, 6])
                .style(move |_| {
                    container::Style {
                        background: Some(Background::Color(badge_bg)),
                        border: border::rounded(3.0),
                        ..container::Style::default()
                    }
                });

            let name_label = text(item.name).size(13).color(if is_selected {
                theme.accent()
            } else {
                theme.text_primary()
            });

            let desc_label = text(item.desc).size(11).color(theme.text_secondary());

            let left_info = column![
                row![name_label, item_badge]
                    .spacing(8)
                    .align_y(Alignment::Center),
                desc_label
            ]
            .spacing(2)
            .width(Length::Fill);

            let status_tag: Element<'a, Message> = if is_selected {
                container(text("ACTIVE").size(10).color(Color::WHITE))
                    .padding([3, 8])
                    .style(move |_| {
                        container::Style {
                            background: Some(Background::Color(theme.accent())),
                            border: border::rounded(4.0),
                            ..container::Style::default()
                        }
                    })
                    .into()
            } else {
                container(text("Select").size(10).color(theme.text_muted()))
                    .padding([3, 8])
                    .style(move |_| {
                        container::Style {
                            background: Some(Background::Color(theme.bg_subtle())),
                            border: border::rounded(4.0),
                            ..container::Style::default()
                        }
                    })
                    .into()
            };

            let card_inner = row![left_info, status_tag]
                .spacing(12)
                .align_y(Alignment::Center)
                .padding([8, 12]);

            let item_id_string = item.id.to_string();
            let select_msg = Message::SelectTransition(Some(item_id_string));

            let card_btn = button(card_inner)
                .width(Length::Fill)
                .padding(0)
                .style(move |_t, _s| {
                    let border_color = if is_selected {
                        theme.accent()
                    } else {
                        theme.border_color()
                    };
                    let bg_color = if is_selected {
                        if theme.is_dark() {
                            Color::from_rgba(0.26, 0.58, 1.0, 0.12)
                        } else {
                            Color::from_rgba(0.16, 0.46, 0.92, 0.08)
                        }
                    } else {
                        theme.bg_surface_solid()
                    };

                    button::Style {
                        background: Some(Background::Color(bg_color)),
                        text_color: theme.text_primary(),
                        border: Border {
                            color: border_color,
                            width: if is_selected { 1.5 } else { 1.0 },
                            radius: border::Radius::from(8.0),
                        },
                        shadow: iced::Shadow::default(),
                        snap: true,
                    }
                })
                .on_press(select_msg);

            grid_col = grid_col.push(card_btn);
        }

        body_col = body_col.push(column![cat_label, grid_col].spacing(6));
    }

    // 3. Footer Action Buttons
    let active_transition_opt = if current_id == "cut" {
        None
    } else {
        Some(current_id.to_string())
    };

    let remove_btn = button(text("Remove / Cut").size(12))
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .padding([7, 12])
        .on_press(Message::ApplySlideTransition {
            slide_idx,
            transition: None,
            all_slides: false,
        });

    let apply_slide_btn = button(text(format!("Apply to Slide {}", slide_idx + 1)).size(12))
        .style(move |_t, _s| theme::primary_button_style(theme))
        .padding([7, 16])
        .on_press(Message::ApplySlideTransition {
            slide_idx,
            transition: active_transition_opt.clone(),
            all_slides: false,
        });

    let apply_all_btn = button(text(format!("Apply to All ({total_slides} Slides)")).size(12))
        .style(move |_t, _s| theme::subtle_button_style(theme, true))
        .padding([7, 16])
        .on_press(Message::ApplySlideTransition {
            slide_idx,
            transition: active_transition_opt,
            all_slides: true,
        });

    let cancel_btn = button(text("Cancel").size(12))
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .padding([7, 12])
        .on_press(Message::CloseModal);

    let footer_row = row![
        remove_btn,
        Space::new().width(Length::Fill),
        cancel_btn,
        apply_all_btn,
        apply_slide_btn,
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // 4. Modal Dialog Card
    let dialog_card = container(
        column![
            header_col,
            Space::new().height(8),
            scrollable(body_col).height(Length::FillPortion(1)),
            Space::new().height(10),
            footer_row,
        ]
        .spacing(6),
    )
    .width(Length::Fixed(760.0))
    .max_height(640.0)
    .padding(22)
    .style(move |_| theme::modal_dialog_style(theme));

    // Semi-transparent backdrop overlay
    container(dialog_card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_| theme::modal_backdrop_style(theme))
        .into()
}

/// Render element-level step transition modal
#[must_use]
pub fn view_element_transition_modal<'a>(
    theme: AppTheme,
    slide_idx: usize,
    block_idx: usize,
    order: usize,
    current_effect: &'a str,
    is_existing: bool,
) -> Element<'a, Message> {
    // 1. Header
    let badge = container(
        text("ELEMENT STEP TRANSITION")
            .size(10)
            .color(theme.accent()),
    )
    .padding([2, 8])
    .style(move |_| {
        container::Style {
            background: Some(Background::Color(theme.bg_subtle())),
            border: border::rounded(4.0),
            ..container::Style::default()
        }
    });

    let target_pill = container(
        text(format!(
            "Slide {} • Element #{}",
            slide_idx + 1,
            block_idx + 1
        ))
        .size(11)
        .color(theme.text_secondary()),
    )
    .padding([2, 8])
    .style(move |_| {
        container::Style {
            background: Some(Background::Color(theme.bg_subtle())),
            border: border::rounded(4.0),
            ..container::Style::default()
        }
    });

    let title_row = row![
        badge,
        target_pill,
        text("Element Animation & Step Sequence")
            .size(16)
            .color(theme.text_primary()),
        Space::new().width(Length::Fill),
        button(text("×").size(14))
            .style(move |_t, _s| theme::subtle_button_style(theme, false))
            .padding([2, 8])
            .on_press(Message::CloseModal),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let subtitle = text(
        "Configure step reveal sequence order and entrance animation effect for this block element.",
    )
    .size(12)
    .color(theme.text_muted());

    let header_col = column![title_row, subtitle].spacing(4);

    // 2. Order Sequence Selector
    let order_label = text("Sequence / Step Order:")
        .size(12)
        .color(theme.text_secondary());
    let btn_dec = button(text("-").size(12))
        .padding([2, 8])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::SetElementTransitionOrder(if order > 1 {
            order - 1
        } else {
            1
        }));
    let order_display = container(text(format!("Step {order}")).size(13).color(theme.accent()))
        .padding([3, 12])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.bg_subtle())),
                border: border::rounded(4.0),
                ..container::Style::default()
            }
        });
    let btn_inc = button(text("+").size(12))
        .padding([2, 8])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::SetElementTransitionOrder(order + 1));

    let mut quick_steps = row![order_label, btn_dec, order_display, btn_inc]
        .spacing(6)
        .align_y(Alignment::Center);
    quick_steps = quick_steps.push(Space::new().width(Length::Fixed(12.0)));
    quick_steps = quick_steps.push(text("Quick:").size(11).color(theme.text_muted()));
    for step_num in 1..=6 {
        let is_cur = order == step_num;
        let btn = button(text(format!("{step_num}")).size(11))
            .padding([2, 6])
            .style(move |_t, _s| {
                if is_cur {
                    theme::primary_button_style(theme)
                } else {
                    theme::subtle_button_style(theme, false)
                }
            })
            .on_press(Message::SetElementTransitionOrder(step_num));
        quick_steps = quick_steps.push(btn);
    }

    // 3. Effect selector
    let effects: &[(&str, &str, &str)] = &[
        ("fade-in", "Fade In", "Smooth opacity dissolve entrance"),
        ("slide-up", "Slide Up", "Translate upwards into view"),
        ("slide-down", "Slide Down", "Translate downwards into view"),
        (
            "slide-left",
            "Slide Left",
            "Push horizontally from right to left",
        ),
        (
            "slide-right",
            "Slide Right",
            "Push horizontally from left to right",
        ),
        ("zoom", "Zoom", "Scale expansion reveal from center"),
        (
            "glitch",
            "Cyber Glitch",
            "Digital chromatic displacement entrance",
        ),
        ("cube", "3D Cube", "3D perspective rotation entrance"),
        (
            "iris",
            "Iris Aperture",
            "Circular aperture expansion reveal",
        ),
        ("particles", "Particles", "Particle condensation reveal"),
        (
            "cut",
            "Instant Cut",
            "Instant appearance on step activation",
        ),
    ];

    let mut effect_col = column![].spacing(6);
    for &(eff_id, eff_name, eff_desc) in effects {
        let is_sel = current_effect == eff_id;
        let card = button(
            row![
                container(
                    text(if is_sel { "[x]" } else { "[ ]" })
                        .size(11)
                        .color(if is_sel {
                            theme.accent()
                        } else {
                            theme.text_muted()
                        })
                )
                .padding([2, 4]),
                column![
                    text(eff_name).size(12).color(if is_sel {
                        theme.accent()
                    } else {
                        theme.text_primary()
                    }),
                    text(eff_desc).size(10).color(theme.text_muted()),
                ]
                .spacing(2),
                Space::new().width(Length::Fill),
                text(format!("#{eff_id}"))
                    .size(10)
                    .color(theme.text_secondary()),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([6, 10])
        .style(move |_t, _s| {
            if is_sel {
                button::Style {
                    background: Some(Background::Color(Color::from_rgba(0.18, 0.48, 0.94, 0.12))),
                    border: Border {
                        color: theme.accent(),
                        width: 1.0,
                        radius: border::Radius::from(6.0),
                    },
                    text_color: theme.text_primary(),
                    ..button::Style::default()
                }
            } else {
                theme::subtle_button_style(theme, false)
            }
        })
        .on_press(Message::SelectElementTransitionEffect(eff_id.to_string()));

        effect_col = effect_col.push(card);
    }

    // 4. Code Preview Bar
    let preview_bar = row![
        text("Typst Preview:")
            .size(11)
            .color(theme.text_secondary()),
        container(
            text(format!(
                "#step({}, effect: \"{}\")[ ... ]",
                order, current_effect
            ))
            .size(11)
            .font(iced::Font::MONOSPACE)
            .color(theme.accent())
        )
        .padding([2, 8])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.bg_subtle())),
                border: border::rounded(4.0),
                ..container::Style::default()
            }
        }),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // 5. Footer actions
    let mut footer = row![].spacing(8).align_y(Alignment::Center);
    if is_existing {
        let remove_btn = button(text("Remove Transition").size(12))
            .style(move |_t, _s| theme::subtle_button_style(theme, false))
            .padding([7, 12])
            .on_press(Message::RemoveElementTransition);
        footer = footer.push(remove_btn);
    }

    footer = footer.push(Space::new().width(Length::Fill));
    let cancel_btn = button(text("Cancel").size(12))
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .padding([7, 12])
        .on_press(Message::CloseModal);
    let apply_btn = button(
        text(if is_existing {
            "Update Transition"
        } else {
            "Apply Transition"
        })
        .size(12),
    )
    .style(move |_t, _s| theme::primary_button_style(theme))
    .padding([7, 16])
    .on_press(Message::ApplyElementTransition);

    footer = footer.push(cancel_btn).push(apply_btn);

    let dialog_card = container(
        column![
            header_col,
            Space::new().height(6),
            quick_steps,
            Space::new().height(6),
            scrollable(effect_col).height(Length::FillPortion(1)),
            Space::new().height(6),
            preview_bar,
            Space::new().height(8),
            footer,
        ]
        .spacing(6),
    )
    .width(Length::Fixed(620.0))
    .max_height(580.0)
    .padding(20)
    .style(move |_| theme::modal_dialog_style(theme));

    container(dialog_card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_| theme::modal_backdrop_style(theme))
        .into()
}
