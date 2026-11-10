//! The pipeline runner (card 7.E1): [`submit`] turns a validated
//! [`TranscodeGraph`] into two threads and a [`JobHandle`] — the
//! pump (source/demux → bounded queue) and the worker (queue →
//! decode → WAV encode, or queue → verbatim copy sink).
//!
//! # Thread and hand-off topology (doc 01 §4.1's transcode row)
//!
//! ```text
//! host                    pump thread                 worker thread
//! ----                    -----------                 -------------
//! submit(graph) ──► start ─┐
//!                         │  demuxer.start() → track selection
//!                         │      └─ TrackHandoff ──────────► decoder open/configure
//!                         │  next_event() loop                 pop_wait() loop
//! cancel() ──► token ─┼─► push_wait() ── bounded queue ─► decode/copy → sink
//! wait() ◄────────────┴──────── joins both ──────────► finalize + terminal
//! ```
//!
//! # The stop protocol (why no deadlock and no lost terminal)
//!
//! * **Teardown wakes with [`close`](PacketStageQueue::close), not
//!   the token.** Every thread's exit layer closes the queue (both
//!   condvars notified, no flag flipped), so the peer wakes without
//!   mistaking teardown for a user cancel.
//! * **User cancel fires the token** (plus
//!   [`interrupt`](PacketStageQueue::interrupt), which flips it and
//!   wakes every waiter): both threads see `is_stopping`, and the
//!   first to reach [`terminate`](JobShared::terminate) wins the
//!   outcome cell — a stage that failed a microsecond earlier
//!   already put `Failed` there and the cancel path defers to it.
//! * **Panics are stage faults, not job hangs**: each thread's body
//!   runs under `catch_unwind`; a host stage that panics becomes the
//!   typed `transcode.job.worker-panic` failure and the exit layer
//!   still closes the queue (the peer can never park forever on a
//!   dead stage).
//! * **First-set-wins terminals** ([`OnceLock`] semantics): exactly
//!   one of `Completed` / `Failed` / `Cancelled` lands, and
//!   [`JobHandle::wait`] joins both threads before answering, so the
//!   sink's finalize has happened when the host sees the terminal.
//!
//! # Cancellation promptness (the honest bound)
//!
//! The token is checked before each demux event, between packets,
//! and inside every bounded queue park — so a stop is honored within
//! one stage call. A demuxer or sink blocked *inside* a call is the
//! host stage's own contract violation (the codec-api/plugin-api
//! no-indefinite-wait rules); the job never yanks work out from
//! under a stage.
//!
//! # Bounded resources
//!
//! The stage queue carries [`JobLimits`] (slots and bytes, the
//! streaming stack's two-axis doctrine); the event hub drops and
//! counts; the WAV writer streams through with no payload buffer;
//! the track table is budget-checked right after `start`. The
//! `[PROPOSAL]` ES-sink seam for the later 7.E3 encoder card: the
//! raw ES sink is a verbatim byte dump today — when codec encoders
//! land, the decode stage's output feeds an `Encoder` and the ES
//! sink becomes its packet drain, the same factory shape
//! [`DecoderFactory`] already has.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

use codec_api::{DecodeOutcome, DecoderConfig, FrameSink, ThreadBudget};
use media_types::frame::AudioFrame;
use media_types::ids::MediaKind;
use media_types::packet::SharedBytes;
use media_types::{EngineError, Frame, GenerationId, Track};
use plugin_api::{Demuxer, DemuxEvent};

use crate::cancel::CancelToken;
use crate::convert::{convert_to_wav, identity_passthrough, resolve_target};
use crate::error::{
    audio_track_ambiguous, audio_track_missing, copy_byte_order, copy_facts_mismatch,
    copy_not_pcm, flush_undrained, sink_bad_state, sink_write_io, tracks_over_budget,
    worker_panic, worker_spawn_io,
};
use crate::graph::{
    DecodeStage, PcmFacts, SinkSpec, TrackSelection, TranscodeGraph, WavPcmTarget, WriteSeek,
};
use crate::job::{JobOutcome, JobShared, JobStats, JobHandle};
use crate::limits::JobLimits;
use crate::queue::{PacketStageQueue, PopWait, PushWait};
use crate::wav::WavPcmWriter;

/// Progress cadence `[PROPOSAL: 64 KiB of sink payload between
/// progress events — matches the demux layer's packet ceiling, so
/// one event per handful of packets; a terminal always emits one
/// final snapshot regardless]`.
pub(crate) const PROGRESS_STEP_BYTES: u64 = 64 * 1024;

