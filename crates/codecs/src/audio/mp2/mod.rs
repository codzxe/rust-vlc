//! MP2 (MPEG audio Layer II) decoder (card 5.J3; doc 04 §3.1 row
//! "MP2") — the MPEG audio family completion: the dedicated Layer II
//! surface beside the family module [`crate::audio::mp3`] (card
//! 5.A3, landed).
//!
//! # Format slice (what this module owns)
//!
//! **The raw Layer II elementary stream**: the 4-byte frame headers
//! behind the family's shared parse — the Layer II gate, the MPEG-1
//! vs MPEG-2/2.5 LSF bitrate tables, the intensity-stereo mode
//! extension ([`header`]) — the CBR geometry gate (Layer II is CBR by
//! construction; there is no Xing/Info/VBRI construction in Layer
//! II) and the padding-aware seek arithmetic, both directions
//! ([`seek`]). The entropy decode of one framed frame is the
//! already-admitted family engine's job: `symphonia-bundle-mp3`
//! 0.6.1's `MpaDecoder` configured with `CODEC_ID_MP2` ([`decoder`])
//! — **no new dependency** (the 5.A3 admission covers the whole
//! MPEG-audio family; the record is
//! `docs/dependencies/records/symphonia-mp3-0.6.1.md`).
//!
//! # Module map
//!
//! - [`Mp2Decoder`] — the codec-api [`Decoder`](codec_api::Decoder)
//!   implementation (one packet = one frame; the descriptor gate is
//!   exactly `mp2`).
//! - [`decode_stream`] — the free whole-stream surface: ID3 skip →
//!   Layer II walk → CBR gate → decode (the conformance and
//!   differential workhorse).
//! - [`header`] — the Layer II gate, bitrate tables, mode extension,
//!   walk + geometry.
//! - [`seek`] — CBR arithmetic both directions + padding refine.
//! - [`fuzz`] — the `fuzz_codec_mp2` harness (`+FB`; wiring status
//!   documented there, honestly).
//!
//! # Relationship to the family module (the honest overlap note)
//!
//! Card 5.A3's `Mp3Decoder` accepts the descriptors `mp1`/`mp2`/`mp3`
//! (its Layer II rows run through the same engine). This module is
//! the dedicated Layer II surface the registry can point `mp2` at:
//! the stricter header gate (a non-Layer II frame is typed damage,
//! not silently decoded), the CBR geometry + seek arithmetic, and the
//! no-gapless-facts contract. The R3 registration card routes the
//! `mp2` descriptor HERE (snippet in the worklog); the family's
//! multi-layer acceptance stays its recorded behavior.
//!
//! # Conventions (crate-wide, unchanged)
//!
//! No `unsafe` on this module's own paths (the engine's crates are
//! `#![forbid(unsafe_code)]` themselves, recorded in the dependency
//! record); no panic on any input — every decode/parse/seek path is
//! bounds-checked and damage is a typed [`EngineError`] (bad sync
//! resyncs, truncated frames and tags truncate, a mid-stream layer or
//! bitrate switch is refused), never a crash (doc 03 §4.5(d)).
//!
//! # Bit-exactness (the honest AC posture)
//!
//! MP2 is lossy, but the engine's Layer II synthesis is the
//! reference DCT/quantization cascade: the differential against
//! ffmpeg compares whole-decode PCM MD5s on the identical
//! `frames × 1152` window (no gapless arithmetic exists in Layer II)
//! and records the max sample delta per file
//! (`tests/mp2_corpus_differential.rs`).

pub(crate) mod decoder;
pub(crate) mod error;
/// Fuzz harness (doc 04 §8.5 registry name `fuzz_codec_mp2`): `pub`
/// because the registered target file and the in-tree survival tests
/// call it from outside the crate (the 5.A3 precedent).
pub mod fuzz;
pub(crate) mod header;
pub(crate) mod seek;

pub use decoder::{DecodedMp2, Mp2Decoder, decode_stream};
pub use header::{
    LAYER_II_BITRATES_MPEG1, LAYER_II_BITRATES_MPEG2_LSF, LayerIiModeExtension, Mp2FrameAt,
    Mp2FrameHeader, Mp2Geometry, geometry, intensity_bound, is_mp2_stream, layer_ii_bitrate,
    layer_ii_bitrate_table, parse_mp2_header, walk_mp2_frames,
};
pub use seek::{Mp2SeekPlan, cbr_seek, duration_nanos, refine_to_frame, sample_at_byte};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_public_surface_reexports_resolve() {
        // The registry/demuxer-facing names all exist with the
        // expected arities (compile-level pin; the behavior is the
        // submodules' suites).
        assert_eq!(LayerIiModeExtension::from_bits(2).bound_subband(), 12);
        assert_eq!(
            layer_ii_bitrate(crate::audio::mp3::MpegVersion::V1, 14),
            Some(384)
        );
        assert_eq!(LAYER_II_BITRATES_MPEG2_LSF[13], 160);
        assert!(is_mp2_stream(&[0xFF, 0xFD, 0xE0, 0x00]));
        assert_eq!(
            intensity_bound(
                crate::audio::mp3::ChannelMode::Mono,
                LayerIiModeExtension::Bound8
            ),
            None
        );
    }
}
