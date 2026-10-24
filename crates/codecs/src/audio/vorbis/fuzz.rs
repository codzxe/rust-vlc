//! The Vorbis codec fuzz harness (card 5.B2 `+FB`; doc 04 §8.5
//! registry name `fuzz_codec_vorbis`, table §3.1 row "Vorbis").
//!
//! One call = one untrusted byte buffer through **all three** decode
//! surfaces:
//!
//! 1. the header chain ([`super::headers::VorbisHeaders::
//!    from_extra_data`]) — the dirty-world parser under the dirtiest
//!    input, plus the comment parser over the same bytes;
//! 2. the free block decode ([`super::decode_block`]) when the chain
//!    happens to validate (a mutated setup usually dies in the
//!    engine — typed, never a panic);
//! 3. the codec-api contract path (configure → packet → sink →
//!    flush) with the input-derived chain and the buffer sliced by
//!    input bytes, twice — once clean, once with the discontinuity
//!    flag set (the seek-reanchor reset path).
//!
//! **The harness contract is absence-of-panic/hang/unbounded-
//! allocation, nothing else** — damage is expected to surface as a
//! typed error and is swallowed here; the AC suites (integration
//! tests) pin the specific outcomes.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target file is proposed at
//! `fuzz/fuzz_targets/fuzz_codec_vorbis.rs` exactly as doc 04 §8.5
//! registers it. Building it needs the `libfuzzer-sys` dev-dependency
//! plus the `fuzz/Cargo.toml` scaffolding — both Lead-owned
//! dependency decisions (AGENTS.md §3.2, C5) — so until they land the
//! harness logic is proven by the in-crate deterministic smoke tests
//! here and by the committed-fixture + seeded-mutation survival run
//! (`crates/codecs/tests/vorbis_fuzz_survival.rs`); the libFuzzer
//! campaign itself is owed CI wiring (the same status as
//! `fuzz_codec_flac` / `fuzz_codec_mjpeg` / `fuzz_codec_pcm`).
//!
//! # Bounds (MP §12)
//!
//! Memory is bounded by the input: the decode output is at most
//! `blocksize_long (≤ 8192) / 2 × channels (≤ 8) × 4` bytes with the
//! geometry derived from the input itself, one buffer per pass, a
//! fixed number of passes per call; the header chain's length fields
//! are all bounds-checked against the buffer (a lying length buys a
//! typed truncation, not an allocation). There is no
//! attacker-controlled event count.

use codec_api::prelude::*;
use media_types::{GenerationId, Stamp, TrackId, packet::SharedBytes};

use super::decoder::VorbisDecoder;
use super::headers::VorbisHeaders;
use super::{decode_block, parse_identification};

/// A frame sink that only counts (the panic-free emission target of
/// the contract-path pass).
#[derive(Default)]
struct CountingSink {
    frames: usize,
}

impl FrameSink for CountingSink {
    fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
        self.frames = self.frames.saturating_add(1);
        Ok(())
    }
}

/// Run one fuzz pass over `data` (see the module docs). Never panics,
/// never hangs, never allocates beyond input-derived bounds — by
/// construction of the bounded parse/decode paths.
pub fn fuzz_codec_vorbis(data: &[u8]) {
    // Pass 1: the header chain + comment parse. A valid chain also
    // feeds the free block decode (pass 2) — the engine's setup
    // parser runs under the input's own bytes.
    let headers = match VorbisHeaders::from_extra_data(data) {
        Ok(headers) => headers,
        Err(_) => {
            // Even a rejected chain must have parsed the
            // identification prefix without panic; exercise it
            // standalone too (it is the first surface any container
            // sniffs).
            let _ = parse_identification(data);
            return;
        }
    };

    // Pass 2: the free block decode — the whole buffer as one audio
    // packet (invalid as a block or not; only survivability is
    // asserted; a mid-buffer slice exercises the truncated tail).
    let cut = usize::from(data.first().copied().unwrap_or(0)) % (data.len() + 1);
    let _ = decode_block(&data[cut.min(data.len())..], &headers);

    // Pass 3: the codec-api contract path, one full configure →
    // decode → flush cycle, twice (clean + discontinuity reset).
    let Ok(descriptor) = media_types::ids::CodecDescriptor::new("vorbis") else {
        return;
    };
    let Ok(generation) = GenerationId::FIRST.next() else {
        return;
    };
    let mut decoder = VorbisDecoder::new();
    if decoder
        .configure(&DecoderConfig::new(
            descriptor,
            generation,
            ThreadBudget::default(),
            None,
            Some(SharedBytes::from_vec(data.to_vec())),
        ))
        .is_err()
    {
        return;
    }
    for discontinuity in [false, true] {
        let packet = Packet {
            track: TrackId::new(0),
            // Both passes land unstamped: the harness never invents a
            // timeline (the shift/pts surface is exercised by the AC
            // suites); the flag under test is the overlap reset.
            pts: Stamp::Missing,
            dts: Stamp::Missing,
            duration: None,
            data: SharedBytes::from_vec(data[..cut].to_vec()),
            keyframe: true,
            discontinuity,
            generation,
        };
        let mut sink = CountingSink::default();
        let _ = decoder.decode(packet, &mut sink);
        let _ = decoder.flush(&mut sink);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_survives_all_passes() {
        fuzz_codec_vorbis(&[]);
    }

    #[test]
    fn marker_only_and_short_inputs_survive() {
        for prefix in [&[0x01u8][..], &[0x01u8, b'v'][..], &b"\x01vorbis"[..]] {
            fuzz_codec_vorbis(prefix);
        }
        for length in 1..=40usize {
            let data: Vec<u8> = (0..length).map(|i| (i * 37 % 256) as u8).collect();
            fuzz_codec_vorbis(&data);
        }
    }

    #[test]
    fn hostile_wide_inputs_survive() {
        // The two degenerate byte patterns: all-0xFF (maximal
        // length-field claims) and all-0x00 (zero fields, clear
        // framing bits).
        fuzz_codec_vorbis(&vec![0xff; 4096]);
        fuzz_codec_vorbis(&vec![0x00; 4096]);
        // A legal-looking identification prefix followed by hostile
        // comment/setup bytes.
        let mut hostile = vec![0x01u8];
        hostile.extend_from_slice(b"vorbis");
        hostile.extend_from_slice(&[0, 0, 0, 0]); // version 0
        hostile.push(2); // channels
        hostile.extend_from_slice(&44_100u32.to_le_bytes());
        hostile.extend_from_slice(&[0u8; 12]); // bitrates
        hostile.push(8 | (11 << 4)); // blocksizes
        hostile.push(0x01); // framing
        hostile.extend_from_slice(&vec![0xff; 2048]);
        fuzz_codec_vorbis(&hostile);
        fuzz_codec_vorbis(&hostile[..hostile.len() / 2]);
    }
}