/// How many non-`Drained` flush calls the drain loop tolerates before
/// the typed `transcode.decode.flush-undrained` refusal `[PROPOSAL:
/// 1024 — PCM drains in one call; the budget only exists so a
/// misbehaving host decoder cannot spin the worker forever]`.
pub(crate) const MAX_FLUSH_CALLS: u32 = 1024;

/// The little-endian PCM descriptor family the WAV stream-copy path
/// can transcribe verbatim (the demux layer's own vocabulary —
/// doc 04 §3.1).
const PCM_LE_DESCRIPTORS: [&str; 6] = [
    "pcm_u8", "pcm_s16le", "pcm_s24le", "pcm_s32le", "pcm_f32le", "pcm_f64le",
];
/// The big-endian twin family: byte-swapped payloads a verbatim copy
/// would lie about (the decode path normalizes them instead).
const PCM_BE_DESCRIPTORS: [&str; 5] = [
    "pcm_s16be", "pcm_s24be", "pcm_s32be", "pcm_f32be", "pcm_f64be",
];

/// Submit a job: validate the graph's shape, spawn the pipeline,
/// hand back the [`JobHandle`].
///
/// This is synchronous and does no container I/O — the demuxer's
/// `start` runs on the pump thread, so a slow source never blocks
/// the submitting host (the `queued → running` transition is the
/// pump's first action).
///
/// # Errors
/// [`TranscodeGraph::validate`]'s typed refusals (filter rows, sink
/// shape, missing copy facts), or
/// [`worker_spawn_io`](crate::error::worker_spawn_io) if the
/// process refuses a thread (the already-spawned peer is torn down
/// before the error returns).
pub fn submit(graph: TranscodeGraph, limits: JobLimits) -> Result<JobHandle, EngineError> {
    graph.validate(&limits)?;
    let cancel = CancelToken::new();
    let queue = Arc::new(PacketStageQueue::new(
        limits.queue_slots(),
        limits.queue_bytes(),
        cancel.clone(),
    ));
    let shared = Arc::new(JobShared::new(limits.event_slots(), cancel.clone()));
    let handoff = Arc::new(TrackHandoff::new(cancel.clone()));

    let require_audio = matches!(
        graph.sink,
        SinkSpec::WavPcm { .. } | SinkSpec::WavCopyPcm { .. }
    );
    let TranscodeGraph {
        source,
        decode,
        filters: _,
        sink,
    } = graph;
    let plan = build_plan(sink, decode, source.extra_data)?;

    let pump_ctx = PumpCtx {
        demuxer: source.demuxer,
        selection: source.selection,
        require_audio,
        limits,
        queue: Arc::clone(&queue),
        handoff: Arc::clone(&handoff),
        shared: Arc::clone(&shared),
    };
    let worker_ctx = WorkerCtx {
        plan,
        queue: Arc::clone(&queue),
        handoff: Arc::clone(&handoff),
        shared: Arc::clone(&shared),
    };

    let pump_shared = Arc::clone(&shared);
    let pump_queue = Arc::clone(&queue);
    let pump = std::thread::Builder::new()
        .name("transcode-pump".to_string())
        .spawn(move || {
            let result = catch_unwind(AssertUnwindSafe(move || pump_body(pump_ctx)));
            if result.is_err() {
                pump_shared.terminate(JobOutcome::Failed(worker_panic()));
            }
            pump_queue.close();
        })
        .map_err(|err| {
            // Nothing is running yet: refuse typed, no teardown needed.
            worker_spawn_io(err)
        })?;

    let worker_shared = Arc::clone(&shared);
    let worker_queue = Arc::clone(&queue);
    let worker = match std::thread::Builder::new()
        .name("transcode-worker".to_string())
        .spawn(move || {
            let result = catch_unwind(AssertUnwindSafe(move || worker_body(worker_ctx)));
            if result.is_err() {
                worker_shared.terminate(JobOutcome::Failed(worker_panic()));
            }
            worker_queue.close();
        }) {
        Ok(handle) => handle,
        Err(err) => {
            // The pump IS running: stop it before refusing.
            queue.interrupt();
            let _ = pump.join();
            return Err(worker_spawn_io(err));
        }
    };

    Ok(JobHandle::new(shared, cancel, PipelineThreads { pump, worker, queue }))
}

/// Both pipeline threads plus the queue they share (the handle's
/// join/interrupt surface).
pub(crate) struct PipelineThreads {
    pump: JoinHandle<()>,
    worker: JoinHandle<()>,
    queue: Arc<PacketStageQueue>,
}

impl PipelineThreads {
    /// Wake every parked waiter (user cancel: token + notify).
    pub(crate) fn interrupt(&self) {
        self.queue.interrupt();
    }

