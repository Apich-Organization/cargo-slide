#![allow(unsafe_code, non_camel_case_types, non_snake_case, dead_code)]
//! Dynamic, zero-dependency PC/SC (WinSCard) smartcard subsystem interface.
//!
//! Communicates with real YubiKey hardware across Linux, macOS, and Windows
//! by dynamically loading the OS native smart card library at runtime.
//! This ensures 100% self-containment without hard development header dependencies.

use crate::error::Result;
use crate::error::SlideError;
use std::ffi::CStr;
use std::ffi::CString;
use std::ffi::c_char;
use std::ffi::c_void;
use std::sync::Arc;

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "ios"))]
pub type DWORD = u32;
#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "ios")))]
pub type DWORD = std::ffi::c_ulong;

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "ios"))]
pub type LONG = i32;
#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "ios")))]
pub type LONG = std::ffi::c_long;

pub type SCARDCONTEXT = LONG;
pub type SCARDHANDLE = LONG;

pub const SCARD_SCOPE_USER: DWORD = 0;
pub const SCARD_SCOPE_SYSTEM: DWORD = 2;

pub const SCARD_SHARE_SHARED: DWORD = 2;
pub const SCARD_SHARE_EXCLUSIVE: DWORD = 1;
pub const SCARD_SHARE_DIRECT: DWORD = 3;

pub const SCARD_PROTOCOL_T0: DWORD = 1;
pub const SCARD_PROTOCOL_T1: DWORD = 2;
pub const SCARD_PROTOCOL_ANY: DWORD = SCARD_PROTOCOL_T0 | SCARD_PROTOCOL_T1;

pub const SCARD_LEAVE_CARD: DWORD = 0;
pub const SCARD_RESET_CARD: DWORD = 1;
pub const SCARD_UNPOWER_CARD: DWORD = 2;
pub const SCARD_EJECT_CARD: DWORD = 3;

pub const SCARD_S_SUCCESS: LONG = 0;
pub const SCARD_E_NO_READERS_AVAILABLE: u32 = 0x8010002E;
pub const SCARD_E_NO_SMARTCARD: u32 = 0x8010000C;
pub const SCARD_E_SHARING_VIOLATION: u32 = 0x8010000B;

type SCardEstablishContextFn = unsafe extern "C" fn(
    dwScope: DWORD,
    pvReserved1: *const c_void,
    pvReserved2: *const c_void,
    phContext: *mut SCARDCONTEXT,
) -> LONG;

type SCardReleaseContextFn = unsafe extern "C" fn(hContext: SCARDCONTEXT) -> LONG;

type SCardListReadersFn = unsafe extern "C" fn(
    hContext: SCARDCONTEXT,
    mszGroups: *const c_char,
    mszReaders: *mut c_char,
    pcchReaders: *mut DWORD,
) -> LONG;

type SCardConnectFn = unsafe extern "C" fn(
    hContext: SCARDCONTEXT,
    szReader: *const c_char,
    dwShareMode: DWORD,
    dwPreferredProtocols: DWORD,
    phCard: *mut SCARDHANDLE,
    pdwActiveProtocol: *mut DWORD,
) -> LONG;

type SCardDisconnectFn = unsafe extern "C" fn(hCard: SCARDHANDLE, dwDisposition: DWORD) -> LONG;

type SCardBeginTransactionFn = unsafe extern "C" fn(hCard: SCARDHANDLE) -> LONG;
type SCardEndTransactionFn = unsafe extern "C" fn(hCard: SCARDHANDLE, dwDisposition: DWORD) -> LONG;

type SCardTransmitFn = unsafe extern "C" fn(
    hCard: SCARDHANDLE,
    pioSendPci: *const c_void,
    pbSendBuffer: *const u8,
    cbSendLength: DWORD,
    pioRecvPci: *mut c_void,
    pbRecvBuffer: *mut u8,
    pcbRecvLength: *mut DWORD,
) -> LONG;

