//! The Objective-C runtime surface (card 7.A3) — shared by the
//! Metal layer glue and the AppKit panels.
//!
//! Hand-rolled per the clean-room rule: no `objc2`/`objc` crates.
//! The runtime's C ABI (`libobjc.A.dylib`: `objc_getClass`,
//! `sel_registerName`, `objc_retain`/`objc_release`, and
//! `objc_msgSend`) is declared here, `objc_msgSend` **typed per
//! signature** via `#[link_name]` aliases — the one sound way to
//! call a variadic-ABI dispatcher from Rust without a variadic
//! declaration (each alias pins the exact argument/return shape the
//! call site wants; the ABI is identical because `objc_msgSend`
//! forwards register-passed arguments verbatim).
//!
//! # Why per-signature aliases are sound
//!
//! `objc_msgSend` is a machine-code trampoline: it moves the
//! receiver/selector/arguments exactly as the C ABI passed them and
//! tail-jumps to the method implementation. Declaring
//! `fn(receiver, sel, args…) -> R` with `#[link_name =
//! "objc_msgSend"]` therefore typechecks a call that is
//! ABI-identical to what Objective-C code would emit for a method
//! with that signature. The one Apple-platform caveat: methods
//! returning **large structs** need `objc_msgSend_stret` (x86_64) —
//! this crate declares no such method (returns are pointers,
//! integers, or `BOOL`), so both `aarch64-apple-darwin` and
//! `x86_64-apple-darwin` are covered by the plain symbol.
//!
//! **Evidence status:** compile-verified via `cargo check --target
//! aarch64-apple-darwin` and `x86_64-apple-darwin`; runtime NOT
//! exercised in this sandbox (no macOS SDK) — `[VERIFY]`.
//!
//! # Ownership discipline (the whole unsafe story of this module)
//!
//! - Objects returned by `alloc`/`new`/`copy`/`mutableCopy`-style
//!   methods and by `objc_retain` are **ours**: released exactly once
//!   (the owning newtype's `Drop`, or an explicit
//!   [`release`] after use).
//! - Objects returned by plain factory/instance methods are
//!   **autoreleased**: valid until the enclosing
//!   [`AutoreleasePool`] drains. Every round that creates such
//!   values wraps itself in a pool; values that must outlive the
//!   pool are `objc_retain`ed first.
//! - `objc_getClass`/`sel_registerName` return runtime-owned,
//!   process-lifetime values — never released.

// SAFETY-SCOPED MODULE (doc 01 §2.7 / MP §9): the libobjc C ABI is
// the audited OS-service exception this file exercises.
#![allow(unsafe_code)]

use std::ffi::{CStr, CString, c_char, c_void};

use crate::error::PlatformError;

/// An opaque Objective-C object (`id`).
pub type ObjcId = *mut c_void;
/// An opaque Objective-C selector (`SEL`).
pub type ObjcSel = *mut c_void;
/// Objective-C `BOOL` (objc-api.h: an 8-bit integer on every
/// Apple platform; `0` false, non-zero true).
pub type ObjcBool = u8;

