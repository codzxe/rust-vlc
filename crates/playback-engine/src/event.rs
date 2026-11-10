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
//! - `Seeked` → `timing.seek-completed` (card 3.A1, doc 03 §6 step 7;
//!   the terminal for the generation that completed — doc 05 §4.2)
//! - `SeekFailed` → `timing.seek-failed` (contract D4: typed reason,
//!   old position; exactly one terminal per seek generation — doc 05
//!   §4.3 invariant 4)
//! - `TimeTick` → `timing.time-tick` (card 3.B4, doc 05 §3.2/§3.4
//!   rule 1: the only continuous stream, position-judged, never
//!   queued — the outbox replaces the undelivered older tick)
//! - `DurationKnown` → `timing.duration-known` (card 3.B4: the
//!   greatest declared track extent, once per open, before any
//!   time-tick that assumes it — doc 05 §4.3 invariant 4)
//! - `MediaInfoReady` → `media.info-ready` (card 3.B4: the open-time
//!   snapshot, once per open, before the first `→ playing`)
//! - `EndOfTrack` → `playback.end-of-track` (card 3.B4: a limb
//!   newly drained while the media continues)
//! - `RecoveredDamage` → `playback.error` with `fatal: false` (card
//!   3.B4: bounded-recovery damage, coalesced per 1 s window with a
//!   count — doc 05 §3.2)
//! - `FirstFrame` → **engine-private** (doc 05 §4.2: "internal
//!   trigger of the `playing` transition, no separate v1 wire event")
//! - `DamagedData` → **engine-private** (no v1 wire row; it is
//!   bounded-recovery observability for the log/testkit side)
//!
//! Doc 03 §6 step 1's engine-side `SeekAccepted` has no wire event
//! and no variant here: the class-A `Accepted` receipt IS the accept
//! evidence (doc 05 §1.4 C3 — completion arrives as the terminal
//! events above). Device events and `ClockMasterChanged` remain
//! Phase 3+ (`2.C3`/`7.C1`, `3.B2`). Event *completeness* — every
//! catalog row with payloads and throttling — is card `3.B4`.
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
use std::path::PathBuf;
use std::sync::Arc;

use commands::catalog::types::{
    CodecDescriptor as WireCodecDescriptor, SeekKind as WireSeekKind, TrackInfo as WireTrackInfo,
    TrackKind as WireTrackKind, TrackOrigin as WireTrackOrigin,
};
use commands::envelope::{Generation, RequestId, SessionId};
use commands::{CommandError, EventKind, PlaybackState, WireEvent};
use media_types::error::{ErrorKind, PipelineStage};
use media_types::{EngineError, MediaKind, SourceUri, Stamp, Track, TrackId};

