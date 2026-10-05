//! Typora-style WYSIWYG Live Preview widgets.
//!
//! Provides true in-place editing on 16:9 canvas slides, with zero-wildcard official Typst AST.

pub mod block_view;
pub mod complex_modal;
pub mod slide_canvas;
pub mod transition_modal;

pub use block_view::FormatAction;
pub use block_view::InsertBlockKind;
pub use block_view::render_rich_text;
pub use block_view::view_block;
pub use complex_modal::ComplexElementModal;
pub use complex_modal::ComplexModalState;
pub use complex_modal::view_complex_element_modal;
pub use slide_canvas::view_slide_canvas;
pub use transition_modal::view_slide_transition_modal;
