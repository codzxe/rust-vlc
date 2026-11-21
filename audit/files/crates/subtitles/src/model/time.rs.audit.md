# AUDIT: crates/subtitles/src/model/time.rs
| Field | Value |
|---|---|
| Auditor | W09-A09 |
| Date | 2026-11-21 |
| Lines / functions | 236 / 14 (11 API items + 6 tests) |
| File verdict | UNREACHABLE — correct, saturated math, tests baseline-executed green; 8 of 11 methods production-dead |
| Reachable from shipped entry point? | NO (crate unlinked; cross-ref layout.rs F-001 census) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | `rg "CueInterval|\.touches\(|\.hull\(|\.intersection\(|duration_us|shift_us"` (caller census across crate); `rg "\.interval\.|active_at"` (production consumers); read of model/cue.rs:80-93, compositing/timing.rs:32, compositing/bitmap.rs:239-241; quote verifies via Read tool; baseline: orchrun-test-workspace-nofailfast.log.txt:5430-5435 (all 6 time tests ok), :5540 (lib 267/0) |

## 1. One-paragraph verdict
Textbook interval arithmetic: half-open `[start,end)` µs with a single validity gate, saturating math proven at the i64 extremes, and six mutation-sensitive tests covering the empty/touching/overlap/saturation corners — all six executed green in the baseline no-fail-fast run (orchrun-test-workspace-nofailfast.log.txt:5430-5435, lib 267/0 at :5540). The correctness is not in question. What is in question is why three quarters of the API exists: only `new`, `EMPTY`, and the two accessors have production callers (parsers + wire serialization); `overlaps/touches/intersection/hull/shift_us/duration_us/contains/is_empty` are called by nothing outside their own tests, because the compositing half re-implements the identical half-open rule in nanoseconds (`compositing::timing::active_at`, `CompositedCue::{start,end}`) instead of consuming this type. And none of it ships in any binary.

## 2. What actually works (verified)
— `duration_us` at `(i64::MIN, i64::MAX)`: `MAX.saturating_sub(MIN)` → `i64::MAX` (plain sub would overflow) — verified by read; pinned by test `saturating_extremes` (:209-225).
— `overlaps` as `max(starts) < min(ends)` (:91-101): correct half-open semantics; empty interval `[t,t)` vs `[t,x)` → `t < t` false → no overlap ✓; touching `[0,10)`/`[10,20)` → `10 < 10` false ✓ (test :183-193).
— `shift_us` monotonicity: saturating add preserves `start <= end`; `CueInterval::new(i64::MAX-5, i64::MAX).shift_us(i64::MAX)` → empty-at-MAX, pinned by test (:219-223).
— Production callers verified: `CueInterval::new` — ass/parser.rs:621, vtt/mod.rs:222, srt/parser.rs:252, model/wire.rs:512; `EMPTY` — srt/parser.rs:252, model/wire.rs:514; accessors — model/wire.rs:119-120.
— Executed in baseline: all 6 tests green (orchrun-test-workspace-nofailfast.log.txt:5430-5435).

## 3. Findings

### F-001 [S2]
Where: crates/subtitles/src/model/time.rs:31-151 (whole type)
Code:
```
pub struct CueInterval {
    start_us: i64,
    end_us: i64,
}
```
What is wrong: Crate-level unreachability (cross-ref layout.rs F-001: only fuzz/Cargo.toml:50 depends on `subtitles`; no shipped binary links it). Every parser in the crate builds `CueInterval`s that no shipped code path can ever read.
Why it matters: The type is correct and tested, but its entire production audience is zero.
What correct looks like: Pipeline wiring (sidecar subtitles → compositing) or honest UNREACHABLE status.
Confidence: HIGH — Verification: rg census + baseline engine-info.

### F-002 [S3] — CORRECTED v2 (was S2 "tests never executed in any baseline run")
Where: crates/subtitles/src/model/time.rs:153-236 (test module) + audit/runs/baseline/INDEX.md
Code:
```
#[cfg(test)]
mod tests {
    use super::CueInterval;
    use crate::model::error::ModelError;
```
What is wrong: The v1 claim "these 6 tests have no execution evidence in audit/runs/baseline/" was FALSE: `orchrun-test-workspace-nofailfast.log.txt:5430-5435` shows all six green (`zero_duration_is_legal_and_empty` … `contains_is_half_open`), inside the lib's 267/0 at :5540. The actual defect is the evidence INDEX: INDEX.md's table cites only the two fail-fast workspace runs, both stopping at packages alphabetically before `subtitles`, and lists none of the 10 orchrun-* artifacts in its own directory — violating the "INDEX.md lists everything" contract the audit instructions rely on (cross-ref layout.rs F-002 for the full chain; this auditor initially drew the false conclusion in five files and corrects them all here).
Why it matters: A load-bearing index that omits the only full-coverage artifact manufactures false "no evidence" findings across every wave.
What correct looks like: INDEX.md rows for the orchrun-* artifacts; --no-fail-fast in the headline gate runs.
Confidence: HIGH — Verification: read of all three baseline logs.

