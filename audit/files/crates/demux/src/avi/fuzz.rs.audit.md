# AUDIT: crates/demux/src/avi/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-23 |
| Lines / functions | 323 / 1 pub fuzz fn + 3 budget consts + 4 tests + scrambler |
| File verdict | REAL |
| Reachable from shipped entry point? | Compiled into every build (`pub mod fuzz;` avi/mod.rs:119, no cfg gate) but not called by lumen-cli/desktop — exercised via cargo test + the cargo-fuzz target `fuzz/fuzz_targets/fuzz_demux_avi.rs` (exists, verified) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | sed/rg/python3 reads; verified fuzz target file + fuzz/Cargo.toml existence; baseline citations :1803-1806, :2278 |

## 1. One-paragraph verdict
A real deterministic fuzz harness: one pass = construct + start + bounded
drain (200k event budget) + 4 data-derived seek probes each followed by a
64-event drain. Errors are swallowed by documented contract (the property
under test is absence of panic/hang/unbounded allocation). The panic-free
claim rests on the demuxer's own ceilings (verified in my avi/mod.rs audit)
plus explicit budgets; the seed read is independently guarded (85-91); seek
targets are provably non-negative i64 (seed>>1 ≤ 2^63−1). The four in-crate
tests (empty/tiny, scrambled prefixes ×64 severities, adversarial size
fields incl. u32::MAX at 8 sites, scrambled 40-byte WAVEFORMATEXTENSIBLE
headers + 8 strf truncations) all pass in baseline, and the seeded
longer-loop integration test `avi_seed_loop_survives` passed in 1.74s
(baseline :2278). Two S4s: the wiring-status doc is stale (says the
fuzz/Cargo.toml scaffolding hasn't landed; it landed — card W1-12), and the
harness ships in release builds as uncalled pub API.

## 2. What actually works (verified)
- All 4 tests pass: baseline cargo-test-workspace.txt:1803-1806
  (`avi::fuzz::tests::adversarial_size_fields_survive ... ok` etc.).
- Seeded survival loop over the shipped harness:
  `avi_seed_loop_survives ... ok` (baseline :2278, 1.74s) +
  `avi_seed_loop_long ... ignored` (the documented long lane).
- The cargo-fuzz target exists and forwards to this harness:
  fuzz/fuzz_targets/fuzz_demux_avi.rs (`fuzz_target!(|data: &[u8]|` →
  verified by read; its own header documents the remaining nightly-wiring
  gap honestly).
- Integration consumers run the harness over committed bytes:
  fixtures_class_b.rs (baseline :2255 trunc4 salvage lane context) and
  wav_aiff-style corpus sweeps reference `fuzz_demux_avi`-family harnesses.

## 3. Findings
F-001 [S4]
Where: crates/demux/src/avi/fuzz.rs:14-21
Code:
```
//! registers it. Building it needs `libfuzzer-sys` (an external
//! dev-dependency) plus the `fuzz/Cargo.toml` scaffolding — both are
//! Lead-owned dependency decisions (AGENTS.md §3.2), so until they
//! land the harness logic is proven by the in-crate deterministic
```
What is wrong: stale wiring status. fuzz/Cargo.toml (package `lumen-fuzz`,
libfuzzer-sys 0.4.13) EXISTS — the target file's own header (updated
2026-10-22, card W1-12) says "the cargo-fuzz scaffolding now EXISTS … What
is still owed is the *nightly wiring*". This module's doc still claims both
are absent. Two docs disagree; the target file is the newer truth.
Confidence: HIGH — Verification: read (both files).
F-002 [S4]
Where: crates/demux/src/avi/mod.rs:118-121 (affecting this file)
Code:
```
/// The `+FB` fuzz harness (doc 04 §8.5 registry name `fuzz_demux_avi`).
pub mod fuzz;
```
What is wrong: the harness (and test_support) compile into the release lib
unconditionally — shipped binaries carry fuzz/test-seam code no entry point
calls. Deliberate (lib API surface, "doctests, fuzz, downstream"), but it
is binary mass with zero shipped callers. Confidence: HIGH — Verification:
read (no cfg attribute; release lane builds the lib).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FUZZ_EVENT_BUDGET / FUZZ_POST_SEEK_DRAIN / FUZZ_SEEK_PROBES | 37-47 | REAL | — | documented proposals; budgets real (used below) |
| fuzz_demux_avi | 52-113 | REAL | — | construct→start→bounded drain→4 seek probes×64 drains; guarded seed read; i64-safe targets; sticky errors naturally gate post-error probes |
| Scrambler (xorshift64*) | 121-141 | REAL | — | hand-rolled, deterministic; modulo indexing bounds-checked via is_empty |
| empty_and_tiny_inputs_survive | 143-149 | REAL | — | [], 2B, 12B, 4096×0xff |
| scrambled_structured_prefixes_survive | 151-191 | REAL | — | 0..63 flips × seeded scrambler over a structured AVI prefix |
| adversarial_size_fields_survive | 193-232 | REAL | — | u32::MAX/0/1 patches at 8 header sites + clean core |
| scrambled_extensible_audio_headers_survive | 234-322 | REAL | — | 40-byte WAVEFORMATEXTENSIBLE strf, 64 flip severities + 8 strf-region truncations [160..214] |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 4 | YES for panics/hangs on their input families | a panic propagates (no catch); budgets bound hangs; nothing else is asserted BY CONTRACT (documented :6-10) — the error-flavor pinning lives in the integration suites, correctly separated |
| avi_seed_loop_survives (integration, other file) | YES | longer seeded campaign over the same harness |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "One call = one full untrusted-container pass: construct, start, drain … then probe seeks … and drain a little after each" | :4-6 | REAL | :52-113 implements exactly this; executed via baseline :1803-1806 |
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | :6-8 | REAL | all Err arms break/return; no asserts inside the harness |
| "Building it needs libfuzzer-sys … plus the fuzz/Cargo.toml scaffolding — both are Lead-owned … until they land" | :16-19 | FALSE (stale) | F-001: scaffolding landed (fuzz/Cargo.toml + 12+ targets verified present; target header card W1-12) |
| "every loop in here has a hard cap" | :25-26 | REAL | 200k events (:68), 64 drains (:105), 4 probes (:93) |

## 7. Dead / duplicated / unreachable code
None dead in this file. Shipped-but-uncalled status noted in F-002.

## 8. Dependencies & flags
std-only in-crate; the cargo-fuzz target pulls libfuzzer-sys (dev lane
only, fuzz/Cargo.toml). No feature gates. No unsafe, no unwrap (the `let
Ok(...) else` shapes are the refusal vocabulary).

## 9. Open questions for the next agent
- fuzz/ crate as a whole (another agent's lane): whether the nightly
  libFuzzer campaign ever ran (no evidence in baseline — "owed CI wiring").
- test_support.rs (my next file) shares the ships-unconditionally property.
