//! The HEVC codec-api decoder (card 5.G2): **the hardware
//! decode-path integration — no software decode, by design and by
//! ADR-001** (the exact mirror of the 5.F2 H.264 shape).
//!
//! [`HevcDecoder`] is the `Decoder` impl for HEVC/H.265 tracks: it
//! configures from the container's out-of-band bytes (`hvcC` record
//! or Annex-B `CodecPrivate`) or from in-band parameter sets, routes
//! every packet through the LANDED 5.G1 parser, assembles access
//! units (§7.4.2.4.4 boundary detection), and feeds them to a
//! hardware session through the [`super::hw`] seam. There is no
//! software fallback anywhere in this file: per ADR-001 §1 a machine
//! without a usable hardware path gets a **typed
//! [`ErrorKind::Unsupported`]** configure error (`hevc.hw.required`
//! / `hevc.hw.no_bridge`), which the feature matrix reports
//! truthfully — never a silent skip, never a fabricated frame.
//!
//! # Access-unit assembly (§7.4.2.4.4)
//!
//! A slice segment with `first_slice_segment_in_pic_flag == 1`
//! begins a new picture when it differs from the pending picture in
//! any of the spec's comparison fields — `slice_pic_order_cnt_lsb`
//! (absent for IDR: §7.4.8 defines POC = 0 there), PPS id, NAL type,
//! layer/temporal ids, `pic_output_flag`,
//! `no_output_of_prior_pics_flag` — **or** when a non-VCL unit
//! intervened since the pending picture's last VCL (the spec's
//! "the previous NAL unit was not a VCL NAL unit" condition; this
//! matters more than it did for H.264, whose IDR keying has
//! `idr_pic_id` — HEVC has no such field, and an all-IDR stream with
//! identical headers separates its pictures *only* by the SEI /
//! parameter sets between them; the corpus's 0044 all-intra stream
//! is exactly that shape). Segments with the flag clear are
//! continuation segments of the pending picture; one arriving with
//! no picture pending is counted damage. Delimiter NALs (AUD, end of
//! sequence, end of bitstream) close the pending picture, and so
//! does end-of-packet under length-prefixed framing (ISO/IEC
//! 14496-15: one HEVC sample is one access unit). Non-VCL units
//! attach to the picture that is pending when they arrive (the
//! 5.F2 attachment shape); in-band VPS/SPS/PPS activate before the
//! slices that follow and are re-offered to the session immediately.
//! An identical re-activation (same id, same raw bytes) is a no-op —
//! streams that repeat their parameter sets before every picture do
//! not re-negotiate the session.
//!
//! # Damage model
//!
//! A NAL that fails parsing (truncated, forbidden bit, emulation
//! violation, missing SPS/PPS context) is **skipped and counted**:
//! the call reports [`DecodeOutcome::Recovered { CorruptInput }`]
//! with the skipped count, the decoder stays usable, and the
//! remaining units still feed the session. No panic on any byte
//! string (the fuzz-lite survival suite pins this).
//!
//! # Surface format honesty
//!
//! Hardware output is an opaque surface
//! ([`FrameStorage::OpaqueHandle`]); `PixelFormat` has no opaque
//! variant, so the frame's format is the **SPS-derived presentation
//! view** — 4:2:0/monochrome 8-bit → [`PixelFormat::Nv12`] (the
//! GPU-native form OS decoders emit for those streams, with neutral
//! chroma for monochrome — the same convention the H.264 path and
//! the corpus record), 4:2:2 8-bit → [`PixelFormat::Yuyv422`].
//! Samplings with no vocabulary (4:4:4, over-8-bit — Main 10
//! included, a real HEVC population) are refused with typed
//! `hevc.hw.surface_format` — an honest gap for the media-types
//! extension card, not a wrong label `[PROPOSAL]`.

use codec_api::prelude::*;
use media_types::error::{DecodeCause, ErrorContext, FormatInfo};
use media_types::frame::{ColorDescription, Frame, FrameStorage, VideoFrame};
use media_types::{EngineError, ErrorKind, GenerationId, Packet, PipelineStage, Stamp};
use std::collections::HashMap;
use std::sync::Arc;

