//! DRM/KMS adapter enumeration (card 7.A2) — pure std sysfs walking,
//! no `unsafe`, no ioctls.
//!
//! What the scanner reads (all best-effort, absence handled):
//!
//! - `/sys/class/drm/card*` and `/sys/class/drm/renderD*` — the
//!   adapter nodes the kernel published;
//! - `<node>/device/vendor` and `<node>/device/device` — PCI ids as
//!   `0x`-prefixed hex text;
//! - `/dev/dri/<node>` — the device node a userspace driver would
//!   actually open (cards without one are reported but flagged
//!   `device_node: false` — see [`DrmNode`]).
//!
//! `[VERIFY]` The wgpu integration point: when renderer-wgpu's wgpu
//! import is admitted, this enumeration is the hardware-presence
//! input to the 7.C1 "auto-negotiated wgpu backend → software
//! present" fallback chain; the scanner deliberately does not create
//! GPU contexts (that is wgpu's own job).

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::PlatformError;
use crate::gpu::{GpuAdapterInfo, GpuAdapters};

/// The sysfs/devfs roots, injectable for deterministic tests.
#[derive(Debug, Clone)]
pub struct DrmAdapterScanner {
    sys_root: PathBuf,
    dev_root: PathBuf,
}

impl DrmAdapterScanner {
    /// Scan the real system (`/sys`, `/dev`).
    #[must_use = "constructs the scanner"]
    pub fn new() -> DrmAdapterScanner {
        DrmAdapterScanner {
            sys_root: PathBuf::from("/"),
            dev_root: PathBuf::from("/"),
        }
    }

    /// Scan a fixture tree (the test seam: `sys_root` carries
    /// `sys/class/drm/…`, `dev_root` carries `dev/dri/…`).
    #[must_use = "constructs the fixture-driven scanner"]
    pub fn with_roots(sys_root: &Path, dev_root: &Path) -> DrmAdapterScanner {
        DrmAdapterScanner {
            sys_root: sys_root.to_path_buf(),
            dev_root: dev_root.to_path_buf(),
        }
    }

    /// The sysfs root being scanned.
    #[must_use = "pure accessor"]
    pub fn sys_root(&self) -> &Path {
        &self.sys_root
    }
}

impl Default for DrmAdapterScanner {
    fn default() -> Self {
        DrmAdapterScanner::new()
    }
}

impl GpuAdapters for DrmAdapterScanner {
    fn enumerate(&mut self) -> Result<Vec<GpuAdapterInfo>, PlatformError> {
        let drm_class = self.sys_root.join("sys/class/drm");
        let entries = match fs::read_dir(&drm_class) {
            Ok(entries) => entries,
            // A kernel without DRM (or a container without sysfs)
            // honestly has no adapters: empty, not an error.
            Err(_) => return Ok(Vec::new()),
        };
        let mut adapters = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            // Version-scratch entries (`version`, `minorN`) are not
            // adapter nodes.
            if !(name.starts_with("card") || name.starts_with("renderD")) {
                continue;
            }
            if let Some(info) = scan_node(&entry.path(), &self.dev_root.join("dev/dri"), &name) {
                adapters.push(info);
            }
        }
        adapters.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(adapters)
    }
}

/// One scanned node → [`GpuAdapterInfo`] (`None` when the node is so
/// degenerate it cannot be described honestly).
fn scan_node(sys_node: &Path, dri_root: &Path, name: &str) -> Option<GpuAdapterInfo> {
    let device_dir = sys_node.join("device");
    let vendor_id = read_hex(&device_dir.join("vendor"));
    let device_id = read_hex(&device_dir.join("device"));
    // The userspace-openable node: presence distinguishes a usable
    // adapter from a sysfs ghost (e.g. a driverless card).
    let node_exists = dri_root.join(name).exists();
    let vendor = vendor_id.unwrap_or(0);
    // The software-adapter vendor set: Bochs/QEMU, VMware and
    // virtio-driven display devices (llvmpipe/softpipe userspace on
    // top of them) — classified by PCI vendor, the only stable
    // signal sysfs carries.
    let is_software = matches!(vendor, 0x1234 | 0x15ad | 0x1b36 | 0x1af4);
    let display = format!(
        "DRM {name} ({} vendor {:#06x} device {:#06x}{})",
        vendor_label(vendor),
        vendor,
        device_id.unwrap_or(0),
        if node_exists { "" } else { ", no device node" }
    );
    Some(GpuAdapterInfo {
        vendor_id: vendor,
        device_id: device_id.unwrap_or(0),
        name: display,
        // sysfs does not expose VRAM size (debugfs-only, root);
        // `None` is the honest answer.
        dedicated_video_memory: None,
        is_software,
    })
}

/// Read a `0x`-prefixed hex text file as a u32 (absence → `None`).
fn read_hex(path: &Path) -> Option<u32> {
    let text = fs::read_to_string(path).ok()?;
    let trimmed = text.trim().trim_start_matches("0x");
    u32::from_str_radix(trimmed, 16).ok()
}

/// Vendor label (the shared well-known-PCI table).
fn vendor_label(vendor: u32) -> &'static str {
    crate::gpu::pci_vendor_name(vendor)
}
