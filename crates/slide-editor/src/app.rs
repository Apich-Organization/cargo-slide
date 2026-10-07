//! Main application state machine and event dispatcher.

use crate::compiler_bridge::CompilerBridge;
use crate::compiler_bridge::CompilerDiagnostic;
use crate::document::DocumentFormat;
use crate::document::EditorDocument;
use crate::model::ast_engine::TypstDocumentEngine;
use crate::model::ast_engine::WysiwygBlock;
use crate::model::ast_engine::instrument_source_for_focus;
use crate::model::ast_engine::parse_length_to_pt;
use crate::ui::modals::view_error_details_modal;
use crate::ui::modals::view_export_modal;
use crate::ui::modals::view_open_modal;
use crate::ui::modals::view_presentation_health_modal;
use crate::ui::modals::view_template_library_modal;
use crate::ui::sidebar::view_sidebar;
use crate::ui::statusbar::view_statusbar;
use crate::ui::theme::AppTheme;
use crate::ui::toolbar::view_formatting_bar;
use crate::ui::toolbar::view_toolbar;
use crate::ui::views::focus_mode::view_focus_mode;
use crate::ui::views::live_preview::view_live_preview;
use crate::ui::views::source_mode::view_source_mode;
use crate::ui::wysiwyg::ComplexElementModal;
use crate::ui::wysiwyg::ComplexModalState;
use crate::ui::wysiwyg::FormatAction;
use crate::ui::wysiwyg::InsertBlockKind;
use crate::ui::wysiwyg::view_complex_element_modal;

use iced::Alignment;
use iced::Element;
use iced::Length;
use iced::Subscription;
use iced::Task;
use iced::widget::column;
use iced::widget::container;
use iced::widget::row;
use iced::widget::text_editor;
use std::ops::Range;
use std::path::Path;
use std::path::PathBuf;

/// Available editor view modes (Typora-style)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorMode {
    /// Seamless live preview with in-place WYSIWYG block editing
    #[default]
    LivePreview,
    /// Focused single slide editor with synchronized live SVG preview
    FocusMode,
    /// Full-document Typst source code editor
    SourceMode,
}

/// Compilation status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompilationStatus {
    Ready,
    Compiling,
    Error(CompilerDiagnostic),
}

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportFormat {
    #[default]
    Pdf,
    Png,
    Svg,
    SlidePackage,
}

impl ExportFormat {
    #[must_use]
    pub const fn default_extension(&self) -> &'static str {
        match self {
            | Self::Pdf => "pdf",
            | Self::Png => "png",
            | Self::Svg => "svg",
            | Self::SlidePackage => "slide",
        }
    }
}

/// Export slide page selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportPageSelection {
    #[default]
    All,
    Current,
    Custom,
}

/// Sidebar presentation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarViewMode {
    /// Compact text outline list
    #[default]
    Outline,
    /// 16:9 thumbnail previews of rendered slides
    Thumbnails,
}

/// Active modal dialog
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum ActiveModal {
    Export,
    Open,
    ErrorDetails(CompilerDiagnostic),
    ComplexElement(ComplexElementModal),
    SlideTransition {
        slide_idx: usize,
        current_transition: Option<String>,
        apply_to_all: bool,
    },
    ElementTransition {
        slide_idx: usize,
        block_idx: usize,
        order: usize,
        effect: String,
        is_existing: bool,
    },
    MoveBlockToSlide {
        from_slide_idx: usize,
        block_idx: usize,
        range: Range<usize>,
        block_label: String,
    },
    SlideContextMenu {
        slide_idx: usize,
        position: Option<iced::Point>,
    },
    BlockContextMenu {
        slide_idx: usize,
        block_idx: usize,
        range: Range<usize>,
        block_label: String,
        block_id: String,
        position: Option<iced::Point>,
    },
    FontSelector {
        search_query: String,
        current_font: String,
    },
    HeaderFooter {
        header_enabled: bool,
        header_left: String,
        header_right: String,
        footer_enabled: bool,
        footer_left: String,
        footer_right_mode: usize,
        footer_right_custom: String,
    },
    TemplateLibrary,
    PresentationHealth(
        Vec<slide_core::compiler::PresentationHealthIssue>,
        slide_core::pacing::DeckPacingReport,
    ),
    CommandPalette {
        query: String,
        selected_idx: usize,
    },
    RecoveryDraft {
        draft_content: String,
    },
}

/// State for search and replace operations (supporting regular expressions)
#[derive(Debug, Clone, Default)]
pub struct SearchState {
    pub is_visible: bool,
    pub search_query: String,
    pub replace_query: String,
    pub is_regex: bool,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub status_message: Option<String>,
    pub match_count: usize,
    pub current_match_idx: usize,
}

/// Main application state
pub struct SlideEditorApp {
    pub doc: EditorDocument,
    pub engine: TypstDocumentEngine,
    pub theme: AppTheme,
    pub mode: EditorMode,
    pub active_slide: usize,
    pub in_place_editing_slide: Option<usize>,
    pub in_place_content: text_editor::Content,
    pub focus_slide_content: text_editor::Content,
    pub source_content: text_editor::Content,
    // WYSIWYG Active in-place block state
    pub active_block_id: Option<String>,
    pub active_block_range: Option<Range<usize>>,
    pub active_block_content: text_editor::Content,
    pub raw_code_blocks: std::collections::HashSet<String>,
    pub sidebar_visible: bool,
    pub sidebar_view_mode: SidebarViewMode,
    pub zoom_percent: u32,
    pub speaking_wpm: u32,
    pub compiler: CompilerBridge,
    pub compilation_status: CompilationStatus,
    pub active_modal: Option<ActiveModal>,
    // Modal states
    pub export_format: ExportFormat,
    pub export_path: String,
    pub export_png_scale: f32,
    pub export_status: Option<String>,
    pub export_page_selection: ExportPageSelection,
    pub export_custom_range: String,
    pub export_include_source: bool,
    pub open_path: String,
    pub open_error: Option<String>,
    pub font_search_query: String,
    // Search and Replace state
    pub search_state: SearchState,
    // Hover math formula pre-render HUD state
    pub active_hover_formula: Option<String>,
    // Pre-rendered slide cache for buttery-smooth 60 FPS scrolling
    pub slide_images: Vec<iced::widget::image::Handle>,
    pub slide_titles: Vec<String>,
    pub cached_svg_hashes: Vec<u64>,
    pub cached_char_count: usize,
    pub cached_word_count: usize,
    pub compilation_in_flight: bool,
    pub pending_recompile: bool,
    pub slide_view_vector: std::collections::HashSet<usize>,
    pub equation_images: std::collections::HashMap<String, iced::widget::image::Handle>,
    pub code_images: std::collections::HashMap<String, iced::widget::image::Handle>,
    pub preview_hover_ratio: f32,
    pub dragging_block: Option<(usize, usize, f32, bool)>,
    pub dragging_spacing: Option<(usize, usize, f32)>,
    pub modal_col_editors: Vec<text_editor::Content>,
    pub modal_box_editor: text_editor::Content,
    pub modal_title_slide_editor: text_editor::Content,
    pub modal_callout_editor: text_editor::Content,
    pub undo_stack: Vec<DocumentSnapshot>,
    pub redo_stack: Vec<DocumentSnapshot>,
    pub recent_files: Vec<String>,
    pub window_size: iced::Size,
    pub last_cursor_pos: Option<iced::Point>,
}

/// Undo / Redo history snapshot
#[derive(Debug, Clone)]
pub struct DocumentSnapshot {
    pub source_text: String,
    pub active_slide: usize,
}

pub const TEMPLATE_SOURCES: [&str; 8] = [
    // 0: Title Hero
    "#title-slide(\n  title: [Presentation Title],\n  subtitle: [A Code-Driven Presentation],\n  author: [Presenter Name],\n  date: datetime.today().display(),\n)",
    // 1: 2-Column Comparison
    "#slide(title: [Feature Comparison])[\n  #cols(columns: (1fr, 1fr))[\n    #callout(title: [Traditional Tools], kind: \"warning\")[\n      - Manual alignment friction\n      - Bulky binary formats\n      - Clunky diff & review\n    ]\n  ][\n    #callout(title: [Cargo Slide], kind: \"info\")[\n      - Declarative Typst markup\n      - Instant live preview\n      - Zero-overhead native runtime\n    ]\n  ]\n]",
    // 2: 3-Column Feature Pillars
    "#slide(title: [Architecture Pillars])[\n  #cols(columns: (1fr, 1fr, 1fr))[\n    #block(stroke: 1pt + rgb(\"0284c7\"), inset: 10pt, radius: 6pt)[\n      === Blazing Fast\n      Instant SVG recompilation and cached rendering\n    ]\n  ][\n    #block(stroke: 1pt + rgb(\"16a34a\"), inset: 10pt, radius: 6pt)[\n      === Expressive\n      Native charts, math, audio, and transitions\n    ]\n  ][\n    #block(stroke: 1pt + rgb(\"9333ea\"), inset: 10pt, radius: 6pt)[\n      === Standalone\n      Compressed LZMA2 `.slide` bundles\n    ]\n  ]\n]",
    // 3: Code & Commentary
    "#slide(title: [Implementation Details])[\n  #cols(columns: (3fr, 2fr))[\n    ```rust\n    fn main() {\n        println!(\"Hello, Cargo Slide!\");\n    }\n    ```\n  ][\n    - Zero-dependency native Rust player\n    - Hardware-smooth page transitions\n    - Presenter timer & whiteboard ink\n  ]\n]",
    // 4: Metric KPI Showcase
    "#slide(title: [Key Performance Metrics])[\n  #cols(columns: (1fr, 1fr, 1fr))[\n    #metric(title: [Render Latency], value: [16 ms], delta: [-45%])\n  ][\n    #metric(title: [Playback FPS], value: [60 FPS], delta: [+100%])\n  ][\n    #metric(title: [Bundle Size], value: [1.2 MB], delta: [-80%])\n  ]\n]",
    // 5: Quote Spotlight
    "#slide(title: [Design Philosophy])[\n  #align(center + horizon)[\n    #quote(attribution: [Design Principles])[\n      Simplicity is prerequisite for reliability.\n    ]\n  ]\n]",
    // 6: Roadmap Timeline
    "#slide(title: [Product Roadmap])[\n  - [x] Phase 1: Core Compiler & SVG Engine\n  - [x] Phase 2: Native Player & Presentation Tools\n  - [x] Phase 3: Typora-Style WYSIWYG Editor\n  - [ ] Phase 4: Collaborative Presentation Cloud\n]",
    // 7: Closing Q&A
    "#slide(title: [Thank You!])[\n  #align(center + horizon)[\n    = Questions & Discussion\n\n    #v(0.5cm)\n    GitHub: `github.com/user/cargo-slide`\n\n    #v(0.3cm)\n    Thank you for listening!\n  ]\n]",
];

fn get_recent_editor_file() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(|h| {
            PathBuf::from(h)
                .join(".config")
                .join("cargo-slide")
                .join("recent_editor.json")
        })
}

pub fn load_recent_editor_files() -> Vec<String> {
    if let Some(path) = get_recent_editor_file()
        && path.is_file()
        && let Ok(content) = std::fs::read_to_string(path)
        && let Ok(list) = serde_json::from_str::<Vec<String>>(&content)
    {
        return list;
    }
    Vec::new()
}

pub fn save_recent_editor_file(file_path: &Path) {
    let mut recents = load_recent_editor_files();
    let canonical = file_path.canonicalize().unwrap_or_else(|_| {
        if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_default().join(file_path)
        }
    });
    let s = canonical.to_string_lossy().to_string();
    recents.retain(|p| p != &s);
    recents.insert(0, s);
    if recents.len() > 10 {
        recents.truncate(10);
    }
    if let Some(path) = get_recent_editor_file() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&recents) {
            let _ = std::fs::write(path, json);
        }
    }
}

