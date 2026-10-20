//! Card 1.D4 — wire-form round-trips (doc 05 §5.1).
//!
//! The one serde path every host shares: camelCase field names,
//! internally-tagged envelopes (`"cmd"`/`"kind"`), kebab-case variant
//! tags. The doc's own §5.1 examples are pinned BYTE FOR BYTE (key
//! order included — serde emits struct fields in declaration order, the
//! doc examples follow the same order), plus representative payload
//! shapes chosen to exercise the rename rules that are easy to get
//! wrong (`deltaMs`, `seekKind`, `deviceId`, `toIndex`, `schemaMeta`,
//! `restartRequired`).

use std::error::Error;

use commands::CommandError;
use commands::CommandOutcome;
use commands::ErrorCode;
use commands::Event;
use commands::Generation;
use commands::RequestId;
use commands::SessionId;
use commands::Stage;
use commands::WireEvent;
use commands::WireRequest;
use commands::WireResponse;
use commands::catalog::Command;
use commands::catalog::CommandResult;
use commands::catalog::EventKind;
use commands::catalog::types::SeekTarget;
use commands::catalog::types::Timestamp;

/// Serialize, assert the exact JSON, deserialize back, assert equality —
/// the full round-trip with a golden string. Serde failures propagate
/// as test errors with the golden string in the message.
fn round_trip<T>(value: &T, golden: &str) -> Result<(), Box<dyn Error>>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let json = serde_json::to_string(value)?;
    assert_eq!(json, golden, "serialized wire form drifted");
    let back: T = serde_json::from_str(golden)
        .map_err(|err| format!("golden wire form {golden} rejected: {err}"))?;
    assert_eq!(&back, value, "round-trip lost information");
    Ok(())
}

/// The §5.1 request example, byte for byte.
#[test]
fn doc_05_request_example() -> Result<(), Box<dyn Error>> {
    round_trip(
        &WireRequest {
            request_id: RequestId(42),
            command: Command::PlaybackSeek {
                target: SeekTarget::Relative { delta_ms: -10_000 },
            },
        },
        r#"{"requestId":42,"cmd":"playback.seek","args":{"target":{"kind":"relative","deltaMs":-10000}}}"#,
    )?;
    Ok(())
}

/// The §5.1 response example, byte for byte — `Accepted` for a
/// gen-bumping command carries the NEW generation (§4.2).
#[test]
fn doc_05_response_example() -> Result<(), Box<dyn Error>> {
    round_trip(
        &WireResponse {
            request_id: RequestId(42),
            session: Some(SessionId(7)),
            generation: Generation(3),
            result: Ok(CommandOutcome::Accepted {
                session: Some(SessionId(7)),
                generation: Generation(4),
            }),
        },
        r#"{"requestId":42,"session":7,"gen":3,"result":{"accepted":{"session":7,"gen":4}}}"#,
    )?;
    Ok(())
}

/// The §5.1 event example, byte for byte (via the Tauri event channel —
/// same envelope on every host).
#[test]
fn doc_05_event_example() -> Result<(), Box<dyn Error>> {
    round_trip(
        &WireEvent {
            seq: 1031,
            session: Some(SessionId(7)),
            generation: Some(Generation(4)),
            origin: Some(RequestId(42)),
            kind: EventKind::TimingSeekCompleted {
                position: Timestamp(63_000_000),
                seek_kind: commands::catalog::types::SeekKind::Command,
            },
        },
        r#"{"seq":1031,"session":7,"gen":4,"origin":42,"kind":"timing.seek-completed","position":63000000,"seekKind":"command"}"#,
    )?;
    Ok(())
}

/// A failed command answers `{"error": …}` — NOT std's `{"ok":…}` shape
/// (the `result_wire` adapter; §1.6/§5.1).
#[test]
fn failure_response_is_the_error_envelope() -> Result<(), Box<dyn Error>> {
    round_trip(
        &WireResponse {
            request_id: RequestId(7),
            session: None,
            generation: Generation(2),
            result: Err(CommandError::new(
                Stage::Demux,
                ErrorCode::Inaccessible,
                "no such file",
            )),
        },
        r#"{"requestId":7,"gen":2,"result":{"error":{"stage":"demux","code":"inaccessible","message":"no such file","details":{}}}}"#,
    )?;
    Ok(())
}

/// Class-S completion: the bare `{"completed": …}` object carries the
/// result variant — internally tagged `kind`, kebab-case tag, camelCase
/// fields (`restartRequired`).
#[test]
fn completed_response_carries_the_result_variant() -> Result<(), Box<dyn Error>> {
    round_trip(
        &WireResponse {
            request_id: RequestId(9),
            session: Some(SessionId(1)),
            generation: Generation(0),
            result: Ok(CommandOutcome::Completed(CommandResult::SettingApplied {
                applied: commands::catalog::types::SettingValue::Bool(true),
                restart_required: true,
            })),
        },
        r#"{"requestId":9,"session":1,"gen":0,"result":{"completed":{"kind":"setting-applied","applied":true,"restartRequired":true}}}"#,
    )?;
    Ok(())
}

/// Sessionless events omit the optional envelope fields; payload fields
/// are camelCase (`deviceId`).
#[test]
fn sessionless_event_omits_optional_fields() -> Result<(), Box<dyn Error>> {
    round_trip(
        &Event {
            seq: 5,
            session: None,
            generation: None,
            origin: None,
            kind: EventKind::AudioDeviceChanged {
                device_id: "webaudio".to_owned(),
            },
        },
        r#"{"seq":5,"kind":"audio.device-changed","deviceId":"webaudio"}"#,
    )?;
    Ok(())
}

/// Inline command args are camelCase too (`toIndex`, `delayMs`), and
/// unit commands carry no `args` key at all.
#[test]
fn inline_args_are_camel_case_and_units_are_bare() -> Result<(), Box<dyn Error>> {
    round_trip(
        &WireRequest {
            request_id: RequestId(3),
            command: Command::PlaylistMove {
                id: commands::catalog::types::PlaylistItemId(4),
                to_index: 1,
            },
        },
        r#"{"requestId":3,"cmd":"playlist.move","args":{"id":4,"toIndex":1}}"#,
    )?;
    round_trip(
        &WireRequest {
            request_id: RequestId(4),
            command: Command::AudioSetDelay { delay_ms: 250 },
        },
        r#"{"requestId":4,"cmd":"audio.set-delay","args":{"delayMs":250}}"#,
    )?;
    round_trip(
        &WireRequest {
            request_id: RequestId(5),
            command: Command::PlaybackPause,
        },
        r#"{"requestId":5,"cmd":"playback.pause"}"#,
    )?;
    Ok(())
}
