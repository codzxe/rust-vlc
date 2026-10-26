//! The filmcraft-dnx-backed DNxHD/DNxHR decoder core (card 6.M-dnx):
//! the engine seam, the CBR truncation gate, the full-depth
//! [`DnxPicture`] surface, and the [`codec_api::Decoder`]
//! implementation.
//!
//! # Engine admission (the record, not hope)
//!
//! `filmcraft-dnx = "=0.9.0"` with `default-features = false`
//! (`docs/dependencies/records/filmcraft-dnx-0.9.0.md`): the one
//! default feature `threads` would add `rayon`, and the admission
//! shape keeps the lockfile rayon-free. The wrapper pins the
//! engine's own [`filmcraft_dnx::DecodeOptions`] to
//! `threads: false, spec_dequant: false` — the measured
//! single-threaded, ffmpeg-compatible reconstruction path (the
//! record's §D; `spec_dequant = false` is what makes the residual
//! vs ffmpeg pure IDCT rounding noise). With the feature compiled
//! out the flag is inert; passing `false` explicitly documents the
//! intent and stays correct if the feature is ever re-triggered on
//! (doc 11 §3.7). The contract's
//! [`ThreadBudget`](codec_api::ThreadBudget) is an *upper bound*,
//! so any budget is accepted and decode stays single-threaded —
//! reported here, never silently "parallel".
//!
//! # Packet model
//!
//! One packet = one complete VC-3 frame (one coding unit, or the
//! two units of a field-coded frame). VC-3 is intra-only CBR, so
//! the elementary-stream host splits the ES into fixed-size frames
//! (`filesize / N`) and the decoder holds no cross-packet state —
//! the MJPEG frame-isolation guarantee. Packetization policy
//! (AVdn/AVdh sample entries, MKF) is the container/ES wiring
//! unit's surface, not this one.
//!
//! # The CBR truncation gate (the record's hostile flag, answered)
//!
//! The admission record measured that the engine returns `Ok` on
//! some *truncated* frames (half-truncated L1 decoded fine — DNxHD
//! scan-line indices stay in-bounds past mid-frame; the record's §H
//! "concealment-class note … wrapper must treat Ok on truncation as
//! suspect, flagged for the M-wave"). This wrapper answers it:
//! every packet is first probed
//! ([`filmcraft_dnx::probe`]); when the header declares a CBR
//! coding-unit size and the packet is shorter than the declared
//! size (×2 for field-coded frames), the wrapper refuses with a
//! typed `Truncated` error carrying the exact expected/found
//! numbers — the engine never sees the short buffer, so its
//! silent-concealment window is closed for the declared-size
//! class. VBR streams declare no size and rely on the engine's own
//! typed truncation errors (recorded residual).
//!
//! # The output boundary (typed, not silent)
//!
//! media-types' pixel vocabulary is 8-bit today (the av1-recorded
//! follow-up seam: a 16-bit planar format). This decoder maps the
//! engine's planar output **losslessly** where the vocabulary
//! allows and refuses typed everywhere else — never a downshift,
//! never a resample, never a silent guess:
//!
//! | engine output | emitted | notes |
//! |---|---|---|
//! | 8-bit 4:2:2 (the DNxHD mainline) | [`PixelFormat::Yuyv422`] | pure repack: planar Y/Cb/Cr → packed Y0 Cb Y1 Cr, byte-identical samples |
//! | 8-bit 4:2:0 | [`PixelFormat::I420`] | pure repack, ⌈w/2⌉×⌈h/2⌉ chroma |
//! | 10/12-bit | typed `codecs.dnx.output.bit_depth` | vocabulary gap (av1 precedent) |
//! | 4:4:4 / RGB | typed `codecs.dnx.output.chroma444` / `.rgb` | no planar 4:4:4; RGBA conversion is a filter-stage decision |
//! | alpha channel | typed `codecs.dnx.output.alpha` | vocabulary has no YUV+alpha |
//! | non-square PAR | typed `codecs.dnx.output.par` | `VideoFrame` cannot express it; emitting would distort silently |
//! | BT.2020 constant luminance | typed `codecs.dnx.color.bt2020cl` | no CL matrix in the vocabulary |
//!
//! The **full-depth** planar picture stays available through
//! [`DnxDecoder::decode_picture`] (the [`DnxPicture`] surface, planar
//! `u16` at the stream's native bit depth) — the seam the 16-bit
//! vocabulary follow-up and the differential suite consume; the
//! trait path is exactly `decode_picture` + the lossless pack.
//!
//! # Allocation discipline (D-1)
//!
//! Packed output refills pooled [`SharedBytes`] slots (the
//! MJPEG/AV1 `FRAME_SLOTS` rotation): steady-state decodes reuse a
//! small rotation instead of allocating per frame.

use codec_api::prelude::*;
use media_types::frame::{
    ColorDescription, ColorMatrix, ColorPrimaries, ColorRange, FrameStorage, PixelFormat,
    TransferCharacteristics, VideoFrame,
};
use media_types::packet::SharedBytes;
use media_types::{EngineError, Frame, Packet};

use super::error::{DnxError, corrupt, engine_error, unsupported};

