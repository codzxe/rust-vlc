//! FX4-A06 regression tests (wave FX4, FIX-PLAN §3 F1.1): the
//! invocation-level audio flag surface — `--audio-backend <null|auto>`
//! and `--no-audio` — parsed at the argv level, recorded on
//! [`lumen_cli::parse::Invocation`], and told honestly by `--help`
//! (W11's S0 class, executed: the help advertised a flag surface the
//! parser rejects — the advertised surface must be the parser's real
//! one, never more). In-process grammar tests (the fx1_a05
//! convention) plus binary-level probes of the shipped argv refusal
//! lane and the `--help` output (the command_surface.rs convention).

use std::io::Write;
use std::process::{Command, Stdio};

use lumen_cli::parse::{AudioBackend, CliCommand, parse_argv, parse_command};
use testkit::Must;

/// One whitespace-split word list (the parse.rs suite's own helper).
fn words(line: &str) -> Vec<String> {
    line.split_whitespace().map(str::to_string).collect()
}

/// The shipped binary, argv only: the parse-refusal / `--help` lanes
/// never read stdin, so an empty script is written and the pipe is
/// left to close (command_surface.rs's `run` shape; `wait_with_output`
/// drops the stdin handle before waiting, so the child cannot block).
fn run(args: &[&str]) -> (String, String, i32) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lumen"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .must();
    {
        let stdin = child.stdin.as_mut().must();
        stdin.write_all(b"").must();
    }
    let output = child.wait_with_output().must();
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().unwrap_or(-1),
    )
}

/// Both honest values parse and the choice is recorded on the
/// [`lumen_cli::parse::Invocation`] — `null`, `auto`, the
/// position-independent and the bare-source forms — and an absent flag
/// is the default posture (`None`), never a guessed `null`.
#[test]
fn fx4_a06_audio_backend_values_parse_onto_invocation() {
    for (line, backend) in [
        ("--audio-backend null open clip.avi", AudioBackend::Null),
        ("--audio-backend auto open clip.avi", AudioBackend::Auto),
    ] {
        let invocation = match parse_argv(&words(line)) {
            Ok(invocation) => invocation,
            Err(failure) => unreachable!("{line} parses: {}", failure.message),
        };
        assert_eq!(invocation.audio_backend, Some(backend), "{line}");
        match invocation.command {
            Some(CliCommand::Open {
                source, no_audio, ..
            }) => {
                assert_eq!(source, "clip.avi", "{line}");
                assert!(!no_audio, "{line}");
            }
            other => unreachable!("{line} open parse: {other:?}"),
        }
    }
    // The bare-source form (VLC-style `lumen clip.avi`): the global
    // flag's value consumption never eats the media path.
    let invocation = match parse_argv(&words("--audio-backend null clip.avi")) {
        Ok(invocation) => invocation,
        Err(failure) => unreachable!("bare source parses: {}", failure.message),
    };
    assert_eq!(invocation.audio_backend, Some(AudioBackend::Null));
    match invocation.command {
        Some(CliCommand::Open { source, .. }) => assert_eq!(source, "clip.avi"),
        other => unreachable!("bare source parse: {other:?}"),
    }
    // Global flags are position-independent (parse_argv's own lane):
    // the flag after the verb is still the invocation-level one.
    let invocation = match parse_argv(&words("open clip.avi --audio-backend auto")) {
        Ok(invocation) => invocation,
        Err(failure) => unreachable!("post-verb flag parses: {}", failure.message),
    };
    assert_eq!(invocation.audio_backend, Some(AudioBackend::Auto));
    // Absent = the default posture (None), not a guessed null.
    let invocation = match parse_argv(&words("open clip.avi")) {
        Ok(invocation) => invocation,
        Err(failure) => unreachable!("plain open parses: {}", failure.message),
    };
    assert_eq!(invocation.audio_backend, None);
    assert!(!invocation.no_audio);
}

