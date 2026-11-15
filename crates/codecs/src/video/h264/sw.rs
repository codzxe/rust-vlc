//! The H.264 **software decode track** — the Lead-admitted
//! `rust_h264` engine (0.6.1, exact pin) behind the codec-api
//! contract, closing ADR-001's conformance-gated software lane.
//!
//! # Admission (the record, not hope)
//!
//! `rust_h264 = "=0.6.1"`, default features (`std` only) —
//! `docs/dependencies/records/rust_h264-0.6.1.md`, building on the
//! issue-#2 candidate audits
//! (`docs/dependencies/records/h264-software-decoder-audits.md`:
//! pure Rust, **zero dependencies**, no build script, no FFI; the
//! SIMD/once/picture-sync `unsafe` is crate-internal and unreachable
//! from the public API). This module's differential suite
//! (`tests/h264_sw_differential.rs`) is the admission's binding
//! evidence: six ffmpeg-authored lanes (Constrained Baseline CAVLC,
//! Main CABAC, Main + B-frames, High + B-frames, odd-size crop,
//! single-IDR still) measured **bit-exact against ffmpeg 7.1.5's
//! own `libx264` decode** on BOTH entry forms — Annex-B elementary
//! stream and MP4/AVCC samples — plus the hostile-input battery
//! (garbage packets recovered, truncation sweep panic-free).
//!
//! # What is ours, what is the engine's (the boundary)
//!
//! Ours (the in-house [`super::parser`] surface, reused from the
//! 5.F1 cards): NAL framing on both wires
//! ([`AnnexBNals`](super::parser::nal::AnnexBNals),
//! [`AvcNals`](super::parser::nal::AvcNals)), emulation-prevention
//! removal ([`unescape_rbsp_into`](super::parser::nal::unescape_rbsp_into)),
//! the `avcC` record parse
//! ([`AvcConfig`](super::parser::nal::AvcConfig)), the **damage
//! accounting** (every skipped unit is counted and reported as
//! [`DecodeOutcome::Recovered`], never a silent drop), and the whole
//! codec-api contract surface (configure/decode/flush, generational
//! staleness, pooled output slots, typed refusals). The engine's:
//! entropy decoding, prediction, reconstruction, DPB management and
//! display-order output (its `OrderedDecoder` carries our per-picture
//! [`Stamp`] metas through B-frame reordering, so each frame leaves
//! with the presentation time of the packet that carried its first
//! slice).
//!
//! # Output surface and its honest boundary
//!
//! Frames are tight planar I420 (`y ‖ u ‖ v`) on
//! [`FrameStorage::Cpu`] with the SPS-cropped display size — the
//! first vertical's presentable shape. Colorimetry is the SPS VUI
//! when it specifies one, else the H.264 family's recorded default
//! posture ([`ColorDescription::BT709_LIMITED`], the `0019`-corpus
//! convention — the hw lane's `HW_COLOR` twin). Interlaced-source
//! pictures (PAFF/MBAFF, `frame_mbs_only_flag` clear) are weaved
//! into full frames by the engine and flagged `interlaced`. The
//! engine decodes more than the v1 vocabulary carries — 4:2:2 and
//! over-8-bit surfaces refuse with the typed `h264.sw.surface_format`
//! (the `vp9.bitdepth` precedent), never a mislabel, never silence.
//! Profiles beyond the engine's reach (4:4:4, lossless bypass, SP/SI
//! slices, FMO slice groups) refuse with the typed
//! `h264.sw.unsupported` carrying the engine's own rule string.
//!
//! # Damage model
//!
//! A NAL that fails framing, header split, EPB removal or engine
//! parsing is **skipped and counted** — the call reports
//! [`DecodeOutcome::Recovered { CorruptInput }`], the decoder stays
//! usable, and later NALs (a following IDR, in-band parameter sets)
//! resynchronize it. Configure-time extradata damage is a typed
//! error (the stream is not decodable as configured — the
//! `H264Decoder` twin's posture). A typed `Err` from a slice's
//! `Unsupported` is fatal per generation (the contract's rule); the
//! engine family that produced it answers every later slice the
//! same way, so the first refusal is the honest one.
//!
//! # Seek / reconfiguration
//!
//! The engine has no mid-stream reset (its DPB would corrupt P
//! pictures after a seek), so [`Decoder::configure`] rebuilds the
//! whole session per generation — the contract's fresh-generation
//! rule — re-seeding the retained extradata (the host's
//! `ReseedingDecoder` re-offers it on every reconfigure).

