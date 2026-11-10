//! The H.264 codec-api decoder (card 5.F2): **the hardware
//! decode-path integration — no software decode, by design and by
//! ADR-001**.
//!
//! [`H264Decoder`] is the `Decoder` impl for H.264/AVC tracks: it
//! configures from the container's out-of-band bytes (`avcC` record
//! or Annex-B `CodecPrivate`) or from in-band parameter sets, routes
//! every packet through the LANDED 5.F1 parser, assembles access
//! units (§7.4.1.2.4 boundary detection), and feeds them to a
//! hardware session through the [`super::hw`] seam. There is no
//! software fallback anywhere in this file: per ADR-001 §1 a machine
//! without a usable hardware path gets a **typed
//! [`ErrorKind::Unsupported`]** configure error (`h264.hw.required`
//! / `h264.hw.no_bridge`), which the feature matrix reports
//! truthfully — never a silent skip, never a fabricated frame.
//!
//! # Access-unit assembly (§7.4.1.2.4)
//!
//! VCL units accumulate; a new picture is detected by the spec's
//! first-VCL rule set (frame_num, pps id, field flags, nal_ref_idc
//! zero-crossing, IDR flag/id, POC type-0 lsb/delta — the POC type-1
//! entry needs decoder-side frame history, and the LANDED SPS parser
//! does not retain `delta_pic_order_always_zero_flag` (sps.rs reads
//! and discards it), so the type-1 projection is `false` and a
//! set-flag stream surfaces as slice-header parse damage — recorded
//! as a parser follow-up, not papered over). Delimiter NALs (AUD,
//! end-of-sequence/stream) close the pending picture, and so does
//! end-of-packet under length-prefixed framing (ISO/IEC 14496-15:
//! one AVC sample is one access unit). Non-VCL units attach to the
//! picture that follows them; in-band SPS/PPS activate before the
//! slices that follow and are re-offered to the session immediately.
//!
//! # Damage model
//!
//! A NAL that fails parsing (truncated, forbidden bit, legacy data
//! partitions, missing SPS/PPS context) is **skipped and counted**:
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
//! chroma for monochrome exactly as ffmpeg's no-`--enable-gray`
//! default renders it — the corpus 0019 convention), 4:2:2 8-bit →
//! [`PixelFormat::Yuyv422`]. Samplings with no vocabulary (4:4:4,
//! over-8-bit) are refused with typed `h264.hw.surface_format` —
//! an honest gap for the media-types extension card, not a wrong
//! label `[PROPOSAL]`.

use codec_api::prelude::*;
use media_types::error::{DecodeCause, ErrorContext, FormatInfo};
use media_types::frame::{ColorDescription, Frame, FrameStorage, VideoFrame};
use media_types::{EngineError, ErrorKind, GenerationId, Packet, PipelineStage, Stamp};
use std::collections::HashMap;
use std::sync::Arc;

use super::hw::capability::{HwDecodeEvent, HwDecodePolicy, HwPolicyError};
use super::hw::{
    H264Framing, H264HwAccessUnit, H264HwBridge, H264HwError, H264HwOpenError, H264HwPath,
    H264HwPps, H264HwSps, H264HwStreamInfo, H264HwUnit, H264HwVcl,
};
use super::parser::nal::{
    AnnexBNals, AvcConfig, AvcNals, H264Error, NalHeader, NalUnitType, unescape_rbsp_into,
};
use super::parser::pps::PicParameterSet;
use super::parser::slice::{PicOrderSyntax, SliceHeader, SliceNalInfo, SlicePpsInfo, SliceSpsInfo};
use super::parser::sps::{ChromaFormat, SeqParameterSet};

/// Cap on NAL units buffered per pending access unit (MP §12: an
/// adversarial stream must not grow the accumulator unboundedly; a
/// conformant picture is orders of magnitude below this).
const MAX_PENDING_UNITS: usize = 4096;

/// The conservative hw-path colorimetry (matches the MJPEG sibling's
/// JFIF default posture; SPS VUI color is a 7.B negotiation item).
const HW_COLOR: ColorDescription = ColorDescription::BT709_LIMITED;

/// Typed configure-time refusal (`h264.*` code, h264 format info).
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