/// A missing or unknown `--audio-backend` value is the grammar's typed
/// refusal — a message naming the offending token plus the usage hint
/// naming the two honest values — through the library grammar AND the
/// shipped argv lane (exit 64 EX_USAGE, run.rs's parse-error printer).
#[test]
fn fx4_a06_audio_backend_bad_value_is_typed_refusal() {
    // The library grammar: exact message + exact hint.
    let failure = match parse_argv(&words("--audio-backend")) {
        Err(failure) => failure,
        Ok(invocation) => unreachable!("missing value must refuse: {invocation:?}"),
    };
    assert_eq!(failure.message, "--audio-backend needs a backend argument");
    assert_eq!(failure.hint, "e.g. --audio-backend null | auto");
    let failure = match parse_argv(&words("--audio-backend cpal open clip.avi")) {
        Err(failure) => failure,
        Ok(invocation) => unreachable!("unknown value must refuse: {invocation:?}"),
    };
    assert_eq!(failure.message, "bad --audio-backend `cpal`");
    assert_eq!(failure.hint, "e.g. --audio-backend null | auto");
    // A flag-shaped token is an unknown VALUE, not a swallowed flag.
    let failure = match parse_argv(&words("--audio-backend --json")) {
        Err(failure) => failure,
        Ok(invocation) => unreachable!("flag-as-value must refuse: {invocation:?}"),
    };
    assert_eq!(failure.message, "bad --audio-backend `--json`");
    // The shipped argv lane prints the same typed refusal, exit 64.
    for (args, message) in [
        (
            &["--audio-backend", "cpal"][..],
            "error: bad --audio-backend `cpal`",
        ),
        (
            &["--audio-backend"][..],
            "error: --audio-backend needs a backend argument",
        ),
    ] {
        let (out, err, code) = run(args);
        assert_eq!(code, 64, "stdout:\n{out}\nstderr:\n{err}");
        assert!(err.contains(message), "{err}");
        assert!(err.contains("e.g. --audio-backend null | auto"), "{err}");
    }
}

/// The `--no-audio` posture: the argv and per-command forms are ONE
/// choice — the global token is folded into the argv `open`'s own
/// `--no-audio` (the pipeline-construction knob the command already
/// threads), the REPL lane sets it in `parse_open`'s own arm, and a
/// non-open command records the posture with nothing to apply it to.
#[test]
fn fx4_a06_no_audio_posture_folds_into_the_open_command() {
    for line in [
        "--no-audio open clip.avi",
        "open clip.avi --no-audio",
        "--no-audio open clip.avi --no-audio",
    ] {
        let invocation = match parse_argv(&words(line)) {
            Ok(invocation) => invocation,
            Err(failure) => unreachable!("{line} parses: {}", failure.message),
        };
        assert!(invocation.no_audio, "{line}");
        match invocation.command {
            Some(CliCommand::Open { no_audio, .. }) => assert!(no_audio, "{line}"),
            other => unreachable!("{line} open parse: {other:?}"),
        }
    }
    // The REPL lane (parse_command): the open's own flag arm — the
    // per-command form the catalog synopsis advertises.
    match parse_command(&words("open clip.avi --no-audio")) {
        Ok(CliCommand::Open { no_audio, .. }) => assert!(no_audio),
        other => unreachable!("REPL open parse: {other:?}"),
    }
    // A non-open argv command: the posture is recorded, the command
    // survives untouched (nothing to apply the choice to).
    let invocation = match parse_argv(&words("--no-audio info")) {
        Ok(invocation) => invocation,
        Err(failure) => unreachable!("info parses: {}", failure.message),
    };
    assert!(invocation.no_audio);
    assert!(matches!(invocation.command, Some(CliCommand::Info)));
}

/// The `--help` text tells the honest backend story, through the
/// shipped binary: the advertised value set is the parser's real one
/// (`null|auto`), `auto`'s real-device preference is gated on the
/// named `backend-cpal` build feature with the instrumented null
/// fallback (this build carries none), `null` forces the null backend
/// (samples counted — the session clock's truth source — and
/// discarded), `--no-audio` skips the audio path (wall-time clock),
/// and the reserved-flags line lists exactly both new flags.
#[test]
fn fx4_a06_help_tells_the_honest_backend_story() {
    let (out, err, code) = run(&["--help"]);
    assert_eq!(code, 0, "stderr:\n{err}");
    // The advertised value set IS the parser's accepted set.
    assert!(out.contains("--audio-backend <null|auto>"), "{out}");
    // `auto`: real device only behind the named feature, null fallback.
    assert!(
        out.contains("real device backend when this build carries one"),
        "{out}"
    );
    assert!(out.contains("backend-cpal"), "{out}");
    assert!(out.contains("falls back to the instrumented null"), "{out}");
    // `null`: forced null backend, samples counted and discarded.
    assert!(
        out.contains("forces the instrumented null backend"),
        "{out}"
    );
    assert!(
        out.contains("the sample counter is the session clock's"),
        "{out}"
    );
    // `--no-audio`: no audio path, the wall-time session clock.
    assert!(
        out.contains("skip the audio path entirely: no audio"),
        "{out}"
    );
    assert!(out.contains("the session clock runs"), "{out}");
    assert!(out.contains("on wall time."), "{out}");
    // The reserved-flags line carries both flags, no more.
    assert!(
        out.contains("--control-stdio, --audio-backend <null|auto>, --no-audio."),
        "{out}"
    );
    // The generated half still rides after the inserted section.
    assert!(out.contains("Areas:"), "{out}");
}
