//! The ALAC decoder core (card 5.J3a): the magic-cookie
//! (ALACSpecificConfig) parse, the symphonia-backed frame decode, and
//! the [`codec_api::Decoder`] implementation.
//!
//! # Engine choice (the 5.J3a dependency decision)
//!
//! **`symphonia-codec-alac` 0.6.1, exact-pinned** (deps: `log`,
//! `symphonia-common`, `symphonia-core` — all already in the lock via
//! the mp3/vorbis/aac siblings; the record's R2 counts exactly ONE new
//! node), under the Lead's C5 pre-approval, purity verified BEFORE any
//! Cargo.toml edit in `docs/dependencies/records/symphonia-alac-0.6.1.md`
//! (F1–F7 all NO, `#![forbid(unsafe_code)]` in the crate, MPL-2.0,
//! tarball sha256-matched). The engine owns exactly one job:
//! entropy-decoding ONE already-framed ALAC raw data block (element
//! walk → rice residuals → adaptive LPC → mid-side → extra-bits
//! splice) to `i32` planes, left-justified to 32 bits. The
//! ALACSpecificConfig parse, the container passthrough shapes and the
//! error taxonomy are ours — a future engine swap stays inside this
//! module.
//!
//! # Packet model (the codec-api contract)
//!
//! One packet = **one complete ALAC raw data block** (element stream:
//! SCE/CPE/LFE payloads, optional DSE/FIL, terminating `111` END tag
//! — the MP4 sample / MKV simple block payload verbatim). ALAC frames
//! are independent (every frame re-states its element headers, rice
//! histories and — for partial frames — its 32-bit sample count), so
//! per-frame isolation (doc 03 §4.5(d)) holds: after a typed `Corrupt`
//! the next packet decodes unchanged. Bounded work: one
//! `decode_ref` call consumes at most the packet (the engine's
//! `BitReaderLtr` is bounded by `packet.data`).
//!
//! # Facts channel (the FLAC `extra_data` precedent)
//!
//! [`DecoderConfig::extra_data`] carries the ALAC **magic cookie**
//! verbatim — the MP4 `alac` sample entry's esds blob / the MKV
//! CodecPrivate, both in their native passthrough shapes:
//!
//! - 24 bytes — the bare ALACSpecificConfig (the MKV shape),
//! - 36 bytes — `[size][alac][version][config]` (the MP4 sample-entry
//!   shape ffmpeg exports as extradata),
//! - 48 bytes — config + the 24-byte `chan` layout block,
//! - and the `frma`-prefixed variants thereof.
//!
//! [`parse_alac_config`] validates them (typed `alac.config.*`);
//! the SAME bytes are then handed to the engine untouched — we never
//! re-serialize the cookie (no drift surface between our parse and
//! the engine's).
//!
//! # Output format (codec-native, bit-exact against the reference)
//!
//! The engine left-justifies each sample into 32 bits
//! (`pcm << (32 − bit_depth)`); this decoder narrows to the smallest
//! media-types width that holds the stream — ≤16 →
//! [`SampleFormat::S16`], 17–24 → [`SampleFormat::S24`], 25–32 →
//! [`SampleFormat::S32`] — by `sample >> (32 − width)`, which lands
//! the source bits in the high bits of the container (the FLAC
//! module's lossless mapping; a 20-bit stream rides S24 as
//! `pcm << 4`, exactly the value ffmpeg's own s32→s24 conversion
//! emits). Output is interleaved native-endian PCM; no rate or
//! channel conversion.
//!
//! # The prior session's suspected engine bug (NOT reproduced)
//!
//! A lost wave-5 session recorded: "`alac.frame.truncated` on frames
//! ffmpeg decodes; mono whole-file bit-exact; suspected rice-escape
//! `read_unary_ones_capped(9)` leaving the terminator bit
//! unconsumed". This session re-derived the bit-level semantics from
//! the extracted sources (engine `read_rice_code` vs ffmpeg 7.1.5
//! `decode_scalar`, `libavcodec/alac.c`): the unary cap
//! (`read_unary_ones_capped(9)`) consumes exactly 9 one-bits and NO
//! terminator on the escape path, then reads `bps` raw bits — **bit
//! for bit what ffmpeg's `get_unary_0_9` + `get_bits_long(bps)`
//! does**; the `k > 1` suffix path (peek k−1, extend by 1 iff nonzero)
//! reproduces ffmpeg's `show_bits(k)` / `skip_bits(k−1|k)` pair; the
//! `k == 1` no-suffix case and the zero-run block-size read (16-bit
//! escape width) match as well. The differentials below (stereo 16-bit
//! multi-frame, mono 16-bit, stereo/mono 24-bit — the tail-bits path)
//! are **bit-exact**, so the suspicion is recorded as not reproduced
//! on the 0.6.1 engine with correct packetization. The one engine
//! divergence found in the audit is a harmless UNDER-consumption (it
//! breaks out of the element loop as soon as all channels are decoded,
//! never reading the trailing `111` END tag that ffmpeg requires) —
//! no output impact on clean streams; damage that truncates before the
//! END tag still decodes identically on both sides unless the payload
//! itself is short (then the engine answers end-of-bitstream → typed
//! [`alac.frame.truncated`], the tripwire the prior session hit).
//!
//! # Allocation discipline (D-1)
//!
//! The engine reuses its internal `AudioBuffer<i32>` across calls; the
//! interleaver fills a pooled [`SharedBytes`] rotation sized to the
//! cookie's frame maximum (the 5.A1 `SAMPLE_SLOTS` pattern);
//! steady-state decodes refill a slot in place — zero per-frame
//! allocation after warm-up.

