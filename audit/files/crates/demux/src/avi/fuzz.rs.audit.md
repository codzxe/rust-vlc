# AUDIT: crates/demux/src/avi/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-21 |
| Lines / functions | 323 / 10 (1 harness fn, 3 budget consts, Scrambler + 2 methods, 4 tests) |
| File verdict | UNREACHABLE (real, executed harness — but only from tests; the libFuzzer target that would ship it has never been built by any gate) |
| Reachable from shipped entry point? | NO — `fuzz_demux_avi` is called only by: the in-crate tests below, `crates/demux/tests/fuzz_loop_stable.rs:59` (integration), and `fuzz/fuzz_targets/fuzz_demux_avi.rs:31` (a cargo-fuzz shim that is compile-ready but "not built by any gate" per its own header). `lumen`/`lumen-tauri` never invoke it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | full Read; `ls fuzz/ fuzz/fuzz_targets/` (scaffold + 35 targets EXIST); `cat fuzz/Cargo.toml` (libfuzzer-sys 0.4.13 declared); `cat fuzz/fuzz_targets/fuzz_demux_avi.rs` (shim calls this fn); `sed -n '110,170p' fuzz/README.md` ("**No such run has happened yet**" — campaign never executed); `head -30 scripts/fuzz_registry_check.sh` (gate = name-consistency + non-empty corpus + fn exists — no execution); `git log --diff-filter=A -- fuzz/Cargo.toml` (scaffold added in 5d34901e, W1-12); baseline `orchrun-test-workspace-nofailfast.log.txt:1590-1593` (4/4 file tests ok), `:963-973` (fuzz_loop_stable 5 passed / 2 ignored), `:975-983` (fuzz_survival 6 passed) |

## 1. One-paragraph verdict
A clean, honest, bounded fuzz harness: one full construct→start→drain→seek-probe pass over attacker bytes, hard event budget (200k), data-derived non-negative seek targets (arithmetic verified), every error swallowed by contract, and four deterministic in-crate tests plus two integration survival suites that all ran green in the baseline — this is the *only* fuzz harness in my assigned set with real execution evidence behind it. The defects are documentation-level: the module's "Wiring status (honest)" section is stale — it claims the cargo-fuzz scaffolding and libfuzzer-sys are still-unlanded Lead decisions, when commit 5d34901e (W1-12) landed both and the target shim's own header was updated to say so (2026-10-22) while this module doc was not. The larger truth it half-states: **no libFuzzer campaign has ever run** (README: "No such run has happened yet"), so every "fuzz target registered" claim in the registry proves file existence, not execution — the registry gate (`fuzz_registry_check.sh`) checks names, corpus presence, and that the harness fn exists, nothing more.

