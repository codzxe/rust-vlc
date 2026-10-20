//! Surface formats, sRGB present semantics and the pixel vocabulary
//! (card 2.D1 "sRGB present"; doc 03 §4.11 a).
//!
//! # The v0 sRGB contract (honest scope)
//!
//! Both v0 surface formats are **sRGB-encoded 8-bit** — the
//! desktop-compositor default (`Bgra8UnormSrgb` is the typical wgpu
//! swapchain format). The contract the v0 upload/composite path proves:
//!
//! 1. **No double conversion**: an RGBA8 frame already carries
//!    display-encoded sRGB bytes; the sRGB surface treats framebuffer
//!    bytes as encoded values, so the pipeline is a *passthrough* —
//!    proven byte-exactly by the headless round-trip AC (frame in ==
//!    readback out at 1:1 placement).
//! 2. **Sampling domain**: scaling filters operate in linear light on
//!    real compositors. The v0 software compositor uses point sampling
//!    (nearest neighbor), which needs no gamma-domain conversion — the
//!    byte-exact invariant above holds under scaling too. Bilinear
//!    filtering (linear-light) joins with the GPU color-conversion
//!    shader path (doc 03 §4.11 e "GPU color conversion"; MP §11).
//! 3. **Shape-level wgpu mapping**: [`SurfaceFormat::wgpu_texture_format`]
//!    returns the exact wgpu 26/30 enum variant name the hardware
//!    backend will configure, pinned by unit test before the dependency
//!    exists (the Lead-gated import — see the crate docs and allowlist
//!    §5).

/// An sRGB-encoded 8-bit surface format (the only kind v0 configures).
///
/// The two variants are the two byte orders desktop compositors hand
/// out; the renderer composites in the surface's native order and
/// converts only at the readback boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceFormat {
    /// 8-bit BGRA, sRGB-encoded (the desktop-compositor favorite —
    /// media-types' `PixelFormat::Bgra8` doc names it so).
    Bgra8Srgb,
    /// 8-bit RGBA, sRGB-encoded.
    Rgba8Srgb,
}

impl SurfaceFormat {
    /// The exact `wgpu::TextureFormat` enum variant this format maps
    /// to on the hardware backend (shape-level contract, unit-pinned).
    #[must_use = "the hardware backend configures surfaces from this name"]
    pub fn wgpu_texture_format(self) -> &'static str {
        match self {
            SurfaceFormat::Bgra8Srgb => "Bgra8UnormSrgb",
            SurfaceFormat::Rgba8Srgb => "Rgba8UnormSrgb",
        }
    }

    /// Bytes per pixel (both v0 formats are 4).
    #[must_use = "pure constant"]
    pub fn bytes_per_pixel(self) -> u32 {
        4
    }

    /// Convert one RGBA pixel into this surface's native byte order.
    #[must_use = "pure swizzle"]
    pub fn pack(self, pixel: Rgba) -> [u8; 4] {
        match self {
            SurfaceFormat::Bgra8Srgb => [pixel.0[2], pixel.0[1], pixel.0[0], pixel.0[3]],
            SurfaceFormat::Rgba8Srgb => pixel.0,
        }
    }

    /// Convert one native-order pixel of this surface back to RGBA.
    #[must_use = "pure swizzle"]
    pub fn unpack(self, bytes: [u8; 4]) -> Rgba {
        match self {
            SurfaceFormat::Bgra8Srgb => Rgba([bytes[2], bytes[1], bytes[0], bytes[3]]),
            SurfaceFormat::Rgba8Srgb => Rgba(bytes),
        }
    }
}

/// A straight-alpha 8-bit RGBA pixel / color (doc 03 §4.11 b's
/// `background: ColorDesc` adaptation — see [`crate::params`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba(pub [u8; 4]);