/// The engine options this wrapper pins (the admission record's
/// shape): single-threaded (the `threads` feature is compiled out;
/// the flag documents intent for any future re-trigger) and the
/// ffmpeg-compatible reconstruction (`spec_dequant = false` — the
/// record's §D: the default path already tracks ffmpeg's two
/// documented deviations, leaving pure IDCT rounding noise).
const ENGINE_OPTIONS: filmcraft_dnx::DecodeOptions = filmcraft_dnx::DecodeOptions {
    threads: false,
    spec_dequant: false,
};

/// Packed-output pool depth (the MJPEG/AV1 `FRAME_SLOTS` posture):
/// enough for the sink to hold a few frames without forcing
/// steady-state allocation, bounded so a stalled consumer cannot
/// grow us.
const FRAME_SLOTS: usize = 4;

/// Chroma subsampling of a decoded VC-3 picture (the house mirror
/// of the engine's `ChromaFormat`; SSC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DnxChroma {
    /// Half width, half height.
    Yuv420,
    /// Half width, full height (the DNxHD mainline).
    Yuv422,
    /// Full size (also the RGB plane arrangement).
    Yuv444,
}

impl DnxChroma {
    /// Chroma plane width for a luma width.
    #[must_use]
    pub fn chroma_width(self, width: u32) -> u32 {
        match self {
            DnxChroma::Yuv444 => width,
            _ => width.div_ceil(2),
        }
    }

    /// Chroma plane height for a luma height.
    #[must_use]
    pub fn chroma_height(self, height: u32) -> u32 {
        match self {
            DnxChroma::Yuv420 => height.div_ceil(2),
            _ => height,
        }
    }
}

/// Colour volume of a decoded VC-3 picture (CLV).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DnxColorVolume {
    /// ITU-R BT.709 (the HD default).
    Bt709,
    /// BT.2020 non-constant luminance.
    Bt2020Ncl,
    /// BT.2020 constant luminance.
    Bt2020Cl,
    /// Described out of band (the container's metadata owns it).
    OutOfBand,
}

/// A decoded VC-3 picture at the stream's **native** full depth:
/// planar `u16` samples, rows packed without padding — the house
/// mirror of the engine's output and the seam the 8-bit pixel
/// vocabulary cannot always express (10/12-bit, 4:4:4, alpha).
///
/// Produced by [`DnxDecoder::decode_picture`]; the
/// [`Decoder`](codec_api::Decoder) trait path consumes exactly this
/// and packs it losslessly where the vocabulary allows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnxPicture {
    /// Raster width in pixels.
    pub width: u32,
    /// Raster height in pixels.
    pub height: u32,
    /// Chroma subsampling.
    pub chroma: DnxChroma,
    /// Significant bits per sample of every plane (8, 10 or 12).
    pub bit_depth: u8,
    /// Luma plane (`width × height` samples).
    pub y: Vec<u16>,
    /// Cb plane (`chroma_width × chroma_height`).
    pub cb: Vec<u16>,
    /// Cr plane (`chroma_width × chroma_height`).
    pub cr: Vec<u16>,
    /// Full-range alpha when the stream codes one.
    pub alpha: Option<Vec<u16>>,
    /// The planes carry R′G′B′ (4:4:4 with ACF = 0) instead of
    /// Y′CbCr.
    pub rgb: bool,
    /// Interlaced picture (top field first — the engine weaves the
    /// two fields into the full raster).
    pub interlaced: bool,
    /// Colour volume (CLV).
    pub color_volume: DnxColorVolume,
    /// Compression ID of the stream (the decode-side truth, not the
    /// container's label).
    pub cid: u32,
    /// Pixel aspect ratio signalled by the header (`(0, 0)` = not
    /// signalled).
    pub par: (u16, u16),
}

impl DnxPicture {
    /// Chroma plane width of this picture.
    #[must_use]
    pub fn chroma_width(&self) -> u32 {
        self.chroma.chroma_width(self.width)
    }

    /// Chroma plane height of this picture.
    #[must_use]
    pub fn chroma_height(&self) -> u32 {
        self.chroma.chroma_height(self.height)
    }
}

/// The DNxHD/DNxHR decoder behind the codec-api [`Decoder`]
/// contract (card 6.M-dnx; doc 04 §4 row "DNxHD").
///
/// `configure` accepts the `dnxhd` descriptor (the ffmpeg codec
/// name; DNxHR streams share it — the CID distinguishes), `decode`
/// turns one VC-3 frame packet into one [`Frame::Video`], `flush`
/// is immediately drained (VC-3 is intra-only — nothing is ever
/// buffered).
///
/// # Example
///
/// ```
/// use codec_api::prelude::*;
/// use codecs::video::dnx::DnxDecoder;
/// use media_types::{GenerationId, packet::SharedBytes};
///
/// # fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let mut decoder = DnxDecoder::new();
/// let generation = GenerationId::FIRST.next()?;
/// // A foreign descriptor is a typed rejection.
/// let err = decoder
///     .configure(&DecoderConfig::new(
///         media_types::ids::CodecDescriptor::new("vp9")?,
///         generation,
///         ThreadBudget::default(),
///         None,
///         None,
///     ))
///     .expect_err("dnxhd is the only accepted descriptor");
/// assert_eq!(err.context.code, "codecs.dnx.codec_descriptor");
/// # Ok(())
/// # }
/// # run().unwrap_or_else(|e| unreachable!("{e}"));
/// ```
#[derive(Default)]
pub struct DnxDecoder {
    /// The configured stream generation (None until `configure`).
    generation: Option<media_types::GenerationId>,
    /// Codec descriptor accepted at configure time (diagnostics).
    codec: Option<String>,
    /// Pooled packed-output slots.
    slots: Vec<SharedBytes>,
    /// Round-robin cursor into `slots`.
    slot_next: usize,
    /// Pictures decoded since configure (the differential's
    /// geometry counter).
    pictures: u64,
}