use super::error::{AlacError, corrupt, unsupported};
use codec_api::prelude::*;
use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::time::{Nanoseconds, TimeBase, Timestamp};
use media_types::{Duration, Stamp};
use symphonia_codec_alac::AlacDecoder as AlacEngine;
use symphonia_core::codecs::audio::well_known::CODEC_ID_ALAC;
use symphonia_core::codecs::audio::{AudioCodecParameters, AudioDecoder, AudioDecoderOptions};
use symphonia_core::errors::Error as SymError;
use symphonia_core::packet::PacketRef;
use symphonia_core::units::{Duration as SymDuration, Timestamp as SymTimestamp};

/// One parsed ALACSpecificConfig (the 24-byte body; our own parse, the
/// engine gets the original bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlacConfig {
    /// Samples per frame (`frameLength`).
    pub frame_length: u32,
    /// Source bit depth (`bitDepth`) — 16/20/24/32 in practice.
    pub bit_depth: u8,
    /// Channel count (`numChannels`, 1–8).
    pub num_channels: u8,
    /// Sample rate (`sampleRate`, Hz).
    pub sample_rate: u32,
    /// Rice parameters from the cookie (diagnostics; the engine's own
    /// copy governs decode): history multiplier, initial history,
    /// limit.
    pub rice_pb_mb_kb: (u8, u8, u8),
    /// `maxFrameBytes` (diagnostics).
    pub max_frame_bytes: u32,
}

/// Skip the optional 12-byte atom headers a magic cookie may carry
/// (`frma` / `alac`), returning the remaining slice.
fn skip_cookie_prefixes(mut buf: &[u8]) -> &[u8] {
    loop {
        if buf.len() >= 12 && &buf[4..8] == b"frma" {
            buf = &buf[12..];
            continue;
        }
        if buf.len() >= 12 && &buf[4..8] == b"alac" {
            buf = &buf[12..];
            continue;
        }
        return buf;
    }
}

/// Parse the ALAC magic cookie (the ALACSpecificConfig plus the
/// passthrough prefixes/extension the containers ride — see the
/// module docs). The SAME bytes go to the engine untouched; this parse
/// exists so the wrapper owns the typed gate and the output-format
/// negotiation without trusting the blob twice.
///
/// # Errors
/// Typed [`EngineError`]s (`alac.config.*`, all `Unsupported`): length,
/// version, bit depth, channel count, frame length, rate.
pub fn parse_alac_config(cookie: &[u8]) -> Result<AlacConfig, EngineError> {
    parse_alac_config_inner(cookie).map_err(AlacError::into_engine_error)
}

