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

/// What the engine knows about a container whose tracks it could not
/// wire (card 3.C2, defect G1): the known-format info the
/// `session.no-decodable-track` error must carry — the container's
/// name and the codec id of every track the demuxer reported, so the
/// failure answers "which formats would I have needed" instead of a
/// bare refusal (the card's known-format-info AC).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoDecodableKnown {
    /// The container's name, as the opening host declared it (probe
    /// truth — the engine itself never learns the name from the
    /// demuxer trait). `None` when the host did not say; the error
    /// then degrades honestly to the codec list alone.
    pub container: Option<String>,
    /// One `(kind, codec id)` pair per reported track, in demuxer
    /// order — the fourcc list (the `Track.codec` descriptor string
    /// `Pipeline::wire` consumed).
    pub tracks: Vec<(media_types::MediaKind, String)>,
}

impl NoDecodableKnown {
    /// Collect the enrichment from a wired track list (the zero-count
    /// `wire` outcome still stores the demuxer's tracks — they are
    /// the evidence).
    #[must_use]
    pub fn from_tracks(container: Option<&str>, tracks: &[media_types::Track]) -> NoDecodableKnown {
        NoDecodableKnown {
            container: container.map(str::to_string),
            tracks: tracks
                .iter()
                .map(|track| (track.kind, track.codec.to_string()))
                .collect(),
        }
    }
}

/// The error type of session/engine operations that fail before the
/// state machine is asked anything (see the module docs).
#[derive(Debug)]
pub enum SessionError {
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

/// The source leaf of an enriched `NoDecodableTrack` failure (the
/// `DemuxSourceInfo` chaining pattern, plugin-api's demux.rs): the
/// codec list rides the error's `source` chain where `Display`'s
/// kind summary cannot — structured data only, never a secret.
#[derive(Debug)]
struct NoDecodableLeaf {
    known: NoDecodableKnown,
}

impl fmt::Display for NoDecodableLeaf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.known.container {
            Some(container) => write!(f, "container `{container}` tracks: ")?,
            None => f.write_str("tracks: ")?,
        }
        if self.known.tracks.is_empty() {
            return f.write_str("(none reported)");
        }
        for (index, (kind, codec)) in self.known.tracks.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            let kind = match kind {
                media_types::MediaKind::Video => "video",
                media_types::MediaKind::Audio => "audio",
                media_types::MediaKind::Subtitle => "subtitle",
                media_types::MediaKind::Data => "data",
            };
            write!(f, "{kind} `{codec}`")?;
        }
        Ok(())
    }
}
impl std::error::Error for NoDecodableLeaf {}

/// The engine-side tag for a config failure (stage `Control`, kind
/// `Unsupported` — the L0 vocabulary, for callers that want a plain
/// [`EngineError`]).
///
/// `known` enriches the `NoDecodableTrack` failure with the
/// known-format info (defect G1, card 3.C2): the container name rides
/// the `Unsupported` kind's [`media_types::error::FormatInfo`] and the
/// track codec list rides the error's `source` leaf. `None` keeps the
/// historical un-enriched shape (hosts that declare no container).
#[must_use]
pub fn config_error(cause: SessionConfigError, known: Option<&NoDecodableKnown>) -> EngineError {
    let code = match cause {
        SessionConfigError::NoDecodableTrack => "session.no-decodable-track",
    };
    let format = known.and_then(|known| {
        known
            .container
            .as_ref()
            .map(|name| media_types::error::FormatInfo {
                name: Some(name.clone()),
                // The engine never sniffed bytes itself (the probe
                // did, before the demuxer existed) — an empty sniff
                // list is the honest form, never a fabricated one.
                sniffed: Vec::new(),
            })
    });
    let mut error = EngineError::new(
        PipelineStage::Control,
        media_types::ErrorKind::Unsupported { format },
        media_types::error::ErrorContext::new(code),
    );
    if let Some(known) = known {
        error = error.with_source(NoDecodableLeaf {
            known: known.clone(),
        });
    }
    error
}
