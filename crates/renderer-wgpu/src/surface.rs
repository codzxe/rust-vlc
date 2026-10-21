//! Surfaces: where pixels land (doc 03 §4.11 a/b; doc 01 §2.6
//! "`RenderSurface` — opaque target, strategy per ADR-003").
//!
//! One trait, two worlds:
//!
//! - [`HeadlessSurface`] — an offscreen framebuffer that runs **everywhere
//!   a CPU does** (this crate's AC path; doc 03 §4.11 f "headless …
//!   renders frames to an offscreen buffer"): no GPU, no window, no
//!   display server. It is the testkit target and the fallback the
//!   Phase-7 chain formalizes ("auto-negotiated wgpu backend → software
//!   present", card 7.C1).
//! - The windowed surface — created from a platform window handle by
//!   the L5 adapters or the wgpu backend (doc 01 §2.7; ADR-003 decides
//!   the strategy). v0 types the contract ([`RenderSurface`],
//!   [`SurfaceConfig`], [`PresentMode`]) and shape-pins the wgpu
//!   mappings; the construction path joins with the Lead-gated wgpu
//!   import (crate docs, allowlist §5).
//!
//! # The CPU-driven target protocol
//!
//! [`fill_rect`](RenderSurface::fill_rect) / [`write_rect`](RenderSurface::write_rect)
//! are the CPU-driven write protocol every surface must honor (a GPU
//! surface implements them through staging uploads — correct, possibly
//! slow). The hardware compositor may bypass them with its own draw
//! path; the v0 software compositor uses **only** the contract
//! surface, so the acceptance proof runs entirely through the public
//! protocol.
//!
//! # vsync (card 2.D1)
//!
//! [`PresentMode`] mirrors wgpu's vocabulary: `Fifo` (vsync, always
//! supported), `Mailbox` (vsync-ish triple buffer), `Immediate`
//! (tearing). [`SurfaceConfig::validate`] refuses an unsupported mode
//! with a typed error naming what *is* supported — configuration is
//! explicit, never silently downgraded.

use crate::color::{Rgba, SurfaceFormat};
use crate::error::{RendererError, RendererResult};
use crate::params::{Extent2D, Rect};

/// The vsync policy for presenting (wgpu's `PresentMode` vocabulary,
/// shape-pinned by unit test).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresentMode {
    /// Vsync, queue-ordered: the present blocks until the next refresh
    /// slot. The only universally-supported mode; the headless surface
    /// reports exactly this one.
    Fifo,
    /// Vsync, latest-wins: triple-buffered, never blocks, may drop
    /// intermediate frames.
    Mailbox,
    /// No vsync: presents immediately, may tear.
    Immediate,
}

impl PresentMode {
    /// The exact `wgpu::PresentMode` enum variant name (shape-level
    /// contract, unit-pinned before the dependency exists).
    #[must_use = "the hardware backend configures the surface from this name"]
    pub fn wgpu_present_mode(self) -> &'static str {
        match self {
            PresentMode::Fifo => "Fifo",
            PresentMode::Mailbox => "Mailbox",
            PresentMode::Immediate => "Immediate",
        }
    }

    /// Whether this mode synchronizes presents to the display refresh
    /// (Fifo and Mailbox do; Immediate tears).
    #[must_use = "pure classification"]
    pub fn is_vsync(self) -> bool {
        !matches!(self, PresentMode::Immediate)
    }
}

/// Surface configuration: format + vsync policy (card 2.D1 "sRGB
/// present, vsync").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceConfig {
    /// The surface's pixel format (both v0 variants are sRGB-encoded).
    pub format: SurfaceFormat,
    /// The present (vsync) mode; validated against the surface's
    /// supported set before use.
    pub present_mode: PresentMode,
}

impl SurfaceConfig {
    /// The default: sRGB BGRA (the compositor favorite) + Fifo vsync.
    pub const DEFAULT: SurfaceConfig = SurfaceConfig {
        format: SurfaceFormat::Bgra8Srgb,
        present_mode: PresentMode::Fifo,
    };

