//! Color conversion and chroma upsampling for the RGBA output path
//! (ITU-R BT.601 full-range YCbCr ↔ RGB, the JFIF conversion, fixed
//! point — no floats, deterministic output).
//!
//! The I420 path needs none of this (planes pass through at their
//! native sampling); only 4:4:4 / 4:2:2 / 4:4:0 streams — which have
//! no planar vocabulary in `PixelFormat` yet — convert here.
//!
//! Upsampling is nearest-neighbor (box) replication: the spec-minimum
//! decoder behavior. libjpeg's "fancy" triangular upsampler is a
//! quality refinement, not a conformance requirement — recorded as an
//! owed refinement for the 5.H3 polish card.

/// Full-range BT.601 integer constants, Q16 (rounded at authoring
/// time; the classic JFIF equations).
const CR_R: i64 = 91881; // 1.402
const CB_G: i64 = -22554; // -0.344136
const CR_G: i64 = -46802; // -0.714136
const CB_B: i64 = 116127; // 1.772

/// Convert one YCbCr triple to RGB (Q16 fixed point, rounded, clamped).
fn ycc_to_rgb(y: u8, cb: u8, cr: u8) -> [u8; 3] {
    let y = i64::from(y);
    let cb = i64::from(cb) - 128;
    let cr = i64::from(cr) - 128;
    let r = y + shr16_round(CR_R * cr);
    let g = y + shr16_round(CB_G * cb + CR_G * cr);
    let b = y + shr16_round(CB_B * cb);
    [
        r.clamp(0, 255) as u8,
        g.clamp(0, 255) as u8,
        b.clamp(0, 255) as u8,
    ]
}

/// Divide by 2¹⁶ with round-half-away-from-zero.
fn shr16_round(value: i64) -> i64 {
    if value >= 0 {
        (value + (1i64 << 15)) >> 16
    } else {
        -((-value + (1i64 << 15)) >> 16)
    }
}

/// A decoded component plane (padded to full 8×8 blocks; `stride` is
/// the padded row pitch).
pub(crate) struct PlaneView<'a> {
    pub data: &'a [u8],
    pub stride: usize,
}

/// Where one component's samples sit on the MCU grid.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ComponentGeom {
    /// Horizontal sampling factor (1..=2).
    pub h: usize,
    /// Vertical sampling factor (1..=2).
    pub v: usize,
    /// Valid sample width (component pixels).
    pub valid_w: usize,
    /// Valid sample height.
    pub valid_h: usize,
}

/// Row composer: upsampled YCbCr → RGBA for one full-resolution row.
pub(crate) struct RgbComposer<'a> {
    /// Luma plane (always present).
    luma: PlaneView<'a>,
    /// Chroma planes (absent for grayscale — neutral 128 used).
    chroma: Option<(PlaneView<'a>, PlaneView<'a>)>,
    /// Geometry per component: Y, Cb, Cr.
    geoms: [ComponentGeom; 3],
    h_max: usize,
    v_max: usize,
}

