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
    #[cfg(target_os = "windows")]
    {
        let ps_script = format!(
            "[void][System.Reflection.Assembly]::LoadWithPartialName('Microsoft.VisualBasic'); \
             [Microsoft.VisualBasic.Interaction]::InputBox('{}', '{}')",
            subtitle, title
        );
        if let Ok(out) = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_script])
            .output()
        {
            if out.status.success() {
                let mut s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !s.is_empty() {
                    let res = Zeroizing::new(s.clone());
                    s.zeroize();
                    return Some(res);
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "Tell application \"System Events\" to display dialog \"{}\" default answer \"\" with hidden answer with title \"{}\"",
            subtitle, title
        );
        if let Ok(out) = std::process::Command::new("osascript")
            .args(["-e", &script])
            .output()
        {
            if out.status.success() {
                let stdout_str = String::from_utf8_lossy(&out.stdout);
                if let Some(pos) = stdout_str.find("text returned:") {
                    let after = &stdout_str[pos + "text returned:".len()..];
                    let mut pin = after.split(',').next().unwrap_or("").trim().to_string();
                    if !pin.is_empty() {
                        let res = Zeroizing::new(pin.clone());
                        pin.zeroize();
                        return Some(res);
                    }
                }
            }
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok() {
            // 1. Zenity (GNOME / standard desktop) with prompt text and masked input
            let zenity_title = format!("--title={title}");
            let zenity_text = format!("--text={subtitle}");
            if let Ok(out) = std::process::Command::new("zenity")
                .args(["--entry", "--hide-text", &zenity_title, &zenity_text])
                .output()
            {
                if out.status.success() {
                    let mut s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !s.is_empty() {
                        let res = Zeroizing::new(s.clone());
                        s.zeroize();
                        return Some(res);
                    }
                }
                return None;
            }

            // Fallback to zenity --password
            if let Ok(out) = std::process::Command::new("zenity")
                .args(["--password", &zenity_title])
                .output()
            {
                if out.status.success() {
                    let mut s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !s.is_empty() {
                        let res = Zeroizing::new(s.clone());
                        s.zeroize();
                        return Some(res);
                    }
                }
                return None;
            }

            // 2. Kdialog (KDE Plasma)
            if let Ok(out) = std::process::Command::new("kdialog")
                .args(["--password", subtitle, "--title", title])
                .output()
            {
                if out.status.success() {
                    let mut s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !s.is_empty() {
                        let res = Zeroizing::new(s.clone());
                        s.zeroize();
                        return Some(res);
                    }
                }
                return None;
            }
        }
    }

    None
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
        // Collect key inputs
        let keys = window.get_keys_pressed(minifb::KeyRepeat::Yes);
        for key in keys {
            match key {
                | Key::Key0 | Key::NumPad0 => {
                    if input.len() < 8 {
                        input.push('0');
                    }
                },
                | Key::Key1 | Key::NumPad1 => {
                    if input.len() < 8 {
                        input.push('1');
                    }
                },
                | Key::Key2 | Key::NumPad2 => {
                    if input.len() < 8 {
                        input.push('2');
                    }
                },
                | Key::Key3 | Key::NumPad3 => {
                    if input.len() < 8 {
                        input.push('3');
                    }
                },
                | Key::Key4 | Key::NumPad4 => {
                    if input.len() < 8 {
                        input.push('4');
                    }
                },
                | Key::Key5 | Key::NumPad5 => {
                    if input.len() < 8 {
                        input.push('5');
                    }
                },
                | Key::Key6 | Key::NumPad6 => {
                    if input.len() < 8 {
                        input.push('6');
                    }
                },
                | Key::Key7 | Key::NumPad7 => {
                    if input.len() < 8 {
                        input.push('7');
                    }
                },
                | Key::Key8 | Key::NumPad8 => {
                    if input.len() < 8 {
                        input.push('8');
                    }
                },
                | Key::Key9 | Key::NumPad9 => {
                    if input.len() < 8 {
                        input.push('9');
                    }
                },
                | Key::Backspace => {
                    input.pop();
                },
                | Key::Enter => {
                    if !input.is_empty() {
                        confirmed = true;
                        break;
                    }
                },
                | _ => {},
            }
        }

        if confirmed {
            break;
        }

        // Render dialog card
        buffer.fill(0xFF1E_1E2E); // Dark background

        // Title & subtitle
        crate::hud::draw_text(
            &mut buffer,
            width,
            height,
            24,
            24,
            "YUBIKEY PIV PIN REQUIRED",
            0xFFCA_A6F7,
        );
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
