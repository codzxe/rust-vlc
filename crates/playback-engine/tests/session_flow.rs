//! The engine-level session suite (card 2.C2): the state machine
//! driven end-to-end through [`Engine::poll`] with fake stage objects
//! (scripted demuxer, scripted decoder, collecting sink, virtual
//! clock) — the doc 03 §4.6(f) model exercised through the real
//! executor, not a mock of it.
//!
//! What is proven here (the table-level proofs live in `table.rs`):
//!
//! - the **lifetimes**: happy path to `Ended`, pause/resume, the
//!   rebuffer cycle, short media, the `stop` exit, the failure exits
//!   (open failure, armed decode failure, nothing-decodable), and the
//!   reopen cycle;
//! - **output ordering** (doc 05 §4.3): receipts before the events
//!   they cause, `playback.error` before `state-changed → error`,
//!   `playback.end-of-media` before `state-changed → ended`,
//!   `tracks.list-ready` before the first `state-changed → playing`,
//!   and engine-global strictly-increasing `seq`;
//! - **session epochs** (doc 03 §5 / doc 05 §4.2): generation bump at
//!   stop accept time with the NEW gen in the receipt, stale-handle
//!   commands rejected typed, superseded pipeline events suppressed
//!   at drain, cross-epoch reopen with fresh epochs;
//! - **pipeline wiring facts** (demux → queues → decoders → sinks):
//!   priming by the queue watermark, bounded per-tick decode, the
//!   end-of-stream flush-to-drain, backpressure parking that never
//!   drops fresh data, discontinuity/damage observability, and the
//!   doc 03 §5 point-5 checkpoint never dropping a live-generation
//!   frame.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink};
use commands::envelope::{Generation, RequestId};
use commands::{EventKind, SessionId};
use media_types::error::ErrorKind;
use media_types::packet::SharedBytes;
use media_types::{
    Duration, EngineError, Frame, GenerationId, MediaKind, Nanoseconds, Packet, PipelineStage,
    SourceUri, Stamp, Track, TrackId,
};
use playback_engine::state::{RejectionReason, SessionState};
use playback_engine::{
    CommandReceipt, DecoderProvider, Engine, EngineEvent, EngineEventKind, EngineOutput,
    FrameConsumer, ReceiptResult, SessionCommandKind, SessionConfig, SessionError, SessionHandle,
};
use plugin_api::DemuxEvent;
use testkit::Must;

// The shared fake harness (card 3.C1's hoist — the fakes, builders,
// and poll drivers this suite authored in 2.C2, now the common
// substrate for the model-walk suite too).
mod common;

use common::{
    Collector, FakeDecoder, FakeDemuxer, FakeProvider, Fixture, at_micros, config_with, frames_of,
    frozen_throttle, inaccessible, open_audio, open_stalled_audio, open_with, packet, poll_until,
    receipt_for, shallow_clone, take_events, tiny_budget, tiny_queues, track, virtual_clock,
};

// ---------------------------------------------------------------------------
// Lifetimes
// ---------------------------------------------------------------------------

#[test]
fn the_happy_path_reaches_ended_with_the_documented_event_chain() {
    let mut fixture = open_audio(40);
    // poll 1: the open step (probe + wiring) moves us to Buffering.
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Buffering);
    // polls: priming (12 resident packets) moves us to Playing.
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    // polls: the script exhausts, queues drain, decoders flush.
    poll_until(&mut fixture, |s| s == SessionState::Ended, 40, "drain");

    let outputs = take_events(&mut fixture);
    let wire: Vec<EventKind> = outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Event(event) => event.as_wire_event().map(|wire| wire.kind),
            EngineOutput::Receipt(_) => None,
        })
        .collect();
    let state_changes: Vec<(commands::PlaybackState, commands::PlaybackState)> = wire
        .iter()
        .filter_map(|kind| match kind {
            EventKind::SessionStateChanged { from, to } => Some((*from, *to)),
            _ => None,
        })
        .collect();
    assert_eq!(
        state_changes,
        vec![
            (
                commands::PlaybackState::Idle,
                commands::PlaybackState::Opening
            ),
            (
                commands::PlaybackState::Opening,
                commands::PlaybackState::Buffering
            ),
            (
                commands::PlaybackState::Buffering,
                commands::PlaybackState::Playing
            ),
            (
                commands::PlaybackState::Playing,
                commands::PlaybackState::Ended
            ),
        ]
    );
    // end-of-media precedes state-changed -> ended (doc 05 §3.2).
    let end_index = wire
        .iter()
        .position(|kind| matches!(kind, EventKind::PlaybackEndOfMedia))
        .must();
    let ended_index = wire
        .iter()
        .position(|kind| {
            matches!(
                kind,
                EventKind::SessionStateChanged {
                    to: commands::PlaybackState::Ended,
                    ..
                }
            )
        })
        .must();
    assert!(end_index < ended_index);
    // tracks.list-ready precedes the first state-changed -> playing
    // (doc 05 §4.3 terminal invariant 4).
    let tracks_index = wire
        .iter()
        .position(|kind| matches!(kind, EventKind::TracksListReady { .. }))
        .must();
    let playing_index = wire
        .iter()
        .position(|kind| {
            matches!(
                kind,
                EventKind::SessionStateChanged {
                    to: commands::PlaybackState::Playing,
                    ..
                }
            )
        })
        .must();
    assert!(tracks_index < playing_index);
    // Buffering episodes bracket with 0.0 and 1.0 (doc 05 §3.2).
    let mut progress: Vec<f32> = wire
        .iter()
        .filter_map(|kind| match kind {
            EventKind::SessionBuffering { progress } => Some(*progress),
            _ => None,
        })
        .collect();
    assert_eq!(progress.len(), 2, "enter + exit only in v0");
    assert_eq!(progress.remove(0), 0.0);
    assert_eq!(progress.remove(0), 1.0);
    // The media's frames all reached the sink: 40 decoded, 0 dropped.
    assert_eq!(frames_of(&fixture.collector), 40);
    let stats = fixture.engine.stats();
    assert_eq!(stats.frames_delivered, 40);
    assert_eq!(stats.stale_frames_dropped, 0, "no live frame is ever stale");
    // The clock was resumed for playing and stopped at the end.
    assert!(fixture.clock.is_paused(), "ended freezes the clock");
}

#[test]
fn pause_and_resume_freeze_and_thaw_the_clock() {
    let mut fixture = open_audio(200);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    let delivered_at_pause = frames_of(&fixture.collector);
    assert!(delivered_at_pause > 0, "buffering ticks already decode");

    fixture.handle.pause(RequestId(1)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Paused);
    assert!(fixture.clock.is_paused());

    // While paused the sinks are frozen: polling does not present.
    let mut quiet = true;
    for _ in 0..4 {
        fixture.engine.poll();
        quiet = quiet && frames_of(&fixture.collector) == delivered_at_pause;
    }
    assert!(quiet, "paused sessions do not present frames");

    fixture.handle.play(RequestId(2)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Playing);
    assert!(!fixture.clock.is_paused());
    poll_until(&mut fixture, |s| s != SessionState::Playing, 400, "resume");
}

#[test]
fn an_underrun_rebuffers_and_recovery_primes_again() {
    // Preload 20 packets, then stall the producer: the queue runs low
    // while the demuxer can (later) refill — the rebuffer shape.
    let mut fixture = open_stalled_audio(20);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    // The stall bites: decode drains the queue below the low
    // watermark while eof is still false -> Underrun -> Buffering.
    poll_until(
        &mut fixture,
        |s| s == SessionState::Buffering,
        200,
        "underrun",
    );
    // Release the stall: the script refills, priming resumes.
    let mut script = fixture
        .script
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for index in 20..120 {
        script.push_back(DemuxEvent::Packet(packet(TrackId::new(1), index)));
    }
    script.push_back(DemuxEvent::EndOfStream);
    drop(script);
    *fixture
        .stall
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = false;
    poll_until(&mut fixture, |s| s == SessionState::Playing, 200, "reprime");
    poll_until(&mut fixture, |s| s == SessionState::Ended, 400, "drain");
    // Two buffering episodes: initial + rebuffer.
    let outputs = take_events(&mut fixture);
    let episodes = outputs
        .iter()
        .filter(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::Buffering { progress: 0.0 },
                    ..
                })
            )
        })
        .count();
    assert_eq!(episodes, 2, "initial + rebuffer episodes");
}

