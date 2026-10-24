//! The `fmt ` chunk parser (card 5.A2): WAVEFORMATEX /
//! WAVEFORMATEXTENSIBLE → the stream facts.
//!
//! The PCM half accepts exactly the blobs the engine's PCM decoder
//! accepts (`codecs::audio::pcm::decoder::parse_waveformatex`'s rules,
//! read and mirrored — the demux layer must not depend on codecs,
//! `boundaries.toml`; **one accepted-set, no demux-ok/decode-fail
//! divergence** for PCM). The registry descriptor string is the decode
//! seam: [`WaveFormat::codec_descriptor`] emits the doc 04 §3.1
//! vocabulary (`pcm_u8`, `pcm_s16le`, …) the PCM decoder parses.
//!
//! Beyond PCM, the parser recognizes the two non-PCM payloads the
//! randomized real corpus actually carries inside WAV containers —
//! **IMA ADPCM** (`wFormatTag 0x0011`), **MS ADPCM** (`0x0002`) and
//! **FLAC-in-WAV** (the `KSDATAFORMAT_SUBTYPE_FLAC` SubFormat GUID,
//! `{0000F1AC-0000-0010-8000-00AA00389B71}`, whose 34-byte FLAC
//! STREAMINFO rides the `fmt ` vendor tail). These enumerate a track
//! with the honest registry name (`adpcm_ima_wav`, `adpcm_ms`,
//! `flac`); the engine reports "no decoder" until their codec cards
//! land — the demuxer never pretends to decode them, and never
//! refuses a WAV it can faithfully packetize `[PROPOSAL: the
//! block-aligned ADPCM packetization contract is the seam the future
//! `codecs::audio::adpcm` card consumes; FLAC-in-WAV packets are
//! byte-quantum, not frame-aligned — frame-aligned packetization
//! needs the FLAC frame-header grammar and belongs with 5.A1's
//! decoder]`.

use media_types::EngineError;

use super::error;

/// Maximum `fmt ` payload trusted into memory
/// `[PROPOSAL: 4 KiB — the parse reads at most the 74 bytes the
/// FLAC-in-WAV form needs; a cbSize vendor tail beyond this is
/// corruption, skipped positionally]`.
pub const MAX_FMT_BYTES: usize = 4096;

/// Maximum accepted channel count `[PROPOSAL: 64 — the engine's
/// `MAX_PCM_CHANNELS` decode ceiling (W2-15 g1); parity here keeps the
/// accepted-set identical]`.
pub const MAX_CHANNELS: u16 = 64;

/// `wFormatTag` for MS ADPCM.
pub const TAG_ADPCM_MS: u16 = 0x0002;
/// `wFormatTag` for integer PCM.
pub const TAG_PCM: u16 = 1;
/// `wFormatTag` for IEEE float PCM.
pub const TAG_IEEE_FLOAT: u16 = 3;
/// `wFormatTag` for IMA ADPCM (the WAV variant).
pub const TAG_ADPCM_IMA_WAV: u16 = 0x0011;
/// `wFormatTag` for WAVEFORMATEXTENSIBLE — the real format lives in
/// the 16-byte SubFormat GUID at offset 24 of the ≥ 40-byte form.
pub const TAG_EXTENSIBLE: u16 = 0xFFFE;

/// Minimum `fmt ` payload: the 16-byte PCMWAVEFORMAT.
pub const MIN_FMT_BYTES: usize = 16;
/// The WAVEFORMATEX adds `cbSize` (18 bytes).
pub const WAVEFORMATEX_BYTES: usize = 18;
/// The extensible form: WAVEFORMATEX + validBits + channelMask +
/// SubFormat GUID (40 bytes).
pub const EXTENSIBLE_BYTES: usize = 40;
/// The FLAC-in-WAV form: extensible + the 34-byte FLAC STREAMINFO in
/// the vendor tail (74 bytes — the shape ffmpeg writes).
pub const FLAC_WAV_BYTES: usize = 74;
/// The FLAC STREAMINFO length (the FLAC spec's fixed metadata block).
pub const FLAC_STREAMINFO_BYTES: usize = 34;

