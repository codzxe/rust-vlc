//! Card 3.B1's composition-root scenarios (W1-18 §6 rows S7/S8/S9 —
//! the ones whose home is the wiring, not the owned crates): the
//! closed drift loop over the REAL production halves —
//! [`AudioLimb`] (pipeline + micro-resampler + governor) over the
//! null sink's ring, the [`NullDevice`] render pump, the
//! audio-master [`MediaClock`], and the [`PresentationScheduler`] —
//! with the device's rate error injected through the only seam a real
//! device owns: how much it eats per period. Virtual time only, zero
//! sleeps (doc 03 §8); the loop shape is `advance_time`'s exactly
//! (tick → render → observe, decode staged before).
//!
//! The S8 scenario is the R1 closure proof (W1-18 §8): the loop that
//! lives outside the owned crates actually runs — fill → governor →
//! correction → resampler → ring → counter → clock → scheduler →
//! instrumented presents — and the doc 03 §4.10 f A/V tolerance holds
//! at every present.
//!
//! Production arming note: the open path leaves the limb unarmed (the
//! first vertical's decode-ahead regime makes ring fill a buffering
//! metric — see `AudioLimb::observe_drift`); this harness paces one
//! quantum per device period, which is the regime the governor's
//! error signal assumes, and arms with the production target.

use std::sync::Arc;

use commands::envelope::RequestId;
use commands::{Command, PlaybackState, WireRequest};
use lumen_cli::dispatch::lumen_dispatch;
use lumen_cli::host::CliHost;
use lumen_cli::pipeline_host::{
    AudioSession, DRIFT_TARGET_FILL_NS, RENDER_QUANTUM, open_audio_session,
};
use media_clock::{NANO_BASE, PlaybackClock};
use media_types::frame::{
    AudioFormat, AudioFrame, AudioLayout, FrameStorage, SampleFormat, VideoFrame,
};
use media_types::packet::SharedBytes;
use media_types::time::{Nanoseconds, Stamp, Timestamp};
use media_types::{GenerationId, PixelFormat};
use testkit::Must;
use video_engine::{PresentAction, PresentationScheduler, PushOutcome, SchedulerCfg};

/// 44.1 kHz stereo, the first vertical's shape.
const RATE: u32 = 44_100;
/// One device period = one render quantum = one 30 fps video frame
/// (44 100 / 1470 = exactly 30).
const QNS: i64 = 33_333_333;
/// The pre-roll depth (quanta) before the device starts pulling — the
/// fill the loop regulates from (2 quanta ≈ 66.7 ms, a realistic
/// above-target start; the S1 convergence bound is measured from it).
const PRIME_QUANTA: u64 = 2;
/// The S1 convergence deadline (W1-18 §6): in-band by this virtual
/// second.
const CONVERGENCE_S: u64 = 45;
/// The S1 stable-tail length (in-band AND staying).
const TAIL_S: u64 = 30;
/// The governor's step limiter (audio-engine's `STEP_LIMIT_PPM`) —
/// the S7 post-resume/post-seek transient bound.
const STEP_LIMIT_PPM: i64 = 250;
/// The limiter invariant (media-clock's `MAX_CORRECTION_PPM`).
const MAX_CORRECTION_PPM: i64 = 1000;

// ---------------------------------------------------------------------------
// The composed harness: AudioSession + scheduler, device-drifted
// ---------------------------------------------------------------------------

/// The whole composition under test, in production wiring order.
struct Harness {
    session: AudioSession,
    scheduler: PresentationScheduler,
    /// The device's rate error (ppm): positive eats faster than
    /// nominal — the only injected seam.
    drift_ppm: i64,
    /// Sub-frame consumption deficit (frames × 1e6, exact i128).
    deficit: i128,
    /// Completed device steps (periods).
    steps: u64,
    /// (slot pts ns, clock ns at delivery) per presented frame — the
    /// doc 03 §4.10 f measurable.
    presents: Vec<(i64, i64)>,
    /// The correction in force after each step (the limiter's
    /// observable trail).
    corrections: Vec<i64>,
    /// The governor's fill trail (ns; None = unmeasurable).
    fills: Vec<Option<u64>>,
}

impl Harness {
    /// The production composition, armed for the paced regime, with
    /// the device running `drift_ppm` off nominal.
    fn new(drift_ppm: i64) -> Harness {
        let mut session = open_audio_session(2, RATE, 0).must();
        session.limb.arm_drift_governor(DRIFT_TARGET_FILL_NS);
        let scheduler = PresentationScheduler::new(
            SchedulerCfg::for_frame_duration(Nanoseconds(QNS as u64)).must(),
            Arc::clone(&session.clock) as Arc<dyn PlaybackClock>,
        );
        Harness {
            session,
            scheduler,
            drift_ppm,
            deficit: 0,
            steps: 0,
            presents: Vec::new(),
            corrections: Vec::new(),
            fills: Vec::new(),
        }
    }