/// Card 3.B3, design C2: the starvation metrics ride the engine's
/// stats surface — one applied episode per underrun transition, the
/// raw pre-debounce low-tick count, the post-debounce signal count,
/// the ingestion-stall (held-packet) ticks, and the which-cap
/// full-rejection tally. The tiny 16-packet fixture makes the packets
/// axis provably the binding one (64 B payloads and 1 ms durations
/// sit far below their caps).
#[test]
fn starvation_metrics_count_the_underrun_episode_in_engine_stats() {
    let mut fixture = open_stalled_audio(20);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    poll_until(
        &mut fixture,
        |s| s == SessionState::Buffering,
        200,
        "underrun",
    );
    // Release the stall and refill past the cap so the queue actually
    // refuses pushes (the full-rejection and held-packet metrics need
    // a producer faster than the consumer).
    let mut script = fixture
        .script
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for index in 20..120 {
        script.push_back(DemuxEvent::Packet(packet(TrackId::new(1), index)));
    }
    script.push_back(DemuxEvent::EndOfStream);
    drop(script);
    *fixture
        .stall
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = false;
    poll_until(&mut fixture, |s| s == SessionState::Playing, 200, "reprime");
    poll_until(&mut fixture, |s| s == SessionState::Ended, 400, "drain");

    let stats = fixture.engine.stats();
    // Exactly one applied episode and one reported signal: the
    // stall was sustained (no flapping), and no higher-priority
    // signal preempted the underrun.
    assert_eq!(stats.starve_episodes, 1, "one applied underrun episode");
    assert_eq!(stats.starve_signals, 1, "one post-debounce starve signal");
    assert_eq!(
        stats.starve_signals, stats.starve_episodes,
        "signal and episode agree when nothing preempts"
    );
    // The raw count saw the low line (pre-debounce; ≥ the applied
    // count is the invariant, equality is this scenario's shape).
    assert!(
        stats.starved_ticks >= stats.starve_episodes,
        "raw low ticks ({}) ≥ applied episodes ({})",
        stats.starved_ticks,
        stats.starve_episodes
    );
    // The fill raced the consumer: the 16-packet cap refused pushes
    // (packets axis — the binding-cap diagnostic) and the held packet
    // waited for queue space (the ingestion-stall metric).
    assert!(
        stats.full_rejections_packets > 0,
        "the packet cap refused pushes during the fills: {}",
        stats.full_rejections_packets
    );
    assert_eq!(
        (stats.full_rejections_bytes, stats.full_rejections_duration),
        (0, 0),
        "64 B payloads and 1 ms durations never bind their caps"
    );
    assert!(
        stats.pending_hold_ticks > 0,
        "the held packet waited across ticks: {}",
        stats.pending_hold_ticks
    );
}

#[test]
fn short_media_starts_with_what_exists_and_drains_to_ended() {
    // 3 packets then eof: below the 12-packet high watermark, so the
    // start is the short-media row (buffering -> playing on
    // EofReached), then a quick drain to ended.
    let mut fixture = open_audio(3);
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Buffering);
    poll_until(
        &mut fixture,
        |s| s == SessionState::Playing,
        5,
        "short start",
    );
    poll_until(
        &mut fixture,
        |s| s == SessionState::Ended,
        10,
        "quick drain",
    );
    assert_eq!(frames_of(&fixture.collector), 3);
    // And the buffering episode still closed with 1.0.
    let outputs = take_events(&mut fixture);
    let closed = outputs.iter().any(|output| {
        matches!(
            output,
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::Buffering { progress: 1.0 },
                ..
            })
        )
    });
    assert!(closed, "the short-media episode still closes with 1.0");
}

#[test]
fn stop_while_playing_tears_down_and_bumps_the_generation() {
    let mut fixture = open_audio(200);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    let frames_before = frames_of(&fixture.collector);

    fixture.handle.stop(RequestId(7)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Stopped);
    // The stop is gen-bumping: the Accepted receipt carries the NEW
    // generation (doc 05 §4.2) — generation 1 after the open's 0.
    let outputs = take_events(&mut fixture);
    let receipt = receipt_for(&outputs, RequestId(7));
    assert_eq!(
        receipt.result,
        ReceiptResult::Accepted {
            session: SessionId(1),
            generation: Generation(1),
        }
    );
    // No further frames are decoded after the stop.
    let mut quiet = true;
    for _ in 0..4 {
        fixture.engine.poll();
        quiet = quiet && frames_of(&fixture.collector) == frames_before;
    }
    assert!(quiet, "stopped sessions are inert");
    // Close from Stopped returns to Idle (class S completion).
    fixture.handle.close(RequestId(8)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Idle);
    let outputs = take_events(&mut fixture);
    let receipt = receipt_for(&outputs, RequestId(8));
    assert_eq!(
        receipt.result,
        ReceiptResult::Completed {
            session: SessionId(1)
        }
    );
}

#[test]
fn close_while_opening_returns_to_idle() {
    // doc 05 §3.3's `opening -> idle` edge: a close racing the open.
    let mut fixture = open_with(
        vec![track(1, MediaKind::Audio)],
        vec![DemuxEvent::EndOfStream],
        None,
        None,
    );
    // The session is Opening before the first poll runs the open step.
    assert_eq!(fixture.engine.state(), SessionState::Opening);
    fixture.handle.close(RequestId(1)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Idle);
}

/// Card 3.C1's stop-during-opening row (doc 03 §4.6 `Opening | Stop |
/// Stopped | cancel token; join threads`): the stop lands from the
/// un-wired side — the row's `TeardownPipeline` is the `Option` no-op
/// by construction (no pipeline was ever attached), no track list or
/// buffering episode ever existed, and the stopped session is inert
/// until close.
#[test]
fn stop_while_opening_tears_down_without_wiring() {
    let mut fixture = open_with(
        vec![track(1, MediaKind::Audio)],
        vec![DemuxEvent::EndOfStream],
        None,
        None,
    );
    assert_eq!(fixture.engine.state(), SessionState::Opening);
    fixture.handle.stop(RequestId(11)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Stopped);
    let outputs = take_events(&mut fixture);
    // The stop is the class-A gen-bump: generation 1 after the
    // open's 0.
    let receipt = receipt_for(&outputs, RequestId(11));
    assert_eq!(
        receipt.result,
        ReceiptResult::Accepted {
            session: SessionId(1),
            generation: Generation(1),
        }
    );
    // The trail is exactly the one edge — no tracks, no episode, no
    // other work existed to report.
    let mut trail = Vec::new();
    for output in &outputs {
        if let EngineOutput::Event(event) = output {
            trail.push(match event.kind {
                EngineEventKind::StateChanged { from, to } => {
                    format!("{from} → {to}")
                }
                EngineEventKind::TracksChanged { .. } => "tracks".to_string(),
                EngineEventKind::Buffering { progress } => {
                    format!("buffering {progress}")
                }
                _ => "other".to_string(),
            });
        }
    }
    assert_eq!(trail, vec!["opening → stopped".to_string()]);
    // Inert afterwards, and the settled close exits to Idle.
    for _ in 0..3 {
        fixture.engine.poll();
        assert_eq!(fixture.engine.state(), SessionState::Stopped);
    }
    fixture.handle.close(RequestId(12)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Idle);
}

