//! Motion-JPEG / baseline JPEG video decoder (card 2.B4; doc 04 §4
//! row "Motion-JPEG", doc 03 §7 first vertical).
//!
//! MJPEG is one complete JPEG image per packet (every frame a
//! keyframe), so the decoder is stateless between packets and `flush`
//! is immediately drained. The implemented subset is **baseline
//! sequential DCT, Huffman-coded** (ISO/IEC 10918-1 SOF0), the
//! entirety of what Motion-JPEG streams carry:
//!
//! - 8-bit samples; 1 (grayscale) or 3 (YCbCr) components;
//! - sampling factors 1..=2 per axis: 4:4:4, 4:2:2, 4:4:0, 4:2:0;
//! - one interleaved scan; restart markers (DRI/RSTn) honored;
//! - DQT (8-bit) and DHT tables anywhere before SOF0/SOS as the spec
//!   allows.
//!
//! # Output format (the card's `[VERIFY-P0]`, resolved)
//!
//! - **4:2:0 and grayscale → [`PixelFormat::I420`]**: codec-native
//!   planar output (doc 01 §3.3: "I420 — the codec-native default"),
//!   buffer layout `Y (w·h) || Cb (⌈w/2⌉·⌈h/2⌉) || Cr`, zero
//!   conversion on the decode path; grayscale carries neutral
//!   (constant-128) chroma planes — a documented representation, not
//!   fabricated data.
//! - **4:4:4 / 4:2:2 / 4:4:0 → [`PixelFormat::Rgba8`]**: the samplings
//!   have no planar vocabulary yet (a media-types extension is a
//!   `[PROPOSAL]` owed to a later card), so the decoder converts
//!   (BT.601 full-range, fixed point) after box upsampling.
//! - Colorimetry: JFIF streams (or no APP0) are tagged
//!   `matrix BT.601, range Full` — the JFIF convention — with
//!   sRGB/BT.709 primaries and sRGB transfer as the pragmatic default
//!   `[PROPOSAL]`; explicit color metadata (e.g. Adobe APP14) is not
//!   parsed in v0.
//!
//! # Unsupported (typed [`ErrorKind::Unsupported`], honest list)
//!
//! progressive JPEG (SOF2), extended/lossless/arithmetic coding,
//! 12-bit precision, 16-bit quant tables, 2-/4-component streams
//! (CMYK/YCCK), sampling factors 3–4, non-interleaved scans,
//! spectral-selection scans, field-based (interlaced) MJPEG and the
//! AVI1 chunk format (doc 04's gap list — the 5.H3 polish card).
//!
//! # Damage model (card 2.B4 / doc 03 §4.5(d))
//!
//! Header-level garbage is a typed error (nothing is emitted).
//! Entropy-level damage after a valid header — truncation, invalid
//! Huffman codes, restart mismatches — is **concealed**: the frame is
//! emitted with the undecodable region filled mid-gray and the call
//! reports [`DecodeOutcome::Recovered { reason: ConcealedFrame }`]
//! (bounded, observable, never silent). Every decode path is
//! bounds-checked and overflow-checked: no panic on any input (the
//! `fuzz_codec_mjpeg` target's guarantee).

pub(crate) mod bitreader;
pub(crate) mod color;
pub(crate) mod decoder;
pub(crate) mod error;
/// Fuzz harness (doc 04 §8.5 `fuzz_codec_mjpeg`): `pub` because the
/// registered target file and the in-tree survival tests call it from
/// outside the crate (the `demux::avi::fuzz` precedent, worklog 2.B2).
pub mod fuzz;
pub(crate) mod huffman;
pub(crate) mod idct;
pub(crate) mod parser;

use codec_api::prelude::*;
use decoder::{ConcealCause, OutputFormat};
use media_types::frame::{ColorDescription, ColorMatrix, ColorPrimaries, ColorRange, VideoFrame};
use media_types::packet::SharedBytes;

