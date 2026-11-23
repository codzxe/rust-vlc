# AUDIT: crates/audio-output/tests/common/mod.rs
| Field | Value |
|---|---|
| Auditor | W08-A09 |
| Date | 2026-11-23 |
| Lines / functions | 262 / 10 |
| File verdict | REAL (independently re-verified: my python reimplementation of the spec'd sine reproduces the committed corpus digest byte-for-byte) |
| Reachable from shipped entry point? | YES as test-support: `cargo test -p audio-output` compiles it (baseline ran all 5 consumer suites); the crate itself ships — apps/cli (lumen-cli) depends on audio-output directly (apps/cli/Cargo.toml:49), desktop depends on lumen-cli (apps/desktop/Cargo.toml:40) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | python3 (SIN_QUARTER check 65/65, corpus PCM sha256, SHA-256 K-table 64/64), rg (expectation TOMLs, fixture writer, dependents), sed quote verification |

## 1. One-paragraph verdict
This is a genuinely verified test-support module, and I verified it the hard
way: I reimplemented the documented integer NCO math (SIN_QUARTER table,
phase-accumulator oscillator, tone plan) independently in python and the
SHA-256 of the regenerated 352 800-byte stream is
`1d9f5019c9b5181f0910528dce360f46e74f9c4df8d9bc9a20c2c68fd218b95b` — exactly
the `audio_stream_sha256` committed in both expectation TOMLs
(tests/fixtures/expectations/valid/avi_mjpeg_pcm.avi.toml:108 and
mp4_mjpeg_pcm.mp4.toml:108). The doc's central claim ("the reconstruction is
pinned against the expectation TOML's recorded audio_stream_sha256") is true,
the pinned math is real public math (all 65 SIN_QUARTER entries are exactly
round(sin(i*pi/128)*32767)), and the SHA-256 constants are FIPS 180-4 exact
(64/64). The only defects are hygiene-grade.

## 2. What actually works (verified)
- python3 reimplementation of `corpus_pcm()` -> sha256
  `1d9f5019c9b5181f0910528dce360f46e74f9c4df8d9bc9a20c2c68fd218b95b` == the
  digest recorded in both expectation TOMLs (lines 108). MATCH.
- python3 SIN_QUARTER check: `mismatches: NONE (65/65 exact)` against
  round(sin(i*pi/128)*32767).
- python3 SHA-256 K-table check: `NONE (64/64)` against the FIPS 180-4
  cube-root-of-primes constants; IVs at common/mod.rs:77-80 are the standard
  IVs.
- Baseline executed every consumer suite green: sine_sink_hash 3/3 (incl.
  `specd_sine_is_the_committed_corpus_tone ... ok`), callback_budget 4/4,
  clock_truth 1/1, pcm_pipeline 4/4, zero_alloc_null_sink 1/1
  (cargo-test-workspace.txt:145-240, exit_code=0).
- The fixture writer carries the same constants
  (tools/xtask/src/cmd/fixtures.rs:525-632: same SineOsc, SIN_QUARTER, tone
  plan 440/660 Hz, 12000/9000 amp), so the writer/consumer pin is real.
- Fixtures referenced exist: tests/fixtures/generated/valid/avi_mjpeg_pcm.avi
  present (ls).

## 3. Findings
F-001 [S4]
Where: crates/audio-output/tests/common/mod.rs:255-262
Code:
```
pub fn s16_stream_to_f32_bytes(s16le: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s16le.len() * 2);
    for pair in s16le.as_chunks::<2>().0 {
        let sample = i16::from_le_bytes(*pair);
        out.extend_from_slice(&(f32::from(sample) / 32_768.0).to_le_bytes());
    }
    out
}
```
What is wrong: `as_chunks::<2>().0` silently discards a trailing odd byte
(the `.1` remainder is dropped without a check). For the corpus input
(352 800 bytes, even) this never triggers, but a truncated/corrupt fixture
would be converted with its tail silently ignored and the digest test would
then diagnose a phantom mismatch. What correct looks like:
`debug_assert!(s16le.as_chunks::<2>().1.is_empty())` or a typed error.
Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/audio-output/tests/common/mod.rs:1-9
Code:
```
#![allow(dead_code)]
//! Shared support for the 2.C3 acceptance suites (std-only; the same
```
What is wrong: whole-module `allow(dead_code)` (one of the tree's 121 lint
allows) means an orphaned helper here stops warning; and this file is the
third in-tree copy of the std-only SHA-256 + NCO family (codecs
tests/common, demux tests/common, audio-output tests/common) — the doc
itself says "same implementation family as the codecs test common". The
duplication is a documented std-only pinning choice, but three hand-copied
SHA-256s are three chances to drift; today they are pinned by the same
committed digest, so drift would be caught. Confidence: HIGH —
Verification: read + rg (`sha256_hex` defined in 3 test commons).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixtures_root | 22-24 | REAL | — | CARGO_MANIFEST_DIR/../../tests/fixtures — verified exists |
| fixture_path | 28-30 | REAL | — | trivial join |
| fixture_bytes | 36-39 | REAL | — | propagates fs errors with path context |
| toml_string | 48-59 | REAL | — | first-match `key = "value"` reader; documented as targeted, machine-generated inputs only |
| sha256_hex | 76-144 | REAL | — | FIPS 180-4; K table + IV verified by python; padding/bit-len correct |
| sine_full_table | 177-193 | REAL | — | quarter-wave expansion; wrapping point duplicated (no OOB) |
| SineOsc::new | 203-210 | REAL | — | round-half-up via u64 intermediate; saturates via try_from |
| SineOsc::sample | 213-220 | REAL | — | max product 32767*65535 < 2^31, no overflow; arithmetic shift matches writer |
| corpus_pcm | 236-248 | REAL | — | digest independently reproduced by my python — MATCH |
| s16_stream_to_f32_bytes | 255-262 | REAL | S4 | F-001 odd-tail drop |

## 5. Test honesty
This module IS the honesty machinery for the audio-output suites. The pinning
test `specd_sine_is_the_committed_corpus_tone` (sine_sink_hash.rs) compares
sha256_hex(corpus_pcm()) to the TOML digest — I reproduced both sides
independently, so the loop is not self-certifying: the digest is committed
data, the math is public math, and my python third implementation agrees.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "pinned by the corpus's own recorded digest, not by faith" | common/mod.rs:8-9 | REAL | my python sha256 == TOML:108 digest |
| "transcribed from the fixture writer's documented public math" | common/mod.rs:11-13 | REAL | fixtures.rs:525-632 identical constants/formulas |
| "if the writer's tone plan ever changes, this suite breaks first" | common/mod.rs:15-17 | REAL | consumer digest vs committed TOML — change on either side fails `specd_sine_is_the_committed_corpus_tone` |
| "each scalar v → v as f32 / 32768.0 … bit-deterministic" | common/mod.rs:250-254 | REAL | i16 exact in f32; power-of-two divisor; audio-engine stage A comment matches (pipeline.rs:497 "exactly v / 32768") |

## 7. Dead / duplicated / unreachable code
No dead code found (all helpers consumed by the 4 suites — rg `common::`).
Duplication of the SHA-256/NCO family across three test commons noted in
F-002.

## 8. Dependencies & flags
std-only (no deps in the test target beyond the crate under test). The
`#![allow(dead_code)]` is the only lint suppression.

## 9. Open questions for the next agent
- (resolved while writing) The demuxed-bytes side IS verified elsewhere:
  demux/tests/fixtures_parent.rs:167-168 asserts the demuxed PCM's length and
  sha256 equal the expectation (`sha256_hex(&pcm) == expect.audio_stream_sha256`),
  and codecs/tests/pcm_fixtures.rs:172-177 asserts the raw `01wb` walk
  reproduces the writer's stream digest. The full loop spec == digest ==
  file bytes == decoded output is closed in-tree. No open question remains
  for this file.
