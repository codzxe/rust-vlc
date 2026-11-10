//! The CoreAudio client (card 7.A3): output-device enumeration +
//! change listeners, hand-rolled over the CoreAudio/CoreFoundation C
//! ABI (no `coreaudio-sys`/`core-foundation` crates).
//!
//! # Runtime model (the honest part)
//!
//! - **Enumeration** reads the HAL's object-property store on the
//!   calling thread: `kAudioHardwarePropertyDevices` on the system
//!   object, then per-device `DeviceUID`/`Name`/
//!   `NominalSampleRate`/output `StreamConfiguration`. The HAL has
//!   no apartment/thread requirements for property reads — every
//!   call is a bounded synchronous round ([`D-2`]-style deadlines do
//!   not apply: HAL property calls do not block on the audio IO
//!   path, they are answered from the HAL's own database).
//! - **Notifications** register an `AudioObjectPropertyListenerProc`
//!   against the system object for the devices set
//!   (`kAudioHardwarePropertyDevices`) and the default output device
//!   (`kAudioHardwarePropertyDefaultOutputDevice`). Listeners fire
//!   on a HAL-owned thread; the callbacks touch only the
//!   thread-safe [`Arc<Mutex<ListenerState>>`] and push
//!   [`PlatformEvent`]s (push-based — [`AudioDevices::poll`]
//!   returns 0).
//! - The HAL library is a link-time OS component on every macOS
//!   build — the "probe" of this lane is the enumeration call itself
//!   (the WASAPI precedent).
//!
//! # Listener safety (why the raw pointers are sound)
//!
//! The listener's `inClientData` is `Arc::as_ptr` of the shared
//! state. The registration is removed — with the exact same proc and
//! client-data pointer the HAL requires — **before** the owning
//! [`Watch`] drops its `Arc`, and `AudioObjectRemovePropertyListener`
//! is documented to wait for in-flight callbacks, so no callback can
//! touch freed memory. The one deadlock rule this creates is honored
//! everywhere: the state `Mutex` is **never held across** a
//! `RemovePropertyListener`/`AddPropertyListener` call (the HAL
//! waits for callbacks that may be waiting on that same mutex).
//!
//! **Evidence status:** compile-verified via `cargo check --target
//! aarch64-apple-darwin` and `x86_64-apple-darwin`; runtime NOT
//! exercised in this sandbox (no macOS SDK) — `[VERIFY]` on
//! real-device behavior (in particular the numeric value of
//! `kAudioObjectPropertyElementMaster` and the claim that reading
//! `kAudioHardwarePropertyDefaultOutputDevice` inside its own
//! listener is deadlock-free — the HAL is documented to invoke
//! property listeners without holding its global lock, but that
//! claim needs hardware confirmation).

// SAFETY-SCOPED MODULE (doc 01 §2.7 / MP §9): the CoreAudio/
// CoreFoundation C ABI is the audited OS-service exception this
// file exercises (ADR-005 is the audio seam's decision record).
#![allow(unsafe_code)]

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Arc;
use std::sync::Mutex;

use audio_output::{DeviceDesc, DeviceId};
use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};

use crate::audio::{AudioDevices, stable_device_id};
use crate::error::PlatformError;
use crate::events::{DeviceEventQueue, PlatformEvent};

// ---------------------------------------------------------------------------
// ABI types (CoreAudioBaseTypes.h / AudioObjectBase.h)
// ---------------------------------------------------------------------------

/// `AudioObjectID` (UInt32).
type AudioObjectId = u32;
/// `OSStatus` (MacTypes.h: Int32).
type OsStatus = i32;
/// CoreFoundation `Boolean` (unsigned char).
type CfBoolean = u8;

/// `AudioObjectPropertyAddress` (AudioObjectBase.h, repr(C): three
/// UInt32 words).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct AudioObjectPropertyAddress {
    /// The four-char-code selector (`mSelector`).
    selector: u32,
    /// The scope (`mScope`).
    scope: u32,
    /// The element (`mElement`).
    element: u32,
}

