# AUDIT: crates/transcode/src/limits.rs
| Field | Value |
|---|---|
| Auditor | W10-A07 |
| Date | 2026-11-27 |
| Lines / functions | 264 / 11 (incl. 4 tests) |
| File verdict | REAL — but unreachable (crate orphaned) |
| Reachable from shipped entry point? | NO — transcode crate has zero dependent manifests (see runner.rs audit F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg (dependents, baseline tests) exit 0; sed -n verifications exit 0; baseline citation |

## 1. One-paragraph verdict
A small, correct, self-validating resource envelope: five axes (queue slots/bytes,
event slots, tracks, filter rows), zero refused typed, over-ceiling refused typed with
first-offender-wins ordering, defaults matching the documented posture table exactly,
and — verified by cross-reading the consumers — every "enforced at" claim in the
module-doc table is true (queue axes → PacketStageQueue::new at runner.rs:140-144,
event slots → JobShared::new at runner.rs:145, tracks → runner.rs:359-365, filter rows
→ graph.rs:318-326). The only defect is the inherited one: none of it is reachable
from any shipped binary.

## 2. What actually works (verified)
— verified by baseline cargo-test-workspace.txt (PACKAGE transcode, unittests):
limits::tests 4/4 `ok` (defaults_are_the_documented_posture, exact_ceilings_are_legal,
over_ceiling_fields_are_refused_typed_first_offender, zero_fields_are_refused_typed).
— verified by cross-read: runner.rs:140-145 consumes queue_slots()/queue_bytes()/
event_slots(); runner.rs:359-365 enforces max_tracks(); graph.rs:318-326 enforces
max_filter_stages() (read during the runner.rs audit).
— doc-table arithmetic checked by hand: defaults 64/1 MiB/32/64/4 and ceilings
1024/16 MiB/1024/4096/16 all match the constants at limits.rs:52-76.

## 3. Findings
F-001 [S3]
Where: crates/transcode/src/limits.rs:1-3 (crate scope)
Code:
```
//! The job resource envelope (card 7.E1): every bound the transcode
//! runner enforces, in one self-validating type (MP §12 — bounded
```
What is wrong: correct, tested code with no consumer — the transcode crate is absent
from every shipped app manifest (proof in runner.rs audit F-001). The envelope
bounds a pipeline nothing ships. Confidence: HIGH — Verification: read (manifest scan).
No other findings: validation order, typed errors, ceiling arithmetic, and the
doc table were all checked and match.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| JobLimits::new | 84-113 | REAL | — | zero→limit_zero, over→limit_over_budget, declaration order |
| queue_slots / queue_bytes / event_slots / max_tracks / max_filter_stages | 117-148 | REAL | — | #[must_use] accessors |
| Default | 151-162 | REAL | — | in-construction constant (cannot fail) |
| 4 tests | 169-263 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| defaults_are_the_documented_posture | YES | pins all five defaults to the table |
| zero_fields_are_refused_typed | YES | all five zero axes, code + stage + kind |
| over_ceiling_fields_are_refused_typed_first_offender | YES | first-offender order (slots before bytes), each axis alone, budget/attempted values |
| exact_ceilings_are_legal | YES | boundary inclusive |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| bounds table (defaults + ceilings + enforcement sites) | limits.rs:7-13 | REAL | constants L52-76 + consumers cross-read (runner.rs:140-145,359-365; graph.rs:318-326) |
| "Built only through JobLimits::new, which refuses zero values and values above the hard ceilings" | limits.rs:35-38 | REAL | new:98-105 + tests |
| "progress accounting is saturating u64 counters" | limits.rs:19-21 | REAL (sibling) | runner.rs:1111 saturating_sub; job.rs absorb |

## 7. Dead / duplicated / unreachable code
Unreachable crate (F-001). Note: other crates (e.g. the one owning baseline
`limits::tests::binding_axis...` at L3832-3837) have their own distinct limits.rs —
name collision across crates, no sharing; acceptable.

## 8. Dependencies & flags
crate::error (limit_zero/limit_over_budget) + media-types. std-only, no features,
no unsafe, no env reads.

## 9. Open questions for the next agent
None specific — the file does exactly what it says. Inherit the crate-wiring question
from the runner.rs audit (§9).
