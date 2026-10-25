//! PGS palettes — the PDS 16-bit YCbCrA entries converted to
//! straight-alpha RGBA8 (card 5.N1).
//!
//! The reference decoder's rule (ffmpeg `pgssubdec.c`
//! `parse_palette_segment`, constants from `libavutil/colorspace.h`,
//! fetched 2026-10-25): BT.709 chroma coefficients when the video is
//! taller than 576 lines, BT.601 otherwise, both on the CCIR
//! studio-range integer core shared with the DVB CLUT path
//! (SCALEBITS=10, `FIX(x) = int(x·2^10 + 0.5)`):
//!
//! ```text
//! r_add = FIX(Kr * 255/224)·(Cr-128) + 512
//! g_add = -FIX(Kg_cb * 255/224)·(Cb-128) - FIX(Kg_cr * 255/224)·(Cr-128) + 512
//! b_add = FIX(Kb * 255/224)·(Cb-128) + 512
//! r     = clamp(((Y-16)·FIX(255/219) + r_add) >> 10)
//! ```
//!
//! Unlike DVB, the PGS alpha byte is straight (`0` = transparent,
//! `255` = opaque) and there is no "Y==0 forces transparency" rule.
//! PGS palettes are full per-epoch redefinitions: a PDS with the
//! same id+version is a no-op (unchanged); a new version REPLACES
//! every entry (the unmentioned ones revert to transparent black,
//! the reference `av_mallocz` behavior).

use crate::compositing::target::Rgba;

/// Which chroma matrix a palette converts with (video-height rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMatrix {
    /// BT.601 (video height <= 576).
    Bt601,
    /// BT.709 (video height > 576 — 720p/1080p Blu-ray content).
    Bt709,
}

impl ColorMatrix {
    /// The reference rule for a video height.
    #[must_use = "the palette conversion depends on it"]
    pub fn for_video_height(height: u32) -> ColorMatrix {
        if height > 576 {
            ColorMatrix::Bt709
        } else {
            ColorMatrix::Bt601
        }
    }
}

// FIX() constants (SCALEBITS = 10, ONE_HALF = 512). BT.601 column
// identical to `dvb/clut.rs` (same ffmpeg macros); BT.709 column
// from colorspace.h's YUV_TO_RGB1_CCIR_BT709.
const FIX_R_CR_601: i32 = 1634; // FIX(1.40200 * 255/224)
const FIX_G_CB_601: i32 = 401; // FIX(0.34414 * 255/224)
const FIX_G_CR_601: i32 = 832; // FIX(0.71414 * 255/224)
const FIX_B_CB_601: i32 = 2066; // FIX(1.77200 * 255/224)
const FIX_R_CR_709: i32 = 1836; // FIX(1.5747 * 255/224)
const FIX_G_CB_709: i32 = 218; // FIX(0.1873 * 255/224)
const FIX_G_CR_709: i32 = 546; // FIX(0.4682 * 255/224)
const FIX_B_CB_709: i32 = 2163; // FIX(1.8556 * 255/224)
const FIX_Y: i32 = 1192; // FIX(255/219)
const ONE_HALF: i32 = 512;

#[inline]
fn clamp255(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

/// Studio-range YCbCr + straight alpha → RGBA8 under a matrix.
pub(crate) fn ycbcr_to_rgba(y: u8, cb: u8, cr: u8, alpha: u8, matrix: ColorMatrix) -> Rgba {
    let cb = i32::from(cb) - 128;
    let cr = i32::from(cr) - 128;
    let (r_add, g_add, b_add) = match matrix {
        ColorMatrix::Bt601 => (
            FIX_R_CR_601 * cr + ONE_HALF,
            -FIX_G_CB_601 * cb - FIX_G_CR_601 * cr + ONE_HALF,
            FIX_B_CB_601 * cb + ONE_HALF,
        ),
        ColorMatrix::Bt709 => (
            FIX_R_CR_709 * cr + ONE_HALF,
            -FIX_G_CB_709 * cb - FIX_G_CR_709 * cr + ONE_HALF,
            FIX_B_CB_709 * cb + ONE_HALF,
        ),
    };
    let luma = (i32::from(y) - 16) * FIX_Y;
    Rgba {
        r: clamp255((luma + r_add) >> 10),
        g: clamp255((luma + g_add) >> 10),
        b: clamp255((luma + b_add) >> 10),
        a: alpha,
    }
}

/// One PGS palette: 256 straight-alpha RGBA8 entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    pub id: u8,
    pub version: u8,
    pub rgba: [Rgba; 256],
}