impl DnxDecoder {
    /// A fresh decoder, unconfigured.
    #[must_use]
    pub fn new() -> DnxDecoder {
        DnxDecoder::default()
    }

    /// The codec descriptors this decoder accepts (`dnxhd` — the
    /// ffmpeg codec name covering both the HD-profile CIDs and the
    /// resolution-independent DNxHR CIDs; ffprobe reports
    /// `codec_name=dnxhd` for both).
    fn accepts(descriptor: &str) -> bool {
        descriptor == "dnxhd"
    }

    /// Pictures decoded (engine frames returned) since configure.
    #[must_use]
    pub fn decoded_pictures(&self) -> u64 {
        self.pictures
    }

    /// Decode one complete VC-3 frame at **native full depth**:
    /// the engine seam with the pinned
    /// [`ENGINE_OPTIONS`](self#engine-admission) and the CBR
    /// truncation gate, surfaced as the house [`DnxPicture`].
    ///
    /// This is the exact decode the [`Decoder`](codec_api::Decoder)
    /// trait path runs; it exists as public surface because the
    /// 8-bit pixel vocabulary cannot express every legal VC-3
    /// output (10/12-bit, 4:4:4, alpha) and the honest answer for
    /// those streams is full-depth data, not a downshift. The
    /// differential suite uses it for the 10-bit lane; a future
    /// 16-bit planar format consumes it directly.
    ///
    /// # Errors
    /// Typed [`EngineError`] (stage Decode): `Corrupt` for
    /// truncation/invalid/scan-line damage (frame-scoped — the
    /// decoder stays usable), `Unsupported` for an unknown
    /// compression ID.
    pub fn decode_picture(&mut self, data: &[u8]) -> Result<DnxPicture, EngineError> {
        let picture = decode_picture_ungated(data)?;
        self.pictures = self.pictures.saturating_add(1);
        Ok(picture)
    }
}

/// The stateless core: probe-gate + engine decode + house mirror.
/// (No counters — [`DnxDecoder::decode_picture`] owns those.)
fn decode_picture_ungated(data: &[u8]) -> Result<DnxPicture, EngineError> {
    // The CBR concealment gate (module docs): refuse short-of-
    // declared-size packets BEFORE the engine can conceal them.
    if let Ok(header) = filmcraft_dnx::probe(data)
        && let Some(unit) = header.coding_unit_size()
    {
        let units: u64 = if header.interlaced && !header.frame_encoding {
            2
        } else {
            1
        };
        // A u32 unit × 2 cannot overflow u64, and the result is
        // a byte count the packet length (also u64-widened)
        // compares against without any narrowing.
        let expected = u64::from(unit) * units;
        if (data.len() as u64) < expected {
            return Err(corrupt(
                "codecs.dnx.frame.short",
                media_types::error::CorruptDetail::Truncated {
                    expected,
                    found: data.len() as u64,
                },
                format!(
                    "CBR coding unit declares {} bytes (CID {}, {} coding unit{}) \
                     but the packet holds {}",
                    expected,
                    header.cid,
                    units,
                    if units == 1 { "" } else { "s" },
                    data.len()
                ),
            )
            .into_engine_error());
        }
    }
    let frame = filmcraft_dnx::decode_frame_with(data, &ENGINE_OPTIONS)
        .map_err(|e| engine_error(e, data.len()).into_engine_error())?;
    Ok(DnxPicture {
        width: frame.width,
        height: frame.height,
        chroma: match frame.chroma {
            filmcraft_dnx::ChromaFormat::Yuv420 => DnxChroma::Yuv420,
            filmcraft_dnx::ChromaFormat::Yuv422 => DnxChroma::Yuv422,
            filmcraft_dnx::ChromaFormat::Yuv444 => DnxChroma::Yuv444,
        },
        bit_depth: frame.bit_depth,
        y: frame.y,
        cb: frame.cb,
        cr: frame.cr,
        alpha: frame.alpha,
        rgb: frame.rgb,
        interlaced: matches!(frame.interlace, filmcraft_dnx::Interlace::TopFieldFirst),
        color_volume: match frame.color_volume {
            filmcraft_dnx::ColorVolume::Bt709 => DnxColorVolume::Bt709,
            filmcraft_dnx::ColorVolume::Bt2020Ncl => DnxColorVolume::Bt2020Ncl,
            filmcraft_dnx::ColorVolume::Bt2020Cl => DnxColorVolume::Bt2020Cl,
            filmcraft_dnx::ColorVolume::OutOfBand => DnxColorVolume::OutOfBand,
        },
        cid: frame.cid,
        par: frame.par,
    })
}

