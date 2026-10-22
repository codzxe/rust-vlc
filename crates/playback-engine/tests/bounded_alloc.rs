//! The D-1 boundedness proofs for the session's command/output path
//! (directives-audit M1, card W1-07): the receipt hard cap, the
//! outbox event cap and the bounded inbox held under **10k-command
//! storms**, measured with the testkit counting allocator — runtime
//! evidence, not syntactic green (D-6).
//!
//! One `#[test]` per binary by design (the allocator is
//! process-global; sibling test threads and concurrent harness
//! reporting would be counted — see `testkit::alloc_count` docs and
//! the `zero_alloc_quantum.rs` precedent). The binary runs
//! identically under `--test-threads=1` and the default parallel
//! runner. The two storm phases share the one test:
//!
//! - **receipt storm** (session settled in `Ended`, every command a
//!   typed rejection → receipt-only pushes): after the hub's ring is
//!   warmed to its steady bound, 10k pushes allocate **nothing**, the
//!   dropped-receipt counter shows the exact cap arithmetic
//!   (`10_000 − RECEIPT_CAPACITY`), and the drain answers exactly
//!   `RECEIPT_CAPACITY` receipts — the newest, in order;
//! - **confirmation storm** (session in `Paused`, every command an
//!   accepted self-transition → one receipt + one `state-changed`
//!   each): both caps hold with exact arithmetic, memory stays
//!   **flat across the whole storm** (`live_bytes` delta 0 — the
//!   per-command output vecs are host-paced transients, allocated
//!   and freed, and the counted allocation events are reported in
//!   the assertion message), and receipt→event causal adjacency
//!   survives the drops for every command that kept both halves.
//!
//! The non-measuring halves of the M1 contract — the inbox's typed
//! overflow refusal and the pinned cap constants — live in
//! `session_flow.rs` with the rest of the session-level behavior
//! suite; the hub-internal white-box tests (bounded slots at 10k
//! pressure, replace-resident-tick, receipts pinned from event
//! overflow) live in `session.rs`'s unit tests, and the ordering
//! contract in `contract_suite.rs`.

use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink};
use commands::envelope::RequestId;
use media_clock::MonotonicSource;
use media_types::frame::{AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::{
    AudioFormat, Duration, EngineError, Frame, GenerationId, MediaKind, Nanoseconds, Packet, Rate,
    SourceUri, Stamp, TimeBase, TimeSpan, Timestamp, Track, TrackFlags, TrackId, TrackTiming,
};
use packet_queue::QueueLimits;
use playback_engine::{
    CommandReceipt, DecoderProvider, Engine, EngineEventKind, EngineOutput, FrameConsumer,
    OUTBOX_CAPACITY, QueuePlan, RECEIPT_CAPACITY, ReceiptResult, SessionCommandKind, SessionConfig,
    SessionError, SessionHandle, TickBudget,
};
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;
use testkit::alloc_count::{CountingGlobalAlloc, serialize_tests};

#[global_allocator]
static ALLOC: CountingGlobalAlloc = CountingGlobalAlloc::new();

// ---------------------------------------------------------------------------
// Fakes (trimmed from `session_flow.rs` to the storm tests' needs: no
// stall, no armed failures — the storms never need them)
// ---------------------------------------------------------------------------

/// Shared virtual monotonic time (doc 03 §8: no sleeps): frozen at 0
/// for both the session clock's source and the event throttle.
struct SharedNow(AtomicU64);

impl MonotonicSource for SharedNow {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// The collecting sink: counts the frames the engine delivered.
#[derive(Default)]
struct Collector {
    frames: Mutex<Vec<(TrackId, Frame)>>,
}

impl FrameConsumer for Collector {
    fn accept(&self, frame: Frame, track: TrackId) -> Result<(), EngineError> {
        self.frames
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push((track, frame));
        Ok(())
    }
}

/// The scripted demux stage: a fixed track list and a finite packet
/// script; an exhausted script answers end-of-stream.
struct FakeDemuxer {
    tracks: Vec<Track>,
    script: VecDeque<DemuxEvent>,
}

impl Demuxer for FakeDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        Ok(self.tracks.clone())
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        match self.script.pop_front() {
            Some(event) => Ok(event),
            None => Ok(DemuxEvent::EndOfStream),
        }
    }

    fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
        Err(EngineError::new(
            media_types::PipelineStage::Demux,
            media_types::ErrorKind::Unsupported { format: None },
            media_types::error::ErrorContext::new("demux.seek.not-implemented"),
        ))
    }
}

/// The scripted decode stage: one frame out per packet.
struct FakeDecoder {
    generation: GenerationId,
}

