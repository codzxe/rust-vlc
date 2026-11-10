//! The canvas seam (card 7.H2, issue #48; doc 09 §4.3) — DPR-aware
//! extent arithmetic and the typed surface-lost vocabulary.
//!
//! # What this module is
//!
//! The Rust half of the canvas contract, mirroring
//! `renderer-wgpu`'s `RenderSurface` discipline (doc 03 §4.11 b/d:
//! resize is non-fatal by construction; loss is explicit and typed)
//! at the browser's canvas shape:
//!
//! - [`pixel_extent`] — doc 09 §4.3's `canvas.width =
//!   round(cssWidth × devicePixelRatio)`, the SAME pure arithmetic
//!   the native surface runs (no browser needed to prove it);
//! - [`CanvasSurface`] — the typed surface state: the negotiated
//!   backend, the DPR-derived extent, the resize bookkeeping, and
//!   the [`CanvasError::SurfaceLost`] vocabulary the JS glue's
//!   `webglcontextlost` / `device.lost` reports drive.
//!
//! # What this is NOT
//!
//! There is no DOM here: the wasm module cannot touch
//! `HTMLCanvasElement`. The JS glue (`player.js`) owns the real
//! canvas and reports resize/loss through the typed messages; this
//! half keeps the arithmetic and the loss state honest and
//! testable headless-native. Real-canvas behavior (ResizeObserver,
//! `devicePixelRatio` change events, context-loss events) is
//! `[VERIFY]` — card 7.I1's matrix owns those rows.

use serde::{Deserialize, Serialize};

use super::backend::RenderBackend;

/// The largest canvas edge this lane will allocate (the browser
/// ceiling guard — canvas areas beyond this are refused TYPED, never
/// allocated into a silent failure). 16384 is the classic WebGL2
/// `MAX_TEXTURE_SIZE` floor across browsers `[VERIFY per browser]`;
/// a 16384×16384 RGBA canvas is already 1 GiB — far past the frame
/// upload bound, so this guard fires long before the upload cap.
pub const MAX_CANVAS_EDGE: u32 = 16_384;

/// A pixel extent (the canvas's backing-store size).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasExtent {
    /// Pixels across.
    pub width: u32,
    /// Pixels down.
    pub height: u32,
}

/// The typed canvas errors (doc 03 §4.11 d's vocabulary, browser
/// shaped — every failure names itself, none are fatal to the
/// session).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CanvasError {
    /// A CSS extent was zero, negative or not finite (a hidden
    /// canvas is legal in the DOM; it is not a present target).
    CssInvalid,
    /// The device pixel ratio was zero, negative or not finite.
    DprInvalid,
    /// The derived pixel extent has a zero component (typed refusal
    /// — mirrors the native surface's `ZeroExtent`).
    ZeroExtent,
    /// The derived extent exceeds [`MAX_CANVAS_EDGE`] on an edge —
    /// the browser ceiling guard (typed, never a silent allocation
    /// failure).
    ExtentTooLarge,
    /// The surface is gone (the context/device loss report) — every
    /// present answers this until a NEW surface is negotiated (doc
    /// 09 §4.4: audio-only degrade, never a frozen last frame).
    SurfaceLost,
}

impl CanvasError {
    /// The human sentence (V7).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            CanvasError::CssInvalid => "the CSS extent was zero, negative or not finite",
            CanvasError::DprInvalid => "the device pixel ratio was zero, negative or not finite",
            CanvasError::ZeroExtent => "the derived pixel extent has a zero component",
            CanvasError::ExtentTooLarge => "the derived extent exceeds the canvas edge ceiling",
            CanvasError::SurfaceLost => "the render surface was lost",
        }
    }
}

impl std::fmt::Display for CanvasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Doc 09 §4.3's DPR arithmetic: `round(css × dpr)` per component,
/// the same pure math the native surface's extent derivation runs.
///
/// # Errors
/// [`CanvasError::CssInvalid`] / [`CanvasError::DprInvalid`] for
/// non-finite or non-positive inputs; [`CanvasError::ZeroExtent`]
/// when a component rounds to zero; [`CanvasError::ExtentTooLarge`]
/// past the [`MAX_CANVAS_EDGE`] guard.
///
/// # Rounding
/// Half-away-from-zero (Rust's `f64::round`) on the POSITIVE
/// quadrant only — inputs are validated positive first, so this is
/// exactly the `Math.round` the JS glue would compute for the same
/// report (pinned by test: `100.25 css × 2.0 dpr = 200.5 → 201`).
pub fn pixel_extent(
    css_width: f64,
    css_height: f64,
    dpr: f64,
) -> Result<CanvasExtent, CanvasError> {
    if !css_width.is_finite() || !css_height.is_finite() || css_width <= 0.0 || css_height <= 0.0 {
        return Err(CanvasError::CssInvalid);
    }
    if !dpr.is_finite() || dpr <= 0.0 {
        return Err(CanvasError::DprInvalid);
    }
    let width = (css_width * dpr).round();
    let height = (css_height * dpr).round();
    if width < 1.0 || height < 1.0 {
        return Err(CanvasError::ZeroExtent);
    }
    if width > f64::from(MAX_CANVAS_EDGE) || height > f64::from(MAX_CANVAS_EDGE) {
        return Err(CanvasError::ExtentTooLarge);
    }
    Ok(CanvasExtent {
        width: width as u32,
        height: height as u32,
    })
}

