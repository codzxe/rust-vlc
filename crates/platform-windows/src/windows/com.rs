//! COM foundation for the Windows integrations (card 7.A1): the
//! apartment guard, `CoCreateInstance`, GUIDs, wide-string helpers
//! and the PROVARIANT/PROPERTYKEY vocabulary the WASAPI property
//! store speaks.
//!
//! Hand-rolled per the clean-room rule: no `windows-s`/`winapi`
//! crates — the ole32 C-ABI is declared here, every `unsafe` block
//! carrying its invariant. **Compile-verified via `cargo check
//! --target x86_64-pc-windows-msvc`; runtime NOT exercised in this
//! sandbox (no Windows SDK) — `[VERIFY]`.**
//!
//! # Vtable discipline
//!
//! COM interfaces are vtable pointers: `*mut T` where `T` is a
//! `#[repr(C)]` struct whose first field is a `*const TVtbl`. Calls
//! are `((*(*ptr).vtbl).method)(ptr, args…)`. Every vtable struct in
//! this crate declares **all** slots up to the last one used, in
//! interface order (slots before the used one keep the ABI honest;
//! unused slots are named with a leading underscore so the compiler
//! does not demand reads that never happen). The layouts mirror the
//! SDK headers named per interface; the orderings are ABI-stable
//! history, but runtime confirmation is `[VERIFY]`.

// SAFETY-SCOPED MODULE (doc 01 §2.7 / MP §9): the ole32/COM ABI is
// the audited OS-service exception this file exercises.
#![allow(unsafe_code)]

use std::ffi::c_void;

use crate::error::PlatformError;

/// `HRESULT` (win32 status).
pub type Hresult = i32;

/// `S_OK`.
pub const S_OK: Hresult = 0;
/// `S_FALSE` (success, "already done").
pub const S_FALSE: Hresult = 1;
/// `RPC_E_CHANGED_MODE` — the thread already joined a different
/// apartment model.
pub const RPC_E_CHANGED_MODE: Hresult = 0x8001_0106u32 as i32;

/// `COINIT_APARTMENTTHREADED` — a single-threaded apartment, the
/// model the shell dialogs (and the MMDevice notification callbacks
/// they share a thread with) expect.
pub const COINIT_APARTMENTTHREADED: u32 = 0x2;

/// `CLSCTX_INPROC_SERVER` — the only creation context the integrations
/// use (all their classes are in-proc).
pub const CLSCTX_INPROC_SERVER: u32 = 0x1;

/// `STGM_READ` for `IMMDevice::OpenPropertyStore`.
pub const STGM_READ: u32 = 0;

/// A GUID (win32 layout: u32, u16, u16, u64-as-bytes — all fields
/// little-endian on every supported Windows target).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Guid {
    /// First dword.
    pub data1: u32,
    /// First word.
    pub data2: u16,
    /// Second word.
    pub data3: u16,
    /// Final eight bytes.
    pub data4: [u8; 8],
}

/// `IUnknown` — the three root vtable slots every interface shares.
#[repr(C)]
pub struct UnknownVtbl {
    /// `QueryInterface(this, riid, ppv)`.
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> Hresult,
    /// `AddRef(this)`.
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    /// `Release(this)`.
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
}

// SAFETY: the declared ABI of ole32.dll (CoInitializeEx/
// CoUninitialize/CoCreateInstance/CoTaskMemFree/PropVariantClear).
// Invariants honored at every call site below and in the per-API
// modules: arguments are valid for the documented prototypes;
// CoInitializeEx/CoUninitialize are paired per thread; handles
// returned by CoCreateInstance are released exactly once.
#[link(name = "ole32")]
unsafe extern "system" {
    fn CoInitializeEx(pvreserved: *mut c_void, dwcoinit: u32) -> Hresult;
    fn CoUninitialize();
    fn CoCreateInstance(
        rclsid: *const Guid,
        punkouter: *mut c_void,
        dwclscontext: u32,
        riid: *const Guid,
        ppv: *mut *mut c_void,
    ) -> Hresult;
    fn CoTaskMemFree(pv: *mut c_void);
    fn PropVariantClear(pvar: *mut PropVariant) -> Hresult;
}