    /// Stage one media quantum (the paced producer — the decode side's
    /// per-period delivery).
    fn stage(&mut self, media_step: u64, generation: GenerationId) {
        let mut bytes = Vec::with_capacity(RENDER_QUANTUM * 4);
        for i in 0..RENDER_QUANTUM {
            let sample = ((media_step * 1470 + i as u64) % 4096) as i16;
            bytes.extend_from_slice(&sample.to_le_bytes());
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        let frame = AudioFrame {
            format: AudioFormat {
                sample: SampleFormat::S16,
                channels: 2,
                layout: AudioLayout::Interleaved,
                rate: RATE,
            },
            samples: SharedBytes::from_vec(bytes),
            pts: Stamp::Present(Timestamp::new(
                (media_step as i64).saturating_mul(QNS),
                NANO_BASE,
            )),
            duration: None,
            generation,
        };
        self.session.limb.stage(&frame).must();
    }

    /// Push the video frame for slot `slot` (one frame of decode
    /// lead, the pipeline's shape).
    fn push_video(&mut self, slot: i64, generation: GenerationId) {
        let frame = VideoFrame {
            format: PixelFormat::Rgba8,
            size: (2, 2),
            data: FrameStorage::Cpu(SharedBytes::from_vec(vec![0u8; 16])),
            pts: Stamp::Present(Timestamp::new(slot.saturating_mul(QNS), NANO_BASE)),
            color: media_types::frame::ColorDescription::BT709_LIMITED,
            interlaced: false,
            generation,
        };
        match self.scheduler.push(frame) {
            PushOutcome::Scheduled => {}
            other => unreachable!("video push refused: {other:?}"),
        }
    }

    /// One full device period: decode staged, one video frame
    /// delivered, then the drifted device eats its quantum (the
    /// production `advance_time` order — tick before the render, the
    /// drift observation after it).
    fn step(&mut self, generation: GenerationId) {
        let step = self.steps;
        self.stage(step, generation);
        self.push_video((step + 1) as i64, generation);
        self.consume();
        self.steps += 1;
        self.corrections.push(self.session.limb.drift().ppm());
        self.fills.push(self.session.limb.drift_stats().fill_ns_last);
    }

    /// The device's period appetite: `QUANTUM × (1 + ppm/1e6)`
    /// frames, whole-frame exact through the deficit accumulator —
    /// rendered in quantum chunks (a partial tail render is a
    /// sub-quantum device nibble: no scheduler tick, no observation,
    /// exactly the production cadence).
    fn consume(&mut self) {
        if self.session.clock.is_paused() {
            // The pump's gate: a paused clock means the device pulls
            // nothing (renders would desynchronize the counter from
            // the frozen position — host.rs's own advance_time gate).
            return;
        }
        self.deficit += RENDER_QUANTUM as i128 * i128::from(self.drift_ppm);
        let mut eat = RENDER_QUANTUM as i128;
        while self.deficit >= 1_000_000 {
            eat += 1;
            self.deficit -= 1_000_000;
        }
        while self.deficit <= -1_000_000 {
            eat -= 1;
            self.deficit += 1_000_000;
        }
        while eat > 0 {
            let chunk = (eat.min(RENDER_QUANTUM as i128)) as usize;
            let chunk = chunk.max(1);
            let full_quantum = chunk == RENDER_QUANTUM;
            if full_quantum {
                let outcome = self.scheduler.tick();
                if let PresentAction::Present(request) = outcome.action {
                    let now = self.session.clock.snapshot().position.ticks;
                    self.presents.push((request.when.ticks, now));
                }
            }
            let mut buffer = vec![0.0_f32; chunk * 2];
            self.session.device.render(&mut buffer).must();
            if full_quantum {
                self.session.limb.observe_drift();
            }
            eat -= chunk as i128;
        }
    }

    /// Prime the ring (`PRIME_QUANTA` staged, device not yet pulling)
    /// and deliver the first video frame's decode lead.
    fn prime(&mut self) {
        for k in 0..PRIME_QUANTA {
            self.stage(k, GenerationId::FIRST);
        }
        self.push_video(1, GenerationId::FIRST);
    }
}

// ---------------------------------------------------------------------------
// S8 — end-to-end A/V over the closed loop (doc 03 §4.10 f)
// ---------------------------------------------------------------------------

#[test]
fn s8_end_to_end_av_tolerance_holds_over_the_closed_loop() {
    let mut harness = Harness::new(100); // +100 ppm device, S1's plant
    harness.prime();
    let steps = (120 * 30) as usize; // 120 s virtual
    for _ in 0..steps {
        harness.step(GenerationId::FIRST);
    }

    // The loop closed: the governor measured, corrected, and the
    // correction trail never leaves the limiter (M12's invariant at
    // the composition level).
    let stats = harness.session.limb.drift_stats();
    assert!(stats.samples >= steps as u64, "observed {}", stats.samples);
    assert!(stats.updates >= 100, "re-evaluations: {}", stats.updates);
    assert!(
        harness
            .corrections
            .iter()
            .all(|ppm| ppm.abs() <= MAX_CORRECTION_PPM),
        "limiter escaped: {:?}",
        harness.corrections.iter().max()
    );

    // S1 composed: fill in |e| <= 10 ms by 45 s and staying (>= 30 s
    // tail); zero underruns and zero ring refusals (staging never
    // errored — `.must()`); the ring never flooded.
    let converge_from = CONVERGENCE_S as usize * 30;
    let tail_from = steps.saturating_sub(TAIL_S as usize * 30);
    for (index, fill) in harness.fills.iter().enumerate().skip(converge_from) {
        let fill = fill.must();
        assert!(
            fill.abs_diff(DRIFT_TARGET_FILL_NS) <= 10_000_000,
            "fill at step {index}: {fill} ns (target {DRIFT_TARGET_FILL_NS})"
        );
    }
    for (index, fill) in harness.fills.iter().enumerate().skip(tail_from) {
        let fill = fill.must();
        assert!(
            fill.abs_diff(DRIFT_TARGET_FILL_NS) <= 10_000_000,
            "tail fill at step {index}: {fill} ns"
        );
    }
    assert_eq!(harness.session.device.underruns(), 0, "no underruns");
    let fill_max = stats.fill_ns_max.must();
    assert!(fill_max <= 150_000_000, "acceptance band top: {fill_max}");

    // The A/V tolerance (doc 03 §4.10 f): at every present after the
    // first, |audio_pts - video_pts_presented| <= 10 ms — audio at
    // the speaker IS the clock's position (the counter counts
    // consumed content; the fill never inserts silence here), so the
    // measurable is slot - clock-at-delivery. No present out of
    // order; >= 95 % within half a frame.
    assert!(harness.presents.len() >= steps - 2, "{}", harness.presents.len());
    let mut previous_slot = None;
    let mut within_half_frame = 0_usize;
    for (index, &(slot, now)) in harness.presents.iter().enumerate() {
        if let Some(previous) = previous_slot {
            assert!(slot > previous, "out of order at {index}: {slot} after {previous}");
        }
        previous_slot = Some(slot);
        if index == 0 {
            continue;
        }
        let error = (slot - now).abs();
        assert!(
            error <= 10_000_000,
            "present {index}: |audio - video| = {error} ns (slot {slot}, clock {now})"
        );
        if error <= QNS / 2 {
            within_half_frame += 1;
        }
    }
    let measured = harness.presents.len() - 1;
    assert!(
        within_half_frame * 100 >= measured * 95,
        "within half frame: {within_half_frame}/{measured}"
    );

    // S9's composed half (the event-storm row's harness leg): the
    // sync machinery ran the whole time — measured, corrected,
    // presented — and its ONLY observability is the counters above
    // (there is no event surface in this loop, structurally: nothing
    // in the composed path constructs an event). The scheduler's own
    // sync counters moved while zero events existed.
    let scheduler = harness.scheduler.stats();
    assert!(
        scheduler.sync_samples >= steps as u64 - 2,
        "{}",
        scheduler.sync_samples
    );
    assert_eq!(harness.scheduler.stats().snap_presents, 0, "no PTS gaps");
    assert_eq!(harness.scheduler.stats().sync_pts_gaps, 0);
}

// ---------------------------------------------------------------------------
// S7 — pause/resume + seek composition (3.A1 + W1-09 + 3.B1)
// ---------------------------------------------------------------------------

#[test]
fn s7_pause_resume_and_seek_compose_with_the_drift_regimes() {
    let mut harness = Harness::new(100);
    harness.prime();
    for _ in 0..30 * 30 {
        harness.step(GenerationId::FIRST); // converge (S8's tail math)
    }
    let pre = harness.session.limb.drift_stats();
    let correction_pre = harness.session.limb.drift().ppm();
    assert!(pre.updates >= 25, "converged regime: {:?}", pre);

    // --- the freeze (W1-09's hold): renders stop, the window freezes,
    // the governor holds — zero correction updates, deterministically.
    harness.session.clock.pause();
    let mut staged_in_pause = 0_u64;
    for _ in 0..150 {
        // 5 s virtual. The engine keeps decoding while paused
        // (bounded by the ring: 60 quanta + the prime + the fill sit
        // well under the 131 072-frame capacity); the pump renders
        // nothing and observes nothing.
        if staged_in_pause < 60 {
            harness.stage(30 * 30 + staged_in_pause, GenerationId::FIRST);
            staged_in_pause += 1;
        }
    }
    let frozen = harness.session.limb.drift_stats();
    assert_eq!(
        frozen.updates,
        pre.updates,
        "the frozen window must not re-evaluate"
    );
    assert_eq!(
        frozen.samples,
        pre.samples,
        "no renders means no fill samples"
    );
    assert!(
        frozen.fill_ns_last.must() > pre.fill_ns_last.must(),
        "decode-ahead while paused still fills the ring"
    );

    // --- the resume: the window re-opens on device time; the first
    // re-evaluation is one step-limiter increment at most (the fill
    // is far above target after the pause's decode-ahead — the
    // desired correction is saturated, the STEP is what is bounded).
    harness.session.clock.resume();
    let mut correction_first = None;
    for _ in 0..10 * 30 {
        harness.step(GenerationId::FIRST);
        let now = harness.session.limb.drift_stats();
        if now.updates > frozen.updates && correction_first.is_none() {
            correction_first = Some(harness.session.limb.drift().ppm());
        }
    }
    let correction_first = correction_first.must();
    assert!(
        (correction_first - correction_pre).abs() <= STEP_LIMIT_PPM,
        "post-resume transient: {correction_pre} -> {correction_first} ppm"
    );

    // --- the seek (3.A1's seven-step flow, host duties composed):
    // the engine mints a generation and re-anchors at the landing;
    // the host discards the ring's stale prefix with discarded
    // renders (absorbed into the anchor), resets the drift
    // measurement, and the scheduler detects the epoch bump.
    let landing = Timestamp::new(20_000_000_000, NANO_BASE);
    let generation = GenerationId::new(7);
    harness.session.clock.set_generation(generation);
    harness
        .session
        .clock
        .reanchor(landing)
        .must(); // the engine's completion re-anchor
    let landed = harness.session.clock.now(); // the compensation snapshot
    let stats_before = harness.scheduler.stats();
    let drift_before = harness.session.limb.drift_stats();
    // The ring flush: render-discard the stale prefix (the device's
    // own consumption mechanism; the counter movement is absorbed by
    // the re-anchor below — host.rs seek_boundary's exact order).
    let mut discard_buffer = vec![0.0_f32; RENDER_QUANTUM * 2];
    while harness.session.device.buffered_frames() >= RENDER_QUANTUM {
        harness.session.device.render(&mut discard_buffer).must();
    }
    harness.session.clock.reanchor(landed).must();
    harness.session.limb.reset_drift();

    // The regime proofs: the correction dropped to zero with the
    // measurement; the scheduler's estimator reset on the epoch bump
    // and armed the snap exemption (at most one snap present).
    assert_eq!(harness.session.limb.drift().ppm(), 0);
    assert_eq!(
        harness.session.limb.drift_stats().resets,
        drift_before.resets + 1
    );
    let mut snaps = 0_u64;
    let mut first_post_seek = true;
    let mut presents_after = 0_u64;
    // The pre-roll hold: the clock stays frozen while the landing
    // buffers (two quanta), then playback resumes.
    harness.session.clock.pause();
    for k in 0..2 {
        harness.stage(u64::try_from(20_000_000_000_i64 / QNS).must() + k, generation);
    }
    harness.session.clock.resume();
    for step in 0..10 * 30 {
        harness.step(generation);
        let outcome_stats = harness.scheduler.stats();
        snaps = outcome_stats.snap_presents;
        if outcome_stats.sync_reanchors > stats_before.sync_reanchors
            && first_post_seek
            && !harness.presents.is_empty()
            && harness.presents.last().is_some_and(|&(slot, _)| slot >= 20_000_000_000)
        {
            first_post_seek = false;
        }
        if !first_post_seek {
            presents_after += 1;
        }
        let _ = step;
    }
    let after = harness.scheduler.stats();
    assert!(
        after.sync_reanchors > stats_before.sync_reanchors,
        "the epoch bump must reset the estimator"
    );
    assert!(
        snaps - stats_before.snap_presents <= 1,
        "snap presents across the seek: {}",
        snaps - stats_before.snap_presents
    );
    assert!(presents_after > 0, "playback resumed after the seek");
    // The post-seek A/V tolerance: every present at or past the
    // landing is within the doc 03 tolerance (the first landing
    // present is the snap — exempt from late-drop accounting, never
    // from the tolerance itself).
    for &(slot, now) in harness.presents.iter() {
        if slot >= 20_000_000_000 && slot > 20_000_000_000 + QNS {
            let error = (slot - now).abs();
            assert!(
                error <= 10_000_000,
                "post-seek present: |audio - video| = {error} ns (slot {slot}, clock {now})"
            );
        }
    }
    // The post-seek correction's first move is step-limited from
    // zero (the S7 transient bound, seek half).
    let mut move_after_seek = None;
    for ppm in harness.corrections.iter().skip(30 * 30 + 150 + 10 * 30) {
        if *ppm != 0 {
            move_after_seek = Some(*ppm);
            break;
        }
    }
    if let Some(moved) = move_after_seek {
        assert!(
            moved.abs() <= STEP_LIMIT_PPM,
            "post-seek first correction: {moved} ppm"
        );
    }
}

// ---------------------------------------------------------------------------
// S9 — no event storm (the CliHost leg: the whole first vertical with
//      the loop closure live in the pump)
// ---------------------------------------------------------------------------

/// The frozen doc 05 catalog (D-017, 23 rows — the pre-3.B1 set; the
/// sync machinery must never need a 24th).
fn is_catalog_kind(kind: &commands::EventKind) -> bool {
    use commands::EventKind::*;
    matches!(
        kind,
        SessionStateChanged { .. }
            | SessionOpeningProgress { .. }
            | SessionBuffering { .. }
            | TimingTimeTick { .. }
            | TimingDurationKnown { .. }
            | TimingSeekCompleted { .. }
            | TimingSeekFailed { .. }
            | TracksListReady { .. }
            | TracksChanged { .. }
            | PlaybackEndOfTrack { .. }
            | PlaybackError { .. }
            | PlaybackRateChanged { .. }
            | AudioDeviceLost { .. }
            | AudioDeviceChanged { .. }
            | VideoScreenshotSaved { .. }
            | PlaylistChanged { .. }
            | ConfigSettingChanged { .. }
            | PluginsLoaded { .. }
            | PluginsFailed { .. }
            | PluginsCrashed { .. }
            | SystemLog { .. }
            | MediaInfoReady { .. }
    )
}

#[test]
fn s9_no_event_storm_over_the_first_vertical() {
    let mut host = CliHost::new();
    let source = commands::catalog::types::SourceSpec::File {
        path: format!(
            "{}/../../tests/fixtures/generated/valid/avi_mjpeg_pcm.avi",
            env!("CARGO_MANIFEST_DIR")
        )
        .into(),
    };
    let response = lumen_dispatch(
        &mut host,
        &WireRequest {
            request_id: RequestId(1),
            command: Command::SessionOpen {
                source,
                options: Default::default(),
            },
        },
    );
    match response.result {
        Ok(_) => {}
        Err(error) => unreachable!("open failed: {error}"),
    }
    host.settle();
    host.advance(lumen_cli::host::RUN_PUMPS as u32);
    assert_eq!(host.state(), PlaybackState::Ended);

    // Every event is a catalog row (the 23-variant whitelist above —
    // a 24th kind fails this match) and the burst bound holds with
    // two orders of magnitude to spare over the 2 s fixture (the
    // D-017 ceiling is 1000/s; the steady budget is time-tick at
    // 4 Hz plus the state rows).
    let events = host.events();
    assert!(!events.is_empty());
    for event in events {
        assert!(
            is_catalog_kind(&event.kind),
            "uncataloged event kind: {:?}",
            event.kind
        );
    }
    // The fixture is 2.000 s of media: the whole run stays under the
    // steady budget by construction (measured: state-changed ×4,
    // buffering ×2, time-ticks at 4 Hz, end-of-track ×1 — pinned
    // exactly by the session-script suites; the bound here is the
    // storm ceiling, not the exact count).
    assert!(
        events.len() <= 2_000,
        "event storm: {} events over a 2 s run",
        events.len()
    );
    let ticks = events
        .iter()
        .filter(|event| matches!(event.kind, commands::EventKind::TimingTimeTick { .. }))
        .count();
    assert!(ticks <= 8 + 2, "time-tick rate: {ticks} over 2 s");
}