    /// Validate this configuration against a surface's supported
    /// present modes (refuses unsupported vsync modes loudly).
    ///
    /// # Errors
    /// [`RendererError::PresentModeUnsupported`] naming the supported
    /// set; the caller re-configures or picks another surface — never a
    /// silent downgrade.
    pub fn validate(&self, supported: &'static [PresentMode]) -> RendererResult<()> {
        if supported.contains(&self.present_mode) {
            Ok(())
        } else {
            Err(RendererError::PresentModeUnsupported {
                requested: self.present_mode,
                supported,
            })
        }
    }
}

impl Default for SurfaceConfig {
    fn default() -> Self {
        SurfaceConfig::DEFAULT
    }
}

/// Where pixels land (doc 03 §4.11 b; doc 01 §2.6 "opaque target").
///
/// Implemented by [`HeadlessSurface`] (offscreen, everywhere) and, on
/// the hardware path, by the wgpu surface the L5 adapters/windowing
/// host creates (ADR-003). All methods are the CPU-driven protocol —
/// object-safe by design so the renderer owns its target as
/// `Box<dyn RenderSurface>`.
///
/// # Contract for implementors
///
/// - **Resize is non-fatal by construction** (doc 03 §4.11 d): a new
///   extent reallocates; it never loses the surface.
/// - **Loss is explicit**: a closed window or GPU reset surfaces as
///   [`RendererError::SurfaceLost`] from any method; the session
///   degrades or reattaches — never crashes.
/// - **Fifo presents are synchronous**: `present` returns with the
///   frame logically on the surface.
pub trait RenderSurface: Send {
    /// The current target extent in pixels.
    fn extent(&self) -> Extent2D;
    /// The surface's pixel format.
    fn format(&self) -> SurfaceFormat;
    /// The present modes this surface supports (headless: Fifo only).
    fn supported_present_modes(&self) -> &'static [PresentMode];
    /// Reallocate to a new extent (non-fatal by construction).
    ///
    /// # Errors
    /// [`RendererError::ZeroExtent`] for a zero component.
    fn resize(&mut self, size: Extent2D) -> RendererResult<()>;
    /// Clear a rectangle to one color (the background pass).
    ///
    /// # Errors
    /// [`RendererError::ZeroExtent`] for a zero-sized rectangle or a
    /// rectangle outside the surface extent.
    fn fill_rect(&mut self, rect: Rect, color: Rgba) -> RendererResult<()>;
    /// Write an RGBA8 block (row-major, `rect.width * rect.height * 4`
    /// bytes) at a rectangle.
    ///
    /// # Errors
    /// [`RendererError::ZeroExtent`] for a zero-sized or out-of-bounds
    /// rectangle; [`RendererError::UploadSizeMismatch`] when the pixel
    /// buffer is shorter than the rectangle demands (never a partial
    /// write).
    fn write_rect(&mut self, rect: Rect, pixels: &[u8]) -> RendererResult<()>;
    /// Present the current framebuffer (Fifo: synchronous swap).
    ///
    /// # Errors
    /// [`RendererError::SurfaceLost`] when the target is gone.
    fn present(&mut self) -> RendererResult<()>;
    /// Read the framebuffer back as RGBA8 bytes (`extent` rows, no
    /// padding) — the screenshot/AC path (doc 03 §4.11 f).
    ///
    /// # Errors
    /// [`RendererError::SurfaceLost`] when the target is gone.
    fn readback_rgba(&self) -> RendererResult<Vec<u8>>;
}