/// Card 3.C1's stop-during-buffering row (doc 03 §4.6 `Buffering |
/// Stop | Stopped | flush, join`): a mid-episode stop closes the
/// episode with its `1.0` bookend BEFORE the `state-changed →
/// stopped` (doc 05 §3.2's "an episode must close with 1.0" — the
/// never-lose-the-last-fact discipline; the alternative, an episode
/// that simply never closes, breaks the bracket invariant the
/// contract suite pins).
#[test]
fn stop_while_buffering_closes_the_episode() {
    // Hold the session mid-episode: 4 packets preload (below the
    // high watermark of 12) with the producer stalled.
    let mut fixture = open_stalled_audio(4);
    poll_until(
        &mut fixture,
        |s| s == SessionState::Buffering,
        10,
        "the episode opens",
    );
    fixture.handle.stop(RequestId(21)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Stopped);
    let outputs = take_events(&mut fixture);
    // The receipt: the class-A stop, gen-bumped.
    let receipt = receipt_for(&outputs, RequestId(21));
    assert_eq!(
        receipt.result,
        ReceiptResult::Accepted {
            session: SessionId(1),
            generation: Generation(1),
        }
    );
    // The bookend precedes the settling state-change.
    let mut exit_at = None;
    let mut stopped_at = None;
    for (index, output) in outputs.iter().enumerate() {
        if let EngineOutput::Event(event) = output {
            match event.kind {
                EngineEventKind::Buffering { progress: 1.0 } => exit_at = Some(index),
                EngineEventKind::StateChanged {
                    to: SessionState::Stopped,
                    ..
                } => stopped_at = Some(index),
                _ => {}
            }
        }
    }
    let (exit_at, stopped_at) = (exit_at.must(), stopped_at.must());
    assert!(
        exit_at < stopped_at,
        "the episode's 1.0 (at {exit_at}) must precede → stopped (at {stopped_at})"
    );
    // Exactly one episode bracketed: the enter was in the pre-stop
    // outputs, the exit here — one 0.0 and one 1.0 overall.
    let exits = outputs
        .iter()
        .filter(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::Buffering { progress: 1.0 },
                    ..
                })
            )
        })
        .count();
    assert_eq!(exits, 1);
}

/// Card 3.C1's close-from-active macro: a client's `session.close`
/// in `Playing` is the executor's two-step composition — apply the
/// Stop row (settle: teardown, gen-bump), then the Close row
/// (destroy) — answering ONE Completed receipt for the whole
/// composition and the honest `playing → stopped → idle` trail on
/// the event stream, every edge wire-conformant (doc 05 §3.3 has no
/// direct `playing → idle` edge; the pure table keeps the typed
/// `SettleFirst` pair for its own totality).
#[test]
fn close_from_playing_is_the_two_step_composition() {
    let mut fixture = open_audio(200);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    fixture.handle.close(RequestId(31)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Idle);
    let outputs = take_events(&mut fixture);
    // ONE receipt, Completed — the whole composition is one command.
    let receipts: Vec<&CommandReceipt> = outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Receipt(receipt) if receipt.request == RequestId(31) => Some(receipt),
            _ => None,
        })
        .collect();
    assert_eq!(receipts.len(), 1, "exactly one receipt for the close");
    assert_eq!(
        receipts[0].result,
        ReceiptResult::Completed {
            session: SessionId(1)
        }
    );
    // The honest trail, in order, and nothing else state-shaped.
    let trail: Vec<String> = outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::StateChanged { from, to },
                ..
            }) => Some(format!("{from} → {to}")),
            _ => None,
        })
        .collect();
    assert_eq!(
        trail,
        vec![
            "playing → stopped".to_string(),
            "stopped → idle".to_string(),
        ],
        "the two-step composition's trail"
    );
    // The composition's events carry the close's final generation
    // (2: the stop's mint, then the close's) — the staleness rule
    // must not swallow the stop half of the trail (the model-walk
    // finding, pinned here at unit level).
    for output in &outputs {
        if let EngineOutput::Event(event) = output {
            assert_eq!(
                event.generation,
                Generation(2),
                "the composition's trail is stamped at its completion point"
            );
        }
    }
    // The engine holds no session afterwards, and a second close on
    // the stale handle is the typed refusal (never a panic).
    assert!(fixture.engine.session_id().is_none());
    assert!(matches!(
        fixture.handle.close(RequestId(32)),
        Err(SessionError::SessionClosed)
    ));
}

/// Card 3.C1's double-failure reopen cycle (the model-walk mix's
/// named shape, pinned deterministically): two full open → fail →
/// close cycles — each fatal strictly before its `→ error`, each
/// epoch fresh (1 then 2), seq engine-global strictly increasing
/// across the close/reopen boundary.
#[test]
fn error_cycle_twice() {
    let mut engine = Engine::new();
    let mut last_seq = 0u64;
    for cycle in 0..2u64 {
        let config = config_with(
            vec![track(1, MediaKind::Audio)],
            vec![],
            Some(inaccessible("open.header.missing")),
            None,
            virtual_clock(),
            Arc::new(Collector::default()),
        );
        let handle = engine.open(config).must();
        assert_eq!(handle.session_id(), SessionId(cycle + 1));
        // The open failure lands: fatal first, then → error.
        for _ in 0..10 {
            if engine.state() == SessionState::Error {
                break;
            }
            engine.poll();
        }
        assert_eq!(
            engine.state(),
            SessionState::Error,
            "the armed open failure lands"
        );
        let outputs = engine.take_outputs();
        let mut fatal_at = None;
        let mut error_at = None;
        for (index, output) in outputs.iter().enumerate() {
            if let EngineOutput::Event(event) = output {
                match event.kind {
                    EngineEventKind::Fatal { .. } => fatal_at = Some(index),
                    EngineEventKind::StateChanged {
                        to: SessionState::Error,
                        ..
                    } => error_at = Some(index),
                    _ => {}
                }
                assert!(
                    event.seq > last_seq,
                    "seq {} not past {} (cycle {cycle})",
                    event.seq,
                    last_seq
                );
                last_seq = event.seq;
                assert_eq!(event.session, SessionId(cycle + 1));
            }
        }
        assert!(fatal_at.must() < error_at.must(), "the fatal comes first");
        // The error session closes like any error (the settled row).
        handle.close(RequestId(40 + cycle)).must();
        engine.poll();
        assert_eq!(engine.state(), SessionState::Idle);
        let outputs = engine.take_outputs();
        let receipt = receipt_for(&outputs, RequestId(40 + cycle));
        assert_eq!(
            receipt.result,
            ReceiptResult::Completed {
                session: SessionId(cycle + 1)
            }
        );
        // The close's → idle continues the same strictly-increasing
        // seq stream (nothing resets across the cycle).
        for output in &outputs {
            if let EngineOutput::Event(event) = output {
                assert!(event.seq > last_seq);
                last_seq = event.seq;
            }
        }
    }
}

#[test]
fn an_open_failure_lands_in_error_with_the_fatal_first() {
    let mut fixture = open_with(
        vec![track(1, MediaKind::Audio)],
        vec![],
        Some(inaccessible("open.header.missing")),
        None,
    );
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Error);
    let outputs = take_events(&mut fixture);
    // playback.error precedes state-changed -> error (doc 05 §3.3).
    let fatal_index = outputs
        .iter()
        .position(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::Fatal { .. },
                    ..
                })
            )
        })
        .must();
    let error_index = outputs
        .iter()
        .position(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::StateChanged {
                        to: SessionState::Error,
                        ..
                    },
                    ..
                })
            )
        })
        .must();
    assert!(fatal_index < error_index);
    // The wire form is a fatal playback.error with the typed facts.
    match &outputs[fatal_index] {
        EngineOutput::Event(event) => {
            let wire = event.as_wire_event().must();
            match &wire.kind {
                EventKind::PlaybackError {
                    stage,
                    code,
                    details,
                    fatal,
                    ..
                } => {
                    assert_eq!(*stage, commands::Stage::Input);
                    assert_eq!(*code, commands::ErrorCode::Inaccessible);
                    assert!(matches!(
                        details.get("context").map(String::as_str),
                        Some("open.header.missing")
                    ));
                    assert!(*fatal);
                }
                other => unreachable!("fatal maps to PlaybackError, got {other:?}"),
            }
        }
        EngineOutput::Receipt(_) => unreachable!("the fatal is an event"),
    }
    // Error is inert until close.
    fixture.handle.play(RequestId(2)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Error);
    fixture.handle.close(RequestId(3)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Idle);
}

