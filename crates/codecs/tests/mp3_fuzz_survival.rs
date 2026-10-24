//! `+FB` fuzz-lite survival for MP3/MP2 (card 5.A3): mutated and
//! truncated forms of the committed fixtures plus the corpus
//! `dirty`/`garbage` pools, driven through `decode_stream`.
//!
//! The contract (doc 03 §4.5(d)): NO panic, NO hang — every input is
//! either decoded or refused with a typed [`EngineError`]. Verdicts
//! are recorded per class (decoded / typed-error), never "must fail":
//! damage that walks to zero frames is a typed truncation, damage
//! inside a frame resyncs and decodes short.

mod common;
mod mp3;

use codecs::audio::mp3::decode_stream;
use media_types::EngineError;
use mp3::{corpus_dir, fixture_bytes};
use testkit::Must;

/// Deterministic LCG (the flac fuzz-survival convention: reproducible
/// mutants, no external rng dependency).
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
    for name in [
        "cbr128.mp3",
        "vbr_q4.mp3",
        "mono48.mp3",
        "mpeg2_24k.mp3",
        "mp2_48k.mp2",
        "tagged_art.mp3",
    ] {
        let pristine = fixture_bytes(name);
        let mut rng = Lcg(0x5A3_2026_1015 + u64::from(name.len() as u16));
        // 60 byte-flip mutants per fixture (single + burst).
        for k in 0..60 {
            let mut m = pristine.clone();
            let flips = if k % 3 == 0 { 4 } else { 1 };
            for _ in 0..flips {
                let at = rng.below(m.len());
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
    // typed refusals are all mp3.* codes (never a foreign stage).
    assert!(decoded + refused == 6 * 90, "tally sanity");
    for code in &codes {
        assert!(
            code.starts_with("mp3."),
            "foreign error code {code} leaked from the mp3 module"
        );
    }
    eprintln!(
        "mp3 fuzz-lite: {decoded} mutants decoded (possibly short), {refused} typed refusals, \
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
fn corpus_dirty_and_garbage_pools_survive() {
    let Some(corpus) = corpus_dir() else {
        eprintln!("SKIP mp3 corpus dirty/garbage survival: corpus root absent");
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
            // Any input, mp3 or not: the probe/decode surface must
            // settle. (decode_stream on non-MPEG data → typed refusal.)
            match decode_stream(&data) {
                Ok(_) => {}
                Err(EngineError { context, .. }) => {
                    // A non-MP3 file may legitimately be refused by a
                    // non-mp3 code only via the header parse — which
                    // lives in this module, so it is `mp3.*`.
                    assert!(
                        context.code.starts_with("mp3."),
                        "{}: foreign code {}",
                        path.display(),
                        context.code
                    );
                }
            }
            files_checked += 1;
        }
    }
    eprintln!("mp3 fuzz-lite: {files_checked} corpus dirty/garbage files survived");
}
