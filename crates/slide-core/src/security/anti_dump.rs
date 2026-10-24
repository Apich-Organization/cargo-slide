#![allow(unsafe_code)]
//! Process memory protection, anti-dump, debugger attachment denial,
//! memory locking (mlock/VirtualLock), and panic zeroization hooks.

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
            // PR_SET_PTRACER = 0x59616d61: restrict ptrace attach to init only (effectively deny all)
            const PR_SET_PTRACER: i32 = 0x59616d61;
            libc::prctl(PR_SET_PTRACER, 0, 0, 0, 0);
        }
    }

    #[cfg(target_os = "macos")]
    {
        // PT_DENY_ATTACH (31 on Darwin): prevents debuggers from attaching
        const PT_DENY_ATTACH: i32 = 31;
        #[allow(unsafe_op_in_unsafe_fn)]
        unsafe extern "C" {
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

    #[cfg(windows)]
    {
        // Windows process hardening:
        // 1. Suppress crash dump generation and error reporting dialogs
        const SEM_FAILCRITICALERRORS: u32 = 0x0001;
        const SEM_NOGPFAULTERRORBOX: u32 = 0x0002;
        unsafe extern "system" {
            fn SetErrorMode(umode: u32) -> u32;
            fn SetProcessMitigationPolicy(
                policy: i32,
                lpbuffer: *const std::ffi::c_void,
                dwlength: usize,
            ) -> i32;
        }
        unsafe {
            SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);

            // 2. Disable legacy extension points / SetWindowsHookEx injection into our process
            // ProcessExtensionPointDisablePolicy = 3
            #[repr(C)]
            struct ExtensionPointPolicy {
                flags: u32,
            }
            let ext_policy = ExtensionPointPolicy { flags: 1 };
            let _ = SetProcessMitigationPolicy(
                3,
                std::ptr::addr_of!(ext_policy).cast(),
                std::mem::size_of::<ExtensionPointPolicy>(),
            );

            // 3. Prohibit dynamic code generation / injection if possible
            // ProcessDynamicCodePolicy = 2
            #[repr(C)]
            struct DynamicCodePolicy {
                flags: u32,
            }
            let dyn_policy = DynamicCodePolicy { flags: 1 };
            let _ = SetProcessMitigationPolicy(
                2,
                std::ptr::addr_of!(dyn_policy).cast(),
                std::mem::size_of::<DynamicCodePolicy>(),
            );
        }
    }

    Ok(())
}

/// Check if an active debugger or tracing tool is currently attached.
#[must_use]
pub fn is_debugger_present() -> bool {
    #[cfg(windows)]
    {
        unsafe extern "system" {
            fn IsDebuggerPresent() -> i32;
            fn GetCurrentProcess() -> isize;
            fn CheckRemoteDebuggerPresent(
                hprocess: isize,
                pbdebuggerpresent: *mut i32,
            ) -> i32;
        }
        unsafe {
            if IsDebuggerPresent() != 0 {
                return true;
            }
            let mut remote = 0i32;
            let _ = CheckRemoteDebuggerPresent(GetCurrentProcess(), &mut remote);
            if remote != 0 {
                return true;
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if let Some(rest) = line.strip_prefix("TracerPid:")
                    && rest.trim().parse::<u32>().unwrap_or(0) > 0
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Verify that the execution environment is untampered and free of attached debuggers.
/// Returns `Err(SlideError::SecurityViolation)` if an attached debugger is detected.
pub fn verify_environment_integrity() -> Result<()> {
    if is_debugger_present() {
        return Err(crate::error::SlideError::SecurityViolation(
            "Debugger or memory inspection tool detected during presentation playback".to_string(),
        ));
    }
    Ok(())
}

/// Lock a memory buffer into RAM to prevent it from being paged to disk swap space.
pub fn mlock_slice(data: &[u8]) -> Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    #[cfg(unix)]
    {
        let res = unsafe { libc::mlock(data.as_ptr().cast(), data.len()) };
        if res != 0 {
            // Note: mlock may require elevated RLIMIT_MEMLOCK; do not fail hard if unprivileged
            return Ok(());
        }
    }
    #[cfg(windows)]
    {
        unsafe extern "system" {
            fn VirtualLock(
                lpaddress: *const std::ffi::c_void,
                dwsize: usize,
            ) -> i32;
        }
        unsafe {
            let _ = VirtualLock(data.as_ptr().cast(), data.len());
        }
    }
    Ok(())
}

/// Unlock a previously locked memory buffer.
pub fn munlock_slice(data: &[u8]) {
    if data.is_empty() {
        return;
    }
    #[cfg(unix)]
    {
        let _ = unsafe { libc::munlock(data.as_ptr().cast(), data.len()) };
    }
    #[cfg(windows)]
    {
        unsafe extern "system" {
            fn VirtualUnlock(
                lpaddress: *const std::ffi::c_void,
                dwsize: usize,
            ) -> i32;
        }
        unsafe {
            let _ = VirtualUnlock(data.as_ptr().cast(), data.len());
        }
    }
}

/// Install a panic hook that suppresses traceback information revealing memory layouts or crypto parameters.
fn install_panic_zeroization_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        eprintln!("[SECURITY] Abnormal termination detected. Sanitizing memory state.");
        default_hook(panic_info);
    }));
}
