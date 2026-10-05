//! Binary launcher for Slide Editor (`slide-editor`).

use clap::Parser;
use slide_editor::app::SlideEditorApp;
use slide_editor::ui::theme::AppTheme;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "slide-editor",
    bin_name = "slide-editor",
    version,
    about = "Typora-style WYSIWYG live-preview desktop editor for Typst presentations and documents"
)]
struct Args {
    /// Path to .typ or .slide presentation file to edit
    file: Option<PathBuf>,

    /// Start directly in dark mode
    #[arg(long)]
    dark: bool,
}

fn main() -> iced::Result {
    let args = Args::parse();
    let initial_file = args.file;
    let dark_mode = args.dark;

    iced::application(
        move || SlideEditorApp::new(initial_file.clone(), dark_mode),
        SlideEditorApp::update,
        SlideEditorApp::view,
    )
    .subscription(SlideEditorApp::subscription)
    .title(|app: &SlideEditorApp| {
        let dirty = if app.doc.is_dirty { "* " } else { "" };
        format!(
            "{}{} - Slide Editor ({})",
            dirty,
            app.doc.title,
            app.doc.format.label()
        )
    })
    .theme(|app: &SlideEditorApp| {
        match app.theme {
            | AppTheme::Light => iced::Theme::Light,
            | AppTheme::Dark => iced::Theme::Dark,
        }
    })
    .window_size(iced::Size::new(1280.0, 820.0))
    .centered()
    .resizable(true)
    .run()
}
