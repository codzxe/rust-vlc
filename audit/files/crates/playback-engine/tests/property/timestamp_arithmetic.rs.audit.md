# AUDIT: crates/playback-engine/tests/property/timestamp_arithmetic.rs
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-23 |
| Lines / functions | 623 / 19 (4 tests + 15 oracle/generator/check fns) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test-only file (module of tests/property/main.rs), but it hammers PRODUCTION surfaces: `media_types::time::{Timestamp::convert, as_nanos, Scaler, TimeBase, Rate, Stamp}` — used by shipped code at apps/cli/src/containers/{ogg,mp4,adts,wav}.rs (`convert(&NANO_BASE)`), apps/cli/src/demux_host.rs:178, crates/playback-engine/src/{pipeline.rs:2540,2592, seek.rs:320, session.rs:2818}. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo test -p playback-engine --test property timestamp_arithmetic` -> 4 passed / 0 failed (exit 0, my run); baseline audit/runs/baseline/cargo-test-workspace.txt:4299-4303 (same 4 ok); sed/rg verifications; oracle overflow-margin analysis by hand (see §2). |

## 1. One-paragraph verdict
A legitimately adversarial differential property suite: 64 fixed seeds × 512 ops (plus env-widening via LUMEN_PROP_SEEDS) of i64-edge/powers-of-two/near-MAX ticks against common, uniform, AND degenerate struct-literal bases, every answer compared to an independently authored u128 oracle implementing half-away-from-zero rational conversion, with typed-Overflow agreement checked in BOTH directions including the error payload's context fields. The oracle itself is sound — I hand-checked the extreme u128 products ((2^64−1)·(u32::MAX)² ≈ 2^128−2^97 < u128::MAX, no oracle-internal overflow) and the round-trip drift bound math (drift ≤ 0.5·den/num + 0.5, floored integer bound valid since drift is an integer). The one code defect is boilerplate: the `canonical` empty-ops regeneration branch in `run` is dead in every family suite (all callers pre-generate non-empty ops), duplicated across 5 files. One doc-level contradiction: the header claims round-trip recovery "within one tick" while the code correctly asserts the wider (num+den)/(2·num) bound. The honest limitation — 64 seeds is "a sampler, not an exhaustive prover" — is disclosed in rng.rs:12-16.

## 2. What actually works (verified)
— `cargo test -p playback-engine --test property timestamp_arithmetic` -> `test result: ok. 4 passed; 0 failed` (exit 0). Checked and verified by read: (1) oracle extreme-value margins — `ticks.unsigned_abs() * multiplier` with multiplier = (2^32−1)² stays under u128::MAX, so the oracle cannot itself wrap; (2) `rounded_i64`'s negative corner — `rounded > 1u128 << 63` rejects, `-(rounded as i128) as i64` fits exactly i64::MIN (lines 102-106); (3) the round-trip bound `(num + den) / (2 * num)` (443-446) is the correct closed form for two half-away roundings and flooring it is safe because drift is an integer; (4) the monotone+sign checks compose: a sign-flipped high end forces `low > high` and trips the monotone assert, so the single-sided sign check (481) loses nothing; (5) `Scaler`/`nanos_scaler` agreement is checked in both error directions (358-416); (6) the shrink machinery (rng.rs) has its own self-test and only activates on failure with env opt-in — it cannot silently mask a failure. Mutation reasoning: if `convert` wrapped on i64::MAX ticks, check_convert's `out.ticks != expected` fires; if it panicked on a zero-denominator struct literal, the test panics = fail; if Scaler's fast tier disagreed with the reference on one path, check_scaler fires.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/property/timestamp_arithmetic.rs:489-496
Code:
```
fn run(seed: u64, ops: &[TimeOp]) -> Result<(), String> {
    let canonical = ops.is_empty();
    let mut rng = XorShift::new(seed);
    let ops = if canonical {
        generate_ops(&mut rng, 512)
    } else {
        ops.to_vec()
    };
```
What is wrong: the `canonical` branch is dead code in practice — the only caller of `run` in this file (test at 521-530) always generates 512 ops first and passes them, so `ops.is_empty()` is never true; rg shows the same generate-then-pass shape in seek_roundtrip.rs:459, generation_monotonicity.rs:491, queue_invariants.rs:275 and packet-queue's property.rs:722, so the identical dead branch is copy-pasted in five files. Harmless (the fallback would regenerate the identical op list from the seed — deterministic), but it is untested duplication. What correct looks like: drop the branch and make callers responsible for generation, or have `run_case` own generation. Confidence: HIGH — Verification: read + rg across all five suites.
Confidence: HIGH — Verification: read / ran

F-002 [S4] (doc claim, logged here because it lives in this file's header)
Where: crates/playback-engine/tests/property/timestamp_arithmetic.rs:27-29
Code:
```
//! Conversions are monotone and sign-preserving; a
//! round-trip through an intermediate base recovers the original
//! within one tick (two half-away roundings).
```
What is wrong: the implemented and correct bound (443-447) is `(num + den) / (2·num)` ticks, which is ONE tick only when den == num (identity-ish conversions); for 90 kHz → ms it permits 45 ticks of drift. The header sentence contradicts the code's own (correct, wider) bound — the doc understates the drift the property tolerates. Not a code bug; a false comment. What correct looks like: state the real bound. Confidence: HIGH — Verification: read + hand-derivation of the bound.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| oracle_convert | 55-69 | REAL | — | independent u128 rational oracle; zero-divisor None, zero-multiplier Some(0) per contract |
| oracle_nanos | 76-95 | REAL | — | negative-rounds-to-zero is Ok(0); negative nonzero and >u64 are None |
| rounded_i64 | 99-113 | REAL | — | half-away rounding; −2^63 corner representable |
| nanos_base | 118-120 | REAL | — | tripwire for the crate's NANOS_BASE payload |
| raw_base | 130-149 | REAL | — | common rates + explicit degenerate shapes + uniform nonzero pairs |
| adversarial_ticks | 153-178 | REAL | — | i64 edges, near-MAX clusters, powers of two, full range |
| build_base | 183-192 | REAL | — | struct literal for degenerates (the documented escape hatch) |
| TimeOp (enum) | 201-227 | REAL | — | 5 op shapes with recorded inputs |
| generate_ops | 229-266 | REAL | — | uniform 5-way mix; monotone pair swap |
| check_convert | 268-309 | REAL | — | value + error-payload agreement, output base checked |
| check_nanos | 311-349 | REAL | — | same discipline for as_nanos |
| check_scaler | 351-418 | REAL | — | 3-way agreement (scaler_to, nanos_scaler, one-shots), both error directions |
| check_roundtrip | 420-454 | REAL | — | overflow legs skip legally; drift bound verified by hand |
| check_monotone | 456-487 | REAL | — | monotone + sign preservation (composition covers the high end) |
| run | 489-515 | REAL | S4 | dead canonical branch (F-001) |
| conversions_answer_typed_errors_or_exact_values_never_panics | 521-530 | REAL | — | 64 seeds × 512 ops; ran ok |
| base_construction_is_typed_and_normalizing | 539-569 | REAL | — | 512 random pairs construct; zeros refuse typed; 2/180000 ≡ 1/90000 |
| stamp_absence_is_typed_never_fabricated | 574-591 | REAL | — | typed Missing both span ends |
| negative_offsets_are_legal_ticks | 597-623 | REAL | — | negative-DTS shape, oracle agreement over 256 draws |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| conversions_answer_typed_errors_or_exact_values_never_panics | YES | wrap → value mismatch; silent wrong rounding → oracle mismatch; panic → test fails; wrong error payload → context assert |
| base_construction_is_typed_and_normalizing | YES | constructor accepting zero or failing to normalize fails the asserts |
| stamp_absence_is_typed_never_fabricated | YES | any sentinel/fabrication breaks the typed-Missing equality |
| negative_offsets_are_legal_ticks | YES | oracle agreement incl. overflow agreement on both sides |
| (harness) run_case/shrink_prefix | YES | self-tested in rng.rs:179-191 (not this file) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "independently authored u128 oracle ... no shared code with the crate" | timestamp_arithmetic.rs:39-40 | REAL | oracle fns use only u128 arithmetic + media_types constructors for inputs; different implementation from any crate path (read) |
| "wrap-around must be impossible: overflow is a typed error" | timestamp_arithmetic.rs:11-12 | REAL (tested) | near-i64::MAX clusters generated (169); my run ok; sign-preservation property checked |
| "a non-zero rounded negative value is an error" (as_nanos subtlety) | timestamp_arithmetic.rs:19-21 | REAL | oracle_nanos 88-90 + property runs |
| "round-trip ... recovers the original within one tick" | timestamp_arithmetic.rs:27-29 | FALSE (as stated) | code asserts the wider (num+den)/(2·num) bound (443-447); for 90 kHz→ms that is 45 ticks, not 1 (F-002) |
| "64-seed bank ... a sampler, not an exhaustive prover" | rng.rs:12-16 (harness) | REAL | SEED_BANK = first 64 odd ints (rng.rs:81-89); honest disclosure |

## 7. Dead / duplicated / unreachable code
- `run`'s canonical branch (490-496) — dead in this suite and duplicated across 4 sibling suites + packet-queue (F-001).
- No #[ignore], no todo!/unimplemented! (rg verified).

## 8. Dependencies & flags
media_types + testkit only, plus the in-tree rng.rs harness (XorShift, 64-seed bank, bisection shrink). No proptest (Lead-only dependency rule, documented at rng.rs:4-9). Env knobs LUMEN_PROP_SEEDS / LUMEN_PROP_SHRINK are triage-only, never required for the gate.

## 9. Open questions for the next agent
- The header's "within one tick" round-trip claim vs the code's (num+den)/(2·num) bound — which is the intended contract? If one tick is the real requirement the test under-asserts; if the wider bound is right the header is false. The crate's own doc for `convert` should adjudicate (media-types docs = another agent).
- The exhaustive "104-pair model check" referenced in rng.rs:13-15 — confirm it exists and covers the degenerate struct-literal bases (likely media-types' own suite or tests/property/model files).
