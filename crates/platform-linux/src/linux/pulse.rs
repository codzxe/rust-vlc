//! The PulseAudio client (card 7.A2): sink enumeration + change
//! notifications over the synchronous `pa_mainloop` — no threads,
//! bounded waits (D-2), one short-lived connection per enumeration
//! and one persistent watch per subscription.
//!
//! # Runtime model (the honest part)
//!
//! - The library is `dlopen`ed ([`PulseAudioDevices::probe`]): a
//!   machine without libpulse gets a typed
//!   [`PlatformError::LibraryUnavailable`], never a link error or a
//!   fake device list.
//! - A machine **with** libpulse but no running server gets a typed
//!   [`PlatformError::ServiceUnavailable`] from the connect phase
//!   (the sandbox case — this path is runtime-tested here).
//! - A machine with a live server gets real sinks as
//!   [`DeviceDesc`]s, ids minted from the stable sink *name*
//!   ([`crate::audio::stable_device_id`]).
//!
//! # Callback discipline (why the raw pointers are sound)
//!
//! All PulseAudio callbacks fire **inside**
//! `pa_mainloop_prepare/poll/dispatch`, which this client calls
//! synchronously from `&mut self` methods. Stack-resident callback
//! states therefore cannot outlive their userdata: every
//! registration that points at stack memory is cleared (or its
//! operation cancelled) before the owning scope returns. The one
//! persistent registration (the subscription callback) points into
//! the heap [`WatchState`], which outlives the context serving it —
//! `Watch`'s drop order unregisters before the state frees.

// SAFETY-SCOPED MODULE (doc 01 §2.7 / MP §9): the dlopen'd
// PulseAudio C ABI is the audited OS-service exception this file
// exercises (ADR-005 is the audio seam's decision record).
#![allow(unsafe_code)]

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::sync::Arc;
use std::time::{Duration, Instant};

use audio_output::{DeviceDesc, DeviceId};
use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};

use crate::audio::{AudioDevices, stable_device_id};
use crate::error::PlatformError;
use crate::events::{DeviceEventQueue, PlatformEvent};
use crate::linux::dl::DynamicLibrary;
use crate::linux::pulse_ffi::{PaContext, PaMainloop, PaSampleSpec, PaSinkInfo, PulseApi};

/// Candidate SONAMEs, newest stable ABI first.
const PULSE_SONAMES: [&str; 2] = ["libpulse.so.0", "libpulse.so"];

/// `pa_context_state_t` values we branch on.
const PA_CONTEXT_READY: c_int = 4;
const PA_CONTEXT_FAILED: c_int = 5;

/// Subscription mask: sinks + server + card change events
/// (`pulse/def.h`: `PA_SUBSCRIPTION_MASK_SINK` 0x0001 |
/// `PA_SUBSCRIPTION_MASK_SERVER` 0x0080 | `PA_SUBSCRIPTION_MASK_CARD`
/// 0x0200).
const PA_SUBSCRIPTION_MASK: u32 = 0x0001 | 0x0080 | 0x0200;

/// Event facility/type masks (`pulse/def.h`).
const FACILITY_MASK: u32 = 0x0F00;
const TYPE_MASK: u32 = 0x0030;
const FACILITY_SINK: u32 = 0x0000;
const FACILITY_CARD: u32 = 0x0900;
const FACILITY_SERVER: u32 = 0x0700;
const TYPE_NEW: u32 = 0x0000;
const TYPE_REMOVE: u32 = 0x0020;

/// `pa_operation_state_t`: RUNNING is the only state worth waiting
/// through (DONE/CANCELLED both end the wait).
const OPERATION_RUNNING: c_int = 0;

/// Connect deadline (D-2: no unbounded waits on foreign loops).
const CONNECT_DEADLINE: Duration = Duration::from_millis(2_000);
/// Enumeration deadline after connect.
const ENUM_DEADLINE: Duration = Duration::from_millis(2_000);
/// Per-iteration mainloop wait bound (ms).
const PUMP_SLICE: c_int = 50;

