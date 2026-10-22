//! fuzz_loop_stable.rs — the stable-toolchain seed-loop equivalent of the
//! cargo-fuzz campaign (card W1-12, audit row M14 / ED-FIX-5 part (b);
//! D-5 "adversarial & chaos testing").
//!
//! The libFuzzer campaign needs a nightly toolchain (see
//! `fuzz/Cargo.toml` and `fuzz/README.md`; one Lead decision is
//! outstanding). This file runs the SAME harness loop bodies —
//! [`codecs::audio::pcm::fuzz::fuzz_codec_pcm`] and
//! [`codecs::video::mjpeg::fuzz::fuzz_codec_mjpeg`], reused verbatim,
//! never rewritten — on the pinned stable toolchain, over N seeded
//! pseudo-random inputs per target. The contract is the harnesses'
//! own: survival (no panic, no hang, no unbounded allocation); damage
//! surfaces as typed errors or concealed output inside the harness.
//!
//! # Generator (deterministic, std-only)
//!
//! An inline xorshift64* stream (splitmix64-decorrelated seed — no
//! external RNG dependency, C5) drives six input flavors:
//!
//! 0. pure random bytes (length uniform over `0..=cap`),
//! 1. magic/pattern-prefixed random (SOI for MJPEG, byte-periodic
//!    audio-shaped data for PCM),
//! 2. a real fixture seed with 1..=64 seeded bit flips (± truncation),
//! 3. a real fixture seed truncated at a seeded position,
//! 4. a real fixture seed's head with a random tail spliced on,
//! 5. a real fixture seed with one random u32 LE smashed in (the
//!    declared-size corruption class).
//!
//! The real seeds are **externally authored** bytes (the class-A
//! fixtures, written by the in-repo generator but pinned against
//! ffmpeg's ffprobe oracle; frames/chunks cross-contaminate codecs —
//! JPEG frames fed to PCM, PCM chunks fed to MJPEG), so the loop does
//! not only test what this file generates (D-5's echo-chamber rule).
//!
//! # Bounds (MP §12)
//!
//! Per input: size is hard-capped (PCM 8 KiB, MJPEG 16 KiB — splice
//! tails are truncated to the cap) and the harnesses' own budgets
//! apply underneath (PCM: permutation of the input's own bytes;
//! MJPEG: MAX_PIXELS + per-plane ceilings). Per run: the iteration
//! count N is a fixed constant. Total work is therefore bounded by
//! `N × (cap + harness budget)` — no unbounded collection anywhere.
//!
//! # Determinism
//!
//! Fixed seeds (documented below): any survival regression is exactly
//! reproducible. The deliberately NON-deterministic layer of the
//! testing strategy remains the D-11 randomized real-media corpus
//! (`tools/corpus/`); this file is its deterministic in-repo sibling.

mod common;

use std::collections::HashSet;
use std::time::Instant;

use codecs::audio::pcm::fuzz::fuzz_codec_pcm;
use codecs::video::mjpeg::fuzz::fuzz_codec_mjpeg;
use common::{
    extract_avi_mjpeg_frames, extract_avi_pcm_chunks, extract_mp4_mjpeg_frames, fixture_bytes,
};

/// xorshift64* with a splitmix64 seed-decorrelator: deterministic,
/// integer-only (platform-stable), and immune to the all-zero state.
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

/// The prefix flavor of generated inputs (mode 1).
enum Magic {
    /// No container magic: byte-periodic, audio-shaped data (strong
    /// sample-boundary alignment coverage for the PCM surfaces).
    PeriodicAudio,
    /// `FF D8` — a JPEG SOI, so the MJPEG marker walk engages.
    Soi,
}