/// Application messages
#[derive(Debug, Clone)]
pub enum Message {
    SwitchMode(EditorMode),
    ToggleTheme,
    ToggleSidebar,
    SelectSlide(usize),
    ToggleSlideViewMode(usize),
    ToggleInPlaceEdit(usize),
    InPlaceEditorAction(text_editor::Action),
    FocusSlideEditorAction(text_editor::Action),
    SourceEditorAction(text_editor::Action),
    // Focus mode jump navigation messages
    PreviewHoverRatio(f32),
    JumpToCodeFromPreview,
    JumpToBlockCode(String),
    JumpToLine(usize),
    JumpToFocusBlock {
        slide_idx: usize,
        block_idx: usize,
    },
    // WYSIWYG in-place block editing messages
    ActivateBlock {
        slide_idx: usize,
        id: String,
        range: Range<usize>,
        raw: String,
    },
    DeactivateBlock,
    ToggleBlockRawCode(String),
    ActiveBlockAction(text_editor::Action),
    StartDragBlock {
        slide_idx: usize,
        block_idx: usize,
    },
    DragBlockY {
        slide_idx: usize,
        block_idx: usize,
        y: f32,
    },
    GlobalCursorMoved(iced::Point),
    WindowResized(iced::Size),
    GlobalButtonReleased,
    EndDragBlock,
    EndBlockInteraction {
        slide_idx: usize,
        block_idx: usize,
        id: String,
        range: Range<usize>,
        raw: String,
    },
    StartDragSpacing {
        slide_idx: usize,
        block_idx: usize,
    },
    DragSpacingY {
        slide_idx: usize,
        block_idx: usize,
        y: f32,
    },
    EndDragSpacing,
    MoveBlockUp {
        slide_idx: usize,
        block_idx: usize,
    },
    MoveBlockDown {
        slide_idx: usize,
        block_idx: usize,
    },
    AdjustBlockSpacing {
        slide_idx: usize,
        block_idx: usize,
        delta_pt: i32,
    },
    UpdateBlockRange {
        slide_idx: Option<usize>,
        range: Range<usize>,
        new_text: String,
    },
    InsertBlockAfter {
        offset: usize,
        kind: InsertBlockKind,
    },
    DeleteBlockAtRange(Range<usize>),
    FormatBlock(FormatAction),
    InsertSlide,
    InsertSlideAt(usize),
    DuplicateSlide(usize),
    MoveSlide {
        from_idx: usize,
        to_idx: usize,
    },
    DeleteSlide(usize),
    ToggleSidebarViewMode,
    OpenSlideContextMenu(usize),
    OpenBlockContextMenu {
        slide_idx: usize,
        block_idx: usize,
        range: Range<usize>,
        block_label: String,
        block_id: String,
    },
    OpenMoveBlockModal {
        from_slide_idx: usize,
        block_idx: usize,
        range: Range<usize>,
        block_label: String,
    },
    MoveBlockToSlide {
        from_slide_idx: usize,
        block_range: Range<usize>,
        to_slide_idx: usize,
    },
    DuplicateBlock(Range<usize>),
    OpenFontModal,
    FontSearchQueryChanged(String),
    SelectFont(String),
    UpdateSlideTitle {
        slide_idx: usize,
        new_title: String,
    },
    InsertSnippet(&'static str),
    CompilationCompleted(std::result::Result<slide_core::model::SlideDeck, CompilerDiagnostic>),
    ZoomIn,
    ZoomOut,
    NewDocument,
    OpenDocumentDialog,
    BrowseOpenPath,
    OpenPathChanged(String),
    ExecuteOpen,
    SaveDocument,
    OpenExportDialog,
    SelectExportFormat(ExportFormat),
    SelectExportPageSelection(ExportPageSelection),
    ExportCustomRangeChanged(String),
    ToggleExportIncludeSource,
    ExportPathChanged(String),
    SelectPngScale(f32),
    BrowseExportPath,
    ExecuteExport,
    PlayPresentation,
    OpenErrorDetailsDialog,
    CloseModal,
    // Complex element dedicated modal dialog messages
    OpenComplexModal {
        slide_idx: usize,
        block_id: String,
        range: Range<usize>,
        raw: String,
        callee: String,
    },
    CloseComplexModal,
    ApplyComplexModal,
    ModalTableUpdateCell {
        row: usize,
        col: usize,
        val: String,
    },
    ModalTableAddRow,
    ModalTableDeleteRow(usize),
    ModalTableAddCol,
    ModalTableDeleteCol(usize),
    ModalTableUpdateColSpec(String),
    ModalTableToggleHeader,
    ModalTableUpdateHeaderCell {
        col: usize,
        val: String,
    },
    ModalChartUpdateTitle(String),
    ModalChartSetType(String),
    ModalChartUpdateSource(String),
    ModalChartUpdateSql(String),
    ModalChartUpdateDsl(String),
    ModalChartUpdateFormat(String),
    ModalChartUpdateUnit(String),
    ModalChartUpdatePrefix(String),
    ModalChartUpdateItemLabel {
        idx: usize,
        label: String,
    },
    ModalChartUpdateItemValue {
        idx: usize,
        val: f32,
    },
    ModalChartAddItem,
    ModalChartDeleteItem(usize),
    ModalGridUpdateCol {
        idx: usize,
        val: String,
    },
    ModalGridEditorAction {
        idx: usize,
        action: text_editor::Action,
    },
    ModalGridAddCol,
    ModalGridDeleteCol(usize),
    ModalCalloutSetKind(String),
    ModalCalloutSetStrokeColor(String),
    ModalCalloutUpdateStrokeColor(String),
    ModalCalloutUpdateTitle(String),
    ModalCalloutUpdateBody(String),
    ModalCalloutEditorAction(text_editor::Action),
    ModalBoxUpdateContent(String),
    ModalBoxEditorAction(text_editor::Action),
    ModalBoxApplyPreset(&'static str),
    ModalBoxSetFill(String),
    ModalBoxSetStroke(String),
    ModalBoxSetRadius(String),
    ModalBoxSetInset(String),
    ModalBoxSetWidth(String),
    ModalLinkUpdateUrl(String),
    ModalLinkUpdateLabel(String),
    ModalTitleSlideUpdateTitle(String),
    ModalTitleSlideUpdateSubtitle(String),
    ModalTitleSlideUpdateAuthor(String),
    ModalTitleSlideUpdateDate(String),
    ModalTitleSlideUpdateVersion(String),
    ModalTitleSlideUpdateInstitution(String),
    ModalTitleSlideAddExtraArg,
    ModalTitleSlideUpdateExtraKey(usize, String),
    ModalTitleSlideUpdateExtraVal(usize, String),
    ModalTitleSlideRemoveExtraArg(usize),
    ModalTitleSlideEditorAction(text_editor::Action),
    ModalTitleSlideApplyPreset {
        title: Option<String>,
        subtitle: Option<String>,
        author: Option<String>,
        date: Option<String>,
        version: Option<String>,
        institution: Option<String>,
    },
    // Slide transition messages
    OpenTransitionModal(usize),
    SelectTransition(Option<String>),
    ApplySlideTransition {
        slide_idx: usize,
        transition: Option<String>,
        all_slides: bool,
    },
    // Badge modal messages
    ModalBadgeUpdateLabel(String),
    ModalBadgeSetFill(String),
    ModalBadgeSetTextColor(String),
    // Video modal messages
    ModalVideoUpdateSource(String),
    ModalVideoUpdateCaption(String),
    ModalVideoUpdateDuration(String),
    ModalVideoSetQuality(String),
    ModalVideoSetStyle(String),
    ModalVideoSetWidth(String),
    // Audio modal messages
    ModalAudioUpdateSource(String),
    ModalAudioSetIsPlayer(bool),
    ModalAudioUpdateTitle(String),
    ModalAudioUpdateArtist(String),
    ModalAudioToggleAutoplay,
    ModalAudioToggleLoop,
    ModalAudioSetVolume(f32),
    // Element step transition messages
    OpenElementTransitionModal {
        slide_idx: usize,
        block_idx: usize,
    },
    SelectElementTransitionEffect(String),
    SetElementTransitionOrder(usize),
    ApplyElementTransition,
    RemoveElementTransition,
    OpenUrl(String),
    // Search and Replace messages
    OpenSearchModal,
    CloseSearchBar,
    SearchQueryChanged(String),
    ReplaceQueryChanged(String),
    ToggleSearchRegex,
    ToggleSearchCaseSensitive,
    ToggleSearchWholeWord,
    FindNextMatch,
    FindPrevMatch,
    ExecuteReplaceCurrent,
    ExecuteReplaceAllMatches,
    // Header & Footer customization messages
    OpenHeaderFooterModal,
    ToggleHeaderEnabled(bool),
    HeaderLeftChanged(String),
    HeaderRightChanged(String),
    ToggleFooterEnabled(bool),
    FooterLeftChanged(String),
    FooterRightModeChanged(usize),
    FooterRightCustomChanged(String),
    ApplyHeaderFooterSettings,
    // Intelligent quick fix
    ApplyQuickFix(crate::compiler_bridge::QuickFix),
    // Multi-level undo / redo
    Undo,
    Redo,
    // Template library modal
    OpenTemplateLibraryModal,
    InsertTemplateSlide(usize),
    // Presentation health & pacing inspector
    OpenPresentationHealthModal,
    // Recent documents
    OpenRecentFile(String),
    // Slide keyboard management
    MoveActiveSlideUp,
    MoveActiveSlideDown,
    DuplicateActiveSlide,
    InsertSlideAfterActive,
    DeleteActiveSlide,
    // Zoom & Pacing management
    ResetZoom,
    CycleSpeakingPace,
    // Command Palette
    OpenCommandPalette,
    CommandPaletteQueryChanged(String),
    CommandPaletteSelectIndex(usize),
    CommandPaletteExecute,
    // Agenda slide generation
    GenerateAgendaSlide,
    // Session recovery draft restoration
    RestoreRecoveryDraft(String),
    DiscardRecoveryDraft,
}

impl SlideEditorApp {
    /// Initialize application state
    #[must_use]
    pub fn new(
        initial_file: Option<PathBuf>,
        dark_mode: bool,
    ) -> Self {
        let theme = if dark_mode {
            AppTheme::Dark
        } else {
            AppTheme::Light
        };
        let mut doc = if let Some(ref path) = initial_file {
            EditorDocument::open(path)
                .unwrap_or_else(|_| EditorDocument::new_presentation("New Presentation"))
        } else {
            EditorDocument::new_presentation("New Presentation")
        };

        let compiler = CompilerBridge::new();
        let initial_text = doc.source_text.clone();

        // Compile initial document only if not already loaded from a pre-compiled package
        let (deck, status) = if doc.deck.is_some() {
            (None, CompilationStatus::Ready)
        } else {
            match compiler.compile_source(&initial_text, doc.assets_dir.as_deref()) {
                | Ok(d) => (Some(d), CompilationStatus::Ready),
                | Err(diag) => (None, CompilationStatus::Error(diag)),
            }
        };

        if doc.deck.is_none() {
            doc.deck = deck;
        }

        let mut slide_view_vector = std::collections::HashSet::new();
        if doc.format == DocumentFormat::SlidePackage {
            let has_editable_source =
                doc.has_editable_source() || !doc.source_text.trim().is_empty();
            if !has_editable_source {
                slide_view_vector = (0..doc.total_slides()).collect();
            }
        }

        let engine = TypstDocumentEngine::from_source(doc.source_text.clone());
        let first_slide_chunk = doc.get_slide_chunks().first().cloned().unwrap_or_default();

        let in_place_content = text_editor::Content::with_text(&first_slide_chunk);
        let focus_slide_content = text_editor::Content::with_text(&first_slide_chunk);
        let source_content = text_editor::Content::with_text(&doc.source_text);
        let active_block_content = text_editor::Content::new();

        let default_export_path = doc.file_path.as_ref().map_or_else(
            || "presentation.pdf".to_string(),
            |p| {
                let mut out = p.clone();
                out.set_extension("pdf");
                out.to_string_lossy().to_string()
            },
        );

        let mut app = Self {
            doc,
            engine,
            theme,
            mode: EditorMode::LivePreview,
            active_slide: 0,
            in_place_editing_slide: None,
            in_place_content,
            focus_slide_content,
            source_content,
            active_block_id: None,
            active_block_range: None,
            active_block_content,
            raw_code_blocks: std::collections::HashSet::new(),
            sidebar_visible: true,
            sidebar_view_mode: SidebarViewMode::default(),
            zoom_percent: 100,
            speaking_wpm: 130,
            compiler,
            compilation_status: status,
            active_modal: None,
            export_format: ExportFormat::Pdf,
            export_path: default_export_path,
            export_png_scale: 2.0,
            export_status: None,
            export_page_selection: ExportPageSelection::All,
            export_custom_range: String::new(),
            export_include_source: true,
            open_path: String::new(),
            open_error: None,
            font_search_query: String::new(),
            search_state: SearchState::default(),
            active_hover_formula: None,
            slide_images: Vec::new(),
            slide_titles: Vec::new(),
            cached_svg_hashes: Vec::new(),
            cached_char_count: 0,
            cached_word_count: 0,
            compilation_in_flight: false,
            pending_recompile: false,
            slide_view_vector,
            equation_images: std::collections::HashMap::new(),
            code_images: std::collections::HashMap::new(),
            preview_hover_ratio: 0.0,
            dragging_block: None,
            dragging_spacing: None,
            modal_col_editors: Vec::new(),
            modal_box_editor: text_editor::Content::new(),
            modal_title_slide_editor: text_editor::Content::new(),
            modal_callout_editor: text_editor::Content::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            recent_files: load_recent_editor_files(),
            window_size: iced::Size::new(1280.0, 720.0),
            last_cursor_pos: None,
        };
        if let Some(ref p) = initial_file {
            save_recent_editor_file(p);
            if let Some(draft) = EditorDocument::check_recovery_draft(p)
                && draft.trim() != app.doc.source_text.trim()
            {
                app.active_modal = Some(ActiveModal::RecoveryDraft { draft_content: draft });
            }
        }
        app.update_slide_cache();
        app
    }

    /// Record a snapshot for multi-level Undo before document mutation
    pub fn push_undo_snapshot(&mut self) {
        if self.undo_stack.last().map(|s| &s.source_text) == Some(&self.doc.source_text) {
            return;
        }
        self.undo_stack.push(DocumentSnapshot {
            source_text: self.doc.source_text.clone(),
            active_slide: self.active_slide,
        });
        if self.undo_stack.len() > 50 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    /// Commit active block editing in-place into engine and document
    pub fn commit_active_block(&mut self) {
        if let Some(range) = self.active_block_range.take() {
            let new_text = self.active_block_content.text();
            if self.engine.source_text.get(range.clone()) != Some(&new_text) {
                self.push_undo_snapshot();
            }
            let _ = self.engine.update_block_at_range(range, &new_text);
            self.doc.source_text = self.engine.source_text.clone();
            self.doc.is_dirty = true;
            self.doc.sync_chunks_from_source();
        }
        self.active_block_id = None;
        self.active_block_range = None;
    }

    /// Re-compute search matches for current query and regex/case options
    pub fn compute_search_matches(&mut self) {
        let query = &self.search_state.search_query;
        if query.is_empty() {
            self.search_state.match_count = 0;
            self.search_state.status_message = None;
            return;
        }

        let text = &self.doc.source_text;
        let res: Result<Vec<(usize, usize)>, String> = if self.search_state.is_regex {
            regex::RegexBuilder::new(query)
                .case_insensitive(!self.search_state.case_sensitive)
                .build()
                .map(|re| re.find_iter(text).map(|m| (m.start(), m.end())).collect())
                .map_err(|e| format!("Regex error: {e}"))
        } else if self.search_state.whole_word {
            let pat = format!(r"\b{}\b", regex::escape(query));
            regex::RegexBuilder::new(&pat)
                .case_insensitive(!self.search_state.case_sensitive)
                .build()
                .map(|re| re.find_iter(text).map(|m| (m.start(), m.end())).collect())
                .map_err(|e| format!("Regex error: {e}"))
        } else if self.search_state.case_sensitive {
            let mut matches = Vec::new();
            let mut start = 0;
            while let Some(pos) = text[start..].find(query) {
                let abs = start + pos;
                matches.push((abs, abs + query.len()));
                start = abs + query.len().max(1);
            }
            Ok(matches)
        } else {
            let lower_text = text.to_lowercase();
            let lower_q = query.to_lowercase();
            let mut matches = Vec::new();
            let mut start = 0;
            while let Some(pos) = lower_text[start..].find(&lower_q) {
                let abs = start + pos;
                matches.push((abs, abs + query.len()));
                start = abs + query.len().max(1);
            }
            Ok(matches)
        };

        match res {
            | Ok(matches) => {
                self.search_state.match_count = matches.len();
                if matches.is_empty() {
                    self.search_state.status_message = Some("No matches found".to_string());
                } else {
                    let cur = self.search_state.current_match_idx.min(matches.len() - 1);
                    self.search_state.current_match_idx = cur;
                    self.search_state.status_message =
                        Some(format!("Match {} of {}", cur + 1, matches.len()));
                }
            },
            | Err(err) => {
                self.search_state.match_count = 0;
                self.search_state.status_message = Some(err);
            },
        }
    }

    /// Retrieve all byte ranges matching the active search query
    #[must_use]
    pub fn get_search_matches(&self) -> Vec<(usize, usize)> {
        let query = &self.search_state.search_query;
        if query.is_empty() {
            return Vec::new();
        }
        let text = &self.doc.source_text;
        if self.search_state.is_regex {
            if let Ok(re) = regex::RegexBuilder::new(query)
                .case_insensitive(!self.search_state.case_sensitive)
                .build()
            {
                return re.find_iter(text).map(|m| (m.start(), m.end())).collect();
            }
        } else if self.search_state.whole_word {
            let pat = format!(r"\b{}\b", regex::escape(query));
            if let Ok(re) = regex::RegexBuilder::new(&pat)
                .case_insensitive(!self.search_state.case_sensitive)
                .build()
            {
                return re.find_iter(text).map(|m| (m.start(), m.end())).collect();
            }
        } else if self.search_state.case_sensitive {
            let mut matches = Vec::new();
            let mut start = 0;
            while let Some(pos) = text[start..].find(query) {
                let abs = start + pos;
                matches.push((abs, abs + query.len()));
                start = abs + query.len().max(1);
            }
            return matches;
        } else {
            let lower_text = text.to_lowercase();
            let lower_q = query.to_lowercase();
            let mut matches = Vec::new();
            let mut start = 0;
            while let Some(pos) = lower_text[start..].find(&lower_q) {
                let abs = start + pos;
                matches.push((abs, abs + query.len()));
                start = abs + query.len().max(1);
            }
            return matches;
        }
        Vec::new()
    }

    /// Calculate target vertical scroll offset for sidebar based on slide index and view mode
    #[must_use]
    pub fn get_sidebar_scroll_offset(
        &self,
        slide_idx: usize,
    ) -> f32 {
        let item_height = if self.sidebar_view_mode == SidebarViewMode::Thumbnails {
            190.0
        } else {
            48.0
        };
        (slide_idx as f32 * item_height).max(0.0)
    }

    /// Construct a task to scroll the sidebar to keep the active slide in view
    pub fn scroll_sidebar_to_active_slide(&self) -> Task<Message> {
        let target_y = self.get_sidebar_scroll_offset(self.active_slide);
        iced::widget::operation::scroll_to(
            iced::widget::Id::new("sidebar_scrollable"),
            iced::widget::scrollable::AbsoluteOffset {
                x: None,
                y: Some(target_y),
            },
        )
    }

    /// Jump to match location, synchronize active slide, and scroll left sidebar and right preview/editor
    pub fn jump_to_and_select_match(
        &mut self,
        start_byte: usize,
        end_byte: usize,
    ) -> Task<Message> {
        let target_slide = self
            .engine
            .slides
            .iter()
            .position(|s| s.range.contains(&start_byte))
            .or_else(|| {
                if self
                    .engine
                    .slides
                    .first()
                    .map(|s| start_byte < s.range.start)
                    .unwrap_or(false)
                {
                    Some(0)
                } else {
                    None
                }
            })
            .unwrap_or(0);

        let old_slide = self.active_slide;
        self.active_slide = target_slide.min(self.engine.slides.len().saturating_sub(1));

        let chunks = self.doc.get_slide_chunks();
        let chunk_text = chunks.get(self.active_slide).cloned().unwrap_or_default();
        self.focus_slide_content = text_editor::Content::with_text(&chunk_text);

        let full_text = &self.doc.source_text;
        if start_byte <= full_text.len() && end_byte <= full_text.len() {
            let start_line = full_text[..start_byte]
                .bytes()
                .filter(|&b| b == b'\n')
                .count();
            let line_start_byte = full_text[..start_byte]
                .rfind('\n')
                .map(|p| p + 1)
                .unwrap_or(0);
            let start_col = full_text[line_start_byte..start_byte].chars().count();
            let match_len = full_text[start_byte..end_byte].chars().count();

            // Select in source editor
            self.source_content = text_editor::Content::with_text(full_text);
            self.source_content.perform(text_editor::Action::Move(
                text_editor::Motion::DocumentStart,
            ));
            for _ in 0..start_line {
                self.source_content
                    .perform(text_editor::Action::Move(text_editor::Motion::Down));
            }
            for _ in 0..start_col {
                self.source_content
                    .perform(text_editor::Action::Move(text_editor::Motion::Right));
            }
            for _ in 0..match_len {
                self.source_content
                    .perform(text_editor::Action::Select(text_editor::Motion::Right));
            }

            // Also select in focus editor if on active slide
            if let Some(slide) = self.engine.slides.get(self.active_slide)
                && start_byte >= slide.range.start
                && end_byte <= slide.range.end
            {
                let rel_start = start_byte - slide.range.start;
                let rel_end = end_byte - slide.range.start;
                let slide_text = self.focus_slide_content.text();
                if rel_end <= slide_text.len() {
                    let f_start_line = slide_text[..rel_start]
                        .bytes()
                        .filter(|&b| b == b'\n')
                        .count();
                    let f_line_start = slide_text[..rel_start]
                        .rfind('\n')
                        .map(|p| p + 1)
                        .unwrap_or(0);
                    let f_start_col = slide_text[f_line_start..rel_start].chars().count();
                    self.focus_slide_content.perform(text_editor::Action::Move(
                        text_editor::Motion::DocumentStart,
                    ));
                    for _ in 0..f_start_line {
                        self.focus_slide_content
                            .perform(text_editor::Action::Move(text_editor::Motion::Down));
                    }
                    for _ in 0..f_start_col {
                        self.focus_slide_content
                            .perform(text_editor::Action::Move(text_editor::Motion::Right));
                    }
                    for _ in 0..match_len {
                        self.focus_slide_content
                            .perform(text_editor::Action::Select(text_editor::Motion::Right));
                    }
                }
            }
        }

        let sidebar_task = self.scroll_sidebar_to_active_slide();

        match self.mode {
            | EditorMode::LivePreview => {
                let scale = self.zoom_percent as f32 / 100.0;
                let preview_target_y =
                    (24.0 + (self.active_slide as f32) * (461.25 * scale + 88.0)).max(0.0);
                let preview_task = iced::widget::operation::scroll_to(
                    iced::widget::Id::new("live_preview_scrollable"),
                    iced::widget::scrollable::AbsoluteOffset {
                        x: Some(0.0),
                        y: Some(preview_target_y),
                    },
                );
                Task::batch(vec![preview_task, sidebar_task])
            },
            | EditorMode::FocusMode => {
                let mut tasks = vec![sidebar_task];
                if old_slide != self.active_slide {
                    tasks.push(self.trigger_recompile_task());
                }
                Task::batch(tasks)
            },
            | EditorMode::SourceMode => sidebar_task,
        }
    }

    /// Update pre-rendered slide images and titles cache using SIMD tiny-skia rasters
    pub fn update_slide_cache(&mut self) {
        let mut new_images = Vec::new();
        let mut new_titles = Vec::new();
        let mut new_hashes = Vec::new();

        // 1. Extract titles from official AST engine slides
        if !self.engine.slides.is_empty() {
            for slide in &self.engine.slides {
                new_titles.push(slide.title.clone());
            }
        } else {
            let chunks = self.doc.get_slide_chunks();
            for (idx, c) in chunks.iter().enumerate() {
                let mut title = None;
                for line in c.lines() {
                    let trimmed = line.trim();
                    if let Some(h) = trimmed.strip_prefix("= ") {
                        title = Some(h.trim().to_string());
                        break;
                    }
                }
                new_titles.push(title.unwrap_or_else(|| format!("Slide {}", idx + 1)));
            }
        }

        // 2. Render and cache slide preview rasters
        if let Some(ref deck) = self.doc.deck {
            for (idx, slide) in deck.slides.iter().enumerate() {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                std::hash::Hash::hash(&slide.svg_data, &mut hasher);
                let hash = std::hash::Hasher::finish(&hasher);
                new_hashes.push(hash);

                // Check if we can reuse the cached image Handle
                if idx < self.cached_svg_hashes.len()
                    && self.cached_svg_hashes[idx] == hash
                    && idx < self.slide_images.len()
                {
                    new_images.push(self.slide_images[idx].clone());
                } else {
                    let handle = CompilerBridge::render_slide_to_image(&slide.svg_data, 820.0)
                        .unwrap_or_else(|| {
                            iced::widget::image::Handle::from_rgba(1, 1, vec![255, 255, 255, 255])
                        });
                    new_images.push(handle);
                }
            }
        }

        self.slide_images = new_images;
        self.slide_titles = new_titles;
        self.cached_svg_hashes = new_hashes;
        self.cached_char_count = self.doc.source_text.chars().count();
        self.cached_word_count = self.doc.source_text.split_whitespace().count();

        // 3. Render and cache vector math equations & code blocks via Typst
        for slide in &self.engine.slides {
            for block in &slide.blocks {
                match block {
                    | WysiwygBlock::Equation { formula, id, .. } => {
                        if !self.equation_images.contains_key(id)
                            && let Some(handle) = self
                                .compiler
                                .render_equation_to_image(formula, self.theme.is_dark())
                        {
                            self.equation_images.insert(id.clone(), handle.clone());
                            self.equation_images.insert(formula.clone(), handle);
                        }
                    },
                    | WysiwygBlock::CodeBlock {
                        code, language, id, ..
                    } => {
                        if !self.code_images.contains_key(id)
                            && let Some(handle) = self.compiler.render_code_to_image(
                                code,
                                language,
                                self.theme.is_dark(),
                            )
                        {
                            self.code_images.insert(id.clone(), handle.clone());
                            self.code_images.insert(code.clone(), handle);
                        }
                    },
                    | _ => {},
                }
            }
        }
    }

    /// Re-compile document from current source_text synchronously
    pub fn trigger_recompile(&mut self) {
        self.compilation_status = CompilationStatus::Compiling;
        match self
            .compiler
            .compile_source(&self.doc.source_text, self.doc.assets_dir.as_deref())
        {
            | Ok(deck) => {
                self.doc.deck = Some(deck);
                self.compilation_status = CompilationStatus::Ready;
                self.update_slide_cache();
            },
            | Err(diag) => {
                self.compilation_status = CompilationStatus::Error(diag);
            },
        }
    }

    /// Trigger asynchronous background compilation without blocking the UI thread
    #[allow(clippy::result_large_err)]
    pub fn trigger_recompile_task(&mut self) -> Task<Message> {
        if self.compilation_in_flight {
            self.pending_recompile = true;
            return Task::none();
        }

        self.compilation_in_flight = true;
        self.pending_recompile = false;
        self.compilation_status = CompilationStatus::Compiling;

        if self.doc.is_dirty {
            let _ = self.doc.save_recovery_draft();
        }

        let fallback_clean = self.doc.source_text.clone();
        let source = if self.mode == EditorMode::FocusMode {
            instrument_source_for_focus(&self.doc.source_text, self.active_slide, &self.engine)
        } else {
            fallback_clean.clone()
        };
        let assets = self.doc.assets_dir.clone();

        Task::perform(
            async move {
                std::thread::spawn(move || {
                    let compiler = CompilerBridge::new();
                    compiler
                        .compile_source(&source, assets.as_deref())
                        .or_else(|_| compiler.compile_source(&fallback_clean, assets.as_deref()))
                })
                .join()
                .unwrap_or_else(|_| {
                    Err(CompilerDiagnostic {
                        message: "Compilation worker thread panicked".to_string(),
                        line: None,
                        column: None,
                        full_stderr: String::new(),
                        quick_fix: None,
                    })
                })
            },
            Message::CompilationCompleted,
        )
    }

    /// Synchronize editor contents when slide or source changes
    pub fn sync_editors_from_doc(&mut self) {
        let chunks = self.doc.get_slide_chunks();
        let chunk_text = chunks.get(self.active_slide).cloned().unwrap_or_default();

        self.focus_slide_content = text_editor::Content::with_text(&chunk_text);
        if let Some(s_idx) = self.in_place_editing_slide {
            let in_place_chunk = chunks.get(s_idx).cloned().unwrap_or_default();
            self.in_place_content = text_editor::Content::with_text(&in_place_chunk);
        }
        self.source_content = text_editor::Content::with_text(&self.doc.source_text);
    }

    /// Handle application messages
    pub fn update(
        &mut self,
        message: Message,
    ) -> Task<Message> {
        let mut task = Task::none();

        match message {
            | Message::SwitchMode(mode) => {
                self.commit_active_block();
                let old_mode = self.mode;
                self.mode = mode;
                self.sync_editors_from_doc();
                if old_mode != mode {
                    task = self.trigger_recompile_task();
                }
            },
            | Message::ToggleTheme => {
                self.theme = self.theme.toggle();
                self.equation_images.clear();
                self.code_images.clear();
                self.update_slide_cache();
            },
            | Message::ToggleSidebar => {
                self.sidebar_visible = !self.sidebar_visible;
            },
            | Message::SelectSlide(idx) => {
                self.commit_active_block();
                let old_slide = self.active_slide;
                self.active_slide = idx.min(self.engine.slides.len().saturating_sub(1));
                let chunks = self.doc.get_slide_chunks();
                let chunk_text = chunks.get(self.active_slide).cloned().unwrap_or_default();
                self.focus_slide_content = text_editor::Content::with_text(&chunk_text);
                let sidebar_task = self.scroll_sidebar_to_active_slide();
                if self.mode == EditorMode::FocusMode && old_slide != self.active_slide {
                    task = Task::batch(vec![self.trigger_recompile_task(), sidebar_task]);
                } else if self.mode == EditorMode::LivePreview {
                    let scale = self.zoom_percent as f32 / 100.0;
                    let target_y =
                        (24.0 + (self.active_slide as f32) * (461.25 * scale + 88.0)).max(0.0);
                    let preview_task = iced::widget::operation::scroll_to(
                        iced::widget::Id::new("live_preview_scrollable"),
                        iced::widget::scrollable::AbsoluteOffset {
                            x: Some(0.0),
                            y: Some(target_y),
                        },
                    );
                    task = Task::batch(vec![preview_task, sidebar_task]);
                } else if self.mode == EditorMode::SourceMode
                    && let Some(slide) = self.engine.slides.get(self.active_slide)
                {
                    let target_line = self.doc.source_text
                        [..slide.range.start.min(self.doc.source_text.len())]
                        .lines()
                        .count()
                        .saturating_sub(1);
                    self.source_content
                        .move_to(iced::advanced::text::editor::Cursor {
                            position: iced::advanced::text::editor::Position {
                                line: target_line,
                                column: 0,
                            },
                            selection: Some(iced::advanced::text::editor::Position {
                                line: target_line,
                                column: usize::MAX,
                            }),
                        });
                    let focus_task =
                        iced::widget::operation::focus(iced::widget::Id::new("source_mode_editor"));
                    task = Task::batch(vec![focus_task, sidebar_task]);
                } else {
                    task = Task::batch(vec![task, sidebar_task]);
                }
            },
            | Message::ToggleSlideViewMode(idx) => {
                if self.slide_view_vector.contains(&idx) {
                    self.slide_view_vector.remove(&idx);
                } else {
                    self.slide_view_vector.insert(idx);
                }
            },
            | Message::ToggleInPlaceEdit(idx) => {
                self.commit_active_block();
                if self.in_place_editing_slide == Some(idx) {
                    self.in_place_editing_slide = None;
                } else {
                    self.in_place_editing_slide = Some(idx);
                    self.active_slide = idx;
                    let chunks = self.doc.get_slide_chunks();
                    let chunk_text = chunks.get(idx).cloned().unwrap_or_default();
                    self.in_place_content = text_editor::Content::with_text(&chunk_text);
                }
            },
            | Message::InPlaceEditorAction(action) => {
                let is_edit = action.is_edit();
                self.in_place_content.perform(action);
                if is_edit && let Some(slide_idx) = self.in_place_editing_slide {
                    let new_text = self.in_place_content.text();
                    self.doc.update_slide_chunk(slide_idx, &new_text);
                    self.engine.source_text = self.doc.source_text.clone();
                    self.engine.reparse();
                    task = self.trigger_recompile_task();
                }
            },
            | Message::FocusSlideEditorAction(action) => {
                let is_edit = action.is_edit();
                self.focus_slide_content.perform(action);
                let slide_text = self.focus_slide_content.text();

                // Math formula hover pre-render
                let cursor_line = self.focus_slide_content.cursor().position.line;
                if let Some(line) = slide_text.lines().nth(cursor_line) {
                    if let Some(formula) =
                        crate::model::ast_engine::extract_math_formula_from_line(line)
                    {
                        if !self.equation_images.contains_key(&formula)
                            && let Some(handle) = self
                                .compiler
                                .render_equation_to_image(&formula, self.theme.is_dark())
                        {
                            self.equation_images.insert(formula.clone(), handle);
                        }
                        self.active_hover_formula = Some(formula);
                    } else {
                        self.active_hover_formula = None;
                    }
                } else {
                    self.active_hover_formula = None;
                }

                if is_edit {
                    self.doc.update_slide_chunk(self.active_slide, &slide_text);
                    self.engine.source_text = self.doc.source_text.clone();
                    self.engine.reparse();
                    task = self.trigger_recompile_task();
                }
            },
            | Message::SourceEditorAction(action) => {
                let is_edit = action.is_edit();
                self.source_content.perform(action);
                let full_source = self.source_content.text();

                // Math formula hover pre-render
                let cursor_line = self.source_content.cursor().position.line;
                if let Some(line) = full_source.lines().nth(cursor_line) {
                    if let Some(formula) =
                        crate::model::ast_engine::extract_math_formula_from_line(line)
                    {
                        if !self.equation_images.contains_key(&formula)
                            && let Some(handle) = self
                                .compiler
                                .render_equation_to_image(&formula, self.theme.is_dark())
                        {
                            self.equation_images.insert(formula.clone(), handle);
                        }
                        self.active_hover_formula = Some(formula);
                    } else {
                        self.active_hover_formula = None;
                    }
                } else {
                    self.active_hover_formula = None;
                }

                if is_edit {
                    self.doc.source_text = full_source;
                    self.doc.is_dirty = true;
                    self.doc.sync_chunks_from_source();
                    self.engine.source_text = self.doc.source_text.clone();
                    self.engine.reparse();
                    task = self.trigger_recompile_task();
                }
            },

            // Focus mode jump navigation messages
            | Message::PreviewHoverRatio(ratio) => {
                self.preview_hover_ratio = ratio;
            },
            | Message::JumpToCodeFromPreview => {
                if let Some(slide) = self.engine.slides.get(self.active_slide) {
                    let content_blocks: Vec<(usize, &WysiwygBlock)> = slide
                        .blocks
                        .iter()
                        .enumerate()
                        .filter(|(_, b)| b.is_content_element())
                        .collect();
                    if !content_blocks.is_empty() {
                        let idx = ((self.preview_hover_ratio * content_blocks.len() as f32).floor()
                            as usize)
                            .min(content_blocks.len().saturating_sub(1));
                        let (b_idx, _) = content_blocks[idx];
                        return self.update(Message::JumpToFocusBlock {
                            slide_idx: self.active_slide,
                            block_idx: b_idx,
                        });
                    }
                }
                let total_lines = self.focus_slide_content.line_count();
                if total_lines > 0 {
                    let target_line = ((self.preview_hover_ratio * total_lines as f32).floor()
                        as usize)
                        .min(total_lines.saturating_sub(1));
                    self.focus_slide_content
                        .move_to(iced::advanced::text::editor::Cursor {
                            position: iced::advanced::text::editor::Position {
                                line: target_line,
                                column: 0,
                            },
                            selection: Some(iced::advanced::text::editor::Position {
                                line: target_line,
                                column: usize::MAX,
                            }),
                        });
                    task =
                        iced::widget::operation::focus(iced::widget::Id::new("focus_mode_editor"));
                }
            },
            | Message::JumpToBlockCode(raw) => {
                let text = self.focus_slide_content.text();
                let trimmed_raw = raw.trim();
                let first_line_raw = trimmed_raw.lines().next().unwrap_or("").trim();

                let mut target_line = None;

                // 1. Try finding exact full block match in the slide text
                if !trimmed_raw.is_empty() {
                    for (idx, line) in text.lines().enumerate() {
                        let trimmed_line = line.trim();
                        if trimmed_line == trimmed_raw {
                            target_line = Some(idx);
                            break;
                        }
                    }
                }

                // 2. Try finding exact first line match
                if target_line.is_none() && !first_line_raw.is_empty() {
                    for (idx, line) in text.lines().enumerate() {
                        let trimmed_line = line.trim();
                        if trimmed_line == first_line_raw {
                            target_line = Some(idx);
                            break;
                        }
                    }
                }

                // 3. Try line containing first_line_raw or vice-versa (with at least 4 chars)
                if target_line.is_none() && first_line_raw.len() >= 4 {
                    for (idx, line) in text.lines().enumerate() {
                        let trimmed_line = line.trim();
                        if trimmed_line.contains(first_line_raw)
                            || first_line_raw.contains(trimmed_line)
                        {
                            target_line = Some(idx);
                            break;
                        }
                    }
                }

                // 4. Try semantic text match without Typst prefix symbols (=, -, +, $, #, /, `)
                if target_line.is_none() {
                    let clean_needle = first_line_raw
                        .trim_start_matches(['=', '-', '+', '$', '#', '/', '`'])
                        .trim();
                    if clean_needle.len() >= 3 {
                        for (idx, line) in text.lines().enumerate() {
                            let clean_line = line
                                .trim()
                                .trim_start_matches(['=', '-', '+', '$', '#', '/', '`'])
                                .trim();
                            if clean_line == clean_needle || clean_line.contains(clean_needle) {
                                target_line = Some(idx);
                                break;
                            }
                        }
                    }
                }

                let target_line = target_line.unwrap_or(0);
                let block_line_count = trimmed_raw.lines().count().max(1);
                let end_line = target_line + block_line_count.saturating_sub(1);

                self.focus_slide_content
                    .move_to(iced::advanced::text::editor::Cursor {
                        position: iced::advanced::text::editor::Position {
                            line: target_line,
                            column: 0,
                        },
                        selection: Some(iced::advanced::text::editor::Position {
                            line: end_line,
                            column: usize::MAX,
                        }),
                    });
                task = iced::widget::operation::focus(iced::widget::Id::new("focus_mode_editor"));
            },
            | Message::JumpToFocusBlock { slide_idx, block_idx } => {
                if let Some(slide) = self.engine.slides.get(slide_idx)
                    && let Some(block) = slide.blocks.get(block_idx)
                {
                    let slide_text = self.focus_slide_content.text();
                    let slide_start = slide.range.start;
                    let block_range = block.range();

                    let rel_start = block_range
                        .start
                        .saturating_sub(slide_start)
                        .min(slide_text.len());
                    let raw_end = block_range
                        .end
                        .saturating_sub(slide_start)
                        .min(slide_text.len());
                    let block_slice = &slide_text[rel_start..raw_end];
                    let trimmed_len = block_slice.trim_end().len();
                    let rel_end = rel_start + trimmed_len;

                    let start_line = slide_text[..rel_start]
                        .bytes()
                        .filter(|&b| b == b'\n')
                        .count();
                    let end_line = slide_text[..rel_end]
                        .bytes()
                        .filter(|&b| b == b'\n')
                        .count();

                    self.focus_slide_content
                        .move_to(iced::advanced::text::editor::Cursor {
                            position: iced::advanced::text::editor::Position {
                                line: start_line,
                                column: 0,
                            },
                            selection: Some(iced::advanced::text::editor::Position {
                                line: end_line,
                                column: usize::MAX,
                            }),
                        });
                    task =
                        iced::widget::operation::focus(iced::widget::Id::new("focus_mode_editor"));
                }
            },
            | Message::JumpToLine(line) => {
                let target_line = line.saturating_sub(1);
                let slide_text = self.focus_slide_content.text();

                // Find if any AST content block in the active slide contains target_line
                let mut block_bounds = None;
                if let Some(slide) = self.engine.slides.get(self.active_slide) {
                    let slide_start = slide.range.start;
                    for block in &slide.blocks {
                        if !block.is_content_element() {
                            continue;
                        }
                        let b_start = block
                            .range()
                            .start
                            .saturating_sub(slide_start)
                            .min(slide_text.len());
                        let raw_end = block
                            .range()
                            .end
                            .saturating_sub(slide_start)
                            .min(slide_text.len());
                        let block_slice = &slide_text[b_start..raw_end];
                        let trimmed_len = block_slice.trim_end().len();
                        let b_end = b_start + trimmed_len;

                        let b_start_line = slide_text[..b_start]
                            .bytes()
                            .filter(|&b| b == b'\n')
                            .count();
                        let b_end_line =
                            slide_text[..b_end].bytes().filter(|&b| b == b'\n').count();
                        if target_line >= b_start_line && target_line <= b_end_line {
                            block_bounds = Some((b_start_line, b_end_line));
                            break;
                        }
                    }
                }

                let (start_line, end_line) = block_bounds.unwrap_or((target_line, target_line));
                self.focus_slide_content
                    .move_to(iced::advanced::text::editor::Cursor {
                        position: iced::advanced::text::editor::Position {
                            line: start_line,
                            column: 0,
                        },
                        selection: Some(iced::advanced::text::editor::Position {
                            line: end_line,
                            column: usize::MAX,
                        }),
                    });
                task = iced::widget::operation::focus(iced::widget::Id::new("focus_mode_editor"));
            },

            // WYSIWYG In-Place Block Messages
            | Message::ActivateBlock {
                slide_idx,
                id,
                range,
                raw,
            } => {
                if self.active_block_id.is_some() {
                    self.commit_active_block();
                    self.trigger_recompile();
                }
                self.active_slide = slide_idx;
                self.active_block_id = Some(id);
                self.active_block_range = Some(range);
                self.active_block_content = text_editor::Content::with_text(&raw);
                task = iced::widget::operation::focus(iced::widget::Id::new("active_block_editor"));
            },
            | Message::DeactivateBlock => {
                self.commit_active_block();
                self.sync_editors_from_doc();
                self.trigger_recompile();
            },
            | Message::ToggleBlockRawCode(id) => {
                self.active_modal = None;
                if self.raw_code_blocks.contains(&id) {
                    self.raw_code_blocks.remove(&id);
                } else {
                    self.raw_code_blocks.insert(id.clone());
                    for (s_idx, slide) in self.engine.slides.iter().enumerate() {
                        for block in &slide.blocks {
                            if block.id() == id {
                                self.active_slide = s_idx;
                                self.active_block_id = Some(id.clone());
                                self.active_block_range = Some(block.range());
                                self.active_block_content =
                                    text_editor::Content::with_text(block.raw());
                                task = iced::widget::operation::focus(iced::widget::Id::new(
                                    "active_block_editor",
                                ));
                                break;
                            }
                        }
                    }
                }
            },
            | Message::ActiveBlockAction(action) => {
                let is_edit = action.is_edit();
                self.active_block_content.perform(action);
                if is_edit && let Some(range) = self.active_block_range.clone() {
                    let new_text = self.active_block_content.text();
                    if let Some(new_range) =
                        self.engine.update_block_at_range_unparsed(range, &new_text)
                    {
                        self.active_block_range = Some(new_range);
                        self.doc.source_text = self.engine.source_text.clone();
                        self.doc.is_dirty = true;
                        self.doc.sync_chunks_from_source();
                        task = self.trigger_recompile_task();
                    }
                }
            },
            | Message::UpdateBlockRange {
                slide_idx,
                range,
                new_text,
            } => {
                if let Some(s_idx) = slide_idx {
                    self.active_slide = s_idx;
                }
                if let Some(new_range) = self.engine.update_block_at_range(range, &new_text) {
                    if self.active_block_id.is_some() {
                        self.active_block_range = Some(new_range);
                        self.active_block_content = text_editor::Content::with_text(&new_text);
                    }
                    self.doc.source_text = self.engine.source_text.clone();
                    self.doc.is_dirty = true;
                    self.doc.sync_chunks_from_source();
                    self.sync_editors_from_doc();
                    self.trigger_recompile();
                }
            },
            | Message::InsertBlockAfter { offset, kind } => {
                self.commit_active_block();
                self.engine.insert_block_after(offset, kind.template());
                self.doc.source_text = self.engine.source_text.clone();
                self.doc.is_dirty = true;
                self.doc.sync_chunks_from_source();
                self.sync_editors_from_doc();
                self.trigger_recompile();
            },
            | Message::DeleteBlockAtRange(range) => {
                self.active_block_id = None;
                self.active_block_range = None;
                self.active_modal = None;
                if range.start <= range.end && range.end <= self.engine.source_text.len() {
                    self.engine.source_text.drain(range);
                    self.engine.reparse();
                    self.doc.source_text = self.engine.source_text.clone();
                    self.doc.is_dirty = true;
                    self.doc.sync_chunks_from_source();
                    self.sync_editors_from_doc();
                    self.trigger_recompile();
                }
            },
            | Message::FormatBlock(fmt) => {
                let (prefix, suffix) = match fmt {
                    | FormatAction::Bold => ("*", "*"),
                    | FormatAction::Italic => ("_", "_"),
                    | FormatAction::Monospace => ("`", "`"),
                    | FormatAction::Equation => ("$", "$"),
                };
                if let Some(range) = self.active_block_range.clone() {
                    let current = self.active_block_content.text();
                    let updated = format!("{current} {prefix}text{suffix}");
                    if let Some(new_range) = self.engine.update_block_at_range(range, &updated) {
                        self.active_block_range = Some(new_range);
                        self.active_block_content = text_editor::Content::with_text(&updated);
                        self.doc.source_text = self.engine.source_text.clone();
                        self.doc.is_dirty = true;
                        self.doc.sync_chunks_from_source();
                        task = self.trigger_recompile_task();
                    }
                }
            },
            | Message::StartDragBlock { slide_idx, block_idx } => {
                self.dragging_block = Some((slide_idx, block_idx, f32::NAN, false));
            },
            | Message::DragBlockY {
                slide_idx,
                block_idx,
                y,
            } => {
                return self.handle_drag_move(slide_idx, block_idx, y);
            },
            | Message::GlobalCursorMoved(position) => {
                self.last_cursor_pos = Some(position);
                if let Some((slide_idx, block_idx, _, _)) = self.dragging_block {
                    return self.handle_drag_move(slide_idx, block_idx, position.y);
                } else if let Some((slide_idx, block_idx, _)) = self.dragging_spacing {
                    return self.handle_spacing_drag(slide_idx, block_idx, position.y);
                }
            },
            | Message::WindowResized(size) => {
                self.window_size = size;
            },
            | Message::EndDragBlock => {
                self.dragging_block = None;
            },
            | Message::EndBlockInteraction {
                slide_idx,
                id,
                range,
                raw,
                ..
            } => {
                let drag_state = self.dragging_block.take();
                if let Some((_, _, _, true)) = drag_state {
                    return Task::none();
                }
                return self.update(Message::ActivateBlock {
                    slide_idx,
                    id,
                    range,
                    raw,
                });
            },
            | Message::GlobalButtonReleased => {
                if let Some((slide_idx, block_idx, _, has_moved)) = self.dragging_block.take() {
                    if !has_moved && let Some(slide) = self.engine.slides.get(slide_idx) {
                        let content_blocks: Vec<&WysiwygBlock> = slide
                            .blocks
                            .iter()
                            .filter(|b| b.is_content_element())
                            .collect();
                        if let Some(block) = content_blocks.get(block_idx) {
                            return self.update(Message::ActivateBlock {
                                slide_idx,
                                id: block.id().to_string(),
                                range: block.range(),
                                raw: block.raw().to_string(),
                            });
                        }
                    }
                    return Task::none();
                }
                if self.dragging_spacing.take().is_some() {
                    return Task::none();
                }
            },
            | Message::StartDragSpacing { slide_idx, block_idx } => {
                self.dragging_spacing = Some((slide_idx, block_idx, f32::NAN));
            },
            | Message::DragSpacingY {
                slide_idx,
                block_idx,
                y,
            } => {
                return self.handle_spacing_drag(slide_idx, block_idx, y);
            },
            | Message::EndDragSpacing => {
                self.dragging_spacing = None;
            },
            | Message::MoveBlockUp { slide_idx, block_idx } => {
                self.commit_active_block();
                if block_idx > 0
                    && let Some(slide) = self.engine.slides.get(slide_idx)
                {
                    let content_blocks: Vec<&WysiwygBlock> = slide
                        .blocks
                        .iter()
                        .filter(|b| b.is_content_element())
                        .collect();
                    if block_idx < content_blocks.len() {
                        let prev_block = content_blocks[block_idx - 1];
                        let curr_block = content_blocks[block_idx];
                        let r_prev = prev_block.range();
                        let r_curr = curr_block.range();
                        if r_prev.start < r_curr.end
                            && r_curr.end <= self.engine.source_text.len()
                            && r_prev.start >= slide.range.start
                            && r_curr.end <= slide.range.end
                        {
                            let sep = if r_prev.end <= r_curr.start {
                                &self.engine.source_text[r_prev.end..r_curr.start]
                            } else {
                                "\n\n"
                            };
                            // STRICT BOUNDARY CHECK: Only swap if separator between blocks is whitespace / comments.
                            // Must NEVER cross ')' or '[' or slide macro boundaries!
                            if sep
                                .chars()
                                .all(|c| c.is_whitespace() || c == '/' || c == '*')
                            {
                                let prev_raw = prev_block.raw().to_string();
                                let curr_raw = curr_block.raw().to_string();
                                let replacement = format!("{}{}{}", curr_raw, sep, prev_raw);
                                self.engine
                                    .source_text
                                    .replace_range(r_prev.start..r_curr.end, &replacement);
                                self.engine.reparse();
                                self.doc.source_text = self.engine.source_text.clone();
                                self.doc.is_dirty = true;
                                self.doc.sync_chunks_from_source();
                                self.sync_editors_from_doc();
                                task = self.trigger_recompile_task();
                            }
                        }
                    }
                }
            },
            | Message::MoveBlockDown { slide_idx, block_idx } => {
                self.commit_active_block();
                if let Some(slide) = self.engine.slides.get(slide_idx) {
                    let content_blocks: Vec<&WysiwygBlock> = slide
                        .blocks
                        .iter()
                        .filter(|b| b.is_content_element())
                        .collect();
                    if block_idx + 1 < content_blocks.len() {
                        let curr_block = content_blocks[block_idx];
                        let next_block = content_blocks[block_idx + 1];
                        let r_curr = curr_block.range();
                        let r_next = next_block.range();
                        if r_curr.start < r_next.end
                            && r_next.end <= self.engine.source_text.len()
                            && r_curr.start >= slide.range.start
                            && r_next.end <= slide.range.end
                        {
                            let sep = if r_curr.end <= r_next.start {
                                &self.engine.source_text[r_curr.end..r_next.start]
                            } else {
                                "\n\n"
                            };
                            // STRICT BOUNDARY CHECK: Only swap if separator between blocks is whitespace / comments.
                            // Must NEVER cross ')' or '[' or slide macro boundaries!
                            if sep
                                .chars()
                                .all(|c| c.is_whitespace() || c == '/' || c == '*')
                            {
                                let curr_raw = curr_block.raw().to_string();
                                let next_raw = next_block.raw().to_string();
                                let replacement = format!("{}{}{}", next_raw, sep, curr_raw);
                                self.engine
                                    .source_text
                                    .replace_range(r_curr.start..r_next.end, &replacement);
                                self.engine.reparse();
                                self.doc.source_text = self.engine.source_text.clone();
                                self.doc.is_dirty = true;
                                self.doc.sync_chunks_from_source();
                                self.sync_editors_from_doc();
                                task = self.trigger_recompile_task();
                            }
                        }
                    }
                }
            },
            | Message::AdjustBlockSpacing {
                slide_idx,
                block_idx,
                delta_pt,
            } => {
                self.commit_active_block();
                if let Some(slide) = self.engine.slides.get(slide_idx) {
                    let content_blocks: Vec<&WysiwygBlock> = slide
                        .blocks
                        .iter()
                        .filter(|b| b.is_content_element())
                        .collect();
                    if block_idx < content_blocks.len() {
                        let curr_block = content_blocks[block_idx];
                        let curr_end = curr_block.range().end;
                        let next_start = if block_idx + 1 < content_blocks.len() {
                            content_blocks[block_idx + 1].range().start
                        } else {
                            self.engine.get_slide_content_insert_offset(slide_idx)
                        };
                        let gap = if curr_end <= next_start
                            && next_start <= self.engine.source_text.len()
                        {
                            &self.engine.source_text[curr_end..next_start]
                        } else {
                            ""
                        };
                        if let Some(rel_v_start) = gap.find("#v(")
                            && let Some(rel_v_end) = gap[rel_v_start..].find(')')
                        {
                            let v_start = curr_end + rel_v_start;
                            let v_end = v_start + rel_v_end + 1;
                            let inside = self.engine.source_text[v_start + 3..v_end - 1].trim();
                            let current_pt = parse_length_to_pt(inside).unwrap_or(12.0) as i32;
                            if delta_pt == -999 || (current_pt + delta_pt) <= 0 {
                                // Cleanly remove the #v(...)
                                self.engine.source_text.drain(v_start..v_end);
                            } else {
                                let new_pt = (current_pt + delta_pt).max(0);
                                let replacement = format!("#v({new_pt}pt)");
                                self.engine
                                    .source_text
                                    .replace_range(v_start..v_end, &replacement);
                            }
                            self.engine.reparse();
                            self.doc.source_text = self.engine.source_text.clone();
                            self.doc.is_dirty = true;
                            self.doc.sync_chunks_from_source();
                            self.sync_editors_from_doc();
                            task = self.trigger_recompile_task();
                        } else if delta_pt > 0 && delta_pt != -999 {
                            let spacing_snippet = format!("\n#v({delta_pt}pt)");
                            self.engine
                                .source_text
                                .insert_str(curr_end, &spacing_snippet);
                            self.engine.reparse();
                            self.doc.source_text = self.engine.source_text.clone();
                            self.doc.is_dirty = true;
                            self.doc.sync_chunks_from_source();
                            self.sync_editors_from_doc();
                            task = self.trigger_recompile_task();
                        }
                    }
                }
            },

            | Message::InsertSlide => {
                self.push_undo_snapshot();
                self.commit_active_block();
                self.doc.add_new_slide();
                self.engine.source_text = self.doc.source_text.clone();
                self.engine.reparse();
                self.active_slide = self.doc.total_slides().saturating_sub(1);
                self.sync_editors_from_doc();
                self.trigger_recompile();
            },
            | Message::InsertSlideAt(target_idx) => {
                self.push_undo_snapshot();
                self.commit_active_block();
                self.doc.insert_slide_at(target_idx);
                self.engine.source_text = self.doc.source_text.clone();
                self.engine.reparse();
                self.active_slide = target_idx.min(self.doc.total_slides().saturating_sub(1));
                self.active_modal = None;
                self.sync_editors_from_doc();
                self.trigger_recompile();
            },
            | Message::DuplicateSlide(idx) => {
                self.push_undo_snapshot();
                self.commit_active_block();
                self.doc.duplicate_slide(idx);
                self.engine.source_text = self.doc.source_text.clone();
                self.engine.reparse();
                self.active_slide = (idx + 1).min(self.doc.total_slides().saturating_sub(1));
                self.active_modal = None;
                self.sync_editors_from_doc();
                self.trigger_recompile();
            },
            | Message::MoveSlide { from_idx, to_idx } => {
                self.push_undo_snapshot();
                self.commit_active_block();
                self.doc.move_slide(from_idx, to_idx);
                self.engine.source_text = self.doc.source_text.clone();
                self.engine.reparse();
                self.active_slide = to_idx.min(self.doc.total_slides().saturating_sub(1));
                self.active_modal = None;
                self.sync_editors_from_doc();
                self.trigger_recompile();
            },
            | Message::MoveActiveSlideUp => {
                if self.active_slide > 0 {
                    self.push_undo_snapshot();
                    self.commit_active_block();
                    let target = self.active_slide.saturating_sub(1);
                    self.doc.move_slide(self.active_slide, target);
                    self.engine.source_text = self.doc.source_text.clone();
                    self.engine.reparse();
                    self.active_slide = target;
                    self.sync_editors_from_doc();
                    self.trigger_recompile();
                }
            },
            | Message::MoveActiveSlideDown => {
                let next = self.active_slide.saturating_add(1);
                if next < self.doc.total_slides() {
                    self.push_undo_snapshot();
                    self.commit_active_block();
                    self.doc.move_slide(self.active_slide, next);
                    self.engine.source_text = self.doc.source_text.clone();
                    self.engine.reparse();
                    self.active_slide = next;
                    self.sync_editors_from_doc();
                    self.trigger_recompile();
                }
            },
            | Message::DuplicateActiveSlide => {
                self.push_undo_snapshot();
                self.commit_active_block();
                self.doc.duplicate_slide(self.active_slide);
                self.engine.source_text = self.doc.source_text.clone();
                self.engine.reparse();
                self.active_slide =
                    (self.active_slide + 1).min(self.doc.total_slides().saturating_sub(1));
                self.sync_editors_from_doc();
                self.trigger_recompile();
            },
            | Message::InsertSlideAfterActive => {
                self.push_undo_snapshot();
                self.commit_active_block();
                let target = self.active_slide + 1;
                self.doc.insert_slide_at(target);
                self.engine.source_text = self.doc.source_text.clone();
                self.engine.reparse();
                self.active_slide = target.min(self.doc.total_slides().saturating_sub(1));
                self.sync_editors_from_doc();
                self.trigger_recompile();
            },
            | Message::Undo => {
                if let Some(snapshot) = self.undo_stack.pop() {
                    self.redo_stack.push(DocumentSnapshot {
                        source_text: self.doc.source_text.clone(),
                        active_slide: self.active_slide,
                    });
                    self.commit_active_block();
                    self.doc.source_text = snapshot.source_text;
                    self.doc.sync_chunks_from_source();
                    self.engine.source_text = self.doc.source_text.clone();
                    self.engine.reparse();
                    self.active_slide = snapshot
                        .active_slide
                        .min(self.doc.total_slides().saturating_sub(1));
                    self.sync_editors_from_doc();
                    self.trigger_recompile();
                }
            },
            | Message::Redo => {
                if let Some(snapshot) = self.redo_stack.pop() {
                    self.undo_stack.push(DocumentSnapshot {
                        source_text: self.doc.source_text.clone(),
                        active_slide: self.active_slide,
                    });
                    self.commit_active_block();
                    self.doc.source_text = snapshot.source_text;
                    self.doc.sync_chunks_from_source();
                    self.engine.source_text = self.doc.source_text.clone();
                    self.engine.reparse();
                    self.active_slide = snapshot
                        .active_slide
                        .min(self.doc.total_slides().saturating_sub(1));
                    self.sync_editors_from_doc();
                    self.trigger_recompile();
                }
            },
            | Message::OpenTemplateLibraryModal => {
                self.active_modal = Some(ActiveModal::TemplateLibrary);
            },
            | Message::InsertTemplateSlide(idx) => {
                if let Some(&template_code) = TEMPLATE_SOURCES.get(idx) {
                    self.push_undo_snapshot();
                    self.commit_active_block();
                    let target_idx = (self.active_slide + 1).min(self.doc.total_slides());
                    self.doc
                        .slide_chunks
                        .insert(target_idx, template_code.to_string());
                    self.doc.rebuild_source_from_chunks();
                    self.doc.is_dirty = true;
                    self.engine.source_text = self.doc.source_text.clone();
                    self.engine.reparse();
                    self.active_slide = target_idx.min(self.doc.total_slides().saturating_sub(1));
                    self.active_modal = None;
                    self.sync_editors_from_doc();
                    self.trigger_recompile();
                }
            },
            | Message::OpenPresentationHealthModal => {
                let deck = self.doc.deck.clone().unwrap_or_default();
                let issues = slide_core::compiler::check_presentation_health(
                    &deck,
                    &self.doc.source_text,
                    self.doc.assets_dir.as_deref(),
                );
                let pacing_report = deck.pacing_report(&self.doc.slide_chunks);
                self.active_modal = Some(ActiveModal::PresentationHealth(issues, pacing_report));
            },
            | Message::ApplyQuickFix(qf) => {
                self.push_undo_snapshot();
                if let Some(ref prepend) = qf.prepend_text {
                    self.doc.source_text = format!("{}{}", prepend, self.doc.source_text);
                }
                if let Some((ref from, ref to)) = qf.replace_pair {
                    self.doc.source_text = self.doc.source_text.replacen(from, to, 1);
                }
                self.doc.sync_chunks_from_source();
                self.doc.is_dirty = true;
                self.engine.source_text = self.doc.source_text.clone();
                self.engine.reparse();
                self.active_modal = None;
                self.sync_editors_from_doc();
                self.trigger_recompile();
            },
            | Message::OpenRecentFile(path_str) => {
                let p = PathBuf::from(&path_str);
                let resolved = if p.is_absolute() {
                    p.clone()
                } else {
                    std::env::current_dir().unwrap_or_default().join(&p)
                };
                self.commit_active_block();
                match EditorDocument::open(&resolved) {
                    | Ok(new_doc) => {
                        self.push_undo_snapshot();
                        save_recent_editor_file(&resolved);
                        self.recent_files = load_recent_editor_files();
                        self.doc = new_doc;
                        self.engine =
                            TypstDocumentEngine::from_source(self.doc.source_text.clone());
                        self.active_slide = 0;
                        self.active_block_id = None;
                        self.active_block_range = None;
                        self.in_place_editing_slide = None;
                        self.active_modal = None;
                        self.slide_images.clear();
                        self.equation_images.clear();
                        self.code_images.clear();
                        self.sync_editors_from_doc();
                        let has_editable_source = self.doc.has_editable_source()
                            || !self.doc.source_text.trim().is_empty();
                        if self.doc.format == DocumentFormat::SlidePackage && !has_editable_source {
                            self.compilation_status = CompilationStatus::Ready;
                            self.slide_view_vector = (0..self.doc.total_slides()).collect();
                            self.update_slide_cache();
                        } else {
                            self.slide_view_vector.clear();
                            task = self.trigger_recompile_task();
                        }
                    },
                    | Err(e) => {
                        self.open_error = Some(format!(
                            "Failed to open recent presentation '{path_str}': {e}"
                        ));
                    },
                }
            },
            | Message::ToggleSidebarViewMode => {
                self.sidebar_view_mode = match self.sidebar_view_mode {
                    | SidebarViewMode::Outline => SidebarViewMode::Thumbnails,
                    | SidebarViewMode::Thumbnails => SidebarViewMode::Outline,
                };
            },
            | Message::OpenSlideContextMenu(slide_idx) => {
                self.active_modal = Some(ActiveModal::SlideContextMenu {
                    slide_idx,
                    position: self.last_cursor_pos,
                });
            },
            | Message::OpenBlockContextMenu {
                slide_idx,
                block_idx,
                range,
                block_label,
                block_id,
            } => {
                self.active_modal = Some(ActiveModal::BlockContextMenu {
                    slide_idx,
                    block_idx,
                    range,
                    block_label,
                    block_id,
                    position: self.last_cursor_pos,
                });
            },
            | Message::OpenMoveBlockModal {
                from_slide_idx,
                block_idx,
                range,
                block_label,
            } => {
                self.active_modal = Some(ActiveModal::MoveBlockToSlide {
                    from_slide_idx,
                    block_idx,
                    range,
                    block_label,
                });
            },
            | Message::MoveBlockToSlide {
                from_slide_idx,
                block_range,
                to_slide_idx,
            } => {
                self.commit_active_block();
                let success =
                    self.engine
                        .move_block_to_slide(from_slide_idx, block_range, to_slide_idx);
                if success {
                    self.doc.source_text = self.engine.source_text.clone();
                    self.doc.sync_chunks_from_source();
                    self.doc.is_dirty = true;
                    self.active_slide = to_slide_idx.min(self.doc.total_slides().saturating_sub(1));
                    self.sync_editors_from_doc();
                    self.trigger_recompile();
                }
                self.active_modal = None;
            },
            | Message::DuplicateBlock(range) => {
                self.commit_active_block();
                if let Some(text) = self.engine.source_text.get(range.clone()) {
                    let dup = format!("\n\n{}", text.trim());
                    self.engine.source_text.insert_str(range.end, &dup);
                    self.engine.reparse();
                    self.doc.source_text = self.engine.source_text.clone();
                    self.doc.sync_chunks_from_source();
                    self.doc.is_dirty = true;
                    self.sync_editors_from_doc();
                    self.trigger_recompile();
                }
                self.active_modal = None;
            },
            | Message::OpenFontModal => {
                self.font_search_query.clear();
                let curr_font =
                    slide_core::font::extract_font_families_from_typst(&self.doc.source_text)
                        .into_iter()
                        .next()
                        .unwrap_or_else(slide_core::font::detect_default_sans_font);
                self.active_modal = Some(ActiveModal::FontSelector {
                    search_query: String::new(),
                    current_font: curr_font,
                });
            },
            | Message::FontSearchQueryChanged(q) => {
                self.font_search_query = q.clone();
                if let Some(ActiveModal::FontSelector {
                    ref mut search_query, ..
                }) = self.active_modal
                {
                    *search_query = q;
                }
            },
            | Message::SelectFont(font_name) => {
                self.commit_active_block();
                if let Some(start_pos) = self.doc.source_text.find("#set text(font:") {
                    if let Some(end_quote) = self.doc.source_text[start_pos..].find(')') {
                        let full_end = start_pos + end_quote + 1;
                        let new_rule = format!("#set text(font: \"{font_name}\")");
                        self.doc
                            .source_text
                            .replace_range(start_pos..full_end, &new_rule);
                    }
                } else {
                    let insert_pos = if self.doc.source_text.starts_with("#import") {
                        self.doc.source_text.find('\n').map(|p| p + 1).unwrap_or(0)
                    } else {
                        0
                    };
                    self.doc
                        .source_text
                        .insert_str(insert_pos, &format!("#set text(font: \"{font_name}\")\n"));
                }
                self.engine.source_text = self.doc.source_text.clone();
                self.engine.reparse();
                self.doc.sync_chunks_from_source();
                self.doc.is_dirty = true;
                self.sync_editors_from_doc();
                self.active_modal = None;
                self.trigger_recompile();
            },
            | Message::OpenSearchModal => {
                self.search_state.is_visible = true;
                self.compute_search_matches();
                let matches = self.get_search_matches();
                let focus_task =
                    iced::widget::operation::focus(iced::widget::Id::new("floating_search_input"));
                if !matches.is_empty() {
                    let idx = self.search_state.current_match_idx.min(matches.len() - 1);
                    let (s, e) = matches[idx];
                    let jump_task = self.jump_to_and_select_match(s, e);
                    task = Task::batch(vec![focus_task, jump_task]);
                } else {
                    task = focus_task;
                }
            },
            | Message::CloseSearchBar => {
                self.search_state.is_visible = false;
            },
            | Message::SearchQueryChanged(q) => {
                self.search_state.search_query = q;
                self.compute_search_matches();
                let matches = self.get_search_matches();
                if !matches.is_empty() {
                    self.search_state.current_match_idx = 0;
                    self.search_state.status_message =
                        Some(format!("Match 1 of {}", matches.len()));
                    let (s, e) = matches[0];
                    task = self.jump_to_and_select_match(s, e);
                }
            },
            | Message::ReplaceQueryChanged(q) => {
                self.search_state.replace_query = q;
            },
            | Message::ToggleSearchRegex => {
                self.search_state.is_regex = !self.search_state.is_regex;
                self.compute_search_matches();
                let matches = self.get_search_matches();
                if !matches.is_empty() {
                    self.search_state.current_match_idx = 0;
                    self.search_state.status_message =
                        Some(format!("Match 1 of {}", matches.len()));
                    let (s, e) = matches[0];
                    task = self.jump_to_and_select_match(s, e);
                }
            },
            | Message::ToggleSearchCaseSensitive => {
                self.search_state.case_sensitive = !self.search_state.case_sensitive;
                self.compute_search_matches();
                let matches = self.get_search_matches();
                if !matches.is_empty() {
                    self.search_state.current_match_idx = 0;
                    self.search_state.status_message =
                        Some(format!("Match 1 of {}", matches.len()));
                    let (s, e) = matches[0];
                    task = self.jump_to_and_select_match(s, e);
                }
            },
            | Message::ToggleSearchWholeWord => {
                self.search_state.whole_word = !self.search_state.whole_word;
                self.compute_search_matches();
                let matches = self.get_search_matches();
                if !matches.is_empty() {
                    self.search_state.current_match_idx = 0;
                    self.search_state.status_message =
                        Some(format!("Match 1 of {}", matches.len()));
                    let (s, e) = matches[0];
                    task = self.jump_to_and_select_match(s, e);
                }
            },
            | Message::FindNextMatch => {
                let matches = self.get_search_matches();
                if !matches.is_empty() {
                    self.search_state.current_match_idx =
                        (self.search_state.current_match_idx + 1) % matches.len();
                    self.search_state.status_message = Some(format!(
                        "Match {} of {}",
                        self.search_state.current_match_idx + 1,
                        matches.len()
                    ));
                    let (s, e) = matches[self.search_state.current_match_idx];
                    task = self.jump_to_and_select_match(s, e);
                }
            },
            | Message::FindPrevMatch => {
                let matches = self.get_search_matches();
                if !matches.is_empty() {
                    self.search_state.current_match_idx =
                        if self.search_state.current_match_idx == 0 {
                            matches.len() - 1
                        } else {
                            self.search_state.current_match_idx - 1
                        };
                    self.search_state.status_message = Some(format!(
                        "Match {} of {}",
                        self.search_state.current_match_idx + 1,
                        matches.len()
                    ));
                    let (s, e) = matches[self.search_state.current_match_idx];
                    task = self.jump_to_and_select_match(s, e);
                }
            },
            | Message::ExecuteReplaceCurrent => {
                let matches = self.get_search_matches();
                if !matches.is_empty() && self.search_state.current_match_idx < matches.len() {
                    let (start_byte, end_byte) = matches[self.search_state.current_match_idx];
                    let replacement = self.search_state.replace_query.clone();
                    self.doc
                        .source_text
                        .replace_range(start_byte..end_byte, &replacement);
                    self.doc.is_dirty = true;
                    self.doc.sync_chunks_from_source();
                    self.engine.source_text = self.doc.source_text.clone();
                    self.engine.reparse();
                    self.sync_editors_from_doc();
                    self.compute_search_matches();
                    let recompile_task = self.trigger_recompile_task();
                    let new_matches = self.get_search_matches();
                    if !new_matches.is_empty() {
                        let idx = self
                            .search_state
                            .current_match_idx
                            .min(new_matches.len() - 1);
                        let (s, e) = new_matches[idx];
                        let jump_task = self.jump_to_and_select_match(s, e);
                        task = Task::batch(vec![recompile_task, jump_task]);
                    } else {
                        task = recompile_task;
                    }
                }
            },
            | Message::ExecuteReplaceAllMatches => {
                let query = self.search_state.search_query.clone();
                let replace = self.search_state.replace_query.clone();
                if !query.is_empty() {
                    let matches = self.get_search_matches();
                    let count = matches.len();
                    if count > 0 {
                        let new_text = if self.search_state.is_regex {
                            if let Ok(re) = regex::RegexBuilder::new(&query)
                                .case_insensitive(!self.search_state.case_sensitive)
                                .build()
                            {
                                re.replace_all(&self.doc.source_text, replace.as_str())
                                    .to_string()
                            } else {
                                self.doc.source_text.clone()
                            }
                        } else if self.search_state.whole_word {
                            let pat = format!(r"\b{}\b", regex::escape(&query));
                            if let Ok(re) = regex::RegexBuilder::new(&pat)
                                .case_insensitive(!self.search_state.case_sensitive)
                                .build()
                            {
                                re.replace_all(&self.doc.source_text, replace.as_str())
                                    .to_string()
                            } else {
                                self.doc.source_text.clone()
                            }
                        } else if self.search_state.case_sensitive {
                            self.doc.source_text.replace(&query, &replace)
                        } else {
                            let pat = regex::escape(&query);
                            if let Ok(re) = regex::RegexBuilder::new(&pat)
                                .case_insensitive(true)
                                .build()
                            {
                                re.replace_all(&self.doc.source_text, replace.as_str())
                                    .to_string()
                            } else {
                                self.doc.source_text.clone()
                            }
                        };
                        self.doc.source_text = new_text;
                        self.doc.is_dirty = true;
                        self.doc.sync_chunks_from_source();
                        self.engine.source_text = self.doc.source_text.clone();
                        self.engine.reparse();
                        self.sync_editors_from_doc();
                        self.compute_search_matches();
                        self.search_state.status_message =
                            Some(format!("Replaced {count} occurrences"));
                        task = self.trigger_recompile_task();
                    } else {
                        self.search_state.status_message =
                            Some("0 occurrences found to replace".to_string());
                    }
                }
            },
            | Message::OpenHeaderFooterModal => {
                let cfg = crate::model::ast_engine::extract_header_footer_from_source(
                    &self.doc.source_text,
                );
                self.active_modal = Some(ActiveModal::HeaderFooter {
                    header_enabled: cfg.header_enabled,
                    header_left: cfg.header_left,
                    header_right: cfg.header_right,
                    footer_enabled: cfg.footer_enabled,
                    footer_left: cfg.footer_left,
                    footer_right_mode: cfg.footer_right_mode,
                    footer_right_custom: cfg.footer_right_custom,
                });
            },
            | Message::ToggleHeaderEnabled(enabled) => {
                if let Some(ActiveModal::HeaderFooter {
                    ref mut header_enabled,
                    ..
                }) = self.active_modal
                {
                    *header_enabled = enabled;
                }
            },
            | Message::HeaderLeftChanged(val) => {
                if let Some(ActiveModal::HeaderFooter {
                    ref mut header_left, ..
                }) = self.active_modal
                {
                    *header_left = val;
                }
            },
            | Message::HeaderRightChanged(val) => {
                if let Some(ActiveModal::HeaderFooter {
                    ref mut header_right, ..
                }) = self.active_modal
                {
                    *header_right = val;
                }
            },
            | Message::ToggleFooterEnabled(enabled) => {
                if let Some(ActiveModal::HeaderFooter {
                    ref mut footer_enabled,
                    ..
                }) = self.active_modal
                {
                    *footer_enabled = enabled;
                }
            },
            | Message::FooterLeftChanged(val) => {
                if let Some(ActiveModal::HeaderFooter {
                    ref mut footer_left, ..
                }) = self.active_modal
                {
                    *footer_left = val;
                }
            },
            | Message::FooterRightModeChanged(mode) => {
                if let Some(ActiveModal::HeaderFooter {
                    ref mut footer_right_mode,
                    ..
                }) = self.active_modal
                {
                    *footer_right_mode = mode;
                }
            },
            | Message::FooterRightCustomChanged(val) => {
                if let Some(ActiveModal::HeaderFooter {
                    ref mut footer_right_custom,
                    ..
                }) = self.active_modal
                {
                    *footer_right_custom = val;
                }
            },
            | Message::ApplyHeaderFooterSettings => {
                if let Some(ActiveModal::HeaderFooter {
                    header_enabled,
                    ref header_left,
                    ref header_right,
                    footer_enabled,
                    ref footer_left,
                    footer_right_mode,
                    ref footer_right_custom,
                }) = self.active_modal
                {
                    let cfg = crate::model::ast_engine::HeaderFooterConfig {
                        header_enabled,
                        header_left: header_left.clone(),
                        header_right: header_right.clone(),
                        footer_enabled,
                        footer_left: footer_left.clone(),
                        footer_right_mode,
                        footer_right_custom: footer_right_custom.clone(),
                    };
                    let updated = crate::model::ast_engine::update_header_footer_in_source(
                        &self.doc.source_text,
                        &cfg,
                    );
                    self.doc.source_text = updated;
                    self.doc.is_dirty = true;
                    self.doc.sync_chunks_from_source();
                    self.engine.source_text = self.doc.source_text.clone();
                    self.engine.reparse();
                    self.sync_editors_from_doc();
                    self.active_modal = None;
                    task = self.trigger_recompile_task();
                }
            },
            | Message::DeleteSlide(idx) => {
                self.push_undo_snapshot();
                self.commit_active_block();
                self.doc.delete_slide(idx);
                self.engine.source_text = self.doc.source_text.clone();
                self.engine.reparse();
                if self.active_slide >= self.doc.total_slides() {
                    self.active_slide = self.doc.total_slides().saturating_sub(1);
                }
                self.active_modal = None;
                self.sync_editors_from_doc();
                self.trigger_recompile();
            },
            | Message::UpdateSlideTitle { slide_idx, new_title } => {
                if let Some(slide) = self.engine.slides.get(slide_idx) {
                    let slide_raw = self
                        .engine
                        .source_text
                        .get(slide.range.clone())
                        .unwrap_or("");
                    if slide_raw.contains("#title-slide") || slide_raw.contains("title-slide(") {
                        let updated = crate::ui::wysiwyg::block_view::update_title_slide_param(
                            slide_raw, "title", &new_title,
                        );
                        self.engine
                            .source_text
                            .replace_range(slide.range.clone(), &updated);
                        self.engine.reparse();
                        self.doc.source_text = self.engine.source_text.clone();
                        self.doc.is_dirty = true;
                        self.doc.sync_chunks_from_source();
                        self.sync_editors_from_doc();
                        task = self.trigger_recompile_task();
                    } else if let Some(pos) = slide_raw.find("title:") {
                        let after = &slide_raw[pos + 6..];
                        if let Some(first_quote) = after.find('"') {
                            let s_quote = pos + 6 + first_quote;
                            if let Some(second_quote) = after[first_quote + 1..].find('"') {
                                let e_quote = s_quote + 1 + second_quote;
                                let abs_range = (slide.range.start + s_quote + 1)
                                    ..(slide.range.start + e_quote);
                                self.engine.source_text.replace_range(abs_range, &new_title);
                                self.engine.reparse();
                                self.doc.source_text = self.engine.source_text.clone();
                                self.doc.is_dirty = true;
                                self.doc.sync_chunks_from_source();
                                self.sync_editors_from_doc();
                                task = self.trigger_recompile_task();
                            }
                        }
                    } else if let Some(heading_pos) = slide_raw.find("= ") {
                        let after = &slide_raw[heading_pos + 2..];
                        let line_end = after.find('\n').unwrap_or(after.len());
                        let abs_range = (slide.range.start + heading_pos + 2)
                            ..(slide.range.start + heading_pos + 2 + line_end);
                        self.engine.source_text.replace_range(abs_range, &new_title);
                        self.engine.reparse();
                        self.doc.source_text = self.engine.source_text.clone();
                        self.doc.is_dirty = true;
                        self.doc.sync_chunks_from_source();
                        self.sync_editors_from_doc();
                        task = self.trigger_recompile_task();
                    }
                }
            },
            | Message::CompilationCompleted(res) => {
                self.compilation_in_flight = false;
                match res {
                    | Ok(deck) => {
                        self.doc.deck = Some(deck);
                        self.compilation_status = CompilationStatus::Ready;
                        self.update_slide_cache();
                    },
                    | Err(diag) => {
                        self.compilation_status = CompilationStatus::Error(diag);
                    },
                }
                if self.pending_recompile {
                    task = self.trigger_recompile_task();
                }
            },
            | Message::InsertSnippet(snippet) => {
                match self.mode {
                    | EditorMode::LivePreview => {
                        let offset = if let Some(ref range) = self.active_block_range {
                            range.end
                        } else {
                            self.engine
                                .get_slide_content_insert_offset(self.active_slide)
                        };
                        self.commit_active_block();
                        self.engine.insert_block_after(offset, snippet);
                        self.doc.source_text = self.engine.source_text.clone();
                        self.doc.is_dirty = true;
                        self.doc.sync_chunks_from_source();
                        self.sync_editors_from_doc();
                        task = self.trigger_recompile_task();
                    },
                    | EditorMode::FocusMode => {
                        let mut chunk = self.focus_slide_content.text();
                        chunk.push('\n');
                        chunk.push_str(snippet);
                        self.doc.update_slide_chunk(self.active_slide, &chunk);
                        self.engine.source_text = self.doc.source_text.clone();
                        self.engine.reparse();
                        self.focus_slide_content = text_editor::Content::with_text(&chunk);
                        task = self.trigger_recompile_task();
                    },
                    | EditorMode::SourceMode => {
                        let mut full = self.source_content.text();
                        full.push('\n');
                        full.push_str(snippet);
                        self.doc.source_text = full.clone();
                        self.doc.is_dirty = true;
                        self.doc.sync_chunks_from_source();
                        self.engine.source_text = full.clone();
                        self.engine.reparse();
                        self.source_content = text_editor::Content::with_text(&full);
                        task = self.trigger_recompile_task();
                    },
                }
            },
            | Message::ZoomIn => {
                if self.zoom_percent < 200 {
                    self.zoom_percent = self.zoom_percent.saturating_add(10);
                }
            },
            | Message::ZoomOut => {
                if self.zoom_percent > 50 {
                    self.zoom_percent = self.zoom_percent.saturating_sub(10);
                }
            },
            | Message::ResetZoom => {
                self.zoom_percent = 100;
            },
            | Message::CycleSpeakingPace => {
                self.speaking_wpm = match self.speaking_wpm {
                    | 100 => 130,
                    | 130 => 160,
                    | _ => 100,
                };
            },
            | Message::NewDocument => {
                self.commit_active_block();
                self.doc = EditorDocument::new_presentation("Untitled Presentation");
                self.engine = TypstDocumentEngine::from_source(self.doc.source_text.clone());
                self.active_slide = 0;
                self.in_place_editing_slide = None;
                self.sync_editors_from_doc();
                task = self.trigger_recompile_task();
            },
            | Message::OpenDocumentDialog => {
                self.open_path.clear();
                self.open_error = None;
                self.active_modal = Some(ActiveModal::Open);
            },
            | Message::BrowseOpenPath => {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("All Presentation Files", &["slide", "typ"])
                    .add_filter("Typst Documents (*.typ)", &["typ"])
                    .add_filter("Cargo Slide Packages (*.slide)", &["slide"])
                    .pick_file()
                {
                    self.open_path = file.to_string_lossy().to_string();
                }
            },
            | Message::OpenPathChanged(path) => {
                self.open_path = path;
                self.open_error = None;
            },
            | Message::ExecuteOpen => {
                let path = PathBuf::from(self.open_path.trim());
                if path.exists() {
                    match EditorDocument::open(&path) {
                        | Ok(new_doc) => {
                            self.commit_active_block();
                            save_recent_editor_file(&path);
                            self.recent_files = load_recent_editor_files();
                            self.doc = new_doc;
                            self.engine =
                                TypstDocumentEngine::from_source(self.doc.source_text.clone());
                            self.active_slide = 0;
                            self.in_place_editing_slide = None;
                            self.active_modal = None;
                            self.sync_editors_from_doc();
                            let has_editable_source = self.doc.has_editable_source()
                                || !self.doc.source_text.trim().is_empty();
                            if self.doc.format == DocumentFormat::SlidePackage
                                && !has_editable_source
                            {
                                self.compilation_status = CompilationStatus::Ready;
                                self.slide_view_vector = (0..self.doc.total_slides()).collect();
                                self.update_slide_cache();
                            } else {
                                self.slide_view_vector.clear();
                                task = self.trigger_recompile_task();
                            }
                        },
                        | Err(e) => {
                            self.open_error = Some(format!("Failed to open file: {e}"));
                        },
                    }
                } else {
                    self.open_error = Some("Specified file does not exist.".to_string());
                }
            },
            | Message::SaveDocument => {
                self.commit_active_block();
                if self.doc.file_path.is_none() {
                    self.open_export_with_format(match self.doc.format {
                        | DocumentFormat::Typst => ExportFormat::Pdf,
                        | DocumentFormat::SlidePackage => ExportFormat::SlidePackage,
                    });
                } else {
                    let _ = self.doc.save(None);
                    if let Some(ref p) = self.doc.file_path {
                        save_recent_editor_file(p);
                        self.recent_files = load_recent_editor_files();
                    }
                    task = self.trigger_recompile_task();
                }
            },
            | Message::OpenExportDialog => {
                self.commit_active_block();
                self.open_export_with_format(ExportFormat::Pdf);
            },
            | Message::SelectExportFormat(fmt) => {
                self.export_format = fmt;
                self.update_export_path_extension();
            },
            | Message::SelectExportPageSelection(sel) => {
                self.export_page_selection = sel;
            },
            | Message::ExportCustomRangeChanged(range) => {
                self.export_custom_range = range;
            },
            | Message::ToggleExportIncludeSource => {
                self.export_include_source = !self.export_include_source;
            },
            | Message::ExportPathChanged(path) => {
                self.export_path = path;
                self.export_status = None;
            },
            | Message::SelectPngScale(scale) => {
                self.export_png_scale = scale;
            },
            | Message::BrowseExportPath => {
                let ext = self.export_format.default_extension();
                let dialog = rfd::FileDialog::new().add_filter("Target Format", &[ext]);
                let file = match self.export_format {
                    | ExportFormat::Svg | ExportFormat::Png if self.engine.slides.len() > 1 => {
                        dialog.pick_folder()
                    },
                    | _ => dialog.save_file(),
                };
                if let Some(p) = file {
                    self.export_path = p.to_string_lossy().to_string();
                }
            },
            | Message::ExecuteExport => {
                self.commit_active_block();
                let target = PathBuf::from(self.export_path.trim());
                let result = match self.export_format {
                    | ExportFormat::Pdf => self.compiler.export_pdf(&self.doc, &target),
                    | ExportFormat::Svg => {
                        if let Some(ref deck) = self.doc.deck {
                            let total = deck.total_slides();
                            let pages = match self.export_page_selection {
                                | ExportPageSelection::All => None,
                                | ExportPageSelection::Current => Some(vec![self.active_slide + 1]),
                                | ExportPageSelection::Custom => {
                                    Some(CompilerBridge::parse_page_range(
                                        &self.export_custom_range,
                                        total,
                                    ))
                                },
                            };
                            CompilerBridge::export_svgs(deck, &target, pages.as_deref()).map(|_| ())
                        } else {
                            Err(slide_core::error::SlideError::Compilation(
                                "No compiled deck available".to_string(),
                            ))
                        }
                    },
                    | ExportFormat::Png => {
                        if let Some(ref deck) = self.doc.deck {
                            let total = deck.total_slides();
                            let pages = match self.export_page_selection {
                                | ExportPageSelection::All => None,
                                | ExportPageSelection::Current => Some(vec![self.active_slide + 1]),
                                | ExportPageSelection::Custom => {
                                    Some(CompilerBridge::parse_page_range(
                                        &self.export_custom_range,
                                        total,
                                    ))
                                },
                            };
                            CompilerBridge::export_all_pngs(
                                deck,
                                &target,
                                self.export_png_scale,
                                pages.as_deref(),
                            )
                            .map(|_| ())
                        } else {
                            Err(slide_core::error::SlideError::Compilation(
                                "No compiled deck available".to_string(),
                            ))
                        }
                    },
                    | ExportFormat::SlidePackage => {
                        self.compiler.export_slide_package(
                            &self.doc,
                            self.export_include_source,
                            &target,
                        )
                    },
                };

                match result {
                    | Ok(()) => {
                        self.export_status =
                            Some(format!("Successfully exported to {}", target.display()));
                    },
                    | Err(e) => {
                        self.export_status = Some(format!("Export failed: {e}"));
                    },
                }
            },
            | Message::PlayPresentation => {
                // 1. Commit active block
                self.commit_active_block();

                // 2. Commit in-place editing slide content
                if let Some(s_idx) = self.in_place_editing_slide {
                    let new_text = self.in_place_content.text();
                    self.doc.update_slide_chunk(s_idx, &new_text);
                    self.engine.source_text = self.doc.source_text.clone();
                    self.engine.reparse();
                }

                // 3. Commit focus mode slide content
                if self.mode == EditorMode::FocusMode {
                    let new_text = self.focus_slide_content.text();
                    self.doc.update_slide_chunk(self.active_slide, &new_text);
                    self.engine.source_text = self.doc.source_text.clone();
                    self.engine.reparse();
                }

                // 4. Commit source mode content
                if self.mode == EditorMode::SourceMode {
                    let full = self.source_content.text();
                    self.doc.source_text = full.clone();
                    self.doc.is_dirty = true;
                    self.doc.sync_chunks_from_source();
                    self.engine.source_text = full;
                    self.engine.reparse();
                }

                let is_light = self.theme.is_light();
                let file_path = self.doc.file_path.clone();
                if let Some(ref p) = file_path {
                    if self.doc.is_dirty {
                        let _ = self.doc.save(None);
                    }
                    if let Some(parent) = p.parent() {
                        let macro_path = parent.join("slide.typ");
                        if !macro_path.exists() {
                            let _ = std::fs::write(&macro_path, slide_theme::SLIDE_MACROS);
                        }
                    }
                    launch_presentation_player(p, is_light);
                } else {
                    // Write untitled/unsaved presentation buffer to persistent presentation cache
                    let cache_dir = std::env::temp_dir().join("cargo_slide_presentation_cache");
                    let _ = std::fs::create_dir_all(&cache_dir);
                    let macro_path = cache_dir.join("slide.typ");
                    let _ = std::fs::write(&macro_path, slide_theme::SLIDE_MACROS);

                    let temp_file = cache_dir.join("untitled_presentation.typ");
                    if std::fs::write(&temp_file, &self.doc.source_text).is_ok() {
                        launch_presentation_player(&temp_file, is_light);
                    }
                }
            },
            | Message::OpenErrorDetailsDialog => {
                if let CompilationStatus::Error(ref diag) = self.compilation_status {
                    self.active_modal = Some(ActiveModal::ErrorDetails(diag.clone()));
                }
            },
            | Message::CloseModal => {
                if self.active_modal.is_some() {
                    self.active_modal = None;
                } else if self.search_state.is_visible {
                    self.search_state.is_visible = false;
                }
            },
            | Message::OpenComplexModal {
                slide_idx,
                block_id,
                range,
                raw,
                callee,
            } => {
                self.active_slide = slide_idx;
                let modal =
                    ComplexElementModal::from_raw(slide_idx, block_id, range, &callee, &raw);
                match &modal.state {
                    | ComplexModalState::Grid { columns, .. } => {
                        self.modal_col_editors = columns
                            .iter()
                            .map(|c| text_editor::Content::with_text(c))
                            .collect();
                    },
                    | ComplexModalState::BoxBlock { content, .. } => {
                        self.modal_box_editor = text_editor::Content::with_text(content);
                    },
                    | ComplexModalState::Callout { body, .. } => {
                        self.modal_callout_editor = text_editor::Content::with_text(body);
                    },
                    | ComplexModalState::TitleSlide { body, .. } => {
                        self.modal_title_slide_editor = text_editor::Content::with_text(body);
                    },
                    | _ => {},
                }
                self.active_modal = Some(ActiveModal::ComplexElement(modal));
            },
            | Message::CloseComplexModal => {
                self.active_modal = None;
            },
            | Message::ApplyComplexModal => {
                if let Some(ActiveModal::ComplexElement(modal)) = self.active_modal.take() {
                    let slide_idx = modal.slide_idx;
                    self.active_slide = slide_idx;
                    let new_text = modal.to_typst();
                    let update_task = self.update(Message::UpdateBlockRange {
                        slide_idx: Some(slide_idx),
                        range: modal.range,
                        new_text,
                    });
                    let scroll_task = self.update(Message::SelectSlide(slide_idx));
                    return Task::batch(vec![update_task, scroll_task]);
                }
            },
            | Message::ModalTableUpdateCell { row, col, val } => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Table { ref mut rows, .. } = modal.state
                    && let Some(r) = rows.get_mut(row)
                    && let Some(c) = r.get_mut(col)
                {
                    *c = val;
                }
            },

            | Message::ModalTableAddRow => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Table {
                        cols, ref mut rows, ..
                    } = modal.state
                {
                    let new_row = vec![String::new(); cols];
                    rows.push(new_row);
                }
            },
            | Message::ModalTableDeleteRow(idx) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Table { ref mut rows, .. } = modal.state
                    && rows.len() > 1
                    && idx < rows.len()
                {
                    rows.remove(idx);
                }
            },
            | Message::ModalTableAddCol => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Table {
                        ref mut cols,
                        ref mut col_spec,
                        ref mut header_cells,
                        ref mut rows,
                        ..
                    } = modal.state
                {
                    *cols += 1;
                    if col_spec.trim().parse::<usize>().is_ok() {
                        *col_spec = cols.to_string();
                    }
                    header_cells.push(format!("Header {}", *cols));
                    for r in rows.iter_mut() {
                        r.push(String::new());
                    }
                }
            },
            | Message::ModalTableDeleteCol(idx) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Table {
                        ref mut cols,
                        ref mut col_spec,
                        ref mut header_cells,
                        ref mut rows,
                        ..
                    } = modal.state
                    && *cols > 1
                {
                    *cols -= 1;
                    if col_spec.trim().parse::<usize>().is_ok() {
                        *col_spec = cols.to_string();
                    }
                    if idx < header_cells.len() {
                        header_cells.remove(idx);
                    }
                    for r in rows.iter_mut() {
                        if idx < r.len() {
                            r.remove(idx);
                        }
                    }
                }
            },
            | Message::ModalTableUpdateColSpec(spec) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Table { ref mut col_spec, .. } = modal.state
                {
                    *col_spec = spec;
                }
            },
            | Message::ModalTableToggleHeader => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Table {
                        ref mut has_header, ..
                    } = modal.state
                {
                    *has_header = !*has_header;
                }
            },
            | Message::ModalTableUpdateHeaderCell { col, val } => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Table {
                        ref mut header_cells, ..
                    } = modal.state
                    && let Some(cell) = header_cells.get_mut(col)
                {
                    *cell = val;
                }
            },
            | Message::ModalChartUpdateTitle(title) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Chart { title: ref mut t, .. } = modal.state
                {
                    *t = title;
                }
            },
            | Message::ModalChartSetType(chart_type) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Chart {
                        chart_type: ref mut ct,
                        ..
                    } = modal.state
                {
                    *ct = chart_type;
                }
            },
            | Message::ModalChartUpdateSource(source) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Chart {
                        source: ref mut s, ..
                    } = modal.state
                {
                    *s = source;
                }
            },
            | Message::ModalChartUpdateSql(sql) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Chart { sql: ref mut q, .. } = modal.state
                {
                    *q = sql;
                }
            },
            | Message::ModalChartUpdateDsl(dsl) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Chart { dsl: ref mut d, .. } = modal.state
                {
                    *d = dsl;
                }
            },
            | Message::ModalChartUpdateFormat(format) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Chart {
                        format: ref mut f, ..
                    } = modal.state
                {
                    *f = format;
                }
            },
            | Message::ModalChartUpdateUnit(unit) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Chart { unit: ref mut u, .. } = modal.state
                {
                    *u = unit;
                }
            },
            | Message::ModalChartUpdatePrefix(prefix) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Chart {
                        prefix: ref mut p, ..
                    } = modal.state
                {
                    *p = prefix;
                }
            },
            | Message::ModalChartUpdateItemLabel { idx, label } => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Chart { ref mut items, .. } = modal.state
                    && let Some(item) = items.get_mut(idx)
                {
                    item.0 = label;
                }
            },
            | Message::ModalChartUpdateItemValue { idx, val } => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Chart { ref mut items, .. } = modal.state
                    && let Some(item) = items.get_mut(idx)
                {
                    item.1 = val;
                }
            },
            | Message::ModalChartAddItem => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Chart { ref mut items, .. } = modal.state
                {
                    let next_idx = items.len() + 1;
                    items.push((format!("Item {next_idx}"), 50.0));
                }
            },
            | Message::ModalChartDeleteItem(idx) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Chart { ref mut items, .. } = modal.state
                    && items.len() > 1
                    && idx < items.len()
                {
                    items.remove(idx);
                }
            },
            | Message::ModalGridUpdateCol { idx, val } => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Grid { ref mut columns, .. } = modal.state
                    && let Some(col) = columns.get_mut(idx)
                {
                    *col = val.clone();
                    if let Some(ed) = self.modal_col_editors.get_mut(idx) {
                        *ed = text_editor::Content::with_text(&val);
                    }
                }
            },
            | Message::ModalGridEditorAction { idx, action } => {
                if let Some(editor) = self.modal_col_editors.get_mut(idx) {
                    editor.perform(action);
                    let new_text = editor.text();
                    if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                        && let ComplexModalState::Grid { ref mut columns, .. } = modal.state
                        && let Some(col) = columns.get_mut(idx)
                    {
                        *col = new_text;
                    }
                }
            },
            | Message::ModalGridAddCol => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Grid {
                        ref mut cols,
                        ref mut columns,
                    } = modal.state
                {
                    *cols += 1;
                    let idx = *cols;
                    let col_str = format!("*Column {idx}*\nContent");
                    columns.push(col_str.clone());
                    self.modal_col_editors
                        .push(text_editor::Content::with_text(&col_str));
                }
            },
            | Message::ModalGridDeleteCol(idx) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Grid {
                        ref mut cols,
                        ref mut columns,
                    } = modal.state
                    && *cols > 1
                    && idx < columns.len()
                {
                    *cols -= 1;
                    columns.remove(idx);
                    if idx < self.modal_col_editors.len() {
                        self.modal_col_editors.remove(idx);
                    }
                }
            },
            | Message::ModalCalloutSetKind(k) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Callout {
                        ref mut stroke_color, ..
                    } = modal.state
                {
                    *stroke_color = match k.as_str() {
                        | "tip" | "cyan" => "slide-colors.accent-cyan".to_string(),
                        | "warning" | "alert" | "orange" => {
                            "slide-colors.accent-orange".to_string()
                        },
                        | "quote" | "purple" => "slide-colors.accent-purple".to_string(),
                        | "red" => "slide-colors.accent-red".to_string(),
                        | _ => "slide-colors.accent".to_string(),
                    };
                }
            },
            | Message::ModalCalloutSetStrokeColor(sc) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Callout {
                        ref mut stroke_color, ..
                    } = modal.state
                {
                    *stroke_color = sc;
                }
            },
            | Message::ModalCalloutUpdateStrokeColor(sc) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Callout {
                        ref mut stroke_color, ..
                    } = modal.state
                {
                    *stroke_color = sc;
                }
            },
            | Message::ModalCalloutUpdateTitle(title) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Callout { title: ref mut t, .. } = modal.state
                {
                    *t = title;
                }
            },
            | Message::ModalCalloutUpdateBody(body) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Callout { body: ref mut b, .. } = modal.state
                {
                    *b = body.clone();
                    self.modal_callout_editor = text_editor::Content::with_text(&body);
                }
            },
            | Message::ModalCalloutEditorAction(action) => {
                self.modal_callout_editor.perform(action);
                let new_text = self.modal_callout_editor.text();
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Callout { body: ref mut b, .. } = modal.state
                {
                    *b = new_text;
                }
            },
            | Message::ModalBoxUpdateContent(content) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::BoxBlock {
                        content: ref mut c, ..
                    } = modal.state
                {
                    *c = content.clone();
                    self.modal_box_editor = text_editor::Content::with_text(&content);
                }
            },
            | Message::ModalBoxEditorAction(action) => {
                self.modal_box_editor.perform(action);
                let new_text = self.modal_box_editor.text();
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::BoxBlock {
                        content: ref mut c, ..
                    } = modal.state
                {
                    *c = new_text;
                }
            },
            | Message::ModalBoxApplyPreset(preset) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::BoxBlock {
                        ref mut fill,
                        ref mut stroke,
                        ref mut radius,
                        ref mut inset,
                        ..
                    } = modal.state
                {
                    match preset {
                        | "subtle" => {
                            *fill = "rgb(\"f8fafc\")".to_string();
                            *stroke = "1pt + rgb(\"e2e8f0\")".to_string();
                            *radius = "6pt".to_string();
                            *inset = "8pt".to_string();
                        },
                        | "blue" => {
                            *fill = "rgb(\"eff6ff\")".to_string();
                            *stroke = "1pt + rgb(\"3b82f6\")".to_string();
                            *radius = "6pt".to_string();
                            *inset = "8pt".to_string();
                        },
                        | "green" => {
                            *fill = "rgb(\"f0fdf4\")".to_string();
                            *stroke = "1pt + rgb(\"10b981\")".to_string();
                            *radius = "6pt".to_string();
                            *inset = "8pt".to_string();
                        },
                        | "amber" => {
                            *fill = "rgb(\"fffbeb\")".to_string();
                            *stroke = "1pt + rgb(\"f59e0b\")".to_string();
                            *radius = "6pt".to_string();
                            *inset = "8pt".to_string();
                        },
                        | "red" => {
                            *fill = "rgb(\"fef2f2\")".to_string();
                            *stroke = "1pt + rgb(\"ef4444\")".to_string();
                            *radius = "6pt".to_string();
                            *inset = "8pt".to_string();
                        },
                        | "outline" => {
                            *fill = "none".to_string();
                            *stroke = "1.5pt + rgb(\"64748b\")".to_string();
                            *radius = "6pt".to_string();
                            *inset = "8pt".to_string();
                        },
                        | _ => {
                            *fill = "none".to_string();
                            *stroke = "none".to_string();
                            *radius = "0pt".to_string();
                            *inset = "6pt".to_string();
                        },
                    }
                }
            },
            | Message::ModalBoxSetFill(f) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::BoxBlock {
                        fill: ref mut f_val, ..
                    } = modal.state
                {
                    *f_val = f;
                }
            },
            | Message::ModalBoxSetStroke(s) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::BoxBlock {
                        stroke: ref mut s_val,
                        ..
                    } = modal.state
                {
                    *s_val = s;
                }
            },
            | Message::ModalBoxSetRadius(r) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::BoxBlock {
                        radius: ref mut r_val,
                        ..
                    } = modal.state
                {
                    *r_val = r;
                }
            },
            | Message::ModalBoxSetInset(i) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::BoxBlock {
                        inset: ref mut i_val, ..
                    } = modal.state
                {
                    *i_val = i;
                }
            },
            | Message::ModalBoxSetWidth(w) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::BoxBlock {
                        width: ref mut w_val, ..
                    } = modal.state
                {
                    *w_val = w;
                }
            },
            | Message::ModalLinkUpdateUrl(u) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Link {
                        url: ref mut u_val, ..
                    } = modal.state
                {
                    *u_val = u;
                }
            },
            | Message::ModalLinkUpdateLabel(l) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Link {
                        label: ref mut l_val, ..
                    } = modal.state
                {
                    *l_val = l;
                }
            },
            | Message::ModalBadgeUpdateLabel(lbl) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Badge {
                        label: ref mut l_val, ..
                    } = modal.state
                {
                    *l_val = lbl;
                }
            },
            | Message::ModalBadgeSetFill(f) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Badge {
                        fill: ref mut f_val, ..
                    } = modal.state
                {
                    *f_val = f;
                }
            },
            | Message::ModalBadgeSetTextColor(tc) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Badge {
                        text_color: ref mut tc_val,
                        ..
                    } = modal.state
                {
                    *tc_val = tc;
                }
            },
            | Message::ModalVideoUpdateSource(s) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Video {
                        source: ref mut s_val,
                        ..
                    } = modal.state
                {
                    *s_val = s;
                }
            },
            | Message::ModalVideoUpdateCaption(c) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Video {
                        caption: ref mut c_val,
                        ..
                    } = modal.state
                {
                    *c_val = c;
                }
            },
            | Message::ModalVideoUpdateDuration(d) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Video {
                        duration: ref mut d_val,
                        ..
                    } = modal.state
                {
                    *d_val = d;
                }
            },
            | Message::ModalVideoSetQuality(q) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Video {
                        quality: ref mut q_val,
                        ..
                    } = modal.state
                {
                    *q_val = q;
                }
            },
            | Message::ModalVideoSetStyle(st) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Video {
                        style: ref mut s_val, ..
                    } = modal.state
                {
                    *s_val = st;
                }
            },
            | Message::ModalVideoSetWidth(w) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Video {
                        width: ref mut w_val, ..
                    } = modal.state
                {
                    *w_val = w;
                }
            },
            | Message::ModalAudioUpdateSource(s) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Audio {
                        source: ref mut s_val,
                        ..
                    } = modal.state
                {
                    *s_val = s;
                }
            },
            | Message::ModalAudioSetIsPlayer(p) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Audio {
                        is_player: ref mut p_val,
                        ..
                    } = modal.state
                {
                    *p_val = p;
                }
            },
            | Message::ModalAudioUpdateTitle(t) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Audio {
                        title: ref mut t_val, ..
                    } = modal.state
                {
                    *t_val = t;
                }
            },
            | Message::ModalAudioUpdateArtist(a) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Audio {
                        artist: ref mut a_val,
                        ..
                    } = modal.state
                {
                    *a_val = a;
                }
            },
            | Message::ModalAudioToggleAutoplay => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Audio {
                        autoplay: ref mut a_val,
                        ..
                    } = modal.state
                {
                    *a_val = !*a_val;
                }
            },
            | Message::ModalAudioToggleLoop => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Audio {
                        loop_playback: ref mut l_val,
                        ..
                    } = modal.state
                {
                    *l_val = !*l_val;
                }
            },
            | Message::ModalAudioSetVolume(v) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::Audio {
                        volume: ref mut v_val,
                        ..
                    } = modal.state
                {
                    *v_val = v.clamp(0.0, 1.0);
                }
            },
            | Message::ModalTitleSlideUpdateTitle(t) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::TitleSlide {
                        title: ref mut t_val, ..
                    } = modal.state
                {
                    *t_val = t;
                }
            },
            | Message::ModalTitleSlideUpdateSubtitle(s) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::TitleSlide {
                        subtitle: ref mut s_val,
                        ..
                    } = modal.state
                {
                    *s_val = s;
                }
            },
            | Message::ModalTitleSlideUpdateAuthor(a) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::TitleSlide {
                        author: ref mut a_val,
                        ..
                    } = modal.state
                {
                    *a_val = a;
                }
            },
            | Message::ModalTitleSlideUpdateDate(d) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::TitleSlide {
                        date: ref mut d_val, ..
                    } = modal.state
                {
                    *d_val = d;
                }
            },
            | Message::ModalTitleSlideUpdateVersion(v) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::TitleSlide {
                        version: ref mut v_val,
                        ..
                    } = modal.state
                {
                    *v_val = v;
                }
            },
            | Message::ModalTitleSlideUpdateInstitution(i) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::TitleSlide {
                        institution: ref mut i_val,
                        ..
                    } = modal.state
                {
                    *i_val = i;
                }
            },
            | Message::ModalTitleSlideAddExtraArg => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::TitleSlide {
                        extra_args: ref mut args,
                        ..
                    } = modal.state
                {
                    args.push(("param".to_string(), "val".to_string()));
                }
            },
            | Message::ModalTitleSlideUpdateExtraKey(idx, k) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::TitleSlide {
                        extra_args: ref mut args,
                        ..
                    } = modal.state
                    && let Some((key, _)) = args.get_mut(idx)
                {
                    *key = k;
                }
            },
            | Message::ModalTitleSlideUpdateExtraVal(idx, v) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::TitleSlide {
                        extra_args: ref mut args,
                        ..
                    } = modal.state
                    && let Some((_, val)) = args.get_mut(idx)
                {
                    *val = v;
                }
            },
            | Message::ModalTitleSlideRemoveExtraArg(idx) => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::TitleSlide {
                        extra_args: ref mut args,
                        ..
                    } = modal.state
                    && idx < args.len()
                {
                    args.remove(idx);
                }
            },
            | Message::ModalTitleSlideEditorAction(action) => {
                self.modal_title_slide_editor.perform(action);
                let new_body = self.modal_title_slide_editor.text();
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::TitleSlide {
                        body: ref mut b_val, ..
                    } = modal.state
                {
                    *b_val = new_body;
                }
            },
            | Message::ModalTitleSlideApplyPreset {
                title,
                subtitle,
                author,
                date,
                version,
                institution,
            } => {
                if let Some(ActiveModal::ComplexElement(ref mut modal)) = self.active_modal
                    && let ComplexModalState::TitleSlide {
                        title: ref mut t_val,
                        subtitle: ref mut s_val,
                        author: ref mut a_val,
                        date: ref mut d_val,
                        version: ref mut v_val,
                        institution: ref mut i_val,
                        ..
                    } = modal.state
                {
                    if let Some(t) = title {
                        *t_val = t;
                    }
                    if let Some(s) = subtitle {
                        *s_val = s;
                    }
                    if let Some(a) = author {
                        *a_val = a;
                    }
                    if let Some(d) = date {
                        *d_val = d;
                    }
                    if let Some(v) = version {
                        *v_val = v;
                    }
                    if let Some(i) = institution {
                        *i_val = i;
                    }
                }
            },
            | Message::OpenTransitionModal(slide_idx) => {
                let current_transition = self
                    .engine
                    .slides
                    .get(slide_idx)
                    .and_then(|s| s.transition.clone());
                self.active_modal = Some(ActiveModal::SlideTransition {
                    slide_idx,
                    current_transition,
                    apply_to_all: false,
                });
            },
            | Message::SelectTransition(trans) => {
                if let Some(ActiveModal::SlideTransition {
                    ref mut current_transition,
                    ..
                }) = self.active_modal
                {
                    *current_transition = trans;
                }
            },
            | Message::ApplySlideTransition {
                slide_idx,
                transition,
                all_slides,
            } => {
                self.active_modal = None;
                let trans_ref = transition.as_deref();
                if all_slides {
                    let updated = crate::model::ast_engine::update_all_slides_transition(
                        &self.engine.source_text,
                        &self.engine,
                        trans_ref,
                    );
                    self.engine.source_text = updated;
                } else if let Some(slide) = self.engine.slides.get(slide_idx) {
                    let updated = crate::model::ast_engine::update_slide_transition(
                        &self.engine.source_text,
                        slide.range.clone(),
                        trans_ref,
                    );
                    self.engine.source_text = updated;
                }
                self.engine.reparse();
                self.doc.source_text = self.engine.source_text.clone();
                self.doc.is_dirty = true;
                self.doc.sync_chunks_from_source();
                self.sync_editors_from_doc();
                task = self.trigger_recompile_task();
            },
            | Message::OpenElementTransitionModal { slide_idx, block_idx } => {
                let (order, effect, is_existing) = if let Some(slide) =
                    self.engine.slides.get(slide_idx)
                    && let Some(block) = slide.blocks.get(block_idx)
                    && let Some(tr) = slide.element_transitions.get(block.id())
                {
                    (tr.order, tr.effect.clone(), true)
                } else {
                    (1, "fade-in".to_string(), false)
                };
                self.active_modal = Some(ActiveModal::ElementTransition {
                    slide_idx,
                    block_idx,
                    order,
                    effect,
                    is_existing,
                });
            },
            | Message::SelectElementTransitionEffect(eff) => {
                if let Some(ActiveModal::ElementTransition { ref mut effect, .. }) =
                    self.active_modal
                {
                    *effect = eff;
                }
            },
            | Message::SetElementTransitionOrder(ord) => {
                if let Some(ActiveModal::ElementTransition { ref mut order, .. }) =
                    self.active_modal
                {
                    *order = ord.max(1);
                }
            },
            | Message::ApplyElementTransition => {
                if let Some(ActiveModal::ElementTransition {
                    slide_idx,
                    block_idx,
                    order,
                    ref effect,
                    is_existing,
                }) = self.active_modal.take()
                {
                    let effect_clone = effect.clone();
                    if let Some(slide) = self.engine.slides.get(slide_idx)
                        && let Some(block) = slide.blocks.get(block_idx)
                    {
                        let new_source = if is_existing {
                            if let Some(tr) = slide.element_transitions.get(block.id()) {
                                crate::model::ast_engine::update_existing_step_transition(
                                    &self.engine.source_text,
                                    tr.wrapper_range.clone(),
                                    order,
                                    &effect_clone,
                                )
                            } else {
                                crate::model::ast_engine::apply_block_transition(
                                    &self.engine.source_text,
                                    block.range(),
                                    order,
                                    &effect_clone,
                                )
                            }
                        } else {
                            crate::model::ast_engine::apply_block_transition(
                                &self.engine.source_text,
                                block.range(),
                                order,
                                &effect_clone,
                            )
                        };
                        self.engine.source_text = new_source;
                        self.engine.reparse();
                        self.doc.source_text = self.engine.source_text.clone();
                        self.doc.is_dirty = true;
                        self.doc.sync_chunks_from_source();
                        self.sync_editors_from_doc();
                        task = self.trigger_recompile_task();
                    }
                }
            },
            | Message::RemoveElementTransition => {
                if let Some(ActiveModal::ElementTransition {
                    slide_idx, block_idx, ..
                }) = self.active_modal.take()
                    && let Some(slide) = self.engine.slides.get(slide_idx)
                    && let Some(block) = slide.blocks.get(block_idx)
                    && let Some(tr) = slide.element_transitions.get(block.id())
                {
                    let new_source = crate::model::ast_engine::remove_step_transition(
                        &self.engine.source_text,
                        tr.wrapper_range.clone(),
                    );
                    self.engine.source_text = new_source;
                    self.engine.reparse();
                    self.doc.source_text = self.engine.source_text.clone();
                    self.doc.is_dirty = true;
                    self.doc.sync_chunks_from_source();
                    self.sync_editors_from_doc();
                    task = self.trigger_recompile_task();
                }
            },
            | Message::OpenUrl(url) => {
                #[cfg(target_os = "windows")]
                let _ = std::process::Command::new("cmd")
                    .args(["/C", "start", &url])
                    .spawn();
                #[cfg(target_os = "macos")]
                let _ = std::process::Command::new("open").arg(&url).spawn();
                #[cfg(target_os = "linux")]
                let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
            },
            | Message::DeleteActiveSlide => {
                let current_idx = self.active_slide;
                if self.doc.total_slides() > 1 {
                    self.push_undo_snapshot();
                    self.commit_active_block();
                    self.doc.delete_slide(current_idx);
                    self.engine.source_text = self.doc.source_text.clone();
                    self.engine.reparse();
                    self.active_slide = current_idx.min(self.doc.total_slides().saturating_sub(1));
                    self.sync_editors_from_doc();
                    task = self.trigger_recompile_task();
                }
            },
            | Message::OpenCommandPalette => {
                self.active_modal = Some(ActiveModal::CommandPalette {
                    query: String::new(),
                    selected_idx: 0,
                });
            },
            | Message::CommandPaletteQueryChanged(q) => {
                if let Some(ActiveModal::CommandPalette {
                    ref mut query,
                    ref mut selected_idx,
                }) = self.active_modal
                {
                    *query = q;
                    *selected_idx = 0;
                }
            },
            | Message::CommandPaletteSelectIndex(idx) => {
                if let Some(ActiveModal::CommandPalette {
                    ref mut selected_idx, ..
                }) = self.active_modal
                {
                    *selected_idx = idx;
                }
            },
            | Message::CommandPaletteExecute => {
                if let Some(ActiveModal::CommandPalette {
                    ref query,
                    selected_idx,
                }) = self.active_modal
                {
                    let actions = crate::ui::modals::get_palette_actions();
                    let q_lower = query.trim().to_lowercase();
                    let filtered: Vec<_> = actions
                        .into_iter()
                        .filter(|a| {
                            if q_lower.is_empty() {
                                true
                            } else {
                                a.title.to_lowercase().contains(&q_lower)
                                    || a.category.to_lowercase().contains(&q_lower)
                                    || a.shortcut.to_lowercase().contains(&q_lower)
                            }
                        })
                        .collect();

                    if let Some(act) = filtered.get(selected_idx) {
                        let target_msg = act.message.clone();
                        self.active_modal = None;
                        return self.update(target_msg);
                    }
                }
                self.active_modal = None;
            },
            | Message::GenerateAgendaSlide => {
                let mut titles = Vec::new();
                for slide in &self.engine.slides {
                    let trim = slide.title.trim();
                    if !trim.is_empty()
                        && !trim.eq_ignore_ascii_case("agenda")
                        && !trim.eq_ignore_ascii_case("table of contents")
                        && !trim.eq_ignore_ascii_case("outline")
                    {
                        titles.push(trim.to_string());
                    }
                }
                if titles.is_empty() {
                    titles = vec![
                        "Executive Summary & Objectives".to_string(),
                        "System Architecture & Core Engine".to_string(),
                        "Benchmark Results & Metrics".to_string(),
                        "Future Roadmap & Q&A".to_string(),
                    ];
                }

                let items_str = titles
                    .iter()
                    .map(|t| format!("    \"{}\",", t.replace('"', "\\\"")))
                    .collect::<Vec<_>>()
                    .join("\n");

                let agenda_chunk = format!(
                    "#slide(title: [Agenda & Overview])[\n  == Presentation Outline\n\n  #agenda(\n{}\n  )\n]",
                    items_str
                );

                self.push_undo_snapshot();
                self.commit_active_block();
                let insert_pos = if self.doc.total_slides() > 1 {
                    1
                } else {
                    self.doc.total_slides()
                };
                self.doc
                    .insert_custom_slide_chunk_at(insert_pos, &agenda_chunk);
                self.engine.source_text = self.doc.source_text.clone();
                self.engine.reparse();
                self.active_slide = insert_pos;
                self.active_modal = None;
                self.sync_editors_from_doc();
                task = self.trigger_recompile_task();
            },
            | Message::RestoreRecoveryDraft(content) => {
                self.push_undo_snapshot();
                self.doc.source_text = content;
                self.doc.is_dirty = true;
                self.doc.sync_chunks_from_source();
                self.engine.source_text = self.doc.source_text.clone();
                self.engine.reparse();
                self.sync_editors_from_doc();
                self.active_modal = None;
                task = self.trigger_recompile_task();
            },
            | Message::DiscardRecoveryDraft => {
                let _ = self.doc.clear_recovery_draft();
                self.active_modal = None;
            },
        }

        task
    }

    fn open_export_with_format(
        &mut self,
        fmt: ExportFormat,
    ) {
        self.export_format = fmt;
        self.update_export_path_extension();
        self.export_status = None;
        self.active_modal = Some(ActiveModal::Export);
    }

    fn update_export_path_extension(&mut self) {
        let ext = self.export_format.default_extension();
        let current = PathBuf::from(if self.export_path.is_empty() {
            "presentation".to_string()
        } else {
            self.export_path.clone()
        });
        let mut new_path = current;
        new_path.set_extension(ext);
        self.export_path = new_path.to_string_lossy().to_string();
    }

    fn handle_drag_move(
        &mut self,
        slide_idx: usize,
        block_idx: usize,
        y: f32,
    ) -> Task<Message> {
        if let Some((s_idx, b_idx, start_y, _)) = self.dragging_block
            && s_idx == slide_idx
            && b_idx == block_idx
        {
            if start_y.is_nan() {
                self.dragging_block = Some((slide_idx, block_idx, y, false));
                return Task::none();
            }
            let dy = y - start_y;
            if dy < -30.0 && block_idx > 0 {
                self.dragging_block = Some((slide_idx, block_idx - 1, y, true));
                return self.update(Message::MoveBlockUp { slide_idx, block_idx });
            } else if dy > 30.0
                && let Some(slide) = self.engine.slides.get(slide_idx)
            {
                let total = slide
                    .blocks
                    .iter()
                    .filter(|b| b.is_content_element())
                    .count();
                if block_idx + 1 < total {
                    self.dragging_block = Some((slide_idx, block_idx + 1, y, true));
                    return self.update(Message::MoveBlockDown { slide_idx, block_idx });
                }
            }
        }
        Task::none()
    }

    fn handle_spacing_drag(
        &mut self,
        slide_idx: usize,
        block_idx: usize,
        y: f32,
    ) -> Task<Message> {
        if let Some((s_idx, b_idx, start_y)) = self.dragging_spacing
            && s_idx == slide_idx
            && b_idx == block_idx
        {
            if start_y.is_nan() {
                self.dragging_spacing = Some((slide_idx, block_idx, y));
                return Task::none();
            }
            let dy = y - start_y;
            if dy.abs() >= 10.0 {
                let delta_pt = if dy > 0.0 { 4 } else { -4 };
                self.dragging_spacing = Some((slide_idx, block_idx, y));
                return self.update(Message::AdjustBlockSpacing {
                    slide_idx,
                    block_idx,
                    delta_pt,
                });
            }
        }
        Task::none()
    }

    /// Global application subscription for events and drag gestures
    pub fn subscription(&self) -> Subscription<Message> {
        iced::event::listen_with(handle_global_drag_event)
    }

    /// Render application UI
    #[must_use]
    pub fn view(&self) -> Element<'_, Message> {
        let win_w = self.window_size.width;

        let toolbar = view_toolbar(
            self.theme,
            &self.doc.title,
            self.doc.format,
            self.doc.is_dirty,
            self.mode,
            self.sidebar_visible,
            !self.undo_stack.is_empty(),
            !self.redo_stack.is_empty(),
            win_w,
        );

        let formatting_bar = view_formatting_bar(self.theme, self.active_slide, win_w);

        let main_view: Element<'_, Message> = match self.mode {
            | EditorMode::LivePreview => {
                view_live_preview(
                    self.theme,
                    &self.engine.slides,
                    &self.slide_images,
                    &self.equation_images,
                    &self.code_images,
                    self.active_slide,
                    self.active_block_id.as_deref(),
                    if self.active_block_id.is_some() {
                        Some(&self.active_block_content)
                    } else {
                        None
                    },
                    self.in_place_editing_slide,
                    if self.in_place_editing_slide.is_some() {
                        Some(&self.in_place_content)
                    } else {
                        None
                    },
                    self.zoom_percent,
                    &self.slide_view_vector,
                    self.dragging_block.map(|(s, b, _, _)| (s, b)),
                    &self.raw_code_blocks,
                    &self.engine,
                )
            },
            | EditorMode::FocusMode => {
                view_focus_mode(
                    self.theme,
                    &self.doc,
                    &self.slide_images,
                    self.active_slide,
                    &self.focus_slide_content,
                    self.zoom_percent,
                    &self.engine.slides,
                    self.active_hover_formula.as_deref(),
                    &self.equation_images,
                )
            },
            | EditorMode::SourceMode => {
                view_source_mode(
                    self.theme,
                    &self.source_content,
                    self.doc.file_path.as_deref(),
                    self.doc.source_text.lines().count(),
                    self.active_hover_formula.as_deref(),
                    &self.equation_images,
                )
            },
        };

        let center_content: Element<'_, Message> = if self.sidebar_visible {
            let sidebar = view_sidebar(
                self.theme,
                &self.doc,
                &self.slide_titles,
                &self.engine.slides,
                &self.slide_images,
                self.active_slide,
                self.sidebar_view_mode,
                win_w,
            );
            row![sidebar, main_view]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            main_view
        };

        let center_content: Element<'_, Message> = if self.search_state.is_visible {
            let search_bar = crate::ui::modals::view_search_replace_bar(
                self.theme,
                &self.search_state.search_query,
                &self.search_state.replace_query,
                self.search_state.is_regex,
                self.search_state.case_sensitive,
                self.search_state.whole_word,
                self.search_state.status_message.as_deref(),
            );
            let search_overlay = container(search_bar)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::End)
                .align_y(Alignment::Start)
                .padding(iced::Padding {
                    top: 10.0,
                    right: 16.0,
                    bottom: 0.0,
                    left: 0.0,
                });
            iced::widget::stack![center_content, search_overlay].into()
        } else {
            center_content
        };

        let words = self.cached_word_count;
        let chars = self.cached_char_count;

        let statusbar = view_statusbar(
            self.theme,
            self.active_slide,
            self.engine.slides.len().max(1),
            self.doc.format,
            self.mode,
            &self.compilation_status,
            chars,
            words,
            self.zoom_percent,
            self.speaking_wpm,
            win_w,
            self.doc.is_dirty,
        );

        let base_content = column![toolbar, formatting_bar, center_content, statusbar]
            .width(Length::Fill)
            .height(Length::Fill);

        let root_content: Element<'_, Message> = base_content.into();

        // Check if any modal is active
        if let Some(ref modal) = self.active_modal {
            let modal_element: Element<'_, Message> = match modal {
                | ActiveModal::Export => {
                    view_export_modal(
                        self.theme,
                        self.export_format,
                        &self.export_path,
                        self.export_png_scale,
                        self.export_page_selection,
                        &self.export_custom_range,
                        self.export_include_source,
                        self.active_slide,
                        self.engine.slides.len(),
                        self.export_status.as_deref(),
                    )
                },
                | ActiveModal::Open => {
                    view_open_modal(
                        self.theme,
                        &self.open_path,
                        self.open_error.as_deref(),
                        &self.recent_files,
                    )
                },
                | ActiveModal::ErrorDetails(diag) => view_error_details_modal(self.theme, diag),
                | ActiveModal::ComplexElement(modal) => {
                    view_complex_element_modal(
                        self.theme,
                        modal,
                        Some(&self.modal_col_editors),
                        Some(&self.modal_box_editor),
                        Some(&self.modal_title_slide_editor),
                        Some(&self.modal_callout_editor),
                    )
                },
                | ActiveModal::SlideTransition {
                    slide_idx,
                    current_transition,
                    ..
                } => {
                    crate::ui::wysiwyg::transition_modal::view_slide_transition_modal(
                        self.theme,
                        *slide_idx,
                        current_transition.as_deref(),
                        self.engine.slides.len(),
                    )
                },
                | ActiveModal::ElementTransition {
                    slide_idx,
                    block_idx,
                    order,
                    effect,
                    is_existing,
                } => {
                    crate::ui::wysiwyg::transition_modal::view_element_transition_modal(
                        self.theme,
                        *slide_idx,
                        *block_idx,
                        *order,
                        effect,
                        *is_existing,
                    )
                },
                | ActiveModal::MoveBlockToSlide {
                    from_slide_idx,
                    range,
                    block_label,
                    ..
                } => {
                    crate::ui::modals::view_move_block_modal(
                        self.theme,
                        *from_slide_idx,
                        range.clone(),
                        block_label,
                        &self.slide_titles,
                        self.doc.total_slides(),
                    )
                },
                | ActiveModal::SlideContextMenu { slide_idx, position } => {
                    crate::ui::modals::view_slide_context_menu_modal(
                        self.theme,
                        *slide_idx,
                        self.doc.total_slides(),
                        *position,
                        self.window_size,
                    )
                },
                | ActiveModal::BlockContextMenu {
                    slide_idx,
                    block_idx,
                    range,
                    block_label,
                    block_id,
                    position,
                } => {
                    crate::ui::modals::view_block_context_menu_modal(
                        self.theme,
                        *slide_idx,
                        *block_idx,
                        range.clone(),
                        block_label,
                        block_id,
                        *position,
                        self.window_size,
                    )
                },
                | ActiveModal::FontSelector {
                    search_query,
                    current_font,
                } => {
                    crate::ui::modals::view_font_selector_modal(
                        self.theme,
                        search_query,
                        current_font,
                    )
                },
                | ActiveModal::HeaderFooter {
                    header_enabled,
                    header_left,
                    header_right,
                    footer_enabled,
                    footer_left,
                    footer_right_mode,
                    footer_right_custom,
                } => {
                    crate::ui::modals::view_header_footer_modal(
                        self.theme,
                        *header_enabled,
                        header_left,
                        header_right,
                        *footer_enabled,
                        footer_left,
                        *footer_right_mode,
                        footer_right_custom,
                    )
                },
                | ActiveModal::TemplateLibrary => view_template_library_modal(self.theme),
                | ActiveModal::PresentationHealth(issues, pacing) => {
                    view_presentation_health_modal(self.theme, issues, pacing)
                },
                | ActiveModal::CommandPalette { query, selected_idx } => {
                    crate::ui::modals::view_command_palette_modal(self.theme, query, *selected_idx)
                },
                | ActiveModal::RecoveryDraft { draft_content } => {
                    crate::ui::modals::view_recovery_draft_modal(self.theme, draft_content)
                },
            };

            // Overlay modal on top of base content
            iced::widget::stack![root_content, modal_element].into()
        } else {
            // Keep stack at root level so opening/closing modals never discards child 0 scrollable state tree!
            iced::widget::stack![root_content].into()
        }
    }
}

