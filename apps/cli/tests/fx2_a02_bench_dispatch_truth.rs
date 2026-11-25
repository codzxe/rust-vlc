//! FX2-A02 regression tests: `decode-bench` measures through THE
//! decoder dispatch (`pipeline_host`'s `HostDecoderProvider` — the
//! one registry `lumen open` wires), so the instrument reports real
//! counts for the codec families the build ships. The audit's
//! executed lie (W13-A07 / F-001): the hand-coded first-vertical
//! registry answered an all-zero BENCH line + "no decoder in this
//! build (first vertical: mjpeg + pcm)" + exit 0 for the very file
//! the same binary plays. These tests pin both halves of the fix:
//! the file the player plays now MEASURES (non-zero counts), and an
//! unsupported codec family is the dispatch's own honest typed
//! refusal — never a success-shaped zero line claiming the build
//! lacks decoders it has.

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

/// F0.2's acceptance, as code: `decode-bench` on the committed
/// h264+aac MP4 — the file the same binary plays through the
/// dispatch — reports NON-ZERO counts on both limbs (the audit's
/// reproduction answered v_frames=0 while `lumen open` played the
/// file). The coverage note names both limbs "decoding"; the
/// "first vertical" framing is dead.
#[test]
fn fx2_a02_h264_aac_benches_nonzero_through_the_dispatch() {
    let path = fixture("generated/valid/mp4_h264_aac.mp4");
    let (out, err, code) = run_bench(&[&path]);
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    let lines: Vec<&str> = out.lines().filter(|line| !line.is_empty()).collect();
    assert_eq!(lines.len(), 1, "stdout is the one BENCH line:\n{out}");
    let line = lines[0];
    assert!(line.starts_with("BENCH file="), "{line}");
    assert!(line.contains(&format!("file={path}")), "{line}");
    let fields = bench_fields(line);
    // What the binary plays, the bench measures — both limbs real.
    assert!(
        number(&fields, "v_frames") > 0.0,
        "h264 packets benched (the old bench answered 0): {line}"
    );
    assert!(
        number(&fields, "v_decoded") > 0.0,
        "h264 frames decoded (the old bench answered 0): {line}"
    );
    assert!(
        number(&fields, "a_frames") > 0.0,
        "aac packets benched (the old bench answered 0): {line}"
    );
    assert!(
        number(&fields, "a_decoded") > 0.0,
        "aac frames decoded (the old bench answered 0): {line}"
    );
    assert!(
        number(&fields, "decode_s") <= number(&fields, "wall_s") + 0.002,
        "decode loop inside the process wall: {line}"
    );
    // The per-stream coverage note rides stderr, naming the limbs it
    // is decoding — and the "first vertical" lie is gone (the build
    // decodes 14 families; the note may not claim otherwise).
    assert!(err.contains("h264 — decoding"), "coverage note:\n{err}");
    assert!(err.contains("aac — decoding"), "coverage note:\n{err}");
    assert!(
        !err.contains("first vertical"),
        "the first-vertical lie is dead:\n{err}"
    );
}

/// An unsupported codec family (h263 — no dispatch arm) answers the
/// dispatch's own honest typed refusal: exit 65, the sentence names
/// the missing codec and the landed inventory, no BENCH line —
/// never the old shape (all-zero success + "no decoder in this
/// build" while the same binary decodes 14 families).
#[test]
fn fx2_a02_unsupported_codec_is_the_honest_dispatch_refusal() {
    let path = fixture("generated/valid/mp4_3gp_h263.3gp");
    let (out, err, code) = run_bench(&[&path]);
    assert_eq!(code, 65, "stdout:\n{out}\nstderr:\n{err}");
    assert!(!out.contains("BENCH"), "no fabricated zero line:\n{out}");
    assert!(
        err.contains("no decoder for `h263`"),
        "the dispatch's own refusal names the codec:\n{err}"
    );
    assert!(
        err.contains("landed"),
        "the refusal names what the build HAS:\n{err}"
    );
    assert!(
        !err.contains("first vertical"),
        "the first-vertical lie is dead:\n{err}"
    );
}
