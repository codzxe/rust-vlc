# AUDIT: crates/playback-engine/tests/property/main.rs
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-23 |
| Lines / functions | 41 / 0 (module docs + 6 `mod` declarations; the harness root) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test-harness root) — its binary runs in the workspace lane: baseline cargo-test-workspace.txt:4286-4305, 15 tests, all ok |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. One-paragraph verdict
The harness root does exactly what its docs promise and nothing else: it
declares the four property families plus the shared rng/engine_fx helpers,
documents the no-proptest discipline honestly (a "sampler, not an exhaustive
prover" scope note with the exhaustive proofs' locations named), and every
claim it makes is independently verifiable in-tree — the seed bank, the
env-widening knob, the bisection shrink, and the committed-regression policy
all exist and behave as described (rng.rs:81-169; a committed fixed-value
regression anchor exists in seek_roundtrip.rs:466). All 15 tests across the
families are green in the baseline. No defects.

## 2. What actually works (verified)
- Baseline (:4286-4305): `Running tests/property/main.rs ... 15 passed; 0
  failed` — queue_invariants (3), generation_monotonicity (1), seek_roundtrip
  (6), timestamp_arithmetic (4), rng (1: shrink_prefix_bisects_to_the_minimal_
  failing_prefix).
- Module inventory: all six declared modules exist on disk
  (tests/property/{engine_fx,generation_monotonicity,queue_invariants,rng,
  seek_roundtrip,timestamp_arithmetic}.rs) — no phantom declarations.
- Doc-claim cross-verification:
  - "the exhaustive proofs (104-pair model check, bit-exactness proofs inside
    media-types) live in their own suites" — the 104-pair model check is real
    (my expected_table.toml audit; model_check.rs 5/5 green in baseline).
  - "LUMEN_PROP_SHRINK=<seed> ... bisection on the failing op sequence" —
    rng.rs:123-142 `shrink_prefix`, unit-tested by rng's own baseline test.
  - "Shrunk counterexamples become committed regressions with fixed values" —
    seek_roundtrip.rs:466 carries committed fixed-value boundary anchors.
  - "no proptest — Lead-only dependency approval, C5" — packet-queue and
    media-types Cargo.tomls confirm the no-external-deps discipline.

## 3. Findings
None. Exactly what was checked, and how: every doc sentence in the file was
traced to its implementor (list above); the module set was diffed against the
directory; the suite's 15 tests were located in the baseline log; no
fakeness markers, no dead modules (engine_fx is the shared fixture module —
helper-only by design, consumed by the families).

## 4. Per-function table
Not applicable (41 lines; no functions). Structure: module docs (1-36), six
`mod` declarations (38-41... precisely: engine_fx, generation_monotonicity,
queue_invariants, rng, seek_roundtrip, timestamp_arithmetic).

## 5. Test honesty
| Test (via this harness) | Would it fail if code were broken? | Reason |
|---|---|---|
| all 15 family tests | YES | per my queue_invariants.rs audit and the family names (typed-refusal, clamp-then-end, supersession silence, stamp-absence typing) — assert-rich, seed-addressable failures |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "black-box, seeded, deterministic property loops over the public APIs" | main.rs:2-3 | REAL | families use Engine/commands/Packet public API only (queue_invariants verified in depth) |
| "one file per invariant family" | main.rs:4 | REAL | 4 family files + 2 shared helpers |
| "64 seeds; LUMEN_PROP_SEEDS widens; LUMEN_PROP_SHRINK shrinks via bisection" | main.rs:26-31 | REAL | rng.rs:81-169 |
| "Shrunk counterexamples become committed regressions with fixed values" | main.rs:33-35 | REAL | seek_roundtrip.rs:466 |
| "a 64-seed bank ... is a sampler, not an exhaustive prover" (scope note, rng.rs:9-11) | harness docs | REAL (honest limitation) | read |

## 7. Dead / duplicated / unreachable code
None — engine_fx is consumed by the families; every declared module is
compiled into the test binary.

## 8. Dependencies & flags
No dependencies of its own; the harness modules use playback_engine,
packet-queue, media-types public APIs + testkit. Env knobs:
LUMEN_PROP_SEEDS, LUMEN_PROP_SHRINK (documented, default-off).

## 9. Open questions for the next agent
- generation_monotonicity has a single test for a whole family — is its
  internal coverage commensurate with the other families (6 and 4 tests), or
  is that family under-served? (Its file is another agent's assignment.)
- timestamp_arithmetic's four tests cover the media-types time API — if
  media-types' own suites already prove bit-exactness (per the scope note),
  is this family partially redundant, or the engine-side black-box twin?
