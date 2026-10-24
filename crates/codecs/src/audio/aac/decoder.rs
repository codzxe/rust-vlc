//! The AAC-LC decoder core (card 5.C2a): the symphonia-backed frame
//! decode, the ADTS stream-level pipeline, and the
//! [`codec_api::Decoder`] implementation.
//!
//! # Engine choice (the 5.C2a dependency decision)
//!
//! **`symphonia-codec-aac` 0.6.1, exact-pinned** (plus its
//! `symphonia-core`, and transitively `symphonia-common` /
//! `symphonia-metadata` / `regex-lite`), under the Lead's C5
//! pre-approval, purity verified BEFORE any Cargo.toml edit in
//! `docs/dependencies/records/symphonia-aac-0.6.1.md` (record's
//! reality check: the brief's `symphonia-bundle-aac` spelling does
//! not exist on crates.io). The engine owns exactly one job:
//! entropy-decoding ONE already-framed raw data block (SCE/CPE →
//! ICS → codebooks → TNS/gain → synthesis) to F32 planes. The ADTS
//! header parse, the frame walk/resync, the profile/HE gates and the
//! error taxonomy are ours — a future engine swap stays inside this
//! module.
//!
//! # Packet model (the codec-api contract)
//!
//! One packet = **one complete ADTS frame, header included** (ADTS
//! mode — no `extra_data` at configure time), or **one raw data
//! block** (ASC mode — `extra_data` carries the AudioSpecificConfig,
//! the MP4/esds path). In ADTS mode the engine is built lazily from
//! the FIRST packet's header (an ADTS stream announces its parameters
//! per frame, not in a container header — the decode entry is
//! ADTS-frame-aware by construction). ADTS frames are independent
//! (each carries its full side information), so per-frame isolation
//! (doc 03 §4.5(d)) holds: after a typed `Corrupt` the next packet
//! decodes unchanged.
//!
//! # HE-AAC honesty (doc 04 §3.2 note, the card's "honestly partial")
//!
//! The engine implements **AAC-LC only** (its own gate: non-LC object
//! types, `sbr_present`, > 2 channels, non-1024 frames are refused).
//! 5.C2a maps that honestly:
//!
//! - **Explicit ASC signaling** (audio object type 5 = SBR, 29 = PS —
//!   the MP4 path): refused typedly as `aac.he_sbr` (`Unsupported`)
//!   before the engine is even built.
//! - **Non-LC object types** (Main 1, SSR 3, LTP 4, reserved):
//!   `aac.profile` (`Unsupported`) — sniffed from the ASC's first 5
//!   bits, or the ADTS profile field.
//! - **Implicit ADTS HE signaling**: undetectable from the 7-byte
//!   header — the SBR extension lives inside the raw data block
//!   behind full SCE/CPE syntax, and the engine exposes no
//!   `sbr_present` accessor (record R6). Such streams decode as
//!   **half-rate LC** (bandwidth-limited); the differential records
//!   the degradation (`lumen_rate × 2 = ffmpeg_rate`) instead of
//!   silently passing it. SBR/PS itself remains doc 04's self-build
//!   M — unscheduled here.
//! - **Channel configuration 0** (PCE in band) and **> 2 channels**:
//!   typed `aac.channel_config_pce` / `aac.channel_count`
//!   (`Unsupported`) — the engine requires the count up front and
//!   caps at stereo.

use super::adts::{AdtsProfile, parse_adts_header, walk_adts_frames};
use super::error::{AacError, corrupt, unsupported};
use codec_api::prelude::*;
use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::time::{Nanoseconds, TimeBase, Timestamp};
use media_types::{Duration, Stamp};
use symphonia_codec_aac::AacDecoder as AacEngine;
use symphonia_core::audio::layouts::{CHANNEL_LAYOUT_MONO, CHANNEL_LAYOUT_STEREO};
use symphonia_core::codecs::audio::well_known::CODEC_ID_AAC;
use symphonia_core::codecs::audio::{AudioCodecParameters, AudioDecoder, AudioDecoderOptions};
use symphonia_core::packet::PacketRef;
use symphonia_core::units::{Duration as SymDuration, Timestamp as SymTimestamp};

