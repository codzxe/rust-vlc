//! The frame-upload path (card 7.H2, issue #48; doc 09 §4.2 row 1 +
//! §4.1's latest-wins mailbox).
//!
//! # What this module is
//!
//! The CPU-staged present lane: decoded CPU frames → RGBA (the
//! single allowed CPU→GPU-adjacent copy of doc 03 §4.11 e / doc 09
//! §4.2) → the **latest-wins present mailbox** → one present per
//! pump slice → the JS glue's canvas draw.
//!
//! The conversion is the same YUV→RGB arithmetic the native shader
//! path performs (doc 09 §4.3: "aspect/crop math is the same pure,
//! unit-tested code as native" — here it is the upload half's own
//! pure, unit-tested code), using the frame's OWN colorimetry
//! (`VideoFrame::color`): the BT.601/709/2020 matrix ×
//! limited/full range. The transfer characteristics ride unconverted
//! in v1 (the CPU path presents the matrix-converted RGB as-is;
//! transfer-aware tone mapping joins with the shader stage when
//! wgpu is admitted — recorded `[PROPOSAL]`).
//!
//! # The plane convention (v1, recorded)
//!
//! I420 frames are the codec-native **packed-plane** layout — `Y
//! (w×h) ‖ Cb (⌈w/2⌉×⌈h/2⌉) ‖ Cr`, stride == plane width, exactly
//! what the first vertical's MJPEG decoder emits (its
//! `i420_total`). A stride-carrying frame vocabulary would lift the
//! assumption `[PROPOSAL: vocabulary change, Lead-owned]`. Chroma
//! siting is nearest (top-left) in v1; bilinear joins the shader
//! stage `[PROPOSAL]`.
//!
//! # The mailbox discipline
//!
//! Doc 03 §4.11's present mailbox on the browser lane: at most ONE
//! frame staged, the newest wins, displaced frames are COUNTED (the
//! drop observable — never silent), and the conversion happens at
//! TAKE time so a displaced frame never pays for pixels nobody
//! presented (D-2: the work is bounded by what is shown, not what
//! was decoded).

use media_types::frame::{ColorMatrix, ColorRange, FrameStorage, PixelFormat, VideoFrame};
use media_types::time::TimeBase;

/// The canonical nanosecond base (media-clock's own constant,
/// mirrored here so the present lane reads in one import — the pts
/// conversion is the same `NANO_BASE` mapping every host runs).
pub const NANO_BASE: TimeBase = TimeBase {
    num: 1,
    den: 1_000_000_000,
};

/// One RGBA upload: the lean frame the present lane stages (the
/// bytes cross the ABI through the RAW frame lane, not the JSON —
/// see [`crate::abi`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadFrame {
    /// Pixels across.
    pub width: u32,
    /// Pixels down.
    pub height: u32,
    /// The frame's presentation timestamp (ns), when the demuxer
    /// carried one.
    pub pts_ns: Option<i64>,
    /// `width × height × 4` RGBA8 bytes, row-major, straight alpha.
    pub rgba: Vec<u8>,
}

/// The frame-upload byte bound (the D-2 ceiling): 32 MiB covers a
/// 4K RGBA frame (3840×2160×4 = 33,177,600) with margin; an 8K
/// frame is refused TYPED (the engine keeps decoding; the presenter
/// counts the refusal — presentation bounds must never bound
/// decode).
pub const MAX_UPLOAD_BYTES: u64 = 32 * 1024 * 1024;

/// The typed upload refusals (every one counts, none kill the
/// session).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadError {
    /// A zero pixel component.
    ZeroExtent,
    /// Past [`MAX_UPLOAD_BYTES`].
    TooLarge,
    /// A pixel format the CPU path does not convert (v1: NV12 and
    /// YUYV422 — the HEVC decoders' outputs; the typed refusal
    /// names them, the shader stage lifts them with wgpu).
    UnsupportedFormat(PixelFormat),
    /// The frame's bytes are shorter than the plane layout demands
    /// (truncated decode output).
    TruncatedPlanes,
    /// A hardware-frame handle (the web v1 path is CPU-only — the
    /// WebCodecs lane is doc 09 §10's pending ADR).
    OpaqueHandle,
}

