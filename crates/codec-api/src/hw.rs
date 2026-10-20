//! The backend-neutral hardware seam (doc 01 §2.4 sketch;
//! `HwAccelerator`), bounded by ADR-001.
//!
//! ADR-001 allows OS-decoder hardware paths only as audited E2 OS-ABI
//! exceptions with **truthful active-decoder reporting**. This crate
//! stays abstract on purpose (doc 01 §2.4(d): no platform types): the
//! trait names no API, no device, no surface format. Concrete adapters
//! (Phase 7.B) implement it; the [`DecoderConfig`](crate::DecoderConfig)
//! carries the chosen instance to the codec that knows how to drive it.

/// An opaque hardware acceleration handle (doc 01 §2.4 sketch:
/// "opaque hardware contract — bounded by ADR-001").
///
/// * **Opaque** — the methods below are identity and downcast only. The
///   real driver contract (surface negotiation, session setup) is a
///   private agreement between the adapter that mints the accelerator
///   and the decoder wrapper that consumes it; `codec-api` deliberately
///   knows none of it.
/// * **Bounded by ADR-001** — instances exist only behind the audited
///   per-OS adapters (E2 exception); software codecs never see one.
///   `backend_id` exists so the engine can satisfy ADR-001's truthful
///   active-decoder report ("which decoder produced the output").
///
/// # Supertraits
///
/// `Send + Sync` is required compositionally: the handle is shared as
/// `Arc<dyn HwAccelerator>` inside the negotiated
/// [`DecoderConfig`](crate::DecoderConfig), which must itself be `Send`
/// to cross onto the decode worker, and `Sync` so the engine can query
/// the active backend from its reporting thread. (A `Box`-shaped
/// alternative would force exclusive ownership and break reporting.)
pub trait HwAccelerator: Send + Sync {
    /// Stable opaque backend identifier for ADR-001's active-decoder
    /// report. Registry-minted dotted id (e.g. `"hw.mock"`); names no
    /// platform API — it is a label for UI/engine queries, not a
    /// capability switch.
    fn backend_id(&self) -> &str;

    /// Downcast seam `[PROPOSAL]` — mirrors plugin-api's type-erasure
    /// pattern: the decoder wrapper that knows its concrete backend
    /// retrieves it; an unknown backend is a typed configure error,
    /// never a panic.
    fn as_any(&self) -> &dyn std::any::Any;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct MockAccelerator;

    impl HwAccelerator for MockAccelerator {
        fn backend_id(&self) -> &str {
            "hw.mock"
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[test]
    fn accelerator_identity_and_downcast_round_trip() {
        let accel: Arc<dyn HwAccelerator> = Arc::new(MockAccelerator);
        assert_eq!(accel.backend_id(), "hw.mock");
        // The downcast seam finds the concrete type behind the erasure.
        let concrete = accel.as_any().downcast_ref::<MockAccelerator>().is_some();
        assert!(concrete, "downcast must find the concrete backend");
        // And rejects unrelated types without panicking.
        assert!(accel.as_any().downcast_ref::<String>().is_none());
    }
}