/// Decode one complete JPEG/MJPEG image (the fuzz-target entry point:
/// arbitrary bytes in, one frame or a typed error out — never a
/// panic).
///
/// This is the whole pipeline behind [`MjpegDecoder::decode`], exposed
/// separately because it needs no contract state.
///
/// # Errors
/// Typed [`EngineError`]s from the decode stage (doc 01 §3.4):
/// `Unsupported` (feature outside the baseline subset), `Corrupt`
/// (structural marker damage / truncation in the headers),
/// `Decoder`/`Bitstream` (entropy-level, when concealment is not
/// applicable), `ResourceLimit` (dimension cap).
pub fn decode_image(jpeg: &[u8]) -> Result<DecodedFrame, EngineError> {
    let image = decoder::decode(jpeg).map_err(|e| e.into_engine_error())?;
    Ok(DecodedFrame {
        width: image.width,
        height: image.height,
        format: match image.format {
            OutputFormat::I420 => DecodedFormat::I420,
            OutputFormat::Rgba8 => DecodedFormat::Rgba8,
        },
        data: image.data,
        concealed: image.concealed,
        conceal_reason: image.cause.map(|cause| match cause {
            ConcealCause::EndOfData => ConcealReason::EndOfData,
            ConcealCause::PrematureMarker(_) => ConcealReason::PrematureMarker,
            ConcealCause::InvalidCode => ConcealReason::InvalidCode,
            ConcealCause::RestartMismatch => ConcealReason::RestartMismatch,
        }),
    })
}

/// The damage class behind a concealed decode (observability for
/// `Recovered` outcomes — doc 03 §4.5(d)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConcealReason {
    /// Entropy data ended before the scan completed (truncation).
    EndOfData,
    /// A marker terminated the scan with MCUs still missing.
    PrematureMarker,
    /// A Huffman code absent from the table / a coefficient run past
    /// 63.
    InvalidCode,
    /// A restart marker was missing or misnumbered.
    RestartMismatch,
}

/// Output format of a [`DecodedFrame`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodedFormat {
    /// Planar 4:2:0: `Y (w·h) || Cb (⌈w/2⌉·⌈h/2⌉) || Cr`.
    I420,
    /// Interleaved RGBA, straight alpha.
    Rgba8,
}

/// One decoded MJPEG frame, format-independent form (the contract's
/// `Frame::Video` is assembled by [`MjpegDecoder`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedFrame {
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Pixel format of `data`.
    pub format: DecodedFormat,
    /// Frame bytes, layout per `format`.
    pub data: Vec<u8>,
    /// Entropy-level damage was concealed (mid-gray fill; see module
    /// docs).
    pub concealed: bool,
    /// The damage class when `concealed`.
    pub conceal_reason: Option<ConcealReason>,
}

/// The JFIF colorimetry default (see module docs).
const JFIF_COLOR: ColorDescription = ColorDescription {
    primaries: ColorPrimaries::Bt709,
    transfer: media_types::frame::TransferCharacteristics::Srgb,
    matrix: ColorMatrix::Bt601,
    range: ColorRange::Full,
};

/// Motion-JPEG decoder behind the codec-api [`Decoder`] contract
/// (card 2.B4; doc 01 §2.5, doc 03 §4.5).
///
/// Stateless between packets (one JPEG per packet, every frame a
/// keyframe); `configure` only installs the stream generation, so
/// re-configuration after a seek is free.
///
/// # Example
///
/// ```
/// use codec_api::prelude::*;
/// use codecs::video::mjpeg::MjpegDecoder;
/// use media_types::{GenerationId, Stamp, TrackId};
///
/// let mut decoder = MjpegDecoder::new();
/// let generation = GenerationId::FIRST
///     .next()
///     .unwrap_or_else(|e| unreachable!("{e}"));
/// decoder
///     .configure(&DecoderConfig::new(
///         media_types::ids::CodecDescriptor::new("mjpeg")
///             .unwrap_or_else(|e| unreachable!("{e}")),
///         generation,
///         ThreadBudget::default(),
///         None,
///         None,
///     ))
///     .unwrap_or_else(|e| unreachable!("{e}"));
///
/// // An empty packet is header-level damage: a typed error, nothing
/// // emitted (never a panic, never a fabricated frame).
/// let packet = Packet {
///     track: TrackId::new(0),
///     pts: Stamp::Missing,
///     dts: Stamp::Missing,
///     duration: None,
///     data: SharedBytes::from_vec(Vec::new()),
///     keyframe: true,
///     discontinuity: false,
///     generation,
/// };
/// struct Sink;
/// impl FrameSink for Sink {
///     fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
///         Ok(())
///     }
/// }
/// let outcome = decoder.decode(packet, &mut Sink);
/// assert!(outcome.is_err());
/// ```
#[derive(Debug, Default)]
pub struct MjpegDecoder {
    /// The configured stream generation (None until `configure`).
    generation: Option<GenerationId>,
    /// Codec descriptor accepted at configure time (diagnostics).
    codec: Option<String>,
}

