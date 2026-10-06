//! Sleek, modern Iced GUI Launcher for Slide Viewer.
//! Provides a refined dashboard to discover, preview, configure, and launch presentations.

use crate::SlideFileEntry;
use crate::installer::install_viewer_to_system;
use crate::installer::pick_presentation_file;
use crate::theme::ViewerTheme;
use crate::theme::badge_container;
use crate::theme::canvas_container;
use crate::theme::card_container;
use crate::theme::chip_button_style;
use crate::theme::footer_container;
use crate::theme::header_container;
use crate::theme::modal_backdrop_style;
use crate::theme::modal_dialog_style;
use crate::theme::pick_list_style;
use crate::theme::primary_button_style;
use crate::theme::search_input_style;
use crate::theme::secondary_button_style;
use crate::theme::sidebar_container;
use iced::Alignment;
use iced::Background;
use iced::Border;
use iced::Color;
use iced::Element;
use iced::Length;
use iced::Shadow;
use iced::Task;
use iced::widget::Space;
use iced::widget::button;
use iced::widget::checkbox;
use iced::widget::column;
use iced::widget::container;
use iced::widget::pick_list;
use iced::widget::row;
use iced::widget::scrollable;
use iced::widget::text;
use iced::widget::text_input;
use slide_player::hud::HudTheme;
use std::path::PathBuf;
use std::time::Instant;

pub const AVAILABLE_ANIMATIONS: &[&str] = &[
    "fade",
    "zoom",
    "slide-left",
    "slide-right",
    "particles",
    "cut",
];

/// Presentation filter category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PresentationFilter {
    #[default]
    All,
    Packages,
    Typst,
    Favorites,
}

/// Presentation list sort order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PresentationSort {
    #[default]
    Name,
    SlideCount,
    FileSize,
}

#[derive(Debug, Clone)]
pub enum Message {
    SearchChanged(String),
    SelectPresentation(usize),
    SelectNext,
    SelectPrevious,
    PlaySelected,
    PlaySelectedWithTheme(HudTheme),
    PlayEntry(PathBuf),
    PlayEntryWithTheme(PathBuf, HudTheme),
    PlayDemo,
    PlayDemoWithTheme(HudTheme),
    OpenFilePicker,
    OpenCustomOpenModal,
    CloseCustomOpenModal,
    CustomOpenPathChanged(String),
    BrowseCustomOpenPath,
    SetCustomOpenAnimation(String),
    ToggleCustomOpenFullscreen(bool),
    SetCustomOpenHudTheme(HudTheme),
    ExecuteCustomOpenPresentation(HudTheme),
    ExecuteCustomOpenInEditor,
    OpenSelectedInEditor,
    Refresh,
    ToggleTheme,
    SetAnimation(String),
    ToggleFullscreen(bool),
    ToggleFullscreenShortcut,
    SetHudTheme(HudTheme),
    OpenInEditor(PathBuf),
    CopyPath(PathBuf),
    InstallToSystem,
    DismissToast,
    EscapePressed,
    SetFilter(PresentationFilter),
    SetSort(PresentationSort),
    ToggleFavorite(PathBuf),
    VerifyPackageIntegrity(PathBuf),
}

pub struct ViewerLauncherApp {
    pub presentations: Vec<SlideFileEntry>,
    pub selected_index: Option<usize>,
    pub search_query: String,
    pub filter: PresentationFilter,
    pub sort: PresentationSort,
    pub integrity_report: Option<(PathBuf, slide_core::package::PackageVerificationReport)>,
    pub theme: ViewerTheme,
    pub selected_animation: String,
    pub is_fullscreen: bool,
    pub hud_theme: HudTheme,
    pub toast_message: Option<(String, Instant)>,
    pub current_dir: PathBuf,
    // Custom open modal state
    pub is_custom_open_open: bool,
    pub custom_open_path: String,
    pub custom_open_animation: String,
    pub custom_open_fullscreen: bool,
    pub custom_open_hud_theme: HudTheme,
}

impl ViewerLauncherApp {
    pub fn new(
        default_animation: String,
        fullscreen: bool,
    ) -> (Self, Task<Message>) {
        let presentations = crate::scan_local_presentations();
        let selected_index = if presentations.is_empty() {
            None
        } else {
            Some(0)
        };
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        (
            Self {
                presentations,
                selected_index,
                search_query: String::new(),
                filter: PresentationFilter::All,
                sort: PresentationSort::Name,
                integrity_report: None,
                theme: ViewerTheme::Dark,
                selected_animation: default_animation.clone(),
                is_fullscreen: fullscreen,
                hud_theme: HudTheme::Dark,
                toast_message: None,
                current_dir,
                is_custom_open_open: false,
                custom_open_path: String::new(),
                custom_open_animation: default_animation,
                custom_open_fullscreen: fullscreen,
                custom_open_hud_theme: HudTheme::Dark,
            },
            Task::none(),
        )
    }

