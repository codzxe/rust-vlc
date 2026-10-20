//! Typed errors for the renderer (doc 03 §4.11 d; doc 01 §3.4 stage
//! `Present` at the engine boundary).
//!
//! The renderer keeps its own stage vocabulary (finer than the engine's
//! `Present`) so every failure names the exact pipeline leg it died on;
//! [`RendererError::to_engine_error`] folds it into the shared
//! [`EngineError`] the [`Presenter`] contract speaks (stage `Present`,
//! structured kinds, dotted context codes — never a panic, MP §8).

use media_types::error::{
    CorruptDetail, DeviceCause, DeviceSlot, EngineError, ErrorContext, ErrorKind,
};
use media_types::frame::PixelFormat;

use crate::params::Extent2D;
use crate::surface::PresentMode;

/// Which leg of the render pipeline failed (finer than the engine's
/// `Present` stage; carried on every [`RendererError`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RendererStage {
    /// Surface construction / configuration / resize.
    Configure,
    /// CPU → staging → texture transfer of frame bytes.
    Upload,
    /// Placing the frame onto the target (aspect/crop/background).
    Composite,
    /// Presenting the frame to the surface (swap/flip, vsync policy).
    Present,
    /// Reading pixels back from an offscreen surface (screenshots, AC).
    Readback,
    /// Encoding a readback to PNG bytes (screenshot facility).
    Encode,
}

impl RendererStage {
    /// Stable lowercase name for machine-readable logs.
    #[must_use = "the stage name is what log filters key on"]
    pub fn as_str(self) -> &'static str {
        match self {
            RendererStage::Configure => "configure",
            RendererStage::Upload => "upload",
            RendererStage::Composite => "composite",
            RendererStage::Present => "present",
            RendererStage::Readback => "readback",
            RendererStage::Encode => "encode",
        }
    }
}

impl std::fmt::Display for RendererStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Every failure mode of the v0 renderer, stage-tagged with a stable
/// dotted context code (doc 01 §3.4 `ErrorContext` shape).
///
/// Surface loss is its own variant because doc 03 §4.11 d makes it the
/// session-degradation signal (audio continues, video degrades or
/// reattaches — never fatal here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RendererError {
    /// The surface went away (window closed, GPU reset). Non-fatal by
    /// design: the session degrades to audio-only or reattaches.
    SurfaceLost,
    /// A width or height of zero where pixels are required (surface,
    /// frame, or resize request). Zero extents are unsupported values,
    /// not crashes.
    ZeroExtent {
        /// The offending extent (one or both components zero).
        extent: Extent2D,
        /// Whether this was a frame (`true`) or a surface/target.
        from_frame: bool,
    },
    /// The frame's pixel format is not one v0 uploads (the YUV family
    /// converts in the GPU shader path — doc 03 §4.11 e — a later card;
    /// the error names the format so callers can route).
    UploadFormatUnsupported {
        /// The frame's format (from media-types).
        format: PixelFormat,
    },
    /// The frame carries an `OpaqueHandle` (hardware surface): import
    /// is the wgpu backend's job (doc 01 §2.6), not the v0 software
    /// path's.
    OpaqueHandleUnsupported,
    /// The frame buffer's byte count does not match its declared
    /// width × height × 4 — decode-boundary corruption, never guessed.
    UploadSizeMismatch {
        /// Bytes the format demands.
        expected: u64,
        /// Bytes the buffer holds.
        found: u64,
    },
    /// The configured present mode is not supported by the surface
    /// (e.g. Mailbox on the headless surface, which is Fifo-only).
    PresentModeUnsupported {
        /// What was requested.
        requested: PresentMode,
        /// What the surface supports (headless: Fifo only).
        supported: &'static [PresentMode],
    },
    /// A configured crop rectangle does not intersect the frame.
    CropEmpty {
        /// The crop rectangle (in frame pixels).
        crop: crate::params::Rect,
        /// The frame's extent.
        frame: Extent2D,
    },
    /// Readback buffer size disagrees with the surface extent.
    ReadbackSizeMismatch {
        /// Bytes the extent demands.
        expected: u64,
        /// Bytes offered.
        found: u64,
    },
    /// PNG encode input size disagrees with the declared dimensions.
    EncodeSizeMismatch {
        /// Bytes width × height × 4 demands.
        expected: u64,
        /// Bytes offered.
        found: u64,
    },
    /// The renderer was asked to render before a surface was attached.
    NoSurface,
}

