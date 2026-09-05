use crate::audio::AudioEngine;
use crate::hud::ChartInspectorState;
use crate::hud::DockAction;
use crate::hud::InkStroke;
use crate::hud::InspectorAction;
use crate::hud::PALETTE_COLORS;
use crate::hud::PresenterMode;
use crate::hud::draw_chart_inspector;
use crate::hud::draw_chart_visualizer;
use crate::hud::draw_help_overlay;
use crate::hud::draw_hotspot_highlight;
use crate::hud::draw_page_badge;
use crate::hud::draw_volume_toast;
use crate::hud::get_chart_quick_action_rects;
use crate::hud::get_dock_rects;
use crate::hud::get_inspector_layout;
use crate::hud::get_volume_slider_rects;
use crate::hud::hit_test_chart_inspector;
use crate::hud::hit_test_dock;
use crate::hud::hit_test_palette;
use crate::hud::hit_test_volume_slider;
use crate::hud::render_chart_hover;
use crate::hud::render_dock;
use crate::hud::render_ink_strokes;
use crate::hud::render_laser_pointer;
use crate::hud::render_laser_trail;
use crate::hud::render_palette_popup;
use crate::hud::render_volume_slider_popup;
use crate::media::MediaPlayer;
use crate::renderer::RenderMetrics;
use crate::renderer::SvgRenderer;
use crate::transition::TransitionManager;
use minifb::CursorStyle;
use minifb::Key;
use minifb::MouseButton;
use minifb::MouseMode;
use minifb::Window;
use minifb::WindowOptions;
use slide_core::animation::AnimationRegistry;
use slide_core::animation::ComponentAnimation;
use slide_core::animation::SlideAnimation;
use slide_core::animation::SlideSurface;
use slide_core::chart::ChartType;
use slide_core::compiler::SlideCompiler;
use slide_core::error::Result;
use slide_core::error::SlideError;
use slide_core::model::Hotspot;
use slide_core::model::Rect;
use slide_core::model::SlideDeck;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct PlayerConfig {
    pub title: String,
    pub width: usize,
    pub height: usize,
    pub fullscreen: bool,
    pub default_animation: String,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            title: "Cargo Slide Presentation".to_string(),
            width: 1280,
            height: 720,
            fullscreen: false,
            default_animation: "fade".to_string(),
        }
    }
}

/// Slide presentation application builder and runner
pub struct SlideApp {
    source_file: Option<PathBuf>,
    deck: Option<SlideDeck>,
    config: PlayerConfig,
    registry: AnimationRegistry,
}

impl SlideApp {
    /// Create a SlideApp from a Typst file path
    pub fn new(file: impl AsRef<Path>) -> Self {
        Self {
            source_file: Some(file.as_ref().to_path_buf()),
            deck: None,
            config: PlayerConfig::default(),
            registry: AnimationRegistry::default(),
        }
    }

    /// Create a SlideApp directly from a pre-compiled or embedded `SlideDeck`
    pub fn from_deck(deck: SlideDeck) -> Self {
        let title = deck.title.clone();
        let default_animation = deck.default_animation.clone();
        Self {
            source_file: None,
            deck: Some(deck),
            config: PlayerConfig {
                title,
                default_animation,
                ..Default::default()
            },
            registry: AnimationRegistry::default(),
        }
    }

    pub fn title(
        mut self,
        title: impl Into<String>,
    ) -> Self {
        self.config.title = title.into();
        self
    }

    pub fn window_size(
        mut self,
        width: usize,
        height: usize,
    ) -> Self {
        self.config.width = width;
        self.config.height = height;
        self
    }

    pub fn default_animation(
        mut self,
        animation: impl Into<String>,
    ) -> Self {
        self.config.default_animation = animation.into();
        self
    }

    pub fn register_animation(
        mut self,
        anim: impl SlideAnimation + 'static,
    ) -> Self {
        self.registry.register(anim);
        self
    }

    pub fn register_transition(
        self,
        anim: impl SlideAnimation + 'static,
    ) -> Self {
        self.register_animation(anim)
    }

    pub fn register_component_animation(
        mut self,
        anim: impl ComponentAnimation + 'static,
    ) -> Self {
        self.registry.register_component(anim);
        self
    }

    pub fn with_registry(
        mut self,
        registry: AnimationRegistry,
    ) -> Self {
        self.registry = registry;
        self
    }

    /// Run the presentation player
    pub fn run(self) -> Result<()> {
        let deck = match self.deck {
            | Some(d) => d,
            | None => {
                let file = self.source_file.as_ref().ok_or_else(|| {
                    SlideError::Compilation("No slide file or deck provided".to_string())
                })?;
                let compiler = SlideCompiler::new()?;
                compiler.compile_file(file)?
            },
        };

        SlidePlayer::new(deck)
            .with_config(self.config)
            .with_source_file(self.source_file)
            .with_registry(self.registry)
            .run()
    }
}

pub struct SlidePlayer {
    deck: SlideDeck,
    config: PlayerConfig,
    registry: AnimationRegistry,
    source_file: Option<PathBuf>,
}

fn resolve_media_path(
    path: &str,
    source_file: Option<&Path>,
) -> String {
    let p = Path::new(path);
    if p.is_absolute() || p.exists() {
        return path.to_string();
    }
    if let Some(sf) = source_file
        && let Some(parent) = sf.parent()
    {
        let resolved = parent.join(p);
        if resolved.exists() {
            return resolved.to_string_lossy().to_string();
        }
    }
    path.to_string()
}

