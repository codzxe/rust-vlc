# AUDIT: crates/playback-engine/src/frame_step.rs
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-21 |
| Lines / functions | 170 / 7 (sign, plan_frame_step, const, 5 tests) |
| File verdict | UNREACHABLE (real, correct, tested — zero callers) |
| Reachable from shipped entry point? | NO: `plan_frame_step`/`FrameStepPlan` exported (lib.rs:122-123) but called by nothing outside this file's tests; `playback.step-frame` is cataloged (commands/src/catalog/command.rs:129-131) yet absent from `SessionCommandKind` (session.rs:127-147: Play/Pause/Stop/Close/Seek/SelectVideo/SelectAudio only) and dispatched by no host (rg `PlaybackStepFrame` in apps/ → zero) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "plan_frame_step\|FrameStepPlan\|frame_step" crates/ apps/` → lib.rs re-export + snapshot.rs doc-crossref only; `rg -n "PlaybackStepFrame" apps/ crates/playback-engine/ --type rust` (minus commands crate) → zero executors; SessionCommandKind enum read (session.rs:127+); matrix rows read (:1534-1543, :3250-3257); UI demo handler read (ui/src/client/demo-transport.ts:447-452); baseline: 5 `frame_step::tests` ok (orchrun :3784-3788) |

## 1. One-paragraph verdict
A small, correct, honestly-documented pure planner: one signed
frame-period `SeekTarget::Relative` plus an auto-pause flag, a guarded
24 fps fallback (non-positive periods replaced, never trusted, never a
divide), all pinned by 5 exact tests that ran ok in the no-fail-fast
baseline. And it is called by absolutely nothing: the engine's command
vocabulary has no step command, no host dispatches the cataloged
`playback.step-frame`, and the only working frame-step in the product is
the UI demo transport's client-side simulation (`deltaMs: 1000/24`,
demo-transport.ts:447-452) — a separate TypeScript implementation of the
same policy. The snapshot.rs pattern repeats: catalog command + keymap
bindings (`.`/`,`/e) + pure planner + UI simulation, with no wire between
the engine planner and any of them.

## 2. What actually works (verified)
— All 5 tests executed ok: baseline
`audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3784-3788`
(incl. `non_positive_period_is_replaced_not_trusted` and the
`FALLBACK_FRAME_PERIOD_NS == 41_666_666` pin).
— Mutation checks (by reading): flipping `sign()` breaks both direction
tests (:113-134); dropping the `ns > 0` guard fails :150-162; the
fallback value itself is pinned at :146.
— Overflow safety verified: `direction.sign() * period` cannot overflow —
period is either `> 0` (guarded, :96-99) or the 41.6 ms constant; `-1 ×
positive-i64` is in range.
— The fallback constant is honest about being floor(1e9/24) = 41_666_666
(integer division, test-pinned).

## 3. Findings
### F-001 [S3]
Where: crates/playback-engine/src/frame_step.rs:91-106
Code:
```rust
pub fn plan_frame_step(
    direction: FrameStepDirection,
    frame_period_ns: Option<i64>,
    is_playing: bool,
) -> FrameStepPlan {
    let period = match frame_period_ns {
        Some(ns) if ns > 0 => ns,
        _ => FALLBACK_FRAME_PERIOD_NS,
    };
```
What is wrong: The function has zero callers. `playback.step-frame` is a
cataloged class-A command with keymap bindings (keymaps/default.toml:68,
94-95) and a UI demo handler, but the engine's `SessionCommandKind`
(session.rs:127-147) has no Step variant, and no host translates the
command into `plan_frame_step` + pause + seek. The module doc's premise —
"Execution rides the existing seek flow" (:9-13) — describes a ride no
code ever takes.
Why it matters: The feature matrix row `playback-frame-step` is "partial"
on all platforms citing this file as engine evidence; in reality no
shipped binary can frame-step — the only implementation that runs is the
UI's client-side simulation (demo-transport.ts:447-452), which the matrix
also cites. Pressing `.` against any real engine session cannot produce a
frame step.
What correct looks like: A `SessionCommandKind::StepFrame` (or host-side
composition: Pause-if-playing + Seek with the planned Relative target),
one dispatch arm, one integration test through the session.
Confidence: HIGH — Verification: rg sweeps + SessionCommandKind read +
UI handler read.
Severity S3 (not S2): the UI demo path gives users a working simulation
in demo mode; the gap is the engine wire, not the concept.