/// The loaded-and-resolved client. **Field order is the drop
/// order**: the watch tears down before the API table and library
/// handle it calls into. The API table is an `Arc` so the watch can
/// hold its own reference — no self-referential pointer that would
/// dangle if the owner moves (moving a struct invalidates pointers
/// into its fields; the shared heap allocation does not move).
pub struct PulseAudioDevices {
    watch: Option<Watch>,
    api: Arc<PulseApi>,
    lib: DynamicLibrary,
}

/// One persistent subscription session. Field order is the drop
/// order: `api` (a shared reference to the owner's table) is read
/// during the teardown of `ml`/`ctx`, and `state` — the subscription
/// callback's userdata — frees only after the context that references
/// it is destroyed.
struct Watch {
    api: Arc<PulseApi>,
    ml: *mut PaMainloop,
    ctx: *mut PaContext,
    state: Box<WatchState>,
}

// SAFETY: the raw mainloop/context pointers are only ever touched
// from `&mut self` methods of PulseAudioDevices (single thread at a
// time by construction — every method takes exclusive access), and
// Watch's Drop performs the teardown in an order that keeps every
// pointer valid until after its last use. Moving the value between
// threads carries no pinned or thread-affine state of its own.
unsafe impl Send for Watch {}

/// Shared state of a persistent watch.
struct WatchState {
    queue: Arc<DeviceEventQueue>,
    /// Sink/card index → stable id, from the last enumeration
    /// (indexes are session-scoped; ids are name-stable).
    index_ids: HashMap<u32, DeviceId>,
}

/// Stack state for one connect phase.
struct ConnState {
    api: Arc<PulseApi>,
    ready: bool,
    failed: bool,
}

/// Stack state for one enumeration pass.
struct EnumState {
    devices: Vec<DeviceDesc>,
    index_ids: HashMap<u32, DeviceId>,
    done: bool,
}

// ---------------------------------------------------------------------------
// C string helpers
// ---------------------------------------------------------------------------

/// Copy a library-owned C string into an owned `String` (lossy —
/// PulseAudio strings are documented UTF-8; lossy keeps a bad server
/// from failing the whole enumeration).
fn owned(raw: *const c_char) -> String {
    if raw.is_null() {
        return String::new();
    }
    // SAFETY: the pointer comes from PulseAudio and points at a
    // NUL-terminated array valid for the duration of the callback
    // that handed it to us; we copy before returning.
    unsafe { CStr::from_ptr(raw).to_string_lossy().into_owned() }
}

/// The library's error text for a context error code (static
/// storage, copied out).
fn strerror(api: &PulseApi, code: c_int) -> String {
    // SAFETY: pa_strerror returns a pointer into static storage for
    // any int; NULL is not documented but guarded anyway.
    unsafe {
        let raw = (api.strerror)(code);
        if raw.is_null() {
            format!("pa error {code}")
        } else {
            CStr::from_ptr(raw).to_string_lossy().into_owned()
        }
    }
}

// ---------------------------------------------------------------------------
// Callbacks (fire only inside our own dispatch calls — module docs)
// ---------------------------------------------------------------------------

/// Context-state callback (connect phase).
unsafe extern "C" fn on_context_state(ctx: *mut PaContext, userdata: *mut c_void) {
    // SAFETY: userdata points at a live ConnState on the caller's
    // stack for the whole duration of the connect loop that owns this
    // registration (cleared before that scope returns).
    let state = unsafe { &mut *(userdata as *mut ConnState) };
    let api = &state.api;
    let current = unsafe { (api.context_get_state)(ctx) };
    if current == PA_CONTEXT_READY {
        state.ready = true;
    } else if current == PA_CONTEXT_FAILED {
        state.failed = true;
    }
}