use std::borrow::Cow;

use codec_api::prelude::*;
use media_types::error::{DecodeCause, ErrorContext, FormatInfo};
use media_types::frame::{
    ColorDescription, ColorMatrix, ColorPrimaries, ColorRange, Frame, FrameStorage,
    TransferCharacteristics, VideoFrame,
};
use media_types::{EngineError, ErrorKind, GenerationId, Packet, PipelineStage, Stamp};
use rust_h264::decoder::OrderedDecoder;
use rust_h264::error::DecodeError;
use rust_h264::nal::{NalUnit, NalUnitType as EngineNalType, parse_sps_info};

use super::parser::nal::{
    AnnexBNals, AvcConfig, AvcNals, H264Error, NalHeader, NalUnitType, unescape_rbsp_into,
};

/// Pooled output-slot ceiling (the mjpeg/vp9/av1 `FRAME_SLOTS`
/// convention — bounded memory by construction, MP §12).
const FRAME_SLOTS: usize = 4;

/// The H.264 software decoder: the `rust_h264` engine behind the
/// codec-api [`Decoder`] contract.
///
/// Construct with [`H264SwDecoder::new`] and hand it to the host's
/// provider seam; [`Decoder::configure`] accepts the `h264` /
/// `avc1`[.profile] descriptor family with optional `extra_data`
/// (an `avcC` record for ISO-BMFF tracks — the framing switches to
/// length-prefixed samples; an Annex-B `CodecPrivate` for
/// byte-stream containers) and decodes with zero hardware
/// involvement (the `features.hw-decode` OFF default's path).
pub struct H264SwDecoder {
    /// The engine: display-order decoder carrying per-picture
    /// [`Stamp`] metas (B-frame reordering keeps each frame's own
    /// presentation time).
    ordered: OrderedDecoder<Stamp>,
    /// Configured stream generation (`None` until `configure`).
    generation: Option<GenerationId>,
    /// Packet framing in force (Annex-B until an `avcC` says
    /// otherwise).
    framing: Option<u8>, // Some(length_size) = AVC, None = Annex-B
    /// Active colorimetry (VUI when specified, else the family
    /// default posture).
    color: ColorDescription,
    /// Interlaced source (`frame_mbs_only_flag` clear on the
    /// active SPS).
    interlaced: bool,
    /// Pooled output slots (the mjpeg/vp9 rotation pattern).
    slots: Vec<SharedBytes>,
    /// Rotation cursor for `slots`.
    slot_next: usize,
    /// Reusable RBSP buffer (zero steady-state allocation; the
    /// engine's `NalUnit` borrows it per feed).
    rbsp: Vec<u8>,
    /// Damage skipped in the current call (recovery accounting).
    skipped: u32,
}

impl std::fmt::Debug for H264SwDecoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("H264SwDecoder")
            .field("backend", &"rust_h264-0.6.1")
            .field("configured", &self.generation.is_some())
            .field(
                "framing",
                &if self.framing.is_some() {
                    "avc"
                } else {
                    "annexb"
                },
            )
            .field("interlaced", &self.interlaced)
            .finish()
    }
}

impl Default for H264SwDecoder {
    fn default() -> Self {
        H264SwDecoder {
            ordered: OrderedDecoder::new(),
            generation: None,
            framing: None,
            color: ColorDescription::BT709_LIMITED,
            interlaced: false,
            slots: Vec::new(),
            slot_next: 0,
            rbsp: Vec::new(),
            skipped: 0,
        }
    }
}

impl H264SwDecoder {
    /// A fresh software decoder (the explicit-zero constructor; no
    /// settings, no hardware — the `software-only` default's H.264
    /// path).
    pub fn new() -> H264SwDecoder {
        H264SwDecoder::default()
    }

