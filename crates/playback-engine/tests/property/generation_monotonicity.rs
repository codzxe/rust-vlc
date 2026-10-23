//! Family 3 — generation monotonicity (card 3.D3; doc 03 §8
//! "Generation races: the §5 guarantee as a machine-checked property
//! over random command storms").
//!
//! A seeded storm of pause/play/seek/track-switch/stop/close/reopen
//! cycles over a multi-track timeline, asserting the doc 03 §5
//! cancellation laws at every step:
//!
//! - **epochs strictly increase**: every `Engine::open` mints a
//!   session id one beyond the last (the reopen storm); a handle of
//!   a dead epoch refuses typed (`SessionError::SessionClosed`), it
//!   never reaches the dead session;
//! - **pipeline generations never rewind** within an epoch, and every
//!   mint is exactly the documented delta: `+1` per accepted
//!   gen-bumping command (seek / A-V track switch / stop),
//!   `close_plan(state).len()` per completed close (stop-then-close
//!   from active states), `0` for class-S subtitle selects, play and
//!   pause, and every typed rejection;
//! - **superseded pairs are stale forever**: the stage-held
//!   [`CancelToken`] answers `is_stale` for every pair captured
//!   before a mint, and accepts only the live one while the session
//!   lives — across epochs too (epoch N+1's pipeline 0 is not epoch
//!   N's pipeline 0);
//! - **stale packets can never carry a current generation** (the
//!   engine-level floor): within an epoch, every delivered frame's
//!   generation never exceeds the wire generation of the poll that
//!   delivered it, and per TRACK the delivered generation sequence
//!   never decreases — a flushed generation's stragglers drop at the
//!   limb's floor, they never deliver. (Per-track, not global: the
//!   engine's generation floor is limb-scoped by card 3.A3 — a track
//!   switch mint advances the demux stamp without invalidating the
//!   unchanged kinds' consumer paths, so their already-queued
//!   old-stamped residents remain presentable; a seek mint bumps
//!   every limb's floor.)

use std::collections::HashMap;
use std::sync::Arc;

use media_types::{MediaKind, TrackId};
use playback_engine::{
    CancelToken, CommandReceipt, Engine, EngineOutput, GenerationPair, ReceiptResult,
    SessionCommandKind, SessionError, SessionHandle, SessionState, TrackSelector, close_plan,
};

use super::engine_fx::{Built, Delivered, Slot, build, track};
use super::rng::{XorShift, run_case, seeds};

/// Milliseconds in nanoseconds.
const MS: i64 = 1_000_000;
/// Steps per track in the timeline.
const STEPS: usize = 320;
/// Reopen cycles per storm case (the epoch-minting budget).
const REOPENS: usize = 6;

/// The multi-track timeline: video (1) at even ms, two audio tracks
/// (2, 3) alternating at odd ms, one subtitle track (4) — the
/// switch-storm vocabulary. Audio packets carry `dts < pts` (the
/// B-lead shape); the engine must never reorder by or panic on it.
fn storm_timeline() -> (Vec<media_types::Track>, Vec<Slot>) {
    let tracks = vec![
        track(1, MediaKind::Video, Some(STEPS as i64 * 2 * MS)),
        track(2, MediaKind::Audio, Some(STEPS as i64 * 2 * MS)),
        track(3, MediaKind::Audio, Some(STEPS as i64 * 2 * MS)),
        track(4, MediaKind::Subtitle, Some(STEPS as i64 * 2 * MS)),
    ];
    let mut slots = Vec::with_capacity(STEPS * 2);
    for step in 0..STEPS {
        slots.push(Slot {
            track: TrackId::new(1),
            pts_ns: step as i64 * 2 * MS,
            dts_ns: None,
        });
        let audio = if step % 2 == 0 { 2 } else { 3 };
        slots.push(Slot {
            track: TrackId::new(audio),
            pts_ns: step as i64 * 2 * MS + MS,
            dts_ns: Some(step as i64 * 2 * MS),
        });
    }
    (tracks, slots)
}

/// One storm op.
#[derive(Debug, Clone, Copy)]
enum Op {
    Pause,
    Play,
    Seek { position_ns: i64 },
    SelectVideo,
    SelectAudio(u32),
    DeselectAudio,
    SelectSubtitle,
    Stop,
    Close,
}