use crate::seek::SeekFailure;
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
    /// The seek completed (card 3.A1; doc 03 §6 step 7): the
    /// completion truth `{requested, actual}` — where the pipeline
    /// stands now, not where it was asked to go. The terminal for
    /// the generation that completed (doc 05 §3.2
    /// `timing.seek-completed`, `seek_kind: command`).
    Seeked {
        /// The requested, clamped target (ns).
        requested: i64,
        /// Where the pipeline actually landed (ns) — the first audio
        /// frame's own timestamp when audio delivered (the anchor
        /// truth), else the demuxer's landing.
        actual: i64,
    },
    /// The seek failed (card 3.A1; contract D4): the typed reason
    /// and the position the session is still at — the prior
    /// position, never a fabricated landing. The terminal for the
    /// failed generation (doc 05 §3.2 `timing.seek-failed`).
    SeekFailed {
        /// Why (typed; the wire form is the redacted
        /// [`CommandError`]).
        reason: SeekFailure,
        /// The position playback continues from (ns).
        position: i64,
    },
    /// The continuous position report (card 3.B4, doc 05 §3.2
    /// `timing.time-tick`): the playback clock's position and its
    /// clamped fraction of the known duration. The only continuous
    /// stream — never queued (doc 05 §3.4 rule 1: a newer tick
    /// replaces an undelivered older one in the outbox). The payload
    /// is plain data so the emission path itself stays allocation-
    /// free (engineering directive D-1).
    TimeTick {
        /// The playback-clock position (ns).
        position_ns: i64,
        /// `0.0..=1.0` of the known duration (`0.0` when the
        /// duration is unknown — the honest floor, never a guess).
        fraction: f32,
    },
    /// The duration became known (card 3.B4, doc 05 §3.2
    /// `timing.duration-known`): the greatest declared track extent,
    /// once per open when derivable — delivered before any time-tick
    /// that assumes it (doc 05 §4.3 invariant 4).
    DurationKnown {
        /// The duration (ns).
        duration_ns: i64,
    },
    /// The metadata snapshot (card 3.B4, doc 05 §3.2
    /// `media.info-ready`): the engine's open-time [`MediaInfo`],
    /// once per open, before the first `state-changed → playing`.
    /// The payload is the wire type itself (the assembly is the
    /// bridge's job — see [`crate::event::media_info_wire`]), boxed
    /// so the one fat variant does not inflate every
    /// [`EngineEventKind`](crate::event::EngineEventKind) — and with
    /// it every outbox slot — by 200 bytes (engineering directive
    /// D-2's mechanical-sympathy rule). The once-per-open cadence is
    /// the documented low-rate exception to the zero-allocation
    /// rule (the `Clone` deep-copies the track list, and `as_wire`
    /// hands over one owned copy per host conversion).
    MediaInfoReady {
        /// The assembled snapshot (source, duration, seekable,
        /// tracks — the same data `media.get-info` answers).
        info: Box<commands::catalog::types::MediaInfo>,
    },
    /// One elementary stream ended while the media continues (card
    /// 3.B4, doc 05 §3.2 `playback.end-of-track`): a limb newly
    /// drained while another still has work. When every limb drains
    /// together the end-of-media event speaks instead — the last
    /// limb's end IS the media's end.
    EndOfTrack {
        /// Which track ended.
        track_kind: MediaKind,
        /// Its engine handle.
        id: TrackId,
    },
    /// A coalesced non-fatal recovery report (card 3.B4, doc 05
    /// §3.2 `playback.error` with `fatal: false`): bounded-recovery
    /// damage per pipeline stage, counted over the 1 s window — the
    /// wire-visible half of the engine-private per-tick
    /// [`EngineEventKind::DamagedData`] fact.
    RecoveredDamage {
        /// The stage that recovered the units.
        stage: PipelineStage,
        /// Units recovered in the window.
        count: u64,
    },
    /// A track selection changed (card 3.A3, doc 05 §3.2
    /// `tracks.changed`): which selection moved and what is now
    /// selected. The **terminal** for the class-A `set-*-track`
    /// commands (emitted under the switch's own generation, doc 05
    /// §4.3 invariant 4 — exactly one terminal per completed
    /// generation) and the immediate confirmation for the class-S
    /// subtitle selection (emitted at the unbumped generation).
    TrackSelectionChanged {
        /// Which selection changed (audio/video/subtitle).
        track_kind: MediaKind,
        /// The now-selected track, if any (`None` = deselected).
        selected: Option<TrackId>,
    },
    /// The active audio output device went away (card 7.C1, doc 05
    /// §3.2 `audio.device-lost`): the OS removed it mid-playback.
    /// Emitted by the engine's hotplug hooks the moment the host's
    /// device seam reports the loss — before the suspend
    /// transition's `state-changed → paused` (the fact precedes the
    /// reaction, the C2 ordering).
    AudioDeviceLost {
        /// The lost device's id (host vocabulary, opaque string).
        device_id: String,
    },
    /// The active audio output device changed (card 7.C1, doc 05
    /// §3.2 `audio.device-changed`): a re-acquired device after a
    /// loss, or a user-picked migration. Emitted before the resume
    /// transition so the wire shows cause-then-effect.
    AudioDeviceChanged {
        /// The new device's id.
        device_id: String,
    },
    /// The audio device suspend's terminal notice (card 7.C1): the
    /// bounded reopen window expired — playback stays suspended at
    /// the remembered position with a **non-fatal** error (the
    /// session's media is fine; killing it would be dishonest). The
    /// wire form is the catalog's generic `playback.error` row,
    /// stage `audio-output`, code `device`, `fatal: false`.
    AudioDeviceSuspended {
        /// The device that was lost.
        device_id: String,
        /// Failed reopen attempts in the exhausted window.
        reopen_attempts: u32,
        /// The last observed failure's display-safe summary, when a
        /// failure was observed (a pure time-expiry has none).
        last_error: Option<String>,
    },
    /// The render backend changed (card 7.C1, doc 05 §3.2
    /// `video.backend-changed`): the initial auto-negotiation
    /// report, a hardware→software fallback, or a user-forced
    /// selection. The host relays the renderer's typed capability
    /// report into the engine's event stream here — the engine owns
    /// the ONE wire event stream, the renderer owns the negotiation.
    RendererBackendReport {
        /// The now-active backend id (redacted vocabulary).
        backend: String,
        /// Why it changed (negotiation outcome / fallback cause /
        /// "user").
        reason: String,
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
            EngineEventKind::DamagedData { count } => {
                EngineEventKind::DamagedData { count: *count }
            }
            EngineEventKind::FirstFrame { position } => EngineEventKind::FirstFrame {
                position: *position,
            },
            EngineEventKind::Seeked { requested, actual } => EngineEventKind::Seeked {
                requested: *requested,
                actual: *actual,
            },
            EngineEventKind::SeekFailed { reason, position } => EngineEventKind::SeekFailed {
                reason: reason.clone(),
                position: *position,
            },
            EngineEventKind::TimeTick {
                position_ns,
                fraction,
            } => EngineEventKind::TimeTick {
                position_ns: *position_ns,
                fraction: *fraction,
            },
            EngineEventKind::DurationKnown { duration_ns } => EngineEventKind::DurationKnown {
                duration_ns: *duration_ns,
            },
            EngineEventKind::MediaInfoReady { info } => EngineEventKind::MediaInfoReady {
                info: Box::new((**info).clone()),
            },
            EngineEventKind::EndOfTrack { track_kind, id } => EngineEventKind::EndOfTrack {
                track_kind: *track_kind,
                id: *id,
            },
            EngineEventKind::RecoveredDamage { stage, count } => EngineEventKind::RecoveredDamage {
                stage: *stage,
                count: *count,
            },
            EngineEventKind::TrackSelectionChanged {
                track_kind,
                selected,
            } => EngineEventKind::TrackSelectionChanged {
                track_kind: *track_kind,
                selected: *selected,
            },
            EngineEventKind::AudioDeviceLost { device_id } => EngineEventKind::AudioDeviceLost {
                device_id: device_id.clone(),
            },
            EngineEventKind::AudioDeviceChanged { device_id } => {
                EngineEventKind::AudioDeviceChanged {
                    device_id: device_id.clone(),
                }
            }
            EngineEventKind::AudioDeviceSuspended {
                device_id,
                reopen_attempts,
                last_error,
            } => EngineEventKind::AudioDeviceSuspended {
                device_id: device_id.clone(),
                reopen_attempts: *reopen_attempts,
                last_error: last_error.clone(),
            },
            EngineEventKind::RendererBackendReport { backend, reason } => {
                EngineEventKind::RendererBackendReport {
                    backend: backend.clone(),
                    reason: reason.clone(),
                }
            }
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
                EngineEventKind::TracksChanged {
                    tracks: a,
                    selected: b,
                },
                EngineEventKind::TracksChanged {
                    tracks: c,
                    selected: d,
                },
            ) => **a == **c && b == d,
            (
                EngineEventKind::Buffering { progress: a },
                EngineEventKind::Buffering { progress: b },
            ) => a == b,
            (EngineEventKind::EndOfMedia, EngineEventKind::EndOfMedia) => true,
            (EngineEventKind::Fatal { error: a }, EngineEventKind::Fatal { error: b }) => {
                a.stage == b.stage && a.kind == b.kind && a.context.code == b.context.code
            }
            (
                EngineEventKind::DamagedData { count: a },
                EngineEventKind::DamagedData { count: b },
            ) => a == b,
            (
                EngineEventKind::FirstFrame { position: a },
                EngineEventKind::FirstFrame { position: b },
            ) => a == b,
            (
                EngineEventKind::Seeked {
                    requested: a,
                    actual: b,
                },
                EngineEventKind::Seeked {
                    requested: c,
                    actual: d,
                },
            ) => a == c && b == d,
            (
                EngineEventKind::SeekFailed {
                    reason: a,
                    position: b,
                },
                EngineEventKind::SeekFailed {
                    reason: c,
                    position: d,
                },
            ) => a == c && b == d,
            (
                EngineEventKind::TimeTick {
                    position_ns: a,
                    fraction: b,
                },
                EngineEventKind::TimeTick {
                    position_ns: c,
                    fraction: d,
                },
            ) => a == c && b == d,
            (
                EngineEventKind::DurationKnown { duration_ns: a },
                EngineEventKind::DurationKnown { duration_ns: b },
            ) => a == b,
            (
                EngineEventKind::MediaInfoReady { info: a },
                EngineEventKind::MediaInfoReady { info: b },
            ) => a == b,
            (
                EngineEventKind::EndOfTrack {
                    track_kind: a,
                    id: b,
                },
                EngineEventKind::EndOfTrack {
                    track_kind: c,
                    id: d,
                },
            ) => a == c && b == d,
            (
                EngineEventKind::RecoveredDamage { stage: a, count: b },
                EngineEventKind::RecoveredDamage { stage: c, count: d },
            ) => a == c && b == d,
            (
                EngineEventKind::TrackSelectionChanged {
                    track_kind: a,
                    selected: b,
                },
                EngineEventKind::TrackSelectionChanged {
                    track_kind: c,
                    selected: d,
                },
            ) => a == c && b == d,
            (
                EngineEventKind::AudioDeviceLost { device_id: a },
                EngineEventKind::AudioDeviceLost { device_id: b },
            ) => a == b,
            (
                EngineEventKind::AudioDeviceChanged { device_id: a },
                EngineEventKind::AudioDeviceChanged { device_id: b },
            ) => a == b,
            (
                EngineEventKind::AudioDeviceSuspended {
                    device_id: a,
                    reopen_attempts: b,
                    last_error: c,
                },
                EngineEventKind::AudioDeviceSuspended {
                    device_id: d,
                    reopen_attempts: e,
                    last_error: f,
                },
            ) => a == d && b == e && c == f,
            (
                EngineEventKind::RendererBackendReport {
                    backend: a,
                    reason: b,
                },
                EngineEventKind::RendererBackendReport {
                    backend: c,
                    reason: d,
                },
            ) => a == c && b == d,
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
                // The message WITHOUT `Display`'s `[stage]` prefix —
                // this event line prints `[stage.code]` itself (the
                // F4 fix: no doubled bracket).
                message: error.message_summary(),
                details: fatal_details(error),
                fatal: true,
            }),
            EngineEventKind::DamagedData { .. } => None,
            EngineEventKind::FirstFrame { .. } => None,
            EngineEventKind::Seeked { actual, .. } => Some(EventKind::TimingSeekCompleted {
                position: commands::catalog::types::Timestamp(ns_to_us(*actual)),
                seek_kind: WireSeekKind::Command,
            }),
            EngineEventKind::SeekFailed { reason, position } => Some(EventKind::TimingSeekFailed {
                error: seek_error_wire(reason, *position),
            }),
            EngineEventKind::TimeTick {
                position_ns,
                fraction,
            } => Some(EventKind::TimingTimeTick {
                time: commands::catalog::types::Timestamp(ns_to_us(*position_ns)),
                position: *fraction,
            }),
            EngineEventKind::DurationKnown { duration_ns } => {
                Some(EventKind::TimingDurationKnown {
                    duration: commands::catalog::types::Timestamp(ns_to_us(*duration_ns)),
                })
            }
            EngineEventKind::MediaInfoReady { info } => Some(EventKind::MediaInfoReady {
                info: (**info).clone(),
            }),
            EngineEventKind::EndOfTrack { track_kind, id } => Some(EventKind::PlaybackEndOfTrack {
                track_kind: wire_track_kind(*track_kind),
                id: commands::catalog::types::TrackId(id.handle()),
            }),
            EngineEventKind::RecoveredDamage { stage, count } => Some(EventKind::PlaybackError {
                stage: wire_stage(*stage),
                code: commands::ErrorCode::Corrupt,
                message: format!(
                    "the engine recovered {count} damaged unit(s) while resynchronizing"
                ),
                details: recovered_damage_details(*count),
                fatal: false,
            }),
            EngineEventKind::TrackSelectionChanged {
                track_kind,
                selected,
            } => Some(EventKind::TracksChanged {
                track_kind: wire_track_change_kind(*track_kind),
                selected: selected.map(|id| commands::catalog::types::TrackId(id.handle())),
            }),
            EngineEventKind::AudioDeviceLost { device_id } => Some(EventKind::AudioDeviceLost {
                device_id: device_id.clone(),
            }),
            EngineEventKind::AudioDeviceChanged { device_id } => {
                Some(EventKind::AudioDeviceChanged {
                    device_id: device_id.clone(),
                })
            }
            EngineEventKind::AudioDeviceSuspended {
                device_id,
                reopen_attempts,
                last_error,
            } => Some(EventKind::PlaybackError {
                stage: commands::Stage::AudioOutput,
                code: commands::ErrorCode::Device,
                message: suspend_notice_message(device_id, *reopen_attempts, last_error.as_deref()),
                details: suspend_notice_details(device_id, *reopen_attempts),
                fatal: false,
            }),
            EngineEventKind::RendererBackendReport { backend, reason } => {
                Some(EventKind::VideoBackendChanged {
                    backend: backend.clone(),
                    reason: reason.clone(),
                })
            }
        }
    }

    /// Whether this kind is pipeline-scoped (doc 05 §3.2 "gen"
    /// column — subject to the §4.2 staleness rule). The v0 kinds all
    /// were; card 7.C1 adds the first session-scoped kinds — the
    /// audio device rows and the renderer backend report — which doc
    /// §3.2 marks "Not pipeline-scoped": they speak for the session's
    /// environment, not for one pipeline generation, so a superseding
    /// mint (a seek, a track switch) must not swallow them at drain.
    #[must_use]
    pub fn is_pipeline_scoped(&self) -> bool {
        !matches!(
            self,
            EngineEventKind::AudioDeviceLost { .. }
                | EngineEventKind::AudioDeviceChanged { .. }
                | EngineEventKind::RendererBackendReport { .. }
        )
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
        PipelineStage::Decode | PipelineStage::AudioFilter | PipelineStage::VideoFilter => {
            W::Decode
        }
        PipelineStage::Mix | PipelineStage::AudioOutput => W::AudioOutput,
        PipelineStage::Present => W::Renderer,
        PipelineStage::Clock
        | PipelineStage::Control
        | PipelineStage::Discovery
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

/// Bridge a seek failure to the wire `timing.seek-failed` payload
/// (doc 05 §3.2: typed, redacted). The engine-private vocabulary for
/// the refusal rides as the `reason` detail; the position the session
/// continues from (the D4 rollback point) rides as `position_us` —
/// `[PROPOSAL: the wire event's only typed field is the CommandError;
/// the old position is completion truth hosts need to show where
/// playback continues]`.
fn seek_error_wire(reason: &SeekFailure, position_ns: i64) -> CommandError {
    let (stage, code, message) = match reason {
        // The engine's policy facts (an unseekable source, an
        // unresolvable target) surface as control-stage refusals;
        // carried failures keep their own stage and kind.
        SeekFailure::NotSeekable => (
            commands::Stage::Control,
            commands::ErrorCode::Unsupported,
            "the source cannot seek".to_string(),
        ),
        SeekFailure::OutOfRange => (
            commands::Stage::Control,
            commands::ErrorCode::Unsupported,
            "the seek target could not be resolved".to_string(),
        ),
        SeekFailure::Demux(error) | SeekFailure::Clock(error) => (
            wire_stage(error.stage),
            wire_error_code(&error.kind),
            error.to_string(),
        ),
    };
    CommandError::new(stage, code, message)
        .with_detail("reason", seek_failure_reason(reason))
        .with_detail("position_us", ns_to_us(position_ns).to_string())
}

/// Nanoseconds → microseconds, rounded up (the wire `Timestamp` unit,
/// doc 05 §2.2). Positions are non-negative by construction (targets
/// clamp to `[0, duration]` and landings are keyframe positions).
fn ns_to_us(ns: i64) -> i64 {
    (ns + 999) / 1_000
}

/// The stable reason key for the wire `timing.seek-failed` details.
fn seek_failure_reason(reason: &SeekFailure) -> &'static str {
    match reason {
        SeekFailure::NotSeekable => "not-seekable",
        SeekFailure::OutOfRange => "out-of-range",
        SeekFailure::Demux(_) => "demux",
        SeekFailure::Clock(_) => "clock",
    }
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

/// Machine-readable hints for the non-fatal `playback.error`
/// payload: the coalesced window count (doc 05 §3.2's "with a
/// count"). `[PROPOSAL: the coalescing rule itself is `[PROPOSAL]`
/// in the doc; the key name is this bridge's choice]`.
fn recovered_damage_details(count: u64) -> BTreeMap<String, String> {
    let mut details = BTreeMap::new();
    details.insert("count".to_string(), count.to_string());
    details
}

/// The `AudioDeviceSuspended` wire payload's machine-readable hints
/// — the same keys the recovery guard's own notice carries (the
/// direct-drive and engine-hook paths must stay byte-identical; the
/// contract suite pins the join).
fn suspend_notice_details(device_id: &str, reopen_attempts: u32) -> BTreeMap<String, String> {
    let mut details = BTreeMap::new();
    details.insert("context".to_string(), "audio.device.suspend".to_string());
    details.insert("device".to_string(), device_id.to_string());
    details.insert("reopen_attempts".to_string(), reopen_attempts.to_string());
    details
}

/// The `AudioDeviceSuspended` wire payload's display-safe message
/// (V8; mirrors [`crate::recovery`]`'s notice wording — the guard's
/// direct-drive emission and this engine-hook translation are the
/// same sentence by construction).
fn suspend_notice_message(
    device_id: &str,
    reopen_attempts: u32,
    last_error: Option<&str>,
) -> String {
    match last_error {
        Some(error) => format!(
            "audio device `{device_id}` lost and re-acquisition failed \
             ({error}); playback is suspended at the remembered position \
             after {reopen_attempts} attempt(s)"
        ),
        None => format!(
            "audio device `{device_id}` lost and the re-acquisition \
             window expired after {reopen_attempts} attempt(s); playback \
             is suspended at the remembered position"
        ),
    }
}

/// Bridge an engine track kind to the wire track kind.
#[must_use]
fn wire_track_kind(kind: MediaKind) -> WireTrackKind {
    match kind {
        MediaKind::Audio => WireTrackKind::Audio,
        MediaKind::Video => WireTrackKind::Video,
        MediaKind::Subtitle => WireTrackKind::Subtitle,
        MediaKind::Data => WireTrackKind::Data,
    }
}

/// Bridge an engine track kind to the `tracks.changed` selection
/// kind (doc 05 §3.2: [`WireTrackKind`] plus the synthetic
/// `secondary-audio` arm, which card 3.A3 does not mint — the
/// follow-up card owns it).
#[must_use]
fn wire_track_change_kind(kind: MediaKind) -> commands::catalog::types::TrackChangeKind {
    use commands::catalog::types::TrackChangeKind;
    match kind {
        MediaKind::Audio => TrackChangeKind::Audio,
        MediaKind::Video => TrackChangeKind::Video,
        MediaKind::Subtitle => TrackChangeKind::Subtitle,
        MediaKind::Data => TrackChangeKind::Data,
    }
}

/// Bridge the engine's source URI to the wire source spec (doc 05
/// §2.2). `[PROPOSAL]`: the engine speaks the URI's **redacted**
/// display form (V8 — it never re-derives the raw path; hosts that
/// need the exact original carry it themselves, as the CLI host
/// does for `media.get-info`); `file` URIs surface as `File` specs
/// over the redacted path, everything else as a `Url`.
#[must_use]
pub fn source_spec_wire(source: &SourceUri) -> commands::catalog::types::SourceSpec {
    use commands::catalog::types::SourceSpec;
    let redacted = source.to_string();
    if source.scheme().as_str() == "file" {
        SourceSpec::File {
            path: PathBuf::from(redacted.trim_start_matches("file://")),
        }
    } else {
        SourceSpec::Url { url: redacted }
    }
}

/// Assemble the wire [`MediaInfo`] snapshot for `media.info-ready`
/// (card 3.B4, doc 05 §3.2 — the same data `media.get-info`
/// answers). Heuristics are `[PROPOSAL]`-tagged at their site:
/// `seekable` is the demuxer's declared [`SeekSupport`](plugin_api::SeekSupport)
/// (non-`None`), `is_live` is `false` (no live sources exist in the
/// engine yet), `container`/`title`/`metadata`/`chapters` are the
/// v0 honest unknowns (the probe-side enrichment lands with the
/// hosts that own it).
#[must_use]
pub fn media_info_wire(
    source: &SourceUri,
    duration_ns: Option<i64>,
    seekable: bool,
    tracks: &[Track],
    selected: &[bool],
) -> commands::catalog::types::MediaInfo {
    commands::catalog::types::MediaInfo {
        source: source_spec_wire(source),
        container: None,
        title: None,
        duration: match duration_ns {
            Some(ns) => commands::catalog::types::Duration::Known {
                time: commands::catalog::types::Timestamp(ns_to_us(ns)),
            },
            None => commands::catalog::types::Duration::Unknown,
        },
        seekable,
        is_live: false,
        metadata: BTreeMap::new(),
        tracks: tracks
            .iter()
            .zip(selected.iter())
            .map(|(track, &is_selected)| track_info_wire(track, is_selected))
            .collect(),
        chapters: Vec::new(),
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
                assert_eq!(
                    details.get("context").map(String::as_str),
                    Some("demux.header.missing")
                );
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
        assert_eq!(
            wire_stage(PipelineStage::Present),
            commands::Stage::Renderer
        );
        assert_eq!(wire_stage(PipelineStage::Mix), commands::Stage::AudioOutput);
    }
}