#[test]
fn an_armed_decode_failure_transitions_to_error_from_playing() {
    // The provider arms every decoder it hands out with a one-shot
    // failure; the first decode call turns fatal from Playing (the
    // selection promised, the decoding did not deliver). The armed
    // error is the NON-per-frame class (`Unsupported`): since 3.C2's
    // G3 fix, a per-frame kind on this AUDIO limb would be
    // skipped-and-reported instead of fatal (pinned in
    // recovery_media.rs) — the fatal transition is the class this
    // test owns.
    struct ArmedProvider;
    impl DecoderProvider for ArmedProvider {
        fn decoder_for(
            &self,
            _track: &Track,
            config: &DecoderConfig,
        ) -> Result<Box<dyn Decoder>, EngineError> {
            // The FATAL decode class (doc 03 §4.5(d)): per-frame
            // damage kinds (Corrupt / Decoder{Bitstream}) are isolated
            // by the concealment policy (W1-08 + 3.C2 G3) — the
            // session-fatal decode failure is `Unsupported`.
            Ok(Box::new(FakeDecoder {
                generation: config.generation,
                fail_decode: Some(EngineError::new(
                    PipelineStage::Decode,
                    ErrorKind::Unsupported {
                        format: Some(media_types::error::FormatInfo {
                            name: Some("armed".to_string()),
                            sniffed: Vec::new(),
                        }),
                    },
                    media_types::error::ErrorContext::new("decode.unsupported.armed"),
                )),
            }))
        }
    }
    let mut script: Vec<DemuxEvent> = (0..40)
        .map(|index| DemuxEvent::Packet(packet(TrackId::new(1), index)))
        .collect();
    script.push(DemuxEvent::EndOfStream);
    let mut config = SessionConfig::new(
        SourceUri::parse("file:///tmp/lumen-armed.media").must(),
        Box::new(FakeDemuxer {
            tracks: vec![track(1, MediaKind::Audio)],
            script: Arc::new(Mutex::new(VecDeque::from(script))),
            stall: Arc::new(Mutex::new(false)),
            fail_start: None,
        }),
        Arc::new(ArmedProvider),
        Arc::new(Collector::default()),
        virtual_clock(),
    );
    config.queues = tiny_queues();
    config.budget = tiny_budget();
    config.throttle = frozen_throttle();
    let mut engine = Engine::new();
    let handle = engine.open(config).must();
    // The failure fires during the first decode (Buffering tick): the
    // session lands in Error with the typed cause on the stream.
    let mut state = engine.state();
    for _ in 0..10 {
        if state == SessionState::Error {
            break;
        }
        engine.poll();
        state = engine.state();
    }
    assert_eq!(state, SessionState::Error, "the armed decode failure");
    let _ = handle;
    let outputs = engine.take_outputs();
    let has_fatal = outputs.iter().any(|output| {
        matches!(
            output,
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::Fatal { error },
                ..
            }) if error.context.code == "decode.unsupported.armed"
        )
    });
    assert!(has_fatal, "the typed decode failure is on the stream");
}

#[test]
fn nothing_decodable_is_a_typed_config_failure() {
    // A subtitle-only source: no audio/video limb is wireable, and the
    // engine refuses to fabricate playback (typed, not a crash).
    let mut fixture = open_with(
        vec![track(9, MediaKind::Subtitle)],
        vec![DemuxEvent::EndOfStream],
        None,
        None,
    );
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Error);
    let outputs = take_events(&mut fixture);
    let code = outputs
        .iter()
        .find_map(|output| match output {
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::Fatal { error },
                ..
            }) => Some(error.context.code.clone()),
            _ => None,
        })
        .must();
    assert_eq!(code, "session.no-decodable-track");
    // The wire error is Unsupported (the config_error mapping).
    let wire_error = outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Event(event) => event.as_wire_event(),
            _ => None,
        })
        .find_map(|wire| match wire.kind {
            EventKind::PlaybackError { code, .. } => Some(code),
            _ => None,
        });
    assert_eq!(wire_error, Some(commands::ErrorCode::Unsupported));
}

// ---------------------------------------------------------------------------
// Epochs and staleness (doc 03 §5 / doc 05 §4.2)
// ---------------------------------------------------------------------------

#[test]
fn a_stale_handle_is_rejected_typed_after_close() {
    let mut fixture = open_audio(40);
    poll_until(&mut fixture, |s| s == SessionState::Ended, 100, "drain");
    let stale = &fixture.handle;
    stale.close(RequestId(11)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Idle);
    // The handle outlived its epoch: submitting is a typed rejection,
    // never a silent drop.
    assert!(matches!(
        stale.play(RequestId(12)),
        Err(SessionError::SessionClosed)
    ));
    assert!(matches!(
        stale.submit(RequestId(13), SessionCommandKind::Pause),
        Err(SessionError::SessionClosed)
    ));
}

#[test]
fn a_second_open_from_playing_replaces_the_session() {
    // Card 3.A2 flipped this contract: `open` on a live
    // (non-Buffering) session is play-new-while-playing — the
    // implicit close runs through the Stop/Close table rows and a
    // fresh epoch is minted (doc 05 §2.1 "Replaces any current
    // session"). The Buffering refusal and the full nine-step
    // acceptance suite live in `replacement.rs`.
    let mut fixture = open_audio(40);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    let frames_at_switch = frames_of(&fixture.collector);
    let mut script: Vec<DemuxEvent> = (0..10)
        .map(|index| DemuxEvent::Packet(packet(TrackId::new(1), index)))
        .collect();
    script.push(DemuxEvent::EndOfStream);
    let config = config_with(
        vec![track(1, MediaKind::Audio)],
        script,
        None,
        None,
        virtual_clock(),
        Arc::new(Collector::default()),
    );
    let second = fixture.engine.open(config).must();
    // A fresh epoch, not a reuse: new session id, `Opening` at once.
    assert_eq!(second.session_id(), SessionId(2));
    assert_eq!(fixture.engine.state(), SessionState::Opening);
    // The old handle is dead — the typed command-side staleness
    // rejection, never a silent drop.
    assert!(matches!(
        fixture.handle.play(RequestId(30)),
        Err(SessionError::SessionClosed)
    ));
    // The second session plays and drains to `Ended` on the same
    // engine (the replacement left it fully functional).
    for _ in 0..100 {
        if fixture.engine.state() == SessionState::Ended {
            break;
        }
        fixture.engine.poll();
    }
    assert_eq!(fixture.engine.state(), SessionState::Ended);
    // No stale delivery from the destroyed epoch: its collector's
    // count froze at the switch (frames only ever flow through the
    // live pipeline, and the old one died inside `open`).
    assert_eq!(frames_of(&fixture.collector), frames_at_switch);
}

