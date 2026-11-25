//! F1.1c, FX4-A02 — the W07 "dead audio-controls surface" (W07-A02/
//! A06/A07/A08), host half: the `audio.set-volume` / `audio.get-volume`
//! / `audio.set-mute` catalog verbs driven through the REAL dispatch
//! entry ([`lumen_dispatch`]) and the REAL binary's `--control-stdio`
//! machine mode. The exact refusal the audit executed on every shipped
//! host (`[command-layer.unsupported-for-host]`, "audio device control
//! joins with the real backends (ADR-005)") is gone: the verbs now
//! reach the live session's controls seam (the FX4-A01 `audio_controls()`
//! contract over FX3-A03's master chain) and answer the chain's own
//! read-back — never the request echoed.
//!
//! The refusals that REMAIN are the honest ones (§2.13): no session
//! (`not-ready`), a session with no audio limb (video-only / limb-less
//! open — the ADTS family's recorded gap), and a bad volume at the
//! control boundary (the engine's own `audio.gain_*` codes).

use std::io::Write;
use std::process::{Command as StdCommand, Stdio};

use commands::catalog::types::SourceSpec;
use commands::envelope::RequestId;
use commands::CommandOutcome;
use commands::CommandResult;
use commands::{Command, WireRequest};
use lumen_cli::dispatch::lumen_dispatch;
use lumen_cli::host::CliHost;
use testkit::Must;

