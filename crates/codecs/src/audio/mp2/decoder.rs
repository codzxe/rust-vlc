//! The MP2 decoder core (card 5.J3): the symphonia-backed frame
//! decode, the stream-level pipeline (ID3 → walk → CBR gate → PCM),
//! and the [`codec_api::Decoder`] implementation.
//!
//! # Engine (no new dependency — the family's admission)
//!
//! **`symphonia-bundle-mp3` 0.6.1**, the exact-pinned engine card
//! 5.A3 already landed under the Lead's C5 pre-approval
//! (`docs/dependencies/records/symphonia-mp3-0.6.1.md`): the bundle
//! is the MPEG-audio family engine — its `MpaDecoder` dispatches
//! Layers I/II/III, and this module configures it with
//! `CODEC_ID_MP2`. The engine owns exactly one job: entropy-decoding
//! ONE already-framed Layer II frame (allocation, SCFSI, quantization
//! → subband synthesis) to F32 planes. The Layer II gate, the CBR
//! geometry, the padding-aware seek arithmetic and the error taxonomy
//! are ours — a future engine swap stays inside this module.
//!
//! # Packet model (the codec-api contract)
//!
//! One packet = **one complete frame** (header + allocation + data,
//! exactly `header.frame_size` bytes — the engine hard-requires the
//! match). Layer II frames are independent (each carries its full
//! side information), so per-frame isolation (doc 03 §4.5(d)) holds:
//! after a typed `Corrupt` the next packet decodes unchanged.
//!
//! # No gapless facts — the honest Layer II posture
//!
//! There is no Xing/Info/LAME or VBRI construction in Layer II
//! (those are Layer III encoder side-channels), so no delay/padding
//! window exists to trim and [`DecoderConfig::extra_data`] accepts
//! nothing: a non-empty blob is a typed refusal. ffmpeg's Layer II
//! decode emits exactly `frames × 1152` samples and so does this
//! module — the differential is sample-locked with no window
//! arithmetic (`tests/mp2_corpus_differential.rs`).

use super::error::{Mp2Error, corrupt, unsupported};
use super::header::{Mp2FrameAt, Mp2FrameHeader, Mp2Geometry, geometry, parse_mp2_header};
use codec_api::prelude::*;
use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::time::{Nanoseconds, TimeBase, Timestamp};
use media_types::{Duration, Stamp};
use symphonia_bundle_mp3::MpaDecoder;
use symphonia_core::codecs::audio::well_known::CODEC_ID_MP2;
use symphonia_core::codecs::audio::{AudioCodecParameters, AudioDecoder, AudioDecoderOptions};
use symphonia_core::packet::PacketRef;
use symphonia_core::units::{Duration as SymDuration, Timestamp as SymTimestamp};

/// One decoded MP2 stream (the free surface's result): interleaved
/// F32 samples plus the walked geometry.
#[derive(Debug, Clone)]
pub struct DecodedMp2 {
    /// The output shape (F32, interleaved, native channels/rate).
    pub format: AudioFormat,
    /// Interleaved F32 samples (`total_frames × 1152 × channels`).
    pub samples: Vec<f32>,
    /// Decoded frame count.
    pub frames: usize,
    /// The parsed leading ID3v2 tag, when present (raw MP2 files in
    /// the wild carry them; parsed by the family module, skipped).
    pub id3: Option<crate::audio::mp3::Id3Tag>,
    /// The verified CBR geometry.
    pub geometry: Mp2Geometry,
    /// Byte offset of the first decoded frame.
    pub first_frame_offset: u64,
}

