//! The GPU seam (card 7.A1): adapter enumeration.
//!
//! The Windows lane enumerates **DXGI adapters**
//! (`IDXGIFactory1::EnumAdapters1` + `IDXGIAdapter1::GetDesc1` in
//! [`crate::windows::dxgi`]) — the PCI ids, description strings and
//! dedicated-video-memory sizes, with the WARP software rasterizer
//! flagged. The wgpu device/adapter itself belongs to
//! `renderer-wgpu`'s Lead-gated wgpu import
//! (records/wgpu-30.0.1.md): this seam answers "*what hardware is
//! present*" (the auto-negotiation input for the 7.C1 fallback chain
//! "wgpu backend → software present"), not "*create a GPU context*".
//!
//! `[VERIFY]` The in-tree wgpu adapter (renderer-wgpu v0 does not
//! import wgpu) will consume these descriptors when its admission
//! lands; the field set is pinned to what the three OS lanes can
//! honestly produce today.

use crate::error::PlatformError;

/// One enumerable graphics adapter, OS vocabulary translated
/// (ARCH-6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuAdapterInfo {
    /// PCI vendor id where the OS reports one (DRM sysfs, DXGI);
    /// `0` when the API has no PCI notion (Metal reports a
    /// registry id instead).
    pub vendor_id: u32,
    /// PCI device id where the OS reports one; Metal's `registryID`
    /// (an IORegistry handle, not PCI) — documented per-lane.
    pub device_id: u32,
    /// Human-readable adapter name (the OS's own string).
    pub name: String,
    /// Dedicated video memory in bytes, when the OS reports it.
    pub dedicated_video_memory: Option<u64>,
    /// Whether the OS flags this adapter as software (DXGI WARP,
    /// virtio nodes are classified by PCI vendor — documented
    /// per-lane).
    pub is_software: bool,
}

impl GpuAdapterInfo {
    /// The vendor's common name for well-known PCI vendor ids
    /// (best-effort labeling; unknown ids stay numeric).
    #[must_use = "pure classification"]
    pub fn vendor_name(&self) -> &'static str {
        pci_vendor_name(self.vendor_id)
    }
}

/// The well-known PCI vendor label table (best effort; `0` and
/// unknown ids → "unknown").
#[must_use = "pure classification"]
pub fn pci_vendor_name(vendor_id: u32) -> &'static str {
    match vendor_id {
        0x8086 => "Intel",
        0x1002 => "AMD",
        0x1022 => "AMD",
        0x10de => "NVIDIA",
        0x13b5 => "Arm",
        0x5143 => "Qualcomm",
        0x1010 => "Imagination",
        0x15ad => "VMware",
        0x1b36 => "VMware",
        0x1af4 => "Red Hat (virtio)",
        0x1234 => "Bochs (QEMU)",
        _ => "unknown",
    }
}

/// The GPU adapter seam.
pub trait GpuAdapters: Send {
    /// Adapters visible right now. An empty list is honest (headless
    /// machines have no DRM nodes; the 7.C1 chain falls back to
    /// software present).
    ///
    /// # Errors
    /// Typed [`PlatformError`] when enumeration cannot run at all
    /// (wrong OS, sysfs unreadable).
    fn enumerate(&mut self) -> Result<Vec<GpuAdapterInfo>, PlatformError>;
}

/// The typed fallback (mirrors [`crate::audio::UnavailableAudioDevices`]).
#[derive(Debug, Clone)]
pub struct UnavailableGpuAdapters {
    reason: PlatformError,
}

impl UnavailableGpuAdapters {
    /// Wrap the reason every method will return.
    #[must_use = "constructs the typed fallback"]
    pub fn because(reason: PlatformError) -> UnavailableGpuAdapters {
        UnavailableGpuAdapters { reason }
    }

    /// The reason this integration is unavailable.
    #[must_use = "pure accessor"]
    pub fn reason(&self) -> &PlatformError {
        &self.reason
    }
}

impl GpuAdapters for UnavailableGpuAdapters {
    fn enumerate(&mut self) -> Result<Vec<GpuAdapterInfo>, PlatformError> {
        Err(self.reason.clone())
    }
}
