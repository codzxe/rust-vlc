# AUDIT: fuzz/fuzz_targets/fuzz_sub_srt.rs
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 21 / 1 (fuzz_target closure) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — fuzz campaign binary; reachable from CI nightly audit.yml `fuzz` tier via scripts/quality/25_fuzz_campaign.sh |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg -n "pub fn fuzz_sub_srt" crates/subtitles/src/srt/` → srt/fuzz.rs:40; `rg -n "fuzz_sub_srt" crates/subtitles/tests/` → srt_fuzz_survival.rs:51; baseline cargo-test-workspace.txt L6109: `Running tests/srt_fuzz_survival.rs` → 4 tests incl. canonical_parents_still_parse_exactly ok |

## 1. One-paragraph verdict
Real thin libFuzzer shim calling a real in-crate harness (`subtitles::srt::fuzz::fuzz_sub_srt`, crates/subtitles/src/srt/fuzz.rs:40). The stable surrogate (crates/subtitles/tests/srt_fuzz_survival.rs — "seeded mutation loop, all through the [`subtitles::srt::fuzz`] harness") ran green in the committed baseline. Header carries the same stale nightly-wiring claim (the nightly campaign tier exists); campaign-level defects inherited from the fuzz_demux_avi.rs audit (F-001/F-002 there: build-fail = `clean` vacuous green; zero committed campaign evidence) — not re-counted here.

## 2. What actually works (verified)
- `rg -n "pub fn fuzz_sub_srt" crates/subtitles/src/srt/` → `fuzz.rs:40: pub fn fuzz_sub_srt(data: &[u8]) {` — harness exists; shim call (L18-20) matches.
- Baseline: `Running tests/srt_fuzz_survival.rs` (cargo-test-workspace.txt L6109) → `running 4 tests`, `test canonical_parents_still_parse_exactly ... ok` — the surrogate ran.
- Not compiled here (cargo absent). UNVERIFIED at binary level.

## 3. Findings
F-001 [S3]
Where: fuzz/fuzz_targets/fuzz_sub_srt.rs:7-10 (header)
Code (verbatim, read of full 21-line file):
```
// Wiring status (honest): building the campaign needs the Lead's
// nightly wiring (see fuzz/README.md); until then this package
// compiles on stable (`cargo check` in fuzz/) and the harness logic
// is proven by the in-crate deterministic hostile-input sweeps.
```
What is wrong: same stale claim class as fuzz_codec_mp2.rs F-001 — the nightly wiring exists in .github/workflows/audit.yml (`fuzz:` tier, cron 0 2 * * *, cargo-fuzz install, 5 shards). The "compiles on stable (cargo check in fuzz/)" has no committed evidence and no CI step performs it. What correct looks like: header names the nightly tier.
Confidence: HIGH — Verification: read + workflow rg.

## 4. Per-function table (file < 100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_target closure | 18-20 | REAL | — | one call to subtitles::srt::fuzz::fuzz_sub_srt |

## 5. Test honesty
No tests in the target (correct for a shim). Surrogate srt_fuzz_survival.rs (4 tests) ran in baseline — seeded mutation loop + canonical-parent exact-parse assertions; would fail if the SRT parser panicked or mis-parsed parents. Honest.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "building the campaign needs the Lead's nightly wiring" | header L7-8 | FALSE (stale) | audit.yml fuzz tier exists |
| "this package compiles on stable (`cargo check` in fuzz/)" | header L9 | REAL-UNPROVEN | no committed log; no CI step (UNVERIFIED: cargo-build-budget) |
| "the harness logic is proven by the in-crate deterministic hostile-input sweeps" | header L9-10 | REAL | baseline srt_fuzz_survival 4 tests ran (L6109+) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
libfuzzer-sys 0.4.13; subtitles path dep. Standalone fuzz workspace.

## 9. Open questions for the next agent
- srt/fuzz.rs: does the harness include a decode step or only parse? ("full untrusted pass" claim at header L4-5 — UNVERIFIED, not my file.)