impl Decoder for FakeDecoder {
    fn configure(&mut self, config: &DecoderConfig) -> Result<(), EngineError> {
        self.generation = config.generation;
        Ok(())
    }

    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        if packet.is_stale(self.generation) {
            return Ok(DecodeOutcome::NeedMoreInput);
        }
        out.accept(audio_frame(packet.pts, self.generation))?;
        Ok(DecodeOutcome::Emitted { frames: 1 })
    }

    fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        Ok(DecodeOutcome::Drained)
    }
}

/// The decoder seam: a working decoder per track.
struct FakeProvider;

impl DecoderProvider for FakeProvider {
    fn decoder_for(
        &self,
        _track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        Ok(Box::new(FakeDecoder {
            generation: config.generation,
        }))
    }
}

// ---------------------------------------------------------------------------
// Builders (mirroring `session_flow.rs`'s deterministic knobs)
// ---------------------------------------------------------------------------

fn base() -> TimeBase {
    TimeBase::new(1, 1_000_000).must()
}

fn at_micros(us: i64) -> Stamp {
    Stamp::Present(Timestamp::new(us, base()))
}

fn audio_track(id: u32) -> Track {
    Track {
        id: TrackId::new(id),
        stream: media_types::StreamId::new(id),
        kind: MediaKind::Audio,
        language: None,
        codec: media_types::ids::CodecDescriptor::new("fake-pcm").must(),
        title: None,
        flags: TrackFlags::NONE,
        timing: TrackTiming {
            span: TimeSpan::UNKNOWN,
            base: base(),
            gaps: Vec::new(),
        },
    }
}

fn packet(track: TrackId, index: u64) -> Packet {
    Packet {
        track,
        pts: at_micros((index * 1_000) as i64),
        dts: Stamp::Missing,
        duration: Some(Duration::Known(Nanoseconds(1_000_000))),
        data: SharedBytes::from_static(&[7u8; 64]),
        keyframe: true,
        discontinuity: false,
        generation: GenerationId::FIRST,
    }
}

fn audio_frame(pts: Stamp, generation: GenerationId) -> Frame {
    Frame::Audio(media_types::frame::AudioFrame {
        format: AudioFormat {
            sample: SampleFormat::S16,
            channels: 2,
            layout: AudioLayout::Interleaved,
            rate: 48_000,
        },
        samples: SharedBytes::from_static(&[0u8; 4]),
        pts,
        duration: None,
        generation,
    })
}

fn tiny_queues() -> QueuePlan {
    let limits = QueueLimits {
        max_packets: NonZeroUsize::new(16).must(),
        max_bytes: NonZeroUsize::new(1 << 20).must(),
        max_duration: Nanoseconds(1_000_000_000),
    };
    QueuePlan::new(limits, limits)
}

fn tiny_budget() -> TickBudget {
    TickBudget {
        demux_events: 64,
        packets_per_limb: 4,
        flush_calls_per_limb: 8,
    }
}

fn frozen_throttle() -> playback_engine::EventPolicy {
    playback_engine::EventPolicy {
        tick: playback_engine::TimeTickPolicy::default_policy(),
        throttle: Arc::new(SharedNow(AtomicU64::new(0))),
    }
}

/// An engine over a short scripted audio media, opened and **unpolled**
/// (the caller drives it to whatever state the storm needs).
fn open_audio(script_len: u64) -> (Engine, SessionHandle) {
    let mut script: Vec<DemuxEvent> = (0..script_len)
        .map(|index| DemuxEvent::Packet(packet(TrackId::new(1), index)))
        .collect();
    script.push(DemuxEvent::EndOfStream);
    let mut config = SessionConfig::new(
        SourceUri::parse("file:///tmp/lumen-bounded-storm.media").must(),
        Box::new(FakeDemuxer {
            tracks: vec![audio_track(1)],
            script: VecDeque::from(script),
        }),
        Arc::new(FakeProvider),
        Arc::new(Collector::default()),
        Arc::new(
            media_clock::MediaClock::system_monotonic(
                Arc::new(SharedNow(AtomicU64::new(0))),
                Timestamp::new(0, base()),
                Rate::ONE,
            )
            .must(),
        ),
    );
    config.queues = tiny_queues();
    config.budget = tiny_budget();
    config.throttle = frozen_throttle();
    let mut engine = Engine::new();
    let handle = engine.open(config).must();
    (engine, handle)
}

/// Drive polls until `predicate` holds on the engine state, or fail
/// loudly after `limit` polls (a hang is a failed test — doc 03
/// §4.6(f)).
fn poll_until(
    engine: &mut Engine,
    predicate: impl Fn(playback_engine::SessionState) -> bool,
    limit: usize,
    context: &str,
) {
    for _ in 0..limit {
        if predicate(engine.state()) {
            return;
        }
        engine.poll();
    }
    assert!(
        predicate(engine.state()),
        "{context}: state is {} after {limit} polls",
        engine.state()
    );
}

