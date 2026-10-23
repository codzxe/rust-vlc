//! The soak scenario (card 3.E4): the deterministic open → play →
//! seek → track-switch → reopen loop script, and its per-fixture plan
//! parsed from the committed expectation TOMLs (the recorded truth —
//! seek targets and track ids are read from the fixtures' own
//! expectation files, never transcribed; the
//! `tests/track_switch_fixture.rs` precedent).
//!
//! # The op order (why seek/switch come EARLY)
//!
//! The committed fixtures are ~2-second containers and the soak's
//! sink accepts frames the moment they decode (a collecting sink
//! would itself be the harness's own leak — see `dispatch.rs`), so
//! the pipeline drains to `Ended` within a tick or two of priming.
//! The seek and switch ops are therefore submitted immediately off
//! the open's first mid-media settle and off the seek's landing —
//! the states the transition table serves — with the skip lanes
//! counted and bounded by the liveness asserts (D-6). Measured with
//! ops-after-advance: 1597/1597 cycles skipped the seek (100%).
//!
//! # Determinism
//!
//! The op script is a pure function of the cycle index (fixture
//! rotation, seek-target rotation, switch direction) over the frozen
//! virtual clock (`dispatch::FrozenNow`) — the same input always
//! replays the same op sequence. The **memory measurement is NOT part
//! of the script**: the host samples on the real wall clock from
//! inside every `pump` (see `dispatch::Sampling`).
//!
//! # Liveness is asserted, not assumed (D-6)
//!
//! Every cycle must actually decode frames (the counting sink must
//! advance), every op must either complete or be skipped for a state
//! reason the transition table itself names, and the completed-op
//! counts are the report's loop-count axis. A soak that skips its ops
//! is a ritual, not a measurement — the scenario fails loudly on that.

use commands::catalog::types::{SeekTarget as WireSeekTarget, SourceSpec, Timestamp};
use commands::envelope::RequestId;
use commands::{Command, WireRequest};
use playback_engine::state::SessionState;
use testkit::Must;

use super::dispatch::{SoakHost, lumen_soak_dispatch};
use super::memory::{Phase, PhaseAccumulator};

/// Polls of playback advance between ops (bounded, small — the play
/// step between the seek/switch ops and the play-to-end drain).
const ADVANCE_POLLS: usize = 3;
/// Bound for the final play-to-end wait of a cycle.
const PLAY_TO_END_POLLS: usize = 1024;

// ---------------------------------------------------------------------------
// The fixture plans (parsed from the committed expectation TOMLs)
// ---------------------------------------------------------------------------

/// One soak fixture: its file, its recorded seek targets, and its
/// audio track menu.
#[derive(Debug, Clone)]
pub struct FixturePlan {
    /// The fixture's repo-relative path under `tests/fixtures/`.
    pub fixture_rel: String,
    /// The report label.
    pub label: &'static str,
    /// The recorded `[seek] targets_us` (µs), in file order.
    pub seek_targets_us: Vec<i64>,
    /// The audio track handles (from `[tracks] audio`).
    pub audio_ids: Vec<u32>,
    /// The multi-track switch target (`[switch_plan] to_track_id`),
    /// when the fixture carries one.
    pub switch_to: Option<u32>,
}

impl FixturePlan {
    /// Parse a plan from the fixture's expectation TOML (the minimal
    /// line-scanner style of `tests/track_switch_fixture.rs` — no
    /// dependency, and the parsing is part of the proof surface).
    ///
    /// Dirty class-B fixtures carry `[derivation]`/`[error]` instead
    /// of `[tracks]`/`[seek]`; their recorded `parent` names the
    /// valid parent whose facts (same container head) fill the
    /// missing sections — the derivation's own recorded truth.
    ///
    /// # Errors
    /// A human-readable sentence naming the missing fact (a broken
    /// fixture means a broken test setup).
    pub fn from_expectation(
        fixture_rel: &str,
        expectation_rel: &str,
        label: &'static str,
    ) -> Result<FixturePlan, String> {
        let mut plan = parse_expectation(fixture_rel, expectation_rel, label)?;
        if plan.seek_targets_us.is_empty() || plan.audio_ids.is_empty() {
            let parent_rel = parent_of(expectation_rel)?;
            let parent = parse_expectation(fixture_rel, &parent_rel, label)?;
            if plan.seek_targets_us.is_empty() {
                plan.seek_targets_us = parent.seek_targets_us;
            }
            if plan.audio_ids.is_empty() {
                plan.audio_ids = parent.audio_ids;
            }
            plan.switch_to = plan.switch_to.or(parent.switch_to);
        }
        if plan.seek_targets_us.is_empty() {
            return Err(format!(
                "{expectation_rel}: no [seek] targets_us recorded (directly or via the parent)"
            ));
        }
        if plan.audio_ids.is_empty() {
            return Err(format!(
                "{expectation_rel}: no [tracks] audio ids recorded (directly or via the parent)"
            ));
        }
        Ok(plan)
    }