/// Sink-list callback (enumeration).
unsafe extern "C" fn on_sink_info(
    _ctx: *mut PaContext,
    info: *const PaSinkInfo,
    eol: c_int,
    userdata: *mut c_void,
) {
    // SAFETY: userdata points at a live EnumState on the caller's
    // stack; the owning operation is completed or cancelled before
    // that scope returns (see the cancel paths in list_sinks).
    let state = unsafe { &mut *(userdata as *mut EnumState) };
    if eol != 0 {
        state.done = true;
        return;
    }
    if info.is_null() {
        return;
    }
    // SAFETY: info points at a library-owned pa_sink_info valid for
    // this callback invocation; we copy out everything we keep.
    let sink = unsafe { &*info };
    let name = owned(sink.name);
    if name.is_empty() {
        return; // defensive: a nameless sink cannot be addressed
    }
    let display = {
        let description = owned(sink.description);
        if description.is_empty() {
            name.clone()
        } else {
            description
        }
    };
    let id = stable_device_id(&name);
    state.devices.push(DeviceDesc {
        id,
        name: display,
        preferred: preferred_format(&sink.sample_spec),
    });
    state.index_ids.insert(sink.index, id);
}

/// Subscription callback (persistent watch).
unsafe extern "C" fn on_subscription_event(
    _ctx: *mut PaContext,
    event_type: u32,
    idx: u32,
    userdata: *mut c_void,
) {
    // SAFETY: userdata points at the Box'd WatchState owned by the
    // Watch whose context registered this callback; Watch's drop
    // order destroys the context before the state frees.
    let state = unsafe { &*(userdata as *const WatchState) };
    let facility = event_type & FACILITY_MASK;
    let kind = event_type & TYPE_MASK;
    if facility == FACILITY_SERVER {
        // Server-wide change (default device among them): advise a
        // re-enumeration rather than guessing a translation.
        state.queue.push(PlatformEvent::AudioSetChanged);
        return;
    }
    if facility == FACILITY_SINK || facility == FACILITY_CARD {
        let known = state.index_ids.get(&idx).copied();
        match (kind, known) {
            (TYPE_NEW, Some(id)) => {
                state.queue.push(PlatformEvent::AudioDeviceAdded { id });
            }
            (TYPE_REMOVE, Some(id)) => {
                state.queue.push(PlatformEvent::AudioDeviceRemoved { id });
            }
            _ => {
                // Change events, or an index not yet mapped: the
                // honest translation is "re-enumerate".
                state.queue.push(PlatformEvent::AudioSetChanged);
            }
        }
    }
    // Other facilities (sink inputs, clients, …) are not output
    // devices — ignored by design.
}

/// No-op success callback for `pa_context_subscribe`.
unsafe extern "C" fn on_subscribe_done(
    _ctx: *mut PaContext,
    _success: c_int,
    _userdata: *mut c_void,
) {
}

// ---------------------------------------------------------------------------
// Format mapping
// ---------------------------------------------------------------------------

/// Map a sink's `pa_sample_spec` to the engine's [`AudioFormat`]
/// suggestion (little-endian native formats — the linux targets are
/// LE; exotic formats fall back to the f32 default, documented,
/// because `preferred` is a suggestion, not a contract — doc 03
/// §4.9).
fn preferred_format(spec: &PaSampleSpec) -> AudioFormat {
    let sample = match spec.format {
        0 => SampleFormat::U8,  // PA_SAMPLE_U8
        3 => SampleFormat::S16, // PA_SAMPLE_S16LE
        5 => SampleFormat::F32, // PA_SAMPLE_FLOAT32LE
        7 => SampleFormat::S32, // PA_SAMPLE_S32LE
        9 => SampleFormat::S24, // PA_SAMPLE_S24LE
        _ => SampleFormat::F32, // documented approximation (BE/alaw/…)
    };
    AudioFormat {
        sample,
        channels: if spec.channels == 0 {
            2
        } else {
            u32::from(spec.channels)
        },
        layout: AudioLayout::Interleaved,
        rate: if spec.rate == 0 { 48_000 } else { spec.rate },
    }
}

// ---------------------------------------------------------------------------
// Mainloop pumping
// ---------------------------------------------------------------------------

