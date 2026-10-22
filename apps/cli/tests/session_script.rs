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

// ---------------------------------------------------------------------------
// M17: the graceful exit (doc 10 §6 scenario 16 — "exit during
// playback leaves no temp state": teardown within deadline, device
// closed, exit code clean)
// ---------------------------------------------------------------------------

/// THE M17 regression: quitting mid-play runs the mandated teardown —
/// stop from `Playing`, close from `Stopped` — through the same
/// dispatch the typed verbs use, with the state trail printed and a
/// clean exit. The bare `break` this replaces abandoned the audio
/// device mid-stream with no `Stopped`/`Idle` observability.
#[test]
fn quit_during_play_tears_the_session_down() {
    let (out, err, code) = run(&["open", &parent()], "quit\n");
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(err.is_empty(), "a clean teardown is silent: {err}");
    // The teardown's own trail, in causal order: stop, then close.
    let stopped = out.find("state-changed Playing → Stopped").must();
    let idle = out.find("state-changed Stopped → Idle").must();
    assert!(idle > stopped, "close follows the stop:\n{out}");
    // The teardown is the transcript's tail: the `→ Idle` event is the
    // last line (nothing follows the close but the exit).
    assert!(
        out.ends_with("event: state-changed Stopped → Idle\n"),
        "the close's event closes the transcript:\n{out}"
    );
}

/// The EOF twin (Ctrl-D): no `quit` line — the stream ends and the
/// same teardown runs.
#[test]
fn eof_during_play_tears_the_session_down() {
    let (out, err, code) = run(&["open", &parent()], "");
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(err.is_empty(), "a clean teardown is silent: {err}");
    assert!(out.contains("state-changed Playing → Stopped"), "{out}");
    assert!(
        out.ends_with("event: state-changed Stopped → Idle\n"),
        "the close's event closes the transcript:\n{out}"
    );
}

/// Quit from `Paused`: the stop lands from the paused state too
/// (`Paused → Stopped → Idle`) — the teardown does not depend on
/// `Playing`.
#[test]
fn quit_while_paused_still_tears_down() {
    let (out, err, code) = run(&["open", &parent(), "--start-paused"], "quit\n");
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(err.is_empty(), "a clean teardown is silent: {err}");
    let stopped = out.find("state-changed Paused → Stopped").must();
    let idle = out.find("state-changed Stopped → Idle").must();
    assert!(idle > stopped, "close follows the stop:\n{out}");
}

/// Quit after a typed close: nothing live, so the teardown is a no-op —
/// no second `Stopped → Idle` echo, no stderr, exit 0. Idempotence is
/// part of the mandate (the typed close already did the work).
#[test]
fn quit_after_close_is_a_no_op() {
    let (out, err, code) = run(&["open", &parent()], "stop\nclose\nquit\n");
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(err.is_empty(), "nothing to report: {err}");
    assert_eq!(
        out.matches("state-changed Playing → Stopped").count(),
        1,
        "the typed stop only:\n{out}"
    );
    assert_eq!(
        out.matches("state-changed Stopped → Idle").count(),
        1,
        "the typed close only — no teardown echo:\n{out}"
    );
}

/// The machine mode's EOF twin (M17): `--control-stdio --json` open
/// (`startPaused` — the open path's settle+pause composition, the
/// deterministic way a machine script lands a teardown-able state;
/// a bare `playback.play` would leave the C8 self-confirmation in
/// `Buffering`) → EOF tears the session down; the teardown's trail
/// rides the NDJSON event stream (doc 05 §5.3 — "`--json` emits
/// every event").
#[test]
fn machine_mode_eof_during_play_tears_down() {
    let open = format!(
        "{{\"requestId\": 1, \"cmd\": \"session.open\", \"args\": {{\"source\": \
         {{\"kind\": \"file\", \"path\": \"{parent}\"}}, \
         \"options\": {{\"startPaused\": true}}}}}}\n",
        parent = parent()
    );
    let (out, err, code) = run(&["--control-stdio", "--json"], &open);
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(err.is_empty(), "a clean teardown is silent: {err}");
    assert!(out.contains("\"kind\":\"session.state-changed\""), "{out}");
    assert!(out.contains("\"to\":\"stopped\""), "{out}");
    assert!(out.contains("\"to\":\"idle\""), "{out}");
    // The teardown is the stream's tail: the `→ idle` event is the
    // last NDJSON line.
    let last = out.lines().last().must();
    assert!(
        last.contains("\"to\":\"idle\""),
        "the close's event closes the stream:\n{out}"
    );
}