/// Decode one framed Layer II packet through the engine (shared by
/// the contract path and the stream pipeline).
///
/// `scratch` is reused between calls. The packet must be exactly one
/// complete frame; anything shorter is the typed truncation and
/// anything with a non-Layer II header is the typed layer gate.
fn decode_one(
    decoder: &mut MpaDecoder,
    packet: &[u8],
    scratch: &mut Vec<f32>,
) -> Result<(usize, Mp2FrameHeader), EngineError> {
    let header = parse_mp2_header(packet)?;
    let need_len = header.family.frame_size as usize;
    if packet.len() < need_len {
        return Err(corrupt(
            "mp2.frame.truncated",
            media_types::error::CorruptDetail::Truncated {
                expected: need_len as u64,
                found: packet.len() as u64,
            },
            "a Layer II packet must carry its whole frame — the engine \
             hard-requires header.frame_size bytes",
        )
        .into_engine_error());
    }
    let frame_bytes = packet.get(..need_len).unwrap_or(&[]);
    let pkt = PacketRef::new(0, SymTimestamp::ZERO, SymDuration::ZERO, frame_bytes);
    let decoded = match decoder.decode_ref(&pkt) {
        Ok(buf) => buf,
        Err(e) => {
            return Err(corrupt(
                "mp2.frame.decode",
                media_types::error::CorruptDetail::InvalidStructure { offset: None },
                format!("engine rejected the frame: {e}"),
            )
            .into_engine_error());
        }
    };
    let frames = decoded.frames();
    let channels = decoded.spec().channels().count();
    let need = frames * channels;
    if scratch.len() < need {
        scratch.resize(need, 0.0);
    }
    decoded.copy_to_slice_interleaved::<f32, _>(&mut scratch[..need]);
    Ok((frames, header))
}

/// The stream-level pipeline (the conformance/differential
/// workhorse): ID3 skip → Layer II walk → CBR geometry gate → decode
/// every walked frame.
///
/// # Errors
/// Typed [`EngineError`]s: ID3 damage, no Layer II frames
/// (`mp2.stream.no_frames`), a mid-stream layer switch
/// (`mp2.stream.layer_mismatch`), a geometry disagreement
/// (`mp2.stream.cbr_mismatch`), engine rejections. Damage is never a
/// panic; garbage between frames is resynced past by the walker.
pub fn decode_stream(data: &[u8]) -> Result<DecodedMp2, EngineError> {
    // Leading ID3v2: parse (kept) and skip — the family parser.
    let mut pos = 0usize;
    let mut id3 = None;
    if data.get(..3) == Some(b"ID3") {
        let tag = crate::audio::mp3::parse_id3v2(data)?;
        pos = tag.total_len;
        id3 = Some(tag);
    }
    let frames: Vec<Mp2FrameAt> = super::header::walk_mp2_frames(data, pos)?;
    if frames.is_empty() {
        return Err(corrupt(
            "mp2.stream.no_frames",
            media_types::error::CorruptDetail::Truncated {
                expected: 4,
                found: (data.len() - pos.min(data.len())) as u64,
            },
            "no Layer II frame header found after the ID3 tag",
        )
        .into_engine_error());
    }
    let geo = match geometry(&frames)? {
        Some(g) => g,
        None => {
            return Err(corrupt(
                "mp2.stream.no_frames",
                media_types::error::CorruptDetail::Truncated {
                    expected: 4,
                    found: 0,
                },
                "the walk produced no frames",
            )
            .into_engine_error());
        }
    };
    // Engine set-up (Layer II of the family bundle; gapless is a
    // Layer III concept — off).
    let params = AudioCodecParameters {
        codec: CODEC_ID_MP2,
        ..AudioCodecParameters::default()
    };
    let mut engine_opts = AudioDecoderOptions::default();
    engine_opts.gapless = false;
    engine_opts.verify = false;
    let mut engine = match MpaDecoder::try_new(&params, &engine_opts) {
        Ok(e) => e,
        Err(e) => {
            return Err(
                unsupported("mp2.engine.init", format!("engine refused the stream: {e}"))
                    .into_engine_error(),
            );
        }
    };
    let mut scratch: Vec<f32> = Vec::new();
    let mut samples: Vec<f32> = Vec::new();
    let mut spec_channels = 0usize;
    let mut spec_rate = 0u32;
    for frame in &frames {
        let bytes = data
            .get(frame.offset..frame.offset + frame.header.family.frame_size as usize)
            .unwrap_or(&[]);
        let (frames_decoded, _h) = decode_one(&mut engine, bytes, &mut scratch)?;
        if spec_channels == 0 {
            let decoded = engine.last_decoded();
            let spec = decoded.spec();
            spec_channels = spec.channels().count();
            spec_rate = spec.rate();
        }
        samples.extend_from_slice(&scratch[..frames_decoded * spec_channels]);
    }
    if spec_channels == 0 {
        return Err(corrupt(
            "mp2.stream.no_decoded_samples",
            media_types::error::CorruptDetail::InvalidStructure { offset: None },
            "no frame decoded",
        )
        .into_engine_error());
    }
    Ok(DecodedMp2 {
        format: AudioFormat {
            sample: SampleFormat::F32,
            channels: spec_channels as u32,
            layout: AudioLayout::Interleaved,
            rate: spec_rate,
        },
        samples,
        frames: frames.len(),
        id3,
        geometry: geo,
        first_frame_offset: frames[0].offset as u64,
    })
}

