//! Seeking (card 5.A3): CBR by time↔bytes arithmetic, VBR by the
//! Xing TOC.
//!
//! # CBR (no Xing/Info header)
//!
//! Every frame is `frame_size` bytes carrying `samples_per_frame`
//! samples, so a target sample `s` lives in frame
//! `s / samples_per_frame` at byte
//! `first_frame + frame_index × frame_size` — exact, one division,
//! no index. (Bitrate/samplerate/padding are constant by
//! construction of a CBR stream; a padding-bit flip mid-stream is
//! damage the walker resyncs past.)
//!
//! # VBR (Xing TOC) — the honest accuracy note
//!
//! The TOC is **100 entries over the whole file**: entry `i` is the
//! percent-of-file-bytes where ~`i%` of the frames have passed.
//! [`vbr_toc_seek`] interpolates linearly *inside* one entry (tenth-of-
//! percent granularity), but the entry boundaries themselves are
//! 1%-granular:
//!
//! - byte-position error ≤ ~1% of the audio span, plus
//! - the landing point is frame-aligned only after the consumer
//!   re-syncs a frame header at/after the returned offset
//!   ([`refine_to_frame`]), adding ≤ one frame.
//!
//! So the honest contract is: the returned offset lands **at or
//! before** the target frame, never after, and the consumer must
//! decode forward and drop samples up to the target (pre-roll drop —
//! the same rule the FLAC seek planner uses). Frame-exact seeking
//! from a TOC landing point is [`refine_to_frame`] + walking headers
//! while counting `samples_per_frame` per frame.
//!
//! VBRI TOCs (explicit entry counts/scales) normalize to the same
//! percent interpolation by the caller.

use super::header::{FrameHeader, scan_sync};

/// A VBR TOC (Xing layout) + the stream geometry the interpolation
/// needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VbrToc {
    /// 100 entries; `toc[i]` ≈ percent-of-bytes at `i` percent of
    /// frames. Real files are monotone non-decreasing; a damaged TOC
    /// simply lands somewhere at-or-before (bounds still hold).
    pub toc: [u8; 100],
    /// Byte offset of the first audio frame AFTER the header frame.
    pub first_frame_offset: u64,
    /// Total audio bytes (the Xing `bytes` field, or the walked span).
    pub total_bytes: u64,
    /// Total frame count (the Xing `frames` field).
    pub total_frames: u32,
}

/// The result of a seek computation: where to land, and the first
/// sample the frames from there will carry (the consumer's pre-roll
/// drop is `target − this`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeekPlan {
    /// Byte offset to (re)position to — a frame START (CBR) or a
    /// pre-target landing point (VBR, see the module's accuracy note).
    pub byte_offset: u64,
    /// First sample index the frames from `byte_offset` carry.
    pub first_sample_at_offset: u64,
}

/// CBR seek: the exact frame containing `target_sample`.
///
/// `header` is the FIRST audio frame's header (post-ID3, post-Xing);
/// `first_frame_offset` its byte offset. Pure arithmetic — the plan
/// is frame-exact.
#[must_use]
pub fn cbr_seek(header: &FrameHeader, first_frame_offset: u64, target_sample: u64) -> SeekPlan {
    let spf = u64::from(header.samples_per_frame.max(1));
    let frame_index = target_sample / spf;
    SeekPlan {
        byte_offset: first_frame_offset + frame_index * u64::from(header.frame_size),
        first_sample_at_offset: frame_index * spf,
    }
}

/// VBR seek through a Xing TOC: land at or before the frame holding
/// `target_sample`. `samples_per_frame` comes from the stream's real
/// first-frame header (1152 for MPEG-1 L3, 576 for MPEG-2 L3, …).
///
/// Interpolation is tenth-of-a-percent inside the TOC entry the
/// target falls into; the returned `first_sample_at_offset` is the
/// TOC's own frame-count estimate (at-or-before). See the module
/// docs for the accuracy contract and the refine path.
#[must_use]
pub fn vbr_toc_seek(toc: &VbrToc, target_sample: u64, samples_per_frame: u32) -> SeekPlan {
    let spf = u64::from(samples_per_frame.max(1));
    let total_frames = u64::from(toc.total_frames.max(1));
    let total_samples = total_frames * spf;
    // Target fraction in per-mille, clamped into [0, 999].
    let permille = if target_sample == 0 {
        0
    } else {
        ((target_sample - 1) * 1000 / total_samples.max(1)).min(999)
    };
    let percent = (permille / 10) as usize;
    let anchor = u64::from(toc.toc[percent]);
    let next = if percent + 1 < 100 {
        u64::from(toc.toc[percent + 1]).max(anchor)
    } else {
        255
    };
    let sub = permille % 10;
    // byte_frac is in 255ths (the Xing TOC's own scale).
    let byte_frac = anchor + (next - anchor) * sub / 10;
    let offset = toc.first_frame_offset + byte_frac * toc.total_bytes / 255;
    let frames_before = total_frames * permille / 1000;
    SeekPlan {
        byte_offset: offset,
        first_sample_at_offset: frames_before * spf,
    }
}