#[test]
fn reopening_mints_a_fresh_epoch_and_continues_the_seq() {
    let mut fixture = open_audio(10);
    poll_until(&mut fixture, |s| s == SessionState::Ended, 100, "drain");
    let first_seqs: Vec<u64> = take_events(&mut fixture)
        .into_iter()
        .filter_map(|output| match output {
            EngineOutput::Event(event) => Some(event.seq),
            EngineOutput::Receipt(_) => None,
        })
        .collect();
    assert!(!first_seqs.is_empty());
    for pair in first_seqs.windows(2) {
        assert!(
            pair[0] < pair[1],
            "engine-global seq is strictly increasing"
        );
    }
    fixture.handle.close(RequestId(20)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Idle);

    // Reopen ON THE SAME ENGINE: a fresh epoch (session id 2), seq
    // strictly continues engine-globally.
    let mut script: Vec<DemuxEvent> = (0..10)
        .map(|index| DemuxEvent::Packet(packet(TrackId::new(1), index)))
        .collect();
    script.push(DemuxEvent::EndOfStream);
    let config = config_with(
        vec![track(1, MediaKind::Audio)],
        script,
        None,
        None,
        virtual_clock(),
        Arc::new(Collector::default()),
    );
    let second_handle = fixture.engine.open(config).must();
    assert_eq!(
        second_handle.session_id(),
        SessionId(2),
        "the second session is a fresh epoch"
    );
    for _ in 0..100 {
        if fixture.engine.state() == SessionState::Ended {
            break;
        }
        fixture.engine.poll();
    }
    assert_eq!(fixture.engine.state(), SessionState::Ended);
    let second_seqs: Vec<u64> = take_events(&mut fixture)
        .into_iter()
        .filter_map(|output| match output {
            EngineOutput::Event(event) => Some(event.seq),
            EngineOutput::Receipt(_) => None,
        })
        .collect();
    assert!(second_seqs[0] > first_seqs[first_seqs.len() - 1]);
    for pair in second_seqs.windows(2) {
        assert!(pair[0] < pair[1]);
    }
}

#[test]
fn superseded_events_are_suppressed_but_receipts_survive() {
    // Play, pause, stop — all without draining: the not-yet-drained
    // pipeline events of generation 0 are suppressed at drain (doc 05
    // §4.2 engine-side rule); the receipts survive.
    let mut fixture = open_audio(200);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    for _ in 0..3 {
        fixture.engine.poll();
    }
    fixture.handle.pause(RequestId(30)).must();
    fixture.engine.poll();
    fixture.handle.stop(RequestId(31)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Stopped);

    let outputs = take_events(&mut fixture);
    let receipts: Vec<RequestId> = outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Receipt(receipt) => Some(receipt.request),
            _ => None,
        })
        .collect();
    assert!(receipts.contains(&RequestId(30)), "receipts always survive");
    assert!(receipts.contains(&RequestId(31)));
    // No generation-0 event survived the bump to generation 1.
    for output in &outputs {
        if let EngineOutput::Event(event) = output {
            assert!(
                event.generation >= Generation(1),
                "stale event leaked: seq {} gen {}",
                event.seq,
                event.generation.0
            );
        }
    }
    assert!(
        fixture.engine.stats().suppressed_stale_events > 0,
        "the suppression is observable"
    );
}

#[test]
fn receipts_precede_the_events_they_cause() {
    let mut fixture = open_audio(200);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    fixture.handle.pause(RequestId(41)).must();
    fixture.engine.poll();
    let outputs = take_events(&mut fixture);
    let receipt_index = outputs
        .iter()
        .position(|output| {
            matches!(
                output,
                EngineOutput::Receipt(CommandReceipt {
                    request: RequestId(41),
                    ..
                })
            )
        })
        .must();
    let paused_index = outputs
        .iter()
        .position(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::StateChanged {
                        to: SessionState::Paused,
                        ..
                    },
                    ..
                })
            )
        })
        .must();
    assert!(
        receipt_index < paused_index,
        "doc 05 §4.3 guarantee 2: the receipt precedes its events"
    );
    // The paused event carries the originating request as origin.
    match &outputs[paused_index] {
        EngineOutput::Event(event) => assert_eq!(event.origin, Some(RequestId(41))),
        EngineOutput::Receipt(_) => unreachable!("positioned an event"),
    }
}

#[test]
fn rejected_commands_answer_typed_receipts_and_change_nothing() {
    let mut fixture = open_audio(200);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    // `stop` in `Ended` is the settled inert rejection (the close
    // macro's own pair — close from active states — composes in the
    // executor; its scenario pin is
    // `close_from_playing_is_the_two_step_composition`).
    poll_until(&mut fixture, |s| s == SessionState::Ended, 400, "drain");
    fixture.handle.stop(RequestId(50)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Ended, "unchanged");
    let outputs = take_events(&mut fixture);
    let receipt = receipt_for(&outputs, RequestId(50));
    match receipt.result {
        ReceiptResult::Rejected { state, reason } => {
            assert_eq!(state, SessionState::Ended);
            assert_eq!(reason, RejectionReason::AlreadySettled);
        }
        other => unreachable!("stop in ended is rejected, got {other:?}"),
    }
    // `play` while playing is the C8 idempotent confirmation.
    let mut fixture = open_audio(200);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    fixture.handle.play(RequestId(51)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Playing);
    let outputs = take_events(&mut fixture);
    let confirmed = outputs.iter().any(|output| {
        matches!(
            output,
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::StateChanged {
                    from: SessionState::Playing,
                    to: SessionState::Playing,
                },
                ..
            })
        )
    });
    assert!(confirmed, "the self-transition confirms idempotently");
}

// ---------------------------------------------------------------------------
// Pipeline wiring facts
// ---------------------------------------------------------------------------

#[test]
fn demux_marks_are_counted_and_damage_is_reported() {
    let mut script: Vec<DemuxEvent> = vec![
        DemuxEvent::Packet(packet(TrackId::new(1), 0)),
        DemuxEvent::Discontinuity,
        DemuxEvent::Recovered {
            skipped: 2,
            reason: "test",
        },
    ];
    for index in 1..20 {
        script.push(DemuxEvent::Packet(packet(TrackId::new(1), index)));
    }
    script.push(DemuxEvent::EndOfStream);
    let mut fixture = open_with(vec![track(1, MediaKind::Audio)], script, None, None);
    poll_until(&mut fixture, |s| s == SessionState::Ended, 200, "drain");
    let stats = fixture.engine.stats();
    assert_eq!(
        stats.demux_discontinuities, 1,
        "the marked splice is counted"
    );
    // The damage report is engine-private (no wire row) but observable.
    let outputs = take_events(&mut fixture);
    let damaged: Vec<u64> = outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::DamagedData { count },
                ..
            }) => Some(*count),
            _ => None,
        })
        .collect();
    assert_eq!(damaged, vec![2], "bounded recovery is observable data");
}

#[test]
fn first_frame_is_reported_and_stays_engine_private() {
    let mut fixture = open_audio(40);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    let outputs = take_events(&mut fixture);
    let mut saw_first_frame = false;
    for output in &outputs {
        if let EngineOutput::Event(EngineEvent {
            kind: EngineEventKind::FirstFrame { position },
            ..
        }) = output
        {
            saw_first_frame = true;
            // The first decoded frame's pts is the first packet's pts.
            match position {
                Stamp::Present(timestamp) => {
                    assert_eq!(timestamp.ticks, 0, "the first frame's presentation time");
                }
                other => unreachable!("first frame reported with a stamp, got {other:?}"),
            }
        }
    }
    assert!(saw_first_frame, "the first frame is reported");
    // Engine-private: every event that reaches the wire is a wire kind
    // — FirstFrame never leaks (doc 05 §4.2 mapping).
    for output in &outputs {
        if let EngineOutput::Event(event) = output {
            let is_first_frame = matches!(event.kind, EngineEventKind::FirstFrame { .. });
            assert!(
                !is_first_frame || event.as_wire_event().is_none(),
                "FirstFrame must not leak onto the wire"
            );
        }
    }
}

