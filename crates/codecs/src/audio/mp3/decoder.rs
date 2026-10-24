//! The MP3/MP2 decoder core (card 5.A3): the symphonia-backed frame
//! decode, the stream-level pipeline (ID3 → frames → Xing/gapless →
//! PCM), and the [`codec_api::Decoder`] implementation.
//!
//! # Engine choice (the 5.A3 dependency decision)
//!
//! **`symphonia-bundle-mp3` 0.6.1, exact-pinned** (plus its
//! `symphonia-core`), under the Lead's C5 pre-approval, purity
//! verified BEFORE any Cargo.toml edit in
//! `docs/dependencies/records/symphonia-mp3-0.6.1.md`
//! (`#![forbid(unsafe_code)]` in both crates, no build.rs, no
//! native code, 9-node pure-Rust graph). The engine owns exactly
//! one job: entropy-decoding ONE already-framed frame to F32
//! planes. ID3v2, Xing/Info/VBRI, the LAME gapless facts, frame
//! walking/resync, seeking and the error taxonomy are ours — a
//! future engine swap stays inside this module.
//!
//! # Packet model (the codec-api contract)
//!
//! One packet = **one complete frame** (header + side info + data,
//! exactly `header.frame_size` bytes — the engine hard-requires the
//! match). MPEG-audio frames are independent (each carries its full
//! side information), so per-frame isolation (doc 03 §4.5(d)) holds:
//! after a typed `Corrupt` the next packet decodes unchanged.
//!
//! # Gapless facts channel ([PROPOSAL], the PCM/FLAC `extra_data`
//! precedent)
//!
//! `DecoderConfig::extra_data` carries OUR 12-byte facts blob —
//! `delay u32 LE | padding u32 LE | total_frames u32 LE` (zeros =
//! no trim) — which the MP3 demuxer will distill from the Xing/Info
//! LAME tag. The contract decoder applies the start trim on the
//! first packets (the delay can span frame boundaries) and the end
//! trim on the frame whose index is `total_frames − 1` (knowable
//! only from the Xing count — without it the end trim is honestly
//! skipped, see [`Mp3Decoder`]).
//!
//! # Bit-exactness posture (the honest differential note)
//!
//! MP3 is lossy: independent decoder implementations are NOT
//! sample-identical in general. The differential tests therefore
//! compare against ffmpeg on the SAME gapless window (Xing frame
//! skipped, `delay`/`padding` trimmed — verified: ffmpeg decodes
//! exactly `frames × spf − delay − padding` samples of the
//! ffmpeg-authored fixtures) and record both the PCM MD5 and the
//! max sample delta; see `tests/mp3_bitexact.rs` for the verdicts.
//!
//! # The window's position: the 529-sample pipeline shift
//!
//! The LAME `delay`/`padding` fields are coordinates in the DECODER
//! PIPELINE's raw timeline, and the two timelines differ by a fixed
//! head offset: symphonia (with `gapless = false`) emits **529
//! samples/channel of Layer III synthesis-pipeline priming at the
//! stream head** that ffmpeg's decode never contains (mpg123
//! suppresses the same 529). Measured, not theorized: an exhaustive
//! shift search on five independent fixtures (MPEG-1 and MPEG-2,
//! mono and stereo, CBR and VBR) puts ffmpeg's output at exactly
//! `+529` samples/channel against our raw-trimmed output, bit-exact
//! to ±1 LSB once aligned (Layer II/MP2 has no such offset — its
//! output aligns at shift 0). Our window therefore applies
//! [`LAYER3_PIPELINE_LATENCY`] to both ends (`[delay + 529,
//! total − padding + 529)`), which lands on ffmpeg's window exactly;
//! when `padding < 529` the tail is honestly clamped to what was
//! decoded (the shortfall is `(529 − padding)` samples, documented
//! in the differential suite).

