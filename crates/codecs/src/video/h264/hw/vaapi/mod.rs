//! The Linux hardware-decode lane: **VA-API** (card 7.B1, issue
//! #41; ADR-001 §5's E2 OS-service binding; ADR-013 **[PROPOSAL]**
//! is the pending Lead decision record for this lane's activation).
//!
//! # What this lane is
//!
//! The typed, honest VA-API adapter behind the 5.F2 seam:
//!
//! - [`probe`] — capability negotiation: DRM render-node discovery
//!   → node open → runtime `dlopen` of `libva.so.2` +
//!   `libva-drm.so.2` → `vaInitialize` → H.264 profile query, each
//!   step's failure typed into
//!   [`HwAvailability::Unavailable`](super::capability::HwAvailability::Unavailable)
//!   with a [`HwUnavailableReason`] — never a guessed success.
//! - [`VaApiAccelerator`] — the opaque
//!   [`HwAccelerator`](codec_api::HwAccelerator) minted only by a
//!   **successful** probe (carries the node + vendor detail; no
//!   device context — the session re-opens its own).
//! - [`VaApiBridge`] — the [`H264HwBridge`](super::H264HwBridge)
//!   plug: downcasts the negotiated accelerator to
//!   [`VaApiAccelerator`] and mints an unopened
//!   [`VaApiSession`](session::VaApiSession); every other
//!   accelerator is `None` ("not mine").
//!
//! # Substrate decision (task-recorded, honest)
//!
//! ADR-013's option (a)/(b) named `libva` 0.1.5 / `libva-sys` 0.1.2
//! as substrate candidates — both remain **unadmitted** (the ADR
//! is `[PROPOSAL]`; its sign-off gate is not met). This lane
//! implements the no-new-dependency reading: **hand-rolled FFI
//! declarations** ([`ffi`], mirrored from the public libva 2.22.0
//! spec headers — the exact `libva.so.2` installed on this build
//! host) + a runtime `dlopen` loader ([`dl`], the audited
//! `platform-linux` 7.A2 pattern, re-implemented here because the
//! L3→L5 layering wall forbids importing the L5 crate) + symbol
//! resolution ([`library`]). Zero new workspace dependencies; the
//! machines-without-libva path stays a typed
//! `LibraryUnavailable`, not a load failure. If the Lead ratifies
//! a `-sys`/bindings substrate at ADR-013 sign-off, this file's
//! `ffi`/`dl`/`library` trio is what gets replaced — the session,
//! probe and bridge contracts are substrate-neutral.
//!
//! # Runtime evidence on this host (the honest label)
//!
//! - **Runtime-tested here**: the probe's no-device /
//!   permission-denied paths (this sandbox has no `/dev/dri`), the
//!   real-libva symbol-table resolution (`library.rs` host test),
//!   and every pure reconstruction function (see `session.rs`
//!   tests). **The VA-API probe fails honestly on this machine —
//!   that path IS the tested path.**
//! - **Compile-verified only; runtime hardware behavior NOT
//!   exercised — `[VERIFY]`**: the driver-mediated decode pipeline
//!   (config/surfaces/context, buffer submission, sync). No GPU
//!   exists in this sandbox; no frame was ever decoded by this
//!   lane. The feature-matrix row says exactly that.
//!
//! # Module map
//!
//! - [`device`] — pure-std DRM render-node discovery (the
//!   `platform-linux` sysfs-scanner design, fixture-testable).
//! - [`dl`] — the `dlopen`/`dlsym` wrapper (OS-ABI exception).
//! - [`ffi`] — hand-rolled libva 2.22.0 ABI declarations
//!   (size-pinned struct layouts).
//! - [`library`] — the typed symbol table (whole-table resolution
//!   at open time).
//! - [`session`] — [`VaApiSession`] (the `H264HwDecoder` impl),
//!   the §8.2 DPB/ref-list reconstruction, and [`probe`].

pub mod device;
pub mod dl;
pub mod ffi;
pub mod library;
pub mod session;

pub use device::{DriDiscovery, DriNode, DriScanner};
pub use session::{VaApiSession, probe, probe_with};

use std::fmt;
use std::sync::Arc;

use super::capability::HwBackend;
use super::{H264HwBridge, H264HwDecoder};
use codec_api::HwAccelerator;

