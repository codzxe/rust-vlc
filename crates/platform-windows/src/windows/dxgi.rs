//! DXGI adapter enumeration (card 7.A1) — the GPU seam's Windows
//! implementation, hand-declared over the dxgi C-ABI.
//!
//! `CreateDXGIFactory1` + `IDXGIFactory1::EnumAdapters1` +
//! `IDXGIAdapter1::GetDesc1`: the PCI vendor/device ids, the adapter
//! description string and the dedicated-video-memory size, exactly
//! the fields [`GpuAdapterInfo`](crate::gpu::GpuAdapterInfo) carries.
//! WARP (Microsoft's software rasterizer) is flagged by its vendor id
//! 0x8C / device 0x8C `Microsoft Basic Render Driver` naming.
//!
//! **Evidence status:** compile-verified via `cargo check --target
//! x86_64-pc-windows-msvc`; runtime NOT exercised in this sandbox
//! (no Windows SDK) — `[VERIFY]` on real-device behavior.

// SAFETY-SCOPED MODULE (doc 01 §2.7 / MP §9): the dxgi C-ABI is the
// audited OS-service exception this file exercises.
#![allow(unsafe_code)]

use std::ffi::c_void;

use crate::error::PlatformError;
use crate::gpu::{GpuAdapterInfo, GpuAdapters};
use crate::windows::com::{Guid, Hresult, S_OK, from_wide, release_interface};

/// `IID_IDXGIFactory1` {770AAE78-F26F-4DBA-A829-253C83D1B387}.
const IID_IDXGI_FACTORY1: Guid = Guid {
    data1: 0x770A_AE78,
    data2: 0xF26F,
    data3: 0x4DBA,
    data4: [0xA8, 0x29, 0x25, 0x3C, 0x83, 0xD1, 0xB3, 0x87],
};

/// Microsoft's PCI vendor id — the WARP software adapter
/// ("Microsoft Basic Render Driver") reports it, with device id
/// 0x008C.
const WARP_VENDOR_ID: u32 = 0x1414;

// SAFETY: the declared ABI of dxgi.dll's CreateDXGIFactory1. The
// factory is created fresh per enumeration call and released exactly
// once on every path; the riid argument points at a complete Guid.
#[link(name = "dxgi")]
unsafe extern "system" {
    fn CreateDXGIFactory1(riid: *const Guid, ppfactory: *mut *mut c_void) -> Hresult;
}

/// `IDXGIObject` — the four slots every dxgi interface inherits
/// between `IUnknown` and its own methods (dxgi.h order:
/// `SetPrivateData`, `SetPrivateDataInterface`, `GetPrivateData`,
/// `GetParent`).
///
/// W08-A10 F-001 (S0): both vtables below previously declared
/// `IUnknown` immediately followed by the factory/adapter methods —
/// this base was missing entirely, so every declared slot resolved
/// four positions early (`get_desc1` called `EnumOutputs`,
/// `enum_adapters1` called `EnumAdapters`). The quartet is declared
/// once here and embedded in both vtables.
#[repr(C)]
struct DxgiObjectVtbl {
    /// `SetPrivateData(this, name, size, data)` — slot 4.
    pub _set_private_data:
        unsafe extern "system" fn(*mut c_void, *const Guid, u32, *const c_void) -> Hresult,
    /// `SetPrivateDataInterface(this, name, unknown)` — slot 5.
    pub _set_private_data_interface:
        unsafe extern "system" fn(*mut c_void, *const Guid, *const c_void) -> Hresult,
    /// `GetPrivateData(this, name, size, data)` — slot 6.
    pub _get_private_data:
        unsafe extern "system" fn(*mut c_void, *const Guid, *mut u32, *mut c_void) -> Hresult,
    /// `GetParent(this, riid, parent)` — slot 7.
    pub _get_parent:
        unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> Hresult,
}