/// The typed canvas surface state (the render lane's present
/// target). The JS glue owns the real canvas; this half owns the
/// extent truth and the loss vocabulary.
///
/// # Contract (mirrors the native `RenderSurface`)
/// - **Resize is non-fatal by construction** (doc 03 §4.11 d): a
///   new CSS/DPR report reallocates the extent; the typed zero /
///   ceiling refusals leave the PREVIOUS extent intact.
/// - **Loss is explicit**: once [`CanvasSurface::mark_lost`], every
///   [`CanvasSurface::present`] answers
///   [`CanvasError::SurfaceLost`] — never a silent no-op, never a
///   frozen frame pretending to play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasSurface {
    extent: CanvasExtent,
    backend: RenderBackend,
    lost: bool,
    resizes: u64,
}

impl CanvasSurface {
    /// A live surface for `backend` at `extent`.
    ///
    /// # Errors
    /// [`CanvasError::ZeroExtent`] for a zero component;
    /// [`CanvasError::ExtentTooLarge`] past the edge guard.
    pub fn new(backend: RenderBackend, extent: CanvasExtent) -> Result<CanvasSurface, CanvasError> {
        if extent.width == 0 || extent.height == 0 {
            return Err(CanvasError::ZeroExtent);
        }
        if extent.width > MAX_CANVAS_EDGE || extent.height > MAX_CANVAS_EDGE {
            return Err(CanvasError::ExtentTooLarge);
        }
        Ok(CanvasSurface {
            extent,
            backend,
            lost: false,
            resizes: 0,
        })
    }

    /// The current pixel extent.
    #[must_use]
    pub fn extent(&self) -> CanvasExtent {
        self.extent
    }

    /// The backend this surface presents through.
    #[must_use]
    pub fn backend(&self) -> RenderBackend {
        self.backend
    }