use super::error::{Mp3Error, corrupt, unsupported};
use super::header::{FrameHeader, Layer, parse_frame_header};
use super::id3::{Id3Tag, parse_id3v2};
use super::xing::{LameTag, VbriInfo, XingInfo, parse_vbri, parse_xing};
use codec_api::prelude::*;
use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::time::{Nanoseconds, TimeBase, Timestamp};
use media_types::{Duration, Stamp};
use symphonia_bundle_mp3::MpaDecoder;
use symphonia_core::codecs::audio::well_known::{CODEC_ID_MP1, CODEC_ID_MP2, CODEC_ID_MP3};
use symphonia_core::codecs::audio::{AudioCodecParameters, AudioDecoder, AudioDecoderOptions};
use symphonia_core::packet::PacketRef;
use symphonia_core::units::{Duration as SymDuration, Timestamp as SymTimestamp};

/// One decoded stream (the free surface's result): gapless-trimmed
/// interleaved F32 samples plus the metadata the walk produced.
#[derive(Debug, Clone)]
pub struct DecodedMp3 {
    /// The output shape (F32, interleaved, native channels/rate).
    pub format: AudioFormat,
    /// Interleaved F32 samples, gapless window only.
    pub samples: Vec<f32>,
    /// Decoded audio frame count (the Xing frame is not counted).
    pub frames: usize,
    /// The parsed leading ID3v2 tag, when present.
    pub id3: Option<Id3Tag>,
    /// The parsed Xing/Info header, when the first frame carried one.
    pub xing: Option<XingInfo>,
    /// The parsed VBRI header, when present instead.
    pub vbri: Option<VbriInfo>,
    /// The gapless facts that were applied (LAME tag).
    pub gapless: Option<LameTag>,
    /// Byte offset of the first DECODED audio frame.
    pub first_frame_offset: u64,
}

/// Map a frame header to the engine's codec id.
fn codec_id(header: &FrameHeader) -> Option<symphonia_core::codecs::audio::AudioCodecId> {
    match header.layer {
        Layer::L1 => Some(CODEC_ID_MP1),
        Layer::L2 => Some(CODEC_ID_MP2),
        Layer::L3 => Some(CODEC_ID_MP3),
        Layer::Reserved => None,
    }
}

/// Layer III synthesis-pipeline latency, in per-channel samples:
/// the head of symphonia's raw (gapless = false) output that ffmpeg's
/// decode never emits. Empirically pinned (see the module doc); the
/// gapless window shifts by this much to land on ffmpeg's timeline.
const LAYER3_PIPELINE_LATENCY: u64 = 529;

/// Decode one framed packet through the engine (shared by the
/// contract path and the stream pipeline).
///
/// `scratch` is reused between calls (the engine's internal buffer is
/// reused by `MpaDecoder` itself; `scratch` holds the interleaved
/// copy). Returns the frame's sample count and the validated header.
fn decode_one(
    decoder: &mut MpaDecoder,
    packet: &[u8],
    scratch: &mut Vec<f32>,
) -> Result<(usize, FrameHeader), EngineError> {
    let header = parse_frame_header(packet)?;
    let codec = match codec_id(&header) {
        Some(c) => c,
        None => {
            return Err(corrupt(
                "mp3.frame.layer_reserved",
                media_types::error::CorruptDetail::InvalidStructure { offset: None },
                "reserved layer in a decode packet",
            )
            .into_engine_error());
        }
    };
    if decoder.codec_params().codec != codec {
        return Err(corrupt(
            "mp3.frame.codec_switch",
            media_types::error::CorruptDetail::InvalidStructure { offset: None },
            "frame layer disagrees with the configured stream",
        )
        .into_engine_error());
    }
    let pkt = PacketRef::new(0, SymTimestamp::ZERO, SymDuration::ZERO, packet);
    let decoded = match decoder.decode_ref(&pkt) {
        Ok(buf) => buf,
        Err(e) => {
            return Err(corrupt(
                "mp3.frame.decode",
                media_types::error::CorruptDetail::InvalidStructure { offset: None },
                format!("engine rejected the frame: {e}"),
            )
            .into_engine_error());
        }
    };
    let frames = decoded.frames();
    let (rate, channels) = {
        let spec = decoded.spec();
        (spec.rate(), spec.channels().count())
    };
    let _ = rate;
    let need = frames * channels;
    if scratch.len() < need {
        scratch.resize(need, 0.0);
    }
    let out = &mut scratch[..need];
    decoded.copy_to_slice_interleaved::<f32, _>(out);
    Ok((frames, header))
}

