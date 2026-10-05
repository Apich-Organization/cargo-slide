//! Source Mode: full document Typst code editor.

use crate::app::Message;
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
use iced::widget::column;
use iced::widget::container;
use iced::widget::image;
use iced::widget::row;
use iced::widget::stack;
use iced::widget::text;
use iced::widget::text_editor;

/// Render the full document source code editor
#[must_use]
pub fn view_source_mode<'a>(
    theme: AppTheme,
    content: &'a text_editor::Content,
    file_path: Option<&'a std::path::Path>,
    line_count: usize,
    hover_formula: Option<&'a str>,
    equation_images: &'a std::collections::HashMap<String, image::Handle>,
) -> Element<'a, Message> {
    let path_label = file_path.map_or("Untitled (Unsaved)", |p| p.to_str().unwrap_or("Document"));

    let header = row![
        text(format!("SOURCE CODE: {path_label}"))
            .size(11)
            .color(theme.text_muted()),
        Space::new().width(Length::Fill),
        text(format!("{line_count} lines"))
            .size(11)
            .color(theme.text_secondary())
    ]
    .align_y(Alignment::Center)
    .padding([8, 16]);

    let editor = text_editor(content)
        .id(iced::widget::Id::new("source_mode_editor"))
        .placeholder("// Write Typst markup here...")
        .wrapping(iced::widget::text::Wrapping::Word)
        .style(move |_theme, _status| theme::editor_style(theme))
        .highlight_with::<TypstHighlighter>(
            TypstHighlightSettings {
                is_dark: theme.is_dark(),
            },
            to_typst_format,
        )
        .on_action(Message::SourceEditorAction);

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
                    right: 20.0,
                    bottom: 20.0,
                    left: 0.0,
                })
                .into(),
        )
    } else {
        None
    };

    let editor_card = container(editor)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(14)
        .style(move |_| theme::slide_card_style(theme, false));

    let editor_area: Element<'a, Message> = if let Some(math_hud) = math_preview_card {
        stack![
            editor_card,
            container(math_hud)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::End)
                .align_y(Alignment::End),
        ]
        .into()
    } else {
        editor_card.into()
    };

    let content_col = column![
        header,
        container(editor_area)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(iced::Padding {
                top: 0.0,
                right: 16.0,
                bottom: 16.0,
                left: 16.0
            })
    ]
    .height(Length::Fill);

    container(content_col)
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
