//! fuzz_loop_stable.rs — the stable-toolchain seed-loop equivalent of
//! the `fuzz_demux_avi` cargo-fuzz target (card W1-12, audit row M14 /
//! ED-FIX-5 part (b); D-5 "adversarial & chaos testing").
//!
//! The libFuzzer campaign needs a nightly toolchain (see
//! `fuzz/Cargo.toml` and `fuzz/README.md`; one Lead decision is
//! outstanding). This file runs the SAME harness loop body —
//! [`demux::avi::fuzz::fuzz_demux_avi`], reused verbatim, never
//! rewritten — on the pinned stable toolchain, over N seeded
//! pseudo-random container inputs. The contract is the harness's own:
//! survival (no panic, no hang, no unbounded allocation); damage
//! surfaces as typed errors, skipped chunks or clamped reads inside
//! the harness (D-3/D-9).
//!
//! # Generator (deterministic, std-only)
//!
//! An inline xorshift64* stream (splitmix64-decorrelated seed — no
//! external RNG dependency, C5) drives six input flavors:
//!
//! 0. pure random bytes (length uniform over `0..=cap`),
//! 1. `RIFF`+random-size+`AVI `-prefixed random (valid magic, garbage
//!    body — the lying-container class),
//! 2. a real fixture seed with 1..=64 seeded bit flips (± truncation),
//! 3. a real fixture seed truncated at a seeded position (the cut
//!    classes of the class-B corpus, at random depths),
//! 4. a real fixture seed's head with a random tail spliced on
//!    (truncation + tail garbage in one input),
//! 5. a real fixture seed with one random u32 LE smashed in (the
//!    lying-declared-size class that D-3 names explicitly).
//!
//! The real seeds are externally authored bytes — the class-A AVI
//! parent (ffmpeg-oracle-pinned, `tests/fixtures/CORPUS.md`) at three
//! depths: the full file (hdrl + full `movi` + `idx1`), a 64 KiB
//! prefix (deep walk, indexless — the `noidx` fallback), and a 2 KiB
//! prefix (header region only, the truncation class). This file
//! therefore does not only test what it generates (D-5). The
//! generator is intentionally a near-verbatim twin of the codecs
//! crate's `tests/fuzz_loop_stable.rs` generator (integration-test
//! binaries cannot share code without a new dependency; each copy
//! stays one screen to audit).
//!
//! # Bounds (MP §12)
//!
//! Per input: size hard-capped (512 KiB — the full real parent plus a
//! spliced tail) and the harness's own budgets apply underneath
//! (`FUZZ_EVENT_BUDGET` events, `MAX_CHUNK_PAYLOAD`, `MAX_IDX1_ENTRIES`,
//! monotone walk). Per run: the iteration count N is a fixed constant.
//! Total work is bounded by `N × (cap + harness budget)`.
//!
//! # Determinism
//!
//! Fixed seed (documented below): any survival regression is exactly
//! reproducible. The deliberately NON-deterministic layer of the
//! testing strategy remains the D-11 randomized real-media corpus
//! (`tools/corpus/`); this file is its deterministic in-repo sibling.

use std::collections::HashSet;
use std::time::Instant;

use demux::avi::fuzz::fuzz_demux_avi;
use testkit::Must;

/// xorshift64* with a splitmix64 seed-decorrelator: deterministic,
/// integer-only (platform-stable), and immune to the all-zero state.
/// (Twin of the codecs crate's generator — see the module docs.)
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        // One splitmix64 step (public-domain reference constants).
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        Rng(if z == 0 { 0x9E37_79B9_7F4A_7C15 } else { z })
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform `0..n` (n ≥ 1).
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    fn byte(&mut self) -> u8 {
        (self.next_u64() >> 32) as u8
    }

    fn flip_bit(&mut self, data: &mut [u8]) {
        if data.is_empty() {
            return;
        }
        let index = self.below(data.len());
        let bit = self.below(8) as u8;
        data[index] ^= 1 << bit;
    }
}

/// Everything `gen_input` needs; kept as one struct so the
/// determinism test can replay a loop's exact stream.
struct Spec<'a> {
    seeds: &'a [&'a [u8]],
    cap: usize,
}