/// MP2 decoder behind the codec-api [`Decoder`] contract (card 5.J3;
/// doc 04 §3.1 row "MP2").
///
/// One packet = one Layer II frame, exactly `frame_size` bytes.
/// The descriptor gate is **exactly `mp2`** (the honest Layer II
/// surface — Layer I/III packets are the family module's, and a
/// non-Layer II header inside a packet is the typed
/// `mp2.header.not_layer_ii`). No `extra_data` facts are accepted:
/// Layer II has no gapless metadata construction (see the module
/// doc), and guessing one is not allowed.
#[derive(Default)]
pub struct Mp2Decoder {
    /// The configured stream generation (None until `configure`).
    generation: Option<GenerationId>,
    /// The engine (created at configure).
    engine: Option<MpaDecoder>,
    /// Packets fed so far (0-based index of the next).
    packet_index: u32,
    /// Per-channel samples fed so far (pts bookkeeping).
    samples_fed: u64,
    /// Reusable interleaved scratch.
    scratch: Vec<f32>,
    /// Pooled output slots (the 5.A1 rotation).
    sample_pool: Vec<SharedBytes>,
    /// Round-robin cursor.
    slot_next: usize,
    /// Negotiated output shape (from the first decoded frame).
    out_channels: u32,
    out_rate: u32,
}

impl std::fmt::Debug for Mp2Decoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mp2Decoder")
            .field("packet_index", &self.packet_index)
            .field("samples_fed", &self.samples_fed)
            .finish_non_exhaustive()
    }
}

/// Output-rotation depth (the 5.A1 bound).
const SAMPLE_SLOTS: usize = 4;

/// Layer II samples per frame, every MPEG version (the 5.A3 bound
/// comment holds 1152 across the family).
const MAX_SAMPLES_PER_FRAME: usize = 1152;

impl Mp2Decoder {
    /// A fresh decoder, unconfigured.
    #[must_use]
    pub fn new() -> Mp2Decoder {
        Mp2Decoder::default()
    }

    /// The codec descriptors this decoder accepts: exactly `mp2`.
    fn accepts(descriptor: &str) -> bool {
        descriptor == "mp2"
    }
}