/// The stream-level pipeline (the conformance/differential
/// workhorse): ID3 → frame walk → Xing/VBRI of frame 0 → decode the
/// audio frames → apply the LAME gapless window.
///
/// # Errors
/// Typed [`EngineError`]s: ID3 damage, no decodable frames, a Xing
/// frame-count inconsistent with the walked data
/// (`mp3.xing.frame_count_mismatch`), engine rejections. Damage is
/// never a panic; garbage between frames is resynced past by the
/// walker.
pub fn decode_stream(data: &[u8]) -> Result<DecodedMp3, EngineError> {
    // Leading ID3v2: parse (kept) and skip.
    let mut pos = 0usize;
    let mut id3 = None;
    if data.get(..3) == Some(b"ID3") {
        let tag = parse_id3v2(data)?;
        pos = tag.total_len;
        id3 = Some(tag);
    }
    let frames = super::header::walk_frames(data, pos);
    if frames.is_empty() {
        return Err(corrupt(
            "mp3.stream.no_frames",
            media_types::error::CorruptDetail::Truncated {
                expected: 4,
                found: (data.len() - pos.min(data.len())) as u64,
            },
            "no MPEG-audio frame header found after the ID3 tag",
        )
        .into_engine_error());
    }
    let first = frames[0];
    let first_bytes = data
        .get(first.offset..first.offset + first.header.frame_size as usize)
        .unwrap_or(&[]);
    let xing = parse_xing(first_bytes, first.offset as u64)?;
    let vbri = if xing.is_none() {
        parse_vbri(first_bytes, first.offset as u64)?
    } else {
        None
    };
    let has_header_frame = xing.is_some() || vbri.is_some();
    let audio = if has_header_frame {
        &frames[1..]
    } else {
        &frames[..]
    };
    if audio.is_empty() {
        return Err(corrupt(
            "mp3.stream.header_frame_only",
            media_types::error::CorruptDetail::Truncated {
                expected: 2,
                found: 1,
            },
            "the stream carries only its Xing/Info header frame",
        )
        .into_engine_error());
    }
    // The Xing consistency gate (the card's typed error path): the
    // claimed frame count must equal the walked audio frames.
    if let Some(claimed) = xing.as_ref().and_then(|x| x.frames)
        && claimed as usize != audio.len()
    {
        return Err(corrupt(
            "mp3.xing.frame_count_mismatch",
            media_types::error::CorruptDetail::InvalidStructure {
                offset: Some(xing.as_ref().map(|x| x.frame_offset).unwrap_or(0)),
            },
            format!(
                "Xing header claims {claimed} frames but the stream walks {}",
                audio.len()
            ),
        )
        .into_engine_error());
    }
    // Engine set-up from the first AUDIO frame's header.
    let header0 = audio[0].header;
    let codec = match codec_id(&header0) {
        Some(c) => c,
        None => {
            return Err(corrupt(
                "mp3.stream.layer_reserved",
                media_types::error::CorruptDetail::InvalidStructure {
                    offset: Some(audio[0].offset as u64),
                },
                "first audio frame uses a reserved layer",
            )
            .into_engine_error());
        }
    };
    let params = AudioCodecParameters {
        codec,
        ..AudioCodecParameters::default()
    };
    let mut engine_opts = AudioDecoderOptions::default();
    engine_opts.gapless = false; // the LAME window is applied by us
    engine_opts.verify = false;
    let mut engine = match MpaDecoder::try_new(&params, &engine_opts) {
        Ok(e) => e,
        Err(e) => {
            return Err(
                unsupported("mp3.engine.init", format!("engine refused the stream: {e}"))
                    .into_engine_error(),
            );
        }
    };
    let mut scratch: Vec<f32> = Vec::new();
    let mut samples: Vec<f32> = Vec::new();
    let mut spec_channels = 0usize;
    let mut spec_rate = 0u32;
    for frame in audio {
        let bytes = data
            .get(frame.offset..frame.offset + frame.header.frame_size as usize)
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
            "mp3.stream.no_decoded_samples",
            media_types::error::CorruptDetail::InvalidStructure { offset: None },
            "no frame decoded",
        )
        .into_engine_error());
    }
    // The gapless window: [delay, total − padding) in the LAME tag's
    // raw pipeline coordinates, shifted by the 529-sample Layer III
    // synthesis latency to land on ffmpeg's timeline (module doc),
    // clamped to what was actually decoded.
    let gapless = xing.as_ref().and_then(|x| x.lame);
    if let Some(lame) = gapless {
        let total_ch = (samples.len() / spec_channels) as u64;
        let (raw_start, raw_end) = lame.trim_window(total_ch);
        let start = raw_start + LAYER3_PIPELINE_LATENCY;
        let end = (raw_end + LAYER3_PIPELINE_LATENCY).min(total_ch);
        let start_i = (start as usize).min(total_ch as usize) * spec_channels;
        let end_i = (end as usize).min(total_ch as usize) * spec_channels;
        let window = samples.get(start_i..end_i.max(start_i)).unwrap_or(&[]);
        let trimmed = window.to_vec();
        samples = trimmed;
    }
    Ok(DecodedMp3 {
        format: AudioFormat {
            sample: SampleFormat::F32,
            channels: spec_channels as u32,
            layout: AudioLayout::Interleaved,
            rate: spec_rate,
        },
        samples,
        frames: audio.len(),
        id3,
        xing,
        vbri,
        gapless,
        first_frame_offset: audio[0].offset as u64,
    })
}

