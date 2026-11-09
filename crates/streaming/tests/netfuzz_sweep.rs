//! netfuzz_sweep.rs — the stable-toolchain sweep for the issue #14
//! (M2) network-facing fuzz targets, `fuzz_hls_playlist` and
//! `fuzz_http_headers` (the fuzz/README convention's runner while the
//! nightly campaign wiring is the outstanding LEAD-DECISION — the
//! same posture as `crates/codecs/tests/fuzz_loop_stable.rs` and
//! `crates/demux/tests/fuzz_loop_stable.rs`).
//!
//! It runs the SAME harness bodies the libFuzzer shims run — the
//! `#[path]`-included `netfuzz/fuzz.rs` module, plus (for the HLS
//! target) the in-crate `playlists::fuzz::fuzz_playlist_m3u` base
//! pass, reused verbatim, never rewritten — on the pinned stable
//! toolchain, over seeded pseudo-random mutations of REAL fixture
//! bytes. The contract is the harness's own: survival (no panic, no
//! hang, no unbounded allocation); damage surfaces as typed errors or
//! damage warnings inside the harness (D-3/D-9).
//!
//! # Seeds (real, externally authored — D-5)
//!
//! The HLS seeds are the committed real HLS playlists under
//! `crates/playlists/tests/fixtures/hls/` (the wave's issue names
//! that directory): the three master playlists (`multi`, `single`,
//! and the already-malformed `malformed`) and the three media
//! playlists (`ts_vod`, `fmp4`, `single_file`). Read cross-crate by
//! path (read-only) because the fixtures belong to the playlists
//! crate; the same files are the committed `fuzz/corpus/` seeds.
//!
//! The HTTP-header seeds are hand-authored response heads (CC0,
//! ours — the "generated malformed files" arm of doc 04 §8.5) plus
//! one cross-format seed (a real media playlist fed to the head
//! grammar — format confusion is fair fuzz material).
//!
//! # Generator (deterministic, std-only)
//!
//! An inline xorshift64\* stream (splitmix64-decorrelated seed — no
//! external RNG dependency, C5), a near-verbatim twin of the
//! demux/codecs sweep generators, drives six input flavors:
//!
//! 0. pure random bytes (length uniform over `0..=cap`),
//! 1. a real seed with 1..=64 seeded bit flips (± truncation),
//! 2. a real seed truncated at a seeded position,
//! 3. tag soup: a real seed with 1..=8 seeded hostile lines
//!    spliced in (HLS tags / HTTP head lines, per target),
//! 4. a real seed's head with a random tail spliced on,
//! 5. (headers only, every 64th iteration) an over-limit random
//!    input past the transport's 64 KiB head budget — the typed
//!    `ProtocolViolation` branch, exercised deterministically.
//!
//! # Bounds (MP §12)
//!
//! Per input: size hard-capped (`HLS_CAP` / `HEAD_CAP`; flavor 5
//! deliberately crosses the head budget by +257 B, the typed
//! refusal's own bound). Per run: the default tests run a fixed
//! iteration count; the `#[ignore]`d long variants run a fixed
//! 60-second budget each and print their ACTUAL iteration count and
//! duration (`--nocapture`).
//!
//! # Determinism
//!
//! Fixed seeds (documented per target): any survival regression is
//! exactly reproducible. The iteration count inside a time budget is
//! machine-dependent by design — the recorded runs live in
//! `docs/formats/streaming-hls.md`'s network-scenarios section.

use std::path::PathBuf;
use std::time::{Duration, Instant};

#[path = "netfuzz/fuzz.rs"]
mod netfuzz;

use netfuzz::{fuzz_hls_playlist, fuzz_http_headers};
use testkit::Must;

// ---------------------------------------------------------------------------
// The generator (twin of the demux/codecs sweeps — see module docs)
// ---------------------------------------------------------------------------

