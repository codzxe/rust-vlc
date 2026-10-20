//! The event payload enum (doc 05 §3.2 — card 1.D4).
//!
//! [`EventKind`] is the payload-carrying twin of
//! [`EventId`](crate::catalog::ids::EventId): one variant per §3.2
//! table row, payloads verbatim from the *Payload* column. It is
//! internally tagged (`"kind"`), so inside
//! [`Event<EventKind>`](crate::envelope::Event) the payload fields sit
//! beside the envelope fields — exactly the §5.1 example:
//!
//! ```json
//! { "seq": 1031, "session": 7, "gen": 4, "origin": 42,
//!   "kind": "timing.seek-completed", "position": 63000000,
//!   "seekKind": "command" }
//! ```
//!
//! Payload `origin`/`gen` mentions in the §3.2 table are the ENVELOPE's
//! fields (§3.1), not duplicated here: the table lists them in the
//! payload column because they are semantically crucial (e.g.
//! `timing.seek-completed`'s `gen` is the completion-matching
//! generation of §1.4 C3), but on the wire each appears exactly once
//! — the flattened envelope's `gen`/`origin` keys.

use std::collections::BTreeMap;
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::catalog::types::LogLevel;
use crate::catalog::types::MediaInfo;
use crate::catalog::types::PlaybackState;
use crate::catalog::types::PlaylistChange;
use crate::catalog::types::PluginAction;
use crate::catalog::types::SeekKind;
use crate::catalog::types::SettingValue;
use crate::catalog::types::Timestamp;
use crate::catalog::types::TrackChangeKind;
use crate::catalog::types::TrackId;
use crate::catalog::types::TrackInfo;
use crate::catalog::types::TrackKind;
use crate::error::CommandError;
use crate::error::ErrorCode;
use crate::error::Stage;