/// Samples per AAC-LC frame (constant; the engine's own gate).
pub(crate) const SAMPLES_PER_FRAME: usize = 1024;

/// One decoded stream (the free surface's result).
#[derive(Debug, Clone)]
pub struct DecodedAac {
    /// The output shape (F32, interleaved, native channels/rate).
    pub format: AudioFormat,
    /// Interleaved F32 samples.
    pub samples: Vec<f32>,
    /// Decoded ADTS frame count.
    pub frames: usize,
    /// Byte offset of the first frame.
    pub first_frame_offset: u64,
}

/// The ASC audio object types we distinguish at configure time (the
/// first 5 bits of the AudioSpecificConfig, MSB-first).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AscObjectType {
    /// 1 — AAC Main.
    Main,
    /// 2 — AAC LC (the supported subset).
    Lc,
    /// 3 — SSR.
    Ssr,
    /// 4 — LTP.
    Ltp,
    /// 5 — SBR (HE-AAC).
    Sbr,
    /// 29 — PS (HE-AACv2).
    Ps,
    /// Anything else.
    Other(u8),
}

/// Sniff the audio object type out of an ASC's first bytes.
pub(crate) fn sniff_asc_object_type(asc: &[u8]) -> AscObjectType {
    if asc.len() < 2 {
        return AscObjectType::Other(0);
    }
    match asc[0] >> 3 {
        1 => AscObjectType::Main,
        2 => AscObjectType::Lc,
        3 => AscObjectType::Ssr,
        4 => AscObjectType::Ltp,
        5 => AscObjectType::Sbr,
        29 => AscObjectType::Ps,
        other => AscObjectType::Other(other),
    }
}

/// Refuse the honestly-absent profiles with the exact typed code
/// (shared by both configure paths).
fn gate_object_type(otype: AscObjectType) -> Result<(), AacError> {
    match otype {
        AscObjectType::Lc => Ok(()),
        AscObjectType::Sbr | AscObjectType::Ps => Err(unsupported(
            "aac.he_sbr",
            "HE-AAC (SBR/PS) is honestly absent in this build — recorded partial, \
             not silently downgraded (doc 04 §3.2)",
        )),
        AscObjectType::Main | AscObjectType::Ssr | AscObjectType::Ltp | AscObjectType::Other(_) => {
            Err(unsupported(
                "aac.profile",
                "engine implements AAC-LC only; the ASC object type is not LC",
            ))
        }
    }
}

/// The ADTS-level profile gate (the header field).
fn gate_adts_profile(profile: AdtsProfile) -> Result<(), AacError> {
    match profile {
        AdtsProfile::Lc => Ok(()),
        AdtsProfile::Main | AdtsProfile::Ssr | AdtsProfile::Reserved => Err(unsupported(
            "aac.profile",
            format!(
                "engine implements AAC-LC only; the ADTS profile field is `{}`",
                profile.name()
            ),
        )),
    }
}

/// The shared channel gates (both the stream pipeline and the
/// contract's ADTS path).
fn gate_channels(channel_config: u8, channels: u8) -> Result<(), AacError> {
    if channel_config == 0 {
        return Err(unsupported(
            "aac.channel_config_pce",
            "channel configuration 0 (PCE in band) is not supported — the engine \
             requires the channel count up front",
        ));
    }
    if channels > 2 {
        return Err(unsupported(
            "aac.channel_count",
            format!(
                "{channels} channels — the engine implements mono/stereo only \
                 (recorded partial)"
            ),
        ));
    }
    Ok(())
}

/// Build the engine for an ADTS stream from its header parameters.
fn adts_engine(header: &super::adts::AdtsHeader) -> Result<AacEngine, AacError> {
    let channels = if header.channels == 1 {
        CHANNEL_LAYOUT_MONO
    } else {
        CHANNEL_LAYOUT_STEREO
    };
    let params = AudioCodecParameters {
        codec: CODEC_ID_AAC,
        sample_rate: Some(header.sample_rate),
        channels: Some(channels),
        ..AudioCodecParameters::default()
    };
    AacEngine::try_new(&params, &AudioDecoderOptions::default()).map_err(|e| {
        unsupported(
            "aac.engine.init",
            format!("engine refused the ADTS stream: {e}"),
        )
    })
}

