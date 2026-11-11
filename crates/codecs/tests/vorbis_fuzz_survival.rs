//! `+FB` fuzz-lite survival for Vorbis (card 5.B2): mutated and
//! truncated forms of the committed fixtures plus the corpus 0085
//! stream, driven through the registered `fuzz_codec_vorbis` harness
//! (all three decode surfaces: header chain, free block, contract
//! path — see `src/audio/vorbis/fuzz.rs`).
//!
//! The contract (doc 03 §4.5(d)): NO panic, NO hang, bounded
//! allocation — every input is either decoded or refused with a typed
//! error inside the harness. This suite's assertion is the survival
//! itself; the specific error-path verdicts (bad magic, truncated
//! setup, mid-stream corruption) are pinned by `vorbis_bitexact.rs`
//! and the in-module unit tests.

mod common;
mod vorbis;

use codecs::audio::vorbis::fuzz::fuzz_codec_vorbis;
use testkit::Must;
use vorbis::{corpus_dir, fixture_bytes};

/// Deterministic LCG (the flac/mp3 fuzz-survival convention:
/// reproducible mutants, no external rng dependency).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 16
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

#[test]
fn mutated_and_truncated_inputs_never_panic() {
    let mut mutants = 0usize;
    let mut truncations = 0usize;
    for name in [
        "vorbis_sine_44100_stereo.ogg",
        "vorbis_sine_8000_mono.ogg",
        "vorbis_silence_48000_stereo.ogg",
        "vorbis_tagged_sine.ogg",
    ] {
        let pristine = fixture_bytes(name);
        let mut rng = Lcg(0x5B2_2026_1026 + u64::from(name.len() as u16));
        // 60 byte-flip mutants per fixture (single + burst): container
        // bytes, header-chain bytes and packet bytes all fall in the
        // blast radius — the harness routes them through whichever
        // surface they parse as.
        for k in 0..60 {
            let mut m = pristine.clone();
            let flips = if k % 3 == 0 { 4 } else { 1 };
            for _ in 0..flips {
                let at = rng.below(m.len());
                m[at] ^= (rng.below(255) + 1) as u8;
            }
            fuzz_codec_vorbis(&m);
            mutants += 1;
        }
        // 20 truncations at deterministic points (head/mid/tail).
        for k in 0..20 {
            let at = (k + 1) * pristine.len() / 21;
            fuzz_codec_vorbis(&pristine[..at]);
            truncations += 1;
        }
        // The pristine fixture itself (the sanity lane).
        fuzz_codec_vorbis(&pristine);
    }
    // The real corpus row when present (skip-with-msg otherwise).
    let mut corpus_lane = false;
    if let Some(root) = corpus_dir() {
        let path = root.join("media/0085-audio-only.ogg");
        if path.is_file() {
            corpus_lane = true;
            let data = std::fs::read(&path).must();
            let mut rng = Lcg(0x0085);
            for _ in 0..40 {
                let mut m = data.clone();
                for _ in 0..2 {
                    let at = rng.below(m.len());
                    m[at] ^= (rng.below(255) + 1) as u8;
                }
                fuzz_codec_vorbis(&m);
                mutants += 1;
            }
            fuzz_codec_vorbis(&data);
        }
    } else {
        eprintln!("corpus: LUMEN_CORPUS unset and /tmp/my-project/corpus-v4 absent — SKIP");
    }
    // 60 mutants x 4 fixtures, + 40 corpus mutants when the corpus lane
    // is runnable (the skip-with-msg path documented above).
    assert_eq!(
        mutants,
        240 + usize::from(corpus_lane) * 40,
        "60 mutants x 4 fixtures + 40 corpus mutants (corpus lane run: {corpus_lane})"
    );
    assert_eq!(truncations, 80, "20 truncations x 4 fixtures");
    eprintln!(
        "vorbis fuzz-lite: {mutants} mutants + {truncations} truncations survived (no panic, no hang)"
    );
}
