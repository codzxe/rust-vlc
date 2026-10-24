//! The AIFF chunk parsers (card 5.A2): COMM (including the 80-bit
//! IEEE 754 extended sample rate — self-built, pure integer math),
//! SSND (offset/blockSize), and the big-endian IFF walk vocabulary.
//!
//! AIFF is IFF: chunk headers are **big-endian** (`ckID + ckSize`),
//! payloads are padded to even offsets exactly like RIFF's (one pad
//! byte, not counted in the size). The one payload read is shared
//! with RIFF: [`SourceReader`](crate::avi::riff::SourceReader) is
//! endian-agnostic byte plumbing, so the AIFF demuxer reuses it
//! verbatim.

use media_types::EngineError;

use super::error;
use crate::avi::riff::FourCC;

/// The `FORM` magic.
pub const FORM: FourCC = FourCC::tag(b"FORM");
/// The AIFF form type (uncompressed; PCM payloads are big-endian).
pub const FORM_AIFF: FourCC = FourCC::tag(b"AIFF");
/// The AIFF-C form type (compressed or byte-swapped; the compression
/// type rides COMM's tail).
pub const FORM_AIFC: FourCC = FourCC::tag(b"AIFC");

/// The `COMM` chunk id.
pub const COMM: FourCC = FourCC::tag(b"COMM");
/// The `SSND` chunk id.
pub const SSND: FourCC = FourCC::tag(b"SSND");
/// The `MARK` chunk id (marker list — skip-tolerant).
pub const MARK: FourCC = FourCC::tag(b"MARK");
/// The `INST` chunk id (instrument parameters — skip-tolerant).
pub const INST: FourCC = FourCC::tag(b"INST");

/// Maximum COMM payload trusted into memory
/// `[PROPOSAL: 4 KiB — COMM is 18 bytes classic, 22 + pstring for
/// AIFC; anything larger is a lying tail, skipped positionally]`.
pub const MAX_COMM_BYTES: usize = 4096;

/// The classic COMM payload: channels, frames, bits, 80-bit rate.
pub const COMM_MIN_BYTES: usize = 18;
/// The AIFC COMM floor: classic + the 4-byte compression type (the
/// pstring name may be zero-length but still costs its length byte).
pub const COMM_AIFC_MIN_BYTES: usize = 22;

/// `NONE` — big-endian PCM (the AIFF classic, also AIFC's explicit
/// spelling).
pub const COMPRESSION_NONE: FourCC = FourCC::tag(b"NONE");
/// `sowt` — little-endian PCM ("twos" reversed; Apple's byte-swap
/// compression).
pub const COMPRESSION_SOWT: FourCC = FourCC::tag(b"sowt");
/// `fl32` — 32-bit IEEE float, big-endian.
pub const COMPRESSION_FL32: FourCC = FourCC::tag(b"fl32");
/// `fl64` — 64-bit IEEE float, big-endian.
pub const COMPRESSION_FL64: FourCC = FourCC::tag(b"fl64");
/// `raw ` — unsigned 8-bit PCM (the ffmpeg AIFC mapping for
/// `pcm_u8`; byte-order-free like the WAV twin's tag-1 8-bit).
pub const COMPRESSION_RAW: FourCC = FourCC::tag(b"raw ");

/// The payload codec a COMM declares — the AIFF twin of the WAV
/// [`WavCodec`](crate::wav::format::WavCodec) vocabulary. AIFF PCM
/// is big-endian by definition; `sowt` is the little-endian corner
/// `[VERIFY-P0: doc 04 §2.2's AIFF note]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiffCodec {
    /// Signed big-endian integer PCM (`NONE`, or classic AIFF):
    /// 16/24/32-bit samples. Classic AIFF's 8-bit samples are
    /// *signed* — no registry descriptor, typed refusal.
    PcmBe {
        /// Bytes per sample (2/3/4).
        bytes_per_sample: u16,
    },
    /// Signed little-endian integer PCM (`sowt`): 16/24/32-bit.
    PcmLe {
        /// Bytes per sample (2/3/4).
        bytes_per_sample: u16,
    },
    /// 32-bit big-endian IEEE float (`fl32`).
    F32Be,
    /// 64-bit big-endian IEEE float (`fl64`).
    F64Be,
    /// Unsigned 8-bit (`raw `, the ffmpeg AIFC mapping) —
    /// byte-order-free, the registry's `pcm_u8`.
    U8,
}