impl Rgba {
    /// Opaque black — the default letterbox background.
    pub const BLACK: Rgba = Rgba([0, 0, 0, 255]);
    /// Opaque white.
    pub const WHITE: Rgba = Rgba([255, 255, 255, 255]);
    /// Fully transparent (surfaces with an alpha channel only).
    pub const TRANSPARENT: Rgba = Rgba([0, 0, 0, 0]);

    /// Build from separate components.
    #[must_use = "pure constructor"]
    pub const fn from_components(r: u8, g: u8, b: u8, a: u8) -> Rgba {
        Rgba([r, g, b, a])
    }

    /// Source-over alpha compositing of `self` over `below` in 8-bit
    /// straight alpha — the subtitle-layer blend primitive (doc 03
    /// §4.11 "the video shader composites"), unit-tested here so the
    /// Phase-5 overlay card consumes a proven function.
    #[must_use = "pure compositing math"]
    pub fn source_over(self, below: Rgba) -> Rgba {
        let (r, g, b, a) = (self.0[0], self.0[1], self.0[2], self.0[3]);
        if a == 255 {
            return self;
        }
        if a == 0 {
            return below;
        }
        let blend = |top: u8, bottom: u8| -> u8 {
            let t = u16::from(top) * u16::from(a) + u16::from(bottom) * (255 - u16::from(a));
            ((t + 127) / 255) as u8
        };
        Rgba([
            blend(r, below.0[0]),
            blend(g, below.0[1]),
            blend(b, below.0[2]),
            255,
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wgpu_format_names_are_the_real_enum_variants() {
        // Shape-level pin of the hardware-backend mapping: these are
        // the literal wgpu::TextureFormat variant names (wgpu 26–30;
        // Bgra8UnormSrgb is the canonical swapchain format on
        // Vulkan/GL, Rgba8UnormSrgb on Metal).
        assert_eq!(
            SurfaceFormat::Bgra8Srgb.wgpu_texture_format(),
            "Bgra8UnormSrgb"
        );
        assert_eq!(
            SurfaceFormat::Rgba8Srgb.wgpu_texture_format(),
            "Rgba8UnormSrgb"
        );
        assert_eq!(SurfaceFormat::Bgra8Srgb.bytes_per_pixel(), 4);
    }

    #[test]
    fn swizzles_round_trip_per_pixel() {
        let pixel = Rgba([0x11, 0x22, 0x33, 0x44]);
        for format in [SurfaceFormat::Bgra8Srgb, SurfaceFormat::Rgba8Srgb] {
            assert_eq!(format.unpack(format.pack(pixel)), pixel, "{format:?}");
        }
        // The BGRA pack is a real byte swap, not an identity.
        assert_eq!(
            SurfaceFormat::Bgra8Srgb.pack(Rgba([1, 2, 3, 4])),
            [3, 2, 1, 4]
        );
    }

    #[test]
    fn source_over_covers_the_alpha_edges_and_a_middle_case() {
        // Opaque replaces everything below.
        assert_eq!(Rgba::WHITE.source_over(Rgba::BLACK), Rgba::WHITE);
        // Fully transparent keeps everything below.
        assert_eq!(
            Rgba::TRANSPARENT.source_over(Rgba([9, 8, 7, 255])),
            Rgba([9, 8, 7, 255])
        );
        // 50 % over opaque black halves each channel (rounded).
        let half = Rgba([200, 100, 50, 128]).source_over(Rgba::BLACK);
        assert_eq!(half, Rgba([100, 50, 25, 255]));
        // Alpha accumulates to opaque when the below layer is opaque.
        let over = Rgba([255, 0, 0, 128]).source_over(Rgba([0, 0, 255, 255]));
        assert_eq!(over.0[3], 255);
        assert_eq!(over.0[1], 0);
        // Blend against white: exact math is 255·(1 − 128/255) = 127.
        let dark = Rgba([0, 0, 0, 128]).source_over(Rgba::WHITE);
        assert_eq!(dark, Rgba([127, 127, 127, 255]));
    }
}