/// Dynamically loaded PC/SC shared library symbols.
pub struct PcscLibrary {
    _handle: *mut c_void,
    fn_establish_context: SCardEstablishContextFn,
    fn_release_context: SCardReleaseContextFn,
    fn_list_readers: SCardListReadersFn,
    fn_connect: SCardConnectFn,
    fn_disconnect: SCardDisconnectFn,
    fn_begin_transaction: SCardBeginTransactionFn,
    fn_end_transaction: SCardEndTransactionFn,
    fn_transmit: SCardTransmitFn,
    pci_t0: *const c_void,
    pci_t1: *const c_void,
    fn_stringify_error: Option<unsafe extern "C" fn(rv: LONG) -> *const c_char>,
}

unsafe impl Send for PcscLibrary {}
unsafe impl Sync for PcscLibrary {}

#[repr(C)]
struct ScardIoRequest {
    protocol: DWORD,
    pci_length: DWORD,
}

static FALLBACK_PCI_T0: ScardIoRequest = ScardIoRequest {
    protocol: SCARD_PROTOCOL_T0,
    pci_length: std::mem::size_of::<ScardIoRequest>() as DWORD,
};

static FALLBACK_PCI_T1: ScardIoRequest = ScardIoRequest {
    protocol: SCARD_PROTOCOL_T1,
    pci_length: std::mem::size_of::<ScardIoRequest>() as DWORD,
};

static PCSC_LIB: std::sync::OnceLock<std::result::Result<Arc<PcscLibrary>, String>> =
    std::sync::OnceLock::new();

impl PcscLibrary {
    /// Attempt to dynamically load the system PC/SC library (cached for process lifetime).
    pub fn load() -> Result<Arc<Self>> {
        PCSC_LIB
            .get_or_init(Self::load_internal)
            .as_ref()
            .map(Arc::clone)
            .map_err(|e| SlideError::Piv(e.clone()))
    }

