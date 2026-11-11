//! The soak scenario runners (card 8.C1, issue #54 / P8-3): the five
//! config-driven scenarios the manifest selects, and the measurement
//! and assertion discipline each one carries.
//!
//! # Architecture (why the scenarios re-use the 3.E4 harness)
//!
//! Every op goes through the SAME command-layer seam the original
//! soak loop drives — [`lumen_soak_dispatch`] over a [`SoakHost`]
//! (ARCH-5; the recorded lineage is in `dispatch.rs`) — so the
//! scenarios exercise the real engine playback path (access → AVI
//! demux → MJPEG/PCM decode → sink) against the committed
//! `tests/fixtures/` media, not a mock. The modules are shared from
//! `tests/soak/` via `#[path]` includes (the manifest config plane,
//! the dispatch seam and the memory instruments are one codebase; the
//! scenario suite is a second DRIVER over them, not a fork).
//!
//! # The five scenarios (kinds, their driver axis)
//!
//! | kind | axis | loop | instruments |
//! |---|---|---|---|
//! | `play-loop` | `duration_min` | open → play → advance to Ended → close, fixture rotation | cadence RSS/live samples, live-bytes slope |
//! | `seek-storm` | `duration_min` | tight absolute/relative/percent seek rotation per open session, epoch drain+close+reopen | cadence samples, seek-terminal counts |
//! | `playlist-cycle` | `duration_min` | sequential playlist passes with wrap counting | cadence samples, pass counts |
//! | `open-close-churn` | `cycles = N` | N rapid open → settle → close cycles | per-cycle fd/RSS/live samples |
//! | `reopen-stress` | `cycles = N` | N full open → play → close cycles | per-cycle fd/RSS/live samples |
//!
//! # Measurement discipline (the 3.E4 rules, carried over)
//!
//! - **Warmup, then measure**: the first `warmup_fraction` of each
//!   scenario (min one unit, max half) is excluded from the fits.
//! - **Two instruments**: `live_bytes` (counting allocator — the
//!   precise leak signal) and `VmRSS` (`/proc/self/status`); the
//!   count-driven scenarios add **fd count** (`/proc/self/fd`) as the
//!   handle-leak instrument.
//! - **Liveness is asserted** (D-6): cycles must complete, ops must
//!   run, frames must decode — a scenario that skips its ops is a
//!   ritual, not a measurement.
//! - **Violations are recorded, not thrown**: a budget or liveness
//!   violation lands in [`ScenarioResult::failures`] and the run
//!   CONTINUES (the report stays complete for triage); the test
//!   asserts the failure list is empty at the end. Only a harness
//!   break (a command the engine refuses, a terminal that never
//!   lands) fails fast — the run is measuring nothing at that point.
//!
//! # Errors
//!
//! `Err` sentences name the scenario, the op and the engine's refusal
//! — a harness break means the scenario setup or the engine broke,
//! and the sentence is the triage.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use commands::catalog::types::OpenOptions;
use commands::catalog::types::{SeekTarget as WireSeekTarget, SourceSpec, Timestamp};
use commands::envelope::RequestId;
use commands::{Command, WireRequest};
use playback_engine::state::SessionState;
use testkit::Must;

use crate::dispatch::{SoakHost, lumen_soak_dispatch};
use crate::manifest::{Defaults, Kind, ScenarioSpec};
use crate::memory::{CadenceSampler, least_squares_slope};
use crate::report::ScenarioResult;
use crate::scenario::FixturePlan;

/// Poll bound for the play-to-End drain of one cycle (the
/// `scenario.rs` `PLAY_TO_END_POLLS` precedent: a hang is a failed
/// test, never a timeout mystery).
const PLAY_TO_END_POLLS: usize = 1024;
/// Poll bound for the reopen-stress decode-evidence wait (bounded
/// pumps until the engine's lifetime frame counter advances — the
/// per-cycle D-6 fact; the same category of structural bound as
/// `PLAY_TO_END_POLLS`, not a config-plane duration).
const DECODE_EVIDENCE_POLLS: usize = 64;
/// The offset keeping this suite's external request ids clear of the
/// host's internal allocator (collision is harmless by construction —
/// the drain matches the id it submitted — distinct is just easier to
/// read in receipts).
const REQUEST_ID_BASE: u64 = 1_000_000;

// ---------------------------------------------------------------------------
// The scenario drivers
// ---------------------------------------------------------------------------

/// Run one manifest scenario to completion (violations recorded into
/// the result; see the module docs for the fail-fast split).
///
/// # Errors
/// A harness-break sentence (the scenario's ops stopped working —
/// nothing after it is measurable).
pub fn run_scenario(
    spec: &ScenarioSpec,
    defaults: &Defaults,
    live: fn() -> isize,
) -> Result<ScenarioResult, String> {
    let plans = fixture_plans(spec)?;
    match spec.kind {
        Kind::PlayLoop => run_play_loop(spec, defaults, plans, live),
        Kind::SeekStorm => run_seek_storm(spec, defaults, plans, live),
        Kind::PlaylistCycle => run_playlist_cycle(spec, defaults, plans, live),
        Kind::OpenCloseChurn => run_count_scenario(spec, defaults, plans, live, false),
        Kind::ReopenStress => run_count_scenario(spec, defaults, plans, live, true),
    }
}

/// Build the fixture plans for a scenario (existence-checked; seek
/// targets and track facts read from the recorded expectation TOMLs —
/// never transcribed).
///
/// # Errors
/// The first fixture that fails to resolve or parse, with its
/// sentence.
fn fixture_plans(spec: &ScenarioSpec) -> Result<Vec<ScenarioPlan>, String> {
    let mut plans = Vec::with_capacity(spec.fixtures.len());
    for fixture_rel in &spec.fixtures {
        let expectation_rel = expectation_for(fixture_rel)?;
        let plan = FixturePlan::from_expectation(fixture_rel, &expectation_rel, "manifest")
            .map_err(|error| format!("scenario {:?}: fixture {fixture_rel:?}: {error}", spec.id))?;
        plans.push(ScenarioPlan {
            plan,
            label: fixture_rel.clone(),
        });
    }
    Ok(plans)
}

/// One manifest fixture: the parsed plan plus the config-file label
/// (the report names fixtures by their manifest path).
struct ScenarioPlan {
    plan: FixturePlan,
    #[allow(dead_code)] // read by the report through the accessor below
    label: String,
}

impl ScenarioPlan {
    /// The manifest path (the report label).
    #[must_use = "report label"]
    fn label(&self) -> &str {
        &self.label
    }
}

