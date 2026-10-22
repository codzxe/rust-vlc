//! The 3.C2 tier-1 recovery matrix: every committed class-B fixture
//! driven through the FULL stack (access → probe → demux → queues →
//! decoders → sinks → wire), each row pinning its outcome CLASS and
//! the never-fabricated terms — the per-fixture pinning the class-B
//! expectation TOMLs' union contract (`typed-error-or-bounded-
//! recovery`) never had at engine level (defect G2: 17 of 22 rows
//! were unpinned; the demux-level halves live in
//! `demux`'s `fixtures_class_b`).
//!
//! # The matrix (fixture class → outcome class)
//!
//! | fixture | outcome | damage observable |
//! |---|---|---|
//! | avi trunc0/1 (12/52 B) | typed open refusal: `demux.corrupt`, `demux.avi.truncated` | — |
//! | avi trunc2 (324 B) | open accepted, then `Error` via FATAL `demux.avi.truncated` | ≥1 non-fatal RecoveredDamage at the terminal |
//! | avi trunc3 (mid-movi cut) | clean `Ended` (ED-FIX-3 clamped prefix + EOS) | exactly 1 non-fatal RecoveredDamage; last time-tick fraction < 1.0 |
//! | avi trunc4 (last byte cut) | clean `Ended` — the walker's 1-byte clamp delivers the full play-out | ≥1 non-fatal RecoveredDamage (the clamped tail) |
//! | avi garbage0 (4 KiB appended) | clean `Ended`, garbage never parsed | ZERO damage — correct: the declared structure ends before the garbage |
//! | avi flip1 | clean `Ended` | ZERO damage — the seeded flip landed in padding (benign; deterministic fixture) |
//! | avi flip4 | clean `Ended` | ≥1 non-fatal RecoveredDamage (the flip hit a chunk field; the clamp+resync recovered) |
//! | avi flip16 | typed open refusal: `probe.unsupported` with the sniffed bytes named | — |
//! | avi noidx0 / negheight0 / absidx1 | clean `Ended`, full accounting (linear walk / V1 height-abs / V4 idx-base autodetect) | zero wire-visible damage |
//! | mp4 ×10 (incl. stszlie0) | typed open refusal: `probe.unsupported` naming `mp4` | — (G4: no MP4 demuxer exists — probe-surface only, honest scoping) |
//! | parent + all-tracks-disabled | typed open refusal: `control.unsupported` naming `avi` | — (G1: known-format info end-to-end) |
//!
//! # Never-terms asserted on every row
//!
//! No panic (the test completing is the proof), no hang (the bounded
//! pump counts — `settle` + `run_pumps` — make it mechanical), wall
//! cap 5 s per row (the expectation TOMLs' `wall_cap_ms`; measured),
//! and NO fabricated output: a clean `Ended` carries exactly one
//! `playback.end-of-media`, `timing.duration-known` only from the
//! container's declared extents, and the truncated play-out's final
//! time-tick fraction stays below 1.0 (delivered data vs declared
//! duration — honest, never clamped to a fake 1.0).

mod common;

use std::time::Instant;

use commands::catalog::types::{OpenOptions, SourceSpec};
use commands::envelope::CommandOutcome;
use commands::{Command, ErrorCode, Stage};
use first_vertical::harness::{Check, SessionHarness, run_pumps};
use testkit::Must;

/// The declared duration of the AVI/MP4 class-B parents (µs) — the
/// header truth every full play-out lands on exactly.
const DECLARED_DURATION_US: i64 = 2_000_000;

/// The expectation TOMLs' wall cap (ms) — one measured bound per row.
const WALL_CAP_MS: u128 = 5_000;

// ---------------------------------------------------------------------------
// The row vocabulary
// ---------------------------------------------------------------------------

/// The damage observable a clean-Ended row pins.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Damage {
    /// Zero `playback.error` events — correct for benign mutations
    /// (padding flips, appended garbage past the declared structure).
    Zero,
    /// Exactly one non-fatal RecoveredDamage (the coalesced terminal
    /// flush under the frozen throttle).
    ExactlyOne,
    /// At least one non-fatal RecoveredDamage.
    AtLeastOne,
}

