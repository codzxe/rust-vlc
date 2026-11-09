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
//! | mp4 trunc0 (empty) / trunc1 (64 B) | typed open refusal: `demux.corrupt` (`demux.mp4.no-moov` / `demux.mp4.truncated`) | — |
//! | mp4 trunc2/3/4 (mdat cuts) | open accepted, then `Error` via FATAL `demux.mp4.truncated` | — (no terminal flush: the fatal precedes any resync) |
//! | mp4 garbage0 (4 KiB appended) | clean `Ended`, garbage never parsed | ZERO damage |
//! | mp4 flip1/flip4 | typed open refusal: `demux.corrupt` (`demux.mp4.box-overruns-parent`) | — |
//! | mp4 flip16 | typed open refusal: `probe.unsupported` with the sniffed bytes named | — |
//! | mp4 stszlie0 | clean `Ended` (the stsz lie recovered by resync) | ≥1 non-fatal RecoveredDamage |
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
    /// The open is accepted; the settle then lands in `Error` via one
    /// FATAL `playback.error` — with NO terminal damage flush (the mp4
    /// truncation shape: the box walker's fatal refusal precedes any
    /// resynchronization, so there is nothing recovered to flush).
    ErrorAfterOpenNoFlush { message_part: &'static str },
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
/// are the ones `event_json` writes — flat objects only, so a depth
/// counter plus the events-array bounds are the whole grammar).
fn event_records(text: &str) -> Vec<EventRecord> {
    let mut records = Vec::new();
    let mut current: Vec<(String, String)> = Vec::new();
    let mut in_events = false;
    let mut depth = 0usize;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if !in_events {
            if trimmed.starts_with("\"events\"") {
                in_events = true;
            }
            continue;
        }
        // The events array closes with `]` (alone or with a trailing
        // comma); the observables block after it is not event data.
        if trimmed == "]" || trimmed == "]," {
            in_events = false;
            continue;
        }
        // An event object opens with `{` alone on the line and closes
        // with `},` or `}` (the harness's pretty renderer).
        if trimmed == "{" {
            if depth == 0 {
                current.clear();
            }
            depth += 1;
            continue;
        }
        if trimmed.starts_with('}') && depth > 0 {
            depth -= 1;
            if depth == 0 {
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
        if depth > 0
            && let Some((key, value)) = parse_field(trimmed)
        {
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
                open.result.is_err(),
                "the open answered success",
            ));
            if let Some(error) = open.result.as_ref().err() {
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
        Outcome::ErrorAfterOpenNoFlush { message_part } => {
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
            checks.push(Check::int(
                &format!("{what}: zero non-fatal damage (the fatal precedes any resync)"),
                non_fatal.len() as i64,
                0,
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
                    !non_fatal.is_empty(),
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
            // The negative-`biHeight` bottom-up DIB is recorded as a
            // ZERO-BYTE note: a legal idiom, zero units lost — the
            // wire's loss-counting damage event deliberately stays
            // silent (demux-level observability only).
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
            // The absolute-offset idx1 interpretation is a zero-byte
            // note — nothing lost, no wire damage (the scenario_09
            // matrix pins the same).
            damage: Damage::Zero,
            frames: 120,
            position_us: DECLARED_DURATION_US,
            fraction_below: None,
        },
    });
    assert_row("absidx1", &checks);
}

// ---------------------------------------------------------------------------
// The MP4 rows (6.OPW-mp4 wired the container host: demux-stage truth)
// ---------------------------------------------------------------------------

/// Every MP4 class-B derivative now goes through the WIRED mp4
/// container host (6.OPW-mp4): these rows pin the demux-stage behavior
/// that replaced G4's pre-wiring probe-surface scoping. The stale
/// blanket `probe.unsupported` expectation (written when "no MP4
/// demuxer exists" was true) is what W18-KK's local CI verification
/// caught — the billing-blocked workflow had never run the suite
/// since the wiring landed.
#[test]
fn matrix_mp4_trunc0_refuses_typed_at_demux() {
    // trunc0 is the 0-byte cut: no `moov` box survives — the refusal
    // names it (the probe still identifies mp4 by content/extension
    // hints, the wired host then refuses at the demux stage).
    let checks = drive(Row {
        fixture: "generated/malformed/mp4_mjpeg_pcm__trunc0.mp4",
        outcome: Outcome::Refused {
            stage: Stage::Demux,
            code: ErrorCode::Corrupt,
            message_part: "demux.mp4.no-moov",
        },
    });
    assert_row("trunc0", &checks);
}

#[test]
fn matrix_mp4_trunc1_refuses_typed_at_demux() {
    let checks = drive(Row {
        fixture: "generated/malformed/mp4_mjpeg_pcm__trunc1.mp4",
        outcome: Outcome::Refused {
            stage: Stage::Demux,
            code: ErrorCode::Corrupt,
            message_part: "demux.mp4.truncated",
        },
    });
    assert_row("trunc1", &checks);
}

#[test]
fn matrix_mp4_trunc2_opens_then_lands_in_error_typed() {
    let checks = drive(Row {
        fixture: "generated/malformed/mp4_mjpeg_pcm__trunc2.mp4",
        outcome: Outcome::ErrorAfterOpenNoFlush {
            message_part: "demux.mp4.truncated",
        },
    });
    assert_row("trunc2", &checks);
}

#[test]
fn matrix_mp4_trunc3_opens_then_lands_in_error_typed() {
    let checks = drive(Row {
        fixture: "generated/malformed/mp4_mjpeg_pcm__trunc3.mp4",
        outcome: Outcome::ErrorAfterOpenNoFlush {
            message_part: "demux.mp4.truncated",
        },
    });
    assert_row("trunc3", &checks);
}

#[test]
fn matrix_mp4_trunc4_opens_then_lands_in_error_typed() {
    // The 1-byte cut (the AVI trunc4 boundary-pin's mp4 counterpart):
    // unlike the AVI walker's clamp, the mp4 box walker treats the
    // missing final byte as fatal — open accepted, play-out runs to the
    // cut, then the typed `demux.mp4.truncated` refusal.
    let checks = drive(Row {
        fixture: "generated/malformed/mp4_mjpeg_pcm__trunc4.mp4",
        outcome: Outcome::ErrorAfterOpenNoFlush {
            message_part: "demux.mp4.truncated",
        },
    });
    assert_row("trunc4", &checks);
}

#[test]
fn matrix_mp4_garbage0_clean_ended_zero_damage() {
    let checks = drive(Row {
        fixture: "generated/malformed/mp4_mjpeg_pcm__garbage0.mp4",
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
fn matrix_mp4_flip1_refuses_typed_at_demux() {
    // The seeded flip lands in a box-size field: the walker's
    // box-overrun guard refuses the open naming the overrun.
    let checks = drive(Row {
        fixture: "generated/malformed/mp4_mjpeg_pcm__flip1.mp4",
        outcome: Outcome::Refused {
            stage: Stage::Demux,
            code: ErrorCode::Corrupt,
            message_part: "demux.mp4.box-overruns-parent",
        },
    });
    assert_row("flip1", &checks);
}

#[test]
fn matrix_mp4_flip4_refuses_typed_at_demux() {
    let checks = drive(Row {
        fixture: "generated/malformed/mp4_mjpeg_pcm__flip4.mp4",
        outcome: Outcome::Refused {
            stage: Stage::Demux,
            code: ErrorCode::Corrupt,
            message_part: "demux.mp4.box-overruns-parent",
        },
    });
    assert_row("flip4", &checks);
}

#[test]
fn matrix_mp4_flip16_refuses_typed_at_probe() {
    // Sixteen header-extent flips smash the signature past the probe's
    // recognition: the refusal names the sniffed bytes (the
    // known-format hint) — content truth, not extension truth.
    let checks = drive(Row {
        fixture: "generated/malformed/mp4_mjpeg_pcm__flip16.mp4",
        outcome: Outcome::Refused {
            stage: Stage::Probe,
            code: ErrorCode::Unsupported,
            message_part: "sniffed",
        },
    });
    assert_row("flip16", &checks);
}

#[test]
fn matrix_mp4_stszlie0_clean_ended_with_damage() {
    // The `stsz` sample-size lie: the demuxer resynchronizes and
    // delivers the full play-out, reporting the recovered damage
    // non-fatally (the huge unit count is the lie's magnitude).
    let checks = drive(Row {
        fixture: "generated/malformed/mp4_mjpeg_pcm__stszlie0.mp4",
        outcome: Outcome::CleanEnded {
            damage: Damage::AtLeastOne,
            frames: 120,
            position_us: DECLARED_DURATION_US,
            fraction_below: None,
        },
    });
    assert_row("stszlie0", &checks);
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
        open.result.is_err(),
        "the open answered success",
    ));
    if let Some(error) = open.result.as_ref().err() {
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

// ---------------------------------------------------------------------------
// Tier 2 — the randomized dirty corpus (env-gated; the D-11 mandate)
// ---------------------------------------------------------------------------

/// One manifest entry tier 2 consumes: the `dirty`, `dirty-lie-ext`
/// and `garbage` classes (avi-family-filtered for the dirty ones —
/// the today-testable surface: AVI demuxer + mjpeg/pcm decoders).
struct CorpusEntry {
    /// The corpus-relative path (`dirty/…`, `garbage/…`).
    rel: String,
    /// `dirty` | `dirty-lie-ext` | `garbage`.
    class: String,
    /// `family[0]` — the true container (content truth).
    container: String,
    /// `params.class` — `video` | `av` | `audio`.
    params_class: String,
    /// `params.codec` — the VIDEO codec (manifest truth, pre-corruption).
    video_codec: String,
    /// `params.audio_codec` — the audio codec, when the file has one.
    audio_codec: String,
    /// `corruption.op`, or `garbage` for the archetypes.
    op: String,
    /// `params.duration` (seconds) — the encoder's truth for the
    /// never-fabricated position bound on structure-intact rows.
    duration_s: f64,
}

/// Parse the corpus manifest (the generator's `indent=1` layout —
/// one field per line; the demux corpus.rs parser's shape). Entries
/// are attributed by the `params`/`corruption`/`family` brace
/// extents; `rejected_draws` records carry no `rel` and drop out.
fn parse_corpus_manifest(text: &str) -> Vec<CorpusEntry> {
    let mut out = Vec::new();
    let (mut rel, mut class) = (String::new(), String::new());
    let (mut container, mut params_class) = (String::new(), String::new());
    let (mut video_codec, mut audio_codec, mut op) = (String::new(), String::new(), String::new());
    let mut duration_s = 0.0_f64;
    let (mut in_params, mut in_corruption, mut in_family) = (false, false, false);
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("\"params\"") {
            in_params = true;
        } else if trimmed.starts_with("\"corruption\"") {
            in_corruption = true;
        } else if trimmed.starts_with("\"family\"") {
            in_family = true;
        } else if in_family {
            if trimmed.starts_with(']') {
                in_family = false;
            } else if container.is_empty() {
                container = json_unquote(trimmed);
            }
        } else if in_params {
            if trimmed.starts_with('}') {
                in_params = false;
            } else if let Some(rest) = trimmed.strip_prefix("\"class\":") {
                params_class = json_unquote(rest);
            } else if let Some(rest) = trimmed.strip_prefix("\"codec\":") {
                video_codec = json_unquote(rest);
            } else if let Some(rest) = trimmed.strip_prefix("\"audio_codec\":") {
                audio_codec = json_unquote(rest);
            } else if let Some(rest) = trimmed.strip_prefix("\"duration\":") {
                duration_s = rest.trim_end_matches(',').parse().unwrap_or(0.0);
            }
        } else if in_corruption {
            if trimmed.starts_with('}') {
                in_corruption = false;
            } else if let Some(rest) = trimmed.strip_prefix("\"op\":") {
                op = json_unquote(rest);
            }
        } else if trimmed.starts_with('}') {
            // A record close: file entries carry `rel`; the top level
            // and `rejected_draws` do not.
            if !rel.is_empty() && !class.is_empty() {
                if class == "garbage" {
                    op = "garbage".to_string();
                }
                out.push(CorpusEntry {
                    rel: rel.clone(),
                    class: class.clone(),
                    container: container.clone(),
                    params_class: params_class.clone(),
                    video_codec: video_codec.clone(),
                    audio_codec: audio_codec.clone(),
                    op: op.clone(),
                    duration_s,
                });
            }
            rel.clear();
            class.clear();
            container.clear();
            params_class.clear();
            video_codec.clear();
            audio_codec.clear();
            op.clear();
            duration_s = 0.0;
        } else if let Some(rest) = trimmed.strip_prefix("\"rel\":") {
            rel = json_unquote(rest);
        } else if let Some(rest) = trimmed.strip_prefix("\"class\":") {
            class = json_unquote(rest);
        }
    }
    out
}

/// The first double-quoted value of a `"key": "value",` remainder
/// (array items arrive bare — `"avi",`).
fn json_unquote(rest: &str) -> String {
    let rest = rest.trim_start();
    let Some(start) = rest.find('"') else {
        return String::new();
    };
    let tail = &rest[start + 1..];
    match tail.find('"') {
        Some(end) => tail[..end].to_string(),
        None => String::new(),
    }
}

/// Whether the v0 host registry can wire EVERY audio/video track the
/// manifest declares (`mjpeg` video + `pcm_*` audio — pipeline_host's
/// first-vertical filter). An undecodable track in the list fails the
/// whole open (`wire`'s per-track `?`), so this is the codec axis of
/// the decision procedure.
fn wireable(entry: &CorpusEntry) -> bool {
    let video_ok = match entry.params_class.as_str() {
        "video" | "av" => entry.video_codec == "mjpeg",
        _ => true,
    };
    let audio_ok = entry.params_class != "av"
        || entry.audio_codec.is_empty()
        || entry.audio_codec.starts_with("pcm_");
    video_ok && audio_ok
}

/// The observed outcome class of one corpus run (the decision
/// procedure's left side — what the full stack actually did).
struct CorpusObservation {
    /// `refused` | `error-state` | `clean-ended` | `clean-ended-zero-damage`.
    outcome: &'static str,
    /// The refusal's `stage.code`, or — for playing outcomes — the
    /// fatal's `stage.code` triage trail (empty when neither).
    refusal: String,
    /// The coalesced non-fatal damage count (blank when absent).
    damage_count: String,
    fatal: usize,
    non_fatal: usize,
    end_of_media: usize,
    duration_known: usize,
    position_us: i64,
    frames: i64,
    elapsed_ms: u128,
}

/// The per-file wall budget: 2× the declared duration + 5 s (the
/// audio-bearing rows drain by pumps alone; the video-only lane is
/// wall-clock-paced by design — the null device presents per the
/// system monotonic — so its rows consume real seconds).
fn wall_budget_ms(entry: &CorpusEntry) -> u128 {
    (entry.duration_s.max(0.0) as u128) * 2_000 + 5_000
}

/// Drive one corpus entry through the full stack and classify what
/// happened (never asserting — the caller owns the verdict).
fn drive_corpus_entry(root: &std::path::Path, entry: &CorpusEntry) -> CorpusObservation {
    let started = Instant::now();
    let path = root.join(&entry.rel);
    // The harness records the fixture name in its transcript; corpus
    // paths are runtime strings, so each is leaked once (bounded by
    // the case count — the demux corpus suite's own allocation
    // posture).
    let name: &'static str = Box::<str>::leak(String::clone(&entry.rel).into_boxed_str());
    let source = SourceSpec::File { path };
    let mut harness = SessionHarness::new(name);
    let open = harness.dispatch(
        "session.open",
        Command::SessionOpen {
            source,
            options: OpenOptions::default(),
        },
    );
    if let Some(error) = open.result.as_ref().err() {
        let refusal = format!("{}.{}", error.stage.as_str(), error.code.as_str());
        drop(harness);
        return CorpusObservation {
            outcome: "refused",
            refusal,
            damage_count: String::new(),
            fatal: 0,
            non_fatal: 0,
            end_of_media: 0,
            duration_known: 0,
            position_us: 0,
            frames: 0,
            elapsed_ms: started.elapsed().as_millis(),
        };
    }
    harness.settle();
    // The bounded run-to-terminal: pump chunks (audio-paced rows
    // drain in a couple) with breaths for the wall-clock-paced
    // video-only lane; the budget IS the never-hang term.
    let budget = wall_budget_ms(entry);
    while !matches!(
        harness.state(),
        commands::PlaybackState::Ended | commands::PlaybackState::Error
    ) && started.elapsed().as_millis() < budget
    {
        harness.advance(run_pumps());
        if !matches!(
            harness.state(),
            commands::PlaybackState::Ended | commands::PlaybackState::Error
        ) {
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    }
    let end_state = harness.state();
    let end_position = harness.position_us();
    let transcript = harness.finish("recovery-corpus").transcript_bytes();
    let events = event_records(&transcript);
    let fatal = events
        .iter()
        .filter(|r| r.kind == "playback.error" && r.fatal.as_deref() == Some("true"))
        .count();
    let non_fatal = events
        .iter()
        .filter(|r| r.kind == "playback.error" && r.fatal.as_deref() == Some("false"))
        .count();
    // The triage trail: the fatal's stage.code and the coalesced
    // damage count (blank when absent).
    let fatal_code = events
        .iter()
        .find(|r| r.kind == "playback.error" && r.fatal.as_deref() == Some("true"))
        .map_or_else(String::new, |r| format!("{}.{}", r.stage, r.code));
    let damage_count = events
        .iter()
        .find(|r| r.kind == "playback.error" && r.fatal.as_deref() == Some("false"))
        .and_then(|r| r.count.clone())
        .unwrap_or_default();
    let end_of_media = events
        .iter()
        .filter(|r| r.kind == "playback.end-of-media")
        .count();
    let duration_known = events
        .iter()
        .filter(|r| r.kind == "timing.duration-known")
        .count();
    let frames = events_position_frames(&transcript);
    let outcome = match end_state {
        commands::PlaybackState::Ended if non_fatal == 0 => "clean-ended-zero-damage",
        commands::PlaybackState::Ended => "clean-ended",
        commands::PlaybackState::Error => "error-state",
        other => {
            // Not a terminal: the budget expired mid-play (the
            // never-hang term tripped). Classified as a hang for the
            // verdict — never silently passed.
            let _ = other;
            "hang-suspect"
        }
    };
    CorpusObservation {
        outcome,
        refusal: fatal_code,
        fatal,
        non_fatal,
        end_of_media,
        duration_known,
        position_us: end_position,
        frames,
        elapsed_ms: started.elapsed().as_millis(),
        damage_count,
    }
}

/// The decision procedure: the allowed outcome classes for one
/// (corruption op × wireability) pair. A randomized corruption lands
/// anywhere in the file, so mid-file ops allow the full
/// typed-refusal/ErrorState/CleanEnded union; the structure-intact
/// classes (tail-garbage, lie-ext) pin strictly. Answers `None` when
/// the observed class is legal — the violation text otherwise.
fn verdict(entry: &CorpusEntry, seen: &CorpusObservation) -> Result<(), String> {
    let wire = wireable(entry);
    let op = entry.op.as_str();
    // The never-terms, every row.
    if seen.outcome == "hang-suspect" {
        return Err(format!(
            "{} ({}): no terminal within the wall budget",
            entry.rel, op
        ));
    }
    if seen.outcome == "refused" {
        if seen.end_of_media != 0 || seen.frames != 0 {
            return Err(format!(
                "{}: refused open yet {} end-of-media / {} frames — fabricated output",
                entry.rel, seen.end_of_media, seen.frames
            ));
        }
        // The content-truth pin for lie-ext: the refusal must come
        // from the codec axis (decode stage), never a probe-stage
        // format refusal — the probe read the AVI bytes, not the
        // lying name (adversarial_gaps family 4's engine half).
        if op == "lie-ext" && seen.refusal.starts_with("probe.") {
            return Err(format!(
                "{}: probe-stage refusal `{}` — the engine trusted the lying extension",
                entry.rel, seen.refusal
            ));
        }
        let allowed_stages: &[&str] = match (op, wire) {
            ("smash-head", _) => &["probe.", "demux."],
            ("garbage", _) => &["probe.", "demux.", "input."],
            // Mid-file corruption can hit the magic (probe), the
            // container headers (demux), or the codec facts
            // (decode — a zeroed strf answers garbage pcm facts).
            ("flip-bytes", _) => &["probe.", "demux.", "decode."],
            ("truncate" | "zero-mid" | "cut-mid-chunk", _) => &["demux.", "decode."],
            // Structure-intact classes on an unwireable codec: the
            // refusal is the codec axis, exactly.
            ("tail-garbage" | "lie-ext", false) => &["decode."],
            ("tail-garbage" | "lie-ext", true) => {
                return Err(format!(
                    "{} ({}, wireable): refused `{}` — the intact structure must play",
                    entry.rel, op, seen.refusal
                ));
            }
            (other, _) => return Err(format!("{}: unknown corruption op `{other}`", entry.rel)),
        };
        if !allowed_stages
            .iter()
            .any(|prefix| seen.refusal.starts_with(prefix))
        {
            return Err(format!(
                "{} ({}): refusal `{}` outside the allowed stages {:?}",
                entry.rel, op, seen.refusal, allowed_stages
            ));
        }
        return Ok(());
    }
    // Playing outcomes (ErrorState / CleanEnded) — only wireable
    // codecs reach them.
    if !wire {
        return Err(format!(
            "{} ({}): outcome `{}` on an unwireable codec ({} + {}) — the open must refuse typed",
            entry.rel, op, seen.outcome, entry.video_codec, entry.audio_codec
        ));
    }
    match seen.outcome {
        "error-state" => {
            if op == "tail-garbage" || op == "lie-ext" {
                return Err(format!(
                    "{} ({}): intact structure landed in ErrorState",
                    entry.rel, op
                ));
            }
            if seen.fatal != 1 {
                return Err(format!(
                    "{}: ErrorState with {} fatal events (exactly 1 is the contract)",
                    entry.rel, seen.fatal
                ));
            }
            if seen.end_of_media != 0 {
                return Err(format!(
                    "{}: ErrorState with {} end-of-media events",
                    entry.rel, seen.end_of_media
                ));
            }
        }
        "clean-ended" | "clean-ended-zero-damage" => {
            if seen.fatal != 0 {
                return Err(format!(
                    "{}: clean end with {} fatal events",
                    entry.rel, seen.fatal
                ));
            }
            if seen.end_of_media != 1 {
                return Err(format!(
                    "{}: {} end-of-media events (exactly 1 — never fabricated)",
                    entry.rel, seen.end_of_media
                ));
            }
            if seen.duration_known > 1 {
                return Err(format!(
                    "{}: {} duration-known events (declared extents answer once)",
                    entry.rel, seen.duration_known
                ));
            }
            if seen.non_fatal > 1 {
                return Err(format!(
                    "{}: {} non-fatal damage events (the frozen throttle coalesces to one terminal flush)",
                    entry.rel, seen.non_fatal
                ));
            }
            // The never-fabricated position bound, structure-intact
            // classes only (mid-file corruption may legitimately
            // rewrite the declared extents the position is judged
            // against — those rows record the delta instead).
            if op == "tail-garbage" || op == "lie-ext" {
                let duration_us = (entry.duration_s * 1_000_000.0) as i64;
                if seen.position_us > duration_us + 100_000 {
                    return Err(format!(
                        "{}: final position {} µs exceeds the declared {} µs — fabricated play-out",
                        entry.rel, seen.position_us, duration_us
                    ));
                }
            }
        }
        other => return Err(format!("{}: unclassified outcome `{other}`", entry.rel)),
    }
    Ok(())
}

/// The env-gated tier-2 run (the corpus.rs ignored-by-design pattern
/// — the local gate never depends on a generated corpus existing):
///
/// ```text
/// LUMEN_CORPUS=/home/z/my-project/corpus-big \
///   [LUMEN_CORPUS_RESULTS=<tsv>] \
///   cargo test -p first-vertical --test recovery_matrix -- --ignored --nocapture
/// ```
///
/// Every avi-family dirty/dirty-lie-ext sibling and every garbage
/// archetype in the corpus manifest is driven through the full stack;
/// the observed outcome class must sit in the decision procedure's
/// allowed set for its (corruption op × wireability) pair, plus the
/// never-terms. The per-class counts (op → outcome → count) print to
/// stderr and append to the results TSV — the real numbers D-11
/// demands, not a green checkmark.
#[test]
#[ignore = "randomized-corpus tier 2: set LUMEN_CORPUS to activate"]
fn corpus_dirty_outcome_classes_hold() {
    let Some(corpus) = std::env::var_os("LUMEN_CORPUS") else {
        eprintln!("corpus: LUMEN_CORPUS not set — no-op");
        return;
    };
    let root = std::path::PathBuf::from(corpus);
    let manifest = std::fs::read_to_string(root.join("manifest.json")).must();
    let entries: Vec<CorpusEntry> = parse_corpus_manifest(&manifest)
        .into_iter()
        .filter(|entry| {
            entry.class == "garbage"
                || ((entry.class == "dirty" || entry.class == "dirty-lie-ext")
                    && entry.container == "avi")
        })
        .collect();
    assert!(
        !entries.is_empty(),
        "corpus manifest carries no avi-family dirty / garbage entries"
    );
    let results = std::env::var_os("LUMEN_CORPUS_RESULTS").map(std::path::PathBuf::from);
    let mut out = results.map(|path| {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .must();
        eprintln!("corpus(tier-2): {} cases", entries.len());
        file
    });
    if let Some(file) = out.as_mut() {
        use std::io::Write as _;
        writeln!(file, "rel\tclass\top\twireable\toutcome\trefusal_or_fatal\tfatal\tnon_fatal\tdamage_count\tend_of_media\tduration_known\tposition_us\tframes\telapsed_ms").must();
    }
    // The per-class count matrix (op → outcome → count) — printed at
    // the end so a failing run still shows its numbers.
    let mut counts: Vec<((String, String), usize)> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    for entry in &entries {
        let path = root.join(&entry.rel);
        assert!(path.is_file(), "corpus file missing: {path:?}");
        let seen = drive_corpus_entry(&root, entry);
        let key = (entry.op.clone(), seen.outcome.to_string());
        match counts.iter_mut().find(|(k, _)| *k == key) {
            Some((_, count)) => *count += 1,
            None => counts.push((key, 1)),
        }
        if let Some(violation) = verdict(entry, &seen).err() {
            failures.push(violation);
        }
        eprintln!(
            "corpus(tier-2): {} {} wireable={} -> {} {} (fatal {} / non-fatal {} count {} / eom {} / pos {} / frames {} / {} ms)",
            entry.op,
            entry.rel,
            wireable(entry),
            seen.outcome,
            seen.refusal,
            seen.fatal,
            seen.non_fatal,
            seen.damage_count,
            seen.end_of_media,
            seen.position_us,
            seen.frames,
            seen.elapsed_ms
        );
        if let Some(file) = out.as_mut() {
            use std::io::Write as _;
            writeln!(
                file,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                entry.rel,
                entry.class,
                entry.op,
                wireable(entry),
                seen.outcome,
                seen.refusal,
                seen.fatal,
                seen.non_fatal,
                seen.damage_count,
                seen.end_of_media,
                seen.duration_known,
                seen.position_us,
                seen.frames,
                seen.elapsed_ms
            )
            .must();
        }
    }
    eprintln!("corpus(tier-2): the outcome-class matrix (op -> outcome -> count):");
    let mut sorted = counts.clone();
    sorted.sort();
    for ((op, outcome), count) in &sorted {
        eprintln!("corpus(tier-2):   {op:16} {outcome:24} x{count}");
    }
    assert!(
        failures.is_empty(),
        "{} of {} corpus rows violated the decision procedure:\n{}",
        failures.len(),
        entries.len(),
        failures.join("\n")
    );
}