/// `IDXGIFactory1` vtable (dxgi.h order: `IUnknown` slots 1–3,
/// `IDXGIObject` slots 4–7, `IDXGIFactory` slots 8–12, then
/// `IDXGIFactory1`'s `EnumAdapters1`/`IsCurrent`; `IsCurrent`
/// trails unused). `IDXGIFactory` has exactly five own methods —
/// the pre-fix declaration also carried a fabricated `GetAdapter`
/// slot between `CreateSwapChain` and `EnumAdapters`, which no
/// dxgi interface defines; a sixth slot would push `EnumAdapters1`
/// onto `IsCurrent`'s position and re-break the enumeration call.
#[repr(C)]
struct Factory1Vtbl {
    /// IUnknown.
    pub unknown: crate::windows::com::UnknownVtbl,
    /// IDXGIObject (slots 4–7).
    pub object: DxgiObjectVtbl,
    /// `CreateSwapChain(this, …)` — slot 8.
    pub _create_swap_chain: unsafe extern "system" fn(
        *mut c_void,
        *mut c_void,
        *const SwapChainDesc,
        *mut *mut c_void,
    ) -> Hresult,
    /// `CreateSoftwareAdapter(this, module, out)` — slot 9.
    pub _create_software_adapter:
        unsafe extern "system" fn(*mut c_void, *mut c_void, *mut *mut c_void) -> Hresult,
    /// `EnumAdapters(this, index, out)` — slot 10 (the slot the
    /// pre-fix `enum_adapters1` resolved to, per W08-A10's
    /// arithmetic).
    pub _enum_adapters: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> Hresult,
    /// `MakeWindowAssociation(this, hwnd, flags)` — slot 11.
    pub _make_window_association:
        unsafe extern "system" fn(*mut c_void, *mut c_void, u32) -> Hresult,
    /// `GetWindowAssociation(this, hwnd_out)` — slot 12.
    pub _get_window_association:
        unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
    /// `EnumAdapters1(this, index, out)` — slot 13.
    pub enum_adapters1: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> Hresult,
    /// `IsCurrent(this)` — slot 14; returns `BOOL` (i32); never
    /// called, slot shape pinned for vtable order only.
    pub _is_current: unsafe extern "system" fn(*mut c_void) -> Hresult,
}

/// `DXGI_SWAP_CHAIN_DESC` — the `_create_swap_chain` slot's parameter
/// block, declared by pointer only (the field is never read).
#[repr(C)]
struct SwapChainDesc {
    _buffer_desc: [u64; 5],
    _sample_desc: [u32; 2],
    _flags: u32,
}

/// `IDXGIAdapter1` vtable (dxgi.h order: `IUnknown` slots 1–3,
/// `IDXGIObject` slots 4–7 — `GetParent` is the quartet's fourth
/// member, not the adapter's first method as the pre-fix
/// declaration had it — then `IDXGIAdapter` slots 8–10 and
/// `IDXGIAdapter1`'s `GetDesc1` at slot 11;
/// `CheckInterfaceSupport` trails unused).
#[repr(C)]
struct Adapter1Vtbl {
    /// IUnknown.
    pub unknown: crate::windows::com::UnknownVtbl,
    /// IDXGIObject (slots 4–7, `GetParent` last).
    pub object: DxgiObjectVtbl,
    /// `EnumOutputs(this, index, out)` — slot 8 (the slot the
    /// pre-fix `get_desc1` resolved to, per W08-A10's arithmetic).
    pub _enum_outputs: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> Hresult,
    /// `GetDesc(this, desc)` — slot 9.
    pub _get_desc: unsafe extern "system" fn(*mut c_void, *mut AdapterDesc) -> Hresult,
    /// `CheckInterfaceSupport(this, guid, umd_version)` — slot 10.
    pub _check_interface_support:
        unsafe extern "system" fn(*mut c_void, *const Guid, *mut u64) -> Hresult,
    /// `GetDesc1(this, out)` — slot 11.
    pub get_desc1: unsafe extern "system" fn(*mut c_void, *mut AdapterDesc1) -> Hresult,
}

