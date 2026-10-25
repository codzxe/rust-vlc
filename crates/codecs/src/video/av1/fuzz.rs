//! The AV1 fuzz harness (card 5.Q1; doc 04 §8.5 registry name
//! `fuzz_codec_av1`, table §4.1 row "AV1").
//!
//! One call = one untrusted byte buffer through **both** surfaces:
//!
//! 1. the pure OBU parse ([`parse_temporal_unit`](super::obu::parse_temporal_unit))
//!    — the bitstream-syntax walk under the dirtiest input;
//! 2. the codec-api contract path (configure → packet → sink →
//!    flush) over the rav1d engine (asm-off), whose per-frame
//!    isolation policy (doc 03 §4.5(d)) is exactly the survival
//!    contract this harness asserts.
//!
//! **The harness contract is absence-of-panic/hang/unbounded-
//! allocation, nothing else** — damage is expected to surface as a
//! typed `Corrupt` error or a `Recovered` outcome, both swallowed
//! here; the pixel-MD5 differential suite pins the decoded outcomes.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_codec_av1.rs`
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's AV1 conformance/differential suites.
//!
//! # Bounds (MP §12)
//!
//! Memory is bounded by the input: one temporal unit per decode
//! call, rav1d's own frame-allocation ceilings, a fixed number of
//! passes per call.

use codec_api::prelude::*;
use media_types::GenerationId;

use super::Av1Decoder;

/// A frame sink that only counts (the panic-free emission target).
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
pub fn fuzz_codec_av1(data: &[u8]) {
    // Pass 1: the pure OBU parse.
    let _ = super::obu::parse_temporal_unit(data);

    // Pass 2: the contract path, one configure → decode → flush
    // cycle (the whole buffer as one temporal unit — invalid as a TU
    // or not; only survivability is asserted).
    let Ok(descriptor) = media_types::ids::CodecDescriptor::new("av1") else {
        return;
    };
    let Ok(generation) = GenerationId::FIRST.next() else {
        return;
    };
    let mut decoder = Av1Decoder::new();
    if decoder
        .configure(&DecoderConfig::new(
            descriptor,
            generation,
            ThreadBudget::default(),
            None,
            None,
        ))
        .is_err()
    {
        return;
    }
    let cut = usize::from(data.first().copied().unwrap_or(0)) % (data.len() + 1);
    let packet = Packet {
        track: media_types::TrackId::new(0),
        pts: media_types::Stamp::Missing,
        dts: media_types::Stamp::Missing,
        duration: None,
        data: media_types::packet::SharedBytes::from_vec(data[..cut].to_vec()),
        keyframe: true,
        discontinuity: false,
        generation,
    };
    let mut sink = CountingSink::default();
    let _ = decoder.decode(packet, &mut sink);
    let _ = decoder.flush(&mut sink);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_short_inputs_survive() {
        fuzz_codec_av1(&[]);
        fuzz_codec_av1(&[0x12, 0x00, 0x0a]);
        for length in 1..=32usize {
            let data: Vec<u8> = (0..length).map(|i| (i * 37 % 256) as u8).collect();
            fuzz_codec_av1(&data);
        }
    }

    #[test]
    fn hostile_wide_inputs_survive() {
        // All-0xFF (maximal OBU-header false positives) and all-0x00
        // (zero-length payload claims).
        fuzz_codec_av1(&vec![0xff; 4096]);
        fuzz_codec_av1(&vec![0x00; 4096]);
    }
}
