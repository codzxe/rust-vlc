//! FX5-A02 — the audio-controls catalog verbs, pinned (F1.1c).
//!
//! FX4-A02's report disclosed the drift this file is the regression pin
//! for: the engine seam (FX3-A03's `AudioControls`) carries
//! `set_balance`/`set_eq`/`balance`/`eq_bands`, but the command catalog
//! had NO `audio.set-balance` and NO `audio.set-eq` — the UI dispatched
//! the eq id into the host's honest `unsupported-for-host` refusal and
//! balance had no verb at all. This file pins the additive quartet
//! exactly the way the set-volume family is pinned across the catalog:
//! id round-trips and family order, the spec-table rows, and
//! byte-exact wire forms (requests AND the shared result variants,
//! `null`-for-absent included).

use std::error::Error;

use commands::catalog::spec::catalog_data;
use commands::catalog::spec::command_spec;
use commands::catalog::spec::CommandTarget;
use commands::catalog::spec::CompletionClass;
use commands::catalog::spec::PayloadDesc;
use commands::catalog::types::AudioEqBands;
use commands::catalog::Command;
use commands::catalog::CommandResult;
use commands::envelope::RequestId;
use commands::CommandId;
use commands::WireRequest;

/// Serialize, assert the exact JSON, deserialize back, assert equality
/// (wire_forms.rs's golden-string round-trip, reused verbatim).
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

/// The quartet's ids round-trip through their stable wire strings, sit
/// in the §2.6 family order (after `audio.set-delay`, mirroring how the
/// issue #32 bookmark trio was appended to its family), and unknown
/// near-miss ids stay rejected — never guessed.
#[test]
fn fx5_a02_audio_control_ids_round_trip_in_the_family_order() {
    for (id, variant) in [
        ("audio.set-balance", CommandId::AudioSetBalance),
        ("audio.get-balance", CommandId::AudioGetBalance),
        ("audio.set-eq", CommandId::AudioSetEq),
        ("audio.get-eq", CommandId::AudioGetEq),
    ] {
        assert_eq!(variant.as_str(), id);
        assert_eq!(variant.to_string(), id);
        assert_eq!(id.parse::<CommandId>().ok(), Some(variant), "{id} parse");
    }
    // The additive rows sit at the tail of the audio family in
    // `CommandId::ALL` — the exact position the spec table must mirror.
    let ids: Vec<&str> = CommandId::ALL_IDS.to_vec();
    let audio: Vec<&str> = ids
        .iter()
        .copied()
        .filter(|id| id.starts_with("audio."))
        .collect();
    assert_eq!(
        audio,
        [
            "audio.set-volume",
            "audio.get-volume",
            "audio.set-mute",
            "audio.list-devices",
            "audio.set-device",
            "audio.set-delay",
            "audio.set-balance",
            "audio.get-balance",
            "audio.set-eq",
            "audio.get-eq",
        ],
        "the §2.6 family order with the FX5-A02 quartet appended"
    );
    // Near-miss ids are still rejected (V1: closed shapes, no guessing).
    for bad in ["audio.set-eqs", "audio.getbalance", "audio.set-EQ"] {
        assert!(bad.parse::<CommandId>().is_err(), "{bad} parsed");
    }
}

/// The quartet's spec rows mirror the set-volume family's shape exactly:
/// class S (atomic between quanta, the response is the completion
/// evidence), engine-targeted, not gen-bumping, and the set/get pairs
/// SHARE one result descriptor the way `set-volume`/`get-volume` share
/// `AudioVolume`.
#[test]
fn fx5_a02_spec_rows_mirror_the_set_volume_family() {
    let family = [
        CommandId::AudioSetBalance,
        CommandId::AudioGetBalance,
        CommandId::AudioSetEq,
        CommandId::AudioGetEq,
    ];
    for id in family {
        let spec = command_spec(id).unwrap_or_else(|| unreachable!("no row for {id}"));
        assert_eq!(spec.class, CompletionClass::S, "{id} must be class S");
        assert_eq!(
            spec.target,
            CommandTarget::Engine,
            "{id} targets the engine"
        );
        assert!(!spec.gen_bumping, "{id} must not bump the generation");
    }
    // The set/get pairs share their result descriptor (the AudioVolume
    // precedent: one result variant serves both family halves).
    for (set, get) in [
        (CommandId::AudioSetBalance, CommandId::AudioGetBalance),
        (CommandId::AudioSetEq, CommandId::AudioGetEq),
    ] {
        let set_spec = command_spec(set).unwrap_or_else(|| unreachable!("no row for {set}"));
        let get_spec = command_spec(get).unwrap_or_else(|| unreachable!("no row for {get}"));
        assert_eq!(set_spec.result.as_str(), get_spec.result.as_str());
    }
    // The arg descriptors carry the engine seam's own parameter truth.
    let balance = command_spec(CommandId::AudioSetBalance)
        .unwrap_or_else(|| unreachable!("no set-balance row"));
    assert_eq!(balance.args, PayloadDesc::Type("position: f32"));
    let get_balance = command_spec(CommandId::AudioGetBalance)
        .unwrap_or_else(|| unreachable!("no get-balance row"));
    assert_eq!(get_balance.args, PayloadDesc::None);
    assert_eq!(balance.result, PayloadDesc::Type("AudioBalance"));
    let eq = command_spec(CommandId::AudioSetEq).unwrap_or_else(|| unreachable!("no set-eq row"));
    assert_eq!(
        eq.args,
        PayloadDesc::Type("low_db: f32, mid_db: f32, high_db: f32, preamp_db: f32")
    );
    let get_eq =
        command_spec(CommandId::AudioGetEq).unwrap_or_else(|| unreachable!("no get-eq row"));
    assert_eq!(get_eq.args, PayloadDesc::None);
    assert_eq!(eq.result, PayloadDesc::Type("AudioEq"));
}

