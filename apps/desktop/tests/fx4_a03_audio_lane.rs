//! FX4-A03 — the native desktop host's audio-lane inheritance proof
//! (FIX-PLAN §3 F1.1, wave-FX4 row FX4-A03: "desktop host audio chain
//! verification/fix (chains through CliHost)"; the desktop-crate twin
//! of the tauri lane's FX4-A04 proof in
//! apps/desktop-tauri/tests/fx4_a04_audio_lane.rs).
//!
//! The verification answer these tests pin: the desktop host owns NO
//! audio construction of its own. [`ShellRuntime`] composes
//! [`desktop::dispatch::ShellHost`], which adopts the CLI's own
//! `lumen_cli::host::CliHost` (apps/desktop/src/dispatch.rs:84 — the
//! ARCH-5 reuse), and every session open — dialog pick, drag-drop,
//! direct path, webview invoke — routes through the ONE
//! `lumen_dispatch` → `lumen_cli::dispatch::lumen_dispatch` →
//! `CliHost::open_session`, which builds the audio limb through the
//! shared `pipeline_host::open_audio_session` (apps/cli/src/host.rs:944)
//! — the exact site FX4-A01's backend factory owns. The pacer
//! delegates to the same host's pumps (`pump_once` → `CliHost::pump`,
//! `pump_session` → `CliHost::pump_wallclock`), and the crate's own
//! transports/taps/bridges carry the LUF1 VIDEO lane only. So there is
//! no per-host hardcoded NullBackend, no audio-discard shortcut, no
//! second audio pipeline, and no desktop-side audio-controls surface
//! that could go dead: rg `audio|volume|mute|NullBackend` over
//! apps/desktop/src finds only two doc-comment mentions.
//!
//! The FX4-A01 backend factory and the FX4-A02 audio verbs are
//! therefore inherited by construction, and the deliverable is the
//! LOCK: four regression bars that fail the moment that inheritance
//! drifts —
//!
//! 1. [`fx4_a03_desktop_runtime_audio_lane_runs_to_end`] — the audio
//!    limb, device, and audio-master clock are alive end to end in the
//!    runtime the ADR-004 wrapper drives;
//! 2. [`fx4_a03_open_funnel_keeps_the_audio_track_enabled`] — both
//!    acquisition funnels ship `session.open` with audio enabled;
//! 3. [`fx4_a03_audio_verbs_delegate_to_the_adopted_host`] — the whole
//!    catalog audio verb surface answers exactly the adopted CLI
//!    host's own answer (no shell-side interception/rewrite);
//! 4. [`fx4_a03_live_session_audio_verbs_delegate_unchanged`] — the
//!    same delegation invariant with a live session playing.
//!
//! Unlike the tauri twin (CI-gated behind webkit2gtk-4.1), this crate
//! builds headlessly — these tests run in the ordinary workspace lane,
//! the same lane the shell.rs pacer proofs run in, whose conventions
//! (fixture paths, pump bounds, hang-guard deadlines) this file
//! mirrors.

use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use commands::envelope::RequestId;
use commands::{Command, CommandOutcome, EventKind, PlaybackState, WireRequest};
use desktop::dispatch::{open_file_request, open_path_request};
use desktop::files::{DragDropHandler, PathDragDrop};
use desktop::shell::{ShellRuntime, DEFAULT_EVENT_CAPACITY};
use lumen_cli::dispatch::lumen_dispatch as adopted_lumen_dispatch;
use lumen_cli::host::CliHost;

/// The audio-only fixture (0.2 s, stereo 44.1 kHz s16le PCM): every
/// frame the consumer accepts in this session is AUDIO, so the
/// consumer's counters answer the audio-lane question with no video
/// lane to confound them. The CLI conformance lane's oracle for this
/// same fixture is `ok: time 0.200000s (100.0%)`
/// (apps/cli/tests/container_conformance.rs).
const AUDIO_ONLY_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/generated/valid/wav_pcm_s16le_44k1_st.wav"
);

/// The pump bound: the AVI pacer proof (apps/desktop/src/shell.rs)
/// needs ~170 full pumps for 2.0 s of demux-paced media; this 0.2 s
/// PCM fixture needs far less — 1024 is the same headroom the shell's
/// own proofs allow, never the pacing truth.
const PUMP_BOUND: usize = 1024;

