//! Modern, high-end Design System Tokens (Light & Dark) and Glassmorphism styles.
//! Inspired by macOS Sonoma, Linear, Raycast, and Typora.

use iced::Background;
use iced::Color;
use iced::Shadow;
use iced::Vector;
use iced::border::Border;
use iced::border::{
    self,
};
use iced::widget::button;
use iced::widget::container;
use iced::widget::text_editor;

// -------------------------------------------------------------------------
// Design System Tokens: Border Radii Hierarchy
// -------------------------------------------------------------------------

/// Micro-radii: tags, badges, indicator dots, inline code snippets
pub const RADIUS_XS: f32 = 4.0;
/// Small-radii: standard action buttons, text inputs, chips, element pills
pub const RADIUS_SM: f32 = 7.0;
/// Medium-radii: content cards, sidebar outline cards, editor panels
pub const RADIUS_MD: f32 = 12.0;
/// Large-radii: 16:9 presentation canvas viewport, floating island panels
pub const RADIUS_LG: f32 = 16.0;
/// Extra large-radii: modals, primary dialog frames, alert cards
pub const RADIUS_XL: f32 = 22.0;
/// Full capsule-radii: segmented switcher pills, floating capsule bars, round indicator tags
pub const RADIUS_FULL: f32 = 999.0;

// -------------------------------------------------------------------------
// Theme Definition & Palette Tokens
// -------------------------------------------------------------------------

/// Application color theme
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppTheme {
    /// Crisp Titanium & Alabaster Light Mode
    #[default]
    Light,
    /// Deep Obsidian & Slate Dark Mode
    Dark,
}