/// One engine → host event, with its payload (doc 05 §3.2 — 23 rows).
///
/// `#[non_exhaustive]`: unknown `kind` values received by older hosts
/// degrade to a logged no-op (§2.1, §6.2). The completion-evidence
/// events for class-A commands (§1.4 C3) are `session.state-changed`,
/// `timing.seek-completed`, `timing.seek-failed`, `playback.error`,
/// `tracks.changed`, `tracks.list-ready`, `media.info-ready`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all_fields = "camelCase")]
pub enum EventKind {
    /// `session.state-changed` — every playback-state transition,
    /// including self-transitions as idempotent confirmations (C8).
    /// Never coalesced.
    #[serde(rename = "session.state-changed")]
    SessionStateChanged {
        /// The state we left.
        from: PlaybackState,
        /// The state we entered.
        to: PlaybackState,
    },
    /// `session.opening-progress` — network opens, best-effort, ≤ 1 Hz;
    /// absent for local files.
    #[serde(rename = "session.opening-progress")]
    SessionOpeningProgress {
        /// `0.0..=1.0`.
        progress: f32,
    },
    /// `session.buffering` — enter/while/exit buffering, ≤ 5 Hz; opens
    /// with 0.0 and closes with 1.0.
    #[serde(rename = "session.buffering")]
    SessionBuffering {
        /// `0.0..=1.0`.
        progress: f32,
    },
    /// `timing.time-tick` — continuous position while playing; the only
    /// continuous stream, never queued (§3.4).
    #[serde(rename = "timing.time-tick")]
    TimingTimeTick {
        /// Position (µs).
        time: Timestamp,
        /// `0.0..=1.0` of duration.
        position: f32,
    },
    /// `timing.duration-known` — the duration transitioned from
    /// unknown/early-estimate to final; once per change.
    #[serde(rename = "timing.duration-known")]
    TimingDurationKnown {
        /// The final duration (µs).
        duration: Timestamp,
    },
    /// `timing.seek-completed` — terminal for the NEWEST seek generation
    /// only (§4.2: superseded seeks are silent). The envelope's `gen`
    /// is the generation that completed; it must match the `Accepted`
    /// response's gen (§1.4 C3).
    #[serde(rename = "timing.seek-completed")]
    TimingSeekCompleted {
        /// Where the pipeline is now (µs).
        position: Timestamp,
        /// What asked for the seek.
        seek_kind: SeekKind,
    },
    /// `timing.seek-failed` — terminal failure for the newest seek
    /// generation; exactly one terminal seek event per completed gen
    /// (§4.3 invariant 4). The envelope's `gen` is the failed
    /// generation.
    #[serde(rename = "timing.seek-failed")]
    TimingSeekFailed {
        /// Why it failed (typed, redacted).
        error: CommandError,
    },
    /// `tracks.list-ready` — the list became available/relevant: before
    /// first `playing`, after `subtitle.add-file`, after rare changes.
    /// Coalesced ≤ 1 Hz. Delivered before the first `state-changed →
    /// playing` of a session (§4.3 terminal invariant 4).
    #[serde(rename = "tracks.list-ready")]
    TracksListReady {
        /// The full live list.
        tracks: Vec<TrackInfo>,
    },
    /// `tracks.changed` — a selection applied (command) or lost (the
    /// track disappeared, engine-initiated).
    #[serde(rename = "tracks.changed")]
    TracksChanged {
        /// Which selection changed.
        track_kind: TrackChangeKind,
        /// The now-selected track, if any.
        selected: Option<TrackId>,
    },
    /// `playback.end-of-track` — one elementary stream ended while the
    /// media continues (e.g. audio before video).
    #[serde(rename = "playback.end-of-track")]
    PlaybackEndOfTrack {
        /// Which track ended.
        track_kind: TrackKind,
        /// Its handle.
        id: TrackId,
    },
    /// `playback.end-of-media` — the media finished; followed by
    /// `state-changed → ended`, then the advance lifecycle (or nothing).
    #[serde(rename = "playback.end-of-media")]
    PlaybackEndOfMedia,
    /// `playback.error` — any stage failure. Fatal errors transition
    /// state to `error` and are always ordered BEFORE their
    /// `state-changed → error` (§3.3). Non-fatal same-code errors are
    /// coalesced per 1 s window with a count `[PROPOSAL]`.
    #[serde(rename = "playback.error")]
    PlaybackError {
        /// Where it failed.
        stage: Stage,
        /// Typed failure class.
        code: ErrorCode,
        /// Redacted, display-safe message (V8).
        message: String,
        /// Machine-readable hints.
        details: BTreeMap<String, String>,
        /// Fatal errors transition the state to `error`.
        fatal: bool,
    },
    /// `playback.rate-changed` — `set-rate` echo and future engine-side
    /// rate changes. Not pipeline-scoped.
    #[serde(rename = "playback.rate-changed")]
    PlaybackRateChanged {
        /// The new rate.
        rate: f32,
    },
    /// `audio.device-lost` — the OS removed the active device; migration
    /// or a non-fatal error follows. Not pipeline-scoped.
    #[serde(rename = "audio.device-lost")]
    AudioDeviceLost {
        /// The lost device's id.
        device_id: String,
    },
    /// `audio.device-changed` — the active output device changed
    /// (command or auto-migration). Not pipeline-scoped.
    #[serde(rename = "audio.device-changed")]
    AudioDeviceChanged {
        /// The new device's id.
        device_id: String,
    },
    /// `video.screenshot-saved` — the screenshot was written; completion
    /// evidence for `video.take-screenshot`. Not pipeline-scoped.
    #[serde(rename = "video.screenshot-saved")]
    VideoScreenshotSaved {
        /// Where it was written.
        path: PathBuf,
        /// Captured frame width.
        width: u32,
        /// Captured frame height.
        height: u32,
    },
    /// `playlist.changed` — any playlist model mutation, command- or
    /// engine-initiated; model edits coalesced per 100 ms
    /// `[PROPOSAL]`. Not pipeline-scoped.
    #[serde(rename = "playlist.changed")]
    PlaylistChanged {
        /// Which mutation happened.
        change: PlaylistChange,
    },
    /// `config.setting-changed` — a setting changed through any layer
    /// (user, CLI flag, admin, plugin). Not pipeline-scoped.
    #[serde(rename = "config.setting-changed")]
    ConfigSettingChanged {
        /// Settings area.
        area: String,
        /// Setting key.
        key: String,
        /// The new value.
        value: SettingValue,
        /// Which layer wrote it.
        layer: String,
    },
    /// `plugins.loaded` — a plugin registered (startup or install flow).
    /// Not pipeline-scoped.
    ///
    /// `[VERIFY]` — §3.2 sketches the payload as `{ id, kind, version }`,
    /// but `kind` is the event envelope's discriminator (§5.1): a payload
    /// field of that name is impossible on the wire. The payload field
    /// serializes as `pluginKind`.
    #[serde(rename = "plugins.loaded")]
    PluginsLoaded {
        /// Plugin id.
        id: String,
        /// Plugin kind (manifest-declared, `[VERIFY]` — string until
        /// the doc 06 taxonomy freezes).
        plugin_kind: String,
        /// Plugin version.
        version: String,
    },
    /// `plugins.failed` — a plugin failed to load; the engine continues
    /// without it. Not pipeline-scoped.
    #[serde(rename = "plugins.failed")]
    PluginsFailed {
        /// Plugin id.
        id: String,
        /// Display-safe failure message (V8).
        error: String,
    },
    /// `plugins.crashed` — a sandboxed WASM extension crashed; playback
    /// survives (MP §6 error isolation). Not pipeline-scoped.
    #[serde(rename = "plugins.crashed")]
    PluginsCrashed {
        /// Plugin id.
        id: String,
        /// Display-safe crash message (V8).
        error: String,
        /// What the engine did about it.
        action_taken: PluginAction,
    },
    /// `system.log` — a log line for the in-app panel; level-filtered
    /// by `system.set-log-level`. Rate-limited per target (default
    /// 100/s then an aggregate "N suppressed") `[PROPOSAL]`.
    #[serde(rename = "system.log")]
    SystemLog {
        /// Line level.
        level: LogLevel,
        /// Module-path target.
        target: String,
        /// The line (redaction-safe, V8).
        message: String,
    },
    /// `media.info-ready` — the metadata snapshot just before first
    /// `playing` (hydration convenience); once per open.
    #[serde(rename = "media.info-ready")]
    MediaInfoReady {
        /// The snapshot (same data as `media.get-info`).
        info: MediaInfo,
    },
}