/// xorshift64\* with a splitmix64 seed-decorrelator: deterministic,
/// integer-only (platform-stable), immune to the all-zero state.
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
        x = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        self.0 = x;
        x
    }

    /// A uniform draw below `bound` (0 when `bound` is 0 — the empty
    /// range, never a division by zero).
    fn below(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            0
        } else {
            self.next_u64() % bound
        }
    }

    /// One random byte.
    fn byte(&mut self) -> u8 {
        (self.next_u64() >> 32) as u8
    }
}

// ---------------------------------------------------------------------------
// Seeds and soup alphabets
// ---------------------------------------------------------------------------

/// The real HLS playlist fixtures (issue #14's named seed directory),
/// read cross-crate read-only — see the module docs.
fn hls_seeds() -> Vec<Vec<u8>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../playlists/tests/fixtures/hls");
    [
        "master/multi.m3u8",
        "master/single.m3u8",
        "master/malformed.m3u8",
        "ts_vod/pl.m3u8",
        "fmp4/pl.m3u8",
        "single_file/pl.m3u8",
    ]
    .iter()
    .map(|rel| {
        let path = root.join(rel);
        std::fs::read(&path)
            .must()
    })
    .collect()
}

/// Hand-authored response-head seeds (CC0, ours) — the shapes the
/// transport actually answers with, plus the cross-format media
/// playlist (its text is a hostile status-line/headers soup).
fn header_seeds() -> Vec<Vec<u8>> {
    let authored: Vec<Vec<u8>> = [
        &b"HTTP/1.1 200 OK\r\nContent-Length: 1000\r\nContent-Type: video/mp2t\r\n\r\n"[..],
        b"HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 250-999/1000\r\nContent-Length: 750\r\n\r\n",
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: X-Sum\r\n\r\n",
        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n",
        b"HTTP/1.1 200\r\nX-Empty:\r\nX-Two: a\r\n b\r\n\r\n",
    ]
    .into_iter()
    .map(|s| s.to_vec())
    .collect();
    // The cross-format seed: a real media playlist's bytes as a head.
    let mut cross = hls_seeds();
    let cross_seed = cross.pop().must();
    let mut seeds = authored;
    seeds.push(cross_seed);
    seeds
}

/// Hostile HLS tag/URI lines for the tag-soup flavor (every hostile
/// number shape the grammar has a typed answer for).
const HLS_SOUP: [&[u8]; 17] = [
    b"#EXTM3U",
    b"#EXT-X-VERSION:999999999999999999",
    b"#EXT-X-TARGETDURATION:-1",
    b"#EXTINF:-1.0e999,",
    b"#EXTINF:1e4096,",
    b"#EXT-X-BYTERANGE:999999999999999999999999@0",
    b"#EXT-X-TIMESTAMP-MAP:MPEGTS=9999999999,LOCAL=2026-13-45T99:99:99.999999999Z",
    b"#EXT-X-TIMESTAMP-MAP:MPEGTS=4500,LOCAL=1970-01-01T00:00:00Z",
    b"#EXT-X-KEY:METHOD=AES-128,URI=\"https://k.example/\",IV=0x",
    b"#EXT-X-DISCONTINUITY",
    b"#EXT-X-MEDIA-SEQUENCE:18446744073709551615",
    b"#EXT-X-ENDLIST",
    b"#EXT-X-MAP:URI=\"init.mp4\"",
    b"#EXT-X-STREAM-INF:BANDWIDTH=99999999999999999999",
    b"#EXT-X-PROGRAM-DATE-TIME:not-a-date",
    b"seg\xFF\xFE.ts",
    b"#GARBAGE-TAG:?\xC3\x28",
];

/// Hostile HTTP head lines for the tag-soup flavor.
const HEAD_SOUP: [&[u8]; 12] = [
    b"HTTP/1.1 999 ",
    b"HTTP/9.9 fine",
    b"HTTP/1.1 200 OK",
    b"Content-Length: -1",
    b"Content-Length: 99999999999999999999999999",
    b"Content-Range: bytes -5-abc/xyz",
    b"Location: https://cross.example/next",
    b"Set-Cookie: a=b; c=d",
    b"X-Binary:\x00\x01\x02\xFF",
    b"no-colon-line",
    b": empty-name",
    b"",
];

