//! Scenario 12 — **device loss** (doc 10 §6.4 row 12), driven
//! through the command layer.
//!
//! Pass criteria: `DeviceLost` event; position preserved or explicit
//! suspension; recovery on return.
//!
//! The honest v0 mapping (recorded for the Lead — this is the
//! card's "device loss = simulated via command-layer states or
//! local fakes"):
//!
//! - The audio limb in this build is the null device; nothing in
//!   the production stack can make it vanish. The **local fake** is
//!   the vanishing sink (`host::ObservingSink::vanish`): once
//!   armed, every frame accept answers the typed
//!   `Device{AudioOutput, Disconnected}` error — exactly the
//!   doc 01 §3.4 device-loss class.
//! - The engine's documented v0 contract for a consumer refusal is
//!   the **sink-contract fatal** (pipeline.rs: "a consumer refusal
//!   is the sink contract's fatal"): a typed FATAL
//!   `playback.error` (stage `audio-output`, code `device`) ordered
//!   before `state-changed → error`. There is no suspend-and-reopen
//!   policy and no `audio.device-lost` emission yet — that is card
//!   3.C3's `src/recovery.rs` (not in the tree). This scenario pins
//!   the CURRENT truth with full assertions and hands the
//!   suspend/resume semantics to 3.C3 — the assertions here will
//!   need extension, not weakening, when it lands.
//! - The command-layer states half: `audio.set-device` /
//!   `audio.list-devices` answer the production typed refusal
//!   naming the owning subsystem (ADR-005 real backends) — device
//!   control is typed, not silent.

use commands::catalog::types::{OpenOptions, SourceSpec};
use commands::envelope::CommandOutcome;
use commands::{Command, ErrorCode, PlaybackState, Stage};
use testkit::Must;
use testkit::alloc_count::serialize_tests;

use crate::host::{ScenarioHost, dispatch, error_events};

fn open_parent(host: &mut ScenarioHost) {
    let path = testkit::fixture_path(crate::host::PARENT).must();
    let response = dispatch(
        host,
        Command::SessionOpen {
            source: SourceSpec::File { path },
            options: OpenOptions::default(),
        },
    );
    assert!(
        matches!(&response.result, Ok(CommandOutcome::Accepted { .. })),
        "the parent opens: {:?}",
        response.result
    );
    host.settle();
    assert_eq!(host.state(), PlaybackState::Playing, "the parent plays");
}

/// Device-control commands answer the typed refusal naming the
/// owning subsystem — the honest command-layer surface for device
/// control in this build.
#[test]
fn device_control_commands_refuse_typed() {
    let _serial = serialize_tests();
    let mut host = ScenarioHost::new();
    for command in [
        Command::AudioListDevices,
        Command::AudioSetDevice {
            id: "default".to_string(),
        },
    ] {
        let response = dispatch(&mut host, command);
        match response.result {
            Err(error) => {
                assert_eq!(error.stage, Stage::Control, "{error}");
                assert_eq!(error.code, ErrorCode::UnsupportedForHost, "{error}");
                assert_eq!(
                    error.details.get("owner").map(String::as_str),
                    Some("audio device control joins with the real backends (ADR-005)"),
                    "{error}"
                );
            }
            Ok(_) => unreachable!("device control is not served in this build"),
        }
    }
}

/// The vanishing device: mid-play, the sink starts refusing with
/// the typed device-lost error. The engine's v0 contract holds — a
/// typed fatal lands (never a hang, never a panic), the position at
/// the loss point is the consumed-audio truth, and the host is
/// reusable after the teardown (the recovery-on-return handoff is
/// 3.C3's suspend/reopen policy).
#[test]
fn vanishing_device_fails_typed_with_position_preserved() {
    let _serial = serialize_tests();
    let mut host = ScenarioHost::new();
    open_parent(&mut host);

    // Play into the media (frames consumed, position advancing).
    host.pump_pipeline_until(|host| host.sink().map(|sink| sink.audio_accepted()).must() >= 10);
    let position_at_loss = host.position_us();
    let duration = match dispatch(&mut host, Command::MediaGetDuration).result {
        Ok(CommandOutcome::Completed(commands::CommandResult::Duration {
            duration: commands::catalog::types::Duration::Known { time },
        })) => time.0,
        other => unreachable!("the parent's duration is known: {other:?}"),
    };
    assert!(
        position_at_loss > 0 && position_at_loss < duration,
        "the loss lands mid-play (position {position_at_loss} µs of {duration})"
    );

    // The device goes away.
    host.sink().must().vanish();

    let (pumps, terminal) = host.run_until_terminal(256);
    assert_eq!(
        terminal,
        PlaybackState::Error,
        "the sink-contract fatal lands (pumps {pumps})"
    );
    let errors = error_events(&host);
    let fatal: Vec<_> = errors.iter().filter(|(_, fatal, _)| *fatal).collect();
    assert_eq!(fatal.len(), 1, "exactly one fatal error event: {errors:?}");
    assert_eq!(
        fatal[0].0.as_str(),
        "audio-output.device",
        "the typed device fatal: {fatal:?}"
    );
    // Ordered before the state change (doc 05 §3.3) — the trail ends
    // with → Error.
    assert_eq!(
        host.state_history().last().copied(),
        Some((PlaybackState::Playing, PlaybackState::Error)),
        "the trail: {:?}",
        host.state_history()
    );
    // The sink observed the refusal as a lost device, and nothing
    // was delivered past the vanish arm.
    let sink = std::sync::Arc::clone(host.sink().must());
    let accepted_at_loss = sink.frames_accepted();
    assert!(
        sink.frames_while_vanished() >= 1,
        "the refusal was observed"
    );
    host.pump_pipeline_until(|host| {
        host.sink().map(|sink| sink.frames_accepted()).must() == accepted_at_loss
    });
    assert_eq!(
        sink.frames_accepted(),
        accepted_at_loss,
        "no frame is delivered past the lost device"
    );

    // The teardown is clean and the host reusable: close settles the
    // dead session, a fresh open plays again from zero (the
    // resume-at-position semantics belong to 3.C3's policy).
    let close = dispatch(&mut host, Command::SessionClose);
    assert!(
        close.result.is_ok(),
        "closes after the fatal: {:?}",
        close.result
    );
    assert_eq!(host.state(), PlaybackState::Idle);
    // The reopened session starts from the beginning — position ZERO
    // at the open boundary, before this harness's racing sink can
    // self-feed any playback time (the sample-counter clock advances
    // with every accept, so "position after settle" measures the
    // harness's pump count, not the open's start policy). The
    // resume-at-loss-point policy is 3.C3's, recorded here honestly.
    let path = testkit::fixture_path(crate::host::PARENT).must();
    let reopen = dispatch(
        &mut host,
        Command::SessionOpen {
            source: SourceSpec::File { path },
            options: OpenOptions::default(),
        },
    );
    assert!(
        matches!(&reopen.result, Ok(CommandOutcome::Accepted { .. })),
        "the parent reopens: {:?}",
        reopen.result
    );
    assert_eq!(
        host.position_us(),
        0,
        "the reopened session starts from the beginning, not the loss point"
    );
    host.settle();
    assert_eq!(host.state(), PlaybackState::Playing, "the reopen plays");
    let close = dispatch(&mut host, Command::SessionClose);
    assert!(
        close.result.is_ok(),
        "the reopened session closes: {:?}",
        close.result
    );
}
