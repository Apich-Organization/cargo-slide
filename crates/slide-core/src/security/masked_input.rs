#![allow(unsafe_code)]
//! Secure masked password and PIN terminal input with automatic echo suppression and memory zeroization.

use crate::error::Result;
use crate::error::SlideError;
use std::io::IsTerminal;
use std::io::Write;
use zeroize::Zeroize;
use zeroize::Zeroizing;

/// Prompt the user for a sensitive PIN or passphrase in the terminal without echoing characters to screen.
pub fn read_masked_secret(prompt: &str) -> Result<Zeroizing<String>> {
    eprint!("{prompt}");
    let _ = std::io::stderr().flush();

    let is_term = std::io::stdin().is_terminal();
    if !is_term {
        // Non-terminal input (piped / scripted)
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .map_err(|e| SlideError::Piv(format!("Failed to read input from stdin: {e}")))?;
        let trimmed = line.trim().to_string();
        line.zeroize();
        if trimmed.is_empty() {
            return Err(SlideError::Piv("Input cannot be empty".to_string()));
        }
        return Ok(Zeroizing::new(trimmed));
    }

    #[cfg(unix)]
    {
        struct TermiosGuard {
            fd: i32,
            orig: libc::termios,
        }

        impl Drop for TermiosGuard {
            fn drop(&mut self) {
                unsafe {
                    libc::tcsetattr(self.fd, libc::TCSANOW, &self.orig);
                }
            }
        }

        let stdin_fd = 0; // STDIN_FILENO
        let mut orig_termios = std::mem::MaybeUninit::<libc::termios>::uninit();
        let ret = unsafe { libc::tcgetattr(stdin_fd, orig_termios.as_mut_ptr()) };
        if ret != 0 {
            return fallback_read_line();
        }

        let orig = unsafe { orig_termios.assume_init() };
        let mut raw = orig;
        raw.c_lflag &= !libc::ECHO;
        raw.c_lflag |= libc::ECHONL; // Ensure newline on Enter

        let _guard = TermiosGuard { fd: stdin_fd, orig };
        let set_ret = unsafe { libc::tcsetattr(stdin_fd, libc::TCSANOW, &raw) };
        if set_ret != 0 {
            return fallback_read_line();
        }

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| SlideError::Piv(format!("Failed to read secret from terminal: {e}")))?;

        eprintln!(); // Blank line for clean terminal transition

        let trimmed = input.trim().to_string();
        input.zeroize();
        if trimmed.is_empty() {
            return Err(SlideError::Piv("Secret cannot be empty".to_string()));
        }
        Ok(Zeroizing::new(trimmed))
    }

    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        struct ConsoleModeGuard {
            handle: *mut std::ffi::c_void,
            orig_mode: u32,
        }
        impl Drop for ConsoleModeGuard {
            fn drop(&mut self) {
                unsafe extern "system" {
                    fn SetConsoleMode(
                        hConsoleHandle: *mut std::ffi::c_void,
                        dwMode: u32,
                    ) -> i32;
                }
                unsafe {
                    SetConsoleMode(self.handle, self.orig_mode);
                }
            }
        }

        unsafe extern "system" {
            fn GetConsoleMode(
                hConsoleHandle: *mut std::ffi::c_void,
                lpMode: *mut u32,
            ) -> i32;
            fn SetConsoleMode(
                hConsoleHandle: *mut std::ffi::c_void,
                dwMode: u32,
            ) -> i32;
        }

        let handle = std::io::stdin().as_raw_handle();
        let mut orig_mode: u32 = 0;
        let ok = unsafe { GetConsoleMode(handle, &mut orig_mode) };
        if ok == 0 {
            return fallback_read_line();
        }

        const ENABLE_ECHO_INPUT: u32 = 0x0004;
        let new_mode = orig_mode & !ENABLE_ECHO_INPUT;
        let _guard = ConsoleModeGuard { handle, orig_mode };
        let set_ok = unsafe { SetConsoleMode(handle, new_mode) };
        if set_ok == 0 {
            return fallback_read_line();
        }

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| SlideError::Piv(format!("Failed to read secret from terminal: {e}")))?;

        eprintln!();

        let trimmed = input.trim().to_string();
        input.zeroize();
        if trimmed.is_empty() {
            return Err(SlideError::Piv("Secret cannot be empty".to_string()));
        }
        Ok(Zeroizing::new(trimmed))
    }

    #[cfg(not(any(unix, windows)))]
    {
        fallback_read_line()
    }
}

fn fallback_read_line() -> Result<Zeroizing<String>> {
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| SlideError::Piv(format!("Failed to read input: {e}")))?;
    let trimmed = input.trim().to_string();
    input.zeroize();
    if trimmed.is_empty() {
        return Err(SlideError::Piv("Secret cannot be empty".to_string()));
    }
    Ok(Zeroizing::new(trimmed))
}