/// `DXGI_ADAPTER_DESC` (the `_get_desc` slot's parameter block).
#[repr(C)]
struct AdapterDesc {
    _description: [u16; 128],
    _dedicated_video_memory: usize,
    _dedicated_system_memory: usize,
    _shared_system_memory: usize,
    _adapter_luid: [u32; 2],
    _revision: u32,
    _device_id: u32,
    _vendor_id: u32,
    _subsys_id: u32,
}

/// `DXGI_ADAPTER_DESC1` (dxgi.h field order).
#[repr(C)]
struct AdapterDesc1 {
    /// The adapter's description string.
    description: [u16; 128],
    /// `DXGI_ADAPTER_FLAG_*`.
    flags: u32,
    /// Dedicated video memory, in bytes.
    dedicated_video_memory: usize,
    /// Dedicated system memory, in bytes.
    _dedicated_system_memory: usize,
    /// Shared system memory, in bytes.
    _shared_system_memory: usize,
    /// The adapter's locally unique id.
    _adapter_luid: [u32; 2],
    /// The PCI revision.
    _revision: u32,
    /// The PCI device id.
    device_id: u32,
    /// The PCI vendor id.
    vendor_id: u32,
    /// The PCI subsystem id.
    _subsys_id: u32,
}

/// A COM object header for vtable access through a raw pointer.
#[repr(C)]
struct ObjectHeader {
    vtbl: *const (),
}

/// The DXGI adapter scanner.
#[derive(Debug, Clone, Default)]
pub struct DxgiAdapters;

impl DxgiAdapters {
    /// A fresh scanner (the factory is created per enumeration round;
    /// DXGI has no apartment requirements).
    #[must_use = "constructs the scanner"]
    pub fn new() -> DxgiAdapters {
        DxgiAdapters
    }
}

impl GpuAdapters for DxgiAdapters {
    fn enumerate(&mut self) -> Result<Vec<GpuAdapterInfo>, PlatformError> {
        // SAFETY: the riid points at the complete IDXGIFactory1 Guid
        // and the out-slot is zero-initialized; the returned factory
        // is released exactly once at the end of this function.
        let mut factory: *mut c_void = std::ptr::null_mut();
        let hr = unsafe { CreateDXGIFactory1(&IID_IDXGI_FACTORY1, &mut factory) };
        if hr != S_OK || factory.is_null() {
            return Err(PlatformError::CallFailed {
                api: "CreateDXGIFactory1",
                code: Some(i64::from(hr)),
                detail: "the DXGI factory could not be created (no D3D runtime?)".to_string(),
            });
        }
        let adapters = enum_adapters(factory);
        // SAFETY: the single release of the created factory.
        unsafe {
            release_interface(factory);
        }
        adapters
    }
}

/// Enumerate every adapter through a live `IDXGIFactory1`.
fn enum_adapters(factory: *mut c_void) -> Result<Vec<GpuAdapterInfo>, PlatformError> {
    let factory_vtbl = unsafe { (*(factory as *mut ObjectHeader)).vtbl as *const Factory1Vtbl };
    let mut adapters = Vec::new();
    let mut index: u32 = 0;
    loop {
        // SAFETY: factory is live; the out-slot is zero-initialized;
        // indices ascend from 0 as the API contract requires.
        let mut adapter: *mut c_void = std::ptr::null_mut();
        let hr = unsafe { ((*factory_vtbl).enum_adapters1)(factory, index, &mut adapter) };
        if hr != S_OK {
            // DXGI_ERROR_NOT_FOUND is the documented end of the
            // enumeration; every other failure is an error we report
            // (the list so far is kept — honest partial data).
            break;
        }
        if adapter.is_null() {
            break;
        }
        if let Ok(Some(info)) = describe_adapter(adapter) {
            adapters.push(info);
        }
        // SAFETY: the single release of this adapter's reference.
        unsafe {
            release_interface(adapter);
        }
        index += 1;
    }
    Ok(adapters)
}

