//! Scenario 9 — **corrupt file** (doc 10 §6.4 row 9): the class-B
//! derivative fixtures (§5.3) plus byte-level corruption applied at
//! load in a temp copy (documented offsets), every one driven
//! through the command layer.
//!
//! Pass criteria: typed `Corrupt`; bounded recovery count; no
//! hang/crash. The expectation files pin the UNION contract
//! ("typed-error-or-bounded-recovery"; never panic, hang, unbounded
//! allocation) — this suite pins the SPECIFIC observed flavor per
//! fixture, so any drift in recovery behavior fails loudly instead
//! of hiding behind the union.
//!
//! The observed flavors (all real, pinned by the tuning run this
//! file was adjusted against — the matrix is the evidence in
//! `docs/scenarios/scenarios-b-report.md`):
//!
//! - `Refused` — the open itself answers the typed corrupt refusal;
//!   nothing plays, the host is `Idle`, no session survives.
//! - `Contained` — the open lands, a bounded prefix decodes, the
//!   session ends CLEANLY (`Playing → Ended` + exactly one
//!   `playback.end-of-media`), with at most one NON-FATAL
//!   `playback.error` — the coalesced "recovered N damaged unit(s)"
//!   accounting (ED-FIX-3; the 2.B2-era fatal contract is
//!   superseded).

use commands::envelope::CommandOutcome;
use commands::{Command, PlaybackState};
use testkit::alloc_count::serialize_tests;
use testkit::Must;

use crate::alloc_state;
use crate::host::{count_events, corrupt_bytes, dispatch, error_events, ScenarioHost, TempDir, PARENT};

/// One row's pinned expectation.
#[derive(Debug, Clone, PartialEq)]
enum Pinned {
    /// The open refuses typed (`stage.code`); nothing plays.
    Refused(&'static str, &'static str),
    /// Bounded play-out to a clean `Ended` with exactly
    /// `damage_events` non-fatal accountings and at least
    /// `min_frames` delivered.
    Contained { damage_events: usize, min_frames: u64 },
}

/// One matrix row.
struct Row {
    /// The repo-relative fixture name (committed class-B set).
    fixture: &'static str,
    expect: Pinned,
}

/// What one row actually did (the observed truth).
#[derive(Debug, Clone, PartialEq)]
enum Observed {
    Refused(String),
    Contained { damage: usize, frames: u64 },
    Fatal(String),
}

/// The committed class-B AVI matrix.
fn avi_rows() -> Vec<Row> {
    let refused = |name: &'static str| Row {
        fixture: name,
        expect: Pinned::Refused("demux", "corrupt"),
    };
    let contained = |name: &'static str, damage_events: usize, min_frames: u64| Row {
        fixture: name,
        expect: Pinned::Contained {
            damage_events,
            min_frames,
        },
    };
    vec![
        refused("generated/malformed/avi_mjpeg_pcm__trunc0.avi"),
        refused("generated/malformed/avi_mjpeg_pcm__trunc1.avi"),
        refused("generated/malformed/avi_mjpeg_pcm__trunc2.avi"),
        contained("generated/malformed/avi_mjpeg_pcm__trunc3.avi", 1, 60),
        contained("generated/malformed/avi_mjpeg_pcm__trunc4.avi", 1, 118),
        contained("generated/malformed/avi_mjpeg_pcm__flip1.avi", 1, 100),
        contained("generated/malformed/avi_mjpeg_pcm__flip4.avi", 1, 100),
        refused("generated/malformed/avi_mjpeg_pcm__flip16.avi"),
        contained("generated/malformed/avi_mjpeg_pcm__garbage0.avi", 1, 118),
        contained("generated/malformed/avi_mjpeg_pcm__noidx0.avi", 1, 118),
        contained("generated/malformed/avi_mjpeg_pcm__absidx1.avi", 0, 118),
        contained("generated/malformed/avi_mjpeg_pcm__negheight0.avi", 1, 59),
    ]
}

