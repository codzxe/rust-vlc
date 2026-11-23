# AUDIT: crates/playback-engine/tests/property/rng.rs
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2025-06-14 |
| Lines / functions | 191 / 9 (XorShift + 5 methods, SEED_BANK, seeds, shrink_requested, shrink_prefix, run_case + 1 self-test) |
| File verdict | REAL — a deterministic hand-rolled property harness (proptest substitute, honestly motivated), with a self-tested bisection shrinker; the suite it powers executes green (15 tests, direct binary run) |
| Reachable from shipped entry point? | N/A (test infrastructure). The 64-seed bank × thousands-of-ops properties it drives cover shipped engine code (seek/timestamps/queue invariants/generations). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `./target/debug/deps/property-25b39d1917495830` -> 15 passed (exit 0); same binary filtered `rng` -> 1 passed (the shrink self-test); three sibling prebuilt property binaries -> 15/4/15/4 passed |

## 1. One-paragraph verdict
Honest, minimal property-test infrastructure: a seeded xorshift generator with the zero-seed lockup handled (`seed | 1`), a fixed 64-seed bank (first 64 odd integers) with env widening for triage, and a documented bisection shrinker that replaces proptest's automatic shrinking — the trade ("a sampler, not an exhaustive prover") is spelled out up front. The shrink machinery is self-tested with a synthetic predicate, and the family suites using it all pass (direct binary runs). Three wart-level notes: the `seed | 1` coercion silently maps even seeds onto the next odd seed's stream, `below(0)`/`pick([])` panic on a documented-but-unenforced caller contract, and the shrinker's "minimal in prefix length" claim holds only for prefix-monotone failure predicates (a later state-fixing op can defeat the bisection) — partially acknowledged in the docs.

## 2. What actually works (verified)
- verified by `./target/debug/deps/property-25b39d1917495830 rng` -> `1 passed` (shrink_prefix_bisects_to_the_minimal_failing_prefix ok); full binary -> 15 passed (exit 0) — the whole property family this harness powers is green.
- Hand-checked xorshift64 step (x^=x<<13; x^=x>>7; x^=x<<17) and the bisection: lo=1, hi=len, `still_fails([..mid]) ? hi=mid : lo=mid+1` — standard minimal-boundary search for monotone predicates; self-test asserts the 41-op minimal prefix.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/property/rng.rs:28-31, 98
Code:
```
    /// Seed the generator. Odd seeds only (a zero seed locks up).
    pub fn new(seed: u64) -> XorShift {
        XorShift(seed | 1)
    }
```
What is wrong: `seed | 1` silently coerces even seeds to the NEXT odd seed's stream — seeds 4 and 5 generate identical sequences. An operator triaging a failure with `LUMEN_PROP_SEEDS=4` (also `| 1`'d at line 98) unknowingly reruns seed 5. The bank itself is odd-only so no intra-bank collision; the coercion is just undocumented in effect.
Confidence: HIGH — Verification: read + arithmetic.
F-002 [S4]
Where: crates/playback-engine/tests/property/rng.rs:46-48, 71-74
Code:
```
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
```
What is wrong: `n == 0` divides by zero (panic); `pick` on an empty slice reaches the same panic. Both are "caller contract" per the doc comments but unenforced — a family suite bug becomes a panic instead of a test failure message. Also `shrink_requested` (107-112) silently disables shrinking when `LUMEN_PROP_SHRINK` holds garbage (parse Err ≠ Ok(seed) → false, no complaint).
Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/playback-engine/tests/property/rng.rs:118-122
Code:
```
/// reruns any prefix (the case is rebuilt from the seed alone, so
/// every rerun is deterministic), return the minimal-length failing
/// prefix. log₂ N reruns; the result is minimal in prefix length
```
What is wrong: minimality holds only if failure is prefix-monotone (every extension of a failing prefix also fails). A predicate whose later ops repair earlier damage (e.g., a seek-back that restores a violated invariant) is non-monotone; bisection then returns a non-minimal prefix while the doc asserts minimality. The op-parameter caveat is acknowledged; the monotonicity assumption is not.
Confidence: HIGH — Verification: read + bisection logic checked.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| XorShift::new | 28-31 | REAL | S4 | F-001 even/odd collapse |
| XorShift::next | 34-41 | REAL | — | xorshift64, deterministic |
| XorShift::below | 46-48 | REAL | S4 | F-002 n=0 panic; modulo bias documented as accepted |
| XorShift::i64 / u32 / flip | 51-68 | REAL | — | u32 rejects zero words (loop) |
| XorShift::pick | 71-74 | REAL | S4 | F-002 empty-slice panic |
| SEED_BANK | 81-89 | REAL | — | const-eval first 64 odds; fixed and reproducible |
| seeds | 93-103 | REAL | — | env widening, `| 1` coercion |
| shrink_requested | 107-112 | REAL | S4 | F-002 garbage silently disables |
| shrink_prefix | 123-142 | REAL | S4 | F-003 monotonicity assumption; correct bisection otherwise |
| run_case | 147-171 | REAL | — | rerun/shrink env protocol with actionable failure text |
| shrink_prefix_bisects... (test) | 179-191 | REAL | — | self-test with monotone predicate + empty/non-failing guards |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| shrink_prefix_bisects_to_the_minimal_failing_prefix | YES | exact minimal length (41) + last-op assertions; empty/non-failing cases |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "proptest is not a workspace dependency ... every property loop here is a deterministic seeded loop" | rng.rs:4-10 | REAL | no proptest in Cargo.toml (denied as Lead-only); the suites use this harness |
| "a seed printed by a failure is its own rerun instruction" | rng.rs:78-80 | REAL | run_case failure text embeds LUMEN_PROP_SEEDS=<seed> / LUMEN_PROP_SHRINK=<seed> |
| "the result is minimal in prefix length" | rng.rs:120-122 | PARTIAL | holds for monotone predicates only (F-003) |

## 7. Dead / duplicated / unreachable code
- Documented duplication: "the same proof lives in packet-queue's property suite — the harness is duplicated across the two test crates" (line 176-178) — copy-paste across test crates, acknowledged.

## 8. Dependencies & flags
- std only. Env knobs: LUMEN_PROP_SEEDS (widen), LUMEN_PROP_SHRINK (shrink one seed). No features.

## 9. Open questions for the next agent
- The family suites (engine_fx, generation_monotonicity, queue_invariants, seek_roundtrip, timestamp_arithmetic) are separate files not in my assignment — their `run` closures must be pure functions of (seed, ops) or shrinking lies (the doc's own warning, line 145-146); each suite's auditor should verify rebuild-from-seed purity.