/// Convenience: the id of the (single) playlist item a change speaks
/// about, when it speaks about exactly one.
impl EventKind {
    /// Whether this event kind is one of the §1.4 C3 completion-evidence
    /// events for class-A commands.
    pub fn is_completion_evidence(&self) -> bool {
        matches!(
            self,
            EventKind::SessionStateChanged { .. }
                | EventKind::TimingSeekCompleted { .. }
                | EventKind::TimingSeekFailed { .. }
                | EventKind::PlaybackError { .. }
                | EventKind::TracksChanged { .. }
                | EventKind::TracksListReady { .. }
                | EventKind::MediaInfoReady { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::types::PlaylistItemId;
    use crate::envelope::Event;
    use crate::envelope::Generation;
    use crate::envelope::RequestId;
    use crate::envelope::SessionId;

    /// The §5.1 event example, byte for byte (modulo key order).
    #[test]
    fn event_wire_form_matches_doc_example() {
        let event = Event {
            seq: 1031,
            session: Some(SessionId(7)),
            generation: Some(Generation(4)),
            origin: Some(RequestId(42)),
            kind: EventKind::TimingSeekCompleted {
                position: Timestamp(63_000_000),
                seek_kind: SeekKind::Command,
            },
        };
        let json = serde_json::to_string(&event).unwrap_or_default();
        assert_eq!(
            json,
            r#"{"seq":1031,"session":7,"gen":4,"origin":42,"kind":"timing.seek-completed","position":63000000,"seekKind":"command"}"#
        );
        match serde_json::from_str::<Event<EventKind>>(&json) {
            Ok(back) => assert_eq!(back, event),
            Err(err) => unreachable!("round-trip failed: {err}"),
        }
    }

    #[test]
    fn sessionless_events_omit_optional_envelope_fields() {
        let event = Event {
            seq: 2,
            session: None,
            generation: None,
            origin: None,
            kind: EventKind::PlaybackRateChanged { rate: 2.0 },
        };
        let json = serde_json::to_string(&event).unwrap_or_default();
        assert_eq!(
            json,
            r#"{"seq":2,"kind":"playback.rate-changed","rate":2.0}"#
        );
        match serde_json::from_str::<Event<EventKind>>(&json) {
            Ok(back) => assert_eq!(back, event),
            Err(err) => unreachable!("round-trip failed: {err}"),
        }
    }

    #[test]
    fn unknown_event_kinds_are_rejected() {
        let json = r#"{"seq":1,"kind":"nope.nope"}"#;
        match serde_json::from_str::<Event<EventKind>>(json) {
            Ok(_) => unreachable!("unknown kind accepted"),
            Err(err) => assert!(
                err.to_string().contains("variant"),
                "unexpected error: {err}"
            ),
        }
    }

    /// §3.2's payload sketch for `playlist.changed` renders as the
    /// nested `change` discriminator (the payload's own `kind` slot is
    /// taken by the event tag).
    #[test]
    fn playlist_changed_nests_its_mutation() {
        let event = Event {
            seq: 5,
            session: None,
            generation: None,
            origin: None,
            kind: EventKind::PlaylistChanged {
                change: PlaylistChange::Moved {
                    id: PlaylistItemId(9),
                    from: 1,
                    to: 2,
                },
            },
        };
        let json = serde_json::to_string(&event).unwrap_or_default();
        assert_eq!(
            json,
            r#"{"seq":5,"kind":"playlist.changed","change":{"kind":"moved","id":9,"from":1,"to":2}}"#
        );
        match serde_json::from_str::<Event<EventKind>>(&json) {
            Ok(back) => assert_eq!(back, event),
            Err(err) => unreachable!("round-trip failed: {err}"),
        }
    }

    /// §3.2's completion-evidence set (§1.4 C3). Internally-tagged
    /// events cannot parse without their payload fields, so each sample
    /// is BUILT, serialized through the envelope (the wire form) and
    /// parsed back — one real round-trip per id, then classified.
    #[test]
    fn completion_evidence_set_matches_section_1_4() {
        let media_info = MediaInfo {
            source: crate::catalog::types::SourceSpec::File {
                path: PathBuf::from("fixture://sine.mkv"),
            },
            container: None,
            title: None,
            duration: crate::catalog::types::Duration::Unknown,
            seekable: true,
            is_live: false,
            metadata: BTreeMap::new(),
            tracks: Vec::new(),
            chapters: Vec::new(),
        };
        let evidence: [(EventKind, bool); 12] = [
            (
                EventKind::SessionStateChanged {
                    from: PlaybackState::Idle,
                    to: PlaybackState::Playing,
                },
                true,
            ),
            (
                EventKind::TimingSeekCompleted {
                    position: Timestamp(1),
                    seek_kind: SeekKind::Command,
                },
                true,
            ),
            (
                EventKind::TimingSeekFailed {
                    error: CommandError::new(Stage::Demux, ErrorCode::NotSeekable, "nope"),
                },
                true,
            ),
            (
                EventKind::PlaybackError {
                    stage: Stage::Decode,
                    code: ErrorCode::Decoder,
                    message: "dropped".to_owned(),
                    details: BTreeMap::new(),
                    fatal: false,
                },
                true,
            ),
            (
                EventKind::TracksChanged {
                    track_kind: TrackChangeKind::Audio,
                    selected: Some(TrackId(1)),
                },
                true,
            ),
            (EventKind::TracksListReady { tracks: Vec::new() }, true),
            (EventKind::MediaInfoReady { info: media_info }, true),
            (
                EventKind::TimingTimeTick {
                    time: Timestamp(0),
                    position: 0.0,
                },
                false,
            ),
            (EventKind::PlaybackRateChanged { rate: 1.0 }, false),
            (
                EventKind::PlaylistChanged {
                    change: PlaylistChange::Cleared,
                },
                false,
            ),
            (
                EventKind::SystemLog {
                    level: LogLevel::Info,
                    target: "lumen::engine".to_owned(),
                    message: "line".to_owned(),
                },
                false,
            ),
            (
                EventKind::PluginsCrashed {
                    id: "org.example.bad".to_owned(),
                    error: "trap".to_owned(),
                    action_taken: PluginAction::Disabled,
                },
                false,
            ),
        ];
        for (kind, expected) in evidence {
            let event = Event {
                seq: 1,
                session: None,
                generation: None,
                origin: None,
                kind,
            };
            let json = serde_json::to_string(&event).unwrap_or_default();
            match serde_json::from_str::<Event<EventKind>>(&json) {
                Ok(back) => assert_eq!(
                    back.kind.is_completion_evidence(),
                    expected,
                    "{json} misclassified"
                ),
                Err(err) => unreachable!("{json} rejected: {err}"),
            }
        }
    }
}