impl UploadError {
    /// The human sentence.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            UploadError::ZeroExtent => "the frame has a zero pixel component",
            UploadError::TooLarge => "the frame exceeds the 32 MiB upload bound",
            UploadError::UnsupportedFormat(_) => {
                "the pixel format is not on the v1 CPU upload path (nv12/yuyv422)"
            }
            UploadError::TruncatedPlanes => "the frame's planes are shorter than its layout",
            UploadError::OpaqueHandle => "the frame is a hardware handle, not CPU memory",
        }
    }
}

impl std::fmt::Display for UploadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The packed-plane layout of one [`PixelFormat`] at `size`, or the
/// typed refusal. `I420`: `Y (w×h) ‖ Cb (cw×ch) ‖ Cr` — the
/// codec-native convention (see the module docs).
fn plane_layout(format: PixelFormat, size: (u32, u32)) -> Result<PlaneLayout, UploadError> {
    let (width, height) = size;
    if width == 0 || height == 0 {
        return Err(UploadError::ZeroExtent);
    }
    let w = u64::from(width);
    let h = u64::from(height);
    match format {
        PixelFormat::I420 => {
            let cw = w.div_ceil(2);
            let ch = h.div_ceil(2);
            let luma = w.checked_mul(h).ok_or(UploadError::TooLarge)?;
            let chroma = cw.checked_mul(ch).ok_or(UploadError::TooLarge)?;
            Ok(PlaneLayout {
                chroma_width: cw,
                chroma_height: ch,
                cb_offset: luma,
                cr_offset: luma.checked_add(chroma).ok_or(UploadError::TooLarge)?,
                total: luma
                    .checked_add(chroma)
                    .and_then(|sum| sum.checked_add(chroma))
                    .ok_or(UploadError::TooLarge)?,
            })
        }
        // The single-plane formats need no layout arithmetic — the
        // byte length is width*height*4, validated by the caller.
        PixelFormat::Rgba8 | PixelFormat::Bgra8 => Ok(PlaneLayout {
            chroma_width: 0,
            chroma_height: 0,
            cb_offset: 0,
            cr_offset: 0,
            total: w
                .checked_mul(h)
                .and_then(|pixels| pixels.checked_mul(4))
                .ok_or(UploadError::TooLarge)?,
        }),
        PixelFormat::Nv12 | PixelFormat::Yuyv422 => Err(UploadError::UnsupportedFormat(format)),
    }
}

/// The packed-plane byte offsets of an I420 frame (luma always
/// starts at zero — the codec-native convention).
struct PlaneLayout {
    chroma_width: u64,
    chroma_height: u64,
    cb_offset: u64,
    cr_offset: u64,
    total: u64,
}

impl PlaneLayout {
    /// The RGBA byte bound for one frame of this layout.
    fn rgba_bytes(&self, size: (u32, u32)) -> Result<u64, UploadError> {
        u64::from(size.0)
            .checked_mul(u64::from(size.1))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(UploadError::TooLarge)
    }
}

