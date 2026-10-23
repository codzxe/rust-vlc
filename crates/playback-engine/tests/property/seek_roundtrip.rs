//! Family 4 — seek roundtrips (card 3.D3; doc 03 §8 → MP §13's
//! "repeated seeks" scenario): seek-to-time → first-emitted-frame
//! boundary properties on scripted synthetic timelines.
//!
//! Per seed, a synthetic timeline is generated (audio-only or
//! interleaved A/V on a stride grid, declared extent known or
//! unknown, `dts < pts` B-lead and `pts < dts` adversarial stamp
//! shapes — stamps are data, the engine must never panic on them),
//! then a list of seek targets (absolute in/out of range, relative,
//! percent, chapter; negative and past-the-end included). Every
//! completed seek must satisfy:
//!
//! - **exactly one terminal** for its generation — `Seeked` (or the
//!   typed `SeekFailed` for the unresolvable/unseekable shapes);
//! - **the reported `requested` is the clamped target** (clamping is
//!   truth, doc 05 §2.4 — the oracle mirrors `resolve_target`);
//! - **the reported `actual` is the anchor truth**: the first audio
//!   frame's own pts at or after the keyframe-safe landing, else the
//!   landing itself (doc 03 §6 step 6);
//! - **the first-emitted-frame boundary**: every frame delivered
//!   from the seek on carries the seek's generation and a pts at or
//!   after the landing — a pre-landing timestamp or a stale
//!   generation after the terminal is a defect; the first audio-limb
//!   frame is the anchor itself;
//! - **the clock re-anchored at `actual`**;
//! - **bounded completion** (a terminal within a bounded poll
//!   budget — a hang is a failed test).
//!
//! A supersession storm (many seeks, no waits between) pins the
//! doc 05 §4.3 invariant: one terminal per completed generation,
//! superseded generations stay silent, and the final accepted seek
//! completes.

use media_types::{MediaKind, TrackId};
use playback_engine::{
    EngineEvent, EngineEventKind, EngineOutput, ReceiptResult, SeekFailure, SeekTarget,
    SessionState,
};
use testkit::Must;

use super::engine_fx::{Delivered, Fixture, Slot, landing_of, open, resolve_target_ns, track};
use super::rng::{XorShift, run_case, seeds};

/// Milliseconds in nanoseconds.
const MS: i64 = 1_000_000;
/// Grid steps per track.
const STEPS: usize = 200;
/// The poll budget for one seek's completion (a hang is a failure).
const TERMINAL_BUDGET: usize = 96;
/// The settle budget after a terminal (re-prime to a steady state).
const SETTLE_BUDGET: usize = 96;

/// One generated timeline.
#[derive(Debug, Clone)]
struct Timeline {
    tracks: Vec<media_types::Track>,
    slots: Vec<Slot>,
    duration_ns: Option<i64>,
    audio_track: TrackId,
}

/// The timeline's declared extent (the grid's full span).
fn extent_of(stride_ms: i64) -> i64 {
    STEPS as i64 * 2 * stride_ms * MS
}

/// Generate the timeline from the seed's rng (deterministic per
/// seed — the shrink harness reruns prefixes against the same one).
fn generate_timeline(rng: &mut XorShift) -> (Timeline, i64) {
    let stride_ms = *rng.pick(&[3i64, 5, 7]);
    let av = rng.flip();
    let declared = rng.below(4) != 0;
    build_timeline(av, stride_ms, declared, rng.below(3))
}

/// Build a timeline: video (1) at even strides when `av`, audio (2)
/// at odd offsets — globally pts-sorted, all keyframes. `dts_shape`:
/// 0 = missing DTS, 1 = B-lead (`dts < pts`, dipping negative at the
/// head), 2 = the adversarial `pts < dts`.
fn build_timeline(av: bool, stride_ms: i64, declared: bool, dts_shape: u64) -> (Timeline, i64) {
    let extent = STEPS as i64 * 2 * stride_ms * MS;
    let audio_track = TrackId::new(if av { 2 } else { 1 });
    let mut slots = Vec::with_capacity(STEPS * 2);
    for step in 0..STEPS {
        let base_ns = step as i64 * 2 * stride_ms * MS;
        if av {
            slots.push(Slot {
                track: TrackId::new(1),
                pts_ns: base_ns,
                dts_ns: None,
            });
        }
        let pts_ns = base_ns + stride_ms * MS;
        let dts_ns = match dts_shape {
            0 => None,
            1 => Some(pts_ns - 2 * stride_ms * MS),
            _ => Some(pts_ns + stride_ms * MS),
        };
        slots.push(Slot {
            track: audio_track,
            pts_ns,
            dts_ns,
        });
    }
    let tracks = if av {
        vec![
            track(1, MediaKind::Video, declared.then_some(extent)),
            track(2, MediaKind::Audio, declared.then_some(extent)),
        ]
    } else {
        vec![track(1, MediaKind::Audio, declared.then_some(extent))]
    };
    (
        Timeline {
            tracks,
            slots,
            duration_ns: declared.then_some(extent),
            audio_track,
        },
        stride_ms,
    )
}