    /// Join both threads; whether any panicked (mapped by the caller
    /// to the typed worker-panic failure). Pump first: it is the
    /// producer, and its exit layer closes the queue, which releases
    /// a parked worker.
    pub(crate) fn join(self) -> bool {
        let pump_panicked = self.pump.join().is_err();
        let worker_panicked = self.worker.join().is_err();
        pump_panicked || worker_panicked
    }
}

/// What the worker was handed (the graph's sink half, destructured
/// into its three runnable shapes).
enum WorkerPlan {
    /// Decode path: factory-built decoder, WAV encode sink.
    Decode {
        factory: Box<dyn crate::graph::DecoderFactory>,
        extra_data: Option<SharedBytes>,
        target: WavPcmTarget,
        output: Box<dyn WriteSeek>,
    },
    /// Copy path: unpacked-PCM packets → WAV container, verbatim.
    CopyWav {
        facts: PcmFacts,
        output: Box<dyn WriteSeek>,
    },
    /// Copy path: any track's packets → raw elementary-stream file.
    RawEs {
        output: Box<dyn WriteSeek>,
    },
}

/// Assemble the worker plan from the validated graph's sink half.
/// Validation has already pinned the shape; the impossible arms are
/// the typed impossible-state answer, not silent fallbacks.
fn build_plan(
    sink: SinkSpec,
    decode: Option<DecodeStage>,
    extra_data: Option<SharedBytes>,
) -> Result<WorkerPlan, EngineError> {
    match (sink, decode) {
        (
            SinkSpec::WavPcm { output, target },
            Some(stage),
        ) => Ok(WorkerPlan::Decode {
            factory: stage.into_factory(),
            extra_data,
            target,
            output,
        }),
        (SinkSpec::WavCopyPcm { output, facts }, None) => match facts {
            Some(facts) => Ok(WorkerPlan::CopyWav { facts, output }),
            // Validation refused this shape already (facts missing).
            None => Err(sink_bad_state()),
        },
        (SinkSpec::RawEsCopy { output }, None) => Ok(WorkerPlan::RawEs { output }),
        // Copy sink with a decode stage, or WavPcm without one:
        // `validate` refuses both before we get here.
        _ => Err(sink_bad_state()),
    }
}

/// The pump thread's owned context.
struct PumpCtx {
    demuxer: Box<dyn Demuxer>,
    selection: TrackSelection,
    require_audio: bool,
    limits: JobLimits,
    queue: Arc<PacketStageQueue>,
    handoff: Arc<TrackHandoff>,
    shared: Arc<JobShared>,
}

/// The pump: start the source, select the track, feed the queue.
fn pump_body(ctx: PumpCtx) {
    let PumpCtx {
        mut demuxer,
        selection,
        require_audio,
        limits,
        queue,
        handoff,
        shared,
    } = ctx;
    shared.set_running();

    let tracks = match demuxer.start() {
        Ok(tracks) => tracks,
        Err(err) => return terminate_with(&shared, &queue, JobOutcome::Failed(err)),
    };
    if tracks.len() > limits.max_tracks() {
        return terminate_with(
            &shared,
            &queue,
            JobOutcome::Failed(tracks_over_budget(tracks.len(), limits.max_tracks())),
        );
    }
    let track = match select_track(&tracks, selection, require_audio) {
        Ok(track) => track,
        Err(err) => return terminate_with(&shared, &queue, JobOutcome::Failed(err)),
    };
    handoff.deliver(track.clone());

    loop {
        if shared.is_stopping() {
            return stop_or_cancel(&shared, &queue);
        }
        match demuxer.next_event() {
            Err(err) => return terminate_with(&shared, &queue, JobOutcome::Failed(err)),
            Ok(DemuxEvent::Packet(packet)) => {
                if packet.track != track.id {
                    shared.update_stats(&JobStats {
                        packets_skipped: 1,
                        ..JobStats::default()
                    });
                    continue;
                }
                let len = packet.data.len() as u64;
                match queue.push_wait(packet) {
                    PushWait::Pushed => shared.update_stats(&JobStats {
                        packets_read: 1,
                        bytes_read: len,
                        ..JobStats::default()
                    }),
                    // Only this pump closes for teardown after a
                    // terminal; the worker never closes first.
                    PushWait::Closed => return stop_or_cancel(&shared, &queue),
                    PushWait::Interrupted => return stop_or_cancel(&shared, &queue),
                    PushWait::TooLarge => {
                        let err =
                            PacketStageQueue::too_large_error(len as usize, limits.queue_bytes());
                        return terminate_with(&shared, &queue, JobOutcome::Failed(err));
                    }
                }
            }
            Ok(DemuxEvent::Recovered { skipped, .. }) => {
                shared.update_stats(&JobStats {
                    recovered: skipped,
                    ..JobStats::default()
                });
            }
            Ok(DemuxEvent::Discontinuity) => {
                shared.update_stats(&JobStats {
                    discontinuities: 1,
                    ..JobStats::default()
                });
            }
            Ok(DemuxEvent::EndOfStream) => {
                // The worker drains the queue, flushes, finalizes and
                // completes; teardown-wake rides the exit layer.
                queue.close();
                return;
            }
        }
    }
}