/// One matrix row's expected outcome class (the decision procedure's
/// target vocabulary — tier 2 reuses it).
enum Outcome {
    /// The open itself answers a typed refusal: stage + code + a
    /// message substring. No session survives.
    Refused {
        stage: Stage,
        code: ErrorCode,
        message_part: &'static str,
    },
    /// The open is accepted; the settle then lands in `Error` via one
    /// FATAL `playback.error` (the message substring) — plus the
    /// terminal's non-fatal damage flush.
    ErrorAfterOpen { message_part: &'static str },
    /// The media plays through to a clean `Ended`.
    CleanEnded {
        damage: Damage,
        frames: u64,
        position_us: i64,
        /// Pin the LAST emitted time-tick's fraction strictly below
        /// this bound (the never-fabricated-duration term; `None` =
        /// no tick pin).
        fraction_below: Option<f32>,
    },
}

/// One row of the matrix.
struct Row {
    /// The repo-relative fixture under `tests/fixtures/`.
    fixture: &'static str,
    /// The expected outcome class.
    outcome: Outcome,
}

// ---------------------------------------------------------------------------
// The driver
// ---------------------------------------------------------------------------

/// One wire event's interesting fields, pulled from the transcript.
struct EventRecord {
    kind: String,
    stage: String,
    code: String,
    message: String,
    fatal: Option<String>,
    count: Option<String>,
    position: Option<String>,
}

/// Extract the event records from a rendered transcript (the harness
/// records one JSON object per event; the fields this matrix reads
/// are the ones `event_json` writes).
fn event_records(text: &str) -> Vec<EventRecord> {
    let mut records = Vec::new();
    let mut current: Vec<(String, String)> = Vec::new();
    let mut in_events = false;
    let mut depth_stack: Vec<bool> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("\"events\"") {
            in_events = true;
            continue;
        }
        if !in_events {
            continue;
        }
        // An event object opens with `{` alone on the line and closes
        // with `},` or `}` (the harness's pretty renderer).
        if trimmed == "{" {
            if depth_stack.is_empty() {
                current.clear();
            }
            depth_stack.push(true);
            continue;
        }
        if trimmed.starts_with('}') {
            depth_stack.pop();
            if depth_stack.is_empty() {
                let field = |name: &str| {
                    current
                        .iter()
                        .find(|(key, _)| key == name)
                        .map(|(_, value)| value.clone())
                        .unwrap_or_default()
                };
                records.push(EventRecord {
                    kind: field("kind"),
                    stage: field("stage"),
                    code: field("code"),
                    message: field("message"),
                    fatal: current
                        .iter()
                        .find(|(key, _)| key == "fatal")
                        .map(|(_, value)| value.clone()),
                    count: current
                        .iter()
                        .find(|(key, _)| key == "count")
                        .map(|(_, value)| value.clone()),
                    position: current
                        .iter()
                        .find(|(key, _)| key == "position")
                        .map(|(_, value)| value.clone()),
                });
                if records.len() > 4096 {
                    // Bounded parse (D-1 paranoia; transcripts are
                    // ~hundreds of events).
                    break;
                }
            }
            continue;
        }
        // A `"key": value,` line inside the current object.
        if let Some((key, value)) = parse_field(trimmed) && !depth_stack.is_empty() {
            current.push((key, value));
        }
    }
    records
}

/// Parse one `"key": value,` line (string values keep their quotes
/// stripped; everything else renders raw).
fn parse_field(line: &str) -> Option<(String, String)> {
    let mut split = line.splitn(2, "\": ");
    let key = split.next()?.trim_start_matches('"');
    let value = split.next()?.trim_end_matches(',');
    Some((key.to_string(), value.trim_matches('"').to_string()))
}

