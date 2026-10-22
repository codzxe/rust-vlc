//! Card 3.B4 — the catalog-side emission ledger: the machine-checked
//! twin of `crates/playback-engine/src/events.rs`'s module-doc audit
//! table (doc 05 §3.2 — 23 rows, D-017-frozen).
//!
//! Every catalog [`EventId`] is accounted for exactly once, in one of
//! two honest buckets:
//!
//! - **emitted-now** — the playback engine (branch state after 3.B4)
//!   produces the event with its doc-05 payload, proven live by the
//!   contract suite (`crates/playback-engine/tests/contract_suite.rs`);
//!   here the *payload shape* is pinned on the wire type itself (the
//!   JSON keys and their doc-05 meaning).
//! - **wired-by-later** — the catalog row exists (frozen data) but no
//!   engine trigger exists yet; the row records the owning card, so
//!   the ledger stays honest about what "event completeness" means
//!   today versus which later card closes the row.
//!
//! This file lives in the `commands` crate (the catalog's home) and
//! deliberately knows nothing about `playback-engine` — a dev edge
//! from commands onto the engine would be a dependency cycle the
//! architecture check rejects. The engine-side proof is the contract
//! suite's job; this is the ledger that keeps both sides counting the
//! same 23 rows.
//!
//! Arithmetic (post-3.A1, post-3.B4): **11 emitted / 12 later**. The
//! seek terminals (3.A1) moved one row from "later" to "emitted" and
//! 3.B4 moved five more (time-tick, duration-known, end-of-track,
//! media.info-ready, and the non-fatal half of playback.error — the
//! fatal half was already emitted by 2.C2).

use std::error::Error;

use commands::EventId;
use commands::catalog::event::EventKind;
use commands::catalog::types::{
    Duration, MediaInfo, SeekKind, SourceSpec, Timestamp, TrackId, TrackInfo, TrackKind,
};
use commands::envelope::Event;
use commands::{CommandError, ErrorCode, PlaybackState, Stage};

/// One ledger row: the catalog id plus the honest note.
struct Row {
    id: EventId,
    /// For emitted rows: the wave that wired the emission. For later
    /// rows: the owning card / phase that will.
    owner: &'static str,
}

/// The 11 rows the engine emits today (post-3.A1 + 3.B4). Order
/// mirrors doc 05 §3.2's table.
const EMITTED: &[Row] = &[
    Row {
        id: EventId::SessionStateChanged,
        owner: "2.C2",
    },
    Row {
        id: EventId::SessionBuffering,
        owner: "2.C2 bookends + 3.B4 in-episode progress",
    },
    Row {
        id: EventId::TimingTimeTick,
        owner: "3.B4",
    },
    Row {
        id: EventId::TimingDurationKnown,
        owner: "3.B4",
    },
    Row {
        id: EventId::TimingSeekCompleted,
        owner: "3.A1",
    },
    Row {
        id: EventId::TimingSeekFailed,
        owner: "3.A1",
    },
    Row {
        id: EventId::TracksListReady,
        owner: "2.C2",
    },
    Row {
        id: EventId::PlaybackEndOfTrack,
        owner: "3.B4",
    },
    Row {
        id: EventId::PlaybackEndOfMedia,
        owner: "2.C2",
    },
    Row {
        id: EventId::PlaybackError,
        owner: "2.C2 fatal + 3.B4 non-fatal (coalesced)",
    },
    Row {
        id: EventId::MediaInfoReady,
        owner: "3.B4",
    },
];

