# AUDIT: crates/playback-engine/tests/property/rng.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 191 / 7 methods + 3 fns + 1 const + 1 test |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A — shared property-test harness (tests/property/), consumed by the engine_fx / queue_invariants / seek_roundtrip / timestamp_arithmetic suites. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:3 |
| Commands run | `cat` full file (exit 0). No cargo — harness run UNVERIFIED; W00 baseline: 1751/2/25 workspace-wide. |

## 1. One-paragraph verdict

A clean, honest property-test harness: deterministic xorshift with odd-seed coercion, a documented 64-seed bank (first 64 odd integers), env-gated widening, and a bisection-based prefix shrinker that is itself self-tested with a synthetic predicate (minimal failing prefix = 41 ops containing the value 40 — exact). The module doc states its own limits plainly (a sampler, not a prover; no automatic value shrinking; `run` must be a pure function of (seed, ops) or shrinking lies). Three hygiene notes: the seed bank is tiny (≤127), modulo bias in `below` (documented as accepted), and the whole harness is duplicated in the packet-queue test crate (admitted at 176-178).

## 2. What actually works (verified)

- `XorShift::new(seed|1)` prevents the zero-seed lockup; `next` is the standard xorshift* triple.
- `shrink_prefix` bisection logic hand-verified: invariant "empty prefix cannot fail" justifies searching lengths 1..=len; the loop converges to the minimal failing prefix. The self-test's arithmetic (op 40 at index 40 ⇒ 41 ops) is correct.
- `run_case`'s failure messages embed both the reproduce (`LUMEN_PROP_SEEDS=<seed>`) and shrink (`LUMEN_PROP_SHRINK=<seed>`) instructions — the rerun discipline is real.

## 3. Findings

### F-001 [S4] Seed bank is 64 fixed small odd seeds (1..127)
Where: crates/playback-engine/tests/property/rng.rs:81-89
Code (81-89, verbatim):
```rust
pub const SEED_BANK: [u64; 64] = {
    let mut seeds = [0u64; 64];
    let mut index = 0;
    while index < 64 {
        seeds[index] = (index as u64) * 2 + 1;
        index += 1;
    }
    seeds
};
```
What is wrong: nothing mechanically — documented as "deliberately small, dense and reproducible" (77-78). But the gate's default adversarial coverage is 64 seeds from a 7-bit space; the doc's own honesty ("a sampler, not an exhaustive prover", 12-13) is the correct reading. Any matrix row citing these property suites as "proven" should inherit that qualifier.
Confidence: HIGH — Verification: read.

### F-002 [S4] `below(n)` panics on n = 0 and carries modulo bias
Where: rng.rs:46-48 — `self.next() % n`. Caller contract "n ≥ 1" is a comment; a driver bug passing 0 panics in the harness (not the code under test), and the bias is documented as accepted (43-45). Hygiene.

### F-003 [S4] The harness is duplicated across test crates
Where: rng.rs:176-178 — "the same proof lives in packet-queue's property suite — the harness is duplicated across the two test crates". Verified: backpressure_property.rs:42-57 carries the same XorShift. Two copies can drift (they already differ: packet-queue's has no `u32`/`flip`/`pick`/shrink machinery). Admitted duplication; recorded.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `XorShift::new` | 29-31 | REAL | — | odd coercion |
| `XorShift::next` | 34-41 | REAL | — | |
| `XorShift::below` | 46-48 | REAL | S4 | F-002 |
| `XorShift::i64`/`u32`/`flip` | 51-68 | REAL | — | u32 rejection-samples nonzero |
| `XorShift::pick` | 71-74 | REAL | — | panics on empty slice (caller contract) |
| `SEED_BANK` | 81-89 | REAL | S4 | F-001 |
| `seeds` | 93-103 | REAL | — | env widening |
| `shrink_requested` | 107-112 | REAL | — | |
| `shrink_prefix` | 123-142 | REAL | — | correct bisection, hand-verified |
| `run_case` | 147-171 | REAL | — | purity requirement documented |
| test `shrink_prefix_bisects_to_the_minimal_failing_prefix` | 180-191 | REAL | — | exact minimal-prefix + non-failing cases |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| shrink_prefix_bisects_to_the_minimal_failing_prefix | YES (reasoned) | asserts exact minimal length 41 + last element + both degenerate cases — an off-by-one bisection fails |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a 64-seed bank … is a sampler, not an exhaustive prover" | rng.rs:12-15 | REAL | honest self-assessment, matches F-001 |
| "log₂ N reruns to a minimal failing op prefix" | rng.rs:17-20 | REAL | bisection verified + self-tested |
| "`run` must be a pure function of (seed, ops) … or shrinking lies" | rng.rs:144-146 | REAL | documented constraint on consumers |

## 7. Dead / duplicated / unreachable code

Duplication with packet-queue's XorShift (F-003) — admitted.

## 8. Dependencies & flags

No external deps (no proptest — the doc explains the Lead-only dependency policy, AGENTS.md §3.2/C5). Env knobs: `LUMEN_PROP_SEEDS`, `LUMEN_PROP_SHRINK`.

## 9. Open questions for the next agent

- Do the consumer suites (engine_fx, seek_roundtrip, timestamp_arithmetic, queue_invariants) honor the `run` purity requirement (rebuild-from-seed per invocation)? If any driver keeps cross-run state, `shrink_prefix`'s results are lies. Checked per-suite in my audits of those files.