/// The worker thread's owned context.
struct WorkerCtx {
    plan: WorkerPlan,
    queue: Arc<PacketStageQueue>,
    handoff: Arc<TrackHandoff>,
    shared: Arc<JobShared>,
}

/// The worker: take the selected track, run the plan's sink loop.
fn worker_body(ctx: WorkerCtx) {
    let WorkerCtx {
        plan,
        queue,
        handoff,
        shared,
    } = ctx;
    let track = match handoff.wait() {
        Some(track) => track,
        // The pump stopped before selecting (failure or cancel):
        // the terminal already says which; only a bare cancel needs
        // an outcome minted here.
        None => return stop_or_cancel(&shared, &queue),
    };
    match plan {
        WorkerPlan::Decode {
            factory,
            extra_data,
            target,
            output,
        } => run_decode(factory, extra_data, target, output, &track, &queue, &shared),
        WorkerPlan::CopyWav { facts, output } => run_copy_wav(facts, output, &track, &queue, &shared),
        WorkerPlan::RawEs { output } => run_raw_es(output, &queue, &shared),
    }
}

/// The decode path: demuxed packets → host decoder → WAV encode.
fn run_decode(
    factory: Box<dyn crate::graph::DecoderFactory>,
    extra_data: Option<SharedBytes>,
    target: WavPcmTarget,
    output: Box<dyn WriteSeek>,
    track: &Track,
    queue: &PacketStageQueue,
    shared: &Arc<JobShared>,
) {
    let mut decoder = match factory.open(track) {
        Ok(decoder) => decoder,
        Err(err) => return terminate_with(shared, queue, JobOutcome::Failed(err)),
    };
    // One job = one demux generation: configured at FIRST, no
    // packet is ever stale (a transcode never re-anchors; the stop
    // surface is the token, not generations).
    let config = DecoderConfig::new(
        track.codec.clone(),
        GenerationId::FIRST,
        ThreadBudget::default(),
        None,
        extra_data,
    );
    if let Err(err) = decoder.configure(&config) {
        return terminate_with(shared, queue, JobOutcome::Failed(err));
    }
    let mut sink = WavEncodeSink::new(output, target, Arc::clone(shared));

    loop {
        if shared.is_stopping() {
            if shared.outcome().is_none() {
                // Cancellation: finalize best-effort so the partial
                // file's sizes tell the truth (crate::cancel docs).
                let _ = sink.finish();
                return terminate_with(
                    shared,
                    queue,
                    JobOutcome::Cancelled(shared.snapshot()),
                );
            }
            return;
        }
        match queue.pop_wait() {
            PopWait::Item(packet) => {
                match decoder.decode(packet, &mut sink) {
                    Err(err) => return terminate_with(shared, queue, JobOutcome::Failed(err)),
                    Ok(DecodeOutcome::Emitted { frames }) => shared.update_stats(&JobStats {
                        frames_decoded: u64::from(frames),
                        ..JobStats::default()
                    }),
                    Ok(DecodeOutcome::Recovered { .. }) => shared.update_stats(&JobStats {
                        recovered: 1,
                        ..JobStats::default()
                    }),
                    Ok(DecodeOutcome::NeedMoreInput | DecodeOutcome::Drained) => {}
                }
            }
            PopWait::Closed => {
                // A terminal already landed ⇒ the pump tore down
                // mid-stream: exit without claiming output validity.
                if shared.outcome().is_some() {
                    return;
                }
                // True end of stream: drain the decoder (bounded).
                let mut calls = 0u32;
                loop {
                    match decoder.flush(&mut sink) {
                        Err(err) => {
                            return terminate_with(shared, queue, JobOutcome::Failed(err))
                        }
                        Ok(DecodeOutcome::Drained) => break,
                        Ok(_) => {
                            calls += 1;
                            if calls >= MAX_FLUSH_CALLS {
                                return terminate_with(
                                    shared,
                                    queue,
                                    JobOutcome::Failed(flush_undrained()),
                                );
                            }
                        }
                    }
                }
                if let Err(err) = sink.finish() {
                    return terminate_with(shared, queue, JobOutcome::Failed(err));
                }
                return terminate_with(shared, queue, JobOutcome::Completed(shared.snapshot()));
            }
            PopWait::Interrupted => {
                if shared.outcome().is_none() {
                    let _ = sink.finish();
                    return terminate_with(
                        shared,
                        queue,
                        JobOutcome::Cancelled(shared.snapshot()),
                    );
                }
                return;
            }
        }
    }
}

