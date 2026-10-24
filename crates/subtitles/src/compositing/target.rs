//! The renderer seam (R4) and the reference blend.
//!
//! [`ComposeTarget`] is what the renderer implements — the ONE trait
//! this contract needs from the presentation world. It hands out
//! destination rows as straight-alpha RGBA8 slices; the blend math
//! lives here, implemented once. No video-engine or renderer-wgpu
//! edit is part of this card: a later adapter forwards the rows (the
//! same shape renderer-wgpu's software compositor already writes).
//!
//! # Blend semantics (pinned by tests, byte-exact)
//!
//! The destination is an OPAQUE video plane: each touched pixel
//! becomes `(a * s + (255 - a) * d + 127) / 255` per RGB channel with
//! alpha set to 255 (`a = 0` skips, `a = 255` copies). Rounding is to
//! nearest. Byte space, no gamma conversion — the point-sampling
//! honesty of the software path; linear-light blending belongs to the
//! GPU shader path. Over an opaque background the operation is
//! associative, so z-stacked cues compose identically one-by-one.

use crate::compositing::bitmap::CueError;
use crate::compositing::layout::Rect;
use media_types::time::TimeError;

/// A straight-alpha RGBA8 color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Alpha (255 = opaque).
    pub a: u8,
}

impl Rgba {
    /// Opaque black.
    pub const BLACK: Rgba = Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    /// Opaque white.
    pub const WHITE: Rgba = Rgba {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    /// Opaque red (the AC overlap fixtures).
    pub const RED: Rgba = Rgba {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    /// Opaque blue (the AC overlap fixtures).
    pub const BLUE: Rgba = Rgba {
        r: 0,
        g: 0,
        b: 255,
        a: 255,
    };

    /// The color with alpha `a`.
    #[must_use = "a computed color nobody reads is a test bug"]
    pub const fn with_alpha(mut self, a: u8) -> Rgba {
        self.a = a;
        self
    }
}

/// Typed compositing errors (MP §8 — no panics on pixel data).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposeError {
    /// The target or layout extent is empty.
    ZeroExtent {
        /// The offending extent.
        extent: (u32, u32),
    },
    /// A target row came back with the wrong stride (the trait
    /// contract is exactly `extent().0 * 4` bytes).
    RowStride {
        /// The row index.
        y: u32,
        /// The expected byte length.
        expected: usize,
        /// The returned byte length.
        got: usize,
    },
    /// The clock position could not be read as nanoseconds.
    Time(TimeError),
    /// A pub-field-constructed layout failed validation at use time.
    InvalidLayout {
        /// What is wrong.
        detail: &'static str,
        /// The construction error.
        source: CueError,
    },
}

impl std::fmt::Display for ComposeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComposeError::ZeroExtent { extent } => {
                write!(f, "compositing target extent is empty: {extent:?}")
            }
            ComposeError::RowStride { y, expected, got } => write!(
                f,
                "target row {y} stride mismatch: expected {expected} bytes, got {got}"
            ),
            ComposeError::Time(err) => write!(f, "clock position unreadable: {err}"),
            ComposeError::InvalidLayout { detail, .. } => {
                write!(f, "invalid cue layout: {detail}")
            }
        }
    }
}

impl std::error::Error for ComposeError {}

impl From<TimeError> for ComposeError {
    fn from(err: TimeError) -> ComposeError {
        ComposeError::Time(err)
    }
}

/// Where composited subtitle pixels go — the R4 seam (module docs).
///
/// Implementors return one destination row per call: `extent().0 * 4`
/// bytes of straight-alpha RGBA8. The compositor calls this at most
/// once per touched row per frame; everything between rows is
/// arithmetic owned here.
pub trait ComposeTarget {
    /// The target's pixel extent.
    fn extent(&self) -> (u32, u32);
    /// Destination row `y` as mutable RGBA8. Called with `y` strictly
    /// below `extent().1`.
    ///
    /// # Errors
    /// The implementor's protocol errors; the slice length is checked
    /// against the trait contract and surfaces as
    /// [`ComposeError::RowStride`].
    fn row_rgba(&mut self, y: u32) -> Result<&mut [u8], ComposeError>;
}

/// The reference [`ComposeTarget`]: an in-memory RGBA8 surface —
/// the test harness, and the shape a CPU renderer forwards.
///
/// Allocation policy: one `Vec` at construction, reused for the
/// surface's lifetime — the zero-alloc steady-state proof composes
/// onto this.
pub struct Rgba8Surface {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Rgba8Surface {
    /// A surface of `extent` filled with `fill`.
    ///
    /// # Errors
    /// [`ComposeError::ZeroExtent`] when a dimension is zero.
    pub fn new(extent: (u32, u32), fill: Rgba) -> Result<Rgba8Surface, ComposeError> {
        if extent.0 == 0 || extent.1 == 0 {
            return Err(ComposeError::ZeroExtent { extent });
        }
        let pixels = vec![[fill.r, fill.g, fill.b, fill.a]; extent.0 as usize * extent.1 as usize]
            .into_flattened();
        Ok(Rgba8Surface {
            width: extent.0,
            height: extent.1,
            pixels,
        })
    }