use super::hw::{
    HevcFraming, HevcHwAccessUnit, HevcHwBridge, HevcHwError, HevcHwOpenError, HevcHwPath,
    HevcHwPps, HevcHwSps, HevcHwStreamInfo, HevcHwUnit, HevcHwVcl, HevcHwVps,
};
use super::parser::nal::{
    AnnexBNals, HevcError, HvcCConfig, LengthPrefixedNals, NalHeader, NalUnitType,
};
use super::parser::pps::PicParameterSet;
use super::parser::slice::{SliceSegmentHeader, SliceSegmentPpsInfo, SliceSegmentSpsInfo};
use super::parser::sps::{ChromaFormat, SeqParameterSet};
use super::parser::vps::VideoParameterSet;

/// Cap on NAL units buffered per pending access unit (MP §12: an
/// adversarial stream must not grow the accumulator unboundedly; a
/// conformant picture is orders of magnitude below this).
const MAX_PENDING_UNITS: usize = 4096;

/// The conservative hw-path colorimetry (matches the H.264 sibling's
/// posture; SPS VUI color is a 7.B negotiation item).
const HW_COLOR: ColorDescription = ColorDescription::BT709_LIMITED;

/// Typed configure-time refusal (`hevc.*` code, hevc format info).
fn unsupported(code: &'static str) -> EngineError {
    EngineError::new(
        PipelineStage::Decode,
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("hevc".to_string()),
                sniffed: Vec::new(),
            }),
        },
        ErrorContext::new(code),
    )
}

/// The SPS-implied presentation pixel format (module docs: the
/// honesty statement). HEVC's Main 10 population lands in the
/// over-8-bit refusal — recorded, not papered over.
fn surface_format(sps: &SeqParameterSet) -> Option<media_types::PixelFormat> {
    use media_types::PixelFormat;
    if sps.bit_depth_luma_minus8 != 0 || sps.bit_depth_chroma_minus8 != 0 {
        return None;
    }
    match sps.chroma_format {
        ChromaFormat::Monochrome | ChromaFormat::Yuv420 => Some(PixelFormat::Nv12),
        ChromaFormat::Yuv422 => Some(PixelFormat::Yuyv422),
        ChromaFormat::Yuv444 => None,
    }
}

/// The per-picture comparison key of §7.4.2.4.4 (the slice-segment
/// header fields that differ between the first VCL units of two
/// *different* pictures; equal keys mean same picture — a
/// continuation or a repeated first segment).
#[derive(Debug, Clone, PartialEq, Eq)]
struct PictureKey {
    slice_pic_parameter_set_id: u8,
    slice_pic_order_cnt_lsb: Option<u32>,
    unit_type: NalUnitType,
    layer_id: u8,
    temporal_id_plus1: u8,
    pic_output_flag: Option<bool>,
    no_output_of_prior_pics: Option<bool>,
}

impl PictureKey {
    fn of(slice: &SliceSegmentHeader) -> PictureKey {
        PictureKey {
            slice_pic_parameter_set_id: slice.slice_pic_parameter_set_id,
            slice_pic_order_cnt_lsb: slice.slice_pic_order_cnt_lsb,
            unit_type: slice.unit_type,
            layer_id: slice.layer_id,
            temporal_id_plus1: slice.temporal_id_plus1,
            pic_output_flag: slice.pic_output_flag,
            no_output_of_prior_pics: slice.no_output_of_prior_pics,
        }
    }
}

