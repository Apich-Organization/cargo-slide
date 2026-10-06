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
use iced::widget::text;

/// Render the primary top toolbar
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
) -> Element<'a, Message> {
    let outline_label = if sidebar_visible {
        "< Outline"
    } else {
        "> Outline"
    };
    let outline_btn = button(text(outline_label).size(12))
        .style(move |_theme, _status| theme::subtle_button_style(theme, sidebar_visible))
        .padding([5, 11])
        .on_press(Message::ToggleSidebar);

    let doc_title = text(title).size(13).color(theme.text_primary());

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
        .spacing(10)
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

    let left_bar = left_bar.push(format_badge);

    // 2. Center section: Segmented Mode Switcher
    let live_btn = button(text("Live Preview").size(12))
        .style(move |_theme, _status| {
            theme::segmented_button_style(theme, mode == EditorMode::LivePreview)
        })
        .padding([5, 14])
        .on_press(Message::SwitchMode(EditorMode::LivePreview));

    let focus_btn = button(text("Focus Mode").size(12))
        .style(move |_theme, _status| {
            theme::segmented_button_style(theme, mode == EditorMode::FocusMode)
        })
        .padding([5, 14])
        .on_press(Message::SwitchMode(EditorMode::FocusMode));

    let source_btn = button(text("Source Code").size(12))
        .style(move |_theme, _status| {
            theme::segmented_button_style(theme, mode == EditorMode::SourceMode)
        })
        .padding([5, 14])
        .on_press(Message::SwitchMode(EditorMode::SourceMode));

    let segmented_bar = container(
        row![live_btn, focus_btn, source_btn]
            .spacing(2)
            .align_y(Alignment::Center),
    )
    .padding([2, 2])
    .style(move |_| theme::segmented_pill_container_style(theme));

    // 3. Right section: Quick actions (New, Open, Save, Export, Present, Theme)
    let new_btn = button(text("New").size(12))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([5, 10])
        .on_press(Message::NewDocument);

    let open_btn = button(text("Open").size(12))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([5, 10])
        .on_press(Message::OpenDocumentDialog);

    let save_btn = button(text("Save").size(12))
        .style(move |_theme, _status| {
            if is_dirty {
                theme::primary_button_style(theme)
            } else {
                theme::subtle_button_style(theme, false)
            }
        })
        .padding([5, 12])
        .on_press(Message::SaveDocument);

    let export_btn = button(text("Export").size(12))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([5, 10])
        .on_press(Message::OpenExportDialog);

    let present_btn = button(
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
    .on_press(Message::PlayPresentation);

    let theme_btn = button(
        text(match theme {
            | AppTheme::Light => "Dark",
            | AppTheme::Dark => "Light",
        })
        .size(12),
    )
    .style(move |_theme, _status| theme::subtle_button_style(theme, false))
    .padding([5, 11])
    .on_press(Message::ToggleTheme);

    let font_btn = button(text("Font").size(12))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([5, 10])
        .on_press(Message::OpenFontModal);

    let find_btn = button(text("Find").size(12))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([5, 10])
        .on_press(Message::OpenSearchModal);

    let header_footer_btn = button(text("Header/Footer").size(12))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([5, 10])
        .on_press(Message::OpenHeaderFooterModal);

    let undo_btn = button(text("↶ Undo").size(12).color(if can_undo {
        theme.text_primary()
    } else {
        theme.text_muted()
    }))
    .style(move |_theme, _status| theme::subtle_button_style(theme, false))
    .padding([5, 9])
    .on_press_maybe(if can_undo {
        Some(Message::Undo)
    } else {
        None
    });

    let redo_btn = button(text("↷ Redo").size(12).color(if can_redo {
        theme.text_primary()
    } else {
        theme.text_muted()
    }))
    .style(move |_theme, _status| theme::subtle_button_style(theme, false))
    .padding([5, 9])
    .on_press_maybe(if can_redo {
        Some(Message::Redo)
    } else {
        None
    });

    let templates_btn = button(text("Templates").size(12))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([5, 10])
        .on_press(Message::OpenTemplateLibraryModal);

    let right_bar = row![
        new_btn,
        templates_btn,
        open_btn,
        save_btn,
        undo_btn,
        redo_btn,
        find_btn,
        font_btn,
        header_footer_btn,
        export_btn,
        present_btn,
        theme_btn
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let main_row = row![
        left_bar,
        Space::new().width(Length::Fill),
        segmented_bar,
        Space::new().width(Length::Fill),
        right_bar
    ]
    .align_y(Alignment::Center)
    .padding([6, 16]);

    container(main_row)
        .width(Length::Fill)
        .style(move |_| theme::header_container_style(theme))
        .into()
}

/// Render the quick formatting toolbar (floating capsule island with micro-shadows)
#[must_use]
pub fn view_formatting_bar<'a>(
    theme: AppTheme,
    active_slide: usize,
) -> Element<'a, Message> {
    let make_btn = |label: &'static str, msg: Message| {
        button(text(label).size(11))
            .style(move |_theme, _status| theme::chip_button_style(theme, false))
            .padding([3, 8])
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
        button(text("+ Slide").size(11))
            .style(move |_theme, _status| theme::primary_button_style(theme))
            .padding([3, 10])
            .on_press(Message::InsertSlide),
        button(text("Transition").size(11))
            .style(move |_theme, _status| theme::chip_button_style(theme, false))
            .padding([3, 8])
            .on_press(Message::OpenTransitionModal(active_slide)),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let heading_group = row![
        make_btn("H1", Message::InsertSnippet("= Heading 1\n")),
        make_btn("H2", Message::InsertSnippet("== Heading 2\n")),
        make_btn("H3", Message::InsertSnippet("=== Heading 3\n")),
        make_btn("H4", Message::InsertSnippet("==== Heading 4\n")),
        make_btn("Title Slide", Message::InsertSnippet("#title-slide(\n  title: \"Presentation Title\",\n  subtitle: \"Subtitle or Tagline\",\n  author: \"Presenter Name\",\n  date: \"2026\",\n)\n")),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let text_group = row![
        make_btn("Bold", Message::InsertSnippet("*bold text*")),
        make_btn("Italic", Message::InsertSnippet("_italic text_")),
        make_btn("Math", Message::InsertSnippet("$ E = m c^2 $")),
        make_btn(
            "Code",
            Message::InsertSnippet("```rust\nfn main() {\n    // Code\n}\n```\n")
        ),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let block_group = row![
        make_btn("List", Message::InsertSnippet("- List item\n")),
        make_btn("Table", Message::InsertSnippet("#table(\n  columns: (1fr, 1fr),\n  [Header 1], [Header 2],\n  [Item 1], [Item 2],\n)\n")),
        make_btn("Box", Message::InsertSnippet("#box(stroke: 1pt + rgb(\"3b82f6\"), inset: 8pt, radius: 4pt)[\n  Box container content here.\n]\n")),
        make_btn("Link", Message::InsertSnippet("#link(\"https://cargo-slide.dev\")[Cargo Slide]\n")),
        make_btn("Chart", Message::InsertSnippet("#chart-bar(\n  title: \"Metrics\",\n  (\"A\", 40),\n  (\"B\", 80),\n)\n")),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let bar = row![
        slide_group,
        divider(),
        heading_group,
        divider(),
        text_group,
        divider(),
        block_group,
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let floating_capsule = container(bar)
        .padding([4, 12])
        .style(move |_| theme::floating_capsule_style(theme));

    container(floating_capsule)
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .padding([4, 16])
        .into()
}