/// The committed class-B MP4 matrix: every row refuses at the probe
/// stage (this build's first vertical serves AVI only, D-018 — the
/// same typed answer the real host gives an mp4).
fn mp4_rows() -> Vec<Row> {
    [
        "trunc0", "trunc1", "trunc2", "trunc3", "trunc4", "flip1", "flip4", "flip16", "garbage0",
        "stszlie0",
    ]
    .iter()
    .map(|op| Row {
        fixture: format!("generated/malformed/mp4_mjpeg_pcm__{op}.mp4").leak(),
        expect: Pinned::Refused("probe", "unsupported"),
    })
    .collect()
}

/// One byte-level temp-copy corruption (offsets from the committed
/// structure map of the class-A parent).
struct Patch {
    tag: &'static str,
    offset: usize,
    len: usize,
    seed: u8,
    expect: Pinned,
}

/// The documented corruptions:
///
/// - `movi-payload` — 8 patterned bytes at offset 532 (the first
///   `00dc` chunk's JPEG entropy data, 200 bytes into the payload):
///   decoder-level damage, the concealment path.
/// - `avih-garbage` — 8 patterned bytes at offset 32 (the `avih`
///   payload's front): header-level corruption.
/// - `idx1-offset-lie` — the first idx1 entry's `dwOffset` (offset
///   423_604) patched to 0xFFFFFFF0-ish: the lying-index variant.
fn patches() -> Vec<Patch> {
    vec![
        Patch {
            tag: "movi-payload",
            offset: 532,
            len: 8,
            seed: 0x5A,
            expect: Pinned::Contained {
                damage_events: 1,
                min_frames: 100,
            },
        },
        Patch {
            tag: "avih-garbage",
            offset: 32,
            len: 8,
            seed: 0xC3,
            expect: Pinned::Refused("demux", "corrupt"),
        },
        Patch {
            tag: "idx1-offset-lie",
            offset: 423_604,
            len: 4,
            seed: 0xF0,
            expect: Pinned::Contained {
                damage_events: 1,
                min_frames: 100,
            },
        },
    ]
}

/// Open one source through the command layer and drive it to its
/// terminal (bounded — a hang is a bound breach, never a mystery).
fn open_and_drive(host: &mut ScenarioHost, path: std::path::PathBuf) -> Observed {
    let open = dispatch(
        host,
        Command::SessionOpen {
            source: commands::catalog::types::SourceSpec::File { path },
            options: Default::default(),
        },
    );
    match &open.result {
        Err(error) => {
            let context = format!(
                "state {:?}, session {:?}",
                host.state(),
                host.session_id()
            );
            assert_eq!(host.state(), PlaybackState::Idle, "after refusal: {context}");
            assert!(
                host.session_id().is_none(),
                "no session survives a refusal: {context}"
            );
            assert_eq!(
                count_events(host, "playback.error"),
                0,
                "an open refusal emits no pipeline events"
            );
            Observed::Refused(format!("{}.{}", error.stage.as_str(), error.code.as_str()))
        }
        Ok(CommandOutcome::Accepted { .. }) => {
            host.settle();
            let (pumps, terminal) = host.run_until_terminal(512);
            let errors = error_events(host);
            let damage = errors.iter().filter(|(_, fatal, _)| !fatal).count();
            let fatal = errors.iter().filter(|(_, fatal, _)| *fatal).count();
            let frames = host.sink().map(|sink| sink.frames_accepted()).must();
            match terminal {
                PlaybackState::Ended => {
                    assert_eq!(
                        count_events(host, "playback.end-of-media"),
                        1,
                        "containment delivers exactly one end-of-media ({pumps} pumps)"
                    );
                    Observed::Contained { damage, frames }
                }
                PlaybackState::Error => {
                    assert_eq!(fatal, 1, "exactly one fatal error event ({pumps} pumps)");
                    Observed::Fatal(errors[0].0.clone())
                }
                other => unreachable!("terminal must be Ended or Error, got {other:?}"),
            }
        }
        Ok(_) => unreachable!("open answers accepted or error"),
    }
}