/// Map a fixture path under `tests/fixtures/generated/` to its
/// recorded expectation TOML under `tests/fixtures/expectations/`.
///
/// # Errors
/// A sentence for paths outside the generated areas (the
/// expectations layout is `expectations/<area>/<file>.toml`).
fn expectation_for(fixture_rel: &str) -> Result<String, String> {
    let Some(rest) = fixture_rel.strip_prefix("generated/") else {
        return Err(format!(
            "fixture {fixture_rel:?} is not under generated/ (the recorded expectations live \
             under expectations/<area>/)"
        ));
    };
    let Some((area, name)) = rest.split_once('/') else {
        return Err(format!(
            "fixture {fixture_rel:?} has no <area>/<name> shape under generated/"
        ));
    };
    if !matches!(area, "valid" | "malformed") {
        return Err(format!(
            "fixture {fixture_rel:?}: unknown generated area {area:?} (valid | malformed)"
        ));
    }
    Ok(format!("expectations/{area}/{name}.toml"))
}

// ---------------------------------------------------------------------------
// Shared op plumbing (all through the one dispatch entry)
// ---------------------------------------------------------------------------

/// A per-scenario external request-id source.
struct RequestIds {
    next: u64,
}

impl RequestIds {
    /// A fresh source (ids start above the host allocator's range).
    fn new() -> RequestIds {
        RequestIds {
            next: REQUEST_ID_BASE,
        }
    }

    /// The next id.
    fn allocate(&mut self) -> RequestId {
        self.next = self.next.saturating_add(1);
        RequestId(self.next)
    }
}

/// One `session.open` request for a fixture path.
fn open_request(path: &Path, id: RequestId) -> WireRequest {
    WireRequest {
        request_id: id,
        command: Command::SessionOpen {
            source: SourceSpec::File {
                path: path.to_path_buf(),
            },
            options: OpenOptions::default(),
        },
    }
}

/// One `playback.play` request (C8: idempotent from
/// Buffering/Playing, resume from Paused).
fn play_request(id: RequestId) -> WireRequest {
    WireRequest {
        request_id: id,
        command: Command::PlaybackPlay,
    }
}

/// One `session.close` request (the engine composes stop from active
/// states).
fn close_request(id: RequestId) -> WireRequest {
    WireRequest {
        request_id: id,
        command: Command::SessionClose,
    }
}

/// The seek-target rotation for one seek (absolute recorded target /
/// relative / percent / next recorded target — the resolution paths
/// all soak, the `scenario.rs` rotation).
fn seek_request(plan: &FixturePlan, seek_index: u64, id: RequestId) -> WireRequest {
    let mid = mid_media_targets(plan);
    let step = (seek_index / 4) as usize;
    let pick = |offset: usize| mid[(step + offset) % mid.len()];
    let target = match seek_index % 4 {
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
        request_id: id,
        command: Command::PlaybackSeek { target },
    }
}

/// The mid-media rotation slice: every recorded target except the
/// end-adjacent last (the `scenario.rs` rule; a single target stays).
fn mid_media_targets(plan: &FixturePlan) -> &[i64] {
    let len = plan.seek_targets_us.len();
    &plan.seek_targets_us[..len.saturating_sub(1).max(1)]
}

/// Dispatch one request through the command layer, mapping a refusal
/// to a harness-break sentence.
///
/// # Errors
/// The engine's refusal, contextualized.
fn dispatch(host: &mut SoakHost, request: WireRequest, context: &str) -> Result<(), String> {
    let response = lumen_soak_dispatch(host, &request);
    response
        .result
        .map(|_| ())
        .map_err(|error| format!("{context}: {error}"))
}

/// Whether the session sits in a state the playback ops serve.
fn in_playback(host: &SoakHost) -> bool {
    matches!(
        host.state(),
        SessionState::Playing | SessionState::Buffering
    )
}

/// The completed-op counters (the report's loop-count axis).
#[derive(Debug, Default, Clone, Copy)]
pub struct ScenarioCounts {
    /// Completed iterations (cycles; reopens for seek-storm).
    pub cycles: u64,
    /// `session.open` accepted.
    pub opens_ok: u64,
    /// Opens that settled in `Error` (the dirty lane).
    pub error_opens: u64,
    /// `playback.play` accepted.
    pub plays_ok: u64,
    /// Seeks completed (`Seeked`).
    pub seeks_ok: u64,
    /// Seeks that failed their terminal (`SeekFailed` — contract D4).
    pub seeks_failed: u64,
    /// Seeks skipped for state reasons.
    pub seeks_skipped: u64,
    /// Closes completed.
    pub closes_ok: u64,
    /// Closes from the dirty `Error` lane.
    pub error_closes: u64,
    /// Full playlist passes (playlist-cycle only).
    pub playlist_passes: u64,
    /// Frames delivered by the engine over the scenario (lifetime
    /// counter delta — the sink-side cross-check reads the sinks).
    pub frames_decoded: u64,
    /// Decoded payload bytes folded from the per-session sinks.
    pub decoded_bytes: u64,
}

/// The running bookkeeping of one scenario (counts + recorded
/// violations + honest-absence notes; the run continues past a
/// violation so the report stays complete for triage).
struct ScenarioRun {
    counts: ScenarioCounts,
    failures: Vec<String>,
    notes: Vec<String>,
}

