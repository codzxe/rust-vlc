//! The ALAC fuzz harness (card 5.Q1; doc 04 §8.5 registry name
//! `fuzz_codec_alac`, table §3.1 row "ALAC").
//!
//! One call = one untrusted byte buffer through **every** ALAC
//! surface:
//!
//! 1. the cookie walk ([`parse_alac_config`](super::decoder::parse_alac_config))
//!    — the atom/QT magic-cookie parser under the dirtiest input;
//! 2. the free frame decode
//!    ([`decode_frame`](super::decoder::decode_frame)) with an
//!    input-derived cookie/packet split;
//! 3. the codec-api contract path (configure → packet → sink →
//!    flush).
//!
//! **The harness contract is absence-of-panic/hang/unbounded-
//! allocation, nothing else** — damage is expected to surface as a
//! typed error and is swallowed here; the AC suites pin the specific
//! outcomes.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_codec_alac.rs`
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's ALAC differential suite.
//!
//! # Bounds (MP §12)
//!
//! Memory is bounded by the input: the frame decode's output is at
//! most `samples-per-packet × channels × 4` bytes with the facts
//! derived from the input's own cookie bytes, a fixed number of
//! passes per call.

use codec_api::prelude::*;
use media_types::GenerationId;

use super::decoder::{AlacDecoder, decode_frame, parse_alac_config};

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
pub fn fuzz_codec_alac(data: &[u8]) {
    // Pass 1: the cookie walk over the whole buffer.
    let _ = parse_alac_config(data);

    // Pass 2: the free frame decode with an input-derived split —
    // the first 24..36 bytes as the cookie, the rest as the packet.
    if data.len() >= 48 {
        let cookie_len = 24 + usize::from(data[0] % 12);
        let _ = decode_frame(&data[cookie_len..], &data[..cookie_len]);
    }

    // Pass 3: the contract path, one configure → decode → flush
    // cycle. The cookie rides `extra_data` (the sanctioned channel).
    let Ok(descriptor) = media_types::ids::CodecDescriptor::new("alac") else {
        return;
    };
    let Ok(generation) = GenerationId::FIRST.next() else {
        return;
    };
    let mut decoder = AlacDecoder::new();
    let cookie = if data.len() >= 36 {
        &data[..36]
    } else {
        data
    };
    if decoder
        .configure(&DecoderConfig::new(
            descriptor,
            generation,
            ThreadBudget::default(),
            None,
            Some(media_types::packet::SharedBytes::from_vec(cookie.to_vec())),
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
        fuzz_codec_alac(&[]);
        fuzz_codec_alac(b"frmaalac");
        for length in 1..=40usize {
            let data: Vec<u8> = (0..length).map(|i| (i * 37 % 256) as u8).collect();
            fuzz_codec_alac(&data);
        }
    }

    #[test]
    fn hostile_wide_inputs_survive() {
        // All-0xFF (maximal false magic) and all-0x00.
        fuzz_codec_alac(&vec![0xff; 4096]);
        fuzz_codec_alac(&vec![0x00; 4096]);
        // A long enough buffer that every pass (incl. the split
        // decode) runs.
        fuzz_codec_alac(&vec![0xa5; 512]);
    }
}