/// `AudioBuffer` (CoreAudioBaseTypes.h).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct AudioBuffer {
    /// `mNumberChannels`.
    number_channels: u32,
    /// `mDataByteSize`.
    data_byte_size: u32,
    /// `mData` (never dereferenced — only the counts are read).
    data: *mut c_void,
}

/// `AudioBufferList` (CoreAudioBaseTypes.h): the fixed header plus
/// the first inline `AudioBuffer`; longer lists continue in the same
/// allocation (the classic variable-length struct — read through the
/// declared prefix with the count the header reports).
#[repr(C)]
struct AudioBufferList {
    /// `mNumberBuffers`.
    number_buffers: u32,
    /// `mBuffers` (the first of `number_buffers`).
    buffers: [AudioBuffer; 1],
}

/// `AudioObjectPropertyListenerProc` (AudioObjectBase.h).
type ListenerProc = unsafe extern "C" fn(
    object: AudioObjectId,
    number_addresses: u32,
    addresses: *const AudioObjectPropertyAddress,
    client_data: *mut c_void,
) -> OsStatus;

// ---------------------------------------------------------------------------
// ABI constants (AudioObjectBase.h / AudioHardware.h /
// AudioHardwareBase.h — four-char codes, little-endian words)
// ---------------------------------------------------------------------------

/// `kAudioObjectSystemObject`.
const K_AUDIO_OBJECT_SYSTEM_OBJECT: AudioObjectId = 1;
/// `kAudioObjectPropertyScopeGlobal` = `'glob'`.
const SCOPE_GLOBAL: u32 = 0x676c_6f62;
/// `kAudioDevicePropertyScopeOutput` = `'outp'`.
const SCOPE_OUTPUT: u32 = 0x6f75_7470;
/// `kAudioObjectPropertyElementMaster` (renamed
/// `kAudioObjectPropertyElementMain`) — the whole-object element.
/// `[VERIFY]` the numeric value on real hardware (the declaration is
/// compile-only evidence in this sandbox).
const ELEMENT_MAIN: u32 = 1;
/// `kAudioHardwarePropertyDevices` = `'dev#'`.
const SELECTOR_DEVICES: u32 = 0x6465_7623;
/// `kAudioHardwarePropertyDefaultOutputDevice` = `'dOut'`.
const SELECTOR_DEFAULT_OUTPUT: u32 = 0x644f_7574;
/// `kAudioObjectPropertyName` = `'name'`.
const SELECTOR_NAME: u32 = 0x6e61_6d65;
/// `kAudioDevicePropertyDeviceUID` = `'uid '`.
const SELECTOR_DEVICE_UID: u32 = 0x7569_6420;
/// `kAudioDevicePropertyStreamConfiguration` = `'str#'`.
const SELECTOR_STREAM_CONFIGURATION: u32 = 0x7374_7223;
/// `kAudioDevicePropertyNominalSampleRate` = `'nsr '`.
const SELECTOR_NOMINAL_SAMPLE_RATE: u32 = 0x6e73_7220;

/// `noErr` (MacTypes.h).
const NO_ERR: OsStatus = 0;
/// `kCFStringEncodingUTF8` (CFString.h: 0x0800_0100).
const CF_ENCODING_UTF8: u32 = 0x0800_0100;

/// The defensive ceiling on any one property read (D-3: bounded
/// memory everywhere — the HAL's honest device lists are a few
/// hundred bytes; a corrupt size is refused, not trusted).
const MAX_PROPERTY_BYTES: usize = 65_536;

// ---------------------------------------------------------------------------
// The C ABI (frameworks)
// ---------------------------------------------------------------------------