/// The twin-drive pump count (the live-session delegation proof): a
/// small, fixed number of identical pumps leaves both hosts deep
/// inside the session's life — past the open burst, with the clock
/// running (or already ended for this short fixture) — deterministically.
const LIVE_PUMPS: usize = 8;

/// The audio-lane inheritance proof: the runtime constructed exactly as
/// the ADR-004 wrapper's `ShellState` manages it
/// (`ShellRuntime::new(DEFAULT_EVENT_CAPACITY)`), audio-only media
/// opened through the desktop host's own funnel (`open_path`), playback
/// started through the single command (`dispatch` — the webview
/// invoke body), then ONLY pump ticks — the pacer thread's exact body
/// — drive the session to its end.
///
/// The audio-lane observables (public seams of the adopted host — the
/// same ones the CLI lane reads):
/// * `audio_lane_counters().accepted > 0` — in an audio-only session
///   every accepted frame is audio: the limb was constructed (through
///   the shared `pipeline_host::open_audio_session`) and consumed
///   decoded audio. A desktop-side discard shortcut or a limb-less
///   open leaves the accepted count at zero forever;
/// * `position_us()` at the fixture's full duration — audio-only media
///   advances ONLY through the audio-master sample-counter clock (the
///   device rendered); a limb-less session latches at 0 forever
///   (issue #16's wall-clock latch);
/// * `Playing` and `Ended` reached, the egress conserving.
///
/// `timing.time-tick` is deliberately NOT asserted: the default tick
/// interval is 250 ms and the fixture is 0.2 s — zero ticks is the
/// correct behavior for this media, not a failure.
#[test]
fn fx4_a03_desktop_runtime_audio_lane_runs_to_end() {
    let mut runtime = ShellRuntime::new(DEFAULT_EVENT_CAPACITY);
    let open = match runtime.open_path(&PathBuf::from(AUDIO_ONLY_FIXTURE)) {
        Some(response) => response,
        None => unreachable!("an audio-only media file must classify as openable"),
    };
    match open.result {
        Ok(CommandOutcome::Accepted { .. }) => {}
        other => unreachable!("a real audio file must open accepted: {other:?}"),
    }
    // The webview-invoke body: play through the single command.
    let play_id = runtime.next_request_id();
    let play = runtime.dispatch(&WireRequest {
        request_id: play_id,
        command: Command::PlaybackPlay,
    });
    match play.result {
        Ok(CommandOutcome::Accepted { .. }) => {}
        other => unreachable!("play must be accepted from every live state: {other:?}"),
    }
    // The pacer loop — the ticker's exact body, driven to the terminal
    // state (the first take drains the open's own event burst).
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut saw_playing = false;
    let mut ended = false;
    let mut pumps = 0usize;
    while pumps < PUMP_BOUND {
        pumps += 1;
        runtime.pump_once();
        for event in runtime.take_pending_events() {
            match event.kind {
                EventKind::SessionStateChanged { to, .. } => match to {
                    PlaybackState::Playing => saw_playing = true,
                    PlaybackState::Ended => ended = true,
                    _ => {}
                },
                _ => {}
            }
        }
        if ended {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the pacer loop must not hang: a bound breach is a failure, never a mystery"
        );
    }
    assert!(
        saw_playing,
        "the audio lane must reach Playing ({pumps} pumps)"
    );
    assert!(
        ended,
        "the audio-only session must reach Ended within {PUMP_BOUND} pumps ({pumps} pumped)"
    );
    // The audio limb's own counters: every accepted frame in an
    // audio-only session is audio — the limb exists and consumed.
    match runtime.engine_host().audio_lane_counters() {
        Some((accepted, refused, stale)) => assert!(
            accepted > 0,
            "decoded audio frames must reach the consumer's audio lane \
             (accepted={accepted}, refused={refused}, stale={stale}, {pumps} pumps)"
        ),
        None => unreachable!("an open session wires a consumer"),
    }
    // The audio-master clock carried the session to the fixture's full
    // duration — the conformance lane's oracle for this fixture is
    // 0.2 s at 100.0%, so the ±10 ms window is the same tolerance the
    // CLI's own end-of-media assertions allow.
    let position_us = runtime.engine_host().position_us();
    assert!(
        (190_000..=210_000).contains(&position_us),
        "the audio-master sample counter must land at the media's end \
         ({position_us} µs, {pumps} pumps)"
    );
    // The egress accounting conserves end to end.
    assert!(runtime.event_stats().conserves());
}

