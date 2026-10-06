//! Modals for Exporting, File Open/Save, and Compiler Error inspection.

use crate::app::ExportFormat;
use crate::app::ExportPageSelection;
use crate::app::Message;
use crate::compiler_bridge::CompilerDiagnostic;
use crate::ui::theme::AppTheme;
use crate::ui::theme::{
    self,
};
use iced::Alignment;
use iced::Background;
use iced::Element;
use iced::Length;
use iced::Point;
use iced::Size;
use iced::widget::Space;
use iced::widget::button;
use iced::widget::column;
use iced::widget::container;
use iced::widget::mouse_area;
use iced::widget::row;
use iced::widget::scrollable;
use iced::widget::stack;
use iced::widget::text;
use iced::widget::text_input;
use std::path::Path;

/// Render the Export modal dialog
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn view_export_modal<'a>(
    theme: AppTheme,
    selected_format: ExportFormat,
    output_path: &'a str,
    png_scale: f32,
    page_selection: ExportPageSelection,
    custom_range: &'a str,
    include_source: bool,
    active_slide: usize,
    total_slides: usize,
    status_msg: Option<&'a str>,
) -> Element<'a, Message> {
    let title = text("Export Presentation & Documents")
        .size(18)
        .color(theme.text_primary());

    let subtitle = text("Render and export your presentation to various formats.")
        .size(13)
        .color(theme.text_muted());

    // Format selection buttons
    let format_btn = |fmt: ExportFormat, label: &'static str| {
        let is_selected = selected_format == fmt;
        button(text(label).size(13))
            .style(move |_theme, _status| theme::subtle_button_style(theme, is_selected))
            .padding([8, 14])
            .on_press(Message::SelectExportFormat(fmt))
    };

    let formats_row = row![
        format_btn(ExportFormat::Pdf, "PDF Document"),
        format_btn(ExportFormat::Png, "PNG Images"),
        format_btn(ExportFormat::Svg, "Vector SVG"),
        format_btn(ExportFormat::SlidePackage, ".slide Bundle"),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // Output path input
    let path_label = text("Target Output File / Directory:")
        .size(13)
        .color(theme.text_secondary());
    let path_input = text_input("e.g. presentation.pdf or dist/", output_path)
        .on_input(Message::ExportPathChanged)
        .padding(8);

    let browse_btn = button(text("Browse...").size(13))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([8, 14])
        .on_press(Message::BrowseExportPath);

    let path_row = row![path_input, browse_btn]
        .spacing(8)
        .align_y(Alignment::Center);

    // Page range selection for SVG and PNG
    let page_options: Element<'a, Message> =
        if matches!(selected_format, ExportFormat::Svg | ExportFormat::Png) {
            let page_btn = |sel: ExportPageSelection, label: String| {
                let is_sel = page_selection == sel;
                button(text(label).size(12))
                    .style(move |_theme, _status| theme::subtle_button_style(theme, is_sel))
                    .padding([4, 10])
                    .on_press(Message::SelectExportPageSelection(sel))
            };

            let mut page_col = column![
                row![
                    text("Slide Range:").size(12).color(theme.text_secondary()),
                    page_btn(
                        ExportPageSelection::All,
                        format!("All Slides ({})", total_slides.max(1))
                    ),
                    page_btn(
                        ExportPageSelection::Current,
                        format!("Current Slide ({})", active_slide + 1)
                    ),
                    page_btn(ExportPageSelection::Custom, "Custom Range".to_string()),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
            ]
            .spacing(6);

            if page_selection == ExportPageSelection::Custom {
                let custom_input = row![
                    text("Pages:").size(12).color(theme.text_muted()),
                    text_input("e.g. 1, 3-5, 8", custom_range)
                        .on_input(Message::ExportCustomRangeChanged)
                        .padding([4, 8])
                        .width(Length::Fixed(160.0)),
                    text(format!("(Valid: 1..{})", total_slides.max(1)))
                        .size(11)
                        .color(theme.text_muted()),
                ]
                .spacing(8)
                .align_y(Alignment::Center);
                page_col = page_col.push(custom_input);
            }

            page_col.into()
        } else {
            Space::new().height(0).into()
        };

    // PNG scale options if PNG is selected
    let extra_options: Element<'a, Message> = if selected_format == ExportFormat::Png {
        let scale_btn = |s: f32, label: &'static str| {
            let is_sel = (png_scale - s).abs() < 0.01;
            button(text(label).size(12))
                .style(move |_theme, _status| theme::subtle_button_style(theme, is_sel))
                .padding([4, 10])
                .on_press(Message::SelectPngScale(s))
        };

        row![
            text("Image Resolution:")
                .size(12)
                .color(theme.text_secondary()),
            scale_btn(1.0, "1x (Standard)"),
            scale_btn(2.0, "2x (High-Res 1080p)"),
            scale_btn(3.0, "3x (Ultra 4K)"),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    } else if selected_format == ExportFormat::SlidePackage {
        let checkbox_label = if include_source {
            "[x]"
        } else {
            "[ ]"
        };
        let source_toggle = button(
            row![
                container(text(checkbox_label).size(12).color(if include_source { theme.accent() } else { theme.text_muted() }))
                    .padding([2, 4]),
                column![
                    text("Include Editable Origin Source & Assets (.typ)").size(12).color(theme.text_primary()),
                    text("Packages full editable source and referenced media so other users can edit this presentation").size(10).color(theme.text_muted()),
                ]
                .spacing(2),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .padding([6, 10])
        .style(move |_t, _s| theme::subtle_button_style(theme, include_source))
        .on_press(Message::ToggleExportIncludeSource);

        container(source_toggle).into()
    } else {
        Space::new().height(0).into()
    };

    // Status or feedback message
    let status_widget: Element<'a, Message> = if let Some(msg) = status_msg {
        text(msg).size(12).color(theme.success()).into()
    } else {
        Space::new().height(0).into()
    };

    // Actions
    let cancel_btn = button(text("Cancel").size(13))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([8, 16])
        .on_press(Message::CloseModal);

    let do_export_btn = button(text("Start Export").size(13))
        .style(move |_theme, _status| theme::primary_button_style(theme))
        .padding([8, 20])
        .on_press(Message::ExecuteExport);

    let actions = row![
        status_widget,
        Space::new().width(Length::Fill),
        cancel_btn,
        do_export_btn
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let dialog_content = column![
        title,
        subtitle,
        Space::new().height(8),
        formats_row,
        Space::new().height(8),
        path_label,
        path_row,
        page_options,
        extra_options,
        Space::new().height(14),
        actions
    ]
    .spacing(8);

    let dialog_card = container(dialog_content)
        .width(Length::Fixed(560.0))
        .padding(24)
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

/// Render the Open Document dialog
#[must_use]
pub fn view_open_modal<'a>(
    theme: AppTheme,
    input_path: &'a str,
    error_msg: Option<&'a str>,
    recent_files: &'a [String],
) -> Element<'a, Message> {
    let title = text("Open Presentation or Document")
        .size(18)
        .color(theme.text_primary());

    let subtitle = text("Open a Typst source file (.typ) or standalone Slide package (.slide)")
        .size(13)
        .color(theme.text_muted());

    let path_input = text_input("Enter path to .typ or .slide file...", input_path)
        .on_input(Message::OpenPathChanged)
        .padding(8);

    let browse_btn = button(text("Browse...").size(13))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([8, 14])
        .on_press(Message::BrowseOpenPath);

    let path_row = row![path_input, browse_btn]
        .spacing(8)
        .align_y(Alignment::Center);

    let mut recents_widget: Element<'a, Message> = Space::new().height(0).into();
    if !recent_files.is_empty() {
        let mut recents_col = column![
            text("Recent Presentations:")
                .size(11)
                .color(theme.text_secondary()),
        ]
        .spacing(4);
        for rf in recent_files.iter().take(5) {
            let rf_clone = rf.clone();
            let display_name = Path::new(rf)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(rf);
            let btn = button(
                row![
                    text("•").size(11),
                    text(display_name).size(12).color(theme.accent()),
                    text(rf).size(10).color(theme.text_muted()),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([4, 8])
            .style(move |_t, _s| theme::subtle_button_style(theme, false))
            .on_press(Message::OpenRecentFile(rf_clone));
            recents_col = recents_col.push(btn);
        }
        recents_widget = container(recents_col)
            .padding(8)
            .style(move |_| {
                container::Style {
                    background: Some(iced::Background::Color(theme.bg_subtle())),
                    border: iced::border::rounded(6.0),
                    ..container::Style::default()
                }
            })
            .into();
    }

    let err_widget: Element<'a, Message> = if let Some(e) = error_msg {
        text(e).size(12).color(theme.danger()).into()
    } else {
        Space::new().height(0).into()
    };

    let cancel_btn = button(text("Cancel").size(13))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([8, 16])
        .on_press(Message::CloseModal);

    let open_btn = button(text("Open Document").size(13))
        .style(move |_theme, _status| theme::primary_button_style(theme))
        .padding([8, 20])
        .on_press(Message::ExecuteOpen);

    let actions = row![
        err_widget,
        Space::new().width(Length::Fill),
        cancel_btn,
        open_btn
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let dialog_content = column![
        title,
        subtitle,
        Space::new().height(10),
        path_row,
        recents_widget,
        Space::new().height(10),
        actions
    ]
    .spacing(8);

    let dialog_card = container(dialog_content)
        .width(Length::Fixed(560.0))
        .padding(24)
        .style(move |_| theme::modal_dialog_style(theme));

    container(dialog_card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_| theme::modal_backdrop_style(theme))
        .into()
}

/// Render the Compiler Diagnostic error details modal
#[must_use]
pub fn view_error_details_modal<'a>(
    theme: AppTheme,
    diagnostic: &'a CompilerDiagnostic,
) -> Element<'a, Message> {
    let title = row![
        text("Diagnostic:").size(16).color(theme.danger()),
        text("Typst Compilation Diagnostics")
            .size(16)
            .color(theme.text_primary())
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let location = if let Some(l) = diagnostic.line {
        format!("Location: Line {l}")
    } else {
        "Location: Document root".to_string()
    };

    let loc_text = text(location).size(13).color(theme.text_secondary());
    let msg_text = text(&diagnostic.message).size(13).color(theme.danger());

    let stderr_box = container(
        scrollable(
            text(&diagnostic.full_stderr)
                .size(12)
                .color(theme.text_secondary()),
        )
        .height(Length::Fixed(200.0)),
    )
    .padding(10)
    .style(move |_| {
        container::Style {
            background: Some(iced::Background::Color(theme.bg_subtle())),
            border: iced::border::rounded(6.0),
            ..container::Style::default()
        }
    });

    let quick_fix_widget: Element<'a, Message> = if let Some(ref qf) = diagnostic.quick_fix {
        let qf_clone = qf.clone();
        container(
            row![
                text("[TIP]").size(11),
                column![
                    text("Automated Quick Fix Available")
                        .size(11)
                        .color(theme.accent()),
                    text(&qf.label).size(12).color(theme.text_primary()),
                ]
                .spacing(2),
                Space::new().width(Length::Fill),
                button(text("Apply Quick Fix").size(12))
                    .style(move |_t, _s| theme::primary_button_style(theme))
                    .padding([6, 14])
                    .on_press(Message::ApplyQuickFix(qf_clone)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding(10)
        .style(move |_| {
            container::Style {
                background: Some(iced::Background::Color(theme.bg_subtle())),
                border: iced::border::rounded(6.0),
                ..container::Style::default()
            }
        })
        .into()
    } else {
        Space::new().height(0).into()
    };

    let close_btn = button(text("Close").size(13))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([8, 18])
        .on_press(Message::CloseModal);

    let actions = row![Space::new().width(Length::Fill), close_btn];

    let dialog_content = column![
        title,
        loc_text,
        msg_text,
        quick_fix_widget,
        Space::new().height(6),
        stderr_box,
        Space::new().height(10),
        actions
    ]
    .spacing(8);

    let dialog_card = container(dialog_content)
        .width(Length::Fixed(600.0))
        .padding(24)
        .style(move |_| theme::modal_dialog_style(theme));

    container(dialog_card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_| theme::modal_backdrop_style(theme))
        .into()
}

/// Render modal to move an element from one slide to any other slide
#[must_use]
pub fn view_move_block_modal<'a>(
    theme: AppTheme,
    from_slide_idx: usize,
    block_range: std::ops::Range<usize>,
    block_label: &'a str,
    slide_titles: &'a [String],
    total_slides: usize,
) -> Element<'a, Message> {
    let title = text("Move Element to Slide")
        .size(18)
        .color(theme.text_primary());

    let subtitle = text(format!(
        "Choose target slide to move this element from Slide {}:",
        from_slide_idx + 1
    ))
    .size(13)
    .color(theme.text_muted());

    let preview_badge = container(
        text(format!("Element: {block_label}"))
            .size(12)
            .color(theme.accent()),
    )
    .padding([4, 10])
    .style(move |_| {
        container::Style {
            background: Some(iced::Background::Color(theme.bg_subtle())),
            border: iced::border::rounded(6.0),
            ..Default::default()
        }
    });

    let mut quick_nav = row![].spacing(8).align_y(Alignment::Center);
    if from_slide_idx > 0 {
        let prev_idx = from_slide_idx - 1;
        let range_c = block_range.clone();
        quick_nav = quick_nav.push(
            button(text("< Move to Previous Slide").size(12))
                .style(move |_theme, _status| theme::subtle_button_style(theme, false))
                .padding([6, 12])
                .on_press(Message::MoveBlockToSlide {
                    from_slide_idx,
                    block_range: range_c,
                    to_slide_idx: prev_idx,
                }),
        );
    }
    if from_slide_idx + 1 < total_slides {
        let next_idx = from_slide_idx + 1;
        let range_c = block_range.clone();
        quick_nav = quick_nav.push(
            button(text("Move to Next Slide >").size(12))
                .style(move |_theme, _status| theme::subtle_button_style(theme, false))
                .padding([6, 12])
                .on_press(Message::MoveBlockToSlide {
                    from_slide_idx,
                    block_range: range_c,
                    to_slide_idx: next_idx,
                }),
        );
    }

    let mut slide_items = column![].spacing(6);
    for idx in 0..total_slides {
        let is_current = idx == from_slide_idx;
        let slide_num = idx + 1;
        let stitle = slide_titles
            .get(idx)
            .cloned()
            .unwrap_or_else(|| format!("Slide {slide_num}"));
        let range_c = block_range.clone();

        let num_badge = container(
            text(format!("{slide_num:02}"))
                .size(11)
                .color(if is_current {
                    theme.text_muted()
                } else {
                    iced::Color::WHITE
                }),
        )
        .padding([2, 6])
        .style(move |_| {
            container::Style {
                background: Some(iced::Background::Color(if is_current {
                    theme.bg_subtle()
                } else {
                    theme.accent()
                })),
                border: iced::border::rounded(theme::RADIUS_FULL),
                ..Default::default()
            }
        });

        let label = text(if is_current {
            format!("{stitle} (Current Slide)")
        } else {
            stitle
        })
        .size(13)
        .color(if is_current {
            theme.text_muted()
        } else {
            theme.text_primary()
        });

        let item_row = row![num_badge, label]
            .spacing(10)
            .align_y(Alignment::Center);

        let mut item_btn = button(item_row)
            .width(Length::Fill)
            .padding([8, 12])
            .style(move |_theme, _status| theme::subtle_button_style(theme, false));

        if !is_current {
            item_btn = item_btn.on_press(Message::MoveBlockToSlide {
                from_slide_idx,
                block_range: range_c,
                to_slide_idx: idx,
            });
        }

        slide_items = slide_items.push(item_btn);
    }

    let list_container =
        container(scrollable(slide_items.padding([4, 6])).height(Length::Fixed(240.0)))
            .padding(6)
            .style(move |_| {
                container::Style {
                    background: Some(iced::Background::Color(theme.bg_subtle())),
                    border: iced::border::rounded(theme::RADIUS_MD),
                    ..Default::default()
                }
            });

    let cancel_btn = button(text("Cancel").size(13))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([8, 18])
        .on_press(Message::CloseModal);

    let content = column![
        title,
        subtitle,
        preview_badge,
        Space::new().height(4),
        quick_nav,
        Space::new().height(4),
        list_container,
        Space::new().height(8),
        row![Space::new().width(Length::Fill), cancel_btn],
    ]
    .spacing(8);

    let card = container(content)
        .width(Length::Fixed(480.0))
        .padding(24)
        .style(move |_| theme::modal_dialog_style(theme));

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_| theme::modal_backdrop_style(theme))
        .into()
}

/// Render right-click context menu popup for a slide
#[must_use]
pub fn view_slide_context_menu_modal<'a>(
    theme: AppTheme,
    slide_idx: usize,
    total_slides: usize,
    position: Option<Point>,
    window_size: Size,
) -> Element<'a, Message> {
    let slide_num = slide_idx + 1;

    let header_label = container(
        text(format!("Slide {slide_num}"))
            .size(11)
            .color(theme.text_muted()),
    )
    .padding([2, 8]);

    let divider = || {
        container(Space::new())
            .height(Length::Fixed(1.0))
            .width(Length::Fill)
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.border_color())),
                    ..container::Style::default()
                }
            })
    };

    let menu_btn = |icon: &'static str, label: &'static str, msg: Message, danger: bool| {
        button(
            row![
                text(icon).size(11).color(if danger {
                    theme.danger()
                } else {
                    theme.text_secondary()
                }),
                Space::new().width(6),
                text(label).size(12).color(if danger {
                    theme.danger()
                } else {
                    theme.text_primary()
                }),
            ]
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([5, 8])
        .style(move |_theme, status| {
            let is_hovered = matches!(status, iced::widget::button::Status::Hovered);
            iced::widget::button::Style {
                background: if is_hovered {
                    Some(iced::Background::Color(if danger {
                        iced::Color::from_rgba(0.9, 0.2, 0.2, 0.15)
                    } else {
                        theme.bg_subtle()
                    }))
                } else {
                    None
                },
                text_color: if danger {
                    theme.danger()
                } else {
                    theme.text_primary()
                },
                border: iced::border::rounded(theme::RADIUS_XS),
                ..Default::default()
            }
        })
        .on_press(msg)
    };

    let mut menu_items = column![
        header_label,
        divider(),
        menu_btn(
            "+",
            "Insert Slide Above",
            Message::InsertSlideAt(slide_idx),
            false
        ),
        menu_btn(
            "+",
            "Insert Slide Below",
            Message::InsertSlideAt(slide_idx + 1),
            false
        ),
        menu_btn(
            "❐",
            "Duplicate Slide",
            Message::DuplicateSlide(slide_idx),
            false
        ),
    ]
    .spacing(2);

    if slide_idx > 0 || slide_idx + 1 < total_slides {
        menu_items = menu_items.push(divider());
        if slide_idx > 0 {
            menu_items = menu_items.push(menu_btn(
                "▲",
                "Move Slide Up",
                Message::MoveSlide {
                    from_idx: slide_idx,
                    to_idx: slide_idx - 1,
                },
                false,
            ));
        }
        if slide_idx + 1 < total_slides {
            menu_items = menu_items.push(menu_btn(
                "▼",
                "Move Slide Down",
                Message::MoveSlide {
                    from_idx: slide_idx,
                    to_idx: slide_idx + 1,
                },
                false,
            ));
        }
    }

    menu_items = menu_items.push(divider());
    menu_items = menu_items.push(menu_btn(
        "⚡",
        "Slide Transition...",
        Message::OpenTransitionModal(slide_idx),
        false,
    ));

    if total_slides > 1 {
        menu_items = menu_items.push(divider());
        menu_items = menu_items.push(menu_btn(
            "✕",
            "Delete Slide",
            Message::DeleteSlide(slide_idx),
            true,
        ));
    }

    let card = container(menu_items)
        .width(Length::Fixed(210.0))
        .padding(4)
        .style(move |_| theme::context_menu_card_style(theme));

    let backdrop = mouse_area(
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(Message::CloseModal);

    let menu_w = 210.0;
    let menu_h = 240.0;
    let (pos_x, pos_y) = if let Some(p) = position {
        let x = (p.x + 2.0).clamp(6.0, (window_size.width - menu_w - 10.0).max(6.0));
        let y = (p.y + 2.0).clamp(6.0, (window_size.height - menu_h - 10.0).max(6.0));
        (x, y)
    } else {
        (180.0, 90.0)
    };

    let menu_placement = row![
        Space::new().width(Length::Fixed(pos_x)),
        column![Space::new().height(Length::Fixed(pos_y)), card,]
    ];

    stack![backdrop, menu_placement,].into()
}

/// Render right-click context menu modal for a WYSIWYG block
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn view_block_context_menu_modal<'a>(
    theme: AppTheme,
    slide_idx: usize,
    block_idx: usize,
    range: std::ops::Range<usize>,
    block_label: &'a str,
    block_id: &'a str,
    position: Option<Point>,
    window_size: Size,
) -> Element<'a, Message> {
    let header_label = container(
        text(format!("Block: {block_label}"))
            .size(11)
            .color(theme.text_muted()),
    )
    .padding([2, 8]);

    let divider = || {
        container(Space::new())
            .height(Length::Fixed(1.0))
            .width(Length::Fill)
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.border_color())),
                    ..container::Style::default()
                }
            })
    };

    let menu_btn = |icon: &'static str, label: &'static str, msg: Message, danger: bool| {
        button(
            row![
                text(icon).size(11).color(if danger {
                    theme.danger()
                } else {
                    theme.text_secondary()
                }),
                Space::new().width(6),
                text(label).size(12).color(if danger {
                    theme.danger()
                } else {
                    theme.text_primary()
                }),
            ]
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([5, 8])
        .style(move |_theme, status| {
            let is_hovered = matches!(status, iced::widget::button::Status::Hovered);
            iced::widget::button::Style {
                background: if is_hovered {
                    Some(iced::Background::Color(if danger {
                        iced::Color::from_rgba(0.9, 0.2, 0.2, 0.15)
                    } else {
                        theme.bg_subtle()
                    }))
                } else {
                    None
                },
                text_color: if danger {
                    theme.danger()
                } else {
                    theme.text_primary()
                },
                border: iced::border::rounded(theme::RADIUS_XS),
                ..Default::default()
            }
        })
        .on_press(msg)
    };

    let range_c1 = range.clone();
    let range_c2 = range.clone();
    let range_c3 = range.clone();
    let label_s = block_label.to_string();

    let menu_items = column![
        header_label,
        divider(),
        menu_btn(
            "➔",
            "Move to Another Slide...",
            Message::OpenMoveBlockModal {
                from_slide_idx: slide_idx,
                block_idx,
                range: range_c1,
                block_label: label_s,
            },
            false
        ),
        menu_btn(
            "❐",
            "Duplicate Element",
            Message::DuplicateBlock(range_c2),
            false
        ),
        menu_btn(
            "✎",
            "Edit Raw Typst Code",
            Message::ToggleBlockRawCode(block_id.to_string()),
            false
        ),
        menu_btn(
            "⚡",
            "Configure Step Animation...",
            Message::OpenElementTransitionModal { slide_idx, block_idx },
            false
        ),
        divider(),
        menu_btn(
            "✕",
            "Delete Element",
            Message::DeleteBlockAtRange(range_c3),
            true
        ),
    ]
    .spacing(2);

    let card = container(menu_items)
        .width(Length::Fixed(220.0))
        .padding(4)
        .style(move |_| theme::context_menu_card_style(theme));

    let backdrop = mouse_area(
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(Message::CloseModal);

    let menu_w = 220.0;
    let menu_h = 190.0;
    let (pos_x, pos_y) = if let Some(p) = position {
        let x = (p.x + 2.0).clamp(6.0, (window_size.width - menu_w - 10.0).max(6.0));
        let y = (p.y + 2.0).clamp(6.0, (window_size.height - menu_h - 10.0).max(6.0));
        (x, y)
    } else {
        (240.0, 160.0)
    };

    let menu_placement = row![
        Space::new().width(Length::Fixed(pos_x)),
        column![Space::new().height(Length::Fixed(pos_y)), card,]
    ];

    stack![backdrop, menu_placement,].into()
}

/// Render font selector modal
#[must_use]
pub fn view_font_selector_modal<'a>(
    theme: AppTheme,
    search_query: &'a str,
    current_font: &'a str,
) -> Element<'a, Message> {
    let title = text("Select Presentation Font")
        .size(18)
        .color(theme.text_primary());

    let subtitle = text("Choose from installed system fonts. Font files are automatically bundled when exporting origin packages.")
        .size(13)
        .color(theme.text_muted());

    let search_bar = text_input(
        "Search font family (e.g. Nimbus, Sans, Mono)...",
        search_query,
    )
    .on_input(Message::FontSearchQueryChanged)
    .padding(8);

    let mut all_fonts: Vec<&slide_core::font::FontFamilyInfo> =
        slide_core::font::get_system_fonts().values().collect();
    all_fonts.sort_by(|a, b| a.name.cmp(&b.name));
    let q = search_query.trim().to_lowercase();
    let filtered_fonts: Vec<_> = all_fonts
        .into_iter()
        .filter(|f| q.is_empty() || f.name.to_lowercase().contains(&q))
        .collect();

    let mut font_list = column![].spacing(4);
    for f in filtered_fonts {
        let is_curr = f.name.eq_ignore_ascii_case(current_font);
        let name_clone = f.name.clone();

        let badge = if is_curr {
            container(text("ACTIVE").size(9).color(iced::Color::WHITE))
                .padding([1, 6])
                .style(move |_| {
                    container::Style {
                        background: Some(iced::Background::Color(theme.accent())),
                        border: iced::border::rounded(theme::RADIUS_FULL),
                        ..Default::default()
                    }
                })
        } else {
            container(
                text(format!("{} files", f.file_paths.len()))
                    .size(9)
                    .color(theme.text_muted()),
            )
            .padding([1, 6])
            .style(move |_| {
                container::Style {
                    background: Some(iced::Background::Color(theme.bg_subtle())),
                    border: iced::border::rounded(theme::RADIUS_FULL),
                    ..Default::default()
                }
            })
        };

        let row_content = row![
            text(&f.name).size(13).color(if is_curr {
                theme.accent()
            } else {
                theme.text_primary()
            }),
            Space::new().width(Length::Fill),
            badge,
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let item_btn = button(row_content)
            .width(Length::Fill)
            .padding([8, 12])
            .style(move |_theme, _status| theme::subtle_button_style(theme, is_curr))
            .on_press(Message::SelectFont(name_clone));

        font_list = font_list.push(item_btn);
    }

    let list_box = container(scrollable(font_list.padding([4, 6])).height(Length::Fixed(280.0)))
        .padding(6)
        .style(move |_| {
            container::Style {
                background: Some(iced::Background::Color(theme.bg_subtle())),
                border: iced::border::rounded(theme::RADIUS_MD),
                ..Default::default()
            }
        });

    let close_btn = button(text("Close").size(13))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([8, 18])
        .on_press(Message::CloseModal);

    let content = column![
        title,
        subtitle,
        Space::new().height(4),
        search_bar,
        Space::new().height(4),
        list_box,
        Space::new().height(8),
        row![Space::new().width(Length::Fill), close_btn],
    ]
    .spacing(8);

    let card = container(content)
        .width(Length::Fixed(520.0))
        .padding(24)
        .style(move |_| theme::modal_dialog_style(theme));

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_| theme::modal_backdrop_style(theme))
        .into()
}