// SAFETY: the declared ABI of libobjc.A.dylib. Invariants honored at
// every call site in this crate: name pointers are NUL-terminated
// CStrings owned for the call; retained objects are released exactly
// once; msgSend aliases are only called with arguments matching the
// pinned signature; nothing is called after objc_release.
#[link(name = "libobjc", kind = "dylib")]
unsafe extern "C" {
    fn objc_getClass(name: *const c_char) -> ObjcId;
    fn sel_registerName(name: *const c_char) -> ObjcSel;
    fn objc_retain(object: ObjcId) -> ObjcId;
    fn objc_release(object: ObjcId);

    // `objc_msgSend`, typed per signature (module docs). Only the
    // shapes this crate uses are declared; every new shape needs its
    // own alias and its own invariant comment.
    /// `id method(id self, SEL _cmd)` — no arguments.
    #[link_name = "objc_msgSend"]
    pub fn msg_send_id(receiver: ObjcId, selector: ObjcSel) -> ObjcId;
    /// `id method(id self, SEL _cmd, id arg)`.
    #[link_name = "objc_msgSend"]
    pub fn msg_send_id_id(receiver: ObjcId, selector: ObjcSel, arg: ObjcId) -> ObjcId;
    /// `id method(id self, SEL _cmd, NSUInteger arg)` — e.g.
    /// `objectAtIndex:`.
    #[link_name = "objc_msgSend"]
    pub fn msg_send_id_u64(receiver: ObjcId, selector: ObjcSel, arg: u64) -> ObjcId;
    /// `id method(id self, SEL _cmd, const char *arg)` — e.g.
    /// `stringWithUTF8String:`.
    #[link_name = "objc_msgSend"]
    pub fn msg_send_id_cstr(receiver: ObjcId, selector: ObjcSel, arg: *const c_char) -> ObjcId;
    /// `id method(id self, SEL _cmd, const id *objects, NSUInteger
    /// count)` — `initWithObjects:count:`.
    #[link_name = "objc_msgSend"]
    pub fn msg_send_id_ptr_u64(
        receiver: ObjcId,
        selector: ObjcSel,
        objects: *const ObjcId,
        count: u64,
    ) -> ObjcId;
    /// `NSUInteger method(id self, SEL _cmd)` — e.g. `count`.
    #[link_name = "objc_msgSend"]
    pub fn msg_send_u64(receiver: ObjcId, selector: ObjcSel) -> u64;
    /// `NSInteger method(id self, SEL _cmd)` — e.g. `runModal`.
    #[link_name = "objc_msgSend"]
    pub fn msg_send_i64(receiver: ObjcId, selector: ObjcSel) -> i64;
    /// `BOOL method(id self, SEL _cmd)` — e.g. `lowPower`.
    #[link_name = "objc_msgSend"]
    pub fn msg_send_bool(receiver: ObjcId, selector: ObjcSel) -> ObjcBool;
    /// `void method(id self, SEL _cmd)` — e.g. `drain`.
    #[link_name = "objc_msgSend"]
    pub fn msg_send_void(receiver: ObjcId, selector: ObjcSel);
    /// `void method(id self, SEL _cmd, id arg)` — e.g. `setDevice:`.
    #[link_name = "objc_msgSend"]
    pub fn msg_send_void_id(receiver: ObjcId, selector: ObjcSel, arg: ObjcId);
    /// `void method(id self, SEL _cmd, BOOL arg)` — e.g.
    /// `setAllowsMultipleSelection:`.
    #[link_name = "objc_msgSend"]
    pub fn msg_send_void_bool(receiver: ObjcId, selector: ObjcSel, arg: ObjcBool);
    /// `void method(id self, SEL _cmd, CGSize arg)` —
    /// `setDrawableSize:` (CGSize = two `CGFloat` = two `f64`,
    /// passed in the two float registers on both Apple ABIs).
    #[link_name = "objc_msgSend"]
    pub fn msg_send_void_f64_f64(receiver: ObjcId, selector: ObjcSel, width: f64, height: f64);
}

/// Look up a class by name (runtime-owned, never released).
///
/// # Errors
/// [`PlatformError::ServiceUnavailable`] when the runtime does not
/// know the class (framework not linked — the honest refusal).
pub fn class(name: &str) -> Result<ObjcId, PlatformError> {
    let cname = cstring(name, "objc_getClass")?;
    // SAFETY: cname is a NUL-terminated CString owned by this call;
    // objc_getClass reads it for the duration of the call only.
    let cls = unsafe { objc_getClass(cname.as_ptr()) };
    if cls.is_null() {
        return Err(PlatformError::ServiceUnavailable {
            service: "Objective-C runtime",
            detail: format!("the class {name} is not present (framework not linked?)"),
        });
    }
    Ok(cls)
}

/// Register (or look up) a selector by name (runtime-owned).
///
/// # Errors
/// [`PlatformError::ServiceUnavailable`] if the runtime refused the
/// registration (allocation failure — the documented NULL return).
pub fn sel(name: &str) -> Result<ObjcSel, PlatformError> {
    let sname = cstring(name, "sel_registerName")?;
    // SAFETY: sname is a NUL-terminated CString owned by this call;
    // sel_registerName reads it for the duration of the call only.
    let selector = unsafe { sel_registerName(sname.as_ptr()) };
    if selector.is_null() {
        return Err(PlatformError::ServiceUnavailable {
            service: "Objective-C runtime",
            detail: format!("the selector {name} could not be registered"),
        });
    }
    Ok(selector)
}

/// Retain an object this crate wants to own beyond its current
/// scope (`objc_retain` — e.g. an autoreleased factory result a
/// newtype must outlive the pool that created it). Returns the
/// retained pointer (the same object).
///
/// # Errors
/// [`PlatformError::CallFailed`] when the runtime refused the
/// retain (NULL object or allocation failure — the honest refusal).
pub fn retain(object: ObjcId) -> Result<ObjcId, PlatformError> {
    // SAFETY: objc_retain on a live object adds one reference the
    // caller must later release exactly once (the owning newtype's
    // Drop does); a NULL input returns NULL — checked.
    let retained = unsafe { objc_retain(object) };
    if retained.is_null() {
        return Err(PlatformError::CallFailed {
            api: "objc_retain",
            code: None,
            detail: "the runtime refused the retain (NULL object?)".to_string(),
        });
    }
    Ok(retained)
}

/// Release an object this crate owns (alloc/new/copy/retain
/// semantics). NULL is a documented no-op for our own safety.
///
/// # Safety (caller invariant)
/// `object` was obtained from a method or function with
/// create/copy/retain semantics and this is the caller's only use
/// after the call.
pub unsafe fn release(object: ObjcId) {
    if object.is_null() {
        return;
    }
    // SAFETY: the caller invariant is exactly objc_release's; the
    // call drops one reference on the runtime's owned object.
    unsafe {
        objc_release(object);
    }
}