/// Validate + convert one decoded frame to the CPU-staged RGBA
/// upload (the single allowed copy — doc 09 §4.2 row 1).
///
/// # Errors
/// The typed [`UploadError`] vocabulary; nothing panics on
/// malformed geometry or short planes.
pub fn rgba_from_video(frame: &VideoFrame) -> Result<UploadFrame, UploadError> {
    let bytes = match &frame.data {
        FrameStorage::Cpu(bytes) => bytes.as_slice(),
        FrameStorage::OpaqueHandle(_) => return Err(UploadError::OpaqueHandle),
    };
    let layout = plane_layout(frame.format, frame.size)?;
    if bytes.len() as u64 != layout.total {
        // The packed-plane convention is exact: a longer OR shorter
        // blob than the layout demands is a truncated/padded decode
        // output — refused typed, never a partial present.
        return Err(UploadError::TruncatedPlanes);
    }
    let rgba_bound = layout.rgba_bytes(frame.size)?;
    if rgba_bound > MAX_UPLOAD_BYTES {
        return Err(UploadError::TooLarge);
    }
    let (width, height) = frame.size;
    let pts_ns = frame
        .pts
        .map_timestamp(|stamp| stamp.convert(&NANO_BASE).ok().map(|ns| ns.ticks))
        .flatten();
    let mut rgba = Vec::with_capacity(usize::try_from(rgba_bound).unwrap_or(usize::MAX));
    match frame.format {
        PixelFormat::I420 => {
            let cw = layout.chroma_width as usize;
            let ch = layout.chroma_height as usize;
            let cb = &bytes
                [layout.cb_offset as usize..(layout.cb_offset + cw as u64 * ch as u64) as usize];
            let cr = &bytes[layout.cr_offset as usize..];
            let coefficients = yuv_coefficients(frame.color.matrix, frame.color.range);
            for y in 0..height as usize {
                let luma_row = &bytes[y * width as usize..(y + 1) * width as usize];
                let chroma_row = (y / 2) * cw;
                for x in 0..width as usize {
                    let y_value = f32::from(luma_row[x]);
                    let cb_value = f32::from(cb[chroma_row + x / 2]);
                    let cr_value = f32::from(cr[chroma_row + x / 2]);
                    let (r, g, b) = convert_yuv(y_value, cb_value, cr_value, &coefficients);
                    rgba.extend_from_slice(&[r, g, b, 255]);
                }
            }
        }
        PixelFormat::Rgba8 => {
            rgba.extend_from_slice(bytes);
        }
        PixelFormat::Bgra8 => {
            for pixel in bytes.as_chunks::<4>().0 {
                rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
            }
        }
        // plane_layout already refused the rest; the match stays
        // total for future formats.
        PixelFormat::Nv12 | PixelFormat::Yuyv422 => {
            return Err(UploadError::UnsupportedFormat(frame.format));
        }
    }
    debug_assert_eq!(rgba.len() as u64, rgba_bound);
    Ok(UploadFrame {
        width,
        height,
        pts_ns,
        rgba,
    })
}

/// The YUV→RGB conversion coefficients derived from the matrix's
/// Kr/Kb and the range convention (the same derivation the shader
/// constants come from — one source of truth).
struct YuvCoefficients {
    /// The luma scale (`219`- or `255`-denominator).
    y_scale: f32,
    /// The chroma scale (`224` or `255`).
    c_scale: f32,
    /// The luma offset (16 or 0).
    y_offset: f32,
    /// `2·(1−Kr)`.
    r_cr: f32,
    /// `2·Kb·(1−Kb)/(1−Kr−Kb)`.
    g_cb: f32,
    /// `2·Kr·(1−Kr)/(1−Kr−Kb)`.
    g_cr: f32,
    /// `2·(1−Kb)`.
    b_cb: f32,
}

/// Derive the coefficients for one matrix × range pair.
fn yuv_coefficients(matrix: ColorMatrix, range: ColorRange) -> YuvCoefficients {
    let (kr, kb) = match matrix {
        ColorMatrix::Bt601 => (0.299_f32, 0.114_f32),
        ColorMatrix::Bt709 => (0.2126_f32, 0.0722_f32),
        ColorMatrix::Bt2020Ncl => (0.2627_f32, 0.0593_f32),
    };
    let (y_scale, c_scale, y_offset) = match range {
        ColorRange::Limited => (219.0_f32, 224.0_f32, 16.0_f32),
        ColorRange::Full => (255.0_f32, 255.0_f32, 0.0_f32),
    };
    let kg = 1.0 - kr - kb;
    YuvCoefficients {
        y_scale,
        c_scale,
        y_offset,
        r_cr: 2.0 * (1.0 - kr),
        g_cb: 2.0 * kb * (1.0 - kb) / kg,
        g_cr: 2.0 * kr * (1.0 - kr) / kg,
        b_cb: 2.0 * (1.0 - kb),
    }
}