/// Query the screen resolution of the active display across platforms
pub fn get_screen_resolution() -> Option<(usize, usize)> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(xlib) = x11_dl::xlib::Xlib::open() {
            unsafe {
                let display = (xlib.XOpenDisplay)(std::ptr::null());
                if !display.is_null() {
                    let screen = (xlib.XDefaultScreen)(display);
                    let w = (xlib.XDisplayWidth)(display, screen) as usize;
                    let h = (xlib.XDisplayHeight)(display, screen) as usize;
                    (xlib.XCloseDisplay)(display);
                    if w > 0 && h > 0 {
                        return Some((w, h));
                    }
                }
            }
        }
        // Fallback: /sys/class/graphics/fb0/virtual_size
        if let Ok(content) = std::fs::read_to_string("/sys/class/graphics/fb0/virtual_size")
            && let Some((w_s, h_s)) = content.trim().split_once(',')
            && let (Ok(w), Ok(h)) = (w_s.parse::<usize>(), h_s.parse::<usize>())
            && w > 0
            && h > 0
        {
            return Some((w, h));
        }
        // Fallback: query xrandr if available
        if let Ok(output) = std::process::Command::new("xrandr").output() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if line.contains(" connected ") {
                    for part in line.split_whitespace() {
                        if let Some((w_s, rest)) = part.split_once('x')
                            && let Some((h_s, _)) = rest.split_once('+')
                            && let (Ok(w), Ok(h)) = (w_s.parse::<usize>(), h_s.parse::<usize>())
                            && w > 0
                            && h > 0
                        {
                            return Some((w, h));
                        }
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    {
        extern "system" {
            fn GetSystemMetrics(nIndex: i32) -> i32;
        }
        let w = unsafe { GetSystemMetrics(0) }; // SM_CXSCREEN
        let h = unsafe { GetSystemMetrics(1) }; // SM_CYSCREEN
        if w > 0 && h > 0 {
            return Some((w as usize, h as usize));
        }
    }

    #[cfg(target_os = "macos")]
    {
        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGMainDisplayID() -> u32;
            fn CGDisplayPixelsWide(display: u32) -> usize;
            fn CGDisplayPixelsHigh(display: u32) -> usize;
        }
        unsafe {
            let display = CGMainDisplayID();
            let w = CGDisplayPixelsWide(display);
            let h = CGDisplayPixelsHigh(display);
            if w > 0 && h > 0 {
                return Some((w, h));
            }
        }
    }

    None
}

#[cfg(target_os = "linux")]
fn set_x11_fullscreen(
    window_handle: *mut std::ffi::c_void,
    fullscreen: bool,
) {
    if window_handle.is_null() {
        return;
    }
    let xid = window_handle as std::os::raw::c_ulong;
    if let Ok(xlib) = x11_dl::xlib::Xlib::open() {
        unsafe {
            let display = (xlib.XOpenDisplay)(std::ptr::null());
            if display.is_null() {
                return;
            }
            let root = (xlib.XDefaultRootWindow)(display);
            let net_wm_state =
                (xlib.XInternAtom)(display, b"_NET_WM_STATE\0".as_ptr() as *const _, 0);
            let net_wm_state_fullscreen = (xlib.XInternAtom)(
                display,
                b"_NET_WM_STATE_FULLSCREEN\0".as_ptr() as *const _,
                0,
            );

            let mut data = x11_dl::xlib::ClientMessageData::new();
            data.set_long(0, if fullscreen { 1 } else { 0 }); // 1 = ADD, 0 = REMOVE
            data.set_long(1, net_wm_state_fullscreen as _);
            data.set_long(2, 0);
            data.set_long(3, 1); // 1 = normal application
            data.set_long(4, 0);

            let xev = x11_dl::xlib::XClientMessageEvent {
                type_: x11_dl::xlib::ClientMessage,
                serial: 0,
                send_event: 1, // True
                display,
                window: xid,
                message_type: net_wm_state,
                format: 32,
                data,
            };

            let mut ev: x11_dl::xlib::XEvent = std::mem::zeroed();
            ev.client_message = xev;

            let mask =
                x11_dl::xlib::SubstructureRedirectMask | x11_dl::xlib::SubstructureNotifyMask;
            (xlib.XSendEvent)(display, root, 0, mask, &mut ev);
            (xlib.XFlush)(display);
            (xlib.XCloseDisplay)(display);
        }
    }
}

fn create_window(
    title: &str,
    width: usize,
    height: usize,
    fullscreen: bool,
) -> Result<Window> {
    let options = if fullscreen {
        WindowOptions {
            borderless: true,
            title: false,
            resize: true,
            scale: minifb::Scale::X1,
            topmost: true,
            ..WindowOptions::default()
        }
    } else {
        WindowOptions {
            borderless: false,
            title: true,
            resize: true,
            scale: minifb::Scale::X1,
            topmost: false,
            ..WindowOptions::default()
        }
    };

    let mut window = Window::new(title, width, height, options)
        .map_err(|e| SlideError::Format(format!("Failed to create window: {}", e)))?;
    window.set_target_fps(60);

    if fullscreen {
        window.set_position(0, 0);
        #[cfg(target_os = "linux")]
        set_x11_fullscreen(window.get_window_handle(), true);
    } else {
        #[cfg(target_os = "linux")]
        set_x11_fullscreen(window.get_window_handle(), false);
    }

    Ok(window)
}

impl SlidePlayer {
    pub fn new(deck: SlideDeck) -> Self {
        let title = deck.title.clone();
        let default_anim = deck.default_animation.clone();
        Self {
            deck,
            config: PlayerConfig {
                title,
                default_animation: default_anim,
                ..Default::default()
            },
            registry: AnimationRegistry::default(),
            source_file: None,
        }
    }

    pub fn with_config(
        mut self,
        config: PlayerConfig,
    ) -> Self {
        self.config = config;
        self
    }

    pub fn with_registry(
        mut self,
        registry: AnimationRegistry,
    ) -> Self {
        self.registry = registry;
        self
    }

    pub fn with_source_file(
        mut self,
        file: Option<PathBuf>,
    ) -> Self {
        self.source_file = file;
        self
    }

    pub fn register_animation(
        mut self,
        anim: impl SlideAnimation + 'static,
    ) -> Self {
        self.registry.register(anim);
        self
    }

    pub fn register_transition(
        self,
        anim: impl SlideAnimation + 'static,
    ) -> Self {
        self.register_animation(anim)
    }

    pub fn register_component_animation(
        mut self,
        anim: impl ComponentAnimation + 'static,
    ) -> Self {
        self.registry.register_component(anim);
        self
    }

