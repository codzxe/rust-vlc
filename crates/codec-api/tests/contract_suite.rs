//! Card 1.D1-R C2: exercises `codec_api::contract` against mock codecs
//! kept **out of the public API** (this file is an integration test, not
//! a library module), and proves the suite itself catches misbehavior
//! via rogues. Also carries the static `Send` assertions for the trait
//! objects and configs.
//!
//! The mocks are deliberately tiny fake codecs with a documented
//! mini-language, not real decoders — the point is to drive the generic
//! suite the way a Phase-2 codec crate will.

use codec_api::contract;
use codec_api::prelude::*;
use media_types::error::{DecodeCause, ErrorContext};
use media_types::frame::{AudioFrame, ColorDescription, FrameStorage, SampleFormat, VideoFrame};
use media_types::packet::SharedBytes;
use media_types::{AudioFormat, PipelineStage, PixelFormat, Stamp, TrackId};

// ---- shared fixtures ------------------------------------------------------

/// The generation the mocks are configured at (the engine bumped once).
fn current_generation() -> GenerationId {
    GenerationId::FIRST
        .next()
        .unwrap_or_else(|e| unreachable!("{e}"))
}

/// A packet speaking the mock decoder's one-byte opcode language.
fn packet(opcode: u8, generation: GenerationId) -> Packet {
    Packet {
        track: TrackId::new(1),
        pts: Stamp::Missing,
        dts: Stamp::Missing,
        duration: None,
        data: SharedBytes::from_vec(vec![opcode]),
        keyframe: true,
        discontinuity: false,
        generation,
    }
}

/// A well-formed Nv12 2x2 video frame (6 bytes for a 3/2-plane layout).
fn nv12_frame(pts: Stamp, generation: GenerationId) -> Frame {
    Frame::Video(VideoFrame {
        format: PixelFormat::Nv12,
        size: (2, 2),
        data: FrameStorage::Cpu(SharedBytes::from_vec(vec![0u8; 6])),
        pts,
        color: ColorDescription::BT709_LIMITED,
        interlaced: false,
        generation,
    })
}

fn video_frame(generation: GenerationId) -> Frame {
    nv12_frame(Stamp::Missing, generation)
}

/// A video frame whose buffer does not match its declared dimensions.
fn corrupt_video_frame(generation: GenerationId) -> Frame {
    Frame::Video(VideoFrame {
        data: FrameStorage::Cpu(SharedBytes::from_vec(vec![0u8; 3])),
        ..match video_frame(generation) {
            Frame::Video(v) => v,
            _ => unreachable!("video_frame builds a video frame"),
        }
    })
}

/// A well-formed audio frame.
fn audio_frame(generation: GenerationId) -> Frame {
    Frame::Audio(AudioFrame {
        format: AudioFormat {
            sample: SampleFormat::S16,
            channels: 2,
            layout: media_types::frame::AudioLayout::Interleaved,
            rate: 48_000,
        },
        samples: SharedBytes::from_vec(vec![0; 4]),
        pts: Stamp::Missing,
        duration: None,
        generation,
    })
}

fn bad_state() -> EngineError {
    EngineError::new(
        PipelineStage::Decode,
        ErrorKind::Decoder {
            cause: DecodeCause::BadState,
        },
        ErrorContext::new("mock.unconfigured"),
    )
}

// ---- mock decoder (private: not part of the public API) -------------------
//
// Packet mini-language (first payload byte):
//   0x01 EMIT    -> one frame out (Emitted { frames: 1 })
//   0x02 BUFFER  -> hold one frame back (NeedMoreInput; flush releases)
//   0xCC CORRUPT -> bounded recovery (Recovered { skipped: 1, CorruptInput })
//   0xEE FATAL   -> typed fatal error (Decoder { Bitstream })
//   anything else is treated as corruption (Recovered { skipped: 1 }).
// Stale packets are dropped on sight via the canonical guard.

const OP_EMIT: u8 = 0x01;
const OP_BUFFER: u8 = 0x02;
const OP_CORRUPT: u8 = 0xCC;
const OP_FATAL: u8 = 0xEE;

struct MockDecoder {
    generation: GenerationId,
    configured: bool,
    pending: u32,
}

impl MockDecoder {
    fn new() -> MockDecoder {
        MockDecoder {
            generation: GenerationId::FIRST,
            configured: false,
            pending: 0,
        }
    }