/// Render sleek, non-blocking floating Find & Replace bar (docked at top-right, no modal backdrop)
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn view_search_replace_bar<'a>(
    theme: AppTheme,
    search_query: &'a str,
    replace_query: &'a str,
    is_regex: bool,
    case_sensitive: bool,
    whole_word: bool,
    match_info: Option<&'a str>,
) -> Element<'a, Message> {
    let search_input = text_input("Find in document...", search_query)
        .id(iced::widget::Id::new("floating_search_input"))
        .on_input(Message::SearchQueryChanged)
        .on_submit(Message::FindNextMatch)
        .padding(5)
        .size(12);

    let regex_btn = button(text(".*").size(11))
        .style(move |_t, _s| {
            if is_regex {
                theme::primary_button_style(theme)
            } else {
                theme::subtle_button_style(theme, false)
            }
        })
        .padding([3, 7])
        .on_press(Message::ToggleSearchRegex);

    let case_btn = button(text("Aa").size(11))
        .style(move |_t, _s| {
            if case_sensitive {
                theme::primary_button_style(theme)
            } else {
                theme::subtle_button_style(theme, false)
            }
        })
        .padding([3, 7])
        .on_press(Message::ToggleSearchCaseSensitive);

    let word_btn = button(text("\\b").size(11))
        .style(move |_t, _s| {
            if whole_word {
                theme::primary_button_style(theme)
            } else {
                theme::subtle_button_style(theme, false)
            }
        })
        .padding([3, 7])
        .on_press(Message::ToggleSearchWholeWord);

    let match_badge = text(match_info.unwrap_or(if search_query.is_empty() {
        ""
    } else {
        "No matches"
    }))
    .size(11)
    .color(theme.accent());

    let prev_btn = button(text("<").size(11))
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .padding([3, 8])
        .on_press(Message::FindPrevMatch);

    let next_btn = button(text(">").size(11))
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .padding([3, 8])
        .on_press(Message::FindNextMatch);

    let close_btn = button(text("X").size(11))
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .padding([3, 7])
        .on_press(Message::CloseSearchBar);

    let find_row = row![
        search_input,
        regex_btn,
        case_btn,
        word_btn,
        match_badge,
        prev_btn,
        next_btn,
        close_btn
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let replace_input = text_input("Replace with...", replace_query)
        .on_input(Message::ReplaceQueryChanged)
        .on_submit(Message::ExecuteReplaceCurrent)
        .padding(5)
        .size(12);

    let replace_btn = button(text("Replace").size(11))
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .padding([3, 10])
        .on_press(Message::ExecuteReplaceCurrent);

    let replace_all_btn = button(text("Replace All").size(11))
        .style(move |_t, _s| theme::primary_button_style(theme))
        .padding([3, 10])
        .on_press(Message::ExecuteReplaceAllMatches);

    let replace_row = row![replace_input, replace_btn, replace_all_btn]
        .spacing(6)
        .align_y(Alignment::Center);

    let content = column![find_row, replace_row].spacing(6);

    container(content)
        .width(Length::Fixed(440.0))
        .padding(10)
        .style(move |_| {
            container::Style {
                background: Some(iced::Background::Color(theme.bg_surface())),
                border: iced::border::Border {
                    color: theme.accent().scale_alpha(0.7),
                    width: 1.5,
                    radius: iced::border::Radius::from(8.0),
                },
                shadow: iced::Shadow {
                    color: iced::Color::BLACK.scale_alpha(0.35),
                    offset: iced::Vector::new(0.0, 4.0),
                    blur_radius: 12.0,
                },
                ..container::Style::default()
            }
        })
        .into()
}