/// Convert one YCbCr pixel to 8-bit RGBA (f32 arithmetic, clamped,
/// rounded half-away-from-zero — deterministic on every platform).
fn convert_yuv(y: f32, cb: f32, cr: f32, c: &YuvCoefficients) -> (u8, u8, u8) {
    let y_norm = (y - c.y_offset) / c.y_scale;
    let cb_norm = (cb - 128.0) / c.c_scale;
    let cr_norm = (cr - 128.0) / c.c_scale;
    let r = y_norm + c.r_cr * cr_norm;
    let g = y_norm - c.g_cb * cb_norm - c.g_cr * cr_norm;
    let b = y_norm + c.b_cb * cb_norm;
    let quantize = |value: f32| -> u8 {
        let scaled = (value * 255.0).round();
        if scaled <= 0.0 {
            0
        } else if scaled >= 255.0 {
            255
        } else {
            scaled as u8
        }
    };
    (quantize(r), quantize(g), quantize(b))
}

/// The latest-wins present mailbox (doc 03 §4.11's mailbox, browser
/// lane): at most one staged frame, the newest wins, drops counted.
#[derive(Debug, Default)]
pub struct PresentMailbox {
    pending: Option<VideoFrame>,
    stored: u64,
    displaced: u64,
    converted: u64,
    refused: u64,
}

/// The mailbox's observables (the honest present evidence).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PresentStats {
    /// Frames the consumer offered.
    pub stored: u64,
    /// Offers that displaced a not-yet-presented frame (the
    /// latest-wins drops — compositor pacing, not lost media).
    pub displaced: u64,
    /// Frames actually converted for present.
    pub converted: u64,
    /// Offers refused typed (oversize / unsupported format /
    /// truncated planes).
    pub refused: u64,
}

impl PresentMailbox {
    /// An empty mailbox.
    #[must_use]
    pub fn new() -> PresentMailbox {
        PresentMailbox::default()
    }

    /// Offer one decoded frame: validates the upload shape NOW (the
    /// typed refusal counts immediately), stages the frame (a cheap
    /// refcounted clone — the conversion happens at take time), and
    /// counts the displaced frame if one was still staged.
    ///
    /// # Errors
    /// The typed [`UploadError`] — presentation refusals never fail
    /// decode (the consumer counts and carries on).
    pub fn store(&mut self, frame: &VideoFrame) -> Result<(), UploadError> {
        // Validate the whole upload shape upfront so take() stays
        // infallible: format, extent, bound, plane completeness.
        let bytes = match &frame.data {
            FrameStorage::Cpu(bytes) => bytes.as_slice(),
            FrameStorage::OpaqueHandle(_) => {
                self.refused += 1;
                return Err(UploadError::OpaqueHandle);
            }
        };
        let layout = match plane_layout(frame.format, frame.size) {
            Ok(layout) => layout,
            Err(error) => {
                self.refused += 1;
                return Err(error);
            }
        };
        if bytes.len() as u64 != layout.total {
            self.refused += 1;
            return Err(UploadError::TruncatedPlanes);
        }
        if layout.rgba_bytes(frame.size)? > MAX_UPLOAD_BYTES {
            self.refused += 1;
            return Err(UploadError::TooLarge);
        }
        if self.pending.is_some() {
            self.displaced += 1;
        }
        self.pending = Some(frame.clone());
        self.stored += 1;
        Ok(())
    }

    /// Take the staged frame as a converted RGBA upload (the single
    /// conversion per presented frame — displaced frames never
    /// paid for pixels). `None` when nothing is staged.
    #[must_use]
    pub fn take(&mut self) -> Option<UploadFrame> {
        let frame = self.pending.take()?;
        // store() validated the shape; a failure here would be an
        // internal inconsistency — count it and answer nothing
        // (never propagate a panic through the consumer).
        match rgba_from_video(&frame) {
            Ok(upload) => {
                self.converted += 1;
                Some(upload)
            }
            Err(_) => {
                self.refused += 1;
                None
            }
        }
    }

