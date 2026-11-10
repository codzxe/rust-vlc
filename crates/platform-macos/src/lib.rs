//! `platform-macos` — the macOS platform adapter (card 7.A3, issue
//! #40; doc 01 §2.7 L5 registry row).
//!
//! One of the three per-OS adapter crates (with `platform-windows`
//! and `platform-linux`). Its shape follows the doc 01 §2.7
//! template: it is the audited home of MP §9's OS-service
//! exceptions for macOS — the only `unsafe` in this crate lives in
//! the FFI modules ([`macos`]), every block safety-commented — it
//! implements seams defined against the core vocabulary
//! ([`audio_output`]'s `DeviceDesc`/`DeviceId`, doc 01 §2.6,
//! ADR-005), and it never leaks OS types upward (ARCH-6).
//!
//! # The typed-seam design (why the crate is honest on any machine)
//!
//! Each OS integration sits behind a **seam trait**
//! ([`audio::AudioDevices`], [`gpu::GpuAdapters`],
//! [`dialog::FileDialogs`]) with a typed fallback
//! ([`audio::UnavailableAudioDevices`], …). [`MacOsPlatform::discover`]
//! probes the host and wires either the real integration or the typed
//! fallback carrying the exact reason — capability negotiation is a
//! first-class outcome ([`capability::PlatformReport`]), never a
//! guessed success (MP §13 "no invented crate capabilities").
//!
//! # Runtime reality of this crate's integrations
//!
//! - **CoreAudio** (device enumeration + change listeners): the
//!   HAL's object-property API over the `CoreAudio` +
//!   `CoreFoundation` frameworks — no `coreaudio-sys` crate
//!   ([`macos::coreaudio`]).
//! - **GPU**: Metal device enumeration (`MTLCopyAllDevices`) plus
//!   the `CAMetalLayer` glue — the typed handoff the ADR-003-gated
//!   wgpu renderer consumes ([`macos::metal`]). The wgpu context
//!   itself stays behind `renderer-wgpu`'s Lead-gated import.
//! - **Dialogs**: `NSOpenPanel`/`NSSavePanel` through the
//!   Objective-C runtime (`objc_msgSend` typed per signature), with
//!   an enforced main-thread guard ([`macos::dialogs`]).
//!
//! **Evidence status (sandbox honesty):** this Linux sandbox has no
//! macOS SDK — the FFI surface is compile-verified via
//! `cargo check --target aarch64-apple-darwin` and
//! `x86_64-apple-darwin` (typechecked declarations, no linking, no
//! execution); runtime behavior on real Apple hardware is
//! `[VERIFY]`. The host-runnable tests cover the typed seams,
//! capability negotiation, fallbacks and event plumbing with
//! simulated inputs.
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

/// The macOS integrations (cfg-gated: on any other OS every seam
/// is the typed fallback and the report says `WrongOs` honestly).
#[cfg(target_os = "macos")]
pub mod macos;

use audio::{AudioDevices, UnavailableAudioDevices};
use capability::{Capability, Feature, PlatformReport};
use dialog::{FileDialogs, UnavailableDialogs};
use error::PlatformError;
use gpu::{GpuAdapters, UnavailableGpuAdapters};

/// The assembled macOS platform adapter: one probe, three seams,
/// one honest capability report (card 7.A3's entry point).
///
/// Construct via [`MacOsPlatform::discover`] — it performs the live
/// probes and wires either each real integration or its typed
/// fallback. The accessor methods hand out `dyn` seams so callers
/// never branch on which one they got: a call that cannot work
/// returns a typed [`PlatformError`], it never fakes success.
pub struct MacOsPlatform {
    audio: Box<dyn AudioDevices>,
    gpu: Box<dyn GpuAdapters>,
    dialogs: Box<dyn FileDialogs>,
    report: PlatformReport,
}

impl MacOsPlatform {
    /// Probe the host and assemble the best available seams.
    ///
    /// Probes run: one CoreAudio enumeration round (the HAL
    /// frameworks are always present on macOS — the enumeration call
    /// itself is the probe); one Metal device round; one
    /// non-intrusive AppKit panel-class lookup. Each probe's
    /// outcome lands verbatim in the returned [`PlatformReport`].
    #[must_use = "probing has side effects on OS handles; reuse the value"]
    pub fn discover() -> MacOsPlatform {
        let mut report = PlatformReport::new();
        let (audio, audio_capability) = audio_seam();
        let (gpu, gpu_capability) = gpu_seam();
        let (dialogs, dialogs_capability) = dialog_seam();
        // The CoreAudio client provides enumeration and listeners
        // together (the listener registers against the same HAL
        // object); one verdict covers both rows.
        report.set(Feature::AudioEnumeration, audio_capability.clone());
        report.set(Feature::AudioNotifications, audio_capability);
        report.set(Feature::GpuEnumeration, gpu_capability);
        report.set(Feature::FileDialogs, dialogs_capability);
        MacOsPlatform { audio, gpu, dialogs, report }
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

/// The audio seam builder: the CoreAudio client when a live
/// enumeration round succeeds, the typed fallback with the round's
/// diagnosis otherwise.
fn audio_seam() -> (Box<dyn AudioDevices>, Capability) {
    #[cfg(target_os = "macos")]
    {
        let mut devices = macos::coreaudio::CoreAudioDevices::new();
        match devices.enumerate() {
            Ok(_) => (Box::new(devices), Capability::Available),
            Err(err) => {
                let capability = Capability::from_probe_error(&err);
                (Box::new(UnavailableAudioDevices::because(err)), capability)
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let reason = PlatformError::NotCompiledForThisOs { crate_os: "macos" };
        let capability = Capability::from_probe_error(&reason);
        (Box::new(UnavailableAudioDevices::because(reason)), capability)
    }
}

/// The GPU seam builder: the Metal scanner when a device round
/// succeeds, the typed fallback otherwise.
fn gpu_seam() -> (Box<dyn GpuAdapters>, Capability) {
    #[cfg(target_os = "macos")]
    {
        let mut adapters = macos::metal::MetalDevices::new();
        match adapters.enumerate() {
            Ok(_) => (Box::new(adapters), Capability::Available),
            Err(err) => {
                let capability = Capability::from_probe_error(&err);
                (Box::new(UnavailableGpuAdapters::because(err)), capability)
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let reason = PlatformError::NotCompiledForThisOs { crate_os: "macos" };
        let capability = Capability::from_probe_error(&reason);
        (Box::new(UnavailableGpuAdapters::because(reason)), capability)
    }
}

/// The dialog seam builder: the AppKit panels when the probe's
/// class lookup succeeds, the typed fallback otherwise.
fn dialog_seam() -> (Box<dyn FileDialogs>, Capability) {
    #[cfg(target_os = "macos")]
    {
        match macos::dialogs::AppKitPanels::probe() {
            Ok(dialogs) => (Box::new(dialogs), Capability::Available),
            Err(err) => {
                let capability = Capability::from_probe_error(&err);
                (Box::new(UnavailableDialogs::because(err)), capability)
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let reason = PlatformError::NotCompiledForThisOs { crate_os: "macos" };
        let capability = Capability::from_probe_error(&reason);
        (Box::new(UnavailableDialogs::because(reason)), capability)
    }
}
