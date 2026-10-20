//! Frames: decoded data ready for filters, mix and presentation
//! (doc 01 §3.3). Sample and pixel formats, minimal colorimetry, and the
//! hardware-frame escape hatch that names no platform.

use crate::ids::GenerationId;
use crate::packet::SharedBytes;
use crate::time::{Duration, Stamp, TimeSpan};

/// Interleaved PCM sample format (doc 01 §3.3 `AudioFormat`).
///
/// Depth and signedness are explicit — no "whatever the decoder felt
/// like". Planar layouts are expressed through
/// [`AudioLayout`](self::AudioLayout), not a format-multiplication table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SampleFormat {
    /// Unsigned 8-bit.
    U8,
    /// Signed 16-bit, native endian.
    S16,
    /// Signed 24-bit (3 bytes), native endian.
    S24,
    /// Signed 32-bit, native endian.
    S32,
    /// IEEE 754 single precision.
    F32,
    /// IEEE 754 double precision.
    F64,
}

impl SampleFormat {
    /// Bytes per sample.
    pub fn bytes_per_sample(self) -> u32 {
        match self {
            SampleFormat::U8 => 1,
            SampleFormat::S16 => 2,
            SampleFormat::S24 => 3,
            SampleFormat::S32 | SampleFormat::F32 => 4,
            SampleFormat::F64 => 8,
        }
    }
}

/// Sample memory layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioLayout {
    /// All channels interleaved in one buffer.
    Interleaved,
    /// One plane per channel, concatenated in the buffer.
    Planar,
}

/// Decoded audio shape: sample format, channel count, layout and rate
/// (doc 01 §3.3 `AudioFormat` — "sample format, channels, rate").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioFormat {
    /// Sample representation.
    pub sample: SampleFormat,
    /// Channel count (1 = mono, 2 = stereo, ...; channel *order* is a
    /// downstream negotiation concern — doc 03 owns mapping).
    pub channels: u32,
    /// Interleaved or planar.
    pub layout: AudioLayout,
    /// Samples per second.
    pub rate: u32,
}

impl AudioFormat {
    /// Bytes one frame of audio occupies (all channels, one sample
    /// instant).
    pub fn frame_bytes(&self) -> u64 {
        u64::from(self.channels) * u64::from(self.sample.bytes_per_sample())
    }

    /// How many complete sample-instants the buffer holds, or `None`
    /// when the buffer is not a whole number of frames (that is
    /// corruption at the decode boundary — data, not a guess).
    pub fn frames_in(&self, bytes: usize) -> Option<u64> {
        let frame = self.frame_bytes();
        if frame == 0 || !(bytes as u64).is_multiple_of(frame) {
            return None;
        }
        Some((bytes as u64) / frame)
    }
}

/// Pixel format for the renderer (doc 01 §3.3 `PixelFormat`). The set is
/// minimal on purpose: what pure-Rust decoders emit and what GPUs accept;
/// exotic formats convert in the video filter stage, not in vocabulary
/// enums.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    /// 4:2:0 planar YUV, 1 plane Y + 1 interleaved UV (the GPU-native
    /// default).
    Nv12,
    /// 4:2:0 planar YUV, 3 planes (the codec-native default).
    I420,
    /// 4:2:2 packed YUYV.
    Yuyv422,
    /// 8-bit RGBA, straight alpha.
    Rgba8,
    /// 8-bit BGRA (the desktop-compositor favorite).
    Bgra8,
}

/// Color primaries (minimal colorimetry, doc 01 §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorPrimaries {
    /// ITU-R BT.709 (HD).
    Bt709,
    /// ITU-R BT.601 (SD).
    Bt601,
    /// ITU-R BT.2020 (UHD).
    Bt2020,
}

