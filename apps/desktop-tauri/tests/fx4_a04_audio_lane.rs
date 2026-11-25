//! FX4-A04 — the tauri audio-lane inheritance proof (FIX-PLAN §3 F1.2's
//! audio half; wave-14's "does the tauri app construct its host/pipeline
//! with the audio lane inherited" question, apps/desktop-tauri scope).
//!
//! The answer these tests pin: the wrapper owns NO pipeline construction
//! of its own. `app.rs`'s `run()` manages exactly one
//! `ShellState(Mutex::new(ShellRuntime::new(DEFAULT_EVENT_CAPACITY)))`,
//! and `ShellRuntime` → `ShellHost::new()` adopts the CLI's own
//! `lumen_cli::host::CliHost` (apps/desktop/src/dispatch.rs:84) — the
//! host whose shared open path constructs the audio session
//! (`pipeline_host::open_audio_session`, the FX4-A01 backend factory's
//! site). Audio therefore flows in the tauri lane by construction, and
//! there is no discard shortcut to find: the wrapper's whole job is to
//! route `session.open` verbatim (`runtime.dispatch(&request)`,
//! app.rs:151) and pump (`runtime.pump_session()`, app.rs:204).
//!
//! The proof is a seam proof: construct the runtime exactly as app.rs
//! does, open audio-only media through the wrapper's own funnels, drive
//! it with the pump body the wrapper's ticker drives, and observe the
//! audio limb alive end to end. `pump_once` is the deterministic twin
//! of the production ticker's `pump_session` (the same host pump; the
//! production cadence adds only the wall-bounded advance — see the
//! `pacer_proof` test in apps/desktop/src/shell.rs, whose conventions
//! this file mirrors).
//!
//! CI-gated with the whole crate: `webkit2gtk-4.1` is absent from dev
//! boxes, so the crate builds only in its dedicated lane (ci.yml
//! `tauri-check`, `cargo test --manifest-path
//! apps/desktop-tauri/Cargo.toml --locked --all-targets`) — the same
//! gate `tests/single_command.rs` rides. Authored against the
//! headless seam tests in apps/desktop/src/shell.rs; the assertions
//! there run in the workspace lane and are the oracle for these.

use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use commands::envelope::RequestId;
use commands::Command;
use commands::CommandOutcome;
use commands::EventKind;
use commands::PlaybackState;
use commands::WireRequest;
use desktop::dispatch::open_file_request;
use desktop::shell::ShellRuntime;
use desktop::shell::DEFAULT_EVENT_CAPACITY;

/// The audio-only fixture (0.2 s, stereo 44.1 kHz PCM): every frame the
/// consumer accepts in this session is AUDIO, so the consumer's
/// counters answer the audio-lane question with no video lane to
/// confound them.
const AUDIO_ONLY_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/generated/valid/wav_pcm_s16le_44k1_st.wav"
);

/// The pump bound: the AVI pacer proof (apps/desktop/src/shell.rs) needs
/// ~170 pumps for 2.0 s of media; this 0.2 s audio-only fixture needs
/// far less — 512 is pure headroom, never the pacing truth.
const PUMP_BOUND: usize = 512;