/// Generate one input (consumes the RNG; pure otherwise). See the
/// module docs for the six flavors.
fn gen_input(rng: &mut Rng, spec: &Spec) -> Vec<u8> {
    let mut input = match rng.below(6) {
        // 0: pure random.
        0 => {
            let len = rng.below(spec.cap + 1);
            let mut data = vec![0u8; len];
            for chunk in data.chunks_mut(8) {
                let bytes = rng.next_u64().to_le_bytes();
                chunk.copy_from_slice(&bytes[..chunk.len()]);
            }
            data
        }
        // 1: RIFF/AVI-prefixed random (valid magic, garbage body).
        1 => {
            let len = rng.below(spec.cap + 1);
            let mut data = vec![0u8; len];
            if len >= 12 {
                data[..4].copy_from_slice(b"RIFF");
                data[4..8].copy_from_slice(&(rng.next_u64() as u32).to_le_bytes());
                data[8..12].copy_from_slice(b"AVI ");
                for byte in &mut data[12..] {
                    *byte = rng.byte();
                }
            } else {
                for byte in &mut data {
                    *byte = rng.byte();
                }
            }
            data
        }
        mode => {
            // 2..=5: real-seed mutations. An empty seed table would
            // make below() divide by zero — the loader asserts against
            // that before any loop runs.
            let seed = spec.seeds[rng.below(spec.seeds.len())];
            match mode {
                // 2: flips (± truncation).
                2 => {
                    let mut data = seed.to_vec();
                    let flips = 1 + rng.below(64);
                    for _ in 0..flips {
                        rng.flip_bit(&mut data);
                    }
                    if rng.below(2) == 1 {
                        data.truncate(rng.below(data.len() + 1));
                    }
                    data
                }
                // 3: plain truncation at a seeded position.
                3 => seed[..rng.below(seed.len() + 1)].to_vec(),
                // 4: seed head + random tail.
                4 => {
                    let head = rng.below(seed.len() + 1);
                    let tail = rng.below(spec.cap.saturating_sub(head) + 1);
                    let mut data = Vec::with_capacity(head + tail);
                    data.extend_from_slice(&seed[..head]);
                    data.resize(head + tail, 0);
                    for byte in &mut data[head..] {
                        *byte = rng.byte();
                    }
                    data
                }
                // 5: one random u32 LE smashed at an aligned offset
                // (the lying-declared-size corruption class, D-3/D-9).
                _ => {
                    let mut data = seed.to_vec();
                    if data.len() >= 4 {
                        let at = rng.below(data.len() / 4) * 4;
                        let word = rng.next_u64() as u32;
                        data[at..at + 4].copy_from_slice(&word.to_le_bytes());
                    }
                    data
                }
            }
        }
    };
    input.truncate(spec.cap);
    input
}

/// One loop run: N generated inputs through the AVI harness, with the
/// accounting that keeps a green run honest (no vacuous pass).
struct Stats {
    inputs: usize,
    bytes: usize,
    deep_entries: usize,
    distinct_lengths: usize,
}

fn run_loop(spec: &Spec, seed: u64, n: usize) -> Stats {
    let mut rng = Rng::new(seed);
    let mut lengths: HashSet<usize> = HashSet::new();
    let mut stats = Stats {
        inputs: 0,
        bytes: 0,
        deep_entries: 0,
        distinct_lengths: 0,
    };
    for _ in 0..n {
        let input = gen_input(&mut rng, spec);
        stats.bytes += input.len();
        if avi_deep(&input) {
            stats.deep_entries += 1;
        }
        lengths.insert(input.len());
        // The pass is the return: the harness contract is survival.
        fuzz_demux_avi(&input);
        stats.inputs += 1;
    }
    stats.distinct_lengths = lengths.len();
    stats
}

/// Deep-entry proxy: the container magic a `start()` parse engages on
/// (`RIFF` + size + `AVI `). A landed `start()` is not guaranteed past
/// this prefix — the floor only guards that the seed mix stays
/// structurally recognizable, i.e. the loop is not surviving by
/// early-rejecting everything.
fn avi_deep(data: &[u8]) -> bool {
    data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"AVI "
}

// ---------------------------------------------------------------------------
// Real seeds (externally authored bytes — see module docs)
// ---------------------------------------------------------------------------

