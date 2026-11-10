//! The H.264 hardware decode-path seam (card 5.F2) **plus the
//! ADR-001 per-OS adapter lanes** (cards 7.B1–7.B3, issue #41):
//! VideoToolbox (macOS), Media Foundation + DXVA (Windows), VA-API
//! (Linux).
//!
//! This module is **the consumption point** where the LANDED parser
//! (5.F1) meets a hardware adapter: it defines what a per-OS adapter
//! receives, and owns the plumbing that turns a negotiated
//! [`HwAccelerator`](codec_api::HwAccelerator) into a fed session.
//! **No software decode lives here or anywhere under `h264`** —
//! ADR-001's honesty boundary: a machine without an OS decoder gets
//! a typed `unsupported` error from [`super::decode`], never a fake
//! success, never a silent skip.
//!
//! # The contracts
//!
//! - [`H264HwDecoder`] — the *session* an adapter opens: it is fed
//!   parsed parameter sets ([`H264HwStreamInfo`]) and access units
//!   ([`H264HwAccessUnit`]: every NAL of one picture, raw bytes plus
//!   the parser's slice-header facts) and hands back decoded
//!   surfaces ([`H264HwFrame`]). This is the surface the 7.B lanes
//!   implement.
//! - [`H264HwBridge`] — the *plug*: the codec-api
//!   [`HwAccelerator`](codec_api::HwAccelerator) handle is opaque by
//!   design (its `as_any` downcast seam belongs to "the decoder
//!   wrapper that knows its concrete backend" — `codec-api/src/hw.rs`).
//!   A bridge is how the wrapper knows: each lane ships one small
//!   type implementing `H264HwBridge` that downcasts the negotiated
//!   accelerator to **its own** concrete type and mints the session.
//!   [`H264HwPath::open`] walks the installed bridge list; an
//!   accelerator no bridge claims is a typed configure error
//!   (`h264.hw.no_bridge`), still never a silent fallback.
//! - [`capability`] — the typed negotiation layer (cards 7.B1–7.B3):
//!   probe → [`HwAvailability`] → policy → [`Negotiation`] with the
//!   truthful `hw-active` event trail. Never fakes hardware.
//! - [`frame_pool`] — the bounded surface-slot model every lane's
//!   session allocates from (MP §12: bounded by construction).
//!
//! # The per-OS lanes (7.B1–7.B3) and their honest status
//!
//! Each lane is `cfg`-gated to its OS and compiled for its target
//! (evidence labels per the card):
//!
//! - **`vaapi`** (Linux, 7.B1): device-node discovery + runtime
//!   `dlopen` of `libva` + typed probe. Runtime-tested here in its
//!   **failure paths** (this sandbox has no GPU: the probe reports
//!   the typed `Unavailable` reason and the software-fallback chain
//!   engages). The driver-mediated decode path is
//!   compile-verified only. **Activation `[PROPOSAL]`** pending
//!   ADR-013 ratification (Lead action).
//! - **`videotoolbox`** (macOS, 7.B2): hand-rolled
//!   `VTDecompressionSession*` FFI, compile-verified via
//!   `cargo check --target aarch64-apple-darwin`; runtime hardware
//!   behavior NOT exercised — `[VERIFY]`.
//! - **`mediafoundation`** (Windows, 7.B3): hand-rolled
//!   `IMFTransform`/DXVA COM FFI, compile-verified via
//!   `cargo check --target x86_64-pc-windows-msvc`; runtime hardware
//!   behavior NOT exercised — `[VERIFY]`.
//!
//! # Feed shape (what the adapter gets, and why)
//!
//! Real OS decoders (VA-API `begin_picture`/`render_picture`,
//! VideoToolbox `CMVideoFormatDescription` from SPS/PPS, MF
//! `MF_MT_MPEG_SEQUENCE_HEADER`) consume **raw NAL bytes plus the
//! parsed geometry** — so each feed carries both: the complete NAL
//! (header byte included, framing prefixes stripped, emulation
//! prevention **kept** — that is the form every OS API takes) next
//! to the parser's structures. Slices additionally carry the parsed
//! [`SliceHeader`] and the SPS/PPS projections it was read against,
//! so an adapter can order pictures without re-parsing. Access units
//! arrive one picture per call, in decode order, boundary-detected
//! per §7.4.1.2.4 by [`super::decode`].

pub mod capability;
pub mod frame_pool;
#[cfg(target_os = "linux")]
pub mod vaapi;

use std::fmt;
use std::sync::Arc;

use codec_api::HwAccelerator;
use media_types::Stamp;

use super::parser::nal::NalUnitType;
use super::parser::pps::PicParameterSet;
use super::parser::slice::{SliceHeader, SliceNalInfo};
use super::parser::sps::SeqParameterSet;

/// Byte framing the packets of a stream use (tells the adapter which
/// wire form it must accept if it re-serializes; feeds themselves
/// are always normalized complete NALs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum H264Framing {
    /// Annex-B start-code byte stream (`.h264` ES, MPEG-TS).
    AnnexB,
    /// AVC length-prefixed samples (MP4/MKV), `length_size` =
    /// `lengthSizeMinusOne + 1` (1..=4).
    Avc {
        /// NAL length prefix width in bytes.
        length_size: u8,
    },
}

