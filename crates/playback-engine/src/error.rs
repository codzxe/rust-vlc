//! Typed engine-side errors (card 2.C2) — the stage-tagged taxonomy
//! style of `media-types` (the L0 precedent) applied to the session
//! orchestrator's own failure surface.
//!
//! Two families, kept distinct on purpose:
//!
//! - [`SessionError`] — *transport and construction* failures:
//!   opening an engine that already has a live session, a broken
//!   session configuration, epoch exhaustion, submitting to a
//!   destroyed session. These answer the **caller's function call**
//!   (doc 05 §1.2: validated at the boundary).
//! - [`crate::state::TransitionRejected`] (in the state module) —
//!   *semantic* rejections at application time: an input that is not
//!   valid for the current state answers `CommandRejected{reason}`
//!   (doc 03 §4.6), never an error crash, and never a session
//!   teardown.
//!
//! Media/pipeline failures are **not** errors here at all: they are
//! data ([`media_types::EngineError`]) carried by the
//! [`Failed`](crate::state::SessionInput::Failed) input and the
//! [`Fatal`](crate::event::EngineEventKind::Fatal) event — the state
//! machine's `Error` state is a *state*, entered through the table.

use std::fmt;

use media_types::{EngineError, PipelineStage};

/// The error type of session/engine operations that fail before the
/// state machine is asked anything (see the module docs).
#[derive(Debug)]
pub enum SessionError {
    /// The engine already hosts a live session. Single-session v0
    /// `[PROPOSAL: multi-session hosting arrives with the hosts that
    /// need it]`; card 3.A2 wires source replacement on `open`, so
    /// this refusal now surfaces for exactly one legal exit — a live
    /// `Buffering` session (no `buffering → {stopped,idle}` wire
    /// edge in v1; card 3.C1 owns the transient-state edges).
    SessionExists {
        /// The live session's wire id.
        session: commands::SessionId,
    },
    /// The session configuration is unusable (see
    /// [`SessionConfigError`] for the typed causes).
    Config(SessionConfigError),
    /// A generation counter ran its full u64 course (the L0
    /// vocabulary's own exhaustion error, re-tagged).
    GenerationExhausted(EngineError),
    /// The command could not be queued: the target session is
    /// destroyed (its inbox is gone). A stale handle's late command
    /// lands here — the typed command-side staleness rejection.
    SessionClosed,
    /// The command could not be queued: the session's bounded inbox
    /// is full (engineering directive D-1 boundedness — the command
    /// was NOT enqueued; the caller retries or sheds load; the
    /// engine never blocks and never silently drops). Surfaces only
    /// to a host submitting far faster than it polls
    /// ([`crate::session::INBOX_CAPACITY`] commands per poll).
    InboxBusy,
    /// An engine-internal invariant breach: an input the table pins as
    /// always-legal was refused (only reachable through a caller bug
    /// or table corruption — e.g. applying `Open` to a freshly
    /// constructed `Idle` session). Carries the rejection's rendered
    /// form; surfaced as a typed error, never panicked on.
    Invariant(String),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::SessionExists { session } => {
                write!(
                    f,
                    "a live session exists (id {}); close it first",
                    session.0
                )
            }
            SessionError::Config(cause) => write!(f, "session configuration: {cause}"),
            SessionError::GenerationExhausted(source) => write!(f, "{source}"),
            SessionError::SessionClosed => {
                f.write_str("the target session is closed (stale handle)")
            }
            SessionError::InboxBusy => {
                f.write_str("the session inbox is full (poll the engine, then retry)")
            }
            SessionError::Invariant(detail) => {
                write!(f, "engine invariant breached: {detail}")
            }
        }
    }
}

impl std::error::Error for SessionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SessionError::Config(cause) => Some(cause),
            SessionError::GenerationExhausted(source) => Some(source),
            _ => None,
        }
    }
}

/// Why a [`crate::SessionConfig`] was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionConfigError {
    /// Zero tracks survived wiring and the engine cannot play what it
    /// cannot decode (no audio/video limb was constructible).
    NoDecodableTrack,
}

impl fmt::Display for SessionConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionConfigError::NoDecodableTrack => f.write_str(
                "no decodable audio/video track: the engine refuses to \
                 fabricate playback of a medium it cannot decode",
            ),
        }
    }
}

impl std::error::Error for SessionConfigError {}

/// The engine-side tag for a config failure (stage `Control`, kind
/// `Unsupported` — the L0 vocabulary, for callers that want a plain
/// [`EngineError`]).
#[must_use]
pub fn config_error(cause: SessionConfigError) -> EngineError {
    let code = match cause {
        SessionConfigError::NoDecodableTrack => "session.no-decodable-track",
    };
    EngineError::new(
        PipelineStage::Control,
        media_types::ErrorKind::Unsupported { format: None },
        media_types::error::ErrorContext::new(code),
    )
}