## 2. What actually works (verified)
— All 4 in-crate tests executed and passed: `orchrun-test-workspace-nofailfast.log.txt:1590-1593` (`avi::fuzz::tests::* ... ok`).
— Cross-suite execution: `tests/fuzz_loop_stable.rs` ran `5 passed; 2 ignored` (:963-973 — the two `#[ignore = "long fuzz run"]` loops are opt-in by design); `tests/fuzz_survival.rs` 6/6 ok (:975-983), including `systematic_single_byte_flips_of_a_real_frame_survive` and `systematic_truncations_of_a_real_frame_survive`.
— Seek-target arithmetic re-derived: `seed ≤ u64::MAX`; `seed >> 1 ≤ 2^63 − 1` fits `i64` non-negative; targets `[0, >>1, >>2, >>3]` all ≥ 0 — the "non-negative and saturate within i64" comment (:92) is exact.
— Budget arithmetic re-derived: outer drain ≤ 200,000 counted events (`events += 1` then `>=` check — at most one event past the budget), post-seek drains ≤ 4 × 64; total work bounded independent of input.
— The shim is real: `fuzz/fuzz_targets/fuzz_demux_avi.rs:31` calls `demux::avi::fuzz::fuzz_demux_avi(data)` — the in-crate harness IS the campaign body; the registry gate verifies this call resolves (baseline #18 PASS, 35 targets, diff empty).

## 3. Findings
### F-001 [S4]
Where: crates/demux/src/avi/fuzz.rs:12-21
Code:
```rust
//! The cargo-fuzz target file lives at
//! `fuzz/fuzz_targets/fuzz_demux_avi.rs` exactly as doc 04 §8.5
//! registers it. Building it needs `libfuzzer-sys` (an external
//! dev-dependency) plus the `fuzz/Cargo.toml` scaffolding — both are
//! Lead-owned dependency decisions (AGENTS.md §3.2), so until they
//! land the harness logic is proven by the in-crate deterministic
//! smoke tests here and by the integration corpus run; the libFuzzer
//! campaign itself is owed CI wiring.
```
What is wrong: the scaffolding and libfuzzer-sys HAVE landed — `fuzz/Cargo.toml` exists declaring `libfuzzer-sys = "0.4.13"` (added by commit 5d34901e, card W1-12), and the target shim's own header ("WIRING STATUS (updated 2026-10-22, card W1-12): the cargo-fuzz scaffolding now EXISTS") supersedes this module doc, which still says "until they land". The only outstanding piece is the nightly toolchain (repo pins stable 1.99.1; cargo-fuzz needs `-Zsanitizer`).
Why it matters: the in-crate module doc is what a wave auditor or the Lead reads first for the security-coverage posture; it under-reports landed state and misdirects the "what's owed" list (nightly wiring, not scaffolding).
What correct looks like: copy the shim's updated status into this doc (scaffold exists; nightly is the outstanding LEAD-DECISION; seeds = class-B AVI derivatives + 256-byte-boundary truncations).
Confidence: HIGH — Verification: read + git log (5d34901e added fuzz/Cargo.toml; avi/fuzz.rs last touched earlier by 33e85dcd) + cat of both files.

### F-002 [S4]
Where: crates/demux/src/avi/mod.rs:71 (`pub mod fuzz;`) consuming crates/demux/src/avi/fuzz.rs:52
Code:
```rust
pub fn fuzz_demux_avi(data: &[u8]) {
```
What is wrong: `fuzz` is an unconditional pub module of the demux lib — the harness (and the `test_support` module it depends on) compile into every release binary although no shipped lane can call them; their only callers are test binaries and the never-built cargo-fuzz shim.
Why it matters: shipped-surface bloat and a standing pattern across the demux crate (aiff `test_support`, asf `test_support`, wav twins — see my chunks.rs and error.rs audits); the fuzz bodies also give an attacker reading the binary a free map of the demuxer's defensive boundaries.
What correct looks like: `#[cfg(any(test, feature = "fuzz"))]` gating (the feature already exists as a pattern in media-probe's `contract`), with the cargo-fuzz manifest enabling it.
Confidence: HIGH — Verification: read + call-site census (callers enumerated in the header row).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FUZZ_EVENT_BUDGET` / `FUZZ_POST_SEEK_DRAIN` / `FUZZ_SEEK_PROBES` | 40-47 | REAL | — | 200k / 64 / 4 with honest `[PROPOSAL]` markers; consumed by fuzz_loop_stable.rs:46 too. |
| `fuzz_demux_avi` | 52-113 | REAL | — | Contract = absence-of-panic/hang/unbounded-alloc only (docs :6-10 honest). Errors swallowed by design; EndOfStream/Err both break; seek probes guarded by independent 8-byte read (:85-91); `TimeBase::new(1, 1e9)` cannot fail (defensive early-return). Memory: one `data.to_vec()` copy, then demuxer ceilings (MAX_CHUNK_PAYLOAD, MAX_IDX1_ENTRIES) — the strl-lane allocation noted in my index.rs#part1 F-001 is bounded by input size, so the harness claim holds under libFuzzer's max_len. |
| `Scrambler::next_u64` / `flip_byte` | 124-140 | REAL | — | xorshift64* correct shape; empty-input guard; modulo bias irrelevant for a scrambler. |
| tests ×4 | 143-322 | REAL | — | See §5. |

## 5. Test honesty (4 in-crate tests, baseline :1590-1593 all ok; plus two integration suites)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| empty_and_tiny_inputs_survive | YES | Empty/2B/12B-RIFF/4096×0xff — any panic in construction or the guarded seed read fails it. |
| scrambled_structured_prefixes_survive | YES | 64 deterministic scrambles (0..63 flips each) of a structurally plausible AVI prefix incl. idx1 — a panic on any flipped size/flag byte fails it; ~2k effective mutations. |
| adversarial_size_fields_survive | YES | Targeted u32::MAX/0/1 patches at 8 structural offsets (RIFF size, both LIST sizes, chunk size) — the classic lying-size cases; a size-driven panic/allocation fails it. |
| scrambled_extensible_audio_headers_survive | YES | The 40-byte WAVEFORMATEXTENSIBLE path (the 33e85dcd surface) with 64 scrambles + 8 boundary truncations inside the strf region — the D-3 dirty-world pin for the newest parse surface. |
| (integration) fuzz_loop_stable `mjpeg/pcm_seed_loop_survives`, `loop_accounting_is_deterministic`, `seeded_generation_is_deterministic_and_decorrelated`, `sub_magic_lengths_are_survivable_inputs` | YES | Baseline :963-973 all ok; the two `_long` variants are `#[ignore = "long fuzz run — cargo test -- --ignored --nocapture"]` (:306) — opt-in by design, visible as ignored in every run (part of the baseline's 49 ignored), not silently skipped. |
| (integration) fuzz_survival 6 tests | YES | Baseline :975-983; systematic single-byte flips and truncations of a real frame. |

Mutation reasoning: the tests are survival-only by contract — they fail on panic/hang/abort (the harness budget turns a non-monotone walk into a visible hang) and cannot fail on wrong-but-safe semantics, which the conformance suite owns. That scoping is stated in the module docs (:6-10) and is honest.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | :6-10 | REAL | Read; the four tests + two integration suites execute exactly that. |
| "Building it needs libfuzzer-sys … plus the fuzz/Cargo.toml scaffolding — both are Lead-owned … until they land" | :16-19 | **STALE/FALSE** | Both landed (commit 5d34901e; fuzz/Cargo.toml declares libfuzzer-sys 0.4.13; shim header updated 2026-10-22) — F-001. |
| "Never panics, never hangs, never allocates without a ceiling — by construction" | :49-51 | REAL-UNPROVEN at campaign scale | By-construction argument + deterministic survival evidence (15 tests, baseline-cited); no libFuzzer run has ever tested it at scale — README: "No such run has happened yet". |
| "the `movi` walk itself is monotone (positions only advance)" | :25-27 | REAL (index/walk cross-check) | index.rs ladder/walk read (my file 4); budget is correctly described as the second belt. |
| doc 04 §8.5 registry row `fuzz_demux_avi` | fuzz/README.md registry + baseline gate #18 | REAL as "registered", PAPER-ONLY as "fuzzed" | Registry gate PASS (35 targets, diff empty) proves names/files/harness-fn existence + non-empty seed corpus — it executes nothing. |

## 7. Dead / duplicated / unreachable code
F-002 (module ships in release, callers are tests + never-built shim). The Scrambler is a deliberate in-file twin of fuzz_loop_stable's Rng (commented as such) — acceptable duplication with named provenance.

## 8. Dependencies & flags
None beyond the demux crate's own (media-types time types, plugin_api Demux/DemuxEvent, super::test_support). The cargo-fuzz side (libfuzzer-sys 0.4.13) lives in the separate fuzz workspace, not this crate. No unsafe, no unwrap (all `let Ok(..) = .. else` / match arms).

## 9. Open questions for the next agent
- The never-run campaign is the project-level gap: doc 04 §8.5's "runs 10 minutes clean in CI (release mode)" shape has zero committed executions for ALL 35 targets, not just AVI. The matrix/docs wave should record "fuzz targets: registered=35, executed=0" as a standing PAPER-ONLY row rather than leaving the registry gate's PASS to imply coverage.
- REQUEST-ORCH-RUN (optional, low priority): `cargo +nightly fuzz run fuzz_demux_avi -s none -- -max_total_time=60` — would either prove the scaffold builds under nightly or pin the exact nightly-wiring blocker; the sandbox toolchain may not have nightly.
