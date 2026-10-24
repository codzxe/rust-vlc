//! The HEVC hardware decode-path seam (card 5.G2 — the mirror of the
//! 5.F2 H.264 seams; ADR-001 §1: "H.264 and HEVC are hardware-first …
//! with truthful active-decoder reporting").
//!
//! This module is **the consumption point** where the LANDED 5.G1
//! parser meets a hardware adapter: it defines what a Phase-7 per-OS
//! adapter (7.B1 VA-API, 7.B2 VideoToolbox, 7.B3 Media Foundation —
//! *not built here*) receives, and owns the plumbing that turns a
//! negotiated [`HwAccelerator`](codec_api::HwAccelerator) into a fed
//! session. **No software decode lives here or anywhere under
//! `hevc`** — ADR-001's honesty boundary: a machine without an OS
//! decoder gets a typed `unsupported` error from [`super::decode`],
//! never a fake success, never a silent skip.
//!
//! # The two contracts (identical shape to 5.F2's)
//!
//! - [`HevcHwDecoder`] — the *session* an adapter opens: it is fed
//!   parsed parameter sets ([`HevcHwStreamInfo`]: VPS **and** SPS/PPS
//!   — HEVC's outermost parameter set is part of the offer) and
//!   access units ([`HevcHwAccessUnit`]: every NAL of one picture,
//!   raw bytes plus the parser's slice-segment facts) and hands back
//!   decoded surfaces ([`HevcHwFrame`]). This is the surface 7.B
//!   adapters implement; it is stable for them to build against now.
//! - [`HevcHwBridge`] — the *plug*: the codec-api
//!   [`HwAccelerator`](codec_api::HwAccelerator) handle is opaque by
//!   design (its `as_any` downcast seam belongs to "the decoder
//!   wrapper that knows its concrete backend" — `codec-api/src/hw.rs`).
//!   Each adapter crate ships one small type implementing
//!   [`HevcHwBridge`] that downcasts the negotiated accelerator to
//!   **its own** concrete type and mints the session.
//!   [`HevcHwPath::open`] walks the installed bridge list; an
//!   accelerator no bridge claims is a typed configure error
//!   (`hevc.hw.no_bridge`), still never a silent fallback.
//!
//! # Feed shape (what the adapter gets, and why)
//!
//! Real OS decoders (VA-API `begin_picture`/`render_picture` with
//! `VAConfigAttrib` PTL, VideoToolbox `CMVideoFormatDescription`
//! built from the parameter sets, MF `MF_MT_MPEG_SEQUENCE_HEADER`)
//! consume **raw NAL bytes plus the parsed geometry** — so each feed
//! carries both: the complete NAL (two-byte header included, framing
//! prefixes stripped, emulation prevention **kept** — the form every
//! OS API takes) next to the parser's structures. VCL units
//! additionally carry the parsed [`SliceSegmentHeader`] (which itself
//! retains the NAL-header projection: unit type, layer id, temporal
//! id — HEVC folds what H.264 split into `SliceNalInfo` into the
//! header struct), so an adapter can order pictures without
//! re-parsing. Access units arrive one picture per call, in decode
//! order, boundary-detected per §7.4.2.4.4 by [`super::decode`].
//!
//! One deliberate difference from the 5.F2 frame type: HEVC has no
//! field pictures (`field_pic_flag` does not exist — interlaced
//! sources are coded as frame pictures with VUI signalling), so
//! [`HevcHwFrame`] carries no `interlaced` flag and every emitted
//! picture is a frame.

use std::fmt;
use std::sync::Arc;

use codec_api::HwAccelerator;
use media_types::Stamp;

use super::parser::nal::NalUnitType;
use super::parser::pps::PicParameterSet;
use super::parser::slice::SliceSegmentHeader;
use super::parser::sps::SeqParameterSet;
use super::parser::vps::VideoParameterSet;