    /// Run the presentation player event loop
    pub fn run(mut self) -> Result<()> {
        if self.deck.slides.is_empty() {
            return Err(SlideError::Format(
                "Slide deck has no slides to display".to_string(),
            ));
        }

        let mut is_fullscreen = self.config.fullscreen;
        let (screen_w, screen_h) = get_screen_resolution().unwrap_or((1920, 1080));
        let (mut width, mut height) = if is_fullscreen {
            (screen_w, screen_h)
        } else {
            (self.config.width, self.config.height)
        };

        let mut window = create_window(&self.config.title, width, height, is_fullscreen)?;

        let renderer = SvgRenderer::new();
        let mut transition_mgr = TransitionManager::new(self.registry);

        let mut current_idx = 0;
        let mut total_slides = self.deck.total_slides();

        // Audio engine
        let mut audio_engine = AudioEngine::new();
        let mut last_volume_change: Option<Instant> = None;

        // Presenter interactive tools state
        let mut presenter_mode = PresenterMode::Normal;
        let mut active_color_idx = 0usize; // Cyan by default (index 0)
        let mut palette_open = false;
        let mut slide_ink: HashMap<usize, Vec<InkStroke>> = HashMap::new();
        let mut active_pen_stroke: Option<InkStroke> = None;
        let mut laser_trail: VecDeque<(usize, usize, Instant)> = VecDeque::new();

        // In-slide component step state
        let get_max_step = |deck: &SlideDeck, idx: usize| -> usize {
            deck.get_slide(idx).map(|s| s.max_step()).unwrap_or(0)
        };
        let mut current_step: usize = if get_max_step(&self.deck, 0) > 0 {
            1
        } else {
            0
        };

        // Chart interactivity and inspector state
        let mut chart_type_overrides: HashMap<(usize, usize), ChartType> = HashMap::new();
        let mut active_chart_inspector: Option<ChartInspectorState> = None;

        // Surface cache mapped by slide_index
        let mut cache: HashMap<usize, (Arc<SlideSurface>, RenderMetrics)> = HashMap::new();

        let mut buffer = vec![0u32; width * height];
        let mut prev_pressed_keys = Vec::new();
        let mut prev_mouse_down = false;
        let mut prev_mouse_right_down = false;
        let mut prev_mouse_pos: Option<(f32, f32)> = None;
        let mut last_mouse_activity = Instant::now();
        let mut hovered_hotspot_key: Option<(usize, usize)> = None;
        let mut hotspot_hover_alpha: f32 = 0.0;
        let mut last_frame_time = Instant::now();
        let mut cursor_hidden = false;
        let mut show_help = false;
        let mut exit_requested = false;

        let trigger_slide_audio =
            |deck: &SlideDeck, idx: usize, audio: &mut AudioEngine, sf: Option<&Path>| {
                if let Some(slide) = deck.get_slide(idx) {
                    for hs in &slide.hotspots {
                        if let Hotspot::Audio {
                            source,
                            autoplay,
                            loop_audio,
                            volume,
                            ..
                        } = hs
                            && *autoplay
                        {
                            let resolved = resolve_media_path(source, sf);
                            let _ = audio.play_track(
                                &resolved,
                                *loop_audio,
                                *volume,
                                Duration::from_millis(600),
                            );
                            return;
                        }
                    }
                }
                audio.stop_with_fade(Duration::from_millis(600));
            };

        trigger_slide_audio(
            &self.deck,
            current_idx,
            &mut audio_engine,
            self.source_file.as_deref(),
        );

        while window.is_open() && !exit_requested && !window.is_key_down(Key::Q) {
            let (new_w, new_h) = window.get_size();
            if (new_w != width || new_h != height) && new_w >= 100 && new_h >= 100 {
                width = new_w;
                height = new_h;
                buffer.resize(width * height, 0);
                cache.clear(); // Invalidate cache on resize
            }

            let dt = last_frame_time.elapsed().as_secs_f32().clamp(0.001, 0.1);
            last_frame_time = Instant::now();

            // Update smooth volume curves in audio engine
            audio_engine.update();

            // Handle mouse scroll wheel for volume adjustment or Data Inspector table scrolling
            if let Some((_scroll_x, scroll_y)) = window.get_scroll_wheel()
                && scroll_y != 0.0
            {
                if let Some(ref mut inspector) = active_chart_inspector {
                    if scroll_y < 0.0 {
                        inspector.table_scroll = (inspector.table_scroll + 1)
                            .min(inspector.chart_data.categories.len().saturating_sub(1));
                    } else {
                        inspector.table_scroll = inspector.table_scroll.saturating_sub(1);
                    }
                } else {
                    let delta = if scroll_y > 0.0 {
                        0.05
                    } else {
                        -0.05
                    };
                    audio_engine.adjust_volume(delta);
                    last_volume_change = Some(Instant::now());
                }
            }

            // Ensure current slide surface is rendered
            let (current_surface, current_metrics) = match cache.get(&current_idx) {
                | Some(cached) => (cached.0.clone(), cached.1),
                | None => {
                    let slide = self.deck.get_slide(current_idx).unwrap();
                    let (surf, metrics) = renderer.render_svg(&slide.svg_data, width, height)?;
                    let surf_arc = Arc::new(surf);
                    cache.insert(current_idx, (surf_arc.clone(), metrics));
                    (surf_arc, metrics)
                },
            };

            // Handle transition animation or render static slide
            let is_animating = transition_mgr.render_frame(width, height, &mut buffer);
            if !is_animating {
                let copy_len = buffer.len().min(current_surface.pixels.len());
                buffer[..copy_len].copy_from_slice(&current_surface.pixels[..copy_len]);

                // Blank / mask component steps that have not yet been revealed
                if let Some(slide) = self.deck.get_slide(current_idx) {
                    let bg_color = 0xFF0f111a; // Slide dark background
                    for step in &slide.steps {
                        if step.order > current_step {
                            let sx1 = (current_metrics.offset_x
                                + step.rect.x * current_metrics.scale)
                                .max(0.0) as usize;
                            let sy1 = (current_metrics.offset_y
                                + step.rect.y * current_metrics.scale)
                                .max(0.0) as usize;
                            let sx2 = ((current_metrics.offset_x
                                + (step.rect.x + step.rect.width) * current_metrics.scale)
                                as usize)
                                .min(width);
                            let sy2 = ((current_metrics.offset_y
                                + (step.rect.y + step.rect.height) * current_metrics.scale)
                                as usize)
                                .min(height);

                            for y in sy1..sy2 {
                                let row = y * width;
                                for x in sx1..sx2 {
                                    buffer[row + x] = bg_color;
                                }
                            }
                        }
                    }

                    // Render active in-slide component reveal animation if running
                    transition_mgr.render_component_animation(
                        width,
                        height,
                        &current_surface,
                        &mut buffer,
                        bg_color,
                    );
                }

                // Render in-slide chart type overrides if any
                if let Some(slide) = self.deck.get_slide(current_idx) {
                    for (hs_idx, hs) in slide.hotspots.iter().enumerate() {
                        if let Hotspot::Chart { rect, data } = hs
                            && let Some(&override_type) =
                                chart_type_overrides.get(&(current_idx, hs_idx))
                            && override_type != data.chart_type
                        {
                            let screen_x =
                                current_metrics.offset_x + rect.x * current_metrics.scale;
                            let screen_y =
                                current_metrics.offset_y + rect.y * current_metrics.scale;
                            let screen_w = rect.width * current_metrics.scale;
                            let screen_h = rect.height * current_metrics.scale;
                            let screen_rect = Rect::new(screen_x, screen_y, screen_w, screen_h);
                            let empty_set = std::collections::HashSet::new();
                            draw_chart_visualizer(
                                &mut buffer,
                                width,
                                height,
                                screen_rect,
                                data,
                                override_type,
                                &empty_set,
                                None,
                                None,
                            );
                        }
                    }
                }
            }

            // Mouse handling and cursor style
            let mouse_pos = window.get_mouse_pos(MouseMode::Pass);
            if mouse_pos != prev_mouse_pos {
                last_mouse_activity = Instant::now();
                if cursor_hidden && presenter_mode != PresenterMode::Laser {
                    window.set_cursor_visibility(true);
                    cursor_hidden = false;
                }
                prev_mouse_pos = mouse_pos;
            } else if !cursor_hidden && last_mouse_activity.elapsed() > Duration::from_secs(3) {
                // Auto-hide cursor during presentation if idle for 3 seconds
                window.set_cursor_visibility(false);
                cursor_hidden = true;
            }

            // Synchronize mouse hovering within Data Inspector panes if open
            if let Some(ref mut inspector) = active_chart_inspector
                && let Some((mx, my)) = mouse_pos
            {
                let (_, _, _, _, _, _, left_pane, right_pane) = get_inspector_layout(width, height);
                if left_pane.contains(mx, my) {
                    let px = left_pane.x + 44.0;
                    let pw = left_pane.width - 60.0;
                    let cat_count = inspector.chart_data.categories.len().max(1);
                    let col_w = pw / cat_count as f32;
                    if mx >= px && mx <= (px + pw) {
                        let c_idx = (((mx - px) / col_w) as usize).min(cat_count.saturating_sub(1));
                        inspector.hovered_category = Some(c_idx);
                    } else {
                        inspector.hovered_category = None;
                    }
                } else if right_pane.contains(mx, my) {
                    let header_h = 26.0;
                    let row_h = 24.0;
                    let table_y = right_pane.y + header_h;
                    if my >= table_y {
                        let row_idx = ((my - table_y) / row_h) as usize + inspector.table_scroll;
                        if row_idx < inspector.chart_data.categories.len() {
                            inspector.hovered_category = Some(row_idx);
                        } else {
                            inspector.hovered_category = None;
                        }
                    } else {
                        inspector.hovered_category = None;
                    }
                } else {
                    inspector.hovered_category = None;
                }
            }

            let (dock_rect, _) = get_dock_rects(width, height, is_fullscreen);
            let mouse_in_dock = mouse_pos
                .map(|(mx, my)| dock_rect.contains(mx, my))
                .unwrap_or(false);
            let mouse_near_dock = mouse_pos
                .map(|(_, my)| my >= (height as f32 - 70.0))
                .unwrap_or(false);
            let hovered_dock_action =
                mouse_pos.and_then(|(mx, my)| hit_test_dock(width, height, is_fullscreen, mx, my));

            let (slider_popup_rect, _) = get_volume_slider_rects(width, height, is_fullscreen);
            let mouse_in_slider = mouse_pos
                .map(|(mx, my)| slider_popup_rect.contains(mx, my))
                .unwrap_or(false);
            let show_volume_slider =
                mouse_in_slider || hovered_dock_action == Some(DockAction::ToggleMute);

            let hovered_palette_idx = mouse_pos
                .and_then(|(mx, my)| hit_test_palette(width, height, is_fullscreen, mx, my));

            let mut hovered_hotspot_idx: Option<usize> = None;
            if active_chart_inspector.is_none()
                && !is_animating
                && let Some((mx, my)) = mouse_pos
                && !mouse_in_dock
                && !mouse_in_slider
                && (!palette_open || hovered_palette_idx.is_none())
                && let Some((svg_x, svg_y)) = current_metrics.screen_to_svg(mx, my)
                && let Some(slide) = self.deck.get_slide(current_idx)
            {
                for (idx, hotspot) in slide.hotspots.iter().enumerate() {
                    if hotspot.contains(svg_x, svg_y) {
                        hovered_hotspot_idx = Some(idx);
                        break;
                    }
                }
            }

            // Smooth animation: lerp hotspot hover alpha (fades in ~80ms, fades out ~100ms)
            let current_target_key = if !is_animating {
                hovered_hotspot_idx.map(|hs_i| (current_idx, hs_i))
            } else {
                None
            };

            if current_target_key != hovered_hotspot_key {
                if current_target_key.is_none() {
                    hotspot_hover_alpha = (hotspot_hover_alpha - dt * 10.0).max(0.0);
                    if hotspot_hover_alpha <= 0.01 {
                        hovered_hotspot_key = None;
                    }
                } else {
                    if hovered_hotspot_key.is_none()
                        || hovered_hotspot_key.unwrap().0 != current_idx
                    {
                        hotspot_hover_alpha = 0.0;
                    }
                    hovered_hotspot_key = current_target_key;
                    hotspot_hover_alpha = (hotspot_hover_alpha + dt * 12.0).min(1.0);
                }
            } else if hovered_hotspot_key.is_some() {
                hotspot_hover_alpha = (hotspot_hover_alpha + dt * 12.0).min(1.0);
            } else {
                hotspot_hover_alpha = (hotspot_hover_alpha - dt * 10.0).max(0.0);
            }

            // Set cursor style based on mode and hover
            if presenter_mode == PresenterMode::Laser {
                if !cursor_hidden {
                    window.set_cursor_visibility(false);
                    cursor_hidden = true;
                }
            } else if active_chart_inspector.is_some() {
                window.set_cursor_style(CursorStyle::Arrow);
            } else {
                if hovered_dock_action.is_some()
                    || hovered_hotspot_idx.is_some()
                    || hovered_palette_idx.is_some()
                    || mouse_in_slider
                {
                    window.set_cursor_style(CursorStyle::OpenHand);
                } else if presenter_mode == PresenterMode::Pen {
                    window.set_cursor_style(CursorStyle::Crosshair);
                } else {
                    window.set_cursor_style(CursorStyle::Arrow);
                }
            }

            // Highlight hovered hotspot (in Normal mode when inspector is closed, not animating transition)
            if presenter_mode == PresenterMode::Normal
                && active_chart_inspector.is_none()
                && !is_animating
                && hotspot_hover_alpha > 0.01
                && let Some((slide_i, hs_idx)) = hovered_hotspot_key
                && slide_i == current_idx
                && let Some(slide) = self.deck.get_slide(current_idx)
                && let Some(hotspot) = slide.hotspots.get(hs_idx)
            {
                let rect = hotspot.rect();
                let screen_x = current_metrics.offset_x + rect.x * current_metrics.scale;
                let screen_y = current_metrics.offset_y + rect.y * current_metrics.scale;
                let screen_w = rect.width * current_metrics.scale;
                let screen_h = rect.height * current_metrics.scale;
                let screen_rect = Rect::new(screen_x, screen_y, screen_w, screen_h);

                let color = match hotspot {
                    | Hotspot::Link { .. } => 0xFF58a6ff,  // Blue outline
                    | Hotspot::Video { .. } => 0xFFf0883e, // Orange outline
                    | Hotspot::Audio { .. } => 0xFF3fb950, // Green outline
                    | Hotspot::Chart { .. } => 0xFF38bdf8, // Cyan outline
                };
                draw_hotspot_highlight(
                    &mut buffer,
                    width,
                    height,
                    screen_rect,
                    color,
                    hotspot_hover_alpha,
                );

                if let Hotspot::Chart { rect, data } = hotspot {
                    let active_t = chart_type_overrides
                        .get(&(current_idx, hs_idx))
                        .copied()
                        .unwrap_or(data.chart_type);
                    if let Some((mx, my)) = mouse_pos {
                        render_chart_hover(
                            &mut buffer,
                            width,
                            height,
                            &current_metrics,
                            *rect,
                            data,
                            active_t,
                            mx,
                            my,
                        );
                    }
                }
            }

            // Mouse click handling
            let mouse_down = window.get_mouse_down(MouseButton::Left);
            let mouse_right = window.get_mouse_down(MouseButton::Right);
            let left_clicked = mouse_down && !prev_mouse_down;
            let right_clicked = mouse_right && !prev_mouse_right_down;

            let mut trigger_forward = false;
            let mut trigger_backward = false;
            let mut jump_target = None;
            let mut toggle_fullscreen_requested = false;

            if right_clicked {
                if active_chart_inspector.is_some() {
                    active_chart_inspector = None;
                } else {
                    // Right click steps backward
                    trigger_backward = true;
                }
            }

            // Interactive dragging on volume slider track
            if mouse_down
                && mouse_in_slider
                && active_chart_inspector.is_none()
                && let Some((mx, my)) = mouse_pos
                && let Some(vol) = hit_test_volume_slider(width, height, is_fullscreen, mx, my)
            {
                audio_engine.set_volume(vol);
                last_volume_change = Some(Instant::now());
            }

            if left_clicked {
                if let Some(ref mut inspector) = active_chart_inspector {
                    if let Some((mx, my)) = mouse_pos
                        && let Some(action) =
                            hit_test_chart_inspector(inspector, width, height, mx, my)
                    {
                        match action {
                            | InspectorAction::Close => {
                                if let Some(hs_idx) = hovered_hotspot_idx {
                                    chart_type_overrides
                                        .insert((current_idx, hs_idx), inspector.active_type);
                                }
                                active_chart_inspector = None;
                            },
                            | InspectorAction::SetType(t) => {
                                inspector.active_type = t;
                            },
                            | InspectorAction::SetTransform(t) => {
                                inspector.set_transform(t);
                            },
                            | InspectorAction::ToggleSeries(s_idx) => {
                                inspector.toggle_series(s_idx);
                            },
                            | InspectorAction::ExportCsv => {
                                let csv = inspector.chart_data.export_csv(&inspector.hidden_series);
                                if let Err(e) = std::fs::write("chart_export.csv", csv) {
                                    eprintln!("Failed to export CSV: {}", e);
                                    inspector.show_toast("Export failed!");
                                } else {
                                    inspector.show_toast("Exported to chart_export.csv");
                                }
                            },
                            | InspectorAction::SelectCategory(cat_idx) => {
                                inspector.hovered_category = Some(cat_idx);
                            },
                            | InspectorAction::ScrollTable(delta) => {
                                if delta > 0 {
                                    inspector.table_scroll =
                                        (inspector.table_scroll + delta as usize).min(
                                            inspector.chart_data.categories.len().saturating_sub(1),
                                        );
                                } else {
                                    inspector.table_scroll =
                                        inspector.table_scroll.saturating_sub((-delta) as usize);
                                }
                            },
                        }
                    }
                } else if let Some(pal_idx) = hovered_palette_idx {
                    active_color_idx = pal_idx;
                } else if let Some(dock_action) = hovered_dock_action {
                    match dock_action {
                        | DockAction::Prev => trigger_backward = true,
                        | DockAction::Next => trigger_forward = true,
                        | DockAction::ModeNormal => presenter_mode = PresenterMode::Normal,
                        | DockAction::ModeLaser => presenter_mode = PresenterMode::Laser,
                        | DockAction::ModePen => presenter_mode = PresenterMode::Pen,
                        | DockAction::TogglePalette => palette_open = !palette_open,
                        | DockAction::ClearInk => {
                            slide_ink.remove(&current_idx);
                            active_pen_stroke = None;
                        },
                        | DockAction::ToggleMute => {
                            audio_engine.toggle_mute();
                            last_volume_change = Some(Instant::now());
                        },
                        | DockAction::ToggleFullscreen => toggle_fullscreen_requested = true,
                        | DockAction::ToggleHelp => show_help = !show_help,
                    }
                } else if mouse_in_slider {
                    if let Some((mx, my)) = mouse_pos
                        && let Some(vol) =
                            hit_test_volume_slider(width, height, is_fullscreen, mx, my)
                    {
                        audio_engine.set_volume(vol);
                        last_volume_change = Some(Instant::now());
                    }
                } else if presenter_mode != PresenterMode::Pen {
                    if let Some(hs_idx) = hovered_hotspot_idx {
                        if let Some(slide) = self.deck.get_slide(current_idx)
                            && let Some(hotspot) = slide.hotspots.get(hs_idx)
                        {
                            match hotspot {
                                | Hotspot::Link { target, rect } => {
                                    let clean_target = target.strip_prefix('#').unwrap_or(target);
                                    if clean_target.starts_with("chart:") {
                                        // Fallback guard: if a chart hotspot was parsed as Link, open Inspector directly
                                        if let Some(payload) = clean_target.strip_prefix("chart:")
                                            && let Some(Hotspot::Chart { data, .. }) =
                                                slide_core::svg::parse_chart_href(payload, *rect)
                                        {
                                            let curr_t = chart_type_overrides
                                                .get(&(current_idx, hs_idx))
                                                .copied()
                                                .unwrap_or(data.chart_type);
                                            active_chart_inspector =
                                                Some(ChartInspectorState::new(data, curr_t));
                                        }
                                    } else if clean_target.starts_with("video:")
                                        || clean_target.starts_with("audio:")
                                        || clean_target.starts_with("step:")
                                        || clean_target.starts_with("transition:")
                                    {
                                        // Internal metadata markers; never invoke OS open
                                    } else if target.starts_with("http://")
                                        || target.starts_with("https://")
                                        || target.starts_with("mailto:")
                                    {
                                        let _ = open::that(target);
                                    } else if let Some(page_str) = target
                                        .strip_prefix("#page=")
                                        .or_else(|| target.strip_prefix("#slide="))
                                        .or_else(|| target.strip_prefix("#"))
                                    {
                                        if let Ok(page) = page_str.parse::<usize>()
                                            && page >= 1
                                            && page <= total_slides
                                            && page - 1 != current_idx
                                        {
                                            jump_target = Some(page - 1);
                                        }
                                    } else {
                                        // Local document or file path relative to presentation
                                        let clean_file =
                                            target.strip_prefix("file://").unwrap_or(target);
                                        if !clean_file.is_empty()
                                            && !clean_file.starts_with('#')
                                            && !clean_file.contains(':')
                                        {
                                            let resolved = resolve_media_path(
                                                clean_file,
                                                self.source_file.as_deref(),
                                            );
                                            let _ = open::that(&resolved);
                                        }
                                    }
                                },
                                | Hotspot::Video { source, .. } => {
                                    let resolved =
                                        resolve_media_path(source, self.source_file.as_deref());
                                    let _ = MediaPlayer::play_video(&resolved);
                                },
                                | Hotspot::Audio {
                                    source,
                                    loop_audio,
                                    volume,
                                    ..
                                } => {
                                    let resolved =
                                        resolve_media_path(source, self.source_file.as_deref());
                                    if audio_engine.current_source() == Some(&resolved) {
                                        audio_engine.toggle_pause();
                                    } else {
                                        let _ = audio_engine.play_track(
                                            &resolved,
                                            *loop_audio,
                                            *volume,
                                            Duration::from_millis(500),
                                        );
                                    }
                                },
                                | Hotspot::Chart { rect, data } => {
                                    let (type_btn, _) =
                                        get_chart_quick_action_rects(&current_metrics, *rect);
                                    let clicked_type_btn = mouse_pos
                                        .map(|(mx, my)| type_btn.contains(mx, my))
                                        .unwrap_or(false);

                                    let curr_t = chart_type_overrides
                                        .get(&(current_idx, hs_idx))
                                        .copied()
                                        .unwrap_or(data.chart_type);

                                    if clicked_type_btn {
                                        let next_t = match curr_t {
                                            | ChartType::Bar => ChartType::Line,
                                            | ChartType::Line => ChartType::Area,
                                            | ChartType::Area => ChartType::Pie,
                                            | ChartType::Pie => ChartType::Donut,
                                            | ChartType::Donut => ChartType::Bar,
                                            | ChartType::Scatter => ChartType::Bar,
                                        };
                                        chart_type_overrides.insert((current_idx, hs_idx), next_t);
                                    } else {
                                        // Open Data Inspector Modal!
                                        active_chart_inspector =
                                            Some(ChartInspectorState::new(data.clone(), curr_t));
                                    }
                                },
                            }
                        }
                    } else {
                        // Canvas click steps forward
                        trigger_forward = true;
                    }
                }
            }

            let active_color = PALETTE_COLORS[active_color_idx].1;

            // Whiteboard pen stroke tracking
            if presenter_mode == PresenterMode::Pen {
                if mouse_down
                    && !mouse_in_dock
                    && !mouse_in_slider
                    && (!palette_open || hovered_palette_idx.is_none())
                {
                    if let Some((mx, my)) = mouse_pos {
                        let pt = (mx as usize, my as usize);
                        if let Some(ref mut stroke) = active_pen_stroke {
                            if stroke.points.last() != Some(&pt) {
                                stroke.points.push(pt);
                            }
                        } else {
                            active_pen_stroke = Some(InkStroke {
                                points: vec![pt],
                                color: active_color,
                                width: 4,
                            });
                        }
                    }
                } else if !mouse_down
                    && prev_mouse_down
                    && let Some(stroke) = active_pen_stroke.take()
                {
                    slide_ink.entry(current_idx).or_default().push(stroke);
                }
            }

            // Laser pointer position recording for trailing effect
            if presenter_mode == PresenterMode::Laser
                && let Some((mx, my)) = mouse_pos
                && mx >= 0.0
                && my >= 0.0
                && (mx as usize) < width
                && (my as usize) < height
            {
                laser_trail.push_back((mx as usize, my as usize, Instant::now()));
            }
            // Trim old laser trail points (decay over 220ms)
            let now = Instant::now();
            while let Some(front) = laser_trail.front() {
                if now.duration_since(front.2) > Duration::from_millis(220) {
                    laser_trail.pop_front();
                } else {
                    break;
                }
            }

            prev_mouse_down = mouse_down;
            prev_mouse_right_down = mouse_right;

            // Keyboard navigation & tools
            let keys = window.get_keys();
            let mut reload_triggered = false;

            for key in &keys {
                if !prev_pressed_keys.contains(key) {
                    if let Some(ref mut inspector) = active_chart_inspector {
                        match key {
                            | Key::Escape => {
                                if let Some(hs_idx) = hovered_hotspot_idx {
                                    chart_type_overrides
                                        .insert((current_idx, hs_idx), inspector.active_type);
                                }
                                active_chart_inspector = None;
                            },
                            | Key::Tab => {
                                inspector.cycle_type();
                            },
                            | Key::Key1 | Key::NumPad1 => inspector.toggle_series(0),
                            | Key::Key2 | Key::NumPad2 => inspector.toggle_series(1),
                            | Key::Key3 | Key::NumPad3 => inspector.toggle_series(2),
                            | Key::Key4 | Key::NumPad4 => inspector.toggle_series(3),
                            | Key::Key5 | Key::NumPad5 => inspector.toggle_series(4),
                            | Key::Key6 | Key::NumPad6 => inspector.toggle_series(5),
                            | Key::Key7 | Key::NumPad7 => inspector.toggle_series(6),
                            | Key::Key8 | Key::NumPad8 => inspector.toggle_series(7),
                            | Key::Key9 | Key::NumPad9 => inspector.toggle_series(8),
                            | Key::C | Key::E => {
                                let csv = inspector.chart_data.export_csv(&inspector.hidden_series);
                                if let Err(e) = std::fs::write("chart_export.csv", csv) {
                                    eprintln!("Failed to export CSV: {}", e);
                                    inspector.show_toast("Export failed!");
                                } else {
                                    inspector.show_toast("Exported to chart_export.csv");
                                }
                            },
                            | Key::Up => {
                                inspector.table_scroll = inspector.table_scroll.saturating_sub(1);
                            },
                            | Key::Down => {
                                inspector.table_scroll = (inspector.table_scroll + 1)
                                    .min(inspector.chart_data.categories.len().saturating_sub(1));
                            },
                            | Key::O => {
                                inspector.set_transform(slide_core::chart::ChartTransform::None)
                            },
                            | Key::T => {
                                inspector.set_transform(slide_core::chart::ChartTransform::TopK(5))
                            },
                            | Key::S => {
                                inspector.set_transform(slide_core::chart::ChartTransform::SortDesc)
                            },
                            | Key::A => {
                                inspector.set_transform(slide_core::chart::ChartTransform::SortAsc)
                            },
                            | Key::U => {
                                inspector
                                    .set_transform(slide_core::chart::ChartTransform::Cumulative)
                            },
                            | Key::P => {
                                inspector
                                    .set_transform(slide_core::chart::ChartTransform::Percent100)
                            },
                            | Key::M => {
                                inspector
                                    .set_transform(slide_core::chart::ChartTransform::MovingAvg(3))
                            },
                            | _ => {},
                        }
                    } else {
                        match key {
                            | Key::Right | Key::Down | Key::Space | Key::PageDown | Key::Enter => {
                                trigger_forward = true;
                            },
                            | Key::Left | Key::Up | Key::Backspace | Key::PageUp => {
                                trigger_backward = true;
                            },
                            | Key::Home => jump_target = Some(0),
                            | Key::End => jump_target = Some(total_slides.saturating_sub(1)),
                            | Key::NumPad1 | Key::Key1 => {
                                if palette_open {
                                    active_color_idx = 0;
                                } else {
                                    jump_target = Some(0);
                                }
                            },
                            | Key::NumPad2 | Key::Key2 => {
                                if palette_open {
                                    active_color_idx = 1;
                                } else {
                                    jump_target = Some(1);
                                }
                            },
                            | Key::NumPad3 | Key::Key3 => {
                                if palette_open {
                                    active_color_idx = 2;
                                } else {
                                    jump_target = Some(2);
                                }
                            },
                            | Key::NumPad4 | Key::Key4 => {
                                if palette_open {
                                    active_color_idx = 3;
                                } else {
                                    jump_target = Some(3);
                                }
                            },
                            | Key::NumPad5 | Key::Key5 => {
                                if palette_open {
                                    active_color_idx = 4;
                                } else {
                                    jump_target = Some(4);
                                }
                            },
                            | Key::NumPad6 | Key::Key6 => {
                                if palette_open {
                                    active_color_idx = 5;
                                } else {
                                    jump_target = Some(5);
                                }
                            },
                            | Key::NumPad7 | Key::Key7 => {
                                if palette_open {
                                    active_color_idx = 6;
                                } else {
                                    jump_target = Some(6);
                                }
                            },
                            | Key::NumPad8 | Key::Key8 => jump_target = Some(7),
                            | Key::NumPad9 | Key::Key9 => jump_target = Some(8),
                            | Key::F11 | Key::F => {
                                toggle_fullscreen_requested = true;
                            },
                            | Key::L => {
                                presenter_mode = if presenter_mode == PresenterMode::Laser {
                                    PresenterMode::Normal
                                } else {
                                    PresenterMode::Laser
                                };
                            },
                            | Key::P => {
                                presenter_mode = if presenter_mode == PresenterMode::Pen {
                                    PresenterMode::Normal
                                } else {
                                    PresenterMode::Pen
                                };
                            },
                            | Key::K => {
                                palette_open = !palette_open;
                            },
                            | Key::C | Key::X => {
                                slide_ink.remove(&current_idx);
                                active_pen_stroke = None;
                            },
                            | Key::Equal | Key::NumPadPlus => {
                                audio_engine.adjust_volume(0.05);
                                last_volume_change = Some(Instant::now());
                            },
                            | Key::Minus | Key::NumPadMinus => {
                                audio_engine.adjust_volume(-0.05);
                                last_volume_change = Some(Instant::now());
                            },
                            | Key::M => {
                                audio_engine.toggle_mute();
                                last_volume_change = Some(Instant::now());
                            },
                            | Key::Escape => {
                                if is_fullscreen {
                                    toggle_fullscreen_requested = true;
                                } else {
                                    exit_requested = true;
                                }
                            },
                            | Key::H | Key::Slash => show_help = !show_help,
                            | Key::R => reload_triggered = true,
                            | _ => {},
                        }
                    }
                }
            }
            prev_pressed_keys = keys;

            // Fullscreen toggle
            if toggle_fullscreen_requested {
                is_fullscreen = !is_fullscreen;
                let (target_w, target_h) = if is_fullscreen {
                    get_screen_resolution().unwrap_or((1920, 1080))
                } else {
                    (self.config.width, self.config.height)
                };
                width = target_w;
                height = target_h;
                window = create_window(&self.config.title, width, height, is_fullscreen)?;
                buffer.resize(width * height, 0);
                cache.clear();
            }

            // Live reload (R key)
            if reload_triggered {
                active_chart_inspector = None;
                if let Some(ref file) = self.source_file
                    && let Ok(compiler) = SlideCompiler::new()
                    && let Ok(new_deck) = compiler.compile_file(file)
                {
                    self.deck = new_deck;
                    total_slides = self.deck.total_slides();
                    current_idx = current_idx.min(total_slides.saturating_sub(1));
                    current_step = if get_max_step(&self.deck, current_idx) > 0 {
                        1
                    } else {
                        0
                    };
                    cache.clear();
                    println!("🔄 Slides live-reloaded! ({} slides)", total_slides);
                }
            }

            // Compute step builds or slide progression
            let max_substep = get_max_step(&self.deck, current_idx);
            let mut next_slide = false;
            let mut prev_slide = false;

            if trigger_forward {
                if current_step < max_substep {
                    current_step += 1;
                    // Trigger in-slide component animation
                    if let Some(slide) = self.deck.get_slide(current_idx)
                        && let Some(fragment) = slide.steps.iter().find(|s| s.order == current_step)
                    {
                        let screen_x =
                            current_metrics.offset_x + fragment.rect.x * current_metrics.scale;
                        let screen_y =
                            current_metrics.offset_y + fragment.rect.y * current_metrics.scale;
                        let screen_w = fragment.rect.width * current_metrics.scale;
                        let screen_h = fragment.rect.height * current_metrics.scale;
                        transition_mgr.start_component_step(
                            &fragment.effect,
                            Rect::new(screen_x, screen_y, screen_w, screen_h),
                        );
                    }
                } else if current_idx + 1 < total_slides {
                    next_slide = true;
                }
            } else if trigger_backward {
                if current_step > 1 {
                    current_step -= 1;
                } else if current_idx > 0 {
                    prev_slide = true;
                }
            }

            // Compute new slide index
            let mut new_slide_idx = current_idx;
            if next_slide && current_idx + 1 < total_slides {
                new_slide_idx = current_idx + 1;
            } else if prev_slide && current_idx > 0 {
                new_slide_idx = current_idx - 1;
            } else if let Some(target) = jump_target
                && target < total_slides
            {
                new_slide_idx = target;
            }

            if new_slide_idx != current_idx {
                active_chart_inspector = None;
                hovered_hotspot_key = None;
                hotspot_hover_alpha = 0.0;
                // If slide specified transition override in Typst (e.g. transition: "glitch"), use it!
                let anim_name = self
                    .deck
                    .get_slide(new_slide_idx)
                    .and_then(|s| s.animation.as_deref())
                    .unwrap_or(&self.config.default_animation);

                let (target_surf, target_metrics) = match cache.get(&new_slide_idx) {
                    | Some(cached) => (cached.0.clone(), cached.1),
                    | None => {
                        let s = self.deck.get_slide(new_slide_idx).unwrap();
                        let (sf, met) = renderer.render_svg(&s.svg_data, width, height)?;
                        let sf_arc = Arc::new(sf);
                        cache.insert(new_slide_idx, (sf_arc.clone(), met));
                        (sf_arc, met)
                    },
                };

                let from_slide = self.deck.get_slide(current_idx).unwrap();
                let to_slide = self.deck.get_slide(new_slide_idx).unwrap();
                let new_max_step = get_max_step(&self.deck, new_slide_idx);
                let next_step = if prev_slide {
                    new_max_step
                } else if new_max_step > 0 {
                    1
                } else {
                    0
                };

                let bg_color = 0xFF0f111a;
                let mut from_stepped = (*current_surface).clone();
                from_stepped.mask_steps(
                    &from_slide.steps,
                    current_step,
                    &current_metrics,
                    bg_color,
                );

                let mut to_stepped = (*target_surf).clone();
                to_stepped.mask_steps(&to_slide.steps, next_step, &target_metrics, bg_color);

                transition_mgr.start_transition(
                    anim_name,
                    Arc::new(from_stepped),
                    Arc::new(to_stepped),
                );
                current_idx = new_slide_idx;
                current_step = next_step;
                trigger_slide_audio(
                    &self.deck,
                    current_idx,
                    &mut audio_engine,
                    self.source_file.as_deref(),
                );
            }

            // Draw whiteboard ink strokes for current slide
            if let Some(strokes) = slide_ink.get(&current_idx) {
                render_ink_strokes(&mut buffer, width, height, strokes);
            }
            if let Some(ref current_stroke) = active_pen_stroke {
                render_ink_strokes(
                    &mut buffer,
                    width,
                    height,
                    std::slice::from_ref(current_stroke),
                );
            }

            // Draw laser pointer and trailing effect if in Laser mode
            if presenter_mode == PresenterMode::Laser {
                let trail_data: Vec<(usize, usize, f32)> = laser_trail
                    .iter()
                    .map(|(x, y, t)| {
                        let age_ms = now.duration_since(*t).as_millis() as f32;
                        let freshness = (1.0 - age_ms / 220.0).clamp(0.0, 1.0);
                        (*x, *y, freshness)
                    })
                    .collect();
                render_laser_trail(&mut buffer, width, height, &trail_data, active_color);

                if let Some((mx, my)) = mouse_pos
                    && mx >= 0.0
                    && my >= 0.0
                    && (mx as usize) < width
                    && (my as usize) < height
                {
                    render_laser_pointer(
                        &mut buffer,
                        width,
                        height,
                        mx as usize,
                        my as usize,
                        active_color,
                    );
                }
            }

            // Draw floating interactive dock
            render_dock(
                &mut buffer,
                width,
                height,
                presenter_mode,
                audio_engine.is_muted(),
                audio_engine.volume_percent(),
                hovered_dock_action,
                mouse_near_dock,
                is_fullscreen,
                active_color,
                palette_open,
            );

            // Draw Color Palette popup if open
            if palette_open {
                render_palette_popup(
                    &mut buffer,
                    width,
                    height,
                    is_fullscreen,
                    active_color_idx,
                    hovered_palette_idx,
                );
            }

            // Draw Volume Slider popup if hovering volume
            if show_volume_slider {
                render_volume_slider_popup(
                    &mut buffer,
                    width,
                    height,
                    is_fullscreen,
                    audio_engine.volume(),
                    audio_engine.is_muted(),
                );
            }

            // Draw temporary Volume HUD Toast overlay if volume recently adjusted
            if let Some(t) = last_volume_change {
                let elapsed = t.elapsed().as_secs_f32();
                if elapsed < 1.5 {
                    let toast_alpha = if elapsed < 1.0 {
                        1.0
                    } else {
                        1.0 - (elapsed - 1.0) / 0.5
                    };
                    draw_volume_toast(
                        &mut buffer,
                        width,
                        height,
                        audio_engine.volume_percent(),
                        audio_engine.is_muted(),
                        toast_alpha,
                    );
                }
            }

            // Draw HUD slide & step badges
            draw_page_badge(
                &mut buffer,
                width,
                height,
                current_idx + 1,
                total_slides,
                current_step,
                max_substep,
            );

            if show_help {
                draw_help_overlay(&mut buffer, width, height);
            }

            // Render active Chart Data Inspector modal
            if let Some(ref inspector) = active_chart_inspector {
                let (mx, my) = mouse_pos.unwrap_or((-1.0, -1.0));
                draw_chart_inspector(&mut buffer, width, height, inspector, mx, my);
            }

            window
                .update_with_buffer(&buffer, width, height)
                .map_err(|e| SlideError::Format(format!("Window update error: {}", e)))?;
        }

        Ok(())
    }
}
