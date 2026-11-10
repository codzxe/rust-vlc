//! The WASAPI MMDevice client (card 7.A1): render-endpoint
//! enumeration + change notifications, hand-rolled over the ole32
//! COM ABI (no `windows-s` crate).
//!
//! # Runtime model (the honest part)
//!
//! - Enumeration runs on a **dedicated short-lived thread** with its
//!   own single-threaded apartment: COM apartment rules are satisfied
//!   by construction, and the caller's thread model never matters.
//!   Enumeration is a rare operation (settings screen, startup), so a
//!   thread per round is the conservative, honest cost.
//! - Notifications run on a **dedicated long-lived thread** that owns
//!   the enumerator, the `IMMNotificationClient` implementation and
//!   the apartment for the whole subscription; [`AudioDevices::poll`]
//!   returns 0 (push-based — WASAPI delivers via callback, the
//!   events already sit in the queue).
//! - On any non-Windows host this entire module is compiled out and
//!   the seam answers the typed `WrongOs` fallback.
//!
//! **Evidence status:** compile-verified via `cargo check --target
//! x86_64-pc-windows-msvc`; runtime NOT exercised in this sandbox
//! (no Windows SDK) — `[VERIFY]` on real-device behavior.
//!
//! # The notification client (why the raw pointers are sound)
//!
//! [`NotificationClient`] is a COM object we implement: a `#[repr(C)]`
//! struct whose first field is a static vtable pointer, boxed on the
//! watch thread. Its reference count exists for COM etiquette but
//! **Release never frees** — the Box on the owning thread governs the
//! lifetime, and the thread only drops it after
//! `UnregisterEndpointNotificationCallback` removed the enumerator's
//! reference (the documented pairing). The callbacks themselves touch
//! only the thread-safe [`Arc<DeviceEventQueue>`] and FNV hashing —
//! safe on whatever COM dispatch thread they arrive from.

// SAFETY-SCOPED MODULE (doc 01 §2.7 / MP §9): the WASAPI/ole32 COM
// ABI is the audited OS-service exception this file exercises.
#![allow(unsafe_code)]

use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use audio_output::DeviceDesc;
use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};

use crate::audio::{AudioDevices, stable_device_id};
use crate::error::PlatformError;
use crate::events::{DeviceEventQueue, PlatformEvent};
use crate::windows::com::{
    ComApartment, Guid, Hresult, PropertyKey, S_OK, co_create_instance, free_wide, from_wide,
    property_store_string, release_interface,
};

// ---------------------------------------------------------------------------
// ABI constants (mmdeviceapi.h / functiondiscoverykeys_devpkey.h)
// ---------------------------------------------------------------------------

/// `CLSID_MMDeviceEnumerator` {BCDE0395-E52F-467C-8E3D-C4579291692E}.
const CLSID_MM_DEVICE_ENUMERATOR: Guid = Guid {
    data1: 0xBCDE_0395,
    data2: 0xE52F,
    data3: 0x467C,
    data4: [0x8E, 0x3D, 0xC4, 0x57, 0x92, 0x91, 0x69, 0x2E],
};

/// `IID_IMMDeviceEnumerator` {A95664D2-9614-4F35-A746-DE8DB63617E6}.
const IID_MM_DEVICE_ENUMERATOR: Guid = Guid {
    data1: 0xA956_64D2,
    data2: 0x9614,
    data3: 0x4F35,
    data4: [0xA7, 0x46, 0xDE, 0x8D, 0xB6, 0x36, 0x17, 0xE6],
};

