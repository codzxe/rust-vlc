# AUDIT: fuzz/fuzz_targets/fuzz_codec_mp2.rs
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 21 / 1 (fuzz_target closure) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — fuzz campaign binary; reachable from CI nightly audit.yml `fuzz` tier via scripts/quality/25_fuzz_campaign.sh |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg -n "pub fn fuzz_mp2_stream" crates/codecs/src/audio/mp2/` → mp2/fuzz.rs:78; `rg -n "fuzz_mp2_stream" crates/codecs/tests/` → mp2_fuzz_survival.rs:21; baseline cargo-test-workspace.txt L1241 `Running tests/mp2_fuzz_survival.rs` → 3 tests, corpus_dirty_and_garbage_pools_survive ok |

## 1. One-paragraph verdict
Real thin libFuzzer shim (21 lines) calling a real in-crate harness (`codecs::audio::mp2::fuzz::fuzz_mp2_stream`, crates/codecs/src/audio/mp2/fuzz.rs:78, which returns a typed FuzzOutcome and has its own in-crate unit tests at fuzz.rs:161-185). The stable surrogate (crates/codecs/tests/mp2_fuzz_survival.rs) ran in the committed baseline. The header's wiring status is stale the same way as the other targets (nightly campaign exists in .github/workflows/audit.yml) — see the fuzz_demux_avi.rs audit F-001/F-002 for the campaign-level defects (no `set -e`, build-failures render as `clean`, zero committed campaign evidence); those apply to this target verbatim and are not re-counted here.

## 2. What actually works (verified)
- `rg -n "pub fn fuzz_mp2_stream" crates/codecs/src/audio/mp2/` → `fuzz.rs:78: pub fn fuzz_mp2_stream(data: &[u8]) -> FuzzOutcome {` — harness exists; shim calls it correctly (L18-20).
- Baseline (cargo-test-workspace.txt L1241+): `Running tests/mp2_fuzz_survival.rs` → `running 3 tests`, `test corpus_dirty_and_garbage_pools_survive ... ok` — the surrogate ran.
- In-crate unit tests of the harness itself exist (fuzz.rs:161,170,179,185 call fuzz_mp2_stream on empty/garbage/valid inputs).
- Not compiled here (cargo absent). UNVERIFIED at binary level.

## 3. Findings
F-001 [S3]
Where: fuzz/fuzz_targets/fuzz_codec_mp2.rs:7-10 (header) — stale wiring claim, identical defect class to fuzz_demux_avi.rs F-001
Code (verbatim, read of full 21-line file):
```
// Wiring status (honest): building the campaign needs the Lead's
// nightly wiring (see fuzz/README.md); until then this package
// compiles on stable (`cargo check` in fuzz/) and the harness logic
// is proven by the in-crate deterministic hostile-input sweeps.
```
What is wrong: "building the campaign needs the Lead's nightly wiring" — the wiring exists: .github/workflows/audit.yml `fuzz:` tier installs nightly + cargo-fuzz and runs scripts/quality/25_fuzz_campaign.sh over all targets (5 shards, cron 0 2 * * *). Why it matters: same stale-doc class; also the header says the package "compiles on stable (`cargo check` in fuzz/)" — UNVERIFIED: no committed log of that check exists and no CI step performs it (rg for `cargo check` in audit.yml/ci.yml shows none targeting fuzz/). What correct looks like: header names the nightly tier; the claimed stable check is either wired or deleted from the claim.
Confidence: HIGH — Verification: read + workflow rg.
Campaign-level defects (build-fail = "clean" vacuous green; zero committed campaign evidence) inherited from the avi target audit — not double-counted.

## 4. Per-function table (file < 100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_target closure | 18-20 | REAL | — | one call to codecs::audio::mp2::fuzz::fuzz_mp2_stream; no own assertions (libFuzzer detects crashes) |

## 5. Test honesty
No tests in the target (correct for a shim). Surrogate mp2_fuzz_survival.rs ran in baseline (3 tests) — honest survival tests over dirty/garbage pools. Would fail if the harness panicked? YES (survival loop calls the same entry point with hostile bytes).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Wiring status (honest): building the campaign needs the Lead's nightly wiring" | header L7-8 | FALSE (stale) | audit.yml fuzz tier exists (F-001) |
| "this package compiles on stable (`cargo check` in fuzz/)" | header L9 | REAL-UNPROVEN | no committed evidence of that check (UNVERIFIED: cargo-build-budget; no CI step found) |
| "the harness logic is proven by the in-crate deterministic hostile-input sweeps" | header L9-10 | REAL | baseline mp2_fuzz_survival 3 tests ran; in-crate unit tests at mp2/fuzz.rs:161-185 |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
libfuzzer-sys 0.4.13; codecs path dep. Standalone fuzz workspace.

## 9. Open questions for the next agent
- Does codecs::audio::mp2::fuzz::fuzz_mp2_stream actually decode every packet (full untrusted pass as header claims) or stop early? fuzz.rs is not my file.