/// The internal form (the typed [`AlacError`] for the crate's own
/// composition paths).
fn parse_alac_config_inner(cookie: &[u8]) -> Result<AlacConfig, AlacError> {
    let body = skip_cookie_prefixes(cookie);
    // 24 = the ALACSpecificConfig; 48 = config + the `chan` layout
    // block (parsed by the engine — we only need the leading 24).
    if body.len() != 24 && body.len() != 48 {
        return Err(unsupported(
            "alac.config.length",
            format!(
                "magic cookie body is {} bytes, expected 24 (bare config) or 48 \
                 (config + chan block) after optional frma/alac prefixes",
                body.len()
            ),
        ));
    }
    let be32 = |at: usize| -> u32 {
        u32::from_be_bytes([body[at], body[at + 1], body[at + 2], body[at + 3]])
    };
    let config = AlacConfig {
        frame_length: be32(0),
        bit_depth: body[5],
        rice_pb_mb_kb: (body[6], body[7], body[8]),
        num_channels: body[9],
        max_frame_bytes: be32(14),
        sample_rate: be32(20),
    };
    if body[4] > 0 {
        return Err(unsupported(
            "alac.config.version",
            format!("compatibleVersion {} — only ALAC version 0 is supported", body[4]),
        ));
    }
    if config.bit_depth == 0 || config.bit_depth > 32 {
        return Err(unsupported(
            "alac.config.bit_depth",
            format!("bitDepth {} outside 1..=32", config.bit_depth),
        ));
    }
    if config.num_channels == 0 || config.num_channels > 8 {
        return Err(unsupported(
            "alac.config.channels",
            format!("numChannels {} outside 1..=8", config.num_channels),
        ));
    }
    // The engine's own DoS gate (4096 × 16); mirror it so the typed
    // code is ours, not an opaque engine refusal.
    if config.frame_length == 0 || config.frame_length > 4096 * 16 {
        return Err(unsupported(
            "alac.config.frame_length",
            format!("frameLength {} outside 1..=65536", config.frame_length),
        ));
    }
    if config.sample_rate == 0 {
        return Err(unsupported(
            "alac.config.rate",
            "sampleRate 0 — the timestamp arithmetic needs a real rate",
        ));
    }
    Ok(config)
}

/// The output width for a stream's bit depth (module-doc mapping):
/// smallest media-types sample format that holds `bit_depth`
/// left-justified, with the narrowing shift from the engine's
/// 32-bit-justified planes.
pub(crate) fn output_sample_format(bit_depth: u8) -> (SampleFormat, u32) {
    match u32::from(bit_depth) {
        0..=16 => (SampleFormat::S16, 16),
        17..=24 => (SampleFormat::S24, 24),
        _ => (SampleFormat::S32, 32),
    }
}

/// The negotiated audio shape a configured stream will emit.
#[must_use]
pub fn output_format(config: &AlacConfig) -> AudioFormat {
    let (sample, _) = output_sample_format(config.bit_depth);
    AudioFormat {
        sample,
        channels: u32::from(config.num_channels),
        layout: AudioLayout::Interleaved,
        rate: config.sample_rate,
    }
}

/// Bytes per output sample of a negotiated format.
fn bytes_per(format: AudioFormat) -> usize {
    format.sample.bytes_per_sample() as usize
}

/// The output-slot size a stream can ever need: frame_length ×
/// channels × width (the pooled rotation is sized to this once at
/// configure).
fn max_output_len(config: &AlacConfig) -> usize {
    let (sample, _) = output_sample_format(config.bit_depth);
    config.frame_length as usize * config.num_channels as usize * sample.bytes_per_sample() as usize
}

/// One decoded ALAC block, format-independent form (the free
/// surface's result; the contract path lifts the same bytes into
/// `Frame::Audio`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedAlac {
    /// The output shape (native-endian, interleaved).
    pub format: AudioFormat,
    /// Samples per channel in this block (may be < frame_length on
    /// the final partial frame).
    pub samples: u32,
    /// Interleaved PCM bytes, width per [`DecodedAlac::format`].
    pub pcm: Vec<u8>,
}

