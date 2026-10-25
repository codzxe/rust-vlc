//! MP2 seeking (card 5.J3): CBR arithmetic both directions, plus the
//! padding-refine note that keeps it honest on fractional rates.
//!
//! Layer II streams are CBR by construction (the geometry gate in
//! [`super::header`] guarantees a constant table bitrate, rate and
//! channel count), so the core arithmetic is exact in *frames*:
//! sample `s` lives in frame `s / 1152` at nominal byte
//! `first_offset + frame_index × frame_size`. The one wrinkle is the
//! **padding bit**: at 44.1/22.05/16/32 kHz (and the LSF halves) the
//! nominal frame size is fractional and the encoder sprinkles +1-slot
//! padded frames to track it, so a byte position computed from
//! nominal sizes can sit a few slots early — never late in a way that
//! matters, because [`refine_to_frame`] snaps forward to the next
//! real frame header and the consumer drops the pre-roll samples up
//! to `first_sample_at_offset` (the same rule the family's VBR seek
//! and the FLAC seek planner use).
//!
//! The inverse direction ([`sample_at_byte`]) is the demuxer's
//! duration/progress arithmetic: fractional nominal frames make it an
//! estimate within one frame, documented on the function.

use super::header::Mp2Geometry;
use crate::audio::mp3::scan_sync;

/// The result of a seek computation: where to land, and the first
/// sample the frames from there will carry (the consumer's pre-roll
/// drop is `target − first_sample_at_offset`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mp2SeekPlan {
    /// Nominal byte offset of the target frame (see the module doc's
    /// padding note; snap with [`refine_to_frame`] for exactness).
    pub byte_offset: u64,
    /// First sample index the frames from `byte_offset` carry.
    pub first_sample_at_offset: u64,
    /// The target frame's 0-based index.
    pub frame_index: u64,
}

/// CBR seek: the frame containing `target_sample`, as a byte offset
/// (nominal arithmetic) plus the sample bookkeeping.
///
/// `target_sample` beyond the stream end clamps to the last frame
/// (the plan is always inside the geometry — honest for progress
/// bars, harmless for seeks that will miss anyway).
#[must_use]
pub fn cbr_seek(geo: &Mp2Geometry, target_sample: u64) -> Mp2SeekPlan {
    let spf = u64::from(geo.samples_per_frame.max(1));
    let frame_index = (target_sample / spf).min(geo.total_frames.saturating_sub(1));
    Mp2SeekPlan {
        byte_offset: geo.first_offset + frame_index * u64::from(geo.frame_size),
        first_sample_at_offset: frame_index * spf,
        frame_index,
    }
}

/// The inverse arithmetic: which sample index a byte offset falls on
/// (floor to the frame start). An estimate within one frame on
/// fractional-rate streams (padding slots accumulate ahead of the
/// nominal count); exact on 48/24/12 kHz where Layer II frame sizes
/// are integral.
#[must_use]
pub fn sample_at_byte(geo: &Mp2Geometry, byte_offset: u64) -> u64 {
    let spf = u64::from(geo.samples_per_frame.max(1));
    if byte_offset <= geo.first_offset {
        return 0;
    }
    let frames_in = (byte_offset - geo.first_offset) / u64::from(geo.frame_size.max(1));
    (frames_in * spf).min(geo.total_samples)
}

/// The stream's total duration in nanoseconds (truncated), the
/// demuxer's timeline arithmetic.
#[must_use]
pub fn duration_nanos(geo: &Mp2Geometry) -> u64 {
    let rate = u64::from(geo.sample_rate.max(1));
    geo.total_samples
        .saturating_mul(1_000_000_000)
        .saturating_div(rate)
}