/// The teardown-refusal scenario (M17 AC (b)): EOF while the session
/// still sits in `Buffering` — machine mode answers the open at its
/// receipt (no settle), and the engine's v1 table refuses the close
/// from the transient state (card 3.C1 owns active-close). The exit is
/// still bounded and clean: the typed refusal lands on stderr, the
/// process answers 0. `wait_with_output` returning (and these
/// assertions running) is itself the no-hang proof.
///
/// W2-17's re-drive note: the round-0 refusal pinned here is still the
/// honest FIRST answer (the settle window and second drive happen
/// after it — see the neighboring
/// `machine_mode_eof_transient_refusal_settles_then_tears_down`, which
/// pins that the re-drive then lands the teardown instead of leaking
/// the session). The assertions below are deliberately unchanged from
/// the pre-re-drive pin.
#[test]
fn machine_mode_eof_refusal_is_bounded_and_honest() {
    let open = format!(
        "{{\"requestId\": 1, \"cmd\": \"session.open\", \"args\": {{\"source\": \
         {{\"kind\": \"file\", \"path\": \"{parent}\"}}}}}}\n",
        parent = parent()
    );
    let (out, err, code) = run(&["--control-stdio"], &open);
    assert_eq!(code, 0, "the quit still exits clean:\n{out}\n{err}");
    assert!(err.contains("[session.not-ready]"), "{err}");
    assert!(err.contains("stop-or-close from Buffering"), "{err}");
    assert!(err.contains("card 3.C1"), "{err}");
}

/// W2-17's re-drive pin: the mid-quit `Buffering` refusal no longer
/// LEAKS a session that becomes stoppable pumps later. After the
/// round-0 refusal (pinned above), the exit's bounded settle window
/// lets the fixture prime (`Buffering → Playing`), and the second
/// stop → close drive lands. The event stream carries the whole
/// honest story — the settle's `→ playing`, the re-driven `→ stopped`,
/// the close's `→ idle` as the stream's tail — and the engine holds
/// no session at the exit.
#[test]
fn machine_mode_eof_transient_refusal_settles_then_tears_down() {
    let open = format!(
        "{{\"requestId\": 1, \"cmd\": \"session.open\", \"args\": {{\"source\": \
         {{\"kind\": \"file\", \"path\": \"{parent}\"}}}}}}\n",
        parent = parent()
    );
    let (out, err, code) = run(&["--control-stdio", "--json"], &open);
    assert_eq!(code, 0, "the quit still exits clean:\n{out}\n{err}");
    // Round 0's refusal is still the honest first answer — the
    // re-drive composes on top of it, it does not hide it.
    assert!(err.contains("[session.not-ready]"), "{err}");
    assert!(err.contains("stop-or-close from Buffering"), "{err}");
    assert!(err.contains("card 3.C1"), "{err}");
    // The settle window primes the fixture …
    assert!(out.contains("\"to\":\"playing\""), "{out}");
    // … and the second drive lands the teardown — no leaked session.
    assert!(out.contains("\"to\":\"stopped\""), "{out}");
    let last = out.lines().last().must();
    assert!(
        last.contains("\"to\":\"idle\""),
        "the re-driven close's event closes the stream:\n{out}"
    );
}

/// W2-17's attack regression: a malformed machine request mid-session
/// must not abandon the live session — the teardown runs BEFORE the
/// usage exit (previously the `Paused` session was dropped with exit
/// 64 and no `Stopped`/`Idle` trail; the serialization-failure twin at
/// exit 70 has the same cure in the runner).
#[test]
fn machine_mode_malformed_request_still_tears_down() {
    let open = format!(
        "{{\"requestId\": 1, \"cmd\": \"session.open\", \"args\": {{\"source\": \
         {{\"kind\": \"file\", \"path\": \"{parent}\"}}, \
         \"options\": {{\"startPaused\": true}}}}}}\n",
        parent = parent()
    );
    let (out, err, code) = run(&["--control-stdio", "--json"], &format!("{open}not json\n"));
    assert_eq!(
        code, 64,
        "the usage error is still the answer:\n{out}\n{err}"
    );
    assert!(err.contains("[control.invalid-argument]"), "{err}");
    let last = out.lines().last().must();
    assert!(
        last.contains("\"to\":\"idle\""),
        "the teardown still closes the stream — no abandonment:\n{out}"
    );
}