/// SPS → slice-parser projection. `delta_pic_order_always_zero_flag`
/// is `false` because the LANDED SPS parser discards that bit (see
/// module docs — recorded limitation, never a guess).
fn slice_sps_info(sps: &SeqParameterSet) -> SliceSpsInfo {
    SliceSpsInfo {
        log2_max_frame_num_minus4: sps.log2_max_frame_num_minus4,
        pic_order_cnt_type: sps.pic_order_cnt_type,
        log2_max_pic_order_cnt_lsb_minus4: sps.log2_max_pic_order_cnt_lsb_minus4.unwrap_or(0),
        delta_pic_order_always_zero_flag: false,
        frame_mbs_only_flag: sps.frame_mbs_only,
        separate_colour_plane_flag: sps.separate_colour_plane,
        chroma_format_idc: sps.chroma_format_idc.unwrap_or(1),
    }
}

/// PPS → slice-parser projection.
fn slice_pps_info(pps: &PicParameterSet) -> SlicePpsInfo {
    SlicePpsInfo {
        bottom_field_pic_order_in_frame_present_flag: pps.bottom_field_pic_order_in_frame_present,
        redundant_pic_cnt_present_flag: pps.redundant_pic_cnt_present,
        weighted_pred_flag: pps.weighted_pred,
        weighted_bipred_idc: pps.weighted_bipred_idc,
        entropy_coding_mode_flag: pps.entropy_coding_mode,
        deblocking_filter_control_present_flag: pps.deblocking_filter_control_present,
        num_slice_groups_minus1: pps.num_slice_groups_minus1,
        num_ref_idx_l0_default_active_minus1: u32::from(pps.num_ref_idx_l0_default_active_minus1),
        num_ref_idx_l1_default_active_minus1: u32::from(pps.num_ref_idx_l1_default_active_minus1),
    }
}

/// The SPS-implied presentation pixel format (module docs: the
/// honesty statement).
fn surface_format(sps: &SeqParameterSet) -> Option<media_types::PixelFormat> {
    use media_types::PixelFormat;
    if sps.bit_depth_luma_minus8 != 0 {
        return None;
    }
    match sps.chroma_format() {
        ChromaFormat::Monochrome | ChromaFormat::Yuv420 => Some(PixelFormat::Nv12),
        ChromaFormat::Yuv422 => Some(PixelFormat::Yuyv422),
        ChromaFormat::Yuv444 => None,
    }
}

/// The per-picture comparison key of §7.4.1.2.4 (the VCL header
/// fields that differ between two units of *different* pictures;
/// equal keys mean same picture — multi-slice).
#[derive(Debug, Clone, PartialEq, Eq)]
struct PictureKey {
    frame_num: u32,
    pps_id: u8,
    field_pic_flag: bool,
    bottom_field_flag: bool,
    nal_ref_idc_nonzero: bool,
    idr_pic_flag: bool,
    idr_pic_id: u32,
    poc_type0: Option<(u32, i32)>,
}

impl PictureKey {
    fn of(nal: SliceNalInfo, slice: &SliceHeader) -> PictureKey {
        let poc_type0 = match &slice.pic_order {
            PicOrderSyntax::Type0 { lsb, delta_bottom } => Some((*lsb, delta_bottom.unwrap_or(0))),
            _ => None,
        };
        PictureKey {
            frame_num: slice.frame_num,
            pps_id: slice.pic_parameter_set_id,
            field_pic_flag: slice.field_pic_flag,
            bottom_field_flag: slice.bottom_field_flag.unwrap_or(false),
            nal_ref_idc_nonzero: nal.nal_ref_idc != 0,
            idr_pic_flag: nal.idr_pic_flag,
            idr_pic_id: slice.idr_pic_id.unwrap_or(0),
            poc_type0,
        }
    }
}