/// A C string for a runtime call, rejecting interior NULs (they
/// would truncate the name at the ABI boundary).
fn cstring(name: &str, api: &'static str) -> Result<CString, PlatformError> {
    CString::new(name).map_err(|_| PlatformError::CallFailed {
        api,
        code: None,
        detail: format!("the name {name:?} contains an interior NUL"),
    })
}

/// Copy an `NSString`'s UTF-8 bytes into an owned `String`.
///
/// # Errors
/// [`PlatformError::InvalidEncoding`] when the string cannot be read
/// (NULL pointer, or bytes that are not UTF-8).
///
/// # Safety (caller invariant)
/// `string` is a live `NSString` (not NULL-checked callers must pass
/// non-NULL), valid for the duration of this call.
pub unsafe fn nsstring_to_string(string: ObjcId) -> Result<String, PlatformError> {
    if string.is_null() {
        return Ok(String::new());
    }
    let utf8_sel = sel("UTF8String")?;
    // SAFETY: string is live (caller invariant); UTF8String takes no
    // arguments and returns a pointer to the string's internal
    // NUL-terminated UTF-8 buffer, valid until the string is
    // released — copied out below before any release can run.
    let raw = unsafe { msg_send_id(string, utf8_sel) };
    if raw.is_null() {
        return Err(PlatformError::InvalidEncoding {
            api: "NSString UTF8String",
        });
    }
    // SAFETY: the pointer is a NUL-terminated C string per the
    // UTF8String contract; from_ptr only borrows it for the copy.
    let cstr = unsafe { CStr::from_ptr(raw.cast::<c_char>()) };
    cstr.to_str()
        .map(str::to_string)
        .map_err(|_| PlatformError::InvalidEncoding {
            api: "NSString UTF8String",
        })
}

/// Create an `NSString` from UTF-8 text (`+[NSString
/// stringWithUTF8String:]`). **Autoreleased** — valid until the
/// enclosing pool drains; retain it (or copy it out) to keep it.
///
/// # Errors
/// [`PlatformError::InvalidEncoding`] when the text contains an
/// interior NUL.
pub fn nsstring_from_utf8(text: &str) -> Result<ObjcId, PlatformError> {
    let cname = cstring(text, "stringWithUTF8String:")?;
    let cls = class("NSString")?;
    let selector = sel("stringWithUTF8String:")?;
    // SAFETY: cls is the live NSString class; the C string is
    // NUL-terminated and owned by this call; the signature (class,
    // sel, const char*) -> id matches the pinned alias.
    let string = unsafe { msg_send_id_cstr(cls, selector, cname.as_ptr()) };
    if string.is_null() {
        return Err(PlatformError::CallFailed {
            api: "NSString stringWithUTF8String:",
            code: None,
            detail: "the runtime refused the string creation".to_string(),
        });
    }
    Ok(string)
}

/// A scoped autorelease pool: `[[NSAutoreleasePool alloc] init]` on
/// the calling thread, `drain` on scope exit (the documented
/// pairing — drain releases the pool and every autoreleased object
/// it captured).
pub struct AutoreleasePool {
    pool: ObjcId,
    drain: ObjcSel,
}

impl AutoreleasePool {
    /// Create a pool on the calling thread.
    ///
    /// # Errors
    /// Typed [`PlatformError`] when the runtime refused the class or
    /// the allocation (out of memory — the honest refusal).
    pub fn new() -> Result<AutoreleasePool, PlatformError> {
        let cls = class("NSAutoreleasePool")?;
        let alloc = sel("alloc")?;
        let init = sel("init")?;
        // SAFETY: cls is the live NSAutoreleasePool class; alloc
        // returns an owned instance; the signature (class, sel) -> id
        // matches the pinned alias.
        let allocated = unsafe { msg_send_id(cls, alloc) };
        if allocated.is_null() {
            return Err(PlatformError::CallFailed {
                api: "NSAutoreleasePool alloc",
                code: None,
                detail: "the runtime refused the pool allocation".to_string(),
            });
        }
        // SAFETY: allocated is the live, owned instance from alloc;
        // init consumes it and returns the initialized (still owned)
        // object.
        let pool = unsafe { msg_send_id(allocated, init) };
        if pool.is_null() {
            // SAFETY: alloc succeeded but init failed — release the
            // un-initialized memory before refusing.
            unsafe {
                release(allocated);
            }
            return Err(PlatformError::CallFailed {
                api: "NSAutoreleasePool init",
                code: None,
                detail: "the pool could not be initialized".to_string(),
            });
        }
        Ok(AutoreleasePool {
            pool,
            drain: sel("drain")?,
        })
    }
}

impl Drop for AutoreleasePool {
    fn drop(&mut self) {
        // SAFETY: pairs with the alloc/init in `new` on the same
        // thread; drain releases the pool exactly once (self is
        // never reused after drop).
        unsafe {
            msg_send_void(self.pool, self.drain);
        }
    }
}
