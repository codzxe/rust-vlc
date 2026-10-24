//! The FLAC decoder core (card 5.A1): the claxon-backed frame decode,
//! the bit-depth normalization, and the [`codec_api::Decoder`]
//! implementation.
//!
//! # Engine choice (the 5.A1 dependency decision)
//!
//! The card offered `symphonia-claxon` or plain `claxon`; the decision
//! is **`claxon` 0.4.3, exact-pinned** — the card's stated preference
//! (tiny, zero transitive deps) — under the Lead's C5 conditional
//! approval, purity-verified BEFORE any Cargo.toml edit in
//! `docs/dependencies/records/claxon-0.4.3.md` (F1–F7 all NO, no
//! normal dependencies, Apache-2.0, sha256-matched provenance, the
//! historical RUSTSEC-2018-0004 advisory is outside the pinned range).
//! claxon owns exactly one job here: entropy-coded **frame decode**
//! (header + subframes + CRC-16 verification, `i32` samples). All
//! metadata (STREAMINFO, SEEKTABLE), seeking, error taxonomy, and the
//! contract surface are ours — a future engine swap (e.g.
//! `symphonia-bundle-flac`) stays inside this module.
//!
//! # Packet model (the codec-api contract)
//!
//! One packet = **one FLAC frame** (self-delimiting: header + subframes
//! plus CRC-16). The decoder is stateless between packets — every frame
//! restarts prediction from the frame header — so per-frame isolation
//! (doc 03 §4.5(d)) holds trivially, as in MJPEG: after a typed
//! `Corrupt` error the next packet decodes unchanged. Bounded work: one
//! claxon `read_next_or_eof` call consumes at most the frame in the
//! packet (trailing bytes, if a demuxer over-read, are not touched).
//!
//! # Facts channel ([PROPOSAL], the PCM `extra_data` precedent)
//!
//! [`DecoderConfig::extra_data`](codec_api::DecoderConfig) carries the
//! 34-byte STREAMINFO body verbatim — the container's own out-of-band
//! facts ([`super::metadata::parse_streaminfo`] validates them). FLAC
//! frame headers may omit rate/width ("get from STREAMINFO"), so
//! configure without facts is a typed `Unsupported`
//! (`flac.streaminfo.missing`), never a guess.
//!
//! # Output format (codec-native, bit-exact against the reference)
//!
//! FLAC samples are signed integers of `bits_per_sample` significant
//! bits. The decoder left-justifies into the smallest media-types
//! width that holds the stream — ≤16 → [`SampleFormat::S16`],
//! 17–24 → [`SampleFormat::S24`], 25–32 → [`SampleFormat::S32`] —
//! shifting by `width − bits_per_sample` (lossless: the source bits
//! land in the high bits, matching the reference decoder's own
//! convention; pinned bit-exactly by the ffmpeg differential tests in
//! `tests/flac_bitexact.rs`). Output is interleaved native-endian PCM
//! (`AudioLayout::Interleaved`); no rate or channel conversion.
//!
//! # Allocation discipline (D-1)
//!
//! The claxon decode buffer is a reused `Vec<i32>`
//! (`read_next_or_eof` → `Block::into_buffer` round-trip); the output
//! bytes land in a pooled [`SharedBytes`] rotation sized to the
//! stream's maximum block (the MJPEG `pooled_frame_slot` pattern,
//! `SharedBytes::slice` cutting the exact frame view) — steady-state
//! decodes refill a slot in place, zero per-frame allocation after
//! warm-up.

use super::error::{FlacError, corrupt, unsupported};
use super::metadata::StreamInfo;
use codec_api::prelude::*;
use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::time::{Nanoseconds, TimeBase, Timestamp};
use media_types::{Duration, Stamp};

/// One decoded FLAC block, format-independent form (the free
/// surface's result; the contract path lifts the same bytes into
/// `Frame::Audio`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedFlac {
    /// Inter-channel sample number of the block's first sample.
    pub first_sample: u64,
    /// Block size (inter-channel samples in this block).
    pub block_size: u32,
    /// The output shape (native-endian, interleaved).
    pub format: AudioFormat,
    /// Interleaved PCM bytes, width per [`DecodedFlac::format`].
    pub samples: Vec<u8>,
}