fn generate_ops(rng: &mut XorShift, count: usize) -> Vec<Op> {
    (0..count)
        .map(|_| match rng.below(16) {
            0 => Op::Pause,
            1 => Op::Play,
            2..=4 => Op::Seek {
                position_ns: rng.below(STEPS as u64 * 2) as i64 * MS,
            },
            5 | 6 => Op::SelectVideo,
            7 => Op::SelectAudio(2),
            8 => Op::SelectAudio(3),
            9 => Op::DeselectAudio,
            10 => Op::SelectSubtitle,
            11 => Op::Stop,
            12 => Op::Close,
            _ => Op::Seek {
                position_ns: rng.below(STEPS as u64 * 2) as i64 * MS,
            },
        })
        .collect()
}

fn op_kind(op: Op) -> SessionCommandKind {
    match op {
        Op::Pause => SessionCommandKind::Pause,
        Op::Play => SessionCommandKind::Play,
        Op::Seek { .. } => SessionCommandKind::Seek,
        Op::SelectVideo => SessionCommandKind::SelectVideo,
        Op::SelectAudio(_) | Op::DeselectAudio => SessionCommandKind::SelectAudio,
        Op::SelectSubtitle => SessionCommandKind::SelectSubtitle,
        Op::Stop => SessionCommandKind::Stop,
        Op::Close => SessionCommandKind::Close,
    }
}

/// The live session's observation state.
struct Live {
    handle: SessionHandle,
    session_id: u64,
    token: CancelToken,
    live_pair: GenerationPair,
    /// The last generation delivered per TRACK — the engine's
    /// generation floor is limb-scoped (card 3.A3: a switch mint
    /// advances the demux stamp without invalidating the unchanged
    /// kinds' consumer paths — their old-stamped residents stay
    /// presentable), so monotonicity is a per-track law, not a
    /// global one. A seek mint bumps every limb's floor (the flush
    /// drops every queue's old-stamped residents), so within one
    /// track the delivered generation never decreases.
    delivered_by_track: HashMap<TrackId, u64>,
    collector: Arc<super::engine_fx::Collector>,
}

/// Open a fresh session on the engine and pin the epoch law.
fn open_session(
    engine: &mut Engine,
    tracks: &[media_types::Track],
    slots: &[Slot],
    expected_session: u64,
    previous: Option<&CancelToken>,
    context: &str,
) -> Result<Live, String> {
    let built: Built = build(tracks.to_vec(), slots.to_vec(), true);
    let handle = engine
        .open(built.config)
        .map_err(|error| format!("{context}: open refused {error:?}"))?;
    let session_id = engine
        .session_id()
        .ok_or_else(|| format!("{context}: open left no session"))?
        .0;
    if session_id != expected_session {
        return Err(format!(
            "{context}: minted session {session_id}, expected {expected_session} — epochs \
             must strictly increase"
        ));
    }
    let token = engine
        .cancel_token()
        .ok_or_else(|| format!("{context}: live session has no token"))?;
    let live_pair = token.snapshot();
    if let Some(previous_token) = previous {
        // Cross-epoch staleness: the old epoch's token refuses the new
        // epoch's live pair, equal pipeline counters included.
        if !previous_token.is_stale(&live_pair) {
            return Err(format!(
                "{context}: the previous epoch's token accepts epoch {session_id}'s live \
                 pair — cross-epoch is stale by construction"
            ));
        }
    }
    // The fresh pair is current (no mint, no interrupt yet).
    if !token.is_current(&live_pair) {
        return Err(format!(
            "{context}: the fresh pair is not current on its own token"
        ));
    }
    Ok(Live {
        handle,
        session_id,
        token,
        live_pair,
        delivered_by_track: HashMap::new(),
        collector: built.collector,
    })
}