    /// The codec descriptors this decoder accepts (the
    /// [`super::decode::H264Decoder`] twin's set: the MKV `h264`
    /// mapping and the MP4 `avc1`[.profile] fourcc style).
    fn accepts(descriptor: &str) -> bool {
        descriptor == "h264" || descriptor == "avc1" || descriptor.starts_with("avc1.")
    }

    /// Typed decode-time refusal (`h264.*` code, h264 format info).
    fn unsupported(code: &'static str) -> EngineError {
        EngineError::new(
            PipelineStage::Decode,
            ErrorKind::Unsupported {
                format: Some(FormatInfo {
                    name: Some("h264".to_string()),
                    sniffed: Vec::new(),
                }),
            },
            ErrorContext::new(code),
        )
    }

    /// Map a parser/extradata failure to a typed engine error
    /// (configure-time only — in-stream damage is the recovery
    /// path instead).
    fn parse_error(e: H264Error, code: &'static str) -> EngineError {
        EngineError::new(
            PipelineStage::Decode,
            ErrorKind::Decoder {
                cause: DecodeCause::Bitstream,
            },
            ErrorContext::new(code),
        )
        .with_source(e)
    }

    /// Map an engine refusal to a typed engine error, carrying the
    /// engine's own rule string in the source (the honest
    /// "why unsupported" — never a guess).
    fn engine_refusal(e: DecodeError, code: &'static str) -> EngineError {
        EngineError::new(
            PipelineStage::Decode,
            ErrorKind::Unsupported {
                format: Some(FormatInfo {
                    name: Some("h264".to_string()),
                    sniffed: Vec::new(),
                }),
            },
            ErrorContext::new(code),
        )
        .with_source(e)
    }

    /// Seed parameter sets + framing from out-of-band bytes (the
    /// [`super::decode::H264Decoder::seed_extradata`] posture: avcC
    /// switches the framing, Annex-B keeps it; damage here is a
    /// typed configure error).
    fn seed_extradata(&mut self, extra: &[u8]) -> Result<(), EngineError> {
        if AvcConfig::looks_like_avcc(extra) {
            let cfg = AvcConfig::parse(extra)
                .map_err(|e| Self::parse_error(e, "h264.sw.extradata.avcc"))?;
            self.framing = Some(cfg.length_size);
            for nal in &cfg.sps {
                self.feed_param_set(nal, Stamp::Missing)?;
            }
            for nal in &cfg.pps {
                self.feed_param_set(nal, Stamp::Missing)?;
            }
        } else {
            // Annex-B CodecPrivate (TS-flavored containers): the
            // parameter sets are extracted; framing stays byte-stream.
            for nal in AnnexBNals::new(extra) {
                if let Ok((header, _)) = NalHeader::split(nal)
                    && matches!(header.unit_type, NalUnitType::Sps | NalUnitType::Pps)
                {
                    self.feed_param_set(nal, Stamp::Missing)?;
                }
            }
        }
        Ok(())
    }

