//! Modern Design System Tokens, Glassmorphism, and Theme styles for Slide Viewer launcher.

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
use iced::widget::pick_list;
use iced::widget::text_input;

// -------------------------------------------------------------------------
// Radii Hierarchy Tokens
// -------------------------------------------------------------------------

pub const RADIUS_XS: f32 = 4.0;
pub const RADIUS_SM: f32 = 8.0;
pub const RADIUS_MD: f32 = 12.0;
pub const RADIUS_LG: f32 = 16.0;
pub const RADIUS_XL: f32 = 22.0;
pub const RADIUS_FULL: f32 = 999.0;

// -------------------------------------------------------------------------
// ViewerTheme Definition & Palette Tokens
// -------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewerTheme {
    Light,
    #[default]
    Dark,
}

impl ViewerTheme {
    #[must_use]
    pub const fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }

    #[must_use]
    pub const fn toggle(self) -> Self {
        match self {
            | Self::Light => Self::Dark,
            | Self::Dark => Self::Light,
        }
    }

    /// Main canvas background
    #[must_use]
    pub const fn bg_canvas(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.953, 0.961, 0.973), // #f3f5f8
            | Self::Dark => Color::from_rgb(0.043, 0.055, 0.078),  // #0b0e14
        }
    }

    /// Acrylic translucent surface background (panels, headers)
    #[must_use]
    pub const fn bg_surface(self) -> Color {
        match self {
            | Self::Light => Color::from_rgba(1.0, 1.0, 1.0, 0.92),
            | Self::Dark => Color::from_rgba(0.078, 0.098, 0.145, 0.90), // #141925 90%
        }
    }

    /// Card background
    #[must_use]
    pub const fn bg_card(self) -> Color {
        match self {
            | Self::Light => Color::WHITE,
            | Self::Dark => Color::from_rgb(0.094, 0.121, 0.176), // #181f2d
        }
    }

    /// Active / hovered card background
    #[must_use]
    pub const fn bg_card_active(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.965, 0.980, 1.0), // Tinted subtle blue
            | Self::Dark => Color::from_rgb(0.125, 0.165, 0.243), // #202a3e
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

    /// Primary accent color (Sapphire / Sky blue)
    #[must_use]
    pub const fn accent(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.008, 0.518, 0.780), // #0284c7
            | Self::Dark => Color::from_rgb(0.220, 0.741, 0.973),  // #38bdf8
        }
    }

    /// Accent hover color
    #[must_use]
    pub const fn accent_hover(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.012, 0.420, 0.635), // #0369a1
            | Self::Dark => Color::from_rgb(0.447, 0.827, 0.988),  // #7dd3fc
        }
    }

    /// Secondary accent color (Purple / Violet)
    #[must_use]
    pub const fn accent_purple(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.486, 0.227, 0.929), // #7c3aed
            | Self::Dark => Color::from_rgb(0.655, 0.482, 0.980),  // #a78bfa
        }
    }

    /// Emerald success color
    #[must_use]
    pub const fn success(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.020, 0.588, 0.412), // #059669
            | Self::Dark => Color::from_rgb(0.204, 0.827, 0.600),  // #34d399
        }
    }

    /// Primary high-contrast text
    #[must_use]
    pub const fn text_primary(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.059, 0.090, 0.165), // #0f172a
            | Self::Dark => Color::from_rgb(0.973, 0.980, 0.988),  // #f8fafc
        }
    }

    /// Secondary muted text
    #[must_use]
    pub const fn text_secondary(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.392, 0.447, 0.537), // #64748b
            | Self::Dark => Color::from_rgb(0.580, 0.639, 0.722),  // #94a3b8
        }
    }

    /// Subtle border
    #[must_use]
    pub const fn border_color(self) -> Color {
        match self {
            | Self::Light => Color::from_rgba(0.059, 0.090, 0.165, 0.18),
            | Self::Dark => Color::from_rgba(1.0, 1.0, 1.0, 0.18),
        }
    }

    /// Active / focus border
    #[must_use]
    pub const fn border_active(self) -> Color {
        match self {
            | Self::Light => Color::from_rgb(0.008, 0.518, 0.780),
            | Self::Dark => Color::from_rgb(0.220, 0.741, 0.973),
        }
    }
}

// -------------------------------------------------------------------------
// Shadows
// -------------------------------------------------------------------------