/// One seek request in the op stream.
#[derive(Debug, Clone, Copy)]
enum SeekOp {
    Absolute { position_ns: i64 },
    Relative { delta_ns: i64 },
    Percent { fraction: f32 },
    Chapter { index: u32 },
}

impl SeekOp {
    fn target(self) -> SeekTarget {
        match self {
            SeekOp::Absolute { position_ns } => SeekTarget::Absolute { position_ns },
            SeekOp::Relative { delta_ns } => SeekTarget::Relative { delta_ns },
            SeekOp::Percent { fraction } => SeekTarget::Percent { fraction },
            SeekOp::Chapter { index } => SeekTarget::Chapter { index },
        }
    }

    /// Whether resolution depends on the live position (relative
    /// targets) — the storm only drives position-independent ops.
    fn position_dependent(self) -> bool {
        matches!(self, SeekOp::Relative { .. })
    }
}

fn generate_seeks(rng: &mut XorShift, count: usize, extent_ns: i64) -> Vec<SeekOp> {
    (0..count)
        .map(|_| match rng.below(8) {
            0..=2 => SeekOp::Absolute {
                position_ns: rng.below(extent_ns as u64 * 2) as i64,
            },
            3 => SeekOp::Absolute {
                position_ns: -(rng.below(10) as i64 + 1) * MS,
            },
            4 => SeekOp::Absolute {
                position_ns: extent_ns + rng.below(extent_ns as u64) as i64,
            },
            5 => SeekOp::Relative {
                delta_ns: if rng.flip() {
                    rng.below(40) as i64 * MS
                } else {
                    -(rng.below(40) as i64 * MS)
                },
            },
            6 => SeekOp::Percent {
                fraction: rng.i64() as f32 / i64::MAX as f32,
            },
            _ => SeekOp::Chapter {
                index: rng.below(4) as u32,
            },
        })
        .collect()
}

/// The expected outcome of one seek, computed from the oracle.
#[derive(Debug)]
enum Expected {
    /// Completes: (clamped target, landing pts, anchor-truth actual).
    Completed {
        requested: i64,
        landing_ns: i64,
        actual_ns: i64,
    },
    /// The typed refusal (unresolvable target or unseekable source).
    Refused(SeekFailure),
}

fn expect(op: SeekOp, timeline: &Timeline, seekable: bool, position_ns: i64) -> Expected {
    let Some(requested) = resolve_target_ns(op.target(), timeline.duration_ns, position_ns) else {
        return Expected::Refused(SeekFailure::OutOfRange);
    };
    if !seekable {
        return Expected::Refused(SeekFailure::NotSeekable);
    }
    let (landing_index, landing_ns) = landing_of(&timeline.slots, requested);
    // The anchor truth: the first audio frame at or after the
    // landing, else the landing itself (the limb that can provably
    // never deliver hands the anchor to the landing).
    let actual_ns = timeline.slots[landing_index..]
        .iter()
        .find(|slot| slot.track == timeline.audio_track)
        .map_or(landing_ns, |slot| slot.pts_ns);
    Expected::Completed {
        requested,
        landing_ns,
        actual_ns,
    }
}

/// Find the terminal (`Seeked`/`SeekFailed`) of `generation` in the
/// output stream.
fn terminal_of(outputs: &[EngineOutput], generation: u64) -> Option<&EngineEventKind> {
    outputs.iter().find_map(|output| match output {
        EngineOutput::Event(event) if event.generation.0 == generation => match &event.kind {
            kind @ (EngineEventKind::Seeked { .. } | EngineEventKind::SeekFailed { .. }) => {
                Some(kind)
            }
            _ => None,
        },
        _ => None,
    })
}