/// One bounded prepare/poll/dispatch cycle. Returns the number of
/// events dispatched (0 = the slice elapsed with nothing ready).
fn pump_once(api: &PulseApi, ml: *mut PaMainloop, timeout_ms: c_int) -> Result<u32, PlatformError> {
    // SAFETY: ml is a live mainloop owned by the caller for the
    // duration of this call; the three calls run in their documented
    // sequence (prepare → poll → dispatch).
    unsafe {
        if (api.mainloop_prepare)(ml, timeout_ms) < 0 {
            return Err(PlatformError::CallFailed {
                api: "pa_mainloop_prepare",
                code: None,
                detail: "mainloop refused the prepare step".to_string(),
            });
        }
        let ready = (api.mainloop_poll)(ml);
        if ready < 0 {
            return Err(PlatformError::CallFailed {
                api: "pa_mainloop_poll",
                code: None,
                detail: "poll failed on the PulseAudio mainloop".to_string(),
            });
        }
        if ready == 0 {
            return Ok(0); // slice elapsed, nothing ready
        }
        let dispatched = (api.mainloop_dispatch)(ml);
        if dispatched < 0 {
            return Err(PlatformError::CallFailed {
                api: "pa_mainloop_dispatch",
                code: None,
                detail: "dispatch failed on the PulseAudio mainloop".to_string(),
            });
        }
        Ok(dispatched.max(0) as u32)
    }
}

impl PulseAudioDevices {
    /// Load and resolve the client library.
    ///
    /// # Errors
    /// [`PlatformError::LibraryUnavailable`] (no SONAME loadable) or
    /// [`PlatformError::SymbolMissing`] (version skew).
    pub fn probe() -> Result<PulseAudioDevices, PlatformError> {
        let lib = DynamicLibrary::open(&PULSE_SONAMES)?;
        let api = Arc::new(crate::linux::pulse_ffi::load(&lib)?);
        Ok(PulseAudioDevices {
            watch: None,
            api,
            lib,
        })
    }

    /// The SONAME the client was loaded from (capability-report
    /// vocabulary).
    #[must_use = "pure accessor"]
    pub fn loaded_from(&self) -> &str {
        self.lib.name()
    }

