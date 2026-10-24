//! ADPCM audio decoders (card 5.J2a, doc 04 §3.1 rows "ADPCM MS" /
//! "ADPCM IMA"): the two WAV-carried ADPCM flavors, in-house on the
//! D-018 zero-new-dependency logic (the 2.B4/2.B5 precedent — the
//! algorithms are a few hundred lines of shift-and-add arithmetic
//! with no entropy coding; an external engine would owe a full doc 11
//! §1.3 intake to buy what std-only Rust does in one screen).
//!
//! # Scope (two flavors, one honest gate)
//!
//! | descriptor | wFormatTag | module |
//! |---|---|---|
//! | `adpcm_ms` | `0x0002` | [`ms`] — 7-pair coefficient table, field-interleaved per-channel block headers, multiplicative delta |
//! | `adpcm_ima_wav` | `0x0011` | [`ima`] — 89-entry step table, 4-byte-per-channel nibble granules, low nibble first |
//!
//! The headerless DVI/raw IMA flavors (`adpcm_ima` and friends) are
//! **not** here: they carry no per-block headers and are a typed
//! `Unsupported` at the gate (see [`ima`]'s docs). Both accepted
//! flavors decode to interleaved [`SampleFormat::S16`] native-endian
//! output, bit-exactly against the ffmpeg 7.1 reference on the real
//! corpus — the differential table lives in
//! `tests/adpcm_corpus_differential.rs` and the worklog.
//!
//! # Facts channel (the PCM `extra_data` precedent)
//!
//! [`DecoderConfig::extra_data`](codec_api::DecoderConfig) carries the
//! WAV `fmt ` chunk payload verbatim (the demuxer's
//! `fmt_chunk_bytes()` seam); the decoder re-parses the WAVEFORMATEX
//! (tag, channels, rate, block align, and for MS the coefficient
//! pairs) and cross-checks the tag against the descriptor. Configure
//! without facts is a typed `Unsupported` (`adpcm.facts.missing`) —
//! the decoder refuses to guess the block geometry.
//!
//! # Damage model (doc 03 §4.5(d))
//!
//! ADPCM blocks are self-contained decode units (the demuxer's
//! invariant D2 grammar clause), so damage is **block-local**:
//! configure-time problems (zero block align, tag mismatch, a
//! coefficient table that is not the 7-pair form) are typed
//! `Unsupported`; block-level damage inside a packet (a block shorter
//! than its own header, an MS predictor index out of the coefficient
//! table) skips that block and reports
//! [`DecodeOutcome::Recovered`](codec_api::DecodeOutcome) — bounded,
//! counted, never a panic, never a silent guess. A short final block
//! decodes its whole-nibble prefix (`ConcealedFrame`).

pub(crate) mod ima;
pub(crate) mod ms;
//! ADPCM breadth audio decoders (card 5.J2b, doc 04 §3.1's ADPCM
//! rows beyond the WAV pair): Yamaha, OKI/Dialogic, SWF/Flash, Apple
//! IMA4 and Westwood IMA — in-house on the D-018 zero-new-dependency
//! logic (the 2.B4/2.B5/5.J2a precedent: each flavor is a few hundred
//! lines of shift-and-add arithmetic with no entropy coding, and an
//! external engine would owe a full doc 11 §1.3 intake to buy what
//! std-only Rust does in one screen per flavor).
//!
//! # Base-state note (recorded for the Lead merge)
//!
//! This module was authored in a sandbox snapshot where sibling card
//! 5.J2a's WAV flavors (`adpcm_ms`/`adpcm_ima_wav`) are **not on
//! main** (the dispatch metadata pointed at a `ca4d8bf` that does not
//! exist in this snapshot's history — see the worklog). The module
//! therefore registers only the breadth flavors and is written in
//! J2a's exact adapter shape (same `AdpcmDecoder` type, same
//! `Plan`/`accepts`/pooled-slot decode structure) so the Lead's
//! union-merge of the two halves is mechanical — the recorded
//! `audio/mod.rs` + h264 add/add merge precedent.
//!
//! # Scope (five flavors, two honest gates)
//!
//! | descriptor | module | stream shape | oracle |
//! |---|---|---|---|
//! | `adpcm_yamaha` | [`yamaha`] | raw continuous nibbles, per-byte pair | ffmpeg enc+dec, NUT-carried fixture, bit-exact |
//! | `adpcm_ima_oki` | [`oki`] | raw continuous, 12-bit accumulator ×16 | ffmpeg forced decoder, hand-crafted vectors, bit-exact |
//! | `adpcm_swf` | [`swf`] | self-contained bit-packed blocks | ffmpeg enc+dec, FLV-carried fixture, bit-exact |
//! | `adpcm_ima_qt` | [`misc`] | 34-byte/channel chunks, big-endian headers | ffmpeg enc+dec, MOV-carried fixture, bit-exact |
//! | `adpcm_ima_ws` | [`misc`] | raw continuous, per-pair channel bytes | ffmpeg forced decoder, hand-crafted vectors, bit-exact |
//! | `adpcm_oki` (32k/24k lettered) | — | — | **no ffmpeg 7.1.5 decoder or encoder exists** — typed `Unsupported`, see [`oki`]'s docs |
//!
//! The differential table (PCM MD5 per flavor, both channels paths)
//! lives in `tests/adpcm_breadth_differential.rs` and the worklog.
//!
//! # Facts channel (the raw-facts seam)
//!
//! The raw flavors arrive from containers that are not WAV: Sega
//! `.adp`, Dialogic `.vox`, SWF/FLV sound tags, QuickTime chunks,
//! Westwood AUD chunks — none of which have demuxers yet (future
//! cards; the demux crate currently carries AVI only). Until those
//! land, the facts ride
//! [`DecoderConfig::extra_data`](codec_api::DecoderConfig) as a
//! **16-byte WAVEFORMATEX skeleton** with `wFormatTag = 0` as the raw
//! marker:
//!
//! ```text
//! 00 00            wFormatTag  = 0 (raw — the descriptor names the flavor)
//! xx xx            nChannels   = 1..=2 (the nibble pair is defined for ≤ 2)
//! xx xx xx xx      nSamplesPerSec (nonzero)
//! xx xx xx xx      nAvgBytesPerSec (advisory, unchecked)
//! xx xx            nBlockAlign (advisory, unchecked — blocks are either
//!                   in-band (SWF/IMA4) or absent (raw continuous))
//! 04 00            wBitsPerSample = 4
//! ```
//!
//! Configure without facts is a typed `Unsupported`
//! (`adpcm.facts.missing`) — the decoder refuses to guess the channel
//! count. A nonzero tag is refused (`adpcm.raw.facts.tag`): raw
//! flavors have no WAV identity to cross-check.
//!
//! # Damage model (doc 03 §4.5(d))
//!
//! The two raw continuous flavors (Yamaha, OKI, Westwood) have no
//! headers, so payload damage is undetectable and decodes as-is —
//! the honest posture is exact decode of what arrived. The
//! self-contained flavors (SWF, IMA4) carry in-band headers: a
//! packet too short for its own structure, or an IMA4 header index
//! above 88, skips that packet and reports
//! [`DecodeOutcome::Recovered`](codec_api::DecodeOutcome) — bounded,
//! counted, never a panic, never a silent guess. State-bearing
//! flavors reset their channel state at `configure` (a generation
//! boundary), matching the reference decoder's fresh-context
//! behavior.