impl ScenarioRun {
    /// A fresh run.
    fn new() -> ScenarioRun {
        ScenarioRun {
            counts: ScenarioCounts::default(),
            failures: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Record a budget/liveness violation (the test asserts the list
    /// empty at the end; the report carries every sentence).
    fn fail(&mut self, sentence: String) {
        self.failures.push(sentence);
    }

    /// Record an honest-absence note (an instrument this platform
    /// lacks, a wall-cap shortfall) — reported, never failed.
    fn note(&mut self, sentence: String) {
        self.notes.push(sentence);
    }
}

// ---------------------------------------------------------------------------
// The duration-driven scenarios
// ---------------------------------------------------------------------------

/// Shared timing state for a duration-driven scenario.
struct DurationClock {
    deadline: Instant,
    warmup_deadline: Instant,
    warmup_ended: bool,
}

impl DurationClock {
    /// The clock for one scenario (`minutes` real wall minutes; the
    /// warmup window is the 3.E4 formula: the configured fraction,
    /// min 1 s, max half the run).
    fn new(minutes: f64, defaults: &Defaults) -> DurationClock {
        let duration = Duration::from_secs_f64(minutes * 60.0);
        let start = Instant::now();
        let warmup = Duration::from_secs_f64(
            (duration.as_secs_f64() * defaults.warmup_fraction)
                .min(duration.as_secs_f64() * 0.5)
                .max(1.0),
        );
        DurationClock {
            deadline: start + duration,
            warmup_deadline: start + warmup,
            warmup_ended: false,
        }
    }

    /// Whether the run window is over.
    fn running(&self) -> bool {
        Instant::now() < self.deadline
    }

    /// End the warmup at the first loop-boundary sighting past the
    /// warmup deadline (idempotent).
    fn maybe_end_warmup(&mut self, host: &mut SoakHost) {
        if !self.warmup_ended
            && Instant::now() >= self.warmup_deadline
            && host.state() == SessionState::Idle
        {
            host.end_warmup();
            self.warmup_ended = true;
        }
    }
}

/// The common epilogue of a duration scenario: the endpoint sample,
/// the measured-window fits, the liveness and budget assertions, and
/// the result assembly.
fn finish_duration_scenario(
    spec: &ScenarioSpec,
    defaults: &Defaults,
    run: &mut ScenarioRun,
    host: &mut SoakHost,
    plans: &[ScenarioPlan],
    warmup: &mut DurationClock,
    wall_start: Instant,
) -> Result<ScenarioResult, String> {
    if !warmup.warmup_ended {
        // A run too short to cross the warmup deadline inside the loop
        // still gets a measured window (the endpoint sample below).
        host.end_warmup();
        warmup.warmup_ended = true;
    }
    host.force_sample();

    let sampling = host.take_sampling().must();
    let samples = sampling.sampler.samples().to_vec();
    let warmup_samples = samples.iter().filter(|sample| sample.warmup).count();
    let measured: Vec<&crate::memory::Sample> =
        samples.iter().filter(|sample| !sample.warmup).collect();
    let measured_window_s = measured
        .last()
        .zip(measured.first())
        .map(|(last, first)| last.elapsed_s - first.elapsed_s)
        .unwrap_or(0.0);

    let live_points: Vec<(f64, f64)> = measured
        .iter()
        .map(|sample| (sample.elapsed_s, sample.live_bytes as f64))
        .collect();
    let rss_points: Vec<(f64, f64)> = measured
        .iter()
        .filter_map(|sample| sample.rss_kb.map(|kb| (sample.elapsed_s, kb as f64)))
        .collect();
    let live_slope_b_per_s = least_squares_slope(&live_points);
    let rss_slope_kb_per_s = least_squares_slope(&rss_points);

    let ceiling = spec
        .leak_ceiling_mb_per_h
        .unwrap_or(defaults.leak_ceiling_mb_per_h);
    let verdict = check_slope(
        live_slope_b_per_s,
        ceiling,
        defaults.target_mb_per_h,
        defaults.assert_target,
    );

    // ---- liveness (D-6) ----
    if run.counts.cycles < defaults.min_cycles {
        run.fail(format!(
            "liveness: only {} cycles completed (min_cycles = {}) — the run proves nothing",
            run.counts.cycles, defaults.min_cycles
        ));
    }
    if run.counts.frames_decoded == 0 {
        run.fail(
            "liveness: the engine delivered no frames across the whole scenario — the loop is a \
             ritual, not a soak (D-6)"
                .to_string(),
        );
    }

    // ---- the slope budgets ----
    if let Some(mb_per_h) = verdict.live_mb_per_h {
        if verdict.over_ceiling {
            run.fail(format!(
                "bounded memory: measured-window live-bytes slope is {mb_per_h:.3} MB/h \
                 (EXTRAPOLATION from a {measured_window_s:.1} s window, {} samples), at/above the \
                 configured ceiling {ceiling:.3} MB/h",
                measured.len()
            ));
        }
        if verdict.over_target {
            run.fail(format!(
                "design target: measured-window live-bytes slope is {mb_per_h:.3} MB/h, at/above \
                 the configured target {} MB/h (assert_target is set for this profile)",
                defaults.target_mb_per_h
            ));
        }
    } else if !measured.is_empty() {
        run.fail(format!(
            "bounded memory: {} measured samples could not define a slope line — the window is \
             too degenerate to certify",
            measured.len()
        ));
    }

    Ok(ScenarioResult {
        id: spec.id.clone(),
        kind_label: spec.kind.label(),
        driver: format!("duration {:.2} min", spec.duration_min.unwrap_or(0.0)),
        fixtures: plans.iter().map(|p| p.label().to_string()).collect(),
        wall_s: wall_start.elapsed().as_secs_f64(),
        samples_total: samples.len(),
        warmup_samples,
        measured_window_s,
        measured_cycles: run.counts.cycles,
        counts: run.counts,
        live_slope_b_per_s,
        rss_slope_kb_per_s,
        fd_baseline: None,
        fd_final: None,
        fd_max: None,
        rss_baseline_kb: None,
        rss_final_kb: None,
        live_baseline_bytes: None,
        live_final_bytes: None,
        live_mean_delta_per_cycle: None,
        measured_cycles_span: None,
        wall_capped: false,
        cycles_configured: None,
        ceiling_mb_per_h: ceiling,
        verdict,
        failures: std::mem::take(&mut run.failures),
        notes: std::mem::take(&mut run.notes),
    })
}

/// The play-loop scenario: open → play → advance to Ended → close,
/// over the fixture rotation, for the configured duration.
fn run_play_loop(
    spec: &ScenarioSpec,
    defaults: &Defaults,
    plans: Vec<ScenarioPlan>,
    live: fn() -> isize,
) -> Result<ScenarioResult, String> {
    let mut run = ScenarioRun::new();
    let mut host = SoakHost::new();
    let minutes = spec.duration_min.must();
    let expected = expected_samples(minutes, defaults);
    host.install_sampling(CadenceSampler::new(defaults.sample_ms, expected), live);

    let wall_start = Instant::now();
    let mut warmup = DurationClock::new(minutes, defaults);
    let mut ids = RequestIds::new();
    let mut cycle_index = 0u64;

    while warmup.running() {
        let entry = &plans[(cycle_index as usize) % plans.len()];
        play_loop_cycle(
            &mut run,
            &mut host,
            entry,
            &mut ids,
            &mut warmup,
            cycle_index,
        )?;
        cycle_index += 1;
        warmup.maybe_end_warmup(&mut host);
    }
    finish_duration_scenario(
        spec,
        defaults,
        &mut run,
        &mut host,
        &plans,
        &mut warmup,
        wall_start,
    )
}

/// Fold the just-closed session's sink payload into the scenario
/// totals (the D-6 decode-evidence axis — the counting sink survives
/// the close and dies at the NEXT open, so the close site is the read
/// point; the `play_loop_cycle`/`run_count_scenario` epilogues and
/// every seek-storm close call this).
fn fold_sink_bytes(run: &mut ScenarioRun, host: &SoakHost) {
    if let Some(sink) = host.sink() {
        run.counts.decoded_bytes = run.counts.decoded_bytes.saturating_add(sink.bytes_total());
    }
}

/// One play-loop cycle (also the playlist-cycle body: the caller owns
/// the position bookkeeping).
fn play_loop_cycle(
    run: &mut ScenarioRun,
    host: &mut SoakHost,
    entry: &ScenarioPlan,
    ids: &mut RequestIds,
    warmup: &mut DurationClock,
    cycle_index: u64,
) -> Result<(), String> {
    let path: PathBuf = entry
        .plan
        .path()
        .map_err(|error| format!("scenario cycle {cycle_index}: {error}"))?;
    let frames_before = host.stats().frames_delivered;

    dispatch(host, open_request(&path, ids.allocate()), "play-loop open")?;
    run.counts.opens_ok += 1;
    if host.state() == SessionState::Error {
        run.counts.error_opens += 1;
        dispatch(
            host,
            close_request(ids.allocate()),
            "play-loop close (error lane)",
        )?;
        run.counts.error_closes += 1;
        run.counts.cycles += 1;
        host.note_cycle_complete();
        return Ok(());
    }

    dispatch(host, play_request(ids.allocate()), "play-loop play")?;
    run.counts.plays_ok += 1;
    host.pump_until(
        |state| matches!(state, SessionState::Ended | SessionState::Error),
        PLAY_TO_END_POLLS,
    );
    let error_lane = host.state() == SessionState::Error;

    dispatch(host, close_request(ids.allocate()), "play-loop close")?;
    run.counts.closes_ok += 1;

    // The per-cycle D-6 fact: a cycle that never decoded a frame (and
    // did not take the error lane) is a ritual iteration.
    let frames = host.stats().frames_delivered;
    let decoded = frames.saturating_sub(frames_before);
    if decoded == 0 && !error_lane {
        run.fail(format!(
            "liveness: cycle {cycle_index} on {} decoded no frames (D-6)",
            entry.label()
        ));
    }
    run.counts.frames_decoded += decoded;
    fold_sink_bytes(run, host);
    run.counts.cycles += 1;
    host.note_cycle_complete();
    let _ = warmup; // timing owned by the driver loop; parameter kept for symmetry
    Ok(())
}

/// The playlist-cycle scenario: strict sequential passes with
/// wrap-around counting.
fn run_playlist_cycle(
    spec: &ScenarioSpec,
    defaults: &Defaults,
    plans: Vec<ScenarioPlan>,
    live: fn() -> isize,
) -> Result<ScenarioResult, String> {
    let mut run = ScenarioRun::new();
    let mut host = SoakHost::new();
    let minutes = spec.duration_min.must();
    let expected = expected_samples(minutes, defaults);
    host.install_sampling(CadenceSampler::new(defaults.sample_ms, expected), live);

    let wall_start = Instant::now();
    let mut warmup = DurationClock::new(minutes, defaults);
    let mut ids = RequestIds::new();
    let mut position = 0usize;
    let mut cycle_index = 0u64;

    while warmup.running() {
        let entry = &plans[position];
        play_loop_cycle(
            &mut run,
            &mut host,
            entry,
            &mut ids,
            &mut warmup,
            cycle_index,
        )?;
        position += 1;
        if position >= plans.len() {
            position = 0;
            run.counts.playlist_passes += 1;
        }
        cycle_index += 1;
        warmup.maybe_end_warmup(&mut host);
    }
    if run.counts.playlist_passes == 0 {
        run.fail(format!(
            "liveness: zero full playlist passes in {minutes} min over {} fixtures — the \
             playlist semantics were never exercised",
            plans.len()
        ));
    }
    finish_duration_scenario(
        spec,
        defaults,
        &mut run,
        &mut host,
        &plans,
        &mut warmup,
        wall_start,
    )
}

/// The seek-storm scenario: a tight seek rotation against one open
/// session (the `seeks_per_reopen` rotation, default 16), then a
/// bounded drain to the end lane and close — the reopen is the
/// scenario's cycle.
fn run_seek_storm(
    spec: &ScenarioSpec,
    defaults: &Defaults,
    plans: Vec<ScenarioPlan>,
    live: fn() -> isize,
) -> Result<ScenarioResult, String> {
    let mut run = ScenarioRun::new();
    let mut host = SoakHost::new();
    let minutes = spec.duration_min.must();
    let expected = expected_samples(minutes, defaults);
    host.install_sampling(CadenceSampler::new(defaults.sample_ms, expected), live);

    let wall_start = Instant::now();
    let mut warmup = DurationClock::new(minutes, defaults);
    let mut ids = RequestIds::new();
    let mut seek_index = 0u64;
    let mut seeks_this_epoch = 0u64;
    let mut reopen_index = 0u64;

    while warmup.running() {
        // Ensure a playable session (the reopen path is the scenario's
        // cycle; the original soak's cycle opens WITHOUT an explicit
        // play — open lands in Buffering/Playing on these fixtures).
        if !in_playback(&host) {
            if host.state() != SessionState::Idle {
                dispatch(
                    &mut host,
                    close_request(ids.allocate()),
                    "seek-storm close (drained)",
                )?;
                run.counts.closes_ok += 1;
                // The just-drained session's payload joins the totals
                // (its sink dies at the reopen below).
                fold_sink_bytes(&mut run, &host);
            }
            // The idle boundary is the warmup's end-of-window sighting
            // point (the play-loop loop-boundary rule).
            warmup.maybe_end_warmup(&mut host);
            let entry = &plans[(reopen_index as usize) % plans.len()];
            let path: PathBuf = entry
                .plan
                .path()
                .map_err(|error| format!("seek-storm reopen {reopen_index}: {error}"))?;
            let frames_before = host.stats().frames_delivered;
            dispatch(
                &mut host,
                open_request(&path, ids.allocate()),
                "seek-storm open",
            )?;
            run.counts.opens_ok += 1;
            if host.state() == SessionState::Error {
                run.counts.error_opens += 1;
                dispatch(
                    &mut host,
                    close_request(ids.allocate()),
                    "seek-storm close (error lane)",
                )?;
                run.counts.error_closes += 1;
                // NO sink fold here: the failed open left the PREVIOUS
                // session's already-counted sink in place (the sink is
                // only replaced by a successful open) — folding would
                // double-count it. The failed session decoded nothing.
            } else {
                let decoded = host.stats().frames_delivered.saturating_sub(frames_before);
                run.counts.frames_decoded += decoded;
            }
            run.counts.cycles += 1;
            host.note_cycle_complete();
            reopen_index += 1;
            seeks_this_epoch = 0;
        }

        // The seek op.
        if in_playback(&host) {
            let entry = &plans[(reopen_index.saturating_sub(1) as usize) % plans.len()];
            dispatch(
                &mut host,
                seek_request(&entry.plan, seek_index, ids.allocate()),
                "seek-storm seek",
            )?;
            match host.last_seek_completed {
                Some(true) => run.counts.seeks_ok += 1,
                Some(false) => run.counts.seeks_failed += 1,
                None => {
                    return Err(format!(
                        "seek-storm seek {seek_index}: no seek terminal recorded"
                    ));
                }
            }
            seek_index += 1;
            seeks_this_epoch += 1;
        } else {
            run.counts.seeks_skipped += 1;
        }

        // The configured small advance between seeks.
        for _ in 0..spec.advance_polls {
            host.pump();
        }
        run.counts.frames_decoded = host.stats().frames_delivered;

        // The epoch end: bounded drain to the end lane, then close —
        // the next loop iteration reopens (the cycle). The epoch
        // length is the manifest's shape knob (default 16).
        if seeks_this_epoch >= spec.seeks_per_reopen {
            host.pump_until(
                |state| matches!(state, SessionState::Ended | SessionState::Error),
                PLAY_TO_END_POLLS,
            );
            dispatch(
                &mut host,
                close_request(ids.allocate()),
                "seek-storm close (epoch)",
            )?;
            run.counts.closes_ok += 1;
            // The stormed session's payload joins the totals (the
            // reopen on the next iteration replaces its sink).
            fold_sink_bytes(&mut run, &host);
            seeks_this_epoch = 0;
            warmup.maybe_end_warmup(&mut host);
        }
    }
    if host.state() != SessionState::Idle {
        dispatch(
            &mut host,
            close_request(ids.allocate()),
            "seek-storm final close",
        )?;
        run.counts.closes_ok += 1;
        fold_sink_bytes(&mut run, &host);
    }

    // ---- the seek-storm liveness assertions ----
    let attempts = run
        .counts
        .seeks_ok
        .saturating_add(run.counts.seeks_failed)
        .saturating_add(run.counts.seeks_skipped);
    if attempts == 0 {
        run.fail("liveness: no seek was ever attempted (D-6)".to_string());
    } else {
        let skip_rate = run.counts.seeks_skipped as f64 / attempts as f64;
        if skip_rate > defaults.max_skip_rate {
            run.fail(format!(
                "liveness: {}/{} seeks skipped for state ({:.0}% > the configured {:.0}% cap) — \
                 the storm is not exercising the seek path",
                run.counts.seeks_skipped,
                attempts,
                skip_rate * 100.0,
                defaults.max_skip_rate * 100.0
            ));
        }
    }

    finish_duration_scenario(
        spec,
        defaults,
        &mut run,
        &mut host,
        &plans,
        &mut warmup,
        wall_start,
    )
}

/// The expected sample count for a duration scenario (capacity
/// reservation — the instrument must not be a growth term in its own
/// measurement).
fn expected_samples(minutes: f64, defaults: &Defaults) -> usize {
    (minutes * 60.0 / (defaults.sample_ms as f64 / 1000.0)).ceil() as usize
}

// ---------------------------------------------------------------------------
// The count-driven scenarios (open-close-churn, reopen-stress)
// ---------------------------------------------------------------------------

/// The sample-count ceiling for count-driven scenarios — the bounded
/// instrument (AGENTS §3.5a "bounded memory everywhere", applied to
/// the instrument itself).
///
/// The 2026-11-10 sandbox finding: at `cycles = 2_000_000` the original
/// per-cycle series (3 × `n` × 8 B, page-faulted in as the pushes
/// advance) grew `VmRSS` by ~48 MB — the INSTRUMENT's own storage,
/// not an engine leak (live bytes stayed flat to ±82 B), tripping the
/// 8 MB RSS budget with a clean engine. So the count scenarios sample
/// every `ceil(n / MAX_COUNT_SAMPLES)` cycles: per-cycle resolution
/// at the original design points (`n` ≈ 1000–4000, stride 1), bounded
/// instrument memory at stress scale (≤ 4096 samples ≈ 100 kB).
const MAX_COUNT_SAMPLES: u64 = 4096;

/// Run a count-driven scenario: `N` cycles of open → (play, for
/// reopen-stress) → close on one engine, sampling fd/RSS/live on the
/// bounded cadence and asserting the growth budgets (the 8.C1
/// reopen-cycle stress).
fn run_count_scenario(
    spec: &ScenarioSpec,
    defaults: &Defaults,
    plans: Vec<ScenarioPlan>,
    live: fn() -> isize,
    with_play: bool,
) -> Result<ScenarioResult, String> {
    let n = spec.cycles.must();
    let warmup_cycles = count_warmup_cycles(n, defaults);
    let wall_cap = spec
        .duration_min
        .map(|minutes| Instant::now() + Duration::from_secs_f64(minutes * 60.0));

    // The bounded sample cadence: every `stride` cycles, at most
    // MAX_COUNT_SAMPLES samples (see the const's finding note). Each
    // sample is tagged with the completed-cycle count it was taken at.
    let stride = count_sample_stride(n);
    let mut live_series: Vec<(u64, isize)> = Vec::with_capacity((n / stride + 2) as usize);
    let mut rss_series: Vec<(u64, Option<u64>)> = Vec::with_capacity((n / stride + 2) as usize);
    let mut fd_series: Vec<(u64, Option<u64>)> = Vec::with_capacity((n / stride + 2) as usize);
    let mut last_sampled_at = 0u64;

    let mut run = ScenarioRun::new();
    let mut host = SoakHost::new();
    let mut ids = RequestIds::new();
    let wall_start = Instant::now();
    let kind_word = if with_play { "reopen" } else { "churn" };

    let mut completed = 0u64;
    while completed < n {
        if let Some(cap) = wall_cap
            && Instant::now() >= cap
        {
            break;
        }
        let entry = &plans[(completed as usize) % plans.len()];
        let live_before = live();
        let frames_before = host.stats().frames_delivered;
        let path: PathBuf = entry
            .plan
            .path()
            .map_err(|error| format!("{kind_word} cycle {completed}: {error}"))?;

        dispatch(&mut host, open_request(&path, ids.allocate()), "count open")?;
        run.counts.opens_ok += 1;
        let mut error_lane = false;
        if host.state() == SessionState::Error {
            run.counts.error_opens += 1;
            error_lane = true;
        } else if with_play {
            dispatch(&mut host, play_request(ids.allocate()), "reopen play")?;
            run.counts.plays_ok += 1;
            pump_until_frames(&mut host, frames_before, DECODE_EVIDENCE_POLLS);
        }

        dispatch(&mut host, close_request(ids.allocate()), "count close")?;
        if error_lane {
            run.counts.error_closes += 1;
        } else {
            run.counts.closes_ok += 1;
        }

        let decoded = host.stats().frames_delivered.saturating_sub(frames_before);
        if decoded == 0 && !error_lane && with_play {
            // Only the playing kind owes decode evidence per cycle —
            // the churn kind's ops ARE the open/close pair (the open
            // settles the demux inside its dispatch; frames are not
            // its business).
            run.fail(format!(
                "liveness: {kind_word} cycle {completed} on {} decoded no frames (D-6)",
                entry.label()
            ));
        }
        run.counts.frames_decoded += decoded;
        if !error_lane {
            // Only a successful session's sink joins the totals: a
            // failed open leaves the PREVIOUS session's already-folded
            // sink in place (the sink is replaced only by a successful
            // open) — folding on the error lane would double-count it.
            fold_sink_bytes(&mut run, &host);
        }
        // The bounded cadence: sample at every `stride` completed
        // cycles and at the configured end, each sample tagged with
        // the completed-cycle count it was taken at (the wall-cap cut
        // below appends the true final sample when it lands
        // mid-stride).
        run.counts.cycles += 1;
        completed += 1;
        if completed.is_multiple_of(stride) || completed == n {
            live_series.push((completed, live()));
            rss_series.push((completed, crate::memory::read_vm_rss_kb()));
            fd_series.push((completed, read_fd_count()));
            last_sampled_at = completed;
        }
        let _ = live_before; // the per-cycle delta folds into the series diff below
    }
    // The wall cap cut the run mid-stride: append the true final
    // sample so baseline-vs-final spans the cycles that actually ran.
    if completed > 0 && last_sampled_at != completed {
        live_series.push((completed, live()));
        rss_series.push((completed, crate::memory::read_vm_rss_kb()));
        fd_series.push((completed, read_fd_count()));
    }

    // ---- baselines and growth (post-warmup) ----
    // The baseline sample is the first taken at/after the warmup
    // boundary (tag >= warmup_cycles); the final is the last sample.
    let (baseline_idx, measured_cycles_span) = count_baseline_and_span(&live_series, warmup_cycles);
    let live_baseline = live_series.get(baseline_idx).map(|(_, v)| *v);
    let live_final = live_series.last().map(|(_, v)| *v);
    let rss_baseline_kb = rss_series.get(baseline_idx).and_then(|(_, v)| *v);
    let rss_final_kb = rss_series.last().and_then(|(_, v)| *v);
    let fd_baseline = fd_series.get(baseline_idx).and_then(|(_, v)| *v);
    let fd_final = fd_series.last().and_then(|(_, v)| *v);
    let fd_max = fd_series.iter().filter_map(|(_, v)| *v).max();

    if let (Some(base), Some(final_)) = (live_baseline, live_final)
        && (final_ - base) as f64 > defaults.live_growth_budget_bytes as f64
    {
        run.fail(format!(
            "bounded memory: live bytes grew from {base} to {final_} across {measured_cycles_span} \
             cycles ({} B over the post-warmup baseline, budget {} B)",
            final_ - base,
            defaults.live_growth_budget_bytes
        ));
    }
    if let (Some(base), Some(final_)) = (rss_baseline_kb, rss_final_kb) {
        let growth_mb = (final_.saturating_sub(base)) as f64 / 1024.0;
        if growth_mb > defaults.rss_growth_budget_mb {
            run.fail(format!(
                "bounded memory: VmRSS grew from {base} kB to {final_} kB across \
                 {measured_cycles_span} cycles ({growth_mb:.2} MB, budget {} MB)",
                defaults.rss_growth_budget_mb
            ));
        }
    }
    if let (Some(base), Some(final_)) = (fd_baseline, fd_final)
        && final_.saturating_sub(base) > defaults.fd_growth_budget
    {
        run.fail(format!(
            "handle growth: open fds grew from {base} to {final_} across \
             {measured_cycles_span} cycles (budget {})",
            defaults.fd_growth_budget
        ));
    }
    if rss_baseline_kb.is_none() {
        // The honest-absent note (review W24-R4 P3-4), mirroring the fd
        // instrument's below: off-Linux there is no /proc/self/status,
        // so the RSS-growth block above silently skipping its check
        // would read as a measurement (the duration scenarios already
        // print "VmRSS slope: n/a" — only the count scenarios lacked
        // the note).
        run.note(
            "the VmRSS instrument is not available on this platform (/proc/self/status) — \
             the RSS-growth budget was NOT evaluated"
                .to_string(),
        );
    }
    if fd_baseline.is_none() {
        // The honest-absent note (never a pass, never a fail): the fd
        // instrument does not exist on this platform.
        run.note(
            "the fd-count instrument is not available on this platform (/proc/self/fd) — \
             the handle-growth budget was NOT evaluated"
                .to_string(),
        );
    }

    // ---- liveness ----
    if completed < n {
        run.note(format!(
            "the wall cap ended the scenario at {completed}/{n} configured cycles (honest \
             shortfall, not a failure; recalibrate the manifest if unintended)"
        ));
    }
    if run.counts.cycles < defaults.min_cycles {
        run.fail(format!(
            "liveness: only {} cycles completed (min_cycles = {})",
            run.counts.cycles, defaults.min_cycles
        ));
    }
    if run.counts.frames_decoded == 0 && with_play {
        run.fail(format!(
            "liveness: the engine delivered no frames across {} cycles (D-6)",
            run.counts.cycles
        ));
    }
    let wall_capped = completed < n;

    // The mean net live delta per measured cycle (the drift triage
    // number for the report).
    let live_mean_delta_per_cycle = match (live_baseline, live_final) {
        (Some(base), Some(final_)) if measured_cycles_span > 0 => {
            Some((final_ - base) as f64 / measured_cycles_span as f64)
        }
        _ => None,
    };

    Ok(ScenarioResult {
        id: spec.id.clone(),
        kind_label: spec.kind.label(),
        driver: format!("{n} cycles"),
        fixtures: plans.iter().map(|p| p.label().to_string()).collect(),
        wall_s: wall_start.elapsed().as_secs_f64(),
        samples_total: live_series.len(),
        warmup_samples: baseline_idx,
        measured_window_s: 0.0,
        measured_cycles: run.counts.cycles,
        counts: run.counts,
        live_slope_b_per_s: None,
        rss_slope_kb_per_s: None,
        fd_baseline,
        fd_final,
        fd_max,
        rss_baseline_kb,
        rss_final_kb,
        live_baseline_bytes: live_baseline,
        live_final_bytes: live_final,
        live_mean_delta_per_cycle,
        measured_cycles_span: Some(measured_cycles_span),
        wall_capped,
        cycles_configured: Some(n),
        ceiling_mb_per_h: spec
            .leak_ceiling_mb_per_h
            .unwrap_or(defaults.leak_ceiling_mb_per_h),
        verdict: check_slope(None, 0.0, defaults.target_mb_per_h, false),
        failures: std::mem::take(&mut run.failures),
        notes: std::mem::take(&mut run.notes),
    })
}

/// The warmup cycle count for a count-driven scenario (the configured
/// fraction, min 1, max half the run; collapses to 0 for tiny runs).
fn count_warmup_cycles(n: u64, defaults: &Defaults) -> u64 {
    let raw = (n as f64 * defaults.warmup_fraction).floor() as u64;
    raw.min(n / 2).max(1).min(n.saturating_sub(1))
}

/// The post-warmup baseline index and measured span of a count-driven
/// series: the first sample tag at/after the warmup boundary, and the
/// baseline → final tag distance — the cycles the growth budgets and
/// the drift mean actually cover (the honest span, not the configured
/// count). Surfaced in the report (`Measured cycles (post-warmup
/// baseline sample → final sample)`) since the W24-R4 P2-3 finding: a
/// synthesis had copied the 1M forced-threshold run's span
/// (849,815) into the 2M churn row because no report stated the
/// number — now one always does.
fn count_baseline_and_span(series: &[(u64, isize)], warmup_cycles: u64) -> (usize, u64) {
    let baseline_idx = series
        .iter()
        .position(|(at, _)| *at >= warmup_cycles)
        .unwrap_or(0);
    let span = match (series.get(baseline_idx), series.last()) {
        (Some((base_at, _)), Some((final_at, _))) => final_at.saturating_sub(*base_at),
        _ => 0,
    };
    (baseline_idx, span)
}

/// The sample stride for a count-driven scenario: `ceil(n /
/// MAX_COUNT_SAMPLES)`, min 1 — per-cycle sampling while `n` fits the
/// sample ceiling, then a uniform cadence that keeps the instrument's
/// series at ≤ `MAX_COUNT_SAMPLES` + 1 entries whatever `n` is (the
/// 2026-11-10 instrument-memory regression, `MAX_COUNT_SAMPLES`
/// docs).
fn count_sample_stride(n: u64) -> u64 {
    n.div_ceil(MAX_COUNT_SAMPLES).max(1)
}

/// Pump until the engine's lifetime frame counter advances (bounded —
/// the per-cycle decode-evidence wait).
fn pump_until_frames(host: &mut SoakHost, frames_before: u64, limit: usize) {
    for _ in 0..limit {
        if host.stats().frames_delivered > frames_before {
            return;
        }
        host.pump();
    }
}

// ---------------------------------------------------------------------------
// The fd instrument (the handle-leak detector)
// ---------------------------------------------------------------------------

/// Count the process's open file descriptors — the handle-leak
/// instrument for the count-driven scenarios (Linux `/proc/self/fd`;
/// see the sibling lane below for the other platforms).
///
/// The `read_dir` handle itself occupies one descriptor for the
/// duration of the read — constant across samples, so growth
/// detection is unaffected (noted in the report).
#[cfg(target_os = "linux")]
#[must_use = "an unread sample proves nothing"]
pub fn read_fd_count() -> Option<u64> {
    std::fs::read_dir("/proc/self/fd")
        .ok()
        .map(|entries| entries.filter_map(|entry| entry.ok()).count() as u64)
}

/// The non-Linux lane: no `/proc` filesystem, no fd instrument. The
/// honest absence (an absent instrument is REPORTED, never faked as a
/// zero or a pass — the `VmRSS` precedent in `memory.rs`). The
/// Windows/macOS lanes land with their platform cards (the doc 10
/// cross-platform rule); wiring them needs `winapi`/`libproc` seams
/// that are dependency decisions, not soak decisions.
#[cfg(not(target_os = "linux"))]
#[must_use = "an unread sample proves nothing"]
pub fn read_fd_count() -> Option<u64> {
    None
}

// ---------------------------------------------------------------------------
// The slope verdict (pure — the forced-threshold proof surface)
// ---------------------------------------------------------------------------

/// The slope-budget verdict for one scenario (pure arithmetic over
/// the measured slope and the configured budgets — unit-tested, and
/// the function the forced-threshold run exercises end-to-end).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlopeVerdict {
    /// The measured live-bytes slope in MB/h (an EXTRAPOLATION from
    /// the measured window; `None` when no line was definable).
    pub live_mb_per_h: Option<f64>,
    /// Whether the slope is at/above the configured ceiling (the
    /// asserted CI budget).
    pub over_ceiling: bool,
    /// Whether the slope is at/above the configured design target
    /// (only meaningful when the profile asserts it).
    pub over_target: bool,
}

/// Check a measured live-bytes slope (B/s) against the configured
/// ceiling and target. A `None` slope (no definable line) passes
/// neither flag — the caller decides whether a degenerate window is
/// itself a failure (the duration scenarios record it as one).
#[must_use = "an unchecked slope proves nothing"]
pub fn check_slope(
    live_slope_b_per_s: Option<f64>,
    ceiling_mb_per_h: f64,
    target_mb_per_h: f64,
    assert_target: bool,
) -> SlopeVerdict {
    let live_mb_per_h = live_slope_b_per_s.map(|slope| slope * 3600.0 / 1e6);
    SlopeVerdict {
        live_mb_per_h,
        over_ceiling: live_mb_per_h.is_some_and(|mb| mb >= ceiling_mb_per_h),
        over_target: assert_target && live_mb_per_h.is_some_and(|mb| mb >= target_mb_per_h),
    }
}

// ---------------------------------------------------------------------------
// Runner unit tests (pure logic — no engine, no fixtures)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole-binary serializer (the counting allocator is
    /// process-global; a sibling test's allocations inside a measured
    /// window are pollution).
    fn serial() -> std::sync::MutexGuard<'static, ()> {
        testkit::alloc_count::serialize_tests()
    }

