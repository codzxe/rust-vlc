# AUDIT: crates/subtitles/src/compositing/timing.rs
| Field | Value |
|---|---|
| Auditor | W09-A07 |
| Date | 2026-11-21 |
| Lines / functions | 87 (44 production + 43 tests) / 1 production function + 4 tests |
| File verdict | UNREACHABLE (correct, boundary-tested predicate; zero shipped-entry-point callers) |
| Reachable from shipped entry point? | **NO.** Call chain inside the crate only: `timing::active_at` ← `CompositedCue::active_at` (`compositing/bitmap.rs:239-240`) ← `SubtitleCompositor::compose` (`compositing/compose.rs:79`) ← crate-internal tests only (`tests/timing_clock.rs`, `tests/compositing_ac.rs`, `tests/compositing_alloc.rs`, `tests/dvb_vobsub_fuzz_survival.rs`, `ass/render.rs:867` test). `rg -n "SubtitleCompositor|CompositedCue" apps crates --glob '!**/subtitles/**'` → no hits outside `crates/subtitles`. No shipped binary path. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cat crates/subtitles/src/compositing/timing.rs` (full read — the whole production surface is 13 lines); `rg -n "active_at" crates/subtitles/src --glob '!**/timing.rs'` (caller map); `rg -n "SubtitleCompositor|CompositedCue" apps crates --glob '!**/subtitles/**'` (external-caller search: none); `sed -n '790,810p;1073,1085p' crates/video-engine/src/scheduler.rs` (verify the doc's cross-crate consistency claim — see F-001); `rg -n "fn |assert" crates/subtitles/tests/timing_clock.rs crates/subtitles/tests/timing_edges.rs` (integration suites that exercise this rule); `rg -n "test result" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (timing_clock 4/0, timing_edges 8/0 green). |

## 1. One-paragraph verdict
The smallest file in my assignment and the cleanest: one pure, total, allocation-free predicate implementing the half-open `[start, end)` rule with open-ended `Option` ends, four boundary-exact tests, and a real-clock integration suite (`tests/timing_clock.rs` drives the shared `MediaClock` and asserts sample-exact handovers at 48 kHz — 4/0 green in the no-fail-fast baseline). I traced every branch and every boundary including `i64::MIN/MAX` (compares only, no arithmetic — no overflow possible). The single defect is a doc line claiming its comparison "matches the video scheduler's frame-presentation comparisons": the scheduler's due-bound is inclusive (`pts <= now + lead`) and its lateness bound presents on-the-bound, while this rule is end-exclusive — deliberately different conventions, so the "matches" sentence is false as written. And, as with the entire crate: nothing shipped can call it.

## 2. What actually works (verified)
— Half-open rule, both-open policy, zero-duration, extremes: verified by branch trace and by the 4 unit tests + `tests/timing_clock.rs` (`cue_duration_is_sample_exact_on_the_real_clock`, `adjacent_cues_hand_over_without_gap_or_double_draw` — asserts `at(96_000) == 1` at the exact handover sample) + `tests/timing_edges.rs` (`touching_is_not_overlap_at_the_wire`), all green in baseline `orchrun-test-workspace-nofailfast.log.txt` (lib 267/0 includes the 4; timing_edges 8/0).

## 3. Findings

