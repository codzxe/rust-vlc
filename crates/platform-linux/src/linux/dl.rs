//! Runtime dynamic-library loading (card 7.A2) — the mechanism that
//! lets the PulseAudio integration degrade to a **typed
//! `Unavailable`** instead of a link-time dependency.
//!
//! The crate declares no link-time dependency on libpulse: it
//! `dlopen`s the client library at runtime, so a machine without
//! PulseAudio still runs Lumen and gets an honest capability report.
//! This module is the tiny `unsafe` wrapper around `dlopen`/`dlsym`
//! (glibc's `dlfcn` surface, merged into libc since glibc 2.34 and
//! still passed `-ldl` by the linux-gnu std).
//!
//! # Safety posture
//!
//! The only `unsafe` here is the raw loader ABI (module-level
//! `#[allow(unsafe_code)]`, the tools/mem-audit precedent for the
//! workspace's `deny(unsafe_code)`). Every invariant is written next
//! to its block. [`DynamicLibrary`] owns its handle and closes it
//! exactly once on drop.

// SAFETY-SCOPED MODULE (doc 01 §2.7 / MP §9): the dlopen(3) ABI is
// the audited OS-service exception this file exercises. Nothing in
// here interprets library contents — symbol typing lives in the
// per-API modules with their own invariants.
#![allow(unsafe_code)]
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fmt;

use crate::error::PlatformError;

/// `RTLD_NOW | RTLD_LOCAL` on Linux (resolve everything eagerly,
/// keep symbols private — the conservative choice for an audio
/// client we call synchronously).
const RTLD_NOW_LOCAL: c_int = 0x2;

// SAFETY: declared ABI of dlopen(3)/dlsym(3)/dlclose(3)/dlerror(3)
// (POSIX + glibc). Invariants honored at every call site below:
// - `dlopen` gets a valid NUL-terminated path or NULL;
// - `dlsym` gets a handle returned by a successful `dlopen`;
// - the handle is closed exactly once (ownership in DynamicLibrary);
// - `dlerror` results are read before any other dl* call and only
//   inspected as C strings (never freed by us — static storage).
unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
    fn dlerror() -> *const c_char;
}

/// Take the pending loader error, if any (clears it — POSIX).
fn last_error() -> Option<String> {
    // SAFETY: dlerror takes no arguments, returns a pointer to
    // thread-local static storage owned by libc; we only read it
    // as a C string and never free it. NULL means "no error".
    unsafe {
        let raw = dlerror();
        if raw.is_null() {
            None
        } else {
            Some(CStr::from_ptr(raw).to_string_lossy().into_owned())
        }
    }
}

/// An owned `dlopen` handle. `Send` is sound because glibc's
/// loader is thread-safe (`dlclose` may run on any thread; the
/// invariant we must uphold — no use after close — is enforced
/// by ownership).
pub struct DynamicLibrary {
    handle: *mut c_void,
    name: String,
}

// SAFETY: the raw handle is never used after drop (single owner)
// and glibc's dlopen/dlsym/dlclose are thread-safe. Moving the
// handle between threads carries no library state of its own.
unsafe impl Send for DynamicLibrary {}

impl fmt::Debug for DynamicLibrary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynamicLibrary")
            .field("name", &self.name)
            .finish()
    }
}

impl DynamicLibrary {
    /// Try each candidate SONAME in order; first success wins.
    ///
    /// # Errors
    /// [`PlatformError::LibraryUnavailable`] naming every
    /// candidate and the loader's diagnosis for the last failure.
    pub fn open(candidate_names: &[&str]) -> Result<DynamicLibrary, PlatformError> {
        let mut last_detail = String::from("no candidate names given");
        for name in candidate_names {
            // A CString keeps the path NUL-terminated for the ABI;
            // interior NULs would truncate the path, so treat them
            // as a failed candidate rather than mis-calling libc.
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
                return Ok(DynamicLibrary {
                    handle,
                    name: (*name).to_string(),
                });
            }
            last_detail = last_error()
                .unwrap_or_else(|| format!("dlopen({name:?}) returned NULL without an error"));
        }
        Err(PlatformError::LibraryUnavailable {
            names: candidate_names.iter().map(|s| (*s).to_string()).collect(),
            detail: last_detail,
        })
    }

    /// Resolve a symbol. The returned pointer carries no lifetime
    /// — callers must transmute it to a concrete function type
    /// with the exact documented signature of that symbol (the
    /// per-API modules own that step and its invariants).
    ///
    /// # Errors
    /// [`PlatformError::SymbolMissing`] on a NULL result.
    pub fn symbol(&self, name: &'static str) -> Result<*mut c_void, PlatformError> {
        let Ok(symbol) = CString::new(name) else {
            return Err(PlatformError::SymbolMissing {
                library: self.name.clone(),
                symbol: name,
            });
        };
        // SAFETY: `self.handle` is a live dlopen handle (owned,
        // not yet closed) and `symbol` is NUL-terminated. dlsym
        // returns NULL for "not found" — checked, with dlerror
        // consulted for the diagnosis.
        let _ = last_error(); // clear stale error (POSIX)
        let address = unsafe { dlsym(self.handle, symbol.as_ptr()) };
        if address.is_null() {
            return Err(PlatformError::SymbolMissing {
                library: self.name.clone(),
                symbol: name,
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

impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            // SAFETY: the handle is a live dlopen result owned
            // exclusively by this value; drop runs exactly once,
            // so dlclose pairs with the dlopen in `open`.
            unsafe {
                dlclose(self.handle);
            }
            self.handle = std::ptr::null_mut();
        }
    }
}