    #[allow(clippy::missing_transmute_annotations)]
    fn load_internal() -> std::result::Result<Arc<Self>, String> {
        #[cfg(target_os = "windows")]
        {
            unsafe extern "system" {
                fn LoadLibraryA(lpLibFileName: *const c_char) -> *mut c_void;
                fn GetProcAddress(
                    hModule: *mut c_void,
                    lpProcName: *const c_char,
                ) -> *mut c_void;
            }

            let name = CString::new("winscard.dll").map_err(|e| e.to_string())?;
            let handle = unsafe { LoadLibraryA(name.as_ptr()) };
            if handle.is_null() {
                return Err("Failed to load winscard.dll".to_string());
            }

            macro_rules! get_sym {
                ($n:ident, $str:expr) => {{
                    let s = CString::new($str).map_err(|e| e.to_string())?;
                    let ptr = unsafe { GetProcAddress(handle, s.as_ptr()) };
                    if ptr.is_null() {
                        return Err(format!(
                            "Symbol {} not found in winscard.dll",
                            $str
                        ));
                    }
                    unsafe { std::mem::transmute(ptr) }
                }};
            }

            let fn_establish_context = get_sym!(SCardEstablishContext, "SCardEstablishContext");
            let fn_release_context = get_sym!(SCardReleaseContext, "SCardReleaseContext");
            let fn_list_readers = get_sym!(SCardListReadersA, "SCardListReadersA");
            let fn_connect = get_sym!(SCardConnectA, "SCardConnectA");
            let fn_disconnect = get_sym!(SCardDisconnect, "SCardDisconnect");
            let fn_begin_transaction = get_sym!(SCardBeginTransaction, "SCardBeginTransaction");
            let fn_end_transaction = get_sym!(SCardEndTransaction, "SCardEndTransaction");
            let fn_transmit = get_sym!(SCardTransmit, "SCardTransmit");

            let pci_t0_sym = CString::new("g_rgSCardT0Pci").map_err(|e| e.to_string())?;
            let pci_t0 = unsafe { GetProcAddress(handle, pci_t0_sym.as_ptr()) };
            let pci_t1_sym = CString::new("g_rgSCardT1Pci").map_err(|e| e.to_string())?;
            let pci_t1 = unsafe { GetProcAddress(handle, pci_t1_sym.as_ptr()) };

            Ok(Arc::new(Self {
                _handle: handle,
                fn_establish_context,
                fn_release_context,
                fn_list_readers,
                fn_connect,
                fn_disconnect,
                fn_begin_transaction,
                fn_end_transaction,
                fn_transmit,
                pci_t0,
                pci_t1,
                fn_stringify_error: None,
            }))
        }

        #[cfg(not(target_os = "windows"))]
        {
            let candidate_paths = if cfg!(target_os = "macos") {
                vec![
                    "/System/Library/Frameworks/PCSC.framework/PCSC",
                    "/Library/Frameworks/PCSC.framework/PCSC",
                    "PCSC",
                ]
            } else {
                vec![
                    "/lib64/libpcsclite.so.1",
                    "/usr/lib64/libpcsclite.so.1",
                    "/lib/x86_64-linux-gnu/libpcsclite.so.1",
                    "/usr/lib/x86_64-linux-gnu/libpcsclite.so.1",
                    "/usr/lib/libpcsclite.so.1",
                    "/lib/libpcsclite.so.1",
                    "/lib64/libpcsclite_real.so.1",
                    "libpcsclite.so.1",
                    "libpcsclite.so",
                ]
            };

            let mut handle = std::ptr::null_mut();
            for path in &candidate_paths {
                if let Ok(c_path) = CString::new(*path) {
                    let h = unsafe { libc::dlopen(c_path.as_ptr(), libc::RTLD_NOW) };
                    if !h.is_null() {
                        handle = h;
                        break;
                    }
                }
            }

            if handle.is_null() {
                return Err(
                    "PC/SC smart card library (libpcsclite) not found on system. Please ensure pcscd / pcsc-lite is installed.".to_string(),
                );
            }

            macro_rules! get_sym {
                ($str:expr) => {{
                    let s = CString::new($str).map_err(|e| e.to_string())?;
                    let ptr = unsafe { libc::dlsym(handle, s.as_ptr()) };
                    if ptr.is_null() {
                        return Err(format!(
                            "Symbol {} not found in PC/SC library",
                            $str
                        ));
                    }
                    unsafe { std::mem::transmute(ptr) }
                }};
            }

            let fn_establish_context = get_sym!("SCardEstablishContext");
            let fn_release_context = get_sym!("SCardReleaseContext");
            let fn_list_readers = get_sym!("SCardListReaders");
            let fn_connect = get_sym!("SCardConnect");
            let fn_disconnect = get_sym!("SCardDisconnect");
            let fn_begin_transaction = get_sym!("SCardBeginTransaction");
            let fn_end_transaction = get_sym!("SCardEndTransaction");
            let fn_transmit = get_sym!("SCardTransmit");

            let pci_t0_sym = CString::new("g_rgSCardT0Pci").map_err(|e| e.to_string())?;
            let pci_t0 = unsafe { libc::dlsym(handle, pci_t0_sym.as_ptr()) };
            let pci_t1_sym = CString::new("g_rgSCardT1Pci").map_err(|e| e.to_string())?;
            let pci_t1 = unsafe { libc::dlsym(handle, pci_t1_sym.as_ptr()) };

            let fn_stringify_error = {
                let sym = CString::new("pcsc_stringify_error").ok();
                sym.and_then(|s| {
                    let ptr = unsafe { libc::dlsym(handle, s.as_ptr()) };
                    if ptr.is_null() {
                        None
                    } else {
                        Some(unsafe { std::mem::transmute(ptr) })
                    }
                })
            };

            Ok(Arc::new(Self {
                _handle: handle,
                fn_establish_context,
                fn_release_context,
                fn_list_readers,
                fn_connect,
                fn_disconnect,
                fn_begin_transaction,
                fn_end_transaction,
                fn_transmit,
                pci_t0,
                pci_t1,
                fn_stringify_error,
            }))
        }
    }