/// Render Presentation Header & Footer customization modal
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn view_header_footer_modal<'a>(
    theme: AppTheme,
    header_enabled: bool,
    header_left: &'a str,
    header_right: &'a str,
    footer_enabled: bool,
    footer_left: &'a str,
    footer_right_mode: usize,
    footer_right_custom: &'a str,
) -> Element<'a, Message> {
    let title = text("Header & Footer Settings")
        .size(18)
        .color(theme.text_primary());

    let subtitle = text("Customize top header and bottom footer (left title and page numbering).")
        .size(12)
        .color(theme.text_muted());

    // --- Header Section ---
    let header_toggle = button(
        text(if header_enabled {
            "[x] Enable Header"
        } else {
            "[ ] Enable Header"
        })
        .size(12),
    )
    .style(move |_t, _s| {
        if header_enabled {
            theme::primary_button_style(theme)
        } else {
            theme::subtle_button_style(theme, false)
        }
    })
    .padding([4, 10])
    .on_press(Message::ToggleHeaderEnabled(!header_enabled));

    let header_inputs = if header_enabled {
        row![
            text_input("Left header (e.g. Presentation Topic)", header_left)
                .on_input(Message::HeaderLeftChanged)
                .padding(7),
            text_input("Right header (e.g. Chapter / Date)", header_right)
                .on_input(Message::HeaderRightChanged)
                .padding(7)
        ]
        .spacing(10)
    } else {
        row![]
    };

    // --- Footer Section ---
    let footer_toggle = button(
        text(if footer_enabled {
            "[x] Enable Footer"
        } else {
            "[ ] Enable Footer"
        })
        .size(12),
    )
    .style(move |_t, _s| {
        if footer_enabled {
            theme::primary_button_style(theme)
        } else {
            theme::subtle_button_style(theme, false)
        }
    })
    .padding([4, 10])
    .on_press(Message::ToggleFooterEnabled(!footer_enabled));

    let footer_inputs = if footer_enabled {
        let left_input = text_input(
            "Left footer (e.g. Company / Brand, blank = empty)",
            footer_left,
        )
        .on_input(Message::FooterLeftChanged)
        .padding(7);

        let p1 = button(text("Page Number (1 / N)").size(11))
            .style(move |_t, _s| {
                if footer_right_mode == 0 {
                    theme::primary_button_style(theme)
                } else {
                    theme::subtle_button_style(theme, false)
                }
            })
            .padding([3, 8])
            .on_press(Message::FooterRightModeChanged(0));

        let p2 = button(text("Custom Text").size(11))
            .style(move |_t, _s| {
                if footer_right_mode == 1 {
                    theme::primary_button_style(theme)
                } else {
                    theme::subtle_button_style(theme, false)
                }
            })
            .padding([3, 8])
            .on_press(Message::FooterRightModeChanged(1));

        let p3 = button(text("None").size(11))
            .style(move |_t, _s| {
                if footer_right_mode == 2 {
                    theme::primary_button_style(theme)
                } else {
                    theme::subtle_button_style(theme, false)
                }
            })
            .padding([3, 8])
            .on_press(Message::FooterRightModeChanged(2));

        let mode_row = row![
            text("Right footer:").size(11).color(theme.text_secondary()),
            p1,
            p2,
            p3
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let custom_row = if footer_right_mode == 1 {
            row![
                text_input("Custom right footer text", footer_right_custom)
                    .on_input(Message::FooterRightCustomChanged)
                    .padding(7)
            ]
        } else {
            row![]
        };

        column![left_input, mode_row, custom_row].spacing(8)
    } else {
        column![]
    };

    // Live layout preview mockup
    let preview_hl = if header_enabled && !header_left.is_empty() {
        header_left
    } else {
        "Header Left"
    };
    let preview_hr = if header_enabled && !header_right.is_empty() {
        header_right
    } else {
        "Header Right"
    };
    let preview_fl = if footer_enabled && !footer_left.is_empty() {
        footer_left
    } else if footer_enabled {
        "(No Left Footer)"
    } else {
        ""
    };
    let preview_fr = if footer_enabled && footer_right_mode == 0 {
        "1 / 10"
    } else if footer_enabled && footer_right_mode == 1 {
        footer_right_custom
    } else {
        ""
    };

    let mock_header = if header_enabled {
        row![
            text(preview_hl).size(9).color(theme.text_muted()),
            Space::new().width(Length::Fill),
            text(preview_hr).size(9).color(theme.text_muted()),
        ]
        .padding([4, 8])
    } else {
        row![
            text("Header disabled")
                .size(9)
                .color(theme.text_muted().scale_alpha(0.4))
        ]
        .padding([4, 8])
    };

    let mock_footer = if footer_enabled {
        row![
            text(preview_fl)
                .size(9)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..iced::Font::DEFAULT
                })
                .color(theme.text_muted()),
            Space::new().width(Length::Fill),
            text(preview_fr).size(9).color(theme.text_muted()),
        ]
        .padding([4, 8])
    } else {
        row![
            text("Footer disabled")
                .size(9)
                .color(theme.text_muted().scale_alpha(0.4))
        ]
        .padding([4, 8])
    };

    let mock_slide = container(
        column![
            mock_header,
            container(
                text("Slide Content Preview")
                    .size(11)
                    .color(theme.text_muted().scale_alpha(0.5))
            )
            .height(Length::Fixed(40.0))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center),
            mock_footer
        ]
        .spacing(4),
    )
    .width(Length::Fill)
    .padding(6)
    .style(move |_| {
        container::Style {
            background: Some(iced::Background::Color(theme.bg_subtle())),
            border: iced::border::rounded(theme::RADIUS_SM),
            ..container::Style::default()
        }
    });

    let apply_btn = button(text("Apply to Document").size(12))
        .style(move |_t, _s| theme::primary_button_style(theme))
        .padding([6, 14])
        .on_press(Message::ApplyHeaderFooterSettings);

    let cancel_btn = button(text("Cancel").size(12))
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .padding([6, 12])
        .on_press(Message::CloseModal);

    let actions = row![Space::new().width(Length::Fill), cancel_btn, apply_btn]
        .spacing(10)
        .align_y(Alignment::Center);

    let content = column![
        title,
        subtitle,
        Space::new().height(4),
        header_toggle,
        header_inputs,
        Space::new().height(4),
        footer_toggle,
        footer_inputs,
        Space::new().height(4),
        text("LAYOUT PREVIEW")
            .size(10)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::DEFAULT
            })
            .color(theme.accent()),
        mock_slide,
        Space::new().height(6),
        actions
    ]
    .spacing(8);

    let card = container(content)
        .width(Length::Fixed(560.0))
        .padding(22)
        .style(move |_| theme::modal_dialog_style(theme));

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_| theme::modal_backdrop_style(theme))
        .into()
}