/// The lane's full negotiation step: probe + accelerator mint.
///
/// Returns the [`HwAvailability`](super::capability::HwAvailability)
/// truth and, **iff** the probe succeeded, the minted
/// [`VaApiAccelerator`] — the pair
/// [`capability::negotiate`](super::capability::negotiate)
/// consumes under the caller's policy. A failed probe mints
/// nothing (never an accelerator off a device that answered no).
pub fn discover() -> (
    super::capability::HwAvailability,
    Option<Arc<VaApiAccelerator>>,
) {
    let (availability, minted) = session::probe();
    match minted {
        Some((node, vendor)) => {
            let accel = VaApiAccelerator::new(node, vendor);
            (availability, Some(Arc::new(accel)))
        }
        None => (availability, None),
    }
}

/// The accelerator a successful VA-API probe mints (ADR-001's
/// opaque handle; the truthful `hw.vaapi` backend label).
///
/// It deliberately holds **no device state**: the probe's display
/// is terminated after the capability query (probes must be
/// side-effect-free on the decode path), and the session the
/// bridge mints performs its own bring-up. If the device vanished
/// between probe and session, the session's `configure` reports
/// the typed `h264.hw.vaapi.*` refusal — never a fake success.
pub struct VaApiAccelerator {
    /// The DRM node the probe succeeded on.
    node: DriNode,
    /// The vendor string the driver reported (the truthful detail).
    vendor: String,
}

impl VaApiAccelerator {
    /// Mint (probe-internal): carries the discovered node + vendor.
    pub(crate) fn new(node: DriNode, vendor: String) -> VaApiAccelerator {
        VaApiAccelerator { node, vendor }
    }

    /// The DRM node this accelerator serves.
    #[must_use = "pure accessor"]
    pub fn node(&self) -> &DriNode {
        &self.node
    }

    /// The driver's vendor string (the `hw-active` detail).
    #[must_use = "pure accessor"]
    pub fn vendor(&self) -> &str {
        &self.vendor
    }
}

impl fmt::Debug for VaApiAccelerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VaApiAccelerator")
            .field("node", &self.node.name)
            .field("vendor", &self.vendor)
            .finish()
    }
}

impl HwAccelerator for VaApiAccelerator {
    fn backend_id(&self) -> &str {
        HwBackend::VaApi.backend_id()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// The Linux lane's [`H264HwBridge`] plug (7.B1): claims exactly
/// [`VaApiAccelerator`] accelerators and mints an unopened
/// [`VaApiSession`] bound to the accelerator's node.
pub struct VaApiBridge;

impl H264HwBridge for VaApiBridge {
    fn backend_id(&self) -> &str {
        HwBackend::VaApi.backend_id()
    }

    fn open(&self, accel: &Arc<dyn HwAccelerator>) -> Option<Box<dyn H264HwDecoder>> {
        let concrete = accel.as_any().downcast_ref::<VaApiAccelerator>()?;
        Some(Box::new(VaApiSession::new(concrete.node.clone())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_claims_only_its_own_accelerator() {
        struct OtherBackend;
        impl HwAccelerator for OtherBackend {
            fn backend_id(&self) -> &str {
                "hw.mock.other"
            }
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }
        let mine: Arc<dyn HwAccelerator> = Arc::new(VaApiAccelerator::new(
            DriNode {
                name: "renderD128".into(),
                render: true,
            },
            "test driver".into(),
        ));
        let other: Arc<dyn HwAccelerator> = Arc::new(OtherBackend);
        let bridge = VaApiBridge;
        assert_eq!(bridge.backend_id(), "hw.vaapi");
        // Mine: a session is minted, unopened, with the right label.
        let session = bridge
            .open(&mine)
            .unwrap_or_else(|| unreachable!("mine must open"));
        assert_eq!(session.backend_id(), "hw.vaapi");
        // Not mine: None (the walk moves on; a full miss is the
        // typed h264.hw.no_bridge error upstream).
        assert!(bridge.open(&other).is_none());
    }

    #[test]
    fn accelerator_is_opaque_but_truthful() {
        let accel = VaApiAccelerator::new(
            DriNode {
                name: "renderD129".into(),
                render: true,
            },
            "Mesa Intel".into(),
        );
        let erased: &dyn HwAccelerator = &accel;
        assert_eq!(erased.backend_id(), "hw.vaapi");
        assert_eq!(
            erased
                .as_any()
                .downcast_ref::<VaApiAccelerator>()
                .unwrap_or(&accel)
                .vendor(),
            "Mesa Intel"
        );
        assert_eq!(
            format!("{accel:?}"),
            "VaApiAccelerator { node: \"renderD129\", vendor: \"Mesa Intel\" }",
            "Debug carries the truthful facts, no pointers"
        );
    }
}