    /// The absolute fixture path (existence-checked by `testkit`).
    ///
    /// # Errors
    /// The fixture-missing sentence.
    pub fn path(&self) -> Result<std::path::PathBuf, String> {
        testkit::fixture_path(&self.fixture_rel).map_err(|error| error.to_string())
    }
}

/// The raw expectation scan (one pass; sections `[seek]`, `[tracks]`,
/// `[switch_plan]`).
fn parse_expectation(
    fixture_rel: &str,
    expectation_rel: &str,
    label: &'static str,
) -> Result<FixturePlan, String> {
    let text = std::fs::read_to_string(
        testkit::fixture_path(expectation_rel).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("{expectation_rel}: {error}"))?;
    let mut seek_targets_us = Vec::new();
    let mut audio_ids = Vec::new();
    let mut switch_to = None;
    let mut section = "";
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            section = line.trim_start_matches('[').trim_end_matches(']');
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match (section, key.trim()) {
            ("seek", "targets_us") => {
                // targets_us = [0, 500_000, 1_000_000, 1_999_000]
                let inner = value.trim().trim_start_matches('[').trim_end_matches(']');
                for item in inner.split(',') {
                    let digits: String = item
                        .chars()
                        .filter(|c| c.is_ascii_digit() || *c == '-')
                        .collect();
                    if !digits.is_empty() {
                        seek_targets_us.push(digits.parse::<i64>().must());
                    }
                }
            }
            ("tracks", "audio") => {
                // audio = [{ id = 1, ... }, { id = 2, ... }] — the
                // ids, in order.
                if value.trim().starts_with('[') {
                    for chunk in value.split('{').skip(1) {
                        for pair in chunk.split(',') {
                            if let Some((field, field_value)) = pair.split_once('=')
                                && field.trim() == "id"
                            {
                                let digits: String =
                                    field_value.chars().filter(|c| c.is_ascii_digit()).collect();
                                if !digits.is_empty() {
                                    audio_ids.push(digits.parse::<u32>().must());
                                }
                            }
                        }
                    }
                }
            }
            ("switch_plan", "to_track_id") => {
                let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
                if !digits.is_empty() {
                    switch_to = Some(digits.parse::<u32>().must());
                }
            }
            _ => {}
        }
    }
    Ok(FixturePlan {
        fixture_rel: fixture_rel.to_string(),
        label,
        seek_targets_us,
        audio_ids,
        switch_to,
    })
}

/// The `parent = "generated/valid/…"` line of a dirty expectation,
/// mapped to its expectation TOML path.
fn parent_of(expectation_rel: &str) -> Result<String, String> {
    let text = std::fs::read_to_string(
        testkit::fixture_path(expectation_rel).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("{expectation_rel}: {error}"))?;
    for line in text.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("parent = ") {
            let fixture = value.trim().trim_matches('"');
            let name = fixture.rsplit('/').next().unwrap_or(fixture);
            return Ok(format!("expectations/valid/{name}.toml"));
        }
    }
    Err(format!(
        "{expectation_rel}: no parent recorded for the missing facts"
    ))
}

/// The default fixture rotation (the card: "committed fixtures, the
/// multi-track ones included"): the two valid parents — the 2-track
/// A/V parent and the 3-track multi-audio parent — plus one dirty
/// class-B truncation so the damage/concealment lanes soak too.
///
/// # Errors
/// The first plan that fails to parse, with its sentence.
pub fn fixture_rotation() -> Result<Vec<FixturePlan>, String> {
    Ok(vec![
        FixturePlan::from_expectation(
            "generated/valid/avi_mjpeg_pcm.avi",
            "expectations/valid/avi_mjpeg_pcm.avi.toml",
            "valid-2track",
        )?,
        FixturePlan::from_expectation(
            "generated/valid/avi_mjpeg_2pcm.avi",
            "expectations/valid/avi_mjpeg_2pcm.avi.toml",
            "valid-3track",
        )?,
        FixturePlan::from_expectation(
            "generated/malformed/avi_mjpeg_pcm__trunc3.avi",
            "expectations/malformed/avi_mjpeg_pcm__trunc3.avi.toml",
            "dirty-trunc3",
        )?,
    ])
}