/// The WAV stream-copy path: PCM packets → container, verbatim.
fn run_copy_wav(
    facts: PcmFacts,
    output: Box<dyn WriteSeek>,
    track: &Track,
    queue: &PacketStageQueue,
    shared: &JobShared,
) {
    if let Err(err) = check_copy_legality(track.codec.as_str(), &facts) {
        return terminate_with(shared, queue, JobOutcome::Failed(err));
    }
    let mut writer = match WavPcmWriter::new(output, facts.sample, facts.channels, facts.rate) {
        Ok(writer) => writer,
        Err(err) => return terminate_with(shared, queue, JobOutcome::Failed(err)),
    };
    let frame_bytes = u64::from(u32::from(facts.channels) * u32::from(facts.sample.bytes_per_sample()));
    let mut watermark = 0u64;
    loop {
        if shared.is_stopping() {
            if shared.outcome().is_none() {
                let _ = writer.finalize();
                return terminate_with(
                    shared,
                    queue,
                    JobOutcome::Cancelled(shared.snapshot()),
                );
            }
            return;
        }
        match queue.pop_wait() {
            PopWait::Item(packet) => {
                let len = packet.data.len() as u64;
                match writer.write_pcm(packet.data.as_slice()) {
                    Ok(written) => {
                        let frames = len / frame_bytes;
                        shared.update_stats(&JobStats {
                            sample_frames_written: frames,
                            bytes_written: written,
                            ..JobStats::default()
                        });
                        emit_progress_cadence(shared, &mut watermark);
                    }
                    Err(err) => return terminate_with(shared, queue, JobOutcome::Failed(err)),
                }
            }
            PopWait::Closed => {
                if shared.outcome().is_some() {
                    return;
                }
                if let Err(err) = writer.finalize() {
                    return terminate_with(shared, queue, JobOutcome::Failed(err));
                }
                return terminate_with(shared, queue, JobOutcome::Completed(shared.snapshot()));
            }
            PopWait::Interrupted => {
                if shared.outcome().is_none() {
                    let _ = writer.finalize();
                    return terminate_with(
                        shared,
                        queue,
                        JobOutcome::Cancelled(shared.snapshot()),
                    );
                }
                return;
            }
        }
    }
}

/// The raw ES path: any track's packets, payload bytes concatenated
/// (the 7.E3 encoder seam's drain — module docs' [PROPOSAL]).
fn run_raw_es(
    mut output: Box<dyn WriteSeek>,
    queue: &PacketStageQueue,
    shared: &JobShared,
) {
    let mut watermark = 0u64;
    loop {
        if shared.is_stopping() {
            if shared.outcome().is_none() {
                let _ = output.flush();
                return terminate_with(
                    shared,
                    queue,
                    JobOutcome::Cancelled(shared.snapshot()),
                );
            }
            return;
        }
        match queue.pop_wait() {
            PopWait::Item(packet) => {
                let len = packet.data.len() as u64;
                if let Err(err) = std::io::Write::write_all(&mut output, packet.data.as_slice()) {
                    return terminate_with(shared, queue, JobOutcome::Failed(sink_write_io(err)));
                }
                // The ES sink makes no sample claim: payload bytes are
                // not sample frames until a decoder says so.
                shared.update_stats(&JobStats {
                    bytes_written: len,
                    ..JobStats::default()
                });
                emit_progress_cadence(shared, &mut watermark);
            }
            PopWait::Closed => {
                if shared.outcome().is_some() {
                    return;
                }
                match output.flush() {
                    Ok(()) => {
                        return terminate_with(
                            shared,
                            queue,
                            JobOutcome::Completed(shared.snapshot()),
                        )
                    }
                    Err(err) => {
                        return terminate_with(
                            shared,
                            queue,
                            JobOutcome::Failed(sink_write_io(err)),
                        )
                    }
                }
            }
            PopWait::Interrupted => {
                if shared.outcome().is_none() {
                    let _ = output.flush();
                    return terminate_with(
                        shared,
                        queue,
                        JobOutcome::Cancelled(shared.snapshot()),
                    );
                }
                return;
            }
        }
    }
}