/// The HEVC decoder: parser → access-unit assembler → hardware
/// session (the [`super::hw`] seam). Construct with
/// [`HevcDecoder::with_bridges`] once Phase-7 adapters exist; the
/// bare [`HevcDecoder::new`] answers every hardware attempt with the
/// typed `hevc.hw.no_bridge` error (truthful today: zero adapters
/// are installed in-tree).
pub struct HevcDecoder {
    /// Installed hardware bridges (the 7.B plug point).
    bridges: Vec<Arc<dyn HevcHwBridge>>,
    /// Configured stream generation (`None` until `configure`).
    generation: Option<GenerationId>,
    /// Packet framing in force.
    framing: HevcFraming,
    /// Active VPS store (id → parsed + raw).
    vps: HashMap<u8, HevcHwVps>,
    /// Active SPS store (id → parsed + raw).
    sps: HashMap<u8, HevcHwSps>,
    /// Active PPS store (id → parsed + raw).
    pps: HashMap<u8, HevcHwPps>,
    /// The open hardware session (opened at configure).
    hw: Option<HevcHwPath>,
    /// The presentation format derived from the active SPS.
    format: Option<media_types::PixelFormat>,
    /// Conformance-window size from the active SPS.
    display_size: Option<(u32, u32)>,
    /// Whether the hw session holds the current parameter-set view.
    hw_configured: bool,
    /// NAL units of the access unit being assembled (owned copies —
    /// an AU may span packets in Annex-B/TS streams).
    pending: Vec<HevcHwUnit>,
    /// The pending picture's §7.4.2.4.4 key (last first-segment VCL).
    pending_key: Option<PictureKey>,
    /// A non-VCL unit arrived since the pending picture's last VCL
    /// (the spec's previous-NAL-was-not-VCL boundary condition).
    non_vcl_since_vcl: bool,
    /// The pending picture's pts (its first VCL's packet).
    pending_pts: Stamp,
    /// pts of the packet currently being walked.
    current_pts: Stamp,
    /// Damage skipped in the current call (recovery accounting).
    skipped: u32,
}

impl std::fmt::Debug for HevcDecoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Trait objects cannot derive Debug; the diagnostics that
        // matter are the negotiated backend and the assembly state.
        let backend = self
            .hw
            .as_ref()
            .map(HevcHwPath::backend_id)
            .unwrap_or("none");
        f.debug_struct("HevcDecoder")
            .field("backend", &backend)
            .field("bridges", &self.bridges.len())
            .field("configured", &self.generation.is_some())
            .field("pending_units", &self.pending.len())
            .finish()
    }
}

impl Default for HevcDecoder {
    fn default() -> Self {
        HevcDecoder {
            bridges: Vec::new(),
            generation: None,
            framing: HevcFraming::AnnexB,
            vps: HashMap::new(),
            sps: HashMap::new(),
            pps: HashMap::new(),
            hw: None,
            format: None,
            display_size: None,
            hw_configured: false,
            pending: Vec::new(),
            pending_key: None,
            non_vcl_since_vcl: false,
            pending_pts: Stamp::Missing,
            current_pts: Stamp::Missing,
            skipped: 0,
        }
    }
}

impl HevcDecoder {
    /// A fresh decoder with **no hardware bridges installed** — every
    /// hardware attempt fails typed (`hevc.hw.no_bridge`) until
    /// Phase-7 adapters are wired in. This is the truthful v0 state.
    pub fn new() -> HevcDecoder {
        HevcDecoder::default()
    }

    /// A decoder with hardware bridges installed (the Phase-7 plug:
    /// one [`HevcHwBridge`] per adapter crate; the first bridge that
    /// claims the negotiated accelerator wins).
    pub fn with_bridges(bridges: Vec<Arc<dyn HevcHwBridge>>) -> HevcDecoder {
        HevcDecoder {
            bridges,
            ..HevcDecoder::default()
        }
    }

    /// The codec descriptors this decoder accepts (`hevc` per the
    /// MKV/MP4 mapping; `hvc1`/`hev1` per the MP4 fourcc style).
    fn accepts(descriptor: &str) -> bool {
        descriptor == "hevc"
            || descriptor == "hvc1"
            || descriptor == "hev1"
            || descriptor.starts_with("hvc1.")
            || descriptor.starts_with("hev1.")
    }

