//! Metal device enumeration + the CAMetalLayer glue (card 7.A3) —
//! the GPU seam's macOS implementation, hand-declared over the
//! Metal + libobjc C ABIs (no `metal-rs`/`objc2` crates).
//!
//! # What lives here
//!
//! - **Enumeration** (`MTLCopyAllDevices`): every Metal device as a
//!   [`GpuAdapterInfo`] — the name, `registryID`, and
//!   `maxWorkingSetSize` the 7.C1 fallback chain consumes. Metal
//!   has no software adapter (no WARP/llvmpipe equivalent), so
//!   `is_software` is `false` by construction; `lowPower` marks
//!   Apple-silicon *efficiency* GPUs, which are still hardware —
//!   recorded in the name, not in `is_software`.
//! - **Layer glue** ([`MetalDevice`], [`MetalLayer`],
//!   [`MetalDrawable`]): the typed handoff point the ADR-003-gated
//!   wgpu renderer will consume — an owned `id<MTLDevice>`, a
//!   retained `CAMetalLayer` bound to it, and an owned next
//!   drawable. The presentation pipeline itself (textures, command
//!   buffers) belongs to `renderer-wgpu`'s Lead-gated import
//!   (records/wgpu-30.0.1.md); this module answers "give the
//!   renderer the platform objects it cannot create itself"
//!   (doc 01 §2.7's "render surface support").
//!
//! # Ownership discipline (why the raw pointers are sound)
//!
//! `MTLCopyAllDevices` follows the CoreFoundation **Create/Copy
//! rule** (the returned array is retained by the caller) — released
//! exactly once after the enumeration copied everything out. The
//! devices inside the array are owned by it, so [`MetalDevice`]
//! retains the one it wraps and releases it on `Drop`;
//! [`MetalLayer`] (`+[CAMetalLayer layer]` returns an autoreleased
//! instance) retains before its creating pool drains and releases
//! on `Drop`; [`MetalDrawable`] (`nextDrawable`) does the same.
//! `MTLDevice` is documented thread-safe — [`MetalDevice`] is
//! `Send`; `CAMetalLayer` (a CALayer) carries main-thread affinity —
//! [`MetalLayer`] deliberately is **not** `Send`.
//!
//! **Evidence status:** compile-verified via `cargo check --target
//! aarch64-apple-darwin` and `x86_64-apple-darwin`; runtime NOT
//! exercised in this sandbox (no macOS SDK) — `[VERIFY]`.

// SAFETY-SCOPED MODULE (doc 01 §2.7 / MP §9): the Metal + libobjc C
// ABIs are the audited OS-service exception this file exercises.
#![allow(unsafe_code)]

use std::fmt;

use crate::error::PlatformError;
use crate::gpu::{GpuAdapterInfo, GpuAdapters};
use crate::macos::objc::{
    AutoreleasePool, ObjcId, class, msg_send_bool, msg_send_id, msg_send_id_id, msg_send_id_u64,
    msg_send_u64, msg_send_void_f64_f64, msg_send_void_id, nsstring_to_string, release, retain, sel,
};

// SAFETY: the declared ABI of the Metal framework's one C entry
// point this crate needs. MTLCopyAllDevices follows the CF
// Create/Copy rule: the returned NSArray is owned by the caller and
// released exactly once by the enumeration that called it.
#[link(name = "Metal", kind = "framework")]
unsafe extern "C" {
    /// `MTLCopyAllDevices()` — every Metal device on the system
    /// (retained `NSArray<id<MTLDevice>>`).
    fn MTLCopyAllDevices() -> ObjcId;
}

// ---------------------------------------------------------------------------
// The GPU seam implementation
// ---------------------------------------------------------------------------

/// The Metal device scanner.
#[derive(Debug, Clone, Default)]
pub struct MetalDevices;

impl MetalDevices {
    /// A fresh scanner.
    #[must_use = "constructs the scanner"]
    pub fn new() -> MetalDevices {
        MetalDevices
    }
}

impl GpuAdapters for MetalDevices {
    fn enumerate(&mut self) -> Result<Vec<GpuAdapterInfo>, PlatformError> {
        // SAFETY: no arguments; the returned array is released
        // exactly once at the end of this function (Create/Copy
        // rule) after every value is copied out.
        let array = unsafe { MTLCopyAllDevices() };
        if array.is_null() {
            return Err(PlatformError::CallFailed {
                api: "MTLCopyAllDevices",
                code: None,
                detail: "the Metal framework returned no device array".to_string(),
            });
        }
        let adapters = devices_from_array(array);
        // SAFETY: the single release of the retained array (every
        // device it owned has been copied out or retained on its
        // own above).
        unsafe {
            release(array);
        }
        adapters
    }
}