/// Byte framing the packets of a stream use (tells the adapter which
/// wire form it must accept if it re-serializes; feeds themselves
/// are always normalized complete NALs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HevcFraming {
    /// Annex-B start-code byte stream (`.265` ES, MPEG-TS).
    AnnexB,
    /// Length-prefixed samples (MP4 `hvc1`/`hev1`, Matroska
    /// `V_MPEGH/ISO/HEVC`), `length_size` = `lengthSizeMinusOne + 1`
    /// (1..=4).
    LengthPrefixed {
        /// NAL length prefix width in bytes.
        length_size: u8,
    },
}

/// One video parameter set offered to a hardware session: the
/// complete raw NAL (two-byte header included, framing stripped)
/// plus the parsed structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HevcHwVps {
    /// Complete raw VPS NAL bytes.
    pub nal: Vec<u8>,
    /// The parsed parameter set.
    pub vps: VideoParameterSet,
}

/// One sequence parameter set offered to a hardware session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HevcHwSps {
    /// Complete raw SPS NAL bytes.
    pub nal: Vec<u8>,
    /// The parsed parameter set.
    pub sps: SeqParameterSet,
}

/// One picture parameter set offered to a hardware session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HevcHwPps {
    /// Complete raw PPS NAL bytes.
    pub nal: Vec<u8>,
    /// The parsed parameter set.
    pub pps: PicParameterSet,
}

/// Configure-time stream description: the parameter sets a session
/// needs to negotiate surfaces, plus the packet framing.
#[derive(Debug, Clone, Default)]
pub struct HevcHwStreamInfo {
    /// The entry form the stream's packets use.
    pub framing: Option<HevcFraming>,
    /// Every distinct VPS seen so far (out-of-band `hvcC` and
    /// in-band), most recent activation last.
    pub vps: Vec<HevcHwVps>,
    /// Every distinct SPS seen so far.
    pub sps: Vec<HevcHwSps>,
    /// Every distinct PPS seen so far.
    pub pps: Vec<HevcHwPps>,
    /// The active SPS's conformance-window (cropped) dimensions,
    /// when an SPS has been seen — the surface negotiation size.
    pub display_size: Option<(u32, u32)>,
}

/// Parsed facts about one VCL unit of an access unit (§7.3.6.1
/// projection a session orders pictures by). The NAL-header
/// projection (unit type, layer id, temporal id) lives inside the
/// [`SliceSegmentHeader`] itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HevcHwVcl {
    /// The parsed slice-segment header (honest-subset prefix).
    pub slice: SliceSegmentHeader,
}

/// One NAL unit of an access unit, in stream order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HevcHwUnit {
    /// Complete raw NAL bytes (two-byte header included, framing
    /// prefix stripped, emulation prevention kept).
    pub nal: Vec<u8>,
    /// Classified NAL type (mirrors the header's 6-bit type field).
    pub unit_type: NalUnitType,
    /// Slice-segment facts — present on VCL units only.
    pub vcl: Option<HevcHwVcl>,
}

/// One access unit (one picture: its slice segments plus the non-VCL
/// units around them — parameter-set activations, SEI, AUD),
/// normalized for a hardware session.
#[derive(Debug, Clone)]
pub struct HevcHwAccessUnit {
    /// The unit list, stream order.
    pub units: Vec<HevcHwUnit>,
    /// True when the picture is an IRAP (IDR/CRA/BLA,
    /// `nal_unit_type` 16..=23) — a clean random-access point.
    pub irap: bool,
    /// Presentation timestamp of the packet that carried this
    /// picture's first VCL unit (`Stamp::Missing` when the container
    /// gave none).
    pub pts: Stamp,
    /// The active SPS's conformance dimensions at this picture.
    pub display_size: Option<(u32, u32)>,
}

impl HevcHwAccessUnit {
    /// The first VCL unit's slice facts, when present.
    #[must_use]
    pub fn first_vcl(&self) -> Option<&HevcHwVcl> {
        self.units.iter().find_map(|u| u.vcl.as_ref())
    }
}