// ---------------------------------------------------------------------------
// The mutation engine
// ---------------------------------------------------------------------------

/// The per-target shape of a sweep.
struct Target {
    /// The harness passes (verbatim shim bodies; non-capturing, so
    /// a plain fn pointer).
    passes: fn(&[u8]),
    /// Real seed inputs.
    seeds: Vec<Vec<u8>>,
    /// The tag-soup line alphabet.
    soup: &'static [&'static [u8]],
    /// The random-input size cap.
    cap: usize,
    /// Whether flavor 5 (over-limit head) applies.
    over_limit_flavor: bool,
}

/// One mutated input (flavors per the module docs; deterministic in
/// `rng` and `iteration`).
fn generate(rng: &mut Rng, target: &Target, iteration: u64) -> Vec<u8> {
    let seeds = &target.seeds;
    if target.over_limit_flavor && iteration % 64 == 63 {
        // Flavor 5: deliberately past the transport's 64 KiB head
        // budget — the typed over-limit branch.
        let len = 64 * 1024 + 257;
        return (0..len).map(|_| rng.byte()).collect();
    }
    match iteration % 5 {
        // 0: pure random bytes.
        0 => {
            let len = rng.below(target.cap as u64 + 1) as usize;
            (0..len).map(|_| rng.byte()).collect()
        }
        // 1: seed + seeded bit flips, ± truncation.
        1 => {
            let seed = rng_pick(rng, seeds);
            let mut input = seed.to_vec();
            let flips = 1 + rng.below(64) as usize;
            for _ in 0..flips {
                if input.is_empty() {
                    break;
                }
                let at = rng.below(input.len() as u64) as usize;
                input[at] ^= 1 << rng.below(8);
            }
            if rng.below(2) == 0 && !input.is_empty() {
                input.truncate(rng.below(input.len() as u64 + 1) as usize);
            }
            input
        }
        // 2: seed truncated at a seeded position.
        2 => {
            let seed = rng_pick(rng, seeds);
            let cut = rng.below(seed.len() as u64 + 1) as usize;
            seed[..cut].to_vec()
        }
        // 3: tag soup — seed with hostile lines spliced in.
        3 => {
            let seed = rng_pick(rng, seeds);
            let mut lines: Vec<Vec<u8>> = seed
                .split(|&b| b == b'\n')
                .map(|line| line.to_vec())
                .collect();
            let splices = 1 + rng.below(8) as usize;
            for _ in 0..splices {
                let line = target.soup[rng.below(target.soup.len() as u64) as usize];
                let at = rng.below(lines.len() as u64 + 1) as usize;
                lines.insert(at, line.to_vec());
            }
            let mut souped = Vec::new();
            for (i, line) in lines.iter().enumerate() {
                if i > 0 {
                    souped.push(b'\n');
                }
                souped.extend_from_slice(line);
            }
            souped
        }
        // 4: seed head + random tail.
        _ => {
            let seed = rng_pick(rng, seeds);
            let head = rng.below(seed.len() as u64 + 1) as usize;
            let tail = rng.below(target.cap as u64 / 2 + 1) as usize;
            let mut input = seed[..head].to_vec();
            input.extend((0..tail).map(|_| rng.byte()));
            input
        }
    }
}

/// Pick a seed (uniform; the empty-seed case answers the empty input
/// — a fair fuzz input too).
fn rng_pick<'a>(rng: &mut Rng, seeds: &'a [Vec<u8>]) -> &'a [u8] {
    if seeds.is_empty() {
        &[]
    } else {
        &seeds[rng.below(seeds.len() as u64) as usize]
    }
}

/// How long a sweep runs.
enum Limit {
    /// A fixed iteration count (the default tests).
    Iters(u64),
    /// A fixed wall-clock budget (the `#[ignore]`d long variants).
    Time(Duration),
}

