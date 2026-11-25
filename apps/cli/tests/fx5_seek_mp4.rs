//! FX5-A04 — the F1.3 acceptance made executable (FIX-PLAN §3): the
//! CLI-level MP4 seek integration proof that `seek 1s` on an MP4 →
//! `TimingSeekCompleted`, position ~1.0 s, playback continuing to the
//! media's end.
//!
//! The executed defect these tests flip (wave-05 S0; W20-A02 F-1;
//! W20-A05 F-001; MP4-REALITY-TRACE): every in-session MP4 seek
//! answered the `Demuxer` trait's default reanchor —
//! `TimingSeekFailed { "[demux] unsupported
//! (plugin.demux.reanchor.unsupported)" }` with the position stuck at
//! the old one — while the same session's `MediaInfoReady` said
//! `seekable: true` (the C-003 seek triple-lie's code side). The WAV
//! contrast completed (`TimingSeekCompleted(1000000)`) — the
//! reference implementation of the contract these tests hold the MP4
//! to. FX5-A01 lands `Mp4Demuxer::reanchor` on the existing
//! `seek()`/index machinery in this wave; this file is that landing's
//! acceptance oracle, coded to the CONTRACT exactly as `native_seek.rs`
//! models completed seeks for the AVI/OGG lanes: the in-process
//! [`CliHost`]/[`lumen_dispatch`] harness over the committed class-A
//! fixtures, the same session-epoch/event-stream discipline.
//!
//! The pinned invariants:
//!
//! - **The flip**: the seek completes — exactly one
//!   `timing.seek-completed` per seek, at the demuxer's ACTUAL
//!   keyframe-safe landing (never the request), zero fabricated
//!   failure terminals. The landing window is the MP4 fixture's own
//!   committed seek oracle — its expectations toml pins `[seek]
//!   tolerance_us = 33_333` (one 30 fps frame; the AVI reanchor
//!   tolerance model, avi/tests.rs:2695: the landing is the keyframe
//!   at/below the target).
//! - **The epoch survives** (D-4/D-7): one `Idle → Opening` for the
//!   whole session, zero teardown edges — the seek runs in-pipeline,
//!   never the forbidden stop→close→reopen composition.
//! - **Composed seeks compose**: forward to 1 s, back to the head,
//!   forward to the half again — one terminal per seek, each at its
//!   own actual (the AVI double-reanchor / rapid-storm model).
//! - **The out-of-range discipline** (F0.4's shape): a crafted
//!   beyond-duration target answers typed and honestly, never a
//!   panic — the engine clamps it to the duration (doc 05 §2.4:
//!   clamping is truth, not failure) and the seek lands at the end;
//!   the impossible one (from `Ended`) is the state table's typed
//!   refusal at the standing position.
//! - **The WAV guard**: the same in-session protocol on the reference
//!   implementation (the committed PCM fixture) lands within the
//!   recorded tolerance — one guard test, not a WAV re-proof.

use commands::catalog::types::{OpenOptions, SeekTarget, SourceSpec, Timestamp};
use commands::envelope::RequestId;
use commands::{Command, EventKind, PlaybackState, WireRequest};
use lumen_cli::dispatch::lumen_dispatch;
use lumen_cli::host::CliHost;
use testkit::Must;

/// The class-A MP4 fixture's committed seek oracle: its expectations
/// toml pins `[seek] tolerance_us = 33_333` — one frame of the 30 fps
/// grid, the keyframe-safe landing window.
const FRAME_TOLERANCE_US: i64 = 33_333;

/// The MP4 parent fixture (class A): MJPEG+PCM, 2.000 s, 320x240 @
/// 30 fps, stereo s16le 44.1 kHz — the AVI parent's exact twin
/// through the ISO family. Every one of its 60 video samples is a
/// sync sample (stss 1..60), so the reanchor's landing granularity is
/// the 33.333 ms frame grid and the frame-aligned 1 s target lands
/// exactly at 1.000 s (the binary lane's `--start-time 1s` pin: "ok:
/// time 1.000000s (50.0%)").
fn mp4_parent() -> SourceSpec {
    SourceSpec::File {
        path: format!(
            "{}/../../tests/fixtures/generated/valid/mp4_mjpeg_pcm.mp4",
            env!("CARGO_MANIFEST_DIR")
        )
        .into(),
    }
}

