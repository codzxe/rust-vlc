//! Sample-format conversion: the mixer format's edges (doc 03 §4.8 a
//! — stage A "format normalization" and stage D "device adaptation").
//!
//! **Stage A (decode side):** any decoded layout (interleaved or
//! planar; u8/s16/s24/s32/f32/f64) becomes f32 **planar** planes —
//! the mixer format (doc 03 §4.8 e, floating-point planar
//! `[PROPOSAL]`). The int→f32 scaling is the exact power-of-two
//! divide (`v / 2^(bits-1)`), the same pinned conversion the fixture
//! corpus's `[audio_probe]` provenance uses — every int sample is
//! exact in f32, so the conversion is bit-deterministic on every
//! host.
//!
//! **Stage D (device side):** the mixed f32 planes become
//! **interleaved** device bytes. v0 devices speak f32 (doc 03 §4.9's
//! `RenderCallback` is `&mut [f32]`); the s16 target exists as the
//! tested integer seam for the backends later cards add, with the
//! saturating-clamp inverse `clamp(round(x · 2^(bits-1)))` — the
//! exact algebraic inverse of stage A, proven by the full-range
//! round-trip test below. The soft limiter at stage D end is
//! `[PROPOSAL]` (doc 03 §4.8 e) and owed to the hardening card; what
//! ships is hard saturation at the int boundary (a conversion
//! correctness rule, not a limiter).
//!
//! Frame counts are conserved through every function here (the
//! clock-correctness invariant, doc 03 §4.8 e).

use media_types::EngineError;
use media_types::frame::{AudioFrame, AudioLayout, SampleFormat};

use crate::error;

/// Validate plane geometry: one plane per channel, one common length.
fn check_planes(planes: &[Vec<f32>], channels: u32) -> Result<usize, EngineError> {
    if planes.len() != channels as usize {
        return Err(error::plane_mismatch("audio.planar_plane_mismatch"));
    }
    let frames = planes.first().map_or(0, Vec::len);
    if planes.iter().any(|p| p.len() != frames) {
        return Err(error::plane_mismatch("audio.planar_plane_mismatch"));
    }
    Ok(frames)
}

/// Decode one scalar of `sample` format from little-endian bytes at
/// `data[i..]` into f32 (the exact power-of-two scaling).
fn scalar_to_f32(sample: SampleFormat, data: &[u8], i: usize) -> f32 {
    match sample {
        SampleFormat::U8 => (f32::from(data[i]) - 128.0) / 128.0,
        SampleFormat::S16 => {
            let v = i16::from_le_bytes([data[i], data[i + 1]]);
            f32::from(v) / 32_768.0
        }
        SampleFormat::S24 => {
            let b0 = u32::from(data[i]);
            let b1 = u32::from(data[i + 1]);
            let b2 = u32::from(data[i + 2]);
            // Little-endian 24-bit, sign-extended: lift the sign bit
            // (bit 23) to bit 31, then arithmetic-shift back down.
            let v24 = b0 | (b1 << 8) | (b2 << 16);
            let v = ((v24 << 8) as i32) >> 8;
            v as f32 / 8_388_608.0
        }
        SampleFormat::S32 => {
            let v = i32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
            v as f32 / 2_147_483_648.0
        }
        SampleFormat::F32 => f32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]),
        SampleFormat::F64 => {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&data[i..i + 8]);
            f64::from_le_bytes(bytes) as f32
        }
    }
}

/// Stage A: a decoded [`AudioFrame`] of any supported format/layout →
/// f32 planar planes. `planes` is resized to the frame's channel
/// count and its capacity is reused across calls (doc 03 §4.8 c — no
/// per-frame allocation after warm-up). Returns the frame count.
///
/// # Errors
/// [`Corrupt`](media_types::ErrorKind::Corrupt) with code
/// `audio.partial_frame` when the payload is not a whole number of
/// frames (data, not a guess — the boundary never rounds).
pub fn frame_to_f32_planes(
    frame: &AudioFrame,
    planes: &mut Vec<Vec<f32>>,
) -> Result<usize, EngineError> {
    let fmt = frame.format;
    let data = frame.samples.as_slice();
    let bps = fmt.sample.bytes_per_sample() as usize;
    let frame_bytes = fmt.frame_bytes() as usize;
    if !data.len().is_multiple_of(frame_bytes) {
        return Err(error::partial_frame(
            frame_bytes as u64,
            (data.len() % frame_bytes) as u64,
        ));
    }
    let frames = data.len() / frame_bytes;
    planes.clear();
    match fmt.layout {
        AudioLayout::Interleaved => {
            for _ in 0..fmt.channels {
                planes.push(Vec::with_capacity(frames));
            }
            for f in 0..frames {
                for (c, plane) in planes.iter_mut().enumerate() {
                    let i = f * frame_bytes + c * bps;
                    plane.push(scalar_to_f32(fmt.sample, data, i));
                }
            }
        }
        AudioLayout::Planar => {
            let plane_bytes = frames * bps;
            for c in 0..fmt.channels as usize {
                let base = c * plane_bytes;
                let plane = Vec::with_capacity(frames);
                let mut out = plane;
                for f in 0..frames {
                    out.push(scalar_to_f32(fmt.sample, data, base + f * bps));
                }
                planes.push(out);
            }
        }
    }
    Ok(frames)
}

