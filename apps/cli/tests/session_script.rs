//! The card 2.D2 acceptance session, driven through the REAL binary
//! (doc 13: "scripted CLI session passes (open fixture → play →
//! pause → seek → stop) incl. the unsupported-file error path").
//!
//! The driver is `std::process` with piped stdio — the same surface a
//! script or the conformance-suite `CliHost` would use (doc 05 §5.3's
//! controller mode). Determinism: the engine advances only at command
//! boundaries and `advance` (the poll model); the binary performs no
//! wall-clock sleep anywhere, so two runs must be byte-identical
//! (pinned by test).

use std::io::Write;
use std::process::{Command, Stdio};

use testkit::Must;

/// The AVI parent fixture (class A): MJPEG+PCM, 2.000 s, 320x240
/// @30 fps, stereo s16le 44.1 kHz.
fn parent() -> String {
    format!(
        "{}/../../tests/fixtures/generated/valid/avi_mjpeg_pcm.avi",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// Run the binary once with `args` and a piped stdin script; answer
/// (stdout, stderr, exit code).
fn run(args: &[&str], script: &str) -> (String, String, i32) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lumen"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .must();
    {
        let stdin = child.stdin.as_mut().must();
        stdin.write_all(script.as_bytes()).must();
    }
    let output = child.wait_with_output().must();
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().unwrap_or(-1),
    )
}

/// THE scripted session: open → play → pause → seek → stop, with the
/// `info` snapshot asserted between the steps (the card's AC).
#[test]
fn scripted_session_open_play_pause_seek_stop() {
    let script = "play\npause\ninfo\nseek 50%\nstop\ntime\nquit\n";
    let (out, err, code) = run(&["open", &parent()], script);
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(err.is_empty(), "stderr:\n{err}");

    // The open lands (class A) and auto-plays through the real
    // transitions: Idle → Opening → Buffering → Playing.
    assert!(out.contains("ok: accepted session=1"), "{out}");
    assert!(out.contains("state-changed Idle → Opening"), "{out}");
    assert!(out.contains("state-changed Opening → Buffering"), "{out}");
    assert!(out.contains("state-changed Buffering → Playing"), "{out}");

    // play is an idempotent self-confirmation while playing (C8).
    // pause lands: Playing → Paused.
    let paused_at = out.find("state-changed Playing → Paused").must();
    assert!(out.contains("ok: accepted session=1"), "{out}");

    // The info snapshot (the AC's "info output"): container, duration,
    // the two tracks with their container-declared facts.
    let info_at = out.find("ok: media").must();
    assert!(info_at > paused_at, "info comes after the pause:\n{out}");
    let info = &out[info_at..];
    assert!(info.contains("container: avi"), "{info}");
    assert!(info.contains("duration: 2.000000s"), "{info}");
    assert!(info.contains("seekable: yes"), "{info}");
    assert!(info.contains("tracks: 2"), "{info}");
    assert!(info.contains("#0 video mjpeg 320x240"), "{info}");
    assert!(info.contains("#1 audio pcm_s16le 2ch 44100Hz"), "{info}");
    assert!(info.contains("(selected)"), "{info}");

    // The seek composes through the engine's real transitions (the
    // recorded 2.D2 shape): Paused → Stopped → Idle → Opening →
    // Buffering → Playing, under a NEW session id.
    let seek_stop = out.find("state-changed Paused → Stopped").must();
    assert!(
        seek_stop > info_at,
        "seek follows the info snapshot:\n{out}"
    );
    assert!(out.contains("state-changed Stopped → Idle"), "{out}");
    assert!(out.contains("state-changed Idle → Opening"), "{out}");
    assert!(out.contains("ok: accepted session=2"), "{out}");

    // stop tears the pipeline down (the session survives).
    assert!(out.contains("state-changed Playing → Stopped"), "{out}");

    // time after the 50% seek + stop: the clock froze at the seek
    // landing — exactly half of 2.000 s (pipeline-only settles never
    // advance media time; only `advance` does).
    assert!(out.contains("ok: time 1.000000s (50.0%)"), "{out}");
}