/// The WAV reference fixture (class A): 0.200 s s16le stereo 44.1 kHz
/// — the reference implementation's lane (WavDemuxer implements
/// `reanchor`, the executed W20-A02 F-1 contrast). The binary lane's
/// own pins: `--start-time 100ms` lands exactly at half ("ok: time
/// 0.100000s (50.0%)"), the play-out ends exactly at 0.200000 s.
fn wav_reference() -> SourceSpec {
    SourceSpec::File {
        path: format!(
            "{}/../../tests/fixtures/generated/valid/wav_pcm_s16le_44k1_st.wav",
            env!("CARGO_MANIFEST_DIR")
        )
        .into(),
    }
}

/// Dispatch one command, asserting the envelope's success (the
/// honest-failure shapes match on the `Err` themselves).
fn dispatch(host: &mut CliHost, request_id: u64, command: Command) {
    let response = lumen_dispatch(
        host,
        &WireRequest {
            request_id: RequestId(request_id),
            command,
        },
    );
    match response.result {
        Ok(_) => {}
        Err(error) => unreachable!("command failed: {error}"),
    }
}

/// Open the given fixture (settled) and answer the host.
fn opened(source: SourceSpec) -> CliHost {
    let mut host = CliHost::new();
    dispatch(
        &mut host,
        1,
        Command::SessionOpen {
            source,
            options: OpenOptions::default(),
        },
    );
    host.settle();
    host
}

/// One `playback.seek` through the wire.
fn seek(host: &mut CliHost, request_id: u64, target: SeekTarget) {
    dispatch(host, request_id, Command::PlaybackSeek { target });
}

/// The position payloads of EVERY seek terminal in `host`'s event
/// stream, in order (the landing truths — `timing.seek-completed`,
/// doc 05 §3.2; exactly one per completed seek, doc 05 §4.3
/// invariant 4).
fn seek_completed_positions(host: &CliHost) -> Vec<Timestamp> {
    host.events()
        .iter()
        .filter_map(|event| match event.kind {
            EventKind::TimingSeekCompleted { position, .. } => Some(position),
            _ => None,
        })
        .collect()
}

/// How many `timing.seek-failed` terminals the stream holds — the
/// pre-F1.3 MP4 answer was exactly this one, every seek (the executed
/// F-001 shape); the honest completed lanes answer zero.
fn seek_failed_count(host: &CliHost) -> usize {
    host.events()
        .iter()
        .filter(|event| matches!(event.kind, EventKind::TimingSeekFailed { .. }))
        .count()
}

/// The teardown edges a reopen composition would have produced
/// (the stop→close half of the forbidden D-4/D-7 shape). These tests
/// never issue an explicit `stop`, so the honest answer is zero.
fn teardown_edges(host: &CliHost) -> usize {
    host.state_history()
        .iter()
        .filter(|(from, to)| {
            matches!(
                (from, to),
                (PlaybackState::Playing, PlaybackState::Stopped)
                    | (PlaybackState::Paused, PlaybackState::Stopped)
                    | (PlaybackState::Stopped, PlaybackState::Idle)
            )
        })
        .count()
}

/// How many `Idle → Opening` transitions the trail holds — the
/// native seek answers ONE (the open's); a reopen composition answers
/// one per seek.
fn open_edges(host: &CliHost) -> usize {
    host.state_history()
        .iter()
        .filter(|(from, to)| matches!((from, to), (PlaybackState::Idle, PlaybackState::Opening)))
        .count()
}

/// Whether a landing sits in the target's keyframe-safe window (the
/// fixture's committed frame-grid oracle).
fn lands_within(landed_us: i64, target_us: i64) -> bool {
    (landed_us - target_us).abs() <= FRAME_TOLERANCE_US
}