    fn configured_at(generation: GenerationId) -> MockDecoder {
        let mut decoder = MockDecoder::new();
        match decoder.configure(&decoder_config(generation)) {
            Ok(()) => decoder,
            Err(err) => unreachable!("{err}"),
        }
    }
}

fn decoder_config(generation: GenerationId) -> DecoderConfig {
    DecoderConfig::new(
        CodecDescriptor::new("mock.video").unwrap_or_else(|e| unreachable!("{e}")),
        generation,
        ThreadBudget::default(),
        None,
        None,
    )
}

fn frame_from(packet: &Packet) -> Frame {
    nv12_frame(packet.pts, packet.generation)
}

impl Decoder for MockDecoder {
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
        if cfg.codec.as_str().contains("reject") {
            return Err(EngineError::new(
                PipelineStage::Decode,
                ErrorKind::Unsupported { format: None },
                ErrorContext::new("mock.configure.rejected"),
            ));
        }
        self.generation = cfg.generation;
        self.configured = true;
        self.pending = 0;
        Ok(())
    }

    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        if let Some(outcome) = drop_stale_packet(&packet, self.generation) {
            return Ok(outcome);
        }
        if !self.configured {
            return Err(bad_state());
        }
        match packet.data.as_slice().first() {
            Some(&OP_EMIT) => {
                out.accept(frame_from(&packet))?;
                Ok(DecodeOutcome::Emitted { frames: 1 })
            }
            Some(&OP_BUFFER) => {
                self.pending = self.pending.saturating_add(1);
                Ok(DecodeOutcome::NeedMoreInput)
            }
            Some(&OP_CORRUPT) => Ok(DecodeOutcome::Recovered {
                skipped: 1,
                reason: RecoverReason::CorruptInput,
            }),
            Some(&OP_FATAL) => Err(EngineError::new(
                PipelineStage::Decode,
                ErrorKind::Decoder {
                    cause: DecodeCause::Bitstream,
                },
                ErrorContext::new("mock.fatal"),
            )),
            _ => Ok(DecodeOutcome::Recovered {
                skipped: 1,
                reason: RecoverReason::CorruptInput,
            }),
        }
    }

    fn flush(&mut self, out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        if !self.configured {
            return Err(bad_state());
        }
        if self.pending == 0 {
            return Ok(DecodeOutcome::Drained);
        }
        let frames = self.pending;
        self.pending = 0;
        for _ in 0..frames {
            out.accept(nv12_frame(Stamp::Missing, self.generation))?;
        }
        Ok(DecodeOutcome::Emitted { frames })
    }
}

// ---- mock encoder (private) ------------------------------------------------
//
// Encodes CPU Nv12 frames into single-packet "GOPs" of two frames:
// the first frame buffers (NeedMoreInput), the second emits. Video
// frames whose buffer does not match Nv12 3/2 plane math for their
// declared dimensions are corrupt input (Recovered { skipped: 1 }).
// Stale frames are dropped on sight via the canonical guard.

struct MockEncoder {
    generation: GenerationId,
    configured: bool,
    pending: bool,
}

impl MockEncoder {
    fn new() -> MockEncoder {
        MockEncoder {
            generation: GenerationId::FIRST,
            configured: false,
            pending: false,
        }
    }

    fn configured_at(generation: GenerationId) -> MockEncoder {
        let mut encoder = MockEncoder::new();
        match encoder.configure(&encoder_config(generation)) {
            Ok(()) => encoder,
            Err(err) => unreachable!("{err}"),
        }
    }
}

fn encoder_config(generation: GenerationId) -> EncoderConfig {
    EncoderConfig::new(
        CodecDescriptor::new("mock.video").unwrap_or_else(|e| unreachable!("{e}")),
        generation,
        ThreadBudget::default(),
        None,
    )
}

fn nv12_bytes(width: u32, height: u32) -> Option<u64> {
    Some(
        u64::from(width)
            .checked_mul(u64::from(height))?
            .checked_mul(3)?
            / 2,
    )
}

fn packet_from(frame: &Frame) -> Packet {
    Packet {
        track: TrackId::new(7),
        pts: frame.pts(),
        dts: frame.pts(),
        duration: None,
        data: SharedBytes::from_vec(vec![0x42]),
        keyframe: true,
        discontinuity: false,
        generation: frame.generation(),
    }
}