/// Snap a nominal landing offset to the next real frame header
/// at-or-after it, walking `data` from `from` (the family's
/// scan-and-verify sync scan — reused, not duplicated). `None` when
/// no parseable header exists from `from`.
#[must_use]
pub fn refine_to_frame(data: &[u8], from: usize) -> Option<u64> {
    scan_sync(data, from).map(|offset| offset as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::mp2::header::{geometry, parse_mp2_header, walk_mp2_frames};
    use crate::audio::mp3::parse_frame_header;

    /// The corpus `0005` geometry as walked frames (MPEG-2 LSF L2
    /// 80 kbps 16 kHz mono — integral 720-byte frames).
    fn lsf_frames() -> Vec<crate::audio::mp2::header::Mp2FrameAt> {
        let mut data = Vec::new();
        for _ in 0..5 {
            data.extend_from_slice(&[0xFF, 0xF5, 0x98, 0xC0]);
            data.resize(data.len() + 720 - 4, 0);
        }
        match walk_mp2_frames(&data, 0) {
            Ok(f) => f,
            Err(e) => unreachable!("{e:?}"),
        }
    }

    #[test]
    fn cbr_seek_is_exact_frame_arithmetic() {
        let frames = lsf_frames();
        let geo = match geometry(&frames) {
            Ok(Some(g)) => g,
            other => unreachable!("{other:?}"),
        };
        let plan = cbr_seek(&geo, 0);
        assert_eq!(
            (
                plan.byte_offset,
                plan.first_sample_at_offset,
                plan.frame_index
            ),
            (0, 0, 0)
        );
        let plan = cbr_seek(&geo, 1152);
        assert_eq!(plan.byte_offset, 720);
        assert_eq!(plan.first_sample_at_offset, 1152);
        assert_eq!(plan.frame_index, 1);
        let plan = cbr_seek(&geo, 4 * 1152 + 1111);
        assert_eq!(plan.byte_offset, 4 * 720);
        assert_eq!(plan.first_sample_at_offset, 4 * 1152);
        assert_eq!(plan.frame_index, 4);
        // Beyond the end clamps into the last frame, never past it.
        let plan = cbr_seek(&geo, 100 * 1152);
        assert_eq!(plan.frame_index, 4);
        assert_eq!(plan.byte_offset, 4 * 720);
    }

    #[test]
    fn cbr_seek_agrees_with_the_family_planner() {
        // The family (mp3) CBR planner and ours must agree on the
        // same header — a cross-module pin against drift.
        let frames = lsf_frames();
        let geo = match geometry(&frames) {
            Ok(Some(g)) => g,
            other => unreachable!("{other:?}"),
        };
        let family_header = match parse_frame_header(&[0xFF, 0xF5, 0x98, 0xC0]) {
            Ok(h) => h,
            Err(e) => unreachable!("{e:?}"),
        };
        for target in [0u64, 1, 1151, 1152, 5 * 1152 - 1] {
            let ours = cbr_seek(&geo, target);
            let theirs = crate::audio::mp3::cbr_seek(&family_header, 0, target);
            assert_eq!(ours.byte_offset, theirs.byte_offset, "target {target}");
            assert_eq!(
                ours.first_sample_at_offset, theirs.first_sample_at_offset,
                "target {target}"
            );
        }
    }

    #[test]
    fn sample_at_byte_inverts_on_integral_frame_sizes() {
        let frames = lsf_frames();
        let geo = match geometry(&frames) {
            Ok(Some(g)) => g,
            other => unreachable!("{other:?}"),
        };
        assert_eq!(sample_at_byte(&geo, 0), 0);
        assert_eq!(sample_at_byte(&geo, 719), 0);
        assert_eq!(sample_at_byte(&geo, 720), 1152);
        assert_eq!(sample_at_byte(&geo, 3 * 720 + 100), 3 * 1152);
        // Beyond the stream clamps to the total.
        assert_eq!(sample_at_byte(&geo, 10_000), 5 * 1152);
        // Duration: 5760 samples at 16 kHz = 360 ms exactly.
        assert_eq!(duration_nanos(&geo), 360_000_000);
    }

    #[test]
    fn refine_snaps_a_padded_stream_to_a_real_header() {
        // A 44.1 kHz stream mixes 1253/1254-byte frames; a nominal
        // landing point can sit a slot early — refine must snap it
        // forward onto the real header.
        let mut data = Vec::new();
        let padded = [0xFF, 0xFD, 0xE2, 0x00]; // 384 kbps 44.1 kHz, padding set
        for k in 0..6 {
            let hdr: [u8; 4] = if k % 3 == 0 {
                padded
            } else {
                [0xFF, 0xFD, 0xE0, 0x00]
            };
            let size = match parse_mp2_header(&hdr) {
                Ok(h) => h.family.frame_size as usize,
                Err(e) => unreachable!("{e:?}"),
            };
            data.extend_from_slice(&hdr);
            data.resize(data.len() + size - 4, 0);
        }
        let frames = match walk_mp2_frames(&data, 0) {
            Ok(f) => f,
            Err(e) => unreachable!("{e:?}"),
        };
        let geo = match geometry(&frames) {
            Ok(Some(g)) => g,
            other => unreachable!("{other:?}"),
        };
        assert_eq!(geo.frame_size, 1253);
        // Nominal byte for frame 4 vs the real offset (padding put
        // extra slots ahead); refine lands exactly on the frame.
        let plan = cbr_seek(&geo, 4 * 1152);
        let snapped = refine_to_frame(&data, plan.byte_offset as usize);
        assert_eq!(snapped, Some(frames[4].offset as u64));
        // And the snapped position parses as a Layer II header.
        let window = data
            .get(frames[4].offset..frames[4].offset + 4)
            .unwrap_or(&[]);
        assert!(parse_mp2_header(window).is_ok());
        // Past the end: nothing left to snap to.
        assert_eq!(refine_to_frame(&data, data.len()), None);
    }
}
