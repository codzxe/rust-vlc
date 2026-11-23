# AUDIT: crates/codecs/tests/mp3_fuzz_survival.rs
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2025-07-14 |
| Lines / functions | 172 / 4 (+2 tests) |
| File verdict | REAL (executed) |
| Reachable from shipped entry point? | N/A test suite over the production `codecs::audio::mp3::decode_stream` free entry |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/mp3_fuzz_survival-a38f4eaafbfa5a92 --nocapture` → 2 passed (exit 0, 21.2 s); output: "401 mutants decoded (possibly short), 139 typed refusals, codes observed: {mp3.frame.decode, mp3.id3.past_end, mp3.stream.header_frame_only, mp3.stream.no_frames, mp3.xing.frame_count_mismatch}"; corpus-pool test SKIPped (corpus absent, message observed) |

## 1. One-paragraph verdict
Genuine fuzz-lite survival suite with honest bookkeeping: 6 fixtures × 90 deterministic mutants each (byte flips incl. 4-flip bursts, head/mid/tail truncations, garbage prefixes over a decapitated head) driven through the real production decode entry; every mutant must settle as decoded-or-typed, all refusal codes must be `mp3.*` (foreign-stage leak = failure), and the taxonomy must be reachable (`refused > 0` — non-vacuity). I executed it: 540 mutants, 5 distinct typed codes observed, zero panics/hangs. The corpus dirty/garbage pool lane SKIPs cleanly when the corpus is absent (observed live). One trivial note only.

## 2. What actually works (verified)
— verified by executing the suite (output above): the tally assert (540), the `mp3.*` code filter, and the reachability assert all passed on real adversarial input; the five observed codes are a live taxonomy, not a dead branch.

## 3. Findings
F-001 [S4]
Where: mp3_fuzz_survival.rs:62,80-82
Code:
```
        let mut rng = Lcg(0x5A3_2026_1015 + u64::from(name.len() as u16));
```
What is wrong: (a) the per-fixture seed is derived from NAME LENGTH, so two equal-length fixtures would receive identical mutation positions (masked here — the six fixtures have distinct lengths — and the pristine data still differs); (b) the truncation sweep `cut = len*(k+1)/21` for k in 0..20 never tests cut=0 (empty input) or cut=len (whole file) — those edges are covered by other suites but not this one. Cosmetic coverage notes on an otherwise sound sweep. Confidence: HIGH — Verification: read + executed.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Lcg (next/below) | 21-33 | REAL | — | deterministic Knuth-style LCG, documented convention; modulo bias irrelevant for tests |
| Outcome/classify | 36-46 | REAL | — | decoded vs typed(code) — records, never requires failure |
| mutated_fixtures_never_panic_and_settle_typed | 48-129 | REAL | F-001 | 540 mutants; assert decoded+refused==540 (tally sanity); mp3.* code filter; refused>0 |
| corpus_dirty_and_garbage_pools_survive | 131-172 | REAL | — | SKIP-aware; up to 40 files/pool; same typed-code contract |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mutated_fixtures_never_panic_and_settle_typed | YES | a panic fails the test; a hang hangs it; a foreign error code trips the filter; an all-decodes run trips `refused > 0`; executed for real (139 refusals observed) |
| corpus_dirty_and_garbage_pools_survive | YES when corpus present | same contract over pool files; SKIPs (documented) when absent — vacuous pass in this environment |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "NO panic, NO hang — every input is either decoded or refused with a typed EngineError" | mp3_fuzz_survival.rs:5-6 | REAL (for the 540 swept mutants) | executed; note scope: fuzz-lite (60 mutants/fixture), not a libFuzzer campaign — honestly named |
| "Verdicts are recorded per class ... never 'must fail'" | mp3_fuzz_survival.rs:7-9 | REAL | tally + code census in the eprintln (observed) |

## 7. Dead / duplicated / unreachable code
None. (`mod common; mod mp3;` pull the codecs tests/common and tests/mp3 helper modules — outside my assignment.)

## 8. Dependencies & flags
codecs::audio::mp3::decode_stream, media-types, testkit. No ignores, no feature gates, no unsafe.

## 9. Open questions for the next agent
None.
