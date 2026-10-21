//! The class-B corpus + the missing-file path, through the FULL stack
//! (the CLI's `unsupported_path` covers argv-level; these drive the
//! same failures through the command layer in-process): every
//! derivative satisfies the union contract of its expectation TOML —
//! typed error OR bounded recovery, never panic, never hang, never
//! unbounded allocation (the bounded pump counts make "never hang"
//! mechanical).

mod common;

use common::assert_green;
use first_vertical::scenarios::{
    scenario_corrupt_garbage_appended, scenario_corrupt_noidx, scenario_corrupt_truncated_midplay,
    scenario_corrupt_truncated_open, scenario_missing_file,
};

/// `missing-file`: the input stage's typed inaccessible, redacted.
#[test]
fn class_b_missing_file() {
    assert_green(&scenario_missing_file());
}

/// `corrupt-truncated-open`: the 52-byte truncation (headers
/// incomplete) — the demuxer's typed corrupt refusal at open.
#[test]
fn class_b_truncated_open() {
    assert_green(&scenario_corrupt_truncated_open());
}

/// `corrupt-truncated-midplay`: the 2/3 truncation (movi cut
/// mid-stream) — opens, plays, then the typed demux error surfaces as
/// a `playback.error` event and the session lands in Error.
#[test]
fn class_b_truncated_midplay() {
    assert_green(&scenario_corrupt_truncated_midplay());
}

/// `corrupt-garbage-appended`: 4 KiB past every declared structure
/// end — bounded recovery, plays to the exact end.
#[test]
fn class_b_garbage_appended() {
    assert_green(&scenario_corrupt_garbage_appended());
}

/// `corrupt-noidx`: idx1 dropped (RIFF size patched, avih still
/// claims AVIF_HASINDEX) — the linear movi walk recovers the full
/// media with the whole accounting closing.
#[test]
fn class_b_noidx() {
    assert_green(&scenario_corrupt_noidx());
}