/// Drive one row through the full stack and answer its checks.
fn drive(row: Row) -> Vec<Check> {
    let started = Instant::now();
    let source = SourceSpec::File {
        path: first_vertical::harness::fixture_path(row.fixture).must(),
    };
    let mut harness = SessionHarness::new(row.fixture);
    let open = harness.dispatch(
        "session.open",
        Command::SessionOpen {
            source,
            options: OpenOptions::default(),
        },
    );
    harness.settle();
    let settled = harness.state();
    let session_after_settle = harness.session_id();
    harness.advance(run_pumps());
    let end_state = harness.state();
    let end_position = harness.position_us();
    let transcript = harness.finish("recovery-matrix").transcript_bytes();
    let events = event_records(&transcript);
    if std::env::var_os("MATRIX_DEBUG").is_some() {
        eprintln!("=== TRANSCRIPT for {}:\n{transcript}", row.fixture);
    }
    let mut checks = Vec::new();
    let what = row.fixture.rsplit('/').next().unwrap_or(row.fixture);

    let errors: Vec<&EventRecord> = events
        .iter()
        .filter(|record| record.kind == "playback.error")
        .collect();
    let fatal_errors: Vec<&&EventRecord> = errors
        .iter()
        .filter(|r| r.fatal.as_deref() == Some("true"))
        .collect();
    let non_fatal: Vec<&&EventRecord> = errors
        .iter()
        .filter(|r| r.fatal.as_deref() == Some("false"))
        .collect();
    let end_events = events
        .iter()
        .filter(|record| record.kind == "playback.end-of-media")
        .count();
    let duration_known = events
        .iter()
        .filter(|record| record.kind == "timing.duration-known")
        .count();
    let ticks: Vec<&EventRecord> = events
        .iter()
        .filter(|record| record.kind == "timing.time-tick")
        .collect();
    let frames = events_position_frames(&transcript);

    match row.outcome {
        Outcome::Refused {
            stage,
            code,
            message_part,
        } => {
            checks.push(Check::is(
                &format!("{what}: the open refuses typed"),
                matches!(&open.result, Err(_)),
                "the open answered success",
            ));
            if let Err(error) = &open.result {
                checks.push(Check::same(
                    &format!("{what}: the refusal is {stage}.{code}"),
                    &format!("{}.{}", error.stage.as_str(), error.code.as_str()),
                    &format!("{}.{}", stage.as_str(), code.as_str()),
                ));
                checks.push(Check::is(
                    &format!("{what}: the message names the failure"),
                    error.to_string().contains(message_part),
                    &format!("message {:?} lacks {message_part:?}", error.to_string()),
                ));
            }
            checks.push(Check::same(
                &format!("{what}: no session survives"),
                &format!("{session_after_settle:?}"),
                "None",
            ));
            checks.push(Check::same(
                &format!("{what}: the host is Idle after the refusal"),
                &format!("{end_state:?}"),
                "Idle",
            ));
            checks.push(Check::int(
                &format!("{what}: zero wire events (nothing played, nothing fabricated)"),
                events.len() as i64,
                0,
            ));
        }
        Outcome::ErrorAfterOpen { message_part } => {
            checks.push(Check::is(
                &format!("{what}: the open is accepted (the header prefix parses)"),
                matches!(&open.result, Ok(CommandOutcome::Accepted { .. })),
                "the open refused",
            ));
            checks.push(Check::same(
                &format!("{what}: the settle lands in Error"),
                &format!("{settled:?}"),
                "Error",
            ));
            checks.push(Check::int(
                &format!("{what}: exactly one FATAL playback.error"),
                fatal_errors.len() as i64,
                1,
            ));
            if let Some(error) = fatal_errors.first() {
                checks.push(Check::is(
                    &format!("{what}: the fatal names the truncation"),
                    error.message.contains(message_part),
                    &format!("message {:?} lacks {message_part:?}", error.message),
                ));
                checks.push(Check::is(
                    &format!("{what}: the fatal is demux-stage"),
                    error.stage == "demux",
                    &format!("stage {:?}", error.stage),
                ));
            }
            checks.push(Check::is(
                &format!("{what}: the terminal damage flush is non-fatal and present"),
                non_fatal.len() == 1,
                &format!("non-fatal count {}", non_fatal.len()),
            ));
        }
        Outcome::CleanEnded {
            damage,
            frames: want_frames,
            position_us: want_position,
            fraction_below,
        } => {
            checks.push(Check::is(
                &format!("{what}: the open is accepted (bounded recovery, not a refusal)"),
                matches!(&open.result, Ok(CommandOutcome::Accepted { .. })),
                "the open refused",
            ));
            checks.push(Check::same(
                &format!("{what}: the media plays to Ended"),
                &format!("{end_state:?}"),
                "Ended",
            ));
            checks.push(Check::int(
                &format!("{what}: zero FATAL errors (bounded recovery only)"),
                fatal_errors.len() as i64,
                0,
            ));
            match damage {
                Damage::Zero => checks.push(Check::int(
                    &format!("{what}: zero playback.error events (zero damage is CORRECT here)"),
                    errors.len() as i64,
                    0,
                )),
                Damage::ExactlyOne => checks.push(Check::int(
                    &format!("{what}: exactly one non-fatal RecoveredDamage"),
                    non_fatal.len() as i64,
                    1,
                )),
                Damage::AtLeastOne => checks.push(Check::is(
                    &format!("{what}: at least one non-fatal RecoveredDamage"),
                    non_fatal.len() >= 1,
                    &format!("non-fatal count {}", non_fatal.len()),
                )),
            }
            checks.push(Check::int(
                &format!("{what}: exactly one playback.end-of-media"),
                end_events as i64,
                1,
            ));
            // Duration honesty: known only from the declared extents
            // (the intact headers), exactly once.
            checks.push(Check::int(
                &format!("{what}: exactly one duration-known (declared extents)"),
                duration_known as i64,
                1,
            ));
            checks.push(Check::int(
                &format!("{what}: the full accounting delivers {want_frames} frames"),
                frames,
                want_frames as i64,
            ));
            checks.push(Check::int(
                &format!("{what}: the final position is the honest play-out end"),
                end_position,
                want_position,
            ));
            if let Some(bound) = fraction_below {
                if let Some(last) = ticks.last() {
                    let fraction: f32 = last
                        .position
                        .clone()
                        .unwrap_or_default()
                        .parse()
                        .unwrap_or(1.0);
                    checks.push(Check::is(
                        &format!(
                            "{what}: the last time-tick fraction {fraction} stays below {bound} (never a fake 1.0)"
                        ),
                        fraction < bound,
                        "the truncated play-out pretended completeness",
                    ));
                } else {
                    checks.push(Check::is(
                        &format!("{what}: time-ticks flowed"),
                        false,
                        "no timing.time-tick observed",
                    ));
                }
            }
        }
    }
    // The never-terms, every row: bounded wall time (the TOML cap).
    let elapsed_ms = started.elapsed().as_millis();
    checks.push(Check::is(
        &format!("{what}: wall time within the 5 s cap"),
        elapsed_ms < WALL_CAP_MS,
        &format!("row took {elapsed_ms} ms"),
    ));
    checks
}