impl MjpegDecoder {
    /// A fresh decoder, unconfigured.
    pub fn new() -> MjpegDecoder {
        MjpegDecoder::default()
    }

    /// The codec descriptors this decoder accepts (the track codec
    /// vocabulary of the first vertical: `mjpeg` per the fixture
    /// expectation TOMLs; `jpeg` accepted as the plain-image synonym).
    fn accepts(descriptor: &str) -> bool {
        matches!(descriptor, "mjpeg" | "jpeg")
    }
}

impl Decoder for MjpegDecoder {
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
        if !Self::accepts(cfg.codec.as_str()) {
            return Err(EngineError::new(
                media_types::PipelineStage::Decode,
                ErrorKind::Unsupported {
                    format: Some(media_types::error::FormatInfo {
                        name: Some(cfg.codec.to_string()),
                        sniffed: Vec::new(),
                    }),
                },
                media_types::error::ErrorContext::new("mjpeg.codec_descriptor"),
            ));
        }
        self.generation = Some(cfg.generation);
        self.codec = Some(cfg.codec.to_string());
        Ok(())
    }

    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        let Some(generation) = self.generation else {
            return Err(EngineError::new(
                media_types::PipelineStage::Decode,
                ErrorKind::Decoder {
                    cause: media_types::error::DecodeCause::BadState,
                },
                media_types::error::ErrorContext::new("mjpeg.not_configured"),
            ));
        };
        if let Some(outcome) = drop_stale_packet(&packet, generation) {
            return Ok(outcome);
        }
        let image = decoder::decode(packet.data.as_slice()).map_err(|e| e.into_engine_error())?;

        let format = match image.format {
            OutputFormat::I420 => media_types::PixelFormat::I420,
            OutputFormat::Rgba8 => media_types::PixelFormat::Rgba8,
        };
        let frame = Frame::Video(VideoFrame {
            format,
            size: (image.width, image.height),
            data: media_types::frame::FrameStorage::Cpu(SharedBytes::from_vec(image.data)),
            pts: packet.pts,
            color: JFIF_COLOR,
            interlaced: false,
            generation: packet.generation,
        });
        out.accept(frame)?;
        if image.concealed {
            Ok(DecodeOutcome::Recovered {
                skipped: 0,
                reason: RecoverReason::ConcealedFrame,
            })
        } else {
            Ok(DecodeOutcome::Emitted { frames: 1 })
        }
    }

    fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        // MJPEG carries no inter-frame state: nothing is buffered.
        if self.generation.is_none() {
            return Err(EngineError::new(
                media_types::PipelineStage::Decode,
                ErrorKind::Decoder {
                    cause: media_types::error::DecodeCause::BadState,
                },
                media_types::error::ErrorContext::new("mjpeg.not_configured"),
            ));
        }
        Ok(DecodeOutcome::Drained)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_gate_accepts_the_vertical_vocabulary() {
        assert!(MjpegDecoder::accepts("mjpeg"));
        assert!(MjpegDecoder::accepts("jpeg"));
        assert!(!MjpegDecoder::accepts("h264"));
        assert!(!MjpegDecoder::accepts(""));
    }

    #[test]
    fn jfif_color_tag_is_bt601_full_range() {
        assert_eq!(JFIF_COLOR.matrix, ColorMatrix::Bt601);
        assert_eq!(JFIF_COLOR.range, ColorRange::Full);
        assert_eq!(JFIF_COLOR.primaries, ColorPrimaries::Bt709);
    }
}