/// The offscreen surface: an in-memory framebuffer in the configured
/// native format (BGRA bytes for [`SurfaceFormat::Bgra8Srgb`]), read
/// back as RGBA. Runs on any CPU — the deterministic headless target
/// for tests, benchmarks and the 7.C1 software-present fallback.
///
/// There is no compositor clock here: `present` under Fifo is the
/// synchronous contract's trivial case (the framebuffer *is* the front
/// buffer), so the presented timestamp is exactly the scheduled slot
/// (see [`crate::renderer::Renderer`]'s `Presenter` notes).
#[derive(Debug)]
pub struct HeadlessSurface {
    extent: Extent2D,
    config: SurfaceConfig,
    /// Native-format framebuffer, `extent.height` rows of
    /// `extent.width * 4` bytes, no padding.
    buffer: Vec<u8>,
    /// Present counter (Fifo bookkeeping: one slot per present).
    presented: u64,
}

impl HeadlessSurface {
    /// Create an offscreen surface of the given extent and
    /// configuration.
    ///
    /// # Errors
    /// [`RendererError::ZeroExtent`] for a zero component;
    /// [`RendererError::PresentModeUnsupported`] for a non-Fifo mode
    /// (headless reports Fifo only — validation is part of
    /// construction, not a runtime surprise).
    pub fn new(extent: Extent2D, config: SurfaceConfig) -> RendererResult<HeadlessSurface> {
        config.validate(&[PresentMode::Fifo])?;
        if extent.width == 0 || extent.height == 0 {
            return Err(RendererError::ZeroExtent {
                extent,
                from_frame: false,
            });
        }
        let bytes = extent.rgba8_bytes()?;
        Ok(HeadlessSurface {
            extent,
            config,
            buffer: vec![0; bytes as usize],
            presented: 0,
        })
    }

    /// The surface's configuration (format + present mode).
    #[must_use = "pure accessor"]
    pub fn config(&self) -> SurfaceConfig {
        self.config
    }

    /// How many Fifo slots have been presented (test observability).
    #[must_use = "pure accessor"]
    pub fn presented_count(&self) -> u64 {
        self.presented
    }

    /// Bounds-check a rectangle and return its (stride, row bytes).
    fn checked_span(&self, rect: &Rect) -> RendererResult<(usize, usize)> {
        if rect.width == 0 || rect.height == 0 {
            return Err(RendererError::ZeroExtent {
                extent: rect.extent(),
                from_frame: false,
            });
        }
        if rect.x + rect.width > self.extent.width || rect.y + rect.height > self.extent.height {
            return Err(RendererError::ZeroExtent {
                extent: rect.extent(),
                from_frame: false,
            });
        }
        let stride = self.extent.width as usize * self.config.format.bytes_per_pixel() as usize;
        Ok((stride, rect.width as usize * 4))
    }
}

impl RenderSurface for HeadlessSurface {
    fn extent(&self) -> Extent2D {
        self.extent
    }

    fn format(&self) -> SurfaceFormat {
        self.config.format
    }