/// The decode path's frame sink: converts/writes each decoded audio
/// frame, lazily building the WAV writer from the first frame's
/// format (channels/rate live in frames, not the L0 track).
struct WavEncodeSink {
    output: Option<Box<dyn WriteSeek>>,
    writer: Option<WavPcmWriter>,
    target: WavPcmTarget,
    /// The resolved output sample kind (first frame decides).
    resolved: Option<demux::wav::format::PcmSample>,
    /// The pinned format triple (sample, channels, rate): a change
    /// mid-stream is the documented v0 refusal.
    fixed: Option<(media_types::frame::SampleFormat, u32, u32)>,
    /// Reused conversion scratch (bounded by one frame, D-1).
    scratch: Vec<u8>,
    shared: Arc<JobShared>,
    watermark: u64,
}

impl WavEncodeSink {
    fn new(output: Box<dyn WriteSeek>, target: WavPcmTarget, shared: Arc<JobShared>) -> WavEncodeSink {
        WavEncodeSink {
            output: Some(output),
            writer: None,
            target,
            resolved: None,
            fixed: None,
            scratch: Vec::new(),
            shared,
            watermark: 0,
        }
    }

    /// Finalize the writer (patch sizes). A source that decoded zero
    /// frames leaves the output EMPTY — no header facts were ever
    /// observed, and fabricating a 44-byte header would invent
    /// channels and a rate (the copy path, whose facts the host
    /// supplied, writes the honest empty header instead).
    fn finish(mut self) -> Result<(), EngineError> {
        match self.writer.take() {
            Some(writer) => writer.finalize().map(|_| ()),
            None => Ok(()),
        }
    }
}

impl FrameSink for WavEncodeSink {
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
        let audio = match frame {
            Frame::Audio(audio) => audio,
            // The decode path transcribes audio; anything else is a
            // typed refusal, never a silent drop.
            _ => return Err(crate::error::frame_not_audio()),
        };
        let AudioFrame { format, samples, .. } = audio;
        let key = (format.sample, format.channels, format.rate);
        let target_sample = match (self.resolved, self.fixed) {
            (None, None) => {
                // First frame: resolve the output kind (identity or a
                // legal widening — the honest conversion matrix).
                let resolved = resolve_target(format.sample, &self.target)?;
                let output = match self.output.take() {
                    Some(output) => output,
                    None => return Err(sink_bad_state()),
                };
                let writer =
                    WavPcmWriter::new(output, resolved, format.channels as u16, format.rate)?;
                self.writer = Some(writer);
                self.fixed = Some(key);
                self.resolved = Some(resolved);
                resolved
            }
            (Some(resolved), Some(fixed)) => {
                if fixed != key {
                    // Sample kind, channels or rate changed mid-stream.
                    return Err(crate::error::wav_format_change());
                }
                resolved
            }
            // Writer built without a resolved kind (or vice versa):
            // unreachable by construction.
            _ => return Err(sink_bad_state()),
        };
        // Whole-frame discipline (the passthrough path re-checks what
        // the conversion path guarantees by construction).
        let frames = match format.frames_in(samples.len()) {
            Some(frames) => frames,
            None => return Err(crate::error::wav_partial_frame()),
        };
        let writer = match self.writer.as_mut() {
            Some(writer) => writer,
            None => return Err(sink_bad_state()),
        };
        let written = if identity_passthrough(&format, target_sample) {
            writer.write_pcm(samples.as_slice())?
        } else {
            convert_to_wav(samples.as_slice(), &format, target_sample, &mut self.scratch);
            let scratch = std::mem::take(&mut self.scratch);
            let result = writer.write_pcm(scratch.as_slice());
            self.scratch = scratch;
            result?
        };
        self.shared.update_stats(&JobStats {
            sample_frames_written: frames,
            bytes_written: written,
            ..JobStats::default()
        });
        emit_progress_cadence(&self.shared, &mut self.watermark);
        Ok(())
    }
}

/// The one-track hand-off between the pump (which owns the demuxer)
/// and the worker (which needs the track to open the decoder and
/// cross-check copy facts). One slot, condvar-parked, cancel-aware.
struct TrackHandoff {
    state: Mutex<HandoffState>,
    ready: Condvar,
    cancel: CancelToken,
}

struct HandoffState {
    track: Option<Track>,
    closed: bool,
}

impl TrackHandoff {
    fn new(cancel: CancelToken) -> TrackHandoff {
        TrackHandoff {
            state: Mutex::new(HandoffState {
                track: None,
                closed: false,
            }),
            ready: Condvar::new(),
            cancel,
        }
    }

