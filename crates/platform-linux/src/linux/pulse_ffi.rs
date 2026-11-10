//! The PulseAudio C-ABI declarations (card 7.A2) — hand-rolled, no
//! `-sys` crate (AGENTS.md rule 2: zero new dependencies).
//!
//! Scope: exactly the 21 functions + 3 value structs the
//! enumeration/notification seam needs (the synchronous `pa_mainloop`
//! family — no threads, bounded waits via `pa_mainloop_prepare`).
//! Everything else of PulseAudio is deliberately NOT declared.
//!
//! # Layout invariants
//!
//! [`PaSinkInfo`] mirrors `pulse/introspect.h` field-for-field (the
//! v1.0 shape; later PulseAudio versions only *appended* fields, so
//! prefix reads stay correct). We never allocate this struct —
//! pointers come from the library; we only read the prefix we
//! declared. [`PaSampleSpec`], [`PaChannelMap`], [`PaCvolume`] are
//! complete by value.
//!
//! # Safety posture
//!
//! Module-level `#[allow(unsafe_code)]` (the audited OS-service
//! exception, MP §9 / doc 01 §2.7 — ADR-005 is the audio seam's
//! decision record). Every call into these declarations happens in
//! `pulse.rs` with its invariants written at the call site.

// SAFETY-SCOPED MODULE: raw PulseAudio ABI surface.
#![allow(unsafe_code)]
use std::ffi::{c_char, c_int, c_void};

use crate::error::PlatformError;
use crate::linux::dl::DynamicLibrary;

// -------------------------------------------------------------------------
// Opaque library types
// -------------------------------------------------------------------------

/// `pa_mainloop` — opaque (we hold the pointer, never deref).
#[repr(C)]
pub struct PaMainloop {
    _opaque: [u8; 0],
}
/// `pa_mainloop_api` — opaque vtable (used only as a token).
#[repr(C)]
pub struct PaMainloopApi {
    _opaque: [u8; 0],
}
/// `pa_context` — opaque.
#[repr(C)]
pub struct PaContext {
    _opaque: [u8; 0],
}
/// `pa_operation` — opaque.
#[repr(C)]
pub struct PaOperation {
    _opaque: [u8; 0],
}
/// `pa_spawn_api` — opaque; we always pass NULL.
#[repr(C)]
pub struct PaSpawnApi {
    _opaque: [u8; 0],
}
/// `pa_proplist` — opaque (present in PaSinkInfo; never touched).
#[repr(C)]
pub struct PaProplist {
    _opaque: [u8; 0],
}

// -------------------------------------------------------------------------
// Value structs (introspect.h / sample.h / channelmap.h / volume.h)
// -------------------------------------------------------------------------

/// `pa_sample_spec` — complete by value.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PaSampleSpec {
    /// `pa_sample_format_t` (a C enum — int-width).
    pub format: c_int,
    /// Samples per second.
    pub rate: u32,
    /// Channel count.
    pub channels: u8,
}

/// `pa_channel_map` — complete by value (we never read the map).
#[repr(C)]
pub struct PaChannelMap {
    /// Channel count.
    pub channels: u8,
    /// `pa_channel_position_t[32]`.
    pub map: [c_int; 32],
}

/// `pa_cvolume` — complete by value (never read).
#[repr(C)]
pub struct PaCvolume {
    /// Channel count.
    pub channels: u8,
    /// `pa_volume_t[32]`.
    pub values: [u32; 32],
}

/// `pa_sink_port_info` — prefix we never deref beyond existence.
#[repr(C)]
pub struct PaSinkPortInfo {
    /// Port name.
    pub name: *const c_char,
}

/// `pa_format_info` — opaque to us.
#[repr(C)]
pub struct PaFormatInfo {
    _opaque: [u8; 0],
}

/// `pa_sink_info` — field-for-field with `pulse/introspect.h`
/// (v1.0 shape; later versions appended fields only — prefix
/// reads remain valid). We read `name`, `index`, `description`,
/// `sample_spec` and nothing else.
#[repr(C)]
pub struct PaSinkInfo {
    /// Sink name (stable, UTF-8; library-owned).
    pub name: *const c_char,
    /// Sink index (session-scoped).
    pub index: u32,
    /// Human description (library-owned).
    pub description: *const c_char,
    /// The sink's sample spec.
    pub sample_spec: PaSampleSpec,
    /// Channel map.
    pub channel_map: PaChannelMap,
    /// Owning module index.
    pub owner_module: u32,
    /// Volume.
    pub volume: PaCvolume,
    /// Mute switch.
    pub mute: c_int,
    /// Monitor source index.
    pub monitor_source: u32,
    /// Monitor source name.
    pub monitor_source_name: *const c_char,
    /// Queued latency (µs).
    pub latency: u64,
    /// Driver name.
    pub driver: *const c_char,
    /// `pa_sink_flags_t`.
    pub flags: u32,
    /// Property list.
    pub proplist: *mut PaProplist,
    /// Configured latency (µs).
    pub configured_latency: u64,
    /// Base volume.
    pub base_volume: u32,
    /// `pa_sink_state_t`.
    pub state: c_int,
    /// Volume steps.
    pub n_volume_steps: u32,
    /// Card index.
    pub card: u32,
    /// Port count.
    pub n_ports: u32,
    /// Port array.
    pub ports: *mut *const PaSinkPortInfo,
    /// Active port.
    pub active_port: *const PaSinkPortInfo,
    /// Format count.
    pub n_formats: u8,
    /// Format array.
    pub formats: *mut *mut PaFormatInfo,
}