    /// Build the engine's [`NalUnit`] from one raw NAL (header byte
    /// included, framing stripped): the in-house EPB unescaper fills
    /// the reusable RBSP buffer and the engine borrows it for the
    /// one call (zero steady-state allocation). `Err(())` = framing
    /// or EPB damage (counted by the caller).
    fn bridge_nal<'r>(rbsp: &'r mut Vec<u8>, nal: &[u8]) -> Result<NalUnit<'r>, ()> {
        let (header, payload) = NalHeader::split(nal).map_err(|_| ())?;
        unescape_rbsp_into(payload, rbsp).map_err(|_| ())?;
        Ok(NalUnit {
            nal_ref_idc: header.nal_ref_idc,
            nal_unit_type: EngineNalType::from(header.unit_type.id()),
            rbsp: Cow::Borrowed(rbsp.as_slice()),
        })
    }

    /// Feed one parameter-set NAL (configure path: a typed error on
    /// damage — the stream is not decodable as configured) and, for
    /// an SPS, adopt its VUI colorimetry + progressive flag (the
    /// engine's own probe, on the same RBSP we feed it).
    fn feed_param_set(&mut self, nal: &[u8], meta: Stamp) -> Result<(), EngineError> {
        // The SPS facts are extracted while the RBSP borrow is live
        // and adopted after the feed (the engine's refusal decides
        // whether the set becomes active).
        let mut sps_info = None;
        let feed = {
            // Disjoint field borrows: `rbsp` (through the borrowed
            // `NalUnit`) and `ordered` (mutably, for the feed).
            let unit = match Self::bridge_nal(&mut self.rbsp, nal) {
                Ok(unit) => unit,
                Err(()) => return Err(Self::unsupported("h264.sw.extradata.nal")),
            };
            if unit.nal_unit_type == EngineNalType::Sps {
                sps_info = parse_sps_info(&unit).ok();
            }
            self.ordered.decode_nal_with_meta(&unit, meta)
        };
        match feed {
            Ok(_) => {
                if let Some(info) = sps_info {
                    self.adopt_sps_color(info);
                }
                Ok(())
            }
            Err(DecodeError::Unsupported(what)) => Err(Self::engine_refusal(
                DecodeError::Unsupported(what),
                "h264.sw.unsupported",
            )),
            Err(e) => Err(Self::engine_refusal(e, "h264.sw.extradata.decode")),
        }
    }

    /// Adopt an SPS's VUI colorimetry + scan posture (unspecified
    /// values keep the family default — the recorded 0019-corpus
    /// convention, never a guess dressed as fact).
    fn adopt_sps_color(&mut self, info: rust_h264::nal::SpsInfo) {
        if !info.progressive {
            self.interlaced = true;
        }
        let primaries = match info.primaries {
            1 => Some(ColorPrimaries::Bt709),
            4..=7 => Some(ColorPrimaries::Bt601),
            9 => Some(ColorPrimaries::Bt2020),
            _ => None,
        };
        let transfer = match info.transfer {
            13 => Some(TransferCharacteristics::Srgb),
            16 => Some(TransferCharacteristics::Pq),
            18 => Some(TransferCharacteristics::Hlg),
            // 1 (BT.709), 6 (BT.601), 14/15 (extended gamma) and
            // every unspecified value: the BT.709 posture.
            _ => Some(TransferCharacteristics::Bt709),
        };
        let matrix = match info.matrix {
            1 => Some(ColorMatrix::Bt709),
            5..=7 => Some(ColorMatrix::Bt601),
            9 => Some(ColorMatrix::Bt2020Ncl),
            _ => None,
        };
        let range = if info.full_range {
            ColorRange::Full
        } else {
            ColorRange::Limited
        };
        let mut color = self.color;
        if let Some(p) = primaries {
            color.primaries = p;
        }
        if let Some(t) = transfer {
            color.transfer = t;
        }
        if let Some(m) = matrix {
            color.matrix = m;
        }
        color.range = range;
        self.color = color;
    }

    /// Convert one engine frame to the engine's [`Frame`] and push
    /// it (the pooled tight-I420 slot pattern). Returns frames
    /// emitted (0 or 1).
    fn emit_frame(
        &mut self,
        frame: rust_h264::decoder::Frame,
        pts: Stamp,
        out: &mut dyn FrameSink,
    ) -> Result<u32, EngineError> {
        // The v1 output vocabulary: 8-bit 4:2:0 planar (monochrome
        // decodes to 420 with neutral chroma — the corpus 0019
        // convention, same as the hw lane's Nv12 posture). 4:2:2 and
        // over-8-bit are honest typed refusals, never mislabels.
        let eight_bit_420 =
            frame.bit_depth == 8 && frame.chroma_format == rust_h264::decoder::ChromaFormat::Yuv420;
        if !eight_bit_420 {
            return Err(Self::unsupported("h264.sw.surface_format"));
        }
        let width = frame.width as usize;
        let height = frame.height as usize;
        if width == 0 || height == 0 {
            // A crop that consumed the whole picture (engine drops
            // it as undisplayable): damage, not a frame.
            self.skipped = self.skipped.saturating_add(1);
            return Ok(0);
        }
        // 4:2:0 geometry note: H.264 crop offsets come in units of
        // two luma samples for 4:2:0 (Table 6-1), so the accepted
        // subset's display sizes are always even — floor division
        // equals ceil here, matching both the engine's plane lengths
        // and the consumer's div_ceil expectation. (4:2:2/4:4:4,
        // where odd displays exist, refused above.)
        let chroma_w = width / 2;
        let chroma_h = height / 2;
        let y_len = width * height;
        let c_len = chroma_w * chroma_h;
        let total = y_len + 2 * c_len;
        if frame.y.len() < y_len || frame.u.len() < c_len || frame.v.len() < c_len {
            // Engine contract breach (its frames are always whole):
            // count the damage, never a panic, never a partial frame.
            self.skipped = self.skipped.saturating_add(1);
            return Ok(0);
        }
        let generation = self.generation.unwrap_or(GenerationId::FIRST);
        let data = pooled_frame_slot(&mut self.slots, &mut self.slot_next, total, |slot| {
            slot[..y_len].copy_from_slice(&frame.y[..y_len]);
            slot[y_len..y_len + c_len].copy_from_slice(&frame.u[..c_len]);
            slot[y_len + c_len..].copy_from_slice(&frame.v[..c_len]);
        })?;
        out.accept(Frame::Video(VideoFrame {
            format: media_types::PixelFormat::I420,
            size: (frame.width, frame.height),
            data: FrameStorage::Cpu(data),
            pts,
            color: self.color,
            interlaced: self.interlaced,
            generation,
        }))?;
        Ok(1)
    }
}

