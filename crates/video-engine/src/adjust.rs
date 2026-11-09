//! Video color-adjust parameters (issue #33 / D-012 — the `adjust`
//! filter's engine-side model: brightness, contrast, saturation, hue,
//! gamma).
//!
//! # Placement
//!
//! This is the **parameter model + CPU reference** for the adjust
//! stage. The production path is a renderer-wgpu shader consuming
//! these as uniforms (doc 03 §2.3 rule 3 — one pipeline, zero extra
//! copies); wiring the uniforms into the wgpu pipeline is
//! `[PROPOSAL]` follow-up (a bind-group layout change is structural —
//! recorded here, not smuggled in). Until then [`AdjustParams`]
//! carries the validated, clamped-by-contract values and
//! [`AdjustParams::apply_rgba8`] is the byte-exact CPU reference the
//! GPU path must match (and the test oracle).
//!
//! # Ranges (feature row `vfeat-colors-adjust`; VLC-corroborated)
//!
//! | field        | range        | neutral |
//! |--------------|--------------|---------|
//! | brightness   | `0.0 ..= 2.0`| `1.0`   |
//! | contrast     | `0.0 ..= 2.0`| `1.0`   |
//! | saturation   | `0.0 ..= 3.0`| `1.0`   |
//! | hue          | `−180 ..= 180` (degrees) | `0.0` |
//! | gamma        | `0.1 ..= 10.0` [VERIFY exact VLC range] | `1.0` |
//!
//! Neutral parameters are the **identity**: [`AdjustParams::NEUTRAL`]
//! leaves pixels bit-unchanged (the apply path short-circuits before
//! any float conversion, so identity is exact, not approximately
//! equal). Out-of-range values are rejected at construction — no
//! silent clamping of a control value (the caller learns, per the
//! never-invent discipline).
//!
//! # Never
//!
//! Allocation per pixel, panics, `unsafe`.

/// Invalid adjust parameters (typed: failure is data, never a panic).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjustError {
    /// A field was non-finite (NaN/∞ must never reach a shader).
    NotFinite {
        /// The offending field's name (`"brightness"`, …).
        field: &'static str,
    },
    /// A field was outside its documented range.
    OutOfRange {
        /// The offending field's name.
        field: &'static str,
    },
}

impl std::fmt::Display for AdjustError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdjustError::NotFinite { field } => {
                write!(f, "adjust parameter `{field}` must be finite")
            }
            AdjustError::OutOfRange { field } => {
                write!(f, "adjust parameter `{field}` outside its documented range")
            }
        }
    }
}

impl std::error::Error for AdjustError {}

/// The validated color-adjust parameter set (see module docs for the
/// range table and the GPU-wiring proposal).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdjustParams {
    /// Multiplier on luma (`1.0` = unchanged).
    pub brightness: f32,
    /// Scale around mid-gray (`1.0` = unchanged).
    pub contrast: f32,
    /// Chroma scale (`1.0` = unchanged; `0.0` = grayscale).
    pub saturation: f32,
    /// Hue rotation in degrees (`0.0` = unchanged; ±180 full turn).
    pub hue: f32,
    /// Gamma exponent (`1.0` = unchanged). [VERIFY] exact upper range.
    pub gamma: f32,
}

impl Default for AdjustParams {
    fn default() -> Self {
        Self::NEUTRAL
    }
}

/// Field bounds as `(min, max)` pairs — the one place ranges live, so
/// docs, validation and tests cannot drift apart.
fn bounds(field: &str) -> Option<(f32, f32)> {
    match field {
        "brightness" => Some((0.0, 2.0)),
        "contrast" => Some((0.0, 2.0)),
        "saturation" => Some((0.0, 3.0)),
        "hue" => Some((-180.0, 180.0)),
        "gamma" => Some((0.1, 10.0)),
        _ => None,
    }
}

fn check(field: &'static str, value: f32) -> Result<f32, AdjustError> {
    if !value.is_finite() {
        return Err(AdjustError::NotFinite { field });
    }
    let (min, max) = bounds(field).expect("known field");
    if !(min..=max).contains(&value) {
        return Err(AdjustError::OutOfRange { field });
    }
    Ok(value)
}

impl AdjustParams {
    /// The identity: every field at neutral, [`apply_rgba8`] is a
    /// bit-exact no-op.
    pub const NEUTRAL: AdjustParams = AdjustParams {
        brightness: 1.0,
        contrast: 1.0,
        saturation: 1.0,
        hue: 0.0,
        gamma: 1.0,
    };