/// Transfer characteristics (minimal colorimetry, doc 01 §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransferCharacteristics {
    /// ITU-R BT.709.
    Bt709,
    /// SMPTE ST 2084 PQ (HDR).
    Pq,
    /// Hybrid Log-Gamma (HDR).
    Hlg,
    /// sRGB.
    Srgb,
}

/// YCbCr matrix (minimal colorimetry, doc 01 §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorMatrix {
    /// ITU-R BT.709.
    Bt709,
    /// ITU-R BT.601.
    Bt601,
    /// ITU-R BT.2020 non-constant luminance.
    Bt2020Ncl,
}

/// Luma range convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorRange {
    /// Studio/limited (16-235 in 8-bit).
    Limited,
    /// Full 0-255.
    Full,
}

/// Colorimetry for the renderer (doc 01 §3.3 `ColorDescription`, the MP
/// §11 GPU path's contract).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColorDescription {
    pub primaries: ColorPrimaries,
    pub transfer: TransferCharacteristics,
    pub matrix: ColorMatrix,
    pub range: ColorRange,
}

impl ColorDescription {
    /// BT.709 / limited — the conservative default.
    pub const BT709_LIMITED: ColorDescription = ColorDescription {
        primaries: ColorPrimaries::Bt709,
        transfer: TransferCharacteristics::Bt709,
        matrix: ColorMatrix::Bt709,
        range: ColorRange::Limited,
    };
}

/// Where a decoded video frame's bytes live (doc 01 §3.3).
///
/// `OpaqueHandle` carries hardware frames **without naming any platform**
/// `[PROPOSAL]`: the handle's meaning is a private contract between the
/// decoder adapter that minted it and the renderer that consumes it; the
/// vocabulary crate only guarantees it is not CPU memory.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FrameStorage {
    /// CPU memory (shared, zero-copy).
    Cpu(SharedBytes),
    /// A hardware surface handle (platform-neutral by construction).
    OpaqueHandle(u64),
}

/// One decoded subtitle cue (doc 01 §3.3 `SubtitleCue`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleCue {
    /// When the cue is on screen (either end may be `Missing` for
    /// open-ended live cues).
    pub span: TimeSpan,
    /// The cue payload — plain text in v1; styled/bitmap payloads are a
    /// Phase 5 vocabulary extension (doc 04 subtitles roadmap).
    pub text: String,
}

/// One decoded audio buffer (doc 01 §3.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioFrame {
    /// Sample format, channels, layout, rate.
    pub format: AudioFormat,
    /// PCM bytes.
    pub samples: SharedBytes,
    /// Presentation timestamp.
    pub pts: Stamp,
    /// Duration when known.
    pub duration: Option<Duration>,
    /// Demux generation (stale means dropped at the checkpoint).
    pub generation: GenerationId,
}

/// One decoded video picture (doc 01 §3.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoFrame {
    /// Pixel format.
    pub format: PixelFormat,
    /// `(width, height)` in pixels.
    pub size: (u32, u32),
    /// CPU memory or an opaque hardware handle.
    pub data: FrameStorage,
    /// Presentation timestamp.
    pub pts: Stamp,
    /// Colorimetry for the renderer (MP §11 GPU path).
    pub color: ColorDescription,
    /// Interlaced picture.
    pub interlaced: bool,
    /// Demux generation.
    pub generation: GenerationId,
}

/// One decoded subtitle update (doc 01 §3.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleFrame {
    /// The cues visible from this decode onward.
    pub cues: Vec<SubtitleCue>,
    /// Presentation timestamp of the update.
    pub pts: Stamp,
    /// Demux generation.
    pub generation: GenerationId,
}

/// A decoded frame of any kind (doc 01 §3.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    Audio(AudioFrame),
    Video(VideoFrame),
    Subtitle(SubtitleFrame),
}

impl Frame {
    /// Presentation timestamp of any frame kind.
    pub fn pts(&self) -> Stamp {
        match self {
            Frame::Audio(f) => f.pts,
            Frame::Video(f) => f.pts,
            Frame::Subtitle(f) => f.pts,
        }
    }

