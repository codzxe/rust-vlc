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

/// Exact int→f32 scale reciprocals. Each divisor is a power of two,
/// so the reciprocal `2^-k` is exactly representable in f32 and
/// multiplying by it performs the *same exact exponent shift* the
/// division did — bit-identical results (D-2: multiply, never divide,
/// in the sample loop; correctness pinned by the full-range round-trip
/// test below and the sine-hash AC).
const U8_RECIP: f32 = 1.0 / 128.0;
const S16_RECIP: f32 = 1.0 / 32_768.0;
const S24_RECIP: f32 = 1.0 / 8_388_608.0;
const S32_RECIP: f32 = 1.0 / 2_147_483_648.0;

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
/// `data[i..]` into f32 (the exact power-of-two scaling). Cold,
/// single-sample use (tests, probes); the per-frame loops below hoist
/// the format `match` out and call the monomorphic per-format
/// decoders instead (D-2).
fn scalar_to_f32(sample: SampleFormat, data: &[u8], i: usize) -> f32 {
    match sample {
        SampleFormat::U8 => u8_at(data, i),
        SampleFormat::S16 => s16_at(data, i),
        SampleFormat::S24 => s24_at(data, i),
        SampleFormat::S32 => s32_at(data, i),
        SampleFormat::F32 => f32_at(data, i),
        SampleFormat::F64 => f64_at(data, i),
    }
}

// The monomorphic per-format decoders: one static call site per
// format, no per-sample `match`, no per-sample division.

#[inline]
fn u8_at(data: &[u8], i: usize) -> f32 {
    (f32::from(data[i]) - 128.0) * U8_RECIP
}

#[inline]
fn s16_at(data: &[u8], i: usize) -> f32 {
    let v = i16::from_le_bytes([data[i], data[i + 1]]);
    f32::from(v) * S16_RECIP
}

#[inline]
fn s24_at(data: &[u8], i: usize) -> f32 {
    let b0 = u32::from(data[i]);
    let b1 = u32::from(data[i + 1]);
    let b2 = u32::from(data[i + 2]);
    // Little-endian 24-bit, sign-extended: lift the sign bit
    // (bit 23) to bit 31, then arithmetic-shift back down.
    let v24 = b0 | (b1 << 8) | (b2 << 16);
    let v = ((v24 << 8) as i32) >> 8;
    v as f32 * S24_RECIP
}

#[inline]
fn s32_at(data: &[u8], i: usize) -> f32 {
    let v = i32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
    v as f32 * S32_RECIP
}

#[inline]
fn f32_at(data: &[u8], i: usize) -> f32 {
    f32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]])
}

#[inline]
fn f64_at(data: &[u8], i: usize) -> f32 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[i..i + 8]);
    f64::from_le_bytes(bytes) as f32
}

/// Fill the planes from **interleaved** `data`, one monomorphic
/// instantiation per format: the format `match` is hoisted out of the
/// sample loops (D-2 — branch-free inner loops, no per-sample
/// division, vectorizable).
macro_rules! decode_interleaved {
    ($planes:expr, $data:expr, $frames:expr, $frame_bytes:expr, $bps:expr, $decode:ident) => {
        for f in 0..$frames {
            for (c, plane) in $planes.iter_mut().enumerate() {
                plane.push($decode($data, f * $frame_bytes + c * $bps));
            }
        }
    };
}

/// Fill the planes from **planar** `data` (same monomorphic-per-format
/// discipline as [`decode_interleaved`]).
macro_rules! decode_planar {
    ($planes:expr, $data:expr, $frames:expr, $bps:expr, $plane_bytes:expr, $decode:ident) => {
        for (c, plane) in $planes.iter_mut().enumerate() {
            let base = c * $plane_bytes;
            for f in 0..$frames {
                plane.push($decode($data, base + f * $bps));
            }
        }
    };
}