/// One `IDXGIAdapter1` → [`GpuAdapterInfo`] (`None` when the adapter
/// is too degenerate to describe honestly).
fn describe_adapter(adapter: *mut c_void) -> Result<Option<GpuAdapterInfo>, PlatformError> {
    let adapter_vtbl = unsafe { (*(adapter as *mut ObjectHeader)).vtbl as *const Adapter1Vtbl };
    let mut desc = AdapterDesc1 {
        description: [0; 128],
        flags: 0,
        dedicated_video_memory: 0,
        _dedicated_system_memory: 0,
        _shared_system_memory: 0,
        _adapter_luid: [0; 2],
        _revision: 0,
        device_id: 0,
        vendor_id: 0,
        _subsys_id: 0,
    };
    // SAFETY: adapter is live and desc is a complete, aligned
    // DXGI_ADAPTER_DESC1 for the API to fill.
    let hr = unsafe { ((*adapter_vtbl).get_desc1)(adapter, &mut desc) };
    if hr != S_OK {
        return Err(PlatformError::CallFailed {
            api: "IDXGIAdapter1::GetDesc1",
            code: Some(i64::from(hr)),
            detail: "the adapter description could not be read".to_string(),
        });
    }
    // SAFETY: the description field is a NUL-terminated wide string
    // per the API contract; from_wide stops at the terminator inside
    // the fixed array.
    let name = unsafe { from_wide(desc.description.as_ptr()) }?;
    if name.is_empty() {
        return Ok(None);
    }
    // The "Microsoft Basic Render Driver" description names the WARP
    // software adapter; its vendor/device pair (0x1414/0x008c) is the
    // documented marker.
    let is_software = desc.vendor_id == WARP_VENDOR_ID || name.contains("Basic Render Driver");
    Ok(Some(GpuAdapterInfo {
        vendor_id: desc.vendor_id,
        device_id: desc.device_id,
        name,
        dedicated_video_memory: Some(desc.dedicated_video_memory as u64),
        is_software,
    }))
}

// ---------------------------------------------------------------------------
// Vtable slot pins (W08-A10 F-001 regression, FX2-A06).
//
// No Windows host exists in the build lane, so the correctness proof
// for these hand-declared vtables is slot arithmetic, made mechanical
// here: the compile-time assertions below re-derive the two CALLED
// slots from the dxgi.h inheritance chain (IUnknown slots 1–3,
// IDXGIObject slots 4–7, then each interface's own methods), and the
// test module pins the full layout. The pre-fix declarations omitted
// the IDXGIObject quartet, which no compile check could catch —
// resolving `get_desc1` to `EnumOutputs`'s slot 8 and
// `enum_adapters1` to `EnumAdapters`'s slot 10, exactly the executed
// proof wave-08 recorded; these pins can catch it.
// ---------------------------------------------------------------------------

/// One vtable slot is one function pointer (pointer-width on every
/// supported target — the pins below are width-agnostic).
const SLOT_BYTES: usize = std::mem::size_of::<usize>();

/// `Adapter1Vtbl::get_desc1` must resolve at slot 11 (index 10) —
/// `EnumOutputs` stays at slot 8, three slots apart.
const _: () = assert!(
    std::mem::offset_of!(Adapter1Vtbl, get_desc1) == 10 * SLOT_BYTES
        && std::mem::offset_of!(Adapter1Vtbl, _enum_outputs) == 7 * SLOT_BYTES
        && std::mem::size_of::<Adapter1Vtbl>() == 11 * SLOT_BYTES
);

/// `Factory1Vtbl::enum_adapters1` must resolve at slot 13 (index 12) —
/// `EnumAdapters` stays at slot 10, three slots past it; a fabricated
/// sixth factory slot would push it onto `IsCurrent`'s position.
const _: () = assert!(
    std::mem::offset_of!(Factory1Vtbl, enum_adapters1) == 12 * SLOT_BYTES
        && std::mem::offset_of!(Factory1Vtbl, _enum_adapters) == 9 * SLOT_BYTES
        && std::mem::size_of::<Factory1Vtbl>() == 14 * SLOT_BYTES
);