fn run(seed: u64, ops: &[SeekOp]) -> Result<(), String> {
    let canonical = ops.is_empty();
    let mut rng = XorShift::new(seed);
    let (timeline, stride_ms) = generate_timeline(&mut rng);
    let ops = if canonical {
        generate_seeks(&mut rng, 6, extent_of(stride_ms))
    } else {
        ops.to_vec()
    };
    let seekable = true;
    let mut fixture = open(timeline.tracks.clone(), timeline.slots.clone(), seekable);
    let mut request = 0u64;
    let mut position_ns = 0i64;

    // Prime to a seek-legal state.
    for _ in 0..64 {
        fixture.engine.poll();
        drop(fixture.engine.take_outputs());
        if fixture.engine.state() == SessionState::Playing {
            break;
        }
    }
    if fixture.engine.state() != SessionState::Playing {
        return Err(format!(
            "seed {seed}: never reached Playing (state {})",
            fixture.engine.state()
        ));
    }

    for (index, op) in ops.iter().enumerate() {
        let step = format!("seed {seed} seek {index} ({op:?})");
        let state_before = fixture.engine.state();
        let generation_before = fixture.engine.generation().map_or(0, |g| g.0);
        let frame_count_before = fixture.collector.len();

        request += 1;
        let request_id = commands::envelope::RequestId(request);
        fixture
            .handle
            .seek(request_id, op.target())
            .map_err(|error| format!("{step}: submit refused {error:?}"))?;
        fixture.engine.poll();
        let outputs = fixture.engine.take_outputs();
        let generation_after = fixture.engine.generation().map_or(0, |g| g.0);

        if state_before == SessionState::Ended {
            // From Ended the seek refuses typed: no mint, no terminal.
            let rejected = outputs.iter().any(|output| match output {
                EngineOutput::Receipt(receipt) if receipt.request == request_id => {
                    matches!(receipt.result, ReceiptResult::Rejected { .. })
                }
                _ => false,
            });
            if !rejected {
                return Err(format!(
                    "{step}: a seek from Ended was not a typed rejection"
                ));
            }
            if generation_after != generation_before {
                return Err(format!(
                    "{step}: the rejected seek minted {generation_before} → {generation_after}"
                ));
            }
            continue;
        }

        // The class-A accept: exactly one mint.
        let accepted = outputs.iter().any(|output| match output {
            EngineOutput::Receipt(receipt) if receipt.request == request_id => {
                matches!(
                    receipt.result,
                    ReceiptResult::Accepted {
                        generation,
                        ..
                    } if generation.0 == generation_before + 1
                )
            }
            _ => false,
        });
        if !accepted {
            return Err(format!(
                "{step}: no class-A accept at generation {} from state {state_before}",
                generation_before + 1
            ));
        }
        let seek_generation = generation_before + 1;

        // Bounded completion: the terminal arrives within the budget.
        let mut terminal = terminal_of(&outputs, seek_generation).cloned();
        let mut polls = 0usize;
        while terminal.is_none() && polls < TERMINAL_BUDGET {
            fixture.engine.poll();
            let outputs = fixture.engine.take_outputs();
            terminal = terminal_of(&outputs, seek_generation).cloned();
            polls += 1;
        }
        let Some(terminal) = terminal else {
            return Err(format!(
                "{step}: no terminal within {TERMINAL_BUDGET} polls — a seek must complete \
                 or fail typed, never hang"
            ));
        };
        let expected = expect(*op, &timeline, seekable, position_ns);
        match (&terminal, &expected) {
            (
                EngineEventKind::Seeked { requested, actual },
                Expected::Completed {
                    requested: want_requested,
                    landing_ns,
                    actual_ns: want_actual,
                },
            ) => {
                if requested != want_requested {
                    return Err(format!(
                        "{step}: requested {requested}, the clamped target is {want_requested}"
                    ));
                }
                if actual != want_actual {
                    return Err(format!(
                        "{step}: actual {actual}, the anchor truth is {want_actual} \
                         (landing {landing_ns})"
                    ));
                }
                position_ns = *actual;
                // The first-emitted-frame boundary: every frame from
                // the seek on carries its generation and a pts at or
                // after the landing; the first audio-limb frame is
                // the anchor itself.
                let frames = fixture.collector.frames_from(frame_count_before);
                if frames.is_empty() {
                    return Err(format!(
                        "{step}: the completed seek delivered no frames at all"
                    ));
                }
                let mut first_audio: Option<i64> = None;
                for Delivered {
                    track,
                    generation,
                    pts_ns,
                } in frames
                {
                    if generation != seek_generation {
                        return Err(format!(
                            "{step}: a frame of generation {generation} delivered under the \
                             seek's generation {seek_generation} (pts {pts_ns:?}) — stale output"
                        ));
                    }
                    let Some(pts) = pts_ns else {
                        return Err(format!(
                            "{step}: a delivered frame lost its pts (the timeline always stamps)"
                        ));
                    };
                    if pts < *landing_ns {
                        return Err(format!(
                            "{step}: post-seek frame at {pts} ns is before the landing \
                             {landing_ns} ns — a pre-landing timestamp escaped the flush"
                        ));
                    }
                    if track == timeline.audio_track && first_audio.is_none() {
                        first_audio = Some(pts);
                    }
                }
                let Some(first) = first_audio else {
                    return Err(format!(
                        "{step}: the audio limb delivered nothing post-seek"
                    ));
                };
                if first != *want_actual {
                    return Err(format!(
                        "{step}: the first audio frame sits at {first} ns, the anchor truth \
                         is {want_actual} ns"
                    ));
                }
                // The clock re-anchored at the truth.
                let anchor = fixture.clock.anchor().media.ticks;
                if anchor != *want_actual {
                    return Err(format!(
                        "{step}: the clock anchor is {anchor}, the truth is {want_actual}"
                    ));
                }
            }
            (EngineEventKind::SeekFailed { reason, position }, Expected::Refused(want)) => {
                if reason != want {
                    return Err(format!(
                        "{step}: refused with {reason:?}, expected {want:?}"
                    ));
                }
                if *position != position_ns {
                    return Err(format!(
                        "{step}: the refusal reports position {position}, the session stood \
                         at {position_ns}"
                    ));
                }
            }
            (got, want) => {
                return Err(format!(
                    "{step}: terminal {got:?} disagrees with the oracle {want:?}"
                ));
            }
        }

        // Settle: re-prime to a steady state (or drain to Ended —
        // both legal landings), bounded.
        for _ in 0..SETTLE_BUDGET {
            if matches!(
                fixture.engine.state(),
                SessionState::Playing | SessionState::Paused | SessionState::Ended
            ) {
                break;
            }
            fixture.engine.poll();
            drop(fixture.engine.take_outputs());
        }
    }
    Ok(())
}