    /// 100 B/s = 0.36 MB/h: below the generous ceiling, above a
    /// tight one — the forced-threshold arithmetic, proven here in
    /// isolation (the end-to-end forced-threshold run lives in the
    /// suite report).
    #[test]
    fn slope_verdict_flags_budget_violations() {
        let _serial = serial();
        let clean = check_slope(Some(100.0), 50.0, 5.0, false);
        assert_eq!(clean.live_mb_per_h, Some(0.36));
        assert!(!clean.over_ceiling);
        assert!(!clean.over_target);

        let forced = check_slope(Some(100.0), 0.1, 0.05, true);
        assert!(forced.over_ceiling, "0.36 MB/h >= 0.1 MB/h ceiling");
        assert!(forced.over_target, "0.36 MB/h >= 0.05 MB/h target");

        // The target is only asserted when the profile says so: 3.6 MB/h
        // is above the 0.05 target but below the 50 ceiling, and with
        // assert_target = false neither flag fires.
        let reported_only = check_slope(Some(1000.0), 50.0, 0.05, false);
        assert!(!reported_only.over_ceiling);
        assert!(!reported_only.over_target);

        // No line, no verdict (the degenerate-window rule).
        let none = check_slope(None, 50.0, 5.0, true);
        assert_eq!(none.live_mb_per_h, None);
        assert!(!none.over_ceiling);
        assert!(!none.over_target);

        // A negative slope (memory returning) is a pass on both axes.
        let falling = check_slope(Some(-1000.0), 50.0, 5.0, true);
        assert!(falling.live_mb_per_h.is_some_and(|mb| mb < 0.0));
        assert!(!falling.over_ceiling);
        assert!(!falling.over_target);
    }