    /// Provide a human-readable explanation for a PC/SC error code.
    pub fn stringify_error(
        &self,
        code: LONG,
    ) -> String {
        if let Some(f) = self.fn_stringify_error {
            let ptr = unsafe { f(code) };
            if !ptr.is_null()
                && let Ok(s) = unsafe { CStr::from_ptr(ptr) }.to_str()
            {
                return format!(": {s}");
            }
        }
        match code as u32 {
            | 0x80100001 => ": Internal error".to_string(),
            | 0x80100002 => ": Cancelled by user".to_string(),
            | 0x80100003 => ": Invalid smartcard handle".to_string(),
            | 0x80100004 => ": Invalid parameter".to_string(),
            | 0x80100008 => ": Insufficient buffer".to_string(),
            | 0x80100009 => ": Unknown smartcard reader".to_string(),
            | 0x8010000A => ": Communication timeout".to_string(),
            | 0x8010000B => ": Sharing violation".to_string(),
            | 0x8010000C => ": No smartcard inserted".to_string(),
            | 0x8010000E => ": Smartcard unsupported".to_string(),
            | 0x8010000F => ": Card unresponsive or unpowered".to_string(),
            | 0x80100010 => ": Reset required".to_string(),
            | 0x80100011 => ": Card removed".to_string(),
            | 0x80100016 => ": Reader unavailable".to_string(),
            | 0x8010001D => ": Context invalid or released".to_string(),
            | 0x8010002E => ": No smartcard readers available".to_string(),
            | _ => String::new(),
        }
    }
}

/// Format an ISO/IEC 7816-4 status word with standard description.
pub fn format_iso7816_status_word(
    sw1: u8,
    sw2: u8,
) -> String {
    let code = format!("SW=0x{sw1:02X}{sw2:02X}");
    let desc = match (sw1, sw2) {
        | (0x90, 0x00) => "Success",
        | (0x63, s2) if (s2 & 0xF0) == 0xC0 => {
            return format!(
                "{code} (Verification failed, {} retry attempts remaining)",
                s2 & 0x0F
            );
        },
        | (0x69, 0x82) => "Security status not satisfied (PIN verification required)",
        | (0x69, 0x83) => "Authentication method blocked (PIN blocked, PUK required)",
        | (0x69, 0x85) => "Condition of use not satisfied",
        | (0x6A, 0x80) => "Incorrect data field parameters",
        | (0x6A, 0x81) => "Function not supported",
        | (0x6A, 0x82) => "File or application not found",
        | (0x6A, 0x86) => "Incorrect parameters P1-P2",
        | (0x6A, 0x88) => "Referenced data not found",
        | (0x67, 0x00) => "Wrong length (extended APDU unsupported)",
        | (0x6D, 0x00) => "Instruction code not supported",
        | (0x6E, 0x00) => "Class not supported",
        | _ => "Operation rejected by smartcard",
    };
    format!("{code} ({desc})")
}

impl Drop for PcscLibrary {
    fn drop(&mut self) {
        // Intentionally keep the system smartcard library open for the remainder of process execution.
        // Operating system smartcard subsystems (libpcsclite on Linux, winscard.dll on Windows)
        // maintain process-global thread state, mutexes, and active daemon sockets to pcscd.
        // Calling dlclose() triggers heap corruption and SIGSEGV inside list_seek upon subsequent operations.
    }
}

/// Established PC/SC Resource Manager Context.
pub struct PcscContext {
    pub lib: Arc<PcscLibrary>,
    pub h_context: SCARDCONTEXT,
}

impl PcscContext {
    /// Establish a new application context with PC/SC.
    pub fn establish() -> Result<Arc<Self>> {
        let lib = PcscLibrary::load()?;
        let mut h_context: SCARDCONTEXT = 0;
        let mut res = unsafe {
            (lib.fn_establish_context)(
                SCARD_SCOPE_USER,
                std::ptr::null(),
                std::ptr::null(),
                &mut h_context,
            )
        };
        if res != SCARD_S_SUCCESS {
            // Fallback to SCARD_SCOPE_SYSTEM (often required on macOS or headless daemons)
            let fallback_res = unsafe {
                (lib.fn_establish_context)(
                    SCARD_SCOPE_SYSTEM,
                    std::ptr::null(),
                    std::ptr::null(),
                    &mut h_context,
                )
            };
            if fallback_res == SCARD_S_SUCCESS {
                res = SCARD_S_SUCCESS;
            }
        }
        if res != SCARD_S_SUCCESS {
            let detail = lib.stringify_error(res);
            let macos_hint = if cfg!(target_os = "macos") {
                " On macOS, smartcard access may require SmartCardServices / CryptoTokenKit permissions or fallback to software passphrase encryption (--auth passphrase)."
            } else {
                ""
            };
            return Err(SlideError::Piv(format!(
                "Failed to establish PC/SC context (SCardEstablishContext error code: 0x{res:08X}{detail}).{macos_hint}"
            )));
        }

        Ok(Arc::new(Self { lib, h_context }))
    }