/// Decode one FLAC frame (the free entry point: the whole byte-level
/// pipeline with no contract state — the `decode_image` pattern of
/// 2.B4, and the first surface of the `fuzz_codec_flac` harness).
///
/// # Errors
/// Typed [`EngineError`]s from the decode stage: `Unsupported`
/// (feature outside claxon's implemented set / bad facts),
/// `Corrupt` (structural damage, truncation, CRC mismatch). Damage is
/// **per-packet**: the next packet decodes independently.
pub fn decode_frame(packet: &[u8], facts: &StreamInfo) -> Result<DecodedFlac, EngineError> {
    let mut scratch = Vec::new();
    let mut out = vec![0u8; max_output_len(facts)];
    let meta = decode_into(packet, facts, &mut scratch, &mut out)?;
    out.truncate(meta.block_size as usize * facts.channels as usize * bytes_per(meta.format));
    Ok(DecodedFlac {
        first_sample: meta.first_sample,
        block_size: meta.block_size,
        format: meta.format,
        samples: out,
    })
}

/// The metadata part of one decoded block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BlockMeta {
    /// First inter-channel sample number (frame header time).
    pub first_sample: u64,
    /// Block size (inter-channel samples).
    pub block_size: u32,
    /// The negotiated output shape.
    pub format: AudioFormat,
}

/// Bytes per output sample of a negotiated format (local helper: the
/// block-size arithmetic needs the width next to the format).
fn bytes_per(format: AudioFormat) -> usize {
    format.sample.bytes_per_sample() as usize
}

/// The output-slot size a stream can ever need: max block × channels
/// × width (the pooled rotation is sized to this once at configure).
pub(crate) fn max_output_len(facts: &StreamInfo) -> usize {
    let (sample, _) = output_sample_format(facts.bits_per_sample);
    facts.max_block_size as usize * facts.channels as usize * sample.bytes_per_sample() as usize
}

/// The output width for a stream's bit depth (module-doc mapping):
/// smallest media-types sample format that holds
/// `bits_per_sample` left-justified, with the left-justify shift.
pub(crate) fn output_sample_format(bits_per_sample: u32) -> (SampleFormat, u32) {
    match bits_per_sample {
        0..=16 => (SampleFormat::S16, 16 - bits_per_sample.min(16)),
        17..=24 => (SampleFormat::S24, 24 - bits_per_sample),
        _ => (SampleFormat::S32, 32 - bits_per_sample.min(32)),
    }
}

