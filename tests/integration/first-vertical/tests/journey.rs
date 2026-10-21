//! The journey scenarios (doc 10 §6.4's Phase-2 rows): open → play →
//! pause → seek → stop → run-to-end — every one through the ONE
//! command layer, every observable from the host's wire surfaces.

mod common;

use common::assert_green;
use first_vertical::scenarios::{
    scenario_open, scenario_pause, scenario_play, scenario_run_to_end, scenario_seek, scenario_stop,
};

/// `open`: the generated fixture opens through the real stack and the
/// class-S snapshots report the container's declared truth.
#[test]
fn journey_open() {
    assert_green(&scenario_open());
}

/// `play`: after a pause, resume flows samples again (the null audio
/// device renders quanta — the audio-master clock moves) and frames
/// are delivered on both limbs.
#[test]
fn journey_play() {
    assert_green(&scenario_play());
}

/// `pause`: the clock freezes exactly — a long bounded advance while
/// paused moves the position by nothing.
#[test]
fn journey_pause() {
    assert_green(&scenario_pause());
}

/// `seek`: the 50% target lands at exactly half the duration under a
/// new session id, and the remainder plays to the exact duration.
#[test]
fn journey_seek() {
    assert_green(&scenario_seek());
}

/// `stop`: teardown truth — Stopped with the position frozen,
/// snapshots answering while the session lives, the honest not-ready
/// after `session.close`.
#[test]
fn journey_stop() {
    assert_green(&scenario_stop());
}

/// `run-to-end`: THE vertical — whole media demuxed/decoded/output/
/// presented, the per-frame accounting closing, the position landing
/// on the duration exactly, the final PNG carrying the media's extent.
#[test]
fn journey_run_to_end() {
    assert_green(&scenario_run_to_end());
}

/// In-process determinism, suite-level: two runs of the full journey
/// produce byte-identical transcripts (the poll model's own AC,
/// re-proven at the integration level).
#[test]
fn journey_transcripts_are_deterministic() {
    let first = scenario_run_to_end();
    let second = scenario_run_to_end();
    assert_green(&first);
    assert_green(&second);
    assert_eq!(
        first.transcript_bytes(),
        second.transcript_bytes(),
        "two run-to-end transcripts must be byte-identical"
    );
}
