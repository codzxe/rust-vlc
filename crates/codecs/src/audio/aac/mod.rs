//! AAC audio decoder (card 5.C2a — the DECODER half of 5.C2; doc 04
//! §3.1 rows "AAC-LC" and "AAC in MP4 vs ADTS", §2.2 row "ADTS AAC")
//! — the lossy-audio workhorse for the MP4/M4A and ADTS lanes.
//!
//! # Format slice (what this module owns)
//!
//! **The codec side of AAC**: the 7/9-byte ADTS fixed-header parse
//! ([`adts`]) — syncword, ID/layer, profile, sampling-frequency
//! index, channel configuration, frame length — with the
//! resync-on-garbage walker; the stream pipeline and the
//! [`codec_api::Decoder`] implementation ([`decoder`]) — ADTS mode
//! (one packet = one complete ADTS frame, header included; the engine
//! is built lazily from the first frame's parameters) and ASC mode
//! (`extra_data` = the AudioSpecificConfig, the MP4/esds path); the
//! typed error taxonomy ([`error`]). The entropy decode of one raw
//! data block is the purity-pinned `symphonia-codec-aac` 0.6.1
//! engine's job. The **demuxer side** (ADTS probe/packetization/
//! seeking, the esds unwrap) is the parallel 5.C2b card's — this
//! module never touches `crates/demux`.
//!
//! # HE-AAC honesty (doc 04 §3.2, the card's "honestly partial")
//!
//! The engine implements AAC-LC only, mono/stereo. Explicit-ASC HE
//! (object type 5/29) refuses typedly (`aac.he_sbr`); non-LC profiles
//! refuse `aac.profile`; channel-config 0 (PCE) and > 2 channels
//! refuse `aac.channel_config_pce` / `aac.channel_count`. Implicit
//! ADTS HE signaling is undetectable from the header (the SBR
//! extension hides behind full SCE/CPE syntax) — such streams decode
//! as **half-rate LC**, and the differential records the degradation
//! rather than silently passing it. SBR/PS is doc 04's self-build M,
//! unscheduled.
//!
//! # Module map
//!
//! - [`AacDecoder`] — the codec-api [`Decoder`](codec_api::Decoder)
//!   implementation.
//! - [`decode_stream`] — the free whole-ADTS-stream surface (the
//!   conformance and differential workhorse).
//! - [`adts`] — the ADTS fixed-header parse + the resync walker.
//! - [`error`] — the typed taxonomy.
//! - [`fuzz`] — the `fuzz_codec_aac` harness (`+FB`; wiring status
//!   documented there, honestly).
//!
//! # Conventions (crate-wide, unchanged)
//!
//! No `unsafe` on this module's own paths (the engine crate is
//! `#![forbid(unsafe_code)]` itself, recorded in the dependency
//! record); no panic on any input — every decode/parse path is
//! bounds-checked and damage is a typed [`EngineError`] (bad sync
//! resyncs, truncated frames truncate, lying headers refuse), never
//! a crash (doc 03 §4.5(d)).
//!
//! # Bit-exactness (the honest AC posture)
//!
//! AAC is lossy: two independent decoder implementations are not
//! sample-identical in general. The differential compares against
//! ffmpeg on the SAME frame window (all complete ADTS frames,
//! trailing partial dropped on both sides — verified: ffmpeg decodes
//! exactly `frames × 1024` samples) and records the PCM MD5 plus the
//! max sample delta per file (`tests/aac_corpus_differential.rs`).

pub(crate) mod adts;
pub(crate) mod decoder;
pub(crate) mod error;
/// Fuzz harness (doc 04 §3.1 registry name `fuzz_codec_aac`): `pub`
/// because the registered target file and the in-tree survival tests
/// call it from outside the crate (the 2.B2/2.B4/5.A1 precedent).
pub mod fuzz;

pub use decoder::{AacDecoder, DecodedAac, decode_stream};

/// Whether `data` begins like a raw ADTS AAC stream: a valid fixed
/// header (full validation — sync, layer, tables) at offset 0 (the
/// codec-side probe the 5.C2b demuxer row complements; doc 04 §2.2
/// "ADTS AAC" row).
#[must_use]
pub fn is_adts_stream(data: &[u8]) -> bool {
    adts::parse_adts_header(data).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_accepts_validated_headers_only() {
        // MPEG-4, no CRC, LC, 44.1 kHz, stereo, 64-byte frame.
        assert!(is_adts_stream(&[0xFF, 0xF1, 0x50, 0x80, 0x08, 0x1F, 0xFC]));
        assert!(!is_adts_stream(&[0xFF, 0xF3, 0x50, 0x80, 0x08, 0x1F, 0xFC])); // layer 1
        assert!(!is_adts_stream(&[0xFF, 0x00, 0x00, 0x00]));
        assert!(!is_adts_stream(b"fLaC"));
        assert!(!is_adts_stream(&[]));
    }
}