    /// Seed parameter sets + framing from out-of-band bytes.
    fn seed_extradata(&mut self, extra: &[u8]) -> Result<(), EngineError> {
        if HvcCConfig::looks_like_hvcc(extra) {
            let cfg =
                HvcCConfig::parse(extra).map_err(|e| parse_error(e, "hevc.extradata.hvcc"))?;
            self.framing = HevcFraming::LengthPrefixed {
                length_size: cfg.length_size,
            };
            for array in &cfg.arrays {
                // Only the parameter-set types are ours to activate;
                // an hvcC may also carry prefix-SEI arrays (type 39),
                // which the in-band stream re-delivers anyway.
                for nal in &array.nals {
                    match array.nal_unit_type {
                        32 => self.store_vps(nal)?,
                        33 => self.store_sps(nal)?,
                        34 => self.store_pps(nal)?,
                        _ => {}
                    }
                }
            }
        } else {
            // Annex-B CodecPrivate (TS-flavored containers): the
            // parameter sets are extracted; framing stays byte-stream.
            for nal in AnnexBNals::new(extra) {
                if let Ok((header, _)) = NalHeader::split(nal) {
                    match header.unit_type {
                        NalUnitType::VpsNut => self.store_vps(nal)?,
                        NalUnitType::SpsNut => self.store_sps(nal)?,
                        NalUnitType::PpsNut => self.store_pps(nal)?,
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    /// Parse + activate one VPS NAL (typed error; in-band callers
    /// may count it as damage instead). An identical re-activation
    /// (same id, same bytes) is a no-op — repeated parameter sets do
    /// not re-negotiate the session.
    fn store_vps(&mut self, nal: &[u8]) -> Result<(), EngineError> {
        let vps = VideoParameterSet::parse(nal).map_err(|e| parse_error(e, "hevc.vps"))?;
        if let Some(prev) = self.vps.get(&vps.video_parameter_set_id)
            && prev.nal.as_slice() == nal
        {
            return Ok(());
        }
        self.hw_configured = false; // a changed parameter set re-offers the stream
        self.vps.insert(
            vps.video_parameter_set_id,
            HevcHwVps {
                nal: nal.to_vec(),
                vps,
            },
        );
        Ok(())
    }

    /// Parse + activate one SPS NAL.
    fn store_sps(&mut self, nal: &[u8]) -> Result<(), EngineError> {
        let sps = SeqParameterSet::parse(nal).map_err(|e| parse_error(e, "hevc.sps"))?;
        if let Some(prev) = self.sps.get(&sps.seq_parameter_set_id)
            && prev.nal.as_slice() == nal
        {
            return Ok(());
        }
        self.hw_configured = false;
        self.sps.insert(
            sps.seq_parameter_set_id,
            HevcHwSps {
                nal: nal.to_vec(),
                sps,
            },
        );
        Ok(())
    }

    /// Parse + activate one PPS NAL.
    fn store_pps(&mut self, nal: &[u8]) -> Result<(), EngineError> {
        let pps = PicParameterSet::parse(nal).map_err(|e| parse_error(e, "hevc.pps"))?;
        if let Some(prev) = self.pps.get(&pps.pic_parameter_set_id)
            && prev.nal.as_slice() == nal
        {
            return Ok(());
        }
        self.hw_configured = false;
        self.pps.insert(
            pps.pic_parameter_set_id,
            HevcHwPps {
                nal: nal.to_vec(),
                pps,
            },
        );
        Ok(())
    }

    /// The "active" SPS for surface negotiation: the one referenced
    /// by the lowest-id known PPS (single-parameter-set streams —
    /// the whole corpus and virtually every real file — have exactly
    /// one; multi-PS streams re-sync on their in-band activations).
    fn active_sps(&self) -> Option<&HevcHwSps> {
        let pps = self
            .pps
            .values()
            .min_by_key(|p| p.pps.pic_parameter_set_id)?;
        self.sps.get(&pps.pps.seq_parameter_set_id)
    }

    /// (Re-)offer the current parameter-set view to the hw session.
    fn sync_hw_config(&mut self) -> Result<(), EngineError> {
        if self.hw_configured || self.sps.is_empty() || self.pps.is_empty() {
            return Ok(());
        }
        // Snapshot the active SPS facts before mutating self (the
        // borrow of `active_sps` must end before the assignments).
        let active = self.active_sps().map(|a| {
            a.sps
                .cropped_size()
                .map(|size| (surface_format(&a.sps), size))
        });
        match active {
            Some(Ok((Some(format), size))) => {
                self.format = Some(format);
                self.display_size = Some(size);
            }
            Some(Ok((None, _))) => return Err(unsupported("hevc.hw.surface_format")),
            Some(Err(e)) => return Err(parse_error(e, "hevc.sps.conf_window")),
            None => {}
        }
        let stream = HevcHwStreamInfo {
            framing: Some(self.framing),
            vps: self.vps.values().cloned().collect(),
            sps: self.sps.values().cloned().collect(),
            pps: self.pps.values().cloned().collect(),
            display_size: self.display_size,
        };
        if let Some(hw) = self.hw.as_mut() {
            hw.configure(&stream)
                .map_err(|e| hw_error(e, "hevc.hw.configure"))?;
        }
        self.hw_configured = true;
        Ok(())
    }

    /// Close the pending access unit, feed it to the session, push
    /// its frames; returns frames emitted.
    fn close_picture(&mut self, out: &mut dyn FrameSink) -> Result<u32, EngineError> {
        if !self.pending.iter().any(|u| u.vcl.is_some()) {
            // Non-VCL-only pending (a parameter-set-only packet): no
            // picture to decode; the units carry over to the next.
            return Ok(0);
        }
        let units = std::mem::take(&mut self.pending);
        self.pending_key = None;
        self.non_vcl_since_vcl = false;
        let irap = units
            .iter()
            .any(|u| u.unit_type.is_irap() && u.vcl.is_some());
        let au = HevcHwAccessUnit {
            units,
            irap,
            pts: self.pending_pts,
            display_size: self.display_size,
        };
        let Some(hw) = self.hw.as_mut() else {
            // Unreachable post-configure (configure opens or fails);
            // guarded anyway — never a panic.
            return Err(unsupported("hevc.hw.required"));
        };
        let decoded = hw.decode(&au).map_err(|e| hw_error(e, "hevc.hw.decode"))?;
        let mut emitted = 0u32;
        for frame in decoded.frames {
            out.accept(Frame::Video(VideoFrame {
                format: self.format.unwrap_or(media_types::PixelFormat::Nv12),
                size: frame.size,
                data: FrameStorage::OpaqueHandle(frame.surface),
                pts: frame.pts,
                color: HW_COLOR,
                // HEVC pictures are frames — no field pictures exist
                // in this spec generation (see super::hw docs).
                interlaced: false,
                generation: self.generation.unwrap_or(GenerationId::FIRST),
            }))?;
            emitted += 1;
        }
        Ok(emitted)
    }

    /// Append a non-VCL unit to the pending access unit (bounded).
    fn push_opaque(&mut self, nal: &[u8], unit_type: NalUnitType) {
        if self.pending.len() < MAX_PENDING_UNITS {
            self.pending.push(HevcHwUnit {
                nal: nal.to_vec(),
                unit_type,
                vcl: None,
            });
            self.non_vcl_since_vcl = true;
        } else {
            self.skipped += 1;
        }
    }

    /// Parse one VCL NAL against every known PPS context; the parse
    /// succeeds under (at most) the context whose id the slice
    /// itself declares (§7.3.6.1 reads `slice_pic_parameter_set_id`
    /// — the round-trip check rejects wrong contexts' misparses).
    fn parse_vcl(&mut self, nal: &[u8]) -> Option<SliceSegmentHeader> {
        let mut candidates: Vec<(u8, SliceSegmentSpsInfo, SliceSegmentPpsInfo)> = Vec::new();
        for (pps_id, pps) in &self.pps {
            let Some(sps) = self.sps.get(&pps.pps.seq_parameter_set_id) else {
                continue;
            };
            // A context whose CTB-grid arithmetic fails cannot serve
            // this slice's address width — skip it, never guess.
            if let Ok(seam_sps) = SliceSegmentSpsInfo::from_sps(&sps.sps) {
                candidates.push((*pps_id, seam_sps, SliceSegmentPpsInfo::from_pps(&pps.pps)));
            }
        }
        // Deterministic order: try lowest PPS id first.
        candidates.sort_by_key(|(id, _, _)| *id);
        let mut best: Option<SliceSegmentHeader> = None;
        let mut ambiguity = false;
        for (pps_id, sps_info, pps_info) in candidates {
            if let Ok(slice) = SliceSegmentHeader::parse(nal, &sps_info, &pps_info)
                && slice.slice_pic_parameter_set_id == pps_id
                && best.replace(slice).is_some()
            {
                // Two contexts both parse and round-trip-match: the
                // slice is ambiguous -- refuse rather than guess.
                ambiguity = true;
                break;
            }
        }
        if ambiguity { None } else { best }
    }
}

/// Map a parser failure to a typed engine error (configure-time
/// extradata + stream-geometry damage only — in-stream NAL damage is
/// the recovery path instead).
fn parse_error(e: HevcError, code: &'static str) -> EngineError {
    EngineError::new(
        PipelineStage::Decode,
        ErrorKind::Decoder {
            cause: DecodeCause::Bitstream,
        },
        ErrorContext::new(code),
    )
    .with_source(e)
}

/// Map a hardware-session failure to a typed engine error.
fn hw_error(e: HevcHwError, code: &'static str) -> EngineError {
    let kind = match &e {
        HevcHwError::Configure { .. } => ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("hevc".to_string()),
                sniffed: Vec::new(),
            }),
        },
        HevcHwError::Decode { .. } => ErrorKind::Decoder {
            cause: DecodeCause::Bitstream,
        },
        HevcHwError::DeviceLost => ErrorKind::Decoder {
            cause: DecodeCause::HardwareLost,
        },
    };
    EngineError::new(PipelineStage::Decode, kind, ErrorContext::new(code)).with_source(e)
}

impl Decoder for HevcDecoder {
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
        if !Self::accepts(cfg.codec.as_str()) {
            let mut err = unsupported("hevc.codec_descriptor");
            err.kind = ErrorKind::Unsupported {
                format: Some(FormatInfo {
                    name: Some(cfg.codec.to_string()),
                    sniffed: Vec::new(),
                }),
            };
            return Err(err);
        }
        // Reset per-generation state (seek / source switch).
        self.vps.clear();
        self.sps.clear();
        self.pps.clear();
        self.pending.clear();
        self.pending_key = None;
        self.non_vcl_since_vcl = false;
        self.hw_configured = false;
        self.format = None;
        self.display_size = None;
        self.framing = HevcFraming::AnnexB;
        self.generation = Some(cfg.generation);

        if let Some(extra) = cfg.extra_data.as_ref() {
            self.seed_extradata(extra.as_slice())?;
        }
        // The hw-first consumption point: open a session off the
        // negotiated accelerator or fail typed (ADR-001 — there is
        // no software fallback to catch this).
        match HevcHwPath::open(cfg.accelerator.as_ref(), &self.bridges) {
            Ok(path) => self.hw = Some(path),
            Err(HevcHwOpenError::NoAccelerator) => return Err(unsupported("hevc.hw.required")),
            Err(HevcHwOpenError::NoBridge { .. }) => return Err(unsupported("hevc.hw.no_bridge")),
        }
        // Offer the seeded parameter sets (hvcC-configured streams
        // are session-ready here; in-band streams configure lazily
        // at their first parameter-set sighting).
        self.sync_hw_config()?;
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
                ErrorContext::new("hevc.not_configured"),
            ));
        };
        if let Some(outcome) = drop_stale_packet(&packet, generation) {
            return Ok(outcome);
        }
        self.current_pts = packet.pts;
        self.skipped = 0;
        let mut emitted = 0u32;

        let mut nals: Vec<&[u8]> = Vec::new();
        match self.framing {
            HevcFraming::AnnexB => {
                // The Annex-B splitter is total: it yields every NAL
                // body it can see, damage included (typed downstream).
                nals.extend(AnnexBNals::new(packet.data.as_slice()));
            }
            HevcFraming::LengthPrefixed { length_size } => {
                match LengthPrefixedNals::new(packet.data.as_slice(), length_size) {
                    Ok(iter) => {
                        for item in iter {
                            match item {
                                Ok(nal) => nals.push(nal),
                                Err(_) => {
                                    // Framing damage: the splitter has
                                    // terminated itself; count and keep
                                    // what did split.
                                    self.skipped += 1;
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => return Err(parse_error(e, "hevc.av.length_size")),
                }
            }
        }

        for nal in nals {
            let (header, _payload) = match NalHeader::split(nal) {
                Ok(split) => split,
                Err(_) => {
                    self.skipped += 1;
                    continue;
                }
            };
            match header.unit_type {
                NalUnitType::VpsNut | NalUnitType::SpsNut | NalUnitType::PpsNut => {
                    let stored = match header.unit_type {
                        NalUnitType::VpsNut => self.store_vps(nal),
                        NalUnitType::SpsNut => self.store_sps(nal),
                        _ => self.store_pps(nal),
                    };
                    if stored.is_err() {
                        self.skipped += 1;
                        continue;
                    }
                    self.sync_hw_config()?;
                    self.push_opaque(nal, header.unit_type);
                }
                NalUnitType::AudNut | NalUnitType::EosNut | NalUnitType::EobNut => {
                    // Delimiter: the pending picture is complete.
                    emitted += self.close_picture(out)?;
                    self.push_opaque(nal, header.unit_type);
                }
                t if t.is_vcl() => match self.parse_vcl(nal) {
                    Some(slice) => {
                        if slice.first_slice_segment_in_pic {
                            // A first segment: new picture when any
                            // §7.4.2.4.4 comparison field differs, or
                            // a non-VCL intervened since the pending
                            // picture's last VCL.
                            let boundary = match &self.pending_key {
                                None => true,
                                Some(prev) => {
                                    *prev != PictureKey::of(&slice) || self.non_vcl_since_vcl
                                }
                            };
                            if boundary {
                                emitted += self.close_picture(out)?;
                                // Non-VCL units already pending (SEI,
                                // parameter sets, AUD) attach ahead of
                                // this picture's first VCL — the 5.F2
                                // attachment shape.
                                self.pending_pts = self.current_pts;
                            }
                        } else if self.pending_key.is_none() {
                            // A continuation segment with no picture
                            // pending: damage, counted.
                            self.skipped += 1;
                            continue;
                        }
                        let key = PictureKey::of(&slice);
                        self.pending_key = Some(key);
                        self.non_vcl_since_vcl = false;
                        if self.pending.len() < MAX_PENDING_UNITS {
                            self.pending.push(HevcHwUnit {
                                nal: nal.to_vec(),
                                unit_type: header.unit_type,
                                vcl: Some(HevcHwVcl { slice }),
                            });
                        } else {
                            self.skipped += 1;
                        }
                    }
                    // Unparsable segment (damage, missing context,
                    // ambiguity across PPS contexts): counted skip.
                    None => self.skipped += 1,
                },
                _ => {
                    // SEI, filler, reserved, unspecified: opaque
                    // bytes to the session (adapter's choice).
                    self.push_opaque(nal, header.unit_type);
                }
            }
        }

        // Length-prefixed framing: one sample is one access unit
        // (ISO/IEC 14496-15 §5.3.4.2) — close at end of packet.
        if matches!(self.framing, HevcFraming::LengthPrefixed { .. }) {
            emitted += self.close_picture(out)?;
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
                ErrorContext::new("hevc.not_configured"),
            ));
        }
        let mut emitted = self.close_picture(out)?;
        if let Some(hw) = self.hw.as_mut() {
            let drained = hw.flush().map_err(|e| hw_error(e, "hevc.hw.decode"))?;
            for frame in drained.frames {
                out.accept(Frame::Video(VideoFrame {
                    format: self.format.unwrap_or(media_types::PixelFormat::Nv12),
                    size: frame.size,
                    data: FrameStorage::OpaqueHandle(frame.surface),
                    pts: frame.pts,
                    color: HW_COLOR,
                    interlaced: false,
                    generation: self.generation.unwrap_or(GenerationId::FIRST),
                }))?;
                emitted += 1;
            }
        }
        Ok(if emitted > 0 {
            DecodeOutcome::Emitted { frames: emitted }
        } else {
            DecodeOutcome::Drained
        })
    }
}