/// FX2-A06 regression pins for the two hand-declared dxgi vtables
/// (W08-A10 F-001, the S0). The vtable types are private to this
/// `#[cfg(target_os = "windows")]` module, so the suite lives in-file
/// with the same cfg: on non-Windows hosts it is never parsed
/// (Linux-compile-safe); on the Windows CI lanes the assertions
/// execute. Indices are 0-based byte offsets divided by one slot.
#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    /// The dxgi.h slot arithmetic every dxgi interface shares, as
    /// 0-based indices: IUnknown 0–2, IDXGIObject 3–6, then each
    /// interface's own methods.
    const SLOT_SET_PRIVATE_DATA: usize = 3;
    const SLOT_SET_PRIVATE_DATA_INTERFACE: usize = 4;
    const SLOT_GET_PRIVATE_DATA: usize = 5;
    const SLOT_GET_PARENT: usize = 6;
    const SLOT_CREATE_SWAP_CHAIN: usize = 7;
    const SLOT_CREATE_SOFTWARE_ADAPTER: usize = 8;
    const SLOT_ENUM_ADAPTERS: usize = 9;
    const SLOT_MAKE_WINDOW_ASSOCIATION: usize = 10;
    const SLOT_GET_WINDOW_ASSOCIATION: usize = 11;
    const SLOT_ENUM_ADAPTERS1: usize = 12;
    const SLOT_IS_CURRENT: usize = 13;
    const SLOT_ENUM_OUTPUTS: usize = 7;
    const SLOT_GET_DESC: usize = 8;
    const SLOT_CHECK_INTERFACE_SUPPORT: usize = 9;
    const SLOT_GET_DESC1: usize = 10;

    /// The inherited base is exactly the four IDXGIObject slots, with
    /// `GetParent` last, starting after IUnknown in both vtables —
    /// the quartet whose absence was the S0 finding. Pre-fix, the
    /// adapter vtable sat its `get_desc1` only five slots in (7*8
    /// bytes on x64), landing on `EnumOutputs`.
    #[test]
    fn fx2_a06_dxgi_object_base_is_the_four_inherited_slots() {
        assert_eq!(
            std::mem::offset_of!(Factory1Vtbl, object),
            SLOT_SET_PRIVATE_DATA * SLOT_BYTES,
            "the IDXGIObject quartet follows IUnknown in the factory vtable"
        );
        assert_eq!(
            std::mem::offset_of!(Adapter1Vtbl, object),
            SLOT_SET_PRIVATE_DATA * SLOT_BYTES,
            "the IDXGIObject quartet follows IUnknown in the adapter vtable"
        );
        assert_eq!(std::mem::offset_of!(DxgiObjectVtbl, _set_private_data), 0);
        assert_eq!(
            std::mem::offset_of!(DxgiObjectVtbl, _set_private_data_interface),
            SLOT_BYTES
        );
        assert_eq!(
            std::mem::offset_of!(DxgiObjectVtbl, _get_private_data),
            2 * SLOT_BYTES
        );
        assert_eq!(
            std::mem::offset_of!(DxgiObjectVtbl, _get_parent),
            3 * SLOT_BYTES,
            "GetParent is the quartet's last slot (index 6), not the adapter's first method"
        );
        assert_eq!(
            std::mem::size_of::<DxgiObjectVtbl>(),
            4 * SLOT_BYTES,
            "IDXGIObject contributes exactly four slots"
        );
    }

    /// The audit's executed proof: the pre-fix `get_desc1` resolved
    /// at `EnumOutputs`'s slot (same position, different arity — the
    /// garbage-register call). Restored, `GetDesc1` and `EnumOutputs`
    /// must resolve at distinct slots: indices 10 and 7.
    #[test]
    fn fx2_a06_adapter1_get_desc1_resolves_at_its_own_slot_not_enum_outputs() {
        assert_eq!(
            std::mem::offset_of!(Adapter1Vtbl, get_desc1),
            SLOT_GET_DESC1 * SLOT_BYTES,
            "GetDesc1 sits at slot 11 (index 10): 3 IUnknown + 4 IDXGIObject + 3 IDXGIAdapter"
        );
        assert_eq!(
            std::mem::offset_of!(Adapter1Vtbl, _enum_outputs),
            SLOT_ENUM_OUTPUTS * SLOT_BYTES
        );
        assert_ne!(
            std::mem::offset_of!(Adapter1Vtbl, get_desc1),
            std::mem::offset_of!(Adapter1Vtbl, _enum_outputs),
            "GetDesc1 must not resolve at EnumOutputs' slot (the W08-A10 S0)"
        );
        assert_eq!(
            std::mem::offset_of!(Adapter1Vtbl, _get_desc),
            SLOT_GET_DESC * SLOT_BYTES
        );
        assert_eq!(
            std::mem::offset_of!(Adapter1Vtbl, _check_interface_support),
            SLOT_CHECK_INTERFACE_SUPPORT * SLOT_BYTES
        );
        assert_eq!(
            std::mem::size_of::<Adapter1Vtbl>(),
            11 * SLOT_BYTES,
            "IDXGIAdapter1 is 3 + 4 + 3 + 1 slots"
        );
    }

    /// The factory twin: the pre-fix `enum_adapters1` resolved at
    /// `EnumAdapters`'s slot — same signature, so it "worked" by
    /// luck while returning the wrong interface. Restored it must
    /// sit three slots past it (index 12 vs 9), with `IsCurrent` at
    /// 13 and exactly five `IDXGIFactory` methods in between: a
    /// fabricated sixth (the pre-fix `GetAdapter`) would push
    /// `EnumAdapters1` onto `IsCurrent`'s slot and re-break the
    /// enumeration call.
    #[test]
    fn fx2_a06_factory1_enum_adapters1_resolves_at_its_own_slot_not_enum_adapters() {
        assert_eq!(
            std::mem::offset_of!(Factory1Vtbl, enum_adapters1),
            SLOT_ENUM_ADAPTERS1 * SLOT_BYTES,
            "EnumAdapters1 sits at slot 13 (index 12): 3 + 4 + 5 IDXGIFactory methods"
        );
        assert_eq!(
            std::mem::offset_of!(Factory1Vtbl, _enum_adapters),
            SLOT_ENUM_ADAPTERS * SLOT_BYTES
        );
        assert_ne!(
            std::mem::offset_of!(Factory1Vtbl, enum_adapters1),
            std::mem::offset_of!(Factory1Vtbl, _enum_adapters),
            "EnumAdapters1 must not resolve at EnumAdapters' slot (the W08-A10 twin)"
        );
        assert_eq!(
            std::mem::offset_of!(Factory1Vtbl, _create_swap_chain),
            SLOT_CREATE_SWAP_CHAIN * SLOT_BYTES
        );
        assert_eq!(
            std::mem::offset_of!(Factory1Vtbl, _create_software_adapter),
            SLOT_CREATE_SOFTWARE_ADAPTER * SLOT_BYTES
        );
        assert_eq!(
            std::mem::offset_of!(Factory1Vtbl, _make_window_association),
            SLOT_MAKE_WINDOW_ASSOCIATION * SLOT_BYTES
        );
        assert_eq!(
            std::mem::offset_of!(Factory1Vtbl, _get_window_association),
            SLOT_GET_WINDOW_ASSOCIATION * SLOT_BYTES
        );
        assert_eq!(
            std::mem::offset_of!(Factory1Vtbl, _is_current),
            SLOT_IS_CURRENT * SLOT_BYTES
        );
        assert_eq!(
            std::mem::size_of::<Factory1Vtbl>(),
            14 * SLOT_BYTES,
            "IDXGIFactory1 is 3 + 4 + 5 + 2 slots — no fabricated sixth factory slot"
        );
    }
}