impl AppTheme {
    /// Return whether this theme is Dark mode
    #[must_use]
    pub const fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }

    /// Return whether this theme is Light mode
    #[must_use]
    pub const fn is_light(self) -> bool {
        matches!(self, Self::Light)
    }

    /// Toggle between Light and Dark mode
    #[must_use]
    pub const fn toggle(self) -> Self {
        match self {
            | Self::Light => Self::Dark,
            | Self::Dark => Self::Light,
        }
    }

    /// Main canvas backdrop (Crisp titanium snow vs Deep obsidian space)
    #[must_use]
    pub const fn bg_canvas(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.953, 0.961, 0.973), // #f3f5f8
            | Self::Dark => Color::from_rgb(0.043, 0.055, 0.078),  // #0b0e14
        }
    }

    /// Acrylic translucent background of headers, toolbars, and panels (85% opacity)
    #[must_use]
    pub const fn bg_surface(self) -> Color {
        match self {
            | Self::Light => Color::from_rgba(1.0, 1.0, 1.0, 0.85),
            | Self::Dark => Color::from_rgba(0.078, 0.098, 0.145, 0.85), // #141925 85%
        }
    }

    /// Solid surface background
    #[must_use]
    pub const fn bg_surface_solid(self) -> Color {
        match self {
            | Self::Light => Color::WHITE,
            | Self::Dark => Color::from_rgb(0.078, 0.098, 0.145), // #141925
        }
    }

    /// Background of cards, slide containers, and editor panels
    #[must_use]
    pub const fn bg_card(self) -> Color {
        match self {
            | Self::Light => Color::WHITE,
            | Self::Dark => Color::from_rgb(0.098, 0.125, 0.180), // #19202e
        }
    }

    /// Interactive subtle background for hover states, tracks, and chips
    #[must_use]
    pub const fn bg_subtle(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.925, 0.941, 0.961), // #ecf0f5
            | Self::Dark => Color::from_rgb(0.133, 0.169, 0.235),  // #222b3c
        }
    }

    /// Elevated surface for floating popovers and active cards
    #[must_use]
    pub const fn bg_elevated(self) -> Color {
        match self {
            | Self::Light => Color::WHITE,
            | Self::Dark => Color::from_rgb(0.165, 0.208, 0.290), // #2a354a
        }
    }

    /// Standard hairline border color (crisp Project Nova definition)
    #[must_use]
    pub const fn border_color(self) -> Color {
        match self {
            | Self::Light => Color::from_rgba(0.06, 0.09, 0.16, 0.16),
            | Self::Dark => Color::from_rgba(1.0, 1.0, 1.0, 0.16),
        }
    }

    /// Micro hairline border color for nested elements
    #[must_use]
    pub const fn border_subtle(self) -> Color {
        match self {
            | Self::Light => Color::from_rgba(0.06, 0.09, 0.16, 0.10),
            | Self::Dark => Color::from_rgba(1.0, 1.0, 1.0, 0.10),
        }
    }

    /// Active / focused border color (Electric Indigo)
    #[must_use]
    pub const fn border_active(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.310, 0.275, 0.898), // #4f46e5 (Indigo-600)
            | Self::Dark => Color::from_rgb(0.388, 0.400, 0.945),  // #6366f1 (Indigo-500)
        }
    }

    /// Primary text color
    #[must_use]
    pub const fn text_primary(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.059, 0.090, 0.165), // #0f172a (Slate-900)
            | Self::Dark => Color::from_rgb(0.973, 0.980, 0.988),  // #f8fafc (Slate-50)
        }
    }

    /// Secondary text color
    #[must_use]
    pub const fn text_secondary(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.278, 0.333, 0.412), // #475569 (Slate-600)
            | Self::Dark => Color::from_rgb(0.580, 0.639, 0.722),  // #94a3b8 (Slate-400)
        }
    }

    /// Muted hint text color
    #[must_use]
    pub const fn text_muted(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.580, 0.639, 0.722), // #94a3b8 (Slate-400)
            | Self::Dark => Color::from_rgb(0.392, 0.455, 0.545),  // #64748b (Slate-500)
        }
    }

    /// Modern accent brand color (Electric Indigo)
    #[must_use]
    pub const fn accent(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.310, 0.275, 0.898), // #4f46e5
            | Self::Dark => Color::from_rgb(0.388, 0.400, 0.945),  // #6366f1
        }
    }

    /// Modern accent hover color
    #[must_use]
    pub const fn accent_hover(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.263, 0.220, 0.792), // #4338ca
            | Self::Dark => Color::from_rgb(0.506, 0.549, 0.973),  // #818cf8
        }
    }

    /// Vibrant Cyan secondary accent color
    #[must_use]
    pub const fn accent_cyan(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.008, 0.518, 0.780), // #0284c7 (Sky-600)
            | Self::Dark => Color::from_rgb(0.220, 0.741, 0.973),  // #38bdf8 (Sky-400)
        }
    }

    /// Danger / error color (Rose)
    #[must_use]
    pub const fn danger(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.882, 0.114, 0.282), // #e11d48 (Rose-600)
            | Self::Dark => Color::from_rgb(0.957, 0.247, 0.369),  // #f43f5e (Rose-500)
        }
    }

    /// Warning color (Amber)
    #[must_use]
    pub const fn warning(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.851, 0.467, 0.024), // #d97706 (Amber-600)
            | Self::Dark => Color::from_rgb(0.961, 0.620, 0.043),  // #f59e0b (Amber-500)
        }
    }

    /// Success color (Emerald)
    #[must_use]
    pub const fn success(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.020, 0.588, 0.412), // #059669 (Emerald-600)
            | Self::Dark => Color::from_rgb(0.063, 0.725, 0.506),  // #10b981 (Emerald-500)
        }
    }

    /// Glass highlight top edge reflection
    #[must_use]
    pub const fn glass_highlight(self) -> Color {
        match self {
            | Self::Light => Color::from_rgba(1.0, 1.0, 1.0, 0.85),
            | Self::Dark => Color::from_rgba(1.0, 1.0, 1.0, 0.12),
        }
    }

    /// Deep backdrop mask for modal overlays
    #[must_use]
    pub const fn backdrop(self) -> Color {
        match self {
            | Self::Light => Color::from_rgba(0.06, 0.09, 0.16, 0.45),
            | Self::Dark => Color::from_rgba(0.02, 0.03, 0.05, 0.75),
        }
    }
}

