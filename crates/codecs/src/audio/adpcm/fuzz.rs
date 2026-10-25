//! The ADPCM fuzz harness (card 5.Q1; doc 04 §8.5 registry name
//! `fuzz_codec_adpcm`, table §3.1 row "ADPCM (MS/IMA/OKI/Yamaha…)").
//!
//! One call = one untrusted byte buffer through the codec-api
//! contract path for **every accepted flavor**: configure → packet →
//! sink → flush, over a descriptor cycle
//! (`adpcm_ms`, `adpcm_ima_wav`, `adpcm_yamaha`, `adpcm_ima_oki`,
//! `adpcm_swf`, `adpcm_ima_qt`, `adpcm_ima_ws`, `adpcm_oki`):
//!
//! - the WAV flavors take the input bytes themselves as `extra_data`
//!   (the `fmt `-chunk seam — hostile tag/block geometry under the
//!   dirtiest input);
//! - the raw breadth flavors take a **synthesized 16-byte
//!   WAVEFORMATEX skeleton** derived from the input (tag 0x0000, a
//!   legal channel pair, a real clock), so the decode surface is
//!   exercised even when no container is present;
//! - `adpcm_oki` is the lettered-variant typed refusal — the
//!   configure error is the expected outcome and is swallowed.
//!
//! **The harness contract is absence-of-panic/hang/unbounded-
//! allocation, nothing else** — damage is expected to surface as a
//! typed error and is swallowed here; the AC suites pin the specific
//! outcomes.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_codec_adpcm.rs`
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's breadth differential suite.
//!
//! # Bounds (MP §12)
//!
//! Memory is bounded by the input: the flavor geometry bounds the
//! worst-case output (≤ 8 output bytes per input byte), a fixed
//! number of passes per call, one packet per configure.

use codec_api::prelude::*;
use media_types::GenerationId;

use super::AdpcmDecoder;

/// The descriptor cycle (see the module docs). `adpcm_oki` is the
/// typed-refusal flavor; the headerless DVI/raw `adpcm_ima` gap is
/// documented in the decoder module and deliberately absent here.
const FLAVORS: [&str; 8] = [
    "adpcm_ms",
    "adpcm_ima_wav",
    "adpcm_yamaha",
    "adpcm_ima_oki",
    "adpcm_swf",
    "adpcm_ima_qt",
    "adpcm_ima_ws",
    "adpcm_oki",
];

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

/// The synthesized raw-facts skeleton (16 bytes, `parse_raw_facts`'s
/// exact floor): tag 0x0000, 1–2 channels, a real clock — every
/// field derived from the input so hostile bytes stay hostile.
fn raw_facts_skeleton(data: &[u8]) -> [u8; 16] {
    let a = data.first().copied().unwrap_or(0);
    let b = data.get(1).copied().unwrap_or(0);
    let mut out = [0u8; 16];
    out[0..2].copy_from_slice(&0u16.to_le_bytes());
    out[2..4].copy_from_slice(&u16::from(1 + a % 2).to_le_bytes());
    let rate = [8_000u32, 11_025, 22_050, 44_100, 48_000][(b % 5) as usize];
    out[4..8].copy_from_slice(&rate.to_le_bytes());
    out
}

/// Run one fuzz pass over `data` (see the module docs). Never panics,
/// never hangs, never allocates beyond input-derived bounds — by
/// construction of the bounded parse/decode paths.
pub fn fuzz_codec_adpcm(data: &[u8]) {
    for (index, flavor) in FLAVORS.iter().enumerate() {
        let Ok(descriptor) = media_types::ids::CodecDescriptor::new(flavor) else {
            continue;
        };
        let Ok(generation) = GenerationId::FIRST.next() else {
            return;
        };
        let extra = if index < 2 {
            media_types::packet::SharedBytes::from_vec(data.to_vec())
        } else {
            media_types::packet::SharedBytes::from_vec(raw_facts_skeleton(data).to_vec())
        };
        let mut decoder = AdpcmDecoder::new();
        if decoder
            .configure(&DecoderConfig::new(
                descriptor,
                generation,
                ThreadBudget::default(),
                None,
                Some(extra),
            ))
            .is_err()
        {
            continue;
        }
        let packet = Packet {
            track: media_types::TrackId::new(0),
            pts: media_types::Stamp::Missing,
            dts: media_types::Stamp::Missing,
            duration: None,
            data: media_types::packet::SharedBytes::from_vec(data.to_vec()),
            keyframe: true,
            discontinuity: false,
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
    fn empty_and_short_inputs_survive() {
        fuzz_codec_adpcm(&[]);
        fuzz_codec_adpcm(&[0x00, 0x00, 0x01, 0x00]);
        for length in 1..=32usize {
            let data: Vec<u8> = (0..length).map(|i| (i * 37 % 256) as u8).collect();
            fuzz_codec_adpcm(&data);
        }
    }

    #[test]
    fn hostile_wide_inputs_survive() {
        fuzz_codec_adpcm(&vec![0xff; 4096]);
        fuzz_codec_adpcm(&vec![0x00; 4096]);
    }

    #[test]
    fn raw_facts_skeleton_stays_legal() {
        for data in [&[0u8][..], &[255u8, 255][..], &[7u8, 3][..]] {
            let skeleton = raw_facts_skeleton(data);
            let channels = u16::from_le_bytes([skeleton[2], skeleton[3]]);
            let rate = u32::from_le_bytes([skeleton[4], skeleton[5], skeleton[6], skeleton[7]]);
            assert!((1..=2).contains(&channels));
            assert!((8_000..=48_000).contains(&rate));
        }
    }
}
