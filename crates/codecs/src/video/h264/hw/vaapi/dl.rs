//! Runtime dynamic-library loading for the VA-API lane (card
//! 7.B1) — the same mechanism `platform-linux` (7.A2) uses for
//! PulseAudio: **no link-time dependency**, so machines without
//! libva still run Lumen and get a typed
//! [`LibraryUnavailable`](crate::video::h264::hw::capability::HwUnavailableReason::LibraryUnavailable)
//! capability report instead of a load failure.
//!
//! This is a minimal sibling of `platform-linux`'s audited
//! `linux::dl` (the layering wall forbids importing it from this
//! L3 crate — edges point downward only, doc 01 §1.4), kept to the
//! four `dlfcn` symbols with the same invariants.
//!
//! # Safety posture
//!
// SAFETY-SCOPED MODULE (MP §9 / doc 01 §2.7, ADR-001 §5 — VA-API
// is the E2 OS-service binding): the dlopen(3) ABI is the audited
// exception exercised here. Nothing interprets library contents —
// symbol typing lives in `library.rs` with its own invariants.
#![allow(unsafe_code)]

use std::ffi::{CStr, CString, c_char, c_int, c_void};

/// `RTLD_NOW | RTLD_LOCAL` (resolve eagerly, keep symbols private —
/// the conservative synchronous-call choice).
const RTLD_NOW_LOCAL: c_int = 0x2;

// SAFETY: declared ABI of dlopen(3)/dlsym(3)/dlclose(3)/dlerror(3)
// (POSIX + glibc). Invariants honored at every call site below:
// - `dlopen` gets a valid NUL-terminated path;
// - `dlsym` gets a handle returned by a successful `dlopen`;
// - the handle is closed exactly once (ownership in SharedLibrary);
// - `dlerror` results are read before any other dl* call and only
//   inspected as C strings (static storage, never freed by us).
unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
    fn dlerror() -> *const c_char;
}

/// Take the pending loader error, if any (clears it — POSIX).
fn last_error() -> Option<String> {
    // SAFETY: dlerror takes no arguments and returns a pointer to
    // thread-local static storage owned by libc; we only read it as
    // a C string and never free it. NULL means "no error".
    unsafe {
        let raw = dlerror();
        if raw.is_null() {
            None
        } else {
            Some(CStr::from_ptr(raw).to_string_lossy().into_owned())
        }
    }
}

/// An owned `dlopen` handle. `Send` is sound because glibc's loader
/// is thread-safe and the single-owner discipline below upholds the
/// one invariant that matters: no use after close.
pub struct SharedLibrary {
    handle: *mut c_void,
    name: String,
}

// SAFETY: the raw handle is never used after drop (single owner,
// enforced by ownership) and glibc's dl* functions are thread-safe;
// moving the handle across threads carries no loader state.
unsafe impl Send for SharedLibrary {}

impl std::fmt::Debug for SharedLibrary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedLibrary")
            .field("name", &self.name)
            .finish()
    }
}

/// The typed load failure (rendered into
/// [`HwUnavailableReason::LibraryUnavailable`](crate::video::h264::hw::capability::HwUnavailableReason::LibraryUnavailable)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryLoadError {
    /// Every SONAME that was attempted.
    pub names: Vec<String>,
    /// The loader's diagnosis for the last failure (or why no
    /// candidate was tried).
    pub detail: String,
}

impl SharedLibrary {
    /// Try each candidate SONAME in order; first success wins.
    ///
    /// # Errors
    /// [`LibraryLoadError`] naming every candidate and the loader's
    /// diagnosis for the last failure.
    pub fn open(candidate_names: &[&str]) -> Result<SharedLibrary, LibraryLoadError> {
        let mut last_detail = String::from("no candidate names given");
        for name in candidate_names {
            // A CString keeps the SONAME NUL-terminated for the ABI;
            // interior NULs would truncate it, so treat them as a
            // failed candidate rather than mis-calling libc.
            let Ok(path) = CString::new(*name) else {
                last_detail = format!("candidate {name:?} contains an interior NUL");
                continue;
            };
            // SAFETY: `path` is a valid NUL-terminated C string
            // (owned above); flags are the documented RTLD_* mask.
            // The returned handle is NULL on failure — checked.
            let _ = last_error(); // clear stale error first (POSIX)
            let handle = unsafe { dlopen(path.as_ptr(), RTLD_NOW_LOCAL) };
            if !handle.is_null() {
                return Ok(SharedLibrary {
                    handle,
                    name: (*name).to_string(),
                });
            }
            last_detail = last_error()
                .unwrap_or_else(|| format!("dlopen({name:?}) returned NULL without an error"));
        }
        Err(LibraryLoadError {
            names: candidate_names.iter().map(|s| (*s).to_string()).collect(),
            detail: last_detail,
        })
    }

    /// Resolve a symbol to a raw address; the caller (library.rs)
    /// owns transmuting it to the exact documented function type.
    ///
    /// # Errors
    /// [`LibraryLoadError`] on a NULL result.
    pub fn symbol(&self, name: &str) -> Result<*mut c_void, LibraryLoadError> {
        let Ok(symbol) = CString::new(name) else {
            return Err(LibraryLoadError {
                names: vec![self.name.clone()],
                detail: format!("symbol name {name:?} contains an interior NUL"),
            });
        };
        // SAFETY: `self.handle` is a live dlopen handle (owned, not
        // yet closed) and `symbol` is NUL-terminated. dlsym returns
        // NULL for "not found" — checked, with dlerror consulted.
        let _ = last_error(); // clear stale error (POSIX)
        let address = unsafe { dlsym(self.handle, symbol.as_ptr()) };
        if address.is_null() {
            return Err(LibraryLoadError {
                names: vec![self.name.clone()],
                detail: format!("symbol {name} not found"),
            });
        }
        Ok(address)
    }

    /// The SONAME this handle was opened with.
    #[must_use = "pure accessor"]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for SharedLibrary {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            // SAFETY: the handle is a live dlopen result owned
            // exclusively by this value; drop runs exactly once, so
            // dlclose pairs with the dlopen in `open`.
            unsafe {
                dlclose(self.handle);
            }
            self.handle = std::ptr::null_mut();
        }
    }
}
