//! Graphical PIN prompt dialog for YubiKey PIV authentication during presentation playback.
//!
//! Provides a seamless multi-tier graphical dialog:
//! 1. Native desktop dialog (zenity / kdialog on Linux, osascript on macOS, PowerShell on Windows)
//! 2. Built-in `minifb` software-rendered modal dialog window with password masking
//! 3. Zeroizes sensitive PIN buffers on exit

use minifb::Key;
use minifb::Window;
use minifb::WindowOptions;
use zeroize::Zeroize;
use zeroize::Zeroizing;

/// Prompt the user for their YubiKey PIV PIN using a graphical dialog window.
/// Returns `Some(Zeroizing<String>)` on confirmation, or `None` if canceled or unavailable.
#[must_use]
pub fn prompt_pin_gui(
    title: &str,
    subtitle: &str,
) -> Option<Zeroizing<String>> {
    // Tier 1: Native OS dialogs
    if let Some(pin) = try_native_os_dialog(title, subtitle) {
        return Some(pin);
    }

    // Tier 2: Built-in minifb graphical modal window
    try_minifb_pin_dialog(title, subtitle)
}

fn try_native_os_dialog(
    title: &str,
    subtitle: &str,
) -> Option<Zeroizing<String>> {
    slide_core::security::prompt_secret_gui(title, subtitle)
}

/// Fallback built-in minifb dialog window with dark theme and masked PIN display.
fn try_minifb_pin_dialog(
    title: &str,
    subtitle: &str,
) -> Option<Zeroizing<String>> {
    let width = 480usize;
    let height = 220usize;
    let mut buffer = vec![0u32; width * height];

    let mut window = Window::new(
        title,
        width,
        height,
        WindowOptions {
            resize: false,
            topmost: true,
            ..WindowOptions::default()
        },
    )
    .ok()?;

    window.set_target_fps(60);

    let mut input = String::with_capacity(16);
    let mut confirmed = false;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let shift = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
        // Collect key inputs
        let keys = window.get_keys_pressed(minifb::KeyRepeat::Yes);
        for key in keys {
            if let Some(ch) = map_minifb_key_to_char(key, shift)
                && input.len() < 128
            {
                input.push(ch);
            } else if key == Key::Backspace {
                input.pop();
            } else if key == Key::Enter && !input.is_empty() {
                confirmed = true;
                break;
            }
        }

        if confirmed {
            break;
        }

        // Render dialog card
        buffer.fill(0xFF1E_1E2E); // Dark background

        // Title & subtitle
        crate::hud::draw_text(&mut buffer, width, height, 24, 24, title, 0xFFCA_A6F7);
        crate::hud::draw_text(&mut buffer, width, height, 24, 46, subtitle, 0xFFA6_ADC8);

        // Input box rect
        let box_x = 24usize;
        let box_y = 80usize;
        let box_w = width.saturating_sub(48);
        let box_h = 44usize;

        crate::hud::fill_rect(
            &mut buffer,
            width,
            height,
            box_x,
            box_y,
            box_w,
            box_h,
            0xFF18_1825,
        );
        crate::hud::draw_rect_outline(
            &mut buffer,
            width,
            height,
            box_x,
            box_y,
            box_w,
            box_h,
            0xFF89_B4FA,
        );

        // Masked PIN display: * * * * * *
        let masked = if input.is_empty() {
            String::from("Enter 6-8 digit PIN...")
        } else {
            let mut s = String::new();
            for _ in 0..input.len() {
                s.push('*');
                s.push(' ');
            }
            s
        };
        let text_col = if input.is_empty() {
            0xFF58_5B70
        } else {
            0xFFFF_FFFF
        };
        crate::hud::draw_text(
            &mut buffer,
            width,
            height,
            box_x + 16,
            box_y + 14,
            &masked,
            text_col,
        );

        // Button hints
        crate::hud::draw_text(
            &mut buffer,
            width,
            height,
            24,
            160,
            "[ Enter ] Confirm PIN           [ Esc ] Cancel",
            0xFF93_99B2,
        );

        if window.update_with_buffer(&buffer, width, height).is_err() {
            break;
        }
    }

    if confirmed && !input.is_empty() {
        let res = Zeroizing::new(input.clone());
        input.zeroize();
        Some(res)
    } else {
        input.zeroize();
        None
    }
}

fn map_minifb_key_to_char(
    key: Key,
    shift: bool,
) -> Option<char> {
    match key {
        | Key::Key0 | Key::NumPad0 => Some(if shift { ')' } else { '0' }),
        | Key::Key1 | Key::NumPad1 => Some(if shift { '!' } else { '1' }),
        | Key::Key2 | Key::NumPad2 => Some(if shift { '@' } else { '2' }),
        | Key::Key3 | Key::NumPad3 => Some(if shift { '#' } else { '3' }),
        | Key::Key4 | Key::NumPad4 => Some(if shift { '$' } else { '4' }),
        | Key::Key5 | Key::NumPad5 => Some(if shift { '%' } else { '5' }),
        | Key::Key6 | Key::NumPad6 => Some(if shift { '^' } else { '6' }),
        | Key::Key7 | Key::NumPad7 => Some(if shift { '&' } else { '7' }),
        | Key::Key8 | Key::NumPad8 => Some(if shift { '*' } else { '8' }),
        | Key::Key9 | Key::NumPad9 => Some(if shift { '(' } else { '9' }),
        | Key::A => Some(if shift { 'A' } else { 'a' }),
        | Key::B => Some(if shift { 'B' } else { 'b' }),
        | Key::C => Some(if shift { 'C' } else { 'c' }),
        | Key::D => Some(if shift { 'D' } else { 'd' }),
        | Key::E => Some(if shift { 'E' } else { 'e' }),
        | Key::F => Some(if shift { 'F' } else { 'f' }),
        | Key::G => Some(if shift { 'G' } else { 'g' }),
        | Key::H => Some(if shift { 'H' } else { 'h' }),
        | Key::I => Some(if shift { 'I' } else { 'i' }),
        | Key::J => Some(if shift { 'J' } else { 'j' }),
        | Key::K => Some(if shift { 'K' } else { 'k' }),
        | Key::L => Some(if shift { 'L' } else { 'l' }),
        | Key::M => Some(if shift { 'M' } else { 'm' }),
        | Key::N => Some(if shift { 'N' } else { 'n' }),
        | Key::O => Some(if shift { 'O' } else { 'o' }),
        | Key::P => Some(if shift { 'P' } else { 'p' }),
        | Key::Q => Some(if shift { 'Q' } else { 'q' }),
        | Key::R => Some(if shift { 'R' } else { 'r' }),
        | Key::S => Some(if shift { 'S' } else { 's' }),
        | Key::T => Some(if shift { 'T' } else { 't' }),
        | Key::U => Some(if shift { 'U' } else { 'u' }),
        | Key::V => Some(if shift { 'V' } else { 'v' }),
        | Key::W => Some(if shift { 'W' } else { 'w' }),
        | Key::X => Some(if shift { 'X' } else { 'x' }),
        | Key::Y => Some(if shift { 'Y' } else { 'y' }),
        | Key::Z => Some(if shift { 'Z' } else { 'z' }),
        | Key::Space => Some(' '),
        | Key::Minus => Some(if shift { '_' } else { '-' }),
        | Key::Equal => Some(if shift { '+' } else { '=' }),
        | Key::Period => Some(if shift { '>' } else { '.' }),
        | Key::Comma => Some(if shift { '<' } else { ',' }),
        | Key::Slash => Some(if shift { '?' } else { '/' }),
        | _ => None,
    }
}