/// The 12 rows honestly owned by later cards — the same notes the
/// engine-side module-doc table carries (the two files must not
/// drift; the partition test below is the tripwire).
const WIRED_BY_LATER: &[Row] = &[
    Row {
        id: EventId::SessionOpeningProgress,
        owner: "network opens do not exist in the engine yet (streaming sources are Phase 6; local opens are synchronous)",
    },
    Row {
        id: EventId::TracksChanged,
        owner: "3.A3 (track-selection commands; no engine-side track-loss trigger exists in v0)",
    },
    Row {
        id: EventId::PlaybackRateChanged,
        owner: "3.B1 (rate support; no set-rate engine input exists)",
    },
    Row {
        id: EventId::AudioDeviceLost,
        owner: "2.C3/7.C1 (device observation; the null device cannot be lost)",
    },
    Row {
        id: EventId::AudioDeviceChanged,
        owner: "2.C3/7.C1 (same)",
    },
    Row {
        id: EventId::VideoScreenshotSaved,
        owner: "renderer/host work (the engine presents through the sink seam, never writes files)",
    },
    Row {
        id: EventId::PlaylistChanged,
        owner: "no playlist model in the engine (Phase 4, 4.D1)",
    },
    Row {
        id: EventId::ConfigSettingChanged,
        owner: "settings writes are host/config-layer work (doc 07, Phase 4)",
    },
    Row {
        id: EventId::PluginsLoaded,
        owner: "plugin registry (doc 06)",
    },
    Row {
        id: EventId::PluginsFailed,
        owner: "plugin registry (doc 06)",
    },
    Row {
        id: EventId::PluginsCrashed,
        owner: "sandboxed WASM host (plugin-host-wasm, Phase 7)",
    },
    Row {
        id: EventId::SystemLog,
        owner: "no logging bridge; doc 05 §3.4's burst-bound drop refinement activates here",
    },
];

/// The ledger partitions the catalog exactly: every one of the 23
/// `EventId`s appears in exactly one bucket, no bucket mentions an id
/// that is not in the catalog (typo guard), and the arithmetic is the
/// module-doc claim (11 + 12 = 23).
#[test]
fn the_ledger_partitions_the_catalog_exactly() {
    assert_eq!(EMITTED.len(), 11, "the emitted bucket's count");
    assert_eq!(WIRED_BY_LATER.len(), 12, "the later bucket's count");
    assert_eq!(
        EMITTED.len() + WIRED_BY_LATER.len(),
        EventId::ALL.len(),
        "the two buckets must sum to the full catalog"
    );
    for id in EventId::ALL {
        let emitted = EMITTED.iter().filter(|row| row.id == *id).count();
        let later = WIRED_BY_LATER.iter().filter(|row| row.id == *id).count();
        assert_eq!(
            emitted + later,
            1,
            "{} must appear in exactly one bucket (emitted: {emitted}, later: {later})",
            id.as_str()
        );
        assert!(
            !row_has_empty_owner(EMITTED, *id) && !row_has_empty_owner(WIRED_BY_LATER, *id),
            "{}'s ledger row must carry its owner note",
            id.as_str()
        );
    }
}

fn row_has_empty_owner(rows: &[Row], id: EventId) -> bool {
    rows.iter()
        .any(|row| row.id == id && row.owner.trim().is_empty())
}

/// The serialized keys of one `EventKind`, as a sorted set.
fn kind_keys(kind: &EventKind) -> Result<Vec<String>, Box<dyn Error>> {
    let event = Event {
        seq: 1,
        session: None,
        generation: None,
        origin: None,
        kind: kind.clone(),
    };
    let value = serde_json::to_value(&event)?;
    let Some(object) = value.as_object() else {
        return Err("the wire event did not serialize to an object".into());
    };
    let mut keys: Vec<String> = object
        .keys()
        .filter(|key| !matches!(key.as_str(), "kind" | "seq" | "session" | "gen" | "origin"))
        .cloned()
        .collect();
    keys.sort();
    Ok(keys)
}

