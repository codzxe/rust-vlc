//! FLAC audio decoder (card 5.A1; doc 04 §3.1 row "FLAC", doc 04 §2.1
//! row "FLAC (native)" — the Phase-5 lossless core).
//!
//! # Format slice (what this module owns)
//!
//! **Native FLAC** (the `fLaC` byte stream): the metadata chain
//! (STREAMINFO / SEEKTABLE parse — [`metadata`]), frame decode through
//! the pinned `claxon` 0.4.3 engine ([`decoder`], the dependency
//! decision and its purity record are documented there), and seeking —
//! SEEKTABLE resolution with a sample-accurate binary-search fallback
//! over frame headers ([`seek`]). Ogg-FLAC is out of this card's slice
//! (it is the Ogg demuxer card's pairing, doc 04 §2.2 "FLAC").
//!
//! # Module map
//!
//! - [`FlacDecoder`] — the codec-api [`Decoder`](codec_api::Decoder)
//!   implementation (negotiation + damage model documented on
//!   [`decoder`]).
//! - [`metadata`] — the `fLaC` probe, STREAMINFO, the metadata walk,
//!   SEEKTABLE → [`metadata::SeekIndex`].
//! - [`seek`] — [`seek::SeekPlanner`]: target sample → containing
//!   frame's byte offset (+ its first sample for the consumer's
//!   pre-roll drop).
//! - [`decode_frame`] — the free surface: one packet + facts → one
//!   decoded block with no contract state (the `decode_image` pattern
//!   of 2.B4; the [`fuzz`] entry).
//! - [`fuzz`] — the `fuzz_codec_flac` harness (`+FB`; wiring status
//!   documented there, honestly).
//!
//! # Conventions (crate-wide, unchanged)
//!
//! No `unsafe` on this module's own paths (the engine's 5 documented
//! `unsafe` sites are recorded in the dependency record); no panic on
//! any input — every decode/parse/seek path is bounds-checked, damage
//! is a typed [`EngineError`] and the decoder stays usable per frame
//! (FLAC frames are independent), never a crash (doc 03 §4.5(d)).
//!
//! # Bit-exactness (the card AC)
//!
//! FLAC is lossless; the reference output is the encoder's own PCM.
//! The differential tests (`crates/codecs/tests/flac_bitexact.rs`)
//! decode ffmpeg-generated fixtures (16-bit stereo 44.1 kHz, 24-bit,
//! 8-bit shift pin, SEEKTABLE carrier) and require **byte equality**
//! against ffmpeg's own decoded PCM, with the oracle commands and
//! digests recorded next to the fixtures. The VLC differential hook
//! (`tests/flac_vlc_differential.rs`) is registered and
//! honestly pending infra (no VLC in the sandbox — the Phase-3/4
//! pattern).

pub(crate) mod decoder;
pub(crate) mod error;
/// Fuzz harness (doc 04 §8.5 registry name `fuzz_codec_flac`): `pub`
/// because the registered target file and the in-tree survival tests
/// call it from outside the crate (the 2.B2/2.B4/2.B5 precedent).
pub mod fuzz;
pub(crate) mod metadata;
pub(crate) mod seek;

use codec_api::prelude::*;
use media_types::frame::AudioFormat;

pub use decoder::{DecodedFlac, FlacDecoder, decode_frame};
pub use metadata::{
    SeekIndex, SeekPoint, StreamInfo, StreamLayout, is_flac, parse_stream, parse_streaminfo,
};
pub use seek::{ParsedFrameHeader, SeekPlan, SeekPlanner};

/// Parse the FLAC frame header at `offset` (the public navigation
/// surface over [`seek::SeekPlanner::parse_header_at`]): sync,
/// reserved rules, UTF-8 number, escapes and CRC-8 all verified.
/// `None` = no parseable header at that offset. Consumers that need
/// the first-sample conversion pass the stream's [`StreamInfo`] to
/// [`ParsedFrameHeader::first_sample`].
#[must_use]
pub fn frame_header_at(data: &[u8], offset: u64) -> Option<ParsedFrameHeader> {
    SeekPlanner::parse_header_at(data, offset)
}

/// Decode one frame with facts parsed on the fly (convenience
/// composition over [`parse_streaminfo`] + [`decode_frame`] for
/// callers that hold a raw STREAMINFO blob).
///
/// # Errors
/// Typed [`EngineError`]s: the facts blob's rejection
/// ([`parse_streaminfo`]) or the frame's ([`decode_frame`]).
pub fn decode_frame_with_streaminfo(
    packet: &[u8],
    streaminfo_body: &[u8],
) -> Result<DecodedFlac, EngineError> {
    let facts = parse_streaminfo(streaminfo_body)?;
    decode_frame(packet, &facts)
}

/// The negotiated audio shape a configured stream will emit (the
/// registry/adapters' preview of [`AudioFormat`] without decoding —
/// derived from the same left-justify mapping as [`decode_frame`]).
#[must_use]
pub fn output_format(facts: &StreamInfo) -> AudioFormat {
    let (sample, _) = decoder::output_sample_format(facts.bits_per_sample);
    AudioFormat {
        sample,
        channels: facts.channels,
        layout: media_types::frame::AudioLayout::Interleaved,
        rate: facts.sample_rate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_format_matches_the_decode_mapping() {
        let mut body = [0u8; 34];
        body[0..2].copy_from_slice(&4096u16.to_be_bytes());
        body[2..4].copy_from_slice(&4096u16.to_be_bytes());
        let packed: u64 = (48_000u64 << 44) | (1 << 41) | (23 << 36) | 1_000;
        body[10..18].copy_from_slice(&packed.to_be_bytes());
        let facts = match parse_streaminfo(&body) {
            Ok(f) => f,
            Err(e) => unreachable!("{e:?}"),
        };
        let format = output_format(&facts);
        assert_eq!(format.sample, media_types::frame::SampleFormat::S24);
        assert_eq!(format.channels, 2);
        assert_eq!(format.rate, 48_000);
        assert_eq!(format.layout, media_types::frame::AudioLayout::Interleaved);
        assert!(decode_frame_with_streaminfo(&[], &body).is_err());
    }
}