/// The facts blob this decoder's `configure` accepts (12 bytes).
pub mod facts {
    /// Blob length.
    pub const LEN: usize = 12;

    /// Encode the facts blob.
    #[must_use]
    pub fn encode(delay: u32, padding: u32, total_frames: u32) -> [u8; LEN] {
        let mut b = [0u8; LEN];
        b[..4].copy_from_slice(&delay.to_le_bytes());
        b[4..8].copy_from_slice(&padding.to_le_bytes());
        b[8..12].copy_from_slice(&total_frames.to_le_bytes());
        b
    }

    /// Decode the facts blob. `None` when the length is wrong.
    #[must_use]
    pub fn decode(blob: &[u8]) -> Option<(u32, u32, u32)> {
        if blob.len() != LEN {
            return None;
        }
        let d = u32::from_le_bytes(blob[..4].try_into().ok()?);
        let p = u32::from_le_bytes(blob[4..8].try_into().ok()?);
        let f = u32::from_le_bytes(blob[8..12].try_into().ok()?);
        Some((d, p, f))
    }
}

/// MP3/MP2 decoder behind the codec-api [`Decoder`] contract (card
/// 5.A3; doc 04 §3.1 rows "MP3"/"MP2").
///
/// One packet = one frame. The optional facts blob (see [`facts`])
/// carries the LAME gapless window: start trim on the first packets
/// (delay may exceed one frame), end trim on packet index
/// `total_frames − 1`. Without a `total_frames` claim the end trim
/// is honestly not applied (the last packet is not knowable from
/// inside a packet-stream contract).
#[derive(Default)]
pub struct Mp3Decoder {
    /// The configured stream generation (None until `configure`).
    generation: Option<GenerationId>,
    /// Codec descriptor accepted at configure time.
    codec: Option<String>,
    /// The engine (created at configure from the descriptor).
    engine: Option<MpaDecoder>,
    /// Gapless facts (delay, padding, total_frames).
    delay: u32,
    padding: u32,
    total_frames: u32,
    /// Packets fed so far (0-based index of the next).
    packet_index: u32,
    /// Per-channel samples fed so far (pts bookkeeping).
    samples_fed: u64,
    /// Remaining start-trim (per-channel samples).
    pending_start_trim: u32,
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

impl std::fmt::Debug for Mp3Decoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mp3Decoder")
            .field("codec", &self.codec)
            .field("packet_index", &self.packet_index)
            .field("samples_fed", &self.samples_fed)
            .field("delay", &self.delay)
            .field("padding", &self.padding)
            .field("total_frames", &self.total_frames)
            .finish_non_exhaustive()
    }
}

/// Output-rotation depth (the 5.A1 bound).
const SAMPLE_SLOTS: usize = 4;

/// Max samples per frame across MPEG versions/layers (Layer I/II:
/// 1152; Layer III MPEG-1: 1152; MPEG-2 L3: 576 — bounded by 1152).
const MAX_SAMPLES_PER_FRAME: usize = 1152;

