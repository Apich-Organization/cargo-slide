//! # `cargo-slide`
//!
//! CLI tool for initializing, running, developing, building, and exporting
//! presentations powered by Typst and native Rust player engine.

// -------------------------------------------------------------------------
// LEVEL 1: CRITICAL ERRORS (Deny)
// -------------------------------------------------------------------------
#![deny(
    // Rust Compiler Errors
    unreachable_code,
    improper_ctypes_definitions,
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms,
    clippy::perf,
    clippy::correctness,
    clippy::suspicious,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::missing_safety_doc,
    clippy::same_item_push,
    clippy::implicit_clone,
    clippy::all,
    clippy::pedantic,
    missing_docs,
    clippy::nursery,
    clippy::single_call_fn,
)]
// -------------------------------------------------------------------------
// LEVEL 2: STYLE WARNINGS (Warn)
// -------------------------------------------------------------------------
#![warn(
    // For `no-std` Situation Issues
    dead_code,
    warnings,
    unsafe_code,
    clippy::dbg_macro,
    clippy::todo,
    clippy::unnecessary_safety_comment
)]
// -------------------------------------------------------------------------
// LEVEL 3: ALLOW/IGNORABLE (Allow)
// -------------------------------------------------------------------------
#![allow(
    clippy::restriction,
    clippy::inline_always,
    unused_doc_comments,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::empty_line_after_doc_comments
)]

mod cli;
mod commands;

use clap::Parser;
use cli::Cli;
use cli::Commands;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = std::env::args().collect();
    // Handle Cargo subcommand invocation (when called via `cargo slide ...`)
    if args.len() > 1 && args[1] == "slide" {
        args.remove(1);
    }

    let cli = Cli::parse_from(args);

    // Initialize global log format
    let log_format = slide_core::logger::LogFormat::from_str(&cli.log_format);
    slide_core::logger::init_logger(log_format);

    dispatch_command(cli.command, cli.file)
}

#[allow(clippy::too_many_lines)]
fn dispatch_command(
    command: Option<Commands>,
    default_file: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        | Some(Commands::Init { path, rust, template }) => {
            commands::init::execute(&path, rust, &template)?;
        },
        | Some(Commands::New { name, rust, template }) => {
            commands::new::execute(&name, rust, &template)?;
        },
        | Some(Commands::Run {
            file,
            animation,
            watch,
            fullscreen,
        }) => {
            commands::run::execute(&file, &animation, watch, fullscreen)?;
        },
        | Some(Commands::Dev {
            file,
            animation,
            fullscreen,
        }) => {
            commands::run::execute(&file, &animation, true, fullscreen)?;
        },
        | Some(Commands::Edit { file, dark }) => {
            commands::edit::execute(file.as_deref(), dark)?;
        },
        | Some(Commands::Build {
            file,
            output,
            format,
            animation,
            target,
            source,
        }) => {
            commands::build::execute(
                &file,
                output,
                &format,
                &animation,
                target.as_deref(),
                source,
            )?;
        },
        | Some(Commands::Pack {
            file,
            output,
            animation,
            source,
        }) => {
            commands::pack::execute(&file, output, &animation, source)?;
        },
        | Some(Commands::Unpack { file, output }) => {
            commands::unpack::execute(&file, output)?;
        },
        | Some(Commands::Serve {
            file,
            port,
            ip,
            dir,
            open,
            watch,
        }) => {
            commands::serve::execute(&file, port, &ip, dir, open, watch)?;
        },
        | Some(Commands::Info { file }) => {
            commands::info::execute(&file)?;
        },
        | Some(Commands::Check { file }) => {
            commands::check::execute(&file)?;
        },
        | Some(Commands::Export { file, format, output }) => {
            commands::export::execute(&file, &format, output)?;
        },
        | Some(Commands::InstallViewer) => {
            println!("Installing slide-viewer universal presentation player...");
            let res = std::process::Command::new("cargo")
                .args([
                    "install",
                    "--path",
                    "crates/slide-viewer",
                    "--bin",
                    "slide-viewer",
                    "--force",
                ])
                .status();
            match res {
                | Ok(st) if st.success() => {
                    let _ = std::process::Command::new("slide-viewer")
                        .arg("install")
                        .status();
                    println!("[OK] slide-viewer installed and registered successfully!");
                },
                | _ => {
                    eprintln!("[ERR] Failed to build/install slide-viewer.");
                },
            }
        },
        | None => {
            let file = default_file.unwrap_or_else(|| PathBuf::from("slides.typ"));
            commands::run::execute(&file, "fade", false, false)?;
        },
    }

    Ok(())
}
