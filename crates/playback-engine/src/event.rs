//! The engine-side event stream (card 2.C2): the doc 03 §4.6(b)
//! engine-event subset that the minimal machine can produce, its
//! envelope, and its **wire reconciliation** with the command layer
//! (doc 05 §3 — the card requires the two to agree, not merely
//! resemble each other).
//!
//! # Engine-side vs wire events
//!
//! Doc 03 names engine events; doc 05 §3.2 is the wire catalog, and
//! §4.2 fixes the mapping. This module implements that mapping as
//! code:
//!
//! - `StateChanged` → `session.state-changed`
//! - `TracksChanged` → `tracks.list-ready`
//! - `Buffering` → `session.buffering` (episodes open with `0.0` and
//!   close with `1.0`, doc 05 §3.2)
//! - `Ended` → `playback.end-of-media` (emitted *before* the
//!   `state-changed → ended` event, doc 05 §3.2)
//! - `Error(stage, e)` → `playback.error` (fatal; emitted *before*
//!   the `state-changed → error` event, doc 05 §3.3)
//! - `FirstFrame` → **engine-private** (doc 05 §4.2: "internal
//!   trigger of the `playing` transition, no separate v1 wire event")
//! - `DamagedData` → **engine-private** (no v1 wire row; it is
//!   bounded-recovery observability for the log/testkit side)
//!
//! Seek events (`SeekAccepted`/`Seeked`/`SeekFailed`), device events
//! and `ClockMasterChanged` are Phase 3+ (`3.A1`, `2.C3`/`7.C1`,
//! `3.B2`) and deliberately absent. Event *completeness* — every
//! catalog row with payloads and throttling — is card `3.B4`; this
//! card ships the subset the minimal machine can honestly emit.
//!
//! # Engine vocabulary → wire vocabulary
//!
//! The bridges ([`wire_stage`], [`wire_error_code`],
//! [`track_info_wire`]) are the "engine-side bridging" the 1.D4
//! worklog deferred to Phase 2: L0 [`media_types`] vocabulary in,
//! `commands` wire types out. They are total functions; the mapping
//! choices that were not dictated by the docs are `[PROPOSAL]`-tagged
//! inline.

use std::collections::BTreeMap;
use std::sync::Arc;

use commands::catalog::types::{
    CodecDescriptor as WireCodecDescriptor, TrackInfo as WireTrackInfo, TrackKind as WireTrackKind,
    TrackOrigin as WireTrackOrigin,
};
use commands::envelope::{Generation, RequestId, SessionId};
use commands::{EventKind, PlaybackState, WireEvent};
use media_types::error::{ErrorKind, PipelineStage};
use media_types::{EngineError, Stamp, Track};

use crate::state::SessionState;

/// One engine → host event payload (doc 03 §4.6(b), minimal-machine
/// subset — see the module docs for the wire mapping of each variant).
///
/// `#[non_exhaustive]` per doc 05 §6.2: later waves add variants
/// (seek terminals, device loss, clock-master changes); hosts must
/// degrade unknown kinds to a logged no-op, exactly as the wire
/// catalog's own `#[non_exhaustive]` rule prescribes.
///
/// `Clone`/`PartialEq` are **manual**: the [`Fatal`] variant carries an
/// [`EngineError`], whose optional boxed `source` chain is neither.
/// Cloning rebuilds the typed error without the source chain (events
/// are data, not error-chain carriers); equality compares
/// stage + kind + context code — the fields the wire event exposes.
#[derive(Debug)]
#[non_exhaustive]
pub enum EngineEventKind {
    /// Every playback-state transition, including self-transitions as
    /// idempotent command confirmations (doc 05 C8). Never coalesced.
    StateChanged {
        /// The state we left.
        from: SessionState,
        /// The state we entered (may equal `from`).
        to: SessionState,
    },
    /// The initial track list became available (doc 03 §4.6's initial
    /// `TracksChanged`). Shared behind an `Arc` so the event clones
    /// cheaply alongside the wire conversion.
    TracksChanged {
        /// The demuxed, engine-vocabulary track list.
        tracks: Arc<Vec<Track>>,
        /// Which tracks the engine wired limbs for (selection truth).
        selected: Vec<bool>,
    },
    /// A buffering episode progress report (doc 05 §3.2: episodes
    /// open with `0.0` and close with `1.0`; v0 emits only the
    /// enter/exit bookends — the ≤ 5 Hz continuous stream is 3.B4).
    Buffering {
        /// `0.0..=1.0`.
        progress: f32,
    },
    /// The current media finished (followed by
    /// `StateChanged { to: Ended }`).
    EndOfMedia,
    /// A fatal failure (doc 03 §4.6 `Error(stage, e)`). Always
    /// ordered before its `StateChanged { to: Error }`.
    Fatal {
        /// The typed, stage-tagged engine error.
        error: EngineError,
    },
    /// Bounded-recovery damage report (doc 03 §4.6 `DamagedData`):
    /// how many units a stage skipped while resynchronizing.
    /// Engine-private — no v1 wire row.
    DamagedData {
        /// Units (packets/blocks) dropped by recovery in one tick.
        count: u64,
    },
    /// The first frame reached the sink (doc 03 §4.6 `FirstFrame`).
    /// Engine-private — the v1 wire treats it as the internal trigger
    /// of the `playing` transition.
    FirstFrame {
        /// The frame's presentation timestamp (may be `Missing` for
        /// formats without stamps — the event still fires).
        position: Stamp,
    },
}