    /// The warmup-cycle arithmetic: fraction, floor, clamp to half,
    /// min 1, collapse-safe for tiny runs.
    #[test]
    fn count_warmup_cycles_clamps() {
        let _serial = serial();
        let defaults = Defaults {
            sample_ms: 1000,
            warmup_fraction: 0.15,
            min_cycles: 3,
            max_skip_rate: 0.25,
            leak_ceiling_mb_per_h: 50.0,
            target_mb_per_h: 5.0,
            assert_target: false,
            rss_growth_budget_mb: 8.0,
            fd_growth_budget: 4,
            live_growth_budget_bytes: 524_288,
        };
        assert_eq!(count_warmup_cycles(20, &defaults), 3);
        assert_eq!(count_warmup_cycles(4, &defaults), 1, "min 1");
        assert_eq!(count_warmup_cycles(2, &defaults), 1, "max n-1");
        assert_eq!(
            count_warmup_cycles(1, &defaults),
            0,
            "a single-cycle run measures it"
        );
        assert_eq!(count_warmup_cycles(100, &defaults), 15);
    }

    /// The bounded instrument cadence (the 2026-11-10 regression:
    /// per-cycle sampling at `cycles = 2_000_000` faulted in ~48 MB of
    /// series pages and tripped the RSS budget with a flat engine).
    /// The stride keeps the sample count ≤ MAX_COUNT_SAMPLES + 1 for
    /// any `n`, and stays 1 (per-cycle) at the original design points.
    #[test]
    fn count_sample_stride_bounds_the_instrument_memory() {
        let _serial = serial();
        // The original design points: per-cycle resolution.
        assert_eq!(count_sample_stride(1), 1);
        assert_eq!(count_sample_stride(1000), 1);
        assert_eq!(count_sample_stride(4000), 1);
        assert_eq!(count_sample_stride(MAX_COUNT_SAMPLES), 1);
        // One past the ceiling: every second cycle.
        assert_eq!(count_sample_stride(MAX_COUNT_SAMPLES + 1), 2);
        // The regression's scale: 2M cycles sample ≤ 4096 + 1 times.
        let n = 2_000_000u64;
        let stride = count_sample_stride(n);
        assert_eq!(stride, 489);
        assert!(
            n / stride <= MAX_COUNT_SAMPLES,
            "sample count {} exceeds the ceiling {}",
            n / stride,
            MAX_COUNT_SAMPLES
        );
        // The series capacity the runner preallocates from the stride
        // (three series × entry size): bounded whatever n is.
        let capacity = n / stride + 2;
        assert!(
            capacity <= MAX_COUNT_SAMPLES + 2,
            "preallocated capacity {capacity} is not bounded by the sample ceiling"
        );
        // A pathological huge n still bounds (no overflow, no zero).
        let huge = u32::MAX as u64;
        let stride = count_sample_stride(huge);
        assert!(stride >= 1);
        assert!(huge / stride <= MAX_COUNT_SAMPLES);
    }