/// The audio-lane inheritance proof: the runtime constructed exactly
/// as app.rs constructs it (`ShellRuntime::new(DEFAULT_EVENT_CAPACITY)`
/// — the `ShellState` app.rs:290 manages), audio-only media opened
/// through the wrapper's own argv/drag-drop funnel (`open_path` — the
/// call app.rs:281 makes), playback started through the single command
/// (`dispatch` — the `lumen_dispatch` body app.rs:151 runs), then only
/// pump ticks drive the session to its end.
///
/// The audio-lane observables (all public host seams — the same ones
/// the CLI lane reads):
/// * `audio_lane_counters().accepted > 0` — an audio-only session's
///   every accepted frame is audio: the audio limb was constructed and
///   consumed decoded audio. A discard shortcut (a tauri-side
///   `no_audio` default, a cfg gate, or a limb-less open) would leave
///   the accepted count at zero forever;
/// * `position_us() > 0` — audio-only media advance ONLY through the
///   audio-master sample-counter clock (the device rendered). A
///   session with no audio limb latches at the anchor (issue #16's
///   wall-clock latch) and, with no video to present either, reports
///   position 0 forever;
/// * `Playing` reached and `Ended` reached within the bound — the
///   session ran as a session, not a frozen shell.
///
/// `timing.time-tick` is deliberately NOT asserted: the default tick
/// interval is 250 ms (TIME_TICK_DEFAULT_MS) and the fixture is 0.2 s —
/// zero ticks is the correct behavior for this media, not a failure.
#[test]
fn fx4_a04_tauri_runtime_audio_lane_runs_to_end() {
    // app.rs:290-292's exact construction (the managed ShellState).
    let mut runtime = ShellRuntime::new(DEFAULT_EVENT_CAPACITY);
    // The argv/drag-drop funnel (app.rs:281's call, verbatim).
    let open = match runtime.open_path(&PathBuf::from(AUDIO_ONLY_FIXTURE)) {
        Some(response) => response,
        None => unreachable!("an audio-only media file must classify as openable"),
    };
    match open.result {
        Ok(CommandOutcome::Accepted { .. }) => {}
        other => unreachable!("a real audio file must open accepted: {other:?}"),
    }
    // The webview-invoke body (app.rs:151's dispatch, verbatim shape).
    let play_id = runtime.next_request_id();
    let play = runtime.dispatch(&WireRequest {
        request_id: play_id,
        command: Command::PlaybackPlay,
    });
    match play.result {
        Ok(CommandOutcome::Accepted { .. }) => {}
        other => unreachable!("play must be accepted from every live state: {other:?}"),
    }
    // The pacer loop — the ticker's exact body (app.rs:204's
    // pump_session; pump_once is its deterministic twin), driven to
    // the terminal state. The first take also drains the open's own
    // event burst (the queue the dispatch path filled).
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
            "the pump loop must not hang: a bound breach is a failure, never a mystery"
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
        Some((accepted, refused, stale)) => {
            assert!(
                accepted > 0,
                "decoded audio frames must reach the consumer's audio lane \
                 (accepted={accepted}, refused={refused}, stale={stale}, {pumps} pumps)"
            );
        }
        None => unreachable!("an open session wires a consumer"),
    }
    // The audio-master clock moved: audio-only media advance ONLY
    // through the device's sample counter (a limb-less session latches
    // at 0 forever — no audio device, no video to present).
    assert!(
        runtime.engine_host().position_us() > 0,
        "the audio-master sample-counter clock must have advanced ({pumps} pumps)"
    );
    // The egress accounting conserves end to end.
    assert!(runtime.event_stats().conserves());
}

/// The discard-shortcut oracle: the `session.open` request the tauri
/// lane's file funnels ship (argv at startup, drag-drop) carries the
/// audio track ENABLED — `open_file_request` builds
/// `options: Default::default()` (apps/desktop/src/dispatch.rs:225)
/// and `OpenOptions`' default is `no_audio: false` /
/// `no_video: false` (crates/commands/src/catalog/types.rs:240-252).
///
/// This is the regression bar for the audit's "no discard shortcut in
/// the tauri configuration": a future drift that disables audio in the
/// shell funnel (a `no_audio: true` default, a headless cfg) fails
/// here at the wire level, before any engine behavior changes.
#[test]
fn fx4_a04_open_funnel_keeps_the_audio_track_enabled() {
    // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
    let request = open_file_request(RequestId(42), &PathBuf::from("/tmp/a.wav"));
    assert_eq!(request.request_id, RequestId(42));
    match request.command {
        Command::SessionOpen { options, .. } => {
            assert!(
                !options.no_audio,
                "the shell's file funnel must never discard the audio track"
            );
            assert!(
                !options.no_video,
                "the shell's file funnel must never discard the video track"
            );
            assert!(!options.start_paused, "the funnel opens live, not paused");
            assert!(
                options.start_time.is_none(),
                "the funnel opens from the start position"
            );
        }
        other => unreachable!("a dropped/picked media file must open a session: {other:?}"),
    }
}
