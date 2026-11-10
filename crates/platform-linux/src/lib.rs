//! `platform-linux` — the Linux platform adapter (card 7.A2, issue
//! #40; doc 01 §2.7 L5 registry row).
//!
//! One of the three per-OS adapter crates (with `platform-windows`
//! and `platform-macos`). Its shape follows the doc 01 §2.7 template:
//! it is the audited home of MP §9's OS-service exceptions for Linux
//! — the only `unsafe` in this crate lives in the FFI modules
//! ([`linux`]), every block safety-commented — it implements seams
//! defined against the core vocabulary ([`audio_output`]'s
//! `DeviceDesc`/`DeviceId`, doc 01 §2.6, ADR-005), and it never leaks
//! OS types upward (ARCH-6).
//!
//! # The typed-seam design (why the crate is honest on any machine)
//!
//! Each OS integration sits behind a **seam trait**
//! ([`audio::AudioDevices`], [`gpu::GpuAdapters`],
//! [`dialog::FileDialogs`]) with a typed fallback
//! ([`audio::UnavailableAudioDevices`], …). [`LinuxPlatform::discover`]
//! probes the host and wires either the real integration or the typed
//! fallback carrying the exact reason — capability negotiation is a
//! first-class outcome ([`capability::PlatformReport`]), never a
//! guessed success (MP §13 "no invented crate capabilities").
//!
//! # Runtime reality of this crate's integrations
//!
//! - **PulseAudio** (audio device enumeration + change notifications):
//!   the client library is `dlopen`ed at runtime (no link-time
//!   dependency — [`linux::dl`]), so machines without it get a typed
//!   [`PlatformError::LibraryUnavailable`]; machines with the library
//!   but no server get a typed `ServiceUnavailable`. Real-server
//!   behavior is `[VERIFY]` (this sandbox has libpulse but no
//!   PulseAudio daemon; the absent-server path is what is
//!   runtime-tested here).
//! - **GPU**: DRM/KMS sysfs scanning (pure std, no `unsafe`). The
//!   wgpu adapter itself stays behind `renderer-wgpu`'s Lead-gated
//!   import (`docs/dependencies/records/wgpu-30.0.1.md`); this seam
//!   answers *what hardware is present* (the 7.C1 fallback-chain
//!   input), not *create a GPU context*.
//! - **Dialogs**: xdg-desktop-portal session-bus probe (std) with a
//!   zenity/kdialog command fallback chain. The portal's FileChooser
//!   D-Bus client is a recorded `[PROPOSAL]` follow-up
//!   ([`linux::xdg::portal_filechooser_implemented`]).
//!
//! The seam vocabulary (error/capability/events) is deliberately
//! mirrored across the three `platform-*` crates for the Lead to
//! unify into a shared L5 crate later — `[PROPOSAL]` (creating a
//! fourth crate was outside this card's file ownership).

pub mod audio;
pub mod capability;
pub mod dialog;
pub mod error;
pub mod events;
pub mod gpu;

/// The Linux integrations (cfg-gated: on any other OS every seam is
/// the typed fallback and the report says `WrongOs` honestly).
#[cfg(target_os = "linux")]
pub mod linux;

use audio::{AudioDevices, UnavailableAudioDevices};
use capability::{Capability, Feature, PlatformReport};
use dialog::{FileDialogs, UnavailableDialogs};
use error::PlatformError;
use gpu::{GpuAdapters, UnavailableGpuAdapters};

/// The assembled Linux platform adapter: one probe, three seams, one
/// honest capability report (card 7.A2's entry point).
///
/// Construct via [`LinuxPlatform::discover`] — it performs the live
/// probes and wires either each real integration or its typed
/// fallback. The accessor methods hand out `dyn` seams so callers
/// never branch on which one they got: a call that cannot work
/// returns a typed [`PlatformError`], it never fakes success.
pub struct LinuxPlatform {
    audio: Box<dyn AudioDevices>,
    gpu: Box<dyn GpuAdapters>,
    dialogs: Box<dyn FileDialogs>,
    report: PlatformReport,
}

impl LinuxPlatform {
    /// Probe the host and assemble the best available seams.
    ///
    /// Probes run: PulseAudio client-library load + server connect
    /// (one full enumeration round — `Available` means enumeration
    /// works, not merely that the library is loadable); DRM sysfs
    /// presence; zenity/kdialog discovery plus the portal-bus
    /// reachability check. Each probe's outcome lands verbatim in
    /// the returned [`PlatformReport`] (see
    /// [`report`](LinuxPlatform::report)).
    #[must_use = "probing has side effects on OS handles; reuse the value"]
    pub fn discover() -> LinuxPlatform {
        let mut report = PlatformReport::new();
        let (audio, audio_capability) = audio_seam();
        let (gpu, gpu_capability) = gpu_seam();
        let (dialogs, dialogs_capability) = dialog_seam();
        // The PulseAudio client provides enumeration and notifications
        // together (same connection); one verdict covers both rows.
        report.set(Feature::AudioEnumeration, audio_capability.clone());
        report.set(Feature::AudioNotifications, audio_capability);
        report.set(Feature::GpuEnumeration, gpu_capability);
        report.set(Feature::FileDialogs, dialogs_capability);
        LinuxPlatform { audio, gpu, dialogs, report }
    }