pub(crate) mod misc;
pub(crate) mod oki;
pub(crate) mod swf;
pub(crate) mod yamaha;

use codec_api::prelude::*;
use media_types::frame::{AudioFormat, AudioFrame, AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::time::{Duration, Nanoseconds};
use media_types::{EngineError, ErrorKind, PipelineStage};

use ima::{ImaFacts, ImaStats};
use ms::{MsFacts, MsStats};
use misc::WsChannel;
use oki::OkiChannel;
use yamaha::YamahaChannel;

/// Pooled output slots (the PCM `CONVERT_SLOTS` precedent): the
/// rotation reuses a freed slot via `get_mut`, so steady-state decode
/// is allocation-free per packet.
const OUT_SLOTS: usize = 4;

/// The validated decode plan, assembled once at `configure`.
#[derive(Debug, Clone)]
enum Plan {
    Ms {
        facts: MsFacts,
        block_align: usize,
        rate: u32,
    },
    ImaWav {
        facts: ImaFacts,
        block_align: usize,
        rate: u32,
    },
/// The raw-marker `wFormatTag`: breadth flavors are not WAV-carried,
/// so their facts skeleton carries 0 where a WAV flavor would carry
/// its `wFormatTag` (the descriptor names the flavor instead).
const RAW_TAG: u16 = 0x0000;

/// The validated decode plan, assembled once at `configure`.
#[derive(Debug, Clone)]
enum Plan {
    /// Raw continuous Yamaha nibbles (stateful).
    Yamaha { channels: u32, rate: u32 },
    /// Raw continuous OKI/Dialogic nibbles (stateful).
    ImaOki { channels: u32, rate: u32 },
    /// Self-contained bit-packed SWF blocks.
    Swf { channels: u32, rate: u32 },
    /// QuickTime 34-byte-per-channel IMA4 chunks (stateful — the
    /// continuation heuristic).
    ImaQt { channels: u32, rate: u32 },
    /// Raw continuous Westwood nibbles (stateful).
    ImaWs { channels: u32, rate: u32 },
}

impl Plan {
    fn channels(&self) -> u32 {
        match self {
            Plan::Ms { facts, .. } => facts.channels,
            Plan::ImaWav { facts, .. } => facts.channels,
            Plan::Yamaha { channels, .. }
            | Plan::ImaOki { channels, .. }
            | Plan::Swf { channels, .. }
            | Plan::ImaQt { channels, .. }
            | Plan::ImaWs { channels, .. } => *channels,
        }
    }

    fn rate(&self) -> u32 {
        match self {
            Plan::Ms { rate, .. } | Plan::ImaWav { rate, .. } => *rate,
            Plan::Yamaha { rate, .. }
            | Plan::ImaOki { rate, .. }
            | Plan::Swf { rate, .. }
            | Plan::ImaQt { rate, .. }
            | Plan::ImaWs { rate, .. } => *rate,
        }
    }

    fn output_format(&self) -> AudioFormat {
        AudioFormat {
            sample: SampleFormat::S16,
            channels: self.channels(),
            layout: AudioLayout::Interleaved,
            rate: self.rate(),
        }
    }
}

/// The ADPCM decoder behind [`codec_api::Decoder`] — one type, two
/// flavors, selected by the descriptor at `configure`.
/// The ADPCM breadth decoder behind [`codec_api::Decoder`] — one
/// type, five flavors, selected by the descriptor at `configure`.
///
/// # Example
///
/// ```
/// use codec_api::prelude::*;
/// use codecs::audio::adpcm::AdpcmDecoder;
/// use media_types::ids::CodecDescriptor;
/// use media_types::packet::SharedBytes;
/// use media_types::{EngineError, GenerationId, Stamp, TrackId};
///
/// # fn main() -> Result<(), EngineError> {
/// // The demuxer's `fmt ` payload (IMA WAV, mono, 8 kHz, 256-byte
/// // blocks, 249 samples per block).
/// let fmt = [
///     0x11, 0x00, 0x01, 0x00, 0x40, 0x1F, 0x00, 0x00, 0x00, 0x20,
///     0x00, 0x00, 0x00, 0x04, 0x00, 0x02, 0x00, 0xF9, 0x00,
/// ];
/// let mut decoder = AdpcmDecoder::new();
/// decoder.configure(&DecoderConfig::new(
///     CodecDescriptor::new("adpcm_ima_wav")?,
///     GenerationId::FIRST,
///     ThreadBudget::default(),
///     None,
///     Some(SharedBytes::from_vec(fmt.to_vec())),
/// // A raw-facts skeleton for mono 22 050 Hz Yamaha (tag 0 = raw).
/// let facts = [
///     0x00, 0x00, // wFormatTag = 0 (raw marker)
///     0x01, 0x00, // mono
///     0x22, 0x56, 0x00, 0x00, // 22 050 Hz
///     0x00, 0x00, 0x00, 0x00, // byte rate (advisory)
///     0x00, 0x00, // block align (advisory)
///     0x04, 0x00, // 4 bits per sample
/// ];
/// let mut decoder = AdpcmDecoder::new();
/// decoder.configure(&DecoderConfig::new(
///     CodecDescriptor::new("adpcm_yamaha")?,
///     GenerationId::FIRST,
///     ThreadBudget::default(),
///     None,
///     Some(SharedBytes::from_vec(facts.to_vec())),
/// ))?;
/// # Ok(())
/// # }
/// ```
#[derive(Default)]
pub struct AdpcmDecoder {
    /// The validated decode plan (None until `configure`).
    plan: Option<Plan>,
    /// The configured stream generation (None until `configure`).
    generation: Option<GenerationId>,
    /// Codec descriptor accepted at configure time (diagnostics).
    codec: Option<String>,
    /// Cross-packet channel state for the raw continuous flavors
    /// (Yamaha shape — one entry per channel, reset at configure).
    yamaha_state: Vec<YamahaChannel>,
    /// Cross-packet channel state for OKI.
    oki_state: Vec<OkiChannel>,
    /// Cross-packet channel state for Westwood and IMA4 (the shared
    /// IMA predictor/index shape; IMA4 uses it for the continuation
    /// heuristic, Westwood for the continuous stream).
    ima_state: Vec<WsChannel>,
    /// The pooled output-slot rotation (allocation-free per packet
    /// after warm-up; the PCM `convert_pool` pattern).
    slots: Vec<SharedBytes>,
    slot_next: usize,
}

impl AdpcmDecoder {
    /// A fresh decoder, unconfigured; the stream facts must arrive in
    /// `DecoderConfig::extra_data` (the WAV `fmt ` payload) at
    /// `DecoderConfig::extra_data` (the raw-facts skeleton) at
    /// configure time.
    pub fn new() -> AdpcmDecoder {
        AdpcmDecoder::default()
    }

    /// Whether this decoder accepts the descriptor (the gate: exactly
    /// the two WAV-carried flavors).
    fn accepts(descriptor: &str) -> bool {
        descriptor == "adpcm_ms" || descriptor == "adpcm_ima_wav"
    }

    /// Parse the WAVEFORMATEX facts blob. Shared by both flavors;
    /// returns `(tag, channels, rate, block_align)`.
    fn parse_waveformat(extra: &[u8]) -> Result<(u16, u32, u32, u32), AdpcmError> {
        if extra.len() < 16 {
            return Err(unsupported(
                "adpcm.waveformat.short",
                format!(
                    "fmt payload is {} bytes; the 16-byte WAVEFORMATEX base \
                     is the floor (channels, rate and block align live there)",
                    extra.len()
                ),
            ));
        }
        let tag = u16::from_le_bytes([extra[0], extra[1]]);
        let channels = u16::from_le_bytes([extra[2], extra[3]]);
        let rate = u32::from_le_bytes([extra[4], extra[5], extra[6], extra[7]]);
        let block_align = u16::from_le_bytes([extra[12], extra[13]]);
        if channels == 0 || u32::from(channels) > 64 {
            // The demuxer's MAX_CHANNELS ceiling (parity keeps the
            // accepted set identical).
            return Err(unsupported(
                "adpcm.waveformat.channels",
                format!("channel count {channels} is outside 1..=64"),
            ));
        }
        if rate == 0 {
            return Err(unsupported(
                "adpcm.waveformat.rate",
                "sample rate 0 is not a clock",
            ));
        }
        if block_align == 0 {
            return Err(unsupported(
                "adpcm.waveformat.block_align",
                "nBlockAlign 0 — a zero-byte block carries no header",
            ));
        }
        Ok((tag, u32::from(channels), rate, u32::from(block_align)))
    /// Whether this decoder handles the descriptor. `adpcm_oki` is
    /// accepted **into the gate** only to be refused with its own
    /// typed reason (no ffmpeg 7.1.5 reference exists — see [`oki`]).
    fn accepts(descriptor: &str) -> bool {
        matches!(
            descriptor,
            "adpcm_yamaha"
                | "adpcm_ima_oki"
                | "adpcm_oki"
                | "adpcm_swf"
                | "adpcm_ima_qt"
                | "adpcm_ima_ws"
        )
    }
}

impl Decoder for AdpcmDecoder {
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
        let descriptor = cfg.codec.as_str();
        if !Self::accepts(descriptor) {
            return Err(unsupported(
                "adpcm.codec_descriptor",
                format!(
                    "codec descriptor `{descriptor}` is not `adpcm_ms` or `adpcm_ima_wav` \
                     (the headerless DVI/raw flavors are a typed gap, see the module docs)"
                    "codec descriptor `{descriptor}` is not one of the breadth flavors \
                     (adpcm_yamaha, adpcm_ima_oki, adpcm_swf, adpcm_ima_qt, adpcm_ima_ws)"
                ),
            )
            .into_engine_error());
        }
        if descriptor == "adpcm_oki" {
            // The lettered 32k/24k OKI variants: ffmpeg 7.1.5 ships
            // neither decoder nor encoder — no oracle, no vectors, no
            // implementation. The honest gate.
            return Err(unsupported(
                "adpcm.oki.no_reference",
                "the `adpcm_oki` lettered variants (32 kHz / 24 kHz) have no \
                 ffmpeg 7.1.5 decoder or encoder to pin tables against — \
                 refused rather than guessed (see the oki module docs)",
            )
            .into_engine_error());
        }
        let extra = cfg
            .extra_data
            .as_ref()
            .map(|d| d.as_slice())
            .ok_or_else(|| {
                unsupported(
                    "adpcm.facts.missing",
                    "no extra_data: the decoder requires the container's `fmt ` \
                     payload and refuses to guess the block geometry",
                )
                .into_engine_error()
            })?;
        let (tag, channels, rate, block_align) =
            Self::parse_waveformat(extra).map_err(AdpcmError::into_engine_error)?;
        let plan = match descriptor {
            "adpcm_ms" => {
                if tag != ms::TAG_WAVE_FORMAT {
                    return Err(unsupported(
                        "adpcm.waveformat.tag",
                        format!("wFormatTag {tag:#06x} is not the MS-ADPCM tag 0x0002"),
                    )
                    .into_engine_error());
                }
                if channels > 2 {
                    return Err(unsupported(
                        "adpcm.ms.channels",
                        format!(
                            "{channels} channels: the MS nibble packing \
                             (one instant per byte, hi = ch0, lo = ch1) is defined \
                             for at most stereo"
                        ),
                    )
                    .into_engine_error());
                }
                // The 7-pair coefficient table rides the fmt tail:
                // wSamplesPerBlock (advisory), wNumCoef, then pairs.
                if extra.len() < 22 {
                    return Err(unsupported(
                        "adpcm.ms.fmt_short",
                        format!(
                            "fmt payload is {} bytes; MS ADPCM needs ≥ 22 for \
                             the samples-per-block and coefficient-count fields",
                            extra.len()
                        ),
                    )
                    .into_engine_error());
                }
                let coef_count = u16::from_le_bytes([extra[20], extra[21]]);
                if coef_count != 7 {
                    return Err(unsupported(
                        "adpcm.ms.coef_count",
                        format!(
                            "wNumCoef {coef_count} — the MS coefficient table \
                             is exactly 7 pairs (the ffmpeg init check)"
                        ),
                    )
                    .into_engine_error());
                }
                if extra.len() < 22 + 7 * 4 {
                    return Err(unsupported(
                        "adpcm.ms.coef_short",
                        "wNumCoef is 7 but the fmt tail carries fewer than 28 \
                         coefficient bytes",
                    )
                    .into_engine_error());
                }
                let mut coefficients = [(0i32, 0i32); 7];
                for (i, pair) in coefficients.iter_mut().enumerate() {
                    let base = 22 + 4 * i;
                    pair.0 = i16::from_le_bytes([extra[base], extra[base + 1]]) as i32;
                    pair.1 = i16::from_le_bytes([extra[base + 2], extra[base + 3]]) as i32;
                }
                if coefficients != ms::STANDARD_COEFFICIENTS {
                    return Err(unsupported(
                        "adpcm.ms.coef_nonstandard",
                        "the file's 7 coefficient pairs differ from the \
                         published MS-ADPCM table (every real encoder writes \
                         the standard pairs; a divergent table is damage, not \
                         a variant this decoder will guess at)",
                    )
                    .into_engine_error());
                }
                let header = ms::header_bytes(channels);
                if (block_align as usize) < header + 1 {
                    return Err(unsupported(
                        "adpcm.waveformat.block_align",
                        format!(
                            "nBlockAlign {block_align} is smaller than the \
                             {header}-byte MS block header plus one nibble byte"
                        ),
                    )
                    .into_engine_error());
                }
                Plan::Ms {
                    facts: MsFacts {
                        channels,
                        coefficients,
                    },
                    block_align: block_align as usize,
                    rate,
                }
            }
            _ => {
                if tag != ima::TAG_WAVE_FORMAT {
                    return Err(unsupported(
                        "adpcm.waveformat.tag",
                        format!("wFormatTag {tag:#06x} is not the IMA-WAV tag 0x0011"),
                    )
                    .into_engine_error());
                }
                let header = ima::header_bytes(channels);
                if (block_align as usize) < header + 1 {
                    return Err(unsupported(
                        "adpcm.waveformat.block_align",
                        format!(
                            "nBlockAlign {block_align} is smaller than the \
                             {header}-byte IMA block header plus one nibble byte"
                        ),
                    )
                    .into_engine_error());
                }
                Plan::ImaWav {
                    facts: ImaFacts { channels },
                    block_align: block_align as usize,
                    rate,
                }
            }
        };
                    "no extra_data: the decoder requires the raw-facts skeleton \
                     (a 16-byte WAVEFORMATEX with wFormatTag 0) and refuses to \
                     guess the channel count",
                )
                .into_engine_error()
            })?;
        let plan = parse_raw_facts(extra).map_err(AdpcmError::into_engine_error)?;
        let plan = match descriptor {
            "adpcm_yamaha" => Plan::Yamaha {
                channels: plan.0,
                rate: plan.1,
            },
            "adpcm_ima_oki" => Plan::ImaOki {
                channels: plan.0,
                rate: plan.1,
            },
            "adpcm_swf" => Plan::Swf {
                channels: plan.0,
                rate: plan.1,
            },
            "adpcm_ima_qt" => Plan::ImaQt {
                channels: plan.0,
                rate: plan.1,
            },
            // `adpcm_oki` returned above; the only remaining accepted
            // descriptor is the Westwood flavor.
            _ => Plan::ImaWs {
                channels: plan.0,
                rate: plan.1,
            },
        };
        let channels = plan.channels() as usize;
        // Fresh channel state at every (re)configure — a generation
        // boundary is a stream boundary (the reference's fresh
        // codec-context behavior).
        self.yamaha_state = vec![YamahaChannel::default(); channels];
        self.oki_state = vec![OkiChannel::default(); channels];
        self.ima_state = vec![WsChannel::default(); channels];
        self.plan = Some(plan);
        self.generation = Some(cfg.generation);
        self.codec = Some(descriptor.to_string());
        Ok(())
    }

    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        let generation = match self.generation {
            Some(generation) => generation,
            None => return Err(AdpcmError::BadState.into_engine_error()),
        };
        if let Some(outcome) = drop_stale_packet(&packet, generation) {
            return Ok(outcome);
        }
        let data = packet.data.as_slice();
        if data.is_empty() {
            return Ok(DecodeOutcome::NeedMoreInput);
        }
        let Some(plan) = self.plan.as_ref() else {
            return Err(AdpcmError::BadState.into_engine_error());
        };
        let (block_align, channels, rate) = match plan {
            Plan::Ms {
                block_align, rate, ..
            }
            | Plan::ImaWav {
                block_align, rate, ..
            } => (*block_align, plan.channels(), *rate),
        };
        // Decode into a pooled slot (the PCM rotation): the geometry
        // bounds the worst-case output, the damage ledgers bound the
        // honest prefix, and the handed-out slice cuts to it.
        let bound = out_bound(data.len(), block_align, channels);
        let mut pool = std::mem::take(&mut self.slots);
        let mut slot_next = self.slot_next;
        let mut ms_stats = MsStats::default();
        let mut ima_stats = ImaStats::default();
        let samples = pooled_slot(&mut pool, &mut slot_next, bound, |bytes| {
            let mut buf = Vec::new();
            match plan {
                Plan::Ms {
                    facts, block_align, ..
                } => {
                    ms_stats = ms::decode_ms_blocks(data, facts, *block_align, &mut buf);
                }
                Plan::ImaWav {
                    facts, block_align, ..
                } => {
                    ima_stats = ima::decode_ima_blocks(data, facts, *block_align, &mut buf);
        let Some(plan) = self.plan.clone() else {
            return Err(AdpcmError::BadState.into_engine_error());
        };
        let channels = plan.channels() as usize;
        // Decode into a pooled slot (the PCM rotation): the geometry
        // bounds the worst-case output (≤ 2 samples per nibble = 4
        // output bytes per input byte) and the handed-out slice cuts
        // to the honest prefix.
        let bound = out_bound(data.len());
        let mut pool = std::mem::take(&mut self.slots);
        let mut slot_next = self.slot_next;
        let mut yamaha_state = std::mem::take(&mut self.yamaha_state);
        let mut oki_state = std::mem::take(&mut self.oki_state);
        let mut ima_state = std::mem::take(&mut self.ima_state);
        let mut stats = Stats::default();
        let samples = pooled_slot(&mut pool, &mut slot_next, bound, |bytes| {
            let mut buf = Vec::new();
            match &plan {
                Plan::Yamaha { .. } => {
                    yamaha::decode_yamaha(data, channels, &mut yamaha_state, &mut buf);
                    stats = Stats {
                        skipped: 0,
                        concealed: false,
                    };
                }
                Plan::ImaOki { .. } => {
                    oki::decode_ima_oki(data, channels, &mut oki_state, &mut buf);
                    stats = Stats {
                        skipped: 0,
                        concealed: false,
                    };
                }
                Plan::Swf { .. } => {
                    let s = swf::decode_swf_packet(data, channels, &mut buf);
                    stats = Stats {
                        skipped: u64::from(s.skipped_packet),
                        concealed: s.concealed_tail,
                    };
                }
                Plan::ImaQt { .. } => {
                    let s = misc::decode_ima_qt_packet(data, channels, &mut ima_state, &mut buf);
                    stats = Stats {
                        skipped: u64::from(s.skipped_packets),
                        concealed: false,
                    };
                }
                Plan::ImaWs { .. } => {
                    misc::decode_ima_ws(data, channels, &mut ima_state, &mut buf);
                    stats = Stats {
                        skipped: 0,
                        concealed: false,
                    };
                }
            }
            bytes[..buf.len()].copy_from_slice(&buf);
            buf.len()
        });
        self.slots = pool;
        self.slot_next = slot_next;
        let filled = samples.len() / 2 / channels as usize; // sample instants
        let frame = Frame::Audio(AudioFrame {
            format: plan.output_format(),
            samples,
            pts: packet.pts,
            duration: Some(Duration::Known(Nanoseconds(
                duration_nanos(filled as u64, rate) as u64,
            ))),
            generation: packet.generation,
        });
        out.accept(frame)?;
        let skipped = ms_stats.skipped_blocks + ima_stats.skipped_blocks;
        let concealed = ms_stats.concealed_tail || ima_stats.concealed_tail;
        if skipped > 0 {
            Ok(DecodeOutcome::Recovered {
                skipped,
                reason: RecoverReason::CorruptInput,
            })
        } else if concealed {
        self.yamaha_state = yamaha_state;
        self.oki_state = oki_state;
        self.ima_state = ima_state;
        // A skipped block-flavor packet decodes to nothing — no frame
        // is emitted for it (the `Recovered` outcome is the report;
        // an empty audio frame would be noise on the wire). Every
        // other path decoded at least one sample.
        if !samples.is_empty() {
            let filled = samples.len() / 2 / channels.max(1); // sample instants
            let frame = Frame::Audio(AudioFrame {
                format: plan.output_format(),
                samples,
                pts: packet.pts,
                duration: Some(Duration::Known(Nanoseconds(duration_nanos(
                    filled as u64,
                    plan.rate(),
                ) as u64))),
                generation: packet.generation,
            });
            out.accept(frame)?;
        }
        if stats.skipped > 0 {
            Ok(DecodeOutcome::Recovered {
                skipped: stats.skipped as u32,
                reason: RecoverReason::CorruptInput,
            })
        } else if stats.concealed {
            Ok(DecodeOutcome::Recovered {
                skipped: 0,
                reason: RecoverReason::ConcealedFrame,
            })
        } else {
            Ok(DecodeOutcome::Emitted { frames: 1 })
        }
    }

    fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        // ADPCM blocks are self-contained decode units: nothing is
        // buffered across packets, so flush is the clean drain.
        // Nothing is buffered ahead of the packet: the raw flavors
        // have consumed every input byte and the block flavors are
        // per-packet. Flush is the clean drain.
        Ok(DecodeOutcome::Drained)
    }
}

/// Worst-case decoded bytes for a payload: every nibble byte yields
/// two samples (mono MS is the expansive shape: 2 samples per input
/// byte), plus the 2 seed samples per block — one byte of headroom
/// rounds the block count up.
fn out_bound(payload: usize, block_align: usize, channels: u32) -> usize {
    let blocks = payload / block_align.max(1) + 1;
    // Seeds: 2 samples/block (MS) or 1 (IMA) — 2 is the safe bound;
    // nibbles: 2 samples per byte / channel... the exact worst case
    // is 4 output bytes per input byte (mono MS).
    blocks
        .saturating_mul(block_align.saturating_mul(4).saturating_add(4))
        .saturating_add(usize::try_from(channels.max(1)).unwrap_or(1) * 2 * blocks)
/// The per-decode damage ledger, unified across flavor stats shapes.
#[derive(Debug, Clone, Copy, Default)]
struct Stats {
    skipped: u64,
    concealed: bool,
}

/// Parse the 16-byte raw-facts skeleton: `(channels, rate)`. The
/// block-align and byte-rate fields are advisory for these flavors
/// (blocks are in-band or absent) and are not cross-checked; the
/// channel count is bounded to the nibble pair (≤ 2) and the rate
/// must be a real clock.
fn parse_raw_facts(extra: &[u8]) -> Result<(u32, u32), AdpcmError> {
    if extra.len() < 16 {
        return Err(unsupported(
            "adpcm.raw.facts.short",
            format!(
                "facts payload is {} bytes; the 16-byte WAVEFORMATEX skeleton \
                 is the floor (channels and rate live there)",
                extra.len()
            ),
        ));
    }
    let tag = u16::from_le_bytes([extra[0], extra[1]]);
    if tag != RAW_TAG {
        return Err(unsupported(
            "adpcm.raw.facts.tag",
            format!(
                "wFormatTag {tag:#06x} — the breadth flavors are raw-carried and \
                 take the 0x0000 raw marker (a WAV tag belongs to the WAV flavors)"
            ),
        ));
    }
    let channels = u16::from_le_bytes([extra[2], extra[3]]);
    let rate = u32::from_le_bytes([extra[4], extra[5], extra[6], extra[7]]);
    if channels == 0 || channels > 2 {
        return Err(unsupported(
            "adpcm.raw.facts.channels",
            format!(
                "channel count {channels} — the 4-bit nibble pair is defined \
                 for at most stereo (one sample per nibble, two nibbles per byte)"
            ),
        ));
    }
    if rate == 0 {
        return Err(unsupported(
            "adpcm.raw.facts.rate",
            "sample rate 0 is not a clock",
        ));
    }
    Ok((u32::from(channels), rate))
}

/// Worst-case decoded bytes for a breadth payload: the expansive
/// shape is SWF at code width 2 stereo — every 4 input bits are one
/// field instant (2 samples = 4 output bytes), i.e. 8 output bytes
/// per input byte; the SWF/IMA4 headers displace bits and every
/// other flavor sits at ≤ 4 output bytes per input byte. The +64
/// slack covers block seeds at the tail.
fn out_bound(payload: usize) -> usize {
    payload.saturating_mul(8).saturating_add(64)
}

/// Duration of `frames` instants at `rate`, in nanoseconds (u128
/// intermediate — the PCM `duration_nanos` shape).
fn duration_nanos(frames: u64, rate: u32) -> u128 {
    if rate == 0 {
        return 0;
    }
    (u128::from(frames) * 1_000_000_000) / u128::from(rate)
}

/// The pooled-slot rotation (the PCM `pooled_convert_slot` pattern):
/// find a free slot of the right length, fill it in place, hand out
/// the decoded prefix as a slice; allocate cold and enter the
/// rotation otherwise. Returns the prefix slice.
fn pooled_slot(
    slots: &mut Vec<SharedBytes>,
    next: &mut usize,
    len: usize,
    fill: impl FnOnce(&mut [u8]) -> usize,
) -> SharedBytes {
    let count = slots.len();
    for i in 0..count {
        let idx = (*next + i) % count;
        if slots[idx].len() != len {
            continue;
        }
        if let Some(bytes) = slots[idx].get_mut() {
            let used = fill(bytes);
            *next = (idx + 1) % count;
            return slots[idx].slice(0..used);
        }
        // The slot still has downstream owners — try the next one.
    }
    // Cold path: allocate, fill in place, then enter the rotation.
    let mut fresh = SharedBytes::from_vec(vec![0; len]);
    let used = match fresh.get_mut() {
        Some(bytes) => fill(bytes),
        None => 0,
    };
    let out = fresh.slice(0..used);
    if count < OUT_SLOTS {
        slots.push(fresh);
        *next = (count + 1) % OUT_SLOTS;
    } else {
        let evict = *next;
        slots[evict] = fresh;
        *next = (evict + 1) % OUT_SLOTS;
    }
    out
}

/// Decode-stage taxonomy (the 2.B4/2.B5 pattern): typed, stage-tagged
/// internal errors that map one-to-one onto [`EngineError`] kinds at
/// the contract boundary. Block-level damage never appears here — it
/// is concealment (`DecodeOutcome::Recovered`), see the module docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AdpcmError {
    /// A descriptor, facts blob or parameter combination outside the
    /// supported ADPCM subset (unknown descriptor, missing fmt,
    /// malformed WAVEFORMATEX, zero block align, coefficient table
    /// damage, …). `code` names the exact rule.
/// the contract boundary. Packet-level damage never appears here —
/// it is concealment (`DecodeOutcome::Recovered`), see the module
/// docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AdpcmError {
    /// A descriptor or facts blob outside the supported breadth
    /// subset (unknown descriptor, missing facts, malformed skeleton,
    /// zero rate, …). `code` names the exact rule.
    Unsupported {
        /// Stable dotted context code naming the exact rule broken.
        code: &'static str,
        /// Human-readable reason (rides in `source`).
        detail: String,
    },
    /// The decoder was asked to `decode`/`flush` before a successful
    /// `configure` (contract misuse — typed, never a panic).
    BadState,
}

impl AdpcmError {
    /// Convert to the public contract error (stage `Decode`).
    pub(crate) fn into_engine_error(self) -> EngineError {
        let stage = PipelineStage::Decode;
        match self {
            AdpcmError::Unsupported { code, detail } => EngineError::new(
                stage,
                ErrorKind::Unsupported {
                    format: Some(media_types::error::FormatInfo {
                        name: Some("adpcm".to_string()),
                        sniffed: Vec::new(),
                    }),
                },
                media_types::error::ErrorContext::new(code),
            )
            .with_source(Detail(detail)),
            AdpcmError::BadState => EngineError::new(
                stage,
                ErrorKind::Decoder {
                    cause: media_types::error::DecodeCause::BadState,
                },
                media_types::error::ErrorContext::new("adpcm.not_configured"),
            ),
        }
    }
}

/// Chain-root carrier for the unsupported detail (the 2.B4 pattern).
#[derive(Debug, Clone)]
struct Detail(String);

impl std::fmt::Display for Detail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Detail {}

/// Shorthand constructor: an unsupported configuration with a reason.
pub(crate) fn unsupported(code: &'static str, detail: impl Into<String>) -> AdpcmError {
    AdpcmError::Unsupported {
        code,
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::ids::CodecDescriptor;

    /// A minimal IMA WAV fmt payload (mono, 8 kHz) with the given
    /// block align (the test blocks are single small blocks).
    fn ima_fmt(block_align: u16) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&ima::TAG_WAVE_FORMAT.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&8_000u32.to_le_bytes());
        b.extend_from_slice(&4_000u32.to_le_bytes());
        b.extend_from_slice(&block_align.to_le_bytes());
        b.extend_from_slice(&4u16.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes()); // cbSize
        b.extend_from_slice(&249u16.to_le_bytes()); // samples/block
        b
    }

    /// A minimal MS WAV fmt payload (stereo, 16 kHz) with the given
    /// block align and the standard 7 coefficient pairs.
    fn ms_fmt(block_align: u16) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&ms::TAG_WAVE_FORMAT.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&16_000u32.to_le_bytes());
        b.extend_from_slice(&8_000u32.to_le_bytes());
        b.extend_from_slice(&block_align.to_le_bytes());
        b.extend_from_slice(&4u16.to_le_bytes());
        b.extend_from_slice(&32u16.to_le_bytes()); // cbSize
        b.extend_from_slice(&504u16.to_le_bytes()); // samples/block
        b.extend_from_slice(&7u16.to_le_bytes()); // wNumCoef
        for (a, c) in ms::STANDARD_COEFFICIENTS {
            b.extend_from_slice(&(a as i16).to_le_bytes());
            b.extend_from_slice(&(c as i16).to_le_bytes());
        }
    use testkit::Must;

    /// A raw-facts skeleton (16-byte WAVEFORMATEX, tag 0) for the
    /// given channels/rate.
    fn raw_facts(channels: u16, rate: u32) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&RAW_TAG.to_le_bytes());
        b.extend_from_slice(&channels.to_le_bytes());
        b.extend_from_slice(&rate.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes()); // advisory byte rate
        b.extend_from_slice(&0u16.to_le_bytes()); // advisory block align
        b.extend_from_slice(&4u16.to_le_bytes()); // bits per sample
        b
    }

    fn configured(
        decoder: &mut AdpcmDecoder,
        descriptor: &str,
        extra: Option<Vec<u8>>,
    ) -> Result<GenerationId, EngineError> {
        let generation = GenerationId::new(7);
        let config = DecoderConfig::new(
            CodecDescriptor::new(descriptor)?,
            generation,
            ThreadBudget::default(),
            None,
            extra.map(SharedBytes::from_vec),
        );
        decoder.configure(&config)?;
        Ok(generation)
    }

    /// A capture sink collecting audio frames.
    #[derive(Default)]
    struct Capture {
        frames: Vec<AudioFrame>,
    }

    impl FrameSink for Capture {
        fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
            match frame {
                Frame::Audio(audio) => self.frames.push(audio),
                Frame::Video(_) | Frame::Subtitle(_) => {
                    return Err(EngineError::new(
                        PipelineStage::Decode,
                        ErrorKind::Decoder {
                            cause: media_types::error::DecodeCause::BadState,
                        },
                        media_types::error::ErrorContext::new("adpcm.test.sink"),
                    ));
                }
            }
            Ok(())
        }
    }

    #[test]
    fn gate_accepts_exactly_the_two_wav_flavors() {
        assert!(AdpcmDecoder::accepts("adpcm_ms"));
        assert!(AdpcmDecoder::accepts("adpcm_ima_wav"));
        for other in ["adpcm_ima", "pcm_s16le", "flac", "adpcm_ima_qt", ""] {
            assert!(!AdpcmDecoder::accepts(other), "{other}");
        }
        let mut decoder = AdpcmDecoder::new();
        let err = match configured(&mut decoder, "adpcm_ima", Some(ima_fmt(256))) {
            Ok(_) => unreachable!("the raw DVI gate must refuse"),
    fn packet(generation: GenerationId, data: Vec<u8>) -> Packet {
        Packet {
            track: media_types::TrackId::new(1),
            pts: media_types::Stamp::Missing,
            dts: media_types::Stamp::Missing,
            duration: None,
            data: SharedBytes::from_vec(data),
            keyframe: true,
            discontinuity: false,
            generation,
        }
    }

    #[test]
    fn gate_accepts_exactly_the_breadth_flavors() {
        for ok in [
            "adpcm_yamaha",
            "adpcm_ima_oki",
            "adpcm_oki",
            "adpcm_swf",
            "adpcm_ima_qt",
            "adpcm_ima_ws",
        ] {
            assert!(AdpcmDecoder::accepts(ok), "{ok}");
        }
        for other in [
            "adpcm_ms",
            "adpcm_ima_wav",
            "adpcm_ima",
            "pcm_s16le",
            "flac",
            "",
        ] {
            assert!(!AdpcmDecoder::accepts(other), "{other}");
        }
    }

    #[test]
    fn lettered_oki_is_the_honest_typed_gap() {
        let mut decoder = AdpcmDecoder::new();
        let err = match configured(&mut decoder, "adpcm_oki", Some(raw_facts(1, 8_000))) {
            Ok(_) => unreachable!("the lettered oki gate must refuse"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "adpcm.oki.no_reference");
        assert!(matches!(
            err.kind,
            media_types::ErrorKind::Unsupported { .. }
        ));
    }

    #[test]
    fn missing_facts_and_skeleton_damage_are_typed_unsupported() {
        let mut decoder = AdpcmDecoder::new();
        let err = match configured(&mut decoder, "adpcm_yamaha", None) {
            Ok(_) => unreachable!("no facts must refuse"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "adpcm.facts.missing");

        // Non-raw tag (an IMA-WAV tag smuggled in).
        let mut tagged = raw_facts(1, 8_000);
        tagged[0] = 0x11;
        let mut decoder = AdpcmDecoder::new();
        let err = match configured(&mut decoder, "adpcm_yamaha", Some(tagged)) {
            Ok(_) => unreachable!("a nonzero tag must refuse"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "adpcm.raw.facts.tag");

        // Channels beyond the nibble pair.
        let mut decoder = AdpcmDecoder::new();
        let err = match configured(&mut decoder, "adpcm_swf", Some(raw_facts(3, 8_000))) {
            Ok(_) => unreachable!("3 channels must refuse"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "adpcm.raw.facts.channels");

        // Zero rate, short skeleton.
        let mut decoder = AdpcmDecoder::new();
        let err = match configured(&mut decoder, "adpcm_ima_ws", Some(raw_facts(1, 0))) {
            Ok(_) => unreachable!("zero rate must refuse"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "adpcm.raw.facts.rate");
        let mut decoder = AdpcmDecoder::new();
        let err = match configured(&mut decoder, "adpcm_ima_oki", Some(vec![0; 8])) {
            Ok(_) => unreachable!("a short skeleton must refuse"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "adpcm.raw.facts.short");

        // Unknown descriptor.
        let mut decoder = AdpcmDecoder::new();
        let err = match configured(&mut decoder, "adpcm_ima_apm", Some(raw_facts(1, 8_000))) {
            Ok(_) => unreachable!("an unowned descriptor must refuse"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "adpcm.codec_descriptor");
    }

    #[test]
    fn missing_facts_and_zero_block_align_are_typed_unsupported() {
        let mut decoder = AdpcmDecoder::new();
        let err = match configured(&mut decoder, "adpcm_ms", None) {
            Ok(_) => unreachable!("no facts must refuse"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "adpcm.facts.missing");

        let mut zero = ima_fmt(256);
        zero[12] = 0;
        zero[13] = 0;
        let mut decoder = AdpcmDecoder::new();
        let err = match configured(&mut decoder, "adpcm_ima_wav", Some(zero)) {
            Ok(_) => unreachable!("zero block align must refuse"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "adpcm.waveformat.block_align");
    }

    #[test]
    fn tag_mismatch_and_coef_damage_are_typed_unsupported() {
        // IMA descriptor, MS tag.
        let mut decoder = AdpcmDecoder::new();
        let err = match configured(&mut decoder, "adpcm_ima_wav", Some(ms_fmt(512))) {
            Ok(_) => unreachable!("tag mismatch must refuse"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "adpcm.waveformat.tag");

        // MS with a non-7 coefficient count.
        let mut damaged = ms_fmt(512);
        damaged[20] = 6;
        let mut decoder = AdpcmDecoder::new();
        let err = match configured(&mut decoder, "adpcm_ms", Some(damaged)) {
            Ok(_) => unreachable!("wNumCoef 6 must refuse"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "adpcm.ms.coef_count");

        // MS with 7 non-standard pairs.
        let mut damaged = ms_fmt(512);
        let last = 22 + 6 * 4;
        damaged[last] = 0xFF;
        let mut decoder = AdpcmDecoder::new();
        let err = match configured(&mut decoder, "adpcm_ms", Some(damaged)) {
            Ok(_) => unreachable!("non-standard coefficients must refuse"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "adpcm.ms.coef_nonstandard");
    }

    #[test]
    fn unconfigured_decode_is_typed_bad_state() {
        let mut decoder = AdpcmDecoder::new();
        let err = match decoder.decode(empty_packet(), &mut Capture::default()) {
    fn unconfigured_decode_is_typed_bad_state() {
        let mut decoder = AdpcmDecoder::new();
        let err = match decoder.decode(
            packet(GenerationId::FIRST, Vec::new()),
            &mut Capture::default(),
        ) {
            Ok(_) => unreachable!("unconfigured decode must refuse"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "adpcm.not_configured");
    }

    #[test]
    fn ima_packet_decodes_and_recovers_typed() {
        let mut decoder = AdpcmDecoder::new();
        let generation = match configured(&mut decoder, "adpcm_ima_wav", Some(ima_fmt(20))) {
    fn yamaha_packets_decode_with_persistent_state() {
        let mut decoder = AdpcmDecoder::new();
        let generation = match configured(&mut decoder, "adpcm_yamaha", Some(raw_facts(1, 22_050)))
        {
            Ok(generation) => generation,
            Err(err) => unreachable!("configure: {err}"),
        };
        let mut capture = Capture::default();
        // A clean single block: header (predictor 42, index 3) + 16
        // nibble bytes → 33 samples.
        let mut block = Vec::new();
        block.extend_from_slice(&42i16.to_le_bytes());
        block.extend_from_slice(&[3, 0]);
        block.extend_from_slice(&[0x7F; 16]);
        let packet = Packet {
            track: media_types::TrackId::new(1),
            pts: media_types::Stamp::Missing,
            dts: media_types::Stamp::Missing,
            duration: None,
            data: SharedBytes::from_vec(block.clone()),
            keyframe: true,
            discontinuity: false,
            generation,
        };
        let outcome = match decoder.decode(packet, &mut capture) {
            Ok(outcome) => outcome,
            Err(err) => unreachable!("decode: {err}"),
        };
        assert_eq!(outcome, DecodeOutcome::Emitted { frames: 1 });
        assert_eq!(capture.frames.len(), 1);
        let frame = &capture.frames[0];
        assert_eq!(frame.format.sample, SampleFormat::S16);
        assert_eq!(frame.format.channels, 1);
        assert_eq!(frame.format.layout, AudioLayout::Interleaved);
        assert_eq!(frame.format.rate, 8_000);
        assert_eq!(frame.samples.len(), 33 * 2);

        // A block shorter than its 4-byte header: skipped, counted.
        capture.frames.clear();
        let packet = Packet {
            track: media_types::TrackId::new(1),
            pts: media_types::Stamp::Missing,
            dts: media_types::Stamp::Missing,
            duration: None,
            data: SharedBytes::from_vec(vec![0x11, 0x22]),
            keyframe: true,
            discontinuity: false,
            generation,
        };
        let outcome = match decoder.decode(packet, &mut capture) {
        let mut outcome_all = DecodeOutcome::Drained;
        for i in 0..3 {
            outcome_all = match decoder.decode(
                packet(generation, [[0x12u8, 0x34]; 8].concat()),
                &mut capture,
            ) {
                Ok(outcome) => outcome,
                Err(err) => unreachable!("decode {i}: {err}"),
            };
        }
        assert_eq!(outcome_all, DecodeOutcome::Emitted { frames: 1 });
        assert_eq!(capture.frames.len(), 3);
        // 16 bytes/packet × 2 samples/byte = 32 samples per packet,
        // and the state carried across packets (the second packet's
        // first sample differs from the first's).
        for frame in &capture.frames {
            assert_eq!(frame.format.sample, SampleFormat::S16);
            assert_eq!(frame.format.channels, 1);
            assert_eq!(frame.format.layout, AudioLayout::Interleaved);
            assert_eq!(frame.samples.len(), 32 * 2);
        }
        let first: Vec<i16> = capture.frames[0]
            .samples
            .as_slice()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_ne_bytes(*b))
            .collect();
        let second: Vec<i16> = capture.frames[1]
            .samples
            .as_slice()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_ne_bytes(*b))
            .collect();
        assert_ne!(first[0], second[0], "cross-packet state continuity");
        // Empty payload: NeedMoreInput, stale: dropped on sight.
        assert_eq!(
            decoder
                .decode(packet(generation, Vec::new()), &mut capture)
                .must(),
            DecodeOutcome::NeedMoreInput
        );
        assert_eq!(
            decoder
                .decode(packet(GenerationId::new(2), vec![0x12]), &mut capture)
                .must(),
            DecodeOutcome::NeedMoreInput
        );
    }

    #[test]
    fn swf_and_qt_packet_damage_recovers_typed() {
        // SWF: a 2-byte packet is short of the 22-bit header → skip.
        let mut decoder = AdpcmDecoder::new();
        let generation = match configured(&mut decoder, "adpcm_swf", Some(raw_facts(2, 22_050))) {
            Ok(generation) => generation,
            Err(err) => unreachable!("configure: {err}"),
        };
        let mut capture = Capture::default();
        let outcome = match decoder.decode(packet(generation, vec![0x80, 0x00]), &mut capture) {
            Ok(outcome) => outcome,
            Err(err) => unreachable!("decode: {err}"),
        };
        assert_eq!(
            outcome,
            DecodeOutcome::Recovered {
                skipped: 1,
                reason: RecoverReason::CorruptInput
            }
        );
        assert!(capture.frames.is_empty() || capture.frames[0].samples.is_empty());

        // Stale packet: dropped on sight.
        let stale = Packet {
            track: media_types::TrackId::new(1),
            pts: media_types::Stamp::Missing,
            dts: media_types::Stamp::Missing,
            duration: None,
            data: SharedBytes::from_vec(block),
            keyframe: true,
            discontinuity: false,
            generation: GenerationId::new(2),
        };
        let outcome = match decoder.decode(stale, &mut capture) {
            Ok(outcome) => outcome,
            Err(err) => unreachable!("decode: {err}"),
        };
        assert_eq!(outcome, DecodeOutcome::NeedMoreInput);
    }

    #[test]
    fn ms_packet_decodes_stereo_interleaved() {
        let mut decoder = AdpcmDecoder::new();
        let generation = match configured(&mut decoder, "adpcm_ms", Some(ms_fmt(16))) {

        // IMA4: a short chunk skips; a full chunk emits 64 frames.
        let mut decoder = AdpcmDecoder::new();
        let generation = match configured(&mut decoder, "adpcm_ima_qt", Some(raw_facts(1, 22_050)))
        {
            Ok(generation) => generation,
            Err(err) => unreachable!("configure: {err}"),
        };
        let mut capture = Capture::default();
        // One stereo block: predictors 0/0, deltas 16/16, s1 100/−100,
        // s2 50/−50, then two nibble bytes.
        let mut block = Vec::new();
        block.extend_from_slice(&[0, 0]);
        block.extend_from_slice(&16i16.to_le_bytes());
        block.extend_from_slice(&16i16.to_le_bytes());
        block.extend_from_slice(&100i16.to_le_bytes());
        block.extend_from_slice(&(-100i16).to_le_bytes());
        block.extend_from_slice(&50i16.to_le_bytes());
        block.extend_from_slice(&(-50i16).to_le_bytes());
        block.extend_from_slice(&[0x88, 0x77]);
        let packet = Packet {
            track: media_types::TrackId::new(1),
            pts: media_types::Stamp::Missing,
            dts: media_types::Stamp::Missing,
            duration: None,
            data: SharedBytes::from_vec(block),
            keyframe: true,
            discontinuity: false,
            generation,
        };
        let outcome = match decoder.decode(packet, &mut capture) {
        let outcome = match decoder.decode(packet(generation, vec![0; 10]), &mut capture) {
            Ok(outcome) => outcome,
            Err(err) => unreachable!("decode: {err}"),
        };
        assert_eq!(
            outcome,
            DecodeOutcome::Recovered {
                skipped: 1,
                reason: RecoverReason::CorruptInput
            }
        );
        let outcome = match decoder.decode(packet(generation, vec![0; 34]), &mut capture) {
            Ok(outcome) => outcome,
            Err(err) => unreachable!("decode: {err}"),
        };
        assert_eq!(outcome, DecodeOutcome::Emitted { frames: 1 });
        let frame = &capture.frames[0];
        assert_eq!(frame.format.channels, 2);
        // 2 seed samples + 2 nibble instants per channel = 4 instants.
        assert_eq!(frame.samples.len(), 4 * 2 * 2);
        let samples: Vec<i16> = frame
            .samples
            .as_slice()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_ne_bytes(*b))
            .collect();
        // Seed order: s2 both channels, s1 both channels — the
        // field-interleaved header read.
        assert_eq!(samples[..4], [50, -50, 100, -100]);
        assert_eq!(capture.frames[0].samples.len(), 64 * 2);
    }

    #[test]
    fn pooled_rotation_is_reused_across_packets() {
        let mut decoder = AdpcmDecoder::new();
        let generation = match configured(&mut decoder, "adpcm_ima_wav", Some(ima_fmt(12))) {
            Ok(generation) => generation,
            Err(err) => unreachable!("configure: {err}"),
        };
        let mut block = Vec::new();
        block.extend_from_slice(&42i16.to_le_bytes());
        block.extend_from_slice(&[3, 0]);
        block.extend_from_slice(&[0x7F; 8]);
        for i in 0..OUT_SLOTS * 3 {
            let packet = Packet {
                track: media_types::TrackId::new(1),
                pts: media_types::Stamp::Missing,
                dts: media_types::Stamp::Missing,
                duration: None,
                data: SharedBytes::from_vec(block.clone()),
                keyframe: true,
                discontinuity: false,
                generation,
            };
            let mut capture = Capture::default();
            let outcome = match decoder.decode(packet, &mut capture) {
        let generation = match configured(&mut decoder, "adpcm_ima_ws", Some(raw_facts(1, 8_000))) {
            Ok(generation) => generation,
            Err(err) => unreachable!("configure: {err}"),
        };
        for i in 0..OUT_SLOTS * 3 {
            let mut capture = Capture::default();
            let outcome = match decoder.decode(packet(generation, vec![0x55; 12]), &mut capture) {
                Ok(outcome) => outcome,
                Err(err) => unreachable!("decode {i}: {err}"),
            };
            assert_eq!(outcome, DecodeOutcome::Emitted { frames: 1 }, "packet {i}");
        }
        // The rotation stays bounded.
        assert!(decoder.slots.len() <= OUT_SLOTS);
    }

    fn empty_packet() -> Packet {
        Packet {
            track: media_types::TrackId::new(1),
            pts: media_types::Stamp::Missing,
            dts: media_types::Stamp::Missing,
            duration: None,
            data: SharedBytes::from_vec(Vec::new()),
            keyframe: true,
            discontinuity: false,
            generation: GenerationId::FIRST,
        }
    }
        assert!(decoder.slots.len() <= OUT_SLOTS);
    }
}