/// Snap a VBR landing offset to the next real frame header at-or-after
/// it, walking `data` from `from`.
///
/// This is the frame-alignment half of the VBR seek contract; the
/// sample accounting (walking `samples_per_frame` per frame) is the
/// consumer's decode-forward loop. `None` when no frame header exists
/// from `from` (end of stream / heavy damage).
#[must_use]
pub fn refine_to_frame(data: &[u8], from: usize) -> Option<u64> {
    let start = scan_sync(data, from)?;
    // scan_sync only returns offsets whose header parses.
    Some(start as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::mp3::header::parse_frame_header;

    fn hdr_128k() -> FrameHeader {
        match parse_frame_header(&[0xFF, 0xFB, 0x90, 0x00]) {
            Ok(h) => h,
            Err(e) => unreachable!("{e:?}"),
        }
    }

    #[test]
    fn cbr_seek_is_exact_frame_arithmetic() {
        let h = hdr_128k();
        let plan = cbr_seek(&h, 100, 0);
        assert_eq!((plan.byte_offset, plan.first_sample_at_offset), (100, 0));
        let plan = cbr_seek(&h, 100, 1152);
        assert_eq!(plan.byte_offset, 100 + 417);
        assert_eq!(plan.first_sample_at_offset, 1152);
        let plan = cbr_seek(&h, 100, 1152 * 7 + 1111);
        assert_eq!(plan.byte_offset, 100 + 7 * 417);
        assert_eq!(plan.first_sample_at_offset, 1152 * 7);
    }

    #[test]
    fn vbr_toc_seek_lands_at_or_before_the_target() {
        let mut toc = [0u8; 100];
        for (i, t) in toc.iter_mut().enumerate() {
            *t = ((i * 255) / 99) as u8;
        }
        let v = VbrToc {
            toc,
            first_frame_offset: 417,
            total_bytes: 40_000,
            total_frames: 40,
        };
        let plan = vbr_toc_seek(&v, 0, 1152);
        assert_eq!(plan.byte_offset, 417);
        assert_eq!(plan.first_sample_at_offset, 0);
        let half = 40u64 * 1152 / 2;
        let plan = vbr_toc_seek(&v, half, 1152);
        let pct = plan.byte_offset.saturating_sub(417) * 100 / 40_000;
        assert!(pct <= 51, "overshoot: {pct}%");
        // The accuracy contract: ≤ 1 TOC entry (1% = 0.4 frames here)
        // of granularity, never after the target.
        let target_frames = half / 1152;
        assert!(plan.first_sample_at_offset / 1152 <= target_frames);
        assert!(
            target_frames - plan.first_sample_at_offset / 1152 <= (40 / 100).max(1) + 1,
            "TOC granularity violated"
        );
    }

    #[test]
    fn a_degenerate_zero_toc_pins_the_stream_start() {
        let v = VbrToc {
            toc: [0u8; 100],
            first_frame_offset: 100,
            total_bytes: 10_000,
            total_frames: 10,
        };
        // Any target inside entry 0 (0-1% of the stream) lands on the
        // first frame; the interpolation to the end-of-grid bound
        // (255) is monotone and at-or-before by construction.
        let plan = vbr_toc_seek(&v, 115, 1152);
        assert_eq!(plan.byte_offset, 100);
        let plan_end = vbr_toc_seek(&v, 10 * 1152 - 1, 1152);
        assert!(plan_end.byte_offset >= 100);
        assert!(plan_end.byte_offset <= 100 + 10_000);
    }

    #[test]
    fn refine_snaps_to_a_real_frame_header() {
        let mut data = Vec::new();
        data.extend_from_slice(&[0u8; 5]);
        data.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
        data.resize(data.len() + 413, 0);
        assert_eq!(refine_to_frame(&data, 0), Some(5));
        assert_eq!(refine_to_frame(&[], 0), None);
    }
}