/// Assert one observed flavor against its pinned expectation (the
/// never-weakened row check: flavor drift is a failure, not a
/// re-pinning).
fn assert_row(name: &str, observed: &Observed, pinned: &Pinned) {
    match (observed, pinned) {
        (Observed::Refused(got), Pinned::Refused(stage, code)) => {
            assert_eq!(got, &format!("{stage}.{code}"), "{name}: refusal flavor");
        }
        (
            Observed::Contained { damage, frames },
            Pinned::Contained {
                damage_events,
                min_frames,
            },
        ) => {
            assert_eq!(damage, damage_events, "{name}: damage-accounting count");
            assert!(
                *frames >= *min_frames,
                "{name}: bounded play-out floor {frames} < {min_frames}"
            );
            assert!(
                *frames <= 121,
                "{name}: bounded recovery, not a runaway ({frames} frames)"
            );
        }
        (observed, pinned) => unreachable!(
            "{name}: flavor drift — pinned {pinned:?}, observed {observed:?}"
        ),
    }
}

/// The full committed class-B matrix: every row's observed flavor
/// matches its pinned expectation, and the universal invariants hold
/// (bounded pumps, no panic, legal terminal, memory-neutral).
#[test]
fn corrupt_media_is_typed_bounded_and_memory_neutral() {
    let _serial = serialize_tests();
    let scratch = TempDir::new("s09");

    // Warm one valid open first so the matrix measures steady state
    // (pool sizing, the decoder's first-decode allocations).
    let mut warm = ScenarioHost::new();
    let parent = testkit::fixture_path(PARENT).must();
    let observed = open_and_drive(&mut warm, parent);
    assert_eq!(
        observed,
        Observed::Contained {
            damage: 0,
            frames: 120
        },
        "the warm-up parent plays clean to the end"
    );
    let close = dispatch(&mut warm, Command::SessionClose);
    assert!(close.result.is_ok(), "parent closes: {:?}", close.result);
    let baseline = alloc_state().snapshot().live_bytes;

    let mut rows = avi_rows();
    rows.extend(mp4_rows());
    for row in &rows {
        let path = testkit::fixture_path(row.fixture).must();
        let mut host = ScenarioHost::new();
        let observed = open_and_drive(&mut host, path);
        assert_row(row.fixture, &observed, &row.expect);
        // Every opened row closes cleanly (the matrix leaves no
        // sessions behind).
        if host.session_id().is_some() {
            let close = dispatch(&mut host, Command::SessionClose);
            assert!(close.result.is_ok(), "{}: closes: {:?}", row.fixture, close.result);
        }
    }

    for patch in patches() {
        let copy = scratch.copy_parent(&format!("corrupt-{}.avi", patch.tag));
        let mut bytes = std::fs::read(&copy).must();
        corrupt_bytes(&mut bytes, patch.offset, patch.len, patch.seed);
        std::fs::write(&copy, bytes).must();
        let mut host = ScenarioHost::new();
        let observed = open_and_drive(&mut host, copy);
        let name = format!("temp-copy:{}", patch.tag);
        assert_row(&name, &observed, &patch.expect);
        if host.session_id().is_some() {
            let close = dispatch(&mut host, Command::SessionClose);
            assert!(close.result.is_ok(), "{name}: closes: {:?}", close.result);
        }
    }

    // D-2 bounded memory: after every row's teardown the allocator's
    // live bytes are back at the warm-up baseline.
    let after = alloc_state().snapshot().live_bytes;
    assert_eq!(
        after, baseline,
        "the corrupt matrix is memory-neutral (live bytes {baseline} → {after})"
    );
}
