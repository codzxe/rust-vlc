//! The macOS hardware-decode lane: **VideoToolbox** (card 7.B1,
//! issue #41; ADR-001 §5's E2 OS-service binding).
//!
//! # What this lane is
//!
//! The typed, honest VideoToolbox adapter behind the 5.F2 seam:
//!
//! - [`probe`](session::probe) — the capability question, one call:
//!   `VTIsHardwareDecodeSupported(kCMVideoCodecType_H264)`. A yes
//!   is [`HwAvailability::Available`]; a no is the typed
//!   `DriverIncompatible` truth (VideoToolbox ships with every
//!   macOS — a "no" is the machine's hardware fact). Never a
//!   guessed success.
//! - [`VideoToolboxAccelerator`] — the opaque
//!   [`HwAccelerator`](codec_api::HwAccelerator) minted only by a
//!   **successful** probe (carries the probe's detail string; no
//!   session state — the session the bridge mints performs its own
//!   bring-up, so a vanished device lands in the typed
//!   `h264.hw.videotoolbox.session` configure refusal, never a
//!   fake success).
//! - [`VideoToolboxBridge`] — the [`H264HwBridge`](super::H264HwBridge)
//!   plug: downcasts the negotiated accelerator to
//!   [`VideoToolboxAccelerator`] and mints an unopened
//!   [`VideoToolboxSession`](session::VideoToolboxSession); every
//!   other accelerator is `None` ("not mine").
//!
//! # Substrate decision (task-recorded, honest)
//!
//! No `-sys`/bindings crate: this lane declares the Apple framework
//! C ABIs by hand ([`ffi`], written from the public header
//! documentation — the provenance and `[VERIFY]` labels are that
//! file's first section) and calls them directly. Zero new
//! workspace dependencies, the MP §9 audited-exception posture of
//! the VA-API lane, and the same rule: if the Lead ever ratifies a
//! bindings substrate, `ffi` is the file it replaces — the session,
//! probe and bridge contracts here are substrate-neutral.
//!
//! # Runtime evidence (the honest label)
//!
//! - **Host-tested** (OS-neutral, in this crate's Linux suite):
//!   the wire forms and timestamp packaging this lane rides —
//!   [`super::feed`]'s AVCC sample serializer and the
//!   nanosecond/`CMTime` round trip.
//! - **Compile-verified only; runtime hardware behavior NOT
//!   exercised — `[VERIFY]`**: everything in [`ffi`] and
//!   [`session`] (`cargo check --target aarch64-apple-darwin`; no
//!   macOS SDK, no Apple hardware in this sandbox — no frame was
//!   ever decoded by this lane). The feature-matrix row says
//!   exactly that.
//!
//! # Module map
//!
//! - [`ffi`] — hand-rolled VideoToolbox / CoreMedia / CoreVideo /
//!   CoreFoundation ABI declarations (size-pinned `CMTime`).
//! - [`session`] — [`VideoToolboxSession`](session::VideoToolboxSession)
//!   (the `H264HwDecoder` impl) and [`probe`](session::probe).

pub mod ffi;
pub mod session;

pub use session::{VideoToolboxSession, probe};

use std::fmt;
use std::sync::Arc;

use super::capability::{HwAvailability, HwBackend};
use super::{H264HwBridge, H264HwDecoder};
use codec_api::HwAccelerator;

/// The lane's full negotiation step: probe + accelerator mint.
///
/// Returns the [`HwAvailability`] truth and, **iff** the probe
/// succeeded, the minted [`VideoToolboxAccelerator`] — the pair
/// [`capability::negotiate`](super::capability::negotiate)
/// consumes under the caller's policy. A failed probe mints
/// nothing (never an accelerator off a service that answered no).
pub fn discover() -> (HwAvailability, Option<Arc<VideoToolboxAccelerator>>) {
    let (availability, detail) = probe();
    let minted = detail.map(|detail| Arc::new(VideoToolboxAccelerator::new(detail)));
    (availability, minted)
}

/// The accelerator a successful VideoToolbox probe mints
/// (ADR-001's opaque handle; the truthful `hw.videotoolbox`
/// backend label). Holds no framework state — see the module docs.
pub struct VideoToolboxAccelerator {
    /// The probe's detail string (the `hw-active` report's truth).
    detail: String,
}

impl VideoToolboxAccelerator {
    /// Mint (probe-internal): carries the probe detail.
    pub(crate) fn new(detail: String) -> VideoToolboxAccelerator {
        VideoToolboxAccelerator { detail }
    }

    /// The probe detail (the `hw-active` report's truth).
    #[must_use = "pure accessor"]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Debug for VideoToolboxAccelerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VideoToolboxAccelerator")
            .field("detail", &self.detail)
            .finish()
    }
}

impl HwAccelerator for VideoToolboxAccelerator {
    fn backend_id(&self) -> &str {
        HwBackend::VideoToolbox.backend_id()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// The macOS lane's [`H264HwBridge`] plug (7.B1): claims exactly
/// [`VideoToolboxAccelerator`] accelerators and mints an unopened
/// [`VideoToolboxSession`].
pub struct VideoToolboxBridge;

impl H264HwBridge for VideoToolboxBridge {
    fn backend_id(&self) -> &str {
        HwBackend::VideoToolbox.backend_id()
    }

    fn open(&self, accel: &Arc<dyn HwAccelerator>) -> Option<Box<dyn H264HwDecoder>> {
        let _concrete = accel.as_any().downcast_ref::<VideoToolboxAccelerator>()?;
        // The session brings itself up at configure (the
        // accelerator carries no device state by design).
        Some(Box::new(VideoToolboxSession::new()))
    }
}