/// One COM apartment membership on the current thread. `Drop` calls
/// `CoUninitialize` exactly once per successful `CoInitializeEx`
/// (both `S_OK` and `S_FALSE` require the balancing call per the
/// ole32 contract).
pub struct ComApartment {
    uninitialize: bool,
}

impl ComApartment {
    /// Join the calling thread to an STA.
    ///
    /// # Errors
    /// Typed [`PlatformError::CallFailed`] when the thread already
    /// belongs to a different apartment model
    /// (`RPC_E_CHANGED_MODE`) or the call otherwise fails — the
    /// integrations refuse to guess.
    pub fn new() -> Result<ComApartment, PlatformError> {
        // SAFETY: pvreserved is documented as NULL; the flags are the
        // documented COINIT_* mask. The returned HRESULT is checked,
        // and CoUninitialize is only paired with the success codes
        // (see Drop).
        let hr = unsafe { CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED) };
        match hr {
            S_OK | S_FALSE => Ok(ComApartment { uninitialize: true }),
            RPC_E_CHANGED_MODE => Err(PlatformError::CallFailed {
                api: "CoInitializeEx",
                code: Some(i64::from(RPC_E_CHANGED_MODE)),
                detail: "the thread already joined the multithreaded apartment".to_string(),
            }),
            other => Err(PlatformError::CallFailed {
                api: "CoInitializeEx",
                code: Some(i64::from(other)),
                detail: "COM refused the apartment initialization".to_string(),
            }),
        }
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.uninitialize {
            // SAFETY: pairs with the CoInitializeEx success in `new`,
            // on the same thread (the guard never crosses threads:
            // every integration creates and drops it inside one call).
            unsafe {
                CoUninitialize();
            }
        }
    }
}

/// `CoCreateInstance` for an in-proc class, returned as a raw
/// interface pointer.
///
/// # Errors
/// Typed [`PlatformError::CallFailed`] on any failing `HRESULT`
/// (class not registered, out of memory, …).
///
/// # Safety (caller invariant)
/// The caller must know the concrete vtable layout behind the
/// returned pointer for `riid`, release it exactly once (via
/// `release_interface`), and use it only while the creating apartment
/// is alive.
pub unsafe fn co_create_instance(
    clsid: &Guid,
    riid: &Guid,
) -> Result<*mut c_void, PlatformError> {
    let mut interface: *mut c_void = std::ptr::null_mut();
    // SAFETY: clsid/riid point at complete Guid values; the context
    // is CLSCTX_INPROC_SERVER; punkouter is NULL (no aggregation);
    // the out-pointer is a valid, zero-initialized slot. The returned
    // HRESULT is checked before the pointer is handed out.
    let hr = unsafe {
        CoCreateInstance(
            clsid,
            std::ptr::null_mut(),
            CLSCTX_INPROC_SERVER,
            riid,
            &mut interface,
        )
    };
    if hr != S_OK || interface.is_null() {
        return Err(PlatformError::CallFailed {
            api: "CoCreateInstance",
            code: Some(i64::from(hr)),
            detail: "the class could not be created".to_string(),
        });
    }
    Ok(interface)
}

/// Release a COM interface obtained from `co_create_instance` (or a
/// vtable method that returned a new reference) exactly once.
///
/// # Safety (caller invariant)
/// `interface` must be a live pointer whose vtable begins with the
/// IUnknown slots, and this must be the caller's only use after the
/// call.
pub unsafe fn release_interface(interface: *mut c_void) {
    if interface.is_null() {
        return;
    }
    // SAFETY: every COM interface's first vtable slot #3 is Release;
    // reading it through the UnknownVtbl prefix is layout-guaranteed
    // for any COM pointer. The call drops one reference only.
    let unknown = interface as *mut ComObject;
    unsafe {
        let release = (*(*unknown).vtbl).unknown.release;
        release(interface);
    }
}