    fn supported_present_modes(&self) -> &'static [PresentMode] {
        &[PresentMode::Fifo]
    }

    fn resize(&mut self, size: Extent2D) -> RendererResult<()> {
        if size.width == 0 || size.height == 0 {
            return Err(RendererError::ZeroExtent {
                extent: size,
                from_frame: false,
            });
        }
        let bytes = size.rgba8_bytes()?;
        self.extent = size;
        self.buffer = vec![0; bytes as usize];
        Ok(())
    }

    fn fill_rect(&mut self, rect: Rect, color: Rgba) -> RendererResult<()> {
        let (stride, _) = self.checked_span(&rect)?;
        let packed = self.config.format.pack(color);
        for row in 0..rect.height as usize {
            let base = (rect.y as usize + row) * stride + rect.x as usize * 4;
            for col in 0..rect.width as usize {
                let at = base + col * 4;
                self.buffer[at..at + 4].copy_from_slice(&packed);
            }
        }
        Ok(())
    }

    fn write_rect(&mut self, rect: Rect, pixels: &[u8]) -> RendererResult<()> {
        let (stride, row_bytes) = self.checked_span(&rect)?;
        let needed = row_bytes * rect.height as usize;
        if pixels.len() < needed {
            return Err(RendererError::UploadSizeMismatch {
                expected: needed as u64,
                found: pixels.len() as u64,
            });
        }
        let format = self.config.format;
        for row in 0..rect.height as usize {
            let src = &pixels[row * row_bytes..(row + 1) * row_bytes];
            let base = (rect.y as usize + row) * stride + rect.x as usize * 4;
            for (col, rgba) in src.as_chunks::<4>().0.iter().enumerate() {
                let pixel = Rgba([rgba[0], rgba[1], rgba[2], rgba[3]]);
                let at = base + col * 4;
                self.buffer[at..at + 4].copy_from_slice(&format.pack(pixel));
            }
        }
        Ok(())
    }

    fn present(&mut self) -> RendererResult<()> {
        // The offscreen target cannot be lost; Fifo's synchronous
        // contract holds trivially (single buffered).
        self.presented = self.presented.saturating_add(1);
        Ok(())
    }

    fn readback_rgba(&self) -> RendererResult<Vec<u8>> {
        let stride = self.extent.width as usize * self.config.format.bytes_per_pixel() as usize;
        let mut out = Vec::with_capacity(self.buffer.len());
        let format = self.config.format;
        for row in self.buffer.chunks_exact(stride) {
            for pixel in row.as_chunks::<4>().0 {
                let unpacked = format.unpack([pixel[0], pixel[1], pixel[2], pixel[3]]);
                out.extend_from_slice(&unpacked.0);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::{Must, assert_err};

    fn surface(w: u32, h: u32) -> HeadlessSurface {
        HeadlessSurface::new(
            Extent2D {
                width: w,
                height: h,
            },
            SurfaceConfig {
                format: SurfaceFormat::Bgra8Srgb,
                present_mode: PresentMode::Fifo,
            },
        )
        .must()
    }

    #[test]
    fn present_mode_wgpu_names_and_vsync_classification() {
        assert_eq!(PresentMode::Fifo.wgpu_present_mode(), "Fifo");
        assert_eq!(PresentMode::Mailbox.wgpu_present_mode(), "Mailbox");
        assert_eq!(PresentMode::Immediate.wgpu_present_mode(), "Immediate");
        assert!(PresentMode::Fifo.is_vsync());
        assert!(PresentMode::Mailbox.is_vsync());
        assert!(!PresentMode::Immediate.is_vsync());
    }

    #[test]
    fn config_validation_refuses_unsupported_modes_loudly() {
        let ok = SurfaceConfig {
            format: SurfaceFormat::Bgra8Srgb,
            present_mode: PresentMode::Fifo,
        };
        ok.validate(&[PresentMode::Fifo]).must();
        let mailbox = SurfaceConfig {
            format: SurfaceFormat::Bgra8Srgb,
            present_mode: PresentMode::Mailbox,
        };
        let err = assert_err(
            mailbox.validate(&[PresentMode::Fifo]),
            "mailbox on headless",
        );
        assert_eq!(
            err,
            RendererError::PresentModeUnsupported {
                requested: PresentMode::Mailbox,
                supported: &[PresentMode::Fifo],
            }
        );
    }

    #[test]
    fn headless_construction_validates_extent_and_mode() {
        let zero = assert_err(
            HeadlessSurface::new(
                Extent2D {
                    width: 0,
                    height: 10,
                },
                SurfaceConfig::DEFAULT,
            ),
            "zero extent",
        );
        assert_eq!(
            zero,
            RendererError::ZeroExtent {
                extent: Extent2D {
                    width: 0,
                    height: 10
                },
                from_frame: false
            }
        );

        let tearing = assert_err(
            HeadlessSurface::new(
                Extent2D {
                    width: 8,
                    height: 8,
                },
                SurfaceConfig {
                    format: SurfaceFormat::Bgra8Srgb,
                    present_mode: PresentMode::Immediate,
                },
            ),
            "immediate mode on headless",
        );
        assert_eq!(
            tearing,
            RendererError::PresentModeUnsupported {
                requested: PresentMode::Immediate,
                supported: &[PresentMode::Fifo],
            }
        );
    }

    #[test]
    fn fill_write_readback_round_trip_through_native_order() {
        let mut s = surface(4, 2);
        s.fill_rect(
            Rect::covering(Extent2D {
                width: 4,
                height: 2,
            }),
            Rgba::BLACK,
        )
        .must();
        // A 2x1 block of red at (1,0).
        let red = [255u8, 0, 0, 255, 255, 0, 0, 255];
        s.write_rect(
            Rect {
                x: 1,
                y: 0,
                width: 2,
                height: 1,
            },
            &red,
        )
        .must();
        s.present().must();
        assert_eq!(s.presented_count(), 1);

        let rgba = s.readback_rgba().must();
        assert_eq!(rgba.len(), 4 * 2 * 4);
        let px = |i: usize| {
            Rgba([
                rgba[i * 4],
                rgba[i * 4 + 1],
                rgba[i * 4 + 2],
                rgba[i * 4 + 3],
            ])
        };
        assert_eq!(px(0), Rgba::BLACK);
        assert_eq!(px(1), Rgba::from_components(255, 0, 0, 255));
        assert_eq!(px(2), Rgba::from_components(255, 0, 0, 255));
        assert_eq!(px(3), Rgba::BLACK);
        assert_eq!(px(4), Rgba::BLACK);

        // The native buffer really is BGRA-ordered.
        assert_eq!(&s.buffer[4..8], &[0, 0, 255, 255], "BGRa order for red");
    }

    #[test]
    fn write_rect_rejects_short_buffers_and_bad_rects() {
        let mut s = surface(4, 2);
        let short = assert_err(
            s.write_rect(
                Rect {
                    x: 0,
                    y: 0,
                    width: 2,
                    height: 2,
                },
                &[0u8; 7],
            ),
            "short pixel buffer",
        );
        assert_eq!(
            short,
            RendererError::UploadSizeMismatch {
                expected: 16,
                found: 7
            }
        );

        let oob = assert_err(
            s.write_rect(
                Rect {
                    x: 3,
                    y: 0,
                    width: 2,
                    height: 1,
                },
                &[0u8; 8],
            ),
            "out-of-bounds rect",
        );
        assert!(matches!(oob, RendererError::ZeroExtent { .. }));

        let zero = assert_err(
            s.fill_rect(
                Rect {
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 1,
                },
                Rgba::BLACK,
            ),
            "zero-sized rect",
        );
        assert!(matches!(zero, RendererError::ZeroExtent { .. }));
    }

    #[test]
    fn resize_is_non_fatal_and_reallocates() {
        let mut s = surface(2, 2);
        s.fill_rect(
            Rect::covering(Extent2D {
                width: 2,
                height: 2,
            }),
            Rgba::WHITE,
        )
        .must();
        s.resize(Extent2D {
            width: 4,
            height: 1,
        })
        .must();
        assert_eq!(
            s.extent(),
            Extent2D {
                width: 4,
                height: 1
            }
        );
        assert_eq!(s.buffer.len(), 16);
        // New buffer starts cleared.
        assert!(s.buffer.iter().all(|&b| b == 0));
        // Zero resize is the typed error, not a loss.
        let err = assert_err(
            s.resize(Extent2D {
                width: 0,
                height: 1,
            }),
            "zero resize",
        );
        assert!(matches!(err, RendererError::ZeroExtent { .. }));
    }

    #[test]
    fn rgba_surface_format_skips_the_swizzle() {
        let mut s = HeadlessSurface::new(
            Extent2D {
                width: 1,
                height: 1,
            },
            SurfaceConfig {
                format: SurfaceFormat::Rgba8Srgb,
                present_mode: PresentMode::Fifo,
            },
        )
        .must();
        s.write_rect(
            Rect {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
            &[1, 2, 3, 4],
        )
        .must();
        assert_eq!(&s.buffer[..], &[1, 2, 3, 4]);
        assert_eq!(s.readback_rgba().must(), vec![1, 2, 3, 4]);
    }
}