    /// Drive the connect phase on `ml` until READY/FAILED or the
    /// deadline. On success the context is returned with the state
    /// callback already cleared (no dangling userdata); on failure
    /// the context **and** `ml` are torn down.
    fn connect(&self, ml: *mut PaMainloop) -> Result<*mut PaContext, PlatformError> {
        // SAFETY: ml is a live mainloop owned by the caller; the API
        // token it returns belongs to that mainloop.
        let api_ptr = unsafe { (self.api.mainloop_get_api)(ml) };
        if api_ptr.is_null() {
            // Ownership: this method owns `ml` for its whole body —
            // every error path frees it exactly once (the callers
            // must not free on Err).
            // SAFETY: ml is the live mainloop the caller created.
            unsafe {
                (self.api.mainloop_free)(ml);
            }
            return Err(PlatformError::CallFailed {
                api: "pa_mainloop_get_api",
                code: None,
                detail: "mainloop API token was NULL".to_string(),
            });
        }
        // lumen-allow-literal: the PulseAudio client name is the stable
        // protocol identity the sound server shows for this connection —
        // product-config owns it once the platform crates gain their
        // config edge (Lead decision, recorded in the 7.A2 report).
        let name = CString::new("lumen").map_err(|_| PlatformError::CallFailed {
            api: "pa_context_new",
            code: None,
            detail: "client name contained an interior NUL".to_string(),
        })?;
        // SAFETY: api_ptr and name are valid for the call; the
        // returned context is checked for NULL.
        let ctx = unsafe { (self.api.context_new)(api_ptr, name.as_ptr()) };
        if ctx.is_null() {
            // SAFETY: ml is the live mainloop the caller created;
            // this error path owns its single free (see above).
            unsafe {
                (self.api.mainloop_free)(ml);
            }
            return Err(PlatformError::CallFailed {
                api: "pa_context_new",
                code: None,
                detail: "the library refused to allocate a context".to_string(),
            });
        }
        let mut conn = ConnState {
            api: Arc::clone(&self.api),
            ready: false,
            failed: false,
        };
        // SAFETY: ctx is live; the callback + userdata are registered
        // for the duration of this function and cleared below before
        // `conn` (a stack local) goes out of scope.
        unsafe {
            (self.api.context_set_state_callback)(
                ctx,
                Some(on_context_state),
                std::ptr::addr_of_mut!(conn).cast::<c_void>(),
            );
        }
        // SAFETY: NULL server = the library's default (env/socket);
        // flags 0 = PA_CONTEXT_NOFLAGS; NULL spawn API is documented.
        let connected =
            unsafe { (self.api.context_connect)(ctx, std::ptr::null(), 0, std::ptr::null()) };
        if connected < 0 {
            let code = unsafe { (self.api.context_errno)(ctx) };
            let detail = strerror(&self.api, code);
            self.teardown_context(ctx, ml, true);
            return Err(PlatformError::ServiceUnavailable {
                service: "PulseAudio server",
                detail,
            });
        }
        let deadline = Instant::now() + CONNECT_DEADLINE;
        let mut failure: Option<PlatformError> = None;
        // `conn.ready`/`conn.failed` flip inside the callbacks the
        // dispatched mainloop invokes (through the userdata pointer
        // below) — invisible to flow analysis, so the loop is a
        // plain `loop` with an explicit guard.
        loop {
            if conn.ready || conn.failed {
                break;
            }
            if Instant::now() >= deadline {
                failure = Some(PlatformError::TimedOut {
                    api: "pa_context_connect",
                    deadline_ms: CONNECT_DEADLINE.as_millis() as u64,
                });
                break;
            }
            pump_once(&self.api, ml, PUMP_SLICE)?;
        }
        // Clear the stack-pointing registration in every outcome
        // before `conn` dies.
        // SAFETY: clearing a callback is always valid on a live ctx.
        unsafe {
            (self.api.context_set_state_callback)(ctx, None, std::ptr::null_mut());
        }
        if let Some(err) = failure {
            self.teardown_context(ctx, ml, true);
            return Err(err);
        }
        if conn.failed {
            let code = unsafe { (self.api.context_errno)(ctx) };
            self.teardown_context(ctx, ml, true);
            return Err(PlatformError::ServiceUnavailable {
                service: "PulseAudio server",
                detail: strerror(&self.api, code),
            });
        }
        Ok(ctx)
    }

    /// Disconnect + unref a context; optionally free its mainloop.
    /// No callback of ours is registered when this runs (callers
    /// clear them first) — the contexts here carry none past their
    /// connect phase or their Watch teardown.
    fn teardown_context(&self, ctx: *mut PaContext, ml: *mut PaMainloop, free_mainloop: bool) {
        // SAFETY: ctx/ml are a live created-against-each-other pair
        // (caller invariant at every call site);
        // disconnect-then-unref-then-free is the documented order.
        unsafe {
            (self.api.context_disconnect)(ctx);
            (self.api.context_unref)(ctx);
            if free_mainloop {
                (self.api.mainloop_free)(ml);
            }
        }
    }