/// THE F1.3 acceptance line, made executable: open the MP4, play
/// briefly, `seek 1s` — the terminal is a COMPLETION (the F-001 flip:
/// pre-F1.3 every MP4 seek answered `TimingSeekFailed` with the
/// position at the old standing), the landing sits in the 1 s
/// frame-grid neighborhood (the actual, not the request), the clock
/// re-anchors at the same actual, the session epoch survives under a
/// minted generation, and the remainder still plays out to the
/// media's exact end (the binary lane's own pin: 2.000000 s, 100.0%).
#[test]
fn fx5_a04_seek_1s_on_mp4_completes_and_lands_near_1s() {
    let mut host = opened(mp4_parent());
    let session_before = host.session_id();
    let generation_before = host.generation();

    // Play briefly: one full pump renders the settled pre-roll into
    // the null device — the counter moves (playback demonstrably
    // under way) and the session is Playing when the seek is fired.
    host.advance(1);
    assert_eq!(host.state(), PlaybackState::Playing);
    assert!(
        host.position_us() > 0,
        "the pre-seek pump played (position {} µs)",
        host.position_us()
    );

    seek(
        &mut host,
        2,
        SeekTarget::Absolute {
            time: Timestamp(1_000_000),
        },
    );

    // THE FLIP: exactly one terminal, a COMPLETION — never the
    // executed TimingSeekFailed{plugin.demux.reanchor.unsupported}
    // with the position stuck at the pre-seek one.
    assert_eq!(
        seek_failed_count(&host),
        0,
        "no failure terminal was fabricated:\n{:?}",
        host.events()
    );
    let landings = seek_completed_positions(&host);
    assert_eq!(
        landings.len(),
        1,
        "exactly one terminal, at the actual landing"
    );
    let landed_us = landings[0].0;
    assert!(
        lands_within(landed_us, 1_000_000),
        "the landing {landed_us} µs must sit in the 1 s ± frame-grid neighborhood \
         (the fixture's committed 33 333 µs seek oracle)"
    );
    // The clock re-anchored at the landing's own neighborhood (the
    // first audio frame's timestamp is the anchor truth; µs
    // truncation per the AVI lane).
    assert!(
        lands_within(host.position_us(), 1_000_000),
        "the clock re-anchored at the actual ({} µs)",
        host.position_us()
    );

    // The epoch survives the seek (D-4/D-7): same session, one open
    // edge for the whole trail, zero teardowns, the seek's mint.
    assert_eq!(host.session_id(), session_before, "the epoch survives");
    assert_eq!(
        host.generation(),
        generation_before.map(|generation| commands::envelope::Generation(generation.0 + 1)),
        "the seek mints at accept"
    );
    host.settle();
    assert_eq!(host.state(), PlaybackState::Playing, "still in-pipeline");
    assert_eq!(teardown_edges(&host), 0, "{:?}", host.state_history());
    assert_eq!(open_edges(&host), 1, "{:?}", host.state_history());

    // Playback continues: the remainder plays out to the exact end.
    host.advance(lumen_cli::host::RUN_PUMPS as u32);
    assert_eq!(
        host.state(),
        PlaybackState::Ended,
        "the tail play-out closes"
    );
    assert_eq!(host.position_us(), 2_000_000, "the tail play-out closes");

    // The boundary stayed healthy: nothing was refused into the
    // present limb across the seek (the video-bearing lanes' own
    // counters).
    let accounting = host.headless_accounting().must();
    assert_eq!(accounting.frames_refused, 0, "{accounting:?}");
    assert_eq!(accounting.video_refused, 0, "{accounting:?}");
}

