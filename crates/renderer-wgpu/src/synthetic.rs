//! Deterministic synthetic frames (doc 03 §4.11 f "synthetic
//! textures"; the card 2.D1 AC generator).
//!
//! The pattern is **fully specified in code and free of time, entropy
//! and environment**: two renderers on two machines (or the same
//! machine twice) produce byte-identical frames — which is what makes
//! a committed PNG hash an acceptance proof rather than a flake.
//!
//! # Pattern specification (per pixel, in override order)
//!
//! 1. Base: a two-axis linear gradient —
//!    `r = round(x · 255 / (w−1))` (clamped), `g = round(y · 255 /
//!    (h−1))`, `b = (x + y) mod 256`, `a = 255`; a single-pixel-wide
//!    axis maps the gradient to the midpoint 128.
//! 2. Corner markers: 4×4 opaque red blocks at the top-left and
//!    bottom-right corners (visual anchoring for humans; a fixed
//!    reference for tests).
//! 3. Grid: every 16th column and row is opaque white.

use media_types::frame::{ColorDescription, FrameStorage, VideoFrame};
use media_types::packet::SharedBytes;
use media_types::time::{Stamp, Timestamp};
use media_types::{GenerationId, PixelFormat, frame};

use crate::params::Extent2D;

/// The fully-specified synthetic RGBA8 pattern (module docs) as raw
/// bytes — `extent.width * extent.height * 4`, row-major.
#[must_use = "pure generator"]
pub fn synthetic_pattern(extent: Extent2D) -> Vec<u8> {
    let w = extent.width;
    let h = extent.height;
    let mut out = Vec::with_capacity((w as usize) * (h as usize) * 4);
    let grad = |i: u32, n: u32| -> u8 {
        if n <= 1 {
            128
        } else {
            ((u64::from(i) * 255 + u64::from(n - 1) / 2) / u64::from(n - 1)) as u8
        }
    };
    for y in 0..h {
        for x in 0..w {
            let pixel = if (x < 4 && y < 4) || (x + 4 >= w && y + 4 >= h) {
                [255, 0, 0, 255]
            } else if x.is_multiple_of(16) || y.is_multiple_of(16) {
                [255, 255, 255, 255]
            } else {
                [grad(x, w), grad(y, h), ((x + y) % 256) as u8, 255]
            };
            out.extend_from_slice(&pixel);
        }
    }
    out
}

/// The synthetic pattern wrapped as a presentation-ready [`VideoFrame`]
/// (Rgba8, PTS zero, first generation, sRGB/full-range color).
#[must_use = "pure generator"]
pub fn synthetic_frame(extent: Extent2D) -> VideoFrame {
    VideoFrame {
        format: PixelFormat::Rgba8,
        size: (extent.width, extent.height),
        data: FrameStorage::Cpu(SharedBytes::from_vec(synthetic_pattern(extent))),
        pts: Stamp::Present(Timestamp::new(0, video_engine::NANO_BASE)),
        color: ColorDescription {
            primaries: frame::ColorPrimaries::Bt709,
            transfer: frame::TransferCharacteristics::Srgb,
            matrix: frame::ColorMatrix::Bt709,
            range: frame::ColorRange::Full,
        },
        interlaced: false,
        generation: GenerationId::FIRST,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern_pixel(extent: Extent2D, x: u32, y: u32) -> [u8; 4] {
        let pattern = synthetic_pattern(extent);
        let stride = extent.width as usize * 4;
        let at = y as usize * stride + x as usize * 4;
        [
            pattern[at],
            pattern[at + 1],
            pattern[at + 2],
            pattern[at + 3],
        ]
    }

    #[test]
    fn pattern_is_deterministic_and_sized() {
        let extent = Extent2D {
            width: 64,
            height: 48,
        };
        assert_eq!(synthetic_pattern(extent), synthetic_pattern(extent));
        assert_eq!(synthetic_pattern(extent).len(), 64 * 48 * 4);
    }

    #[test]
    fn pattern_matches_its_specification() {
        let extent = Extent2D {
            width: 64,
            height: 48,
        };
        // Gradient base away from markers and grid: (5, 5) — x=5 not
        // multiple of 16, y=5 neither; not in corner blocks.
        // r = round(5·255/63) = 20, g = round(5·255/47) = 27, b = 10.
        let base = pattern_pixel(extent, 5, 5);
        assert_eq!(base, [20, 27, 10, 255]);
        // Corner marker at (0,0) and (63,47).
        assert_eq!(pattern_pixel(extent, 0, 0), [255, 0, 0, 255]);
        assert_eq!(pattern_pixel(extent, 63, 47), [255, 0, 0, 255]);
        // Grid lines: x=16 column, y=32 row.
        assert_eq!(pattern_pixel(extent, 16, 5), [255, 255, 255, 255]);
        assert_eq!(pattern_pixel(extent, 5, 32), [255, 255, 255, 255]);
        // The gradient hits its extremes just off the grid/marker:
        // (17, 1) → r = round(17·255/63) = 69, g = round(1·255/47) = 5.
        assert_eq!(pattern_pixel(extent, 17, 1), [69, 5, 18, 255]);
    }

    #[test]
    fn degenerate_axes_map_to_the_midpoint() {
        let extent = Extent2D {
            width: 1,
            height: 1,
        };
        // Everything is the corner marker on a 1x1.
        assert_eq!(pattern_pixel(extent, 0, 0), [255, 0, 0, 255]);
        // 9x2: (4,1) escapes both corner blocks and the grid.
        // r = round(4·255/8) = 128, g = round(1·255/1) = 255, b = 5.
        let extent = Extent2D {
            width: 9,
            height: 2,
        };
        assert_eq!(pattern_pixel(extent, 4, 1), [128, 255, 5, 255]);
        // 9x3 at (4,1): same corner/grid escape, h=3 gradient →
        // g = round(1·255/2) = 128.
        let extent = Extent2D {
            width: 9,
            height: 3,
        };
        assert_eq!(pattern_pixel(extent, 4, 1), [128, 128, 5, 255]);
    }

    #[test]
    fn frame_wraps_the_pattern_with_presentation_metadata() {
        let frame = synthetic_frame(Extent2D {
            width: 8,
            height: 8,
        });
        assert_eq!(frame.format, PixelFormat::Rgba8);
        assert_eq!(frame.size, (8, 8));
        assert_eq!(
            frame.pts,
            Stamp::Present(Timestamp::new(0, video_engine::NANO_BASE))
        );
        assert_eq!(frame.generation, GenerationId::FIRST);
        assert!(!frame.interlaced);
        assert_eq!(frame.color.transfer, frame::TransferCharacteristics::Srgb);
        match &frame.data {
            FrameStorage::Cpu(bytes) => assert_eq!(bytes.len(), 8 * 8 * 4),
            FrameStorage::OpaqueHandle(_) => unreachable!("synthetic frames are CPU frames"),
        }
    }
}