/// A representative minimal `MediaInfo` (the payload of
/// `media.info-ready`; the engine assembles the real one from the
/// open).
fn sample_info() -> MediaInfo {
    MediaInfo {
        source: SourceSpec::Url {
            url: "fixture://sine-colorbars.avi".to_string(),
        },
        container: None,
        title: None,
        duration: Duration::Known {
            time: Timestamp(30_000_000),
        },
        seekable: true,
        is_live: false,
        metadata: std::collections::BTreeMap::new(),
        tracks: vec![TrackInfo {
            id: TrackId(1),
            kind: TrackKind::Video,
            name: None,
            language: None,
            codec: commands::catalog::types::CodecDescriptor {
                fourcc: None,
                description: Some("mjpeg".to_string()),
                profile: None,
                level: None,
            },
            is_default: true,
            is_selected: true,
            origin: commands::catalog::types::TrackOrigin::Stream,
            video: None,
            audio: None,
        }],
        chapters: Vec::new(),
    }
}

/// Every **emitted** row's wire payload carries exactly the doc 05
/// §3.2 payload column — no missing field, no extra field. This is
/// the shape half of the ledger's honesty: the contract suite proves
/// the engine *produces* these; this pins that the wire form the host
/// receives is the documented one.
#[test]
fn emitted_rows_carry_the_doc_05_payload_shapes() -> Result<(), Box<dyn Error>> {
    // (id, a representative payload, the doc-05 §3.2 payload keys in
    // sorted order).
    let cases: Vec<(EventId, EventKind, Vec<&str>)> = vec![
        (
            EventId::SessionStateChanged,
            EventKind::SessionStateChanged {
                from: PlaybackState::Paused,
                to: PlaybackState::Playing,
            },
            vec!["from", "to"],
        ),
        (
            EventId::SessionBuffering,
            EventKind::SessionBuffering { progress: 0.25 },
            vec!["progress"],
        ),
        (
            EventId::TimingTimeTick,
            EventKind::TimingTimeTick {
                time: Timestamp(1_250_000),
                position: 0.041,
            },
            vec!["position", "time"],
        ),
        (
            EventId::TimingDurationKnown,
            EventKind::TimingDurationKnown {
                duration: Timestamp(30_000_000),
            },
            vec!["duration"],
        ),
        (
            EventId::TimingSeekCompleted,
            EventKind::TimingSeekCompleted {
                position: Timestamp(10_000_000),
                seek_kind: SeekKind::Command,
            },
            vec!["position", "seekKind"],
        ),
        (
            EventId::TimingSeekFailed,
            EventKind::TimingSeekFailed {
                error: CommandError::new(Stage::Control, ErrorCode::Unsupported, "not seekable"),
            },
            vec!["error"],
        ),
        (
            EventId::TracksListReady,
            EventKind::TracksListReady { tracks: Vec::new() },
            vec!["tracks"],
        ),
        (
            EventId::PlaybackEndOfTrack,
            EventKind::PlaybackEndOfTrack {
                track_kind: TrackKind::Audio,
                id: TrackId(2),
            },
            vec!["id", "trackKind"],
        ),
        (
            EventId::PlaybackEndOfMedia,
            EventKind::PlaybackEndOfMedia,
            vec![],
        ),
        (
            EventId::PlaybackError,
            EventKind::PlaybackError {
                stage: Stage::Demux,
                code: ErrorCode::Corrupt,
                message: "the engine recovered 3 damaged unit(s) while resynchronizing".to_string(),
                details: std::collections::BTreeMap::new(),
                fatal: false,
            },
            vec!["code", "details", "fatal", "message", "stage"],
        ),
        (
            EventId::MediaInfoReady,
            EventKind::MediaInfoReady {
                info: sample_info(),
            },
            vec!["info"],
        ),
    ];
    assert_eq!(cases.len(), EMITTED.len(), "one shape case per emitted row");
    for (id, kind, expected) in cases {
        assert!(
            EMITTED.iter().any(|row| row.id == id),
            "{} is not in the emitted bucket",
            id.as_str()
        );
        let mut expected: Vec<String> = expected.iter().map(ToString::to_string).collect();
        expected.sort();
        assert_eq!(
            kind_keys(&kind)?,
            expected,
            "{}'s wire payload keys must match doc 05 §3.2",
            id.as_str()
        );
    }
    Ok(())
}