#[test]
fn a_full_queue_holds_the_packet_and_never_drops_fresh_data() {
    // 200 packets with a 16-packet queue and a 64-event demux budget:
    // backpressure parks the overflow in `pending` — nothing is lost
    // (doc 03 §4.4 Q3: fresh data is never dropped).
    let mut fixture = open_audio(200);
    poll_until(&mut fixture, |s| s == SessionState::Ended, 2_000, "drain");
    assert_eq!(frames_of(&fixture.collector), 200);
    assert_eq!(fixture.engine.stats().frames_delivered, 200);
}

#[test]
fn taking_outputs_twice_yields_nothing_the_second_time() {
    let mut fixture = open_audio(40);
    fixture.engine.poll();
    let first = take_events(&mut fixture);
    assert!(!first.is_empty());
    assert!(take_events(&mut fixture).is_empty());
}

#[test]
fn the_outbox_drops_oldest_events_at_capacity_and_counts_them() {
    // A tiny outbox: events overflow (oldest dropped, counted); the
    // receipts are pinned and survive.
    let mut script: Vec<DemuxEvent> = (0..200)
        .map(|index| DemuxEvent::Packet(packet(TrackId::new(1), index)))
        .collect();
    script.push(DemuxEvent::EndOfStream);
    let mut config = config_with(
        vec![track(1, MediaKind::Audio)],
        script,
        None,
        None,
        virtual_clock(),
        Arc::new(Collector::default()),
    );
    config.queues = tiny_queues();
    config.budget = tiny_budget();
    let mut engine = playback_engine::Engine::with_outbox_capacity(4);
    let handle = engine.open(config).must();
    // Flood: many polls without draining, until the media settles.
    for _ in 0..400 {
        if engine.state() == SessionState::Ended {
            break;
        }
        engine.poll();
    }
    assert_eq!(engine.state(), SessionState::Ended, "the flood settles");
    handle.close(RequestId(60)).must();
    engine.poll();
    assert_eq!(engine.state(), SessionState::Idle);
    let outputs = engine.take_outputs();
    let has_close_receipt = outputs.iter().any(|output| {
        matches!(
            output,
            EngineOutput::Receipt(CommandReceipt {
                request: RequestId(60),
                ..
            })
        )
    });
    assert!(has_close_receipt, "receipts are never dropped");
    assert!(
        engine.stats().outbox_dropped_events > 0,
        "the overflow is counted: {:?}",
        engine.stats()
    );
}

#[test]
fn the_engine_debug_renders_the_observable_surface() {
    let fixture = open_audio(40);
    let text = format!("{:?}", fixture.engine);
    assert!(text.contains("Engine"), "{text}");
    assert!(text.contains("Opening"), "{text}");
}

// ---------------------------------------------------------------------------
// Per-frame damage isolation (W1-08, audit M9 / doc 03 §4.5(d))
// ---------------------------------------------------------------------------

/// The scripted video decode stage: one video frame per packet, with
/// a corruption schedule. Corrupt slots fail with the SAME typed
/// error the real MJPEG decoder produces for a header-smashed frame
/// (`Corrupt`/`InvalidStructure`, codecs' `mjpeg.marker.no_soi`) —
/// the engine's isolation policy runs against the real damage
/// vocabulary, not a stand-in kind.
struct VideoDecoder {
    generation: GenerationId,
    /// Packet indices whose decode fails as per-frame damage.
    corrupt: Vec<u64>,
    /// If set, every decode fails with this error verbatim (the
    /// non-damage boundary pins).
    fail_all: Option<EngineError>,
}

impl Decoder for VideoDecoder {
    fn configure(&mut self, config: &DecoderConfig) -> Result<(), EngineError> {
        self.generation = config.generation;
        Ok(())
    }

    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        if let Some(error) = self.fail_all.as_ref() {
            return Err(shallow_clone(error));
        }
        if packet.is_stale(self.generation) {
            return Ok(DecodeOutcome::NeedMoreInput);
        }
        let index = packet_index(&packet);
        if self.corrupt.contains(&index) {
            return Err(corrupt_frame_error(index));
        }
        out.accept(video_frame(index, packet.pts, self.generation))?;
        Ok(DecodeOutcome::Emitted { frames: 1 })
    }

    fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        Ok(DecodeOutcome::Drained)
    }
}

/// The decoder seam for the video-isolation scenarios: every limb
/// gets the scheduled video decoder.
struct VideoProvider {
    corrupt: Vec<u64>,
    fail_all: Option<EngineError>,
}

impl DecoderProvider for VideoProvider {
    fn decoder_for(
        &self,
        _track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        Ok(Box::new(VideoDecoder {
            generation: config.generation,
            corrupt: self.corrupt.clone(),
            fail_all: self.fail_all.as_ref().map(shallow_clone),
        }))
    }
}

/// A video packet whose payload carries its own index (the decode
/// schedule keys on it).
fn video_packet(track: TrackId, index: u64) -> Packet {
    Packet {
        track,
        pts: at_micros((index * 1_000) as i64),
        dts: Stamp::Missing,
        duration: Some(Duration::Known(Nanoseconds(1_000_000))),
        data: SharedBytes::from_vec(index.to_le_bytes().to_vec()),
        keyframe: true,
        discontinuity: false,
        generation: GenerationId::FIRST,
    }
}

/// The index a [`video_packet`] carries.
fn packet_index(packet: &Packet) -> u64 {
    packet
        .data
        .as_slice()
        .first_chunk::<8>()
        .map_or(u64::MAX, |chunk| u64::from_le_bytes(*chunk))
}

/// One distinguishable video frame: pixel value = index mod 256.
fn video_frame(index: u64, pts: Stamp, generation: GenerationId) -> Frame {
    Frame::Video(media_types::frame::VideoFrame {
        format: media_types::PixelFormat::I420,
        size: (8, 8),
        data: media_types::frame::FrameStorage::Cpu(SharedBytes::from_vec(vec![
            (index % 256) as u8;
            8 * 8 * 3 / 2
        ])),
        pts,
        color: media_types::frame::ColorDescription::BT709_LIMITED,
        interlaced: false,
        generation,
    })
}