/// The frames_accepted observable from the transcript's accounting
/// block (0 when no accounting exists — the error rows).
fn events_position_frames(text: &str) -> i64 {
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("\"frames_accepted\": ") {
            return rest.trim_end_matches(',').parse().unwrap_or(0);
        }
    }
    0
}

/// Assert every check of a row held (the common::assert_green shape,
/// for raw check lists).
fn assert_row(fixture: &str, checks: &[Check]) {
    let mut any = false;
    for check in checks {
        any = true;
        assert!(
            check.passed,
            "row {fixture} — check failed: {} — {}",
            check.claim, check.detail
        );
    }
    assert!(any, "row {fixture} produced no checks");
}

// ---------------------------------------------------------------------------
// The AVI rows
// ---------------------------------------------------------------------------

#[test]
fn matrix_avi_trunc0_refuses_typed_at_demux() {
    let checks = drive(Row {
        fixture: "generated/malformed/avi_mjpeg_pcm__trunc0.avi",
        outcome: Outcome::Refused {
            stage: Stage::Demux,
            code: ErrorCode::Corrupt,
            message_part: "truncated",
        },
    });
    assert_row("trunc0", &checks);
}

#[test]
fn matrix_avi_trunc1_refuses_typed_at_demux() {
    let checks = drive(Row {
        fixture: "generated/malformed/avi_mjpeg_pcm__trunc1.avi",
        outcome: Outcome::Refused {
            stage: Stage::Demux,
            code: ErrorCode::Corrupt,
            message_part: "truncated",
        },
    });
    assert_row("trunc1", &checks);
}

#[test]
fn matrix_avi_trunc2_opens_then_lands_in_error_typed() {
    let checks = drive(Row {
        fixture: "generated/malformed/avi_mjpeg_pcm__trunc2.avi",
        outcome: Outcome::ErrorAfterOpen {
            message_part: "truncated",
        },
    });
    assert_row("trunc2", &checks);
}

