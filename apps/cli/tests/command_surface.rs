//! The command-surface AC through the REAL binary: `--help` (the
//! schema-generated text + the catalog-sourced synopsis), `--version`,
//! the catalog introspection, and the machine mode's envelope shape.

use std::io::Write;
use std::process::{Command, Stdio};

use testkit::Must;

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

/// The full `--help`: version header, the usage lines, the command
/// synopsis with catalog ids, and the settings-schema's generated
/// areas index (the card's "--help generated from settings-schema").
#[test]
fn help_lists_commands_and_generated_areas() {
    let (out, err, code) = run(&["--help"], "");
    assert_eq!(code, 0, "stderr:\n{err}");
    assert!(out.starts_with("lumen 0."), "{out}");
    assert!(out.contains("lumen --control-stdio"), "{out}");
    // The synopsis rows (verb → catalog id).
    assert!(out.contains("session.open"), "{out}");
    assert!(out.contains("playback.seek"), "{out}");
    assert!(out.contains("media.get-info"), "{out}");
    assert!(out.contains("config.get-setting / config.set-setting"), "{out}");
    // The generated settings areas (G2 over the embedded schema).
    assert!(out.contains("Areas:"), "{out}");
    for area in ["engine", "global", "interface", "performance", "playback", "privacy"] {
        assert!(out.contains(area), "area {area} missing:\n{out}");
    }
}

/// One settings area's generated section (the VLC --longhelp
/// pattern): the engine area's flags with their schema defaults.
#[test]
fn help_area_prints_the_generated_flags() {
    let (out, err, code) = run(&["--help", "engine"], "");
    assert_eq!(code, 0, "stderr:\n{err}");
    assert!(out.contains("--engine-log-level <error|warn|info|debug|trace>"), "{out}");
    assert!(out.contains("settings.engine.log-level"), "{out}");
    // An unknown area is a usage error naming the known set.
    let (_out, err, code) = run(&["--help", "no-such-area"], "");
    assert_eq!(code, 64, "stderr:\n{err}");
    assert!(err.contains("known areas:"), "{err}");
}

/// `--version`.
#[test]
fn version_prints_the_crate_version() {
    let (out, err, code) = run(&["--version"], "");
    assert_eq!(code, 0, "stderr:\n{err}");
    assert_eq!(out.trim_end(), format!("lumen {}", env!("CARGO_PKG_VERSION")));
}

/// The catalog introspection: all 61 ids with their classes; every
/// id the dispatch table serves is present.
#[test]
fn commands_lists_the_whole_catalog() {
    let (out, err, code) = run(&["commands"], "");
    assert_eq!(code, 0, "stderr:\n{err}");
    assert!(out.contains("ok: 61 command(s)"), "{out}");
    for id in [
        "session.open",
        "session.close",
        "playback.play",
        "playback.pause",
        "playback.toggle-play-pause",
        "playback.stop",
        "playback.seek",
        "media.get-info",
        "media.get-current-time",
        "media.get-duration",
        "tracks.list",
        "system.get-engine-info",
        "system.list-commands",
        "config.get-setting",
        "config.set-setting",
    ] {
        assert!(out.contains(id), "served id {id} missing:\n{out}");
    }
    // Class letters ride every row.
    assert!(out.contains("session.open [a]"), "{out}");
    assert!(out.contains("media.get-info [s]"), "{out}");
}

/// The engine self-description names the first vertical's honest
/// sink decisions (null audio device, software compositor).
#[test]
fn engine_info_names_the_vertical() {
    let (out, err, code) = run(&["engine-info"], "");
    assert_eq!(code, 0, "stderr:\n{err}");
    assert!(out.contains("ok: engine"), "{out}");
    assert!(out.contains("host: cli"), "{out}");
    assert!(out.contains("first-vertical:avi-mjpeg-pcm"), "{out}");
    assert!(out.contains("audio.null-device"), "{out}");
    assert!(out.contains("video.software-compositor"), "{out}");
    assert!(out.contains("mjpeg, pcm_s16le"), "{out}");
}

/// The machine mode: the doc 05 §5.1 envelope, byte-shaped —
/// requestId echoed, camelCase, the adjacent cmd/args tagging.
#[test]
fn control_stdio_speaks_the_wire_envelope() {
    let stdin = "{\"requestId\": 42, \"cmd\": \"playback.seek\", \"args\": {\"target\": {\"kind\": \"relative\", \"deltaMs\": -10000}}}\n";
    let (out, err, code) = run(&["--control-stdio"], stdin);
    assert_eq!(code, 0, "stderr:\n{err}");
    let line = out.lines().next().must();
    assert!(line.contains("\"requestId\":42"), "{line}");
    // A seek with no session is the honest not-ready, in the
    // documented error envelope shape.
    assert!(line.contains("\"error\":"), "{line}");
    assert!(line.contains("\"code\":\"not-ready\""), "{line}");
    assert!(line.contains("\"stage\":\"session\""), "{line}");
}

/// `--json --control-stdio`: the observed events ride the NDJSON
/// stream after the response (doc 05 §5.3's `--json` contract).
#[test]
fn control_stdio_with_json_emits_events() {
    let parent = format!(
        "{}/../../tests/fixtures/generated/valid/avi_mjpeg_pcm.avi",
        env!("CARGO_MANIFEST_DIR")
    );
    let open = format!(
        "{{\"requestId\": 1, \"cmd\": \"session.open\", \"args\": {{\"source\": {{\"kind\": \"file\", \"path\": \"{parent}\"}}}}}}\n"
    );
    let (out, err, code) = run(&["--control-stdio", "--json"], &open);
    assert_eq!(code, 0, "stderr:\n{err}");
    assert!(out.contains("\"requestId\":1"), "{out}");
    assert!(out.contains("\"kind\":\"session.state-changed\""), "{out}");
    assert!(out.contains("\"kind\":\"tracks.list-ready\""), "{out}");
}
