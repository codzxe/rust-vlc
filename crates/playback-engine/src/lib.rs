//! The Lumen playback session orchestrator (card 2.C2; doc 03 §4.6 +
//! §5, doc 05 §3; doc 01 §2.6 registry row: layer L4, "L0–L3 engine
//! crates + commands").
//!
//! # What this crate is
//!
//! The **minimal playback state machine**: the 8 wire-visible states
//! ([`SessionState`]), the inputs that move them ([`SessionInput`]),
//! and the transition table as **data** ([`TRANSITIONS`] — 23 rows,
//! every one a legal doc 05 §3.3 wire transition). Around it:
//!
//! - [`epoch`] — the two-level cancellation generations of doc 03 §5
//!   (session epochs + per-epoch pipeline generations, the shared
//!   [`CancelToken`], and the single-writer [`TokenMinter`]);
//! - [`event`] — the engine-side event stream and its wire
//!   reconciliation with the `commands` catalog (doc 05 §3/§4.2);
//! - [`pipeline`] — the trait-level wiring **demux → packet queues →
//!   decoders → sinks** (plugin-api `Demuxer`, packet-queue
//!   `BoundedQueue<Packet>`, codec-api `Decoder`/`FrameSink`) driven
//!   one bounded tick at a time;
//! - [`session`] — the [`Engine`]/[`Session`] single-writer executor
//!   that applies inputs through the table, interprets the row
//!   effects, mints generations and serializes outputs;
//! - [`state`] — the pure machine: [`apply_input`] is the only way
//!   session state moves, so an illegal transition is not merely
//!   rejected — it is unreachable by construction;
//! - [`error`] — the transport/construction failure surface (semantic
//!   rejections are typed results in [`state`], not errors).
//!
//! # What is deliberately NOT here (Phase 3+)
//!
//! Seek, A/V sync, source replacement, track switching, rate, the
//! thread-per-session model and the event broadcast bus are excluded
//! by the card (3.A1–3.A3, 3.B1, 3.B3, 2.E1). The minimal machine is
//! synchronous and single-writer: the host's [`Engine::poll`] call IS
//! the session thread, which keeps invariant S1 true by construction.
//!
//! # The acceptance criteria, and where they are proven
//!
//! - *Transition-table tests; no invalid transition reachable*: the
//!   integration suite `tests/table.rs` classifies **every**
//!   `(state, input)` pair (8 × 11 = 88: 23 legal rows, 65 typed
//!   rejections), proves every row is wire-conformant against
//!   [`commands::PlaybackState::can_transition_to`], proves the
//!   pipeline driver guard ([`translate_pipeline_signal`]) only ever
//!   yields table-legal inputs, and walks every row from `Idle`
//!   through legal inputs only.
//! - *Session epochs reject stale work*: [`epoch`]'s unit tests (mint
//!   publication, interrupt, cross-epoch staleness) plus the
//!   session-level staleness proofs in `tests/session_flow.rs`
//!   (stale-handle command rejection, superseded-event suppression at
//!   drain, generation bumping at stop/close accept time).
//!
//! # Conventions
//!
//! Std-only, zero external dependencies, no unsafe. The core never
//! blocks, never panics, and answers every failure with a typed error
//! or a typed rejection. Inventions beyond the docs are
//! `[PROPOSAL]`-tagged at their site.

pub mod epoch;
pub mod error;
pub mod event;
pub mod pipeline;
pub mod session;
pub mod state;

pub use epoch::{CancelToken, GenerationPair, PipelineGeneration, SessionEpoch, TokenMinter};
pub use error::{config_error, SessionConfigError, SessionError};
pub use event::{
    track_info_wire, wire_error_code, wire_playback_state, wire_stage, EngineEvent, EngineEventKind,
};
pub use pipeline::{DecoderProvider, FrameConsumer, QueuePlan, TickBudget};
pub use session::{
    transition_table, CommandReceipt, Engine, EngineOutput, EngineStats, OUTBOX_CAPACITY,
    ReceiptResult, SessionCommandKind, SessionConfig, SessionHandle,
};
pub use state::{
    apply_input, rejection_reason, translate_pipeline_signal, Effect, PipelineSignal,
    RejectionReason, SessionInput, SessionState, Transition, TransitionRejected, TRANSITIONS,
};