#[test]
fn matrix_avi_trunc3_plays_to_clean_ended_with_damage() {
    let checks = drive(Row {
        fixture: "generated/malformed/avi_mjpeg_pcm__trunc3.avi",
        outcome: Outcome::CleanEnded {
            damage: Damage::ExactlyOne,
            frames: 61,
            position_us: 1_000_000,
            // The declared duration is 2.000 s; the delivered data is
            // the cut prefix — the last tick's fraction must stay
            // below 1.0 (never a fake completeness).
            fraction_below: Some(1.0),
        },
    });
    assert_row("trunc3", &checks);
}

#[test]
fn matrix_avi_trunc4_plays_to_clean_ended_full_payout() {
    // The boundary pin (design C2.1): the 1-byte cut lands inside the
    // FINAL chunk; the walker's clamp delivers its readable prefix,
    // so the play-out is complete (120 frames, exact duration) WITH a
    // damage note for the clamped tail.
    let checks = drive(Row {
        fixture: "generated/malformed/avi_mjpeg_pcm__trunc4.avi",
        outcome: Outcome::CleanEnded {
            damage: Damage::AtLeastOne,
            frames: 120,
            position_us: DECLARED_DURATION_US,
            fraction_below: None,
        },
    });
    assert_row("trunc4", &checks);
}

#[test]
fn matrix_avi_garbage0_clean_ended_zero_damage() {
    let checks = drive(Row {
        fixture: "generated/malformed/avi_mjpeg_pcm__garbage0.avi",
        outcome: Outcome::CleanEnded {
            damage: Damage::Zero,
            frames: 120,
            position_us: DECLARED_DURATION_US,
            fraction_below: None,
        },
    });
    assert_row("garbage0", &checks);
}

#[test]
fn matrix_avi_flip1_clean_ended_zero_damage() {
    // The seeded single flip landed in padding: a benign mutation.
    // Zero damage is the CORRECT outcome (pinned per-fixture — the
    // flips are deterministic, seeded by the parent's sha256).
    let checks = drive(Row {
        fixture: "generated/malformed/avi_mjpeg_pcm__flip1.avi",
        outcome: Outcome::CleanEnded {
            damage: Damage::Zero,
            frames: 120,
            position_us: DECLARED_DURATION_US,
            fraction_below: None,
        },
    });
    assert_row("flip1", &checks);
}

#[test]
fn matrix_avi_flip4_clean_ended_with_damage() {
    let checks = drive(Row {
        fixture: "generated/malformed/avi_mjpeg_pcm__flip4.avi",
        outcome: Outcome::CleanEnded {
            damage: Damage::AtLeastOne,
            frames: 120,
            position_us: DECLARED_DURATION_US,
            fraction_below: None,
        },
    });
    assert_row("flip4", &checks);
}

#[test]
fn matrix_avi_flip16_refuses_typed_at_probe() {
    // Sixteen header-extent flips smash the signature past the
    // probe's recognition: the refusal names the sniffed bytes (the
    // known-format hint) — content truth, not extension truth.
    let checks = drive(Row {
        fixture: "generated/malformed/avi_mjpeg_pcm__flip16.avi",
        outcome: Outcome::Refused {
            stage: Stage::Probe,
            code: ErrorCode::Unsupported,
            message_part: "sniffed",
        },
    });
    assert_row("flip16", &checks);
}

#[test]
fn matrix_avi_noidx0_clean_ended_full_payout() {
    let checks = drive(Row {
        fixture: "generated/malformed/avi_mjpeg_pcm__noidx0.avi",
        outcome: Outcome::CleanEnded {
            damage: Damage::Zero,
            frames: 120,
            position_us: DECLARED_DURATION_US,
            fraction_below: None,
        },
    });
    assert_row("noidx0", &checks);
}

#[test]
fn matrix_avi_negheight0_clean_ended_full_payout() {
    let checks = drive(Row {
        fixture: "generated/malformed/avi_mjpeg_pcm__negheight0.avi",
        outcome: Outcome::CleanEnded {
            damage: Damage::Zero,
            frames: 120,
            position_us: DECLARED_DURATION_US,
            fraction_below: None,
        },
    });
    assert_row("negheight0", &checks);
}

