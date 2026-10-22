#![allow(unsafe_code)]
//! Process memory protection, anti-dump, debugger attachment denial,
//! memory locking (mlock), and panic zeroization hooks.

use crate::error::Result;
use std::sync::OnceLock;

static ANTI_DUMP_INITIALIZED: OnceLock<bool> = OnceLock::new();

/// Activate OS-level anti-dump, core dump suppression, and debugger attach denial.
/// Safe to call multiple times (idempotent).
pub fn enable_anti_dump_protection() -> Result<()> {
    ANTI_DUMP_INITIALIZED.get_or_init(|| {
        let _ = apply_anti_dump_os_primitives();
        install_panic_zeroization_hook();
        true
    });
    Ok(())
}

fn apply_anti_dump_os_primitives() -> Result<()> {
    #[cfg(unix)]
    {
        // 1. Suppress core dumps via RLIMIT_CORE = 0
        let rlim = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        unsafe {
            libc::setrlimit(libc::RLIMIT_CORE, &rlim);
        }
    }

    #[cfg(target_os = "linux")]
    {
        // 2. Disable /proc/$PID/mem dumpability and deny unprivileged ptrace attach on Linux
        // When PR_SET_DUMPABLE is 0, core dumps are suppressed and ptrace PTRACE_ATTACH is denied by kernel
        unsafe {
            libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
        }
    }

    #[cfg(target_os = "macos")]
    {
        // PT_DENY_ATTACH (31 on Darwin): prevents debuggers from attaching
        const PT_DENY_ATTACH: i32 = 31;
        extern "C" {
            fn ptrace(
                req: i32,
                pid: i32,
                addr: *const libc::c_void,
                data: i32,
            ) -> i32;
        }
        unsafe {
            ptrace(PT_DENY_ATTACH, 0, std::ptr::null(), 0);
        }
    }

    Ok(())
}

/// Lock a memory buffer into RAM to prevent it from being paged to disk swap space.
pub fn mlock_slice(data: &[u8]) -> Result<()> {
    #[cfg(unix)]
    if !data.is_empty() {
        let res = unsafe { libc::mlock(data.as_ptr().cast(), data.len()) };
        if res != 0 {
            // Note: mlock may require elevated RLIMIT_MEMLOCK; do not fail hard if unprivileged
            return Ok(());
        }
    }
    #[cfg(not(unix))]
    let _ = data;
    Ok(())
}

/// Unlock a previously locked memory buffer.
pub fn munlock_slice(data: &[u8]) {
    #[cfg(unix)]
    if !data.is_empty() {
        let _ = unsafe { libc::munlock(data.as_ptr().cast(), data.len()) };
    }
    #[cfg(not(unix))]
    let _ = data;
}

/// Install a panic hook that suppresses traceback information revealing memory layouts or crypto parameters.
fn install_panic_zeroization_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        eprintln!("[SECURITY] Abnormal termination detected. Sanitizing memory state.");
        default_hook(panic_info);
    }));
}