/// Determinism: the same script run twice is byte-identical (the
/// poll-driven model's own AC — no wall clock anywhere).
#[test]
fn scripted_session_is_deterministic() {
    let script = "play\npause\nseek 50%\nstop\ntime\nquit\n";
    let first = run(&["open", &parent()], script);
    let second = run(&["open", &parent()], script);
    assert_eq!(first, second);
}

/// Run-to-end: `advance` (the host verb) drives the whole media
/// through the null device; the end-of-media event fires and the
/// position reaches the duration exactly (the tail play-out).
#[test]
fn scripted_session_runs_to_end() {
    let script = "advance 512\ntime\nquit\n";
    let (out, err, code) = run(&["open", &parent()], script);
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(out.contains("event: end-of-media"), "{out}");
    assert!(out.contains("state-changed Playing → Ended"), "{out}");
    assert!(out.contains("state: Ended (position 2.000000s)"), "{out}");
    assert!(out.contains("ok: time 2.000000s (100.0%)"), "{out}");
}

/// `--start-time` opens mid-media: the demuxer's real seek query
/// validates the landing and the skip wrapper drops pre-target
/// packets (the open path's own ACs, through the binary).
#[test]
fn start_time_opens_at_the_target() {
    let script = "time\nadvance 512\ntime\nquit\n";
    let (out, err, code) = run(&["open", &parent(), "--start-time", "1s"], script);
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    // The position starts at the anchor and the media still ends
    // exactly at its duration.
    assert!(out.contains("ok: time 1.000000s (50.0%)"), "{out}");
    assert!(out.contains("ok: time 2.000000s (100.0%)"), "{out}");
}

/// `--start-paused`: the open settles then pauses from Playing.
#[test]
fn start_paused_opens_then_pauses() {
    let script = "time\nquit\n";
    let (out, err, code) = run(&["open", &parent(), "--start-paused"], script);
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(out.contains("state-changed Playing → Paused"), "{out}");
    assert!(out.contains("ok: time 0.000000s (0.0%)"), "{out}");
}

/// Pause freezes the clock: advance while paused moves nothing.
#[test]
fn pause_freezes_the_position() {
    let script = "pause\nadvance 64\ntime\nquit\n";
    let (out, err, code) = run(&["open", &parent()], script);
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(out.contains("state-changed Playing → Paused"), "{out}");
    assert!(out.contains("ok: time 0.000000s (0.0%)"), "{out}");
}

/// `toggle` flips by the live state (the keymap target).
#[test]
fn toggle_flips_play_pause() {
    let script = "toggle\ntoggle\nquit\n";
    let (out, err, code) = run(&["open", &parent()], script);
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    // Playing already → toggle pauses; toggle again resumes.
    assert!(out.contains("state-changed Playing → Paused"), "{out}");
    assert!(out.contains("state-changed Paused → Playing"), "{out}");
}

/// `close` from Playing is the honest Phase-3 refusal (card 3.C1
/// owns active-close completeness); `stop` first, then `close` lands
/// Stopped → Idle, and a later `time` is the honest not-ready —
/// state truth survives the session.
#[test]
fn close_then_snapshot_is_not_ready() {
    let script = "stop\nclose\ntime\nquit\n";
    let (out, err, code) = run(&["open", &parent()], script);
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(out.contains("state-changed Playing → Stopped"), "{out}");
    assert!(out.contains("state-changed Stopped → Idle"), "{out}");
    assert!(err.contains("[session.not-ready]"), "{err}");
}

/// The chapter grammar refuses honestly (AVI v0 carries no chapters).
#[test]
fn chapter_seek_refuses_typed() {
    let script = "seek chapter 3\nquit\n";
    let (out, err, code) = run(&["open", &parent()], script);
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(err.contains("[session.not-found]"), "{err}");
    assert!(err.contains("no chapter 3"), "{err}");
}