/// The stereo PCM fixture (a session that opens WITH a live audio
/// limb — the host's own `pcm_session_keeps_its_audio_limb` seam pin).
fn wav_path() -> String {
    format!(
        "{}/../../tests/fixtures/generated/valid/wav_pcm_s16le_44k1_st.wav",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// The ADTS fixture (a session that opens honestly LIMB-LESS — no
/// geometry-bearing truth to size an audio half from; the host's own
/// `in_band_config_audio_stays_limb_less` seam pin).
fn adts_path() -> String {
    format!(
        "{}/../../crates/codecs/tests/fixtures/sine48_lc_mono.aac",
        env!("CARGO_MANIFEST_DIR")
    )
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

/// One verb's applied volume, pinned to the `AudioVolume` result.
fn applied_volume(host: &mut CliHost, command: Command) -> f32 {
    match dispatch(host, command) {
        Ok(CommandOutcome::Completed(CommandResult::AudioVolume { volume })) => volume,
        other => unreachable!("the volume verb must complete: {other:?}"),
    }
}

/// THE W07 inversion: `audio.set-volume` completes (never the
/// `unsupported-for-host` refusal), the answer is the applied truth,
/// and the read twin answers the LIVE chain — a fresh session is
/// unity, each set moves the same chain, and the session keeps
/// playing through the controls (a between-quanta parameter swap,
/// never a rebuild).
#[test]
fn fx4_a02_set_volume_is_served_and_reads_back_the_live_chain() {
    let mut host = CliHost::new();
    open(&mut host, &wav_path());
    // The hydration read before any set: the identity (`1.0`).
    let volume = applied_volume(&mut host, Command::AudioGetVolume);
    assert_eq!(volume, 1.0, "a fresh session's master chain is unity");
    // Set → the applied truth; a second set moves the same chain.
    let volume = applied_volume(&mut host, Command::AudioSetVolume { volume: 0.5 });
    assert_eq!(volume, 0.5);
    let volume = applied_volume(&mut host, Command::AudioGetVolume);
    assert_eq!(volume, 0.5, "the read answers the live chain");
    let volume = applied_volume(&mut host, Command::AudioSetVolume { volume: 0.25 });
    assert_eq!(volume, 0.25);
    let volume = applied_volume(&mut host, Command::AudioGetVolume);
    assert_eq!(volume, 0.25, "the second set is the chain's new truth");
    // The controls never disturbed the session.
    assert_eq!(host.state(), commands::PlaybackState::Playing);
}

/// `audio.set-mute` (class S, independent of volume): the applied
/// boolean round-trips and mute never clobbers the volume — the
/// stage is a stateless multiply by zero (FX3-A03's composition
/// property, held through the verb lane).
#[test]
fn fx4_a02_set_mute_round_trips_and_composes_with_volume() {
    let mut host = CliHost::new();
    open(&mut host, &wav_path());
    applied_volume(&mut host, Command::AudioSetVolume { volume: 0.5 });
    // Mute on → the applied truth.
    match dispatch(&mut host, Command::AudioSetMute { muted: true }) {
        Ok(CommandOutcome::Completed(CommandResult::AppliedBool { value })) => {
            assert!(value, "mute applied");
        }
        other => unreachable!("set-mute must complete: {other:?}"),
    }
    // The volume survives muting (mute is not a volume change).
    let volume = applied_volume(&mut host, Command::AudioGetVolume);
    assert_eq!(volume, 0.5, "mute must not clobber the volume");
    // Unmute restores the same applied boolean.
    match dispatch(&mut host, Command::AudioSetMute { muted: false }) {
        Ok(CommandOutcome::Completed(CommandResult::AppliedBool { value })) => {
            assert!(!value, "unmute applied");
        }
        other => unreachable!("set-mute must complete: {other:?}"),
    }
    let volume = applied_volume(&mut host, Command::AudioGetVolume);
    assert_eq!(volume, 0.5, "the chain kept the volume across mute");
}

/// No session: every audio verb answers the honest `not-ready`
/// (stage `session`) — the state truth, never a panic and never the
/// capability lie the audit recorded.
#[test]
fn fx4_a02_audio_verbs_without_a_session_refuse_not_ready() {
    let mut host = CliHost::new();
    let verbs = [
        Command::AudioSetVolume { volume: 0.5 },
        Command::AudioGetVolume,
        Command::AudioSetMute { muted: true },
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

/// A session that opened LIMB-LESS (the ADTS family — no geometry
/// source, so no audio half exists) refuses typed at the
/// `audio-output` stage instead of fabricating a default applied
/// nowhere.
#[test]
fn fx4_a02_limb_less_session_refuses_typed_not_a_fake_default() {
    let mut host = CliHost::new();
    open(&mut host, &adts_path());
    match dispatch(&mut host, Command::AudioSetVolume { volume: 0.5 }) {
        Err(error) => {
            assert_eq!(
                error.code,
                commands::ErrorCode::NotReady,
                "the limb-less refusal: {error}"
            );
            assert_eq!(error.stage, commands::Stage::AudioOutput, "{error}");
        }
        Ok(_) => unreachable!("a limb-less session has no controls to set"),
    }
}

/// A bad volume is refused typed at the control boundary with the
/// engine's own stable codes (W07-A02 F-001's discipline — never a
/// clamp): negative linear gain and non-finite gain.
#[test]
fn fx4_a02_invalid_volume_refuses_typed_at_the_control_boundary() {
    let mut host = CliHost::new();
    open(&mut host, &wav_path());
    for (volume, code) in [
        (-1.0_f32, "audio.gain_negative"),
        (f32::NAN, "audio.gain_not_finite"),
    ] {
        match dispatch(&mut host, Command::AudioSetVolume { volume }) {
            Err(error) => {
                assert_eq!(
                    error.details.get("code").map(String::as_str),
                    Some(code),
                    "{volume}: {error}"
                );
            }
            Ok(_) => unreachable!("the bad volume {volume} must be refused"),
        }
    }
}

/// The W07 reproduction, inverted, through the REAL binary's machine
/// mode: the NDJSON verbs on the wire answer `completed` payloads
/// (audio-volume / applied-bool) — the `unsupported-for-host` refusal
/// string is gone from the transcript.
#[test]
fn fx4_a02_machine_mode_audio_verbs_answer_completed_not_refused() {
    let script = format!(
        "{{\"requestId\": 1, \"cmd\": \"session.open\", \"args\": {{\"source\": {{\"kind\": \"file\", \"path\": \"{path}\"}}}}}}\n\
         {{\"requestId\": 2, \"cmd\": \"audio.set-volume\", \"args\": {{\"volume\": 0.5}}}}\n\
         {{\"requestId\": 3, \"cmd\": \"audio.get-volume\"}}\n\
         {{\"requestId\": 4, \"cmd\": \"audio.set-mute\", \"args\": {{\"muted\": true}}}}\n",
        path = wav_path()
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
        4,
        "one NDJSON response per request:\n{out}{err}"
    );
    // The open is accepted (class A); the machine mode never settles.
    let response = lines[0];
    assert!(response.contains("\"requestId\":1"), "{response}");
    assert!(response.contains("\"accepted\""), "{response}");
    // THE refusal the audit executed is gone, and the verbs answer
    // the §2.6 class-S payloads.
    let response = lines[1];
    assert!(response.contains("\"requestId\":2"), "{response}");
    assert!(response.contains("\"kind\":\"audio-volume\""), "{response}");
    assert!(response.contains("\"volume\":0.5"), "{response}");
    let response = lines[2];
    assert!(response.contains("\"requestId\":3"), "{response}");
    assert!(response.contains("\"volume\":0.5"), "{response}");
    let response = lines[3];
    assert!(response.contains("\"requestId\":4"), "{response}");
    assert!(response.contains("\"kind\":\"applied-bool\""), "{response}");
    assert!(response.contains("\"value\":true"), "{response}");
    for line in lines {
        assert!(
            !line.contains("unsupported-for-host"),
            "the W07 refusal must be gone: {line}"
        );
    }
    assert!(!err.contains("unsupported-for-host"), "{err}");
}