// SAFETY: the declared ABI of the CoreAudio framework's object
// property API. Invariants honored at every call site below:
// - object ids are live (the system object by definition, device ids
//   from a fresh devices-array read);
// - addresses point at complete AudioObjectPropertyAddress values;
// - size out-slots are zero-initialized before the call and checked
//   against MAX_PROPERTY_BYTES before any allocation;
// - listener registration/removal pass the identical proc and
//   client-data pair (the HAL's matching requirement).
#[link(name = "CoreAudio", kind = "framework")]
unsafe extern "C" {
    fn AudioObjectGetPropertyDataSize(
        object: AudioObjectId,
        address: *const AudioObjectPropertyAddress,
        qualifier_size: u32,
        qualifier: *const c_void,
        out_size: *mut u32,
    ) -> OsStatus;
    fn AudioObjectGetPropertyData(
        object: AudioObjectId,
        address: *const AudioObjectPropertyAddress,
        qualifier_size: u32,
        qualifier: *const c_void,
        io_size: *mut u32,
        out_data: *mut c_void,
    ) -> OsStatus;
    fn AudioObjectAddPropertyListener(
        object: AudioObjectId,
        address: *const AudioObjectPropertyAddress,
        listener: ListenerProc,
        client_data: *mut c_void,
    ) -> OsStatus;
    fn AudioObjectRemovePropertyListener(
        object: AudioObjectId,
        address: *const AudioObjectPropertyAddress,
        listener: ListenerProc,
        client_data: *mut c_void,
    ) -> OsStatus;
}

// SAFETY: the declared ABI of the CoreFoundation functions this
// client needs. Invariants honored at every call site: CFString
// pointers are live values returned by the HAL; the CString buffer
// is a caller-owned, caller-sized byte array; CFRelease is called
// exactly once per HAL-returned CFString this crate owns by
// convention (Get rules: the caller does NOT own CoreFoundation
// "Get" results — but HAL property storage returns a CFString the
// caller must release per AudioHardware's documented property
// contract for CF types; the release pairs with that exact read).
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringGetCString(
        string: *const c_void,
        buffer: *mut u8,
        buffer_size: isize,
        encoding: u32,
    ) -> CfBoolean;
    fn CFRelease(cf: *const c_void);
}

// ---------------------------------------------------------------------------
// Property reads
// ---------------------------------------------------------------------------

/// Read one property into a byte buffer (size discovered, bounded,
/// zero-initialized). A zero-size property reads as empty.
///
/// # Errors
/// Typed [`PlatformError`] on any failing `OSStatus`, an unbounded
/// size, or a size that changed between the two calls.
unsafe fn get_property_bytes(
    object: AudioObjectId,
    address: &AudioObjectPropertyAddress,
    api: &'static str,
) -> Result<Vec<u8>, PlatformError> {
    let mut size: u32 = 0;
    // SAFETY: object is live, address is complete, the out-slot is
    // zero-initialized, no qualifier (NULL/0 is the documented
    // "none" shape).
    let status =
        unsafe { AudioObjectGetPropertyDataSize(object, address, 0, std::ptr::null(), &mut size) };
    if status != NO_ERR {
        return Err(PlatformError::CallFailed {
            api,
            code: Some(i64::from(status)),
            detail: "AudioObjectGetPropertyDataSize refused the property".to_string(),
        });
    }
    if size as usize > MAX_PROPERTY_BYTES {
        return Err(PlatformError::CallFailed {
            api,
            code: None,
            detail: format!("property size {size} exceeds the {MAX_PROPERTY_BYTES}-byte bound"),
        });
    }
    let mut bytes = vec![0u8; size as usize];
    if size == 0 {
        return Ok(bytes);
    }
    let mut io_size = size;
    // SAFETY: bytes is `size` bytes of caller-owned memory; the
    // io-size slot carries the same size for the API to decrement.
    let status = unsafe {
        AudioObjectGetPropertyData(
            object,
            address,
            0,
            std::ptr::null(),
            &mut io_size,
            bytes.as_mut_ptr().cast::<c_void>(),
        )
    };
    if status != NO_ERR {
        return Err(PlatformError::CallFailed {
            api,
            code: Some(i64::from(status)),
            detail: "AudioObjectGetPropertyData refused the read".to_string(),
        });
    }
    if io_size as usize > bytes.len() {
        return Err(PlatformError::CallFailed {
            api,
            code: None,
            detail: "the property grew between the size query and the read".to_string(),
        });
    }
    bytes.truncate(io_size as usize);
    Ok(bytes)
}