// -------------------------------------------------------------------------
// Design System Tokens: Multi-Level Shadows & Elevation
// -------------------------------------------------------------------------

/// Micro-elevation: Subtle depth for buttons, chips, and list hover states
#[must_use]
pub fn elevation_subtle(theme: AppTheme) -> Shadow {
    Shadow {
        color: match theme {
            | AppTheme::Light => Color::from_rgba(0.0, 0.0, 0.0, 0.04),
            | AppTheme::Dark => Color::from_rgba(0.0, 0.0, 0.0, 0.20),
        },
        offset: Vector::new(0.0, 1.0),
        blur_radius: 3.0,
    }
}

/// Low elevation: Cards, segmented switchers, and subtle floating items
#[must_use]
pub fn elevation_low(theme: AppTheme) -> Shadow {
    Shadow {
        color: match theme {
            | AppTheme::Light => Color::from_rgba(0.0, 0.0, 0.0, 0.06),
            | AppTheme::Dark => Color::from_rgba(0.0, 0.0, 0.0, 0.32),
        },
        offset: Vector::new(0.0, 2.0),
        blur_radius: 6.0,
    }
}

/// Medium elevation: Floating toolbars, active outline cards, dropdowns
#[must_use]
pub fn elevation_mid(theme: AppTheme) -> Shadow {
    Shadow {
        color: match theme {
            | AppTheme::Light => Color::from_rgba(0.0, 0.0, 0.0, 0.08),
            | AppTheme::Dark => Color::from_rgba(0.0, 0.0, 0.0, 0.45),
        },
        offset: Vector::new(0.0, 4.0),
        blur_radius: 16.0,
    }
}

/// High elevation: Active 16:9 presentation slide canvas, high-depth panels
#[must_use]
pub fn elevation_high(theme: AppTheme) -> Shadow {
    Shadow {
        color: match theme {
            | AppTheme::Light => Color::from_rgba(0.0, 0.0, 0.0, 0.12),
            | AppTheme::Dark => Color::from_rgba(0.0, 0.0, 0.0, 0.60),
        },
        offset: Vector::new(0.0, 8.0),
        blur_radius: 28.0,
    }
}

/// Dialog elevation: Centered modal windows and heavy overlay dialog cards
#[must_use]
pub fn elevation_dialog(theme: AppTheme) -> Shadow {
    Shadow {
        color: match theme {
            | AppTheme::Light => Color::from_rgba(0.0, 0.0, 0.0, 0.18),
            | AppTheme::Dark => Color::from_rgba(0.0, 0.0, 0.0, 0.75),
        },
        offset: Vector::new(0.0, 16.0),
        blur_radius: 48.0,
    }
}

/// Luminous neon accent glow for active buttons and focused cards
#[must_use]
pub fn glow_accent(theme: AppTheme) -> Shadow {
    Shadow {
        color: match theme {
            | AppTheme::Light => Color::from_rgba(0.31, 0.27, 0.90, 0.28),
            | AppTheme::Dark => Color::from_rgba(0.39, 0.40, 0.95, 0.38),
        },
        offset: Vector::new(0.0, 2.0),
        blur_radius: 12.0,
    }
}

/// Soft danger alert glow
#[must_use]
pub fn glow_danger(theme: AppTheme) -> Shadow {
    Shadow {
        color: match theme {
            | AppTheme::Light => Color::from_rgba(0.88, 0.11, 0.28, 0.22),
            | AppTheme::Dark => Color::from_rgba(0.96, 0.25, 0.37, 0.30),
        },
        offset: Vector::new(0.0, 2.0),
        blur_radius: 10.0,
    }
}

// -------------------------------------------------------------------------
// Container Style Helpers: Frosted Glass & Surface Panels
// -------------------------------------------------------------------------