/// The typed per-frame damage a header-smashed MJPEG frame produces
/// (the real decoder's `BadMarker` → `EngineError` mapping).
fn corrupt_frame_error(offset: u64) -> EngineError {
    EngineError::new(
        PipelineStage::Decode,
        ErrorKind::Corrupt {
            detail: media_types::error::CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        media_types::error::ErrorContext::new("mjpeg.marker.no_soi"),
    )
}

/// A video-only session over `count` indexed packets.
fn open_video(count: u64, provider: VideoProvider) -> Fixture {
    let mut script: Vec<DemuxEvent> = (0..count)
        .map(|index| DemuxEvent::Packet(video_packet(TrackId::new(1), index)))
        .collect();
    script.push(DemuxEvent::EndOfStream);
    let script = Arc::new(Mutex::new(VecDeque::from(script)));
    let stall = Arc::new(Mutex::new(false));
    let collector = Arc::new(Collector::default());
    let mut config = SessionConfig::new(
        SourceUri::parse("file:///tmp/lumen-fake-video.media").must(),
        Box::new(FakeDemuxer {
            tracks: vec![track(1, MediaKind::Video)],
            script: Arc::clone(&script),
            stall: Arc::clone(&stall),
            fail_start: None,
        }),
        Arc::new(provider),
        Arc::clone(&collector) as Arc<dyn FrameConsumer>,
        virtual_clock(),
    );
    config.queues = tiny_queues();
    config.budget = tiny_budget();
    config.throttle = frozen_throttle();
    let mut engine = Engine::new();
    let handle = engine.open(config).must();
    Fixture {
        engine,
        handle,
        collector,
        clock: virtual_clock(),
        script,
        stall,
    }
}

/// The collector's frames, copied out.
fn collected(collector: &Collector) -> Vec<(TrackId, Frame)> {
    collector
        .frames
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

#[test]
fn one_corrupt_video_frame_is_concealed_and_playback_runs_to_end() {
    // Audit M9's exact scenario: a valid stream with ONE corrupted
    // frame (packet 10 of 24). Pre-W1-08 the session died with a
    // fatal decode error and teardown; the per-frame isolation (doc
    // 03 §4.5(d) — the PCM sibling's `Recovered` policy shape) must
    // instead play to EOS with exactly one damage event, 23 decoded
    // frames presented and one concealment (the repeated frame).
    let mut fixture = open_video(
        24,
        VideoProvider {
            corrupt: vec![10],
            fail_all: None,
        },
    );
    poll_until(
        &mut fixture,
        |s| s == SessionState::Ended,
        200,
        "drain to EOS",
    );
    let stats = fixture.engine.stats();
    assert_eq!(
        stats.frames_delivered, 23,
        "23 of 24 packets decode a frame"
    );
    assert_eq!(
        stats.frames_concealed, 1,
        "the corrupt slot is concealed once"
    );

    // The collector saw 24 presentations: 23 decoded + 1 repeated.
    let frames = collected(&fixture.collector);
    assert_eq!(frames.len(), 24);
    // The concealment: slot 10 carries slot 9's pixels (a repeat,
    // never new content) re-stamped at slot 10's pts.
    match (&frames[9].1, &frames[10].1) {
        (Frame::Video(good), Frame::Video(concealed)) => {
            assert_eq!(
                concealed.data, good.data,
                "the concealment repeats the last good frame's pixels"
            );
            assert_eq!(concealed.size, good.size);
            match concealed.pts {
                Stamp::Present(timestamp) => {
                    assert_eq!(
                        timestamp.ticks, 10_000,
                        "re-presented at the damaged slot's pts"
                    );
                }
                other => unreachable!("concealed at a present stamp, got {other:?}"),
            }
        }
        other => unreachable!("video frames at slots 9/10: {other:?}"),
    }
    // Slot 11 decodes its own content again (the run healed).
    match &frames[11].1 {
        Frame::Video(next) => match next.pts {
            Stamp::Present(timestamp) => assert_eq!(timestamp.ticks, 11_000),
            other => unreachable!("{other:?}"),
        },
        other => unreachable!("{other:?}"),
    }
    // Exactly one damage fact, counting the one corrupt frame —
    // engine-private but observable (doc 03 §4.6 `DamagedData`).
    let outputs = take_events(&mut fixture);
    let damaged: Vec<u64> = outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::DamagedData { count },
                ..
            }) => Some(*count),
            _ => None,
        })
        .collect();
    assert_eq!(
        damaged,
        vec![1],
        "exactly one damage event for one corrupt frame"
    );
    // Its wire half: ONE coalesced non-fatal playback.error from the
    // decode stage carrying the count (card 3.B4 — the PCM recovery
    // shape, mirrored for video).
    let mut recovered: Vec<EventKind> = Vec::new();
    for output in &outputs {
        if let EngineOutput::Event(event) = output
            && let Some(wire) = event.as_wire_event()
            && matches!(wire.kind, EventKind::PlaybackError { fatal: false, .. })
        {
            recovered.push(wire.kind);
        }
    }
    assert_eq!(recovered.len(), 1, "one coalesced non-fatal playback.error");
    match &recovered[0] {
        EventKind::PlaybackError {
            stage,
            code,
            details,
            fatal,
            ..
        } => {
            assert!(!fatal);
            assert_eq!(*stage, commands::Stage::Decode);
            assert_eq!(*code, commands::ErrorCode::Corrupt);
            assert_eq!(
                details.get("count").map(String::as_str),
                Some("1"),
                "the concealed frame is one counted unit"
            );
        }
        _ => unreachable!("filtered to playback.error"),
    }
    // The FirstFrame fact is the first DECODED frame (slot 0) — a
    // concealment never mints it (nothing was decoded yet at slot 10;
    // the fact already existed anyway).
    assert!(
        outputs.iter().any(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::FirstFrame { .. },
                    ..
                })
            )
        }),
        "the first frame is still reported"
    );
}

#[test]
fn a_corrupt_first_video_frame_holds_and_playback_continues() {
    // The "no prior frame exists" rule: the first packet is corrupt,
    // so there is nothing to repeat — the slot holds (nothing is
    // presented, nothing fabricated), the damage is observable, and
    // the stream continues to EOS with the remaining frames.
    let mut fixture = open_video(
        12,
        VideoProvider {
            corrupt: vec![0],
            fail_all: None,
        },
    );
    poll_until(
        &mut fixture,
        |s| s == SessionState::Ended,
        200,
        "drain to EOS",
    );
    let stats = fixture.engine.stats();
    assert_eq!(
        stats.frames_delivered, 11,
        "the corrupt first frame decodes nothing"
    );
    assert_eq!(
        stats.frames_concealed, 0,
        "no repeat was possible — the slot held"
    );
    assert_eq!(
        collected(&fixture.collector).len(),
        11,
        "slot 0 presented nothing"
    );
    let outputs = take_events(&mut fixture);
    let damaged: Vec<u64> = outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::DamagedData { count },
                ..
            }) => Some(*count),
            _ => None,
        })
        .collect();
    assert_eq!(
        damaged,
        vec![1],
        "the held slot is still one observable damage"
    );
    // The first frame fact is the first REAL decode (slot 1).
    let first = outputs.iter().find_map(|output| match output {
        EngineOutput::Event(EngineEvent {
            kind: EngineEventKind::FirstFrame { position },
            ..
        }) => Some(*position),
        _ => None,
    });
    match first {
        Some(Stamp::Present(timestamp)) => {
            assert_eq!(
                timestamp.ticks, 1_000,
                "the first decoded frame speaks, not a fabrication"
            );
        }
        other => unreachable!("a first frame exists: {other:?}"),
    }
}

#[test]
fn all_corrupt_video_reaches_the_typed_terminal_within_the_budget() {
    // Boundary honesty (D-3): a stream whose EVERY frame fails is not
    // playing — the concealment budget (doc 03 §4.3(d)'s bounded
    // window precedent) spends and the damage reaches a typed
    // terminal: ResourceLimit/RecoveryAttempts with the budget and
    // the attempt count, landing the session in Error.
    let budget = u64::from(playback_engine::VIDEO_CONCEALMENT_BUDGET);
    let mut fixture = open_video(
        budget + 1,
        VideoProvider {
            corrupt: Vec::new(),
            fail_all: Some(corrupt_frame_error(0)),
        },
    );
    poll_until(
        &mut fixture,
        |s| s == SessionState::Error,
        1_600,
        "budget terminal",
    );
    let stats = fixture.engine.stats();
    assert_eq!(stats.frames_delivered, 0, "nothing ever decoded");
    assert_eq!(
        stats.frames_concealed, 0,
        "no prior frame to repeat — every slot held"
    );
    assert_eq!(collected(&fixture.collector).len(), 0);
    // The typed terminal: fatal ResourceLimit naming the spent budget.
    let outputs = take_events(&mut fixture);
    let fatal = outputs
        .iter()
        .find_map(|output| match output {
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::Fatal { error },
                ..
            }) => Some(error),
            _ => None,
        })
        .must();
    assert_eq!(fatal.stage, PipelineStage::Decode);
    assert_eq!(fatal.context.code, "decode.concealment.budget");
    match &fatal.kind {
        ErrorKind::ResourceLimit {
            limit,
            budget: declared,
            attempted,
        } => {
            assert_eq!(*limit, media_types::error::LimitKind::RecoveryAttempts);
            assert_eq!(*declared, budget);
            assert_eq!(
                *attempted,
                budget + 1,
                "the terminal fires one past the window"
            );
        }
        other => unreachable!("the budget terminal is a ResourceLimit: {other:?}"),
    }
    // The concealed attempts were all observable before the terminal:
    // the damage facts sum to exactly the budget.
    let damaged: u64 = outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::DamagedData { count },
                ..
            }) => Some(*count),
            _ => None,
        })
        .sum();
    assert_eq!(damaged, budget, "every concealed attempt was counted");
}

