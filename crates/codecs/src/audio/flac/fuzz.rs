//! The FLAC codec fuzz harness (card 5.A1 `+FB`; doc 04 §8.5 registry
//! name `fuzz_codec_flac`, table §3.1 row "FLAC").
//!
//! One call = one untrusted byte buffer through **all three** decode
//! surfaces:
//!
//! 1. the metadata walk ([`super::metadata::parse_stream`]) — the
//!    dirty-world parser under the dirtiest input;
//! 2. the free frame decode ([`super::decode_frame`]) over input-
//!    derived and synthetic STREAMINFO facts;
//! 3. the codec-api contract path (configure → packet → sink →
//!    flush) plus the seek resolver over the same bytes.
//!
//! **The harness contract is absence-of-panic/hang/unbounded-
//! allocation, nothing else** — damage is expected to surface as a
//! typed error and is swallowed here; the AC suites (integration
//! tests) pin the specific outcomes.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target file is proposed at
//! `fuzz/fuzz_targets/fuzz_codec_flac.rs` exactly as doc 04 §8.5
//! registers it. Building it needs the `libfuzzer-sys` dev-dependency
//! plus the `fuzz/Cargo.toml` scaffolding — both Lead-owned dependency
//! decisions (AGENTS.md §3.2, C5) — so until they land the harness
//! logic is proven by the in-crate deterministic smoke tests here and
//! by the committed-fixture + seeded-mutation survival run
//! (`crates/codecs/tests/flac_fuzz_survival.rs`); the libFuzzer
//! campaign itself is owed CI wiring (the same status as
//! `fuzz_demux_avi` / `fuzz_codec_mjpeg` / `fuzz_codec_pcm`).
//!
//! # Bounds (MP §12)
//!
//! Memory is bounded by the input: the decode output is at most
//! `max_block_size (≤ 65 535) × channels (≤ 8) × 4` bytes with the
//! facts derived from the input itself, one buffer per pass, a fixed
//! number of passes per call; the seek resolver's probes are bounded
//! (`SCAN_WINDOW` sync scan, `O(log n)` bisection). There is no
//! attacker-controlled event count.

use codec_api::prelude::*;
use media_types::GenerationId;

use super::decoder::max_output_len;
use super::metadata::{StreamInfo, parse_stream};
use super::{FlacDecoder, decode_frame};

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

/// Synthetic STREAMINFO facts derived from the input (so the decode
/// surface is exercised even when no valid metadata is present):
/// block size, channels and bit depth all come from input bytes, in
/// their legal ranges.
fn synthetic_facts(data: &[u8]) -> StreamInfo {
    let a = u32::from(data.first().copied().unwrap_or(0));
    let b = u32::from(data.get(1).copied().unwrap_or(0));
    StreamInfo {
        min_block_size: 16,
        max_block_size: (256 << (a % 8)).max(16),
        min_frame_size: None,
        max_frame_size: None,
        sample_rate: [8_000, 44_100, 48_000][(b % 3) as usize],
        channels: 1 + b % 8,
        bits_per_sample: 4 + a % 29,
        total_samples: None,
        md5: [0u8; 16],
    }
}

/// Run one fuzz pass over `data` (see the module docs). Never panics,
/// never hangs, never allocates beyond input-derived bounds — by
/// construction of the bounded parse/decode/seek paths.
pub fn fuzz_codec_flac(data: &[u8]) {
    // Pass 1: the metadata walk.
    let facts = match parse_stream(data) {
        Ok(layout) => {
            // 1b: with a valid layout, the seek resolver must answer
            // (typed error or plan) for any target inside the stream.
            if let Some(total) = layout.stream_info.total_samples {
                let target = total / 2;
                if let Ok(planner) = super::seek::planner(
                    data,
                    &layout.stream_info,
                    layout.seek_index.as_ref(),
                    layout.frames_start,
                ) {
                    let _ = planner.resolve(target);
                }
            }
            layout.stream_info
        }
        Err(_) => synthetic_facts(data),
    };

    // Pass 2: the free frame decode — the whole buffer as one packet
    // (invalid as a frame or not; only survivability is asserted).
    let _ = decode_frame(data, &facts);

    // Pass 3: the codec-api contract path, one full configure →
    // decode → flush cycle. Facts ride `extra_data` (the sanctioned
    // channel); the packet is the buffer, sliced by input bytes.
    let Ok(descriptor) = media_types::ids::CodecDescriptor::new("flac") else {
        return;
    };
    let Ok(generation) = GenerationId::FIRST.next() else {
        return;
    };
    let mut decoder = FlacDecoder::new();
    let facts_bytes = {
        let mut bytes = Vec::with_capacity(34);
        bytes.extend_from_slice(&facts.min_block_size.to_be_bytes());
        bytes.extend_from_slice(&facts.max_block_size.to_be_bytes());
        let packed: u64 = (u64::from(facts.sample_rate) << 44)
            | (u64::from(facts.channels - 1) << 41)
            | (u64::from(facts.bits_per_sample - 1) << 36)
            | facts.total_samples.unwrap_or(0);
        bytes.extend_from_slice(&packed.to_be_bytes());
        bytes.extend_from_slice(&facts.md5);
        bytes
    };
    if decoder
        .configure(&DecoderConfig::new(
            descriptor,
            generation,
            ThreadBudget::default(),
            None,
            Some(media_types::packet::SharedBytes::from_vec(facts_bytes)),
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

    // Pass 4: decode with the maximum-slot sizing (the pooled path's
    // worst case: a frame that fills the whole declared maximum).
    let mut slot = vec![0u8; max_output_len(&facts).min(1 << 20)];
    let mut scratch = Vec::new();
    let _ = super::decoder::decode_into(data, &facts, &mut scratch, &mut slot);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_survives_all_passes() {
        fuzz_codec_flac(&[]);
    }

    #[test]
    fn marker_only_and_short_inputs_survive() {
        fuzz_codec_flac(b"fLaC");
        for length in 1..=40usize {
            let data: Vec<u8> = (0..length).map(|i| (i * 37 % 256) as u8).collect();
            fuzz_codec_flac(&data);
        }
    }

    #[test]
    fn hostile_wide_inputs_survive() {
        // The two degenerate byte patterns: all-0xFF (maximal sync
        // false positives) and all-0x00 (maximal zero-length fields).
        fuzz_codec_flac(&vec![0xff; 4096]);
        fuzz_codec_flac(&vec![0x00; 4096]);
        // A sync-code field of 0xFF bytes with plausible headers.
        let mut sync_flood = vec![0xffu8; 8192];
        for (i, byte) in [0xF8u8, 0x70, 0x00, 0xFF].iter().enumerate() {
            sync_flood[i * 16 + 1] = *byte;
        }
        fuzz_codec_flac(&sync_flood);
    }

    #[test]
    fn synthetic_facts_stay_in_the_legal_domain() {
        for data in [&[0u8][..], &[255u8, 255][..], &[1u8, 2, 3, 4, 5][..]] {
            let facts = synthetic_facts(data);
            assert!(facts.channels >= 1 && facts.channels <= 8);
            assert!((4..=32).contains(&facts.bits_per_sample));
            assert!(facts.max_block_size >= 16);
            assert!(facts.sample_rate > 0);
            // The slot bound stays input-derived and finite.
            assert!(max_output_len(&facts) <= 65_535 * 8 * 4);
        }
    }
}