    /// Deliver the selected track (the pump, once).
    fn deliver(&self, track: Track) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        state.track = Some(track);
        drop(state);
        self.ready.notify_all();
    }

    /// Park until the track arrives, the pump closes without one, or
    /// cancellation fires (the caller resolves which).
    fn wait(&self) -> Option<Track> {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        loop {
            if let Some(track) = state.track.take() {
                return Some(track);
            }
            if state.closed || self.cancel.is_cancelled() {
                return None;
            }
            state = self
                .ready
                .wait(state)
                .unwrap_or_else(|p| p.into_inner());
        }
    }
}

/// Select the track (pure; the typed selection vocabulary).
fn select_track(
    tracks: &[Track],
    selection: TrackSelection,
    require_audio: bool,
) -> Result<Track, EngineError> {
    match selection {
        TrackSelection::DefaultAudio => {
            let audio: Vec<&Track> = tracks
                .iter()
                .filter(|track| track.kind == MediaKind::Audio)
                .collect();
            match audio.len() {
                0 => Err(audio_track_missing()),
                1 => Ok(audio[0].clone()),
                found => Err(audio_track_ambiguous(found)),
            }
        }
        TrackSelection::Track(id) => {
            let track = match tracks.iter().find(|track| track.id == id) {
                Some(track) => track,
                None => return Err(crate::error::track_not_found()),
            };
            if require_audio && track.kind != MediaKind::Audio {
                return Err(crate::error::track_not_audio());
            }
            Ok(track.clone())
        }
    }
}

/// The WAV stream-copy legality check (pure): the track's codec
/// descriptor must be the LE-PCM family, matching the host-supplied
/// facts exactly — a big-endian source needs the decode path, a
/// non-PCM source is not a stream-copy at all.
fn check_copy_legality(descriptor: &str, facts: &PcmFacts) -> Result<(), EngineError> {
    if PCM_LE_DESCRIPTORS.contains(&descriptor) {
        if descriptor == facts.sample.descriptor() {
            Ok(())
        } else {
            Err(copy_facts_mismatch(descriptor, facts.sample.descriptor()))
        }
    } else if PCM_BE_DESCRIPTORS.contains(&descriptor) {
        Err(copy_byte_order(descriptor))
    } else {
        Err(copy_not_pcm(descriptor))
    }
}

/// Snapshot the queue's high-water marks into the stats, then land
/// the terminal (first-set-wins inside).
fn terminate_with(shared: &JobShared, queue: &PacketStageQueue, outcome: JobOutcome) {
    let stats = queue.stats();
    shared.observe_queue(stats.peak_slots, stats.peak_bytes);
    shared.terminate(outcome);
}

/// The stop-path terminal: if nothing terminal landed yet this was a
/// user cancel (every other stop sets its outcome first).
fn stop_or_cancel(shared: &JobShared, queue: &PacketStageQueue) {
    if shared.outcome().is_none() {
        terminate_with(shared, queue, JobOutcome::Cancelled(shared.snapshot()));
    }
}