/// Render the Template Gallery modal dialog offering modern pre-designed slides
#[must_use]
pub fn view_template_library_modal<'a>(theme: AppTheme) -> Element<'a, Message> {
    let title = text("Slide Template Library")
        .size(18)
        .color(theme.text_primary());

    let subtitle =
        text("Choose a professionally crafted layout to instantly insert into your deck.")
            .size(13)
            .color(theme.text_muted());

    let templates = [
        (
            "Title Hero",
            "Hero title slide with subtitle, presenter name, and date badge",
            "HERO",
            0usize,
        ),
        (
            "2-Column Comparison",
            "Side-by-side comparison with colored callout blocks",
            "COLUMNS",
            1usize,
        ),
        (
            "3-Column Feature Pillars",
            "Three architectural pillars with distinct border accents",
            "GRID",
            2usize,
        ),
        (
            "Code & Commentary",
            "Split view with syntax-highlighted code block and explanation",
            "CODE",
            3usize,
        ),
        (
            "Metric KPI Showcase",
            "Highlight three key quantitative performance metrics",
            "METRICS",
            4usize,
        ),
        (
            "Quote Spotlight",
            "Centered elegant quotation with author attribution",
            "QUOTE",
            5usize,
        ),
        (
            "Roadmap Timeline",
            "Sequential checklist of project milestones and progress",
            "TIMELINE",
            6usize,
        ),
        (
            "Closing Q&A",
            "Clean contact, repository links, and thank you card",
            "CLOSING",
            7usize,
        ),
    ];

    let mut grid_col = column![].spacing(10);
    let mut current_row = row![].spacing(10);
    let mut row_count = 0;

    for (i, (t_title, t_desc, t_tag, t_idx)) in templates.iter().enumerate() {
        let tag_badge = container(text(*t_tag).size(9).color(theme.accent()))
            .padding([2, 6])
            .style(move |_| {
                container::Style {
                    background: Some(iced::Background::Color(theme.bg_subtle())),
                    border: iced::border::rounded(theme::RADIUS_FULL),
                    ..container::Style::default()
                }
            });

        let card_header = row![
            text(*t_title).size(13).color(theme.text_primary()),
            Space::new().width(Length::Fill),
            tag_badge,
        ]
        .align_y(Alignment::Center);

        let desc_text = text(*t_desc).size(11).color(theme.text_secondary());

        let insert_btn = button(text("Insert Slide").size(11))
            .style(move |_t, _s| theme::primary_button_style(theme))
            .padding([5, 12])
            .on_press(Message::InsertTemplateSlide(*t_idx));

        let card_content = column![
            card_header,
            desc_text,
            Space::new().height(4),
            row![Space::new().width(Length::Fill), insert_btn],
        ]
        .spacing(6);

        let card = container(card_content)
            .width(Length::Fixed(290.0))
            .padding(12)
            .style(move |_| {
                container::Style {
                    background: Some(iced::Background::Color(theme.bg_surface_solid())),
                    border: iced::border::Border {
                        color: theme.border_color(),
                        width: 1.0,
                        radius: iced::border::Radius::from(theme::RADIUS_MD),
                    },
                    ..container::Style::default()
                }
            });

        current_row = current_row.push(card);
        row_count += 1;

        if (i + 1) % 2 == 0 {
            grid_col = grid_col.push(current_row);
            current_row = row![].spacing(10);
            row_count = 0;
        }
    }

    if row_count > 0 {
        grid_col = grid_col.push(current_row);
    }

    let close_btn = button(text("Close").size(13))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([8, 18])
        .on_press(Message::CloseModal);

    let dialog_content = column![
        title,
        subtitle,
        Space::new().height(8),
        scrollable(grid_col).height(Length::Fixed(360.0)),
        Space::new().height(8),
        row![Space::new().width(Length::Fill), close_btn],
    ]
    .spacing(8);

    let dialog_card = container(dialog_content)
        .width(Length::Fixed(640.0))
        .padding(24)
        .style(move |_| theme::modal_dialog_style(theme));

    container(dialog_card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_| theme::modal_backdrop_style(theme))
        .into()
}

