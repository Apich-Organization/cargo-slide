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
use iced::Element;
use iced::Length;
use iced::widget::Space;
use iced::widget::button;
use iced::widget::column;
use iced::widget::container;
use iced::widget::row;
use iced::widget::scrollable;
use iced::widget::text;
use iced::widget::text_input;

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
        Space::new().height(14),
        actions
    ]
    .spacing(8);

    let dialog_card = container(dialog_content)
        .width(Length::Fixed(520.0))
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

    let close_btn = button(text("Close").size(13))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([8, 18])
        .on_press(Message::CloseModal);

    let actions = row![Space::new().width(Length::Fill), close_btn];

    let dialog_content = column![
        title,
        loc_text,
        msg_text,
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

/// Render right-click context menu modal for a slide
#[must_use]
pub fn view_slide_context_menu_modal<'a>(
    theme: AppTheme,
    slide_idx: usize,
    total_slides: usize,
) -> Element<'a, Message> {
    let slide_num = slide_idx + 1;
    let title = text(format!("Slide {slide_num} Options"))
        .size(15)
        .color(theme.text_primary());

    let menu_btn = |label: &'static str, msg: Message, danger: bool| {
        button(text(label).size(13).color(if danger {
            theme.danger()
        } else {
            theme.text_primary()
        }))
        .width(Length::Fill)
        .padding([8, 14])
        .style(move |_theme, status| {
            let is_hovered = matches!(status, iced::widget::button::Status::Hovered);
            iced::widget::button::Style {
                background: Some(iced::Background::Color(if is_hovered {
                    theme.bg_subtle()
                } else {
                    iced::Color::TRANSPARENT
                })),
                text_color: if danger {
                    theme.danger()
                } else {
                    theme.text_primary()
                },
                border: iced::border::rounded(theme::RADIUS_SM),
                ..Default::default()
            }
        })
        .on_press(msg)
    };

    let mut menu_items = column![
        title,
        Space::new().height(4),
        menu_btn(
            "Insert Slide Above",
            Message::InsertSlideAt(slide_idx),
            false
        ),
        menu_btn(
            "Insert Slide Below",
            Message::InsertSlideAt(slide_idx + 1),
            false
        ),
        menu_btn("Duplicate Slide", Message::DuplicateSlide(slide_idx), false),
    ]
    .spacing(4);

    if slide_idx > 0 {
        menu_items = menu_items.push(menu_btn(
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
            "Move Slide Down",
            Message::MoveSlide {
                from_idx: slide_idx,
                to_idx: slide_idx + 1,
            },
            false,
        ));
    }

    menu_items = menu_items.push(menu_btn(
        "Configure Transition...",
        Message::OpenTransitionModal(slide_idx),
        false,
    ));

    if total_slides > 1 {
        menu_items = menu_items.push(menu_btn(
            "Delete Slide",
            Message::DeleteSlide(slide_idx),
            true,
        ));
    }

    let cancel_btn = button(text("Close").size(12))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([6, 14])
        .on_press(Message::CloseModal);

    menu_items = menu_items.push(Space::new().height(4));
    menu_items = menu_items.push(row![Space::new().width(Length::Fill), cancel_btn]);

    let card = container(menu_items)
        .width(Length::Fixed(280.0))
        .padding(16)
        .style(move |_| theme::modal_dialog_style(theme));

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_| theme::modal_backdrop_style(theme))
        .into()
}

/// Render right-click context menu modal for a WYSIWYG block
#[must_use]
pub fn view_block_context_menu_modal<'a>(
    theme: AppTheme,
    slide_idx: usize,
    block_idx: usize,
    range: std::ops::Range<usize>,
    block_label: &'a str,
    block_id: &'a str,
) -> Element<'a, Message> {
    let title = text(format!("Element: {block_label}"))
        .size(15)
        .color(theme.text_primary());

    let menu_btn = |label: &'static str, msg: Message, danger: bool| {
        button(text(label).size(13).color(if danger {
            theme.danger()
        } else {
            theme.text_primary()
        }))
        .width(Length::Fill)
        .padding([8, 14])
        .style(move |_theme, status| {
            let is_hovered = matches!(status, iced::widget::button::Status::Hovered);
            iced::widget::button::Style {
                background: Some(iced::Background::Color(if is_hovered {
                    theme.bg_subtle()
                } else {
                    iced::Color::TRANSPARENT
                })),
                text_color: if danger {
                    theme.danger()
                } else {
                    theme.text_primary()
                },
                border: iced::border::rounded(theme::RADIUS_SM),
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
        title,
        Space::new().height(4),
        menu_btn(
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
            "Duplicate Element",
            Message::DuplicateBlock(range_c2),
            false
        ),
        menu_btn(
            "Edit Raw Typst Code",
            Message::ToggleBlockRawCode(block_id.to_string()),
            false
        ),
        menu_btn(
            "Configure Step Animation...",
            Message::OpenElementTransitionModal { slide_idx, block_idx },
            false
        ),
        menu_btn(
            "Delete Element",
            Message::DeleteBlockAtRange(range_c3),
            true
        ),
        Space::new().height(4),
        row![
            Space::new().width(Length::Fill),
            button(text("Close").size(12))
                .style(move |_theme, _status| theme::subtle_button_style(theme, false))
                .padding([6, 14])
                .on_press(Message::CloseModal)
        ]
    ]
    .spacing(4);

    let card = container(menu_items)
        .width(Length::Fixed(290.0))
        .padding(16)
        .style(move |_| theme::modal_dialog_style(theme));

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_| theme::modal_backdrop_style(theme))
        .into()
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

    let close_btn = button(text("✕").size(11))
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
