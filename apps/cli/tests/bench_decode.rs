//! The `decode-bench` surface (card W4-R2), through the REAL binary:
//! one machine-parseable BENCH line on stdout and exit 0 for a
//! decodable file; typed stage-tagged errors + sysexits-class exits
//! (never a panic, never a BENCH line) for garbage, missing files and
//! grammar failures — the adversarial-corpus contract.

use std::io::Write;
use std::process::{Command, Stdio};

use testkit::Must;

fn fixture(relative: &str) -> String {
    format!(
        "{}/../../tests/fixtures/{relative}",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// Run `lumen decode-bench <args...>` and collect (stdout, stderr, code).
fn run_bench(args: &[&str]) -> (String, String, i32) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_lumen"));
    cmd.arg("decode-bench")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().must();
    {
        let stdin = child.stdin.as_mut().must();
        // The bench never reads stdin; a closed pipe is the expected
        // shape, not a harness failure.
        if let Err(err) = stdin.write_all(b"") {
            assert_eq!(
                err.kind(),
                std::io::ErrorKind::BrokenPipe,
                "unexpected stdin write failure: {err}"
            );
        }
    }
    let output = child.wait_with_output().must();
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().unwrap_or(-1),
    )
}

/// The one BENCH line, parsed into its key=value fields.
fn bench_fields(line: &str) -> std::collections::HashMap<&str, &str> {
    line.split_whitespace()
        .skip(1)
        .filter_map(|token| token.split_once('='))
        .collect()
}

fn number(fields: &std::collections::HashMap<&str, &str>, key: &str) -> f64 {
    fields
        .get(key)
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(-1.0)
}

/// The happy path over the class-A AVI fixture: exit 0, exactly one
/// BENCH line, every demuxed packet of both streams decoded, the
/// byte count the MANIFEST pins, and the decode loop inside the wall.
#[test]
fn avi_fixture_reports_one_complete_bench_line() {
    let path = fixture("generated/valid/avi_mjpeg_pcm.avi");
    let (out, err, code) = run_bench(&[&path]);
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    let lines: Vec<&str> = out.lines().filter(|line| !line.is_empty()).collect();
    assert_eq!(lines.len(), 1, "stdout is the one BENCH line:\n{out}");
    let line = lines[0];
    assert!(line.starts_with("BENCH file="), "{line}");
    assert!(line.contains(&format!("file={path}")), "{line}");
    let fields = bench_fields(line);
    // The fixture truth (MANIFEST.toml pins size_bytes = 425_516;
    // its structure map counts 120 movi chunks: 60 `00dc` + 60 `01wb`).
    assert_eq!(fields.get("bytes").copied(), Some("425516"), "{line}");
    assert_eq!(fields.get("v_frames").copied(), Some("60"), "{line}");
    assert_eq!(fields.get("v_decoded").copied(), Some("60"), "{line}");
    assert_eq!(fields.get("a_frames").copied(), Some("60"), "{line}");
    assert_eq!(fields.get("a_decoded").copied(), Some("60"), "{line}");
    assert!(number(&fields, "wall_s") >= 0.0, "{line}");
    assert!(
        number(&fields, "decode_s") <= number(&fields, "wall_s") + 0.002,
        "decode loop inside the process wall: {line}"
    );
    // The per-stream coverage note rides stderr, never stdout.
    assert!(err.contains("bench:"), "coverage note on stderr:\n{err}");
}

/// The two-audio-stream fixture: the FIRST audio stream is the benched
/// one (honest first-of-kind coverage), and it decodes completely.
#[test]
fn two_audio_stream_fixture_benches_the_first_stream() {
    let path = fixture("generated/valid/avi_mjpeg_2pcm.avi");
    let (out, err, code) = run_bench(&[&path]);
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    let line = out
        .lines()
        .find(|line| line.starts_with("BENCH "))
        .unwrap_or_default();
    let fields = bench_fields(line);
    assert!(
        number(&fields, "v_frames") > 0.0 && number(&fields, "a_frames") > 0.0,
        "{line}"
    );
    assert_eq!(
        number(&fields, "v_decoded"),
        number(&fields, "v_frames"),
        "{line}"
    );
    assert_eq!(
        number(&fields, "a_decoded"),
        number(&fields, "a_frames"),
        "{line}"
    );
    assert!(
        err.contains("first-of-kind already benched"),
        "the unbenched second audio stream is named:\n{err}"
    );
}

/// Pure garbage: the probe's typed unknown — stage `probe`, code
/// `unsupported`, exit 65 (EX_DATAERR), no BENCH line, no panic.
#[test]
fn garbage_file_is_a_typed_probe_refusal() {
    let garbage = std::env::temp_dir().join("lumen-cli-bench-garbage.bin");
    std::fs::write(&garbage, [0x37u8, 0x13, 0x00, 0x42, 0xff, 0xee].repeat(64)).must();
    let (out, err, code) = run_bench(&[garbage.to_str().must()]);
    std::fs::remove_file(&garbage).ok();
    assert_eq!(code, 65, "stdout:\n{out}\nstderr:\n{err}");
    assert!(err.contains("error: [probe.unsupported]"), "{err}");
    assert!(!out.contains("BENCH"), "no result line on failure:\n{out}");
}

/// A magic-broken class-B derivative (flip16 breaks RIFF[2]): the
/// same typed probe refusal — adversarial AVI-shaped input stays
/// bounded and honest.
#[test]
fn magic_broken_derivative_is_a_typed_probe_refusal() {
    let (out, err, code) = run_bench(&[&fixture("generated/malformed/avi_mjpeg_pcm__flip16.avi")]);
    assert_eq!(code, 65, "stdout:\n{out}\nstderr:\n{err}");
    assert!(err.contains("error: [probe.unsupported]"), "{err}");
    assert!(!out.contains("BENCH"), "{out}");
}

/// A missing file: the input-stage inaccessible truth (exit 66).
#[test]
fn missing_file_is_inaccessible() {
    let (out, err, code) = run_bench(&[&fixture("generated/valid/no-such-file.avi")]);
    assert_eq!(code, 66, "stdout:\n{out}\nstderr:\n{err}");
    assert!(err.contains("error: [input.inaccessible]"), "{err}");
}

/// Grammar failures: usage errors (exit 64) with the honest sentence.
#[test]
fn grammar_failures_are_usage_errors() {
    let (_out, err, code) = run_bench(&[]);
    assert_eq!(code, 64, "stderr:\n{err}");
    assert!(
        err.contains("decode-bench takes exactly one source path"),
        "{err}"
    );
    let (_out, err, code) = run_bench(&["a.avi", "b.avi"]);
    assert_eq!(code, 64, "stderr:\n{err}");
    assert!(err.contains("exactly one source path"), "{err}");
}