impl Clone for EngineEventKind {
    fn clone(&self) -> EngineEventKind {
        match self {
            EngineEventKind::StateChanged { from, to } => EngineEventKind::StateChanged {
                from: *from,
                to: *to,
            },
            EngineEventKind::TracksChanged { tracks, selected } => EngineEventKind::TracksChanged {
                tracks: Arc::clone(tracks),
                selected: selected.clone(),
            },
            EngineEventKind::Buffering { progress } => EngineEventKind::Buffering {
                progress: *progress,
            },
            EngineEventKind::EndOfMedia => EngineEventKind::EndOfMedia,
            EngineEventKind::Fatal { error } => EngineEventKind::Fatal {
                // Rebuilt without the boxed `source` chain (see the
                // type docs): the stage, kind and context code carry
                // the wire-visible truth.
                error: EngineError::new(
                    error.stage,
                    error.kind.clone(),
                    media_types::error::ErrorContext {
                        code: error.context.code.clone(),
                    },
                ),
            },
            EngineEventKind::DamagedData { count } => EngineEventKind::DamagedData { count: *count },
            EngineEventKind::FirstFrame { position } => EngineEventKind::FirstFrame {
                position: *position,
            },
        }
    }
}

impl PartialEq for EngineEventKind {
    fn eq(&self, other: &EngineEventKind) -> bool {
        match (self, other) {
            (
                EngineEventKind::StateChanged { from: a, to: b },
                EngineEventKind::StateChanged { from: c, to: d },
            ) => a == c && b == d,
            (
                EngineEventKind::TracksChanged { tracks: a, selected: b },
                EngineEventKind::TracksChanged { tracks: c, selected: d },
            ) => **a == **c && b == d,
            (EngineEventKind::Buffering { progress: a }, EngineEventKind::Buffering { progress: b }) => {
                a == b
            }
            (EngineEventKind::EndOfMedia, EngineEventKind::EndOfMedia) => true,
            (EngineEventKind::Fatal { error: a }, EngineEventKind::Fatal { error: b }) => {
                a.stage == b.stage && a.kind == b.kind && a.context.code == b.context.code
            }
            (EngineEventKind::DamagedData { count: a }, EngineEventKind::DamagedData { count: b }) => {
                a == b
            }
            (
                EngineEventKind::FirstFrame { position: a },
                EngineEventKind::FirstFrame { position: b },
            ) => a == b,
            _ => false,
        }
    }
}

impl EngineEventKind {
    /// The wire form (doc 05 §4.2 mapping); `None` for engine-private
    /// events that must not leak onto the wire.
    #[must_use]
    pub fn as_wire(&self) -> Option<EventKind> {
        match self {
            EngineEventKind::StateChanged { from, to } => Some(EventKind::SessionStateChanged {
                from: from.as_wire(),
                to: to.as_wire(),
            }),
            EngineEventKind::TracksChanged { tracks, selected } => {
                Some(EventKind::TracksListReady {
                    tracks: tracks
                        .iter()
                        .zip(selected.iter())
                        .map(|(track, &is_selected)| track_info_wire(track, is_selected))
                        .collect(),
                })
            }
            EngineEventKind::Buffering { progress } => Some(EventKind::SessionBuffering {
                progress: *progress,
            }),
            EngineEventKind::EndOfMedia => Some(EventKind::PlaybackEndOfMedia),
            EngineEventKind::Fatal { error } => Some(EventKind::PlaybackError {
                stage: wire_stage(error.stage),
                code: wire_error_code(&error.kind),
                message: error.to_string(),
                details: fatal_details(error),
                fatal: true,
            }),
            EngineEventKind::DamagedData { .. } => None,
            EngineEventKind::FirstFrame { .. } => None,
        }
    }

