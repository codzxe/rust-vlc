//! AAC fuzz-lite survival (card 5.C2a `+FB`): seeded deterministic
//! mutations of the committed `fuzz_base_lc.aac` fixture (an
//! ffmpeg-authored 1 s / 22.05 kHz / 16 kbps mono LC stream, 2.5 KB)
//! through `codecs::audio::aac::fuzz::fuzz_aac_stream` — the harness
//! that the proposed `fuzz/fuzz_targets/fuzz_codec_aac.rs` target
//! will wrap (wiring status honestly documented in the harness's
//! module docs).
//!
//! The contract is absence-of-panic/hang (every mutant either decodes
//! or settles into a TYPED error — recorded by code), never a crash
//! (doc 03 §4.5(d)). The committed bytes stay under the card's
//! 200 KB budget: the base fixture only, mutants generated in memory
//! (the 5.A3 `mp3_fuzz_survival.rs` convention).

mod aac;

use aac::fixture_bytes;
use codecs::audio::aac::fuzz::fuzz_aac_stream;
use media_types::EngineError;

/// A deterministic LCG (the mp3 survival harness's — reproducible
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
    match fuzz_aac_stream(data).decoded {
        true => Outcome::Decoded,
        // The harness swallows the typed error; recover it through the
        // direct stream surface for the code tally.
        false => match codecs::audio::aac::decode_stream(data) {
            Ok(_) => Outcome::Decoded,
            Err(EngineError { context, .. }) => Outcome::Typed(context.code.clone()),
        },
    }
}

#[test]
fn mutated_fixtures_never_panic_and_settle_typed() {
    let pristine = fixture_bytes("fuzz_base_lc.aac");
    let mut decoded = 0usize;
    let mut refused = 0usize;
    let mut codes: Vec<String> = Vec::new();
    let mut rng = Lcg(0x5A3_2026_1027);
    // 60 byte-flip mutants (single + burst).
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
    codes.sort();
    codes.dedup();
    eprintln!(
        "aac fuzz survival: {decoded} decoded, {refused} typed refusals; codes: {:?}",
        codes
    );
    // The honest invariants: nothing panics (the test reaching here
    // proves it), every refusal carries a stable aac.* code, and the
    // pristine base decodes.
    assert!(codes.iter().all(|c| c.starts_with("aac.")));
    assert!(
        matches!(classify(&pristine), Outcome::Decoded),
        "the pristine base must decode"
    );
}

/// The walk-level survival: garbage-interleaved and sync-storm inputs
/// through the full harness (contract path included).
#[test]
fn sync_storms_and_garbage_interleaves_never_panic() {
    let pristine = fixture_bytes("fuzz_base_lc.aac");
    let mut storm = vec![0xFF_u8; 8192];
    storm.extend_from_slice(&pristine);
    storm.extend_from_slice(&[0xFF; 512]);
    let _ = fuzz_aac_stream(&storm);
    let mut sprinkled = pristine.clone();
    let mut rng = Lcg(0xC0FF_EE01);
    for _ in 0..32 {
        let at = rng.below(sprinkled.len());
        sprinkled.insert(at, rng.below(256) as u8);
    }
    let _ = fuzz_aac_stream(&sprinkled);
}