fn handle_global_drag_event(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    match event {
        | iced::Event::Window(iced::window::Event::Resized(size)) => {
            Some(Message::WindowResized(size))
        },
        | iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
            Some(Message::GlobalCursorMoved(position))
        },
        | iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
            Some(Message::GlobalButtonReleased)
        },
        | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
            ..
        }) => Some(Message::CloseModal),
        | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Character(ref c),
            modifiers,
            ..
        }) if (c == "f" || c == "F") && (modifiers.control() || modifiers.command()) => {
            Some(Message::OpenSearchModal)
        },
        | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Character(ref c),
            modifiers,
            ..
        }) if (c == "z" || c == "Z")
            && (modifiers.control() || modifiers.command())
            && !modifiers.shift() =>
        {
            Some(Message::Undo)
        },
        | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Character(ref c),
            modifiers,
            ..
        }) if (c == "y" || c == "Y") && (modifiers.control() || modifiers.command()) => {
            Some(Message::Redo)
        },
        | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Character(ref c),
            modifiers,
            ..
        }) if (c == "z" || c == "Z")
            && (modifiers.control() || modifiers.command())
            && modifiers.shift() =>
        {
            Some(Message::Redo)
        },
        | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Character(ref c),
            modifiers,
            ..
        }) if (c == "s" || c == "S") && (modifiers.control() || modifiers.command()) => {
            Some(Message::SaveDocument)
        },
        | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Character(ref c),
            modifiers,
            ..
        }) if (c == "d" || c == "D") && (modifiers.control() || modifiers.command()) => {
            Some(Message::DuplicateActiveSlide)
        },
        | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Character(ref c),
            modifiers,
            ..
        }) if (c == "k" || c == "K" || c == "p" || c == "P")
            && (modifiers.control() || modifiers.command()) =>
        {
            Some(Message::OpenCommandPalette)
        },
        | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Character(ref c),
            modifiers,
            ..
        }) if (c == "0") && (modifiers.control() || modifiers.command()) => {
            Some(Message::ResetZoom)
        },
        | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Character(ref c),
            modifiers,
            ..
        }) if (c == "=" || c == "+") && (modifiers.control() || modifiers.command()) => {
            Some(Message::ZoomIn)
        },
        | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Character(ref c),
            modifiers,
            ..
        }) if (c == "-") && (modifiers.control() || modifiers.command()) => Some(Message::ZoomOut),
        | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Character(ref c),
            modifiers,
            ..
        }) if (c == "b" || c == "B") && (modifiers.control() || modifiers.command()) => {
            Some(Message::ToggleSidebar)
        },
        | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Character(ref c),
            modifiers,
            ..
        }) if (c == "e" || c == "E") && (modifiers.control() || modifiers.command()) => {
            Some(Message::OpenExportDialog)
        },
        | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Character(ref c),
            modifiers,
            ..
        }) if (c == "o" || c == "O") && (modifiers.control() || modifiers.command()) => {
            Some(Message::OpenDocumentDialog)
        },
        | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Character(ref c),
            modifiers,
            ..
        }) if (c == "n" || c == "N") && (modifiers.control() || modifiers.command()) => {
            Some(Message::NewDocument)
        },
        | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Character(ref c),
            modifiers,
            ..
        }) if (c == "h" || c == "H") && modifiers.alt() => {
            Some(Message::OpenPresentationHealthModal)
        },
        | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Named(iced::keyboard::key::Named::F5),
            ..
        }) => Some(Message::PlayPresentation),
        | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp),
            modifiers,
            ..
        }) if modifiers.alt() => Some(Message::MoveActiveSlideUp),
        | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown),
            modifiers,
            ..
        }) if modifiers.alt() => Some(Message::MoveActiveSlideDown),
        | _ => None,
    }
}