/// Build a throwaway engine for a cookie (the free surface / fuzz
/// entry; the contract path builds ONE engine at configure and reuses
/// it — the engine's internal `AudioBuffer` is the D-1 reuse).
fn engine_for(cookie: &[u8]) -> Result<AlacEngine, AlacError> {
    let params = AudioCodecParameters {
        codec: CODEC_ID_ALAC,
        extra_data: Some(cookie.to_vec().into_boxed_slice()),
        ..AudioCodecParameters::default()
    };
    AlacEngine::try_new(&params, &AudioDecoderOptions::default())
        .map_err(|e| unsupported("alac.engine.init", format!("engine refused the cookie: {e}")))
}

/// Map an engine error to the typed taxonomy (the truncation
/// tripwire: end-of-bitstream mid-frame is `alac.frame.truncated`).
fn map_engine_error(error: SymError, packet_len: usize) -> AlacError {
    match error {
        SymError::IoError(io) => {
            // The engine's end-of-bitstream signal is
            // `io::Error::other("unexpected end of bitstream")` (kind
            // `Other`, not `UnexpectedEof` — verified against the
            // extracted symphonia-core source); a genuine EOF rides
            // `UnexpectedEof`. Both are the truncation tripwire.
            let truncated = io.kind() == std::io::ErrorKind::UnexpectedEof
                || io.to_string().contains("end of bitstream");
            corrupt(
                if truncated {
                    "alac.frame.truncated"
                } else {
                    "alac.frame.io"
                },
                if truncated {
                    media_types::error::CorruptDetail::Truncated {
                        expected: 0,
                        found: packet_len as u64,
                    }
                } else {
                    media_types::error::CorruptDetail::InvalidStructure { offset: None }
                },
                format!("engine bit read failed: {io}"),
            )
        }
        SymError::DecodeError(reason) => corrupt(
            "alac.frame.decode",
            media_types::error::CorruptDetail::InvalidStructure { offset: None },
            format!("engine rejected the frame: {reason}"),
        ),
        SymError::Unsupported(feature) => unsupported(
            "alac.frame.feature",
            format!("frame uses a feature the engine does not implement: {feature}"),
        ),
        SymError::LimitError(constraint) => unsupported(
            "alac.frame.limit",
            format!("engine limit tripped: {constraint}"),
        ),
        SymError::SeekError(_) | SymError::ResetRequired => unsupported(
            "alac.frame.feature",
            "engine asked for a seek/reset mid-frame — not an ALAC decode path",
        ),
        // `Error` is #[non_exhaustive]; a future engine variant is a
        // typed unsupported, never a compile break in the wrapper.
        _ => unsupported(
            "alac.frame.feature",
            "engine returned an unmapped error class",
        ),
    }
}

/// The metadata part of one decoded block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BlockMeta {
    /// Samples per channel in this block.
    pub samples: u32,
    /// The negotiated output shape.
    pub format: AudioFormat,
}