### F-001 [S4] Doc claims consistency with the video scheduler's comparisons; the scheduler's bounds are inclusive, this rule is exclusive
Where: crates/subtitles/src/compositing/timing.rs:6-11 (claim); contradicting sites: crates/video-engine/src/scheduler.rs:796-800, :1078-1082
Code (timing.rs:6-11):
```rust
//! A cue ending at `t` and the next starting at `t` must never both
//! display (double-draw at the seam) nor neither (one-frame gap):
//! `start <= now < end` gives exactly that. This is the subtitle
//! convention (WebVTT/SRT intervals are end-exclusive at the swap
//! instant) and matches the video scheduler's frame-presentation
//! comparisons.
```
Code (scheduler.rs:796-797 — inclusive due-bound; the `take_while(… p.pts.ticks) <= due_bound)` completing it is :800):
```rust
        let due_bound = i128::from(now.ticks) + i128::from(self.cfg.target_lead.0);
        let due_count = self
```
`take_while(… p.pts.ticks) <= due_bound)` at :800; and scheduler.rs:1078-1082 `is_late`: `pts < now − threshold` with the explicit note "`pts == now − threshold` is *on* the bound and presents".
What is wrong: the scheduler presents frames at `pts <= bound` (inclusive) and treats the late-bound as on-the-bound-presents; the cue rule is `now < end` (exclusive). At the seam instant `now == cue.end == frame.pts`, the scheduler presents the frame but this rule has already turned the cue off. Both conventions are individually deliberate and documented; the sentence "matches the video scheduler's frame-presentation comparisons" is false as a literal claim.
Why it matters: a maintainer aligning the two on the strength of this sentence would "fix" one side; the justification given for the half-open choice should stand on the subtitle convention alone (which it does — the WebVTT/SRT argument is correct).
What correct looks like: delete "and matches the video scheduler's frame-presentation comparisons", or state the difference explicitly ("the scheduler's due-bound is inclusive by design; cue ends are exclusive — a cue does not show on the frame presented exactly at its end").
Confidence: HIGH — Verification: read all three sites.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `active_at` | 32-44 | REAL | S4 (F-001 doc only) | Pure predicate: `(start.is_none() || now >= start) && (end.is_none() || now < end)`. Both-None = always active — documented policy with MP §8 rationale (never invent an end); zero-duration never active (tested); reversed (start > end) never active — unreachable from in-crate producers because `CompositedCue::new` validates the span (CueError) and the model layer rejects `end < start` (`tests/timing_edges.rs::end_before_start_is_typed_invalid_at_the_wire`); no arithmetic → no overflow at i64 extremes (tested at MIN/MAX). `#[must_use]` with an honest message. |

## 5. Test honesty (4 tests, lines 53-86)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `interval_is_half_open_to_the_nanosecond` | Yes | All six boundary points asserted. Mutation: `<`→`<=` at :39 fails line 59; `<`→`<=` at :34 fails line 56. |
| `adjacent_cues_never_double_draw_nor_gap` | Yes | The seam property at exactly T1 both ways. |
| `zero_duration_cue_is_never_active` | Yes | Empty span pinned at and before T0. |
| `open_ended_cues_stay_active` | Yes | None/None at i64::MIN/0/MAX; each single-sided case both polarities. |
| Integration (other files) | Yes | `tests/timing_clock.rs` proves the rule sample-exactly on the real `MediaClock` (48 kHz samples 48_000/95_999/96_000/143_999/144_000 asserts); `tests/timing_edges.rs` proves the wire/model layer agrees (zero-duration legal, end-before-start typed-invalid, touching-is-not-overlap). Green in no-fail-fast baseline. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the half-open `[t0, t1)` active-interval rule, evaluated on the shared clock's nanosecond positions" | timing.rs:1-2; compositing/mod.rs:19 | REAL | Code + `tests/timing_clock.rs` (real MediaClock, sample-exact asserts, 4/0 green). |
| "matches the video scheduler's frame-presentation comparisons" | timing.rs:10-11 | FALSE (as a literal consistency claim) | F-001: scheduler due-bound inclusive (`scheduler.rs:796-800`), late-bound presents-on-bound (`:1082`); cue rule end-exclusive. |
| "Both missing = active … hiding it would fabricate a duration" | timing.rs:22-27 | REAL (policy) | Implemented as documented; defensible; untested-with-None-at-compositor level? — `compositing/bitmap.rs:309-311` tests the None-side active behavior through `CompositedCue`. |

## 7. Dead / duplicated / unreachable code
- Unreachable from shipped entry points (Reachability row). No duplication.

## 8. Dependencies & flags
- None beyond `core` (not even `media-types` — the nanosecond `i64` contract is the crate's own). The `media-clock` dev-dependency is exercised by `tests/timing_clock.rs` only — the dev-edge-not-in-public-API claim of `crates/subtitles/Cargo.toml:50-53` holds for this file (no `media_clock` import in src/).

## 9. Open questions for the next agent
- Whoever audits the video scheduler: the seam question F-001 raises (frame presented at exactly `cue.end`) is only a doc nit while subtitles are unwired; if the Lead ever wires the compositor, decide explicitly whether a cue should be visible on the frame whose pts equals its end — and fix the doc line either way.
