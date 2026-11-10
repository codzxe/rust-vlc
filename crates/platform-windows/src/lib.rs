//! `platform-windows` — the Windows platform adapter (card 7.A1,
//! issue #40; doc 01 §2.7 L5 registry row).
//!
//! One of the three per-OS adapter crates (with `platform-linux`
//! and `platform-macos`). Its shape follows the doc 01 §2.7 template:
//! it is the audited home of MP §9's OS-service exceptions for
//! Windows — the only `unsafe` in this crate lives in the FFI
//! modules ([`windows`]), every block safety-commented — it
//! implements seams defined against the core vocabulary
//! ([`audio_output`]'s `DeviceDesc`/`DeviceId`, doc 01 §2.6,
//! ADR-005), and it never leaks OS types upward (ARCH-6).
//!
//! # The typed-seam design (why the crate is honest on any machine)
//!
//! Each OS integration sits behind a **seam trait**
//! ([`audio::AudioDevices`], [`gpu::GpuAdapters`],
//! [`dialog::FileDialogs`]) with a typed fallback
//! ([`audio::UnavailableAudioDevices`], …). [`WindowsPlatform::discover`]
//! probes the host and wires either the real integration or the typed
//! fallback carrying the exact reason — capability negotiation is a
//! first-class outcome ([`capability::PlatformReport`]), never a
//! guessed success (MP §13 "no invented crate capabilities").
//!
//! # Runtime reality of this crate's integrations
//!
//! - **WASAPI** (MMDevice API: render-endpoint enumeration + change
//!   notifications): hand-declared COM vtables over ole32, no
//!   `windows-s` crate. Enumeration runs on a dedicated short-lived
//!   STA thread; notifications run on a dedicated long-lived thread
//!   ([`windows::wasapi`]).
//! - **GPU**: DXGI `IDXGIFactory1::EnumAdapters1` ([`windows::dxgi`])
//!   — the PCI ids, names and VRAM sizes the 7.C1 fallback chain
//!   consumes; the wgpu context itself stays behind
//!   `renderer-wgpu`'s Lead-gated import.
//! - **Dialogs**: the Common Item Dialogs (`IFileOpenDialog`/
//!   `IFileSaveDialog`, [`windows::dialogs`]), each round on its own
//!   STA thread.
//!
//! **Evidence status (sandbox honesty):** this Linux sandbox has no
//! Windows SDK — the FFI surface is compile-verified via
//! `cargo check --target x86_64-pc-windows-msvc` (typechecked
//! declarations, no linking, no execution); runtime behavior on real
//! Windows hardware is `[VERIFY]`. The host-runnable tests cover the
//! typed seams, capability negotiation, fallbacks and event plumbing
//! with simulated inputs.
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

/// The Windows integrations (cfg-gated: on any other OS every seam
/// is the typed fallback and the report says `WrongOs` honestly).
#[cfg(target_os = "windows")]
pub mod windows;

use audio::{AudioDevices, UnavailableAudioDevices};
use capability::{Capability, Feature, PlatformReport};
use dialog::{FileDialogs, UnavailableDialogs};
use error::PlatformError;
use gpu::{GpuAdapters, UnavailableGpuAdapters};

/// The assembled Windows platform adapter: one probe, three seams,
/// one honest capability report (card 7.A1's entry point).
///
/// Construct via [`WindowsPlatform::discover`] — it performs the live
/// probes and wires either each real integration or its typed
/// fallback. The accessor methods hand out `dyn` seams so callers
/// never branch on which one they got: a call that cannot work
/// returns a typed [`PlatformError`], it never fakes success.
pub struct WindowsPlatform {
    audio: Box<dyn AudioDevices>,
    gpu: Box<dyn GpuAdapters>,
    dialogs: Box<dyn FileDialogs>,
    report: PlatformReport,
}

impl WindowsPlatform {
    /// Probe the host and assemble the best available seams.
    ///
    /// Probes run: one WASAPI enumeration round (the library is
    /// always present on Windows — the enumeration call itself is
    /// the probe); one DXGI factory round; one non-intrusive
    /// Common-Item-Dialog creation round. Each probe's outcome lands
    /// verbatim in the returned [`PlatformReport`].
    #[must_use = "probing has side effects on OS handles; reuse the value"]
    pub fn discover() -> WindowsPlatform {
        let mut report = PlatformReport::new();
        let (audio, audio_capability) = audio_seam();
        let (gpu, gpu_capability) = gpu_seam();
        let (dialogs, dialogs_capability) = dialog_seam();
        // The WASAPI client provides enumeration and notifications
        // together (the notification client registers against the
        // same enumerator); one verdict covers both rows.
        report.set(Feature::AudioEnumeration, audio_capability.clone());
        report.set(Feature::AudioNotifications, audio_capability);
        report.set(Feature::GpuEnumeration, gpu_capability);
        report.set(Feature::FileDialogs, dialogs_capability);
        WindowsPlatform { audio, gpu, dialogs, report }
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

/// The audio seam builder: the WASAPI client when a live
/// enumeration round succeeds, the typed fallback with the round's
/// diagnosis otherwise.
fn audio_seam() -> (Box<dyn AudioDevices>, Capability) {
    #[cfg(target_os = "windows")]
    {
        let mut devices = windows::wasapi::WasapiAudioDevices::new();
        match devices.enumerate() {
            Ok(_) => (Box::new(devices), Capability::Available),
            Err(err) => {
                let capability = Capability::from_probe_error(&err);
                (Box::new(UnavailableAudioDevices::because(err)), capability)
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let reason = PlatformError::NotCompiledForThisOs { crate_os: "windows" };
        let capability = Capability::from_probe_error(&reason);
        (Box::new(UnavailableAudioDevices::because(reason)), capability)
    }
}

/// The GPU seam builder: the DXGI scanner when a factory round
/// succeeds, the typed fallback otherwise.
fn gpu_seam() -> (Box<dyn GpuAdapters>, Capability) {
    #[cfg(target_os = "windows")]
    {
        let mut adapters = windows::dxgi::DxgiAdapters::new();
        match adapters.enumerate() {
            Ok(_) => (Box::new(adapters), Capability::Available),
            Err(err) => {
                let capability = Capability::from_probe_error(&err);
                (Box::new(UnavailableGpuAdapters::because(err)), capability)
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let reason = PlatformError::NotCompiledForThisOs { crate_os: "windows" };
        let capability = Capability::from_probe_error(&reason);
        (Box::new(UnavailableGpuAdapters::because(reason)), capability)
    }
}

/// The dialog seam builder: the Common Item Dialogs when the probe's
/// creation round succeeds, the typed fallback otherwise.
fn dialog_seam() -> (Box<dyn FileDialogs>, Capability) {
    #[cfg(target_os = "windows")]
    {
        match windows::dialogs::CommonItemDialogs::probe() {
            Ok(dialogs) => (Box::new(dialogs), Capability::Available),
            Err(err) => {
                let capability = Capability::from_probe_error(&err);
                (Box::new(UnavailableDialogs::because(err)), capability)
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let reason = PlatformError::NotCompiledForThisOs { crate_os: "windows" };
        let capability = Capability::from_probe_error(&reason);
        (Box::new(UnavailableDialogs::because(reason)), capability)
    }
}