impl AiffCodec {
    /// The registry descriptor (doc 04 §3.1 vocabulary).
    pub fn descriptor(&self) -> &'static str {
        match self {
            AiffCodec::PcmBe { bytes_per_sample } => match bytes_per_sample {
                2 => "pcm_s16be",
                3 => "pcm_s24be",
                _ => "pcm_s32be",
            },
            AiffCodec::PcmLe { bytes_per_sample } => match bytes_per_sample {
                2 => "pcm_s16le",
                3 => "pcm_s24le",
                _ => "pcm_s32le",
            },
            AiffCodec::F32Be => "pcm_f32be",
            AiffCodec::F64Be => "pcm_f64be",
            AiffCodec::U8 => "pcm_u8",
        }
    }

    /// Bytes per single sample.
    pub fn bytes_per_sample(&self) -> u16 {
        match self {
            AiffCodec::PcmBe { bytes_per_sample } | AiffCodec::PcmLe { bytes_per_sample } => {
                *bytes_per_sample
            }
            AiffCodec::F32Be => 4,
            AiffCodec::F64Be => 8,
            AiffCodec::U8 => 1,
        }
    }
}

/// The parsed COMM facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommChunk {
    /// Channel count (`numChannels`; 1..=64 accepted, the WAV
    /// parity ceiling).
    pub channels: u16,
    /// Declared sample frames (`numSampleFrames`; the SSND bytes are
    /// authority — a disagreement is damage, not a guess).
    pub num_frames: u32,
    /// Sample width in bits (`sampleSize`).
    pub sample_size: u16,
    /// Sample rate (`sampleRate`, the 80-bit extended converted to
    /// an exact integer; non-integral or non-normalizable rates are
    /// typed refusals).
    pub sample_rate: u32,
    /// AIFC only: the compression type + its resolved codec.
    pub compression: Option<(FourCC, AiffCodec)>,
}

/// The parsed SSND facts (absolute).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SsndChunk {
    /// First sound byte (chunk payload start + 8 + `offset`).
    pub data_start: u64,
    /// The chunk payload start (offset math's base).
    pub payload_start: u64,
    /// The chunk's declared end (payload start + ckSize).
    pub declared_end: u64,
}

/// Read a big-endian `u16` at `data[at]` (bounds are the caller's —
/// the parsers only slice complete buffers).
pub fn u16be(data: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([data[at], data[at + 1]])
}