/// The discard-shortcut oracle at the wire level, for BOTH acquisition
/// funnels the desktop host owns: the classifier funnel
/// (`open_path_request` — what `open_path`/dialog picks build) and the
/// drag-drop translation (`PathDragDrop::on_files_dropped`) ship
/// `session.open` for an audio file with the audio track ENABLED —
/// `OpenOptions` defaults (`no_audio: false`, `no_video: false`, live
/// from the start; crates/commands/src/catalog/types.rs). A future
/// desktop-side drift that discards audio in the funnel fails here at
/// the wire level, before any engine behavior changes.
#[test]
fn fx4_a03_open_funnel_keeps_the_audio_track_enabled() {
    // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
    let dropped_wav = PathBuf::from("/tmp/song.wav");
    // The classifier funnel.
    let request = match open_path_request(RequestId(42), &dropped_wav) {
        Some(request) => request,
        None => unreachable!("a wav is a media path"),
    };
    assert_eq!(request.request_id, RequestId(42));
    match request.command {
        Command::SessionOpen { options, .. } => {
            assert!(
                !options.no_audio,
                "the classifier funnel must never discard the audio track"
            );
            assert!(
                !options.no_video,
                "the classifier funnel must never discard the video track"
            );
            assert!(!options.start_paused, "the funnel opens live, not paused");
            assert!(
                options.start_time.is_none(),
                "the funnel opens from the start"
            );
        }
        other => unreachable!("a media file must open a session: {other:?}"),
    }
    // The drag-drop funnel: the first OPENABLE file wins (a dropped
    // README never becomes the open), and the translation is the same
    // classifier — the dropped wav carries the same audio-enabled
    // open options.
    let mut dnd = PathDragDrop;
    let request = match dnd.on_files_dropped(
        // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
        &[PathBuf::from("/tmp/notes.txt"), dropped_wav],
        RequestId(43),
    ) {
        Ok(Some(request)) => request,
        Ok(None) => unreachable!("a media file was dropped"),
        Err(err) => unreachable!("drop translation is pure logic: {err}"),
    };
    match request.command {
        Command::SessionOpen { options, .. } => {
            assert!(
                !options.no_audio,
                "the drop funnel must never discard the audio track"
            );
            assert!(
                !options.no_video,
                "the drop funnel must never discard the video track"
            );
        }
        other => unreachable!("a dropped media file must open a session: {other:?}"),
    }
}

/// The verb-routing drift-lock: for the whole audio verb surface of the
/// catalog, the desktop shell answers EXACTLY what the adopted CLI
/// host answers for the same request on identically-driven hosts. The
/// shell's single-command rule routes everything except the
/// frame-channel bind (shell-owned, not an audio verb) to
/// `lumen_cli::dispatch::lumen_dispatch` verbatim — so whatever the
/// adopted host's served table says (today: the honest ADR-005
/// unsupported-for-host refusal; after FX4-A02: the pipeline
/// `audio_controls()` routing), the desktop inherits it by
/// construction. A desktop that grows its own audio-verb handling — a
/// dead controls surface, a rewrite, an interception — makes the two
/// answers diverge and fails here. The equality is the invariant, so
/// the test holds on BOTH sides of FX4-A02's landing.
#[test]
fn fx4_a03_audio_verbs_delegate_to_the_adopted_host() {
    let mut runtime = ShellRuntime::default();
    let mut twin = CliHost::new();
    for (index, command) in audio_verb_surface().into_iter().enumerate() {
        let request = WireRequest {
            request_id: RequestId(100 + index as u64),
            command,
        };
        let via_shell = runtime.dispatch(&request);
        let via_host = adopted_lumen_dispatch(&mut twin, &request);
        assert_eq!(via_shell.request_id, via_host.request_id);
        assert_eq!(via_shell.session, via_host.session);
        assert_eq!(via_shell.generation, via_host.generation);
        assert_eq!(
            via_shell.result, via_host.result,
            "the desktop shell must answer audio verb {index} with the adopted host's own answer"
        );
    }
}