/// Submit `count` pause commands, polling only when the bounded inbox
/// refuses (the honest host shape under backpressure-by-rejection).
/// Returns the count of typed busy refusals seen — the overflow policy
/// exercised, never a silent drop.
fn storm_pauses(
    handle: &SessionHandle,
    engine: &mut Engine,
    first_request: u64,
    count: u64,
) -> u64 {
    let mut busy = 0u64;
    for offset in 0..count {
        loop {
            match handle.pause(RequestId(first_request + offset)) {
                Ok(()) => break,
                Err(SessionError::InboxBusy) => {
                    busy += 1;
                    engine.poll();
                }
                Err(other) => unreachable!("storm submit: {other}"),
            }
        }
    }
    // Flush the inbox so every accepted command's receipt is in the
    // hub before the caller measures.
    engine.poll();
    busy
}

fn receipts_of(outputs: &[EngineOutput]) -> Vec<u64> {
    outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Receipt(receipt) => Some(receipt.request.0),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// The proof (one test, two storm phases — see the module docs)
// ---------------------------------------------------------------------------

#[test]
fn the_session_output_path_is_bounded_under_10k_storms() {
    let _serial = serialize_tests();

    // -----------------------------------------------------------------
    // Phase 1 — the receipt storm (M1 item i): a session settled in
    // `Ended` answers every command with a typed rejection —
    // receipt-only pushes, no events — so a host that submits 10k
    // commands without draining outputs exercises the receipt
    // retention path alone.
    // -----------------------------------------------------------------
    let (mut engine, handle) = open_audio(8);
    poll_until(
        &mut engine,
        |state| state == playback_engine::SessionState::Ended,
        400,
        "reach Ended",
    );

    // Warmup: 12k receipt pushes grow the hub's ring to its steady
    // bound (compaction keeps slots ≤ ~2× the live population), and
    // the drain empties it without shrinking the ring.
    storm_pauses(&handle, &mut engine, 0, 12_000);
    let mut sink = engine.take_outputs();
    assert_eq!(
        receipts_of(&sink).len(),
        RECEIPT_CAPACITY,
        "warmup drain: exactly the cap's worth of receipts"
    );
    sink.clear();
    let before = engine.stats();

    // The measured window: 10k more receipt pushes.
    let guard = ALLOC.state().guard();
    let busy = storm_pauses(&handle, &mut engine, 100_000, 10_000);
    guard.assert_zero("10k receipt pushes into the capped hub (post-warmup)");
    assert!(
        busy > 0,
        "the storm saturated the inbox bound (exercised the typed refusal)"
    );

    // The cap arithmetic, exactly: every push past the cap dropped
    // and counted one oldest receipt.
    let after = engine.stats();
    assert_eq!(
        after.outbox_dropped_receipts - before.outbox_dropped_receipts,
        10_000 - RECEIPT_CAPACITY as u64,
        "dropped-receipt arithmetic: 10_000 − RECEIPT_CAPACITY"
    );
    assert_eq!(
        after.outbox_dropped_events - before.outbox_dropped_events,
        0,
        "no event pressure in a receipt-only storm"
    );

    // The drain: exactly the cap's worth of receipts — the newest,
    // in submit order — and nothing else.
    engine.drain_outputs_into(&mut sink);
    let requests = receipts_of(&sink);
    assert_eq!(
        requests.len(),
        RECEIPT_CAPACITY,
        "the collection is at the cap, not past it"
    );
    assert_eq!(
        requests.first(),
        Some(&(100_000 + 10_000 - RECEIPT_CAPACITY as u64)),
        "the oldest survivor is the (cap)th-newest request"
    );
    assert_eq!(requests.last(), Some(&(100_000 + 10_000 - 1)));
    assert!(
        requests.windows(2).all(|pair| pair[0] < pair[1]),
        "delivery order is push order"
    );
    assert!(
        sink.iter().all(|output| matches!(
            output,
            EngineOutput::Receipt(CommandReceipt {
                result: ReceiptResult::Rejected { .. },
                ..
            })
        )),
        "every storm receipt is the honest typed rejection"
    );
    assert!(
        sink.iter().all(|output| matches!(
            output,
            EngineOutput::Receipt(CommandReceipt {
                command: SessionCommandKind::Pause,
                ..
            })
        )),
        "and every receipt echoes the command it answers"
    );
    // Phase 1's engine dies here (deallocations only — the next
    // window opens after them).
    drop((engine, handle, sink));

    // -----------------------------------------------------------------
    // Phase 2 — the confirmation storm (M1 items i+iii together): a
    // session in `Paused` accepts every `pause` as a C8
    // self-transition — one receipt **and** one `state-changed` event
    // per command — so the storm exercises both caps and the
    // interleaved push path at engine level.
    // -----------------------------------------------------------------
    let (mut engine, handle) = open_audio(40);
    poll_until(
        &mut engine,
        |state| state == playback_engine::SessionState::Playing,
        400,
        "reach Playing",
    );
    handle.pause(RequestId(1)).must();
    poll_until(
        &mut engine,
        |state| state == playback_engine::SessionState::Paused,
        10,
        "the pause lands",
    );

    // Warmup: 12k accepted confirmations (receipt + event each) grow
    // the hub's ring past both caps' compaction bound; the drain
    // empties it without shrinking the ring.
    storm_pauses(&handle, &mut engine, 0, 12_000);
    let mut sink = engine.take_outputs();
    assert_eq!(receipts_of(&sink).len(), RECEIPT_CAPACITY);
    assert_eq!(
        sink.iter()
            .filter(|output| matches!(output, EngineOutput::Event(_)))
            .count(),
        OUTBOX_CAPACITY,
        "warmup drain: exactly the event cap's worth of state-changed"
    );
    sink.clear();
    let before = engine.stats();

    // The measured window: 10k accepted confirmations.
    let guard = ALLOC.state().guard();
    let busy = storm_pauses(&handle, &mut engine, 100_000, 10_000);
    let delta = guard.delta();
    assert!(
        busy > 0,
        "the storm saturated the inbox bound (exercised the typed refusal)"
    );
    assert_eq!(
        delta.live_bytes, 0,
        "memory is flat across the 10k-command storm (allocation events: \
         {} — the per-command output vecs, all freed; the hub ring is \
         already at its bound)",
        delta.alloc_calls
    );
    assert!(
        delta.alloc_calls > 0,
        "sanity: the storm actually ran (the transient output vecs were \
         allocated and freed)"
    );

    // Both caps, exact arithmetic.
    let after = engine.stats();
    assert_eq!(
        after.outbox_dropped_receipts - before.outbox_dropped_receipts,
        10_000 - RECEIPT_CAPACITY as u64,
        "dropped-receipt arithmetic"
    );
    assert_eq!(
        after.outbox_dropped_events - before.outbox_dropped_events,
        10_000 - OUTBOX_CAPACITY as u64,
        "dropped-event arithmetic: one state-changed per command, \
         oldest dropped at the cap"
    );

    // The drain: both populations at their caps, receipts strictly
    // ordered, and the causal adjacency intact for every command
    // that kept both halves — the newest OUTBOX_CAPACITY commands
    // still show receipt-then-its-own-event, back to back.
    engine.drain_outputs_into(&mut sink);
    let requests = receipts_of(&sink);
    assert_eq!(requests.len(), RECEIPT_CAPACITY);
    assert_eq!(
        requests.first(),
        Some(&(100_000 + 10_000 - RECEIPT_CAPACITY as u64)),
        "the oldest receipt survivor"
    );
    assert!(
        requests.windows(2).all(|pair| pair[0] < pair[1]),
        "delivery order is push order"
    );
    let events = sink
        .iter()
        .filter(|output| matches!(output, EngineOutput::Event(_)))
        .count();
    assert_eq!(
        events, OUTBOX_CAPACITY,
        "the event population is at the cap"
    );
    let mut paired = 0usize;
    let mut newest_pair = 0u64;
    for window in sink.windows(2) {
        if let [EngineOutput::Receipt(receipt), EngineOutput::Event(event)] = window
            && matches!(event.kind, EngineEventKind::StateChanged { .. })
        {
            assert_eq!(
                event.origin,
                Some(receipt.request),
                "causal adjacency: the event belongs to the receipt it follows"
            );
            assert!(
                receipt.request.0 > newest_pair,
                "pairs arrive in submit order"
            );
            newest_pair = receipt.request.0;
            paired += 1;
        }
    }
    assert_eq!(
        paired, OUTBOX_CAPACITY,
        "every command that kept both halves kept them adjacent, in order"
    );
    assert_eq!(
        newest_pair,
        100_000 + 10_000 - 1,
        "the last pair is the last command"
    );
    // And every storm receipt is the honest class-A accept (the
    // events interleaved among them are the confirmation
    // state-changeds — checked above).
    assert!(
        sink.iter().all(|output| matches!(
            output,
            EngineOutput::Event(_)
                | EngineOutput::Receipt(CommandReceipt {
                    result: ReceiptResult::Accepted { .. },
                    ..
                })
        )),
        "every confirmation receipt is the honest class-A accept"
    );
}