/// The H.264 decoder: parser → access-unit assembler → hardware
/// session (the [`super::hw`] seam). Construct with
/// [`H264Decoder::from_setting`] (the settings-driven wiring of
/// issue #41: `playback.hw-decode-policy`, shipped default OFF) or
/// [`H264Decoder::with_bridges`] (explicit bridges, e.g. the
/// integration suite's fake adapter); the bare [`H264Decoder::new`]
/// answers every hardware attempt with the typed `h264.hw.required`
/// error (no accelerator, no bridges — the truthful zero-adapter
/// state).
pub struct H264Decoder {
    /// Installed hardware bridges (the 7.B plug point).
    bridges: Vec<Arc<dyn H264HwBridge>>,
    /// The accelerator this decoder negotiated itself off the
    /// settings policy ([`H264Decoder::from_setting`]). The
    /// engine-provided [`DecoderConfig::accelerator`] takes
    /// precedence — this fills the absence only (the engine that
    /// negotiates itself owns its decision).
    negotiated_accel: Option<Arc<dyn codec_api::HwAccelerator>>,
    /// The truthful `hw-active` event trail of the last policy
    /// negotiation (empty for the `new`/`with_bridges`
    /// constructions — they performed none; ADR-001's
    /// active-decoder report surface).
    hw_events: Vec<HwDecodeEvent>,
    /// Configured stream generation (`None` until `configure`).
    generation: Option<GenerationId>,
    /// Packet framing in force.
    framing: H264Framing,
    /// Active SPS store (id → parsed + raw).
    sps: HashMap<u8, H264HwSps>,
    /// Active PPS store (id → parsed + raw).
    pps: HashMap<u8, H264HwPps>,
    /// The open hardware session (opened at configure).
    hw: Option<H264HwPath>,
    /// The presentation format derived from the active SPS.
    format: Option<media_types::PixelFormat>,
    /// Display size from the active SPS.
    display_size: Option<(u32, u32)>,
    /// Whether the hw session holds the current parameter-set view.
    hw_configured: bool,
    /// NAL units of the access unit being assembled (owned copies —
    /// an AU may span packets in Annex-B/TS streams).
    pending: Vec<H264HwUnit>,
    /// The pending picture's §7.4.1.2.4 key (last VCL seen).
    pending_key: Option<PictureKey>,
    /// The pending picture's pts (its first VCL's packet).
    pending_pts: Stamp,
    /// pts of the packet currently being walked.
    current_pts: Stamp,
    /// Reusable RBSP buffer (zero steady-state allocation).
    rbsp: Vec<u8>,
    /// Damage skipped in the current call (recovery accounting).
    skipped: u32,
}

impl std::fmt::Debug for H264Decoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Trait objects cannot derive Debug; the diagnostics that
        // matter are the negotiated backend and the assembly state.
        let backend = self
            .hw
            .as_ref()
            .map(H264HwPath::backend_id)
            .unwrap_or("none");
        f.debug_struct("H264Decoder")
            .field("backend", &backend)
            .field("bridges", &self.bridges.len())
            .field("configured", &self.generation.is_some())
            .field("pending_units", &self.pending.len())
            .finish()
    }
}

impl Default for H264Decoder {
    fn default() -> Self {
        H264Decoder {
            bridges: Vec::new(),
            negotiated_accel: None,
            hw_events: Vec::new(),
            generation: None,
            framing: H264Framing::AnnexB,
            sps: HashMap::new(),
            pps: HashMap::new(),
            hw: None,
            format: None,
            display_size: None,
            hw_configured: false,
            pending: Vec::new(),
            pending_key: None,
            pending_pts: Stamp::Missing,
            current_pts: Stamp::Missing,
            rbsp: Vec::new(),
            skipped: 0,
        }
    }
}

impl H264Decoder {
    /// A fresh decoder with **no hardware bridges installed** — every
    /// hardware attempt fails typed (`h264.hw.required`: no
    /// accelerator is carried either). This is the explicit-zero
    /// constructor; [`H264Decoder::from_setting`] is the settings-
    /// driven one.
    pub fn new() -> H264Decoder {
        H264Decoder::default()
    }

    /// A decoder with hardware bridges installed (the Phase-7 plug:
    /// one [`H264HwBridge`] per adapter crate; the first bridge that
    /// claims the negotiated accelerator wins).
    pub fn with_bridges(bridges: Vec<Arc<dyn H264HwBridge>>) -> H264Decoder {
        H264Decoder {
            bridges,
            ..H264Decoder::default()
        }
    }

    /// **The settings-driven wiring** (issue #41): build a decoder
    /// from the raw `playback.hw-decode-policy` setting string.
    ///
    /// The policy is parsed with
    /// [`HwDecodePolicy::from_setting`](super::hw::capability::HwDecodePolicy::from_setting)
    /// — a value outside the vocabulary is a typed
    /// [`HwPolicyError`], never a guess. Under it the current OS's
    /// in-tree adapter lane is negotiated
    /// ([`super::hw::negotiate_lane`]) and its bridges installed
    /// ([`super::hw::installed_bridges`]). The **shipped default
    /// `software-only` touches nothing** (no device scan, no library
    /// load): no accelerator is carried and [`Self::configure`]
    /// answers the typed `h264.hw.required` (ADR-001: no software
    /// H.264 fallback exists) — OFF means off.
    ///
    /// # Errors
    /// [`HwPolicyError`] when `setting` is not one of
    /// `software-only` / `prefer-hardware` / `hardware-only`.
    pub fn from_setting(setting: &str) -> Result<H264Decoder, HwPolicyError> {
        let policy = HwDecodePolicy::from_setting(setting)?;
        let negotiation = super::hw::negotiate_lane(policy);
        Ok(H264Decoder {
            bridges: super::hw::installed_bridges(),
            negotiated_accel: negotiation.accelerator.clone(),
            hw_events: negotiation.events,
            ..H264Decoder::default()
        })
    }