/// Read a big-endian `u32` at `data[at]`.
pub fn u32be(data: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

/// Convert an IEEE 754 80-bit extended (the AIFF `sampleRate` field)
/// to `f64` — self-built from the published Apple numeric format
/// (pure integer math, no library).
///
/// The layout: 1 sign bit, 15 exponent bits (bias 16383), 64
/// mantissa bits with an **explicit** integer bit (unlike the IEEE
/// 754 short/long forms). Only *normalized* values are accepted:
/// exponent 0 (denormals and the Apple pseudo-zero `S 0 0…0`) and
/// exponent 0x7FFF (infinities, NaNs, unnormal integer-bit-clear
/// mantissas) have no honest integer-rate meaning and return `None`.
#[must_use]
pub fn extended80_to_f64(bytes: &[u8; 10]) -> Option<f64> {
    let sign = if bytes[0] & 0x80 != 0 { -1.0 } else { 1.0 };
    let exponent = u16::from_be_bytes([bytes[0] & 0x7F, bytes[1]]);
    if exponent == 0 || exponent == 0x7FFF {
        return None;
    }
    let mut mantissa = [0u8; 8];
    mantissa.copy_from_slice(&bytes[2..10]);
    let mantissa = u64::from_be_bytes(mantissa);
    // The explicit integer bit must be set in a normalized value.
    if mantissa & (1u64 << 63) == 0 {
        return None;
    }
    let value = sign * (mantissa as f64) * f64::powi(2.0, i32::from(exponent) - 16_383 - 63);
    if value.is_finite() { Some(value) } else { None }
}

/// Encode an `f64` back to the 80-bit extended form (the fixture
/// builder's twin of the parse — round-trip proofs).
///
/// # Errors
/// [`error::bad_structure`] when `rate` is not representable
/// (non-finite, zero/negative, or beyond the 64-bit mantissa's exact
/// integers at any exponent — practically: not a nonnegative integer
/// below 2^64).
pub fn f64_to_extended80(rate: f64) -> Result<[u8; 10], EngineError> {
    if !rate.is_finite()
        || rate <= 0.0
        || rate.fract() != 0.0
        || rate >= 18_446_744_073_709_552_960.0
    {
        return Err(error::bad_structure(0));
    }
    let bits = rate as u64;
    let lead = 64 - bits.leading_zeros(); // 1..=64
    let exponent = i32::try_from(lead).unwrap_or(64) + 16_383 - 1;
    if exponent >= 0x7FFF {
        return Err(error::bad_structure(0));
    }
    // Mantissa: the value left-justified into 64 bits (the integer
    // bit explicit at bit 63).
    let shift = 64 - lead;
    let mantissa = bits << shift;
    let mut out = [0u8; 10];
    let head = u16::try_from(exponent).unwrap_or(0);
    out[0] = (head >> 8) as u8;
    out[1] = (head & 0xFF) as u8;
    out[2..10].copy_from_slice(&mantissa.to_be_bytes());
    Ok(out)
}

/// Parse a COMM payload (classic 18-byte or AIFC with the
/// compression tail).
///
/// # Errors
/// - [`error::bad_structure`] for a payload below the classic floor,
///   a non-normalizable / non-integral / zero sample rate, or an
///   AIFC tail too short for its compression type.
/// - [`error::bad_comm`] for zero channels or a channel count past
///   the ceiling.
/// - [`error::unsupported_compression`] for compression types and
///   sample widths outside the PCM vocabulary — classic AIFF's
///   8-bit *signed* samples and 12-bit alike (the classic form has
///   no compression type, but its width vocabulary is exactly
///   `NONE`'s: big-endian 16/24/32), and `ulaw`, `ima4`, … in AIFC.
pub fn parse_comm(bytes: &[u8], aifc: bool, offset: u64) -> Result<CommChunk, EngineError> {
    let floor = if aifc {
        COMM_AIFC_MIN_BYTES
    } else {
        COMM_MIN_BYTES
    };
    if bytes.len() < floor {
        return Err(error::bad_structure(offset));
    }
    let channels = u16be(bytes, 0);
    let num_frames = u32be(bytes, 2);
    let sample_size = u16be(bytes, 6);
    let mut rate_bytes = [0u8; 10];
    rate_bytes.copy_from_slice(&bytes[8..18]);
    let rate = match extended80_to_f64(&rate_bytes) {
        Some(rate) => rate,
        None => return Err(error::bad_structure(offset)),
    };
    if rate <= 0.0 || rate.fract() != 0.0 || rate > f64::from(u32::MAX) {
        return Err(error::bad_structure(offset));
    }
    let sample_rate = rate as u32;

    if channels == 0 {
        return Err(error::bad_comm(offset));
    }
    if channels > 64 {
        return Err(error::bad_comm(offset));
    }

    let compression = if aifc {
        let type_bytes = [bytes[18], bytes[19], bytes[20], bytes[21]];
        let compression_type = FourCC::new(type_bytes);
        // The pstring name follows (1 length byte + chars, padded to
        // even); advisory display text — parsed for the skip math,
        // not kept.
        let codec = resolve_compression(compression_type, sample_size, offset)?;
        Some((compression_type, codec))
    } else {
        // Classic AIFF declares no compression type; its width
        // vocabulary is exactly NONE's (big-endian 16/24/32 — the
        // classic 8-bit *signed* sample has no registry descriptor:
        // the honest typed refusal, never a guessed codec).
        resolve_compression(COMPRESSION_NONE, sample_size, offset)?;
        None
    };
    Ok(CommChunk {
        channels,
        num_frames,
        sample_size,
        sample_rate,
        compression,
    })
}

/// Resolve an AIFC compression type + sample width to the codec
/// vocabulary.
///
/// # Errors
/// [`error::unsupported_compression`] for anything outside
/// {`NONE`, `sowt`, `fl32`, `fl64`} × {16, 24, 32}-bit integer /
/// {32, 64}-bit float widths.
fn resolve_compression(
    compression: FourCC,
    sample_size: u16,
    offset: u64,
) -> Result<AiffCodec, EngineError> {
    let width = |bits: &[u16]| -> Option<u16> { bits.iter().copied().find(|b| *b == sample_size) };
    match compression {
        COMPRESSION_NONE => match width(&[16, 24, 32]) {
            // Classic AIFF's 8-bit samples are signed — no u8/s8
            // registry vocabulary, the honest typed refusal.
            Some(bits) => Ok(AiffCodec::PcmBe {
                bytes_per_sample: bits / 8,
            }),
            None => Err(error::unsupported_compression(offset)),
        },
        COMPRESSION_SOWT => match width(&[16, 24, 32]) {
            Some(bits) => Ok(AiffCodec::PcmLe {
                bytes_per_sample: bits / 8,
            }),
            None => Err(error::unsupported_compression(offset)),
        },
        COMPRESSION_FL32 if sample_size == 32 => Ok(AiffCodec::F32Be),
        COMPRESSION_FL64 if sample_size == 64 => Ok(AiffCodec::F64Be),
        // The ffmpeg AIFC mapping for unsigned 8-bit (verified
        // against an ffmpeg `-f aifc` product: ffprobe reads `raw `
        // as pcm_u8; the committed aifc_raw_u8 fixture pins it).
        COMPRESSION_RAW if sample_size == 8 => Ok(AiffCodec::U8),
        _ => Err(error::unsupported_compression(offset)),
    }
}

/// Parse an SSND payload header: `offset:u32be` + `blockSize:u32be`,
/// both advisory to the walk (the sound data starts `offset` bytes
/// past the 8-byte pair; `blockSize` is Apple block alignment the
/// demuxer does not need).
///
/// # Errors
/// [`error::bad_structure`] for a payload below the 8-byte pair.
pub fn parse_ssnd(bytes: &[u8], offset: u64) -> Result<(u32, u32), EngineError> {
    if bytes.len() < 8 {
        return Err(error::bad_structure(offset));
    }
    Ok((u32be(bytes, 0), u32be(bytes, 4)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comm_bytes(channels: u16, frames: u32, bits: u16, rate: f64) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&channels.to_be_bytes());
        b.extend_from_slice(&frames.to_be_bytes());
        b.extend_from_slice(&bits.to_be_bytes());
        b.extend_from_slice(&match f64_to_extended80(rate) {
            Ok(ext) => ext,
            Err(e) => unreachable!("rate: {e}"),
        });
        b
    }

    #[test]
    fn extended80_roundtrips_the_corpus_rates() {
        for rate in [
            8_000.0, 11_025.0, 16_000.0, 22_050.0, 32_000.0, 44_100.0, 48_000.0, 88_200.0,
            96_000.0, 176_400.0, 192_000.0, 384_000.0, 1.0, 65_536.0,
        ] {
            let ext = match f64_to_extended80(rate) {
                Ok(ext) => ext,
                Err(e) => unreachable!("{rate}: {e}"),
            };
            match extended80_to_f64(&ext) {
                Some(back) => assert_eq!(back, rate, "{rate}"),
                None => unreachable!("{rate} must round-trip"),
            }
        }
    }

    #[test]
    fn extended80_rejects_non_normalized_shapes() {
        // Denormal: exponent 0, nonzero mantissa.
        let mut denorm = [0u8; 10];
        denorm[9] = 1;
        assert_eq!(extended80_to_f64(&denorm), None);
        // Apple pseudo-zero: exponent 0, sign set, all-zero mantissa.
        let pseudo_zero = [0x80, 0x00, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(extended80_to_f64(&pseudo_zero), None);
        // Infinity: exponent all-ones, zero mantissa.
        let inf = [0x7F, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(extended80_to_f64(&inf), None);
        // NaN: exponent all-ones, nonzero mantissa.
        let nan = [0x7F, 0xFF, 0, 0, 0, 0, 0, 0, 0, 1];
        assert_eq!(extended80_to_f64(&nan), None);
        // Unnormal: exponent in range, integer bit clear.
        let unnormal = [0x3F, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
        assert_eq!(extended80_to_f64(&unnormal), None);
        // The canonical 44100 exactly as ffmpeg writes it (verified
        // against an `-f aiff` product): exponent 0x400E, mantissa
        // 0xAC44 left-justified.
        let canon = [0x40, 0x0E, 0xAC, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(extended80_to_f64(&canon), Some(44_100.0));
    }

    #[test]
    fn classic_comm_parses() {
        let comm = match parse_comm(&comm_bytes(2, 1_000, 16, 44_100.0), false, 12) {
            Ok(comm) => comm,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(comm.channels, 2);
        assert_eq!(comm.num_frames, 1_000);
        assert_eq!(comm.sample_size, 16);
        assert_eq!(comm.sample_rate, 44_100);
        assert!(comm.compression.is_none());
    }

    #[test]
    fn aifc_comm_resolves_the_compression_vocabulary() {
        // NONE at 24-bit → pcm_s24be.
        let mut b = comm_bytes(2, 10, 24, 48_000.0);
        b.extend_from_slice(b"NONE");
        b.push(0); // empty pstring (its length byte)
        let comm = match parse_comm(&b, true, 12) {
            Ok(comm) => comm,
            Err(e) => unreachable!("{e}"),
        };
        let (ctype, codec) = comm
            .compression
            .unwrap_or_else(|| unreachable!("compression"));
        assert_eq!(ctype, COMPRESSION_NONE);
        assert_eq!(codec.descriptor(), "pcm_s24be");

        // sowt at 16-bit → pcm_s16le.
        let mut b = comm_bytes(1, 10, 16, 22_050.0);
        b.extend_from_slice(b"sowt");
        b.push(4);
        b.extend_from_slice(b"twos");
        b.push(0); // pstring pad to even
        let comm = match parse_comm(&b, true, 12) {
            Ok(comm) => comm,
            Err(e) => unreachable!("{e}"),
        };
        let (_, codec) = comm
            .compression
            .unwrap_or_else(|| unreachable!("compression"));
        assert_eq!(codec.descriptor(), "pcm_s16le");

        // fl32 → pcm_f32be.
        let mut b = comm_bytes(2, 10, 32, 48_000.0);
        b.extend_from_slice(b"fl32");
        b.push(0);
        let comm = match parse_comm(&b, true, 12) {
            Ok(comm) => comm,
            Err(e) => unreachable!("{e}"),
        };
        let (_, codec) = comm
            .compression
            .unwrap_or_else(|| unreachable!("compression"));
        assert_eq!(codec.descriptor(), "pcm_f32be");

        // fl64 → pcm_f64be.
        let mut b = comm_bytes(2, 10, 64, 48_000.0);
        b.extend_from_slice(b"fl64");
        b.push(0);
        let comm = match parse_comm(&b, true, 12) {
            Ok(comm) => comm,
            Err(e) => unreachable!("{e}"),
        };
        let (_, codec) = comm
            .compression
            .unwrap_or_else(|| unreachable!("compression"));
        assert_eq!(codec.descriptor(), "pcm_f64be");

        // raw  (ffmpeg's AIFC unsigned-8-bit) → pcm_u8.
        let mut b = comm_bytes(1, 10, 8, 44_100.0);
        b.extend_from_slice(b"raw ");
        b.push(0);
        let comm = match parse_comm(&b, true, 12) {
            Ok(comm) => comm,
            Err(e) => unreachable!("{e}"),
        };
        let (ctype, codec) = comm
            .compression
            .unwrap_or_else(|| unreachable!("compression"));
        assert_eq!(ctype, COMPRESSION_RAW);
        assert_eq!(codec.descriptor(), "pcm_u8");
        assert_eq!(codec.bytes_per_sample(), 1);

        // raw  at any other width: not a thing ffmpeg writes — refusal.
        let mut b = comm_bytes(1, 10, 16, 44_100.0);
        b.extend_from_slice(b"raw ");
        b.push(0);
        let err = match parse_comm(&b, true, 12) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.aiff.unsupported-compression");
    }

    #[test]
    fn unsupported_widths_and_compressions_are_typed() {
        // Classic 8-bit: signed — no vocabulary.
        let err = match parse_comm(&comm_bytes(1, 10, 8, 44_100.0), false, 12) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.aiff.unsupported-compression");

        // 12-bit NONE.
        let mut b = comm_bytes(1, 10, 12, 44_100.0);
        b.extend_from_slice(b"NONE");
        b.push(0);
        let err = match parse_comm(&b, true, 12) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.aiff.unsupported-compression");

        // ulaw.
        let mut b = comm_bytes(1, 10, 16, 8_000.0);
        b.extend_from_slice(b"ulaw");
        b.push(0);
        let err = match parse_comm(&b, true, 12) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.aiff.unsupported-compression");

        // ima4.
        let mut b = comm_bytes(2, 10, 16, 44_100.0);
        b.extend_from_slice(b"ima4");
        b.push(0);
        let err = match parse_comm(&b, true, 12) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.aiff.unsupported-compression");
    }

    #[test]
    fn broken_comms_are_typed() {
        // Too short for the classic floor.
        let err = match parse_comm(&comm_bytes(1, 1, 16, 44_100.0)[..17], false, 12) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.aiff.bad-structure");

        // Zero channels.
        let err = match parse_comm(&comm_bytes(0, 1, 16, 44_100.0), false, 12) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.aiff.bad-comm");

        // A non-integral rate (44100.5), hand-encoded in the 80-bit
        // extended form (the encoder refuses non-integral rates by
        // design, so the corrupt shape is built directly): exponent
        // 16383+15 = 0x400E, mantissa 0xAC44800000000000 — the
        // canonical 44100 with the fraction bit set.
        let mut b = comm_bytes(1, 1, 16, 44_100.0);
        b[8..18].copy_from_slice(&[0x40, 0x0E, 0xAC, 0x44, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00]);
        let err = match parse_comm(&b, false, 12) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.aiff.bad-structure");

        // AIFC tail shorter than the 4-byte type.
        let mut b = comm_bytes(1, 1, 16, 44_100.0);
        b.extend_from_slice(b"NO");
        let err = match parse_comm(&b, true, 12) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.aiff.bad-structure");
    }

    #[test]
    fn ssnd_header_parses() {
        let mut b = Vec::new();
        b.extend_from_slice(&0u32.to_be_bytes());
        b.extend_from_slice(&0u32.to_be_bytes());
        match parse_ssnd(&b, 100) {
            Ok((offset, block)) => {
                assert_eq!(offset, 0);
                assert_eq!(block, 0);
            }
            Err(e) => unreachable!("{e}"),
        }
        let err = match parse_ssnd(&b[..7], 100) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.aiff.bad-structure");
    }
}