/// Read a `CFString` property into an owned `String` (the read
/// buffer is bounded; the CFString is released exactly once — the
/// HAL's documented ownership for CF-type property values).
///
/// # Errors
/// Typed [`PlatformError`] on failure or non-UTF-8 content.
unsafe fn get_property_string(
    object: AudioObjectId,
    address: &AudioObjectPropertyAddress,
    api: &'static str,
) -> Result<String, PlatformError> {
    let bytes = unsafe { get_property_bytes(object, address, api)? };
    if bytes.len() != std::mem::size_of::<*const c_void>() {
        return Err(PlatformError::CallFailed {
            api,
            code: None,
            detail: "the CFString property came back with an unexpected size".to_string(),
        });
    }
    let mut pointer_bytes = [0u8; std::mem::size_of::<*const c_void>()];
    pointer_bytes.copy_from_slice(&bytes);
    let string: *const c_void =
        usize::from_ne_bytes(pointer_bytes) as *const c_void;
    if string.is_null() {
        return Err(PlatformError::CallFailed {
            api,
            code: None,
            detail: "the property returned a NULL CFString".to_string(),
        });
    }
    // Bounded copy: device names/UIDs are short; 4096 bytes is the
    // documented practical ceiling for the values this reads.
    const CAP: usize = 4096;
    let mut buffer = [0u8; CAP];
    // SAFETY: string is the live CFString just read; buffer is
    // caller-owned at CAP bytes; the encoding is UTF-8.
    let ok = unsafe { CFStringGetCString(string, buffer.as_mut_ptr(), CAP as isize, CF_ENCODING_UTF8) };
    // SAFETY: the HAL handed this CFString to the caller; this is
    // the single release that pairs with the property read.
    unsafe {
        CFRelease(string);
    }
    if ok == 0 {
        return Err(PlatformError::InvalidEncoding { api });
    }
    let end = buffer.iter().position(|byte| *byte == 0).unwrap_or(CAP);
    std::str::from_utf8(&buffer[..end])
        .map(str::to_string)
        .map_err(|_| PlatformError::InvalidEncoding { api })
}

/// Read a `Float64` property (the nominal sample rate).
///
/// # Errors
/// Typed [`PlatformError`] on failure or a malformed size.
unsafe fn get_property_f64(
    object: AudioObjectId,
    address: &AudioObjectPropertyAddress,
    api: &'static str,
) -> Result<f64, PlatformError> {
    let bytes = unsafe { get_property_bytes(object, address, api)? };
    if bytes.len() != 8 {
        return Err(PlatformError::CallFailed {
            api,
            code: None,
            detail: "the Float64 property came back with an unexpected size".to_string(),
        });
    }
    let mut word = [0u8; 8];
    word.copy_from_slice(&bytes);
    Ok(f64::from_ne_bytes(word))
}

/// The system object's device list (`kAudioHardwarePropertyDevices`).
fn system_device_ids() -> Result<Vec<AudioObjectId>, PlatformError> {
    let address = AudioObjectPropertyAddress {
        selector: SELECTOR_DEVICES,
        scope: SCOPE_GLOBAL,
        element: ELEMENT_MAIN,
    };
    // SAFETY: the system object is live by definition; the address
    // is complete.
    let bytes =
        unsafe { get_property_bytes(K_AUDIO_OBJECT_SYSTEM_OBJECT, &address, "kAudioHardwarePropertyDevices")? };
    if bytes.len() % 4 != 0 {
        return Err(PlatformError::CallFailed {
            api: "kAudioHardwarePropertyDevices",
            code: None,
            detail: "the device id array is not a whole number of UInt32s".to_string(),
        });
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|chunk| u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect())
}

/// The current default output device, when the system has one.
fn default_output_device_id() -> Result<Option<AudioObjectId>, PlatformError> {
    let address = AudioObjectPropertyAddress {
        selector: SELECTOR_DEFAULT_OUTPUT,
        scope: SCOPE_GLOBAL,
        element: ELEMENT_MAIN,
    };
    // SAFETY: the system object is live by definition.
    let bytes = unsafe {
        get_property_bytes(K_AUDIO_OBJECT_SYSTEM_OBJECT, &address, "kAudioHardwarePropertyDefaultOutputDevice")?
    };
    if bytes.len() != 4 {
        return Err(PlatformError::CallFailed {
            api: "kAudioHardwarePropertyDefaultOutputDevice",
            code: None,
            detail: "the default-device property came back with an unexpected size".to_string(),
        });
    }
    let id = u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    Ok(if id == 0 { None } else { Some(id) })
}

