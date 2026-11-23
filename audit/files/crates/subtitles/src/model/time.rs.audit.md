# AUDIT: crates/subtitles/src/model/time.rs
| Field | Value |
|---|---|
| Auditor | W09-A07 |
| Date | 2026-11-23 |
| Lines / functions | 236 / 17 (11 methods incl. const EMPTY, 6 tests) |
| File verdict | REAL-UNPROVEN in product (UNREACHABLE from shipped entry points — crate unlinked, see parser.rs F-001); the module itself is clean, correct, and executed |
| Reachable from shipped entry point? | NO — used in-crate by srt parser/model/compositing only; no app/engine links the crate |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | cargo test -p subtitles --lib model::time (exit 0, 6 passed); sed quote verification (exit 0) |

## 1. One-paragraph verdict
The cleanest file in my batch: a half-open i64-µs interval type with a single validity gate
(`new` rejects `end < start`), saturating-only arithmetic, and correct interval algebra. I
hand-verified the overlap/touch/intersection/hull math against a dozen cases (empty intervals,
touching pairs, containment, i64 extremes) and found no defect; the in-crate tests assert the
same edges and pass (I ran them: 6/6). The only blemish is a doc-comment typo. The module is
unreachable from shipped entry points solely because the crate itself is wired to nothing —
that defect is filed against the crate wiring (parser.rs F-001), not this file.

## 2. What actually works (verified)
— `cargo test -p subtitles --lib model::time` → `test result: ok. 6 passed; 0 failed`
  (exit 0): zero_duration_is_legal_and_empty, end_before_start_is_typed_invalid,
  touching_is_not_overlap, overlap_windows, saturating_extremes, contains_is_half_open.
— Hand-verified algebra (read, no execution needed): `overlaps` = max(starts) < min(ends) —
  empty intervals can never satisfy it; `touches` = ¬overlaps ∧ boundary equality; `hull`
  min/max always satisfies the invariant; `shift_us` saturating-add is monotone so
  `start <= end` survives saturation at both ends (collapse-to-empty at MAX confirmed by the
  test at lines 219-223).

## 3. Findings
F-001 [S4]
Where: crates/subtitles/src/model/time.rs:113-117
Code: (verbatim)
```
/// The intersection `self ∩ other` as a half-open interval.
///
/// `None` when the interiors do not meet — **disjoint *and*
/// touching** both yield `None** (a touching intersection is the
```
What is wrong: markdown typo — the closing fence is `None**` instead of `` `None` ``; renders
as literal `None**`. Cosmetic only; the described semantics (disjoint and touching both yield
None) are correct and test-pinned (tests:190). Confidence: HIGH — Verification: read.
Zero substantive findings — what was checked and how:
- Constructor gate `end < start` → typed Err (read + test end_before_start_is_typed_invalid).
- `contains` half-open at both bounds (read + test contains_is_half_open: 99/100/199/200).
- `overlaps`/`touches`/`intersection` algebra incl. empty intervals (hand-checked 12 cases:
  identical, touching both orders, nested, disjoint, empty-vs-spanning, empty-at-boundary,
  empty-vs-itself).
- `duration_us`/`shift_us` saturation at (MIN,MAX), (MAX-5,MAX)+MAX, MIN+5..MIN+10 shifted by
  MIN (hand-traced monotonicity; saturating_extremes test covers the first two).
- `intersection` constructs the struct directly (line 122-125) — safe because lo < hi is
  checked; `hull` is total by min/max.
- No `as` casts, no panics, no unwraps, no unsafe, no allocation in the module.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CueInterval::EMPTY | 38-41 | REAL | — | const |
| CueInterval::new | 48-53 | REAL | — | the single validity gate |
| start_us / end_us | 56-63 | REAL | — | const getters |
| duration_us | 67-69 | REAL | — | saturating_sub |
| is_empty | 73-75 | REAL | — | |
| contains | 79-81 | REAL | — | half-open, const |
| overlaps | 87-102 | REAL | — | const, branch-wise min/max (documented toolchain reason) |
| touches | 108-110 | REAL | — | documented empty-interval semantics |
| intersection | 118-129 | REAL | F-001 (doc typo only) | direct construction behind lo<hi check |
| hull | 133-138 | REAL | — | total |
| shift_us | 145-150 | REAL | — | saturating, invariant-preserving |
| tests ×6 | 159-235 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| zero_duration_is_legal_and_empty | YES | empty semantics pinned incl. !overlaps with itself/spanning |
| end_before_start_is_typed_invalid | YES | exact Err match |
| touching_is_not_overlap | YES | both orders + intersection None + hull |
| overlap_windows | YES | partial overlap, nesting, disjoint, exact intersection value |
| saturating_extremes | YES | (MIN,MAX) duration, shift saturation, collapse-to-empty |
| contains_is_half_open | YES | all four boundary probes |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "All arithmetic is saturating — shift_us never wraps and never breaks start <= end" | lines 17-21 | REAL | read (saturating_add/sub only) + saturating_extremes test |
| "duration_us at (MIN, MAX) saturates to MAX rather than overflowing" | lines 19-20 | REAL | test line 211 asserts i64::MAX |
| "Touching is not overlap" | lines 2-3, 83-86 | REAL | touching_is_not_overlap test |
| "±292 k years" range | line 7 | REAL | i64 µs ≈ 2.9e5 years — arithmetic checks out |
| "[PROPOSAL: the Lead unifies the exact tick type …]" | lines 11-13 | REAL (admitted open decision) | no unification landed; µs here vs ns in compositing — the ×1000 seam is real and exercised by srt/render |

## 7. Dead / duplicated / unreachable code
None in-file. Module is used by srt/parser.rs:252, model/wire.rs (in-crate). Unreachable from
shipped apps only via the crate-level wiring gap (parser.rs F-001).

## 8. Dependencies & flags
crate::model::error::ModelError only. Std-only, no unsafe, no features. Const-fn heavy
(const interval algebra) — consistent with its "no allocation" claim.

## 9. Open questions for the next agent
- The µs (model) vs ns (compositing/media-types) unit seam is bridged where? model/wire.rs is
  the likely ×1000 site — its auditor should check for truncation/rounding at the conversion.
