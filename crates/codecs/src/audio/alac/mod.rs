//! ALAC audio decoder (card 5.J3a; doc 04 §3.1 row "ALAC" — the
//! lossless Apple codec, MP4/MKV payload).
//!
//! # Format slice (what this module owns)
//!
//! **The ALAC raw data block** behind the MP4 `alac` sample entry /
//! the MKV `A_ALAC` CodecPrivate: the magic-cookie
//! (ALACSpecificConfig) parse with the containers' passthrough shapes
//! ([`parse_alac_config`]), frame decode through the pinned
//! `symphonia-codec-alac` 0.6.1 engine ([`decoder`] — the dependency
//! decision, its purity record, and the prior session's
//! suspected-engine-bug audit are documented there), and the
//! codec-api contract surface ([`AlacDecoder`]). Container-level
//! surfaces (the MP4 `stsd` walk, edit lists, the MKV block framing)
//! are the demuxer cards' slice; this module sees exactly the packet
//! payloads and the cookie.
//!
//! # Module map
//!
//! - [`AlacDecoder`] — the codec-api [`Decoder`](codec_api::Decoder)
//!   implementation (negotiation + damage model documented on
//!   [`decoder`]).
//! - [`parse_alac_config`] — the magic-cookie gate (typed
//!   `alac.config.*`).
//! - [`decode_frame`] — the free surface: one packet + cookie → one
//!   decoded block with no contract state (the `decode_image` pattern
//!   of 2.B4; the fuzz entry).
//! - [`output_format`] — the negotiated shape preview.
//!
//! # Conventions (crate-wide, unchanged)
//!
//! No `unsafe` on this module's own paths (the engine is
//! `#![forbid(unsafe_code)]` — the dependency record); no panic on
//! any input — every decode path is bounds-checked, damage is a
//! typed [`EngineError`] and the decoder stays usable per frame
//! (ALAC frames are independent), never a crash (doc 03 §4.5(d)).
//!
//! # Bit-exactness (the card AC)
//!
//! ALAC is lossless; the differential tests
//! (`crates/codecs/tests/alac_bitexact.rs`) decode ffmpeg-generated
//! fixtures (stereo/mono × 16/24-bit, multi-frame) through the full
//! L1→L3 composition (input FileAccess → `demux::mp4` → this decoder)
//! and require **byte equality** against ffmpeg's own decoded PCM
//! (`-f s16le` / `-f s24le` interleaved dumps — the engine outputs
//! planar, so the interleaved dump is the honest oracle), with the
//! oracle digests recorded next to the fixtures.

pub(crate) mod decoder;
pub(crate) mod error;

pub mod fuzz;
pub use decoder::{
    AlacConfig, AlacDecoder, DecodedAlac, decode_frame, output_format, parse_alac_config,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_format_matches_the_decode_mapping() {
        // A 24-byte ALACSpecificConfig: 4096-sample frames, 16-bit,
        // stereo, 44.1 kHz (the ffmpeg encoder's own shape).
        let mut cookie = [0u8; 24];
        cookie[0..4].copy_from_slice(&4096u32.to_be_bytes());
        cookie[5] = 16;
        cookie[9] = 2;
        cookie[20..24].copy_from_slice(&44_100u32.to_be_bytes());
        let config = match parse_alac_config(&cookie) {
            Ok(config) => config,
            Err(e) => unreachable!("{e:?}"),
        };
        let format = output_format(&config);
        assert_eq!(format.sample, media_types::frame::SampleFormat::S16);
        assert_eq!(format.channels, 2);
        assert_eq!(format.rate, 44_100);
        assert_eq!(format.layout, media_types::frame::AudioLayout::Interleaved);
        // A bare frame-less packet is structure damage on the free
        // surface, not a decode.
        assert!(decode_frame(&[], &cookie).is_err());
    }
}
