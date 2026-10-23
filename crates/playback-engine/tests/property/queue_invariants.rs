//! Family 2 (engine half) — queue invariants under playback storms
//! (card 3.D3; doc 03 §8 "Queue bounds under adversarial
//! interleavings §4.4(f)"). The queue *primitive's* own property
//! suite is `packet-queue/tests/property.rs`; this file proves the
//! engine keeps the primitive's laws when the pipeline drives it:
//!
//! - **bounded work per poll** — with a 16-packet queue plan and a
//!   1200-packet timeline (N ≫ cap), every poll is a bounded slice
//!   and the session still drains to `Ended` with **every** packet
//!   delivered exactly once (backpressure delays, it never drops:
//!   doc 03 Q3 "fresh data is never dropped");
//! - **backpressure exercised** — the packets-axis full-rejection
//!   counter moves (the cap genuinely refused pushes; a queue that
//!   never fills proves nothing), and the observability counters are
//!   monotone across polls;
//! - **nothing fabricated** — the collector never outgrows the
//!   cumulative demux budget (every frame is one demuxed packet, and
//!   one poll demuxes at most `budget.demux_events` = 12 events);
//! - **generations floor the data** — every delivered frame carries
//!   a generation ≤ the wire generation at its poll, and within a
//!   session the delivered-generation sequence never decreases (a
//!   flushed generation's stragglers are dropped at the sink, never
//!   delivered — the engine-level stale-drop law).

use media_types::{MediaKind, TrackId};
use playback_engine::{Engine, EngineStats, SessionState};

use super::engine_fx::{Collector, Delivered, Fixture, Slot, open, track};
use super::rng::{XorShift, run_case, seeds};

/// The timeline length: N ≫ cap (1200 packets vs the 16-packet plan).
const PACKETS: usize = 1200;
/// Milliseconds in nanoseconds.
const MS: i64 = 1_000_000;
/// The demux slice of the fixture's tick budget (`tiny_budget`).
const DEMUX_EVENTS_PER_POLL: u64 = 12;

fn audio_timeline() -> (Vec<media_types::Track>, Vec<Slot>) {
    let slots = (0..PACKETS)
        .map(|index| Slot {
            track: TrackId::new(1),
            pts_ns: index as i64 * MS,
            dts_ns: None,
        })
        .collect();
    (
        vec![track(1, MediaKind::Audio, Some(PACKETS as i64 * MS))],
        slots,
    )
}

/// One storm op.
#[derive(Debug, Clone, Copy)]
enum Op {
    Poll,
    Pause,
    Play,
    Seek { position_ns: i64 },
}

fn generate_ops(rng: &mut XorShift, count: usize) -> Vec<Op> {
    (0..count)
        .map(|_| match rng.below(10) {
            0..=3 => Op::Poll,
            4 => Op::Pause,
            5 => Op::Play,
            _ => Op::Seek {
                position_ns: rng.below(PACKETS as u64) as i64 * MS,
            },
        })
        .collect()
}

/// The per-poll observer: every law, checked after every single
/// poll (the prime, the storm, and the settle all run through it —
/// the fabrication bound needs the exact poll count).
struct Observer {
    polls: u64,
    last_frame_count: usize,
    last_generation: u64,
    last_stats: EngineStats,
}

impl Observer {
    fn new(collector: &Collector) -> Observer {
        Observer {
            polls: 0,
            last_frame_count: collector.len(),
            last_generation: 0,
            last_stats: EngineStats::default(),
        }
    }