/// One decoded picture handed back by a hardware session: an opaque
/// surface handle plus the geometry the engine needs to present it.
///
/// The handle is exactly media-types'
/// [`FrameStorage::OpaqueHandle`](media_types::frame::FrameStorage::OpaqueHandle)
/// payload — the adapter mints the value and owns its lifetime
/// contract (the renderer side of that contract is 7.B/R5 scope).
/// There is no interlaced flag: HEVC pictures are frames (§7.4.2.4 —
/// field pictures do not exist in this spec generation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HevcHwFrame {
    /// Opaque surface handle (adapter-minted).
    pub surface: u64,
    /// Display `(width, height)` of the decoded picture.
    pub size: (u32, u32),
    /// Presentation timestamp (echoed from the access unit's feed).
    pub pts: Stamp,
}

/// What one session call produced (hardware pipelines lag: frames
/// for earlier pictures can surface during a later call or at
/// `flush` — the vec is every frame that became ready).
#[derive(Debug, Clone, Default)]
pub struct HevcHwDecoded {
    /// Frames ready this call.
    pub frames: Vec<HevcHwFrame>,
}

/// Typed hardware-session failure. The codec layer maps these onto
/// engine errors; an adapter never panics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HevcHwError {
    /// The session rejected the stream description (named rule).
    Configure {
        /// Stable dotted rule id (e.g. `hevc.hw.surface_size`).
        code: &'static str,
    },
    /// The session rejected an access unit (named rule).
    Decode {
        /// Stable dotted rule id.
        code: &'static str,
    },
    /// The underlying device/context was lost (adapter is unusable;
    /// the engine must renegotiate or fail typed).
    DeviceLost,
}

impl fmt::Display for HevcHwError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HevcHwError::Configure { code } => write!(f, "hw configure failed: {code}"),
            HevcHwError::Decode { code } => write!(f, "hw decode failed: {code}"),
            HevcHwError::DeviceLost => f.write_str("hardware device lost"),
        }
    }
}

impl std::error::Error for HevcHwError {}

/// **The 7.B adapter contract**: one hardware decode session for
/// HEVC, fed by the parser's output.
///
/// Implementations are per-OS adapter internals (VA-API,
/// VideoToolbox, Media Foundation — Phase 7.B); the in-tree proof
/// is the fake session of the integration suite. The trait is
/// deliberately minimal and synchronous: the engine's decode worker
/// calls it, so every method is bounded work — a real adapter defers
/// to its driver and returns what is ready now.
pub trait HevcHwDecoder: Send {
    /// Truthful active-decoder label (ADR-001: the report says which
    /// decoder produced the output). Conventionally the same string
    /// as the accelerator's `backend_id`.
    fn backend_id(&self) -> &str;

    /// Install the stream description (parameter sets, framing,
    /// surface size). Called once per stream generation; a
    /// re-configure replaces all prior state.
    ///
    /// # Errors
    /// Typed [`HevcHwError::Configure`] when the session cannot
    /// serve this stream (profile/level unsupported by the device,
    /// surface negotiation failure, …).
    fn configure(&mut self, stream: &HevcHwStreamInfo) -> Result<(), HevcHwError>;

    /// Feed one access unit; return every frame that became ready.
    ///
    /// # Errors
    /// Typed [`HevcHwError::Decode`] /
    /// [`HevcHwError::DeviceLost`]. The session stays usable after a
    /// `Decode` error (the codec layer counts damage and continues).
    fn decode(&mut self, au: &HevcHwAccessUnit) -> Result<HevcHwDecoded, HevcHwError>;

    /// Signal end-of-input; drain every buffered picture.
    ///
    /// # Errors
    /// As [`HevcHwDecoder::decode`].
    fn flush(&mut self) -> Result<HevcHwDecoded, HevcHwError>;
}

/// **The 7.B plug**: teaches [`HevcHwPath::open`] how to turn a
/// negotiated [`HwAccelerator`] into an [`HevcHwDecoder`] session.
///
/// One bridge type per adapter crate, shipped beside the adapter: it
/// downcasts the opaque accelerator to **its own concrete type** (the
/// `codec-api/src/hw.rs` downcast seam — only the wrapper that knows
/// the backend may retrieve it) and mints the session. Returning
/// `None` means "this accelerator is not mine" — the open walk moves
/// to the next bridge, and a full miss is the typed
/// `hevc.hw.no_bridge` configure error (never a silent software
/// fallback: there is no software HEVC decode, ADR-001).
pub trait HevcHwBridge: Send + Sync {
    /// The accelerator backend label this bridge serves (matches
    /// `HwAccelerator::backend_id`; documentation and diagnostics).
    fn backend_id(&self) -> &str;