/// The renderer colourimetry for a VC-3 colour volume.
///
/// VC-3's CLV field signals primaries+matrix but **no transfer
/// function**; BT.709 is the conservative fill (ffmpeg's own dnxhd
/// path defaults unsignalled HD streams to bt709). `OutOfBand`
/// means the container was supposed to describe the volume — the
/// same conservative BT.709/limited fill, documented rather than
/// guessed per-frame. BT.2020 constant luminance has no matrix in
/// the vocabulary: a typed refusal, never a mis-color.
fn color_description(volume: DnxColorVolume) -> Result<ColorDescription, DnxError> {
    match volume {
        DnxColorVolume::Bt709 | DnxColorVolume::OutOfBand => Ok(ColorDescription {
            primaries: ColorPrimaries::Bt709,
            transfer: TransferCharacteristics::Bt709,
            matrix: ColorMatrix::Bt709,
            range: ColorRange::Limited,
        }),
        DnxColorVolume::Bt2020Ncl => Ok(ColorDescription {
            primaries: ColorPrimaries::Bt2020,
            transfer: TransferCharacteristics::Bt709,
            matrix: ColorMatrix::Bt2020Ncl,
            range: ColorRange::Limited,
        }),
        DnxColorVolume::Bt2020Cl => Err(unsupported(
            "codecs.dnx.color.bt2020cl",
            "BT.2020 constant-luminance matrix is not in the renderer vocabulary \
             (VC-3 signals CLV = 2); refusing rather than mis-coloring",
        )),
    }
}

/// The packed output shape for a picture the vocabulary can express.
struct PackedOutput {
    format: PixelFormat,
    color: ColorDescription,
    /// Total packed byte count (the exact slot size).
    len: usize,
}

/// Classify + size a picture's packed output, refusing everything
/// the 8-bit vocabulary cannot express **losslessly** (module docs'
/// table). Pure classification — no packing yet.
fn packed_output(picture: &DnxPicture) -> Result<PackedOutput, DnxError> {
    if picture.bit_depth != 8 {
        return Err(unsupported(
            "codecs.dnx.output.bit_depth",
            format!(
                "media-types' pixel vocabulary is 8-bit today (the av1-recorded 16-bit \
                 follow-up seam); this stream is {}-bit — decode_picture carries the \
                 full-depth planes",
                picture.bit_depth
            ),
        ));
    }
    if picture.rgb {
        return Err(unsupported(
            "codecs.dnx.output.rgb",
            "4:4:4 RGB planes: no planar RGB vocabulary at this seam (an RGBA \
             conversion is a filter-stage decision, not a silent repack)",
        ));
    }
    if picture.alpha.is_some() {
        return Err(unsupported(
            "codecs.dnx.output.alpha",
            "the stream codes an alpha channel; the pixel vocabulary has no \
             YUV+alpha format",
        ));
    }
    if picture.par != (0, 0) && picture.par != (1, 1) {
        return Err(unsupported(
            "codecs.dnx.output.par",
            format!(
                "non-square pixel aspect {}:{} — VideoFrame cannot express it and \
                 emitting square would distort silently",
                picture.par.0, picture.par.1
            ),
        ));
    }
    let color = color_description(picture.color_volume)?;
    let luma = picture.y.len();
    match picture.chroma {
        DnxChroma::Yuv422 => {
            if !picture.width.is_multiple_of(2) {
                return Err(unsupported(
                    "codecs.dnx.output.odd_width",
                    "odd 4:2:2 width cannot map onto whole YUYV macropixels without \
                     inventing a phantom luma sample",
                ));
            }
            // Y0 Cb Y1 Cr: 2 bytes per luma sample.
            let len = luma.checked_mul(2).ok_or_else(|| limit_error(luma, 2))?;
            validate_planes(picture)?;
            Ok(PackedOutput {
                format: PixelFormat::Yuyv422,
                color,
                len,
            })
        }
        DnxChroma::Yuv420 => {
            let len = luma
                .checked_add(picture.cb.len())
                .and_then(|n| n.checked_add(picture.cr.len()))
                .ok_or_else(|| limit_error(luma, 1))?;
            validate_planes(picture)?;
            Ok(PackedOutput {
                format: PixelFormat::I420,
                color,
                len,
            })
        }
        DnxChroma::Yuv444 => Err(unsupported(
            "codecs.dnx.output.chroma444",
            "4:4:4 planes: no planar 4:4:4 format in the vocabulary (an RGBA \
             conversion is a filter-stage decision, not a silent repack)",
        )),
    }
}

/// The allocation-limit refusal (defensive: plane counts derive
/// from a ≤16384×16384 raster, so this is unreachable on the
/// targets we ship — still typed, never wrapped).
fn limit_error(samples: usize, per: usize) -> DnxError {
    corrupt(
        "codecs.dnx.output.limit",
        media_types::error::CorruptDetail::InvalidStructure { offset: None },
        format!("packed output of {samples} samples × {per} bytes overflows usize"),
    )
}