    /// List all currently connected smart card readers.
    pub fn list_readers(&self) -> Result<Vec<String>> {
        let mut len: DWORD = 0;
        let res = unsafe {
            (self.lib.fn_list_readers)(
                self.h_context,
                std::ptr::null(),
                std::ptr::null_mut(),
                &mut len,
            )
        };

        if (res as u32) == SCARD_E_NO_READERS_AVAILABLE || (res as u32) == 0x80100016 || len <= 1 {
            return Ok(Vec::new());
        }
        if res != SCARD_S_SUCCESS {
            let detail = self.lib.stringify_error(res);
            return Err(SlideError::Piv(format!(
                "SCardListReaders failed with error 0x{res:08X}{detail}"
            )));
        }

        let mut buf = vec![0u8; len as usize];
        let res2 = unsafe {
            (self.lib.fn_list_readers)(
                self.h_context,
                std::ptr::null(),
                buf.as_mut_ptr().cast::<c_char>(),
                &mut len,
            )
        };
        if res2 != SCARD_S_SUCCESS {
            let detail = self.lib.stringify_error(res2);
            return Err(SlideError::Piv(format!(
                "SCardListReaders buffer retrieval failed: 0x{res2:08X}{detail}"
            )));
        }

        let mut readers = Vec::new();
        let mut cur = &buf[..];
        while !cur.is_empty() {
            if let Ok(c_str) = CStr::from_bytes_until_nul(cur) {
                if let Ok(s) = c_str.to_str()
                    && !s.is_empty()
                {
                    readers.push(s.to_string());
                }
                cur = &cur[c_str.to_bytes_with_nul().len()..];
            } else {
                break;
            }
        }

        Ok(readers)
    }

    /// Connect to a specific smart card reader, binding the card handle to this active context.
    pub fn connect(
        self: &Arc<Self>,
        reader: &str,
    ) -> Result<PcscCard> {
        let c_reader = CString::new(reader).map_err(|e| SlideError::Piv(e.to_string()))?;
        let mut h_card: SCARDHANDLE = 0;
        let mut active_proto: DWORD = 0;

        let res = unsafe {
            (self.lib.fn_connect)(
                self.h_context,
                c_reader.as_ptr(),
                SCARD_SHARE_SHARED,
                SCARD_PROTOCOL_ANY,
                &mut h_card,
                &mut active_proto,
            )
        };

        if res != SCARD_S_SUCCESS {
            let detail = self.lib.stringify_error(res);
            return Err(SlideError::Piv(format!(
                "Failed to connect to smart card reader '{reader}' (error code: 0x{res:08X}{detail}). Is the YubiKey firmly inserted?"
            )));
        }

        Ok(PcscCard {
            context: Arc::clone(self),
            h_card,
            active_protocol: active_proto as u32,
            reader_name: reader.to_string(),
        })
    }
}

impl Drop for PcscContext {
    fn drop(&mut self) {
        if self.h_context != 0 {
            let _ = unsafe { (self.lib.fn_release_context)(self.h_context) };
            self.h_context = 0;
        }
    }
}

/// Active connection to a physical smart card.
pub struct PcscCard {
    context: Arc<PcscContext>,
    pub h_card: SCARDHANDLE,
    pub active_protocol: u32,
    pub reader_name: String,
}