impl Decoder for Mp2Decoder {
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
        let descriptor = cfg.codec.as_str();
        if !Self::accepts(descriptor) {
            return Err(unsupported(
                "mp2.codec_descriptor",
                format!("codec descriptor `{descriptor}` is not mp2 (the Layer II surface)"),
            )
            .into_engine_error());
        }
        if let Some(extra) = cfg.extra_data.as_ref()
            && !extra.is_empty()
        {
            return Err(unsupported(
                "mp2.facts.unexpected",
                "extra_data present but Layer II defines no gapless facts — \
                 there is no Xing/LAME/VBRI construction to carry them",
            )
            .into_engine_error());
        }
        let params = AudioCodecParameters {
            codec: CODEC_ID_MP2,
            ..AudioCodecParameters::default()
        };
        let mut engine_opts = AudioDecoderOptions::default();
        engine_opts.gapless = false;
        engine_opts.verify = false;
        let engine = MpaDecoder::try_new(&params, &engine_opts).map_err(|e| {
            unsupported(
                "mp2.engine.init",
                format!("engine refused the configuration: {e}"),
            )
            .into_engine_error()
        })?;
        self.engine = Some(engine);
        self.generation = Some(cfg.generation);
        self.packet_index = 0;
        self.samples_fed = 0;
        self.scratch.clear();
        self.sample_pool.clear();
        self.slot_next = 0;
        self.out_channels = 0;
        self.out_rate = 0;
        Ok(())
    }

    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        let generation = match self.generation {
            Some(g) => g,
            None => return Err(Mp2Error::BadState.into_engine_error()),
        };
        let engine = match self.engine.as_mut() {
            Some(e) => e,
            None => return Err(Mp2Error::BadState.into_engine_error()),
        };
        if let Some(outcome) = drop_stale_packet(&packet, generation) {
            return Ok(outcome);
        }
        if packet.data.is_empty() {
            return Ok(DecodeOutcome::NeedMoreInput);
        }
        let (frame_samples, _header) =
            decode_one(engine, packet.data.as_slice(), &mut self.scratch)?;
        if self.out_channels == 0 {
            let decoded = engine.last_decoded();
            let spec = decoded.spec();
            self.out_channels = spec.channels().count() as u32;
            self.out_rate = spec.rate();
        }
        let channels = self.out_channels as usize;
        let interleaved = &self.scratch[..frame_samples * channels];
        self.packet_index += 1;
        let emitted_samples = interleaved.len() / channels.max(1);
        if emitted_samples > 0 {
            let slot_len = MAX_SAMPLES_PER_FRAME * channels * 4;
            let bytes = pooled_sample_slot(
                &mut self.sample_pool,
                &mut self.slot_next,
                slot_len,
                |dst| {
                    let mut cursor = 0usize;
                    for v in interleaved {
                        dst[cursor..cursor + 4].copy_from_slice(&v.to_ne_bytes());
                        cursor += 4;
                    }
                    Ok::<(), Mp2Error>(())
                },
            )?;
            let exact = emitted_samples * channels * 4;
            let base = TimeBase::new(1, self.out_rate.max(1)).map_err(|_| {
                unsupported("mp2.stream.rate", "negotiated rate is 0").into_engine_error()
            })?;
            let frame = Frame::Audio(media_types::frame::AudioFrame {
                format: AudioFormat {
                    sample: SampleFormat::F32,
                    channels: self.out_channels,
                    layout: AudioLayout::Interleaved,
                    rate: self.out_rate,
                },
                samples: bytes.slice(0..exact),
                pts: Stamp::Present(Timestamp::new(
                    i64::try_from(self.samples_fed).unwrap_or(i64::MAX),
                    base,
                )),
                duration: Some(Duration::Known(Nanoseconds(
                    u64::try_from(
                        emitted_samples as u128 * 1_000_000_000u128
                            / u128::from(self.out_rate.max(1)),
                    )
                    .unwrap_or(u64::MAX),
                ))),
                generation: packet.generation,
            });
            out.accept(frame)?;
        }
        self.samples_fed += frame_samples as u64;
        Ok(DecodeOutcome::Emitted { frames: 1 })
    }

    fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        if self.generation.is_none() || self.engine.is_none() {
            return Err(Mp2Error::BadState.into_engine_error());
        }
        // Layer II keeps no cross-frame state: nothing buffered.
        Ok(DecodeOutcome::Drained)
    }
}