/// Decode one frame from `packet` into `dst` (the reusable-fill form
/// behind both surfaces).
///
/// `scratch` is the claxon decode buffer, returned through
/// `Block::into_buffer` for reuse; `dst` must hold
/// [`max_output_len`] bytes (the pooled-slot caller pre-sizes it from
/// the same facts; a short slot is a caller bug and a typed error,
/// never a partial write). The actual block's bytes are written at
/// the front of `dst`; the metadata result carries the exact shape.
pub(crate) fn decode_into(
    packet: &[u8],
    facts: &StreamInfo,
    scratch: &mut Vec<i32>,
    dst: &mut [u8],
) -> Result<BlockMeta, FlacError> {
    if packet.is_empty() {
        // The contract path answers `NeedMoreInput` for empty packets
        // before reaching here (the PCM convention); the free surface
        // treats frame-less bytes as structure damage.
        return Err(corrupt(
            "flac.frame.empty",
            media_types::error::CorruptDetail::InvalidStructure { offset: None },
            "packet carries no frame bytes",
        ));
    }
    let taken = std::mem::take(scratch);
    let mut reader = claxon::frame::FrameReader::new(std::io::Cursor::new(packet));
    let decoded = match reader.read_next_or_eof(taken) {
        Ok(Some(block)) => block,
        Ok(None) => {
            // Only reachable for empty input (handled above); defensive.
            return Err(corrupt(
                "flac.frame.absent",
                media_types::error::CorruptDetail::InvalidStructure { offset: None },
                "no frame header in the packet",
            ));
        }
        Err(claxon::Error::IoError(e)) => {
            // A packet is a complete frame: mid-frame EOF is
            // truncation; anything else is structure damage.
            let truncated = e.kind() == std::io::ErrorKind::UnexpectedEof;
            return Err(corrupt(
                "flac.frame.truncated",
                if truncated {
                    media_types::error::CorruptDetail::Truncated {
                        expected: u64::from(facts.max_frame_size.unwrap_or(u32::MAX)),
                        found: packet.len() as u64,
                    }
                } else {
                    media_types::error::CorruptDetail::InvalidStructure { offset: None }
                },
                format!("frame read failed: {e}"),
            ));
        }
        Err(claxon::Error::FormatError(reason)) => {
            // claxon verifies CRC-8 (header) and CRC-16 (frame);
            // surface that as the structured checksum class.
            let is_crc = reason.contains("CRC");
            return Err(corrupt(
                if is_crc {
                    "flac.frame.crc"
                } else {
                    "flac.frame.structure"
                },
                if is_crc {
                    media_types::error::CorruptDetail::ChecksumMismatch
                } else {
                    media_types::error::CorruptDetail::InvalidStructure { offset: None }
                },
                format!("ill-formed FLAC frame: {reason}"),
            ));
        }
        Err(claxon::Error::Unsupported(feature)) => {
            return Err(unsupported(
                "flac.frame.feature",
                format!("frame uses a feature the engine does not implement: {feature}"),
            ));
        }
    };
    // Scalar facts first, then the interleaver's borrow, and only
    // then reclaim the decode buffer (D-1: `into_buffer` hands the
    // `Vec<i32>` back for the next call).
    let block_size = decoded.duration();
    let channels = decoded.channels();
    // The channel-count gate comes BEFORE any plane access: claxon's
    // `Block::channel` panics on `ch >= channels()`, so a frame that
    // disagrees with STREAMINFO must be rejected as typed damage
    // before `interleave_block` can index a missing plane (the
    // fuzz-survival suite pins this — a mutated facts blob plus a
    // real frame is the regression case).
    if channels != facts.channels {
        return Err(corrupt(
            "flac.frame.channels",
            media_types::error::CorruptDetail::InvalidStructure { offset: None },
            format!(
                "frame declares {channels} channel(s), streaminfo declares {}",
                facts.channels
            ),
        ));
    }
    // First sample: prefer our own header parse's spec conversion —
    // in a fixed-blocksize stream it is `frame number × the stream's
    // block size` (ffmpeg's packet pts and libFLAC agree), while
    // claxon's `Block::time()` multiplies by the frame's own size and
    // misplaces every shorter *tail* frame (observed: 47×4608=216 576
    // true vs claxon's 47×3924=184 428 on the 5 s fixture). If our
    // parse cannot explain the packet (exotic-but-decodable header),
    // claxon's value is the honest fallback.
    let first_sample = match super::seek::SeekPlanner::parse_header_at(packet, 0) {
        Some(header) => header.first_sample(facts),
        None => decoded.time(),
    };
    let (sample, shift) = output_sample_format(facts.bits_per_sample);
    let width = sample.bytes_per_sample() as usize;
    let output_len = block_size as usize * facts.channels as usize * width;
    if dst.len() < output_len {
        return Err(corrupt(
            "flac.output.slot_size",
            media_types::error::CorruptDetail::InvalidStructure { offset: None },
            format!(
                "output slot is {} bytes, block needs {output_len}",
                dst.len()
            ),
        ));
    }
    interleave_block(&decoded, block_size, facts.channels, shift, width, dst);
    *scratch = decoded.into_buffer();

    Ok(BlockMeta {
        first_sample,
        block_size,
        format: AudioFormat {
            sample,
            channels: facts.channels,
            layout: AudioLayout::Interleaved,
            rate: facts.sample_rate,
        },
    })
}

