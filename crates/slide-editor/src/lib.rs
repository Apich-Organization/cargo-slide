//! # `slide-editor`
//!
//! Typora-style WYSIWYG live-preview editor for Typst presentations and documents.
//!
//! ## Key Modules
//! - [`app`]: Main application lifecycle and event dispatcher.
//! - [`document`]: Document model for both `.typ` and `.slide` presentation packages.
//! - [`compiler_bridge`]: Typst compilation engine and multi-format exporter (PDF, SVG, PNG, .slide).
//! - [`ui`]: UI components, light/dark themes, and views.

#![allow(
    missing_docs,
    clippy::pedantic,
    clippy::nursery,
    clippy::single_call_fn,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

pub mod app;
pub mod compiler_bridge;
pub mod document;
pub mod model;
pub mod ui;