/// Stage A: a decoded [`AudioFrame`] of any supported format/layout →
/// f32 planar planes. `planes` is resized to the frame's channel
/// count and **both the outer Vec and every inner plane Vec keep
/// their allocations across calls** — `resize_with` (never `clear`,
/// which would drop the inner Vecs) plus a per-plane `clear`/
/// `reserve` pair (doc 03 §4.8 c — no per-frame allocation after
/// warm-up; this function's own doc claim, now true). Returns the
/// frame count.
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
    // True reuse: keep the plane Vecs alive, refit their length and
    // capacity. A channel-count change (geometry change) grows the
    // outer Vec or drops surplus planes — allocation only there.
    planes.resize_with(fmt.channels as usize, Vec::new);
    for plane in planes.iter_mut() {
        plane.clear();
        plane.reserve(frames);
    }
    match fmt.layout {
        AudioLayout::Interleaved => match fmt.sample {
            SampleFormat::U8 => decode_interleaved!(planes, data, frames, frame_bytes, bps, u8_at),
            SampleFormat::S16 => decode_interleaved!(planes, data, frames, frame_bytes, bps, s16_at),
            SampleFormat::S24 => decode_interleaved!(planes, data, frames, frame_bytes, bps, s24_at),
            SampleFormat::S32 => decode_interleaved!(planes, data, frames, frame_bytes, bps, s32_at),
            SampleFormat::F32 => decode_interleaved!(planes, data, frames, frame_bytes, bps, f32_at),
            SampleFormat::F64 => decode_interleaved!(planes, data, frames, frame_bytes, bps, f64_at),
        },
        AudioLayout::Planar => {
            let plane_bytes = frames * bps;
            match fmt.sample {
                SampleFormat::U8 => decode_planar!(planes, data, frames, bps, plane_bytes, u8_at),
                SampleFormat::S16 => decode_planar!(planes, data, frames, bps, plane_bytes, s16_at),
                SampleFormat::S24 => decode_planar!(planes, data, frames, bps, plane_bytes, s24_at),
                SampleFormat::S32 => decode_planar!(planes, data, frames, bps, plane_bytes, s32_at),
                SampleFormat::F32 => decode_planar!(planes, data, frames, bps, plane_bytes, f32_at),
                SampleFormat::F64 => decode_planar!(planes, data, frames, bps, plane_bytes, f64_at),
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

/// Stage D, f32 target, **caller-owned fixed buffer**: mixed planes →
/// interleaved f32 little-endian bytes written into `out` (exactly
/// `frames · channels · 4` bytes). The quantum hot path's zero-
/// allocation interleave — the caller hands a slice it owns (a pooled
/// [`SharedBytes`](media_types::packet::SharedBytes) slot filled via
/// `get_mut`), so no Vec is grown or reallocated per quantum.
/// Byte-identical to [`planes_to_f32_interleaved`] (same values, same
/// order, same little-endian encoding).
///
/// # Errors
/// [`Corrupt`](media_types::ErrorKind::Corrupt) with code
/// `audio.planar_plane_mismatch` on inconsistent plane geometry, or
/// `audio.partial_frame` when `out` is not exactly the interleaved
/// byte length (a caller sizing bug — data, not a guess).
pub fn planes_to_f32_interleaved_into(
    planes: &[Vec<f32>],
    out: &mut [u8],
) -> Result<(), EngineError> {
    let channels = planes.len() as u32;
    let frames = check_planes(planes, channels)?;
    let needed = frames * channels as usize * 4;
    if out.len() != needed {
        return Err(error::partial_frame(needed as u64, out.len() as u64));
    }
    let mut at = 0usize;
    for f in 0..frames {
        for plane in planes {
            out[at..at + 4].copy_from_slice(&plane[f].to_le_bytes());
            at += 4;
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
    fn plane_capacity_is_reused_across_calls() {
        // The doc-03 §4.8 c claim, machine-checked: after the first
        // call the plane Vecs keep their allocations — capacities
        // never shrink and never grow again at the same geometry.
        let mut bytes = Vec::new();
        for v in [100i16, -100, 200, -200, 300, -300, 400, -400] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let frame = frame_of(SampleFormat::S16, AudioLayout::Interleaved, 2, bytes);
        let mut planes: Vec<Vec<f32>> = Vec::new();
        frame_to_f32_planes(&frame, &mut planes).must();
        let outer = planes.capacity();
        let inner: Vec<usize> = planes.iter().map(Vec::capacity).collect();
        // A channel-count change (mono frame) trims to one plane —
        // the survivor keeps its allocation.
        let mono = frame_of(SampleFormat::S16, AudioLayout::Interleaved, 1, vec![0; 8]);
        frame_to_f32_planes(&mono, &mut planes).must();
        assert_eq!(planes.len(), 1);
        assert!(planes[0].capacity() >= inner[0]);
        // Back at stereo, same geometry: capacities intact, no churn.
        let frame = frame_of(
            SampleFormat::S16,
            AudioLayout::Interleaved,
            2,
            vec![0; 16],
        );
        frame_to_f32_planes(&frame, &mut planes).must();
        assert!(planes.capacity() >= outer);
        assert!(planes[0].capacity() >= inner[0]);
        assert!(planes[1].capacity() >= inner[1]);
    }

    #[test]
    fn interleave_into_is_byte_identical_to_the_vec_version() {
        let planes = vec![vec![0.5f32, -0.25, 1.5, -2.5], vec![0.0f32, 3.5, -0.125, 8.0]];
        let mut vec_out = Vec::new();
        planes_to_f32_interleaved(&planes, &mut vec_out).must();
        let mut fixed_out = vec![0u8; vec_out.len()];
        planes_to_f32_interleaved_into(&planes, &mut fixed_out).must();
        assert_eq!(fixed_out, vec_out);
        // Wrong-sized slice: typed, data-not-guess.
        let mut short = vec![0u8; vec_out.len() - 1];
        let err = planes_to_f32_interleaved_into(&planes, &mut short)
            .err()
            .must();
        assert_eq!(err.context.code, "audio.partial_frame");
        let mut long = vec![0u8; vec_out.len() + 1];
        let err = planes_to_f32_interleaved_into(&planes, &mut long)
            .err()
            .must();
        assert_eq!(err.context.code, "audio.partial_frame");
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