#[must_use]
pub fn shadow_card(theme: ViewerTheme) -> Shadow {
    Shadow {
        color: match theme {
            | ViewerTheme::Light => Color::from_rgba(0.0, 0.0, 0.0, 0.06),
            | ViewerTheme::Dark => Color::from_rgba(0.0, 0.0, 0.0, 0.35),
        },
        offset: Vector::new(0.0, 4.0),
        blur_radius: 12.0,
    }
}

#[must_use]
pub fn shadow_active(theme: ViewerTheme) -> Shadow {
    Shadow {
        color: match theme {
            | ViewerTheme::Light => Color::from_rgba(0.008, 0.518, 0.780, 0.20),
            | ViewerTheme::Dark => Color::from_rgba(0.220, 0.741, 0.973, 0.30),
        },
        offset: Vector::new(0.0, 6.0),
        blur_radius: 18.0,
    }
}

#[must_use]
pub fn shadow_button_primary(theme: ViewerTheme) -> Shadow {
    Shadow {
        color: match theme {
            | ViewerTheme::Light => Color::from_rgba(0.008, 0.518, 0.780, 0.30),
            | ViewerTheme::Dark => Color::from_rgba(0.220, 0.741, 0.973, 0.35),
        },
        offset: Vector::new(0.0, 3.0),
        blur_radius: 8.0,
    }
}

#[must_use]
pub fn shadow_subtle(theme: ViewerTheme) -> Shadow {
    Shadow {
        color: match theme {
            | ViewerTheme::Light => Color::from_rgba(0.0, 0.0, 0.0, 0.04),
            | ViewerTheme::Dark => Color::from_rgba(0.0, 0.0, 0.0, 0.22),
        },
        offset: Vector::new(0.0, 1.0),
        blur_radius: 3.0,
    }
}

// -------------------------------------------------------------------------
// Container Styles
// -------------------------------------------------------------------------

#[must_use]
pub fn canvas_container(theme: ViewerTheme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme.bg_canvas())),
        text_color: Some(theme.text_primary()),
        ..container::Style::default()
    }
}

#[must_use]
pub fn header_container(theme: ViewerTheme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme.bg_surface())),
        border: Border {
            color: theme.border_color(),
            width: 1.0,
            radius: border::Radius::from(0.0),
        },
        shadow: Shadow {
            color: match theme {
                | ViewerTheme::Light => Color::from_rgba(0.0, 0.0, 0.0, 0.04),
                | ViewerTheme::Dark => Color::from_rgba(0.0, 0.0, 0.0, 0.25),
            },
            offset: Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
        ..container::Style::default()
    }
}

#[must_use]
pub fn card_container(
    theme: ViewerTheme,
    is_selected: bool,
) -> container::Style {
    if is_selected {
        container::Style {
            background: Some(Background::Color(theme.bg_card_active())),
            border: Border {
                color: theme.border_active(),
                width: 1.5,
                radius: border::Radius::from(RADIUS_MD),
            },
            shadow: shadow_active(theme),
            ..container::Style::default()
        }
    } else {
        container::Style {
            background: Some(Background::Color(theme.bg_card())),
            border: Border {
                color: theme.border_color(),
                width: 1.0,
                radius: border::Radius::from(RADIUS_MD),
            },
            shadow: shadow_card(theme),
            ..container::Style::default()
        }
    }
}

#[must_use]
pub fn sidebar_container(theme: ViewerTheme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme.bg_surface())),
        border: Border {
            color: theme.border_color(),
            width: 1.0,
            radius: border::Radius::from(RADIUS_LG),
        },
        shadow: shadow_card(theme),
        ..container::Style::default()
    }
}

#[must_use]
pub fn footer_container(theme: ViewerTheme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme.bg_surface())),
        border: Border {
            color: theme.border_color(),
            width: 1.0,
            radius: border::Radius::from(0.0),
        },
        ..container::Style::default()
    }
}

#[must_use]
pub fn badge_container(
    bg: Color,
    border_color: Color,
) -> container::Style {
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: border::Radius::from(RADIUS_FULL),
        },
        ..container::Style::default()
    }
}