    fn observe(
        &mut self,
        engine: &Engine,
        collector: &Collector,
        context: &str,
    ) -> Result<(), String> {
        self.polls += 1;
        let stats = engine.stats();
        let wire_generation = engine.generation().map_or(0, |g| g.0);
        // Monotone counters (lifetime observability never rewinds).
        let monotone = [
            (
                stats.frames_delivered,
                self.last_stats.frames_delivered,
                "frames_delivered",
            ),
            (
                stats.full_rejections_packets,
                self.last_stats.full_rejections_packets,
                "full_rejections_packets",
            ),
            (
                stats.stale_frames_dropped,
                self.last_stats.stale_frames_dropped,
                "stale_frames_dropped",
            ),
            (
                stats.preroll_frames_discarded,
                self.last_stats.preroll_frames_discarded,
                "preroll_frames_discarded",
            ),
        ];
        for (now, before, name) in monotone {
            if now < before {
                return Err(format!(
                    "{context}: counter {name} rewound {before} -> {now}"
                ));
            }
        }
        // Nothing fabricated, CUMULATIVELY: every delivered frame is
        // one demuxed packet, and one poll demuxes at most
        // `budget.demux_events` events — so the collector can never
        // outgrow poll count x demux slice. (A per-window bound would
        // be unsound: a window may drain a backlog queued by earlier
        // windows; only the cumulative identity is a law.)
        let total_frames = collector.len() as u64;
        if total_frames > self.polls * DEMUX_EVENTS_PER_POLL {
            return Err(format!(
                "{context}: {total_frames} frames after {} polls -- beyond the cumulative \
                 demux budget ({} x {}): frames were fabricated",
                self.polls, self.polls, DEMUX_EVENTS_PER_POLL
            ));
        }
        // Generation floors the data: every newly delivered frame is
        // at or below the live wire generation, and the sequence
        // never decreases within the session.
        for delivered in collector.frames_from(self.last_frame_count) {
            let Delivered {
                generation, pts_ns, ..
            } = delivered;
            if generation > wire_generation {
                return Err(format!(
                    "{context}: frame of generation {generation} delivered under wire \
                     generation {wire_generation} (pts {pts_ns:?}) -- a future generation \
                     cannot carry data"
                ));
            }
            if generation < self.last_generation {
                return Err(format!(
                    "{context}: delivered generation rewound {} -> {generation} (pts {pts_ns:?}) \
                     -- a flushed generation's stragglers must drop, never deliver",
                    self.last_generation
                ));
            }
            self.last_generation = generation;
        }
        self.last_frame_count = collector.len();
        self.last_stats = stats;
        Ok(())
    }
}

/// One poll through the observer: poll, drain the outbox, run the
/// full law set. Every poll of the storm's lifetime goes through
/// here.
fn poll_and_observe(
    fixture: &mut Fixture,
    observer: &mut Observer,
    context: &str,
) -> Result<(), String> {
    fixture.engine.poll();
    drop(fixture.engine.take_outputs());
    observer.observe(&fixture.engine, &fixture.collector, context)
}

fn run(seed: u64, ops: &[Op]) -> Result<(), String> {
    let canonical = ops.is_empty();
    let mut rng = XorShift::new(seed);
    let ops = if canonical {
        generate_ops(&mut rng, 96)
    } else {
        ops.to_vec()
    };
    let (tracks, slots) = audio_timeline();
    let mut fixture = open(tracks, slots, true);
    let mut observer = Observer::new(&fixture.collector);
    // Prime to Playing first (the storm's seek-legal state) — every
    // prime poll observed too (the fabrication bound counts them).
    for poll in 0..32u64 {
        let context = format!("seed {seed} prime poll {poll}");
        poll_and_observe(&mut fixture, &mut observer, &context)?;
        if fixture.engine.state() == SessionState::Playing {
            break;
        }
    }
    if fixture.engine.state() != SessionState::Playing {
        return Err(format!("seed {seed}: never reached Playing"));
    }

    let mut request = 100u64;
    for (index, op) in ops.iter().enumerate() {
        let step = format!("seed {seed} op {index} ({op:?})");
        match *op {
            Op::Poll => {}
            Op::Pause => {
                request += 1;
                let _ = fixture.handle.pause(commands::envelope::RequestId(request));
            }
            Op::Play => {
                request += 1;
                let _ = fixture.handle.play(commands::envelope::RequestId(request));
            }
            Op::Seek { position_ns } => {
                request += 1;
                let _ = fixture.handle.seek(
                    commands::envelope::RequestId(request),
                    playback_engine::SeekTarget::Absolute { position_ns },
                );
            }
        }
        // Every op is followed by a bounded poll slice, each poll
        // through the full law set.
        for poll in 0..4u64 {
            let context = format!("{step} poll {poll}");
            poll_and_observe(&mut fixture, &mut observer, &context)?;
        }
    }
    // Bounded liveness: the session settles. A storm that ends while
    // Paused is resumed first (paused polls advance nothing -- the
    // settle driver's own law, not an engine one).
    let mut resumed = false;
    for poll in 0..4000u64 {
        if !resumed && fixture.engine.state() == SessionState::Paused {
            request += 1;
            let _ = fixture.handle.play(commands::envelope::RequestId(request));
            resumed = true;
        }
        let context = format!("seed {seed} settle poll {poll}");
        poll_and_observe(&mut fixture, &mut observer, &context)?;
        if fixture.engine.state() == SessionState::Ended {
            break;
        }
    }
    let state = fixture.engine.state();
    if state != SessionState::Ended {
        return Err(format!(
            "seed {seed}: storm never settled -- state {state} after 4000 settle polls"
        ));
    }
    Ok(())
}