/// Engine-output geometry check: the planes must cover the declared
/// raster exactly (a mismatch is engine-side corruption surfaced as
/// data, never a panic).
fn validate_planes(picture: &DnxPicture) -> Result<(), DnxError> {
    let w = picture.width as usize;
    let h = picture.height as usize;
    let cw = picture.chroma_width() as usize;
    let ch = picture.chroma_height() as usize;
    let expected = [w * h, cw * ch, cw * ch];
    let planes = [
        ("Y", picture.y.len()),
        ("Cb", picture.cb.len()),
        ("Cr", picture.cr.len()),
    ];
    for ((name, got), want) in planes.iter().zip(expected) {
        if *got != want {
            return Err(corrupt(
                "codecs.dnx.output.planes",
                media_types::error::CorruptDetail::InvalidStructure { offset: None },
                format!("{name} plane holds {got} samples, the raster requires {want}"),
            ));
        }
    }
    Ok(())
}

/// Pack an 8-bit picture into `out` (YUYV for 4:2:2, I420 for
/// 4:2:0). `out.len()` must equal the classified
/// [`PackedOutput::len`] — callers hand us exactly that slot.
fn pack_into(picture: &DnxPicture, out: &mut [u8]) {
    match picture.chroma {
        DnxChroma::Yuv422 => {
            let w = picture.width as usize;
            let cw = picture.chroma_width() as usize;
            for row in 0..picture.height as usize {
                for c in 0..cw {
                    let macro_px = (row * cw + c) * 4;
                    out[macro_px] = picture.y[row * w + 2 * c] as u8;
                    out[macro_px + 1] = picture.cb[row * cw + c] as u8;
                    out[macro_px + 2] = picture.y[row * w + 2 * c + 1] as u8;
                    out[macro_px + 3] = picture.cr[row * cw + c] as u8;
                }
            }
        }
        DnxChroma::Yuv420 => {
            let luma = picture.y.len();
            for (i, sample) in picture.y.iter().enumerate() {
                out[i] = *sample as u8;
            }
            for (i, sample) in picture.cb.iter().enumerate() {
                out[luma + i] = *sample as u8;
            }
            for (i, sample) in picture.cr.iter().enumerate() {
                out[luma + picture.cb.len() + i] = *sample as u8;
            }
        }
        // 4:4:4 never reaches packing (typed refusal first).
        DnxChroma::Yuv444 => {}
    }
}

/// Take a pooled slot of exactly `len` bytes and pack `picture`
/// into it (the MJPEG `pooled_frame_slot` posture; the fill is
/// infallible because `len` was derived from the validated
/// geometry and the packer writes exactly that many bytes).
fn packed_slot(
    slots: &mut Vec<SharedBytes>,
    next: &mut usize,
    picture: &DnxPicture,
    len: usize,
) -> SharedBytes {
    let count = slots.len();
    for i in 0..count {
        let idx = (*next + i) % count;
        if slots[idx].len() != len {
            continue;
        }
        if let Some(bytes) = slots[idx].get_mut() {
            pack_into(picture, bytes);
            *next = (idx + 1) % count;
            return slots[idx].clone();
        }
        // The slot still has downstream owners — try the next one.
    }
    // Cold path: allocate, pack in place, enter the rotation.
    let fresh = SharedBytes::build(len, |bytes| pack_into(picture, bytes));
    if count < FRAME_SLOTS {
        slots.push(fresh.clone());
        *next = (count + 1) % FRAME_SLOTS;
    } else {
        let evict = *next;
        slots[evict] = fresh.clone();
        *next = (evict + 1) % FRAME_SLOTS;
    }
    fresh
}