/// The class-A AVI parent at three depths: full file (hdrl + full
/// `movi` + `idx1`), 64 KiB prefix (indexless deep walk), 2 KiB
/// prefix (header region).
fn avi_seeds() -> Vec<Vec<u8>> {
    let parent = testkit::fixture_bytes("generated/valid/avi_mjpeg_pcm.avi").must();
    let seeds = vec![
        parent.clone(),
        parent[..64 * 1024].to_vec(),
        parent[..2 * 1024].to_vec(),
    ];
    assert!(!seeds.is_empty(), "fixture corpus must provide seeds");
    seeds
}

/// AVI input cap `[PROPOSAL: 512 KiB — the full real parent (425,516
/// bytes) plus a spliced tail; the walk itself is monotone and
/// event-budgeted underneath]`.
const AVI_CAP: usize = 512 * 1024;

/// Loop seed. Distinct from the codecs crate's loop seeds so a defect
/// found by one target is reproducible from its own constants.
const AVI_SEED: u64 = 0x5eed_0000_0000_0002;

fn avi_loop(n: usize) {
    let seeds = avi_seeds();
    let refs: Vec<&[u8]> = seeds.iter().map(|s| s.as_slice()).collect();
    let spec = Spec {
        seeds: &refs,
        cap: AVI_CAP,
    };
    let started = Instant::now();
    let stats = run_loop(&spec, AVI_SEED, n);
    eprintln!(
        "[fuzz_loop_stable] avi: {} inputs, {} bytes, {} deep, {} distinct lengths, {:?}",
        stats.inputs,
        stats.bytes,
        stats.deep_entries,
        stats.distinct_lengths,
        started.elapsed()
    );
    assert_eq!(stats.inputs, n, "avi: loop must run every iteration");
    assert!(
        stats.deep_entries >= n / 3,
        "avi: only {} of {n} inputs kept container magic — the seed mix went vacuous",
        stats.deep_entries
    );
    assert!(
        stats.distinct_lengths >= 32,
        "avi: only {} distinct input lengths — the RNG stream degenerated",
        stats.distinct_lengths
    );
}

#[test]
fn avi_seed_loop_survives() {
    avi_loop(1_000);
}

#[test]
#[ignore = "long fuzz run — cargo test -- --ignored --nocapture"]
fn avi_seed_loop_long() {
    avi_loop(20_000);
}

#[test]
fn sub_magic_lengths_are_survivable_inputs() {
    // Boundary pin (twin of the codecs crate's regression for the
    // generator off-by-one its long run caught): with cap 11 every
    // mode-1 draw exercises the sub-magic branch deterministically,
    // and the harness must survive every such input.
    let seeds: Vec<Vec<u8>> = vec![vec![0x42; 8]];
    let refs: Vec<&[u8]> = seeds.iter().map(|s| s.as_slice()).collect();
    let spec = Spec {
        seeds: &refs,
        cap: 11,
    };
    let mut rng = Rng::new(0x00ff_00ff_00ff_00ff);
    for _ in 0..64 {
        let input = gen_input(&mut rng, &spec);
        assert!(input.len() <= 11, "cap 11 must cap every flavor");
        fuzz_demux_avi(&input);
    }
}

#[test]
fn seeded_generation_is_deterministic_and_decorrelated() {
    // Same seed → byte-identical input stream (replayability of any
    // survival regression).
    let seeds: Vec<Vec<u8>> = vec![vec![0xa5; 64], vec![0x5a; 7]];
    let refs: Vec<&[u8]> = seeds.iter().map(|s| s.as_slice()).collect();
    let spec = Spec {
        seeds: &refs,
        cap: 4 * 1024,
    };
    let run = |seed: u64| -> Vec<Vec<u8>> {
        let mut rng = Rng::new(seed);
        (0..48).map(|_| gen_input(&mut rng, &spec)).collect()
    };
    let first = run(0x1234_5678_9abc_def0);
    assert_eq!(first, run(0x1234_5678_9abc_def0), "replay must match");
    // A nearby seed must NOT produce the same stream (splitmix64
    // decorrelation — guards against silently-collapsed seeding).
    assert_ne!(first, run(0x1234_5678_9abc_def1));
    // The all-zero seed state must not degenerate (xorshift64*'s
    // only fixed point).
    let zero = run(0);
    assert!(
        zero.iter().map(|v| v.len()).collect::<HashSet<_>>().len() > 1,
        "zero seed must produce a varied stream"
    );
}