/// One sequence parameter set offered to a hardware session: the
/// complete raw NAL (header byte included, framing stripped) plus
/// the parsed structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct H264HwSps {
    /// Complete raw SPS NAL bytes.
    pub nal: Vec<u8>,
    /// The parsed parameter set.
    pub sps: SeqParameterSet,
}

/// One picture parameter set offered to a hardware session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct H264HwPps {
    /// Complete raw PPS NAL bytes.
    pub nal: Vec<u8>,
    /// The parsed parameter set.
    pub pps: PicParameterSet,
}

/// Configure-time stream description: the parameter sets a session
/// needs to negotiate surfaces, plus the packet framing.
#[derive(Debug, Clone, Default)]
pub struct H264HwStreamInfo {
    /// The entry form the stream's packets use.
    pub framing: Option<H264Framing>,
    /// Every distinct SPS seen so far (out-of-band `avcC` and
    /// in-band), most recent activation last.
    pub sps: Vec<H264HwSps>,
    /// Every distinct PPS seen so far.
    pub pps: Vec<H264HwPps>,
    /// The active SPS's display (cropped) dimensions, when an SPS
    /// has been seen — the surface negotiation size.
    pub display_size: Option<(u32, u32)>,
}

/// Parsed facts about one VCL unit of an access unit (§7.3.3
/// projection a session orders pictures by).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct H264HwVcl {
    /// The parsed slice header.
    pub slice: SliceHeader,
    /// The NAL-header projection it was read against.
    pub nal: SliceNalInfo,
}

/// One NAL unit of an access unit, in stream order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct H264HwUnit {
    /// Complete raw NAL bytes (header byte included, framing prefix
    /// stripped, emulation prevention kept).
    pub nal: Vec<u8>,
    /// Classified NAL type (mirrors the header byte's low 5 bits).
    pub unit_type: NalUnitType,
    /// Slice facts — present on VCL units only.
    pub vcl: Option<H264HwVcl>,
}

/// One access unit (one picture: its slices plus the non-VCL units
/// that precede them — parameter-set activations, SEI, AUD),
/// normalized for a hardware session.
#[derive(Debug, Clone)]
pub struct H264HwAccessUnit {
    /// The unit list, stream order.
    pub units: Vec<H264HwUnit>,
    /// True when the picture is an IDR (`nal_unit_type == 5`).
    pub idr: bool,
    /// Presentation timestamp of the packet that carried this
    /// picture's first VCL unit (`Stamp::Missing` when the container
    /// gave none).
    pub pts: Stamp,
    /// The active SPS's display dimensions at this picture.
    pub display_size: Option<(u32, u32)>,
}

impl H264HwAccessUnit {
    /// The first VCL unit's slice facts, when present.
    #[must_use]
    pub fn first_vcl(&self) -> Option<&H264HwVcl> {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct H264HwFrame {
    /// Opaque surface handle (adapter-minted).
    pub surface: u64,
    /// Display `(width, height)` of the decoded picture.
    pub size: (u32, u32),
    /// Interlaced picture (field_pic_flag was set on any slice).
    pub interlaced: bool,
    /// Presentation timestamp (echoed from the access unit's feed).
    pub pts: Stamp,
}

/// What one session call produced (hardware pipelines lag: frames
/// for earlier pictures can surface during a later call or at
/// `flush` — the vec is every frame that became ready).
#[derive(Debug, Clone, Default)]
pub struct H264HwDecoded {
    /// Frames ready this call.
    pub frames: Vec<H264HwFrame>,
}

/// Typed hardware-session failure. The codec layer maps these onto
/// engine errors; an adapter never panics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum H264HwError {
    /// The session rejected the stream description (named rule).
    Configure {
        /// Stable dotted rule id (e.g. `h264.hw.surface_size`).
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

impl fmt::Display for H264HwError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            H264HwError::Configure { code } => write!(f, "hw configure failed: {code}"),
            H264HwError::Decode { code } => write!(f, "hw decode failed: {code}"),
            H264HwError::DeviceLost => f.write_str("hardware device lost"),
        }
    }
}

impl std::error::Error for H264HwError {}

/// **The 7.B adapter contract**: one hardware decode session for
/// H.264, fed by the parser's output.
///
/// Implementations are per-OS adapter internals (VA-API, VideoToolbox,
/// Media Foundation — Phase 7.B); the in-tree proof is the fake
/// session of the integration suite. The trait is deliberately
/// minimal and synchronous: the engine's decode worker calls it, so
/// every method is bounded work — a real adapter defers to its
/// driver and returns what is ready now.
pub trait H264HwDecoder: Send {
    /// Truthful active-decoder label (ADR-001: the report says which
    /// decoder produced the output). Conventionally the same string
    /// as the accelerator's `backend_id`.
    fn backend_id(&self) -> &str;