    /// Demux generation of any frame kind.
    pub fn generation(&self) -> GenerationId {
        match self {
            Frame::Audio(f) => f.generation,
            Frame::Video(f) => f.generation,
            Frame::Subtitle(f) => f.generation,
        }
    }

    /// Whether the frame is stale relative to `current`.
    pub fn is_stale(&self, current: GenerationId) -> bool {
        self.generation() < current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_format_frame_math() {
        let fmt = AudioFormat {
            sample: SampleFormat::S16,
            channels: 2,
            layout: AudioLayout::Interleaved,
            rate: 48_000,
        };
        assert_eq!(fmt.frame_bytes(), 4);
        assert_eq!(fmt.frames_in(4000), Some(1000));
        assert_eq!(
            fmt.frames_in(4001),
            None,
            "partial frame is data, not rounded"
        );
        let planar = AudioFormat {
            sample: SampleFormat::F32,
            channels: 2,
            layout: AudioLayout::Planar,
            rate: 48_000,
        };
        assert_eq!(planar.frame_bytes(), 8);
    }

    #[test]
    fn sample_format_byte_widths() {
        assert_eq!(SampleFormat::U8.bytes_per_sample(), 1);
        assert_eq!(SampleFormat::S24.bytes_per_sample(), 3);
        assert_eq!(SampleFormat::F64.bytes_per_sample(), 8);
    }

    #[test]
    fn frame_accessors_cover_all_kinds() {
        let g = GenerationId::new(7);
        let pts = Stamp::Present(
            crate::time::Timestamp::from_parts(1, 1, 1000).unwrap_or_else(|e| unreachable!("{e}")),
        );
        let audio = Frame::Audio(AudioFrame {
            format: AudioFormat {
                sample: SampleFormat::F32,
                channels: 1,
                layout: AudioLayout::Interleaved,
                rate: 44_100,
            },
            samples: SharedBytes::from_vec(vec![0; 4]),
            pts,
            duration: None,
            generation: g,
        });
        let video = Frame::Video(VideoFrame {
            format: PixelFormat::Nv12,
            size: (64, 64),
            data: FrameStorage::Cpu(SharedBytes::from_vec(vec![0; 64 * 64 * 3 / 2])),
            pts,
            color: ColorDescription::BT709_LIMITED,
            interlaced: false,
            generation: g,
        });
        let subs = Frame::Subtitle(SubtitleFrame {
            cues: vec![SubtitleCue {
                span: TimeSpan::UNKNOWN,
                text: "hello".to_string(),
            }],
            pts,
            generation: g,
        });
        for f in [&audio, &video, &subs] {
            assert_eq!(f.pts(), pts);
            assert_eq!(f.generation(), g);
            assert!(!f.is_stale(g));
            assert!(f.is_stale(g.next().unwrap_or_else(|e| unreachable!("{e}"))));
        }
    }

    #[test]
    fn opaque_handle_names_no_platform() {
        let storage = FrameStorage::OpaqueHandle(0xDEAD_BEEF);
        // The vocabulary contract: an opaque handle is just a number the
        // adapter/renderer pair understands — it renders without any
        // platform word and without dereferencing anything.
        let d = format!("{storage:?}");
        assert!(d.contains("OpaqueHandle"), "{d}");
        assert!(!d.to_lowercase().contains("d3d"), "{d}");
        assert!(!d.to_lowercase().contains("vaapi"), "{d}");
        assert!(!d.to_lowercase().contains("metal"), "{d}");
        assert!(!d.to_lowercase().contains("vulkan"), "{d}");
    }

    #[test]
    fn color_description_default_is_bt709_limited() {
        let c = ColorDescription::BT709_LIMITED;
        assert_eq!(c.primaries, ColorPrimaries::Bt709);
        assert_eq!(c.range, ColorRange::Limited);
    }
}