/// Interleave one claxon block's channels into the front of `dst`
/// with the left-justify shift and native-endian width encoding.
///
/// The caller validated `dst.len() >=` the block's byte length; the
/// per-channel reads are slice-bounded (`get`), so no input value can
/// panic this function.
fn interleave_block(
    block: &claxon::Block,
    block_size: u32,
    channels: u32,
    shift: u32,
    width: usize,
    dst: &mut [u8],
) {
    let mut cursor = 0usize;
    for i in 0..block_size as usize {
        for ch in 0..channels as usize {
            let plane = block.channel(ch as u32);
            let value: i32 = plane
                .get(i)
                .copied()
                .unwrap_or(0)
                .checked_shl(shift)
                .unwrap_or(0);
            match width {
                2 => {
                    let bytes = (value as i16).to_ne_bytes();
                    dst[cursor..cursor + 2].copy_from_slice(&bytes);
                    cursor += 2;
                }
                3 => {
                    write_i24(&mut dst[cursor..cursor + 3], value);
                    cursor += 3;
                }
                _ => {
                    let bytes = value.to_ne_bytes();
                    dst[cursor..cursor + 4].copy_from_slice(&bytes);
                    cursor += 4;
                }
            }
        }
    }
}

/// Write the low 24 bits of `value` in **native** byte order (the
/// media-types `S24` semantics): most-significant byte first on a
/// big-endian host, least-significant first on little-endian.
fn write_i24(dst: &mut [u8], value: i32) {
    let low3 = (value as u32) & 0x00ff_ffff;
    if cfg!(target_endian = "big") {
        dst[0] = (low3 >> 16) as u8;
        dst[1] = (low3 >> 8) as u8;
        dst[2] = low3 as u8;
    } else {
        dst[0] = low3 as u8;
        dst[1] = (low3 >> 8) as u8;
        dst[2] = (low3 >> 16) as u8;
    }
}

/// FLAC decoder behind the codec-api [`Decoder`] contract (card 5.A1;
/// doc 04 §3.1 row "FLAC").
///
/// Stateless between packets (see the module docs): `configure`
/// installs the generation + STREAMINFO facts, `decode` turns one
/// frame packet into one [`Frame::Audio`], `flush` is immediately
/// drained.
///
/// # Example
///
/// ```
/// use codec_api::prelude::*;
/// use codecs::audio::flac::FlacDecoder;
/// use media_types::{GenerationId, Stamp, TrackId, packet::SharedBytes};
///
/// # fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let mut decoder = FlacDecoder::new();
/// let generation = GenerationId::FIRST.next()?;
/// // Without STREAMINFO facts, configure is a typed rejection —
/// // FLAC frame headers may omit the geometry, so guessing is
/// // never allowed.
/// let err = decoder
///     .configure(&DecoderConfig::new(
///         media_types::ids::CodecDescriptor::new("flac")?,
///         generation,
///         ThreadBudget::default(),
///         None,
///         None,
///     ))
///     .expect_err("facts are mandatory");
/// assert_eq!(err.context.code, "flac.streaminfo.missing");
///
/// // With facts (a 34-byte STREAMINFO body), the decoder answers
/// // `decode`/`flush` contractually — an empty packet is honestly
/// // "nothing to emit".
/// let mut facts = [0u8; 34];
/// facts[0..2].copy_from_slice(&4096u16.to_be_bytes());
/// facts[2..4].copy_from_slice(&4096u16.to_be_bytes());
/// let packed: u64 = (44_100u64 << 44) | (1 << 41) | (15 << 36) | 44_100;
/// facts[10..18].copy_from_slice(&packed.to_be_bytes());
/// decoder.configure(&DecoderConfig::new(
///     media_types::ids::CodecDescriptor::new("flac")?,
///     generation,
///     ThreadBudget::default(),
///     None,
///     Some(SharedBytes::from_vec(facts.to_vec())),
/// ))?;
/// let packet = Packet {
///     track: TrackId::new(0),
///     pts: Stamp::Missing,
///     dts: Stamp::Missing,
///     duration: None,
///     data: SharedBytes::from_vec(Vec::new()),
///     keyframe: true,
///     discontinuity: false,
///     generation,
/// };
/// struct Sink;
/// impl FrameSink for Sink {
///     fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
///         Ok(())
///     }
/// }
/// let outcome = decoder.decode(packet, &mut Sink)?;
/// assert_eq!(outcome, DecodeOutcome::NeedMoreInput);
/// # Ok(())
/// # }
/// # run().unwrap_or_else(|e| unreachable!("{e}"));
/// ```
#[derive(Debug, Default)]
pub struct FlacDecoder {
    /// The validated STREAMINFO facts (None until `configure`).
    facts: Option<StreamInfo>,
    /// The configured stream generation (None until `configure`).
    generation: Option<GenerationId>,
    /// Codec descriptor accepted at configure time (diagnostics).
    codec: Option<String>,
    /// Reusable claxon decode buffer (D-1: handed to
    /// `read_next_or_eof`, taken back via `into_buffer`).
    block_scratch: Vec<i32>,
    /// Pooled output slots, each sized to the stream maximum (the
    /// MJPEG `FRAME_SLOTS` rotation; `SharedBytes::slice` cuts the
    /// exact per-block view).
    sample_pool: Vec<SharedBytes>,
    /// Round-robin cursor into `sample_pool`.
    slot_next: usize,
}