impl Encoder for MockEncoder {
    fn configure(&mut self, cfg: &EncoderConfig) -> Result<(), EngineError> {
        if cfg.codec.as_str().contains("reject") {
            return Err(EngineError::new(
                PipelineStage::Transcode,
                ErrorKind::Unsupported { format: None },
                ErrorContext::new("mock.configure.rejected"),
            ));
        }
        self.generation = cfg.generation;
        self.configured = true;
        self.pending = false;
        Ok(())
    }

    fn encode(
        &mut self,
        frame: Frame,
        out: &mut dyn PacketSink,
    ) -> Result<EncodeOutcome, EngineError> {
        if let Some(outcome) = drop_stale_frame(&frame, self.generation) {
            return Ok(outcome);
        }
        if !self.configured {
            return Err(bad_state());
        }
        if let Frame::Video(video) = &frame {
            let expected = nv12_bytes(video.size.0, video.size.1);
            let well_formed = video.format == PixelFormat::Nv12
                && match &video.data {
                    FrameStorage::Cpu(bytes) => expected.is_some_and(|e| bytes.len() as u64 == e),
                    FrameStorage::OpaqueHandle(_) => false,
                };
            if !well_formed {
                return Ok(EncodeOutcome::Recovered {
                    skipped: 1,
                    reason: RecoverReason::CorruptInput,
                });
            }
        }
        if !self.pending {
            self.pending = true;
            return Ok(EncodeOutcome::NeedMoreInput);
        }
        self.pending = false;
        out.accept(packet_from(&frame))?;
        Ok(EncodeOutcome::Emitted { packets: 1 })
    }

    fn flush(&mut self, out: &mut dyn PacketSink) -> Result<EncodeOutcome, EngineError> {
        if !self.configured {
            return Err(bad_state());
        }
        if !self.pending {
            return Ok(EncodeOutcome::Drained);
        }
        self.pending = false;
        let packet = Packet {
            track: TrackId::new(7),
            pts: Stamp::Missing,
            dts: Stamp::Missing,
            duration: None,
            data: SharedBytes::from_vec(vec![0x42]),
            keyframe: true,
            discontinuity: false,
            generation: self.generation,
        };
        out.accept(packet)?;
        Ok(EncodeOutcome::Emitted { packets: 1 })
    }
}

// ---- the suite, exercised ---------------------------------------------------

#[test]
fn mock_decoder_passes_the_contract_suite() {
    let current = current_generation();

    // Stale packets are dropped on sight.
    let mut decoder = MockDecoder::configured_at(current);
    contract::stale_packet_is_dropped_on_sight(
        &mut decoder,
        &packet(OP_EMIT, GenerationId::FIRST),
        current,
    )
    .unwrap_or_else(|v| unreachable!("{v}"));

    // Flush drains: two buffered frames come out, then Drained sticks.
    let mut decoder = MockDecoder::configured_at(current);
    let mut warm = contract::CountingFrameSink::new();
    for opcode in [OP_BUFFER, OP_BUFFER] {
        match decoder.decode(packet(opcode, current), &mut warm) {
            Ok(DecodeOutcome::NeedMoreInput) => {}
            Ok(other) => unreachable!("buffering decode returned {other:?}"),
            Err(err) => unreachable!("{err}"),
        }
    }
    contract::flush_drains(&mut decoder).unwrap_or_else(|v| unreachable!("{v}"));

    // Corrupt input recovers with a skipped count.
    let mut decoder = MockDecoder::configured_at(current);
    contract::corrupt_input_recovers_or_errors_typed(&mut decoder, &packet(OP_CORRUPT, current))
        .unwrap_or_else(|v| unreachable!("{v}"));

    // Sink errors propagate (an emitting packet plus a failing sink).
    let mut decoder = MockDecoder::configured_at(current);
    contract::sink_errors_propagate(&mut decoder, &packet(OP_EMIT, current))
        .unwrap_or_else(|v| unreachable!("{v}"));
}

#[test]
fn contract_suite_accepts_a_typed_fatal_error_for_corrupt_input() {
    // The fatal branch of the corrupt-input rule: a typed
    // Decoder{Bitstream} error is a legal answer.
    let current = current_generation();
    let mut decoder = MockDecoder::configured_at(current);
    contract::corrupt_input_recovers_or_errors_typed(&mut decoder, &packet(OP_FATAL, current))
        .unwrap_or_else(|v| unreachable!("{v}"));
}