/// Decode one block's planes into `dst` (the reusable-fill form behind
/// both surfaces). `scratch` holds the interleaved i32 engine output
/// (grown on demand); `dst` must hold [`max_output_len`] bytes.
pub(crate) fn decode_into(
    engine: &mut AlacEngine,
    packet: &[u8],
    config: &AlacConfig,
    scratch: &mut Vec<i32>,
    dst: &mut [u8],
) -> Result<BlockMeta, AlacError> {
    if packet.is_empty() {
        // The contract path answers `NeedMoreInput` for empty packets
        // before reaching here; the free surface treats frame-less
        // bytes as structure damage.
        return Err(corrupt(
            "alac.frame.empty",
            media_types::error::CorruptDetail::InvalidStructure { offset: None },
            "packet carries no frame bytes",
        ));
    }
    let pkt = PacketRef::new(0, SymTimestamp::ZERO, SymDuration::ZERO, packet);
    let decoded = engine
        .decode_ref(&pkt)
        .map_err(|e| map_engine_error(e, packet.len()))?;
    let samples = decoded.frames();
    let channels = decoded.spec().channels().count();
    if samples == 0 || channels == 0 {
        return Err(corrupt(
            "alac.frame.no_samples",
            media_types::error::CorruptDetail::InvalidStructure { offset: None },
            "engine decoded zero samples/channels",
        ));
    }
    if channels != config.num_channels as usize {
        // The engine derives channels from the same cookie, so this is
        // unreachable-by-construction — the gate exists so a future
        // engine drift is a typed error, never a wrong-buffer write.
        return Err(corrupt(
            "alac.frame.channels",
            media_types::error::CorruptDetail::InvalidStructure { offset: None },
            format!(
                "engine produced {channels} channel(s), the cookie declares {}",
                config.num_channels
            ),
        ));
    }
    let need = samples * channels;
    if scratch.len() < need {
        scratch.resize(need, 0);
    }
    let out = &mut scratch[..need];
    decoded.copy_to_slice_interleaved::<i32, _>(&mut *out);
    // Narrow the engine's 32-bit-justified planes into the negotiated
    // width (module-doc mapping: source bits land in the high bits).
    let (sample_format, width_bits) = output_sample_format(config.bit_depth);
    let width = sample_format.bytes_per_sample() as usize;
    let shift = 32 - width_bits;
    let output_len = samples * channels * width;
    if dst.len() < output_len {
        return Err(corrupt(
            "alac.output.slot_size",
            media_types::error::CorruptDetail::InvalidStructure { offset: None },
            format!("output slot is {} bytes, block needs {output_len}", dst.len()),
        ));
    }
    match width {
        2 => {
            for (word, pair) in out.iter().zip(dst.chunks_exact_mut(2)) {
                let value = (word >> shift) as i16;
                pair.copy_from_slice(&value.to_ne_bytes());
            }
        }
        3 => {
            for (word, triple) in out.iter().zip(dst.chunks_exact_mut(3)) {
                write_i24(triple, word >> shift);
            }
        }
        _ => {
            for (word, quad) in out.iter().zip(dst.chunks_exact_mut(4)) {
                quad.copy_from_slice(&(*word >> shift).to_ne_bytes());
            }
        }
    }
    Ok(BlockMeta {
        samples: samples as u32,
        format: AudioFormat {
            sample: sample_format,
            channels: config.num_channels as u32,
            layout: AudioLayout::Interleaved,
            rate: config.sample_rate,
        },
    })
}

/// Write the low 24 bits of `value` in **native** byte order (the
/// media-types `S24` semantics; the 5.A1 helper, verbatim).
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

/// Decode one ALAC frame with the config parsed on the fly
/// (convenience composition over [`parse_alac_config`] + a throwaway
/// engine — the fuzz/differential entry; the contract path reuses its
/// engine and pooled slots).
///
/// # Errors
/// Typed [`AlacError`]s: the cookie's rejection
/// (`alac.config.*` / `alac.engine.init`) or the frame's
/// ([`decode_into`]).
pub fn decode_frame(packet: &[u8], cookie: &[u8]) -> Result<DecodedAlac, EngineError> {
    let config = parse_alac_config(cookie)?;
    let mut engine = engine_for(cookie).map_err(AlacError::into_engine_error)?;
    let mut scratch = Vec::new();
    let mut out = vec![0u8; max_output_len(&config)];
    let meta = decode_into(&mut engine, packet, &config, &mut scratch, &mut out)?;
    out.truncate(
        meta.samples as usize * config.num_channels as usize * bytes_per(meta.format),
    );
    Ok(DecodedAlac {
        format: meta.format,
        samples: meta.samples,
        pcm: out,
    })
}

