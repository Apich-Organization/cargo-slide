//! Top navigation bar, mode switcher, and quick formatting tools.

use crate::app::EditorMode;
use crate::app::Message;
use crate::document::DocumentFormat;
use crate::ui::theme::AppTheme;
use crate::ui::theme::RADIUS_FULL;
use crate::ui::theme::RADIUS_XS;
use crate::ui::theme::{
    self,
};
use iced::Alignment;
use iced::Element;
use iced::Length;
use iced::widget::Space;
use iced::widget::button;
use iced::widget::container;
use iced::widget::row;
use iced::widget::scrollable;
use iced::widget::text;

/// Breakpoint widths for responsive toolbar layout
const COMPACT_BREAKPOINT: f32 = 1100.0;
const NARROW_BREAKPOINT: f32 = 850.0;
const TINY_BREAKPOINT: f32 = 650.0;

/// Render the primary top toolbar with responsive layout
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn view_toolbar<'a>(
    theme: AppTheme,
    title: &'a str,
    format: DocumentFormat,
    is_dirty: bool,
    mode: EditorMode,
    sidebar_visible: bool,
    can_undo: bool,
    can_redo: bool,
    window_width: f32,
) -> Element<'a, Message> {
    let is_compact = window_width < COMPACT_BREAKPOINT;
    let is_narrow = window_width < NARROW_BREAKPOINT;
    let is_tiny = window_width < TINY_BREAKPOINT;

    // 1. Left section: Outline toggle, document title, format badge
    let outline_label = if is_narrow {
        if sidebar_visible { "<" } else { ">" }
    } else if sidebar_visible {
        "< Outline"
    } else {
        "> Outline"
    };
    let outline_btn = button(text(outline_label).size(12))
        .style(move |_theme, _status| theme::subtle_button_style(theme, sidebar_visible))
        .padding([5, if is_narrow { 6 } else { 11 }])
        .on_press(Message::ToggleSidebar);

    let display_title = if is_narrow {
        let max_chars = if is_tiny { 8 } else { 16 };
        if title.len() > max_chars {
            format!("{}...", &title[..max_chars.saturating_sub(3)])
        } else {
            title.to_string()
        }
    } else {
        title.to_string()
    };
    let doc_title = text(display_title)
        .size(if is_narrow { 12 } else { 13 })
        .color(theme.text_primary());

    let badge_bg = match format {
        | DocumentFormat::Typst => theme.accent(),
        | DocumentFormat::SlidePackage => theme.success(),
    };

    let format_badge = container(
        text(format.extension().to_uppercase())
            .size(9)
            .color(iced::Color::WHITE),
    )
    .padding([2, 7])
    .style(move |_| {
        container::Style {
            background: Some(iced::Background::Color(badge_bg)),
            border: iced::border::rounded(RADIUS_FULL),
            ..container::Style::default()
        }
    });

    let mut left_bar = row![outline_btn, doc_title]
        .spacing(if is_narrow { 4 } else { 10 })
        .align_y(Alignment::Center);

    if is_dirty {
        let dirty_dot = container(
            Space::new()
                .width(Length::Fixed(7.0))
                .height(Length::Fixed(7.0)),
        )
        .style(move |_| {
            container::Style {
                background: Some(iced::Background::Color(theme.warning())),
                border: iced::border::rounded(RADIUS_FULL),
                shadow: iced::Shadow {
                    color: theme.warning().scale_alpha(0.4),
                    offset: iced::Vector::ZERO,
                    blur_radius: 4.0,
                },
                ..container::Style::default()
            }
        });
        left_bar = left_bar.push(dirty_dot);
    }

    if !is_tiny {
        left_bar = left_bar.push(format_badge);
    }

    // 2. Center section: Segmented Mode Switcher
    let (live_label, focus_label, source_label) = if is_narrow {
        ("Live", "Focus", "Source")
    } else {
        ("Live Preview", "Focus Mode", "Source Code")
    };

    let btn_pad = if is_narrow { [5, 8] } else { [5, 14] };

    let live_btn = button(text(live_label).size(12))
        .style(move |_theme, _status| {
            theme::segmented_button_style(theme, mode == EditorMode::LivePreview)
        })
        .padding(btn_pad)
        .on_press(Message::SwitchMode(EditorMode::LivePreview));

    let focus_btn = button(text(focus_label).size(12))
        .style(move |_theme, _status| {
            theme::segmented_button_style(theme, mode == EditorMode::FocusMode)
        })
        .padding(btn_pad)
        .on_press(Message::SwitchMode(EditorMode::FocusMode));

    let source_btn = button(text(source_label).size(12))
        .style(move |_theme, _status| {
            theme::segmented_button_style(theme, mode == EditorMode::SourceMode)
        })
        .padding(btn_pad)
        .on_press(Message::SwitchMode(EditorMode::SourceMode));

    let segmented_bar = container(
        row![live_btn, focus_btn, source_btn]
            .spacing(2)
            .align_y(Alignment::Center),
    )
    .padding([2, 2])
    .style(move |_| theme::segmented_pill_container_style(theme));

    // 3. Right section: Quick actions with responsive labels
    let tb_size = if is_compact { 11 } else { 12 };
    let tb_pad: [u16; 2] = if is_compact {
        [4, 7]
    } else {
        [5, 10]
    };
    let tb_spacing = if is_compact { 3 } else { 6 };

    let new_btn = button(text("New").size(tb_size))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding(tb_pad)
        .on_press(Message::NewDocument);

    let open_btn = button(text("Open").size(tb_size))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding(tb_pad)
        .on_press(Message::OpenDocumentDialog);

    let save_btn = button(text("Save").size(tb_size))
        .style(move |_theme, _status| {
            if is_dirty {
                theme::primary_button_style(theme)
            } else {
                theme::subtle_button_style(theme, false)
            }
        })
        .padding(if is_compact {
            [4, 8]
        } else {
            [5, 12]
        })
        .on_press(Message::SaveDocument);

    let export_btn = button(
        text(if is_compact {
            "Exp"
        } else {
            "Export"
        })
        .size(tb_size),
    )
    .style(move |_theme, _status| theme::subtle_button_style(theme, false))
    .padding(tb_pad)
    .on_press(Message::OpenExportDialog);

    let present_btn = if is_narrow {
        button(text("Play").size(tb_size).color(iced::Color::WHITE))
            .style(move |_theme, _status| theme::primary_button_style(theme))
            .padding([4, 10])
            .on_press(Message::PlayPresentation)
    } else {
        button(
            row![
                container(text("PLAY").size(9).color(iced::Color::WHITE))
                    .padding([1, 5])
                    .style(move |_| {
                        container::Style {
                            background: Some(iced::Background::Color(theme.accent_hover())),
                            border: iced::border::rounded(RADIUS_XS),
                            ..container::Style::default()
                        }
                    }),
                text("Present").size(12).color(iced::Color::WHITE),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .style(move |_theme, _status| theme::primary_button_style(theme))
        .padding([5, 13])
        .on_press(Message::PlayPresentation)
    };

    let theme_btn = button(
        text(match theme {
            | AppTheme::Light => {
                if is_compact {
                    "D"
                } else {
                    "Dark"
                }
            },
            | AppTheme::Dark => {
                if is_compact {
                    "L"
                } else {
                    "Light"
                }
            },
        })
        .size(tb_size),
    )
    .style(move |_theme, _status| theme::subtle_button_style(theme, false))
    .padding(tb_pad)
    .on_press(Message::ToggleTheme);

    let undo_btn = button(
        text(if is_compact {
            "↶"
        } else {
            "↶ Undo"
        })
        .size(tb_size)
        .color(if can_undo {
            theme.text_primary()
        } else {
            theme.text_muted()
        }),
    )
    .style(move |_theme, _status| theme::subtle_button_style(theme, false))
    .padding(if is_compact { [4, 6] } else { [5, 9] })
    .on_press_maybe(if can_undo {
        Some(Message::Undo)
    } else {
        None
    });

    let redo_btn = button(
        text(if is_compact {
            "↷"
        } else {
            "↷ Redo"
        })
        .size(tb_size)
        .color(if can_redo {
            theme.text_primary()
        } else {
            theme.text_muted()
        }),
    )
    .style(move |_theme, _status| theme::subtle_button_style(theme, false))
    .padding(if is_compact { [4, 6] } else { [5, 9] })
    .on_press_maybe(if can_redo {
        Some(Message::Redo)
    } else {
        None
    });

    // Build right bar — progressively hide less-important items at narrow widths
    let mut right_bar = row![].spacing(tb_spacing).align_y(Alignment::Center);

    // Always-visible core actions
    right_bar = right_bar.push(save_btn);
    right_bar = right_bar.push(undo_btn);
    right_bar = right_bar.push(redo_btn);

    if !is_tiny {
        right_bar = right_bar.push(new_btn);
        right_bar = right_bar.push(open_btn);
    }

    if !is_narrow {
        let templates_btn = button(text("Templates").size(tb_size))
            .style(move |_theme, _status| theme::subtle_button_style(theme, false))
            .padding(tb_pad)
            .on_press(Message::OpenTemplateLibraryModal);
        right_bar = right_bar.push(templates_btn);

        let find_btn = button(text("Find").size(tb_size))
            .style(move |_theme, _status| theme::subtle_button_style(theme, false))
            .padding(tb_pad)
            .on_press(Message::OpenSearchModal);
        right_bar = right_bar.push(find_btn);

        let font_btn = button(text("Font").size(tb_size))
            .style(move |_theme, _status| theme::subtle_button_style(theme, false))
            .padding(tb_pad)
            .on_press(Message::OpenFontModal);
        right_bar = right_bar.push(font_btn);

        let header_footer_btn = button(text("Header/Footer").size(tb_size))
            .style(move |_theme, _status| theme::subtle_button_style(theme, false))
            .padding(tb_pad)
            .on_press(Message::OpenHeaderFooterModal);
        right_bar = right_bar.push(header_footer_btn);
    }

    right_bar = right_bar.push(export_btn);
    right_bar = right_bar.push(present_btn);

    if !is_tiny {
        right_bar = right_bar.push(theme_btn);
    }

    let main_row = row![
        left_bar,
        Space::new().width(Length::Fill),
        segmented_bar,
        Space::new().width(Length::Fill),
        right_bar
    ]
    .align_y(Alignment::Center)
    .padding([6, if is_narrow { 8 } else { 16 }]);

    container(main_row)
        .width(Length::Fill)
        .style(move |_| theme::header_container_style(theme))
        .into()
}

/// Render the quick formatting toolbar (floating capsule island with micro-shadows)
/// Wrapped in a horizontal scrollable so buttons never clip at narrow window widths.
#[must_use]
pub fn view_formatting_bar<'a>(
    theme: AppTheme,
    active_slide: usize,
    window_width: f32,
) -> Element<'a, Message> {
    let is_compact = window_width < COMPACT_BREAKPOINT;
    let is_narrow = window_width < NARROW_BREAKPOINT;
    let btn_size = if is_compact { 10 } else { 11 };
    let btn_pad: [u16; 2] = if is_compact { [2, 6] } else { [3, 8] };
    let group_spacing = if is_compact { 3 } else { 4 };

    let make_btn = |label: &'static str, msg: Message| {
        button(text(label).size(btn_size))
            .style(move |_theme, _status| theme::chip_button_style(theme, false))
            .padding(btn_pad)
            .on_press(msg)
    };

    let divider = || {
        container(
            Space::new()
                .width(Length::Fixed(1.0))
                .height(Length::Fixed(14.0)),
        )
        .style(move |_| {
            container::Style {
                background: Some(iced::Background::Color(theme.border_color())),
                ..container::Style::default()
            }
        })
    };

    let slide_group = row![
        button(text("+ Slide").size(btn_size))
            .style(move |_theme, _status| theme::primary_button_style(theme))
            .padding([
                if is_compact { 2 } else { 3 },
                if is_compact { 7 } else { 10 }
            ])
            .on_press(Message::InsertSlide),
        button(
            text(if is_compact {
                "Trans"
            } else {
                "Transition"
            })
            .size(btn_size)
        )
        .style(move |_theme, _status| theme::chip_button_style(theme, false))
        .padding(btn_pad)
        .on_press(Message::OpenTransitionModal(active_slide)),
    ]
    .spacing(group_spacing)
    .align_y(Alignment::Center);

    let mut heading_group = row![
        make_btn("H1", Message::InsertSnippet("= Heading 1\n")),
        make_btn("H2", Message::InsertSnippet("== Heading 2\n")),
    ]
    .spacing(group_spacing)
    .align_y(Alignment::Center);

    if !is_narrow {
        heading_group =
            heading_group.push(make_btn("H3", Message::InsertSnippet("=== Heading 3\n")));
        heading_group =
            heading_group.push(make_btn("H4", Message::InsertSnippet("==== Heading 4\n")));
        heading_group = heading_group.push(make_btn("Title Slide", Message::InsertSnippet("#title-slide(\n  title: \"Presentation Title\",\n  subtitle: \"Subtitle or Tagline\",\n  author: \"Presenter Name\",\n  date: \"2026\",\n)\n")));
    }

    let text_group = row![
        make_btn("Bold", Message::InsertSnippet("*bold text*")),
        make_btn("Italic", Message::InsertSnippet("_italic text_")),
        make_btn("Math", Message::InsertSnippet("$ E = m c^2 $")),
        make_btn(
            "Code",
            Message::InsertSnippet("```rust\nfn main() {\n    // Code\n}\n```\n")
        ),
    ]
    .spacing(group_spacing)
    .align_y(Alignment::Center);

    let mut block_group = row![
        make_btn("List", Message::InsertSnippet("- List item\n")),
        make_btn("Table", Message::InsertSnippet("#table(\n  columns: (1fr, 1fr),\n  [Header 1], [Header 2],\n  [Item 1], [Item 2],\n)\n")),
    ]
    .spacing(group_spacing)
    .align_y(Alignment::Center);

    if !is_narrow {
        block_group = block_group.push(make_btn("Box", Message::InsertSnippet("#box(stroke: 1pt + rgb(\"3b82f6\"), inset: 8pt, radius: 4pt)[\n  Box container content here.\n]\n")));
        block_group = block_group.push(make_btn(
            "Link",
            Message::InsertSnippet("#link(\"https://cargo-slide.dev\")[Cargo Slide]\n"),
        ));
        block_group = block_group.push(make_btn(
            "Chart",
            Message::InsertSnippet(
                "#chart-bar(\n  title: \"Metrics\",\n  (\"A\", 40),\n  (\"B\", 80),\n)\n",
            ),
        ));
    }

    let bar = row![
        slide_group,
        divider(),
        heading_group,
        divider(),
        text_group,
        divider(),
        block_group,
    ]
    .spacing(if is_compact { 5 } else { 8 })
    .align_y(Alignment::Center);

    let floating_capsule = container(bar)
        .padding([
            if is_compact { 3 } else { 4 },
            if is_compact { 8 } else { 12 },
        ])
        .style(move |_| theme::floating_capsule_style(theme));

    // Wrap in a horizontal scrollable to prevent clipping at narrow widths
    let scrollable_capsule = scrollable(
        container(floating_capsule)
            .width(Length::Shrink)
            .align_x(Alignment::Center),
    )
    .direction(scrollable::Direction::Horizontal(
        scrollable::Scrollbar::new().width(0).scroller_width(0),
    ))
    .width(Length::Fill);

    container(scrollable_capsule)
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .padding([4, if is_narrow { 8 } else { 16 }])
        .into()
}