impl Palette {
    /// A fresh palette from a PDS (all unmentioned entries transparent
    /// black — the reference's zero-filled CLUT).
    pub fn from_entries(
        id: u8,
        version: u8,
        entries: impl Iterator<Item = crate::pgs::segment::PaletteEntry>,
        matrix: ColorMatrix,
    ) -> Palette {
        let mut palette = Palette {
            id,
            version,
            rgba: [crate::pgs::TRANSPARENT_BLACK; 256],
        };
        for e in entries {
            palette.rgba[e.id as usize] = ycbcr_to_rgba(e.y, e.cb, e.cr, e.alpha, matrix);
        }
        palette
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn studio_white_and_black_round_trip_601() {
        // Y=235, Cb=Cr=128: opaque studio white.
        let c = ycbcr_to_rgba(235, 128, 128, 255, ColorMatrix::Bt601);
        assert_eq!((c.r, c.g, c.b, c.a), (255, 255, 255, 255));
        // Y=16: opaque studio black.
        let c = ycbcr_to_rgba(16, 128, 128, 255, ColorMatrix::Bt601);
        assert_eq!((c.r, c.g, c.b, c.a), (0, 0, 0, 255));
        // Alpha is straight (unlike DVB's inverted T).
        let c = ycbcr_to_rgba(235, 128, 128, 0, ColorMatrix::Bt601);
        assert_eq!(c.a, 0);
    }

    #[test]
    fn bt709_primaries_are_plausible() {
        // Studio red: Y=81 Cb=90 Cr=240.
        let c = ycbcr_to_rgba(81, 90, 240, 255, ColorMatrix::Bt709);
        assert!(c.r > 200 && c.g < 60 && c.b < 60, "red, got {c:?}");
        // Blue: Y=41 Cb=240 Cr=110.
        let c = ycbcr_to_rgba(41, 240, 110, 255, ColorMatrix::Bt709);
        assert!(c.b > 200 && c.r < 60, "blue, got {c:?}");
    }

    #[test]
    fn matrices_diverge_exactly_as_the_constants_say() {
        // A chroma-heavy color lands on different RGB under 601/709.
        let c601 = ycbcr_to_rgba(128, 60, 200, 255, ColorMatrix::Bt601);
        let c709 = ycbcr_to_rgba(128, 60, 200, 255, ColorMatrix::Bt709);
        assert_ne!(c601, c709);
        assert_eq!(ColorMatrix::for_video_height(576), ColorMatrix::Bt601);
        assert_eq!(ColorMatrix::for_video_height(577), ColorMatrix::Bt709);
        assert_eq!(ColorMatrix::for_video_height(1080), ColorMatrix::Bt709);
    }

    #[test]
    fn palette_from_entries_replaces_and_defaults() {
        use crate::pgs::segment::PaletteEntry;
        let e = PaletteEntry {
            id: 0x05,
            y: 235,
            cr: 128,
            cb: 128,
            alpha: 255,
        };
        let p = Palette::from_entries(1, 0, std::iter::once(e), ColorMatrix::Bt709);
        assert_eq!(
            p.rgba[5],
            Rgba {
                r: 255,
                g: 255,
                b: 255,
                a: 255
            }
        );
        // Untouched entries: transparent black.
        assert_eq!(p.rgba[0], Rgba::BLACK.with_alpha(0));
        assert_eq!(p.rgba[255], Rgba::BLACK.with_alpha(0));
    }
}