/// One device → its stable id and description (`None` when the
/// device is too degenerate to describe honestly — the WASAPI-lane
/// precedent: skip, never guess).
fn describe_device(device: AudioObjectId) -> Result<Option<(AudioObjectId, DeviceDesc)>, PlatformError> {
    let uid_address = AudioObjectPropertyAddress {
        selector: SELECTOR_DEVICE_UID,
        scope: SCOPE_GLOBAL,
        element: ELEMENT_MAIN,
    };
    // SAFETY: device is live (from a fresh devices-array read).
    let uid = match unsafe { get_property_string(device, &uid_address, "kAudioDevicePropertyDeviceUID") } {
        Ok(uid) => uid,
        Err(_) => return Ok(None), // no UID: not addressable for streaming
    };
    if uid.is_empty() {
        return Ok(None);
    }
    let name_address = AudioObjectPropertyAddress {
        selector: SELECTOR_NAME,
        scope: SCOPE_GLOBAL,
        element: ELEMENT_MAIN,
    };
    // SAFETY: device is live.
    let name = unsafe { get_property_string(device, &name_address, "kAudioObjectPropertyName") }
        .ok()
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| format!("CoreAudio device {uid}"));
    Ok(Some((device, DeviceDesc {
        id: stable_device_id(&uid),
        name,
        preferred: preferred_format(device),
    })))
}

/// The device's preferred engine format suggestion: Float32 (the
/// HAL's native mix format), the output-scope channel count, planar
/// (CoreAudio's non-interleaved buffer layout), the nominal rate.
fn preferred_format(device: AudioObjectId) -> AudioFormat {
    let rate_address = AudioObjectPropertyAddress {
        selector: SELECTOR_NOMINAL_SAMPLE_RATE,
        scope: SCOPE_GLOBAL,
        element: ELEMENT_MAIN,
    };
    // SAFETY: device is live; a failing/absent rate is a suggestion
    // gap, not an enumeration failure — fall back to 48 kHz.
    let rate = unsafe { get_property_f64(device, &rate_address, "kAudioDevicePropertyNominalSampleRate") }
        .ok()
        .filter(|rate| *rate > 0.0 && *rate <= u32::MAX as f64)
        .map(|rate| rate as u32)
        .unwrap_or(48_000);
    AudioFormat {
        sample: SampleFormat::F32,
        channels: output_channels(device).max(1),
        layout: AudioLayout::Planar,
        rate,
    }
}

/// The output-scope channel count (the sum of the output
/// `AudioBufferList`'s buffers' channel counts; 0 when unreadable).
fn output_channels(device: AudioObjectId) -> u32 {
    let address = AudioObjectPropertyAddress {
        selector: SELECTOR_STREAM_CONFIGURATION,
        scope: SCOPE_OUTPUT,
        element: ELEMENT_MAIN,
    };
    // SAFETY: device is live; an unreadable configuration is a
    // suggestion gap — the caller clamps to at least one channel.
    let bytes = match unsafe {
        get_property_bytes(device, &address, "kAudioDevicePropertyStreamConfiguration")
    } {
        Ok(bytes) => bytes,
        Err(_) => return 0,
    };
    if bytes.len() < std::mem::size_of::<u32>() {
        return 0;
    }
    let number_buffers = u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    let header = std::mem::size_of::<AudioBufferList>() - std::mem::size_of::<AudioBuffer>();
    let stride = std::mem::size_of::<AudioBuffer>();
    let mut channels = 0u32;
    for index in 0..number_buffers {
        let offset = header + index * stride;
        // Bounds-check every buffer read against the actual size the
        // HAL reported (D-3 discipline: never trust the count).
        if offset + stride > bytes.len() {
            break;
        }
        let buffer = &bytes[offset..offset + stride];
        channels = channels.saturating_add(u32::from_ne_bytes([
            buffer[0], buffer[1], buffer[2], buffer[3],
        ]));
    }
    channels
}

// ---------------------------------------------------------------------------
// The listener (fires on a HAL thread)
// ---------------------------------------------------------------------------