    /// The truthful `hw-active` event trail of the last policy
    /// negotiation ([`Self::from_setting`]'s probe → decision
    /// record, ADR-001's active-decoder report). Empty when the
    /// decoder was built without a policy negotiation.
    #[must_use = "the ADR-001 active-decoder report"]
    pub fn hw_events(&self) -> &[HwDecodeEvent] {
        &self.hw_events
    }

    /// The codec descriptors this decoder accepts (`h264` per the
    /// MKV mapping; `avc1`[.profile] per the MP4 fourcc style).
    fn accepts(descriptor: &str) -> bool {
        descriptor == "h264" || descriptor == "avc1" || descriptor.starts_with("avc1.")
    }

    /// Seed parameter sets + framing from out-of-band bytes.
    fn seed_extradata(&mut self, extra: &[u8]) -> Result<(), EngineError> {
        if AvcConfig::looks_like_avcc(extra) {
            let cfg = AvcConfig::parse(extra).map_err(|e| parse_error(e, "h264.extradata.avcc"))?;
            self.framing = H264Framing::Avc {
                length_size: cfg.length_size,
            };
            for nal in &cfg.sps {
                self.store_sps(nal)?;
            }
            for nal in &cfg.pps {
                self.store_pps(nal)?;
            }
        } else {
            // Annex-B CodecPrivate (TS-flavored containers): the
            // parameter sets are extracted; framing stays byte-stream.
            for nal in AnnexBNals::new(extra) {
                if let Ok((header, _)) = NalHeader::split(nal) {
                    match header.unit_type {
                        NalUnitType::Sps => self.store_sps(nal)?,
                        NalUnitType::Pps => self.store_pps(nal)?,
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    /// Parse + activate one SPS NAL (typed error; in-band callers
    /// may count it as damage instead).
    fn store_sps(&mut self, nal: &[u8]) -> Result<(), EngineError> {
        let sps = SeqParameterSet::parse(nal).map_err(|e| parse_error(e, "h264.sps"))?;
        self.hw_configured = false; // a changed parameter set re-offers the stream
        self.sps.insert(
            sps.seq_parameter_set_id,
            H264HwSps {
                nal: nal.to_vec(),
                sps,
            },
        );
        Ok(())
    }

    /// Parse + activate one PPS NAL.
    fn store_pps(&mut self, nal: &[u8]) -> Result<(), EngineError> {
        let pps = PicParameterSet::parse(nal).map_err(|e| parse_error(e, "h264.pps"))?;
        self.hw_configured = false;
        self.pps.insert(
            pps.pic_parameter_set_id,
            H264HwPps {
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
    fn active_sps(&self) -> Option<&H264HwSps> {
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
        let active = self
            .active_sps()
            .map(|a| (surface_format(&a.sps), a.sps.display_size()));
        match active {
            Some((Some(format), display)) => {
                self.format = Some(format);
                self.display_size = display;
            }
            Some((None, _)) => return Err(unsupported("h264.hw.surface_format")),
            None => {}
        }
        let stream = H264HwStreamInfo {
            framing: Some(self.framing),
            sps: self.sps.values().cloned().collect(),
            pps: self.pps.values().cloned().collect(),
            display_size: self.display_size,
        };
        if let Some(hw) = self.hw.as_mut() {
            hw.configure(&stream)
                .map_err(|e| hw_error(e, "h264.hw.configure"))?;
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
        let idr = units.iter().any(|u| u.unit_type == NalUnitType::SliceIdr);
        let au = H264HwAccessUnit {
            units,
            idr,
            pts: self.pending_pts,
            display_size: self.display_size,
        };
        let Some(hw) = self.hw.as_mut() else {
            // Unreachable post-configure (configure opens or fails);
            // guarded anyway — never a panic.
            return Err(unsupported("h264.hw.required"));
        };
        let decoded = hw.decode(&au).map_err(|e| hw_error(e, "h264.hw.decode"))?;
        let mut emitted = 0u32;
        for frame in decoded.frames {
            out.accept(Frame::Video(VideoFrame {
                format: self.format.unwrap_or(media_types::PixelFormat::Nv12),
                size: frame.size,
                data: FrameStorage::OpaqueHandle(frame.surface),
                pts: frame.pts,
                color: HW_COLOR,
                interlaced: frame.interlaced,
                generation: self.generation.unwrap_or(GenerationId::FIRST),
            }))?;
            emitted += 1;
        }
        Ok(emitted)
    }

    /// Append a non-VCL unit to the pending access unit (bounded).
    fn push_opaque(&mut self, nal: &[u8], unit_type: NalUnitType) {
        if self.pending.len() < MAX_PENDING_UNITS {
            self.pending.push(H264HwUnit {
                nal: nal.to_vec(),
                unit_type,
                vcl: None,
            });
        } else {
            self.skipped += 1;
        }
    }

    /// Parse one VCL NAL against every known PPS context; the parse
    /// succeeds under (at most) the context whose id the slice
    /// itself declares (§7.3.3 reads `pic_parameter_set_id` — the
    /// round-trip check rejects wrong contexts' misparses).
    fn parse_vcl(
        &mut self,
        payload: &[u8],
        header: NalHeader,
    ) -> Option<(SliceHeader, SliceNalInfo)> {
        let nal = SliceNalInfo {
            nal_ref_idc: header.nal_ref_idc,
            idr_pic_flag: header.unit_type == NalUnitType::SliceIdr,
        };
        unescape_rbsp_into(payload, &mut self.rbsp).ok()?;
        let mut candidates: Vec<(u8, SliceSpsInfo, SlicePpsInfo)> = Vec::new();
        for (pps_id, pps) in &self.pps {
            let Some(sps) = self.sps.get(&pps.pps.seq_parameter_set_id) else {
                continue;
            };
            candidates.push((*pps_id, slice_sps_info(&sps.sps), slice_pps_info(&pps.pps)));
        }
        // Deterministic order: try lowest PPS id first.
        candidates.sort_by_key(|(id, _, _)| *id);
        let mut best: Option<(SliceHeader, SliceNalInfo)> = None;
        let mut ambiguity = false;
        for (pps_id, sps_info, pps_info) in candidates {
            if let Ok(slice) = parse_slice_header_pub(&self.rbsp, nal, &sps_info, &pps_info)
                && slice.pic_parameter_set_id == pps_id
                && best.replace((slice, nal)).is_some()
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

/// Thin indirection so the trait import stays out of the hot name
/// space (the parser's free function is the same call).
fn parse_slice_header_pub(
    rbsp: &[u8],
    nal: SliceNalInfo,
    sps: &SliceSpsInfo,
    pps: &SlicePpsInfo,
) -> Result<SliceHeader, super::parser::slice::SliceHeaderError> {
    super::parser::slice::parse_slice_header(rbsp, nal, sps, pps)
}

/// Map a parser failure to a typed engine error (configure-time
/// extradata only — in-stream damage is the recovery path instead).
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

/// Map a hardware-session failure to a typed engine error.
fn hw_error(e: H264HwError, code: &'static str) -> EngineError {
    let kind = match &e {
        H264HwError::Configure { .. } => ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("h264".to_string()),
                sniffed: Vec::new(),
            }),
        },
        H264HwError::Decode { .. } => ErrorKind::Decoder {
            cause: DecodeCause::Bitstream,
        },
        H264HwError::DeviceLost => ErrorKind::Decoder {
            cause: DecodeCause::HardwareLost,
        },
    };
    EngineError::new(PipelineStage::Decode, kind, ErrorContext::new(code)).with_source(e)
}

impl Decoder for H264Decoder {
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
        if !Self::accepts(cfg.codec.as_str()) {
            let mut err = unsupported("h264.codec_descriptor");
            err.kind = ErrorKind::Unsupported {
                format: Some(FormatInfo {
                    name: Some(cfg.codec.to_string()),
                    sniffed: Vec::new(),
                }),
            };
            return Err(err);
        }
        // Reset per-generation state (seek / source switch).
        self.sps.clear();
        self.pps.clear();
        self.pending.clear();
        self.pending_key = None;
        self.hw_configured = false;
        self.format = None;
        self.display_size = None;
        self.framing = H264Framing::AnnexB;
        self.generation = Some(cfg.generation);

        if let Some(extra) = cfg.extra_data.as_ref() {
            self.seed_extradata(extra.as_slice())?;
        }
        // The hw-first consumption point: open a session off the
        // negotiated accelerator or fail typed (ADR-001 — there is
        // no software fallback to catch this). The engine-provided
        // config accelerator is authoritative; the decoder's own
        // policy-negotiated one (from_setting) fills the absence.
        let accel = cfg.accelerator.as_ref().or(self.negotiated_accel.as_ref());
        match H264HwPath::open(accel, &self.bridges) {
            Ok(path) => self.hw = Some(path),
            Err(H264HwOpenError::NoAccelerator) => return Err(unsupported("h264.hw.required")),
            Err(H264HwOpenError::NoBridge { .. }) => return Err(unsupported("h264.hw.no_bridge")),
        }
        // Offer the seeded parameter sets (avcC-configured streams
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
                ErrorContext::new("h264.not_configured"),
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
            H264Framing::AnnexB => {
                // The Annex-B splitter is total: it yields every NAL
                // body it can see, damage included (typed downstream).
                nals.extend(AnnexBNals::new(packet.data.as_slice()));
            }
            H264Framing::Avc { length_size } => {
                match AvcNals::new(packet.data.as_slice(), length_size) {
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
                    Err(e) => return Err(parse_error(e, "h264.avc.length_size")),
                }
            }
        }

        for nal in nals {
            let (header, payload) = match NalHeader::split(nal) {
                Ok(split) => split,
                Err(_) => {
                    self.skipped += 1;
                    continue;
                }
            };
            match header.unit_type {
                NalUnitType::Sps | NalUnitType::Pps => {
                    if header.unit_type == NalUnitType::Sps {
                        if self.store_sps(nal).is_err() {
                            self.skipped += 1;
                            continue;
                        }
                    } else if self.store_pps(nal).is_err() {
                        self.skipped += 1;
                        continue;
                    }
                    self.sync_hw_config()?;
                    self.push_opaque(nal, header.unit_type);
                }
                NalUnitType::Aud | NalUnitType::EndOfStream | NalUnitType::EndOfSequence => {
                    // Delimiter: the pending picture is complete.
                    emitted += self.close_picture(out)?;
                    self.push_opaque(nal, header.unit_type);
                }
                NalUnitType::Slice | NalUnitType::SliceIdr => {
                    match self.parse_vcl(payload, header) {
                        Some((slice, nal_info)) => {
                            let key = PictureKey::of(nal_info, &slice);
                            let boundary = match &self.pending_key {
                                None => true,
                                Some(prev) => *prev != key,
                            };
                            if boundary {
                                emitted += self.close_picture(out)?;
                                // Non-VCL units already pending (SEI,
                                // parameter sets, AUD) precede this
                                // picture's first VCL — they sit in
                                // `pending` ahead of it, which is the
                                // spec's attachment rule.
                                self.pending_pts = self.current_pts;
                            }
                            self.pending_key = Some(key);
                            if self.pending.len() < MAX_PENDING_UNITS {
                                self.pending.push(H264HwUnit {
                                    nal: nal.to_vec(),
                                    unit_type: header.unit_type,
                                    vcl: Some(H264HwVcl {
                                        slice,
                                        nal: nal_info,
                                    }),
                                });
                            } else {
                                self.skipped += 1;
                            }
                        }
                        // Unparsable slice (damage, missing context,
                        // legacy data partitions): counted skip.
                        None => self.skipped += 1,
                    }
                }
                _ => {
                    // SEI, filler, prefix, reserved, unspecified:
                    // opaque bytes to the session (adapter's choice).
                    self.push_opaque(nal, header.unit_type);
                }
            }
        }

        // Length-prefixed framing: one sample is one access unit
        // (ISO/IEC 14496-15 §5.3.4.2) — close at end of packet.
        if matches!(self.framing, H264Framing::Avc { .. }) {
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
                ErrorContext::new("h264.not_configured"),
            ));
        }
        let mut emitted = self.close_picture(out)?;
        if let Some(hw) = self.hw.as_mut() {
            let drained = hw.flush().map_err(|e| hw_error(e, "h264.hw.decode"))?;
            for frame in drained.frames {
                out.accept(Frame::Video(VideoFrame {
                    format: self.format.unwrap_or(media_types::PixelFormat::Nv12),
                    size: frame.size,
                    data: FrameStorage::OpaqueHandle(frame.surface),
                    pts: frame.pts,
                    color: HW_COLOR,
                    interlaced: frame.interlaced,
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