/// Render the Presentation Health & Pacing Inspector modal dialog
#[must_use]
pub fn view_presentation_health_modal<'a>(
    theme: AppTheme,
    issues: &'a [slide_core::compiler::PresentationHealthIssue],
    pacing: &'a slide_core::pacing::DeckPacingReport,
) -> Element<'a, Message> {
    let title = row![
        text("Presentation Health & Pacing Inspector")
            .size(18)
            .color(theme.text_primary()),
    ]
    .align_y(Alignment::Center);

    // Multi-modal pacing model summary card
    let pacing_card = container(
        column![
            row![
                column![
                    text("ESTIMATED TALK TIME")
                        .size(10)
                        .color(theme.text_muted()),
                    text(format!("~{}", pacing.formatted_duration))
                        .size(18)
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..iced::Font::DEFAULT
                        })
                        .color(theme.accent()),
                    text(format!("Range: {}", pacing.duration_range))
                        .size(10)
                        .color(theme.text_secondary()),
                ]
                .spacing(2),
                container(
                    Space::new()
                        .width(Length::Fixed(1.0))
                        .height(Length::Fixed(40.0))
                )
                .style(move |_| {
                    container::Style {
                        background: Some(iced::Background::Color(theme.border_color())),
                        ..container::Style::default()
                    }
                }),
                column![
                    text("SLIDES & PACING").size(10).color(theme.text_muted()),
                    text(format!("{} slides", pacing.total_slides))
                        .size(16)
                        .color(theme.text_primary()),
                    text(format!("Avg: {}s / slide", pacing.avg_seconds_per_slide))
                        .size(10)
                        .color(theme.text_secondary()),
                ]
                .spacing(2),
                container(
                    Space::new()
                        .width(Length::Fixed(1.0))
                        .height(Length::Fixed(40.0))
                )
                .style(move |_| {
                    container::Style {
                        background: Some(iced::Background::Color(theme.border_color())),
                        ..container::Style::default()
                    }
                }),
                column![
                    text("SPEAKER NOTES").size(10).color(theme.text_muted()),
                    text(format!("{}% covered", pacing.notes_coverage_percent))
                        .size(16)
                        .color(if pacing.notes_coverage_percent > 70 {
                            theme.success()
                        } else if pacing.notes_coverage_percent > 30 {
                            theme.warning()
                        } else {
                            theme.text_muted()
                        }),
                    text(format!(
                        "{}w • {} CJK",
                        pacing.total_notes_words, pacing.total_notes_cjk
                    ))
                    .size(10)
                    .color(theme.text_secondary()),
                ]
                .spacing(2),
                container(
                    Space::new()
                        .width(Length::Fixed(1.0))
                        .height(Length::Fixed(40.0))
                )
                .style(move |_| {
                    container::Style {
                        background: Some(iced::Background::Color(theme.border_color())),
                        ..container::Style::default()
                    }
                }),
                column![
                    text("HEALTH PROFILE").size(10).color(theme.text_muted()),
                    row![
                        container(
                            text(format!("OK {}", pacing.optimal_count))
                                .size(10)
                                .color(theme.success())
                        )
                        .padding([2, 5])
                        .style(move |_| {
                            container::Style {
                                background: Some(iced::Background::Color(
                                    theme.success().scale_alpha(0.15),
                                )),
                                border: iced::border::rounded(3.0),
                                ..container::Style::default()
                            }
                        }),
                        container(
                            text(format!("WARN {}", pacing.dense_count))
                                .size(10)
                                .color(theme.warning())
                        )
                        .padding([2, 5])
                        .style(move |_| {
                            container::Style {
                                background: Some(iced::Background::Color(
                                    theme.warning().scale_alpha(0.15),
                                )),
                                border: iced::border::rounded(3.0),
                                ..container::Style::default()
                            }
                        }),
                        container(
                            text(format!("ALERT {}", pacing.overloaded_count))
                                .size(10)
                                .color(theme.danger())
                        )
                        .padding([2, 5])
                        .style(move |_| {
                            container::Style {
                                background: Some(iced::Background::Color(
                                    theme.danger().scale_alpha(0.15),
                                )),
                                border: iced::border::rounded(3.0),
                                ..container::Style::default()
                            }
                        }),
                        container(
                            text(format!("FAST {}", pacing.brisk_count))
                                .size(10)
                                .color(theme.accent())
                        )
                        .padding([2, 5])
                        .style(move |_| {
                            container::Style {
                                background: Some(iced::Background::Color(
                                    theme.accent().scale_alpha(0.15),
                                )),
                                border: iced::border::rounded(3.0),
                                ..container::Style::default()
                            }
                        }),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                    text(format!(
                        "Visual text: {}w • {} CJK",
                        pacing.total_visual_words, pacing.total_visual_cjk
                    ))
                    .size(10)
                    .color(theme.text_muted()),
                ]
                .spacing(2),
            ]
            .spacing(14)
            .align_y(Alignment::Center),
        ]
        .spacing(8),
    )
    .padding(12)
    .style(move |_| {
        container::Style {
            background: Some(iced::Background::Color(theme.bg_subtle())),
            border: iced::border::rounded(6.0),
            ..container::Style::default()
        }
    });

    // Per-slide pacing cards
    let mut slides_pacing_col = column![].spacing(6);
    for s in &pacing.slides {
        let (status_bg, status_fg) = match s.status {
            | slide_core::pacing::SlidePacingStatus::Optimal => {
                (theme.success().scale_alpha(0.15), theme.success())
            },
            | slide_core::pacing::SlidePacingStatus::Dense => {
                (theme.warning().scale_alpha(0.15), theme.warning())
            },
            | slide_core::pacing::SlidePacingStatus::Overloaded => {
                (theme.danger().scale_alpha(0.15), theme.danger())
            },
            | slide_core::pacing::SlidePacingStatus::Brisk => {
                (theme.accent().scale_alpha(0.15), theme.accent())
            },
        };

        let status_badge = container(
            text(format!("{} (~{}s)", s.status.label(), s.estimated_seconds))
                .size(10)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..iced::Font::DEFAULT
                })
                .color(status_fg),
        )
        .padding([2, 6])
        .style(move |_| {
            container::Style {
                background: Some(iced::Background::Color(status_bg)),
                border: iced::border::rounded(4.0),
                ..container::Style::default()
            }
        });

        let s_idx = s.slide_index;
        let jump_btn = button(text(format!("Slide {}", s_idx + 1)).size(11))
            .style(move |_t, _s| theme::subtle_button_style(theme, false))
            .padding([2, 8])
            .on_press(Message::SelectSlide(s_idx));

        let title_str = if s.title.trim().is_empty() {
            format!("(Slide {})", s_idx + 1)
        } else {
            s.title.clone()
        };

        let slide_header = row![
            jump_btn,
            text(title_str)
                .size(12)
                .font(iced::Font {
                    weight: iced::font::Weight::Medium,
                    ..iced::Font::DEFAULT
                })
                .color(theme.text_primary()),
            Space::new().width(Length::Fill),
            status_badge,
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let notes_info = if s.has_speaker_notes {
            format!("Notes: {} words, {} CJK", s.notes_words, s.notes_cjk_chars)
        } else {
            "No speaker notes (visual pacing applied)".to_string()
        };

        let visual_info = format!(
            "Visual: {} words, {} CJK | {} code lines | {} math | {} tables/charts | {} steps",
            s.visual_words,
            s.visual_cjk_chars,
            s.code_lines,
            s.math_formulas,
            s.charts_and_tables,
            s.step_count
        );

        let metrics_text = text(format!("{notes_info}  •  {visual_info}"))
            .size(10)
            .color(theme.text_secondary());

        let rec_text = text(&s.recommendation).size(10).color(theme.text_muted());

        let slide_card = container(column![slide_header, metrics_text, rec_text].spacing(3))
            .padding(8)
            .width(Length::Fill)
            .style(move |_| {
                container::Style {
                    background: Some(iced::Background::Color(theme.bg_subtle())),
                    border: iced::border::rounded(6.0),
                    ..container::Style::default()
                }
            });

        slides_pacing_col = slides_pacing_col.push(slide_card);
    }

    let mut issues_col = column![].spacing(6);
    if issues.is_empty() {
        let healthy_card = container(
            row![
                text("[OK]").size(12).color(theme.success()),
                column![
                    text("Deck diagnostics clear!")
                        .size(12)
                        .color(theme.success()),
                    text(
                        "No excessive text density, missing media, or macro syntax issues detected."
                    )
                    .size(10)
                    .color(theme.text_muted()),
                ]
                .spacing(2),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding(10)
        .width(Length::Fill)
        .style(move |_| {
            container::Style {
                background: Some(iced::Background::Color(theme.bg_subtle())),
                border: iced::border::rounded(6.0),
                ..container::Style::default()
            }
        });
        issues_col = issues_col.push(healthy_card);
    } else {
        for issue in issues {
            let (badge_color, badge_text) = match issue.severity {
                | slide_core::compiler::HealthSeverity::Error => (theme.danger(), "ERROR"),
                | slide_core::compiler::HealthSeverity::Warning => (theme.warning(), "WARNING"),
                | slide_core::compiler::HealthSeverity::Info => (theme.accent(), "INFO"),
            };

            let sev_badge = container(text(badge_text).size(9).color(iced::Color::WHITE))
                .padding([2, 5])
                .style(move |_| {
                    container::Style {
                        background: Some(iced::Background::Color(badge_color)),
                        border: iced::border::rounded(theme::RADIUS_FULL),
                        ..container::Style::default()
                    }
                });

            let mut header_row = row![sev_badge].spacing(6).align_y(Alignment::Center);

            if let Some(s_idx) = issue.slide_index {
                let jump_btn = button(text(format!("Slide {}", s_idx.saturating_add(1))).size(10))
                    .style(move |_t, _s| theme::subtle_button_style(theme, false))
                    .padding([1, 6])
                    .on_press(Message::SelectSlide(s_idx));
                header_row = header_row.push(jump_btn);
            }

            let desc = text(&issue.message).size(11).color(theme.text_primary());
            let rec = text(
                issue
                    .suggestion
                    .as_deref()
                    .unwrap_or("No specific recommendation."),
            )
            .size(10)
            .color(theme.text_muted());

            let issue_card = container(column![header_row, desc, rec].spacing(3))
                .padding(8)
                .width(Length::Fill)
                .style(move |_| {
                    container::Style {
                        background: Some(iced::Background::Color(theme.bg_subtle())),
                        border: iced::border::rounded(6.0),
                        ..container::Style::default()
                    }
                });

            issues_col = issues_col.push(issue_card);
        }
    }

    let close_btn = button(text("Close").size(12))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([6, 16])
        .on_press(Message::CloseModal);

    let dialog_content = column![
        title,
        Space::new().height(2),
        pacing_card,
        Space::new().height(4),
        text(format!(
            "Per-Slide Pacing Breakdown ({})",
            pacing.slides.len()
        ))
        .size(12)
        .font(iced::Font {
            weight: iced::font::Weight::Medium,
            ..iced::Font::DEFAULT
        })
        .color(theme.text_secondary()),
        scrollable(slides_pacing_col).height(Length::Fixed(190.0)),
        Space::new().height(4),
        text(format!("Diagnostic Issues ({})", issues.len()))
            .size(12)
            .font(iced::Font {
                weight: iced::font::Weight::Medium,
                ..iced::Font::DEFAULT
            })
            .color(theme.text_secondary()),
        scrollable(issues_col).height(Length::Fixed(110.0)),
        Space::new().height(6),
        row![Space::new().width(Length::Fill), close_btn],
    ]
    .spacing(6);

    let dialog_card = container(dialog_content)
        .width(Length::Fixed(720.0))
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