    pub fn window_title(&self) -> &'static str {
        "Universal Presentation Launcher"
    }

    pub fn theme(&self) -> ViewerTheme {
        self.theme
    }

    pub fn update(
        &mut self,
        message: Message,
    ) -> Task<Message> {
        match message {
            | Message::SearchChanged(q) => {
                self.search_query = q;
                Task::none()
            },
            | Message::SelectPresentation(idx) => {
                if self.selected_index == Some(idx)
                    && let Some(entry) = self.filtered_presentations().get(idx)
                {
                    return self.launch_path_with_options(
                        entry.path.clone(),
                        self.hud_theme,
                        &self.selected_animation.clone(),
                        self.is_fullscreen,
                    );
                }
                self.selected_index = Some(idx);
                Task::none()
            },
            | Message::SelectNext => {
                let total = self.filtered_presentations().len();
                if total > 0 {
                    let next = match self.selected_index {
                        | Some(i) => (i + 1) % total,
                        | None => 0,
                    };
                    self.selected_index = Some(next);
                }
                Task::none()
            },
            | Message::SelectPrevious => {
                let total = self.filtered_presentations().len();
                if total > 0 {
                    let prev = match self.selected_index {
                        | Some(0) | None => total.saturating_sub(1),
                        | Some(i) => i - 1,
                    };
                    self.selected_index = Some(prev);
                }
                Task::none()
            },
            | Message::PlaySelected => {
                if let Some(idx) = self.selected_index
                    && let Some(entry) = self.filtered_presentations().get(idx)
                {
                    return self.launch_path_with_options(
                        entry.path.clone(),
                        self.hud_theme,
                        &self.selected_animation.clone(),
                        self.is_fullscreen,
                    );
                }
                Task::none()
            },
            | Message::PlaySelectedWithTheme(hud) => {
                if let Some(idx) = self.selected_index
                    && let Some(entry) = self.filtered_presentations().get(idx)
                {
                    return self.launch_path_with_options(
                        entry.path.clone(),
                        hud,
                        &self.selected_animation.clone(),
                        self.is_fullscreen,
                    );
                }
                Task::none()
            },
            | Message::PlayEntry(path) => {
                let hud = self.hud_theme;
                let anim = self.selected_animation.clone();
                let fs = self.is_fullscreen;
                self.launch_path_with_options(path, hud, &anim, fs)
            },
            | Message::PlayEntryWithTheme(path, hud) => {
                let anim = self.selected_animation.clone();
                let fs = self.is_fullscreen;
                self.launch_path_with_options(path, hud, &anim, fs)
            },
            | Message::PlayDemo => self.launch_demo_with_theme(self.hud_theme),
            | Message::PlayDemoWithTheme(hud) => self.launch_demo_with_theme(hud),
            | Message::OpenFilePicker => {
                if let Some(path) = pick_presentation_file() {
                    let hud = self.hud_theme;
                    let anim = self.selected_animation.clone();
                    let fs = self.is_fullscreen;
                    return self.launch_path_with_options(path, hud, &anim, fs);
                }
                Task::none()
            },
            | Message::OpenCustomOpenModal => {
                self.is_custom_open_open = true;
                if self.custom_open_path.is_empty()
                    && let Some(idx) = self.selected_index
                    && let Some(entry) = self.filtered_presentations().get(idx)
                {
                    self.custom_open_path = entry.path.to_string_lossy().to_string();
                }
                Task::none()
            },
            | Message::CloseCustomOpenModal => {
                self.is_custom_open_open = false;
                Task::none()
            },
            | Message::CustomOpenPathChanged(p) => {
                self.custom_open_path = p;
                Task::none()
            },
            | Message::BrowseCustomOpenPath => {
                if let Some(path) = pick_presentation_file() {
                    self.custom_open_path = path.to_string_lossy().to_string();
                }
                Task::none()
            },
            | Message::SetCustomOpenAnimation(anim) => {
                self.custom_open_animation = anim;
                Task::none()
            },
            | Message::ToggleCustomOpenFullscreen(val) => {
                self.custom_open_fullscreen = val;
                Task::none()
            },
            | Message::SetCustomOpenHudTheme(hud) => {
                self.custom_open_hud_theme = hud;
                Task::none()
            },
            | Message::ExecuteCustomOpenPresentation(hud) => {
                self.is_custom_open_open = false;
                let path = PathBuf::from(self.custom_open_path.trim());
                if path.exists() {
                    let anim = self.custom_open_animation.clone();
                    let fs = self.custom_open_fullscreen;
                    self.launch_path_with_options(path, hud, &anim, fs)
                } else {
                    self.show_toast("Specified presentation file does not exist");
                    Task::none()
                }
            },
            | Message::ExecuteCustomOpenInEditor => {
                self.is_custom_open_open = false;
                let path = PathBuf::from(self.custom_open_path.trim());
                if path.exists() {
                    match launch_slide_editor(&path) {
                        | Ok(()) => self.show_toast("Opened in Slide Editor"),
                        | Err(e) => self.show_toast(&format!("Failed to open editor: {}", e)),
                    }
                } else {
                    self.show_toast("Specified presentation file does not exist");
                }
                Task::none()
            },
            | Message::OpenSelectedInEditor => {
                if let Some(idx) = self.selected_index
                    && let Some(entry) = self.filtered_presentations().get(idx)
                {
                    match launch_slide_editor(&entry.path) {
                        | Ok(()) => self.show_toast("Opened in Slide Editor"),
                        | Err(e) => self.show_toast(&format!("Failed to open editor: {}", e)),
                    }
                }
                Task::none()
            },
            | Message::Refresh => {
                self.presentations = crate::scan_local_presentations();
                if self.selected_index.is_some()
                    && self.selected_index.unwrap() >= self.presentations.len()
                {
                    self.selected_index = if self.presentations.is_empty() {
                        None
                    } else {
                        Some(0)
                    };
                }
                self.show_toast("Refreshed local presentations");
                Task::none()
            },
            | Message::ToggleTheme => {
                self.theme = self.theme.toggle();
                Task::none()
            },
            | Message::SetAnimation(anim) => {
                self.selected_animation = anim;
                Task::none()
            },
            | Message::ToggleFullscreen(val) => {
                self.is_fullscreen = val;
                Task::none()
            },
            | Message::ToggleFullscreenShortcut => {
                self.is_fullscreen = !self.is_fullscreen;
                Task::none()
            },
            | Message::SetHudTheme(hud) => {
                self.hud_theme = hud;
                Task::none()
            },
            | Message::OpenInEditor(path) => {
                match launch_slide_editor(&path) {
                    | Ok(()) => self.show_toast("Opened in Slide Editor"),
                    | Err(e) => self.show_toast(&format!("Failed to open editor: {}", e)),
                }
                Task::none()
            },
            | Message::CopyPath(path) => {
                let s = path.to_string_lossy().to_string();
                self.show_toast(&format!("Path: {}", s));
                Task::none()
            },
            | Message::InstallToSystem => {
                match install_viewer_to_system() {
                    | Ok(msg) => {
                        self.show_toast(&format!("[OK] {}", msg));
                        Task::none()
                    },
                    | Err(e) => {
                        self.show_toast(&format!("[ERR] Failed to install: {}", e));
                        Task::none()
                    },
                }
            },
            | Message::DismissToast => {
                self.toast_message = None;
                Task::none()
            },
            | Message::EscapePressed => {
                if self.is_custom_open_open {
                    self.is_custom_open_open = false;
                } else {
                    self.toast_message = None;
                }
                Task::none()
            },
            | Message::SetFilter(f) => {
                self.filter = f;
                self.selected_index = Some(0);
                Task::none()
            },
            | Message::SetSort(s) => {
                self.sort = s;
                Task::none()
            },
            | Message::ToggleFavorite(path) => {
                let is_fav = crate::toggle_favorite_file(&path);
                for p in &mut self.presentations {
                    if p.path == path {
                        p.is_favorite = is_fav;
                    }
                }
                if is_fav {
                    self.show_toast("Added to Favorites");
                } else {
                    self.show_toast("Removed from Favorites");
                }
                Task::none()
            },
            | Message::VerifyPackageIntegrity(path) => {
                match slide_core::package::verify_package_integrity(&path) {
                    | Ok(report) => {
                        let msg = format!(
                            "[OK] Verified: {} slides, {} assets ({:.1} KB)",
                            report.slide_count,
                            report.asset_count,
                            report.file_size as f64 / 1024.0
                        );
                        self.show_toast(&msg);
                        self.integrity_report = Some((path, report));
                    },
                    | Err(e) => {
                        self.show_toast(&format!("[ERR] Verification failed: {}", e));
                    },
                }
                Task::none()
            },
        }
    }

    pub fn launch_path_with_options(
        &mut self,
        path: PathBuf,
        hud_theme: HudTheme,
        animation: &str,
        fullscreen: bool,
    ) -> Task<Message> {
        let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("slide-viewer"));
        let mut cmd = std::process::Command::new(current_exe);
        cmd.arg(&path);
        cmd.arg("--animation").arg(animation);
        if fullscreen {
            cmd.arg("--fullscreen");
        }
        if hud_theme.is_light() {
            cmd.arg("--light");
        }

        match cmd.spawn() {
            | Ok(_) => {
                let theme_str = if hud_theme.is_light() {
                    "Light"
                } else {
                    "Dark"
                };
                self.show_toast(&format!(
                    "Launched in {} HUD: {}",
                    theme_str,
                    path.file_name().unwrap_or_default().to_string_lossy()
                ));
            },
            | Err(e) => {
                self.show_toast(&format!("Failed to launch presentation: {}", e));
            },
        }
        Task::none()
    }

    pub fn launch_demo_with_theme(
        &mut self,
        hud_theme: HudTheme,
    ) -> Task<Message> {
        let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("slide-viewer"));
        let mut cmd = std::process::Command::new(current_exe);
        cmd.arg("--demo");
        cmd.arg("--animation").arg(&self.selected_animation);
        if self.is_fullscreen {
            cmd.arg("--fullscreen");
        }
        if hud_theme.is_light() {
            cmd.arg("--light");
        }

        match cmd.spawn() {
            | Ok(_) => {
                let theme_str = if hud_theme.is_light() {
                    "Light"
                } else {
                    "Dark"
                };
                self.show_toast(&format!("Launched Geek Demo showcase ({} HUD)", theme_str));
            },
            | Err(e) => {
                self.show_toast(&format!("Failed to launch demo: {}", e));
            },
        }
        Task::none()
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::event::listen_with(|event, _status, _id| {
            if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }) = event {
                match key.as_ref() {
                    | iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter)
                    | iced::keyboard::Key::Named(iced::keyboard::key::Named::Space) => {
                        Some(Message::PlaySelected)
                    },
                    | iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                        Some(Message::SelectNext)
                    },
                    | iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp) => {
                        Some(Message::SelectPrevious)
                    },
                    | iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) => {
                        Some(Message::EscapePressed)
                    },
                    | iced::keyboard::Key::Character("o") | iced::keyboard::Key::Character("O") => {
                        Some(Message::OpenCustomOpenModal)
                    },
                    | iced::keyboard::Key::Character("d") | iced::keyboard::Key::Character("D") => {
                        Some(Message::PlayDemo)
                    },
                    | iced::keyboard::Key::Character("e") | iced::keyboard::Key::Character("E") => {
                        Some(Message::OpenSelectedInEditor)
                    },
                    | iced::keyboard::Key::Character("i") | iced::keyboard::Key::Character("I") => {
                        Some(Message::InstallToSystem)
                    },
                    | iced::keyboard::Key::Character("l") | iced::keyboard::Key::Character("L") => {
                        Some(Message::ToggleTheme)
                    },
                    | iced::keyboard::Key::Named(iced::keyboard::key::Named::F11)
                    | iced::keyboard::Key::Character("f")
                    | iced::keyboard::Key::Character("F") => {
                        Some(Message::ToggleFullscreenShortcut)
                    },
                    | _ => None,
                }
            } else {
                None
            }
        })
    }

    fn show_toast(
        &mut self,
        text: &str,
    ) {
        self.toast_message = Some((text.to_string(), Instant::now()));
    }

    pub fn filtered_presentations(&self) -> Vec<&SlideFileEntry> {
        let mut list: Vec<&SlideFileEntry> = self
            .presentations
            .iter()
            .filter(|p| {
                match self.filter {
                    | PresentationFilter::All => true,
                    | PresentationFilter::Packages => p.is_slide_pkg,
                    | PresentationFilter::Typst => !p.is_slide_pkg,
                    | PresentationFilter::Favorites => p.is_favorite,
                }
            })
            .filter(|p| {
                if self.search_query.trim().is_empty() {
                    true
                } else {
                    let q = self.search_query.trim().to_lowercase();
                    p.name.to_lowercase().contains(&q)
                        || p.title.to_lowercase().contains(&q)
                        || p.path.to_string_lossy().to_lowercase().contains(&q)
                }
            })
            .collect();

        match self.sort {
            | PresentationSort::Name => {
                list.sort_by(|a, b| {
                    b.is_favorite
                        .cmp(&a.is_favorite)
                        .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
                });
            },
            | PresentationSort::SlideCount => {
                list.sort_by(|a, b| {
                    b.is_favorite
                        .cmp(&a.is_favorite)
                        .then_with(|| b.slides_count.cmp(&a.slides_count))
                });
            },
            | PresentationSort::FileSize => {
                list.sort_by(|a, b| {
                    let sz_a = std::fs::metadata(&a.path).map(|m| m.len()).unwrap_or(0);
                    let sz_b = std::fs::metadata(&b.path).map(|m| m.len()).unwrap_or(0);
                    b.is_favorite
                        .cmp(&a.is_favorite)
                        .then_with(|| sz_b.cmp(&sz_a))
                });
            },
        }

        list
    }

    pub fn view(&self) -> Element<'_, Message> {
        let theme = self.theme;

        // 1. Header Bar
        let brand_title = text("CARGO SLIDE").size(17).color(theme.accent());

        let brand_sub = text("Universal Presentation Viewer")
            .size(13)
            .color(theme.text_secondary());

        let brand_badge = container(text("v0.2.0").size(11).color(theme.accent()))
            .padding([2, 8])
            .style(move |_| {
                badge_container(
                    if theme.is_dark() {
                        Color::from_rgba(0.22, 0.74, 0.97, 0.15)
                    } else {
                        Color::from_rgba(0.01, 0.52, 0.78, 0.12)
                    },
                    theme.border_active(),
                )
            });

        let header_left = row![brand_title, brand_sub, brand_badge]
            .spacing(12)
            .align_y(Alignment::Center);

        let btn_open = button(text("OPEN FILE...").size(12))
            .style(move |_theme, _status| secondary_button_style(theme))
            .padding([6, 14])
            .on_press(Message::OpenFilePicker);

        let btn_custom = button(text("CUSTOM OPEN...").size(12))
            .style(move |_theme, _status| secondary_button_style(theme))
            .padding([6, 14])
            .on_press(Message::OpenCustomOpenModal);

        let btn_demo = button(text("RUN DEMO").size(12))
            .style(move |_theme, _status| primary_button_style(theme))
            .padding([6, 14])
            .on_press(Message::PlayDemo);

        let btn_refresh = button(text("REFRESH").size(12))
            .style(move |_theme, _status| secondary_button_style(theme))
            .padding([6, 12])
            .on_press(Message::Refresh);

        let theme_label = if theme.is_dark() {
            "LIGHT"
        } else {
            "DARK"
        };
        let btn_theme = button(text(theme_label).size(12))
            .style(move |_theme, _status| secondary_button_style(theme))
            .padding([6, 12])
            .on_press(Message::ToggleTheme);

        let header_right = row![btn_open, btn_custom, btn_demo, btn_refresh, btn_theme]
            .spacing(10)
            .align_y(Alignment::Center);

        let header_bar = container(
            row![header_left, Space::new().width(Length::Fill), header_right]
                .padding([14, 24])
                .align_y(Alignment::Center),
        )
        .style(move |_| header_container(theme))
        .width(Length::Fill);

        // 2. Main Content Split View
        let filtered = self.filtered_presentations();

        // Left Panel: Presentation List
        let search_box = text_input("Search local presentations...", &self.search_query)
            .on_input(Message::SearchChanged)
            .padding([10, 14])
            .size(13)
            .style(move |_theme, _status| search_input_style(theme));

        let mut cards_col = column![].spacing(12);

        if filtered.is_empty() {
            let empty_card = container(
                column![
                    text("No presentations found in current directory").size(15).color(theme.text_secondary()),
                    Space::new().height(6),
                    text("Use 'CUSTOM OPEN...' or 'OPEN FILE...' to select any presentation file, or run the demo.")
                        .size(12)
                        .color(theme.text_secondary()),
                    Space::new().height(14),
                    row![
                        button(text("CUSTOM OPEN...").size(12))
                            .style(move |_theme, _status| primary_button_style(theme))
                            .padding([8, 16])
                            .on_press(Message::OpenCustomOpenModal),
                        button(text("OPEN FILE...").size(12))
                            .style(move |_theme, _status| secondary_button_style(theme))
                            .padding([8, 16])
                            .on_press(Message::OpenFilePicker),
                        button(text("RUN SHOWCASE DEMO").size(12))
                            .style(move |_theme, _status| secondary_button_style(theme))
                            .padding([8, 16])
                            .on_press(Message::PlayDemo),
                    ]
                    .spacing(10)
                ]
                .align_x(Alignment::Center),
            )
            .padding(32)
            .width(Length::Fill)
            .style(move |_| card_container(theme, false));

            cards_col = cards_col.push(empty_card);
        } else {
            for (idx, entry) in filtered.iter().enumerate() {
                let is_selected = self.selected_index == Some(idx);
                let entry_path = entry.path.clone();

                let type_badge = if entry.is_slide_pkg {
                    container(text(".SLIDE PACKAGE").size(10).color(theme.accent()))
                        .padding([2, 8])
                        .style(move |_| {
                            badge_container(
                                if theme.is_dark() {
                                    Color::from_rgba(0.22, 0.74, 0.97, 0.15)
                                } else {
                                    Color::from_rgba(0.01, 0.52, 0.78, 0.12)
                                },
                                theme.border_active(),
                            )
                        })
                } else {
                    container(text(".TYP SOURCE").size(10).color(theme.accent_purple()))
                        .padding([2, 8])
                        .style(move |_| {
                            badge_container(
                                if theme.is_dark() {
                                    Color::from_rgba(0.65, 0.48, 0.98, 0.15)
                                } else {
                                    Color::from_rgba(0.48, 0.22, 0.93, 0.12)
                                },
                                if theme.is_dark() {
                                    Color::from_rgb(0.655, 0.482, 0.980)
                                } else {
                                    Color::from_rgb(0.486, 0.227, 0.929)
                                },
                            )
                        })
                };

                let slides_badge = container(
                    text(format!("{} SLIDES", entry.slides_count))
                        .size(11)
                        .color(theme.text_secondary()),
                )
                .padding([2, 8])
                .style(move |_| badge_container(theme.bg_subtle(), theme.border_color()));

                let size_badge =
                    container(text(&entry.size_str).size(11).color(theme.text_secondary()))
                        .padding([2, 8])
                        .style(move |_| badge_container(theme.bg_subtle(), theme.border_color()));

                let mut meta_badges = row![slides_badge, size_badge].spacing(6);

                if let Some(ref ratio) = entry.aspect_ratio {
                    let ratio_badge = container(text(ratio).size(11).color(theme.text_secondary()))
                        .padding([2, 8])
                        .style(move |_| badge_container(theme.bg_subtle(), theme.border_color()));
                    meta_badges = meta_badges.push(ratio_badge);
                }

                if entry.has_notes {
                    let notes_badge = container(text("NOTES").size(10).color(theme.accent()))
                        .padding([2, 8])
                        .style(move |_| badge_container(theme.bg_subtle(), theme.border_color()));
                    meta_badges = meta_badges.push(notes_badge);
                }

                let play_dark_btn = button(text("▶ DARK").size(10))
                    .style(move |_theme, _status| primary_button_style(theme))
                    .padding([4, 8])
                    .on_press(Message::PlayEntryWithTheme(
                        entry_path.clone(),
                        HudTheme::Dark,
                    ));

                let play_light_btn = button(text("▶ LIGHT").size(10))
                    .style(move |_theme, _status| secondary_button_style(theme))
                    .padding([4, 8])
                    .on_press(Message::PlayEntryWithTheme(
                        entry_path.clone(),
                        HudTheme::Light,
                    ));

                let edit_btn = button(text("EDIT").size(10))
                    .style(move |_theme, _status| secondary_button_style(theme))
                    .padding([4, 8])
                    .on_press(Message::OpenInEditor(entry_path.clone()));

                let fav_icon = if entry.is_favorite {
                    "[FAV]"
                } else {
                    "[ + ]"
                };
                let fav_color = if entry.is_favorite {
                    Color::from_rgb(0.96, 0.72, 0.15)
                } else {
                    theme.text_secondary()
                };
                let fav_btn = button(text(fav_icon).size(16).color(fav_color))
                    .style(move |_t, _s| {
                        button::Style {
                            background: Some(Background::Color(Color::TRANSPARENT)),
                            text_color: fav_color,
                            border: Border::default(),
                            shadow: Shadow::default(),
                            snap: true,
                        }
                    })
                    .padding([0, 4])
                    .on_press(Message::ToggleFavorite(entry_path.clone()));

                let select_header_btn = button(
                    row![
                        text(&entry.title).size(15).color(theme.text_primary()),
                        Space::new().width(Length::Fill),
                        type_badge,
                    ]
                    .align_y(Alignment::Center),
                )
                .style(move |_t, _s| {
                    button::Style {
                        background: Some(Background::Color(Color::TRANSPARENT)),
                        text_color: theme.text_primary(),
                        border: Border::default(),
                        shadow: Shadow::default(),
                        snap: true,
                    }
                })
                .width(Length::Fill)
                .padding(0)
                .on_press(Message::SelectPresentation(idx));

                let card_header = row![fav_btn, select_header_btn].align_y(Alignment::Center);

                let card_content = column![
                    card_header,
                    Space::new().height(4),
                    text(entry.path.to_string_lossy().to_string())
                        .size(11)
                        .color(theme.text_secondary()),
                    Space::new().height(10),
                    row![
                        meta_badges,
                        Space::new().width(Length::Fill),
                        play_dark_btn,
                        play_light_btn,
                        edit_btn
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                ];

                let card_box = container(card_content)
                    .padding(16)
                    .width(Length::Fill)
                    .style(move |_| card_container(theme, is_selected));

                cards_col = cards_col.push(card_box);
            }
        }

        // Filter and sort toolbar
        let filter_all_btn = button(text("All").size(11))
            .style(move |_t, _s| chip_button_style(theme, self.filter == PresentationFilter::All))
            .padding([3, 8])
            .on_press(Message::SetFilter(PresentationFilter::All));

        let filter_pkg_btn = button(text("Packages").size(11))
            .style(move |_t, _s| {
                chip_button_style(theme, self.filter == PresentationFilter::Packages)
            })
            .padding([3, 8])
            .on_press(Message::SetFilter(PresentationFilter::Packages));

        let filter_typ_btn = button(text("Typst").size(11))
            .style(move |_t, _s| chip_button_style(theme, self.filter == PresentationFilter::Typst))
            .padding([3, 8])
            .on_press(Message::SetFilter(PresentationFilter::Typst));

        let filter_fav_btn = button(text("Favorites").size(11))
            .style(move |_t, _s| {
                chip_button_style(theme, self.filter == PresentationFilter::Favorites)
            })
            .padding([3, 8])
            .on_press(Message::SetFilter(PresentationFilter::Favorites));

        let sort_label = text("Sort:").size(11).color(theme.text_secondary());
        let sort_name_btn = button(text("Name").size(10))
            .style(move |_t, _s| chip_button_style(theme, self.sort == PresentationSort::Name))
            .padding([2, 6])
            .on_press(Message::SetSort(PresentationSort::Name));
        let sort_count_btn = button(text("Slides").size(10))
            .style(move |_t, _s| {
                chip_button_style(theme, self.sort == PresentationSort::SlideCount)
            })
            .padding([2, 6])
            .on_press(Message::SetSort(PresentationSort::SlideCount));
        let sort_size_btn = button(text("Size").size(10))
            .style(move |_t, _s| chip_button_style(theme, self.sort == PresentationSort::FileSize))
            .padding([2, 6])
            .on_press(Message::SetSort(PresentationSort::FileSize));

        let filter_sort_bar = row![
            filter_all_btn,
            filter_pkg_btn,
            filter_typ_btn,
            filter_fav_btn,
            Space::new().width(Length::Fill),
            sort_label,
            sort_name_btn,
            sort_count_btn,
            sort_size_btn,
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let left_panel = column![
            row![
                text("PRESENTATION LIBRARY")
                    .size(13)
                    .color(theme.text_secondary()),
                Space::new().width(Length::Fill),
                text(format!("{} items", filtered.len()))
                    .size(12)
                    .color(theme.text_secondary()),
            ]
            .align_y(Alignment::Center),
            search_box,
            filter_sort_bar,
            scrollable(cards_col).height(Length::Fill),
        ]
        .spacing(12)
        .width(Length::FillPortion(3));

        // Right Panel: Details & Playback Controls
        let selected_entry = self
            .selected_index
            .and_then(|idx| filtered.get(idx).copied());

        let right_panel_content: Element<'_, Message> = if let Some(entry) = selected_entry {
            let details_title = text(&entry.title).size(17).color(theme.text_primary());

            let details_path = text(entry.path.to_string_lossy().to_string())
                .size(11)
                .color(theme.text_secondary());

            let stats_row = row![
                column![
                    text("SLIDES").size(10).color(theme.text_secondary()),
                    text(format!("{}", entry.slides_count))
                        .size(14)
                        .color(theme.accent()),
                ],
                Space::new().width(24),
                column![
                    text("PACKAGE SIZE").size(10).color(theme.text_secondary()),
                    text(&entry.size_str).size(14).color(theme.text_primary()),
                ],
                Space::new().width(24),
                column![
                    text("FORMAT").size(10).color(theme.text_secondary()),
                    text(if entry.is_slide_pkg {
                        ".slide"
                    } else {
                        ".typ"
                    })
                    .size(14)
                    .color(theme.text_primary()),
                ],
            ];

            let mut extra_metadata_col = column![].spacing(6);

            if let Some(ref auth) = entry.author {
                extra_metadata_col = extra_metadata_col.push(row![
                    text("Author:").size(11).color(theme.text_secondary()),
                    Space::new().width(8),
                    text(auth).size(12).color(theme.text_primary()),
                ]);
            }

            let notes_status_text = if entry.has_notes {
                "[OK] Includes presenter notes"
            } else {
                "No speaker notes"
            };
            extra_metadata_col = extra_metadata_col.push(row![
                text("Speaker Notes:")
                    .size(11)
                    .color(theme.text_secondary()),
                Space::new().width(8),
                text(notes_status_text).size(12).color(if entry.has_notes {
                    theme.accent()
                } else {
                    theme.text_secondary()
                }),
            ]);

            if let Some(ref ratio) = entry.aspect_ratio {
                extra_metadata_col = extra_metadata_col.push(row![
                    text("Aspect Ratio:").size(11).color(theme.text_secondary()),
                    Space::new().width(8),
                    text(ratio).size(12).color(theme.text_primary()),
                ]);
            }

            if entry.is_slide_pkg {
                let verify_btn = button(text("VERIFY PACKAGE INTEGRITY").size(11))
                    .style(move |_theme, _status| secondary_button_style(theme))
                    .padding([5, 12])
                    .on_press(Message::VerifyPackageIntegrity(entry.path.clone()));

                let report_view = if let Some((ref r_path, ref rep)) = self.integrity_report {
                    if r_path == &entry.path {
                        container(
                            column![
                                text("[OK] Package Integrity Verified")
                                    .size(12)
                                    .color(theme.accent()),
                                text(format!(
                                    "• Slide Count: {} slides (verified)",
                                    rep.slide_count
                                ))
                                .size(11)
                                .color(theme.text_secondary()),
                                text(format!("• Assets: {} files embedded", rep.asset_count))
                                    .size(11)
                                    .color(theme.text_secondary()),
                                text("• Archive Format: Valid LZMA2 TAR")
                                    .size(11)
                                    .color(theme.text_secondary()),
                            ]
                            .spacing(3),
                        )
                        .padding(8)
                        .style(move |_| {
                            container::Style {
                                background: Some(Background::Color(theme.bg_subtle())),
                                border: Border {
                                    color: theme.border_active(),
                                    width: 1.0,
                                    radius: iced::border::Radius::from(4.0),
                                },
                                ..container::Style::default()
                            }
                        })
                    } else {
                        container(column![verify_btn])
                    }
                } else {
                    container(column![verify_btn])
                };

                extra_metadata_col = extra_metadata_col.push(report_view);
            }

            let section_label = text("PLAYBACK & EDITOR CONTROLS")
                .size(11)
                .color(theme.text_secondary());

            let anim_picker = row![
                text("Animation:").size(12).color(theme.text_secondary()),
                Space::new().width(Length::Fill),
                pick_list(
                    AVAILABLE_ANIMATIONS,
                    Some(self.selected_animation.as_str()),
                    |s: &str| Message::SetAnimation(s.to_string())
                )
                .style(move |_theme, _status| pick_list_style(theme))
                .padding([6, 12])
                .text_size(12)
            ]
            .align_y(Alignment::Center);

            let hud_theme_selector = row![
                text("Default HUD Theme:")
                    .size(12)
                    .color(theme.text_secondary()),
                Space::new().width(Length::Fill),
                button(text("DARK").size(11))
                    .style(move |_theme, _status| {
                        chip_button_style(theme, self.hud_theme == HudTheme::Dark)
                    })
                    .padding([4, 10])
                    .on_press(Message::SetHudTheme(HudTheme::Dark)),
                button(text("LIGHT").size(11))
                    .style(move |_theme, _status| {
                        chip_button_style(theme, self.hud_theme == HudTheme::Light)
                    })
                    .padding([4, 10])
                    .on_press(Message::SetHudTheme(HudTheme::Light)),
            ]
            .spacing(6)
            .align_y(Alignment::Center);

            let fullscreen_cb = row![
                checkbox(self.is_fullscreen)
                    .on_toggle(Message::ToggleFullscreen)
                    .size(16),
                text("Start directly in Fullscreen")
                    .size(12)
                    .color(theme.text_secondary()),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            let start_dark_btn = button(text("▶ START IN DARK HUD").size(13))
                .style(move |_theme, _status| primary_button_style(theme))
                .padding([11, 20])
                .width(Length::Fill)
                .on_press(Message::PlaySelectedWithTheme(HudTheme::Dark));

            let start_light_btn = button(text("▶ START IN LIGHT HUD").size(13))
                .style(move |_theme, _status| secondary_button_style(theme))
                .padding([11, 20])
                .width(Length::Fill)
                .on_press(Message::PlaySelectedWithTheme(HudTheme::Light));

            let editor_btn = button(text("OPEN IN SLIDE EDITOR").size(13))
                .style(move |_theme, _status| secondary_button_style(theme))
                .padding([9, 16])
                .width(Length::Fill)
                .on_press(Message::OpenInEditor(entry.path.clone()));

            let copy_btn = button(text("COPY PATH").size(11))
                .style(move |_theme, _status| secondary_button_style(theme))
                .padding([6, 12])
                .on_press(Message::CopyPath(entry.path.clone()));

            let install_btn = button(text("INSTALL FILE ASSOCIATIONS").size(11))
                .style(move |_theme, _status| secondary_button_style(theme))
                .padding([6, 12])
                .on_press(Message::InstallToSystem);

            let secondary_actions = row![copy_btn, Space::new().width(Length::Fill), install_btn]
                .align_y(Alignment::Center);

            column![
                details_title,
                details_path,
                Space::new().height(8),
                stats_row,
                Space::new().height(8),
                extra_metadata_col,
                Space::new().height(12),
                section_label,
                anim_picker,
                hud_theme_selector,
                fullscreen_cb,
                Space::new().height(16),
                start_dark_btn,
                start_light_btn,
                editor_btn,
                Space::new().height(10),
                secondary_actions,
            ]
            .spacing(10)
            .into()
        } else {
            column![
                text("SELECT A PRESENTATION")
                    .size(14)
                    .color(theme.text_secondary()),
                Space::new().height(8),
                text("Choose a presentation from the list on the left to view details and launch.")
                    .size(12)
                    .color(theme.text_secondary()),
                Space::new().height(24),
                row![
                    button(text("CUSTOM OPEN...").size(12))
                        .style(move |_theme, _status| primary_button_style(theme))
                        .padding([8, 16])
                        .on_press(Message::OpenCustomOpenModal),
                    button(text("OPEN FILE...").size(12))
                        .style(move |_theme, _status| secondary_button_style(theme))
                        .padding([8, 16])
                        .on_press(Message::OpenFilePicker),
                ]
                .spacing(10),
            ]
            .into()
        };

        let right_sidebar = container(right_panel_content)
            .padding(24)
            .width(Length::FillPortion(2))
            .style(move |_| sidebar_container(theme));

        let main_content = row![left_panel, right_sidebar]
            .spacing(24)
            .padding([20, 24])
            .height(Length::Fill);

        // 3. Toast Message (if active)
        let toast_view: Element<'_, Message> = if let Some((ref msg, ref t)) = self.toast_message {
            if t.elapsed().as_secs() < 4 {
                container(
                    row![
                        text(msg).size(12).color(theme.accent()),
                        Space::new().width(12),
                        button(text("X").size(11))
                            .style(move |_theme, _status| secondary_button_style(theme))
                            .padding([2, 6])
                            .on_press(Message::DismissToast),
                    ]
                    .align_y(Alignment::Center),
                )
                .padding([6, 14])
                .style(move |_| {
                    badge_container(
                        if theme.is_dark() {
                            Color::from_rgba(0.08, 0.12, 0.18, 0.95)
                        } else {
                            Color::from_rgba(1.0, 1.0, 1.0, 0.95)
                        },
                        theme.border_active(),
                    )
                })
                .into()
            } else {
                Space::new().height(0).into()
            }
        } else {
            Space::new().height(0).into()
        };

        // 4. Footer Bar with Presenter Shortcuts Guide
        let shortcut_item = |k: &'static str, desc: &'static str| -> Element<'_, Message> {
            row![
                container(text(k).size(10).color(theme.text_primary()))
                    .padding([2, 6])
                    .style(move |_| badge_container(theme.bg_subtle(), theme.border_color())),
                text(desc).size(11).color(theme.text_secondary()),
            ]
            .spacing(4)
            .align_y(Alignment::Center)
            .into()
        };

        let shortcuts_guide = row![
            shortcut_item("Enter/Space", "Start"),
            shortcut_item("O", "Custom Open"),
            shortcut_item("E", "Edit in Slide Editor"),
            shortcut_item("D", "Demo"),
            shortcut_item("F", "Fullscreen"),
            shortcut_item("L", "Launcher Theme"),
            shortcut_item("I", "Install"),
            shortcut_item("Esc", "Exit"),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let footer_left = text(format!("Dir: {}", self.current_dir.display()))
            .size(11)
            .color(theme.text_secondary());

        let footer_bar = container(
            row![
                footer_left,
                Space::new().width(Length::Fill),
                shortcuts_guide
            ]
            .padding([8, 24])
            .align_y(Alignment::Center),
        )
        .style(move |_| footer_container(theme))
        .width(Length::Fill);

        // Assemble root layout
        let base_content: Element<'_, Message> =
            container(column![header_bar, toast_view, main_content, footer_bar,])
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| canvas_container(theme))
                .into();

        if self.is_custom_open_open {
            let modal_card = container(
                column![
                    row![
                        text("CUSTOM PRESENTATION LAUNCHER").size(15).color(theme.accent()),
                        Space::new().width(Length::Fill),
                        button(text("X").size(12))
                            .style(move |_theme, _status| secondary_button_style(theme))
                            .padding([2, 8])
                            .on_press(Message::CloseCustomOpenModal),
                    ]
                    .align_y(Alignment::Center),
                    Space::new().height(8),
                    text("Specify any presentation path to launch with custom options or open in Slide Editor:")
                        .size(12)
                        .color(theme.text_secondary()),
                    Space::new().height(16),
                    // Path Input
                    row![
                        text_input("Enter path to .slide, .typ, or deck.json...", &self.custom_open_path)
                            .on_input(Message::CustomOpenPathChanged)
                            .padding([8, 12])
                            .size(13)
                            .style(move |_theme, _status| search_input_style(theme))
                            .width(Length::Fill),
                        button(text("BROWSE...").size(12))
                            .style(move |_theme, _status| secondary_button_style(theme))
                            .padding([8, 14])
                            .on_press(Message::BrowseCustomOpenPath),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                    Space::new().height(14),
                    // Settings Row
                    row![
                        text("HUD Theme:").size(12).color(theme.text_secondary()),
                        button(text("DARK HUD").size(11))
                            .style(move |_theme, _status| chip_button_style(theme, self.custom_open_hud_theme == HudTheme::Dark))
                            .padding([4, 10])
                            .on_press(Message::SetCustomOpenHudTheme(HudTheme::Dark)),
                        button(text("LIGHT HUD").size(11))
                            .style(move |_theme, _status| chip_button_style(theme, self.custom_open_hud_theme == HudTheme::Light))
                            .padding([4, 10])
                            .on_press(Message::SetCustomOpenHudTheme(HudTheme::Light)),
                        Space::new().width(12),
                        text("Animation:").size(12).color(theme.text_secondary()),
                        pick_list(
                            AVAILABLE_ANIMATIONS,
                            Some(self.custom_open_animation.as_str()),
                            |s: &str| Message::SetCustomOpenAnimation(s.to_string())
                        )
                        .style(move |_theme, _status| pick_list_style(theme))
                        .padding([4, 8])
                        .text_size(12),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                    Space::new().height(12),
                    row![
                        checkbox(self.custom_open_fullscreen)
                            .on_toggle(Message::ToggleCustomOpenFullscreen)
                            .size(16),
                        text("Start directly in Fullscreen").size(12).color(theme.text_secondary()),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                    Space::new().height(22),
                    // Action Buttons Row
                    row![
                        button(text("▶ START (DARK HUD)").size(12))
                            .style(move |_theme, _status| primary_button_style(theme))
                            .padding([9, 14])
                            .on_press(Message::ExecuteCustomOpenPresentation(HudTheme::Dark)),
                        button(text("▶ START (LIGHT HUD)").size(12))
                            .style(move |_theme, _status| secondary_button_style(theme))
                            .padding([9, 14])
                            .on_press(Message::ExecuteCustomOpenPresentation(HudTheme::Light)),
                        button(text("OPEN IN SLIDE EDITOR").size(12))
                            .style(move |_theme, _status| secondary_button_style(theme))
                            .padding([9, 14])
                            .on_press(Message::ExecuteCustomOpenInEditor),
                        Space::new().width(Length::Fill),
                        button(text("CANCEL").size(12))
                            .style(move |_theme, _status| secondary_button_style(theme))
                            .padding([9, 14])
                            .on_press(Message::CloseCustomOpenModal),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                ]
            )
            .padding(24)
            .width(Length::Fixed(680.0))
            .style(move |_| modal_dialog_style(theme));

            let modal_overlay: Element<'_, Message> = container(modal_card)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(move |_| modal_backdrop_style(theme))
                .into();

            iced::widget::stack![base_content, modal_overlay].into()
        } else {
            iced::widget::stack![base_content].into()
        }
    }
}

/// Locate and launch slide-editor binary using multi-tier fallback strategy
pub fn launch_slide_editor(path: &std::path::Path) -> std::io::Result<()> {
    // 1. Sibling binary to the current executable (e.g. target/debug/slide-editor)
    if let Ok(current_exe) = std::env::current_exe()
        && let Some(parent) = current_exe.parent()
    {
        let sibling = parent.join("slide-editor");
        if sibling.is_file()
            && let Ok(child) = std::process::Command::new(&sibling).arg(path).spawn()
        {
            drop(child);
            return Ok(());
        }
    }

    // 2. User local directories ~/.cargo/bin and ~/.local/bin
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        let cargo_bin = home.join(".cargo/bin/slide-editor");
        if cargo_bin.is_file()
            && let Ok(child) = std::process::Command::new(&cargo_bin).arg(path).spawn()
        {
            drop(child);
            return Ok(());
        }
        let local_bin = home.join(".local/bin/slide-editor");
        if local_bin.is_file()
            && let Ok(child) = std::process::Command::new(&local_bin).arg(path).spawn()
        {
            drop(child);
            return Ok(());
        }
    }

    // 3. System PATH lookup
    if let Ok(child) = std::process::Command::new("slide-editor").arg(path).spawn() {
        drop(child);
        return Ok(());
    }

    // 4. Fallback to default system application association via open
    open::that(path).map_err(|e| std::io::Error::new(std::io::ErrorKind::NotFound, e.to_string()))
}