/// Style for top navigation bar header (frosted acrylic with subtle bottom glow)
#[must_use]
pub fn header_container_style(theme: AppTheme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme.bg_surface())),
        text_color: Some(theme.text_primary()),
        border: Border {
            color: theme.border_color(),
            width: 1.0,
            radius: border::Radius::from(0.0),
        },
        shadow: elevation_subtle(theme),
        snap: true,
    }
}

/// Style for bottom status bar (frosted acrylic with top hairline border)
#[must_use]
pub fn statusbar_container_style(theme: AppTheme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme.bg_surface())),
        text_color: Some(theme.text_secondary()),
        border: Border {
            color: theme.border_color(),
            width: 1.0,
            radius: border::Radius::from(0.0),
        },
        shadow: Shadow {
            color: match theme {
                | AppTheme::Light => Color::from_rgba(0.0, 0.0, 0.0, 0.03),
                | AppTheme::Dark => Color::from_rgba(0.0, 0.0, 0.0, 0.18),
            },
            offset: Vector::new(0.0, -1.0),
            blur_radius: 4.0,
        },
        snap: true,
    }
}

/// Style for left outline sidebar (frosted acrylic panel)
#[must_use]
pub fn sidebar_container_style(theme: AppTheme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme.bg_surface())),
        text_color: Some(theme.text_primary()),
        border: Border {
            color: theme.border_color(),
            width: 1.0,
            radius: border::Radius::from(0.0),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// Style for floating or slide presentation card frames (16px radius, elevation)
#[must_use]
pub fn slide_card_style(
    theme: AppTheme,
    is_active: bool,
) -> container::Style {
    let (border_color, border_width, shadow) = if is_active {
        (theme.border_active(), 1.5, glow_accent(theme))
    } else {
        (theme.border_color(), 1.0, elevation_low(theme))
    };

    container::Style {
        background: Some(Background::Color(theme.bg_card())),
        text_color: Some(theme.text_primary()),
        border: Border {
            color: border_color,
            width: border_width,
            radius: border::Radius::from(RADIUS_LG),
        },
        shadow,
        snap: true,
    }
}

/// Style for 16:9 presentation slide canvas surface (crisp viewport, high depth)
#[must_use]
pub fn slide_presentation_surface_style(
    theme: AppTheme,
    is_active: bool,
) -> container::Style {
    container::Style {
        background: Some(Background::Color(match theme {
            | AppTheme::Light => Color::WHITE,
            | AppTheme::Dark => Color::from_rgb(0.070, 0.086, 0.120), // #12161f
        })),
        text_color: Some(theme.text_primary()),
        border: Border {
            color: if is_active {
                theme.border_active()
            } else {
                theme.border_color()
            },
            width: if is_active { 1.5 } else { 1.0 },
            radius: border::Radius::from(RADIUS_MD),
        },
        shadow: if is_active {
            elevation_high(theme)
        } else {
            elevation_mid(theme)
        },
        snap: true,
    }
}

/// Style for floating capsule island bars (formatting bar, floating tool palette)
#[must_use]
pub fn floating_capsule_style(theme: AppTheme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme.bg_surface())),
        text_color: Some(theme.text_primary()),
        border: Border {
            color: theme.border_color(),
            width: 1.0,
            radius: border::Radius::from(RADIUS_FULL),
        },
        shadow: elevation_mid(theme),
        snap: true,
    }
}

/// Style for segmented mode switcher pill container track
#[must_use]
pub fn segmented_pill_container_style(theme: AppTheme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme.bg_subtle())),
        text_color: Some(theme.text_primary()),
        border: Border {
            color: theme.border_color(),
            width: 1.0,
            radius: border::Radius::from(RADIUS_FULL),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// Style for general cards and nested editors
#[must_use]
pub fn card_style(theme: AppTheme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme.bg_card())),
        text_color: Some(theme.text_primary()),
        border: Border {
            color: theme.border_color(),
            width: 1.0,
            radius: border::Radius::from(RADIUS_MD),
        },
        shadow: elevation_low(theme),
        snap: true,
    }
}

