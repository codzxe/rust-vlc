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
/// `info` snapshot asserted between the steps (the card's AC). The
/// seek is the engine's NATIVE flow (C8/ED-FIX-4): it lands
/// in-pipeline — the session epoch, the device, the ring and the
/// renderer surface all survive — and the stop→close→reopen
/// composition is nowhere in the trail (D-4/D-7: forbidden).
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

    // THE SEEK (the engine's native flow): from Paused the wire state
    // self-transitions (the decode carve-out keeps the wire paused),
    // the SAME session answers under a minted generation, and the
    // terminal reports the ACTUAL landing — 1.000000s, exactly half
    // of the 2.000s media (a frame-aligned target lands on itself).
    let seek_at = out.find("state-changed Paused → Paused").must();
    assert!(seek_at > info_at, "seek follows the info snapshot:\n{out}");
    assert!(
        out.contains(
            "event: TimingSeekCompleted { position: Timestamp(1000000), \
             seek_kind: Command }"
        ),
        "{out}"
    );
    assert!(out.contains("ok: accepted session=1 gen=1"), "{out}");
    // Epoch continuity — the no-reopen proof: exactly one open in the
    // whole transcript (the initial one; a reopen answers per seek),
    // and no second session id is ever minted.
    assert_eq!(
        out.matches("state-changed Idle → Opening").count(),
        1,
        "one open only — no reopen:\n{out}"
    );
    assert!(
        !out.contains("accepted session=2"),
        "the seek must not mint a session:\n{out}"
    );

    // stop tears the pipeline down (the session survives) — from the
    // seek's Paused, AFTER the seek's receipt.
    let stop_at = out.find("state-changed Paused → Stopped").must();
    assert!(
        stop_at > seek_at,
        "the stop follows the seek (it is not the seek):\n{out}"
    );
    assert_eq!(
        out.matches("state-changed Paused → Stopped").count(),
        1,
        "the seek itself never stops:\n{out}"
    );

    // time after the 50% seek + stop: the clock froze at the seek's
    // ACTUAL landing — exactly half of 2.000 s (pipeline-only settles
    // never advance media time; only `advance` does).
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
// The native seek (C8/ED-FIX-4): the engine's in-pipeline flow through
// the real binary — epoch continuity, the ACTUAL landing, typed
// refusals, the backward-below-start-time composition
// ---------------------------------------------------------------------------

/// A seek from `Playing` re-buffers IN-PIPELINE: `Playing → Buffering
/// → Playing` around the terminal, one open in the transcript, and
/// the position reports the demuxer's actual landing.
#[test]
fn seek_from_playing_rebuffers_in_pipeline() {
    let script = "seek 1500ms\ntime\nquit\n";
    let (out, err, code) = run(&["open", &parent()], script);
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(err.is_empty(), "stderr:\n{err}");
    // The rebuffer episode around the landing (doc 03 §6 step 5 — no
    // fast-start heuristic in v0; the watermark machinery re-enters
    // Playing through the existing transitions).
    let seek_at = out.find("state-changed Playing → Buffering").must();
    assert!(
        out.contains(
            "event: TimingSeekCompleted { position: Timestamp(1500000), \
             seek_kind: Command }"
        ),
        "{out}"
    );
    // The rebuffer's close comes AFTER the seek's own `→ Buffering`
    // (the open's earlier `Buffering → Playing` is not it).
    let resumed = seek_at
        + out[seek_at..]
            .find("state-changed Buffering → Playing")
            .must();
    assert!(resumed > seek_at, "the rebuffer closes:\n{out}");
    // The same session, minted generation, no reopen.
    assert!(out.contains("ok: accepted session=1 gen=1"), "{out}");
    assert_eq!(
        out.matches("state-changed Idle → Opening").count(),
        1,
        "one open only:\n{out}"
    );
    assert!(out.contains("ok: time 1.500000s (75.0%)"), "{out}");
}

/// Position truth is the ACTUAL landing, not the requested target:
/// `seek 250ms` on the 30 fps grid lands at the demuxer's
/// keyframe-safe chunk — 266.667 ms — and the `time` snapshot reports
/// that, never the request.
#[test]
fn seek_reports_the_actual_not_the_requested() {
    let script = "seek 250ms\ntime\nquit\n";
    let (out, err, code) = run(&["open", &parent()], script);
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(err.is_empty(), "stderr:\n{err}");
    assert!(
        out.contains(
            "event: TimingSeekCompleted { position: Timestamp(266667), \
             seek_kind: Command }"
        ),
        "the terminal carries the demuxer's landing:\n{out}"
    );
    assert!(
        out.contains("ok: time 0.266666s (13.3%)"),
        "the snapshot reports the actual (ns → µs truncation):\n{out}"
    );
}

/// A typed failure surfaces honestly (contract D4): seek from `Ended`
/// is the state table's refusal — `[session.not-ready]` on stderr
/// naming the owning wire edge, the position unchanged at the old
/// one, and nothing torn down to refuse.
#[test]
fn seek_from_ended_refuses_typed() {
    let script = "advance 512\nseek 50%\ntime\nquit\n";
    let (out, err, code) = run(&["open", &parent()], script);
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(out.contains("state-changed Playing → Ended"), "{out}");
    assert!(err.contains("[session.not-ready]"), "{err}");
    assert!(err.contains("seek in Ended"), "{err}");
    // The old position stands (nothing moved, nothing died).
    assert!(out.contains("ok: time 2.000000s (100.0%)"), "{out}");
    assert!(
        !out.contains("TimingSeekCompleted"),
        "no terminal was fabricated:\n{out}"
    );
}

/// A backward seek below `--start-time` lands and plays out (the
/// demuxer wrapper clears the open-time skip watermark at the
/// re-anchor — the landing is the new position truth; a standing
/// `1s` filter would starve the pre-roll of a 250 ms landing).
#[test]
fn backward_seek_below_start_time_lands() {
    let script = "seek 250ms\ntime\nadvance 512\ntime\nquit\n";
    let (out, err, code) = run(&["open", &parent(), "--start-time", "1s"], script);
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(err.is_empty(), "stderr:\n{err}");
    assert!(
        out.contains(
            "event: TimingSeekCompleted { position: Timestamp(266667), \
             seek_kind: Command }"
        ),
        "{out}"
    );
    assert!(out.contains("ok: time 0.266666s (13.3%)"), "{out}");
    // The pre-roll was fed below the start time: the media still
    // plays to its exact end.
    assert!(out.contains("event: end-of-media"), "{out}");
    assert!(out.contains("ok: time 2.000000s (100.0%)"), "{out}");
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