    /// Whether this kind is pipeline-scoped (doc 05 §3.2 "gen"
    /// column — subject to the §4.2 staleness rule). Every v0 kind
    /// is; the flag exists so the outbox filter and future
    /// non-scoped kinds stay honest.
    #[must_use]
    pub fn is_pipeline_scoped(&self) -> bool {
        true
    }
}

/// One engine → host event with its correlation envelope (doc 05
/// §3.1): engine-global strictly-increasing `seq`, the session (the
/// epoch, doc 05 §4.2), the pipeline generation, and the causing
/// request when attributable.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineEvent {
    /// Engine-global, strictly increasing (doc 05 §4.3 guarantee 1).
    pub seq: u64,
    /// The emitting session's wire id.
    pub session: SessionId,
    /// The pipeline generation the event was emitted under.
    pub generation: Generation,
    /// The request that caused the event, when attributable (doc 05
    /// §3.1 `origin`).
    pub origin: Option<RequestId>,
    /// The payload.
    pub kind: EngineEventKind,
}

impl EngineEvent {
    /// Whether the event is pipeline-scoped (carries a generation
    /// subject to staleness — doc 05 §3.1).
    #[must_use]
    pub fn is_pipeline_scoped(&self) -> bool {
        self.kind.is_pipeline_scoped()
    }

    /// The full wire event (doc 05 §5.1 shape), or `None` for
    /// engine-private kinds.
    #[must_use]
    pub fn as_wire_event(&self) -> Option<WireEvent> {
        let kind = self.kind.as_wire()?;
        Some(WireEvent {
            seq: self.seq,
            session: Some(self.session),
            generation: Some(self.generation),
            origin: self.origin,
            kind,
        })
    }
}

/// Bridge an L0 pipeline stage to the command-layer stage (doc 05
/// §2.13: "the command-layer view of the pipeline").
///
/// The fourteen L0 stages collapse onto the wire's thirteen with
/// judgment calls, each `[PROPOSAL]` and revisitable with the
/// recovery wave (3.C2) that owns error surfacing:
/// `AudioFilter`/`VideoFilter` surface as decode-stage failures
/// (they are decode-adjacent to the command layer);
/// `Mix`/`AudioOutput` map to the audio-output stage; `Present` maps
/// to the renderer; `Clock`/`Discovery`/`Transcode` surface as
/// control-stage (engine policy) failures.
#[must_use]
pub fn wire_stage(stage: PipelineStage) -> commands::Stage {
    use commands::Stage as W;
    match stage {
        PipelineStage::Open => W::Input,
        PipelineStage::Probe => W::Probe,
        PipelineStage::Demux => W::Demux,
        PipelineStage::Decode | PipelineStage::AudioFilter | PipelineStage::VideoFilter => W::Decode,
        PipelineStage::Mix | PipelineStage::AudioOutput => W::AudioOutput,
        PipelineStage::Present => W::Renderer,
        PipelineStage::Clock | PipelineStage::Control | PipelineStage::Discovery
        | PipelineStage::Transcode => W::Control,
        PipelineStage::Plugin => W::Plugin,
    }
}

/// Bridge an L0 error kind to the wire error code (doc 05 §2.13's
/// "media/engine failures — the seven canonical classes" mirror MP
/// §8's seven names one-to-one).
#[must_use]
pub fn wire_error_code(kind: &ErrorKind) -> commands::ErrorCode {
    use commands::ErrorCode as C;
    match kind {
        ErrorKind::Unsupported { .. } => C::Unsupported,
        ErrorKind::Corrupt { .. } => C::Corrupt,
        ErrorKind::Inaccessible { .. } => C::Inaccessible,
        ErrorKind::Network { .. } => C::Network,
        ErrorKind::Decoder { .. } => C::Decoder,
        ErrorKind::Device { .. } => C::Device,
        ErrorKind::ResourceLimit { .. } => C::ResourceLimit,
    }
}

/// Machine-readable hints for the wire `playback.error` payload: the
/// L0 context code (stable, dotted) plus the pipeline stage.
fn fatal_details(error: &EngineError) -> BTreeMap<String, String> {
    let mut details = BTreeMap::new();
    details.insert("context".to_string(), error.context.code.clone());
    details.insert("stage".to_string(), error.stage.as_str().to_string());
    details
}