    /// The CPU-staged upload format every present uses (RGBA8,
    /// straight alpha — the single allowed CPU→GPU-adjacent copy's
    /// source layout, doc 09 §4.2 row 1).
    #[must_use]
    pub fn format(&self) -> &'static str {
        "rgba8"
    }

    /// Whether the surface is lost.
    #[must_use]
    pub fn is_lost(&self) -> bool {
        self.lost
    }

    /// How many resizes were applied (the ResizeObserver evidence).
    #[must_use]
    pub fn resize_count(&self) -> u64 {
        self.resizes
    }

    /// Apply a new CSS/DPR report (the resize path — non-fatal by
    /// construction: a typed refusal keeps the previous extent).
    ///
    /// # Errors
    /// [`pixel_extent`]'s vocabulary; the surface is unchanged on
    /// every error path.
    pub fn resize(&mut self, css_width: f64, css_height: f64, dpr: f64) -> Result<(), CanvasError> {
        if self.lost {
            // A lost surface does not resize — the recovery path is
            // a NEW surface (the lane's re-negotiation).
            return Err(CanvasError::SurfaceLost);
        }
        let extent = pixel_extent(css_width, css_height, dpr)?;
        self.extent = extent;
        self.resizes = self.resizes.saturating_add(1);
        Ok(())
    }

    /// Mark the surface lost (the glue's context/device-loss
    /// report). Idempotent; there is no un-loss.
    pub fn mark_lost(&mut self) {
        self.lost = true;
    }

    /// Present one frame (the Fifo-shaped contract: returns with
    /// the frame logically on the surface).
    ///
    /// # Errors
    /// [`CanvasError::SurfaceLost`] once lost — the typed degrade
    /// the render lane surfaces as the audio-only event.
    pub fn present(&mut self) -> Result<(), CanvasError> {
        if self.lost {
            return Err(CanvasError::SurfaceLost);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::{Must, assert_err};

    /// Doc 09 §4.3's arithmetic, exactly: `round(css × dpr)` per
    /// component, half-up on the positive quadrant (the Math.round
    /// parity the JS glue computes).
    #[test]
    fn dpr_arithmetic_rounds_half_up() {
        assert_eq!(
            pixel_extent(100.0, 50.0, 2.0).must(),
            CanvasExtent {
                width: 200,
                height: 100
            }
        );
        // 100.25 × 2.0 = 200.5 → 201 (the rounding pin).
        assert_eq!(
            pixel_extent(100.25, 50.0, 2.0).must(),
            CanvasExtent {
                width: 201,
                height: 100
            }
        );
        // Fractional DPR (the zoom levels): 1.25×.
        assert_eq!(
            pixel_extent(1920.0, 1080.0, 1.25).must(),
            CanvasExtent {
                width: 2400,
                height: 1350
            }
        );
        // Low-power DPR 1.0 is the identity.
        assert_eq!(
            pixel_extent(640.0, 480.0, 1.0).must(),
            CanvasExtent {
                width: 640,
                height: 480
            }
        );
    }

    /// The typed refusals: non-finite/zero CSS, bad DPR, sub-pixel
    /// results, and the ceiling guard — each named, none fatal.
    #[test]
    fn extent_refusals_are_typed() {
        for (w, h) in [
            (0.0, 10.0),
            (-5.0, 10.0),
            (f64::NAN, 10.0),
            (10.0, f64::INFINITY),
        ] {
            assert_eq!(
                assert_err(pixel_extent(w, h, 2.0), "css"),
                CanvasError::CssInvalid
            );
        }
        for dpr in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                assert_err(pixel_extent(10.0, 10.0, dpr), "dpr"),
                CanvasError::DprInvalid
            );
        }
        // 0.4 css × 1.0 dpr rounds to zero — the typed zero extent.
        assert_eq!(
            assert_err(pixel_extent(0.4, 10.0, 1.0), "sub-pixel"),
            CanvasError::ZeroExtent
        );
        // Past the ceiling guard.
        assert_eq!(
            assert_err(pixel_extent(16_385.0, 100.0, 1.0), "ceiling"),
            CanvasError::ExtentTooLarge
        );
        assert!(
            pixel_extent(16_384.0, 100.0, 1.0).is_ok(),
            "the edge itself is legal"
        );
    }

    /// Resize is non-fatal by construction: typed refusals keep the
    /// previous extent; successful resizes reallocate and count.
    #[test]
    fn resize_is_non_fatal_and_counted() {
        let mut surface = CanvasSurface::new(
            RenderBackend::WebGl2,
            CanvasExtent {
                width: 640,
                height: 480,
            },
        )
        .must();
        surface.resize(1280.0, 720.0, 1.0).must();
        assert_eq!(
            surface.extent(),
            CanvasExtent {
                width: 1280,
                height: 720
            }
        );
        assert_eq!(surface.resize_count(), 1);
        // The typed refusal leaves the extent untouched.
        let error = assert_err(surface.resize(0.0, 720.0, 1.0), "bad resize");
        assert_eq!(error, CanvasError::CssInvalid);
        assert_eq!(
            surface.extent(),
            CanvasExtent {
                width: 1280,
                height: 720
            },
            "the previous extent survives the refusal"
        );
        assert_eq!(surface.resize_count(), 1, "refusals do not count");
        assert_eq!(surface.format(), "rgba8");
        assert_eq!(surface.backend(), RenderBackend::WebGl2);
    }

    /// The loss vocabulary: once lost, presents answer the typed
    /// error forever (no un-loss); a lost surface refuses resizes
    /// too (the recovery is a NEW surface).
    #[test]
    fn loss_is_explicit_and_permanent() {
        let mut surface = CanvasSurface::new(
            RenderBackend::WebGpu,
            CanvasExtent {
                width: 64,
                height: 48,
            },
        )
        .must();
        surface.present().must();
        surface.mark_lost();
        assert!(surface.is_lost());
        assert_eq!(
            assert_err(surface.present(), "lost present"),
            CanvasError::SurfaceLost
        );
        // Idempotent marking; still lost.
        surface.mark_lost();
        assert_eq!(
            assert_err(surface.resize(100.0, 100.0, 1.0), "lost resize"),
            CanvasError::SurfaceLost
        );
        assert_eq!(
            surface.extent(),
            CanvasExtent {
                width: 64,
                height: 48
            },
            "the lost extent is frozen for the postmortem"
        );
        // A NEW surface over the same backend starts clean.
        let mut fresh = CanvasSurface::new(
            RenderBackend::WebGl2,
            CanvasExtent {
                width: 64,
                height: 48,
            },
        )
        .must();
        fresh.present().must();
    }

    /// Construction validates the extent the same way the resize
    /// path does (one vocabulary, no drift).
    #[test]
    fn construction_validates_typed() {
        assert_eq!(
            assert_err(
                CanvasSurface::new(
                    RenderBackend::WebGpu,
                    CanvasExtent {
                        width: 0,
                        height: 10
                    }
                ),
                "zero construct"
            ),
            CanvasError::ZeroExtent
        );
        assert_eq!(
            assert_err(
                CanvasSurface::new(
                    RenderBackend::WebGpu,
                    CanvasExtent {
                        width: 20_000,
                        height: 10
                    }
                ),
                "oversized construct"
            ),
            CanvasError::ExtentTooLarge
        );
    }
}