#[must_use]
pub fn modal_backdrop_style(theme: ViewerTheme) -> container::Style {
    container::Style {
        background: Some(Background::Color(match theme {
            | ViewerTheme::Light => Color::from_rgba(0.08, 0.12, 0.18, 0.45),
            | ViewerTheme::Dark => Color::from_rgba(0.02, 0.03, 0.05, 0.70),
        })),
        ..container::Style::default()
    }
}

#[must_use]
pub fn modal_dialog_style(theme: ViewerTheme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme.bg_card())),
        border: Border {
            color: theme.border_color(),
            width: 1.0,
            radius: border::Radius::from(RADIUS_LG),
        },
        shadow: shadow_card(theme),
        ..container::Style::default()
    }
}

// -------------------------------------------------------------------------
// Button Styles
// -------------------------------------------------------------------------

#[must_use]
pub fn primary_button_style(theme: ViewerTheme) -> button::Style {
    button::Style {
        background: Some(Background::Color(theme.accent())),
        text_color: match theme {
            | ViewerTheme::Light => Color::WHITE,
            | ViewerTheme::Dark => Color::from_rgb(0.04, 0.06, 0.10),
        },
        border: Border {
            color: match theme {
                | ViewerTheme::Light => Color::from_rgb(0.005, 0.42, 0.65),
                | ViewerTheme::Dark => Color::from_rgba(1.0, 1.0, 1.0, 0.28),
            },
            width: 1.0,
            radius: border::Radius::from(RADIUS_SM),
        },
        shadow: shadow_button_primary(theme),
        snap: true,
    }
}

#[must_use]
pub fn secondary_button_style(theme: ViewerTheme) -> button::Style {
    button::Style {
        background: Some(Background::Color(match theme {
            | ViewerTheme::Light => Color::WHITE,
            | ViewerTheme::Dark => Color::from_rgb(0.115, 0.145, 0.210),
        })),
        text_color: theme.text_primary(),
        border: Border {
            color: theme.border_color(),
            width: 1.0,
            radius: border::Radius::from(RADIUS_SM),
        },
        shadow: shadow_subtle(theme),
        snap: true,
    }
}

#[must_use]
pub fn success_button_style(theme: ViewerTheme) -> button::Style {
    button::Style {
        background: Some(Background::Color(theme.success())),
        text_color: Color::WHITE,
        border: Border {
            color: match theme {
                | ViewerTheme::Light => Color::from_rgb(0.015, 0.48, 0.33),
                | ViewerTheme::Dark => Color::from_rgba(1.0, 1.0, 1.0, 0.28),
            },
            width: 1.0,
            radius: border::Radius::from(RADIUS_SM),
        },
        shadow: shadow_button_primary(theme),
        snap: true,
    }
}

#[must_use]
pub fn chip_button_style(
    theme: ViewerTheme,
    is_active: bool,
) -> button::Style {
    if is_active {
        button::Style {
            background: Some(Background::Color(theme.bg_card_active())),
            text_color: theme.accent(),
            border: Border {
                color: theme.border_active(),
                width: 1.0,
                radius: border::Radius::from(RADIUS_FULL),
            },
            shadow: Shadow::default(),
            snap: true,
        }
    } else {
        button::Style {
            background: Some(Background::Color(theme.bg_subtle())),
            text_color: theme.text_secondary(),
            border: Border {
                color: theme.border_color(),
                width: 1.0,
                radius: border::Radius::from(RADIUS_FULL),
            },
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

// -------------------------------------------------------------------------
// Input & Pick List Styles
// -------------------------------------------------------------------------

#[must_use]
pub fn search_input_style(theme: ViewerTheme) -> text_input::Style {
    text_input::Style {
        background: Background::Color(theme.bg_card()),
        border: Border {
            color: theme.border_color(),
            width: 1.0,
            radius: border::Radius::from(RADIUS_SM),
        },
        icon: theme.text_secondary(),
        placeholder: theme.text_secondary(),
        value: theme.text_primary(),
        selection: theme.accent(),
    }
}

#[must_use]
pub fn pick_list_style(theme: ViewerTheme) -> pick_list::Style {
    pick_list::Style {
        text_color: theme.text_primary(),
        placeholder_color: theme.text_secondary(),
        handle_color: theme.text_secondary(),
        background: Background::Color(theme.bg_card()),
        border: Border {
            color: theme.border_color(),
            width: 1.0,
            radius: border::Radius::from(RADIUS_SM),
        },
    }
}