impl Decoder for H264SwDecoder {
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
        if !Self::accepts(cfg.codec.as_str()) {
            let mut err = Self::unsupported("h264.sw.codec_descriptor");
            err.kind = ErrorKind::Unsupported {
                format: Some(FormatInfo {
                    name: Some(cfg.codec.to_string()),
                    sniffed: Vec::new(),
                }),
            };
            return Err(err);
        }
        // Reset per-generation state (seek / source switch): the
        // engine has no mid-stream reset, so the session is rebuilt
        // whole — DPB, parameter sets, colorimetry, the framing
        // hypothesis (Annex-B until an avcC says otherwise).
        self.ordered = OrderedDecoder::new();
        self.generation = Some(cfg.generation);
        self.framing = None;
        self.color = ColorDescription::BT709_LIMITED;
        self.interlaced = false;
        self.slot_next = 0;
        self.skipped = 0;
        if let Some(extra) = cfg.extra_data.as_ref() {
            self.seed_extradata(extra.as_slice())?;
        }
        Ok(())
    }

    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        let Some(generation) = self.generation else {
            return Err(EngineError::new(
                PipelineStage::Decode,
                ErrorKind::Decoder {
                    cause: DecodeCause::BadState,
                },
                ErrorContext::new("h264.sw.not_configured"),
            ));
        };
        if let Some(outcome) = drop_stale_packet(&packet, generation) {
            return Ok(outcome);
        }
        self.skipped = 0;
        let mut emitted = 0u32;

        // Split by the framing in force (the in-house iterators:
        // total on any byte string, damage counted, never a panic).
        let mut nals: Vec<&[u8]> = Vec::new();
        match self.framing {
            None => nals.extend(AnnexBNals::new(packet.data.as_slice())),
            Some(length_size) => match AvcNals::new(packet.data.as_slice(), length_size) {
                Ok(iter) => {
                    for item in iter {
                        match item {
                            Ok(nal) => nals.push(nal),
                            // Framing damage: the splitter has
                            // terminated itself; count and keep what
                            // did split.
                            Err(_) => {
                                self.skipped += 1;
                                break;
                            }
                        }
                    }
                }
                Err(e) => return Err(Self::parse_error(e, "h264.sw.avc.length_size")),
            },
        }

        for nal in nals {
            // The SPS facts ride out of the RBSP-borrow scope; the
            // color/scan posture is adopted only after the engine
            // accepted the set (its refusal decides activation).
            let mut sps_info = None;
            let feed = {
                // The RBSP buffer borrows into the engine's NalUnit
                // while `ordered` is fed — disjoint fields, sequential
                // lifetimes.
                let unit = match Self::bridge_nal(&mut self.rbsp, nal) {
                    Ok(unit) => unit,
                    Err(()) => {
                        self.skipped += 1;
                        continue;
                    }
                };
                if unit.nal_unit_type == EngineNalType::Sps {
                    sps_info = parse_sps_info(&unit).ok();
                }
                // Every NAL carries this packet's stamp; the engine
                // keeps the picture's FIRST-slice stamp through
                // reordering (continuation slices cannot overwrite
                // it).
                self.ordered.decode_nal_with_meta(&unit, packet.pts)
            };
            match feed {
                Ok(frames) => {
                    if let Some(info) = sps_info {
                        self.adopt_sps_color(info);
                    }
                    for (frame, pts) in frames {
                        emitted += self.emit_frame(frame, pts, out)?;
                    }
                }
                Err(DecodeError::Unsupported(what)) => {
                    // The engine family's hard boundary (4:4:4,
                    // lossless bypass, SP/SI, FMO…): it answers every
                    // later slice identically — the first refusal is
                    // the honest, fatal-for-this-generation one.
                    return Err(Self::engine_refusal(
                        DecodeError::Unsupported(what),
                        "h264.sw.unsupported",
                    ));
                }
                Err(DecodeError::UnexpectedEof | DecodeError::InvalidSyntax(_)) => {
                    // In-stream damage: skipped and counted; the
                    // engine stays usable and resynchronizes on the
                    // next parameter set / IDR.
                    self.skipped += 1;
                }
            }
        }

        Ok(if emitted > 0 {
            if self.skipped > 0 {
                DecodeOutcome::Recovered {
                    skipped: self.skipped,
                    reason: RecoverReason::CorruptInput,
                }
            } else {
                DecodeOutcome::Emitted { frames: emitted }
            }
        } else if self.skipped > 0 {
            DecodeOutcome::Recovered {
                skipped: self.skipped,
                reason: RecoverReason::CorruptInput,
            }
        } else {
            DecodeOutcome::NeedMoreInput
        })
    }

    fn flush(&mut self, out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        if self.generation.is_none() {
            return Err(EngineError::new(
                PipelineStage::Decode,
                ErrorKind::Decoder {
                    cause: DecodeCause::BadState,
                },
                ErrorContext::new("h264.sw.not_configured"),
            ));
        }
        self.skipped = 0;
        let drained = self.ordered.flush_with_meta();
        let mut emitted = 0u32;
        for (frame, pts) in drained {
            emitted += self.emit_frame(frame, pts, out)?;
        }
        Ok(if emitted > 0 {
            if self.skipped > 0 {
                DecodeOutcome::Recovered {
                    skipped: self.skipped,
                    reason: RecoverReason::CorruptInput,
                }
            } else {
                DecodeOutcome::Emitted { frames: emitted }
            }
        } else {
            DecodeOutcome::Drained
        })
    }
}