impl RendererError {
    /// The pipeline leg this error belongs to.
    #[must_use = "stage tags are part of the log contract"]
    pub fn stage(&self) -> RendererStage {
        match self {
            RendererError::SurfaceLost
            | RendererError::ZeroExtent { .. }
            | RendererError::NoSurface => RendererStage::Configure,
            RendererError::PresentModeUnsupported { .. } => RendererStage::Present,
            RendererError::UploadFormatUnsupported { .. }
            | RendererError::OpaqueHandleUnsupported
            | RendererError::UploadSizeMismatch { .. } => RendererStage::Upload,
            RendererError::CropEmpty { .. } => RendererStage::Composite,
            RendererError::ReadbackSizeMismatch { .. } => RendererStage::Readback,
            RendererError::EncodeSizeMismatch { .. } => RendererStage::Encode,
        }
    }

    /// Stable dotted context code (doc 01 §3.4; additive-only once
    /// released).
    #[must_use = "context codes are the machine-readable identity"]
    pub fn code(&self) -> &'static str {
        match self {
            RendererError::SurfaceLost => "render.surface.lost",
            RendererError::ZeroExtent {
                from_frame: true, ..
            } => "render.frame.zero-extent",
            RendererError::ZeroExtent {
                from_frame: false, ..
            } => "render.surface.zero-extent",
            RendererError::UploadFormatUnsupported { .. } => "render.upload.format-unsupported",
            RendererError::OpaqueHandleUnsupported => "render.upload.opaque-handle",
            RendererError::UploadSizeMismatch { .. } => "render.upload.size-mismatch",
            RendererError::PresentModeUnsupported { .. } => "render.present.mode-unsupported",
            RendererError::CropEmpty { .. } => "render.params.crop-empty",
            RendererError::ReadbackSizeMismatch { .. } => "render.readback.size-mismatch",
            RendererError::EncodeSizeMismatch { .. } => "render.encode.size-mismatch",
            RendererError::NoSurface => "render.surface.missing",
        }
    }

    /// Fold into the shared engine error: stage `Present`, an MP §8
    /// kind, this error as the chain source (doc 01 §3.4).
    #[must_use = "callers of Presenter::present must receive EngineError"]
    pub fn to_engine_error(self) -> EngineError {
        let kind = match &self {
            RendererError::SurfaceLost => ErrorKind::Device {
                which: DeviceSlot::Display,
                cause: DeviceCause::Lost,
            },
            RendererError::PresentModeUnsupported { .. } => ErrorKind::Device {
                which: DeviceSlot::Display,
                cause: DeviceCause::FormatUnsupported,
            },
            RendererError::UploadFormatUnsupported { .. }
            | RendererError::OpaqueHandleUnsupported
            | RendererError::ZeroExtent { .. }
            | RendererError::CropEmpty { .. }
            | RendererError::NoSurface => ErrorKind::Unsupported { format: None },
            RendererError::UploadSizeMismatch { expected, found } => ErrorKind::Corrupt {
                detail: CorruptDetail::Truncated {
                    expected: *expected,
                    found: *found,
                },
            },
            RendererError::ReadbackSizeMismatch { expected, found }
            | RendererError::EncodeSizeMismatch { expected, found } => ErrorKind::Corrupt {
                detail: CorruptDetail::Truncated {
                    expected: *expected,
                    found: *found,
                },
            },
        };
        EngineError::new(
            media_types::error::PipelineStage::Present,
            kind,
            ErrorContext::new(self.code()),
        )
        .with_source(self)
    }
}

impl std::fmt::Display for RendererError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Structured data only (the no-secrets rule holds by
        // construction: every payload is a number or an enum name).
        match self {
            RendererError::SurfaceLost => write!(f, "surface lost"),
            RendererError::ZeroExtent { extent, from_frame } => write!(
                f,
                "zero extent {}x{} on the {}",
                extent.width,
                extent.height,
                if *from_frame { "frame" } else { "surface" }
            ),
            RendererError::UploadFormatUnsupported { format } => {
                write!(f, "upload: pixel format unsupported in v0 ({format:?})")
            }
            RendererError::OpaqueHandleUnsupported => {
                write!(f, "upload: opaque hardware handle needs the wgpu path")
            }
            RendererError::UploadSizeMismatch { expected, found } => write!(
                f,
                "upload: buffer is {found} bytes, format needs {expected}"
            ),
            RendererError::PresentModeUnsupported {
                requested,
                supported,
            } => write!(
                f,
                "present mode {requested:?} unsupported (supported: {:?})",
                supported
            ),
            RendererError::CropEmpty { crop, frame } => write!(
                f,
                "crop {}x{} at ({},{}) does not intersect the {}x{} frame",
                crop.width, crop.height, crop.x, crop.y, frame.width, frame.height
            ),
            RendererError::ReadbackSizeMismatch { expected, found } => write!(
                f,
                "readback: buffer is {found} bytes, extent needs {expected}"
            ),
            RendererError::EncodeSizeMismatch { expected, found } => write!(
                f,
                "encode: buffer is {found} bytes, dimensions need {expected}"
            ),
            RendererError::NoSurface => write!(f, "no surface attached"),
        }
    }
}