/// The COM object header every interface pointer starts with.
#[repr(C)]
struct ComObject {
    vtbl: *const VtblPrefix,
}

/// The IUnknown prefix of any vtable.
#[repr(C)]
struct VtblPrefix {
    unknown: UnknownVtbl,
}

/// `PROPERTYKEY` (propsys layout).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PropertyKey {
    /// The property format GUID.
    pub fmtid: Guid,
    /// The property id within the format.
    pub pid: u32,
}

/// `PROPVARIANT` — only the `VT_LPWSTR` arm is ever constructed by
/// the property-store reads this crate performs; the union keeps the
/// 8-byte payload slot.
#[repr(C)]
#[derive(Clone, Copy)]
pub union PropVariant {
    /// Header: `VT_LPWSTR` = 31 plus three reserved words.
    pub header: PropVariantHeader,
    /// Padding pinning the struct at its ABI size.
    pub raw: [u64; 2],
}

/// The leading fields of `PROPVARIANT`.
#[repr(C)]
pub struct PropVariantHeader {
    /// The `VARENUM` type tag.
    pub vt: u16,
    /// Reserved (three words, always zero).
    pub reserved: [u16; 3],
    /// The payload slot (a pointer when `VT_LPWSTR`).
    pub pointer: *mut u16,
}

/// `VT_LPWSTR` — a CoTaskMemAlloc'd wide string.
pub const VT_LPWSTR: u16 = 31;

/// Zero a `PROPVARIANT` for hand-in to `IPropertyStore::GetValue`.
pub fn empty_prop_variant() -> PropVariant {
    PropVariant {
        raw: [0, 0],
    }
}

/// Read a `VT_LPWSTR` `PROPVARIANT` into an owned `String`, then
/// free the payload with `PropVariantClear` (the documented owner of
/// the CoTaskMemAlloc'd string).
///
/// # Errors
/// [`PlatformError::CallFailed`] when the variant is not
/// `VT_LPWSTR`; [`PlatformError::InvalidEncoding`] when the wide
/// string is not valid UTF-16.
fn prop_variant_string(variant: &mut PropVariant) -> Result<String, PlatformError> {
    // SAFETY: reading the header is within the union's declared
    // memory; PROPVARIANT's first two bytes are the VARTYPE for every
    // possible layout.
    let vt = unsafe { variant.header.vt };
    if vt != VT_LPWSTR {
        return Err(PlatformError::CallFailed {
            api: "IPropertyStore::GetValue",
            code: None,
            detail: format!("the property came back as VARTYPE {vt}, not VT_LPWSTR"),
        });
    }
    // SAFETY: vt == VT_LPWSTR, so the payload slot holds a
    // CoTaskMemAlloc'd NUL-terminated wide string owned by the
    // variant; from_wide copies out before PropVariantClear frees.
    let pointer = unsafe { variant.header.pointer };
    let text = from_wide(pointer)?;
    // SAFETY: PropVariantClear is the documented deallocator for a
    // PROPVARIANT the property store filled in; the pointer argument
    // is this live variant.
    unsafe {
        PropVariantClear(variant);
    }
    Ok(text)
}

/// Convert a NUL-terminated UTF-16 (`PCWSTR`/`PWSTR`) buffer to an
/// owned `String` (strict decoding — no lossy guessing).
///
/// # Errors
/// [`PlatformError::InvalidEncoding`] on lone surrogates.
///
/// # Safety (caller invariant)
/// `wide` points at a NUL-terminated UTF-16 buffer valid for the
/// duration of this call; only the bytes up to the NUL are read.
pub unsafe fn from_wide(wide: *const u16) -> Result<String, PlatformError> {
    if wide.is_null() {
        return Ok(String::new());
    }
    let mut len = 0usize;
    // SAFETY: the buffer is NUL-terminated (caller invariant); the
    // loop stops at the terminator and reads each u16 exactly once.
    unsafe {
        while *wide.add(len) != 0 {
            len += 1;
        }
    }
    // SAFETY: len covers exactly the pre-terminator code units the
    // loop validated; slice construction is within the buffer.
    let slice: &[u16] = unsafe { std::slice::from_raw_parts(wide, len) };
    String::from_utf16(slice).map_err(|_| PlatformError::InvalidEncoding {
        api: "wide-string conversion",
    })
}