/// The roundtrip law set over the full bank: 64 seeds × 6 seeks on
/// generated timelines (A/V shapes, B-lead and pts<dts stamps,
/// known/unknown extents) — clamped targets, anchor-truth actuals,
/// first-emitted-frame boundaries, clock re-anchors, typed refusals,
/// bounded completion.
#[test]
fn seeks_roundtrip_with_first_frame_boundaries() {
    for seed in seeds() {
        let mut rng = XorShift::new(seed);
        // The timeline is regenerated inside `run` from the same seed
        // (same rng draws); here only the stride matters, for the
        // seek generator's extent.
        let (_, stride_ms) = generate_timeline(&mut rng);
        let ops = generate_seeks(&mut rng, 6, extent_of(stride_ms));
        if let Err(failure) = run_case(seed, &ops, run) {
            unreachable!("{failure}");
        }
    }
}

// ---------------------------------------------------------------------------
// The boundary anchors (fixed values — the committed regression
// surface every property failure shrinks toward)
// ---------------------------------------------------------------------------

fn prime_to_playing(fixture: &mut Fixture, context: &str) {
    for _ in 0..64 {
        fixture.engine.poll();
        drop(fixture.engine.take_outputs());
        if fixture.engine.state() == SessionState::Playing {
            return;
        }
    }
    unreachable!(
        "{context}: state {} — never reached Playing",
        fixture.engine.state()
    );
}

/// Drive one seek to its terminal and return it (bounded).
fn drive_to_terminal(fixture: &mut Fixture, generation: u64, context: &str) -> EngineEventKind {
    for _ in 0..TERMINAL_BUDGET {
        fixture.engine.poll();
        let outputs = fixture.engine.take_outputs();
        if let Some(kind) = terminal_of(&outputs, generation) {
            return kind.clone();
        }
    }
    unreachable!("{context}: no terminal within {TERMINAL_BUDGET} polls");
}