/// Output-rotation depth (the MJPEG `FRAME_SLOTS` bound).
const SAMPLE_SLOTS: usize = 4;

impl FlacDecoder {
    /// A fresh decoder, unconfigured.
    #[must_use]
    pub fn new() -> FlacDecoder {
        FlacDecoder::default()
    }

    /// The codec descriptors this decoder accepts (`flac`; the
    /// registry vocabulary of doc 04 §3.1).
    fn accepts(descriptor: &str) -> bool {
        descriptor == "flac"
    }
}

#[cfg(test)]
mod contract_tests {
    //! The channel-mismatch panic guard (the regression test for the
    //! fuzz-found class): a facts blob declaring MORE channels than
    //! the frame carries must be a typed `Corrupt`, never a panic in
    //! `Block::channel` (the check now runs before any plane access).
    use super::*;

    #[test]
    fn channel_count_mismatch_is_typed_not_panic() {
        // A facts blob declaring 3 channels…
        let mut body = [0u8; 34];
        body[0..2].copy_from_slice(&4096u16.to_be_bytes());
        body[2..4].copy_from_slice(&4096u16.to_be_bytes());
        let packed: u64 = (44_100u64 << 44) | (2u64 << 41) | (15u64 << 36);
        body[10..18].copy_from_slice(&packed.to_be_bytes());
        let facts = match super::super::metadata::parse_streaminfo_inner(&body) {
            Ok(f) => f,
            Err(e) => unreachable!("{e:?}"),
        };
        assert_eq!(facts.channels, 3);
        // …and any 2-channel-looking frame bytes: the decode must
        // answer a typed error (structure/CRC damage is expected
        // first — what is pinned here is that it NEVER panics and
        // that a clean-looking low-channel frame cannot reach the
        // interleaver: the dedicated fixture-based regression in
        // tests/flac_bitexact.rs covers the decodable variant).
        let frame = [0xFFu8, 0xF8, 0x51, 0x00, 0x00, 0xC5, 0x00, 0x00];
        let mut scratch = Vec::new();
        let mut out = vec![0u8; max_output_len(&facts)];
        let result = decode_into(&frame, &facts, &mut scratch, &mut out);
        assert!(result.is_err());
        // And the contract path reports the same class.
        let mut decoder = FlacDecoder::new();
        let descriptor = match media_types::ids::CodecDescriptor::new("flac") {
            Ok(d) => d,
            Err(e) => unreachable!("{e:?}"),
        };
        let generation = match GenerationId::FIRST.next() {
            Ok(g) => g,
            Err(e) => unreachable!("{e:?}"),
        };
        match decoder.configure(&DecoderConfig::new(
            descriptor,
            generation,
            ThreadBudget::default(),
            None,
            Some(media_types::packet::SharedBytes::from_vec(body.to_vec())),
        )) {
            Ok(()) => {}
            Err(e) => unreachable!("{e:?}"),
        }
        let packet = Packet {
            track: media_types::TrackId::new(0),
            pts: media_types::Stamp::Missing,
            dts: media_types::Stamp::Missing,
            duration: None,
            data: media_types::packet::SharedBytes::from_vec(frame.to_vec()),
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
        let outcome = decoder.decode(packet, &mut Sink);
        assert!(outcome.is_err());
    }
}

impl Decoder for FlacDecoder {
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
        let descriptor = cfg.codec.as_str();
        if !Self::accepts(descriptor) {
            return Err(unsupported(
                "flac.codec_descriptor",
                format!("codec descriptor `{descriptor}` is not `flac`"),
            )
            .into_engine_error());
        }
        let body = cfg
            .extra_data
            .as_ref()
            .map(|d| d.as_slice())
            .ok_or_else(|| {
                unsupported(
                    "flac.streaminfo.missing",
                    "no extra_data: the decoder requires the container's 34-byte \
                     STREAMINFO body and refuses to guess the geometry",
                )
            })?;
        let facts = super::metadata::parse_streaminfo(body)?;
        // The pooled output rotation is sized once per configuration
        // (bounded: SAMPLE_SLOTS × the stream maximum).
        self.sample_pool.clear();
        self.slot_next = 0;
        self.facts = Some(facts);
        self.generation = Some(cfg.generation);
        self.codec = Some(descriptor.to_string());
        Ok(())
    }

    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        let (generation, facts) = match (self.generation, self.facts) {
            (Some(generation), Some(facts)) => (generation, facts),
            _ => return Err(FlacError::BadState.into_engine_error()),
        };
        if let Some(outcome) = drop_stale_packet(&packet, generation) {
            return Ok(outcome);
        }
        if packet.data.is_empty() {
            // Consumed, nothing decodable (the PCM convention; an
            // empty packet is not frame damage).
            return Ok(DecodeOutcome::NeedMoreInput);
        }
        let slot_len = max_output_len(&facts);
        let FlacDecoder {
            block_scratch,
            sample_pool,
            slot_next,
            ..
        } = self;
        let pooled = pooled_sample_slot(sample_pool, slot_next, slot_len, |dst| {
            decode_into(packet.data.as_slice(), &facts, block_scratch, dst)
        })
        .map_err(FlacError::into_engine_error)?;
        let meta = pooled.meta;
        let exact_len = meta.block_size as usize * facts.channels as usize * bytes_per(meta.format);
        let duration_nanos =
            u128::from(meta.block_size) * 1_000_000_000u128 / u128::from(meta.format.rate.max(1));
        // `facts.sample_rate` is validated non-zero at configure
        // (`parse_streaminfo`); a zero rate here would be an
        // unreachable-by-construction state, and the mapping below
        // keeps it a typed error instead of a panic.
        let base = TimeBase::new(1, meta.format.rate).map_err(|_| {
            unsupported("flac.streaminfo.rate", "configured facts carry rate 0").into_engine_error()
        })?;
        let frame = Frame::Audio(media_types::frame::AudioFrame {
            format: meta.format,
            samples: pooled.bytes.slice(0..exact_len),
            pts: Stamp::Present(Timestamp::new(
                i64::try_from(meta.first_sample).unwrap_or(i64::MAX),
                base,
            )),
            duration: Some(Duration::Known(Nanoseconds(
                u64::try_from(duration_nanos).unwrap_or(u64::MAX),
            ))),
            generation: packet.generation,
        });
        out.accept(frame)?;
        Ok(DecodeOutcome::Emitted { frames: 1 })
    }

    fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        // FLAC carries no inter-frame state: nothing is buffered.
        if self.generation.is_none() || self.facts.is_none() {
            return Err(FlacError::BadState.into_engine_error());
        }
        Ok(DecodeOutcome::Drained)
    }
}

