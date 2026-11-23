# AUDIT: crates/codecs/src/audio/flac/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W07-A08 |
| Date | 2026-11-23 |
| Lines / functions | 239 / 4 (+5 tests) |
| File verdict | REAL (harness executed by tests I ran); the libFuzzer campaign itself has never executed anywhere — REAL-UNPROVEN campaign |
| Reachable from shipped entry point? | Not a shipped surface (fuzz/test). The registered targets exist: fuzz/fuzz_targets/fuzz_codec_flac.rs:19 and fuzz_demux_flac.rs:19 both call this module; fuzz-registry gate CLEAN (audit/runs/baseline/full_gate.txt:57 "35 targets, diff empty, corpora present"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/codecs-a0d7a93b7072ff58 --test-threads=2 flac::fuzz` → 5 passed, exit 0; `./target/debug/deps/flac_fuzz_survival-2f2a2aff8aea98f3 --test-threads=2` → 2 passed / 1 ignored (deep), 13.2 s, exit 0 |

## 1. One-paragraph verdict
A clean, honest, bounded fuzz harness: four passes per call (metadata walk, seek resolve, free frame decode, full codec-api configure→decode→flush cycle, plus a max-slot decode), all errors swallowed by contract, all memory input-derived and capped. The in-crate smoke tests and the 48-seed-per-class mutation survival run both pass under my execution, and the survival suite genuinely drives this module's exported entry. Two honesty defects: the "wiring status (honest)" section is stale — it claims the `fuzz/Cargo.toml` scaffolding and `libfuzzer-sys` dependency are still missing Lead decisions, but both landed (card W1-12); and `fuzz_demux_flac` is named for a demuxer that does not exist (it fuzzes the codec crate's metadata walk + seek planner). The real gap it half-admits: no coverage-guided campaign has ever run — the nightly wiring is still a pending Lead decision, so "panic-free by construction" rests entirely on the deterministic suites.

## 2. What actually works (verified)
— `./target/debug/deps/codecs-a0d7a93b7072ff58 --test-threads=2 flac::fuzz` → `test result: ok. 5 passed` (empty/marker/1..40-length/all-0xFF/all-0x00/sync-flood inputs; synthetic-facts domain invariants).
— `./target/debug/deps/flac_fuzz_survival-2f2a2aff8aea98f3 --test-threads=2` → 2 passed, 1 ignored (documented deep run): 4 real fixtures × 48 seeded mutations per class (truncation/flip/splice/fact-lies) through the very `fuzz_codec_flac` entry, with every error asserted to be in the documented `flac.*` typed set (flac_fuzz_survival.rs:209, 282, 343).
— Bounds re-derived by reading: `synthetic_facts` (86-100) keeps channels 1..=8, bps 4..=32, block ≤ 32768; pass 4 slot capped `min(1 << 20)` (182); `cut` in `[0, len]` (165) — no slice panic possible; STREAMINFO packing (145-148) fits 20+3+5+36 = 64 bits.

## 3. Findings
F-001 [S3]
Where: fuzz.rs:19-30 (module doc "Wiring status (honest)") vs fuzz/Cargo.toml:38-58
Code:
```
//! The cargo-fuzz target file is proposed at
//! `fuzz/fuzz_targets/fuzz_codec_flac.rs` exactly as doc 04 §8.5
//! registers it. Building it needs the `libfuzzer-sys` dev-dependency
//! plus the `fuzz/Cargo.toml` scaffolding — both Lead-owned dependency
//! decisions (AGENTS.md §3.2, C5) — so until they land the harness
```
What is wrong: stale. Both things it says are missing have landed: fuzz/Cargo.toml exists with `libfuzzer-sys = "0.4.13"` (line 38) and 35 registered targets including fuzz_codec_flac.rs and fuzz_demux_flac.rs (both call this module), and the full-gate fuzz-registry check passes CLEAN. The still-true remainder — the nightly-toolchain campaign is not wired into any gate (fuzz/README.md "UNTIL: the decision lands, nothing in the local gate or CI builds [the campaign]") — means the "+FB" fuzz deliverable has never executed as a fuzzer: zero coverage-guided runs, zero crash findings possible. "Panic-free by construction" is proven only by the deterministic classes I ran. What correct looks like: update the wiring doc, and get the nightly decision made or the campaign claim removed. Confidence: HIGH — Verification: read + executed the deterministic evidence.

F-002 [S4]
Where: fuzz.rs:62-67
Code:
```
/// The native-FLAC **demux** fuzz pass (card 5.Q1; doc 04 §8.5
/// registry name `fuzz_demux_flac`, table §2.1 row "FLAC (native,
/// Ogg-FLAC)" — the container-surface half of this module's codec
```
What is wrong: the registry name promises a demux surface; there is no native-FLAC demuxer to fuzz (`open_flac` refuses, apps/cli/src/containers/es.rs:262-270). What actually runs is `parse_stream` (a codec-crate metadata walk) plus the seek planner — a demuxer-shaped target for a demuxer that does not exist. The doc body says this honestly; the name and the "container-surface" framing do not. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: fuzz.rs:127-129 (pass 2) and 105
Code:
```
    // Pass 2: the free frame decode — the whole buffer as one packet
    // (invalid as a frame or not; only survivability is asserted).
    let _ = decode_frame(data, &facts);
```
What is wrong: minor coverage shape — pass 2/3/4 always feed the buffer as a single packet sliced by one input byte; no pass walks multiple frames out of one buffer (the survival suite's `walk_frames` loop compensates outside the harness). A real campaign would cover this; the deterministic suites only partially do. Confidence: HIGH — Verification: read.

## 4. Per-function table (file ≤ 100 LOC of non-test code; table included for completeness)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CountingSink (impl FrameSink) | 51-60 | REAL | — | saturating counter, no panic |
| fuzz_demux_flac | 68-80 | REAL | S4 | misnamed surface (F-002); executes planner.resolve only |
| synthetic_facts | 86-100 | REAL | — | legal-domain facts; invariants pinned by test |
| fuzz_codec_flac | 105-185 | REAL | S3 | 4 passes, bounded; never executed as a campaign (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| demux_pass_survives_hostile_input | Only on panic/hang | survival contract by design; fixed 4 inputs |
| empty_input_survives_all_passes | Only on panic | minimal smoke |
| marker_only_and_short_inputs_survive | Only on panic | 1..=40 lengths, deterministic pattern |
| hostile_wide_inputs_survive | Only on panic | 0xFF/0x00 floods + crafted sync flood — the strongest smoke |
| synthetic_facts_stay_in_the_legal_domain | Yes | asserts the bounds the module doc claims |
| (external) fixtures_survive_truncation_flips_splices_and_fact_lies | Yes | asserts typed error codes, not just survival; 48 seeds × 4 classes × 4 fixtures, reproducible |
| (external) deep_exhaustive_flip_sweep_survives | UNVERIFIED | #[ignore] deep run (~3 min claimed); not executed by me |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Building it needs the `libfuzzer-sys` dev-dependency plus the `fuzz/Cargo.toml` scaffolding … until they land" | fuzz.rs:23-27 | STALE/FALSE | fuzz/Cargo.toml:38 `libfuzzer-sys = "0.4.13"`; targets exist; fuzz-registry gate CLEAN (full_gate.txt:57) |
| "the libFuzzer campaign itself is owed CI wiring" | fuzz.rs:28-29 | REAL (still true) | fuzz/README.md LEAD-DECISION marker; nothing in CI builds it |
| "harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:14-17 | REAL | all errors `let _ =`-swallowed; executed |
| "Memory is bounded by the input … ≤ 65 535 × 8 × 4" | fuzz.rs:32-39 | REAL | synthetic_facts + `min(1<<20)` cap re-derived by read; domain pinned by test |

## 7. Dead / duplicated / unreachable code
None dead within the file. The campaign layer (libfuzzer-sys driver) is present but never invoked by any gate — scaffolded, dormant.

## 8. Dependencies & flags
No new deps in this file. Campaign side: fuzz/Cargo.toml (own workspace root) with libfuzzer-sys 0.4.13 caret pin; requires nightly (repo pinned stable 1.99.1) — the documented reason no campaign runs.

## 9. Open questions for the next agent
- Who owns the nightly-toolchain Lead decision (fuzz/README.md)? Every codec fuzz harness in this repo shares F-001's status — a repo-wide "fuzzer that never fuzzed" finding.
- The `#[ignore]` deep survival run (~3 min): worth one CI slot; nobody runs it today (baseline: 51 ignored tests repo-wide).