/// `KSDATAFORMAT_SUBTYPE_PCM` (`00000001-0000-0010-8000-00AA00389B71`).
pub const SUBFORMAT_PCM_GUID: [u8; 16] = [
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];
/// `KSDATAFORMAT_SUBTYPE_IEEE_FLOAT`
/// (`00000003-0000-0010-8000-00AA00389B71`).
pub const SUBFORMAT_IEEE_FLOAT_GUID: [u8; 16] = [
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];
/// `KSDATAFORMAT_SUBTYPE_FLAC` (`0000F1AC-0000-0010-8000-00AA00389B71`)
/// — the Microsoft-registered GUID for FLAC frames inside a WAVE
/// `data` chunk (mmreg.h; ffmpeg writes and reads it).
pub const SUBFORMAT_FLAC_GUID: [u8; 16] = [
    0xAC, 0xF1, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

/// The unpacked-PCM sample representation a `fmt ` chunk declares —
/// the demuxer-side mirror of the registry's PCM descriptor
/// vocabulary (doc 04 §3.1). WAVE PCM byte order is little-endian by
/// container definition, so width + signedness is the whole
/// vocabulary — no endianness axis (the AIFF twin in
/// [`crate::aiff`] carries the BE half).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcmSample {
    /// Tag `1`, 8-bit container: unsigned (byte-order-free).
    U8,
    /// Tag `1`, 16-bit container: signed little-endian.
    S16Le,
    /// Tag `1`, 24-bit container: signed little-endian (3-byte groups).
    S24Le,
    /// Tag `1`, 32-bit container: signed little-endian.
    S32Le,
    /// Tag `3` (or the float SubFormat GUID), 32-bit: IEEE 754.
    F32Le,
    /// Tag `3` (or the float SubFormat GUID), 64-bit: IEEE 754.
    F64Le,
}

impl PcmSample {
    /// The registry descriptor string (doc 04 §3.1 table names — the
    /// codec-side `PcmFormat` parses exactly this vocabulary).
    pub fn descriptor(self) -> &'static str {
        match self {
            PcmSample::U8 => "pcm_u8",
            PcmSample::S16Le => "pcm_s16le",
            PcmSample::S24Le => "pcm_s24le",
            PcmSample::S32Le => "pcm_s32le",
            PcmSample::F32Le => "pcm_f32le",
            PcmSample::F64Le => "pcm_f64le",
        }
    }

    /// Container bits per sample (`wBitsPerSample`).
    pub fn bits_per_sample(self) -> u16 {
        match self {
            PcmSample::U8 => 8,
            PcmSample::S16Le => 16,
            PcmSample::S24Le => 24,
            PcmSample::S32Le | PcmSample::F32Le => 32,
            PcmSample::F64Le => 64,
        }
    }

    /// Bytes per sample in the container (bits ÷ 8).
    pub fn bytes_per_sample(self) -> u16 {
        match self {
            PcmSample::U8 => 1,
            PcmSample::S16Le => 2,
            PcmSample::S24Le => 3,
            PcmSample::S32Le | PcmSample::F32Le => 4,
            PcmSample::F64Le => 8,
        }
    }

    /// Map a resolved format tag + `wBitsPerSample` to the sample
    /// vocabulary. `None` is the typed refusal for every impossible
    /// combination (tag 1 with 20 bits, tag 3 with 8 bits …).
    #[must_use]
    pub fn from_wave_tag(tag: u16, bits_per_sample: u16) -> Option<PcmSample> {
        match tag {
            TAG_PCM => match bits_per_sample {
                8 => Some(PcmSample::U8),
                16 => Some(PcmSample::S16Le),
                24 => Some(PcmSample::S24Le),
                32 => Some(PcmSample::S32Le),
                _ => None,
            },
            TAG_IEEE_FLOAT => match bits_per_sample {
                32 => Some(PcmSample::F32Le),
                64 => Some(PcmSample::F64Le),
                _ => None,
            },
            _ => None,
        }
    }
}

/// The payload codec a `fmt ` chunk declares — what rides the `data`
/// chunk and how the demuxer frames it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavCodec {
    /// Integer or float PCM (tags `1`/`3`, or the extensible GUIDs):
    /// interleaved sample frames of `channels × width` bytes, one
    /// sample per channel.
    Pcm {
        /// The sample representation.
        sample: PcmSample,
    },
    /// IMA ADPCM (tag `0x0011`): self-contained blocks of
    /// `nBlockAlign` bytes, each decoding to `samples_per_block`
    /// sample frames.
    AdpcmImaWav {
        /// `wSamplesPerBlock` from the fmt extension, or the
        /// block-size formula's derivation.
        samples_per_block: u32,
    },
    /// MS ADPCM (tag `0x0002`): the same block framing, the MS
    /// predictor/coefficient block grammar.
    AdpcmMs {
        /// `wSamplesPerBlock` (declared or derived).
        samples_per_block: u32,
    },
    /// FLAC frames inside the `data` chunk (the extensible FLAC
    /// SubFormat GUID). `streaminfo` carries the 34-byte FLAC
    /// STREAMINFO from the fmt vendor tail when present — the
    /// decoder-config seam for the FLAC codec card.
    Flac {
        /// The FLAC STREAMINFO block (`fmt [40..74]`), when the fmt
        /// chunk carried it.
        streaminfo: Option<[u8; FLAC_STREAMINFO_BYTES]>,
    },
}

