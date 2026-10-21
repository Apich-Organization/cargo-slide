//! Binary launcher for Slide Editor (`slide-editor`).

use clap::Parser;
use slide_editor::multi_window::MultiWindowApp;
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

    iced::daemon(
        move || MultiWindowApp::boot(initial_file.clone(), dark_mode),
        MultiWindowApp::update,
        MultiWindowApp::view,
    )
    .subscription(MultiWindowApp::subscription)
    .title(MultiWindowApp::title)
    .theme(MultiWindowApp::theme)
    .run()
}