### F-003 [S3]
Where: crates/subtitles/src/model/time.rs:67-150
Code:
```
    /// True when the two intervals share interior time. **Touching is
    /// not overlap**: `[0,10)` and `[10,20)` do not overlap. Empty
    /// intervals never overlap anything (their intersection is the
    /// empty set — including with an interval that spans them).
    pub const fn overlaps(&self, other: &Self) -> bool {
```
What is wrong: `duration_us`, `is_empty`, `contains`, `overlaps`, `touches`, `intersection`, `hull`, `shift_us` — 8 of the 11 public methods — have ZERO production callers (rg census: hits only in this file's tests). The live half-open timing rule in the crate is a PARALLEL implementation: `compositing::timing::active_at(start: Option<i64>, end: Option<i64>, now_ns)` (timing.rs:32) over `CompositedCue`'s pre-converted nanosecond `Option<i64>` fields (bitmap.rs:198-203, 239-241). Two vocabularies for the same concept (model µs/i64 vs compositing ns/Option), zero reuse.
Why it matters: Drift risk — a semantics fix (e.g. open-ended handling) must be made twice; the crate description's "sample-exact [t0,t1) cue timing against the shared clock" is delivered by the compositing copy, not this type; 100+ lines of dead algebra inflate the "REAL" surface.
What correct looks like: `CompositedCue` holds a `CueInterval`-shaped span (with open ends) delegating to one implementation, or the dead methods are deleted until a consumer lands.
Confidence: HIGH — Verification: rg census (`\.touches\(|\.hull\(|\.intersection\(|duration_us|shift_us` → time.rs only).

### F-004 [S4]
Where: crates/subtitles/src/model/time.rs:11-13 and :115
Code:
```
//! negative lead-in timestamps (e.g. ASS `0:00:00.00` clamps, Matroska
//! blocks before the cluster timestamp). [PROPOSAL: the Lead unifies
//! the exact tick type with 5.L1's parser outputs and `media-types`
```
What is wrong: Two doc defects: (a) the `[PROPOSAL]` tick-type unification is still open — the tree carries THREE time vocabularies this file touches (model µs `CueInterval`, compositing ns `Option<i64>`, `media_types::Timestamp` bases), exactly the fragmentation the proposal was meant to close; (b) :115 has a broken inline-code span: `` `None** (`` — unbalanced backticks in the `intersection` doc.
Why it matters: Doc rot on the crate's most fundamental type; the proposal marker shows an acknowledged design debt that no wave closed.
What correct looks like: Resolve or re-date the proposal; fix the backtick pair.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `CueInterval` (struct) | 31-34 | REAL | — | private fields; invariant only via `new` |
| `EMPTY` | 38-41 | REAL | — | used: srt/parser.rs:252, model/wire.rs:514 |
| `new` | 48-53 | REAL | — | single validity gate; 4 production callers (§2) |
| `start_us` | 56-58 | REAL | — | used: model/wire.rs:119 + tests |
| `end_us` | 61-63 | REAL | — | used: model/wire.rs:120 + tests |
| `duration_us` | 67-69 | REAL-DEAD | S3 | saturating_sub correct; no production caller |
| `is_empty` | 73-75 | REAL-DEAD | S3 | tests only |
| `contains` | 79-81 | REAL-DEAD | S3 | half-open ✓; tests only |
| `overlaps` | 87-102 | REAL-DEAD | S3 | correct lo<hi; tests only; F-003 anchor |
| `touches` | 108-110 | REAL-DEAD | S3 | tests only; empty-touches-boundary semantics documented |
| `intersection` | 118-129 | REAL-DEAD | S3 | touching → None (reserved-zero-length rule) ✓; tests only |
| `hull` | 133-138 | REAL-DEAD | S3 | unchecked hull valid ✓; tests only |
| `shift_us` | 145-150 | REAL-DEAD | S3 | saturating, monotone ✓; tests only |
| tests ×6 | 159-235 | REAL | S3 | all mutation-sensitive (see §5); executed green in baseline orchrun log :5430-5435 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| zero_duration_is_legal_and_empty | YES | pins !contains(bound), !overlaps(self), !overlaps(spanning) — kills any closed-interval mutation |
| end_before_start_is_typed_invalid | YES | exact Err equality |
| touching_is_not_overlap | YES | pins overlaps=false + touches both ways + intersection None + hull exact — kills `<=` mutations |
| overlap_windows | YES | positive/negative overlap + exact intersection value |
| saturating_extremes | YES | pins duration=MAX at (MIN,MAX), shift collapse to empty-at-MAX — kills plain add/sub mutations |
| contains_is_half_open | YES | 99/100/199/200 quadrant |

All six would fail under concrete mutations. All six executed green in the baseline no-fail-fast run (orchrun-test-workspace-nofailfast.log.txt:5430-5435, lib 267/0 at :5540).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "sample-exact [t0,t1) cue timing against the shared clock" | crates/subtitles/Cargo.toml description | SHALLOW | delivered by compositing/timing.rs + CompositedCue ns fields, NOT by this file's type (F-003); this file supplies validity + storage only |
| "zero-duration legal, end < start typed-invalid, touching ≠ overlap" | module doc :1-3 | REAL | :48-53, :87-110 verified; test-pinned |
| "±292 k years… saturating… never breaks the invariant" | module doc :7-21 | REAL | saturating math verified at extremes (§2) |
| card 5.L2 AC (timing) per lib.rs:59 | crates/subtitles/src/lib.rs:59 | REAL | AC tests exist here and ran green in baseline (orchrun :5430-5435) |

## 7. Dead / duplicated / unreachable code
- 8 dead methods (F-003). Whole type unreachable from shipped builds (F-001).
- Duplication: half-open rule implemented twice (this file µs; compositing/timing.rs ns).

## 8. Dependencies & flags
- Only `crate::model::error::ModelError`. No features, no unsafe, no panics, const fns throughout (one non-const `intersection` — cosmetic inconsistency). Zero allocations.

## 9. Open questions for the next agent
- Whoever audits compositing/timing.rs (W09-A07): confirm whether `active_at` and `CueInterval::overlaps/contains` have drifted or are semantics-identical today (my read says identical: both half-open, both exclude the end bound, both treat zero-span as containing nothing).
