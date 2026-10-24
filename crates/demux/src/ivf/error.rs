//! Typed error constructors for the IVF demuxer (card 5.I3b).
//!
//! The taxonomy is media-types' (never redefined here); this module
//! mints `EngineError` values with `ivf`-scoped stable codes at stage
//! `Demux`, mirroring the `adts::error` pattern (the crate-root
//! `error.rs` is the AVI card's file — scope discipline, R3
//! consolidates when the registry merges).
//!
//! # No secrets
//!
//! Constructors take byte counts, offsets and the bare fourcc only —
//! never paths, URIs or payload content. The sniffed bytes in the
//! `not-ivf` refusal are capped by the taxonomy's own `Display` (16).

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo, LimitKind};
use media_types::{EngineError, ErrorKind, PipelineStage};

const STAGE: PipelineStage = PipelineStage::Demux;

fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}

/// The bytes are not an IVF container (broken `DKIF` magic).
/// `sniffed` carries the leading bytes for the known-format hint
/// (capped by the taxonomy's Display, never a secret).
pub fn not_ivf(sniffed: &[u8]) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("ivf".to_string()),
                sniffed: sniffed.to_vec(),
            }),
        },
        "demux.ivf.not-ivf",
    )
}

/// The stream ends before the 32-byte file header does. `expected` is
/// the full header size; `found` the stream length actually available.
pub fn truncated(expected: u64, found: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found },
        },
        "demux.ivf.truncated",
    )
}

/// The header declares structure this demuxer cannot honor: a header
/// length other than 32 (every real muxer — ffmpeg's `ivfenc`, libvpx's
/// `vpxenc` — writes exactly 32), zero width/height, or a zero rate or
/// scale field (ffmpeg refuses those with "Invalid frame rate"; so do
/// we, typed).
pub fn bad_header(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.ivf.bad-header",
    )
}

/// The fourcc is not one of this card's delivery codecs (`AV01`, the
/// AV1 bitstream in IVF, or `VP90`, VP9 profile 0). IVF is a generic
/// frame wrapper — ffmpeg maps many more fourccs (VP80, H264, …) —
/// but v0 delivery is AV1/VP9 only, and the refusal is typed, never a
/// silent track. `fourcc` is the bare tag (4 bytes, not a secret).
pub fn unsupported_fourcc(fourcc: &[u8; 4]) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("ivf".to_string()),
                sniffed: fourcc.to_vec(),
            }),
        },
        "demux.ivf.unsupported-fourcc",
    )
}

/// A valid header and nothing after it: an IVF file with zero frames
/// has nothing to demux (the AVI `no-movi` / ADIF `no-data` precedent;
/// ffmpeg's own demuxer opens it and yields a clean empty stream — the
/// honestly-recorded divergence, this layer refuses loudly instead).
pub fn no_frames() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated {
                expected: 1,
                found: 0,
            },
        },
        "demux.ivf.no-frames",
    )
}

/// A frame's declared size runs past the end of the stream. Sticky
/// (the ADTS truncated-frame precedent): ffmpeg instead emits a short
/// final packet — the honestly-recorded divergence, we refuse typed.
/// `expected` is the frame's declared end; `found` the stream length.
pub fn frame_beyond_file(expected: u64, found: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found },
        },
        "demux.ivf.frame-beyond-file",
    )
}

/// A frame's declared size exceeds the payload ceiling (MP §12: a
/// 32-bit size field can claim 4 GiB; the ceiling refuses it before
/// any allocation is trusted).
pub fn frame_too_large(budget: u64, attempted: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget,
            attempted,
        },
        "demux.ivf.frame-too-large",
    )
}

/// A frame's 64-bit pts does not fit the signed tick arithmetic of the
/// time model (a hostile u64 near 2^63; real muxers count from 0).
pub fn pts_overflow(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.ivf.pts-overflow",
    )
}

/// A trait method was called before `start` opened the container.
pub fn not_started() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.ivf.not-started",
    )
}

/// A seek the container cannot map (target before the first frame, or
/// past the last frame header the bounded scan can reach).
pub fn seek_out_of_range() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.ivf.seek.out-of-range",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_carry_demux_stage_and_ivf_codes() {
        let cases: Vec<(EngineError, &str)> = vec![
            (not_ivf(b"DKF\x00\x00"), "demux.ivf.not-ivf"),
            (truncated(32, 12), "demux.ivf.truncated"),
            (bad_header(0), "demux.ivf.bad-header"),
            (unsupported_fourcc(b"VP80"), "demux.ivf.unsupported-fourcc"),
            (no_frames(), "demux.ivf.no-frames"),
            (frame_beyond_file(900, 800), "demux.ivf.frame-beyond-file"),
            (frame_too_large(100, 200), "demux.ivf.frame-too-large"),
            (pts_overflow(64), "demux.ivf.pts-overflow"),
            (not_started(), "demux.ivf.not-started"),
            (seek_out_of_range(), "demux.ivf.seek.out-of-range"),
        ];
        for (error, code) in cases {
            assert_eq!(error.stage, STAGE, "{code}");
            assert_eq!(error.context.code, code);
            // Display is structured data only: stage + kind + code.
            let text = error.to_string();
            assert!(text.starts_with("[demux] "), "{code}: {text}");
            assert!(text.ends_with(&format!(" ({code})")), "{code}: {text}");
        }
    }

    #[test]
    fn not_ivf_sniff_capped_by_display() {
        let error = not_ivf(&[0xC3; 64]);
        let text = error.to_string();
        assert!(text.matches("c3").count() <= 16, "{text}");
    }

    #[test]
    fn kinds_map_to_the_documented_taxonomy() {
        assert!(matches!(
            not_ivf(b"DKIF").kind,
            ErrorKind::Unsupported { .. }
        ));
        assert_eq!(
            frame_beyond_file(10, 4).kind,
            ErrorKind::Corrupt {
                detail: CorruptDetail::Truncated {
                    expected: 10,
                    found: 4
                }
            }
        );
        assert_eq!(
            frame_too_large(10, 20).kind,
            ErrorKind::ResourceLimit {
                limit: LimitKind::Memory,
                budget: 10,
                attempted: 20
            }
        );
    }
}
