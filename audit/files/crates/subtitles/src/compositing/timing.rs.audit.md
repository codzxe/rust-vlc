# AUDIT: crates/subtitles/src/compositing/timing.rs
| Field | Value |
|---|---|
| Auditor | W09-A03 |
| Date | 2026-11-15 |
| Lines / functions | 87 / 1 fn + 4 tests |
| File verdict | REAL (correct, minimal, tested) — inside the UNREACHABLE island crate |
| Reachable from shipped entry point? | NO — caller chain `timing::active_at` <- `CompositedCue::active_at` (bitmap.rs:239-240) <- `SubtitleCompositor::compose` (compose.rs:79) <- in-crate tests only (island crate; see lib.rs audit) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | prebuilt `subtitles-70f89de07410626f compositing::timing` -> 4 passed / 0 failed (exit 0); Grep `active_at|timing::` (callers mapped); sed quote verification |

## 1. One-paragraph verdict
One pure function, done right: half-open `[start, end)` in integer nanoseconds with
`Option` ends, two compares, no arithmetic (so no i64 edge overflow at `i64::MIN/MAX` —
tested), zero allocation. The half-open seam property (adjacent cues never double-draw nor
gap) and the zero-duration-cue-is-never-active rule are both pinned by tests that pass
(4/4 executed). The only blemish is one UNVERIFIED doc analogy about the video scheduler.

## 2. What actually works (verified)
— verified by `./target/debug/deps/subtitles-70f89de07410626f compositing::timing` ->
`test result: ok. 4 passed; 0 failed` — nanosecond-exact boundaries, adjacent-cue seam,
zero-duration emptiness, open-ended behavior incl. `i64::MIN`/`i64::MAX`.

## 3. Findings
F-001 [S4]
Where: crates/subtitles/src/compositing/timing.rs:8-11
Code:
```
//! `start <= now < end` gives exactly that. This is the subtitle
//! convention (WebVTT/SRT intervals are end-exclusive at the swap
//! instant) and matches the video scheduler's frame-presentation
//! comparisons.
```
What is wrong: the "matches the video scheduler's frame-presentation comparisons" claim is
UNVERIFIED — video-engine has no subtitle timing code (its only subtitle mention is the
doc comment at video-engine/src/scheduler.rs:163), and I did not audit the scheduler's own
comparison operators. If the scheduler uses closed intervals anywhere, the claimed parity
is false. Confidence: LOW (claim unverified, not disproven) — Verification: read + grep.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| active_at | 32-44 | REAL | — | correct half-open rule, Option ends, no panics possible (compares only) |
| interval_is_half_open_to_the_nanosecond | 53-61 | REAL | — | 6 boundary points |
| adjacent_cues_never_double_draw_nor_gap | 63-69 | REAL | — | the seam property |
| zero_duration_cue_is_never_active | 71-75 | REAL | — | empty interval |
| open_ended_cues_stay_active | 77-86 | REAL | — | i64 extremes safe |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 4 | YES | each pins exact boundary booleans; flipping any comparison operator flips an assert |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`start <= now < end`" half-open rule | :8 | REAL | tests pass |
| "the per-frame check below is two integer compares — no float, no conversion, no allocation" | :18-20 | REAL | body is 2 compares + early returns |
| "Both missing = active ... never invent an end" | :24-27 | REAL | tested at :79-81 |
| "matches the video scheduler's frame-presentation comparisons" | :10-11 | UNVERIFIED | no subtitle code in video-engine; scheduler rule not audited (F-001) |

## 7. Dead / duplicated / unreachable code
None in-file. Reachability chain ends at in-crate tests (island crate — see lib.rs audit).

## 8. Dependencies & flags
Std-only, no deps, no unsafe, no features. `#[must_use]` with rationale — good.

## 9. Open questions for the next agent
- Confirm the video-engine scheduler's presentation comparison is also end-exclusive
  (would upgrade F-001 to a real contradiction or clear it).