/// Bridge an engine track to the wire track info (doc 05 §2.2
/// `TrackInfo`). `[PROPOSAL]` in three places, each owed to the wave
/// that owns the missing fact:
///
/// - the codec descriptor string becomes the wire `description`
///   (fourcc/profile/level split lands with the plugin registry's
///   richer descriptors);
/// - `video`/`audio` detail blocks are `None` until the demux
///   implementation (2.B2) reports width/height/rate/channels
///   through an extended L0 `Track`;
/// - `origin` is `Stream` (external subtitles arrive with 3.A5).
#[must_use]
pub fn track_info_wire(track: &Track, is_selected: bool) -> WireTrackInfo {
    WireTrackInfo {
        id: commands::catalog::types::TrackId(track.id.handle()),
        kind: match track.kind {
            media_types::MediaKind::Audio => WireTrackKind::Audio,
            media_types::MediaKind::Video => WireTrackKind::Video,
            media_types::MediaKind::Subtitle => WireTrackKind::Subtitle,
            media_types::MediaKind::Data => WireTrackKind::Data,
        },
        name: track.title.clone(),
        language: track.language.as_ref().map(|tag| tag.as_str().to_string()),
        codec: WireCodecDescriptor {
            fourcc: None,
            description: Some(track.codec.as_str().to_string()),
            profile: None,
            level: None,
        },
        is_default: track.flags.default_on,
        is_selected,
        origin: WireTrackOrigin::Stream,
        video: None,
        audio: None,
    }
}

/// The wire playback state of an engine state — trivially
/// [`SessionState::as_wire`], named here for the event-module readers.
#[must_use]
pub fn wire_playback_state(state: SessionState) -> PlaybackState {
    state.as_wire()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::SessionState as S;
    use media_types::error::{AccessDenied, ErrorContext};

    #[test]
    fn fatal_maps_to_wire_playback_error_with_details() {
        let error = EngineError::new(
            PipelineStage::Demux,
            ErrorKind::Inaccessible {
                reason: AccessDenied::NotFound,
            },
            ErrorContext::new("demux.header.missing"),
        );
        let kind = EngineEventKind::Fatal { error };
        match kind.as_wire() {
            Some(EventKind::PlaybackError {
                stage,
                code,
                message,
                details,
                fatal,
            }) => {
                assert_eq!(stage, commands::Stage::Demux);
                assert_eq!(code, commands::ErrorCode::Inaccessible);
                assert!(fatal, "engine Fatal events are fatal on the wire");
                assert_eq!(details.get("context").map(String::as_str), Some("demux.header.missing"));
                assert!(message.contains("inaccessible"), "{message}");
            }
            other => unreachable!("Fatal must map to PlaybackError, got {other:?}"),
        }
    }

    #[test]
    fn private_kinds_never_reach_the_wire() {
        let damaged = EngineEventKind::DamagedData { count: 3 };
        assert!(damaged.as_wire().is_none());
        let first = EngineEventKind::FirstFrame {
            position: Stamp::Missing,
        };
        assert!(first.as_wire().is_none());
    }

    #[test]
    fn state_changed_round_trips_through_the_wire_envelope() {
        let event = EngineEvent {
            seq: 7,
            session: SessionId(1),
            generation: Generation(2),
            origin: Some(RequestId(42)),
            kind: EngineEventKind::StateChanged {
                from: S::Paused,
                to: S::Playing,
            },
        };
        let wire = match event.as_wire_event() {
            Some(w) => w,
            None => unreachable!("state-changed is a wire event"),
        };
        assert_eq!(wire.seq, 7);
        assert_eq!(wire.session, Some(SessionId(1)));
        assert_eq!(wire.generation, Some(Generation(2)));
        assert_eq!(wire.origin, Some(RequestId(42)));
        assert_eq!(
            wire.kind,
            EventKind::SessionStateChanged {
                from: PlaybackState::Paused,
                to: PlaybackState::Playing,
            }
        );
    }

    #[test]
    fn stage_bridge_covers_every_l0_stage() {
        // Total by construction; the assertions pin the judgment calls.
        let stages = [
            PipelineStage::Open,
            PipelineStage::Probe,
            PipelineStage::Demux,
            PipelineStage::Decode,
            PipelineStage::AudioFilter,
            PipelineStage::VideoFilter,
            PipelineStage::Mix,
            PipelineStage::AudioOutput,
            PipelineStage::Present,
            PipelineStage::Clock,
            PipelineStage::Control,
            PipelineStage::Plugin,
            PipelineStage::Discovery,
            PipelineStage::Transcode,
        ];
        for stage in stages {
            let _ = wire_stage(stage); // must answer, never panic
        }
        assert_eq!(wire_stage(PipelineStage::Open), commands::Stage::Input);
        assert_eq!(wire_stage(PipelineStage::Present), commands::Stage::Renderer);
        assert_eq!(wire_stage(PipelineStage::Mix), commands::Stage::AudioOutput);
    }
}