fn run(seed: u64, ops: &[Op]) -> Result<(), String> {
    let canonical = ops.is_empty();
    let mut rng = XorShift::new(seed);
    let ops = if canonical {
        generate_ops(&mut rng, 56)
    } else {
        ops.to_vec()
    };
    let (tracks, slots) = storm_timeline();
    let mut engine = Engine::new();
    let mut request = 0u64;
    let mut next_session = 1u64;
    let mut reopens = 0usize;
    let mut live = open_session(
        &mut engine,
        &tracks,
        &slots,
        next_session,
        None,
        &format!("seed {seed}: first open"),
    )?;
    next_session += 1;
    // Prime to a seek-legal state before the storm.
    for _ in 0..32 {
        engine.poll();
        drop(engine.take_outputs());
        if engine.state() == SessionState::Playing {
            break;
        }
    }

    for (index, op) in ops.iter().enumerate() {
        let step = format!("seed {seed} op {index} ({op:?})");
        // Note: every close's aftermath below reopens immediately, so
        // the engine never sits Idle inside this loop.

        let state_before = engine.state();
        let generation_before = engine.generation().map_or(0, |g| g.0);
        let frame_count_before = live.collector.len();
        request += 1;
        let request_id = commands::envelope::RequestId(request);
        let kind = op_kind(*op);
        let submit = match *op {
            Op::Pause => live.handle.pause(request_id),
            Op::Play => live.handle.play(request_id),
            Op::Seek { position_ns } => live.handle.seek(
                request_id,
                playback_engine::SeekTarget::Absolute { position_ns },
            ),
            Op::SelectVideo => live.handle.set_video_track(
                request_id,
                TrackSelector::Id {
                    id: TrackId::new(1),
                },
            ),
            Op::SelectAudio(id) => live.handle.set_audio_track(
                request_id,
                TrackSelector::Id {
                    id: TrackId::new(id),
                },
            ),
            Op::DeselectAudio => live.handle.set_audio_track(request_id, TrackSelector::None),
            Op::SelectSubtitle => live.handle.set_subtitle_track(
                request_id,
                TrackSelector::Id {
                    id: TrackId::new(4),
                },
            ),
            Op::Stop => live.handle.stop(request_id),
            Op::Close => live.handle.close(request_id),
        };
        if let Err(error) = submit {
            return Err(format!(
                "{step}: live-session submit refused {error:?} — the inbox is drained every op"
            ));
        }
        engine.poll();
        let outputs = engine.take_outputs();
        let generation_after = engine.generation().map_or(0, |g| g.0);

        // ---- the receipt and its mint law ----
        let receipt: CommandReceipt = outputs
            .iter()
            .find_map(|output| match output {
                EngineOutput::Receipt(receipt) if receipt.request == request_id => Some(*receipt),
                _ => None,
            })
            .ok_or_else(|| format!("{step}: no receipt for request {request}"))?;
        if receipt.command != kind {
            return Err(format!(
                "{step}: receipt is for {:?}, submitted {kind:?}",
                receipt.command
            ));
        }
        match receipt.result {
            ReceiptResult::Accepted {
                session,
                generation,
            } => {
                if session.0 != live.session_id {
                    return Err(format!(
                        "{step}: receipt names session {}, live is {}",
                        session.0, live.session_id
                    ));
                }
                // Class A answers under the post-accept generation:
                // exactly pre + the documented delta.
                let delta = match kind {
                    SessionCommandKind::Seek
                    | SessionCommandKind::SelectVideo
                    | SessionCommandKind::SelectAudio
                    | SessionCommandKind::Stop => 1,
                    SessionCommandKind::Play | SessionCommandKind::Pause => 0,
                    other => {
                        return Err(format!(
                            "{step}: {other:?} must not answer class A — the close and the \
                             subtitle select are class S"
                        ));
                    }
                };
                if generation.0 != generation_before + delta {
                    return Err(format!(
                        "{step}: accepted at generation {} — pre {generation_before} + {delta} \
                         is the only legal mint",
                        generation.0
                    ));
                }
            }
            ReceiptResult::Completed { session } => {
                if session.0 != live.session_id {
                    return Err(format!(
                        "{step}: completion names session {}, live is {}",
                        session.0, live.session_id
                    ));
                }
                match kind {
                    // The close composition: one mint per table row it
                    // applied (stop-then-close from active states).
                    SessionCommandKind::Close => {
                        let delta = close_plan(state_before).len() as u64;
                        if generation_after != generation_before + delta {
                            return Err(format!(
                                "{step}: close moved the generation {generation_before} → \
                                 {generation_after}, expected +{delta} (state {state_before})"
                            ));
                        }
                    }
                    // Class S: no mint, ever.
                    SessionCommandKind::SelectSubtitle => {
                        if generation_after != generation_before {
                            return Err(format!(
                                "{step}: subtitle select minted {generation_before} → \
                                 {generation_after} — class S never mints"
                            ));
                        }
                    }
                    other => {
                        return Err(format!(
                            "{step}: {other:?} completed — only close and subtitle select do"
                        ));
                    }
                }
            }
            ReceiptResult::Rejected { .. } => {
                // A typed rejection mints nothing.
                if generation_after != generation_before {
                    return Err(format!(
                        "{step}: a rejection minted {generation_before} → {generation_after}"
                    ));
                }
            }
        }
        // The wire generation never rewinds within the epoch.
        if generation_after < generation_before {
            return Err(format!(
                "{step}: wire generation rewound {generation_before} → {generation_after}"
            ));
        }

        // ---- the token laws ----
        if generation_after != generation_before {
            // The pair captured before the mint is stale forever.
            if !live.token.is_stale(&live.live_pair) {
                return Err(format!(
                    "{step}: the pre-mint pair is still current after the mint — a superseded \
                     generation can never come back"
                ));
            }
            live.live_pair = live.token.snapshot();
        }
        if kind != SessionCommandKind::Close {
            // While the session lives, the live pair is current.
            if !live.token.is_current(&live.live_pair) {
                return Err(format!(
                    "{step}: the live pair is not current on the live token"
                ));
            }
        }

        // ---- the delivered-frame generation floor ----
        for Delivered {
            track,
            generation,
            pts_ns,
        } in live.collector.frames_from(frame_count_before)
        {
            if generation > generation_after {
                return Err(format!(
                    "{step}: frame of generation {generation} delivered under wire \
                     generation {generation_after} (pts {pts_ns:?})"
                ));
            }
            // Per-track monotonicity (the limb-scoped floor of card
            // 3.A3 — see `Live::delivered_by_track`): within one
            // track's delivery stream the generation never rewinds.
            let floor = live.delivered_by_track.get(&track).copied().unwrap_or(0);
            if generation < floor {
                return Err(format!(
                    "{step}: track {track:?} delivered generation rewound {floor} → \
                     {generation} (pts {pts_ns:?}) — a flushed generation's stragglers must \
                     drop at the limb's floor, never deliver"
                ));
            }
            live.delivered_by_track.insert(track, generation);
        }

        // ---- the close aftermath: the stale handle refuses typed ----
        if kind == SessionCommandKind::Close {
            engine.poll();
            drop(engine.take_outputs());
            if engine.state() != SessionState::Idle {
                return Err(format!(
                    "{step}: state {} after the close settled — close lands in Idle",
                    engine.state()
                ));
            }
            request += 1;
            match live.handle.play(commands::envelope::RequestId(request)) {
                Err(SessionError::SessionClosed) => {}
                other => {
                    return Err(format!(
                        "{step}: a dead epoch's handle answered {other:?} — it must refuse \
                         typed SessionClosed"
                    ));
                }
            }
            // The dead token refuses everything (the interrupt).
            if !live.token.is_stale(&live.live_pair) {
                return Err(format!(
                    "{step}: the interrupted token still accepts its last pair"
                ));
            }
            reopens += 1;
            if reopens > REOPENS {
                break;
            }
            let previous_token = live.token.clone();
            live = open_session(
                &mut engine,
                &tracks,
                &slots,
                next_session,
                Some(&previous_token),
                &step,
            )?;
            next_session += 1;
            for _ in 0..32 {
                engine.poll();
                drop(engine.take_outputs());
                if engine.state() == SessionState::Playing {
                    break;
                }
            }
        }
    }
    Ok(())
}

/// The storm law set over the full bank: 64 seeds × 56 ops of
/// pause/play/seek/switch/stop/close interleavings with up to 7
/// sessions per case — epoch strictness, exact mint deltas, token
/// staleness, stale-handle refusals, and the delivered-generation
/// floor, checked after every op.
#[test]
fn generations_strictly_increase_and_supersede_forever_under_storms() {
    for seed in seeds() {
        let mut rng = XorShift::new(seed);
        let ops = generate_ops(&mut rng, 56);
        if let Err(failure) = run_case(seed, &ops, run) {
            unreachable!("{failure}");
        }
    }
}