/// Seek to exactly zero: lands at the start, the first post-seek
/// frame is the media's first frame again. The timeline anchors its
/// first frame AT zero (pts 0), so the zero seek's landing, anchor
/// and first frame are all exactly zero.
#[test]
fn seek_to_zero_lands_at_the_first_frame() {
    // A dedicated zero-anchored grid: 64 audio packets, pts = k*40 ms,
    // all keyframes, declared extent 2560 ms, the B-lead dts shape.
    let slots: Vec<Slot> = (0..64)
        .map(|step| Slot {
            track: TrackId::new(1),
            pts_ns: step * 40 * MS,
            dts_ns: Some(step * 40 * MS - 5 * MS),
        })
        .collect();
    let timeline = Timeline {
        tracks: vec![track(1, MediaKind::Audio, Some(2560 * MS))],
        slots,
        duration_ns: Some(2560 * MS),
        audio_track: TrackId::new(1),
    };
    let mut fixture = open(timeline.tracks.clone(), timeline.slots.clone(), true);
    prime_to_playing(&mut fixture, "zero");
    let frames_before = fixture.collector.len();
    let generation = fixture.engine.generation().must().0 + 1;
    fixture
        .handle
        .seek(
            commands::envelope::RequestId(1),
            SeekTarget::Absolute { position_ns: 0 },
        )
        .must();
    match drive_to_terminal(&mut fixture, generation, "zero") {
        EngineEventKind::Seeked { requested, actual } => {
            assert_eq!(
                (requested, actual),
                (0, 0),
                "zero clamps to zero, lands at zero"
            );
        }
        other => unreachable!("zero seek: {other:?}"),
    }
    let frames = fixture.collector.frames_from(frames_before);
    let first = frames.first().must();
    assert_eq!(
        first.pts_ns,
        Some(0),
        "the first post-seek frame is frame 0 again"
    );
    assert_eq!(first.generation, generation);
}

/// A far-past-end absolute target clamps to the declared duration
/// (clamping is truth), lands on the last keyframe, and the media
/// then ends cleanly.
#[test]
fn past_end_clamps_then_ends_cleanly() {
    let (timeline, _) = build_timeline(true, 3, true, 0);
    let extent = timeline.duration_ns.must();
    let mut fixture = open(timeline.tracks.clone(), timeline.slots.clone(), true);
    prime_to_playing(&mut fixture, "past-end");
    let generation = fixture.engine.generation().must().0 + 1;
    fixture
        .handle
        .seek(
            commands::envelope::RequestId(1),
            SeekTarget::Absolute {
                position_ns: i64::MAX / 2,
            },
        )
        .must();
    match drive_to_terminal(&mut fixture, generation, "past-end") {
        EngineEventKind::Seeked { requested, actual } => {
            let (landing_index, landing_ns) = landing_of(&timeline.slots, extent);
            let expected_actual = timeline.slots[landing_index..]
                .iter()
                .find(|slot| slot.track == timeline.audio_track)
                .map_or(landing_ns, |slot| slot.pts_ns);
            assert_eq!(requested, extent, "the huge target clamps to the duration");
            assert_eq!(
                actual, expected_actual,
                "the landing is the last keyframe truth"
            );
        }
        other => unreachable!("past-end seek: {other:?}"),
    }
    // The media then ends cleanly, bounded.
    for _ in 0..2048 {
        fixture.engine.poll();
        drop(fixture.engine.take_outputs());
        if fixture.engine.state() == SessionState::Ended {
            return;
        }
    }
    unreachable!("past-end seek: the media never ended");
}