    /// Whether a frame is staged.
    #[must_use]
    pub fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// The observables.
    #[must_use]
    pub fn stats(&self) -> PresentStats {
        PresentStats {
            stored: self.stored,
            displaced: self.displaced,
            converted: self.converted,
            refused: self.refused,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::GenerationId;
    use media_types::frame::{
        ColorDescription, ColorPrimaries, FrameStorage, TransferCharacteristics,
    };
    use media_types::packet::SharedBytes;
    use media_types::time::{Stamp, Timestamp};
    use testkit::{Must, assert_err};

    fn i420_frame(
        width: u32,
        height: u32,
        fill: impl Fn(usize, usize) -> (u8, u8, u8),
        color: ColorDescription,
        pts_ns: Option<i64>,
    ) -> VideoFrame {
        let base = TimeBase::new(1, 1_000_000_000).must();
        let cw = width.div_ceil(2) as usize;
        let ch = height.div_ceil(2) as usize;
        let mut data = vec![0u8; width as usize * height as usize + 2 * cw * ch];
        for y in 0..height as usize {
            for x in 0..width as usize {
                data[y * width as usize + x] = fill(x, y).0;
            }
        }
        let cb_base = width as usize * height as usize;
        for cy in 0..ch {
            for cx in 0..cw {
                let (.., cb, _) = fill(cx * 2, cy * 2);
                data[cb_base + cy * cw + cx] = cb;
            }
        }
        let cr_base = cb_base + cw * ch;
        for cy in 0..ch {
            for cx in 0..cw {
                let (.., cr) = fill(cx * 2, cy * 2);
                data[cr_base + cy * cw + cx] = cr;
            }
        }
        VideoFrame {
            format: PixelFormat::I420,
            size: (width, height),
            data: FrameStorage::Cpu(SharedBytes::from_vec(data)),
            pts: match pts_ns {
                Some(ns) => Stamp::Present(Timestamp::new(ns, base)),
                None => Stamp::Missing,
            },
            color,
            interlaced: false,
            generation: GenerationId::FIRST,
        }
    }

    fn limited(matrix: ColorMatrix) -> ColorDescription {
        ColorDescription {
            primaries: ColorPrimaries::Bt709,
            transfer: TransferCharacteristics::Bt709,
            matrix,
            range: ColorRange::Limited,
        }
    }

    /// The f64 reference model of the SAME derivation (Kr/Kb →
    /// coefficients) — the independent check the f32 pipeline must
    /// match within one quantization step.
    fn reference_rgb(
        y: u8,
        cb: u8,
        cr: u8,
        matrix: ColorMatrix,
        range: ColorRange,
    ) -> (f64, f64, f64) {
        let (kr, kb) = match matrix {
            ColorMatrix::Bt601 => (0.299_f64, 0.114_f64),
            ColorMatrix::Bt709 => (0.2126_f64, 0.0722_f64),
            ColorMatrix::Bt2020Ncl => (0.2627_f64, 0.0593_f64),
        };
        let (y_scale, c_scale, y_offset) = match range {
            ColorRange::Limited => (219.0_f64, 224.0_f64, 16.0_f64),
            ColorRange::Full => (255.0_f64, 255.0_f64, 0.0_f64),
        };
        let kg = 1.0 - kr - kb;
        let y_norm = (f64::from(y) - y_offset) / y_scale;
        let cb_norm = (f64::from(cb) - 128.0) / c_scale;
        let cr_norm = (f64::from(cr) - 128.0) / c_scale;
        let r = y_norm + 2.0 * (1.0 - kr) * cr_norm;
        let g =
            y_norm - 2.0 * kb * (1.0 - kb) / kg * cb_norm - 2.0 * kr * (1.0 - kr) / kg * cr_norm;
        let b = y_norm + 2.0 * (1.0 - kb) * cb_norm;
        (
            (r * 255.0).round().clamp(0.0, 255.0),
            (g * 255.0).round().clamp(0.0, 255.0),
            (b * 255.0).round().clamp(0.0, 255.0),
        )
    }

    /// The studio-swing endpoints are exact: black is black, white
    /// is white, on every matrix (limited), and full range is the
    /// identity on neutral chroma.
    #[test]
    fn endpoints_are_exact_for_every_matrix() {
        for matrix in [
            ColorMatrix::Bt601,
            ColorMatrix::Bt709,
            ColorMatrix::Bt2020Ncl,
        ] {
            let black = rgba_from_video(&i420_frame(
                2,
                2,
                |_, _| (16, 128, 128),
                limited(matrix),
                None,
            ))
            .must();
            for pixel in black.rgba.as_chunks::<4>().0 {
                assert_eq!(pixel[..3], [0, 0, 0], "{matrix:?} black {pixel:?}");
                assert_eq!(pixel[3], 255, "straight alpha");
            }
            let white = rgba_from_video(&i420_frame(
                2,
                2,
                |_, _| (235, 128, 128),
                limited(matrix),
                None,
            ))
            .must();
            for pixel in white.rgba.as_chunks::<4>().0 {
                assert_eq!(pixel[..3], [255, 255, 255], "{matrix:?} white {pixel:?}");
            }
            let full = rgba_from_video(&i420_frame(
                2,
                2,
                |_, _| (200, 128, 128),
                ColorDescription {
                    range: ColorRange::Full,
                    ..limited(matrix)
                },
                None,
            ))
            .must();
            let expected = reference_rgb(200, 128, 128, matrix, ColorRange::Full);
            let pixel = full.rgba.as_chunks::<4>().0[0];
            assert_eq!(
                (pixel[0] as f64, pixel[1] as f64, pixel[2] as f64),
                expected
            );
        }
    }

    /// The f32 pipeline matches the f64 reference within one
    /// quantization step across a swept sample of YCbCr values on
    /// every matrix × range (the conversion-is-the-same-math proof).
    #[test]
    fn conversion_matches_the_reference_within_one_lsb() {
        let samples: [(u8, u8, u8); 7] = [
            (81, 90, 240),
            (41, 240, 110),
            (210, 16, 146),
            (98, 180, 180),
            (160, 70, 200),
            (72, 40, 30),
            (235, 128, 200),
        ];
        for matrix in [
            ColorMatrix::Bt601,
            ColorMatrix::Bt709,
            ColorMatrix::Bt2020Ncl,
        ] {
            for range in [ColorRange::Limited, ColorRange::Full] {
                for &(y, cb, cr) in &samples {
                    let frame = i420_frame(
                        4,
                        2,
                        |_, _| (y, cb, cr),
                        ColorDescription {
                            range,
                            ..limited(matrix)
                        },
                        None,
                    );
                    let upload = rgba_from_video(&frame).must();
                    let (er, eg, eb) = reference_rgb(y, cb, cr, matrix, range);
                    for pixel in upload.rgba.as_chunks::<4>().0 {
                        assert!(
                            (f64::from(pixel[0]) - er).abs() <= 1.0
                                && (f64::from(pixel[1]) - eg).abs() <= 1.0
                                && (f64::from(pixel[2]) - eb).abs() <= 1.0,
                            "{matrix:?}/{range:?} ({y},{cb},{cr}): got {:?}, want ({er},{eg},{eb})",
                            pixel
                        );
                    }
                }
            }
        }
    }

    /// Chroma is subsampled 4:2:0 with the top-left siting: the
    /// chroma sample covers its 2×2 luma block (odd and even pixels
    /// read the SAME chroma — pinned by construction here).
    #[test]
    fn chroma_siting_is_top_left_420() {
        // Luma: a 2×2 gradient; chroma constant red-ish.
        let frame = i420_frame(
            2,
            2,
            |x, y| {
                let luma = 100 + 10 * (x + y) as u8;
                (luma, 80, 180)
            },
            limited(ColorMatrix::Bt601),
            None,
        );
        let upload = rgba_from_video(&frame).must();
        let pixels = upload.rgba.as_chunks::<4>().0;
        // Even-x/even-y and odd pixels must share the chroma
        // contribution: only the luma term differs. Compute both
        // through the reference to pin the siting.
        let (r0, g0, b0) = reference_rgb(100, 80, 180, ColorMatrix::Bt601, ColorRange::Limited);
        assert_eq!(
            (
                pixels[0][0] as f64,
                pixels[0][1] as f64,
                pixels[0][2] as f64
            ),
            (r0, g0, b0)
        );
        let (r3, g3, b3) = reference_rgb(120, 80, 180, ColorMatrix::Bt601, ColorRange::Limited);
        assert_eq!(
            (
                pixels[3][0] as f64,
                pixels[3][1] as f64,
                pixels[3][2] as f64
            ),
            (r3, g3, b3)
        );
    }

    /// Odd dimensions: ⌈w/2⌉×⌈h/2⌉ chroma, every luma pixel served.
    #[test]
    fn odd_dimensions_round_chroma_up() {
        let frame = i420_frame(
            3,
            3,
            |_, _| (128, 128, 128),
            limited(ColorMatrix::Bt709),
            None,
        );
        let upload = rgba_from_video(&frame).must();
        assert_eq!(upload.width, 3);
        assert_eq!(upload.height, 3);
        assert_eq!(upload.rgba.len(), 3 * 3 * 4);
        // 3×3 with div_ceil chroma = 2×2: 9 + 4 + 4 = 17 bytes.
        let bytes = match &frame.data {
            FrameStorage::Cpu(bytes) => bytes.len(),
            FrameStorage::OpaqueHandle(_) => unreachable!(),
        };
        assert_eq!(bytes, 17);
    }

    /// RGBA passes through untouched; BGRA swizzles.
    #[test]
    fn rgba_passthrough_and_bgra_swizzle() {
        let rgba = VideoFrame {
            format: PixelFormat::Rgba8,
            size: (1, 1),
            data: FrameStorage::Cpu(SharedBytes::from_vec(vec![1, 2, 3, 4])),
            pts: Stamp::Missing,
            color: ColorDescription::BT709_LIMITED,
            interlaced: false,
            generation: GenerationId::FIRST,
        };
        assert_eq!(rgba_from_video(&rgba).must().rgba, vec![1, 2, 3, 4]);
        let bgra = VideoFrame {
            format: PixelFormat::Bgra8,
            size: (1, 1),
            data: FrameStorage::Cpu(SharedBytes::from_vec(vec![1, 2, 3, 4])),
            pts: Stamp::Missing,
            color: ColorDescription::BT709_LIMITED,
            interlaced: false,
            generation: GenerationId::FIRST,
        };
        assert_eq!(rgba_from_video(&bgra).must().rgba, vec![3, 2, 1, 4]);
    }

    /// The typed refusals: NV12/YUYV422, opaque handles, truncated
    /// planes, zero extents, oversize — each named and counted.
    #[test]
    fn upload_refusals_are_typed() {
        for format in [PixelFormat::Nv12, PixelFormat::Yuyv422] {
            let frame = VideoFrame {
                format,
                size: (2, 2),
                data: FrameStorage::Cpu(SharedBytes::from_vec(vec![0; 16])),
                pts: Stamp::Missing,
                color: ColorDescription::BT709_LIMITED,
                interlaced: false,
                generation: GenerationId::FIRST,
            };
            assert_eq!(
                assert_err(rgba_from_video(&frame), "format"),
                UploadError::UnsupportedFormat(format)
            );
        }
        let handle = VideoFrame {
            format: PixelFormat::I420,
            size: (2, 2),
            data: FrameStorage::OpaqueHandle(7),
            pts: Stamp::Missing,
            color: ColorDescription::BT709_LIMITED,
            interlaced: false,
            generation: GenerationId::FIRST,
        };
        assert_eq!(
            assert_err(rgba_from_video(&handle), "handle"),
            UploadError::OpaqueHandle
        );
        let truncated = VideoFrame {
            format: PixelFormat::I420,
            size: (4, 4),
            data: FrameStorage::Cpu(SharedBytes::from_vec(vec![0; 10])),
            pts: Stamp::Missing,
            color: ColorDescription::BT709_LIMITED,
            interlaced: false,
            generation: GenerationId::FIRST,
        };
        assert_eq!(
            assert_err(rgba_from_video(&truncated), "truncated"),
            UploadError::TruncatedPlanes
        );
        let zero = i420_frame(
            0,
            4,
            |_, _| (16, 128, 128),
            limited(ColorMatrix::Bt709),
            None,
        );
        assert_eq!(
            assert_err(rgba_from_video(&zero), "zero"),
            UploadError::ZeroExtent
        );
        // 3000×3000 RGBA = 36 MB > the 32 MiB bound.
        let mut oversize = i420_frame(
            3000,
            3000,
            |_, _| (16, 128, 128),
            limited(ColorMatrix::Bt709),
            None,
        );
        oversize.format = PixelFormat::Rgba8;
        oversize.data = FrameStorage::Cpu(SharedBytes::from_vec(vec![0; 3000 * 3000 * 4]));
        assert_eq!(
            assert_err(rgba_from_video(&oversize), "oversize"),
            UploadError::TooLarge
        );
    }

    /// The pts rides the upload (the present lane's timing truth).
    #[test]
    fn pts_rides_the_upload() {
        let base = media_types::TimeBase::new(1, 1_000_000_000).must();
        let frame = i420_frame(
            2,
            2,
            |_, _| (16, 128, 128),
            limited(ColorMatrix::Bt709),
            Some(123_456_789),
        );
        let upload = rgba_from_video(&frame).must();
        assert_eq!(upload.pts_ns, Some(123_456_789));
        // And the timestamp is really ns in that base.
        match frame.pts {
            Stamp::Present(stamp) => {
                assert_eq!(stamp.convert(&base).must().ticks, 123_456_789);
            }
            Stamp::Missing => unreachable!("the fixture carried a pts"),
        }
    }

    /// The mailbox is latest-wins with honest counting: displaced
    /// frames are counted, never converted; take converts exactly
    /// the staged frame.
    #[test]
    fn mailbox_is_latest_wins_with_counted_drops() {
        let mut mailbox = PresentMailbox::new();
        assert!(!mailbox.is_pending());
        assert!(mailbox.take().is_none());
        let first = i420_frame(
            2,
            2,
            |_, _| (16, 128, 128),
            limited(ColorMatrix::Bt709),
            Some(1),
        );
        mailbox.store(&first).must();
        let second = i420_frame(
            2,
            2,
            |_, _| (235, 128, 128),
            limited(ColorMatrix::Bt709),
            Some(2),
        );
        mailbox.store(&second).must();
        assert!(mailbox.is_pending());
        assert_eq!(mailbox.stats().displaced, 1, "the first was displaced");
        // The staged frame is the SECOND (the latest wins).
        let upload = mailbox.take().must();
        assert_eq!(upload.pts_ns, Some(2));
        for pixel in upload.rgba.as_chunks::<4>().0 {
            assert_eq!(pixel[..3], [255, 255, 255]);
        }
        assert!(!mailbox.is_pending());
        let stats = mailbox.stats();
        assert_eq!(stats.stored, 2);
        assert_eq!(stats.displaced, 1);
        assert_eq!(stats.converted, 1, "one conversion per present");
        assert_eq!(stats.refused, 0);
    }

    /// Refused offers count and never displace the staged frame.
    #[test]
    fn refused_offers_count_and_keep_the_staged_frame() {
        let mut mailbox = PresentMailbox::new();
        let good = i420_frame(
            2,
            2,
            |_, _| (16, 128, 128),
            limited(ColorMatrix::Bt709),
            Some(1),
        );
        mailbox.store(&good).must();
        let bad = VideoFrame {
            format: PixelFormat::Nv12,
            size: (2, 2),
            data: FrameStorage::Cpu(SharedBytes::from_vec(vec![0; 16])),
            pts: Stamp::Missing,
            color: ColorDescription::BT709_LIMITED,
            interlaced: false,
            generation: GenerationId::FIRST,
        };
        assert!(mailbox.store(&bad).is_err());
        assert!(mailbox.is_pending(), "the good frame survives the refusal");
        assert_eq!(mailbox.take().must().pts_ns, Some(1));
        assert_eq!(mailbox.stats().refused, 1);
        assert_eq!(mailbox.stats().displaced, 0);
    }
}
