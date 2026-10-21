//! Multi-window application coordinator and event router for Slide Editor.
//!
//! Provides top-level daemon lifecycle management, routing keyboard/mouse interactions
//! to individual window instances, opening new windows, intercepting window close
//! events, and prompting the user with an unsaved changes confirmation modal
//! whenever a dirty document window is closed.

use crate::app::ActiveModal;
use crate::app::Message;
use crate::app::SlideEditorApp;
use crate::app::handle_global_drag_event;
use crate::ui::theme::AppTheme;
use iced::Element;
use iced::Subscription;
use iced::Task;
use std::collections::HashMap;
use std::path::PathBuf;

/// Messages routed across multiple editor windows
#[derive(Debug, Clone)]
pub enum MultiWindowMessage {
    /// Message targeted to or received from a specific window
    WindowMessage(iced::window::Id, Message),
    /// OS close button pressed or close requested for a window
    CloseRequested(iced::window::Id),
    /// Notification that a window was closed by the platform runtime
    WindowClosed(iced::window::Id),
    /// Request to create and open a new editor window
    OpenNewWindow(Option<PathBuf>),
}

/// Coordinator managing multiple independent Slide Editor windows
pub struct MultiWindowApp {
    /// Map of active window IDs to their editor instances
    pub windows: HashMap<iced::window::Id, SlideEditorApp>,
    /// Default dark mode setting for newly opened windows
    pub dark_mode: bool,
}

impl MultiWindowApp {
    /// Initialize daemon state and open the primary initial window
    pub fn boot(
        initial_file: Option<PathBuf>,
        dark_mode: bool,
    ) -> (Self, Task<MultiWindowMessage>) {
        let (first_id, open_task) = iced::window::open(iced::window::Settings {
            size: iced::Size::new(1280.0, 820.0),
            position: iced::window::Position::Centered,
            resizable: true,
            ..Default::default()
        });

        let mut app = SlideEditorApp::new(initial_file, dark_mode);
        app.window_id = Some(first_id);

        let mut windows = HashMap::new();
        windows.insert(first_id, app);

        let state = Self { windows, dark_mode };
        (state, open_task.discard())
    }

    /// Open a new editor window, optionally with a pre-loaded presentation file
    pub fn open_window(
        &mut self,
        file: Option<PathBuf>,
    ) -> Task<MultiWindowMessage> {
        let (new_id, open_task) = iced::window::open(iced::window::Settings {
            size: iced::Size::new(1280.0, 820.0),
            position: iced::window::Position::Centered,
            resizable: true,
            ..Default::default()
        });

        let mut app = SlideEditorApp::new(file, self.dark_mode);
        app.window_id = Some(new_id);
        self.windows.insert(new_id, app);

        open_task.discard()
    }

    /// Handle a window close request with unsaved changes verification.
    ///
    /// If the window's document has unsaved edits (`is_dirty`), an interactive
    /// confirmation dialog (`ActiveModal::UnsavedChangesConfirm`) is activated.
    /// Otherwise, the window is closed immediately. If no windows remain,
    /// the application process terminates via `iced::exit()`.
    pub fn handle_close_request(
        &mut self,
        window_id: iced::window::Id,
    ) -> Task<MultiWindowMessage> {
        if let Some(app) = self.windows.get_mut(&window_id) {
            app.commit_all_editor_states();
            if app.doc.is_dirty {
                app.active_modal = Some(ActiveModal::UnsavedChangesConfirm {
                    target_window_id: Some(window_id),
                });
                Task::none()
            } else {
                self.windows.remove(&window_id);
                let close_task = iced::window::close(window_id);
                if self.windows.is_empty() {
                    Task::batch(vec![close_task, iced::exit()])
                } else {
                    close_task
                }
            }
        } else {
            Task::none()
        }
    }