/// Encode a `&str` as a NUL-terminated UTF-16 buffer for `PCWSTR`
/// arguments (interior NULs are rejected — they would truncate the
/// OS string).
///
/// # Errors
/// [`PlatformError::InvalidEncoding`] when the text contains a NUL.
pub fn to_wide(text: &str) -> Result<Vec<u16>, PlatformError> {
    if text.contains('\u{0}') {
        return Err(PlatformError::InvalidEncoding {
            api: "wide-string conversion",
        });
    }
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);
    Ok(wide)
}

/// Free a `PWSTR`/`LPWSTR` the OS allocated with `CoTaskMemAlloc`
/// (MMDevice ids, shell display names).
///
/// # Safety (caller invariant)
/// `wide` was allocated by `CoTaskMemAlloc` (directly or through a
/// shell/property API documented to do so) and is not used after
/// this call.
pub unsafe fn free_wide(wide: *mut u16) {
    // SAFETY: the caller invariant is exactly CoTaskMemAlloc's; NULL
    // is documented as a legal no-op for CoTaskMemFree.
    unsafe {
        CoTaskMemFree(wide.cast::<c_void>());
    }
}

/// The property-store string read (shared by the WASAPI and dialog
/// integrations): `GetValue` into a zeroed variant, extract, clear.
///
/// # Safety (caller invariant)
/// `store` is a live `IPropertyStore*`; `key` points at a complete
/// `PROPERTYKEY`.
pub unsafe fn property_store_string(
    store: *mut c_void,
    key: &PropertyKey,
) -> Result<String, PlatformError> {
    let mut variant = empty_prop_variant();
    // SAFETY: store is a live IPropertyStore (caller invariant) and
    // the vtable prefix below (IUnknown slots + GetCount/GetAt/
    // GetValue) matches the documented IPropertyStore order; the
    // out-variant is zeroed per the API contract.
    let store_obj = store as *mut PropertyStoreObject;
    let get_value = unsafe { (*(*store_obj).vtbl).get_value };
    // SAFETY: arguments are valid for the GetValue prototype.
    let hr = unsafe { get_value(store, key, &mut variant) };
    if hr != S_OK {
        return Err(PlatformError::CallFailed {
            api: "IPropertyStore::GetValue",
            code: Some(i64::from(hr)),
            detail: "the property read failed".to_string(),
        });
    }
    prop_variant_string(&mut variant)
}

/// The `IPropertyStore` object header (vtable slots 1–5).
#[repr(C)]
struct PropertyStoreObject {
    vtbl: *const PropertyStoreVtbl,
}

/// `IPropertyStore` vtable (propsys.h order).
#[repr(C)]
struct PropertyStoreVtbl {
    /// IUnknown.
    pub unknown: UnknownVtbl,
    /// `GetCount(this, *)`.
    pub _get_count: unsafe extern "system" fn(*mut c_void, *mut u32) -> Hresult,
    /// `GetAt(this, i, *)`.
    pub _get_at: unsafe extern "system" fn(*mut c_void, u32, *mut PropertyKey) -> Hresult,
    /// `GetValue(this, key, *)`.
    pub get_value:
        unsafe extern "system" fn(*mut c_void, *const PropertyKey, *mut PropVariant) -> Hresult,
    /// `SetValue(this, key, *)`.
    pub _set_value:
        unsafe extern "system" fn(*mut c_void, *const PropertyKey, *const PropVariant) -> Hresult,
    /// `Commit(this)`.
    pub _commit: unsafe extern "system" fn(*mut c_void) -> Hresult,
}