#[test]
fn matrix_avi_absidx1_clean_ended_full_payout() {
    let checks = drive(Row {
        fixture: "generated/malformed/avi_mjpeg_pcm__absidx1.avi",
        outcome: Outcome::CleanEnded {
            damage: Damage::Zero,
            frames: 120,
            position_us: DECLARED_DURATION_US,
            fraction_below: None,
        },
    });
    assert_row("absidx1", &checks);
}

// ---------------------------------------------------------------------------
// The MP4 rows (G4: probe-surface only — no MP4 demuxer exists)
// ---------------------------------------------------------------------------

/// Every MP4 class-B derivative refuses at the probe naming `mp4` —
/// the honest scoping (G4): the corpus's MP4 half pins the
/// probe-stage behavior only; the MP4 demux card (5.x) owns the rest.
#[test]
fn matrix_mp4_rows_refuse_at_probe_naming_the_format() {
    let mp4_fixtures = [
        "generated/malformed/mp4_mjpeg_pcm__trunc0.mp4",
        "generated/malformed/mp4_mjpeg_pcm__trunc1.mp4",
        "generated/malformed/mp4_mjpeg_pcm__trunc2.mp4",
        "generated/malformed/mp4_mjpeg_pcm__trunc3.mp4",
        "generated/malformed/mp4_mjpeg_pcm__trunc4.mp4",
        "generated/malformed/mp4_mjpeg_pcm__garbage0.mp4",
        "generated/malformed/mp4_mjpeg_pcm__flip1.mp4",
        "generated/malformed/mp4_mjpeg_pcm__flip4.mp4",
        "generated/malformed/mp4_mjpeg_pcm__flip16.mp4",
        "generated/malformed/mp4_mjpeg_pcm__stszlie0.mp4",
    ];
    for fixture in mp4_fixtures {
        let checks = drive(Row {
            fixture,
            outcome: Outcome::Refused {
                stage: Stage::Probe,
                code: ErrorCode::Unsupported,
                message_part: "mp4",
            },
        });
        assert_row(fixture, &checks);
    }
}

// ---------------------------------------------------------------------------
// The G1 end-to-end row (known-format info through the wire)
// ---------------------------------------------------------------------------

/// **G1 end-to-end.** The all-tracks-disabled open (the production
/// shape of "nothing decodable"): the engine's `NoDecodableTrack`
/// failure reaches the wire naming the container (`avi`) and saying
/// no tracks were reported — the known-format-info AC, full stack.
#[test]
fn matrix_all_disabled_refuses_naming_the_container() {
    let started = Instant::now();
    let fixture: &'static str = "generated/valid/avi_mjpeg_pcm.avi";
    let source = SourceSpec::File {
        path: first_vertical::harness::fixture_path(fixture).must(),
    };
    let mut harness = SessionHarness::new(fixture);
    let open = harness.dispatch(
        "session.open",
        Command::SessionOpen {
            source,
            options: OpenOptions {
                no_video: true,
                no_audio: true,
                ..OpenOptions::default()
            },
        },
    );
    harness.settle();
    let end_state = harness.state();
    let transcript = harness.finish("recovery-matrix").transcript_bytes();
    let mut checks = Vec::new();
    checks.push(Check::is(
        "all-disabled: the open refuses typed",
        matches!(&open.result, Err(_)),
        "the open answered success",
    ));
    if let Err(error) = &open.result {
        checks.push(Check::same(
            "all-disabled: the refusal is control.unsupported",
            &format!("{}.{}", error.stage.as_str(), error.code.as_str()),
            "control.unsupported",
        ));
        checks.push(Check::is(
            "all-disabled: the message names the container (G1)",
            error.to_string().contains("avi"),
            &format!("message {:?} lacks the container", error.to_string()),
        ));
        checks.push(Check::is(
            "all-disabled: the message is the no-decodable-track code",
            error.to_string().contains("session.no-decodable-track"),
            &format!("message {:?}", error.to_string()),
        ));
    }
    checks.push(Check::same(
        "all-disabled: the host is Idle after the refusal",
        &format!("{end_state:?}"),
        "Idle",
    ));
    let elapsed_ms = started.elapsed().as_millis();
    checks.push(Check::is(
        "all-disabled: wall time within the 5 s cap",
        elapsed_ms < WALL_CAP_MS,
        &format!("row took {elapsed_ms} ms"),
    ));
    let _ = transcript;
    assert_row("all-disabled", &checks);
}