impl std::error::Error for RendererError {}

/// `Result` alias for the renderer's internal plumbing.
pub type RendererResult<T> = Result<T, RendererError>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::Rect;

    #[test]
    fn every_error_names_stage_and_stable_code() {
        let cases: [(RendererError, RendererStage, &str); 10] = [
            (
                RendererError::SurfaceLost,
                RendererStage::Configure,
                "render.surface.lost",
            ),
            (
                RendererError::ZeroExtent {
                    extent: Extent2D {
                        width: 0,
                        height: 4,
                    },
                    from_frame: false,
                },
                RendererStage::Configure,
                "render.surface.zero-extent",
            ),
            (
                RendererError::UploadFormatUnsupported {
                    format: PixelFormat::Nv12,
                },
                RendererStage::Upload,
                "render.upload.format-unsupported",
            ),
            (
                RendererError::OpaqueHandleUnsupported,
                RendererStage::Upload,
                "render.upload.opaque-handle",
            ),
            (
                RendererError::UploadSizeMismatch {
                    expected: 16,
                    found: 12,
                },
                RendererStage::Upload,
                "render.upload.size-mismatch",
            ),
            (
                RendererError::PresentModeUnsupported {
                    requested: PresentMode::Mailbox,
                    supported: &[PresentMode::Fifo],
                },
                RendererStage::Present,
                "render.present.mode-unsupported",
            ),
            (
                RendererError::CropEmpty {
                    crop: Rect {
                        x: 9,
                        y: 9,
                        width: 2,
                        height: 2,
                    },
                    frame: Extent2D {
                        width: 8,
                        height: 8,
                    },
                },
                RendererStage::Composite,
                "render.params.crop-empty",
            ),
            (
                RendererError::ReadbackSizeMismatch {
                    expected: 64,
                    found: 60,
                },
                RendererStage::Readback,
                "render.readback.size-mismatch",
            ),
            (
                RendererError::EncodeSizeMismatch {
                    expected: 64,
                    found: 60,
                },
                RendererStage::Encode,
                "render.encode.size-mismatch",
            ),
            (
                RendererError::NoSurface,
                RendererStage::Configure,
                "render.surface.missing",
            ),
        ];
        for (error, stage, code) in cases {
            assert_eq!(error.stage(), stage, "{error:?}");
            assert_eq!(error.code(), code, "{error:?}");
            // Display renders without panicking and names its code's
            // first word family (spot check, not a format lock).
            assert!(!error.to_string().is_empty());
        }
    }

    #[test]
    fn engine_error_folds_stage_present_with_structured_kinds() {
        let lost = RendererError::SurfaceLost.to_engine_error();
        assert_eq!(lost.stage, media_types::error::PipelineStage::Present);
        assert_eq!(
            lost.kind,
            ErrorKind::Device {
                which: DeviceSlot::Display,
                cause: DeviceCause::Lost
            }
        );
        assert_eq!(lost.context.code, "render.surface.lost");

        let truncated = RendererError::UploadSizeMismatch {
            expected: 16,
            found: 12,
        }
        .to_engine_error();
        assert_eq!(
            truncated.kind,
            ErrorKind::Corrupt {
                detail: CorruptDetail::Truncated {
                    expected: 16,
                    found: 12
                }
            }
        );

        let mode = RendererError::PresentModeUnsupported {
            requested: PresentMode::Mailbox,
            supported: &[PresentMode::Fifo],
        }
        .to_engine_error();
        assert_eq!(
            mode.kind,
            ErrorKind::Device {
                which: DeviceSlot::Display,
                cause: DeviceCause::FormatUnsupported
            }
        );

        // The chain source delegates to std (EngineError::source).
        let src = std::error::Error::source(&lost);
        assert!(src.is_some(), "renderer error rides the chain root");
    }

    #[test]
    fn stage_names_are_stable_lowercase() {
        assert_eq!(RendererStage::Configure.as_str(), "configure");
        assert_eq!(RendererStage::Encode.as_str(), "encode");
        assert_eq!(RendererStage::Upload.to_string(), "upload");
    }
}