#[test]
fn mock_encoder_passes_the_contract_suite() {
    let current = current_generation();

    // Stale frames are dropped on sight.
    let mut encoder = MockEncoder::configured_at(current);
    contract::stale_frame_is_dropped_on_sight(
        &mut encoder,
        &video_frame(GenerationId::FIRST),
        current,
    )
    .unwrap_or_else(|v| unreachable!("{v}"));

    // Flush drains: one buffered frame (GOP half-full) comes out.
    let mut encoder = MockEncoder::configured_at(current);
    let mut warm = contract::CountingPacketSink::new();
    match encoder.encode(video_frame(current), &mut warm) {
        Ok(EncodeOutcome::NeedMoreInput) => {}
        Ok(other) => unreachable!("GOP buffering returned {other:?}"),
        Err(err) => unreachable!("{err}"),
    }
    contract::encoder_flush_drains(&mut encoder).unwrap_or_else(|v| unreachable!("{v}"));

    // A malformed frame recovers with a skipped count.
    let mut encoder = MockEncoder::configured_at(current);
    contract::corrupt_frame_recovers_or_errors_typed(&mut encoder, &corrupt_video_frame(current))
        .unwrap_or_else(|v| unreachable!("{v}"));

    // Packet-sink errors propagate. The emitting fixture is the SECOND
    // frame of a GOP (the mock buffers the first), so warm up first.
    let mut encoder = MockEncoder::configured_at(current);
    let mut warm = contract::CountingPacketSink::new();
    match encoder.encode(video_frame(current), &mut warm) {
        Ok(EncodeOutcome::NeedMoreInput) => {}
        Ok(other) => unreachable!("GOP buffering returned {other:?}"),
        Err(err) => unreachable!("{err}"),
    }
    contract::packet_sink_errors_propagate(&mut encoder, &video_frame(current))
        .unwrap_or_else(|v| unreachable!("{v}"));
}

// ---- the suite catches misbehavior (rogues) ---------------------------------

/// Ways a decoder can break the contract; each must be caught by the
/// suite with a matching violation.
enum Rogue {
    /// Decodes stale packets instead of dropping them.
    EmitOnStale,
    /// Swallows sink errors and reports success.
    SwallowSinkErrors,
    /// `flush` never reaches Drained.
    NeverDrain,
}

struct RogueDecoder {
    misbehavior: Rogue,
}

impl RogueDecoder {
    fn new(misbehavior: Rogue) -> RogueDecoder {
        RogueDecoder { misbehavior }
    }
}

impl Decoder for RogueDecoder {
    fn configure(&mut self, _cfg: &DecoderConfig) -> Result<(), EngineError> {
        Ok(())
    }

    fn decode(
        &mut self,
        _packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        match self.misbehavior {
            Rogue::EmitOnStale => Ok(DecodeOutcome::Emitted { frames: 1 }),
            Rogue::SwallowSinkErrors => {
                let frame = nv12_frame(Stamp::Missing, GenerationId::FIRST);
                // Contract violation under test: the failure is discarded.
                let _ = out.accept(frame);
                Ok(DecodeOutcome::Emitted { frames: 1 })
            }
            Rogue::NeverDrain => Ok(DecodeOutcome::NeedMoreInput),
        }
    }

    fn flush(&mut self, out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        match self.misbehavior {
            // Emits one frame per call, forever.
            Rogue::NeverDrain => {
                out.accept(nv12_frame(Stamp::Missing, GenerationId::FIRST))?;
                Ok(DecodeOutcome::Emitted { frames: 1 })
            }
            _ => Ok(DecodeOutcome::Drained),
        }
    }
}