// ---------------------------------------------------------------------------
// The cycle bookkeeping (fixed-size — no per-cycle allocation)
// ---------------------------------------------------------------------------

/// The completed-op counters (the report's loop-count axis and the
/// scenario's own liveness evidence).
#[derive(Debug, Default, Clone, Copy)]
pub struct CycleCounts {
    /// Fully completed open → … → close cycles.
    pub cycles: u64,
    /// `session.open` accepted.
    pub opens_ok: u64,
    /// Sessions that settled in `Error` at open (counted, never
    /// hidden).
    pub error_opens: u64,
    /// Seeks accepted AND completed (`Seeked` terminal).
    pub seeks_ok: u64,
    /// Seeks accepted but failed (`SeekFailed` terminal — contract
    /// D4, playback continues; counted as a completed op).
    pub seeks_failed: u64,
    /// Seeks skipped for state reasons (not `Playing`/`Buffering`).
    pub seeks_skipped: u64,
    /// Track switches completed (hops: a there-and-back round trip is
    /// two ops, each through its own terminal).
    pub switches_ok: u64,
    /// Switch hops skipped for state reasons (per hop — a truncated
    /// remnant's drain to `Ended` between hops is the dirty fixture's
    /// honest lane).
    pub switches_skipped: u64,
    /// Cycles that reached `Ended` before close.
    pub ended_cycles: u64,
    /// Cycles closed from an active state (the composed
    /// `stop → close`).
    pub active_closes: u64,
    /// Cycles closed from the dirty fixture's `Error` lane.
    pub error_closes: u64,
    /// Decoded payload bytes seen at the sinks, accumulated across
    /// cycles (each session owns its sink; the epilogue folds the
    /// closing session's total in).
    pub decoded_bytes: u64,
    /// Frames accepted by the sinks, accumulated across cycles (the
    /// sink-side cross-check of the engine's `frames_delivered`).
    pub decoded_frames: u64,
    /// Total in-cycle wall seconds (for the mean cycle duration).
    pub wall_s: f64,
}

/// How one cycle exited (the close lane).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CycleExit {
    /// Played to `Ended`, closed from there.
    Ended,
    /// Still active at the close (the composed `stop → close`).
    Active,
    /// The dirty lane: the session settled in `Error`.
    Error,
}

// ---------------------------------------------------------------------------
// The cycle
// ---------------------------------------------------------------------------

/// One `session.open` request for a fixture path.
fn open_request(path: &std::path::Path, id: u64) -> WireRequest {
    WireRequest {
        request_id: RequestId(id),
        command: Command::SessionOpen {
            source: SourceSpec::File {
                path: path.to_path_buf(),
            },
            options: commands::catalog::types::OpenOptions::default(),
        },
    }
}

/// The seek-target rotation for one cycle. The recorded absolute
/// targets rotate over the MID-MEDIA ones (all but the last recorded
/// target — the end-adjacent one leaves no switchable remnant for the
/// cycle's track-switch step on these ~2 s fixtures, and its
/// resolution path is seek_flow's own suite), plus relative and
/// percent shapes so the resolution paths soak too (the engine
/// resolves and clamps all of them, doc 05 §2.4).
fn seek_request(plan: &FixturePlan, cycle_index: u64, id: u64) -> WireRequest {
    let step = (cycle_index / 4) as usize;
    // The mid-media rotation slice: every recorded target except the
    // end-adjacent last (a single target stays a single target).
    let mid = mid_media_targets(plan);
    let pick = |offset: usize| mid[(step + offset) % mid.len()];
    let target = match cycle_index % 4 {
        0 => WireSeekTarget::Absolute {
            time: Timestamp(pick(0)),
        },
        1 => WireSeekTarget::Relative { delta_ms: 250 },
        2 => WireSeekTarget::Percent { fraction: 0.4 },
        _ => WireSeekTarget::Absolute {
            time: Timestamp(pick(1)),
        },
    };
    WireRequest {
        request_id: RequestId(id),
        command: Command::PlaybackSeek { target },
    }
}

/// The mid-media rotation slice of a plan's recorded targets: all but
/// the last (end-adjacent) one; a one-target plan keeps its single
/// target (an honest single-point rotation).
fn mid_media_targets(plan: &FixturePlan) -> &[i64] {
    let len = plan.seek_targets_us.len();
    &plan.seek_targets_us[..len.saturating_sub(1).max(1)]
}