/// The live-session twin of the verb drift-lock: the SAME audio-only
/// fixture driven through (a) the desktop runtime's own funnels
/// (open_path → the single command → pump ticks) and (b) a bare
/// adopted `CliHost` driven through the identical dispatch sequence —
/// then the same `audio.set-volume` through both. Identical inputs
/// through the same code path leave the two hosts in identical state
/// (asserted), so identical answers are the delegation invariant.
/// This catches a shell that intercepts audio verbs only once a
/// session is live — the realistic drift for a desktop-side volume
/// knob that bypasses the host's real controls.
#[test]
fn fx4_a03_live_session_audio_verbs_delegate_unchanged() {
    // (a) the desktop runtime, through its own funnel.
    let mut runtime = ShellRuntime::new(DEFAULT_EVENT_CAPACITY);
    let opened = match runtime.open_path(&PathBuf::from(AUDIO_ONLY_FIXTURE)) {
        Some(response) => response,
        None => unreachable!("an audio-only media file must classify as openable"),
    };
    match opened.result {
        Ok(CommandOutcome::Accepted { .. }) => {}
        other => unreachable!("a real audio file must open accepted: {other:?}"),
    }
    let play_id = runtime.next_request_id();
    let played = runtime.dispatch(&WireRequest {
        request_id: play_id,
        command: Command::PlaybackPlay,
    });
    match played.result {
        Ok(CommandOutcome::Accepted { .. }) => {}
        other => unreachable!("play must be accepted: {other:?}"),
    }
    // (b) the twin: a bare adopted host driven through the SAME
    // dispatch sequence (open → play), the exact requests the runtime
    // shipped (the runtime's id allocator started at 1: open=1,
    // play=2).
    let mut twin = CliHost::new();
    let twin_opened = adopted_lumen_dispatch(
        &mut twin,
        &open_file_request(RequestId(1), &PathBuf::from(AUDIO_ONLY_FIXTURE)),
    );
    match twin_opened.result {
        Ok(CommandOutcome::Accepted { .. }) => {}
        other => unreachable!("the twin's open must accept too: {other:?}"),
    }
    let twin_played = adopted_lumen_dispatch(
        &mut twin,
        &WireRequest {
            request_id: RequestId(2),
            command: Command::PlaybackPlay,
        },
    );
    match twin_played.result {
        Ok(CommandOutcome::Accepted { .. }) => {}
        other => unreachable!("the twin's play must accept too: {other:?}"),
    }
    // Identical pumps: `pump_once` is `CliHost::pump` plus the event
    // drain (read-only on the engine), so the twin takes the same
    // engine steps.
    for _ in 0..LIVE_PUMPS {
        runtime.pump_once();
        twin.pump();
    }
    assert_eq!(
        runtime.engine_host().state(),
        twin.state(),
        "identical drive through the same code path must leave identical engine state"
    );
    // The audio verb, once, through each host — the runtime's next
    // allocated id (3) rides both dispatches.
    let verb = WireRequest {
        request_id: runtime.next_request_id(),
        command: Command::AudioSetVolume { volume: 0.5 },
    };
    let via_shell = runtime.dispatch(&verb);
    let via_host = adopted_lumen_dispatch(&mut twin, &verb);
    assert_eq!(via_shell.request_id, via_host.request_id);
    assert_eq!(
        via_shell.session.is_some(),
        via_host.session.is_some(),
        "both hosts answered from a live session"
    );
    assert_eq!(
        via_shell.result, via_host.result,
        "the live desktop shell must answer audio.set-volume with the \
         adopted host's own answer"
    );
}

/// The catalog's whole audio verb surface (the six shipped ids) — one
/// fresh command each, the wire shapes the UI would send.
fn audio_verb_surface() -> Vec<Command> {
    vec![
        Command::AudioSetVolume { volume: 0.5 },
        Command::AudioGetVolume,
        Command::AudioSetMute { muted: true },
        Command::AudioListDevices,
        Command::AudioSetDevice {
            id: "default".to_string(),
        },
        Command::AudioSetDelay { delay_ms: 0 },
    ]
}