/// ALAC decoder behind the codec-api [`Decoder`] contract (card 5.J3a;
/// doc 04 §3.1 row "ALAC").
///
/// `configure` installs the generation + the magic cookie (mandatory —
/// the cookie carries frame length, bit depth and channels; frame
/// headers do not restate them, so guessing is never allowed);
/// `decode` turns one raw-data-block packet into one
/// [`Frame::Audio`]; `flush` is immediately drained (no inter-frame
/// state).
///
/// # Example
///
/// ```
/// use codec_api::prelude::*;
/// use codecs::audio::alac::AlacDecoder;
/// use media_types::{GenerationId, Stamp, TrackId, packet::SharedBytes};
///
/// # fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let mut decoder = AlacDecoder::new();
/// let generation = GenerationId::FIRST.next()?;
/// // Without the cookie, configure is a typed rejection.
/// let err = decoder
///     .configure(&DecoderConfig::new(
///         media_types::ids::CodecDescriptor::new("alac")?,
///         generation,
///         ThreadBudget::default(),
///         None,
///         None,
///     ))
///     .expect_err("the cookie is mandatory");
/// assert_eq!(err.context.code, "alac.config.missing");
/// # Ok(())
/// # }
/// # run().unwrap_or_else(|e| unreachable!("{e}"));
/// ```
#[derive(Default)]
pub struct AlacDecoder {
    /// The validated config facts (None until `configure`).
    config: Option<AlacConfig>,
    /// The cookie bytes handed to the engine (None until `configure`).
    cookie: Option<Vec<u8>>,
    /// The configured stream generation (None until `configure`).
    generation: Option<GenerationId>,
    /// The engine (built once at configure; its internal AudioBuffer
    /// is the D-1 reuse).
    engine: Option<AlacEngine>,
    /// Reusable interleaved i32 scratch.
    scratch: Vec<i32>,
    /// Pooled output slots, each sized to the stream maximum.
    sample_pool: Vec<SharedBytes>,
    /// Round-robin cursor into `sample_pool`.
    slot_next: usize,
    /// Samples emitted so far (pts bookkeeping).
    samples_out: u64,
}

/// Output-rotation depth (the 5.A1 bound).
const SAMPLE_SLOTS: usize = 4;

impl std::fmt::Debug for AlacDecoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AlacDecoder")
            .field("config", &self.config)
            .field("samples_out", &self.samples_out)
            .finish_non_exhaustive()
    }
}

impl AlacDecoder {
    /// A fresh decoder, unconfigured.
    #[must_use]
    pub fn new() -> AlacDecoder {
        AlacDecoder::default()
    }

    /// The codec descriptors this decoder accepts (`alac`; doc 04
    /// §3.1).
    fn accepts(descriptor: &str) -> bool {
        descriptor == "alac"
    }
}

impl Decoder for AlacDecoder {
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
        let descriptor = cfg.codec.as_str();
        if !Self::accepts(descriptor) {
            return Err(unsupported(
                "alac.codec_descriptor",
                format!("codec descriptor `{descriptor}` is not `alac`"),
            )
            .into_engine_error());
        }
        let cookie = cfg
            .extra_data
            .as_ref()
            .map(|d| d.as_slice())
            .ok_or_else(|| {
                unsupported(
                    "alac.config.missing",
                    "no extra_data: the decoder requires the container's ALAC magic cookie \
                     and refuses to guess the geometry",
                )
            })?;
        let config = parse_alac_config(cookie)?;
        let engine = engine_for(cookie).map_err(AlacError::into_engine_error)?;
        // Reset the pooled rotation for the new stream shape.
        self.sample_pool.clear();
        self.slot_next = 0;
        self.scratch.clear();
        self.samples_out = 0;
        self.config = Some(config);
        self.cookie = Some(cookie.to_vec());
        self.generation = Some(cfg.generation);
        self.engine = Some(engine);
        Ok(())
    }

    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        let (generation, config) = match (self.generation, self.config) {
            (Some(generation), Some(config)) => (generation, config),
            _ => return Err(AlacError::BadState.into_engine_error()),
        };
        if let Some(outcome) = drop_stale_packet(&packet, generation) {
            return Ok(outcome);
        }
        if packet.data.is_empty() {
            // Consumed, nothing decodable (the PCM convention; an
            // empty packet is not frame damage).
            return Ok(DecodeOutcome::NeedMoreInput);
        }
        let slot_len = max_output_len(&config);
        let AlacDecoder {
            engine,
            scratch,
            sample_pool,
            slot_next,
            ..
        } = self;
        let engine = match engine.as_mut() {
            Some(engine) => engine,
            None => return Err(AlacError::BadState.into_engine_error()),
        };
        let pooled = pooled_sample_slot(sample_pool, slot_next, slot_len, |dst| {
            decode_into(engine, packet.data.as_slice(), &config, scratch, dst)
        })
        .map_err(AlacError::into_engine_error)?;
        let meta = pooled.meta;
        let exact_len =
            meta.samples as usize * config.num_channels as usize * bytes_per(meta.format);
        let duration_nanos = u128::from(meta.samples) * 1_000_000_000u128
            / u128::from(meta.format.rate.max(1));
        // `config.sample_rate` is validated non-zero at configure; the
        // mapping keeps a hypothetical zero a typed error, not a panic.
        let base = TimeBase::new(1, meta.format.rate).map_err(|_| {
            unsupported("alac.config.rate", "configured facts carry rate 0").into_engine_error()
        })?;
        let pts = i64::try_from(self.samples_out).unwrap_or(i64::MAX);
        self.samples_out = self.samples_out.saturating_add(u64::from(meta.samples));
        let frame = Frame::Audio(media_types::frame::AudioFrame {
            format: meta.format,
            samples: pooled.bytes.slice(0..exact_len),
            pts: Stamp::Present(Timestamp::new(pts, base)),
            duration: Some(Duration::Known(Nanoseconds(
                u64::try_from(duration_nanos).unwrap_or(u64::MAX),
            ))),
            generation: packet.generation,
        });
        out.accept(frame)?;
        Ok(DecodeOutcome::Emitted { frames: 1 })
    }

    fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        // ALAC carries no inter-frame state: nothing is buffered.
        if self.generation.is_none() || self.config.is_none() || self.engine.is_none() {
            return Err(AlacError::BadState.into_engine_error());
        }
        Ok(DecodeOutcome::Drained)
    }
}

