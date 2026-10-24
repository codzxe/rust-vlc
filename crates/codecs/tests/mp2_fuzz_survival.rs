//! `+FB` fuzz-lite survival for MP2 (card 5.J3): mutated and
//! truncated forms of the committed fixtures plus the corpus
//! `dirty`/`garbage` pools, driven through every surface the
//! `fuzz_codec_mp2` harness owns.
//!
//! The contract (doc 03 §4.5(d)): NO panic, NO hang — every input is
//! either decoded or refused with a typed [`EngineError`]. Verdicts
//! are recorded per class (decoded / typed-error), never "must fail":
//! damage that walks to zero frames is a typed truncation, damage
//! inside a frame resyncs and decodes short, a layer or CBR switch is
//! a typed refusal.
//!
//! Committed fuzz inputs: the two mp2 fixtures ≈ 56 KB total (the
//! card's < 200 KB budget; the corpus is read from disk, not
//! committed).

mod common;
mod mp2;

use codecs::audio::mp2::decode_stream;
use codecs::audio::mp2::fuzz::fuzz_mp2_stream;
use media_types::EngineError;
use mp2::{corpus_dir, fixture_bytes};
use testkit::Must;

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

/// One mutant's outcome, for the recorded tally.
enum Outcome {
    Decoded,
    Typed(String),
}

fn classify(data: &[u8]) -> Outcome {
    match decode_stream(data) {
        Ok(_) => Outcome::Decoded,
        Err(EngineError { context, .. }) => Outcome::Typed(context.code.clone()),
    }
}

#[test]
fn mutated_fixtures_never_panic_and_settle_typed() {
    let mut decoded = 0usize;
    let mut refused = 0usize;
    let mut codes: Vec<String> = Vec::new();
    for name in ["mp2_48k.mp2", "mp2_lsf_22k.mp2"] {
        let pristine = fixture_bytes(name);
        let mut rng = Lcg(0x5A3_2026_1024 + u64::from(name.len() as u16));
        // 80 byte-flip mutants per fixture (single + burst), biased
        // into the header region where the layer/CBR gates live.
        for k in 0..80 {
            let mut m = pristine.clone();
            let flips = if k % 3 == 0 { 4 } else { 1 };
            for _ in 0..flips {
                // Half the flips target the first 64 bytes of a frame
                // (headers), half are uniform.
                let at = if rng.below(2) == 0 {
                    rng.below(m.len().min(64))
                } else {
                    rng.below(m.len())
                };
                m[at] ^= (rng.below(255) + 1) as u8;
            }
            match classify(&m) {
                Outcome::Decoded => decoded += 1,
                Outcome::Typed(code) => {
                    refused += 1;
                    codes.push(code);
                }
            }
        }
        // 20 truncations at deterministic points (head/mid/tail).
        for k in 0..20 {
            let cut = (pristine.len() * (k + 1)) / 21;
            let m = &pristine[..cut];
            match classify(m) {
                Outcome::Decoded => decoded += 1,
                Outcome::Typed(code) => {
                    refused += 1;
                    codes.push(code);
                }
            }
        }
        // 10 garbage prefixes (0–64 bytes) in front of pristine audio.
        for k in 0..10 {
            let mut m = Vec::new();
            let len = rng.below(65);
            for _ in 0..len {
                m.push(rng.below(256) as u8);
            }
            m.extend_from_slice(&pristine[k * 17.min(pristine.len())..]);
            match classify(&m) {
                Outcome::Decoded => decoded += 1,
                Outcome::Typed(code) => {
                    refused += 1;
                    codes.push(code);
                }
            }
        }
    }
    // The survival contract itself: every mutant settled (this line
    // only runs because nothing panicked or hung above), and the
    // typed refusals are all this module's codes (never a foreign
    // stage — the family's mp3.* codes are legal passthroughs from
    // the shared header parse).
    assert!(decoded + refused == 2 * 110, "tally sanity");
    for code in &codes {
        assert!(
            code.starts_with("mp2.") || code.starts_with("mp3."),
            "foreign error code {code} leaked from the mp2 module"
        );
    }
    eprintln!(
        "mp2 fuzz-lite: {decoded} mutants decoded (possibly short), {refused} typed refusals, \
         codes observed: {:?}",
        codes
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
    );
    // Damage must be *possible* to refuse: at least one typed refusal
    // happened across the sweep (the taxonomy is reachable, not dead).
    assert!(refused > 0, "no mutant produced a typed refusal");
}

#[test]
fn the_fuzz_harness_survives_the_same_mutations() {
    // The registered-target logic itself (fuzz_codec_mp2) over the
    // same mutant classes — absence-of-panic, outcome digested.
    let pristine = fixture_bytes("mp2_lsf_22k.mp2");
    let mut rng = Lcg(0x5A3_2026_1024 + 0x1B2D);
    for _ in 0..120 {
        let mut m = pristine.clone();
        let flips = 1 + rng.below(8);
        for _ in 0..flips {
            let at = rng.below(m.len());
            m[at] ^= (rng.below(255) + 1) as u8;
        }
        if rng.below(3) == 0 {
            let cut = rng.below(m.len());
            m.truncate(cut);
        }
        let outcome = fuzz_mp2_stream(&m);
        // Observable, bounded results only.
        assert!(outcome.frames <= m.len() / 17 + 1);
        assert!(outcome.contract_frames <= 64);
    }
}

#[test]
fn corpus_dirty_and_garbage_pools_survive() {
    let Some(corpus) = corpus_dir() else {
        eprintln!("SKIP mp2 corpus dirty/garbage survival: corpus root absent");
        return;
    };
    let mut files_checked = 0usize;
    for pool in ["dirty", "garbage"] {
        let dir = corpus.join(pool);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut names: Vec<_> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file())
            .collect();
        names.sort();
        // Whole pools are for the registered fuzz harness; the
        // fuzz-lite slice takes every file up to 40 per pool.
        for path in names.into_iter().take(40) {
            let data = std::fs::read(&path).must();
            // Any input, mp2 or not: the probe/decode surface must
            // settle. (decode_stream on non-MPEG data → typed refusal;
            // a non-Layer II MPEG frame → the layer gate.)
            match decode_stream(&data) {
                Ok(_) => {}
                Err(EngineError { context, .. }) => {
                    assert!(
                        context.code.starts_with("mp2.") || context.code.starts_with("mp3."),
                        "{}: foreign code {}",
                        path.display(),
                        context.code
                    );
                }
            }
            let _ = fuzz_mp2_stream(&data);
            files_checked += 1;
        }
    }
    eprintln!("mp2 fuzz-lite: {files_checked} corpus dirty/garbage files survived");
}