impl WavCodec {
    /// The registry descriptor (doc 04 §3.1 vocabulary; the engine
    /// looks the decoder up by exactly this string).
    pub fn descriptor(&self) -> &'static str {
        match self {
            WavCodec::Pcm { sample } => sample.descriptor(),
            WavCodec::AdpcmImaWav { .. } => "adpcm_ima_wav",
            WavCodec::AdpcmMs { .. } => "adpcm_ms",
            WavCodec::Flac { .. } => "flac",
        }
    }

    /// Bytes of one packetization unit: a sample frame (PCM), one
    /// ADPCM block, or a single byte (FLAC — no frame grammar here).
    /// Always ≥ 1 for an accepted format.
    #[must_use]
    pub fn unit_size(&self, channels: u16, block_align: u16) -> u32 {
        match self {
            WavCodec::Pcm { sample } => u32::from(channels) * u32::from(sample.bytes_per_sample()),
            WavCodec::AdpcmImaWav { .. } | WavCodec::AdpcmMs { .. } => u32::from(block_align),
            WavCodec::Flac { .. } => 1,
        }
    }

    /// Sample frames per packetization unit. `None` for FLAC (the
    /// count needs the frame-header grammar — the honest unknown).
    #[must_use]
    pub fn samples_per_unit(&self) -> Option<u32> {
        match self {
            WavCodec::Pcm { .. } => Some(1),
            WavCodec::AdpcmImaWav { samples_per_block }
            | WavCodec::AdpcmMs { samples_per_block } => Some(*samples_per_block),
            WavCodec::Flac { .. } => None,
        }
    }
}

/// The `fmt ` facts a decoder needs — the v0 seam for the
/// decoder-config handoff (the AVI `StreamFormat::Pcm` precedent:
/// the L0 [`media_types::Track`] deliberately carries no codec
/// parameters; parameters ride the format-specific demuxer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaveFormat {
    /// The resolved format tag: `1`/`3`/`0x0002`/`0x0011` as declared,
    /// and for extensible (`0xFFFE`) the SubFormat GUID's resolution.
    pub format_tag: u16,
    /// Channel count (`nChannels`).
    pub channels: u16,
    /// Samples per second (`nSamplesPerSec`).
    pub samples_per_sec: u32,
    /// Bytes per block (`nBlockAlign`) — for PCM the parse rejects any
    /// declaration that differs from `channels × bytes_per_sample`
    /// (the decoder re-checks the same rule on the verbatim blob).
    pub block_align: u16,
    /// Container bits per sample (`wBitsPerSample`; 4 for ADPCM).
    pub bits_per_sample: u16,
    /// Extensible only: `wValidBitsPerSample` (the [VERIFY] oddity —
    /// a value below `bits_per_sample` has no decode vocabulary and
    /// is typed-refused).
    pub valid_bits_per_sample: Option<u16>,
    /// Extensible only: `dwChannelMask` (speaker positions; recorded,
    /// never fabricated into a layout).
    pub channel_mask: Option<u32>,
    /// The payload codec (PCM sample representation, ADPCM block
    /// facts, or FLAC + STREAMINFO).
    pub codec: WavCodec,
}

impl WaveFormat {
    /// The registry vocabulary descriptor for this format — the
    /// string the L0 `Track.codec` carries and the decoder provider
    /// resolves.
    pub fn codec_descriptor(&self) -> &'static str {
        self.codec.descriptor()
    }

    /// Bytes of one packetization unit (see [`WavCodec::unit_size`]).
    #[must_use]
    pub fn unit_size(&self) -> u32 {
        self.codec.unit_size(self.channels, self.block_align)
    }

    /// Sample frames per packetization unit
    /// (see [`WavCodec::samples_per_unit`]).
    #[must_use]
    pub fn samples_per_unit(&self) -> Option<u32> {
        self.codec.samples_per_unit()
    }
}