/// The storm law set over the full bank: 64 seeds x 96 ops of
/// pause/play/seek interleavings over a 1200-packet timeline on a
/// 16-packet queue plan — bounds, monotone counters, no fabrication,
/// generation floors, bounded settle.
#[test]
fn queue_plan_bounds_hold_under_playback_storms() {
    for seed in seeds() {
        let mut rng = XorShift::new(seed);
        let ops = generate_ops(&mut rng, 96);
        if let Err(failure) = run_case(seed, &ops, run) {
            unreachable!("{failure}");
        }
    }
}

/// The N ≫ cap drain law, no storm: a 1200-packet timeline on a
/// 16-packet queue plan plays to `Ended` with every packet delivered
/// exactly once, backpressure genuinely exercised (a nonzero
/// packets-axis rejection count), zero stale drops (no mints
/// happened), and bounded polls. The drain is fully deterministic
/// (no seed feeds it — the storm above is the seeded half), so one
/// run is the whole law; the poll count and frame count are the
/// honest report.
#[test]
fn long_media_drains_exactly_once_with_backpressure_exercised() {
    let (tracks, slots) = audio_timeline();
    let mut fixture = open(tracks, slots, true);
    let mut polls = 0u32;
    while fixture.engine.state() != SessionState::Ended && polls < 4000 {
        fixture.engine.poll();
        drop(fixture.engine.take_outputs());
        polls += 1;
    }
    let state = fixture.engine.state();
    if state != SessionState::Ended {
        unreachable!("state {state} after {polls} polls -- not a bounded drain");
    }
    let frames = fixture.collector.len();
    if frames != PACKETS {
        unreachable!(
            "{frames} frames delivered for {PACKETS} packets -- backpressure must delay \
             fresh data, never drop it ({polls} polls)"
        );
    }
    let stats = fixture.engine.stats();
    if stats.full_rejections_packets == 0 {
        unreachable!(
            "zero packets-axis rejections over {PACKETS} packets on a 16-packet cap -- the \
             bound was never exercised (backpressure unproven)"
        );
    }
    if stats.stale_frames_dropped != 0 || stats.preroll_frames_discarded != 0 {
        unreachable!(
            "stale drops ({}) or preroll discards ({}) with no mint -- nothing was stale",
            stats.stale_frames_dropped, stats.preroll_frames_discarded
        );
    }
}

/// The demux-cursor bound: with the timeline consumed, further polls
/// deliver nothing (the collector stops growing — memory is bounded
/// by the media, not by poll count).
#[test]
fn exhausted_media_delivers_nothing_more() {
    let (tracks, slots) = audio_timeline();
    let mut fixture = open(tracks, slots, true);
    while fixture.engine.state() != SessionState::Ended {
        fixture.engine.poll();
        drop(fixture.engine.take_outputs());
    }
    let settled = fixture.collector.len();
    for _ in 0..64 {
        fixture.engine.poll();
        drop(fixture.engine.take_outputs());
    }
    assert_eq!(
        fixture.collector.len(),
        settled,
        "polls past the end fabricated {} frames",
        fixture.collector.len() - settled
    );
}