/// One `tracks.set-audio-track` request (by wire track id, or none).
fn switch_request(selector: commands::catalog::types::TrackSelector, id: u64) -> WireRequest {
    WireRequest {
        request_id: RequestId(id),
        command: Command::TracksSetAudioTrack { selector },
    }
}

/// One `session.close` request.
fn close_request(id: u64) -> WireRequest {
    WireRequest {
        request_id: RequestId(id),
        command: Command::SessionClose,
    }
}

/// Run one full soak cycle (open → play → seek → switch → close)
/// against `host`, through the command layer, attributing live-byte
/// deltas to phases (`live` reads the counting allocator).
///
/// # Errors
/// A sentence naming the failing op (the harness treats any command
/// error other than the recorded state-skip lanes as a failure — a
/// soak that errors its ops is measuring nothing).
pub fn run_cycle(
    host: &mut SoakHost,
    plan: &FixturePlan,
    cycle_index: u64,
    phases: &mut PhaseAccumulator,
    counts: &mut CycleCounts,
    live: fn() -> isize,
) -> Result<(), String> {
    let started = std::time::Instant::now();
    let path = plan.path()?;
    // Liveness baseline: the ENGINE's lifetime frame counter (the
    // per-session sinks are replaced at every open — only the engine
    // counts across sessions).
    let frames_before = host.stats().frames_delivered;

    // ---- open (the reopen op after the first cycle) ----
    let live_before = live();
    let response = lumen_soak_dispatch(host, &open_request(&path, cycle_index * 100 + 1));
    if let Err(error) = response.result {
        return Err(format!(
            "cycle {cycle_index} open on {}: {error}",
            plan.label
        ));
    }
    counts.opens_ok += 1;
    let mut exit = CycleExit::Active;
    if host.state() == SessionState::Error {
        counts.error_opens += 1;
        exit = CycleExit::Error;
    }
    phases.record(Phase::Open, live() - live_before);

    if exit == CycleExit::Error {
        // The dirty lane: no playback ops from `Error` (the table has
        // no edge) — close is the remaining exit.
        close_cycle(host, plan, cycle_index, phases, live)?;
        finish_cycle(host, counts, started, frames_before, exit);
        return Ok(());
    }

    // ---- seek (immediately, while the session is mid-media: the
    //      table serves Buffering/Playing — and on these 2-second
    //      fixtures any advance first would drain the pipeline to the
    //      ended lane before the op lands; measured, not assumed) ----
    if in_playback(host) {
        let live_before = live();
        let response = lumen_soak_dispatch(
            host,
            &seek_request(plan, cycle_index, cycle_index * 100 + 2),
        );
        if let Err(error) = response.result {
            return Err(format!(
                "cycle {cycle_index} seek on {}: {error}",
                plan.label
            ));
        }
        match host.last_seek_completed {
            Some(true) => counts.seeks_ok += 1,
            Some(false) => counts.seeks_failed += 1,
            None => return Err(format!("cycle {cycle_index}: no seek terminal recorded")),
        }
        phases.record(Phase::Seek, live() - live_before);
    } else {
        counts.seeks_skipped += 1;
    }

    // ---- track switch (there and back, straight off the seek's
    //      landing — the decoder-cache lane). Each hop is its own op
    //      with its own state pre-check: a near-end or truncated
    //      remnant can drain to Ended between hops (the dirty
    //      fixture's honest lane — the table refuses switch from
    //      Ended by design), so the skip is counted per hop, never
    //      hidden ----
    let (there, back) = switch_selectors(plan);
    for (hop, selector) in [there, back].into_iter().enumerate() {
        if !in_playback(host) {
            counts.switches_skipped += 1;
            continue;
        }
        let live_before = live();
        let response = lumen_soak_dispatch(
            host,
            &switch_request(selector, cycle_index * 100 + 3 + hop as u64),
        );
        if let Err(error) = response.result {
            return Err(format!(
                "cycle {cycle_index} switch hop {hop} on {}: {error}",
                plan.label
            ));
        }
        counts.switches_ok += 1;
        phases.record(Phase::Switch, live() - live_before);
    }

    // ---- play advance (the play step between ops) ----
    let live_before = live();
    advance(host, ADVANCE_POLLS);
    phases.record(Phase::Play, live() - live_before);

    // ---- play to the media's end (bounded) ----
    let live_before = live();
    let ended = host.pump_until(|state| state == SessionState::Ended, PLAY_TO_END_POLLS);
    phases.record(Phase::Play, live() - live_before);
    if host.state() == SessionState::Error {
        exit = CycleExit::Error;
    } else if ended {
        exit = CycleExit::Ended;
    }

    // ---- close (from Ended, the composed stop→close if still
    //      active, or Error — all are the command layer's exits) ----
    close_cycle(host, plan, cycle_index, phases, live)?;
    finish_cycle(host, counts, started, frames_before, exit);
    Ok(())
}

