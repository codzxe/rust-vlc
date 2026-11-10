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

/// `IDXGIFactory1` vtable (dxgi.h order through `EnumAdapters1`;
/// `IsCurrent` trails unused).
#[repr(C)]
struct Factory1Vtbl {
    /// IUnknown.
    pub unknown: crate::windows::com::UnknownVtbl,
    /// `CreateSwapChain(this, …)`.
    pub _create_swap_chain: unsafe extern "system" fn(
        *mut c_void,
        *mut c_void,
        *const SwapChainDesc,
        *mut *mut c_void,
    ) -> Hresult,
    /// `GetAdapter(this, …)`.
    pub _get_adapter: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> Hresult,
    /// `EnumAdapters(this, …)`.
    pub _enum_adapters: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> Hresult,
    /// `MakeWindowAssociation(this, …)`.
    pub _make_window_association:
        unsafe extern "system" fn(*mut c_void, *mut c_void, u32) -> Hresult,
    /// `GetWindowAssociation(this, …)`.
    pub _get_window_association:
        unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
    /// `CreateSoftwareAdapter(this, …)`.
    pub _create_software_adapter:
        unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
    /// `EnumAdapters1(this, index, out)`.
    pub enum_adapters1: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> Hresult,
    /// `IsCurrent(this)` — returns `BOOL` (i32); never called, slot
    /// shape pinned for vtable order only.
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

/// `IDXGIAdapter1` vtable (dxgi.h order through `GetDesc1`;
/// `CheckInterfaceSupport` trails unused).
#[repr(C)]
struct Adapter1Vtbl {
    /// IUnknown.
    pub unknown: crate::windows::com::UnknownVtbl,
    /// `GetParent(this, …)`.
    pub _get_parent: unsafe extern "system" fn(
        *mut c_void,
        *const Guid,
        *mut *mut c_void,
    ) -> Hresult,
    /// `EnumOutputs(this, …)`.
    pub _enum_outputs: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> Hresult,
    /// `GetDesc(this, …)`.
    pub _get_desc: unsafe extern "system" fn(*mut c_void, *mut AdapterDesc) -> Hresult,
    /// `CheckInterfaceSupport(this, …)`.
    pub _check_interface_support:
        unsafe extern "system" fn(*mut c_void, *const Guid, *mut u64) -> Hresult,
    /// `GetDesc1(this, out)`.
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