/// The balance verbs' byte-exact wire forms: the request carries the
/// pan position camelCase-free (a bare float), the read twin takes no
/// args, and the shared `audio-balance` result answers the stage truth
/// — a position in force, or `null` while the stage is absent (the
/// identity split; absence ≠ the pan law's `0.0` center).
#[test]
fn fx5_a02_balance_verbs_round_trip_the_wire() -> Result<(), Box<dyn Error>> {
    round_trip(
        &WireRequest {
            request_id: RequestId(42),
            command: Command::AudioSetBalance { position: -0.5 },
        },
        r#"{"requestId":42,"cmd":"audio.set-balance","args":{"position":-0.5}}"#,
    )?;
    round_trip(
        &WireRequest {
            request_id: RequestId(43),
            command: Command::AudioGetBalance,
        },
        r#"{"requestId":43,"cmd":"audio.get-balance"}"#,
    )?;
    round_trip(
        &CommandResult::AudioBalance {
            position: Some(0.25),
        },
        r#"{"kind":"audio-balance","position":0.25}"#,
    )?;
    // `null` = the stage is absent — the honest identity, never a
    // sentinel position.
    round_trip(
        &CommandResult::AudioBalance { position: None },
        r#"{"kind":"audio-balance","position":null}"#,
    )
}

/// The eq verbs' byte-exact wire forms — the payload shape FX4-A08's UI
/// already dispatches (`{ lowDb, midDb, highDb, preampDb }`, camelCase),
/// now the catalog's own: the UI's forward-declaration cast can retire.
/// The shared `audio-eq` result carries the applied configuration, or
/// `null` while the stage is absent (an armed flat EQ is `(0, 0, 0,
/// 0)` — not absence).
#[test]
fn fx5_a02_eq_verbs_round_trip_the_wire() -> Result<(), Box<dyn Error>> {
    round_trip(
        &WireRequest {
            request_id: RequestId(44),
            command: Command::AudioSetEq {
                low_db: 0.1,
                mid_db: 0.0,
                high_db: -3.5,
                preamp_db: 0.0,
            },
        },
        r#"{"requestId":44,"cmd":"audio.set-eq","args":{"lowDb":0.1,"midDb":0.0,"highDb":-3.5,"preampDb":0.0}}"#,
    )?;
    round_trip(
        &WireRequest {
            request_id: RequestId(45),
            command: Command::AudioGetEq,
        },
        r#"{"requestId":45,"cmd":"audio.get-eq"}"#,
    )?;
    round_trip(
        &CommandResult::AudioEq {
            eq: Some(AudioEqBands {
                low_db: 0.1,
                mid_db: 0.0,
                high_db: -3.5,
                preamp_db: 0.0,
            }),
        },
        r#"{"kind":"audio-eq","eq":{"lowDb":0.1,"midDb":0.0,"highDb":-3.5,"preampDb":0.0}}"#,
    )?;
    round_trip(
        &CommandResult::AudioEq { eq: None },
        r#"{"kind":"audio-eq","eq":null}"#,
    )
}

/// The quartet is in the table exactly once each, in id-enum order, and
/// the family counts carry it: 69 command rows, 10 audio rows.
#[test]
fn fx5_a02_catalog_counts_carry_the_quartet() {
    let data = catalog_data();
    assert_eq!(data.len(), CommandId::ALL.len(), "table covers every id");
    let audio: Vec<&str> = data
        .iter()
        .map(|row| row.id.as_str())
        .filter(|id| id.starts_with("audio."))
        .collect();
    assert_eq!(audio.len(), 10, "the §2.6 family with the quartet");
    for (row, id) in data.iter().zip(CommandId::ALL) {
        assert_eq!(row.id, *id, "table rows follow the §2.1 id order");
    }
}