/// Progress cadence (module docs' constant): one event per
/// [`PROGRESS_STEP_BYTES`] of sink payload, never blocking.
fn emit_progress_cadence(shared: &JobShared, watermark: &mut u64) {
    let current = shared.snapshot().bytes_written;
    if current.saturating_sub(*watermark) >= PROGRESS_STEP_BYTES {
        *watermark = current;
        shared.emit_progress();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::limit_over_budget;
    use media_types::ids::{StreamId, TrackFlags, TrackId};
    use media_types::time::{TimeSpan, TrackTiming};
    use media_types::EngineError;
    use testkit::{Must, assert_err};

    fn track(id: u32, kind: MediaKind, codec: &str) -> Track {
        Track {
            id: TrackId::new(id),
            stream: StreamId::new(0),
            kind,
            language: None,
            codec: media_types::ids::CodecDescriptor::new(codec).must(),
            title: None,
            flags: TrackFlags::NONE,
            timing: TrackTiming {
                span: TimeSpan::UNKNOWN,
                base: media_types::time::TimeBase::new(1, 1000).must(),
                gaps: Vec::new(),
            },
        }
    }

    #[test]
    fn default_audio_selection_is_exactly_one() {
        let audio = track(0, MediaKind::Audio, "pcm_s16le");
        let video = track(1, MediaKind::Video, "mjpeg");
        // Exactly one audio: picked, audio or not.
        let picked = select_track(&[video.clone(), audio.clone()], TrackSelection::DefaultAudio, false)
            .must();
        assert_eq!(picked.id, audio.id);
        // Zero audio: typed missing.
        let err = assert_err(
            select_track(&[video], TrackSelection::DefaultAudio, true),
            "no audio track",
        );
        assert_eq!(err.context.code, "transcode.source.audio-track-missing");
        // Several audio: typed ambiguity, the count reported.
        let stereo_of_audio = track(2, MediaKind::Audio, "pcm_s24le");
        let err = assert_err(
            select_track(
                &[audio.clone(), stereo_of_audio],
                TrackSelection::DefaultAudio,
                true,
            ),
            "two audio tracks",
        );
        assert_eq!(err.context.code, "transcode.source.audio-ambiguous");
    }

    #[test]
    fn explicit_selection_enforces_existence_and_audio_where_needed() {
        let audio = track(0, MediaKind::Audio, "pcm_s16le");
        let video = track(1, MediaKind::Video, "mjpeg");
        let tracks = [audio.clone(), video.clone()];
        // Existing track by id.
        let picked = select_track(&tracks, TrackSelection::Track(video.id), false).must();
        assert_eq!(picked.id, video.id);
        // WAV sinks require audio: a video track is a typed refusal.
        let err = assert_err(
            select_track(&tracks, TrackSelection::Track(video.id), true),
            "video track into a WAV sink",
        );
        assert_eq!(err.context.code, "transcode.source.track-not-audio");
        // The same track is legal for the ES dump sink.
        select_track(&tracks, TrackSelection::Track(video.id), false).must();
        // Unknown id: typed not-found.
        let err = assert_err(
            select_track(&tracks, TrackSelection::Track(TrackId::new(9)), false),
            "unknown track",
        );
        assert_eq!(err.context.code, "transcode.source.track-not-found");
        // Audio by id for a WAV sink: legal.
        select_track(&tracks, TrackSelection::Track(audio.id), true).must();
    }

    #[test]
    fn copy_legality_matrix() {
        let s16 = PcmFacts {
            sample: demux::wav::format::PcmSample::S16Le,
            channels: 2,
            rate: 48_000,
        };
        // Matching LE descriptor: legal.
        check_copy_legality("pcm_s16le", &s16).must();
        // LE descriptor, disagreeing facts: typed mismatch.
        let err = assert_err(check_copy_legality("pcm_s24le", &s16), "facts lie");
        assert_eq!(err.context.code, "transcode.copy.facts-mismatch");
        // Big-endian source: the decode path's job.
        let err = assert_err(check_copy_legality("pcm_s16be", &s16), "byte order");
        assert_eq!(err.context.code, "transcode.copy.byte-order");
        // Not PCM at all: not a stream-copy.
        for descriptor in ["flac", "adpcm_ima_wav", "mjpeg", "opus"] {
            let err = assert_err(check_copy_legality(descriptor, &s16), "not pcm");
            assert_eq!(err.context.code, "transcode.copy.not-pcm", "{descriptor}");
        }
        // The whole LE family is copyable when the facts match.
        for (descriptor, sample) in [
            ("pcm_u8", demux::wav::format::PcmSample::U8),
            ("pcm_s24le", demux::wav::format::PcmSample::S24Le),
            ("pcm_s32le", demux::wav::format::PcmSample::S32Le),
            ("pcm_f32le", demux::wav::format::PcmSample::F32Le),
            ("pcm_f64le", demux::wav::format::PcmSample::F64Le),
        ] {
            let facts = PcmFacts {
                sample,
                channels: 1,
                rate: 44_100,
            };
            check_copy_legality(descriptor, &facts).must();
        }
    }

    #[test]
    fn progress_cadence_fires_on_step_boundaries() {
        let shared = JobShared::new(8, CancelToken::new());
        let mut watermark = 0u64;
        shared.update_stats(&JobStats {
            bytes_written: PROGRESS_STEP_BYTES - 1,
            ..JobStats::default()
        });
        emit_progress_cadence(&shared, &mut watermark);
        assert!(
            shared.try_recv_event().is_none(),
            "below the step: no event"
        );
        shared.update_stats(&JobStats {
            bytes_written: 1,
            ..JobStats::default()
        });
        assert!(
            shared.try_recv_event().is_none(),
            "cadence re-checks only on the next call"
        );
        emit_progress_cadence(&shared, &mut watermark);
        assert!(
            shared.try_recv_event().is_some(),
            "the step boundary emitted a progress event"
        );
    }

    #[test]
    fn too_large_maps_to_the_typed_limit() {
        let err = PacketStageQueue::too_large_error(700_000, 1024 * 1024);
        assert_eq!(err.context.code, "transcode.limits.over-budget");
        let _: EngineError = limit_over_budget(1, 2); // constructor reachability
    }
}