/// Copy every device's description out of the retained array.
fn devices_from_array(array: ObjcId) -> Result<Vec<GpuAdapterInfo>, PlatformError> {
    let count_sel = sel("count")?;
    let at_sel = sel("objectAtIndex:")?;
    // SAFETY: array is the live retained array; count takes no
    // arguments and returns NSUInteger.
    let count = unsafe { msg_send_u64(array, count_sel) };
    let mut adapters = Vec::new();
    for index in 0..count {
        // SAFETY: array is live and index < count (just read);
        // objectAtIndex: takes the NSUInteger index and returns the
        // (array-owned) device.
        let device = unsafe { msg_send_id_u64(array, at_sel, index) };
        if device.is_null() {
            continue; // a vanishing device is not fatal to the list
        }
        if let Some(info) = describe_device(device)? {
            adapters.push(info);
        }
    }
    Ok(adapters)
}

/// One `id<MTLDevice>` → [`GpuAdapterInfo`] (`None` when the device
/// is too degenerate to describe honestly).
fn describe_device(device: ObjcId) -> Result<Option<GpuAdapterInfo>, PlatformError> {
    let name_sel = sel("name")?;
    // SAFETY: device is live (array-owned, array still alive); name
    // takes no arguments and returns an NSString valid while the
    // array lives — copied out below, before the array's release.
    let name_string = unsafe { msg_send_id(device, name_sel) };
    if name_string.is_null() {
        return Ok(None);
    }
    // SAFETY: the NSString is live for this call (its owner, the
    // device/array, is alive); the copy happens before any release.
    let name = unsafe { nsstring_to_string(name_string)? };
    if name.is_empty() {
        return Ok(None);
    }
    let registry_sel = sel("registryID")?;
    // SAFETY: device is live; registryID returns uint64_t.
    let registry_id = unsafe { msg_send_u64(device, registry_sel) };
    let vram_sel = sel("maxWorkingSetSize")?;
    // SAFETY: device is live; maxWorkingSetSize returns uint64_t
    // (a macOS Metal method — both cross-check targets are macOS).
    let working_set = unsafe { msg_send_u64(device, vram_sel) };
    let low_power_sel = sel("lowPower")?;
    // SAFETY: device is live; lowPower returns BOOL.
    let low_power = unsafe { msg_send_bool(device, low_power_sel) } != 0;
    let headless_sel = sel("isHeadless")?;
    // SAFETY: device is live; isHeadless returns BOOL.
    let headless = unsafe { msg_send_bool(device, headless_sel) } != 0;
    let display = if headless {
        format!("{name} (headless)")
    } else if low_power {
        format!("{name} (low power)")
    } else {
        name
    };
    Ok(Some(GpuAdapterInfo {
        // Metal reports no PCI ids: the vendor field stays 0 (the
        // documented per-lane meaning) and the device field carries
        // the IORegistry id, truncated to the seam's u32 (the high
        // bits are a generation counter; the low 32 distinguish the
        // devices of one boot).
        vendor_id: 0,
        device_id: registry_id as u32,
        name: display,
        dedicated_video_memory: Some(working_set),
        // Metal has no software adapter: every device is hardware
        // (low-power GPUs included — recorded in the name above).
        is_software: false,
    }))
}

// ---------------------------------------------------------------------------
// The layer glue (the ADR-003 handoff point)
// ---------------------------------------------------------------------------

/// An owned `id<MTLDevice>` (retained; released on `Drop`).
///
/// `MTLDevice` is documented thread-safe, so the handle is `Send`.
/// Obtain one from an enumeration round via [`MetalDevices`].
pub struct MetalDevice {
    device: ObjcId,
}

// SAFETY: MTLDevice is documented thread-safe by the Metal framework
// (command queues and methods may be called from any thread); the
// raw pointer carries no thread affinity of its own.
unsafe impl Send for MetalDevice {}

impl fmt::Debug for MetalDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MetalDevice").finish_non_exhaustive()
    }
}

impl MetalDevices {
    /// Retain the device at `index` of the last enumeration round as
    /// an owned [`MetalDevice`] handle — the renderer-facing
    /// handoff. The index order matches [`GpuAdapters::enumerate`].
    ///
    /// # Errors
    /// Typed [`PlatformError`] when the round fails or the index is
    /// out of bounds (never a silent NULL handle).
    pub fn retain_device_at(&self, index: u64) -> Result<MetalDevice, PlatformError> {
        // SAFETY: no arguments; the returned array is released
        // exactly once at the end of this function, after the
        // selected device has been retained for the handle.
        let array = unsafe { MTLCopyAllDevices() };
        if array.is_null() {
            return Err(PlatformError::CallFailed {
                api: "MTLCopyAllDevices",
                code: None,
                detail: "the Metal framework returned no device array".to_string(),
            });
        }
        let result = (|| {
            let count_sel = sel("count")?;
            // SAFETY: array is live; count is the documented
            // accessor.
            let count = unsafe { msg_send_u64(array, count_sel) };
            if index >= count {
                return Err(PlatformError::CallFailed {
                    api: "MTLCopyAllDevices",
                    code: None,
                    detail: format!("device index {index} is out of bounds ({count} devices)"),
                });
            }
            let at_sel = sel("objectAtIndex:")?;
            // SAFETY: array is live and index < count.
            let device = unsafe { msg_send_id_u64(array, at_sel, index) };
            if device.is_null() {
                return Err(PlatformError::CallFailed {
                    api: "NSArray objectAtIndex:",
                    code: None,
                    detail: "the device vanished during the round".to_string(),
                });
            }
            // SAFETY: device is a live object (array-owned);
            // objc_retain adds our reference, honored by Drop.
            let retained = retain(device)?;
            Ok(MetalDevice { device: retained })
        })();
        // SAFETY: the single release of the retained array (the
        // selected device now holds its own reference).
        unsafe {
            release(array);
        }
        result
    }
}

