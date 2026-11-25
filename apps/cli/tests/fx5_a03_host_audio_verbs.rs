//! F1.1c, FX5-A03 — the two audio verbs FX4-A02 disclosed as catalog
//! drift (its report's drift note: no `audio.set-balance`/`audio.set-eq`
//! id existed to route), host half: `audio.set-balance` and
//! `audio.set-eq` driven through the REAL dispatch entry
//! ([`lumen_dispatch`]) and the REAL binary's `--control-stdio`
//! machine mode to the live session's controls seam (the FX4-A01
//! `audio_controls()` contract over FX3-A03's master chain). The verbs
//! answer the chain's own read-back — the armed stage's position and
//! the armed Eq3Band's band gains — never the request echoed.
//!
//! The refusals that REMAIN are the honest ones (§2.13): no session
//! (`not-ready`), a session with no audio limb (video-only /
//! limb-less open — the ADTS family's recorded gap), and the control
//! boundary's own typed range refusals — the balance setter's
//! `audio.effects_balance_position`, the EQ setter's
//! `audio.effects_eq_params`, and the preamp crossing's
//! `audio.gain_db_*` codes (W07-A02 F-001's discipline through the
//! verb lane).

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
/// limb — the host's own `pcm_session_keeps_its_audio_limb` seam
/// pin; the balance stage's stereo shape holds through the whole
/// session path).
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

/// One verb's applied volume, pinned to the `AudioVolume` result (the
/// volume family's read twin — the composition check the new verbs
/// must not disturb).
fn applied_volume(host: &mut CliHost, command: Command) -> f32 {
    match dispatch(host, command) {
        Ok(CommandOutcome::Completed(CommandResult::AudioVolume { volume })) => volume,
        other => unreachable!("the volume verb must complete: {other:?}"),
    }
}

/// `audio.set-balance` completes (never the `unsupported-for-host`
/// refusal the drift recorded), the answer is the applied position —
/// the chain's own read-back, never the request echoed — a second
/// set moves the same live chain (a between-quanta parameter swap,
/// never a rebuild), and the balance stage never disturbs the volume
/// family's knobs (FX3-A03's composition property through the verb
/// lane).
#[test]
fn fx5_a03_set_balance_is_served_and_reads_back_the_live_chain() {
    let mut host = CliHost::new();
    open(&mut host, &wav_path());
    for position in [-0.5_f32, 0.25] {
        match dispatch(&mut host, Command::AudioSetBalance { position }) {
            Ok(CommandOutcome::Completed(CommandResult::AudioBalance { position: applied })) => {
                assert_eq!(
                    applied,
                    Some(position),
                    "the applied position is the chain's own read-back"
                );
            }
            other => unreachable!("set-balance must complete: {other:?}"),
        }
    }
    // Balance never clobbers the volume family.
    let volume = applied_volume(&mut host, Command::AudioGetVolume);
    assert_eq!(volume, 1.0, "balance must not clobber the volume");
    // The controls never disturbed the session.
    assert_eq!(host.state(), commands::PlaybackState::Playing);
}

/// `audio.set-eq` completes, the answer is the armed Eq3Band's own
/// configuration (the stored band gains verbatim; the unity preamp
/// renders back to exactly 0 dB — 10^0 = 1 and log10(1) = 0), a
/// second set MOVES the armed chain (the in-place coefficient
/// recompute, the biquad state preserved — never a rebuild), and the
/// EQ never disturbs the volume family's knobs.
#[test]
fn fx5_a03_set_eq_is_served_and_reads_back_the_live_chain() {
    let mut host = CliHost::new();
    open(&mut host, &wav_path());
    // Set a volume first: the EQ lane must compose with the volume
    // family, not clobber it.
    applied_volume(&mut host, Command::AudioSetVolume { volume: 0.5 });
    // Arm → the applied bands.
    match dispatch(
        &mut host,
        Command::AudioSetEq {
            low_db: -12.0,
            mid_db: 0.0,
            high_db: 3.0,
            preamp_db: 0.0,
        },
    ) {
        Ok(CommandOutcome::Completed(CommandResult::AudioEq { eq: Some(bands) })) => {
            assert_eq!(
                (bands.low_db, bands.mid_db, bands.high_db),
                (-12.0, 0.0, 3.0)
            );
            assert_eq!(bands.preamp_db, 0.0, "the unity preamp round-trips exactly");
        }
        other => unreachable!("set-eq must complete: {other:?}"),
    }
    // A second set moves the armed chain without a rebuild.
    match dispatch(
        &mut host,
        Command::AudioSetEq {
            low_db: -6.0,
            mid_db: 2.0,
            high_db: -1.0,
            preamp_db: 0.0,
        },
    ) {
        Ok(CommandOutcome::Completed(CommandResult::AudioEq { eq: Some(bands) })) => {
            assert_eq!(
                (bands.low_db, bands.mid_db, bands.high_db),
                (-6.0, 2.0, -1.0)
            );
            assert_eq!(bands.preamp_db, 0.0);
        }
        other => unreachable!("the second set-eq must complete: {other:?}"),
    }
    // The EQ never clobbered the volume; the session keeps playing
    // through the controls.
    let volume = applied_volume(&mut host, Command::AudioGetVolume);
    assert_eq!(volume, 0.5, "the EQ must not clobber the volume");
    assert_eq!(host.state(), commands::PlaybackState::Playing);
}