    /// Run one `get_sink_info_list` round on a READY `ctx`.
    fn list_sinks(
        &self,
        ctx: *mut PaContext,
        ml: *mut PaMainloop,
    ) -> Result<EnumState, PlatformError> {
        let mut state = EnumState {
            devices: Vec::new(),
            index_ids: HashMap::new(),
            done: false,
        };
        // SAFETY: ctx is READY; the callback userdata points at the
        // stack `state`, and the operation is completed or cancelled
        // before this function returns in every path below.
        let op = unsafe {
            (self.api.context_get_sink_info_list)(
                ctx,
                Some(on_sink_info),
                std::ptr::addr_of_mut!(state).cast::<c_void>(),
            )
        };
        if op.is_null() {
            return Err(PlatformError::CallFailed {
                api: "pa_context_get_sink_info_list",
                code: None,
                detail: "the library refused the enumeration operation".to_string(),
            });
        }
        let deadline = Instant::now() + ENUM_DEADLINE;
        let mut failure: Option<PlatformError> = None;
        // `state.done` flips inside the sink callback the dispatched
        // mainloop invokes (through the userdata pointer) — the same
        // explicit-guard shape as the connect loop above.
        loop {
            if state.done {
                break;
            }
            if Instant::now() >= deadline {
                failure = Some(PlatformError::TimedOut {
                    api: "pa_context_get_sink_info_list",
                    deadline_ms: ENUM_DEADLINE.as_millis() as u64,
                });
                break;
            }
            // A mid-enumeration context failure ends the wait with a
            // typed error rather than spinning to the deadline.
            let context_state = unsafe { (self.api.context_get_state)(ctx) };
            if context_state == PA_CONTEXT_FAILED {
                let code = unsafe { (self.api.context_errno)(ctx) };
                failure = Some(PlatformError::ServiceUnavailable {
                    service: "PulseAudio server",
                    detail: format!(
                        "connection failed mid-enumeration: {}",
                        strerror(&self.api, code)
                    ),
                });
                break;
            }
            pump_once(&self.api, ml, PUMP_SLICE)?;
        }
        if failure.is_some() {
            // Cancel so the callback can never fire against the
            // stack state after we return (a cancelled operation does
            // not invoke its callback — pulse operation semantics),
            // then release the reference.
            // SAFETY: op is the live operation ref returned above.
            unsafe {
                (self.api.operation_cancel)(op);
                (self.api.operation_unref)(op);
            }
            return Err(failure.unwrap_or_else(|| PlatformError::CallFailed {
                api: "pa_context_get_sink_info_list",
                code: None,
                detail: "enumeration failed".to_string(),
            }));
        }
        // SAFETY: the operation completed (the eol callback ran);
        // unreffing releases the library's reference.
        unsafe {
            (self.api.operation_unref)(op);
        }
        Ok(state)
    }
}

impl AudioDevices for PulseAudioDevices {
    fn enumerate(&mut self) -> Result<Vec<DeviceDesc>, PlatformError> {
        // With a live watch, reuse its connection (and refresh the
        // index→id cache the subscription translates through).
        if let Some((ctx, ml)) = self.watch.as_ref().map(|watch| (watch.ctx, watch.ml)) {
            let state = self.list_sinks(ctx, ml)?;
            if let Some(watch) = self.watch.as_mut() {
                watch.state.index_ids = state.index_ids;
            }
            return Ok(state.devices);
        }
        // No watch: a short-lived mainloop+context for this call.
        // SAFETY: pairs with the teardown in every path below.
        let ml = unsafe { (self.api.mainloop_new)() };
        if ml.is_null() {
            return Err(PlatformError::CallFailed {
                api: "pa_mainloop_new",
                code: None,
                detail: "the library refused to allocate a mainloop".to_string(),
            });
        }
        let ctx = match self.connect(ml) {
            Ok(ctx) => ctx,
            Err(err) => {
                // connect() freed the context AND the mainloop on
                // every error path (its ownership contract) — nothing
                // left to tear down here.
                return Err(err);
            }
        };
        let state = self.list_sinks(ctx, ml);
        self.teardown_context(ctx, ml, true);
        state.map(|s| s.devices)
    }