/// Decode one framed raw data block through the engine (shared by the
/// contract path and the stream pipeline). `payload` excludes the
/// ADTS header (and CRC). Returns the frame's sample count (1024 for
/// LC) and the decoded channel count.
fn decode_one(
    decoder: &mut AacEngine,
    payload: &[u8],
    scratch: &mut Vec<f32>,
) -> Result<(usize, usize), EngineError> {
    if payload.is_empty() {
        return Err(corrupt(
            "aac.frame.empty",
            media_types::error::CorruptDetail::Truncated {
                expected: 1,
                found: 0,
            },
            "a raw data block needs at least the element id bits",
        )
        .into_engine_error());
    }
    let pkt = PacketRef::new(0, SymTimestamp::ZERO, SymDuration::ZERO, payload);
    let decoded = match decoder.decode_ref(&pkt) {
        Ok(buf) => buf,
        Err(e) => {
            return Err(corrupt(
                "aac.frame.decode",
                media_types::error::CorruptDetail::InvalidStructure { offset: None },
                format!("engine rejected the frame: {e}"),
            )
            .into_engine_error());
        }
    };
    let frames = decoded.frames();
    let channels = decoded.spec().channels().count();
    if frames == 0 || channels == 0 {
        return Err(corrupt(
            "aac.frame.no_samples",
            media_types::error::CorruptDetail::InvalidStructure { offset: None },
            "engine decoded zero samples/channels",
        )
        .into_engine_error());
    }
    let need = frames * channels;
    if scratch.len() < need {
        scratch.resize(need, 0.0);
    }
    let out = &mut scratch[..need];
    decoded.copy_to_slice_interleaved::<f32, _>(out);
    Ok((frames, channels))
}

/// The stream-level pipeline (the conformance/differential
/// workhorse): walk ADTS frames → gate profile/config → decode each
/// → interleaved F32.
///
/// # Errors
/// Typed [`EngineError`]s: no decodable frame (`aac.stream.no_frames`
/// — `Truncated`), a non-LC profile (`aac.profile`), channel
/// configuration 0 (`aac.channel_config_pce`), > 2 channels
/// (`aac.channel_count`), a mid-stream parameter switch
/// (`aac.adts.param_switch`), engine rejections. Damage is never a
/// panic; garbage between frames is resynced past by the walker; a
/// trailing partial frame is dropped (ffmpeg's behavior).
pub fn decode_stream(data: &[u8]) -> Result<DecodedAac, EngineError> {
    let frames = walk_adts_frames(data, 0);
    if frames.is_empty() {
        return Err(corrupt(
            "aac.stream.no_frames",
            media_types::error::CorruptDetail::Truncated {
                expected: 7,
                found: data.len() as u64,
            },
            "no complete ADTS frame found in the buffer",
        )
        .into_engine_error());
    }
    let first = frames[0];
    gate_adts_profile(first.header.profile)?;
    gate_channels(first.header.channel_config, first.header.channels)?;
    let mut engine = adts_engine(&first.header)?;
    let mut scratch: Vec<f32> = Vec::new();
    let mut samples: Vec<f32> = Vec::new();
    let mut out_channels = first.header.channels as usize;
    let out_rate = first.header.sample_rate;
    for frame in &frames {
        // The mid-stream switch gate: ADTS lets a stream re-declare
        // rate/channels per frame; the engine is configured once, so a
        // disagreement is typed damage, never a wrong-rate decode.
        if frame.header.sample_rate != out_rate
            || frame.header.channels as usize != out_channels
            || frame.header.profile != first.header.profile
        {
            return Err(corrupt(
                "aac.adts.param_switch",
                media_types::error::CorruptDetail::InvalidStructure {
                    offset: Some(frame.offset as u64),
                },
                "an ADTS frame's parameters disagree with the configured stream",
            )
            .into_engine_error());
        }
        let start = frame.offset + frame.header.header_len;
        let end = frame.offset + frame.header.frame_len;
        let payload = data.get(start..end).unwrap_or(&[]);
        let (decoded_frames, channels) = decode_one(&mut engine, payload, &mut scratch)?;
        out_channels = channels;
        samples.extend_from_slice(&scratch[..decoded_frames * channels]);
    }
    Ok(DecodedAac {
        format: AudioFormat {
            sample: SampleFormat::F32,
            channels: out_channels as u32,
            layout: AudioLayout::Interleaved,
            rate: out_rate,
        },
        samples,
        frames: frames.len(),
        first_frame_offset: first.offset as u64,
    })
}