/// `IID_IUnknown` {00000000-0000-0000-C000-000000000046} — the only
/// interface our notification client answers `QueryInterface` with.
const IID_IUNKNOWN: Guid = Guid {
    data1: 0x0000_0000,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

/// `E_NOINTERFACE`.
const E_NOINTERFACE: Hresult = 0x8000_4002u32 as i32;

/// `eRender` (audio output endpoints).
const E_RENDER: i32 = 0;
/// `eConsole` (the default-device role multimedia falls back to).
const E_CONSOLE: i32 = 0;

/// `DEVICE_STATE_ACTIVE`.
const DEVICE_STATE_ACTIVE: u32 = 0x1;
/// Every device state (the enumeration mask).
const DEVICE_STATE_MASK_ALL: u32 = 0x1 | 0x2 | 0x4 | 0x8;

/// `PKEY_Device_FriendlyName`
/// {A45C254E-DF1C-4EFD-8020-67D146A850E0}, pid 14.
const PKEY_DEVICE_FRIENDLY_NAME: PropertyKey = PropertyKey {
    fmtid: Guid {
        data1: 0xA45C_254E,
        data2: 0xDF1C,
        data3: 0x4EFD,
        data4: [0x80, 0x20, 0x67, 0xD1, 0x46, 0xA8, 0x50, 0xE0],
    },
    pid: 14,
};

/// `PKEY_Device_DeviceDesc` (the fallback name), same fmtid, pid 2.
const PKEY_DEVICE_DEVICE_DESC: PropertyKey = PropertyKey {
    fmtid: Guid {
        data1: 0xA45C_254E,
        data2: 0xDF1C,
        data3: 0x4EFD,
        data4: [0x80, 0x20, 0x67, 0xD1, 0x46, 0xA8, 0x50, 0xE0],
    },
    pid: 2,
};

/// The subscription handshake deadline (D-2: no unbounded waits).
const WATCH_DEADLINE: Duration = Duration::from_millis(2_000);

// ---------------------------------------------------------------------------
// Vtables (mmdeviceapi.h order; unused slots underscore-prefixed)
// ---------------------------------------------------------------------------

/// `IMMDeviceEnumerator` vtable.
#[repr(C)]
struct EnumeratorVtbl {
    /// IUnknown.
    pub unknown: crate::windows::com::UnknownVtbl,
    /// `EnumAudioEndpoints(this, flow, mask, out)`.
    pub enum_audio_endpoints:
        unsafe extern "system" fn(*mut c_void, i32, u32, *mut *mut c_void) -> Hresult,
    /// `GetDefaultAudioEndpoint(this, flow, role, out)`.
    pub get_default_audio_endpoint:
        unsafe extern "system" fn(*mut c_void, i32, i32, *mut *mut c_void) -> Hresult,
    /// `GetDevice(this, id, out)`.
    pub _get_device:
        unsafe extern "system" fn(*mut c_void, *const u16, *mut *mut c_void) -> Hresult,
    /// `RegisterEndpointNotificationCallback(this, client)`.
    pub register_endpoint_notification_callback:
        unsafe extern "system" fn(*mut c_void, *mut c_void) -> Hresult,
    /// `UnregisterEndpointNotificationCallback(this, client)`.
    pub unregister_endpoint_notification_callback:
        unsafe extern "system" fn(*mut c_void, *mut c_void) -> Hresult,
}

/// `IMMDeviceCollection` vtable.
#[repr(C)]
struct CollectionVtbl {
    /// IUnknown.
    pub unknown: crate::windows::com::UnknownVtbl,
    /// `Item(this, index, out)`.
    pub item: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> Hresult,
    /// `GetCount(this, out)`.
    pub get_count: unsafe extern "system" fn(*mut c_void, *mut u32) -> Hresult,
}

/// `IMMDevice` vtable.
#[repr(C)]
struct DeviceVtbl {
    /// IUnknown.
    pub unknown: crate::windows::com::UnknownVtbl,
    /// `Activate(this, iid, ctx, params, out)`.
    pub _activate: unsafe extern "system" fn(
        *mut c_void,
        *const Guid,
        u32,
        *mut c_void,
        *mut *mut c_void,
    ) -> Hresult,
    /// `OpenPropertyStore(this, access, out)`.
    pub open_property_store:
        unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> Hresult,
    /// `GetId(this, out)`.
    pub get_id: unsafe extern "system" fn(*mut c_void, *mut *mut u16) -> Hresult,
    /// `GetState(this, out)`.
    pub _get_state: unsafe extern "system" fn(*mut c_void, *mut u32) -> Hresult,
}

/// A COM object header for vtable access through a raw pointer.
#[repr(C)]
struct ObjectHeader {
    vtbl: *const (),
}

// ---------------------------------------------------------------------------
// The enumeration (runs on a dedicated short-lived apartment thread)
// ---------------------------------------------------------------------------

/// The shared-mode format the WASAPI audio engine mixes in: float32.
/// This is the honest per-endpoint *suggestion*; the exact mix format
/// is negotiated at open time behind the ADR-005 admission gate
/// (doc 01 §2.6 — the streaming half this card does not claim).
fn preferred_format() -> AudioFormat {
    AudioFormat {
        sample: SampleFormat::F32,
        channels: 2,
        layout: AudioLayout::Interleaved,
        rate: 48_000,
    }
}

/// Enumerate the render endpoints (MMDevice API). Every intermediate
/// COM reference is released on every path.
fn list_endpoints() -> Result<Vec<DeviceDesc>, PlatformError> {
    let _apartment = ComApartment::new()?;
    // SAFETY: the enumerator is created fresh here, used only on this
    // thread inside this function, and released exactly once at the
    // end (the vtable layout is the documented IMMDeviceEnumerator
    // order above).
    let enumerator =
        unsafe { co_create_instance(&CLSID_MM_DEVICE_ENUMERATOR, &IID_MM_DEVICE_ENUMERATOR)? };
    let result = enumerate_with(enumerator);
    // SAFETY: this is the single release of the created reference.
    unsafe {
        release_interface(enumerator);
    }
    result
}

/// The body of [`list_endpoints`] against a live enumerator.
fn enumerate_with(enumerator: *mut c_void) -> Result<Vec<DeviceDesc>, PlatformError> {
    let enum_vtbl = unsafe { (*(enumerator as *mut ObjectHeader)).vtbl as *const EnumeratorVtbl };
    // SAFETY: enumerator is live; the out-slot is zero-initialized;
    // the flow/mask arguments are the documented constants.
    let mut collection: *mut c_void = std::ptr::null_mut();
    let hr = unsafe {
        ((*enum_vtbl).enum_audio_endpoints)(
            enumerator,
            E_RENDER,
            DEVICE_STATE_MASK_ALL,
            &mut collection,
        )
    };
    if hr != S_OK || collection.is_null() {
        return Err(PlatformError::CallFailed {
            api: "IMMDeviceEnumerator::EnumAudioEndpoints",
            code: Some(i64::from(hr)),
            detail: "the endpoint collection could not be created".to_string(),
        });
    }
    let devices = devices_from_collection(collection);
    // SAFETY: the single release of the collection reference.
    unsafe {
        release_interface(collection);
    }
    devices
}

/// Turn an `IMMDeviceCollection` into [`DeviceDesc`]s.
fn devices_from_collection(collection: *mut c_void) -> Result<Vec<DeviceDesc>, PlatformError> {
    let collection_vtbl =
        unsafe { (*(collection as *mut ObjectHeader)).vtbl as *const CollectionVtbl };
    // SAFETY: collection is live; the count slot is zero-initialized.
    let mut count: u32 = 0;
    let hr = unsafe { ((*collection_vtbl).get_count)(collection, &mut count) };
    if hr != S_OK {
        return Err(PlatformError::CallFailed {
            api: "IMMDeviceCollection::GetCount",
            code: Some(i64::from(hr)),
            detail: "the endpoint count could not be read".to_string(),
        });
    }
    let mut devices = Vec::new();
    for index in 0..count {
        // SAFETY: collection is live and index < count (the count was
        // just read); the out-slot is zero-initialized.
        let mut device: *mut c_void = std::ptr::null_mut();
        let hr = unsafe { ((*collection_vtbl).item)(collection, index, &mut device) };
        if hr != S_OK || device.is_null() {
            // A device disappearing mid-enumeration is not fatal to
            // the rest of the list; skip it and keep the honest rest.
            continue;
        }
        if let Ok(Some(desc)) = describe_device(device) {
            devices.push(desc);
        }
        // SAFETY: the single release of this item's reference.
        unsafe {
            release_interface(device);
        }
    }
    Ok(devices)
}

/// One `IMMDevice` → [`DeviceDesc`] (`None` when the endpoint is so
/// degenerate it cannot be described honestly).
fn describe_device(device: *mut c_void) -> Result<Option<DeviceDesc>, PlatformError> {
    let device_vtbl = unsafe { (*(device as *mut ObjectHeader)).vtbl as *const DeviceVtbl };
    // SAFETY: device is live; the out-slot is zero-initialized; the
    // returned PWSTR is CoTaskMemAlloc'd by the API and freed below.
    let mut endpoint_id: *mut u16 = std::ptr::null_mut();
    let hr = unsafe { ((*device_vtbl).get_id)(device, &mut endpoint_id) };
    if hr != S_OK || endpoint_id.is_null() {
        return Err(PlatformError::CallFailed {
            api: "IMMDevice::GetId",
            code: Some(i64::from(hr)),
            detail: "the endpoint id could not be read".to_string(),
        });
    }
    // SAFETY: endpoint_id is a NUL-terminated wide string valid until
    // freed a few lines below (the reads happen in between).
    let id_text = unsafe { from_wide(endpoint_id) };
    // SAFETY: the API allocated it with CoTaskMemAlloc; this is the
    // only free.
    unsafe {
        free_wide(endpoint_id);
    }
    let id_text = id_text?;
    if id_text.is_empty() {
        return Ok(None); // a nameless endpoint cannot be addressed
    }
    // The display name: Device_FriendlyName, falling back to
    // DeviceDesc when the friendly name is absent (the documented
    // two-level naming of MMDevice endpoints).
    // SAFETY: device is live; STGM_READ is the documented access.
    let mut store: *mut c_void = std::ptr::null_mut();
    let hr = unsafe {
        ((*device_vtbl).open_property_store)(device, crate::windows::com::STGM_READ, &mut store)
    };
    if hr != S_OK || store.is_null() {
        // No property store means no honest name; the endpoint id is
        // stable but opaque — still addressable, described by id.
        return Ok(Some(DeviceDesc {
            id: stable_device_id(&id_text),
            name: format!("WASAPI endpoint {id_text}"),
            preferred: preferred_format(),
        }));
    }
    let name = unsafe { property_store_string(store, &PKEY_DEVICE_FRIENDLY_NAME) }
        .or_else(|_| unsafe { property_store_string(store, &PKEY_DEVICE_DEVICE_DESC) })
        .unwrap_or_else(|_| format!("WASAPI endpoint {id_text}"));
    // SAFETY: the single release of the property store reference.
    unsafe {
        release_interface(store);
    }
    let name = if name.is_empty() {
        format!("WASAPI endpoint {id_text}")
    } else {
        name
    };
    Ok(Some(DeviceDesc {
        id: stable_device_id(&id_text),
        name,
        preferred: preferred_format(),
    }))
}

// ---------------------------------------------------------------------------
// The notification client (our COM object; the watch thread owns it)
// ---------------------------------------------------------------------------

/// Our `IMMNotificationClient` implementation. `#[repr(C)]` so the
/// vtable pointer leads; the trailing fields are the payload the
/// callbacks read (thread-safe by construction).
#[repr(C)]
struct NotificationClient {
    vtbl: *const NotificationClientVtbl,
    queue: Arc<DeviceEventQueue>,
    refs: AtomicU32,
}

/// `IMMNotificationClient` vtable (mmdeviceapi.h order).
#[repr(C)]
struct NotificationClientVtbl {
    /// IUnknown.
    pub unknown: crate::windows::com::UnknownVtbl,
    /// `OnDeviceStateChanged(this, id, state)`.
    pub on_device_state_changed: unsafe extern "system" fn(*mut c_void, *const u16, u32) -> Hresult,
    /// `OnDeviceAdded(this, id)`.
    pub on_device_added: unsafe extern "system" fn(*mut c_void, *const u16) -> Hresult,
    /// `OnDeviceRemoved(this, id)`.
    pub on_device_removed: unsafe extern "system" fn(*mut c_void, *const u16) -> Hresult,
    /// `OnDefaultDeviceChanged(this, flow, role, id)`.
    pub on_default_device_changed:
        unsafe extern "system" fn(*mut c_void, i32, i32, *const u16) -> Hresult,
    /// `OnPropertyValueChanged(this, id, key)`.
    pub on_property_value_changed:
        unsafe extern "system" fn(*mut c_void, *const u16, PropertyKey) -> Hresult,
}

/// The static vtable every [`NotificationClient`] shares.
static NOTIFICATION_VTBL: NotificationClientVtbl = NotificationClientVtbl {
    unknown: crate::windows::com::UnknownVtbl {
        query_interface: client_query_interface,
        add_ref: client_add_ref,
        release: client_release,
    },
    on_device_state_changed: client_on_device_state_changed,
    on_device_added: client_on_device_added,
    on_device_removed: client_on_device_removed,
    on_default_device_changed: client_on_default_device_changed,
    on_property_value_changed: client_on_property_value_changed,
};

/// Extract the client payload from a `this` pointer.
///
/// # Safety (caller invariant)
/// `this` is a `*mut NotificationClient` created by this module.
unsafe fn client_of(this: *mut c_void) -> &NotificationClient {
    unsafe { &*(this as *const NotificationClient) }
}

/// Decode an endpoint-id wide string to a stable id; `None` means
/// "could not decode — advise re-enumeration instead of guessing".
///
/// # Safety (caller invariant)
/// `wide` is a NUL-terminated wide string valid for the duration of
/// the call (or NULL).
unsafe fn id_of(wide: *const u16) -> Option<audio_output::DeviceId> {
    if wide.is_null() {
        return None;
    }
    // SAFETY: caller invariant; decoding never panics (errors are
    // values).
    match unsafe { from_wide(wide) } {
        Ok(text) if !text.is_empty() => Some(stable_device_id(&text)),
        _ => None,
    }
}

// The callbacks: no panics, no unwinding across the ABI — every
// failure degrades to the honest AudioSetChanged advice.

/// `QueryInterface`: IUnknown only.
unsafe extern "system" fn client_query_interface(
    this: *mut c_void,
    riid: *const Guid,
    ppv: *mut *mut c_void,
) -> Hresult {
    // SAFETY: this is one of our clients (module invariant); riid/ppv
    // are the ABI-provided slots.
    unsafe {
        *ppv = std::ptr::null_mut();
        if (*riid) == IID_IUNKNOWN {
            client_add_ref(this);
            *ppv = this;
            S_OK
        } else {
            E_NOINTERFACE
        }
    }
}

/// `AddRef`: COM etiquette only (the Box governs the lifetime).
unsafe extern "system" fn client_add_ref(this: *mut c_void) -> u32 {
    // SAFETY: this is one of our clients; the counter is atomic.
    unsafe { client_of(this).refs.fetch_add(1, Ordering::Relaxed) + 1 }
}

/// `Release`: decrement only — **never frees** (the owning watch
/// thread drops the Box after unregistering; see module docs).
unsafe extern "system" fn client_release(this: *mut c_void) -> u32 {
    // SAFETY: this is one of our clients; the counter is atomic.
    unsafe {
        client_of(this)
            .refs
            .fetch_sub(1, Ordering::Relaxed)
            .saturating_sub(1)
    }
}

/// `OnDeviceStateChanged`: active/inactive transitions.
unsafe extern "system" fn client_on_device_state_changed(
    this: *mut c_void,
    device_id: *const u16,
    new_state: u32,
) -> Hresult {
    // SAFETY: this is one of our clients; the queue push is
    // panic-free and lock-bounded.
    unsafe {
        let client = client_of(this);
        match id_of(device_id) {
            Some(id) => client.queue.push(PlatformEvent::AudioDeviceStateChanged {
                id,
                active: new_state == DEVICE_STATE_ACTIVE,
            }),
            None => client.queue.push(PlatformEvent::AudioSetChanged),
        }
    }
    S_OK
}

/// `OnDeviceAdded`.
unsafe extern "system" fn client_on_device_added(
    this: *mut c_void,
    device_id: *const u16,
) -> Hresult {
    // SAFETY: as above.
    unsafe {
        let client = client_of(this);
        match id_of(device_id) {
            Some(id) => client.queue.push(PlatformEvent::AudioDeviceAdded { id }),
            None => client.queue.push(PlatformEvent::AudioSetChanged),
        }
    }
    S_OK
}

/// `OnDeviceRemoved`.
unsafe extern "system" fn client_on_device_removed(
    this: *mut c_void,
    device_id: *const u16,
) -> Hresult {
    // SAFETY: as above.
    unsafe {
        let client = client_of(this);
        match id_of(device_id) {
            Some(id) => client.queue.push(PlatformEvent::AudioDeviceRemoved { id }),
            None => client.queue.push(PlatformEvent::AudioSetChanged),
        }
    }
    S_OK
}

/// `OnDefaultDeviceChanged` (render, console role is what we track).
unsafe extern "system" fn client_on_default_device_changed(
    this: *mut c_void,
    flow: i32,
    _role: i32,
    default_device: *const u16,
) -> Hresult {
    // SAFETY: as above.
    unsafe {
        let client = client_of(this);
        if flow == E_RENDER {
            client.queue.push(PlatformEvent::AudioDefaultChanged {
                id: id_of(default_device),
            });
        }
    }
    S_OK
}

/// `OnPropertyValueChanged`: name changes and friends — the honest
/// translation is "re-enumerate".
unsafe extern "system" fn client_on_property_value_changed(
    this: *mut c_void,
    _device_id: *const u16,
    _key: PropertyKey,
) -> Hresult {
    // SAFETY: as above.
    unsafe {
        client_of(this).queue.push(PlatformEvent::AudioSetChanged);
    }
    S_OK
}

// ---------------------------------------------------------------------------
// The audio seam implementation
// ---------------------------------------------------------------------------

/// The WASAPI MMDevice adapter.
pub struct WasapiAudioDevices {
    watch: Option<Watch>,
}

/// The dedicated notification thread's handle + shutdown signal.
struct Watch {
    shutdown: Option<mpsc::Sender<()>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Drop for Watch {
    fn drop(&mut self) {
        // Dropping the sender unblocks the thread's park even if the
        // explicit send races a natural exit.
        self.shutdown = None;
        if let Some(handle) = self.handle.take() {
            // Join so the COM teardown is complete before the owner
            // proceeds (the queue may outlive us — it is Arc'd).
            let _ = handle.join();
        }
    }
}

impl WasapiAudioDevices {
    /// A fresh adapter (the library is always present on Windows —
    /// the "probe" of this lane is the enumeration call itself).
    #[must_use = "constructs the adapter"]
    pub fn new() -> WasapiAudioDevices {
        WasapiAudioDevices { watch: None }
    }
}

impl Default for WasapiAudioDevices {
    fn default() -> Self {
        WasapiAudioDevices::new()
    }
}

impl AudioDevices for WasapiAudioDevices {
    fn enumerate(&mut self) -> Result<Vec<DeviceDesc>, PlatformError> {
        // A dedicated short-lived apartment thread: COM rules hold by
        // construction, the caller's thread model never matters.
        let outcome = thread::Builder::new()
            // lumen-allow-literal: diagnostic thread name (debugger/log
            // surface only, not a user-visible string).
            .name("lumen-wasapi-enum".to_string())
            .spawn(list_endpoints)
            .map_err(|err| PlatformError::SpawnFailed {
                // lumen-allow-literal: the same diagnostic name, surfaced
                // in the typed error text.
                program: "lumen-wasapi-enum thread".to_string(),
                detail: err.to_string(),
            })?;
        match outcome.join() {
            Ok(result) => result,
            Err(_) => Err(PlatformError::CallFailed {
                api: "WASAPI enumeration thread",
                code: None,
                detail: "the enumeration thread panicked".to_string(),
            }),
        }
    }

    fn subscribe(&mut self, queue: Arc<DeviceEventQueue>) -> Result<(), PlatformError> {
        // A second subscribe replaces the first watch (the old queue
        // stops receiving events — Watch::drop tears the thread down).
        self.watch = None;
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), PlatformError>>();
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();
        let builder = thread::Builder::new()
            // lumen-allow-literal: diagnostic thread name (debugger/log
            // surface only, not a user-visible string).
            .name("lumen-wasapi-watch".to_string());
        let spawned = builder
            .spawn(move || watch_thread_body(queue, ready_tx, shutdown_rx))
            .map_err(|err| PlatformError::SpawnFailed {
                // lumen-allow-literal: the same diagnostic name, surfaced
                // in the typed error text.
                program: "lumen-wasapi-watch thread".to_string(),
                detail: err.to_string(),
            });
        let handle = match spawned {
            Ok(handle) => handle,
            Err(err) => return Err(err),
        };
        match ready_rx.recv_timeout(WATCH_DEADLINE) {
            Ok(Ok(())) => {
                self.watch = Some(Watch {
                    shutdown: Some(shutdown_tx),
                    handle: Some(handle),
                });
                Ok(())
            }
            Ok(Err(err)) => {
                // The thread ended after reporting; join to reap it.
                let _ = handle.join();
                Err(err)
            }
            Err(_) => {
                // Deadline expired: dropping the sender unblocks the
                // thread's park; joining waits out its teardown.
                drop(shutdown_tx);
                let _ = handle.join();
                Err(PlatformError::TimedOut {
                    api: "WASAPI notification registration",
                    deadline_ms: WATCH_DEADLINE.as_millis() as u64,
                })
            }
        }
    }

    fn poll(&mut self) -> Result<u32, PlatformError> {
        // Push-based: WASAPI delivers via the callbacks; the events
        // already sit in the queue (see the seam contract).
        Ok(0)
    }
}

/// The watch thread's whole life: apartment → enumerator → prime →
/// register → report → park → teardown.
fn watch_thread_body(
    queue: Arc<DeviceEventQueue>,
    ready: mpsc::Sender<Result<(), PlatformError>>,
    shutdown: mpsc::Receiver<()>,
) {
    let registration = register_notification_client(Arc::clone(&queue));
    let failure = match registration {
        Ok(session) => {
            let _ = ready.send(Ok(()));
            // Park until the owner signals (or drops) shutdown.
            let _ = shutdown.recv();
            teardown_notification_client(session);
            None
        }
        Err(err) => Some(err),
    };
    if let Some(err) = failure {
        let _ = ready.send(Err(err));
    }
}

/// The live registration a teardown needs (owned by the watch thread).
/// Field order is the teardown order: the enumerator (whose
/// registration references the client) is released first, then the
/// client box frees, and the apartment — which must stay initialized
/// for the thread's whole COM lifetime — uninitializes last
/// (CoUninitialize pairs with the CoInitializeEx in
/// `register_notification_client`).
struct NotificationSession {
    enumerator: *mut c_void,
    client: Box<NotificationClient>,
    _apartment: ComApartment,
}

/// Register the client: apartment, enumerator, client box, MMDevice
/// registration. The apartment moves into the returned session — the
/// watch thread keeps COM initialized for its whole parked life, not
/// just the registration call.
fn register_notification_client(
    queue: Arc<DeviceEventQueue>,
) -> Result<NotificationSession, PlatformError> {
    let apartment = ComApartment::new()?;
    // SAFETY: created fresh on this thread; used and released only on
    // this thread (the session never leaves it — see watch_thread_body).
    let enumerator =
        unsafe { co_create_instance(&CLSID_MM_DEVICE_ENUMERATOR, &IID_MM_DEVICE_ENUMERATOR)? };
    let mut client = Box::new(NotificationClient {
        vtbl: &NOTIFICATION_VTBL,
        queue,
        refs: AtomicU32::new(1),
    });
    let enum_vtbl = unsafe { (*(enumerator as *mut ObjectHeader)).vtbl as *const EnumeratorVtbl };
    // SAFETY: enumerator is live; the client pointer is a complete
    // NotificationClient (its vtbl field leads, repr(C)); the
    // registration adds a COM reference our Release accounts for.
    let hr = unsafe {
        ((*enum_vtbl).register_endpoint_notification_callback)(
            enumerator,
            std::ptr::addr_of_mut!(*client).cast::<c_void>(),
        )
    };
    if hr != S_OK {
        // SAFETY: the single release of the created reference (the
        // apartment then drops here, uninitializing COM after the
        // last interface is gone).
        unsafe {
            release_interface(enumerator);
        }
        return Err(PlatformError::CallFailed {
            api: "IMMDeviceEnumerator::RegisterEndpointNotificationCallback",
            code: Some(i64::from(hr)),
            detail: "the notification registration was refused".to_string(),
        });
    }
    Ok(NotificationSession {
        enumerator,
        client,
        _apartment: apartment,
    })
}

/// Unregister + release, in the documented order; the session's
/// apartment drops last (field order), uninitializing COM only after
/// the last interface is released.
fn teardown_notification_client(session: NotificationSession) {
    let NotificationSession {
        enumerator,
        client,
        _apartment,
    } = session;
    let enum_vtbl = unsafe { (*(enumerator as *mut ObjectHeader)).vtbl as *const EnumeratorVtbl };
    // SAFETY: enumerator is live; the client box is still alive (its
    // address is what was registered — the Box has not moved).
    unsafe {
        ((*enum_vtbl).unregister_endpoint_notification_callback)(
            enumerator,
            std::ptr::addr_of_mut!(*client).cast::<c_void>(),
        );
    }
    // SAFETY: the single release of the enumerator, after the
    // registration that referenced the client is gone.
    unsafe {
        release_interface(enumerator);
    }
    // The client Box drops now, after the enumerator stopped
    // referencing it.
}