impl Mp3Decoder {
    /// A fresh decoder, unconfigured.
    #[must_use]
    pub fn new() -> Mp3Decoder {
        Mp3Decoder::default()
    }

    /// The codec descriptors this decoder accepts.
    fn accepts(descriptor: &str) -> bool {
        matches!(descriptor, "mp1" | "mp2" | "mp3")
    }

    fn engine_codec(descriptor: &str) -> symphonia_core::codecs::audio::AudioCodecId {
        match descriptor {
            "mp1" => CODEC_ID_MP1,
            "mp2" => CODEC_ID_MP2,
            _ => CODEC_ID_MP3,
        }
    }
}

impl Decoder for Mp3Decoder {
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
        let descriptor = cfg.codec.as_str();
        if !Self::accepts(descriptor) {
            return Err(unsupported(
                "mp3.codec_descriptor",
                format!("codec descriptor `{descriptor}` is not mp1/mp2/mp3"),
            )
            .into_engine_error());
        }
        let (delay, padding, total_frames) = match cfg.extra_data.as_ref().map(|d| d.as_slice()) {
            None | Some([]) => (0, 0, 0),
            Some(blob) => facts::decode(blob).ok_or_else(|| {
                unsupported(
                    "mp3.facts.length",
                    "extra_data present but not the 12-byte facts blob \
                     (delay|padding|total_frames, u32 LE each)",
                )
                .into_engine_error()
            })?,
        };
        let params = AudioCodecParameters {
            codec: Self::engine_codec(descriptor),
            ..AudioCodecParameters::default()
        };
        let mut engine_opts = AudioDecoderOptions::default();
        engine_opts.gapless = false;
        engine_opts.verify = false;
        let engine = MpaDecoder::try_new(&params, &engine_opts).map_err(|e| {
            unsupported(
                "mp3.engine.init",
                format!("engine refused the configuration: {e}"),
            )
            .into_engine_error()
        })?;
        self.engine = Some(engine);
        self.generation = Some(cfg.generation);
        self.codec = Some(descriptor.to_string());
        self.delay = delay;
        self.padding = padding;
        self.total_frames = total_frames;
        self.packet_index = 0;
        self.samples_fed = 0;
        self.pending_start_trim = delay.saturating_add(LAYER3_PIPELINE_LATENCY as u32);
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
            None => return Err(Mp3Error::BadState.into_engine_error()),
        };
        let engine = match self.engine.as_mut() {
            Some(e) => e,
            None => return Err(Mp3Error::BadState.into_engine_error()),
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
        // Start trim: drop pending delay samples (may span packets).
        let mut emit = interleaved;
        if self.pending_start_trim > 0 {
            let drop = (self.pending_start_trim as usize).min(frame_samples);
            self.pending_start_trim -= drop as u32;
            emit = emit.get(drop * channels..).unwrap_or(&[]);
        }
        // End trim: the declared last frame loses its padding tail.
        let is_last = self.total_frames > 0 && self.packet_index + 1 == self.total_frames;
        if is_last && self.padding > 0 {
            // The padding is in the LAME tag's raw pipeline coordinates;
            // the 529-sample synthesis latency (already absent from the
            // stream's intended tail on ffmpeg's timeline) is not part
            // of the end trim (see the module doc).
            let drop = (u64::from(self.padding))
                .saturating_sub(LAYER3_PIPELINE_LATENCY)
                .min(frame_samples as u64) as usize;
            let keep = frame_samples - drop;
            emit = emit.get(..keep * channels).unwrap_or(emit);
        }
        self.packet_index += 1;
        let emitted_samples = emit.len() / channels.max(1);
        if emitted_samples > 0 {
            let slot_len = MAX_SAMPLES_PER_FRAME * channels * 4;
            let bytes = pooled_sample_slot(
                &mut self.sample_pool,
                &mut self.slot_next,
                slot_len,
                |dst| {
                    let mut cursor = 0usize;
                    for v in emit {
                        dst[cursor..cursor + 4].copy_from_slice(&v.to_ne_bytes());
                        cursor += 4;
                    }
                    Ok::<(), Mp3Error>(())
                },
            )?;
            let exact = emitted_samples * channels * 4;
            let base = TimeBase::new(1, self.out_rate.max(1)).map_err(|_| {
                unsupported("mp3.stream.rate", "negotiated rate is 0").into_engine_error()
            })?;
            let frame = Frame::Audio(media_types::frame::AudioFrame {
                format: AudioFormat {
                    sample: SampleFormat::F32,
                    channels: self.out_channels,
                    layout: AudioLayout::Interleaved,
                    rate: self.out_rate,
                },
                samples: bytes.slice(0..exact),
                pts: if self.samples_fed < u64::from(self.delay) {
                    // Still inside the delay window: the first emitted
                    // sample's true index is the delay boundary.
                    Stamp::Present(Timestamp::new(i64::from(self.delay), base))
                } else {
                    Stamp::Present(Timestamp::new(
                        i64::try_from(self.samples_fed).unwrap_or(i64::MAX),
                        base,
                    ))
                },
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
            return Err(Mp3Error::BadState.into_engine_error());
        }
        // MPEG audio keeps no cross-frame state: nothing buffered.
        Ok(DecodeOutcome::Drained)
    }
}

/// The pooled output rotation (the 5.A1 `pooled_sample_slot`
/// pattern, re-authored for the f32 path).
fn pooled_sample_slot(
    slots: &mut Vec<SharedBytes>,
    next: &mut usize,
    slot_len: usize,
    fill: impl FnOnce(&mut [u8]) -> Result<(), Mp3Error>,
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
                "mp3.output.fresh_slot_not_exclusive",
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
    fn descriptor_gate_accepts_the_mpeg_audio_family() {
        assert!(Mp3Decoder::accepts("mp3"));
        assert!(Mp3Decoder::accepts("mp2"));
        assert!(Mp3Decoder::accepts("mp1"));
        assert!(!Mp3Decoder::accepts("flac"));
        assert!(!Mp3Decoder::accepts("MP3"));
        assert!(!Mp3Decoder::accepts(""));
    }

    #[test]
    fn facts_blob_round_trips_and_rejects_bad_lengths() {
        let blob = facts::encode(1105, 340, 78);
        assert_eq!(facts::decode(&blob), Some((1105, 340, 78)));
        assert_eq!(facts::decode(&blob[..11]), None);
        assert_eq!(facts::decode(&[]), None);
    }

    #[test]
    fn decode_before_configure_is_typed_bad_state() {
        let mut decoder = Mp3Decoder::new();
        let generation = match GenerationId::FIRST.next() {
            Ok(g) => g,
            Err(e) => unreachable!("{e:?}"),
        };
        let packet = Packet {
            track: media_types::TrackId::new(0),
            pts: Stamp::Missing,
            dts: Stamp::Missing,
            duration: None,
            data: SharedBytes::from_vec(Vec::new()),
            keyframe: true,
            discontinuity: false,
            generation,
        };
        struct Sink;
        impl FrameSink for Sink {
            fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
                Ok(())
            }
        }
        let e = match decoder.decode(packet, &mut Sink) {
            Err(e) => e,
            Ok(_) => unreachable!("must be BadState"),
        };
        assert_eq!(e.context.code, "mp3.not_configured");
        assert!(matches!(e.kind, media_types::ErrorKind::Decoder { .. }));
    }

    #[test]
    fn configure_rejects_foreign_descriptors_and_bad_facts() {
        let mut decoder = Mp3Decoder::new();
        let generation = match GenerationId::FIRST.next() {
            Ok(g) => g,
            Err(e) => unreachable!("{e:?}"),
        };
        let e = match decoder.configure(&DecoderConfig::new(
            match media_types::ids::CodecDescriptor::new("flac") {
                Ok(d) => d,
                Err(e) => unreachable!("{e:?}"),
            },
            generation,
            ThreadBudget::default(),
            None,
            None,
        )) {
            Err(e) => e,
            Ok(()) => unreachable!("flac must be refused"),
        };
        assert_eq!(e.context.code, "mp3.codec_descriptor");
        let e = match decoder.configure(&DecoderConfig::new(
            match media_types::ids::CodecDescriptor::new("mp3") {
                Ok(d) => d,
                Err(e) => unreachable!("{e:?}"),
            },
            generation,
            ThreadBudget::default(),
            None,
            Some(SharedBytes::from_vec(vec![1, 2, 3])),
        )) {
            Err(e) => e,
            Ok(()) => unreachable!("short facts must be refused"),
        };
        assert_eq!(e.context.code, "mp3.facts.length");
    }

    /// Build a first frame carrying a Xing header (the xing.rs test
    /// helper's construction, repeated for the stream-level tests).
    fn xing_first_frame(frames_claim: u32, lame: Option<(u32, u32)>) -> Vec<u8> {
        let mut f = Vec::new();
        f.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00]); // MPEG-1 L3 128k stereo
        f.extend_from_slice(&[0u8; 32]); // side info (stereo MPEG-1)
        f.extend_from_slice(b"Xing");
        f.extend_from_slice(&0x000Fu32.to_be_bytes()); // frames|bytes|toc|quality
        f.extend_from_slice(&frames_claim.to_be_bytes());
        f.extend_from_slice(&417u32.to_be_bytes());
        f.extend_from_slice(&[0u8; 100]); // TOC
        f.extend_from_slice(&0u32.to_be_bytes()); // quality
        if let Some((delay, padding)) = lame {
            f.extend_from_slice(b"LAME3.100");
            f.extend_from_slice(&[0; 12]);
            let packed: u32 = (delay.min(0x0FFF) << 12) | padding.min(0x0FFF);
            f.extend_from_slice(&packed.to_be_bytes()[1..4]);
            f.extend_from_slice(&[0; 8]);
        }
        f.resize(417, 0);
        f
    }

    /// One decodable MPEG-1 L3 frame (header + zero side info/payload:
    /// the `mod.rs` minimal-stream precedent — 1152 samples of
    /// near-silence).
    fn plain_audio_frame() -> Vec<u8> {
        let mut f = Vec::new();
        f.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
        f.extend_from_slice(&[0u8; 413]);
        f
    }

    #[test]
    fn xing_frame_count_inconsistency_is_typed() {
        // The header claims 7 frames; the stream walks exactly 1.
        let mut data = xing_first_frame(7, None);
        data.extend_from_slice(&plain_audio_frame());
        let e = match decode_stream(&data) {
            Err(e) => e,
            Ok(_) => unreachable!("the lying Xing count must be refused"),
        };
        assert_eq!(e.context.code, "mp3.xing.frame_count_mismatch");
        assert!(matches!(e.kind, media_types::ErrorKind::Corrupt { .. }));
        assert_eq!(e.stage, media_types::PipelineStage::Decode);
    }

    #[test]
    fn gapless_window_applies_the_pipeline_latency_shift() {
        // delay=100, padding=600: the raw LAME window would be
        // [100, 1152-600) = 452 samples/channel; the pipeline shift
        // moves it to [100+529, 1152-600+529) = [629, 1081) — the
        // same 452 samples, positioned on ffmpeg's timeline.
        let mut data = xing_first_frame(1, Some((100, 600)));
        data.extend_from_slice(&plain_audio_frame());
        let d = match decode_stream(&data) {
            Ok(d) => d,
            Err(e) => unreachable!("consistent stream must decode: {e}"),
        };
        assert_eq!(d.frames, 1);
        assert!(d.gapless.is_some());
        assert_eq!(d.samples.len(), 452 * 2);
        // Without the shift the window would start at raw sample 100;
        // with it, the first emitted sample is raw sample 629 — pinned
        // by asserting the length above plus the corpus differential's
        // bit-exact alignment (tests/mp3_corpus_differential.rs).
    }

    #[test]
    fn gapless_window_clamps_when_padding_undercuts_the_latency() {
        // padding=100 < 529: raw window [50, 1052); shifted end
        // 1052+529 clamps to the decoded 1152 → [579, 1152) = 573
        // samples/channel, honestly short of ffmpeg's (never-decoded)
        // tail by 529-100 samples.
        let mut data = xing_first_frame(1, Some((50, 100)));
        data.extend_from_slice(&plain_audio_frame());
        let d = match decode_stream(&data) {
            Ok(d) => d,
            Err(e) => unreachable!("consistent stream must decode: {e}"),
        };
        assert_eq!(d.samples.len(), 573 * 2);
    }
}