/// The shared listener state (the callbacks' client data).
struct ListenerState {
    /// Where events land (bounded — D-3).
    queue: Arc<DeviceEventQueue>,
    /// HAL object id → stable id, from the last enumeration (HAL ids
    /// are session-scoped; the stable ids are UID-minted).
    ids: HashMap<AudioObjectId, DeviceId>,
}

/// The listener callback: translates HAL property changes into
/// [`PlatformEvent`]s. No panics, no unwinding across the C ABI —
/// every failure degrades to the honest `AudioSetChanged` advice.
unsafe extern "C" fn on_property_change(
    _object: AudioObjectId,
    number_addresses: u32,
    addresses: *const AudioObjectPropertyAddress,
    client_data: *mut c_void,
) -> OsStatus {
    // SAFETY: client_data is the Arc'd ListenerState pointer the
    // registration passed; the matching removal (which waits for
    // this callback to finish) runs before the Arc can drop.
    let state = unsafe { &*(client_data as *const Mutex<ListenerState>) };
    for index in 0..number_addresses as usize {
        // SAFETY: the HAL passes `number_addresses` complete
        // addresses; each is read once, by value fields.
        let selector = unsafe { (*addresses.add(index)).selector };
        if selector == SELECTOR_DEFAULT_OUTPUT {
            // Read the new default inline. Reading a *different*
            // property than the one being notified (devices-set vs
            // default-device) is the listener-reentrancy pattern
            // Apple's own samples use; `[VERIFY]` on hardware.
            let new_default = default_output_device_id().ok().flatten();
            let mut mapped = None;
            if let (Some(object), Ok(guard)) = (new_default, state.lock()) {
                mapped = guard.ids.get(&object).copied();
            }
            let event = match mapped {
                Some(id) => PlatformEvent::AudioDefaultChanged { id: Some(id) },
                // Unknown (e.g. a brand-new device): advise a
                // re-enumeration instead of guessing.
                None => PlatformEvent::AudioSetChanged,
            };
            if let Ok(guard) = state.lock() {
                guard.queue.push(event);
            }
        } else {
            // Devices added/removed/re-jigged: the HAL does not
            // itemize — advise a re-enumeration.
            if let Ok(guard) = state.lock() {
                guard.queue.push(PlatformEvent::AudioSetChanged);
            }
        }
    }
    NO_ERR
}

// ---------------------------------------------------------------------------
// The audio seam implementation
// ---------------------------------------------------------------------------

/// The CoreAudio adapter.
pub struct CoreAudioDevices {
    watch: Option<Watch>,
}

impl CoreAudioDevices {
    /// A fresh adapter (the HAL library is a link-time OS component
    /// — the "probe" of this lane is the enumeration call itself).
    #[must_use = "constructs the adapter"]
    pub fn new() -> CoreAudioDevices {
        CoreAudioDevices { watch: None }
    }
}

impl Default for CoreAudioDevices {
    fn default() -> Self {
        CoreAudioDevices::new()
    }
}

/// The live listener registration. Field order is the teardown
/// order: the removal (in `Drop`) runs while the state `Arc` is
/// still alive; the `Arc` drops only afterwards.
struct Watch {
    state: Arc<Mutex<ListenerState>>,
}

impl Drop for Watch {
    fn drop(&mut self) {
        // DEADLOCK INVARIANT (module docs): this runs WITHOUT holding
        // the state mutex — the HAL waits for in-flight callbacks,
        // which may be waiting on that same mutex.
        // SAFETY: the system object is live by definition; the
        // address/proc/client-data triple is exactly the one the
        // subscription registered (the HAL requires the match).
        let client = Arc::as_ptr(&self.state).cast::<c_void>().cast_mut();
        unsafe {
            AudioObjectRemovePropertyListener(
                K_AUDIO_OBJECT_SYSTEM_OBJECT,
                &AudioObjectPropertyAddress {
                    selector: SELECTOR_DEVICES,
                    scope: SCOPE_GLOBAL,
                    element: ELEMENT_MAIN,
                },
                on_property_change,
                client,
            );
            AudioObjectRemovePropertyListener(
                K_AUDIO_OBJECT_SYSTEM_OBJECT,
                &AudioObjectPropertyAddress {
                    selector: SELECTOR_DEFAULT_OUTPUT,
                    scope: SCOPE_GLOBAL,
                    element: ELEMENT_MAIN,
                },
                on_property_change,
                client,
            );
        }
        // The state Arc drops now — after the removals guarantee no
        // callback can still hold its pointer.
    }
}