/// Pooled output-slot rotation (the mjpeg/vp9/av1
/// `pooled_frame_slot` pattern): steady-state decodes refill a slot
/// in place; a slot still owned downstream falls through to the
/// next, and only an all-busy rotation allocates.
fn pooled_frame_slot(
    slots: &mut Vec<SharedBytes>,
    next: &mut usize,
    len: usize,
    fill: impl FnOnce(&mut [u8]),
) -> Result<SharedBytes, EngineError> {
    let count = slots.len();
    for i in 0..count {
        let idx = (*next + i) % count;
        if slots[idx].len() != len {
            continue;
        }
        if let Some(bytes) = slots[idx].get_mut() {
            fill(bytes);
            *next = (idx + 1) % count;
            return Ok(slots[idx].clone());
        }
        // The slot still has downstream owners — try the next one.
    }
    let mut fresh = SharedBytes::from_vec(vec![0; len]);
    match fresh.get_mut() {
        Some(bytes) => fill(bytes),
        None => {
            // A freshly built `SharedBytes` has refcount 1 by
            // construction; `get_mut` failing here is impossible —
            // surface it loudly instead of fabricating an empty
            // frame.
            return Err(EngineError::new(
                PipelineStage::Decode,
                ErrorKind::Decoder {
                    cause: DecodeCause::BadState,
                },
                ErrorContext::new("h264.sw.fresh_slot_not_exclusive"),
            ));
        }
    }
    if slots.len() < FRAME_SLOTS {
        slots.push(fresh.clone());
    }
    *next = 0;
    Ok(fresh)
}