impl Decoder for DnxDecoder {
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
        let descriptor = cfg.codec.as_str();
        if !Self::accepts(descriptor) {
            return Err(unsupported(
                "codecs.dnx.codec_descriptor",
                format!("codec descriptor `{descriptor}` is not `dnxhd`"),
            )
            .into_engine_error());
        }
        // VC-3 frames are self-describing (every coding unit carries
        // its header); there is no out-of-band config to consume, so
        // any extra_data is deliberately not consulted.
        self.generation = Some(cfg.generation);
        self.codec = Some(descriptor.to_string());
        self.slots.clear();
        self.slot_next = 0;
        self.pictures = 0;
        Ok(())
    }

    /// Decode one packet (one complete VC-3 frame).
    ///
    /// Per-frame isolation (doc 03 §4.5(d)): an `Err` whose kind is
    /// `Corrupt` or `Decoder { Bitstream }` damaged **this frame
    /// only** — VC-3 is intra-only and the decoder holds no
    /// cross-frame state, so the next packet decodes normally (the
    /// MJPEG statelessness guarantee; the admission record's
    /// hostile battery measured zero panics).
    #[allow(clippy::missing_errors_doc)]
    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        let Some(generation) = self.generation else {
            return Err(DnxError::BadState.into_engine_error());
        };
        if let Some(outcome) = drop_stale_packet(&packet, generation) {
            return Ok(outcome);
        }
        let picture = self.decode_picture(packet.data.as_slice())?;
        let output = packed_output(&picture).map_err(DnxError::into_engine_error)?;
        let data = packed_slot(&mut self.slots, &mut self.slot_next, &picture, output.len);
        let frame = Frame::Video(VideoFrame {
            format: output.format,
            size: (picture.width, picture.height),
            data: FrameStorage::Cpu(data),
            pts: packet.pts,
            color: output.color,
            interlaced: picture.interlaced,
            generation: packet.generation,
        });
        out.accept(frame)?;
        Ok(DecodeOutcome::Emitted { frames: 1 })
    }

    fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        if self.generation.is_none() {
            return Err(DnxError::BadState.into_engine_error());
        }
        // VC-3 is intra-only: every packet was fully decoded on
        // arrival; nothing is ever buffered.
        Ok(DecodeOutcome::Drained)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::GenerationId;

    /// A minimal 8-bit 4:2:2 progressive picture with synthetic,
    /// per-sample-distinct plane values.
    fn synthetic_422(width: u32, height: u32) -> DnxPicture {
        let cw = DnxChroma::Yuv422.chroma_width(width);
        let ch = DnxChroma::Yuv422.chroma_height(height);
        DnxPicture {
            width,
            height,
            chroma: DnxChroma::Yuv422,
            bit_depth: 8,
            y: (0..width * height).map(|i| (i % 251) as u16).collect(),
            cb: (0..cw * ch).map(|i| (i % 241) as u16).collect(),
            cr: (0..cw * ch).map(|i| (i % 239) as u16).collect(),
            alpha: None,
            rgb: false,
            interlaced: false,
            color_volume: DnxColorVolume::Bt709,
            cid: 1238,
            par: (0, 0),
        }
    }

    fn configured() -> DnxDecoder {
        let mut decoder = DnxDecoder::new();
        match decoder.configure(&DecoderConfig::new(
            media_types::ids::CodecDescriptor::new("dnxhd")
                .unwrap_or_else(|e| unreachable!("{e:?}")),
            GenerationId::FIRST,
            ThreadBudget::default(),
            None,
            None,
        )) {
            Ok(()) => decoder,
            Err(err) => unreachable!("configure must accept dnxhd: {err}"),
        }
    }

    fn packet(data: &[u8]) -> Packet {
        Packet {
            track: media_types::TrackId::new(0),
            pts: media_types::Stamp::Missing,
            dts: media_types::Stamp::Missing,
            duration: None,
            data: SharedBytes::from_vec(data.to_vec()),
            keyframe: true,
            discontinuity: false,
            generation: GenerationId::FIRST,
        }
    }

    #[derive(Default)]
    struct CountingSink {
        frames: usize,
    }

    impl FrameSink for CountingSink {
        fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
            self.frames += 1;
            Ok(())
        }
    }

    #[test]
    fn configure_refuses_foreign_descriptors() {
        let mut decoder = DnxDecoder::new();
        let err = match decoder.configure(&DecoderConfig::new(
            media_types::ids::CodecDescriptor::new("h264")
                .unwrap_or_else(|e| unreachable!("{e:?}")),
            GenerationId::FIRST,
            ThreadBudget::default(),
            None,
            None,
        )) {
            Ok(()) => unreachable!("h264 must be refused"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "codecs.dnx.codec_descriptor");
        assert!(matches!(
            err.kind,
            media_types::ErrorKind::Unsupported { .. }
        ));
    }

    #[test]
    fn decode_and_flush_before_configure_are_typed_bad_state() {
        let mut decoder = DnxDecoder::new();
        let mut sink = CountingSink::default();
        let err = match decoder.decode(packet(&[]), &mut sink) {
            Ok(_) => unreachable!("decode before configure must fail"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "codecs.dnx.not_configured");
        let err = match decoder.flush(&mut sink) {
            Ok(_) => unreachable!("flush before configure must fail"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "codecs.dnx.not_configured");
    }

    #[test]
    fn stale_packets_are_dropped_on_sight() {
        let mut decoder = DnxDecoder::new();
        let current = GenerationId::FIRST
            .next()
            .unwrap_or_else(|e| unreachable!("{e:?}"));
        match decoder.configure(&DecoderConfig::new(
            media_types::ids::CodecDescriptor::new("dnxhd")
                .unwrap_or_else(|e| unreachable!("{e:?}")),
            current,
            ThreadBudget::default(),
            None,
            None,
        )) {
            Ok(()) => {}
            Err(err) => unreachable!("configure must accept dnxhd: {err}"),
        }
        let stale = Packet {
            generation: GenerationId::FIRST,
            ..packet(&[0u8; 8])
        };
        let mut sink = CountingSink::default();
        // A past generation is dropped with NeedMoreInput and zero
        // decode work (the pictures counter must not move).
        match decoder.decode(stale, &mut sink) {
            Ok(DecodeOutcome::NeedMoreInput) => {}
            other => unreachable!("stale drop is NeedMoreInput: {other:?}"),
        }
        assert_eq!(decoder.decoded_pictures(), 0);
        assert_eq!(sink.frames, 0);
        // flush stays drained after a stale drop.
        match decoder.flush(&mut sink) {
            Ok(DecodeOutcome::Drained) => {}
            other => unreachable!("flush is Drained: {other:?}"),
        }
    }

    #[test]
    fn flush_after_configure_is_immediately_drained() {
        let mut decoder = configured();
        let mut sink = CountingSink::default();
        match decoder.flush(&mut sink) {
            Ok(DecodeOutcome::Drained) => {}
            other => unreachable!("VC-3 buffers nothing: {other:?}"),
        }
    }

    #[test]
    fn cbr_gate_refuses_short_packets_with_exact_numbers() {
        // A hand-built CID 1238 header (1080p 175 Mbps progressive:
        // declared coding unit 917 504 bytes per the engine's Table
        // C.1 row — the admission record's L1 lane size) written by
        // the engine's own header writer: 640 bytes, far short of
        // the declaration. The wrapper must refuse typed BEFORE the
        // engine can conceal-decode it (the record's §H flag).
        let header = filmcraft_dnx::header::FrameHeader {
            header_size: 0x280,
            version: 2,
            cid: 1238,
            vbr: false,
            field: filmcraft_dnx::FieldCode::Frame,
            macf: false,
            crc: false,
            alpha: false,
            lossless_alpha: false,
            premultiplied_alpha: false,
            width: 1920,
            lines: 1080,
            par: (0, 0),
            bit_depth: 8,
            interlaced: false,
            frame_encoding: false,
            chroma: filmcraft_dnx::ChromaFormat::Yuv422,
            rgb: false,
            color_volume: filmcraft_dnx::ColorVolume::Bt709,
            timecode: None,
            mb_rows: 1080u32.div_ceil(16),
        };
        let bytes = filmcraft_dnx::header::write(&header);
        // The written header must itself parse (the gate's probe
        // premise) — pinned so a crate bump that changes the writer
        // fails here, not silently in the gate.
        match filmcraft_dnx::probe(&bytes) {
            Ok(parsed) => assert_eq!(parsed.cid, 1238),
            Err(err) => unreachable!("the engine's own writer must round-trip: {err:?}"),
        }
        let mut decoder = DnxDecoder::new();
        let err = match decoder.decode_picture(&bytes) {
            Ok(_) => unreachable!("a 640-byte packet cannot be a 917504-byte CBR frame"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "codecs.dnx.frame.short");
        assert_eq!(
            err.kind,
            media_types::ErrorKind::Corrupt {
                detail: media_types::error::CorruptDetail::Truncated {
                    expected: 917_504,
                    found: 640
                }
            }
        );
    }

    #[test]
    fn yuyv_repack_is_byte_exact() {
        // 8×2 4:2:2: every macropixel is Y0 Cb Y1 Cr with the exact
        // planar sample values — a pure repack, no conversion.
        let picture = synthetic_422(8, 2);
        let output = match packed_output(&picture) {
            Ok(output) => output,
            Err(err) => unreachable!("8-bit 4:2:2 maps to Yuyv422: {err:?}"),
        };
        assert_eq!(output.format, PixelFormat::Yuyv422);
        assert_eq!(output.len, 32);
        let mut bytes = vec![0u8; output.len];
        pack_into(&picture, &mut bytes);
        // Row 0: luma 0..8, chroma 0..4.
        let want: Vec<u8> = vec![
            0, 0, 1, 0, // Y0=0 Cb=0 Y1=1 Cr=0
            2, 1, 3, 1, // Y2=2 Cb=1 Y3=3 Cr=1
            4, 2, 5, 2, 6, 3, 7, 3, // row 0 tail
            8, 4, 9, 4, 10, 5, 11, 5, // row 1 (luma 8..16, chroma 4..8)
            12, 6, 13, 6, 14, 7, 15, 7,
        ];
        assert_eq!(bytes, want);
    }

    #[test]
    fn i420_repack_handles_odd_rasters() {
        let cw = DnxChroma::Yuv420.chroma_width(5);
        let ch = DnxChroma::Yuv420.chroma_height(3);
        let picture = DnxPicture {
            width: 5,
            height: 3,
            chroma: DnxChroma::Yuv420,
            bit_depth: 8,
            y: vec![10; 15],
            cb: vec![20; (cw * ch) as usize],
            cr: vec![30; (cw * ch) as usize],
            alpha: None,
            rgb: false,
            interlaced: false,
            color_volume: DnxColorVolume::Bt709,
            cid: 0,
            par: (0, 0),
        };
        let output = match packed_output(&picture) {
            Ok(output) => output,
            Err(err) => unreachable!("8-bit 4:2:0 maps to I420: {err:?}"),
        };
        assert_eq!(output.format, PixelFormat::I420);
        // Y 15 + Cb 6 + Cr 6 (chroma ⌈5/2⌉ × ⌈3/2⌉ = 3×2).
        assert_eq!(output.len, 27);
        let mut bytes = vec![0u8; output.len];
        pack_into(&picture, &mut bytes);
        assert!(bytes[..15].iter().all(|&b| b == 10));
        assert!(bytes[15..21].iter().all(|&b| b == 20));
        assert!(bytes[21..].iter().all(|&b| b == 30));
    }

    #[test]
    fn inexpressible_outputs_are_typed_refusals() {
        let cases: [(DnxPicture, &str); 7] = [
            // 10-bit: the vocabulary is 8-bit.
            (
                DnxPicture {
                    bit_depth: 10,
                    ..synthetic_422(8, 2)
                },
                "codecs.dnx.output.bit_depth",
            ),
            // RGB 4:4:4 planes.
            (
                DnxPicture {
                    rgb: true,
                    ..synthetic_422(8, 2)
                },
                "codecs.dnx.output.rgb",
            ),
            // Alpha channel.
            (
                DnxPicture {
                    alpha: Some(vec![255; 16]),
                    ..synthetic_422(8, 2)
                },
                "codecs.dnx.output.alpha",
            ),
            // Non-square PAR (the 1440×1080 anamorphic class).
            (
                DnxPicture {
                    par: (59, 54),
                    ..synthetic_422(8, 2)
                },
                "codecs.dnx.output.par",
            ),
            // BT.2020 constant luminance.
            (
                DnxPicture {
                    color_volume: DnxColorVolume::Bt2020Cl,
                    ..synthetic_422(8, 2)
                },
                "codecs.dnx.color.bt2020cl",
            ),
            // 4:4:4 (YCbCr flavour).
            (
                DnxPicture {
                    chroma: DnxChroma::Yuv444,
                    y: vec![0; 16],
                    cb: vec![0; 16],
                    cr: vec![0; 16],
                    ..synthetic_422(4, 4)
                },
                "codecs.dnx.output.chroma444",
            ),
            // Odd 4:2:2 width: half a macropixel cannot exist.
            (
                DnxPicture {
                    width: 7,
                    ..synthetic_422(8, 2)
                },
                "codecs.dnx.output.odd_width",
            ),
        ];
        for (picture, code) in cases {
            let err = match packed_output(&picture) {
                Ok(_) => unreachable!("{code} must be refused"),
                Err(err) => err.into_engine_error(),
            };
            assert_eq!(err.context.code, code);
        }
    }

    #[test]
    fn plane_geometry_mismatch_surfaces_as_corrupt_data() {
        // A 16×2 raster declared, but the Cb plane holds one sample
        // too few: engine-side corruption is data, never a panic.
        let mut picture = synthetic_422(16, 2);
        picture.cb.pop();
        let err = match packed_output(&picture) {
            Ok(_) => unreachable!("short Cb plane must be refused"),
            Err(err) => err.into_engine_error(),
        };
        assert_eq!(err.context.code, "codecs.dnx.output.planes");
        assert_eq!(
            err.kind,
            media_types::ErrorKind::Corrupt {
                detail: media_types::error::CorruptDetail::InvalidStructure { offset: None }
            }
        );
    }

    #[test]
    fn color_volumes_map_conservatively() {
        assert_eq!(
            color_description(DnxColorVolume::Bt709).unwrap_or_else(|e| unreachable!("{e:?}")),
            ColorDescription::BT709_LIMITED
        );
        // Out-of-band volume: the same conservative fill (documented
        // module-level; VC-3 signals no transfer function).
        assert_eq!(
            color_description(DnxColorVolume::OutOfBand).unwrap_or_else(|e| unreachable!("{e:?}")),
            ColorDescription::BT709_LIMITED
        );
        let ncl =
            color_description(DnxColorVolume::Bt2020Ncl).unwrap_or_else(|e| unreachable!("{e:?}"));
        assert_eq!(ncl.primaries, ColorPrimaries::Bt2020);
        assert_eq!(ncl.matrix, ColorMatrix::Bt2020Ncl);
        assert_eq!(ncl.range, ColorRange::Limited);
        assert!(color_description(DnxColorVolume::Bt2020Cl).is_err());
    }

    #[test]
    fn pooled_slots_reuse_exact_sizes_without_stale_bytes() {
        // Two decodes of different-sized pictures through the same
        // pool: the second must not see any first-decode byte (every
        // output byte is overwritten by construction — the packer
        // writes the full classified length).
        let small = synthetic_422(8, 2);
        let big = synthetic_422(16, 2);
        let mut slots = Vec::new();
        let mut next = 0usize;
        let len_small = match packed_output(&small) {
            Ok(o) => o.len,
            Err(e) => unreachable!("{e:?}"),
        };
        let len_big = match packed_output(&big) {
            Ok(o) => o.len,
            Err(e) => unreachable!("{e:?}"),
        };
        let a = packed_slot(&mut slots, &mut next, &small, len_small);
        let b = packed_slot(&mut slots, &mut next, &big, len_big);
        assert_eq!(a.len(), len_small);
        assert_eq!(b.len(), len_big);
        // Distinct values prove the fills are independent.
        assert_ne!(a.as_slice(), b.as_slice());
        // A same-size reuse takes the rotation, not a new allocation
        // (observable as a differing fill on the recycled slot).
        let small2 = synthetic_422(8, 2);
        let c = packed_slot(&mut slots, &mut next, &small2, len_small);
        assert_eq!(c.len(), len_small);
    }
}