/// Style for modal dialog card (22px radius, diffused elevation)
#[must_use]
pub fn modal_dialog_style(theme: AppTheme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme.bg_card())),
        text_color: Some(theme.text_primary()),
        border: Border {
            color: theme.border_color(),
            width: 1.0,
            radius: border::Radius::from(RADIUS_XL),
        },
        shadow: elevation_dialog(theme),
        snap: true,
    }
}

/// Style for full-screen modal backdrop overlay
#[must_use]
pub fn modal_backdrop_style(theme: AppTheme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme.backdrop())),
        ..container::Style::default()
    }
}

/// Style for code block container with modern macOS-style rounded aesthetics
#[must_use]
pub fn code_block_container_style(theme: AppTheme) -> container::Style {
    container::Style {
        background: Some(Background::Color(match theme {
            | AppTheme::Light => Color::from_rgb(0.965, 0.973, 0.985),
            | AppTheme::Dark => Color::from_rgb(0.065, 0.080, 0.115),
        })),
        text_color: Some(theme.text_primary()),
        border: Border {
            color: theme.border_color(),
            width: 1.0,
            radius: border::Radius::from(RADIUS_MD),
        },
        shadow: elevation_subtle(theme),
        snap: true,
    }
}

/// Style for callout container with rounded corners and left-accent bar
#[must_use]
pub fn callout_container_style(
    theme: AppTheme,
    accent_color: Color,
) -> container::Style {
    let bg_color = match theme {
        | AppTheme::Light => Color::from_rgba(accent_color.r, accent_color.g, accent_color.b, 0.06),
        | AppTheme::Dark => Color::from_rgba(accent_color.r, accent_color.g, accent_color.b, 0.10),
    };

    container::Style {
        background: Some(Background::Color(bg_color)),
        text_color: Some(theme.text_primary()),
        border: Border {
            color: accent_color.scale_alpha(0.35),
            width: 1.0,
            radius: border::Radius::from(RADIUS_MD),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

// -------------------------------------------------------------------------
// Button Style Helpers
// -------------------------------------------------------------------------

/// Primary brand action button (Indigo background, white text, crisp contrast border, glow shadow)
#[must_use]
pub fn primary_button_style(theme: AppTheme) -> button::Style {
    button::Style {
        background: Some(Background::Color(theme.accent())),
        text_color: Color::WHITE,
        border: Border {
            color: match theme {
                | AppTheme::Light => Color::from_rgb(0.24, 0.20, 0.72),
                | AppTheme::Dark => Color::from_rgba(1.0, 1.0, 1.0, 0.28),
            },
            width: 1.0,
            radius: border::Radius::from(RADIUS_SM),
        },
        shadow: glow_accent(theme),
        snap: true,
    }
}

/// Subtle / secondary toolbar button (Project Nova style with crisp borders and soft elevation)
#[must_use]
pub fn subtle_button_style(
    theme: AppTheme,
    is_active: bool,
) -> button::Style {
    let (bg, txt, brd, shadow) = if is_active {
        (
            theme.bg_subtle(),
            theme.accent(),
            theme.border_active(),
            elevation_subtle(theme),
        )
    } else {
        (
            match theme {
                | AppTheme::Light => Color::WHITE,
                | AppTheme::Dark => Color::from_rgb(0.125, 0.157, 0.224),
            },
            theme.text_primary(),
            theme.border_color(),
            elevation_subtle(theme),
        )
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: txt,
        border: Border {
            color: brd,
            width: 1.0,
            radius: border::Radius::from(RADIUS_SM),
        },
        shadow,
        snap: true,
    }
}

/// Tactile chip button style for quick formatting tools and toolbar capsules
#[must_use]
pub fn chip_button_style(
    theme: AppTheme,
    is_active: bool,
) -> button::Style {
    let (bg, txt, brd) = if is_active {
        (theme.accent(), Color::WHITE, theme.border_active())
    } else {
        (
            match theme {
                | AppTheme::Light => Color::from_rgb(0.965, 0.973, 0.985),
                | AppTheme::Dark => Color::from_rgb(0.133, 0.169, 0.235),
            },
            theme.text_primary(),
            theme.border_color(),
        )
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: txt,
        border: Border {
            color: brd,
            width: 1.0,
            radius: border::Radius::from(RADIUS_XS),
        },
        shadow: elevation_subtle(theme),
        snap: true,
    }
}

/// Style for segmented button inside a pill bar
#[must_use]
pub fn segmented_button_style(
    theme: AppTheme,
    is_active: bool,
) -> button::Style {
    if is_active {
        button::Style {
            background: Some(Background::Color(theme.bg_card())),
            text_color: theme.accent(),
            border: Border {
                color: theme.border_color(),
                width: 1.0,
                radius: border::Radius::from(RADIUS_FULL),
            },
            shadow: elevation_subtle(theme),
            snap: true,
        }
    } else {
        button::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            text_color: theme.text_secondary(),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: border::Radius::from(RADIUS_FULL),
            },
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

/// Danger action button (Rose tint, danger text and border)
#[must_use]
pub fn danger_button_style(theme: AppTheme) -> button::Style {
    let bg = match theme {
        | AppTheme::Light => Color::from_rgba(0.88, 0.11, 0.28, 0.07),
        | AppTheme::Dark => Color::from_rgba(0.96, 0.25, 0.37, 0.12),
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: theme.danger(),
        border: Border {
            color: theme.danger().scale_alpha(0.35),
            width: 1.0,
            radius: border::Radius::from(RADIUS_SM),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// Style for fine-grained element chips
#[must_use]
pub fn element_chip_button_style(
    theme: AppTheme,
    is_active: bool,
) -> button::Style {
    if is_active {
        button::Style {
            background: Some(Background::Color(match theme {
                | AppTheme::Light => Color::from_rgba(0.31, 0.27, 0.90, 0.12),
                | AppTheme::Dark => Color::from_rgba(0.39, 0.40, 0.95, 0.22),
            })),
            text_color: theme.accent(),
            border: Border {
                color: theme.accent(),
                width: 1.5,
                radius: border::Radius::from(RADIUS_SM),
            },
            shadow: elevation_subtle(theme),
            snap: true,
        }
    } else {
        button::Style {
            background: Some(Background::Color(theme.bg_subtle())),
            text_color: theme.text_secondary(),
            border: Border {
                color: theme.border_color(),
                width: 1.0,
                radius: border::Radius::from(RADIUS_SM),
            },
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

/// Style for circular or capsule pill buttons (full radius)
#[must_use]
pub fn pill_button_style(
    theme: AppTheme,
    is_active: bool,
) -> button::Style {
    let (bg, txt, brd) = if is_active {
        (theme.bg_subtle(), theme.accent(), theme.border_active())
    } else {
        (
            theme.bg_surface_solid(),
            theme.text_primary(),
            theme.border_color(),
        )
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: txt,
        border: Border {
            color: brd,
            width: 1.0,
            radius: border::Radius::from(RADIUS_FULL),
        },
        shadow: elevation_subtle(theme),
        snap: true,
    }
}

// -------------------------------------------------------------------------
// Text Editor Style Helpers
// -------------------------------------------------------------------------

/// Text editor styling for Typst code
#[must_use]
pub fn editor_style(theme: AppTheme) -> text_editor::Style {
    text_editor::Style {
        background: Background::Color(theme.bg_card()),
        border: Border {
            color: theme.border_color(),
            width: 1.0,
            radius: border::Radius::from(RADIUS_SM),
        },
        placeholder: theme.text_muted(),
        value: theme.text_primary(),
        selection: Color::from_rgba(0.39, 0.40, 0.95, 0.25),
    }
}