/// Percent without a known duration and chapter targets refuse typed
/// out-of-range — never a fake success (MP §8 bullet 4).
#[test]
fn unresolvable_targets_refuse_typed() {
    let (timeline, _) = build_timeline(false, 5, false, 0);
    let mut fixture = open(timeline.tracks.clone(), timeline.slots.clone(), true);
    prime_to_playing(&mut fixture, "unresolvable");
    let generation = fixture.engine.generation().must().0 + 1;
    fixture
        .handle
        .seek(
            commands::envelope::RequestId(1),
            SeekTarget::Percent { fraction: 0.5 },
        )
        .must();
    match drive_to_terminal(&mut fixture, generation, "percent-unknown") {
        EngineEventKind::SeekFailed { reason, .. } => {
            assert_eq!(reason, SeekFailure::OutOfRange, "percent needs a duration");
        }
        other => unreachable!("percent-unknown: {other:?}"),
    }
    let generation = fixture.engine.generation().must().0 + 1;
    fixture
        .handle
        .seek(
            commands::envelope::RequestId(2),
            SeekTarget::Chapter { index: 0 },
        )
        .must();
    match drive_to_terminal(&mut fixture, generation, "chapter") {
        EngineEventKind::SeekFailed { reason, .. } => {
            assert_eq!(reason, SeekFailure::OutOfRange, "chapters refuse typed");
        }
        other => unreachable!("chapter: {other:?}"),
    }
    // The session is still a functional playing session afterwards
    // (D4: the pipeline was never touched).
    for _ in 0..64 {
        fixture.engine.poll();
        drop(fixture.engine.take_outputs());
        if fixture.engine.state() == SessionState::Playing {
            return;
        }
    }
    unreachable!("unresolvable: playback never resumed");
}

/// An unseekable source refuses typed (`NotSeekable`) with the
/// session still playing at its old position — and playback keeps
/// flowing (the position is preserved, not fabricated).
#[test]
fn unseekable_source_refuses_typed_and_keeps_playing() {
    let (timeline, _) = build_timeline(false, 7, true, 1);
    let mut fixture = open(timeline.tracks.clone(), timeline.slots.clone(), false); // unseekable
    prime_to_playing(&mut fixture, "unseekable");
    let flowing_before = fixture.collector.len();
    assert!(flowing_before > 0, "the session was playing");
    let generation = fixture.engine.generation().must().0 + 1;
    fixture
        .handle
        .seek(
            commands::envelope::RequestId(1),
            SeekTarget::Absolute { position_ns: 0 },
        )
        .must();
    match drive_to_terminal(&mut fixture, generation, "unseekable") {
        EngineEventKind::SeekFailed { reason, position } => {
            assert_eq!(reason, SeekFailure::NotSeekable);
            assert_eq!(position, 0, "the old position is the truth");
        }
        other => unreachable!("unseekable: {other:?}"),
    }
    // Playback continues: more frames flow (position preserved).
    for _ in 0..16 {
        fixture.engine.poll();
        drop(fixture.engine.take_outputs());
    }
    assert!(
        fixture.collector.len() > flowing_before,
        "the unseekable session kept delivering after the refusal"
    );
}

// ---------------------------------------------------------------------------
// The supersession storm (MP §13 "repeated seeks, no waits between")
// ---------------------------------------------------------------------------