impl<'a> RgbComposer<'a> {
    /// Assemble from the decoded planes. `geoms` covers Y, Cb, Cr in
    /// order; `chroma` is `None` for single-component streams.
    pub(crate) fn new(
        luma: PlaneView<'a>,
        chroma: Option<(PlaneView<'a>, PlaneView<'a>)>,
        geoms: [ComponentGeom; 3],
        h_max: usize,
        v_max: usize,
    ) -> RgbComposer<'a> {
        RgbComposer {
            luma,
            chroma,
            geoms,
            h_max,
            v_max,
        }
    }

    /// Upsampled read: full-resolution `(x, y)` from one plane.
    fn sample_up(&self, plane: &PlaneView<'_>, geom: &ComponentGeom, x: usize, y: usize) -> u8 {
        let sx = (x * geom.h / self.h_max).min(geom.valid_w.saturating_sub(1));
        let sy = (y * geom.v / self.v_max).min(geom.valid_h.saturating_sub(1));
        let index = sy
            .checked_mul(plane.stride)
            .and_then(|row| row.checked_add(sx))
            .unwrap_or(0);
        *plane.data.get(index).unwrap_or(&128)
    }

    /// Compose `width` RGBA pixels of output row `row_y` into `out`
    /// (alpha is straight `0xff`).
    pub(crate) fn compose_row(&self, row_y: usize, width: usize, out: &mut [u8]) {
        for x in 0..width {
            let luma = self.sample_up(&self.luma, &self.geoms[0], x, row_y);
            let (cb, cr) = match &self.chroma {
                Some((cb_plane, cr_plane)) => (
                    self.sample_up(cb_plane, &self.geoms[1], x, row_y),
                    self.sample_up(cr_plane, &self.geoms[2], x, row_y),
                ),
                None => (128, 128),
            };
            let [r, g, b] = ycc_to_rgb(luma, cb, cr);
            let base = x * 4;
            if let Some(slot) = out.get_mut(base) {
                *slot = r;
            }
            if let Some(slot) = out.get_mut(base + 1) {
                *slot = g;
            }
            if let Some(slot) = out.get_mut(base + 2) {
                *slot = b;
            }
            if let Some(slot) = out.get_mut(base + 3) {
                *slot = 255;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plane(bytes: &[u8], stride: usize) -> PlaneView<'_> {
        PlaneView {
            data: bytes,
            stride,
        }
    }

    #[test]
    fn neutral_chroma_is_identity() {
        // Y, Cb=Cr=128 → R=G=B=Y (full-range BT.601).
        for y in [0u8, 1, 64, 128, 200, 254, 255] {
            let [r, g, b] = ycc_to_rgb(y, 128, 128);
            assert_eq!(r, y, "R for Y={y}");
            assert_eq!(g, y, "G for Y={y}");
            assert_eq!(b, y, "B for Y={y}");
        }
    }

    #[test]
    fn known_triples_convert_correctly() {
        // Hand-computed with the Q16 constants above (round-half-away):
        // warm pixel: r dominates; cool pixel: b dominates.
        let [r, g, b] = ycc_to_rgb(90, 110, 200);
        assert_eq!([r, g, b], [191, 45, 58]);
        assert!(r > g && r > b, "warm triple: R must dominate");

        let [r, g, b] = ycc_to_rgb(30, 220, 120);
        assert_eq!([r, g, b], [19, 4, 193]);
        assert!(b > r && b > g, "cool triple: B must dominate");

        let [r, g, b] = ycc_to_rgb(120, 200, 200);
        assert!(
            g < r && g < b,
            "high Cb AND Cr pull G below both: got [{r}, {g}, {b}]"
        );
    }

    #[test]
    fn out_of_gamut_triples_clamp() {
        // R and B saturate high; G stays finite (hand-computed).
        let [r, g, b] = ycc_to_rgb(255, 255, 255);
        assert_eq!([r, g, b], [255, 121, 255]);
        // R and B saturate low; G stays finite.
        let [r, g, b] = ycc_to_rgb(0, 0, 0);
        assert_eq!([r, g, b], [0, 135, 0]);
    }

    #[test]
    fn grayscale_row_passes_through_luma() {
        let y_bytes = [10u8, 20, 30, 40];
        let composer = RgbComposer::new(
            plane(&y_bytes, 4),
            None,
            [
                ComponentGeom {
                    h: 1,
                    v: 1,
                    valid_w: 4,
                    valid_h: 1,
                },
                ComponentGeom {
                    h: 1,
                    v: 1,
                    valid_w: 4,
                    valid_h: 1,
                },
                ComponentGeom {
                    h: 1,
                    v: 1,
                    valid_w: 4,
                    valid_h: 1,
                },
            ],
            1,
            1,
        );
        let mut out = [0u8; 16];
        composer.compose_row(0, 4, &mut out);
        for (i, want) in y_bytes.iter().enumerate() {
            assert_eq!(&out[i * 4..i * 4 + 3], &[*want; 3]);
            assert_eq!(out[i * 4 + 3], 255);
        }
    }

    #[test]
    fn chroma_upsample_box_replication() {
        // 2×1 chroma grid stretched over 4 pixels: each chroma sample
        // covers two luma positions. Exact expectations computed with
        // the Q16 constants (round-half-away, then clamp):
        // (y=200, cb=90, cr=240) → (255, 133, 133);
        // (y=10,  cb=240, cr=110) → (0, 0, 208).
        let y_bytes = [200u8, 10, 200, 10];
        let cb_bytes = [90u8, 240];
        let cr_bytes = [240u8, 110];
        let geom_y = ComponentGeom {
            h: 1,
            v: 1,
            valid_w: 4,
            valid_h: 1,
        };
        let geom_c = ComponentGeom {
            h: 1,
            v: 1,
            valid_w: 2,
            valid_h: 1,
        };
        let composer = RgbComposer::new(
            plane(&y_bytes, 4),
            Some((plane(&cb_bytes, 2), plane(&cr_bytes, 2))),
            [geom_y, geom_c, geom_c],
            2,
            1,
        );
        let mut out = [0u8; 16];
        composer.compose_row(0, 4, &mut out);
        assert_eq!(&out[0..4], &[255, 133, 133, 255]);
        assert_eq!(&out[4..8], &[255, 133, 133, 255]);
        assert_eq!(&out[8..12], &[0, 0, 208, 255]);
        assert_eq!(&out[12..16], &[0, 0, 208, 255]);
    }
}