/// One pooled-slot fill result: the block metadata plus the bytes
/// (the slot view, exact-frame slice pending at the call site).
struct PooledBlock {
    meta: BlockMeta,
    bytes: SharedBytes,
}

/// The pooled output rotation (the MJPEG `pooled_frame_slot` pattern,
/// re-authored locally to keep the audio module independent of the
/// video family's internals): reuse a slot whose bytes are
/// exclusively owned, else allocate a fresh one and enter the
/// rotation (bounded at [`SAMPLE_SLOTS`], then evicting). Every slot
/// is `slot_len` bytes — the stream maximum; `decode_into` writes the
/// actual block at the front and the caller slices.
fn pooled_sample_slot(
    slots: &mut Vec<SharedBytes>,
    next: &mut usize,
    slot_len: usize,
    fill: impl FnOnce(&mut [u8]) -> Result<BlockMeta, FlacError>,
) -> Result<PooledBlock, FlacError> {
    let count = slots.len();
    for i in 0..count {
        let idx = (*next + i) % count;
        if slots[idx].len() != slot_len {
            continue;
        }
        if let Some(bytes) = slots[idx].get_mut() {
            let meta = fill(bytes)?;
            *next = (idx + 1) % count;
            return Ok(PooledBlock {
                meta,
                bytes: slots[idx].clone(),
            });
        }
        // The slot still has downstream owners — try the next one.
    }
    // Cold path: allocate, fill in place, then enter the rotation.
    let mut fresh = SharedBytes::from_vec(vec![0; slot_len]);
    let meta = match fresh.get_mut() {
        Some(bytes) => fill(bytes),
        None => {
            // A freshly built `SharedBytes` has refcount 1 by
            // construction; `get_mut` failing here is impossible —
            // surface it loudly instead of fabricating a frame.
            return Err(corrupt(
                "flac.output.fresh_slot_not_exclusive",
                media_types::error::CorruptDetail::InvalidStructure { offset: None },
                "fresh output slot is not exclusively owned",
            ));
        }
    }?;
    if slots.len() < SAMPLE_SLOTS {
        slots.push(fresh.clone());
        *next = slots.len() - 1;
    } else {
        *next = (*next + 1) % SAMPLE_SLOTS;
        slots[*next] = fresh.clone();
    }
    Ok(PooledBlock { meta, bytes: fresh })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_gate_accepts_only_flac() {
        assert!(FlacDecoder::accepts("flac"));
        assert!(!FlacDecoder::accepts("FLAC"));
        assert!(!FlacDecoder::accepts("mp3"));
        assert!(!FlacDecoder::accepts(""));
    }

    #[test]
    fn output_width_mapping_is_left_justifying() {
        assert_eq!(output_sample_format(16), (SampleFormat::S16, 0));
        assert_eq!(output_sample_format(12), (SampleFormat::S16, 4));
        assert_eq!(output_sample_format(8), (SampleFormat::S16, 8));
        assert_eq!(output_sample_format(4), (SampleFormat::S16, 12));
        assert_eq!(output_sample_format(24), (SampleFormat::S24, 0));
        assert_eq!(output_sample_format(20), (SampleFormat::S24, 4));
        assert_eq!(output_sample_format(32), (SampleFormat::S32, 0));
        assert_eq!(output_sample_format(28), (SampleFormat::S32, 4));
    }

    #[test]
    fn write_i24_is_native_order_low_24_bits() {
        let mut buf = [0u8; 3];
        // -2 in 24-bit two's complement: 0xFFFFFE.
        write_i24(&mut buf, -2);
        let (first, last) = if cfg!(target_endian = "big") {
            (0xFF, 0xFE)
        } else {
            (0xFE, 0xFF)
        };
        assert_eq!(buf, [first, 0xFF, last]);
        // A value with bits above 24 keeps only the low 24 (the
        // shift-normalized value always fits; this pins the rule).
        write_i24(&mut buf, 0x12FF_FFFE);
        assert_eq!(buf[0], 0xFE);
    }
}