/// No session: the new audio verbs answer the honest `not-ready`
/// (stage `session`) — the state truth, never a panic and never the
/// capability lie the audit recorded.
#[test]
fn fx5_a03_new_audio_verbs_without_a_session_refuse_not_ready() {
    let mut host = CliHost::new();
    let verbs = [
        Command::AudioSetBalance { position: 0.0 },
        Command::AudioSetEq {
            low_db: 0.0,
            mid_db: 0.0,
            high_db: 0.0,
            preamp_db: 0.0,
        },
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
/// nowhere — the volume family's limb-less refusal, mirrored exactly.
#[test]
fn fx5_a03_limb_less_session_refuses_typed_not_a_fake_default() {
    let mut host = CliHost::new();
    open(&mut host, &adts_path());
    let verbs = [
        Command::AudioSetBalance { position: -0.5 },
        Command::AudioSetEq {
            low_db: -3.0,
            mid_db: 0.0,
            high_db: 0.0,
            preamp_db: 0.0,
        },
    ];
    for verb in verbs {
        match dispatch(&mut host, verb.clone()) {
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
}

/// Bad control values are refused typed at the control boundary with
/// the engine's own stable codes — never a clamp: the balance knob's
/// hard stops (out of range and non-finite), the EQ band bound (beyond
/// ±40 dB and non-finite), and the preamp dB crossing's overflow
/// (W07-A02 F-001's executed proof through the verb lane).
#[test]
fn fx5_a03_invalid_balance_and_eq_refuse_typed_at_the_control_boundary() {
    let mut host = CliHost::new();
    open(&mut host, &wav_path());
    for position in [1.5_f32, -1.5, f32::NAN] {
        match dispatch(&mut host, Command::AudioSetBalance { position }) {
            Err(error) => {
                assert_eq!(
                    error.details.get("code").map(String::as_str),
                    Some("audio.effects_balance_position"),
                    "{position}: {error}"
                );
            }
            Ok(_) => unreachable!("the bad position {position} must be refused"),
        }
    }
    let bad_eq = [
        (41.0_f32, 0.0, 0.0, 0.0, "audio.effects_eq_params"),
        (0.0, f32::NAN, 0.0, 0.0, "audio.effects_eq_params"),
        (0.0, 0.0, 0.0, 771.0, "audio.gain_db_overflow"),
    ];
    for (low_db, mid_db, high_db, preamp_db, code) in bad_eq {
        match dispatch(
            &mut host,
            Command::AudioSetEq {
                low_db,
                mid_db,
                high_db,
                preamp_db,
            },
        ) {
            Err(error) => {
                assert_eq!(
                    error.details.get("code").map(String::as_str),
                    Some(code),
                    "({low_db}, {mid_db}, {high_db}, {preamp_db}): {error}"
                );
            }
            Ok(_) => unreachable!(
                "the bad eq ({low_db}, {mid_db}, {high_db}, {preamp_db}) must be refused"
            ),
        }
    }
}

/// The machine-mode reproduction, inverted: the NDJSON verbs on the
/// wire answer `completed` payloads (audio-balance / audio-eq, the
/// §2.6 class-S shapes) — the `unsupported-for-host` refusal the drift
/// recorded is gone from the transcript.
#[test]
fn fx5_a03_machine_mode_audio_verbs_answer_completed_not_refused() {
    let script = format!(
        "{{\"requestId\": 1, \"cmd\": \"session.open\", \"args\": {{\"source\": {{\"kind\": \"file\", \"path\": \"{path}\"}}}}}}\n\
         {{\"requestId\": 2, \"cmd\": \"audio.set-balance\", \"args\": {{\"position\": -0.5}}}}\n\
         {{\"requestId\": 3, \"cmd\": \"audio.set-eq\", \"args\": {{\"lowDb\": -12, \"midDb\": 0, \"highDb\": 3, \"preampDb\": 0}}}}\n",
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
        3,
        "one NDJSON response per request:\n{out}{err}"
    );
    // The open is accepted (class A); the machine mode never settles.
    let response = lines[0];
    assert!(response.contains("\"requestId\":1"), "{response}");
    assert!(response.contains("\"accepted\""), "{response}");
    // THE refusal the drift recorded is gone, and the verbs answer
    // the §2.6 class-S payloads.
    let response = lines[1];
    assert!(response.contains("\"requestId\":2"), "{response}");
    assert!(
        response.contains("\"kind\":\"audio-balance\""),
        "{response}"
    );
    assert!(response.contains("\"position\":-0.5"), "{response}");
    let response = lines[2];
    assert!(response.contains("\"requestId\":3"), "{response}");
    assert!(response.contains("\"kind\":\"audio-eq\""), "{response}");
    // FX5-A02's authoritative result shape: the four dB fields ride
    // nested under `eq` (AudioEqBands), not flat beside the kind tag.
    assert!(response.contains("\"eq\":{\"lowDb\":-12.0"), "{response}");
    assert!(response.contains("\"midDb\":0.0"), "{response}");
    assert!(response.contains("\"highDb\":3.0"), "{response}");
    assert!(response.contains("\"preampDb\":0.0"), "{response}");
    for line in lines {
        assert!(
            !line.contains("unsupported-for-host"),
            "the drift's refusal must be gone: {line}"
        );
    }
    assert!(!err.contains("unsupported-for-host"), "{err}");
}