/// Stage D, f32 target: mixed planes → interleaved f32 little-endian
/// bytes. `out` is cleared and refilled; capacity is reused.
///
/// # Errors
/// [`Corrupt`](media_types::ErrorKind::Corrupt) with code
/// `audio.planar_plane_mismatch` on inconsistent plane geometry.
pub fn planes_to_f32_interleaved(
    planes: &[Vec<f32>],
    out: &mut Vec<u8>,
) -> Result<(), EngineError> {
    let channels = planes.len() as u32;
    let frames = check_planes(planes, channels)?;
    out.clear();
    out.reserve(frames * channels as usize * 4);
    for f in 0..frames {
        for plane in planes {
            out.extend_from_slice(&plane[f].to_le_bytes());
        }
    }
    Ok(())
}

/// The exact stage-A inverse for s16: `clamp(round(x · 32768))` in
/// `[-32768, 32767]`. Saturating (never wraps) and round-trip exact
/// (see `s16_round_trip_is_exact_over_the_full_range`).
pub fn f32_to_s16_clamped(x: f32) -> i16 {
    let scaled = x * 32_768.0;
    if scaled.is_nan() {
        // A NaN sample is data; the int boundary renders it as silence
        // (documented, deterministic — never a poison pattern).
        return 0;
    }
    let rounded = scaled.round();
    if rounded >= 32_767.0 {
        32_767
    } else if rounded <= -32_768.0 {
        -32_768
    } else {
        rounded as i16
    }
}