    /// Install the stream description (parameter sets, framing,
    /// surface size). Called once per stream generation; a
    /// re-configure replaces all prior state.
    ///
    /// # Errors
    /// Typed [`H264HwError::Configure`] when the session cannot
    /// serve this stream (profile unsupported by the device, surface
    /// negotiation failure, …).
    fn configure(&mut self, stream: &H264HwStreamInfo) -> Result<(), H264HwError>;

    /// Feed one access unit; return every frame that became ready.
    ///
    /// # Errors
    /// Typed [`H264HwError::Decode`] /
    /// [`H264HwError::DeviceLost`]. The session stays usable after a
    /// `Decode` error (the codec layer counts damage and continues).
    fn decode(&mut self, au: &H264HwAccessUnit) -> Result<H264HwDecoded, H264HwError>;

    /// Signal end-of-input; drain every buffered picture.
    ///
    /// # Errors
    /// As [`H264HwDecoder::decode`].
    fn flush(&mut self) -> Result<H264HwDecoded, H264HwError>;
}

/// **The 7.B plug**: teaches [`H264HwPath::open`] how to turn a
/// negotiated [`HwAccelerator`] into an [`H264HwDecoder`] session.
///
/// One bridge type per adapter crate, shipped beside the adapter: it
/// downcasts the opaque accelerator to **its own concrete type**
/// (the `codec-api/src/hw.rs` downcast seam — only the wrapper that
/// knows the backend may retrieve it) and mints the session.
/// Returning `None` means "this accelerator is not mine" — the open
/// walk moves to the next bridge, and a full miss is the typed
/// `h264.hw.no_bridge` configure error (never a silent software
/// fallback: there is no software H.264 decode, ADR-001).
pub trait H264HwBridge: Send + Sync {
    /// The accelerator backend label this bridge serves (matches
    /// `HwAccelerator::backend_id`; documentation and diagnostics).
    fn backend_id(&self) -> &str;

    /// Attempt to open a session off `accel`. `None` = not this
    /// bridge's backend.
    fn open(&self, accel: &Arc<dyn HwAccelerator>) -> Option<Box<dyn H264HwDecoder>>;
}

/// Why [`H264HwPath::open`] could not open a session (typed, part
/// of the honest-matrix contract).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum H264HwOpenError {
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

impl fmt::Display for H264HwOpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            H264HwOpenError::NoAccelerator => f.write_str(
                "no hardware accelerator negotiated and H.264 has no software decode (ADR-001)",
            ),
            H264HwOpenError::NoBridge { backend } => write!(
                f,
                "accelerator {backend} has no H.264 bridge installed (Phase-7 adapter missing)"
            ),
        }
    }
}

impl std::error::Error for H264HwOpenError {}

/// The consumption point: an open hardware session fed by the
/// parser's output. Owns the [`H264HwDecoder`] the bridge minted;
/// [`super::decode::H264Decoder`] drives it.
pub struct H264HwPath {
    session: Box<dyn H264HwDecoder>,
}

impl fmt::Debug for H264HwPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The session prints as its backend label (ADR-001's truthful
        // report), not a pointer — mirrors DecoderConfig's Debug.
        f.debug_struct("H264HwPath")
            .field("backend", &self.session.backend_id())
            .finish()
    }
}

impl H264HwPath {
    /// Walk `bridges` against the negotiated `accel` and open the
    /// first session a bridge claims.
    ///
    /// # Errors
    /// [`H264HwOpenError::NoAccelerator`] when `accel` is `None`;
    /// [`H264HwOpenError::NoBridge`] when every bridge declined.
    pub fn open(
        accel: Option<&Arc<dyn HwAccelerator>>,
        bridges: &[Arc<dyn H264HwBridge>],
    ) -> Result<H264HwPath, H264HwOpenError> {
        let Some(accel) = accel else {
            return Err(H264HwOpenError::NoAccelerator);
        };
        for bridge in bridges {
            if let Some(session) = bridge.open(accel) {
                return Ok(H264HwPath { session });
            }
        }
        Err(H264HwOpenError::NoBridge {
            backend: accel.backend_id().to_string(),
        })
    }

    /// The active session's backend label (ADR-001 truthful report).
    #[must_use]
    pub fn backend_id(&self) -> &str {
        self.session.backend_id()
    }

    /// Install the stream description (see
    /// [`H264HwDecoder::configure`]).
    ///
    /// # Errors
    /// Propagates the session's typed [`H264HwError`].
    pub fn configure(&mut self, stream: &H264HwStreamInfo) -> Result<(), H264HwError> {
        self.session.configure(stream)
    }

    /// Feed one access unit (see [`H264HwDecoder::decode`]).
    ///
    /// # Errors
    /// Propagates the session's typed [`H264HwError`].
    pub fn decode(&mut self, au: &H264HwAccessUnit) -> Result<H264HwDecoded, H264HwError> {
        self.session.decode(au)
    }

    /// Drain the session (see [`H264HwDecoder::flush`]).
    ///
    /// # Errors
    /// Propagates the session's typed [`H264HwError`].
    pub fn flush(&mut self) -> Result<H264HwDecoded, H264HwError> {
        self.session.flush()
    }
}