#[test]
fn non_damage_video_decode_errors_stay_fatal() {
    // The classifier boundary: `Unsupported` (and by symmetry every
    // non-damage kind) is deterministic per stream — retrying the
    // next frame cannot help — so it must keep the fatal path even on
    // a video limb (doc 03 §4.5(d)). The audio-limb half of the pin
    // is `an_armed_decode_failure_transitions_to_error_from_playing`
    // (an Unsupported error on an audio limb stays session-fatal;
    // audio Bitstream damage is 3.C2 D-2's bounded skip-and-report,
    // pinned by the recovery suites).
    let unsupported = EngineError::new(
        PipelineStage::Decode,
        ErrorKind::Unsupported {
            format: Some(media_types::error::FormatInfo {
                name: Some("mjpeg".to_string()),
                sniffed: Vec::new(),
            }),
        },
        media_types::error::ErrorContext::new("mjpeg.unsupported"),
    );
    let mut fixture = open_video(
        8,
        VideoProvider {
            corrupt: Vec::new(),
            fail_all: Some(unsupported),
        },
    );
    poll_until(&mut fixture, |s| s == SessionState::Error, 40, "fatal");
    let outputs = take_events(&mut fixture);
    let fatal = outputs
        .iter()
        .find_map(|output| match output {
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::Fatal { error },
                ..
            }) => Some(error),
            _ => None,
        })
        .must();
    assert!(
        matches!(fatal.kind, ErrorKind::Unsupported { .. }),
        "unsupported stays fatal, verbatim: {fatal}"
    );
    assert_eq!(fatal.context.code, "mjpeg.unsupported");
    assert_eq!(fixture.engine.stats().frames_concealed, 0);
}

/// The documented session bounds are pinned (directives-audit M1, card
/// W1-07): the command inbox bound, the live-event outbox bound and
/// the undelivered-receipt hard cap. Changing any of these is a policy
/// change — this test is the tripwire that makes the change a visible,
/// reviewed event. (The counting-allocator storm proofs over the same
/// bounds live in `bounded_alloc.rs`.)
#[test]
fn the_documented_session_bounds_are_pinned() {
    assert_eq!(
        playback_engine::INBOX_CAPACITY,
        64,
        "the command inbox bound"
    );
    assert_eq!(
        playback_engine::OUTBOX_CAPACITY,
        1024,
        "the live-event outbox bound"
    );
    assert_eq!(
        playback_engine::RECEIPT_CAPACITY,
        4096,
        "the undelivered-receipt hard cap"
    );
}

/// The inbox overflow policy (directives-audit M1 item ii), engine
/// level: submissions past the bound answer the typed
/// [`SessionError::InboxBusy`] — backpressure by rejection (doc 03
/// §4.6(c)/§9: never a block, never a silent drop; D-1's
/// explicit-rejection option) — exactly at the bound, never before;
/// one poll drains the whole inbox in submit order; and there is room
/// again after the drain.
#[test]
fn the_inbox_refuses_typed_at_its_bound_and_drains_whole() {
    let mut fixture = open_audio(8);
    let mut accepted = 0u64;
    let mut busy = 0u64;
    for index in 0..(playback_engine::INBOX_CAPACITY + 8) as u64 {
        match fixture.handle.pause(RequestId(index)) {
            Ok(()) => accepted += 1,
            Err(SessionError::InboxBusy) => busy += 1,
            Err(other) => unreachable!("pre-poll submit: {other}"),
        }
    }
    assert_eq!(
        accepted,
        playback_engine::INBOX_CAPACITY as u64,
        "the bound admits exactly its capacity"
    );
    assert_eq!(busy, 8, "and refuses the overflow, typed");
    // One poll drains the whole inbox: every queued command answers,
    // in submit order (no drops, no reordering).
    fixture.engine.poll();
    let drained = take_events(&mut fixture);
    let requests: Vec<u64> = drained
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Receipt(receipt) => Some(receipt.request.0),
            _ => None,
        })
        .collect();
    assert_eq!(
        requests,
        (0..playback_engine::INBOX_CAPACITY as u64).collect::<Vec<_>>(),
        "the whole inbox drained, in order"
    );
    // And there is room again.
    fixture.handle.pause(RequestId(999)).must();
}

// ---------------------------------------------------------------------------
// Epoch-aware output hub (W1-10 wiring, audit M11)
// ---------------------------------------------------------------------------

/// Open a session on an EXISTING engine (the M11 scenario needs one
/// engine-scoped output hub across two sessions).
fn open_on(engine: &mut Engine, script_len: u64) -> SessionHandle {
    let script: Vec<DemuxEvent> = (0..script_len)
        .map(|index| DemuxEvent::Packet(packet(TrackId::new(1), index)))
        .collect();
    let mut config = SessionConfig::new(
        SourceUri::parse("file:///tmp/lumen-fake.media").must(),
        Box::new(FakeDemuxer {
            tracks: vec![track(1, MediaKind::Audio)],
            script: Arc::new(Mutex::new(VecDeque::from(script))),
            stall: Arc::new(Mutex::new(false)),
            fail_start: None,
        }),
        Arc::new(FakeProvider { fail_select: None }),
        Arc::new(Collector::default()),
        virtual_clock(),
    );
    config.queues = tiny_queues();
    config.budget = tiny_budget();
    config.throttle = frozen_throttle();
    engine.open(config).must()
}

fn prime_playing(engine: &mut Engine, context: &str) {
    for _ in 0..40 {
        if engine.state() == SessionState::Playing {
            return;
        }
        engine.poll();
    }
    assert_eq!(engine.state(), SessionState::Playing, "{context}");
}

#[test]
fn superseded_epoch_events_never_leak_into_the_next_session() {
    // M11: one engine, one engine-scoped output hub, two sessions.
    // Session 1 (epoch 1) runs open→play→stop→close with NO drain;
    // session 2 (epoch 2) opens at pipeline 0 — every epoch-1 event
    // is now stale BY CONSTRUCTION. The bare `generation < live`
    // comparison waved all ten of them through (verified: seq 1..8
    // at gen 0, seq 9 at gen 1, seq 10 at gen 2 — all leaked).
    let mut engine = Engine::new();

    let handle1 = open_on(&mut engine, 200);
    prime_playing(&mut engine, "priming 1");
    handle1.stop(RequestId(80)).must();
    engine.poll();
    handle1.close(RequestId(81)).must();
    engine.poll();
    assert_eq!(engine.state(), SessionState::Idle);

    let handle2 = open_on(&mut engine, 200);
    prime_playing(&mut engine, "priming 2");
    drop(handle2);

    // NOW drain: current pair is (epoch 2, pipeline 0).
    let outputs = engine.take_outputs();
    let stale_leaked: Vec<(u64, SessionId, Generation)> = outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Event(event) => Some((event.seq, event.session, event.generation)),
            _ => None,
        })
        .filter(|(_, session, _)| *session != SessionId(2))
        .collect();
    assert!(
        stale_leaked.is_empty(),
        "superseded-epoch events leaked across the session boundary: {stale_leaked:?}"
    );
    assert!(
        engine.stats().suppressed_stale_events > 0,
        "the epoch boundary must count its suppressions: {:?}",
        engine.stats()
    );
    // Receipts of the dead session's commands survive (doc 05 §4.3:
    // responses are never suppressed) — the fix must not overreach.
    let receipts: Vec<RequestId> = outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Receipt(receipt) => Some(receipt.request),
            _ => None,
        })
        .collect();
    assert!(receipts.contains(&RequestId(80)));
    assert!(receipts.contains(&RequestId(81)));
}
