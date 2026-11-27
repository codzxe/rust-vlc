//! F3.1, FX9-A01 — the cataloged-but-refused subtitle verbs (the
//! REALITY-SCORE rows 057-068 class: `tracks.set-subtitle-track` +
//! `subtitle.add-file`/`set-sync`/`set-encoding`/`set-scale` all
//! parse cleanly through the catalog while the shipped host answered
//! every one with `[command-layer.unsupported-for-host]`), host
//! half: the five verbs driven through the REAL dispatch entry
//! ([`lumen_dispatch`]) and the REAL binary's `--control-stdio`
//! machine mode to the LIVE session's subtitle lane — the engine's
//! `SelectSubtitle` session command composed with the consumer's
//! sidecar lane (the FX9-A03 seam) — with every answer the chain's
//! own read-back, never the request echoed: the registered sidecar's
//! `tracks.list` row, the sync lane's new absolute total, the
//! applied encoding and scale, and the applied selector.
//!
//! The refusals that REMAIN are the honest ones (§2.13): no session
//! (`not-ready`), and the engine's own typed refusals through the
//! composed lane (a track id this session does not have is the
//! executor's `TrackNotFound` — the embedded-lane half's proof).

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command as StdCommand, Stdio};

use commands::catalog::types::{SourceSpec, TrackId, TrackKind, TrackOrigin, TrackSelector};
use commands::envelope::RequestId;
use commands::CommandOutcome;
use commands::CommandResult;
use commands::{Command, WireRequest};
use lumen_cli::dispatch::lumen_dispatch;
use lumen_cli::host::CliHost;
use testkit::Must;