/// Run `target`'s harness passes over mutated seeds until `limit` is
/// spent; returns `(iterations, actual duration)` for the summary.
fn sweep(target: &Target, limit: Limit, seed: u64) -> (u64, Duration) {
    let started = Instant::now();
    let mut rng = Rng::new(seed);
    let mut iteration = 0u64;
    loop {
        match limit {
            Limit::Iters(max) if iteration >= max => break,
            Limit::Time(budget) if started.elapsed() >= budget => break,
            _ => {}
        }
        let input = generate(&mut rng, target, iteration);
        (target.passes)(&input);
        iteration += 1;
    }
    (iteration, started.elapsed())
}

// ---------------------------------------------------------------------------
// The sweeps
// ---------------------------------------------------------------------------

/// The HLS target's cap (the committed fixtures are 184–572 B; the
/// cap leaves two orders of headroom for splices and soup).
const HLS_CAP: usize = 64 * 1024;
/// The headers target's cap.
const HEAD_CAP: usize = 64 * 1024;
/// The default (fast-gate) iteration count per target.
const SWEEP_ITERS: u64 = 3000;
/// The long-run budget per target (the recorded ACTUAL durations live
/// in docs/formats/streaming-hls.md).
const SWEEP_BUDGET: Duration = Duration::from_secs(60);
/// Fixed seeds (documented — any regression is reproducible).
const HLS_SEED: u64 = 0x5eed_0000_0000_0e14;
const HEAD_SEED: u64 = 0x5eed_0000_0000_0e1f;

fn hls_target() -> Target {
    Target {
        // The shim body, verbatim: the in-crate generic parse pass,
        // then the HLS-specific interpretation passes.
        passes: |data| {
            playlists::fuzz::fuzz_playlist_m3u(data);
            fuzz_hls_playlist(data);
        },
        seeds: hls_seeds(),
        soup: &HLS_SOUP,
        cap: HLS_CAP,
        over_limit_flavor: false,
    }
}

fn head_target() -> Target {
    Target {
        // The shim body's full pass: the response-head grammar plus
        // the request-side Range-value probes.
        passes: |data| fuzz_http_headers(data),
        seeds: header_seeds(),
        soup: &HEAD_SOUP,
        cap: HEAD_CAP,
        over_limit_flavor: true,
    }
}

#[test]
fn netfuzz_sweep_hls_playlist_survives() {
    let target = hls_target();
    let (iterations, elapsed) = sweep(&target, Limit::Iters(SWEEP_ITERS), HLS_SEED);
    assert_eq!(iterations, SWEEP_ITERS);
    println!("netfuzz-sweep: fuzz_hls_playlist {SWEEP_ITERS} iterations in {elapsed:?}");
}

#[test]
fn netfuzz_sweep_http_headers_survives() {
    let target = head_target();
    let (iterations, elapsed) = sweep(&target, Limit::Iters(SWEEP_ITERS), HEAD_SEED);
    assert_eq!(iterations, SWEEP_ITERS);
    println!("netfuzz-sweep: fuzz_http_headers {SWEEP_ITERS} iterations in {elapsed:?}");
}

#[test]
#[ignore = "long fuzz run — cargo test -- --ignored --nocapture"]
fn netfuzz_sweep_hls_playlist_60s() {
    let target = hls_target();
    let (iterations, elapsed) = sweep(&target, Limit::Time(SWEEP_BUDGET), HLS_SEED);
    assert!(elapsed >= SWEEP_BUDGET, "the budget must be honored");
    println!(
        "netfuzz-sweep: fuzz_hls_playlist {iterations} iterations in {elapsed:?} (budget {SWEEP_BUDGET:?})"
    );
}

#[test]
#[ignore = "long fuzz run — cargo test -- --ignored --nocapture"]
fn netfuzz_sweep_http_headers_60s() {
    let target = head_target();
    let (iterations, elapsed) = sweep(&target, Limit::Time(SWEEP_BUDGET), HEAD_SEED);
    assert!(elapsed >= SWEEP_BUDGET, "the budget must be honored");
    println!(
        "netfuzz-sweep: fuzz_http_headers {iterations} iterations in {elapsed:?} (budget {SWEEP_BUDGET:?})"
    );
}