    /// Validate and build (see the range table in the module docs).
    ///
    /// # Errors
    /// [`AdjustError::NotFinite`] / [`AdjustError::OutOfRange`] naming
    /// the first offending field.
    pub fn new(
        brightness: f32,
        contrast: f32,
        saturation: f32,
        hue: f32,
        gamma: f32,
    ) -> Result<AdjustParams, AdjustError> {
        Ok(AdjustParams {
            brightness: check("brightness", brightness)?,
            contrast: check("contrast", contrast)?,
            saturation: check("saturation", saturation)?,
            hue: check("hue", hue)?,
            gamma: check("gamma", gamma)?,
        })
    }

    /// Whether every field is at its neutral value.
    pub fn is_neutral(&self) -> bool {
        *self == Self::NEUTRAL
    }

    /// Apply to an RGBA8 pixel buffer (CPU reference; 4 bytes/pixel,
    /// length checked). Alpha is never touched.
    ///
    /// # Errors
    /// [`AdjustError::OutOfRange`] with field `"pixels"` if the buffer
    /// is not a whole number of RGBA8 pixels.
    pub fn apply_rgba8(&self, pixels: &mut [u8]) -> Result<(), AdjustError> {
        if pixels.len() % 4 != 0 {
            return Err(AdjustError::OutOfRange { field: "pixels" });
        }
        if self.is_neutral() {
            // Exact identity: no float round-trip, byte-for-byte
            // unchanged (the invariant the GPU path must preserve).
            return Ok(());
        }
        // Precompute the per-plane constants once per buffer, not per
        // pixel (hot-path discipline D-1: no allocation, hoisted math).
        let (sin_h, cos_h) = {
            let rad = self.hue.to_radians();
            (rad.sin(), rad.cos())
        };
        for px in pixels.chunks_exact_mut(4) {
            let r = f32::from(px[0]) / 255.0;
            let g = f32::from(px[1]) / 255.0;
            let b = f32::from(px[2]) / 255.0;
            // Brightness (luma multiply) → gamma → contrast around
            // mid-gray → saturation toward Rec.601 luma → hue rotation
            // in a YPbPr-ish plane. Order is fixed and documented; the
            // tests pin the composite, the panel pins the sliders.
            let r = r * self.brightness;
            let g = g * self.brightness;
            let b = b * self.brightness;
            let r = r.powf(self.gamma);
            let g = g.powf(self.gamma);
            let b = b.powf(self.gamma);
            let r = (r - 0.5) * self.contrast + 0.5;
            let g = (g - 0.5) * self.contrast + 0.5;
            let b = (b - 0.5) * self.contrast + 0.5;
            let luma = 0.299 * r + 0.587 * g + 0.114 * b;
            let r = luma + (r - luma) * self.saturation;
            let g = luma + (g - luma) * self.saturation;
            let b = luma + (b - luma) * self.saturation;
            // Hue: rotate the (B−Y, R−Y) chroma pair by the hue angle.
            let cb = b - luma;
            let cr = r - luma;
            let r = luma + cr * cos_h - cb * sin_h;
            let b = luma + cb * cos_h + cr * sin_h;
            let clamp01 = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round();
            px[0] = clamp01(r) as u8;
            px[1] = clamp01(g) as u8;
            px[2] = clamp01(b) as u8;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_params_leave_pixels_bit_unchanged() {
        let original: Vec<u8> = (0u8..=63).map(|i| i.wrapping_mul(7)).collect();
        let mut pixels = original.clone();
        AdjustParams::NEUTRAL.apply_rgba8(&mut pixels).unwrap();
        assert_eq!(pixels, original, "identity must be byte-exact");
    }

    #[test]
    fn default_is_neutral_and_neutral_is_identity() {
        assert_eq!(AdjustParams::default(), AdjustParams::NEUTRAL);
        assert!(AdjustParams::NEUTRAL.is_neutral());
    }

    #[test]
    fn out_of_range_fields_are_rejected_by_name() {
        assert_eq!(
            AdjustParams::new(2.5, 1.0, 1.0, 0.0, 1.0),
            Err(AdjustError::OutOfRange { field: "brightness" })
        );
        assert_eq!(
            AdjustParams::new(1.0, 1.0, 3.5, 0.0, 1.0),
            Err(AdjustError::OutOfRange { field: "saturation" })
        );
        assert_eq!(
            AdjustParams::new(1.0, 1.0, 1.0, 181.0, 1.0),
            Err(AdjustError::OutOfRange { field: "hue" })
        );
        assert_eq!(
            AdjustParams::new(1.0, 1.0, 1.0, -181.0, 1.0),
            Err(AdjustError::OutOfRange { field: "hue" })
        );
        assert_eq!(
            AdjustParams::new(1.0, 1.0, 1.0, 0.0, 10.5),
            Err(AdjustError::OutOfRange { field: "gamma" })
        );
    }

    #[test]
    fn non_finite_fields_are_rejected() {
        assert_eq!(
            AdjustParams::new(f32::NAN, 1.0, 1.0, 0.0, 1.0),
            Err(AdjustError::NotFinite { field: "brightness" })
        );
        assert_eq!(
            AdjustParams::new(1.0, f32::INFINITY, 1.0, 0.0, 1.0),
            Err(AdjustError::NotFinite { field: "contrast" })
        );
    }

    #[test]
    fn boundary_values_are_accepted() {
        assert!(AdjustParams::new(0.0, 0.0, 0.0, -180.0, 0.1).is_ok());
        assert!(AdjustParams::new(2.0, 2.0, 3.0, 180.0, 10.0).is_ok());
    }

    #[test]
    fn brightness_scales_pixels_and_zero_is_black() {
        // Mid-gray × brightness 2.0 (clamped) → white; × 0.0 → black.
        let gray = [128u8, 128, 128, 255];
        let mut px = gray;
        AdjustParams::new(2.0, 1.0, 1.0, 0.0, 1.0)
            .unwrap()
            .apply_rgba8(&mut px)
            .unwrap();
        assert_eq!(&px[..3], &[255, 255, 255]);
        let mut px = gray;
        AdjustParams::new(0.0, 1.0, 1.0, 0.0, 1.0)
            .unwrap()
            .apply_rgba8(&mut px)
            .unwrap();
        assert_eq!(&px[..3], &[0, 0, 0]);
    }

    #[test]
    fn saturation_zero_is_grayscale_but_alpha_survives() {
        // A saturated red desaturates to its Rec.601 luma; alpha stays.
        let mut px = [240u8, 20, 20, 199];
        AdjustParams::new(1.0, 1.0, 0.0, 0.0, 1.0)
            .unwrap()
            .apply_rgba8(&mut px)
            .unwrap();
        let luma: f32 = 0.299 * (240.0 / 255.0) + 0.587 * (20.0 / 255.0) + 0.114 * (20.0 / 255.0);
        let expected = (luma * 255.0).round() as u8;
        assert_eq!(px[0], expected);
        assert_eq!(px[1], expected);
        assert_eq!(px[2], expected);
        assert_eq!(px[3], 199, "alpha is never adjusted");
    }

    #[test]
    fn contrast_pushes_away_from_midgray_and_clamps() {
        // Contrast 2.0 doubles the offset from mid-gray (rounding
        // noted: 128 → 129) and clamps beyond range to black/white.
        let mut px = [128u8, 128, 128, 255];
        AdjustParams::new(1.0, 2.0, 1.0, 0.0, 1.0)
            .unwrap()
            .apply_rgba8(&mut px)
            .unwrap();
        assert_eq!(&px[..3], &[129, 129, 129]);
        let mut px = [200u8, 200, 200, 255];
        AdjustParams::new(1.0, 2.0, 1.0, 0.0, 1.0)
            .unwrap()
            .apply_rgba8(&mut px)
            .unwrap();
        assert_eq!(&px[..3], &[255, 255, 255]);
        let mut px = [55u8, 55, 55, 255];
        AdjustParams::new(1.0, 2.0, 1.0, 0.0, 1.0)
            .unwrap()
            .apply_rgba8(&mut px)
            .unwrap();
        assert_eq!(&px[..3], &[0, 0, 0]);
        // Contrast 0 collapses everything to mid-gray.
        let mut px = [240u8, 10, 200, 255];
        AdjustParams::new(1.0, 0.0, 1.0, 0.0, 1.0)
            .unwrap()
            .apply_rgba8(&mut px)
            .unwrap();
        assert_eq!(&px[..3], &[128, 128, 128]);
    }

    #[test]
    fn hue_180_rotates_red_chroma_to_blue_side() {
        // Pure red, rotated 180°: luma stays, chroma flips sign — the
        // dominant channel becomes B-ish (R's chroma goes negative).
        let mut px = [255u8, 0, 0, 255];
        AdjustParams::new(1.0, 1.0, 1.0, 180.0, 1.0)
            .unwrap()
            .apply_rgba8(&mut px)
            .unwrap();
        assert!(px[2] > px[0], "180° hue turns red toward blue: {px:?}");
        assert!(px[2] > px[1], "and not toward green: {px:?}");
    }

    #[test]
    fn bad_pixel_length_is_a_typed_error() {
        let mut px = [1u8, 2, 3];
        assert_eq!(
            AdjustParams::NEUTRAL.apply_rgba8(&mut px),
            Err(AdjustError::OutOfRange { field: "pixels" })
        );
    }

    #[test]
    fn error_display_names_the_field() {
        let e = AdjustError::OutOfRange { field: "hue" }.to_string();
        assert!(e.contains("hue") && e.contains("range"), "{e}");
    }
}