/// The stereo PCM fixture (a session that opens WITH a live audio
/// limb — the host's own `pcm_session_keeps_its_audio_limb` seam
/// pin). Any live session carries the sidecar lane.
fn wav_path() -> String {
    format!(
        "{}/../../tests/fixtures/generated/valid/wav_pcm_s16le_44k1_st.wav",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// A temp dir that removes itself (the fx1_a09 suite's convention).
struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> TempDir {
        let dir = std::env::temp_dir().join(format!("lumen-fx9-a01-{tag}-{}", std::process::id())); // lumen-allow-literal: cfg(test) scratch filename — test-only, never a shipped surface
        std::fs::create_dir_all(&dir).must();
        TempDir(dir)
    }
    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).must();
        path
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A valid two-cue SRT sidecar (the parser's canonical block shape —
/// the sidecar load the FX9-A03 seam reduces to cues).
fn one_srt(dir: &TempDir) -> String {
    let body = concat!(
        "1\n",
        "00:00:00,000 --> 00:00:02,000\n",
        "Hello subtitle lane\n",
        "\n",
        "2\n",
        "00:00:02,000 --> 00:00:04,000\n",
        "The second cue\n",
    );
    dir.write("one.srt", body.as_bytes()).display().to_string()
}

/// Dispatch one catalog command through the REAL entry (the same
/// `lumen_dispatch` the machine mode and the desktop/tauri shells
/// drive — ARCH-5: one command layer).
fn dispatch(
    host: &mut CliHost,
    command: Command,
) -> Result<CommandOutcome<CommandResult>, commands::CommandError> {
    lumen_dispatch(
        host,
        &WireRequest {
            request_id: RequestId(1),
            command,
        },
    )
    .result
}

/// Open + settle one source (the in-process host harness convention).
fn open(host: &mut CliHost, path: &str) {
    let outcome = dispatch(
        host,
        Command::SessionOpen {
            source: SourceSpec::File { path: path.into() },
            options: Default::default(),
        },
    );
    assert!(outcome.is_ok(), "open failed: {outcome:?}");
    host.settle();
}

/// THE FX9 inversion: `subtitle.add-file` is served (class A — never
/// the `unsupported-for-host` refusal the audit recorded) and the
/// sidecar's arrival is the `tracks.list` chain's own read-back — the
/// appended sidecar row (an external-subtitle track that did not
/// exist before the add, selected as `select` asked).
#[test]
fn fx9_a01_add_file_is_served_and_lists_the_sidecar_track() {
    let dir = TempDir::new("add-file");
    let mut host = CliHost::new();
    open(&mut host, &wav_path());
    // Before: the fixture's own audio track, no subtitle rows.
    match dispatch(&mut host, Command::TracksList) {
        Ok(CommandOutcome::Completed(CommandResult::TrackList { tracks })) => {
            assert!(
                !tracks.iter().any(|row| row.kind == TrackKind::Subtitle),
                "the fixture carries no subtitle rows before the add: {tracks:?}"
            );
        }
        other => unreachable!("tracks.list must complete: {other:?}"),
    }
    // The class-A add: accepted, inside the session.
    match dispatch(
        &mut host,
        Command::SubtitleAddFile {
            path: one_srt(&dir),
            encoding: None,
            select: true,
        },
    ) {
        Ok(CommandOutcome::Accepted { session, .. }) => {
            assert!(session.is_some(), "the add answered inside the session");
        }
        other => unreachable!("subtitle.add-file must be served: {other:?}"),
    }
    // The chain's read-back: the sidecar row (external origin,
    // selected as the add's `select` asked).
    match dispatch(&mut host, Command::TracksList) {
        Ok(CommandOutcome::Completed(CommandResult::TrackList { tracks })) => {
            let Some(sidecar) = tracks.iter().find(|row| row.kind == TrackKind::Subtitle) else {
                unreachable!("the sidecar row must be listed: {tracks:?}")
            };
            assert_eq!(
                sidecar.origin, TrackOrigin::ExternalSubtitle,
                "the added file's row is an external subtitle track: {sidecar:?}"
            );
            assert!(
                sidecar.is_selected,
                "the add's `select: true` selected the sidecar: {sidecar:?}"
            );
        }
        other => unreachable!("tracks.list must complete: {other:?}"),
    }
}

/// `subtitle.set-sync` (class S): the answer is the NEW ABSOLUTE
/// total — the lane's own read-back, never the echoed delta — and a
/// second set moves the same lane's total.
#[test]
fn fx9_a01_set_sync_reads_back_the_new_absolute_total() {
    let dir = TempDir::new("set-sync");
    let mut host = CliHost::new();
    open(&mut host, &wav_path());
    let add = dispatch(
        &mut host,
        Command::SubtitleAddFile {
            path: one_srt(&dir),
            encoding: None,
            select: true,
        },
    );
    assert!(add.is_ok(), "the sidecar load: {add:?}");
    for (delta_ms, total_ms) in [(250_i64, 250_i64), (-100, 150)] {
        match dispatch(&mut host, Command::SubtitleSetSync { delta_ms }) {
            Ok(CommandOutcome::Completed(CommandResult::AppliedDelayMs { delay_ms })) => {
                assert_eq!(
                    delay_ms, total_ms,
                    "the new absolute total after {delta_ms}ms (the read-back, never the delta)"
                );
            }
            other => unreachable!("set-sync {delta_ms} must complete: {other:?}"),
        }
    }
}

/// `subtitle.set-encoding` + `subtitle.set-scale` (class S): both
/// answer the applied truth of the LIVE lane — the encoding the lane
/// re-decoded with and the scale now in force, never the echoed
/// request — and a second set moves the same lane.
#[test]
fn fx9_a01_set_encoding_and_scale_read_back_the_applied_values() {
    let dir = TempDir::new("encoding-scale");
    let mut host = CliHost::new();
    open(&mut host, &wav_path());
    let add = dispatch(
        &mut host,
        Command::SubtitleAddFile {
            path: one_srt(&dir),
            encoding: None,
            select: true,
        },
    );
    assert!(add.is_ok(), "the sidecar load: {add:?}");
    match dispatch(
        &mut host,
        Command::SubtitleSetEncoding {
            encoding: "utf-8".to_string(),
        },
    ) {
        Ok(CommandOutcome::Completed(CommandResult::AppliedEncoding { encoding })) => {
            assert_eq!(
                encoding.to_ascii_lowercase(),
                "utf-8",
                "the applied encoding is the lane's own read-back"
            );
        }
        other => unreachable!("set-encoding must complete: {other:?}"),
    }
    for (scale, applied) in [(1.25_f32, 1.25), (2.0, 2.0)] {
        match dispatch(&mut host, Command::SubtitleSetScale { scale }) {
            Ok(CommandOutcome::Completed(CommandResult::AppliedScale {
                scale: read_back,
            })) => {
                assert_eq!(read_back, applied, "the applied scale is the lane's read-back");
            }
            other => unreachable!("set-scale {scale} must complete: {other:?}"),
        }
    }
}

/// `tracks.set-subtitle-track` composes the engine's `SelectSubtitle`
/// session command with the sidecar lane: a sidecar id selects the
/// sidecar row (read back through the applied selector AND the
/// row's own flag), `None` deselects, `Auto` answers the container's
/// own truth (no subtitle tracks → none), an id this session does
/// not have is the ENGINE's own typed refusal (the embedded-lane
/// half), and the embedded-lane deselects ride the engine's
/// `tracks.changed` events (the composition's observable proof).
#[test]
fn fx9_a01_set_subtitle_track_composes_the_engine_and_sidecar_lanes() {
    let dir = TempDir::new("compose");
    let mut host = CliHost::new();
    open(&mut host, &wav_path());
    let add = dispatch(
        &mut host,
        Command::SubtitleAddFile {
            path: one_srt(&dir),
            encoding: None,
            select: false,
        },
    );
    assert!(add.is_ok(), "the sidecar load: {add:?}");
    let sidecar_id = match dispatch(&mut host, Command::TracksList) {
        Ok(CommandOutcome::Completed(CommandResult::TrackList { tracks })) => {
            let row = tracks
                .into_iter()
                .find(|row| row.kind == TrackKind::Subtitle)
                .must();
            assert!(!row.is_selected, "the add's `select: false` left it unselected");
            row.id
        }
        other => unreachable!("tracks.list must complete: {other:?}"),
    };
    // Select the sidecar through the verb → the applied selector is
    // the chain's read-back, and the row's own flag agrees.
    match dispatch(
        &mut host,
        Command::TracksSetSubtitleTrack {
            selector: TrackSelector::Id { id: sidecar_id },
        },
    ) {
        Ok(CommandOutcome::Completed(CommandResult::AppliedSelector { selector })) => {
            assert_eq!(
                selector,
                TrackSelector::Id { id: sidecar_id },
                "the applied selector is the chain's read-back"
            );
        }
        other => unreachable!("set-subtitle-track must complete: {other:?}"),
    }
    match dispatch(&mut host, Command::TracksList) {
        Ok(CommandOutcome::Completed(CommandResult::TrackList { tracks })) => {
            let row = tracks.iter().find(|row| row.id == sidecar_id).must();
            assert!(row.is_selected, "the sidecar lane holds the selection");
        }
        other => unreachable!("tracks.list must complete: {other:?}"),
    }
    // Deselect: the read-back is no-track, and the row's flag agrees.
    match dispatch(
        &mut host,
        Command::TracksSetSubtitleTrack {
            selector: TrackSelector::None,
        },
    ) {
        Ok(CommandOutcome::Completed(CommandResult::AppliedSelector { selector })) => {
            assert_eq!(selector, TrackSelector::None);
        }
        other => unreachable!("the deselect must complete: {other:?}"),
    }
    match dispatch(&mut host, Command::TracksList) {
        Ok(CommandOutcome::Completed(CommandResult::TrackList { tracks })) => {
            let row = tracks.iter().find(|row| row.id == sidecar_id).must();
            assert!(!row.is_selected, "the deselect reached the row's flag");
        }
        other => unreachable!("tracks.list must complete: {other:?}"),
    }
    // Auto: the product's rules over the CONTAINER's tracks — this
    // fixture has none, so the read-back is no-track (never the
    // echoed `Auto`).
    match dispatch(
        &mut host,
        Command::TracksSetSubtitleTrack {
            selector: TrackSelector::Auto,
        },
    ) {
        Ok(CommandOutcome::Completed(CommandResult::AppliedSelector { selector })) => {
            assert_eq!(
                selector,
                TrackSelector::None,
                "Auto answers the container's resolution, never the echoed request"
            );
        }
        other => unreachable!("auto must complete: {other:?}"),
    }
    // An id this session does not have: the ENGINE's own typed
    // `TrackNotFound` through the composed embedded lane.
    match dispatch(
        &mut host,
        Command::TracksSetSubtitleTrack {
            selector: TrackSelector::Id {
                id: TrackId(9_999),
            },
        },
    ) {
        Err(error) => {
            assert_eq!(
                error.code,
                commands::ErrorCode::NotFound,
                "the engine's own TrackNotFound through the embedded lane: {error}"
            );
        }
        Ok(_) => unreachable!("a track this session does not have must refuse"),
    }
    // The composition's engine-lane proof: the selections above drove
    // the engine's `tracks.changed` (the embedded-lane deselect
    // composed beside every sidecar select).
    let engine_lane = host.events().iter().any(|event| {
        matches!(
            event.kind,
            commands::EventKind::TracksChanged {
                track_kind: commands::catalog::types::TrackChangeKind::Subtitle,
                ..
            }
        )
    });
    assert!(
        engine_lane,
        "the engine's SelectSubtitle lane rode the composition (its tracks.changed events)"
    );
}

/// No session: every subtitle verb answers the honest `not-ready`
/// (stage `session`) — the state truth, never a panic and never the
/// `unsupported-for-host` capability lie the audit recorded.
#[test]
fn fx9_a01_subtitle_verbs_without_a_session_refuse_not_ready() {
    let mut host = CliHost::new();
    let verbs = [
        Command::TracksSetSubtitleTrack {
            selector: TrackSelector::None,
        },
        Command::SubtitleAddFile {
            path: std::env::temp_dir()
                .join("lumen-fx9-a01-absent.srt")
                .display()
                .to_string(),
            encoding: None,
            select: true,
        },
        Command::SubtitleSetSync { delta_ms: 250 },
        Command::SubtitleSetEncoding {
            encoding: "utf-8".to_string(),
        },
        Command::SubtitleSetScale { scale: 1.25 },
    ];
    for verb in verbs {
        match dispatch(&mut host, verb.clone()) {
            Err(error) => {
                assert_eq!(
                    error.code,
                    commands::ErrorCode::NotReady,
                    "{verb:?}: {error}"
                );
                assert_eq!(error.stage, commands::Stage::Session, "{error}");
            }
            Ok(_) => unreachable!("{verb:?} without a session must refuse"),
        }
    }
}

/// The FX9 reproduction, inverted, through the REAL binary's machine
/// mode: the NDJSON verbs on the wire answer their typed payloads —
/// the add's class-A accept, the sync lane's new absolute total, the
/// applied encoding and scale, and the composed selector — and the
/// `unsupported-for-host` refusal string is gone from the
/// transcript.
#[test]
fn fx9_a01_machine_mode_subtitle_verbs_answer_completed_not_refused() {
    let dir = TempDir::new("machine-mode");
    let srt = one_srt(&dir);
    let script = format!(
        "{{\"requestId\": 1, \"cmd\": \"session.open\", \"args\": {{\"source\": {{\"kind\": \"file\", \"path\": \"{path}\"}}}}}}\n\
         {{\"requestId\": 2, \"cmd\": \"subtitle.add-file\", \"args\": {{\"path\": \"{srt}\", \"encoding\": null, \"select\": true}}}}\n\
         {{\"requestId\": 3, \"cmd\": \"subtitle.set-sync\", \"args\": {{\"deltaMs\": 250}}}}\n\
         {{\"requestId\": 4, \"cmd\": \"subtitle.set-encoding\", \"args\": {{\"encoding\": \"utf-8\"}}}}\n\
         {{\"requestId\": 5, \"cmd\": \"subtitle.set-scale\", \"args\": {{\"scale\": 1.25}}}}\n\
         {{\"requestId\": 6, \"cmd\": \"tracks.set-subtitle-track\", \"args\": {{\"selector\": {{\"kind\": \"none\"}}}}}}\n",
        path = wav_path(),
        srt = srt,
    );
    let mut child = StdCommand::new(env!("CARGO_BIN_EXE_lumen"))
        .args(["--control-stdio"])
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
    assert_eq!(output.status.code(), Some(0));
    let out = String::from_utf8_lossy(&output.stdout).into_owned();
    let err = String::from_utf8_lossy(&output.stderr).into_owned();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines.len(),
        6,
        "one NDJSON response per request:\n{out}{err}"
    );
    // The open is accepted (class A); the machine mode never settles.
    let response = lines[0];
    assert!(response.contains("\"requestId\":1"), "{response}");
    assert!(response.contains("\"accepted\""), "{response}");
    // The add is served (class A — its arrival truth is tracks.list).
    let response = lines[1];
    assert!(response.contains("\"requestId\":2"), "{response}");
    assert!(response.contains("\"accepted\""), "{response}");
    // The sync lane's new absolute total (never the delta echoed).
    let response = lines[2];
    assert!(response.contains("\"requestId\":3"), "{response}");
    assert!(response.contains("\"kind\":\"applied-delay-ms\""), "{response}");
    assert!(response.contains("\"delayMs\":250"), "{response}");
    // The applied encoding.
    let response = lines[3];
    assert!(response.contains("\"requestId\":4"), "{response}");
    assert!(response.contains("\"kind\":\"applied-encoding\""), "{response}");
    // The applied scale.
    let response = lines[4];
    assert!(response.contains("\"requestId\":5"), "{response}");
    assert!(response.contains("\"kind\":\"applied-scale\""), "{response}");
    assert!(response.contains("\"scale\":1.25"), "{response}");
    // The composed selector (the `none` deselect's read-back).
    let response = lines[5];
    assert!(response.contains("\"requestId\":6"), "{response}");
    assert!(response.contains("\"kind\":\"applied-selector\""), "{response}");
    for line in lines {
        assert!(
            !line.contains("unsupported-for-host"),
            "the FX9 refusal must be gone: {line}"
        );
    }
    assert!(!err.contains("unsupported-for-host"), "{err}");
}