/// Stage D, s16 target (the integer-device seam): mixed planes →
/// interleaved s16 little-endian bytes with saturating clamp.
///
/// # Errors
/// [`Corrupt`](media_types::ErrorKind::Corrupt) with code
/// `audio.planar_plane_mismatch` on inconsistent plane geometry.
pub fn planes_to_s16_interleaved(
    planes: &[Vec<f32>],
    out: &mut Vec<u8>,
) -> Result<(), EngineError> {
    let channels = planes.len() as u32;
    let frames = check_planes(planes, channels)?;
    out.clear();
    out.reserve(frames * channels as usize * 2);
    for f in 0..frames {
        for plane in planes {
            out.extend_from_slice(&f32_to_s16_clamped(plane[f]).to_le_bytes());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
    use media_types::ids::GenerationId;
    use media_types::packet::SharedBytes;
    use media_types::time::Stamp;
    use testkit::Must;

    fn frame_of(
        sample: SampleFormat,
        layout: AudioLayout,
        channels: u32,
        bytes: Vec<u8>,
    ) -> AudioFrame {
        AudioFrame {
            format: AudioFormat {
                sample,
                channels,
                layout,
                rate: 44_100,
            },
            samples: SharedBytes::from_vec(bytes),
            pts: Stamp::Missing,
            duration: None,
            generation: GenerationId::FIRST,
        }
    }

    #[test]
    fn s16_interleaved_decodes_exactly() {
        // (L, R) pairs: (0, 0), (16384, -16384), (32767, -32768).
        let mut bytes = Vec::new();
        for v in [0i16, 0, 16384, -16384, 32767, -32768] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let frame = frame_of(SampleFormat::S16, AudioLayout::Interleaved, 2, bytes);
        let mut planes = Vec::new();
        let frames = frame_to_f32_planes(&frame, &mut planes).must();
        assert_eq!(frames, 3);
        assert_eq!(planes.len(), 2);
        assert_eq!(
            planes[0],
            vec![0.0, 16384.0 / 32_768.0, 32_767.0 / 32_768.0]
        );
        assert_eq!(planes[1], vec![0.0, -16384.0 / 32_768.0, -1.0]);
    }

    #[test]
    fn s16_planar_decodes_exactly() {
        // Two planes of 2 frames: L = [1, -1], R = [2, -2] (as s16).
        let mut bytes = Vec::new();
        for v in [1i16, -1] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        for v in [2i16, -2] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let frame = frame_of(SampleFormat::S16, AudioLayout::Planar, 2, bytes);
        let mut planes = Vec::new();
        let frames = frame_to_f32_planes(&frame, &mut planes).must();
        assert_eq!(frames, 2);
        assert_eq!(planes[0], vec![1.0 / 32_768.0, -1.0 / 32_768.0]);
        assert_eq!(planes[1], vec![2.0 / 32_768.0, -2.0 / 32_768.0]);
    }

    #[test]
    fn every_format_decodes_a_known_scalar() {
        // u8: 128 -> 0.0, 255 -> 127/128; s24: 0x800000 -> -1.0;
        // s32: min -> -1.0; f32 passthrough; f64 narrows.
        let cases: Vec<(SampleFormat, Vec<u8>, f32)> = vec![
            (SampleFormat::U8, vec![128], 0.0),
            (SampleFormat::U8, vec![255], 127.0 / 128.0),
            (SampleFormat::S24, vec![0x00, 0x00, 0x80], -1.0),
            (
                SampleFormat::S24,
                vec![0xFF, 0xFF, 0x7F],
                8_388_607.0 / 8_388_608.0,
            ),
            (
                SampleFormat::S32,
                0x8000_0000u32.to_le_bytes().to_vec(),
                -1.0,
            ),
            (SampleFormat::F32, 0.5f32.to_le_bytes().to_vec(), 0.5),
            (SampleFormat::F64, 0.25f64.to_le_bytes().to_vec(), 0.25),
        ];
        for (sample, bytes, want) in cases {
            let frame = frame_of(sample, AudioLayout::Interleaved, 1, bytes);
            let mut planes = Vec::new();
            let frames = frame_to_f32_planes(&frame, &mut planes).must();
            assert_eq!(frames, 1, "{sample:?}");
            assert_eq!(planes[0][0], want, "{sample:?}");
        }
    }

    #[test]
    fn partial_frames_are_typed_corrupt() {
        // 3 bytes of stereo s16: not a whole frame.
        let frame = frame_of(
            SampleFormat::S16,
            AudioLayout::Interleaved,
            2,
            vec![1, 2, 3],
        );
        let mut planes = Vec::new();
        let err = frame_to_f32_planes(&frame, &mut planes).err().must();
        assert_eq!(err.context.code, "audio.partial_frame");
        assert!(err.to_string().contains("truncated"), "{}", err);
    }

    #[test]
    fn f32_interleave_round_trips_the_deinterleave() {
        let mut bytes = Vec::new();
        for v in [0.5f32, -0.25, 0.125, -1.0, 1.0, 0.0] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let frame = frame_of(SampleFormat::F32, AudioLayout::Interleaved, 2, bytes);
        let mut planes = Vec::new();
        let frames = frame_to_f32_planes(&frame, &mut planes).must();
        assert_eq!(frames, 3);
        let mut out = Vec::new();
        planes_to_f32_interleaved(&planes, &mut out).must();
        assert_eq!(out, frame.samples.as_slice());
    }

    #[test]
    fn s16_saturates_at_the_boundary() {
        assert_eq!(f32_to_s16_clamped(1.0), 32_767);
        assert_eq!(f32_to_s16_clamped(-1.0), -32_768);
        assert_eq!(f32_to_s16_clamped(2.0), 32_767);
        assert_eq!(f32_to_s16_clamped(-2.0), -32_768);
        assert_eq!(f32_to_s16_clamped(f32::INFINITY), 32_767);
        assert_eq!(f32_to_s16_clamped(f32::NEG_INFINITY), -32_768);
        assert_eq!(f32_to_s16_clamped(f32::NAN), 0);
        assert_eq!(f32_to_s16_clamped(0.0), 0);
    }

    #[test]
    fn s16_round_trip_is_exact_over_the_full_range() {
        // Every i16 value: decode -> encode must be the identity (the
        // exact-inverse claim above, machine-checked end to end).
        let mut bytes = Vec::new();
        for v in -32_768i16..=32_767 {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let frame = frame_of(SampleFormat::S16, AudioLayout::Interleaved, 1, bytes);
        let mut planes = Vec::new();
        frame_to_f32_planes(&frame, &mut planes).must();
        let mut out = Vec::new();
        planes_to_s16_interleaved(&planes, &mut out).must();
        let original = frame.samples.as_slice();
        assert_eq!(out.len(), original.len());
        for (i, pair) in out.as_chunks::<2>().0.iter().enumerate() {
            let got = i16::from_le_bytes(*pair);
            let want = i16::from_le_bytes([original[i * 2], original[i * 2 + 1]]);
            assert_eq!(got, want, "round trip broke at {want}");
        }
    }

    #[test]
    fn interleaved_output_rejects_ragged_planes() {
        let planes = vec![vec![0.0, 1.0], vec![0.0]];
        let mut out = Vec::new();
        let err = planes_to_f32_interleaved(&planes, &mut out).err().must();
        assert_eq!(err.context.code, "audio.planar_plane_mismatch");
        let err = planes_to_s16_interleaved(&planes, &mut out).err().must();
        assert_eq!(err.context.code, "audio.planar_plane_mismatch");
    }
}