// -------------------------------------------------------------------------
// Callback signatures
// -------------------------------------------------------------------------

/// `pa_context_notify_cb_t` — context state changes.
pub type PaContextNotifyCb =
    Option<unsafe extern "C" fn(ctx: *mut PaContext, userdata: *mut c_void)>;
/// `pa_sink_info_cb_t` — one sink per call, then the eol call.
pub type PaSinkInfoCb = Option<
    unsafe extern "C" fn(
        ctx: *mut PaContext,
        info: *const PaSinkInfo,
        eol: c_int,
        userdata: *mut c_void,
    ),
>;
/// `pa_context_subscribe_cb_t` — event facility/type + index.
pub type PaContextSubscribeCb = Option<
    unsafe extern "C" fn(
        ctx: *mut PaContext,
        event_type: u32,
        idx: u32,
        userdata: *mut c_void,
    ),
>;
/// `pa_context_success_cb_t` — subscribe acknowledgement.
pub type PaContextSuccessCb =
    Option<unsafe extern "C" fn(ctx: *mut PaContext, success: c_int, userdata: *mut c_void)>;

// -------------------------------------------------------------------------
// Raw declarations (dlsym-resolved; never linked at build time)
// -------------------------------------------------------------------------

/// The function-pointer table resolved from the loaded library.
#[allow(clippy::struct_field_names)]
pub struct PulseApi {
    /// `pa_mainloop_new`.
    pub mainloop_new: unsafe extern "C" fn() -> *mut PaMainloop,
    /// `pa_mainloop_free`.
    pub mainloop_free: unsafe extern "C" fn(m: *mut PaMainloop),
    /// `pa_mainloop_get_api`.
    pub mainloop_get_api:
        unsafe extern "C" fn(m: *mut PaMainloop) -> *mut PaMainloopApi,
    /// `pa_mainloop_prepare` (timeout in ms; negative = forever).
    pub mainloop_prepare: unsafe extern "C" fn(m: *mut PaMainloop, timeout: c_int) -> c_int,
    /// `pa_mainloop_poll` (after a successful prepare).
    pub mainloop_poll: unsafe extern "C" fn(m: *mut PaMainloop) -> c_int,
    /// `pa_mainloop_dispatch` (after a ready poll).
    pub mainloop_dispatch: unsafe extern "C" fn(m: *mut PaMainloop) -> c_int,
    /// `pa_context_new`.
    pub context_new: unsafe extern "C" fn(
        api: *const PaMainloopApi,
        name: *const c_char,
    ) -> *mut PaContext,
    /// `pa_context_unref`.
    pub context_unref: unsafe extern "C" fn(c: *mut PaContext),
    /// `pa_context_connect`.
    pub context_connect: unsafe extern "C" fn(
        c: *mut PaContext,
        server: *const c_char,
        flags: c_int,
        api: *const PaSpawnApi,
    ) -> c_int,
    /// `pa_context_disconnect`.
    pub context_disconnect: unsafe extern "C" fn(c: *mut PaContext),
    /// `pa_context_get_state`.
    pub context_get_state: unsafe extern "C" fn(c: *const PaContext) -> c_int,
    /// `pa_context_set_state_callback`.
    pub context_set_state_callback:
        unsafe extern "C" fn(c: *mut PaContext, cb: PaContextNotifyCb, userdata: *mut c_void),
    /// `pa_context_errno` — the pa_context's pending error code.
    pub context_errno: unsafe extern "C" fn(c: *const PaContext) -> c_int,
    /// `pa_context_get_sink_info_list`.
    pub context_get_sink_info_list: unsafe extern "C" fn(
        c: *mut PaContext,
        cb: PaSinkInfoCb,
        userdata: *mut c_void,
    ) -> *mut PaOperation,
    /// `pa_context_set_subscribe_callback`.
    pub context_set_subscribe_callback: unsafe extern "C" fn(
        c: *mut PaContext,
        cb: PaContextSubscribeCb,
        userdata: *mut c_void,
    ),
    /// `pa_context_subscribe`.
    pub context_subscribe: unsafe extern "C" fn(
        c: *mut PaContext,
        m: u32,
        cb: PaContextSuccessCb,
        userdata: *mut c_void,
    ) -> *mut PaOperation,
    /// `pa_operation_unref`.
    pub operation_unref: unsafe extern "C" fn(o: *mut PaOperation),
    /// `pa_operation_cancel` — prevents a pending callback from
    /// firing (used on the bounded-wait timeout paths).
    pub operation_cancel: unsafe extern "C" fn(o: *mut PaOperation),
    /// `pa_operation_get_state`.
    pub operation_get_state: unsafe extern "C" fn(o: *const PaOperation) -> c_int,
    /// `pa_strerror` — text for a pa_context_errno code.
    pub strerror: unsafe extern "C" fn(err: c_int) -> *const c_char,
}