#[test]
fn rogue_decoders_are_caught_by_the_suite() {
    let current = current_generation();

    // Stale packets must not be decoded.
    let mut rogue = RogueDecoder::new(Rogue::EmitOnStale);
    match contract::stale_packet_is_dropped_on_sight(
        &mut rogue,
        &packet(OP_EMIT, GenerationId::FIRST),
        current,
    ) {
        Err(v) => assert!(v.detail().contains("NeedMoreInput"), "{}", v.detail()),
        Ok(()) => unreachable!("emitting on stale must be caught"),
    }

    // Sink failures must not be swallowed.
    let mut rogue = RogueDecoder::new(Rogue::SwallowSinkErrors);
    match contract::sink_errors_propagate(&mut rogue, &packet(OP_EMIT, current)) {
        Err(v) => assert!(v.detail().contains("swallowed"), "{}", v.detail()),
        Ok(()) => unreachable!("swallowing sink errors must be caught"),
    }

    // Drains must terminate.
    let mut rogue = RogueDecoder::new(Rogue::NeverDrain);
    match contract::flush_drains(&mut rogue) {
        Err(v) => assert!(
            v.detail().contains("never reached Drained"),
            "{}",
            v.detail()
        ),
        Ok(()) => unreachable!("an unbounded drain must be caught"),
    }
}

// ---- mock behavior + misuse paths -------------------------------------------

#[test]
fn mock_decoder_outcome_language_behaves_as_documented() {
    let current = current_generation();
    let mut decoder = MockDecoder::configured_at(current);

    // Emitting opcode: one frame with the packet's generation.
    let mut sink = contract::CountingFrameSink::new();
    match decoder.decode(packet(OP_EMIT, current), &mut sink) {
        Ok(DecodeOutcome::Emitted { frames: 1 }) => {}
        Ok(other) => unreachable!("{other:?}"),
        Err(err) => unreachable!("{err}"),
    }
    assert_eq!(sink.accepted(), 1);
    assert_eq!(sink.frames()[0].generation(), current);

    // Buffering opcode: held back, released by flush.
    for opcode in [OP_BUFFER, OP_BUFFER] {
        match decoder.decode(packet(opcode, current), &mut sink) {
            Ok(DecodeOutcome::NeedMoreInput) => {}
            Ok(other) => unreachable!("{other:?}"),
            Err(err) => unreachable!("{err}"),
        }
    }
    assert_eq!(sink.accepted(), 1, "buffering emits nothing");
    match decoder.flush(&mut sink) {
        Ok(DecodeOutcome::Emitted { frames: 2 }) => {}
        Ok(other) => unreachable!("{other:?}"),
        Err(err) => unreachable!("{err}"),
    }
    match decoder.flush(&mut sink) {
        Ok(DecodeOutcome::Drained) => {}
        Ok(other) => unreachable!("{other:?}"),
        Err(err) => unreachable!("{err}"),
    }

    // Unknown opcodes are corrupt input, recovered with a skip count.
    match decoder.decode(packet(0x77, current), &mut sink) {
        Ok(DecodeOutcome::Recovered { skipped: 1, reason }) => {
            assert_eq!(reason, RecoverReason::CorruptInput);
        }
        Ok(other) => unreachable!("{other:?}"),
        Err(err) => unreachable!("{err}"),
    }

    // Sink failures propagate verbatim (context code, not a re-mint).
    let mut failing = contract::FailingFrameSink::new();
    match decoder.decode(packet(OP_EMIT, current), &mut failing) {
        Err(err) => assert_eq!(err.context.code, "contract.sink.failure"),
        Ok(other) => unreachable!("sink failure must propagate, got {other:?}"),
    }
    // ...and the decoder remains drainable afterwards.
    match decoder.flush(&mut contract::CountingFrameSink::new()) {
        Ok(DecodeOutcome::Drained) => {}
        Ok(other) => unreachable!("{other:?}"),
        Err(err) => unreachable!("{err}"),
    }
}