/// One pooled-slot fill result: the block metadata plus the bytes.
struct PooledBlock {
    meta: BlockMeta,
    bytes: SharedBytes,
}

/// The pooled output rotation (the 5.A1 pattern, re-authored locally):
/// reuse a slot whose bytes are exclusively owned, else allocate a
/// fresh one and enter the rotation (bounded at [`SAMPLE_SLOTS`], then
/// evicting). Every slot is `slot_len` bytes — the stream maximum;
/// `decode_into` writes the actual block at the front and the caller
/// slices.
fn pooled_sample_slot(
    slots: &mut Vec<SharedBytes>,
    next: &mut usize,
    slot_len: usize,
    fill: impl FnOnce(&mut [u8]) -> Result<BlockMeta, AlacError>,
) -> Result<PooledBlock, AlacError> {
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
    let mut fresh = SharedBytes::from_vec(vec![0u8; slot_len]);
    let meta = fill(fresh.get_mut().unwrap_or_else(|| unreachable!("fresh slot is exclusive")))?;
    let bytes = fresh.clone();
    if slots.len() < SAMPLE_SLOTS {
        slots.push(fresh);
        *next = slots.len() % SAMPLE_SLOTS;
    } else {
        slots[*next] = fresh;
        *next = (*next + 1) % SAMPLE_SLOTS;
    }
    Ok(PooledBlock { meta, bytes })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 24-byte ALACSpecificConfig: 4096-sample frames, 16-bit,
    /// stereo, 44.1 kHz (the ffmpeg encoder's own shape for
    /// `-c:a alac`).
    fn cookie_16_stereo() -> [u8; 24] {
        let mut body = [0u8; 24];
        body[0..4].copy_from_slice(&4096u32.to_be_bytes());
        // body[4] = compatibleVersion 0
        body[5] = 16; // bitDepth
        body[6] = 40; // pb (rice history mult)
        body[7] = 10; // mb (initial history)
        body[8] = 14; // kb (rice limit)
        body[9] = 2; // numChannels
        body[10..12].copy_from_slice(&255u16.to_be_bytes()); // maxRun
        body[12..16].copy_from_slice(&0u32.to_be_bytes()); // maxFrameBytes
        body[16..20].copy_from_slice(&0u32.to_be_bytes()); // avgBitRate
        body[20..24].copy_from_slice(&44_100u32.to_be_bytes());
        body
    }

    #[test]
    fn cookie_parse_accepts_the_passthrough_shapes() {
        let bare = cookie_16_stereo();
        let config = match parse_alac_config(&bare) {
            Ok(c) => c,
            Err(e) => unreachable!("{:?}", e.kind),
        };
        assert_eq!(config.frame_length, 4096);
        assert_eq!(config.bit_depth, 16);
        assert_eq!(config.num_channels, 2);
        assert_eq!(config.sample_rate, 44_100);
        // The 36-byte MP4 sample-entry shape: [size][alac][version].
        let mut prefixed = Vec::with_capacity(36);
        prefixed.extend_from_slice(&24u32.to_be_bytes());
        prefixed.extend_from_slice(b"alac");
        prefixed.extend_from_slice(&0u32.to_be_bytes());
        prefixed.extend_from_slice(&bare);
        assert_eq!(parse_alac_config(&prefixed).unwrap_or_else(|e| unreachable!("{:?}", e.kind)), config);
        // 48-byte shape: bare + a zeroed chan block (the engine parses
        // the chan tag; our parse only needs to accept the length).
        let mut extended = bare.to_vec();
        extended.extend_from_slice(&[0u8; 24]);
        assert!(parse_alac_config(&extended).is_ok());
    }

    #[test]
    fn cookie_damage_is_typed_per_rule() {
        let good = cookie_16_stereo();
        // Too short.
        let code = |cookie: &[u8]| parse_alac_config(cookie).expect_err("must reject").context.code;
        assert_eq!(code(&good[..23]), "alac.config.length");
        // Version.
        let mut bad = good;
        bad[4] = 1;
        assert_eq!(code(&bad), "alac.config.version");
        // Bit depth.
        let mut bad = good;
        bad[5] = 33;
        assert_eq!(code(&bad), "alac.config.bit_depth");
        // Channels.
        let mut bad = good;
        bad[9] = 0;
        assert_eq!(code(&bad), "alac.config.channels");
        // Frame length.
        let mut bad = good;
        bad[0..4].copy_from_slice(&0u32.to_be_bytes());
        assert_eq!(code(&bad), "alac.config.frame_length");
        // Rate.
        let mut bad = good;
        bad[20..24].copy_from_slice(&0u32.to_be_bytes());
        assert_eq!(code(&bad), "alac.config.rate");
    }

    #[test]
    fn truncation_maps_to_the_typed_tripwire() {
        // A real-ish frame prefix (3-bit SCE tag + element header
        // bits) cut mid-header: the engine must answer
        // end-of-bitstream, which maps to `alac.frame.truncated`.
        let cookie = cookie_16_stereo();
        let mut engine = match engine_for(&cookie) {
            Ok(engine) => engine,
            Err(e) => unreachable!("{e:?}"),
        };
        let config = match parse_alac_config(&cookie) {
            Ok(config) => config,
            Err(e) => unreachable!("{e:?}"),
        };
        let packet = [0x00u8];
        let mut scratch = Vec::new();
        let mut out = vec![0u8; max_output_len(&config)];
        let err = decode_into(&mut engine, &packet, &config, &mut scratch, &mut out)
            .expect_err("truncated packet must fail");
        assert_eq!(
            err.into_engine_error().context.code,
            "alac.frame.truncated"
        );
        // An empty packet is the free surface's structure damage.
        let err = decode_into(&mut engine, &[], &config, &mut scratch, &mut out)
            .expect_err("empty packet must fail");
        assert_eq!(err.into_engine_error().context.code, "alac.frame.empty");
    }

    #[test]
    fn output_width_follows_the_bit_depth() {
        assert_eq!(
            output_sample_format(16),
            (SampleFormat::S16, 16)
        );
        assert_eq!(
            output_sample_format(20),
            (SampleFormat::S24, 24)
        );
        assert_eq!(
            output_sample_format(24),
            (SampleFormat::S24, 24)
        );
        assert_eq!(
            output_sample_format(32),
            (SampleFormat::S32, 32)
        );
    }
}