fn launch_presentation_player(
    file_path: &Path,
    is_light: bool,
) {
    let current_exe = std::env::current_exe().ok();
    let current_dir = current_exe.as_ref().and_then(|p| p.parent());

    // Priority 1: slide-viewer in the same directory as current_exe (or parent target/debug when running in deps)
    if let Some(dir) = current_dir {
        let check_dirs = [dir.to_path_buf(), dir.parent().unwrap_or(dir).to_path_buf()];
        for d in check_dirs {
            let viewer = d.join(if cfg!(windows) {
                "slide-viewer.exe"
            } else {
                "slide-viewer"
            });
            if viewer.is_file() {
                let mut cmd = std::process::Command::new(&viewer);
                cmd.arg(file_path);
                if is_light {
                    cmd.arg("--light");
                }
                if cmd.spawn().is_ok() {
                    return;
                }
            }
        }
        // Priority 2: cargo-slide in the same directory as current_exe
        let cargo_slide = dir.join(if cfg!(windows) {
            "cargo-slide.exe"
        } else {
            "cargo-slide"
        });
        if cargo_slide.is_file() {
            let mut cmd = std::process::Command::new(&cargo_slide);
            cmd.args(["run", &file_path.to_string_lossy()]);
            if is_light {
                cmd.arg("--light");
            }
            if cmd.spawn().is_ok() {
                return;
            }
        }
    }

    // Priority 3: slide-viewer in PATH
    let mut cmd = std::process::Command::new("slide-viewer");
    cmd.arg(file_path);
    if is_light {
        cmd.arg("--light");
    }
    if cmd.spawn().is_ok() {
        return;
    }

    // Priority 4: cargo-slide in PATH
    let mut cmd = std::process::Command::new("cargo-slide");
    cmd.args(["run", &file_path.to_string_lossy()]);
    if is_light {
        cmd.arg("--light");
    }
    if cmd.spawn().is_ok() {
        return;
    }

    // Priority 5: fallback to cargo slide run <file_path>
    let mut cmd = std::process::Command::new("cargo");
    cmd.args(["slide", "run", &file_path.to_string_lossy()]);
    if is_light {
        cmd.arg("--light");
    }
    let _ = cmd.spawn();
}
