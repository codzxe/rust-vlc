//! MP3 / MP2 audio decoder (card 5.A3; doc 04 §3.1 rows "MP3 (MPEG-1/
//! 2/2.5 Layer III)" and "MP2", §2.2 row "MP3 raw stream") — the
//! Phase-5 lossy-audio core.
//!
//! # Format slice (what this module owns)
//!
//! **The raw MPEG-audio elementary stream**: the ID3v2.2/2.3/2.4
//! tag ([`id3`]), the ISO 11172-3/13818-3 frame headers — sync,
//! version/layer, bitrate/samplerate tables, channel modes, mode
//! extension, emphasis — with the frame walker's resync-on-garbage
//! ([`header`]), the Xing/Info + LAME and VBRI headers
//! ([`xing`]), seeking — CBR by time↔bytes arithmetic, VBR through
//! the Xing TOC with the honest accuracy note ([`seek`]) — and the
//! gapless window (encoder delay/padding → sample-accurate trim,
//! applied in [`decoder::decode_stream`] and the contract decoder).
//! The entropy decode of one framed packet is the purity-pinned
//! `symphonia-bundle-mp3` 0.6.1 engine's job ([`decoder`]).
//!
//! # Module map
//!
//! - [`Mp3Decoder`] — the codec-api [`Decoder`](codec_api::Decoder)
//!   implementation (one packet = one frame; facts blob documented
//!   on [`decoder::facts`]).
//! - [`decode_stream`] — the free whole-stream surface: ID3 → walk
//!   → Xing/VBRI → decode → gapless trim (the conformance and
//!   differential workhorse).
//! - [`header`] — frame header parse + tables + the resync walker.
//! - [`id3`] — ID3v2 skip + parse (unsynchronisation, extended
//!   header, footer, text/picture frames).
//! - [`xing`] — Xing/Info + LAME gapless facts + VBRI.
//! - [`seek`] — CBR arithmetic, VBR TOC interpolation, refine.
//! - [`fuzz`] — the `fuzz_codec_mp3` harness (`+FB`; wiring status
//!   documented there, honestly).
//!
//! # Conventions (crate-wide, unchanged)
//!
//! No `unsafe` on this module's own paths (the engine's crates are
//! `#![forbid(unsafe_code)]` themselves, recorded in the dependency
//! record); no panic on any input — every decode/parse/seek path is
//! bounds-checked and damage is a typed [`EngineError`] (bad sync
//! resyncs, truncated frames and tags truncate, lying Xing counts
//! are refused), never a crash (doc 03 §4.5(d)).
//!
//! # Bit-exactness (the honest AC posture)
//!
//! MP3 is lossy: two independent decoder implementations are not
//! sample-identical in general. The differential compares against
//! ffmpeg **on the same gapless window** — Xing frame skipped,
//! LAME delay/padding trimmed (verified empirically: ffmpeg decodes
//! exactly `frames × spf − delay − padding` samples of an
//! ffmpeg-authored fixture) — and records the PCM MD5 plus the max
//! sample delta per file (`tests/mp3_bitexact.rs`,
//! `tests/mp3_corpus_differential.rs`).

pub(crate) mod decoder;
pub(crate) mod error;
/// Fuzz harness (doc 04 §8.5 registry name `fuzz_codec_mp3`): `pub`
/// because the registered target file and the in-tree survival tests
/// call it from outside the crate (the 2.B2/2.B4/5.A1 precedent).
pub mod fuzz;
pub(crate) mod header;
pub(crate) mod id3;
pub(crate) mod seek;
pub(crate) mod xing;

pub use decoder::facts;
pub use decoder::{DecodedMp3, Mp3Decoder, decode_stream};
pub use header::{
    ChannelMode, Emphasis, FrameAt, FrameHeader, Layer, MpegVersion, channel_count,
    parse_frame_header, samples_per_frame, scan_sync, side_info_size, walk_frames,
};
pub use id3::{Id3Frame, Id3Tag, id3v2_len, parse_id3v2};
pub use seek::{SeekPlan, VbrToc, cbr_seek, refine_to_frame, vbr_toc_seek};
pub use xing::{LameTag, VbriInfo, XingInfo, parse_vbri, parse_xing};

/// Whether `data` begins like a raw MPEG-audio stream: an ID3v2 tag
/// or a valid frame header at offset 0 (the probe the future MP3
/// demuxer / registry row will call; doc 04 §2.2 "MP3 raw stream").
#[must_use]
pub fn is_mp3_stream(data: &[u8]) -> bool {
    if data.get(..3) == Some(b"ID3") {
        return true;
    }
    parse_frame_header(data).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_accepts_id3_and_frame_headers_only() {
        assert!(is_mp3_stream(b"ID3\x03\x00"));
        assert!(is_mp3_stream(&[0xFF, 0xFB, 0x90, 0x00]));
        assert!(!is_mp3_stream(&[0xFF, 0x00, 0x00, 0x00]));
        assert!(!is_mp3_stream(b"RIFF"));
        assert!(!is_mp3_stream(&[]));
        // The corpus's mislabeled FLAC must not probe as MP3.
        assert!(!is_mp3_stream(b"fLaC"));
    }

    #[test]
    fn a_minimal_synthetic_stream_decodes_end_to_end() {
        // One hand-built frame: header + side info + zero payload.
        // The engine must decode it (1152 samples of near-silence or
        // a typed error) — what is pinned is the pipeline shape.
        let mut frame = Vec::new();
        frame.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
        frame.extend_from_slice(&[0u8; 413]);
        let out = decode_stream(&frame);
        match out {
            Ok(d) => {
                assert_eq!(d.frames, 1);
                assert_eq!(d.format.channels, 2);
                assert_eq!(d.format.rate, 44_100);
                assert_eq!(d.samples.len(), 1152 * 2);
            }
            Err(e) => unreachable!("a valid-shaped frame must decode: {e}"),
        }
    }
}