/// Enumerate the output devices (the probe round).
fn list_devices() -> Result<Vec<(AudioObjectId, DeviceDesc)>, PlatformError> {
    system_device_ids()?
        .into_iter()
        .filter_map(|device| describe_device(device).transpose())
        .collect()
}

impl AudioDevices for CoreAudioDevices {
    fn enumerate(&mut self) -> Result<Vec<DeviceDesc>, PlatformError> {
        let found = list_devices()?;
        // With a live watch, refresh the translation map the listener
        // uses (never holding the mutex across a HAL call — only
        // around the map swap).
        if let Some(watch) = self.watch.as_ref() {
            if let Ok(mut guard) = watch.state.lock() {
                guard.ids = found
                    .iter()
                    .map(|(object, desc)| (*object, desc.id))
                    .collect();
            }
        }
        Ok(found.into_iter().map(|(_, desc)| desc).collect())
    }

    fn subscribe(&mut self, queue: Arc<DeviceEventQueue>) -> Result<(), PlatformError> {
        // A second subscribe replaces the first watch (the old queue
        // stops receiving events — Watch::drop unregisters).
        self.watch = None;
        // Prime the translation map through the same enumeration the
        // seam exposes (also the lane's probe: a failure here is the
        // capability verdict).
        let found = list_devices()?;
        let state = Arc::new(Mutex::new(ListenerState {
            queue,
            ids: found.into_iter().map(|(object, desc)| (object, desc.id)).collect(),
        }));
        let client = Arc::as_ptr(&state).cast::<c_void>().cast_mut();
        // SAFETY: system object live; the proc is module-static; the
        // client pointer stays valid until the matching removal in
        // Watch::drop (the state Arc outlives the registration).
        let status = unsafe {
            AudioObjectAddPropertyListener(
                K_AUDIO_OBJECT_SYSTEM_OBJECT,
                &AudioObjectPropertyAddress {
                    selector: SELECTOR_DEVICES,
                    scope: SCOPE_GLOBAL,
                    element: ELEMENT_MAIN,
                },
                on_property_change,
                client,
            )
        };
        if status != NO_ERR {
            return Err(PlatformError::CallFailed {
                api: "AudioObjectAddPropertyListener(devices)",
                code: Some(i64::from(status)),
                detail: "the HAL refused the devices listener".to_string(),
            });
        }
        // SAFETY: as above, for the default-device property.
        let status = unsafe {
            AudioObjectAddPropertyListener(
                K_AUDIO_OBJECT_SYSTEM_OBJECT,
                &AudioObjectPropertyAddress {
                    selector: SELECTOR_DEFAULT_OUTPUT,
                    scope: SCOPE_GLOBAL,
                    element: ELEMENT_MAIN,
                },
                on_property_change,
                client,
            )
        };
        if status != NO_ERR {
            // Roll the first registration back before refusing
            // (never holding the state mutex across a HAL call).
            // SAFETY: the identical triple that just succeeded.
            unsafe {
                AudioObjectRemovePropertyListener(
                    K_AUDIO_OBJECT_SYSTEM_OBJECT,
                    &AudioObjectPropertyAddress {
                        selector: SELECTOR_DEVICES,
                        scope: SCOPE_GLOBAL,
                        element: ELEMENT_MAIN,
                    },
                    on_property_change,
                    client,
                );
            }
            return Err(PlatformError::CallFailed {
                api: "AudioObjectAddPropertyListener(default device)",
                code: Some(i64::from(status)),
                detail: "the HAL refused the default-device listener".to_string(),
            });
        }
        self.watch = Some(Watch { state });
        Ok(())
    }

    fn poll(&mut self) -> Result<u32, PlatformError> {
        // Push-based: the HAL listener pushes into the queue; the
        // events already sit there (see the seam contract).
        Ok(0)
    }
}