/// Everything `gen_input` needs; kept as one struct so the
/// determinism test can replay a loop's exact stream.
struct Spec<'a> {
    seeds: &'a [&'a [u8]],
    cap: usize,
    magic: Magic,
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
        // 1: magic/pattern-prefixed random.
        1 => {
            let len = rng.below(spec.cap + 1);
            let mut data = vec![0u8; len];
            match spec.magic {
                Magic::Soi => {
                    if len >= 2 {
                        data[0] = 0xff;
                        data[1] = 0xd8;
                        for byte in &mut data[2..] {
                            *byte = rng.byte();
                        }
                    } else {
                        // Sub-magic lengths are their own damage class
                        // (truncated prefix): random bytes, no magic.
                        for byte in &mut data {
                            *byte = rng.byte();
                        }
                    }
                }
                Magic::PeriodicAudio => {
                    // Sawtooth with seeded base/step (mimics PCM sample
                    // data at any width/alignment) + jitter every 4th
                    // byte so the stream is not trivially uniform.
                    let base = rng.byte();
                    let step = rng.byte();
                    for (i, byte) in data.iter_mut().enumerate() {
                        *byte = if i % 4 == 3 {
                            rng.byte()
                        } else {
                            base.wrapping_add(step.wrapping_mul((i / 4) as u8))
                        };
                    }
                }
            }
            data
        }
        mode => {
            // 2..=5: real-seed mutations. An empty seed table would
            // make below() divide by zero — the loaders assert against
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

/// One loop run: N generated inputs through `harness`, with the
/// accounting that keeps a green run honest (no vacuous pass).
struct Stats {
    inputs: usize,
    bytes: usize,
    deep_entries: usize,
    distinct_lengths: usize,
}

fn run_loop(
    harness: fn(&[u8]),
    deep: fn(&[u8]) -> bool,
    spec: &Spec,
    seed: u64,
    n: usize,
) -> Stats {
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
        if deep(&input) {
            stats.deep_entries += 1;
        }
        lengths.insert(input.len());
        // The pass is the return: the harness contract is survival.
        harness(&input);
        stats.inputs += 1;
    }
    stats.distinct_lengths = lengths.len();
    stats
}

fn assert_honest(stats: &Stats, n: usize, min_deep: usize, min_lengths: usize, label: &str) {
    assert_eq!(stats.inputs, n, "{label}: loop must run every iteration");
    assert!(
        stats.deep_entries >= min_deep,
        "{label}: only {} of {n} inputs reached the deep path — the seed mix went vacuous",
        stats.deep_entries
    );
    assert!(
        stats.distinct_lengths >= min_lengths,
        "{label}: only {} distinct input lengths — the RNG stream degenerated",
        stats.distinct_lengths
    );
}

// ---------------------------------------------------------------------------
// Real seeds (externally authored bytes — see module docs)
// ---------------------------------------------------------------------------

/// The committed class-A AVI parent's bytes.
fn avi_parent() -> Vec<u8> {
    match fixture_bytes("generated/valid/avi_mjpeg_pcm.avi") {
        Ok(bytes) => bytes,
        Err(err) => unreachable!("{err}"),
    }
}

/// The committed class-A MP4 parent's bytes.
fn mp4_parent() -> Vec<u8> {
    match fixture_bytes("generated/valid/mp4_mjpeg_pcm.mp4") {
        Ok(bytes) => bytes,
        Err(err) => unreachable!("{err}"),
    }
}

/// PCM loop seeds: the real audio chunks (s16le stereo 44.1 kHz, 5880
/// bytes each), real JPEG frames from both parents (arbitrary bytes to
/// a PCM decoder), and 4 KiB container prefixes.
fn pcm_seeds() -> Vec<Vec<u8>> {
    let avi = avi_parent();
    let mp4 = mp4_parent();
    let mut seeds: Vec<Vec<u8>> = match extract_avi_pcm_chunks(&avi) {
        Ok(chunks) => chunks,
        Err(err) => unreachable!("{err}"),
    };
    for frames in [
        match extract_avi_mjpeg_frames(&avi) {
            Ok(frames) => frames,
            Err(err) => unreachable!("{err}"),
        },
        match extract_mp4_mjpeg_frames(&mp4) {
            Ok(frames) => frames,
            Err(err) => unreachable!("{err}"),
        },
    ] {
        seeds.extend(frames);
    }
    seeds.push(avi[..4 * 1024].to_vec());
    seeds.push(mp4[..4 * 1024].to_vec());
    assert!(!seeds.is_empty(), "fixture corpus must provide seeds");
    seeds
}

/// MJPEG loop seeds: the real JPEG frames from both parents, the real
/// PCM chunks (arbitrary bytes to a JPEG decoder), and 4 KiB container
/// prefixes.
fn mjpeg_seeds() -> Vec<Vec<u8>> {
    let avi = avi_parent();
    let mp4 = mp4_parent();
    let mut seeds: Vec<Vec<u8>> = match extract_avi_mjpeg_frames(&avi) {
        Ok(frames) => frames,
        Err(err) => unreachable!("{err}"),
    };
    seeds.extend(match extract_mp4_mjpeg_frames(&mp4) {
        Ok(frames) => frames,
        Err(err) => unreachable!("{err}"),
    });
    seeds.extend(match extract_avi_pcm_chunks(&avi) {
        Ok(chunks) => chunks,
        Err(err) => unreachable!("{err}"),
    });
    seeds.push(avi[..4 * 1024].to_vec());
    seeds.push(mp4[..4 * 1024].to_vec());
    assert!(!seeds.is_empty(), "fixture corpus must provide seeds");
    seeds
}

// ---------------------------------------------------------------------------
// The loops
// ---------------------------------------------------------------------------