    /// Update daemon and window states in response to routed messages
    pub fn update(
        &mut self,
        message: MultiWindowMessage,
    ) -> Task<MultiWindowMessage> {
        match message {
            | MultiWindowMessage::CloseRequested(id) => self.handle_close_request(id),
            | MultiWindowMessage::WindowClosed(id) => {
                self.windows.remove(&id);
                if self.windows.is_empty() {
                    iced::exit()
                } else {
                    Task::none()
                }
            },
            | MultiWindowMessage::OpenNewWindow(path) => self.open_window(path),
            | MultiWindowMessage::WindowMessage(_id, Message::NewWindow(path)) => {
                self.open_window(path)
            },
            | MultiWindowMessage::WindowMessage(id, Message::RequestCloseWindow) => {
                self.handle_close_request(id)
            },
            | MultiWindowMessage::WindowMessage(id, Message::ForceCloseWindow(target_opt)) => {
                let target = target_opt.unwrap_or(id);
                self.windows.remove(&target);
                let close_task = iced::window::close(target);
                if self.windows.is_empty() {
                    Task::batch(vec![close_task, iced::exit()])
                } else {
                    close_task
                }
            },
            | MultiWindowMessage::WindowMessage(id, Message::SaveAndCloseWindow(target_opt)) => {
                let target = target_opt.unwrap_or(id);
                let mut tasks = Vec::new();
                if let Some(app) = self.windows.get_mut(&target) {
                    app.commit_all_editor_states();
                    let save_task = app.update(Message::SaveDocument);
                    tasks
                        .push(save_task.map(move |m| MultiWindowMessage::WindowMessage(target, m)));
                }
                self.windows.remove(&target);
                tasks.push(iced::window::close(target));
                if self.windows.is_empty() {
                    tasks.push(iced::exit());
                }
                Task::batch(tasks)
            },
            | MultiWindowMessage::WindowMessage(id, Message::CancelCloseWindow) => {
                if let Some(app) = self.windows.get_mut(&id) {
                    app.active_modal = None;
                }
                Task::none()
            },
            | MultiWindowMessage::WindowMessage(id, msg) => {
                if let Some(app) = self.windows.get_mut(&id) {
                    let task = app.update(msg);
                    task.map(move |m| MultiWindowMessage::WindowMessage(id, m))
                } else {
                    Task::none()
                }
            },
        }
    }

    /// Render UI element for the requested window
    #[must_use]
    pub fn view<'a>(
        &'a self,
        window: iced::window::Id,
    ) -> Element<'a, MultiWindowMessage> {
        if let Some(app) = self.windows.get(&window) {
            app.view()
                .map(move |msg| MultiWindowMessage::WindowMessage(window, msg))
        } else {
            iced::widget::Space::new().into()
        }
    }

    /// Determine window title reflecting filename and dirty indicator
    #[must_use]
    pub fn title(
        &self,
        window: iced::window::Id,
    ) -> String {
        if let Some(sub_app) = self.windows.get(&window) {
            let dirty = if sub_app.doc.is_dirty {
                "* "
            } else {
                ""
            };
            format!(
                "{}{} - Slide Editor ({})",
                dirty,
                sub_app.doc.title,
                sub_app.doc.format.label()
            )
        } else {
            "Slide Editor".to_string()
        }
    }

    /// Determine theme for the requested window
    #[must_use]
    pub fn theme(
        &self,
        window: iced::window::Id,
    ) -> iced::Theme {
        if let Some(sub_app) = self.windows.get(&window) {
            match sub_app.theme {
                | AppTheme::Light => iced::Theme::Light,
                | AppTheme::Dark => iced::Theme::Dark,
            }
        } else if self.dark_mode {
            iced::Theme::Dark
        } else {
            iced::Theme::Light
        }
    }

    /// Subscriptions for multi-window daemon: intercepts close requests,
    /// catches window terminations, and routes input events.
    pub fn subscription(&self) -> Subscription<MultiWindowMessage> {
        Subscription::batch(vec![
            iced::window::close_requests().map(MultiWindowMessage::CloseRequested),
            iced::window::close_events().map(MultiWindowMessage::WindowClosed),
            iced::event::listen_with(|event, status, window| {
                handle_global_drag_event(event, status, window)
                    .map(move |msg| MultiWindowMessage::WindowMessage(window, msg))
            }),
        ])
    }

    /// Total count of active open windows
    #[must_use]
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    /// Get list of active window IDs
    #[must_use]
    pub fn window_ids(&self) -> Vec<iced::window::Id> {
        self.windows.keys().copied().collect()
    }

    /// Access reference to editor instance for a given window ID
    #[must_use]
    pub fn get_window(
        &self,
        id: &iced::window::Id,
    ) -> Option<&SlideEditorApp> {
        self.windows.get(id)
    }

    /// Access mutable reference to editor instance for a given window ID
    pub fn get_window_mut(
        &mut self,
        id: &iced::window::Id,
    ) -> Option<&mut SlideEditorApp> {
        self.windows.get_mut(id)
    }
}