/// Resolve the whole table from `lib`.
///
/// # Errors
/// [`PlatformError::SymbolMissing`] for the first absent symbol
/// (version skew — the candidate SONAMEs are tried newest first,
/// so this means an unexpectedly ancient or stripped build).
pub fn load(lib: &DynamicLibrary) -> Result<PulseApi, PlatformError> {
    // Every macro line: resolve the raw address, then transmute to
    // the exact documented signature. SAFETY (shared invariant):
    // the transmute only re-types a dlsym address — sound iff the
    // signature matches the library's prototype, which is checked
    // against pulse/mainloop.h / pulse/context.h / pulse/introspect.h
    // here and pinned by the compile-only cross-checks in CI.
    macro_rules! sym {
        ($lib:expr, $name:literal, $ty:ty) => {{
            let addr = $lib.symbol($name)?;
            // SAFETY: see the shared invariant above.
            unsafe { std::mem::transmute::<*mut std::ffi::c_void, $ty>(addr) }
        }};
    }

    Ok(PulseApi {
        mainloop_new: sym!(lib, "pa_mainloop_new", unsafe extern "C" fn() -> *mut PaMainloop),
        mainloop_free: sym!(
            lib,
            "pa_mainloop_free",
            unsafe extern "C" fn(m: *mut PaMainloop)
        ),
        mainloop_get_api: sym!(
            lib,
            "pa_mainloop_get_api",
            unsafe extern "C" fn(m: *mut PaMainloop) -> *mut PaMainloopApi
        ),
        mainloop_prepare: sym!(
            lib,
            "pa_mainloop_prepare",
            unsafe extern "C" fn(m: *mut PaMainloop, timeout: c_int) -> c_int
        ),
        mainloop_poll: sym!(
            lib,
            "pa_mainloop_poll",
            unsafe extern "C" fn(m: *mut PaMainloop) -> c_int
        ),
        mainloop_dispatch: sym!(
            lib,
            "pa_mainloop_dispatch",
            unsafe extern "C" fn(m: *mut PaMainloop) -> c_int
        ),
        context_new: sym!(
            lib,
            "pa_context_new",
            unsafe extern "C" fn(
                api: *const PaMainloopApi,
                name: *const c_char
            ) -> *mut PaContext
        ),
        context_unref: sym!(
            lib,
            "pa_context_unref",
            unsafe extern "C" fn(c: *mut PaContext)
        ),
        context_connect: sym!(
            lib,
            "pa_context_connect",
            unsafe extern "C" fn(
                c: *mut PaContext,
                server: *const c_char,
                flags: c_int,
                api: *const PaSpawnApi
            ) -> c_int
        ),
        context_disconnect: sym!(
            lib,
            "pa_context_disconnect",
            unsafe extern "C" fn(c: *mut PaContext)
        ),
        context_get_state: sym!(
            lib,
            "pa_context_get_state",
            unsafe extern "C" fn(c: *const PaContext) -> c_int
        ),
        context_set_state_callback: sym!(
            lib,
            "pa_context_set_state_callback",
            unsafe extern "C" fn(
                c: *mut PaContext,
                cb: PaContextNotifyCb,
                userdata: *mut c_void
            )
        ),
        context_errno: sym!(
            lib,
            "pa_context_errno",
            unsafe extern "C" fn(c: *const PaContext) -> c_int
        ),
        context_get_sink_info_list: sym!(
            lib,
            "pa_context_get_sink_info_list",
            unsafe extern "C" fn(
                c: *mut PaContext,
                cb: PaSinkInfoCb,
                userdata: *mut c_void
            ) -> *mut PaOperation
        ),
        context_set_subscribe_callback: sym!(
            lib,
            "pa_context_set_subscribe_callback",
            unsafe extern "C" fn(
                c: *mut PaContext,
                cb: PaContextSubscribeCb,
                userdata: *mut c_void
            )
        ),
        context_subscribe: sym!(
            lib,
            "pa_context_subscribe",
            unsafe extern "C" fn(
                c: *mut PaContext,
                m: u32,
                cb: PaContextSuccessCb,
                userdata: *mut c_void
            ) -> *mut PaOperation
        ),
        operation_unref: sym!(
            lib,
            "pa_operation_unref",
            unsafe extern "C" fn(o: *mut PaOperation)
        ),
        operation_cancel: sym!(
            lib,
            "pa_operation_cancel",
            unsafe extern "C" fn(o: *mut PaOperation)
        ),
        operation_get_state: sym!(
            lib,
            "pa_operation_get_state",
            unsafe extern "C" fn(o: *const PaOperation) -> c_int
        ),
        strerror: sym!(
            lib,
            "pa_strerror",
            unsafe extern "C" fn(err: c_int) -> *const c_char
        ),
    })
}
