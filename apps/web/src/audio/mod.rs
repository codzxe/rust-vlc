//! `apps/web/src/audio` — the web audio pillar (card 7.H3, issue
//! #48; doc 09 §5).
//!
//! # What this module tree is
//!
//! The Rust half of the browser audio lane, in three pieces:
//!
//! - [`autoplay`] — the autoplay-policy gate (doc 09 §5.3): the
//!   page's `AudioContext` state reports drive it; a
//!   suspended-and-not-gesture-authorized context parks playback in
//!   the surfaced *paused-pending-gesture* condition (the typed
//!   [`crate::protocol::WorkerToHost::GestureRequired`] event —
//!   `[PROPOSAL]`, doc 09 §12 OQ-3), never a fake playing state.
//! - [`sink`] — the AudioWorklet sink **model** (doc 09 §5.1): the
//!   arithmetic twin of the hand-written JS processor
//!   (`apps/web/audio-worklet.js`) — the bounded queue, the fixed
//!   128-frame render quantum, silence-on-underrun, and the played
//!   sample counter (the clock's truth source). The model is the
//!   spec the JS transcribes; both are pinned to the same constants.
//! - [`clock`] — the sample-counter clock model (doc 09 §5.2): the
//!   played-instants → media-time mapping, the wallclock mapping
//!   (`AudioContext.currentTime` + `baseLatency` + `outputLatency`
//!   where exposed), the drift policy (anchor refresh at a bound),
//!   and the resample policy (media rate ≠ context rate ⇒ ONE
//!   conversion — the fractional-accumulator arithmetic).
//!
//! # What this is NOT (the honest boundary)
//!
//! No `AudioContext` is constructed here: the wasm module has no
//! DOM and no imports — the PAGE owns the context, the worklet node
//! and the gesture, exactly as doc 09 §5.1/§5.3 draw it. The v1
//! host's own audio output remains the null device (the headless
//! time source the pump drives); the worklet lane's end-to-end
//! browser wiring (Float32 chunk shipping, counter feedback into
//! the anchor) is card 7.I1's `[VERIFY]` matrix. What this card
//! lands is the typed contract: the models' arithmetic is proven
//! native-cfg by tests (labeled **simulation** — no browser, no
//! real audio device in this sandbox), and the JS processor is the
//! hand-audited transcription of the tested model (doc 09 §5.1's
//! "tiny, fixed, hand-audited JS shim" rule).

pub mod autoplay;
pub mod clock;
pub mod sink;

pub use autoplay::{AutoplayGate, GateChange};
pub use clock::{DriftPolicy, Resampler, WallclockAnchor, WorkletClock};
pub use sink::{ProcessReport, WORKLET_QUANTUM, WorkletQueue};