/// Composed seeks compose in-pipeline (the AVI double-reanchor model):
/// forward to 1 s, back to the head, forward to the half again —
/// three seeks back-to-back through the wire. The forbidden
/// composition would have answered three stop→close→reopens; the
/// native flow answers one terminal per seek, each at its own actual
/// landing, the epoch surviving all three (one open, zero teardowns,
/// three mints), and the remainder plays out to the media's exact end
/// from the LAST seek's landing.
#[test]
fn fx5_a04_seek_on_mp4_repeats_and_composes() {
    let mut host = opened(mp4_parent());
    let session_before = host.session_id();
    let generation_before = host.generation();

    seek(
        &mut host,
        2,
        SeekTarget::Absolute {
            time: Timestamp(1_000_000),
        },
    );
    seek(&mut host, 3, SeekTarget::Absolute { time: Timestamp(0) });
    seek(&mut host, 4, SeekTarget::Percent { fraction: 0.5 });

    // One terminal per seek (doc 05 §4.3 invariant 4), each at its
    // own actual landing on the frame grid: 1 s → ~1.000 s, the head
    // → ~0, 50% of the 2.000 s media → ~1.000 s again.
    assert_eq!(
        seek_failed_count(&host),
        0,
        "no failure terminal was fabricated:\n{:?}",
        host.events()
    );
    let landings = seek_completed_positions(&host);
    assert_eq!(landings.len(), 3, "one terminal per seek");
    assert!(
        lands_within(landings[0].0, 1_000_000),
        "the 1 s seek's landing: {} µs",
        landings[0].0
    );
    assert!(
        lands_within(landings[1].0, 0),
        "the back-to-head landing: {} µs",
        landings[1].0
    );
    assert!(
        lands_within(landings[2].0, 1_000_000),
        "the 50% landing: {} µs",
        landings[2].0
    );

    // The storm composed in-pipeline: the epoch survived the whole
    // storm, each seek minted its own generation at accept.
    assert_eq!(
        host.session_id(),
        session_before,
        "one session for three seeks"
    );
    assert_eq!(
        host.generation(),
        generation_before.map(|generation| commands::envelope::Generation(generation.0 + 3)),
        "each seek mints its own generation"
    );
    host.settle();
    assert_eq!(host.state(), PlaybackState::Playing, "still in-pipeline");
    assert_eq!(teardown_edges(&host), 0, "{:?}", host.state_history());
    assert_eq!(open_edges(&host), 1, "{:?}", host.state_history());
    // The clock stands at the LAST seek's actual landing.
    assert!(
        lands_within(host.position_us(), 1_000_000),
        "the last landing stands ({} µs)",
        host.position_us()
    );

    // Nothing wedged: the remainder plays out to the exact end.
    host.advance(lumen_cli::host::RUN_PUMPS as u32);
    assert_eq!(
        host.state(),
        PlaybackState::Ended,
        "the tail play-out closes"
    );
    assert_eq!(host.position_us(), 2_000_000, "the tail play-out closes");
}