impl PcscCard {
    /// Transmit raw APDU command to the card and return response payload (excluding 90 00).
    pub fn transmit(
        &self,
        apdu: &[u8],
    ) -> Result<Vec<u8>> {
        let pci = if self.active_protocol == (SCARD_PROTOCOL_T0 as u32) {
            if !self.context.lib.pci_t0.is_null() {
                self.context.lib.pci_t0
            } else {
                std::ptr::addr_of!(FALLBACK_PCI_T0).cast::<c_void>()
            }
        } else {
            if !self.context.lib.pci_t1.is_null() {
                self.context.lib.pci_t1
            } else {
                std::ptr::addr_of!(FALLBACK_PCI_T1).cast::<c_void>()
            }
        };

        let mut recv_buf = [0u8; 8192];
        let mut recv_len: DWORD = recv_buf.len() as DWORD;

        let res = unsafe {
            (self.context.lib.fn_transmit)(
                self.h_card,
                pci,
                apdu.as_ptr(),
                apdu.len() as DWORD,
                std::ptr::null_mut(),
                recv_buf.as_mut_ptr(),
                &mut recv_len,
            )
        };

        if res != SCARD_S_SUCCESS {
            let detail = self.context.lib.stringify_error(res);
            return Err(SlideError::Piv(format!(
                "SCardTransmit APDU failed (error code 0x{res:08X}{detail})"
            )));
        }

        if recv_len < 2 {
            return Err(SlideError::Piv(
                "APDU response truncated: missing status word".to_string(),
            ));
        }

        let mut result = Vec::with_capacity(recv_len as usize);
        let mut current_sw1 = recv_buf[(recv_len - 2) as usize];
        let mut current_sw2 = recv_buf[(recv_len - 1) as usize];
        result.extend_from_slice(&recv_buf[..(recv_len - 2) as usize]);

        // Handle ISO 7816-4 status word 0x6C (wrong Le: smartcard requests specific Le)
        if current_sw1 == 0x6C {
            let mut retry_apdu = apdu.to_vec();
            if let Some(last) = retry_apdu.last_mut() {
                *last = current_sw2;
            } else {
                retry_apdu.push(current_sw2);
            }
            recv_len = recv_buf.len() as DWORD;
            let retry_res = unsafe {
                (self.context.lib.fn_transmit)(
                    self.h_card,
                    pci,
                    retry_apdu.as_ptr(),
                    retry_apdu.len() as DWORD,
                    std::ptr::null_mut(),
                    recv_buf.as_mut_ptr(),
                    &mut recv_len,
                )
            };
            if retry_res == SCARD_S_SUCCESS && recv_len >= 2 {
                result.clear();
                current_sw1 = recv_buf[(recv_len - 2) as usize];
                current_sw2 = recv_buf[(recv_len - 1) as usize];
                result.extend_from_slice(&recv_buf[..(recv_len - 2) as usize]);
            }
        }

        // Handle ISO 7816-4 chaining / GET RESPONSE (0x61 status word)
        while current_sw1 == 0x61 {
            let get_response_apdu = [0x00, 0xC0, 0x00, 0x00, current_sw2];
            recv_len = recv_buf.len() as DWORD;
            let chunk_res = unsafe {
                (self.context.lib.fn_transmit)(
                    self.h_card,
                    pci,
                    get_response_apdu.as_ptr(),
                    get_response_apdu.len() as DWORD,
                    std::ptr::null_mut(),
                    recv_buf.as_mut_ptr(),
                    &mut recv_len,
                )
            };
            if chunk_res != SCARD_S_SUCCESS || recv_len < 2 {
                break;
            }
            current_sw1 = recv_buf[(recv_len - 2) as usize];
            current_sw2 = recv_buf[(recv_len - 1) as usize];
            result.extend_from_slice(&recv_buf[..(recv_len - 2) as usize]);
        }

        if current_sw1 == 0x90 && current_sw2 == 0x00 {
            Ok(result)
        } else {
            let desc = format_iso7816_status_word(current_sw1, current_sw2);
            Err(SlideError::Piv(format!("PIV Card APDU error: {desc}")))
        }
    }

