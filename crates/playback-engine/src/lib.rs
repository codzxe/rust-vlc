//! The Lumen playback session orchestrator (card 2.C2; doc 03 §4.6 +
//! §5, doc 05 §3; doc 01 §2.6 registry row: layer L4, "L0–L3 engine
//! crates + commands").
//!
//! # What this crate is
//!
//! The **minimal playback state machine**: the 8 wire-visible states
//! ([`SessionState`]), the inputs that move them ([`SessionInput`]),
//! and the transition table as **data** ([`TRANSITIONS`] — 26 rows,
//! every one a legal doc 05 §3.3 wire transition). Around it:
//!
//! - [`epoch`] — the two-level cancellation generations of doc 03 §5
//!   (session epochs + per-epoch pipeline generations, the shared
//!   [`CancelToken`], and the single-writer [`TokenMinter`]);
//! - [`event`] — the engine-side event stream and its wire
//!   reconciliation with the `commands` catalog (doc 05 §3/§4.2);
//! - [`events`] — the throttling/coalescing policy types those
//!   emissions obey (doc 05 §3.4, card 3.B4);
//! - [`pipeline`] — the trait-level wiring **demux → packet queues →
//!   decoders → sinks** (plugin-api `Demuxer`, packet-queue
//!   `BoundedQueue<Packet>`, codec-api `Decoder`/`FrameSink`) driven
//!   one bounded tick at a time;
//! - [`seek`] — the seven-step seek flow of doc 03 §6 (card 3.A1):
//!   target resolution, the pipeline flush + demux re-anchor, the
//!   pre-roll observation, and the failed-seek contract D4;
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
//! A/V sync, source replacement, track switching, rate, the
//! thread-per-session model and the event broadcast bus are excluded
//! by the card (3.A2–3.A3, 3.B1, 3.B3, 2.E1). The minimal machine is
//! synchronous and single-writer: the host's [`Engine::poll`] call IS
//! the session thread, which keeps invariant S1 true by construction.
//!
//! # The acceptance criteria, and where they are proven
//!
//! - *Transition-table tests; no invalid transition reachable*: the
//!   integration suite `tests/table.rs` classifies **every**
//!   `(state, input)` pair (8 × 12 = 96: 26 legal rows, 70 typed
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
//! - *Seek flow (card 3.A1, doc 03 §6)*: [`seek`]'s unit tests (target
//!   resolution, span helpers, failure clones) plus the session-level
//!   AC suite in `tests/seek_flow.rs` (the seven steps end-to-end,
//!   boundary seeks, the D4 rollback, supersession silence, the
//!   stale-purge proofs, and the seek-terminal drain exemption).
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
pub mod events;
pub mod pipeline;
pub mod seek;
pub mod session;
pub mod state;

pub use epoch::{CancelToken, GenerationPair, PipelineGeneration, SessionEpoch, TokenMinter};
pub use error::{SessionConfigError, SessionError, config_error};
pub use event::{
    EngineEvent, EngineEventKind, media_info_wire, source_spec_wire, track_info_wire,
    wire_error_code, wire_playback_state, wire_stage,
};
pub use events::{
    BUFFERING_WINDOW_MS, DAMAGE_WINDOW_MS, DamageCoalescer, DrainStaleness, EventPolicy,
    ProgressWindow, StalenessStamp, TIME_TICK_DEFAULT_MS, TIME_TICK_FLOOR_MS, TimeTickPolicy,
};
pub use pipeline::{
    DecoderProvider, FrameConsumer, QueuePlan, TickBudget, VIDEO_CONCEALMENT_BUDGET,
};
pub use seek::{SeekFailure, SeekTarget};
pub use session::{
    CommandReceipt, Engine, EngineOutput, EngineStats, INBOX_CAPACITY, OUTBOX_CAPACITY,
    RECEIPT_CAPACITY, ReceiptResult, SessionCommandKind, SessionConfig, SessionHandle,
    transition_table,
};
pub use state::{
    Effect, PipelineSignal, RejectionReason, SessionInput, SessionState, TRANSITIONS, Transition,
    TransitionRejected, apply_input, rejection_reason, translate_pipeline_signal,
};