/// The out-of-range discipline (F0.4's shape): a crafted
/// beyond-duration target (9 s on the 2.000 s media, 4.5× over) must
/// answer honestly and without a panic — the engine's documented truth
/// is the CLAMP (doc 05 §2.4: "clamping is truth, not failure"; the
/// CLI's own usage line says the host clamps against the duration),
/// so the seek lands at the end of the media and completes there.
/// The genuinely impossible one — the same target from `Ended` — is
/// the state table's typed refusal (the native-seek D4 discipline):
/// the wire answers the typed error naming the state, the position
/// stands, the session survives for a later close, and no terminal was
/// fabricated.
#[test]
fn fx5_a04_seek_out_of_range_on_mp4_refuses_typed() {
    let mut host = opened(mp4_parent());
    let session_before = host.session_id();

    // The beyond-duration target from `Playing`: the clamp truth.
    seek(
        &mut host,
        2,
        SeekTarget::Absolute {
            time: Timestamp(9_000_000),
        },
    );
    assert_eq!(
        seek_failed_count(&host),
        0,
        "the clamped seek fabricates no failure:\n{:?}",
        host.events()
    );
    let landings = seek_completed_positions(&host);
    assert_eq!(
        landings.len(),
        1,
        "exactly one terminal for the clamped seek"
    );
    let landed_us = landings[0].0;
    assert!(
        landed_us >= 1_900_000 && landed_us <= 2_000_000,
        "the clamp lands at the end of the media (the last keyframes' \
         neighborhood — the last 100 ms of the 2.000 s media), never at \
         the 9 s request: {landed_us} µs"
    );
    // The epoch survived the clamped seek too — no reopen.
    assert_eq!(host.session_id(), session_before, "the epoch survives");
    assert_eq!(teardown_edges(&host), 0, "{:?}", host.state_history());
    assert_eq!(open_edges(&host), 1, "{:?}", host.state_history());

    // The clamped landing plays out: the media ends honestly at the
    // landing's neighborhood.
    host.advance(lumen_cli::host::RUN_PUMPS as u32);
    assert_eq!(host.state(), PlaybackState::Ended, "the end lands");
    assert!(
        host.position_us() >= 1_900_000,
        "materially at the end: {} µs",
        host.position_us()
    );

    // The same beyond-duration target from `Ended`: the state table's
    // typed refusal, nothing moved, nothing died.
    let ended_at = host.position_us();
    let response = lumen_dispatch(
        &mut host,
        &WireRequest {
            request_id: RequestId(3),
            command: Command::PlaybackSeek {
                target: SeekTarget::Absolute {
                    time: Timestamp(9_000_000),
                },
            },
        },
    );
    match response.result {
        Err(ref error) => {
            assert_eq!(error.stage, commands::Stage::Session, "{error}");
            assert!(
                error.message.contains("seek in Ended"),
                "the refusal names the state: {error}"
            );
        }
        Ok(_) => unreachable!("seek from Ended must refuse typed"),
    }
    assert_eq!(host.position_us(), ended_at, "the old position stands");
    assert_eq!(host.session_id(), session_before, "the session survives");
    assert_eq!(
        seek_completed_positions(&host).len(),
        1,
        "no second terminal was fabricated"
    );
    assert_eq!(
        seek_failed_count(&host),
        0,
        "the refusal is the receipt, not a fabricated terminal"
    );
}

/// The WAV cross-container guard (one light consistency test — the
/// reference implementation, not a re-proof): the same in-session
/// protocol — open, settle, `seek 50%` — answers the same completion
/// contract on the PCM family. The 0.200 s fixture's half is 100 ms;
/// the frame-exact PCM reanchor lands it inside the recorded
/// Lands-class ±10% window (the ttf-seek binary convention), and the
/// remainder plays out to the fixture's exact end.
#[test]
fn fx5_a04_seek_on_wav_reference_container_lands_within_tolerance() {
    let mut host = opened(wav_reference());
    let session_before = host.session_id();
    seek(&mut host, 2, SeekTarget::Percent { fraction: 0.5 });

    assert_eq!(
        seek_failed_count(&host),
        0,
        "the reference lane fabricates no failure:\n{:?}",
        host.events()
    );
    let landings = seek_completed_positions(&host);
    assert_eq!(landings.len(), 1, "exactly one terminal");
    let landed_us = landings[0].0;
    assert!(
        landed_us >= 90_000 && landed_us <= 110_000,
        "the WAV landing {landed_us} µs must sit in the requested half's \
         ±10% neighborhood (the recorded Lands-class window)"
    );

    // The epoch survives on the reference lane too (the guard's
    // no-reopen half — the same protocol, the same discipline).
    assert_eq!(host.session_id(), session_before, "the epoch survives");
    assert_eq!(teardown_edges(&host), 0, "{:?}", host.state_history());
    assert_eq!(open_edges(&host), 1, "{:?}", host.state_history());
    host.settle();
    assert_eq!(host.state(), PlaybackState::Playing, "still in-pipeline");

    // Continuation: the remainder plays out to the exact end (the
    // binary lane's own pin: 0.200000 s, 100.0%).
    host.advance(lumen_cli::host::RUN_PUMPS as u32);
    assert_eq!(
        host.state(),
        PlaybackState::Ended,
        "the tail play-out closes"
    );
    assert_eq!(host.position_us(), 200_000, "the tail play-out closes");
}