/// The pooled output rotation (the 5.A1 `pooled_sample_slot`
/// pattern, re-authored for the f32 path — the family's, verbatim
/// in mechanics).
fn pooled_sample_slot(
    slots: &mut Vec<SharedBytes>,
    next: &mut usize,
    slot_len: usize,
    fill: impl FnOnce(&mut [u8]) -> Result<(), Mp2Error>,
) -> Result<SharedBytes, EngineError> {
    let count = slots.len();
    for i in 0..count {
        let idx = (*next + i) % count;
        if slots[idx].len() != slot_len {
            continue;
        }
        if let Some(bytes) = slots[idx].get_mut() {
            fill(bytes)?;
            *next = (idx + 1) % count;
            return Ok(slots[idx].clone());
        }
    }
    let mut fresh = SharedBytes::from_vec(vec![0; slot_len]);
    match fresh.get_mut() {
        Some(bytes) => fill(bytes),
        None => {
            return Err(corrupt(
                "mp2.output.fresh_slot_not_exclusive",
                media_types::error::CorruptDetail::InvalidStructure { offset: None },
                "fresh output slot is not exclusively owned",
            )
            .into_engine_error());
        }
    }?;
    if slots.len() < SAMPLE_SLOTS {
        slots.push(fresh.clone());
        *next = slots.len() - 1;
    } else {
        *next = (*next + 1) % SAMPLE_SLOTS;
        slots[*next] = fresh.clone();
    }
    Ok(fresh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_gate_is_exactly_mp2() {
        assert!(Mp2Decoder::accepts("mp2"));
        assert!(!Mp2Decoder::accepts("mp3"));
        assert!(!Mp2Decoder::accepts("mp1"));
        assert!(!Mp2Decoder::accepts("MP2"));
        assert!(!Mp2Decoder::accepts(""));
    }

    fn packet_of(bytes: Vec<u8>, generation: GenerationId) -> Packet {
        Packet {
            track: media_types::TrackId::new(0),
            pts: Stamp::Missing,
            dts: Stamp::Missing,
            duration: None,
            data: SharedBytes::from_vec(bytes),
            keyframe: true,
            discontinuity: false,
            generation,
        }
    }

    struct Sink {
        frames: usize,
        samples: usize,
        rate: u32,
        channels: u32,
    }
    impl FrameSink for Sink {
        fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
            self.frames += 1;
            if let Frame::Audio(a) = frame {
                self.samples += a.samples.len() / 4;
                self.rate = a.format.rate;
                self.channels = a.format.channels;
            }
            Ok(())
        }
    }

    /// One MPEG-2 LSF Layer II 80 kbps 16 kHz mono frame shell: a
    /// valid header with a zero payload (the family's minimal-frame
    /// precedent — the engine decodes near-silence).
    fn lsf_frame() -> Vec<u8> {
        let mut f = Vec::new();
        f.extend_from_slice(&[0xFF, 0xF5, 0x98, 0xC0]);
        f.resize(720, 0);
        f
    }

    #[test]
    fn decode_before_configure_is_typed_bad_state() {
        let mut decoder = Mp2Decoder::new();
        let generation = match GenerationId::FIRST.next() {
            Ok(g) => g,
            Err(e) => unreachable!("{e:?}"),
        };
        let e = match decoder.decode(
            packet_of(Vec::new(), generation),
            &mut Sink {
                frames: 0,
                samples: 0,
                rate: 0,
                channels: 0,
            },
        ) {
            Err(e) => e,
            Ok(_) => unreachable!("must be BadState"),
        };
        assert_eq!(e.context.code, "mp2.not_configured");
        assert!(matches!(e.kind, media_types::ErrorKind::Decoder { .. }));
    }

    #[test]
    fn configure_rejects_foreign_descriptors_and_unexpected_facts() {
        let mut decoder = Mp2Decoder::new();
        let generation = match GenerationId::FIRST.next() {
            Ok(g) => g,
            Err(e) => unreachable!("{e:?}"),
        };
        let e = match decoder.configure(&DecoderConfig::new(
            match media_types::ids::CodecDescriptor::new("mp3") {
                Ok(d) => d,
                Err(e) => unreachable!("{e:?}"),
            },
            generation,
            ThreadBudget::default(),
            None,
            None,
        )) {
            Err(e) => e,
            Ok(()) => unreachable!("mp3 must be refused by the mp2 surface"),
        };
        assert_eq!(e.context.code, "mp2.codec_descriptor");
        let e = match decoder.configure(&DecoderConfig::new(
            match media_types::ids::CodecDescriptor::new("mp2") {
                Ok(d) => d,
                Err(e) => unreachable!("{e:?}"),
            },
            generation,
            ThreadBudget::default(),
            None,
            Some(SharedBytes::from_vec(vec![1, 2, 3])),
        )) {
            Err(e) => e,
            Ok(()) => unreachable!("a facts blob must be refused for Layer II"),
        };
        assert_eq!(e.context.code, "mp2.facts.unexpected");
    }

    #[test]
    fn a_minimal_synthetic_stream_decodes_end_to_end() {
        // One hand-built Layer II frame: header + zero payload. The
        // pipeline shape is pinned (1152 samples, 16 kHz, mono).
        let mut data = Vec::new();
        data.extend_from_slice(&lsf_frame());
        match decode_stream(&data) {
            Ok(d) => {
                assert_eq!(d.frames, 1);
                assert_eq!(d.format.channels, 1);
                assert_eq!(d.format.rate, 16_000);
                assert_eq!(d.samples.len(), 1152);
                assert_eq!(d.geometry.total_samples, 1152);
                assert_eq!(d.geometry.frame_size, 720);
            }
            Err(e) => unreachable!("a valid-shaped Layer II frame must decode: {e}"),
        }
    }

    #[test]
    fn a_layer_iii_stream_is_refused_not_misdecoded() {
        // The family's minimal Layer III frame through the mp2
        // pipeline: typed layer gate.
        let mut data = Vec::new();
        data.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
        data.resize(417, 0);
        let e = match decode_stream(&data) {
            Err(e) => e,
            Ok(_) => unreachable!("Layer III must be refused by the mp2 pipeline"),
        };
        assert_eq!(e.context.code, "mp2.stream.layer_mismatch");
    }

    #[test]
    fn a_truncated_frame_packet_is_a_typed_truncation() {
        let mut decoder = Mp2Decoder::new();
        let generation = match GenerationId::FIRST.next() {
            Ok(g) => g,
            Err(e) => unreachable!("{e:?}"),
        };
        match decoder.configure(&DecoderConfig::new(
            match media_types::ids::CodecDescriptor::new("mp2") {
                Ok(d) => d,
                Err(e) => unreachable!("{e:?}"),
            },
            generation,
            ThreadBudget::default(),
            None,
            None,
        )) {
            Ok(()) => {}
            Err(e) => unreachable!("mp2 configure must succeed: {e:?}"),
        }
        // A header declaring 720 bytes but carrying 100.
        let mut short = lsf_frame();
        short.truncate(100);
        let mut sink = Sink {
            frames: 0,
            samples: 0,
            rate: 0,
            channels: 0,
        };
        let e = match decoder.decode(packet_of(short, generation), &mut sink) {
            Err(e) => e,
            Ok(_) => unreachable!("a short packet must be a typed truncation"),
        };
        assert_eq!(e.context.code, "mp2.frame.truncated");
        assert!(matches!(
            e.kind,
            media_types::ErrorKind::Corrupt {
                detail: media_types::error::CorruptDetail::Truncated {
                    expected: 720,
                    found: 100
                }
            }
        ));
        // Per-frame isolation: the next, complete packet decodes.
        let mut sink = Sink {
            frames: 0,
            samples: 0,
            rate: 0,
            channels: 0,
        };
        match decoder.decode(packet_of(lsf_frame(), generation), &mut sink) {
            Ok(DecodeOutcome::Emitted { frames }) => assert_eq!(frames, 1),
            other => unreachable!("the frame after damage must decode: {other:?}"),
        }
        assert_eq!(sink.samples, 1152);
        assert_eq!(sink.rate, 16_000);
    }

    #[test]
    fn cbr_seek_plan_from_the_decoded_geometry_lands_on_frames() {
        use super::super::seek::Mp2SeekPlan;
        use super::super::seek::cbr_seek;

        // decode_stream's geometry + our seek planner compose: the
        // plan for the last frame's first sample is inside the file.
        let mut data = Vec::new();
        for _ in 0..3 {
            data.extend_from_slice(&lsf_frame());
        }
        let d = match decode_stream(&data) {
            Ok(d) => d,
            Err(e) => unreachable!("{e}"),
        };
        let plan: Mp2SeekPlan = cbr_seek(&d.geometry, 2 * 1152);
        assert_eq!(plan.byte_offset, 2 * 720);
        assert_eq!(plan.first_sample_at_offset, 2 * 1152);
    }
}