/// Submit a storm of position-independent seeks with no waits
/// between, then settle: no generation emits more than one terminal,
/// superseded generations stay silent, and the final accepted seek
/// completes with its landing boundary intact.
///
/// Attribution is receipt-driven: every applied seek answers exactly
/// one receipt, in submission order — `Accepted` carries the minted
/// generation (the mint IS the accept), `Rejected` (a seek from
/// `Ended`, once the media is over) means no mint and no terminal.
/// So the FIFO of submitted-but-unapplied seeks drains one receipt
/// at a time, and `accepted` is exactly the minted set.
#[test]
fn supersession_storm_is_silent_for_superseded_generations() {
    for seed in seeds() {
        let context = format!("seed {seed}");
        let mut rng = XorShift::new(seed);
        let (timeline, stride_ms) = generate_timeline(&mut rng);
        let seeks: Vec<SeekOp> = generate_seeks(&mut rng, 8, extent_of(stride_ms))
            .into_iter()
            .filter(|op| !op.position_dependent())
            .collect();
        let mut fixture = open(timeline.tracks.clone(), timeline.slots.clone(), true);
        prime_to_playing(&mut fixture, &context);
        let mut request = 0u64;
        let mut accepted: Vec<(u64, SeekOp)> = Vec::new();
        let mut terminals: Vec<(u64, EngineEventKind)> = Vec::new();
        let mut pending: std::collections::VecDeque<(u64, SeekOp)> =
            std::collections::VecDeque::new();
        for op in &seeks {
            request += 1;
            fixture
                .handle
                .seek(commands::envelope::RequestId(request), op.target())
                .must();
            pending.push_back((request, *op));
            // No waits between: 0..2 polls, then the next submit.
            for _ in 0..rng.below(3) {
                storm_poll(&mut fixture, &mut terminals, &mut pending, &mut accepted);
            }
        }
        // Settle: drain the inbox (late seeks apply — minted or
        // rejected), then the final accepted seek completes, bounded.
        let mut settled = false;
        for _ in 0..(TERMINAL_BUDGET + SETTLE_BUDGET) {
            storm_poll(&mut fixture, &mut terminals, &mut pending, &mut accepted);
            if let Some(&(final_generation, _)) = accepted.last()
                && pending.is_empty()
                && terminals
                    .iter()
                    .any(|(generation, _)| *generation == final_generation)
            {
                settled = true;
                break;
            }
        }
        let (final_generation, final_op) = *accepted.last().must();
        assert!(
            settled,
            "{context}: the final accepted seek (generation {final_generation}) never completed"
        );
        // The post-terminal window: a bounded slice of further polls
        // whose every delivery is governed by the final mint (no
        // further mints exist — the inbox is drained and the storm is
        // over). This is what gives Law 4 its teeth.
        let frames_at_settle = fixture.collector.len();
        for _ in 0..16 {
            fixture.engine.poll();
            drop(fixture.engine.take_outputs());
        }
        // Law 1: every terminal's generation was an accepted one.
        for (generation, _) in &terminals {
            assert!(
                accepted
                    .iter()
                    .any(|(accepted_generation, _)| accepted_generation == generation),
                "{context}: terminal for never-accepted generation {generation}"
            );
        }
        // Law 2: no generation emitted two terminals (doc 05 §4.3
        // invariant 4 — exactly one per completed generation).
        let mut seen = std::collections::HashSet::new();
        for (generation, _) in &terminals {
            assert!(
                seen.insert(*generation),
                "{context}: generation {generation} emitted two terminals"
            );
        }
        // Law 3: the final accepted seek completed.
        assert!(
            terminals
                .iter()
                .any(|(generation, _)| *generation == final_generation),
            "{context}: the final accepted seek (generation {final_generation}) never completed"
        );
        // Law 4: the final landing's boundary holds for everything
        // delivered in the post-terminal window (the last mint owns
        // the pipeline; anything older died at its flush).
        if let Expected::Completed { landing_ns, .. } = expect(final_op, &timeline, true, 0) {
            for delivered in fixture.collector.frames_from(frames_at_settle) {
                let Delivered {
                    generation, pts_ns, ..
                } = delivered;
                assert!(
                    generation >= final_generation,
                    "{context}: a generation-{generation} frame delivered after the final \
                     generation {final_generation}"
                );
                if let Some(pts) = pts_ns {
                    assert!(
                        pts >= landing_ns,
                        "{context}: a pre-landing frame ({pts} < {landing_ns}) escaped the \
                         final seek"
                    );
                }
            }
        }
        // Law 5: terminals never exceed accepted seeks (superseded
        // generations are silent — this is the weak form; the strong
        // per-generation silence is Law 2 + Law 3 together).
        assert!(
            terminals.len() <= accepted.len(),
            "{context}: {} terminals for {} accepted seeks",
            terminals.len(),
            accepted.len()
        );
    }
}

/// One storm poll: poll, drain the outbox through both collectors —
/// receipts attribute applied seeks (FIFO by request id, `Accepted`
/// carrying the mint), terminals record seek completions.
fn storm_poll(
    fixture: &mut Fixture,
    terminals: &mut Vec<(u64, EngineEventKind)>,
    pending: &mut std::collections::VecDeque<(u64, SeekOp)>,
    accepted: &mut Vec<(u64, SeekOp)>,
) {
    fixture.engine.poll();
    let outputs = fixture.engine.take_outputs();
    for output in &outputs {
        if let EngineOutput::Receipt(receipt) = output
            && pending
                .front()
                .is_some_and(|(request, _)| *request == receipt.request.0)
        {
            let (_, op) = pending.pop_front().must();
            if let ReceiptResult::Accepted { generation, .. } = receipt.result {
                accepted.push((generation.0, op));
            }
        }
    }
    collect_terminals(terminals, outputs);
}

fn collect_terminals(terminals: &mut Vec<(u64, EngineEventKind)>, outputs: Vec<EngineOutput>) {
    for output in outputs {
        if let EngineOutput::Event(EngineEvent {
            generation,
            kind: kind @ (EngineEventKind::Seeked { .. } | EngineEventKind::SeekFailed { .. }),
            ..
        }) = output
        {
            terminals.push((generation.0, kind.clone()));
        }
    }
}