    fn subscribe(&mut self, queue: Arc<DeviceEventQueue>) -> Result<(), PlatformError> {
        // A second subscribe replaces the first watch (the old queue
        // stops receiving events — Watch::drop unregisters).
        self.watch = None;
        // SAFETY: pairs with the Watch's drop teardown.
        let ml = unsafe { (self.api.mainloop_new)() };
        if ml.is_null() {
            return Err(PlatformError::CallFailed {
                api: "pa_mainloop_new",
                code: None,
                detail: "the library refused to allocate a mainloop".to_string(),
            });
        }
        let ctx = match self.connect(ml) {
            Ok(ctx) => ctx,
            Err(err) => {
                // connect() freed the context AND the mainloop on
                // every error path (its ownership contract) — nothing
                // left to tear down here.
                return Err(err);
            }
        };
        // Prime the index→id cache through the same connection; on
        // failure the connection is torn down before returning.
        let primed = match self.list_sinks(ctx, ml) {
            Ok(state) => state,
            Err(err) => {
                self.teardown_context(ctx, ml, true);
                return Err(err);
            }
        };
        let mut state = Box::new(WatchState {
            queue,
            index_ids: primed.index_ids,
        });
        // SAFETY: ctx is READY; the userdata points at the heap
        // WatchState boxed above, which the Watch returned at the
        // end keeps alive past the context itself (field drop order).
        unsafe {
            (self.api.context_set_subscribe_callback)(
                ctx,
                Some(on_subscription_event),
                std::ptr::addr_of_mut!(*state).cast::<c_void>(),
            );
        }
        // SAFETY: the mask covers sinks/cards/server; the no-op
        // success callback carries no userdata.
        let op = unsafe {
            (self.api.context_subscribe)(
                ctx,
                PA_SUBSCRIPTION_MASK,
                Some(on_subscribe_done),
                std::ptr::null_mut(),
            )
        };
        if op.is_null() {
            // No subscription happened; unregister before bailing.
            // SAFETY: ctx is live.
            unsafe {
                (self.api.context_set_subscribe_callback)(ctx, None, std::ptr::null_mut());
            }
            self.teardown_context(ctx, ml, true);
            return Err(PlatformError::CallFailed {
                api: "pa_context_subscribe",
                code: None,
                detail: "the library refused the subscription".to_string(),
            });
        }
        // Wait for the subscription acknowledgement (bounded).
        let deadline = Instant::now() + CONNECT_DEADLINE;
        let mut failure: Option<PlatformError> = None;
        loop {
            // SAFETY: op is the live operation ref returned above.
            let op_state = unsafe { (self.api.operation_get_state)(op) };
            if op_state != OPERATION_RUNNING {
                break;
            }
            if Instant::now() >= deadline {
                failure = Some(PlatformError::TimedOut {
                    api: "pa_context_subscribe",
                    deadline_ms: CONNECT_DEADLINE.as_millis() as u64,
                });
                break;
            }
            pump_once(&self.api, ml, PUMP_SLICE)?;
        }
        // SAFETY: completed or cancelled; releasing the reference is
        // valid in both outcomes.
        unsafe {
            if failure.is_some() {
                (self.api.operation_cancel)(op);
            }
            (self.api.operation_unref)(op);
        }
        if let Some(err) = failure {
            // SAFETY: ctx is live; clearing then tearing down.
            unsafe {
                (self.api.context_set_subscribe_callback)(ctx, None, std::ptr::null_mut());
            }
            self.teardown_context(ctx, ml, true);
            return Err(err);
        }
        self.watch = Some(Watch {
            api: Arc::clone(&self.api),
            ml,
            ctx,
            state,
        });
        Ok(())
    }

    fn poll(&mut self) -> Result<u32, PlatformError> {
        let Some(watch) = self.watch.as_ref() else {
            return Ok(0);
        };
        pump_once(&self.api, watch.ml, 0)
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        // SAFETY: api/ml/ctx are the live trio this Watch owns (the
        // api outlives the watch — owner field order); unregistering
        // the callback first removes the only userdata pointing at
        // `state`, and disconnect/unref/free is the documented
        // teardown order.
        unsafe {
            let api = &self.api;
            (api.context_set_subscribe_callback)(self.ctx, None, std::ptr::null_mut());
            (api.context_disconnect)(self.ctx);
            (api.context_unref)(self.ctx);
            (api.mainloop_free)(self.ml);
        }
        // `state` (the Box) drops now, after the context that
        // referenced it is gone.
    }
}