/// An owned `CAMetalLayer` bound to a [`MetalDevice`] — the
/// platform surface object a renderer presents to.
///
/// NOT `Send`: `CALayer` carries AppKit/CoreAnimation main-thread
/// affinity; the renderer receives it where it was created (doc 01
/// §2.7 — platform crates hand over surface support, they do not
/// own the render loop).
pub struct MetalLayer {
    layer: ObjcId,
}

impl fmt::Debug for MetalLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MetalLayer").finish_non_exhaustive()
    }
}

impl Drop for MetalLayer {
    fn drop(&mut self) {
        // SAFETY: pairs with the objc_retain in `new`; the single
        // release of the layer's own reference.
        unsafe {
            release(self.layer);
        }
    }
}

impl MetalLayer {
    /// Create a layer bound to `device`
    /// (`[[CAMetalLayer layer] setDevice:]` on a scoped pool).
    ///
    /// # Errors
    /// Typed [`PlatformError`] when the class, the layer or the
    /// binding is refused.
    pub fn new(device: &MetalDevice) -> Result<MetalLayer, PlatformError> {
        let _pool = AutoreleasePool::new()?;
        let cls = class("CAMetalLayer")?;
        let layer_sel = sel("layer")?;
        // SAFETY: cls is the live CAMetalLayer class; `layer` is the
        // documented class factory returning an autoreleased
        // instance — retained below before the pool drains.
        let layer = unsafe { msg_send_id(cls, layer_sel) };
        if layer.is_null() {
            return Err(PlatformError::CallFailed {
                api: "CAMetalLayer layer",
                code: None,
                detail: "the layer factory returned no layer".to_string(),
            });
        }
        // SAFETY: layer is live; objc_retain adds the reference this
        // newtype's Drop releases (outliving the pool).
        let retained = retain(layer)?;
        let set_device_sel = sel("setDevice:")?;
        // SAFETY: retained is live; setDevice: takes the id argument
        // and returns void.
        unsafe { msg_send_void_id(retained, set_device_sel, device.device) };
        Ok(MetalLayer { layer: retained })
    }

    /// The drawable size (points) the layer should render at —
    /// `setDrawableSize:` (two CGFloats; the pinned two-f64
    /// msgSend shape).
    ///
    /// # Errors
    /// Typed [`PlatformError`] when the selector is refused.
    pub fn set_drawable_size(&self, width: f64, height: f64) -> Result<(), PlatformError> {
        let size_sel = sel("setDrawableSize:")?;
        // SAFETY: self.layer is live (owned); the selector takes the
        // two-float CGSize by value and returns void.
        unsafe { msg_send_void_f64_f64(self.layer, size_sel, width, height) };
        Ok(())
    }

    /// Acquire the next drawable (`nextDrawable`) as an owned
    /// [`MetalDrawable`] — the presentation slot. The layer must be
    /// attached to a view/window for the system to hand drawables
    /// out; a refusal is the typed error, never a fake one.
    ///
    /// # Errors
    /// [`PlatformError::CallFailed`] when no drawable is available.
    pub fn next_drawable(&self) -> Result<MetalDrawable, PlatformError> {
        let next_sel = sel("nextDrawable")?;
        // SAFETY: self.layer is live (owned); nextDrawable takes no
        // arguments and returns an autoreleased drawable — retained
        // below for the handle.
        let drawable = unsafe { msg_send_id(self.layer, next_sel) };
        if drawable.is_null() {
            return Err(PlatformError::CallFailed {
                api: "CAMetalLayer nextDrawable",
                code: None,
                detail: "no drawable was available (layer not attached to a window?)".to_string(),
            });
        }
        // SAFETY: drawable is live; objc_retain adds the reference
        // the handle's Drop releases.
        let retained = retain(drawable)?;
        Ok(MetalDrawable { drawable: retained })
    }
}

/// An owned `id<CAMDurableDrawable>` (`nextDrawable`'s result).
///
/// Opaque by design: the texture/command-buffer surface belongs to
/// the ADR-003-gated renderer; this handle is the lifetime-safe way
/// to hold the presentation slot in engine vocabulary (ARCH-6: no
/// OS types leak upward).
pub struct MetalDrawable {
    drawable: ObjcId,
}

impl fmt::Debug for MetalDrawable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MetalDrawable").finish_non_exhaustive()
    }
}

impl Drop for MetalDrawable {
    fn drop(&mut self) {
        // SAFETY: pairs with the objc_retain in `next_drawable`.
        unsafe {
            release(self.drawable);
        }
    }
}
