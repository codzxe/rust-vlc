# AUDIT: fuzz/fuzz_targets/fuzz_demux_mp4.rs
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 21 / 1 (fuzz_target closure) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — fuzz campaign binary; reachable from CI nightly audit.yml `fuzz` tier via scripts/quality/25_fuzz_campaign.sh |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg -n "pub fn fuzz_demux_mp4" crates/demux/src/mp4/` → mp4/fuzz.rs:45; `rg -n "fuzz::fuzz_demux_mp4" crates/demux/tests/` → mp4_fixtures.rs:247; baseline cargo-test-workspace.txt L2368-2375: `Running tests/mp4_fixtures.rs` → malformed_mp4_fixtures_survive_the_fuzz_harness ok, 3 tests ok |

## 1. One-paragraph verdict
Real thin libFuzzer shim calling a real in-crate harness (`demux::mp4::fuzz::fuzz_demux_mp4`, crates/demux/src/mp4/fuzz.rs:45). The stable surrogate — crates/demux/tests/mp4_fixtures.rs:247 `fuzz::fuzz_demux_mp4(&bytes)` over malformed fixtures — ran green in the committed baseline (L2373-2374: `malformed_mp4_fixtures_only_ever_refuse_or_demux_typed ... ok`, `malformed_mp4_fixtures_survive_the_fuzz_harness ... ok`). Header carries the same stale "needs the Lead's nightly wiring" claim as every other target (the nightly tier exists); campaign-level defects (build-fail rendered `clean`; zero committed campaign evidence) are inherited from the fuzz_demux_avi.rs audit F-001/F-002 and not re-counted.

## 2. What actually works (verified)
- `rg -n "pub fn fuzz_demux_mp4" crates/demux/src/mp4/` → `fuzz.rs:45: pub fn fuzz_demux_mp4(data: &[u8]) {` — harness exists; shim call (L18-20) matches signature.
- Baseline: `Running tests/mp4_fixtures.rs` (cargo-test-workspace.txt L2368) → `test malformed_mp4_fixtures_only_ever_refuse_or_demux_typed ... ok`, `test malformed_mp4_fixtures_survive_the_fuzz_harness ... ok`, `test valid_mp4_fixtures_demux_fully ... ok` (L2373-2375).
- Not compiled here (cargo absent). UNVERIFIED at binary level.

## 3. Findings
F-001 [S3]
Where: fuzz/fuzz_targets/fuzz_demux_mp4.rs:7-10 (header)
Code (verbatim, read of full 21-line file):
```
// Wiring status (honest): building the campaign needs the Lead's
// nightly wiring (see fuzz/README.md); until then this package
// compiles on stable (`cargo check` in fuzz/) and the harness logic
// is proven by the in-crate deterministic hostile-input sweeps.
```
What is wrong: identical stale claim as fuzz_codec_mp2.rs F-001 — the nightly wiring EXISTS (.github/workflows/audit.yml `fuzz:` tier, cron 0 2 * * *, cargo-fuzz installed, 5 shards). The "compiles on stable (cargo check in fuzz/)" claim has no committed evidence and no CI step performs it. Why it matters: stale wiring docs + unverifiable compile claims in a file whose only purpose is to be built by that wiring. What correct looks like: header names the nightly tier.
Confidence: HIGH — Verification: read + workflow rg.

## 4. Per-function table (file < 100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_target closure | 18-20 | REAL | — | one call to demux::mp4::fuzz::fuzz_demux_mp4 |

## 5. Test honesty
No tests in the target (correct for a shim). Surrogate mp4_fixtures.rs ran in baseline; `malformed_mp4_fixtures_survive_the_fuzz_harness` would FAIL if the harness panicked on the committed malformed MP4 corpus (including this audit's mp4_h264_aac__garbage0.mp4 fixture family). Honest survival test.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "building the campaign needs the Lead's nightly wiring" | header L7-8 | FALSE (stale) | audit.yml fuzz tier exists |
| "this package compiles on stable (`cargo check` in fuzz/)" | header L9 | REAL-UNPROVEN | no committed check log; no CI step (UNVERIFIED: cargo-build-budget) |
| "the harness logic is proven by the in-crate deterministic hostile-input sweeps" | header L9-10 | REAL | baseline mp4_fixtures 3 tests ok (L2373-2375) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
libfuzzer-sys 0.4.13; demux path dep. Standalone fuzz workspace.

## 9. Open questions for the next agent
- mp4/fuzz.rs: does the harness bound allocation from box/atom size fields before walking? (Not my file.)