    /// Fill every pixel.
    pub fn fill(&mut self, fill: Rgba) {
        for px in self.pixels.as_chunks_mut::<4>().0 {
            px[0] = fill.r;
            px[1] = fill.g;
            px[2] = fill.b;
            px[3] = fill.a;
        }
    }

    /// One pixel (the assertion vocabulary of the AC tests).
    ///
    /// # Errors
    /// [`ComposeError::RowStride`] — misused as an out-of-bounds
    /// signal; the coordinates of the AC tests are in-range by
    /// construction, and a miss is a placement defect.
    pub fn pixel(&self, x: u32, y: u32) -> Result<Rgba, ComposeError> {
        if x >= self.width || y >= self.height {
            return Err(ComposeError::RowStride {
                y,
                expected: 0,
                got: usize::MAX,
            });
        }
        let i = (y as usize * self.width as usize + x as usize) * 4;
        Ok(Rgba {
            r: self.pixels[i],
            g: self.pixels[i + 1],
            b: self.pixels[i + 2],
            a: self.pixels[i + 3],
        })
    }
}

impl ComposeTarget for Rgba8Surface {
    fn extent(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn row_rgba(&mut self, y: u32) -> Result<&mut [u8], ComposeError> {
        if y >= self.height {
            return Err(ComposeError::RowStride {
                y,
                expected: 0,
                got: usize::MAX,
            });
        }
        let stride = self.width as usize * 4;
        let start = y as usize * stride;
        Ok(&mut self.pixels[start..start + stride])
    }
}

/// Blend one source row onto one destination row over the opaque
/// video plane (the module-docs formula). `xmap[i]` is the source
/// column for destination pixel `dst_x + i` (the nearest-neighbor
/// map, precomputed per cue — no per-pixel division).
pub(crate) fn blend_row(
    dst_row: &mut [u8],
    dst_x: usize,
    xmap: &[u32],
    src_row: &[u8],
) -> Result<(), ComposeError> {
    let end = (dst_x + xmap.len()) * 4;
    if end > dst_row.len() {
        return Err(ComposeError::RowStride {
            y: 0,
            expected: end,
            got: dst_row.len(),
        });
    }
    for (i, &sx) in xmap.iter().enumerate() {
        let si = sx as usize * 4;
        let a = src_row[si + 3] as u32;
        if a == 0 {
            continue;
        }
        let di = (dst_x + i) * 4;
        if a == 255 {
            dst_row[di] = src_row[si];
            dst_row[di + 1] = src_row[si + 1];
            dst_row[di + 2] = src_row[si + 2];
            dst_row[di + 3] = 255;
        } else {
            for c in 0..3 {
                let s = src_row[si + c] as u32;
                let d = dst_row[di + c] as u32;
                dst_row[di + c] = ((a * s + (255 - a) * d + 127) / 255) as u8;
            }
            dst_row[di + 3] = 255;
        }
    }
    Ok(())
}

/// Clip `rect` to `extent`, returning `None` when nothing remains.
pub(crate) fn clip(rect: Rect, extent: (u32, u32)) -> Option<Rect> {
    let x0 = rect.x.min(extent.0);
    let y0 = rect.y.min(extent.1);
    let x1 = rect.x.saturating_add(rect.width).min(extent.0);
    let y1 = rect.y.saturating_add(rect.height).min(extent.1);
    if x0 >= x1 || y0 >= y1 {
        return None;
    }
    Some(Rect {
        x: x0,
        y: y0,
        width: x1 - x0,
        height: y1 - y0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    #[test]
    fn blend_formula_is_round_to_nearest() {
        // a=128, white over black: (128*255 + 127*0 + 127)/255 = 32767/255 = 128.
        let mut row = [0u8, 0, 0, 255];
        let src = [255u8, 255, 255, 128];
        blend_row(&mut row, 0, &[0], &src).must();
        assert_eq!(row, [128, 128, 128, 255]);

        // a=0 leaves the destination untouched (alpha stays video's).
        let mut row = [7u8, 8, 9, 255];
        let src = [255u8, 255, 255, 0];
        blend_row(&mut row, 0, &[0], &src).must();
        assert_eq!(row, [7, 8, 9, 255]);

        // a=255 copies.
        let mut row = [7u8, 8, 9, 255];
        let src = [200u8, 100, 50, 255];
        blend_row(&mut row, 0, &[0], &src).must();
        assert_eq!(row, [200, 100, 50, 255]);
    }

    #[test]
    fn clip_bounds_rect_to_extent() {
        let rect = Rect {
            x: 100,
            y: 50,
            width: 200,
            height: 100,
        };
        assert_eq!(clip(rect, (1920, 1080)), Some(rect));
        assert_eq!(
            clip(rect, (150, 80)),
            Some(Rect {
                x: 100,
                y: 50,
                width: 50,
                height: 30
            })
        );
        assert_eq!(clip(rect, (50, 40)), None);
    }

    #[test]
    fn surface_row_contract() {
        let mut surface = Rgba8Surface::new((4, 2), Rgba::BLACK).must();
        let row = surface.row_rgba(1).must();
        assert_eq!(row.len(), 16);
        let err = surface.row_rgba(2);
        assert!(matches!(err, Err(ComposeError::RowStride { .. })));
    }
}
