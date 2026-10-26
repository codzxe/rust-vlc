//! The card 2.D2 error paths, through the REAL binary: every failure
//! is a stage-tagged typed error on stderr plus a sysexits-class exit
//! code (the honest-printing AC); nothing panics, nothing is silent.

use std::io::Write;
use std::process::{Command, Stdio};

use testkit::Must;

fn fixture(relative: &str) -> String {
    format!(
        "{}/../../tests/fixtures/{relative}",
        env!("CARGO_MANIFEST_DIR")
    )
}

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
        // A failing fast-open exits before consuming stdin — a closed
        // pipe here is the expected shape, not a harness failure.
        if let Err(err) = stdin.write_all(script.as_bytes()) {
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

/// Pure garbage: the probe's typed unknown — stage `probe`, code
/// `unsupported`, exit 65 (EX_DATAERR), the sniffed bytes named.
#[test]
fn garbage_file_is_a_typed_probe_refusal() {
    let garbage = std::env::temp_dir().join("lumen-cli-garbage.bin");
    std::fs::write(&garbage, [0x37u8, 0x13, 0x00, 0x42, 0xff, 0xee].repeat(64)).must();
    let (out, err, code) = run(&["open", garbage.to_str().must()], "quit\n");
    std::fs::remove_file(&garbage).ok();
    assert_eq!(code, 65, "stdout:\n{out}\nstderr:\n{err}");
    assert!(err.contains("error: [probe.unsupported]"), "{err}");
    assert!(err.contains("probe.no-demuxer"), "{err}");
    assert!(
        !out.contains("ok:"),
        "no success line on a failed open:\n{out}"
    );
}

/// A magic-broken class-B derivative (flip16 breaks RIFF[2]): the
/// same typed probe refusal, naming the sniffed bytes.
#[test]
fn magic_broken_derivative_is_a_typed_probe_refusal() {
    let (out, err, code) = run(
        &[&fixture("generated/malformed/avi_mjpeg_pcm__flip16.avi")],
        "quit\n",
    );
    let (_out, err, code) = {
        let (_o, e, c) = (out, err, code);
        (String::new(), e, c)
    };
    assert_eq!(code, 65, "stderr:\n{_out}{err}");
    assert!(err.contains("error: [probe.unsupported]"), "{err}");
}

/// A missing file: the input-stage inaccessible truth (exit 66).
#[test]
fn missing_file_is_inaccessible() {
    let (_out, err, code) = run(&[&fixture("generated/valid/no-such-file.avi")], "quit\n");
    assert_eq!(code, 66, "stderr:\n{err}");
    assert!(err.contains("error: [input.inaccessible]"), "{err}");
}

/// A directory as the source: the honest inaccessible refusal too.
#[test]
fn directory_source_is_inaccessible() {
    let (_out, err, code) = run(&[&fixture("generated/valid")], "quit\n");
    assert_eq!(code, 66, "stderr:\n{err}");
    assert!(err.contains("error: [input.inaccessible]"), "{err}");
}

/// The MPEG-PS parent family is now WIRED (card 6.OPW-ps — the
/// FINAL §5 unit: all 15 registered probe signature ids have
/// registry arms), so the registry's default refusal is no longer
/// reachable with real bytes — every signature the probe can emit
/// resolves to a container host. This pin re-targets from "the
/// registry refuses mpegps" (the pre-wiring truth) to the wired
/// family's own honest answer for a 4-byte stub: the pack start
/// code probes `mpegps`, the registry dispatches to the PS host, and
/// the DEMUXER refuses the truncated pack header typed (stage
/// `demux`, code `demux.ps.bad-pack`, exit 65) — proof the arm
/// answers. The default arm's coverage (a hypothetical future
/// signature id, e.g. `mpegps-dv-extended`) is pinned in the lib
/// test `unwired_format_is_refused_with_the_format_named` — no real
/// file can reach the default arm while every registered signature
/// is wired.
#[test]
fn mpegps_stub_is_refused_typed_by_the_wired_demuxer() {
    let ps = [0x00u8, 0x00, 0x01, 0xBA];
    let ps_path = std::env::temp_dir().join("lumen-cli-wired-ps.mpg");
    std::fs::write(&ps_path, ps).must();
    let (_out, err, code) = run(&[ps_path.to_str().must()], "quit\n");
    std::fs::remove_file(&ps_path).ok();
    assert_eq!(code, 65, "stderr:\n{err}");
    assert!(err.contains("error: [demux.corrupt]"), "{err}");
    assert!(err.contains("demux.ps.bad-pack"), "{err}");
}

/// The wired MPEG-PS family through the REAL binary (the family's
/// first end-to-end lane): the committed demux fixture (mpeg2video +
/// mp2) opens; with `--no-video` the mp2 limb decodes (the streaming
/// AU-split: 39 frames from 5 PES) and the session accepts.
#[test]
fn mpegps_container_opens_and_accepts_the_session() {
    let path = format!(
        "{}/../../crates/demux/fixtures/ps/ps-mp2-video-audio.mpg",
        env!("CARGO_MANIFEST_DIR")
    );
    let (out, err, code) = run(&["open", &path, "--no-video"], "quit\n");
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(out.contains("ok: accepted session=1"), "{out}");
}

/// The Matroska parent fixture now OPENS end-to-end (card
/// 6.OPW-mkv wired the family — the refusal this test used to pin
/// is the re-targeted one above): the real binary demuxes the
/// h264+aac fixture, and the h264 track's codec refusal surfaces
/// typed and named (the engine's honest open answer for a session
/// whose video limb cannot decode — exit 65). With `--no-video`
/// the deselected session accepts on the aac limb (exit 0).
#[test]
fn matroska_container_opens_and_the_h264_refusal_is_typed() {
    let (out, err, code) = run(&[&fixture("generated/valid/mkv_h264_aac.mkv")], "quit\n");
    assert_eq!(code, 65, "stdout:\n{out}\nstderr:\n{err}");
    assert!(out.contains("event: error [decode.unsupported]"), "{out}");
    assert!(err.contains("no decoder for `h264`"), "{err}");
    assert!(err.contains("cli.codec_unavailable"), "{err}");
    let (out, err, code) = run(
        &[
            "open",
            &fixture("generated/valid/mkv_h264_aac.mkv"),
            "--no-video",
        ],
        "quit\n",
    );
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(out.contains("ok: accepted session=1"), "{out}");
}

/// The MP4 parent fixture now OPENS end-to-end (card 6.OPW-mp4
/// wired the family — the refusal this test used to pin is the
/// re-targeted one above): the real binary opens the mjpeg+PCM
/// fixture and accepts the session.
#[test]
fn mp4_container_opens_and_accepts_the_session() {
    let (out, err, code) = run(&[&fixture("generated/valid/mp4_mjpeg_pcm.mp4")], "quit\n");
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(out.contains("ok: accepted session=1"), "{out}");
}

/// The AV1-in-IVF lane OPENS end-to-end (card 6.OPW-ivf wired the
/// family): the real binary opens the demux crate's committed AV1
/// fixture (a decodable track — the provider's rav1d-backed arm) and
/// accepts the session.
#[test]
fn ivf_container_opens_and_accepts_the_session() {
    let (out, err, code) = run(&[&fixture("generated/ivf/av1_64x48_30fps.ivf")], "quit\n");
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(out.contains("ok: accepted session=1"), "{out}");
}

/// URL sources: the honest host-capability refusal (exit 69).
#[test]
fn url_source_is_unsupported_for_host() {
    let (_out, err, code) = run(&["open", "https://example.org/x.mp4"], "quit\n");
    assert_eq!(code, 69, "stderr:\n{err}");
    assert!(err.contains("error: [input.unsupported-for-host]"), "{err}");
    assert!(err.contains("Phase 6"), "{err}");
}

/// A class-A command with no session: the honest not-ready (69).
#[test]
fn play_without_session_is_not_ready() {
    let (_out, err, code) = run(&["play"], "");
    assert_eq!(code, 69, "stderr:\n{err}");
    assert!(err.contains("error: [session.not-ready]"), "{err}");
}

/// An unserved catalog surface (rate): the typed host refusal with
/// the owning card named (69).
#[test]
fn unserved_command_is_typed_with_the_owner() {
    let (_out, err, code) = run(
        &["--control-stdio"],
        "{\"requestId\": 1, \"cmd\": \"playback.set-rate\", \"args\": {\"rate\": 2.0}}\n",
    );
    assert_eq!(code, 0, "machine mode answers, then EOF:\n{err}");
    // (the response line itself carries the refusal)
    let (out, _err, _code) = run(
        &["--control-stdio"],
        "{\"requestId\": 1, \"cmd\": \"playback.set-rate\", \"args\": {\"rate\": 2.0}}\n",
    );
    assert!(out.contains("unsupported-for-host"), "{out}");
    assert!(out.contains("card 3.A1"), "{out}");
}

/// Grammar failures: usage errors (exit 64), the offending token in
/// the message, the hint beneath.
#[test]
fn grammar_failures_are_usage_errors() {
    let (_out, err, code) = run(&["seek", "bogus"], "");
    assert_eq!(code, 64, "stderr:\n{err}");
    assert!(err.contains("bad time `bogus`"), "{err}");
    let (_out, err, code) = run(&["seek", "101%"], "");
    assert_eq!(code, 64, "stderr:\n{err}");
    assert!(err.contains("outside 0..100"), "{err}");
    let (_out, err, code) = run(&["open"], "");
    assert_eq!(code, 64, "stderr:\n{err}");
    assert!(err.contains("open needs a source path"), "{err}");
    let (_out, err, code) = run(&["no-such-verb"], "");
    assert_eq!(code, 64, "stderr:\n{err}");
    assert!(err.contains("unknown command `no-such-verb`"), "{err}");
}

/// Redaction (V8): a user-path-bearing error collapses the path to
/// its final component — no full user paths on the error channel.
#[test]
fn error_messages_redact_user_paths() {
    let (_out, err, code) = run(&["open", "/home/someone/secret/missing.avi"], "quit\n");
    assert_eq!(code, 66, "stderr:\n{err}");
    assert!(err.contains("…/missing.avi"), "{err}");
    assert!(!err.contains("/home/someone"), "{err}");
}