/// Whether the session sits in a state the seek/switch ops serve.
fn in_playback(host: &SoakHost) -> bool {
    matches!(
        host.state(),
        SessionState::Playing | SessionState::Buffering
    )
}

/// The switch selectors for one cycle: on the multi-audio fixture the
/// recorded `to_track_id` and back; on single-audio fixtures deselect
/// and reselect (the same mint + limb-swap lanes).
fn switch_selectors(
    plan: &FixturePlan,
) -> (
    commands::catalog::types::TrackSelector,
    commands::catalog::types::TrackSelector,
) {
    let none = commands::catalog::types::TrackSelector::None;
    let id = |handle: u32| commands::catalog::types::TrackSelector::Id {
        id: commands::catalog::types::TrackId(handle),
    };
    match (plan.switch_to, plan.audio_ids.first()) {
        (Some(to), Some(from)) => (id(to), id(*from)),
        (None, Some(only)) => (none, id(*only)),
        _ => (none, none),
    }
}

/// A bounded playback advance step.
fn advance(host: &mut SoakHost, polls: usize) {
    for _ in 0..polls {
        host.pump();
    }
}

/// The close op (one dispatch, phase-attributed).
fn close_cycle(
    host: &mut SoakHost,
    plan: &FixturePlan,
    cycle_index: u64,
    phases: &mut PhaseAccumulator,
    live: fn() -> isize,
) -> Result<(), String> {
    let live_before = live();
    let response = lumen_soak_dispatch(host, &close_request(cycle_index * 100 + 9));
    if let Err(error) = response.result {
        return Err(format!(
            "cycle {cycle_index} close on {}: {error}",
            plan.label
        ));
    }
    phases.record(Phase::Close, live() - live_before);
    Ok(())
}

/// The cycle's epilogue: exit bookkeeping, wall accounting, the
/// decoded-byte fold (the closing session's sink total — the sink
/// dies with the next open), the completed-cycle stamp, and the
/// liveness assert (D-6: a cycle that decoded no frames is a ritual,
/// not a soak).
fn finish_cycle(
    host: &mut SoakHost,
    counts: &mut CycleCounts,
    started: std::time::Instant,
    frames_before: u64,
    exit: CycleExit,
) {
    let decoded = host.stats().frames_delivered;
    assert!(
        decoded > frames_before,
        "soak liveness (D-6): a completed cycle decoded no frames — the loop is a ritual, not a soak"
    );
    if let Some(sink) = host.sink() {
        counts.decoded_bytes = counts.decoded_bytes.saturating_add(sink.bytes_total());
        counts.decoded_frames = counts.decoded_frames.saturating_add(sink.frames_total());
    }
    match exit {
        CycleExit::Ended => counts.ended_cycles += 1,
        CycleExit::Active => counts.active_closes += 1,
        CycleExit::Error => counts.error_closes += 1,
    }
    counts.cycles += 1;
    counts.wall_s += started.elapsed().as_secs_f64();
    host.note_cycle_complete();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fixture rotation parses from the committed expectations:
    /// the recorded seek targets and the multi-track switch plan are
    /// read, not transcribed; the dirty fixture's facts arrive via its
    /// recorded parent (the track_switch_fixture precedent).
    #[test]
    fn fixture_rotation_parses_the_recorded_truth() {
        let _serial = testkit::alloc_count::serialize_tests();
        let plans = fixture_rotation().must();
        assert_eq!(plans.len(), 3, "two valid parents + one dirty truncation");
        let two = &plans[0];
        assert_eq!(two.seek_targets_us, vec![0, 500_000, 1_000_000, 1_999_000]);
        assert_eq!(two.audio_ids, vec![1]);
        assert_eq!(
            two.switch_to, None,
            "single-audio parent: deselect/reselect"
        );
        let three = &plans[1];
        assert_eq!(three.audio_ids, vec![1, 2]);
        assert_eq!(
            three.switch_to,
            Some(2),
            "the recorded multi-track switch plan"
        );
        let dirty = &plans[2];
        assert_eq!(
            dirty.seek_targets_us,
            vec![0, 500_000, 1_000_000, 1_999_000],
            "the dirty fixture's facts come from its recorded parent"
        );
        assert_eq!(dirty.audio_ids, vec![1]);
        assert!(dirty.path().is_ok(), "the dirty fixture exists in the tree");
    }
}