    /// Transmit APDU command, automatically formatting Case 3S or Case 3E (extended APDU)
    /// based on payload length according to NIST SP 800-73-4 and YubiKey CCID.
    /// If extended APDU is unsupported by card/reader, automatically falls back to ISO 7816-4 command chaining.
    pub fn send_apdu(
        &self,
        cla: u8,
        ins: u8,
        p1: u8,
        p2: u8,
        data: &[u8],
    ) -> Result<Vec<u8>> {
        if data.len() <= 255 {
            let mut apdu = Vec::with_capacity(data.len() + 6);
            apdu.push(cla);
            apdu.push(ins);
            apdu.push(p1);
            apdu.push(p2);
            if data.is_empty() {
                apdu.push(0x00);
            } else {
                apdu.push(data.len() as u8);
                apdu.extend_from_slice(data);
            }
            return self.transmit(&apdu);
        }

        // Try Extended APDU (Case 3E) first
        let mut ext_apdu = Vec::with_capacity(data.len() + 8);
        ext_apdu.push(cla);
        ext_apdu.push(ins);
        ext_apdu.push(p1);
        ext_apdu.push(p2);
        ext_apdu.push(0x00);
        ext_apdu.push((data.len() >> 8) as u8);
        ext_apdu.push((data.len() & 0xFF) as u8);
        ext_apdu.extend_from_slice(data);

        match self.transmit(&ext_apdu) {
            | Ok(res) => Ok(res),
            | Err(e) => {
                let err_s = e.to_string();
                if err_s.contains("0x6700")
                    || err_s.contains("0x6A86")
                    || err_s.contains("0x6A81")
                    || err_s.contains("0x6D00")
                {
                    self.send_apdu_chained(cla, ins, p1, p2, data)
                } else {
                    Err(e)
                }
            },
        }
    }

    /// Transmit large APDU payload using ISO/IEC 7816-4 short command chaining (CLA bit 5 = 1).
    pub fn send_apdu_chained(
        &self,
        cla: u8,
        ins: u8,
        p1: u8,
        p2: u8,
        data: &[u8],
    ) -> Result<Vec<u8>> {
        const CHUNK_SIZE: usize = 255;
        let chunks: Vec<&[u8]> = data.chunks(CHUNK_SIZE).collect();
        let total = chunks.len();
        let mut last_response = Vec::new();

        for (idx, chunk) in chunks.into_iter().enumerate() {
            let is_last = idx == total.saturating_sub(1);
            let chunk_cla = if is_last { cla } else { cla | 0x10 };

            let mut chunk_apdu = Vec::with_capacity(chunk.len() + 5);
            chunk_apdu.push(chunk_cla);
            chunk_apdu.push(ins);
            chunk_apdu.push(p1);
            chunk_apdu.push(p2);
            chunk_apdu.push(chunk.len() as u8);
            chunk_apdu.extend_from_slice(chunk);

            last_response = self.transmit(&chunk_apdu)?;
        }

        Ok(last_response)
    }

    /// Begin exclusive transaction.
    pub fn begin_transaction(&self) -> Result<()> {
        let res = unsafe { (self.context.lib.fn_begin_transaction)(self.h_card) };
        if res != SCARD_S_SUCCESS {
            let detail = self.context.lib.stringify_error(res);
            return Err(SlideError::Piv(format!(
                "SCardBeginTransaction error 0x{res:08X}{detail}"
            )));
        }
        Ok(())
    }

    /// End exclusive transaction.
    pub fn end_transaction(&self) -> Result<()> {
        let res = unsafe { (self.context.lib.fn_end_transaction)(self.h_card, SCARD_LEAVE_CARD) };
        if res != SCARD_S_SUCCESS {
            let detail = self.context.lib.stringify_error(res);
            return Err(SlideError::Piv(format!(
                "SCardEndTransaction error 0x{res:08X}{detail}"
            )));
        }
        Ok(())
    }
}

impl Drop for PcscCard {
    fn drop(&mut self) {
        if self.h_card != 0 {
            let _ = unsafe { (self.context.lib.fn_disconnect)(self.h_card, SCARD_LEAVE_CARD) };
            self.h_card = 0;
        }
    }
}