    /// The post-warmup baseline/span arithmetic (the W24-R4 P2-3
    /// regression): recomputed from the committed raw evidence's own
    /// shapes — the 2M churn run (stride 489, warmup boundary 300,000:
    /// first tag at/after the boundary is 614×489 = 300,246 → span
    /// 1,699,754, NOT the 849,815 the synthesis had copied from the
    /// 1M forced-threshold run) and the 1M forced run (stride 245 →
    /// span 849,815, the number its violation sentences carry).
    #[test]
    fn count_baseline_and_span_matches_the_measured_tags() {
        let _serial = serial();
        // The 2M churn series shape: tags at every 489th cycle plus
        // the final at 2,000,000 (4,090 samples, 613 warmup-excluded
        // — the raw report's own counts).
        let series: Vec<(u64, isize)> = (1..=4089u64)
            .map(|k| (489 * k, 0))
            .chain([(2_000_000, 0)])
            .collect();
        assert_eq!(series.len(), 4090);
        let (idx, span) = count_baseline_and_span(&series, 300_000);
        assert_eq!(idx, 613, "the raw report's warmup-excluded count");
        assert_eq!(
            series[idx].0, 300_246,
            "the first tag at/after the boundary"
        );
        assert_eq!(span, 1_699_754, "2,000,000 − 300,246 (the P2-3 fix)");

        // The 150k reopen series shape (stride 37, boundary 22,500):
        // baseline tag 609×37 = 22,533 → span 127,467.
        let series: Vec<(u64, isize)> = (1..=4054u64)
            .map(|k| (37 * k, 0))
            .chain([(150_000, 0)])
            .collect();
        let (idx, span) = count_baseline_and_span(&series, 22_500);
        assert_eq!(series.len(), 4055);
        assert_eq!(idx, 608);
        assert_eq!(span, 127_467);

        // The 1M forced-threshold shape (stride 245, boundary 150,000):
        // span 849,815 — the number the raw violation sentences carry.
        let series: Vec<(u64, isize)> = (1..=4081u64)
            .map(|k| (245 * k, 0))
            .chain([(1_000_000, 0)])
            .collect();
        let (_, span) = count_baseline_and_span(&series, 150_000);
        assert_eq!(span, 849_815);

        // Degenerate shapes stay honest: no samples → (0, 0); a series
        // that never crosses the boundary measures from its first tag.
        assert_eq!(count_baseline_and_span(&[], 10), (0, 0));
        let series: Vec<(u64, isize)> = vec![(5, 0), (9, 0)];
        assert_eq!(count_baseline_and_span(&series, 100), (0, 4));
    }

    /// The expectation-path mapping (the recorded-truth layout).
    #[test]
    fn expectation_paths_map_generated_areas() {
        let _serial = serial();
        assert_eq!(
            expectation_for("generated/valid/avi_mjpeg_pcm.avi").must(),
            "expectations/valid/avi_mjpeg_pcm.avi.toml"
        );
        assert_eq!(
            expectation_for("generated/malformed/avi_mjpeg_pcm__trunc3.avi").must(),
            "expectations/malformed/avi_mjpeg_pcm__trunc3.avi.toml"
        );
        assert!(expectation_for("docs/elsewhere.avi").is_err());
        assert!(expectation_for("generated/bogus/x.avi").is_err());
        assert!(expectation_for("generated/flat.avi").is_err());
    }
}