### F-002 [S4]
Where: crates/playback-engine/src/frame_step.rs:70 (with ui/src/client/demo-transport.ts:447-452)
Code:
```rust
pub const FALLBACK_FRAME_PERIOD_NS: i64 = 1_000_000_000 / 24;
```
What is wrong: The 24 fps fallback exists twice, in two languages, with
no shared source of truth: `1_000_000_000 / 24` here and `1000 / 24` ms
in the UI demo transport. They agree today (41,666,666 ns ≈ 41.6667 ms —
the UI value is fractional ms, the engine value is floored ns, a 0.7 ns
equivalence gap per step); nothing prevents drift.
Why it matters: The module doc explicitly claims the match ("matches the
UI demo transport's `1000 / 24` ms step", :68-69) — a cross-language
invariant held only by comment.
What correct looks like: The fallback lives in one generated artifact
(the commands catalog or a shared schema constant the UI consumes), or a
test asserts the two values agree.
Confidence: HIGH — Verification: both sites read; the ns/ms units and
flooring compared.
Severity S4: sub-nanosecond practical impact today, drift risk tomorrow.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FrameStepDirection` (enum) | 48-55 | REAL | — | Next/Previous; VLC-parity note honest (VLC 3.0.23 has no frame-prev) |
| `FrameStepDirection::sign` | 59-64 | REAL | — | ±1 |
| `FALLBACK_FRAME_PERIOD_NS` | 70 | REAL | S4 (F-002) | 24 fps floor; duplicated in UI TS |
| `FrameStepPlan` (struct) | 74-82 | REAL | — | needs_pause + Relative target — plain data |
| `plan_frame_step` | 91-106 | REAL (unreachable) | S3 (F-001) | Pure; non-positive period guarded; no divide; no panic paths |
| test `next_step_is_plus_one_period_relative` | 113-122 | REAL | — | Ran ok (:3784) |
| test `previous_step_is_minus_one_period_relative` | 125-134 | REAL | — | Ran ok (:3787) |
| test `unknown_period_falls_back_to_24fps` | 137-147 | REAL | — | Ran ok (:3788); pins the constant |
| test `non_positive_period_is_replaced_not_trusted` | 150-162 | REAL | — | Ran ok (:3785); 0 and negative both covered |
| test `paused_step_needs_no_pause_input` | 165-168 | REAL | — | Ran ok (:3786) |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| next/previous_step_... | YES | Exact SeekTarget equality incl. sign and needs_pause |
| unknown_period_falls_back_to_24fps | YES | Exact delta + the constant's literal value pinned |
| non_positive_period_is_replaced_not_trusted | YES | Two bad inputs → exact fallback |
| paused_step_needs_no_pause_input | YES | needs_pause flag pinned |
All 5 ran ok in baseline (orchrun :3784-3788). No tautologies. The tests
prove the planner correct — they cannot prove it wired (it is not, F-001).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Engine seam landed: auto-pause flag + signed one-frame Relative SeekTarget" | matrix :1543 notes | REAL (the seam exists) / the feature remains user-invisible in engine sessions | F-001 — seam uncalled |
| "Execution rides the existing seek flow" | module doc :9-13 | FALSE as written — no code path composes plan→seek | rg zero callers; SessionCommandKind lacks the command |
| "the step lands on the container's keyframe-safe position ≤ the target … a decoded frame boundary is not guaranteed" | module doc :36-43 | Honest v0 limit (REAL description of reanchor semantics) | Cross-checked: plugin_api::Demuxer::reanchor contract cited; seek.rs resolve_target clamps (audited in session.rs part5 wave-mate) |
| "matches the UI demo transport's 1000/24 ms step" | :68-69 | REAL today, unenforced cross-language duplicate | F-002 |

## 7. Dead / duplicated / unreachable code
- The whole public surface is production-dead (F-001) — exported at
  lib.rs:122-123, consumed by tests only.
- The 24 fps fallback is duplicated across Rust/TS (F-002).

## 8. Dependencies & flags
`crate::seek::SeekTarget` only. No unsafe, no I/O, no feature gates, no
allocations beyond the returned Copy struct. The `#[must_use]` on
plan_frame_step is right (an ignored plan is a lost step).

## 9. Open questions for the next agent
- The commands-catalog wave: `playback.step-frame` is class A
  "generated" per doc 05 §2.4 — check whether the catalog's generator
  (spec.rs:240, :950, :983) is supposed to also generate the engine-side
  dispatch; if yes, its absence is a generator gap, not a per-command
  oversight.
- If the session gains StepFrame, the `needs_pause` flag must be applied
  BEFORE the seek in the same poll (the pause-entry tick semantics in
  session.rs apply()) — the composition order is the subtle part.