    /// Attempt to open a session off `accel`. `None` = not this
    /// bridge's backend.
    fn open(&self, accel: &Arc<dyn HwAccelerator>) -> Option<Box<dyn HevcHwDecoder>>;
}

/// Why [`HevcHwPath::open`] could not open a session (typed, part
/// of the honest-matrix contract).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HevcHwOpenError {
    /// No accelerator was negotiated at all (the config carried
    /// `None`): per ADR-001 there is no software path to fall back
    /// to — the error says exactly that.
    NoAccelerator,
    /// An accelerator was negotiated but no installed bridge claims
    /// it (carries the accelerator's backend id for the report).
    NoBridge {
        /// The negotiated accelerator's `backend_id`.
        backend: String,
    },
}

impl fmt::Display for HevcHwOpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HevcHwOpenError::NoAccelerator => f.write_str(
                "no hardware accelerator negotiated and HEVC has no software decode (ADR-001)",
            ),
            HevcHwOpenError::NoBridge { backend } => write!(
                f,
                "accelerator {backend} has no HEVC bridge installed (Phase-7 adapter missing)"
            ),
        }
    }
}

impl std::error::Error for HevcHwOpenError {}

/// The consumption point: an open hardware session fed by the
/// parser's output. Owns the [`HevcHwDecoder`] the bridge minted;
/// [`super::decode::HevcDecoder`] drives it.
pub struct HevcHwPath {
    session: Box<dyn HevcHwDecoder>,
}

impl fmt::Debug for HevcHwPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The session prints as its backend label (ADR-001's truthful
        // report), not a pointer — mirrors DecoderConfig's Debug.
        f.debug_struct("HevcHwPath")
            .field("backend", &self.session.backend_id())
            .finish()
    }
}

impl HevcHwPath {
    /// Walk `bridges` against the negotiated `accel` and open the
    /// first session a bridge claims.
    ///
    /// # Errors
    /// [`HevcHwOpenError::NoAccelerator`] when `accel` is `None`;
    /// [`HevcHwOpenError::NoBridge`] when every bridge declined.
    pub fn open(
        accel: Option<&Arc<dyn HwAccelerator>>,
        bridges: &[Arc<dyn HevcHwBridge>],
    ) -> Result<HevcHwPath, HevcHwOpenError> {
        let Some(accel) = accel else {
            return Err(HevcHwOpenError::NoAccelerator);
        };
        for bridge in bridges {
            if let Some(session) = bridge.open(accel) {
                return Ok(HevcHwPath { session });
            }
        }
        Err(HevcHwOpenError::NoBridge {
            backend: accel.backend_id().to_string(),
        })
    }

    /// The active session's backend label (ADR-001 truthful report).
    #[must_use]
    pub fn backend_id(&self) -> &str {
        self.session.backend_id()
    }

    /// Install the stream description (see
    /// [`HevcHwDecoder::configure`]).
    ///
    /// # Errors
    /// Propagates the session's typed [`HevcHwError`].
    pub fn configure(&mut self, stream: &HevcHwStreamInfo) -> Result<(), HevcHwError> {
        self.session.configure(stream)
    }

    /// Feed one access unit (see [`HevcHwDecoder::decode`]).
    ///
    /// # Errors
    /// Propagates the session's typed [`HevcHwError`].
    pub fn decode(&mut self, au: &HevcHwAccessUnit) -> Result<HevcHwDecoded, HevcHwError> {
        self.session.decode(au)
    }

    /// Drain the session (see [`HevcHwDecoder::flush`]).
    ///
    /// # Errors
    /// Propagates the session's typed [`HevcHwError`].
    pub fn flush(&mut self) -> Result<HevcHwDecoded, HevcHwError> {
        self.session.flush()
    }
}