/// PCM input cap `[PROPOSAL: 8 KiB — a full real chunk (5880 B) plus a
/// spliced tail; PCM work is a permutation of the input's own bytes]`.
const PCM_CAP: usize = 8 * 1024;
/// MJPEG input cap `[PROPOSAL: 16 KiB — 13x the largest real frame;
/// beyond that a JPEG is padding, and the decoder's own MAX_PIXELS
/// bounds memory regardless]`.
const MJPEG_CAP: usize = 16 * 1024;

/// Loop seeds. Distinct per target so a defect found by one target is
/// reproducible from its own constants.
const PCM_SEED: u64 = 0x5eed_0000_0000_000c;
const MJPEG_SEED: u64 = 0x5eed_0000_0000_0004;

/// The PCM harness accepts any bytes (both decode surfaces always
/// run), so every input is a deep entry by construction.
fn pcm_deep(_: &[u8]) -> bool {
    true
}

/// The MJPEG marker walk engages at an SOI prefix.
fn mjpeg_deep(data: &[u8]) -> bool {
    data.len() >= 2 && data[0] == 0xff && data[1] == 0xd8
}

fn pcm_loop(n: usize) {
    let seeds = pcm_seeds();
    let refs: Vec<&[u8]> = seeds.iter().map(|s| s.as_slice()).collect();
    let spec = Spec {
        seeds: &refs,
        cap: PCM_CAP,
        magic: Magic::PeriodicAudio,
    };
    let started = Instant::now();
    let stats = run_loop(fuzz_codec_pcm, pcm_deep, &spec, PCM_SEED, n);
    eprintln!(
        "[fuzz_loop_stable] pcm: {} inputs, {} bytes, {} deep, {} distinct lengths, {:?}",
        stats.inputs,
        stats.bytes,
        stats.deep_entries,
        stats.distinct_lengths,
        started.elapsed()
    );
    assert_honest(&stats, n, n, 48, "pcm");
}

fn mjpeg_loop(n: usize) {
    let seeds = mjpeg_seeds();
    let refs: Vec<&[u8]> = seeds.iter().map(|s| s.as_slice()).collect();
    let spec = Spec {
        seeds: &refs,
        cap: MJPEG_CAP,
        magic: Magic::Soi,
    };
    let started = Instant::now();
    let stats = run_loop(fuzz_codec_mjpeg, mjpeg_deep, &spec, MJPEG_SEED, n);
    eprintln!(
        "[fuzz_loop_stable] mjpeg: {} inputs, {} bytes, {} deep, {} distinct lengths, {:?}",
        stats.inputs,
        stats.bytes,
        stats.deep_entries,
        stats.distinct_lengths,
        started.elapsed()
    );
    assert_honest(&stats, n, n / 4, 48, "mjpeg");
}

#[test]
fn pcm_seed_loop_survives() {
    pcm_loop(2_000);
}

#[test]
#[ignore = "long fuzz run — cargo test -- --ignored --nocapture"]
fn pcm_seed_loop_long() {
    pcm_loop(100_000);
}

#[test]
fn mjpeg_seed_loop_survives() {
    mjpeg_loop(1_000);
}

#[test]
#[ignore = "long fuzz run — cargo test -- --ignored --nocapture"]
fn mjpeg_seed_loop_long() {
    mjpeg_loop(20_000);
}

#[test]
fn sub_magic_lengths_are_survivable_inputs() {
    // Regression pin for the generator's own off-by-one (the long run
    // caught `data[2..]` panicking at len == 1): with cap 1 every
    // mode-1 draw exercises the sub-magic branch deterministically,
    // and the harness must survive every such input.
    let seeds: Vec<Vec<u8>> = vec![vec![0x42; 8]];
    let refs: Vec<&[u8]> = seeds.iter().map(|s| s.as_slice()).collect();
    let spec = Spec {
        seeds: &refs,
        cap: 1,
        magic: Magic::Soi,
    };
    let mut rng = Rng::new(0x00ff_00ff_00ff_00ff);
    for _ in 0..64 {
        let input = gen_input(&mut rng, &spec);
        assert!(input.len() <= 1, "cap 1 must cap every flavor");
        fuzz_codec_mjpeg(&input);
    }
}

#[test]
fn seeded_generation_is_deterministic_and_decorrelated() {
    // Same seed → byte-identical input stream (replayability of any
    // survival regression), for both magic flavors.
    for (magic, cap) in [(Magic::PeriodicAudio, PCM_CAP), (Magic::Soi, MJPEG_CAP)] {
        let seeds: Vec<Vec<u8>> = vec![vec![0xa5; 64], vec![0x5a; 7]];
        let refs: Vec<&[u8]> = seeds.iter().map(|s| s.as_slice()).collect();
        let spec = Spec {
            seeds: &refs,
            cap,
            magic,
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
}
