# AUDIT: crates/codecs/tests/mp2_fuzz_survival.rs
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2025-06-27 |
| Lines / functions | 204 / 4 fns + 3 tests |
| File verdict | REAL (executed: 3/3 pass, direct binary run, exit 0; live tally captured) |
| Reachable from shipped entry point? | N/A (integration test; baseline cargo-test-workspace.txt:1241-1249, 3/3 ok; I re-ran the built binary: 3/3 in 4.63s) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | ./target/debug/deps/mp2_fuzz_survival-ade46227d74772b4 --nocapture -> 3/3 ok EXIT=0 (tally line captured); stat binary-vs-source; sed/rg quote checks; ls fixtures |

## 1. One-paragraph verdict
A real fuzz-lite survival suite, executed by me directly: 220 deterministic mutants (byte flips biased into headers, head/mid/tail truncations, garbage prefixes) across two committed MP2 fixtures, plus 120 mutants through the registered fuzz harness with bounded-outcome asserts, plus a corpus dirty/garbage pool sweep. My run: 178 mutants decoded, 42 typed refusals, all refusals confined to the module's own codes (`mp2.frame.decode`, `mp2.stream.cbr_mismatch`, `mp2.stream.layer_mismatch`) — the foreign-code-leak guard and the `refused > 0` liveness assert both have teeth. The defects are hygiene: the corpus pool test silently passes green when the corpus is absent (this sandbox — confirmed live); the garbage-prefix mutant slices `pristine[k * 17.min(len)..]` whose precedence makes it `k*min(17,len)` — a latent panic if a fixture ever shorter than 153 bytes is used, and it drops real audio bytes the comment says should stay; and `Lcg::below` lacks the n==0 guard its adpcm sibling has (unreachable-zero today).

## 2. What actually works (verified)
- Direct run `./target/debug/deps/mp2_fuzz_survival-ade46227d74772b4 --nocapture` (binary 18:07 newer than source 17:35) → `test result: ok. 3 passed; 0 failed ... finished in 4.63s`, EXIT=0, with the live tally:
  `mp2 fuzz-lite: 178 mutants decoded (possibly short), 42 typed refusals, codes observed: {"mp2.frame.decode", "mp2.stream.cbr_mismatch", "mp2.stream.layer_mismatch"}`
  and `SKIP mp2 corpus dirty/garbage survival: corpus root absent` (the corpus-gated branch executed as designed here).
- Baseline corroborates: cargo-test-workspace.txt:1243-1248, 3/3 ok.
- Fixtures exist: tests/fixtures/mp2_48k.mp2, mp2_lsf_22k.mp2 (the mp2/mod.rs doc says 36 KB + 19 KB ≈ 55 KB — under the card's 200 KB budget claim).

## 3. Findings
F-001 [S4]
Where: mp2_fuzz_survival.rs:165-169
Code:
```
    let Some(corpus) = corpus_dir() else {
        eprintln!("SKIP mp2 corpus dirty/garbage survival: corpus root absent");
        return;
    };
```
What is wrong: with the corpus absent (this sandbox's state, and any CI without the /tmp corpus image) the pool sweep asserts nothing yet reports green — the same silent-narrowing pattern as adpcm_fuzz_survival/av1_conformance. Honest message, pass-counting shape. Confidence: HIGH — Verification: executed (the SKIP line printed in my run).

F-002 [S4]
Where: mp2_fuzz_survival.rs:105
Code:
```
            m.extend_from_slice(&pristine[k * 17.min(pristine.len())..]);
```
What is wrong: precedence — `k * (17.min(len))`, so for k=0..9 the slice starts at 0,17,...,153 and (a) drops up to 153 bytes of real audio the comment ("garbage prefixes … in front of pristine audio") implies should follow intact, and (b) PANICS if `pristine.len() < k*min(17,len)` (e.g. a 100-byte fixture, k=9 → start 153 > 100). Committed fixtures are tens of KB, so today it is latent. What correct looks like: `pristine[(k * 17).min(pristine.len())..]`. Confidence: HIGH — Verification: read (arithmetic reasoned; no panic in my run because fixtures are large).

F-003 [S4]
Where: mp2_fuzz_survival.rs:37-39
Code:
```
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
```
What is wrong: no n==0 guard (the adpcm suite's Lcg has one) — a zero-n call panics with divide-by-zero. All current call sites pass nonzero n (2, 64, m.len()≥1, 255, 65, 256, 8, 3), so unreachable today; latent-only inconsistency. Confidence: HIGH — Verification: read; every call site enumerated.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Lcg::next / below | 29-40 | REAL | S4 | F-003 |
| Outcome / classify | 43-53 | REAL | — | decode-or-typed-code; no panics on the classify path |
| mutated_fixtures_never_panic_and_settle_typed (test) | 55-138 | REAL | — | EXECUTED: 220 mutants, tally sanity (2×110), code-prefix guard (mp2./mp3. — mp3.* legal via shared header parse, documented :117-119), `refused > 0` liveness |
| the_fuzz_harness_survives_the_same_mutations (test) | 140-162 | REAL | — | EXECUTED: 120 mutants; outcome bounds asserted (frames ≤ len/17+1, contract_frames ≤ 64) — the registered harness is genuinely bounded |
| corpus_dirty_and_garbage_pools_survive (test) | 164-204 | REAL | S4 | F-001; weaker asserts than test 2 (`let _ = fuzz_mp2_stream(&data)` — panic-only); code-prefix guard on decode errors |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mutated_fixtures… | YES | any panic fails by construction; a foreign error code fails the prefix guard; `refused > 0` catches a mutation engine too weak to reach the taxonomy |
| the_fuzz_harness… | YES | bounds asserts on frames/contract_frames would catch an unbounded harness |
| corpus_dirty_and_garbage… | only when corpus present | absent → SKIP-green (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "NO panic, NO hang — every input is either decoded or refused with a typed EngineError" | :6-7 | REAL | executed: 220+120 mutants settled; tally printed |
| "Verdicts are recorded per class … never 'must fail'" | :7-9 | REAL | tally + codes observed in my run output |
| "the two mp2 fixtures ≈ 56 KB total (the card's < 200 KB budget" | :13-14 | REAL-UNPROVEN | fixtures exist; exact sizes not re-measured (mp2/mod.rs doc states 36 KB + 19 KB) |
| "the corpus is read from disk, not committed" | :14-15 | REAL | corpus absent in sandbox; SKIP branch ran |

## 7. Dead / duplicated / unreachable code
None in this file. (tests/mp2/mod.rs — the shared helper — carries its own `#![allow(dead_code)]`; another agent's assignment.)

## 8. Dependencies & flags
Test-only; no dev-deps (LCG). Depends on codecs::audio::mp2::{decode_stream, fuzz::fuzz_mp2_stream} (other agents' files) and tests/mp2/mod.rs helpers (fixture_bytes panics on absence — acceptable for committed bytes). Env: LUMEN_CORPUS or the sandbox default corpus root.

## 9. Open questions for the next agent
- codecs::audio::mp2 auditor: confirm `fuzz_mp2_stream`'s outcome bounds (frames ≤ len/17+1, contract_frames ≤ 64) are real invariants of the harness, not just test-side guesses — the test asserts them, so a harness change must keep them true.
- tests/mp2/mod.rs auditor: the `#![allow(dead_code)]` there may hide dead helpers (same pattern as dnx/mod.rs F-001).