fn u16le(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32le(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// Derive IMA ADPCM's samples-per-block from the block geometry (the
/// IMA's own specified formula, the one ffmpeg's WAV muxer writes):
/// each block carries a 4-byte-per-channel preamble, then packed
/// 4-bit nibbles — 2 nibbles per byte, each nibble one sample per
/// channel, plus the preamble's own predictor sample:
/// `((block_align − 4·channels) · 2) / channels + 1`
/// (block 1024, stereo → 1017; block 1024, mono → 2041).
fn ima_samples_per_block(block_align: u32, channels: u32) -> Option<u32> {
    let header = channels.checked_mul(4)?;
    if block_align <= header {
        return None;
    }
    let nibbles = (block_align - header).checked_mul(2)?;
    (nibbles / channels).checked_add(1)
}

/// Derive MS ADPCM's samples-per-block: each block carries a
/// 7-byte-per-channel preamble, then per-channel bytes of packed
/// 4-bit nibble pairs — `(block_align − 7·channels) / channels · 2 + 2`
/// (block 1024, stereo → 1012, the corpus 0102 declaration).
fn ms_samples_per_block(block_align: u32, channels: u32) -> Option<u32> {
    let header = channels.checked_mul(7)?;
    if block_align <= header {
        return None;
    }
    let nibble_pairs = (block_align - header) / channels;
    nibble_pairs.checked_mul(2)?.checked_add(2)
}

/// Parse a `fmt ` payload (PCMWAVEFORMAT / WAVEFORMATEX /
/// WAVEFORMATEXTENSIBLE) into the stream facts.
///
/// # Errors
/// - [`error::bad_structure`] when the payload is shorter than the
///   16-byte PCMWAVEFORMAT floor, an extensible declaration is
///   shorter than its 40-byte body (the "0xFFFE without extensible
///   body" AC path), or an ADPCM declaration is too short for its
///   own samples-per-block field and cannot derive it.
/// - [`error::unsupported_format`] for unknown tags, SubFormat GUIDs
///   outside {PCM, IEEE float, FLAC} and bit depths with no registry
///   descriptor (20-bit, 24-valid-in-32, float-8 …).
/// - [`error::bad_format_params`] for zero channels, zero rate,
///   channels past the ceiling, or an `nBlockAlign` that does not
///   equal `channels × bytes_per_sample` (the decoder re-checks the
///   same rule on the verbatim blob — accepting here would mint a
///   stream the decode path always rejects).
pub fn parse_fmt(bytes: &[u8], offset: u64) -> Result<WaveFormat, EngineError> {
    if bytes.len() < MIN_FMT_BYTES {
        return Err(error::bad_structure(offset));
    }
    let tag = u16le(bytes, 0);
    let channels = u16le(bytes, 2);
    let rate = u32le(bytes, 4);
    let block_align = u16le(bytes, 12);
    let bits = u16le(bytes, 14);

    if channels == 0 {
        return Err(error::bad_format_params(offset));
    }
    if channels > MAX_CHANNELS {
        return Err(error::bad_format_params(offset));
    }
    if rate == 0 {
        return Err(error::bad_format_params(offset));
    }

    // Resolve the extensible wrapper to its plain tag first (the
    // decoder's parse order: GUID before any width/align check).
    let (resolved_tag, valid_bits, channel_mask) = if tag == TAG_EXTENSIBLE {
        if bytes.len() < EXTENSIBLE_BYTES {
            // The "0xFFFE without extensible body" AC path: the GUID
            // cannot physically fit — a length lie.
            return Err(error::bad_structure(offset));
        }
        let guid = &bytes[24..40];
        let resolved = if guid == SUBFORMAT_PCM_GUID {
            TAG_PCM
        } else if guid == SUBFORMAT_IEEE_FLOAT_GUID {
            TAG_IEEE_FLOAT
        } else if guid == SUBFORMAT_FLAC_GUID {
            // FLAC-in-WAV: the vendor tail past the 40-byte
            // extensible body carries the 34-byte STREAMINFO
            // (ffmpeg's shape); a missing tail is tolerated — the
            // GUID itself names the codec.
            let streaminfo = if bytes.len() >= FLAC_WAV_BYTES {
                let mut block = [0u8; FLAC_STREAMINFO_BYTES];
                block.copy_from_slice(&bytes[EXTENSIBLE_BYTES..FLAC_WAV_BYTES]);
                Some(block)
            } else {
                None
            };
            let codec = WavCodec::Flac { streaminfo };
            return Ok(WaveFormat {
                format_tag: TAG_EXTENSIBLE,
                channels,
                samples_per_sec: rate,
                block_align,
                bits_per_sample: bits,
                valid_bits_per_sample: Some(u16le(bytes, 18)),
                channel_mask: Some(u32le(bytes, 20)),
                codec,
            });
        } else {
            return Err(error::unsupported_format(offset));
        };
        (resolved, Some(u16le(bytes, 18)), Some(u32le(bytes, 20)))
    } else {
        (tag, None, None)
    };

    // The non-PCM tags the corpus carries: ADPCM block framing only
    // (the block grammar is the codec card's; the container facts —
    // block size, samples per block — are ours).
    if resolved_tag == TAG_ADPCM_IMA_WAV || resolved_tag == TAG_ADPCM_MS {
        let is_ima = resolved_tag == TAG_ADPCM_IMA_WAV;
        if block_align == 0 {
            return Err(error::bad_format_params(offset));
        }
        // `wSamplesPerBlock` sits at fmt[18] when the writer supplied
        // it (ffmpeg does); otherwise derive it from the block
        // geometry (the format's own formulas).
        let declared = if bytes.len() >= WAVEFORMATEX_BYTES + 2 {
            Some(u32::from(u16le(bytes, WAVEFORMATEX_BYTES)))
        } else {
            None
        };
        let derived = if is_ima {
            ima_samples_per_block(u32::from(block_align), u32::from(channels))
        } else {
            ms_samples_per_block(u32::from(block_align), u32::from(channels))
        };
        let samples_per_block = match (declared, derived) {
            (Some(declared), Some(derived)) => {
                // A declaration that disagrees with the block
                // geometry is a lie about arithmetic, not a variant:
                // trust the geometry, count nothing silently — the
                // typed refusal (the ADPCM decoder would decode the
                // declared count against the same bytes).
                if declared == 0 || declared > derived {
                    return Err(error::bad_format_params(offset));
                }
                declared
            }
            (Some(0), None) | (None, None) => {
                return Err(error::bad_format_params(offset));
            }
            (Some(declared), None) => {
                if declared == 0 {
                    return Err(error::bad_format_params(offset));
                }
                declared
            }
            (None, Some(derived)) => derived,
        };
        let codec = if is_ima {
            WavCodec::AdpcmImaWav { samples_per_block }
        } else {
            WavCodec::AdpcmMs { samples_per_block }
        };
        return Ok(WaveFormat {
            format_tag: resolved_tag,
            channels,
            samples_per_sec: rate,
            block_align,
            bits_per_sample: bits,
            valid_bits_per_sample: None,
            channel_mask: None,
            codec,
        });
    }

    let sample = match PcmSample::from_wave_tag(resolved_tag, bits) {
        Some(sample) => sample,
        // Every other width is a typed refusal (20-bit PCM, float-8,
        // unknown tags …): no decode vocabulary, no fabrication.
        None => return Err(error::unsupported_format(offset)),
    };

    // The [VERIFY] valid-bits oddity: extensible writers may declare
    // fewer valid bits than container bits (20-in-24, 24-in-32). The
    // decode vocabulary cannot express the padding — the honest path
    // is the typed refusal, not a lossy passthrough.
    if let Some(valid) = valid_bits {
        if valid == 0 || valid > bits {
            return Err(error::bad_format_params(offset));
        }
        if valid != bits {
            return Err(error::unsupported_format(offset));
        }
    }

    // The align consistency the decoder re-checks on the verbatim
    // blob: nBlockAlign == channels × bytes_per_sample, exactly.
    let frame_size = u32::from(channels) * u32::from(sample.bytes_per_sample());
    if u32::from(block_align) != frame_size {
        return Err(error::bad_format_params(offset));
    }

    Ok(WaveFormat {
        format_tag: resolved_tag,
        channels,
        samples_per_sec: rate,
        block_align,
        bits_per_sample: bits,
        valid_bits_per_sample: valid_bits,
        channel_mask,
        codec: WavCodec::Pcm { sample },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcm16() -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&TAG_PCM.to_le_bytes()); // tag
        b.extend_from_slice(&2u16.to_le_bytes()); // channels
        b.extend_from_slice(&44_100u32.to_le_bytes()); // rate
        b.extend_from_slice(&176_400u32.to_le_bytes()); // avg bytes/sec
        b.extend_from_slice(&4u16.to_le_bytes()); // block align
        b.extend_from_slice(&16u16.to_le_bytes()); // bits
        b
    }

    #[test]
    fn parses_plain_pcm16() {
        let fmt = match parse_fmt(&pcm16(), 44) {
            Ok(fmt) => fmt,
            Err(e) => unreachable!("parse: {e}"),
        };
        assert_eq!(fmt.format_tag, TAG_PCM);
        assert_eq!(fmt.channels, 2);
        assert_eq!(fmt.samples_per_sec, 44_100);
        assert_eq!(
            fmt.codec,
            WavCodec::Pcm {
                sample: PcmSample::S16Le
            }
        );
        assert_eq!(fmt.unit_size(), 4);
        assert_eq!(fmt.samples_per_unit(), Some(1));
        assert_eq!(fmt.codec_descriptor(), "pcm_s16le");
        assert!(fmt.valid_bits_per_sample.is_none());
        assert!(fmt.channel_mask.is_none());
    }

    #[test]
    fn parses_extensible_pcm24_like_ffmpeg_writes() {
        // The exact shape ffmpeg emits for 24-bit stereo (the
        // wav_pcm_s24le_48k_st fixture): 40-byte extensible, cbSize
        // 22, validBits 24, mask 0x3, PCM SubFormat GUID.
        let mut b = Vec::new();
        b.extend_from_slice(&TAG_EXTENSIBLE.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&48_000u32.to_le_bytes());
        b.extend_from_slice(&288_000u32.to_le_bytes());
        b.extend_from_slice(&6u16.to_le_bytes());
        b.extend_from_slice(&24u16.to_le_bytes());
        b.extend_from_slice(&22u16.to_le_bytes()); // cbSize
        b.extend_from_slice(&24u16.to_le_bytes()); // validBits
        b.extend_from_slice(&0x3u32.to_le_bytes()); // channel mask
        b.extend_from_slice(&SUBFORMAT_PCM_GUID);
        let fmt = match parse_fmt(&b, 44) {
            Ok(fmt) => fmt,
            Err(e) => unreachable!("parse: {e}"),
        };
        assert_eq!(fmt.format_tag, TAG_PCM);
        assert_eq!(
            fmt.codec,
            WavCodec::Pcm {
                sample: PcmSample::S24Le
            }
        );
        assert_eq!(fmt.valid_bits_per_sample, Some(24));
        assert_eq!(fmt.channel_mask, Some(0x3));
        assert_eq!(fmt.codec_descriptor(), "pcm_s24le");
    }

    #[test]
    fn extensible_float_guid_maps_to_f32() {
        let mut b = Vec::new();
        b.extend_from_slice(&TAG_EXTENSIBLE.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&48_000u32.to_le_bytes());
        b.extend_from_slice(&384_000u32.to_le_bytes());
        b.extend_from_slice(&8u16.to_le_bytes());
        b.extend_from_slice(&32u16.to_le_bytes());
        b.extend_from_slice(&22u16.to_le_bytes());
        b.extend_from_slice(&32u16.to_le_bytes());
        b.extend_from_slice(&0x3u32.to_le_bytes());
        b.extend_from_slice(&SUBFORMAT_IEEE_FLOAT_GUID);
        let fmt = match parse_fmt(&b, 44) {
            Ok(fmt) => fmt,
            Err(e) => unreachable!("parse: {e}"),
        };
        assert_eq!(
            fmt.codec,
            WavCodec::Pcm {
                sample: PcmSample::F32Le
            }
        );
        assert_eq!(fmt.codec_descriptor(), "pcm_f32le");
    }

    #[test]
    fn extensible_flac_guid_exposes_streaminfo_and_descriptor() {
        // The 0016 corpus shape: 74-byte fmt, cbSize 56, STREAMINFO
        // at [40..74].
        let mut b = Vec::new();
        b.extend_from_slice(&TAG_EXTENSIBLE.to_le_bytes());
        b.extend_from_slice(&4u16.to_le_bytes());
        b.extend_from_slice(&96_000u32.to_le_bytes());
        b.extend_from_slice(&16_000u32.to_le_bytes());
        b.extend_from_slice(&8u16.to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(&56u16.to_le_bytes()); // cbSize
        b.extend_from_slice(&16u16.to_le_bytes()); // validBits
        b.extend_from_slice(&0x107u32.to_le_bytes()); // channel mask
        b.extend_from_slice(&SUBFORMAT_FLAC_GUID);
        let streaminfo: Vec<u8> = (0..FLAC_STREAMINFO_BYTES as u8).collect();
        b.extend_from_slice(&streaminfo);
        let fmt = match parse_fmt(&b, 44) {
            Ok(fmt) => fmt,
            Err(e) => unreachable!("parse: {e}"),
        };
        assert_eq!(fmt.codec_descriptor(), "flac");
        assert_eq!(fmt.unit_size(), 1);
        assert_eq!(fmt.samples_per_unit(), None);
        match fmt.codec {
            WavCodec::Flac {
                streaminfo: Some(block),
            } => assert_eq!(block.as_slice(), &streaminfo[..]),
            other => unreachable!("flac codec expected, got {other:?}"),
        }
        // A GUID without the vendor tail still names the codec — the
        // STREAMINFO stays honestly absent.
        let short = &b[..EXTENSIBLE_BYTES];
        let fmt = match parse_fmt(short, 44) {
            Ok(fmt) => fmt,
            Err(e) => unreachable!("parse: {e}"),
        };
        match fmt.codec {
            WavCodec::Flac { streaminfo: None } => {}
            other => unreachable!("tailless flac expected, got {other:?}"),
        }
    }

    #[test]
    fn extensible_without_body_is_a_structural_lie() {
        // 0xFFFE with an 18-byte body: the AC's "unknown-format tag
        // (0xFFFE without extensible body)" path.
        let mut b = pcm16();
        b[0..2].copy_from_slice(&TAG_EXTENSIBLE.to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes()); // cbSize = 0
        let err = match parse_fmt(&b, 44) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.wav.bad-structure");
    }

    #[test]
    fn unknown_tags_and_depths_are_typed_refusals() {
        // 20-bit integer PCM: no registry descriptor.
        let mut b = pcm16();
        b[14..16].copy_from_slice(&20u16.to_le_bytes());
        b[12..14].copy_from_slice(&6u16.to_le_bytes());
        let err = match parse_fmt(&b, 44) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.wav.unsupported-format");

        // Float 8-bit: nonsense width for the tag.
        let mut b = pcm16();
        b[0..2].copy_from_slice(&TAG_IEEE_FLOAT.to_le_bytes());
        b[14..16].copy_from_slice(&8u16.to_le_bytes());
        b[12..14].copy_from_slice(&2u16.to_le_bytes());
        let err = match parse_fmt(&b, 44) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.wav.unsupported-format");

        // A wholly unknown tag.
        let mut b = pcm16();
        b[0..2].copy_from_slice(&0x0055u16.to_le_bytes());
        let err = match parse_fmt(&b, 44) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.wav.unsupported-format");
    }

    #[test]
    fn adpcm_tags_parse_their_block_framing() {
        // IMA ADPCM, the 0042 corpus shape: fmt 20 bytes, declared
        // samples-per-block 1017, block 1024, 2 channels.
        let mut b = Vec::new();
        b.extend_from_slice(&TAG_ADPCM_IMA_WAV.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&44_056u32.to_le_bytes());
        b.extend_from_slice(&16_000u32.to_le_bytes());
        b.extend_from_slice(&1_024u16.to_le_bytes());
        b.extend_from_slice(&4u16.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes()); // cbSize
        b.extend_from_slice(&1_017u16.to_le_bytes()); // samples/block
        let fmt = match parse_fmt(&b, 44) {
            Ok(fmt) => fmt,
            Err(e) => unreachable!("parse: {e}"),
        };
        assert_eq!(fmt.codec_descriptor(), "adpcm_ima_wav");
        assert_eq!(
            fmt.codec,
            WavCodec::AdpcmImaWav {
                samples_per_block: 1_017
            }
        );
        assert_eq!(fmt.unit_size(), 1_024);
        assert_eq!(fmt.samples_per_unit(), Some(1_017));

        // MS ADPCM, the 0102 corpus shape: fmt 50 bytes, declared
        // 1012 samples/block at fmt[18] (verified against the corpus
        // bytes `f4 03`), 7 coefficient pairs. Geometry check:
        // (1024 − 7·2)/2 · 2 + 2 = 1012.
        let mut b = Vec::new();
        b.extend_from_slice(&TAG_ADPCM_MS.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&32_000u32.to_le_bytes());
        b.extend_from_slice(&16_000u32.to_le_bytes());
        b.extend_from_slice(&1_024u16.to_le_bytes());
        b.extend_from_slice(&4u16.to_le_bytes());
        b.extend_from_slice(&32u16.to_le_bytes()); // cbSize
        b.extend_from_slice(&1_012u16.to_le_bytes()); // samples/block
        b.extend_from_slice(&7u16.to_le_bytes()); // coefficient sets
        b.extend(&[0u8; 28]); // the 7 (delta, delta) pairs
        let fmt = match parse_fmt(&b, 44) {
            Ok(fmt) => fmt,
            Err(e) => unreachable!("parse: {e}"),
        };
        assert_eq!(fmt.codec_descriptor(), "adpcm_ms");
        assert_eq!(
            fmt.codec,
            WavCodec::AdpcmMs {
                samples_per_block: 1_012
            }
        );

        // A declared IMA count above the block geometry's ceiling is
        // an arithmetic lie — typed bad-fmt.
        let mut b = Vec::new();
        b.extend_from_slice(&TAG_ADPCM_IMA_WAV.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&44_056u32.to_le_bytes());
        b.extend_from_slice(&16_000u32.to_le_bytes());
        b.extend_from_slice(&1_024u16.to_le_bytes());
        b.extend_from_slice(&4u16.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&4_000u16.to_le_bytes()); // impossible
        let err = match parse_fmt(&b, 44) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.wav.bad-fmt");

        // Without the declared field the formula derives it:
        // ((1024 − 4·2)·2)/2 + 1 = 1017 for IMA.
        let mut b = Vec::new();
        b.extend_from_slice(&TAG_ADPCM_IMA_WAV.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&44_056u32.to_le_bytes());
        b.extend_from_slice(&16_000u32.to_le_bytes());
        b.extend_from_slice(&1_024u16.to_le_bytes());
        b.extend_from_slice(&4u16.to_le_bytes());
        let fmt = match parse_fmt(&b, 44) {
            Ok(fmt) => fmt,
            Err(e) => unreachable!("parse: {e}"),
        };
        assert_eq!(
            fmt.codec,
            WavCodec::AdpcmImaWav {
                samples_per_block: 1_017
            }
        );

        // MS formula: (1024 − 7·2)/2 · 2 + 2 = 1012.
        let mut b = Vec::new();
        b.extend_from_slice(&TAG_ADPCM_MS.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&32_000u32.to_le_bytes());
        b.extend_from_slice(&16_000u32.to_le_bytes());
        b.extend_from_slice(&1_024u16.to_le_bytes());
        b.extend_from_slice(&4u16.to_le_bytes());
        let fmt = match parse_fmt(&b, 44) {
            Ok(fmt) => fmt,
            Err(e) => unreachable!("parse: {e}"),
        };
        assert_eq!(
            fmt.codec,
            WavCodec::AdpcmMs {
                samples_per_block: 1_012
            }
        );
    }

    #[test]
    fn valid_bits_below_container_bits_are_refused() {
        // 20 valid in 24: real-writer shape, no decode vocabulary.
        let mut b = Vec::new();
        b.extend_from_slice(&TAG_EXTENSIBLE.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&48_000u32.to_le_bytes());
        b.extend_from_slice(&288_000u32.to_le_bytes());
        b.extend_from_slice(&6u16.to_le_bytes());
        b.extend_from_slice(&24u16.to_le_bytes());
        b.extend_from_slice(&22u16.to_le_bytes());
        b.extend_from_slice(&20u16.to_le_bytes()); // validBits 20
        b.extend_from_slice(&0x3u32.to_le_bytes());
        b.extend_from_slice(&SUBFORMAT_PCM_GUID);
        let err = match parse_fmt(&b, 44) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.wav.unsupported-format");
    }

    #[test]
    fn zero_or_inconsistent_params_are_bad_fmt() {
        // Zero rate.
        let mut b = pcm16();
        b[4..8].copy_from_slice(&0u32.to_le_bytes());
        let err = match parse_fmt(&b, 44) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.wav.bad-fmt");

        // Zero channels.
        let mut b = pcm16();
        b[2..4].copy_from_slice(&0u16.to_le_bytes());
        let err = match parse_fmt(&b, 44) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.wav.bad-fmt");

        // Padded 24-bit align (channels × 4 while bits = 24): the
        // decode path would always reject the blob — refused here.
        let mut b = Vec::new();
        b.extend_from_slice(&TAG_PCM.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&48_000u32.to_le_bytes());
        b.extend_from_slice(&384_000u32.to_le_bytes());
        b.extend_from_slice(&8u16.to_le_bytes()); // align lies (ch×4)
        b.extend_from_slice(&24u16.to_le_bytes());
        let err = match parse_fmt(&b, 44) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.wav.bad-fmt");

        // Channels past the ceiling.
        let mut b = pcm16();
        b[2..4].copy_from_slice(&65u16.to_le_bytes());
        b[12..14].copy_from_slice(&130u16.to_le_bytes());
        let err = match parse_fmt(&b, 44) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.wav.bad-fmt");
    }

    #[test]
    fn short_fmt_is_structural() {
        let err = match parse_fmt(&pcm16()[..15], 44) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "demux.wav.bad-structure");
    }
}