/// AAC-LC decoder behind the codec-api [`Decoder`] contract (card
/// 5.C2a; doc 04 §3.1 rows "AAC-LC" + "AAC in MP4 vs ADTS").
///
/// Two configure shapes:
///
/// - **ADTS mode** (no `extra_data`): one packet = one complete ADTS
///   frame, header included. The header is parsed per packet (the
///   decode entry is ADTS-frame-aware) and the engine is built
///   lazily from the first packet's parameters; later packets must
///   agree with them (`aac.adts.param_switch` otherwise).
/// - **ASC mode** (`extra_data` = the AudioSpecificConfig, ≥ 2 bytes
///   — the MP4/esds path): one packet = one raw data block. The ASC
///   is sniffed up front: HE (AOT 5/29) refuses `aac.he_sbr`, non-LC
///   refuses `aac.profile` — never a silent downgrade.
#[derive(Default)]
pub struct AacDecoder {
    /// The configured stream generation (None until `configure`).
    generation: Option<GenerationId>,
    /// The engine: built at configure (ASC mode) or at the first
    /// packet's header (ADTS mode).
    engine: Option<AacEngine>,
    /// ASC mode (raw data blocks) vs ADTS mode (framed packets).
    raw_mode: bool,
    /// Packets fed so far (pts bookkeeping).
    packet_index: u64,
    /// Per-channel samples fed so far.
    samples_fed: u64,
    /// Reusable interleaved scratch.
    scratch: Vec<f32>,
    /// Pooled output slots (the 5.A1 rotation).
    sample_pool: Vec<SharedBytes>,
    /// Round-robin cursor.
    slot_next: usize,
    /// Negotiated output shape (0 until the first decode).
    out_channels: u32,
    out_rate: u32,
}

impl std::fmt::Debug for AacDecoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AacDecoder")
            .field("raw_mode", &self.raw_mode)
            .field("packet_index", &self.packet_index)
            .field("samples_fed", &self.samples_fed)
            .finish_non_exhaustive()
    }
}

/// Output-rotation depth (the 5.A1 bound).
const SAMPLE_SLOTS: usize = 4;

/// Max interleaved bytes per frame (1024 samples × 2 channels × f32).
const MAX_SLOT_BYTES: usize = SAMPLES_PER_FRAME * 2 * 4;

impl AacDecoder {
    /// A fresh decoder, unconfigured.
    #[must_use]
    pub fn new() -> AacDecoder {
        AacDecoder::default()
    }

    /// The codec descriptors this decoder accepts.
    fn accepts(descriptor: &str) -> bool {
        descriptor == "aac"
    }
}