    /// The capability table computed at discovery time.
    #[must_use = "pure accessor"]
    pub fn report(&self) -> &PlatformReport {
        &self.report
    }

    /// The audio device seam (real or typed fallback).
    pub fn audio(&mut self) -> &mut dyn AudioDevices {
        &mut *self.audio
    }

    /// The GPU adapter seam (real or typed fallback).
    pub fn gpu(&mut self) -> &mut dyn GpuAdapters {
        &mut *self.gpu
    }

    /// The file-dialog seam (real or typed fallback).
    pub fn dialogs(&mut self) -> &mut dyn FileDialogs {
        &mut *self.dialogs
    }
}

/// The audio seam builder: real client when the library loads, typed
/// fallback with the loader's diagnosis otherwise.
fn audio_seam() -> (Box<dyn AudioDevices>, Capability) {
    #[cfg(target_os = "linux")]
    {
        // The probe is a full enumeration round, not just the library
        // load: `Available` must mean "enumeration works end to end"
        // (a loadable library with a dead server is the typed
        // ServiceNotReachable verdict — the honest-negotiation rule
        // the other two lanes follow with their own probe rounds).
        match linux::pulse::PulseAudioDevices::probe() {
            Ok(devices) => {
                let mut devices = devices;
                match devices.enumerate() {
                    Ok(_) => (Box::new(devices), Capability::Available),
                    Err(err) => {
                        let capability = Capability::from_probe_error(&err);
                        (Box::new(UnavailableAudioDevices::because(err)), capability)
                    }
                }
            }
            Err(err) => {
                let capability = Capability::from_probe_error(&err);
                (Box::new(UnavailableAudioDevices::because(err)), capability)
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let reason = PlatformError::NotCompiledForThisOs { crate_os: "linux" };
        let capability = Capability::from_probe_error(&reason);
        (Box::new(UnavailableAudioDevices::because(reason)), capability)
    }
}

/// The GPU seam builder: the sysfs scanner (a kernel without DRM
/// enumerates as an honest empty list), typed fallback only when
/// scanning cannot run at all.
fn gpu_seam() -> (Box<dyn GpuAdapters>, Capability) {
    #[cfg(target_os = "linux")]
    {
        let mut scanner = linux::drm::DrmAdapterScanner::new();
        match scanner.enumerate() {
            Ok(_) => (Box::new(scanner), Capability::Available),
            Err(err) => {
                let capability = Capability::from_probe_error(&err);
                (Box::new(UnavailableGpuAdapters::because(err)), capability)
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let reason = PlatformError::NotCompiledForThisOs { crate_os: "linux" };
        let capability = Capability::from_probe_error(&reason);
        (Box::new(UnavailableGpuAdapters::because(reason)), capability)
    }
}

/// The dialog seam builder: the xdg chain when any command backend
/// resolves, the typed fallback naming everything that was tried
/// otherwise (the portal FileChooser client is the recorded
/// follow-up — [`linux::xdg::portal_filechooser_implemented`] on
/// Linux targets).
fn dialog_seam() -> (Box<dyn FileDialogs>, Capability) {
    #[cfg(target_os = "linux")]
    {
        let dialogs = linux::xdg::XdgDialogs::discover();
        if dialogs.zenity().is_some() || dialogs.kdialog().is_some() {
            (Box::new(dialogs), Capability::Available)
        } else {
            let reason = PlatformError::ServiceUnavailable {
                service: "file dialogs",
                detail: format!(
                    "no dialog backend on this host: {}",
                    if linux::xdg::portal_filechooser_implemented() {
                        "portal FileChooser client is implemented; see its probe"
                    } else {
                        "portal FileChooser client not implemented ([PROPOSAL] \
                         D-Bus follow-up) and neither zenity nor kdialog is on PATH"
                    },
                ),
            };
            let capability = Capability::from_probe_error(&reason);
            (Box::new(UnavailableDialogs::because(reason)), capability)
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let reason = PlatformError::NotCompiledForThisOs { crate_os: "linux" };
        let capability = Capability::from_probe_error(&reason);
        (Box::new(UnavailableDialogs::because(reason)), capability)
    }
}
