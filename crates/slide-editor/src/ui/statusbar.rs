//! Bottom status bar with slide counter, compiler diagnostics, and document stats.

use crate::app::CompilationStatus;
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
use iced::Background;
use iced::Border;
use iced::Color;
use iced::Element;
use iced::Length;
use iced::Shadow;
use iced::Vector;
use iced::border;
use iced::widget::Space;
use iced::widget::button;
use iced::widget::container;
use iced::widget::row;
use iced::widget::text;

/// Render the bottom status bar with responsive layout
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn view_statusbar<'a>(
    theme: AppTheme,
    current_slide: usize,
    total_slides: usize,
    format: DocumentFormat,
    mode: EditorMode,
    status: &'a CompilationStatus,
    char_count: usize,
    word_count: usize,
    zoom_percent: u32,
    speaking_wpm: u32,
    window_width: f32,
    is_dirty: bool,
) -> Element<'a, Message> {
    let is_narrow = window_width < 850.0;
    let is_tiny = window_width < 650.0;
    // Left: Slide Counter and Word/Char count
    let slide_info = text(format!(
        "Slide {} of {}",
        current_slide.saturating_add(1),
        total_slides.max(1)
    ))
    .size(12)
    .color(theme.text_secondary());

    let stats_info = text(format!("{word_count} words, {char_count} chars"))
        .size(11)
        .color(theme.text_muted());

    let safe_wpm = (speaking_wpm as f32).max(50.0);
    let talk_mins = ((word_count as f32) / safe_wpm).max(1.0).ceil() as usize;
    let pacing_btn = button(
        text(format!("~{talk_mins} min ({speaking_wpm} WPM)"))
            .size(11)
            .color(theme.accent()),
    )
    .padding([2, 6])
    .style(move |_theme, _status| theme::subtle_button_style(theme, false))
    .on_press(Message::CycleSpeakingPace);

    let health_btn = button(text("Health").size(11).color(theme.text_secondary()))
        .padding([2, 6])
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .on_press(Message::OpenPresentationHealthModal);

    let cheatsheet_btn = button(
        text("Cheatsheet (F1)")
            .size(11)
            .color(theme.text_secondary()),
    )
    .padding([2, 6])
    .style(move |_theme, _status| theme::subtle_button_style(theme, false))
    .on_press(Message::OpenCheatsheetModal);

    let divider_widget = || {
        container(
            Space::new()
                .width(Length::Fixed(1.0))
                .height(Length::Fixed(10.0)),
        )
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.border_color())),
                ..container::Style::default()
            }
        })
    };

    let mut left_info = row![slide_info].spacing(8).align_y(Alignment::Center);

    if !is_tiny {
        left_info = left_info.push(divider_widget());
        left_info = left_info.push(stats_info);
    }
    if !is_narrow {
        left_info = left_info.push(divider_widget());
        left_info = left_info.push(pacing_btn);
    }
    left_info = left_info.push(health_btn);
    left_info = left_info.push(cheatsheet_btn);

    // Center: Compiler Diagnostic status indicator
    let compiler_indicator: Element<'a, Message> = match status {
        | CompilationStatus::Ready => {
            let dot = container(Space::new())
                .width(Length::Fixed(7.0))
                .height(Length::Fixed(7.0))
                .style(move |_| {
                    container::Style {
                        background: Some(Background::Color(theme.success())),
                        border: Border {
                            radius: border::Radius::from(RADIUS_FULL),
                            ..Border::default()
                        },
                        shadow: Shadow {
                            color: theme.success().scale_alpha(0.55),
                            offset: Vector::ZERO,
                            blur_radius: 5.0,
                        },
                        ..container::Style::default()
                    }
                });
            row![
                dot,
                text("Typst Ready").size(11).color(theme.text_secondary())
            ]
            .spacing(7)
            .align_y(Alignment::Center)
            .into()
        },
        | CompilationStatus::Compiling => {
            let dot = container(Space::new())
                .width(Length::Fixed(7.0))
                .height(Length::Fixed(7.0))
                .style(move |_| {
                    container::Style {
                        background: Some(Background::Color(theme.accent())),
                        border: Border {
                            radius: border::Radius::from(RADIUS_FULL),
                            ..Border::default()
                        },
                        shadow: Shadow {
                            color: theme.accent().scale_alpha(0.60),
                            offset: Vector::ZERO,
                            blur_radius: 6.0,
                        },
                        ..container::Style::default()
                    }
                });
            row![dot, text("Compiling...").size(11).color(theme.accent())]
                .spacing(7)
                .align_y(Alignment::Center)
                .into()
        },
        | CompilationStatus::Error(diag) => {
            let err_summary = diag.line.map_or_else(
                || "Typst Error (Click to view)".to_string(),
                |l| format!("Typst Error at Line {l}"),
            );

            button(
                row![
                    text("!").size(11).color(theme.danger()),
                    text(err_summary).size(11).color(theme.danger())
                ]
                .spacing(5)
                .align_y(Alignment::Center),
            )
            .padding([2, 8])
            .style(move |_theme, _status| theme::danger_button_style(theme))
            .on_press(Message::OpenErrorDetailsDialog)
            .into()
        },
    };

    // Right: Zoom controls and Format & Mode info
    let mode_str = match mode {
        | EditorMode::LivePreview => {
            if is_narrow {
                "Live"
            } else {
                "Live Preview"
            }
        },
        | EditorMode::FocusMode => {
            if is_narrow {
                "Focus"
            } else {
                "Focus Mode"
            }
        },
        | EditorMode::SourceMode => {
            if is_narrow {
                "Src"
            } else {
                "Source Code"
            }
        },
    };

    let mode_badge = container(text(mode_str).size(10).color(theme.text_secondary()))
        .padding([2, 6])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.bg_subtle())),
                border: border::rounded(RADIUS_XS),
                ..container::Style::default()
            }
        });

    let format_badge = container(text(format.label()).size(10).color(theme.text_muted()))
        .padding([2, 6])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.bg_subtle())),
                border: border::rounded(RADIUS_XS),
                ..container::Style::default()
            }
        });

    let zoom_out = button(text("-").size(11))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([2, 7])
        .on_press(Message::ZoomOut);

    let zoom_val = button(
        text(format!("{zoom_percent}%"))
            .size(11)
            .color(theme.text_secondary()),
    )
    .padding([2, 6])
    .style(move |_theme, _status| theme::subtle_button_style(theme, false))
    .on_press(Message::ResetZoom);

    let zoom_in = button(text("+").size(11))
        .style(move |_theme, _status| theme::subtle_button_style(theme, false))
        .padding([2, 7])
        .on_press(Message::ZoomIn);

    let zoom_controls = row![zoom_out, zoom_val, zoom_in]
        .spacing(4)
        .align_y(Alignment::Center);

    let right_divider = || {
        container(
            Space::new()
                .width(Length::Fixed(1.0))
                .height(Length::Fixed(10.0)),
        )
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.border_color())),
                ..container::Style::default()
            }
        })
    };

    let dirty_pill = if is_dirty {
        let amber = Color::from_rgb8(245, 158, 11);
        button(
            row![
                text("●").size(9).color(amber),
                text("Unsaved").size(10).color(amber),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .padding([2, 6])
        .style(move |_theme, _status| {
            button::Style {
                background: Some(Background::Color(Color::from_rgba8(245, 158, 11, 0.12))),
                border: Border {
                    color: Color::from_rgba8(245, 158, 11, 0.35),
                    width: 1.0,
                    radius: border::Radius::from(RADIUS_XS),
                },
                text_color: amber,
                ..button::Style::default()
            }
        })
        .on_press(Message::SaveDocument)
    } else {
        let green = theme.success();
        button(
            row![
                text("✓").size(10).color(green),
                text("Saved").size(10).color(green),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .padding([2, 6])
        .style(move |_theme, _status| {
            button::Style {
                background: Some(Background::Color(Color::from_rgba8(34, 197, 94, 0.10))),
                border: Border {
                    color: Color::from_rgba8(34, 197, 94, 0.25),
                    width: 1.0,
                    radius: border::Radius::from(RADIUS_XS),
                },
                text_color: green,
                ..button::Style::default()
            }
        })
    };

    let mut right_info = row![].spacing(8).align_y(Alignment::Center);

    right_info = right_info.push(dirty_pill);
    if !is_tiny {
        right_info = right_info.push(mode_badge);
    }
    if !is_narrow {
        right_info = right_info.push(right_divider());
        right_info = right_info.push(format_badge);
    }
    right_info = right_info.push(right_divider());
    right_info = right_info.push(zoom_controls);

    let main_row = row![
        left_info,
        Space::new().width(Length::Fill),
        compiler_indicator,
        Space::new().width(Length::Fill),
        right_info
    ]
    .align_y(Alignment::Center)
    .padding([5, if is_narrow { 8 } else { 16 }]);

    container(main_row)
        .width(Length::Fill)
        .style(move |_| theme::statusbar_container_style(theme))
        .into()
}