#[test]
fn mock_encoder_gop_and_recovery_behave_as_documented() {
    let current = current_generation();
    let mut encoder = MockEncoder::configured_at(current);
    let mut sink = contract::CountingPacketSink::new();

    // GOP of two: first frame buffers, second emits.
    match encoder.encode(video_frame(current), &mut sink) {
        Ok(EncodeOutcome::NeedMoreInput) => {}
        Ok(other) => unreachable!("{other:?}"),
        Err(err) => unreachable!("{err}"),
    }
    match encoder.encode(audio_frame(current), &mut sink) {
        Ok(EncodeOutcome::Emitted { packets: 1 }) => {}
        Ok(other) => unreachable!("{other:?}"),
        Err(err) => unreachable!("{err}"),
    }
    assert_eq!(sink.accepted(), 1);
    assert_eq!(sink.packets()[0].generation, current);

    // A half-full GOP flushes out one packet, then Drained.
    match encoder.encode(video_frame(current), &mut sink) {
        Ok(EncodeOutcome::NeedMoreInput) => {}
        Ok(other) => unreachable!("{other:?}"),
        Err(err) => unreachable!("{err}"),
    }
    match encoder.flush(&mut sink) {
        Ok(EncodeOutcome::Emitted { packets: 1 }) => {}
        Ok(other) => unreachable!("{other:?}"),
        Err(err) => unreachable!("{err}"),
    }
    match encoder.flush(&mut sink) {
        Ok(EncodeOutcome::Drained) => {}
        Ok(other) => unreachable!("{other:?}"),
        Err(err) => unreachable!("{err}"),
    }

    // Malformed video input is skipped, not fatal.
    match encoder.encode(corrupt_video_frame(current), &mut sink) {
        Ok(EncodeOutcome::Recovered { skipped: 1, reason }) => {
            assert_eq!(reason, RecoverReason::CorruptInput);
        }
        Ok(other) => unreachable!("{other:?}"),
        Err(err) => unreachable!("{err}"),
    }
}

#[test]
fn unconfigured_use_and_configure_rejection_are_typed_errors() {
    let current = current_generation();

    // Decode/flush before configure: typed Decoder{BadState}.
    let mut decoder = MockDecoder::new();
    match decoder.decode(
        packet(OP_EMIT, current),
        &mut contract::CountingFrameSink::new(),
    ) {
        Err(err) => {
            assert!(
                matches!(
                    err.kind,
                    ErrorKind::Decoder {
                        cause: DecodeCause::BadState
                    }
                ),
                "{err}"
            );
            assert_eq!(err.context.code, "mock.unconfigured");
        }
        Ok(other) => unreachable!("unconfigured decode must fail, got {other:?}"),
    }
    match decoder.flush(&mut contract::CountingFrameSink::new()) {
        Err(err) => assert_eq!(err.context.code, "mock.unconfigured"),
        Ok(other) => unreachable!("unconfigured flush must fail, got {other:?}"),
    }
    let mut encoder = MockEncoder::new();
    match encoder.encode(
        video_frame(current),
        &mut contract::CountingPacketSink::new(),
    ) {
        Err(err) => assert_eq!(err.context.code, "mock.unconfigured"),
        Ok(other) => unreachable!("unconfigured encode must fail, got {other:?}"),
    }

    // Configure rejection is a typed Unsupported error.
    let mut reject = MockDecoder::new();
    let rejected = DecoderConfig::new(
        CodecDescriptor::new("mock.reject-me").unwrap_or_else(|e| unreachable!("{e}")),
        current,
        ThreadBudget::default(),
        None,
        None,
    );
    match reject.configure(&rejected) {
        Err(err) => {
            assert!(matches!(err.kind, ErrorKind::Unsupported { .. }), "{err}");
            assert_eq!(err.context.code, "mock.configure.rejected");
        }
        Ok(()) => unreachable!("the reject-me codec must be refused"),
    }
}

// ---- static Send assertions (misuse-proofing) --------------------------------

#[test]
fn trait_objects_and_configs_are_send() {
    fn assert_send<T: ?Sized + Send>() {}
    fn assert_send_sync<T: ?Sized + Send + Sync>() {}

    // The worker-thread hand-off requires the trait objects to cross
    // threads (the compile_fail doctest on `Decoder` is the negative
    // proof; these are the positive ones).
    assert_send::<dyn Decoder>();
    assert_send::<dyn Encoder>();
    assert_send_sync::<dyn HwAccelerator>();

    // Negotiated configs must travel to the worker: whole-type Send +
    // Sync (the accelerator is Arc-shared, hence the Sync supertrait).
    assert_send_sync::<DecoderConfig>();
    assert_send_sync::<EncoderConfig>();

    // Capability claims and outcomes are plain Send data.
    assert_send::<CodecCapability>();
    assert_send::<DecodeOutcome>();
    assert_send::<EncodeOutcome>();

    // FrameSink/PacketSink are deliberately NOT bound to Send (they are
    // created and consumed on the worker thread that owns the codec) —
    // no assertion is possible or wanted for `dyn FrameSink`.
}