impl Decoder for AacDecoder {
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
        let descriptor = cfg.codec.as_str();
        if !Self::accepts(descriptor) {
            return Err(unsupported(
                "aac.codec_descriptor",
                format!("codec descriptor `{descriptor}` is not aac"),
            )
            .into_engine_error());
        }
        let mut params = AudioCodecParameters {
            codec: CODEC_ID_AAC,
            ..AudioCodecParameters::default()
        };
        let raw_mode = match cfg.extra_data.as_ref().map(|d| d.as_slice()) {
            None | Some([]) => false,
            Some(asc) => {
                if asc.len() < 2 {
                    return Err(unsupported(
                        "aac.asc.length",
                        "extra_data present but shorter than the 2-byte ASC minimum",
                    )
                    .into_engine_error());
                }
                gate_object_type(sniff_asc_object_type(asc))?;
                params.extra_data = Some(asc.to_vec().into_boxed_slice());
                true
            }
        };
        // ASC mode can build the engine now (the ASC carries the
        // parameters); ADTS mode defers to the first packet's header.
        let engine = if raw_mode {
            Some(
                AacEngine::try_new(&params, &AudioDecoderOptions::default()).map_err(|e| {
                    unsupported(
                        "aac.engine.init",
                        format!("engine refused the configuration: {e}"),
                    )
                    .into_engine_error()
                })?,
            )
        } else {
            None
        };
        self.engine = engine;
        self.generation = Some(cfg.generation);
        self.raw_mode = raw_mode;
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
            None => return Err(AacError::BadState.into_engine_error()),
        };
        if let Some(outcome) = drop_stale_packet(&packet, generation) {
            return Ok(outcome);
        }
        let data = packet.data.as_slice();
        if data.is_empty() {
            return Ok(DecodeOutcome::NeedMoreInput);
        }
        // ADTS mode: parse + strip the frame header per packet (the
        // ADTS-frame-aware entry), building the engine from the first
        // header; raw mode feeds the packet as-is.
        let payload: &[u8] = if self.raw_mode {
            data
        } else {
            let header = parse_adts_header(data)?;
            gate_adts_profile(header.profile)?;
            gate_channels(header.channel_config, header.channels)?;
            if data.len() < header.frame_len {
                return Err(corrupt(
                    "aac.adts.truncated",
                    media_types::error::CorruptDetail::Truncated {
                        expected: header.frame_len as u64,
                        found: data.len() as u64,
                    },
                    "the ADTS frame claims more bytes than the packet holds",
                )
                .into_engine_error());
            }
            if self.out_rate == 0 {
                let engine = adts_engine(&header)?;
                self.engine = Some(engine);
                self.out_rate = header.sample_rate;
                self.out_channels = u32::from(header.channels.max(1));
            } else if header.sample_rate != self.out_rate
                || u32::from(header.channels.max(1)) != self.out_channels
            {
                return Err(corrupt(
                    "aac.adts.param_switch",
                    media_types::error::CorruptDetail::InvalidStructure { offset: Some(0) },
                    "the packet's ADTS parameters disagree with the configured stream",
                )
                .into_engine_error());
            }
            data.get(header.header_len..header.frame_len).unwrap_or(&[])
        };
        let engine = match self.engine.as_mut() {
            Some(e) => e,
            None => return Err(AacError::BadState.into_engine_error()),
        };
        let (frame_samples, channels) = decode_one(engine, payload, &mut self.scratch)?;
        if self.out_channels == 0 {
            self.out_channels = channels as u32;
            self.out_rate = engine.last_decoded().spec().rate();
        }
        let channels = self.out_channels as usize;
        let interleaved = &self.scratch[..frame_samples * channels];
        self.packet_index += 1;
        let emitted_samples = interleaved.len() / channels.max(1);
        if emitted_samples > 0 {
            let bytes = pooled_sample_slot(&mut self.sample_pool, &mut self.slot_next, |dst| {
                let mut cursor = 0usize;
                for v in interleaved {
                    dst[cursor..cursor + 4].copy_from_slice(&v.to_ne_bytes());
                    cursor += 4;
                }
                Ok::<(), AacError>(())
            })?;
            let exact = emitted_samples * channels * 4;
            let base = TimeBase::new(1, self.out_rate.max(1)).map_err(|_| {
                unsupported("aac.stream.rate", "negotiated rate is 0").into_engine_error()
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
        if self.generation.is_none() {
            return Err(AacError::BadState.into_engine_error());
        }
        // The engine keeps IMDCT overlap state inside one frame's
        // synthesis; between packets nothing is buffered that flush
        // would emit.
        Ok(DecodeOutcome::Drained)
    }
}

/// The pooled output rotation (the 5.A1 pattern, re-authored for the
/// AAC slot size).
fn pooled_sample_slot(
    slots: &mut Vec<SharedBytes>,
    next: &mut usize,
    fill: impl FnOnce(&mut [u8]) -> Result<(), AacError>,
) -> Result<SharedBytes, EngineError> {
    let count = slots.len();
    for i in 0..count {
        let idx = (*next + i) % count;
        if slots[idx].len() != MAX_SLOT_BYTES {
            continue;
        }
        if let Some(bytes) = slots[idx].get_mut() {
            fill(bytes)?;
            *next = (idx + 1) % count;
            return Ok(slots[idx].clone());
        }
    }
    let mut fresh = SharedBytes::from_vec(vec![0; MAX_SLOT_BYTES]);
    match fresh.get_mut() {
        Some(bytes) => fill(bytes),
        None => {
            return Err(corrupt(
                "aac.output.fresh_slot_not_exclusive",
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
    fn descriptor_gate_accepts_aac_only() {
        assert!(AacDecoder::accepts("aac"));
        assert!(!AacDecoder::accepts("flac"));
        assert!(!AacDecoder::accepts("mp3"));
        assert!(!AacDecoder::accepts("AAC"));
        assert!(!AacDecoder::accepts(""));
    }

    #[test]
    fn asc_object_type_sniffing_spots_the_he_family() {
        // AOT occupies the first 5 bits, MSB-first.
        let asc = |aot: u8| [aot << 3, 0x00];
        assert_eq!(sniff_asc_object_type(&asc(2)), AscObjectType::Lc);
        assert_eq!(sniff_asc_object_type(&asc(1)), AscObjectType::Main);
        assert_eq!(sniff_asc_object_type(&asc(4)), AscObjectType::Ltp);
        assert_eq!(sniff_asc_object_type(&asc(5)), AscObjectType::Sbr);
        assert_eq!(sniff_asc_object_type(&asc(29)), AscObjectType::Ps);
        assert_eq!(sniff_asc_object_type(&asc(6)), AscObjectType::Other(6));
        assert_eq!(sniff_asc_object_type(&[0x00]), AscObjectType::Other(0));
    }

    #[test]
    fn he_asc_refuses_typed_and_never_reaches_the_engine() {
        let mut decoder = AacDecoder::new();
        let generation = match GenerationId::FIRST.next() {
            Ok(g) => g,
            Err(e) => unreachable!("{e:?}"),
        };
        let mut try_configure = |extra: Option<SharedBytes>| {
            decoder.configure(&DecoderConfig::new(
                match media_types::ids::CodecDescriptor::new("aac") {
                    Ok(d) => d,
                    Err(e) => unreachable!("{e:?}"),
                },
                generation,
                ThreadBudget::default(),
                None,
                extra,
            ))
        };
        // AOT=5 (SBR): bits 00101 → 0b0010_1xxx.
        let he_asc = SharedBytes::from_vec(vec![0b0010_1000, 0x00]);
        let e = match try_configure(Some(he_asc)) {
            Err(e) => e,
            Ok(()) => unreachable!("HE-AAC must be refused typedly"),
        };
        assert_eq!(e.context.code, "aac.he_sbr");
        assert!(matches!(e.kind, media_types::ErrorKind::Unsupported { .. }));
        // AOT=4 (LTP): the plain profile refusal.
        let ltp_asc = SharedBytes::from_vec(vec![0b0010_0000, 0x00]);
        let e = match try_configure(Some(ltp_asc)) {
            Err(e) => e,
            Ok(()) => unreachable!("LTP must be refused typedly"),
        };
        assert_eq!(e.context.code, "aac.profile");
        // A 1-byte ASC is a length refusal.
        let e = match try_configure(Some(SharedBytes::from_vec(vec![0x10]))) {
            Err(e) => e,
            Ok(()) => unreachable!("short ASC must be refused"),
        };
        assert_eq!(e.context.code, "aac.asc.length");
    }

    #[test]
    fn decode_before_configure_is_typed_bad_state() {
        let mut decoder = AacDecoder::new();
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
        assert_eq!(e.context.code, "aac.not_configured");
        assert!(matches!(e.kind, media_types::ErrorKind::Decoder { .. }));
    }

    #[test]
    fn configure_rejects_foreign_descriptors() {
        let mut decoder = AacDecoder::new();
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
        assert_eq!(e.context.code, "aac.codec_descriptor");
    }

    #[test]
    fn empty_buffer_and_garbage_refuse_typed_not_panicking() {
        let e = match decode_stream(&[]) {
            Err(e) => e,
            Ok(_) => unreachable!("empty must be refused"),
        };
        assert_eq!(e.context.code, "aac.stream.no_frames");
        let e = match decode_stream(&[0xFF; 4096]) {
            Err(e) => e,
            Ok(_) => unreachable!("garbage must be refused"),
        };
        assert_eq!(e.context.code, "aac.stream.no_frames");
        assert!(matches!(e.kind, media_types::ErrorKind::Corrupt { .. }));
    }
}
