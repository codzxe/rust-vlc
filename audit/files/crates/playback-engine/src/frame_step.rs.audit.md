# AUDIT: crates/playback-engine/src/frame_step.rs
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-23 |
| Lines / functions | 169 / 7 (1 pub fn, 1 enum + sign, 2 types, 1 const, 5 tests) |
| File verdict | UNREACHABLE (real, correct, tested code — no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO. rg repo-wide: `plan_frame_step`/`FrameStepDirection` are called only by this file's own tests. The wire command it exists for (`playback.step-frame`, catalog class A + gen-bumping, commands/src/catalog/command.rs:129-131, spec.rs:240/950/983) has NO dispatch arm: the CLI's route table (apps/cli/src/dispatch.rs:81-155) serves open/close/play/pause/toggle/stop/seek/media-info/time/duration/tracks-list/engine-info/list-commands/config — `other => Err(unserved(other))`, and `unserved` answers `UnsupportedForHost` ("not served by this build's command-line host", dispatch.rs:156-174). The desktop UI ships default hotkeys that send it (ui/src/screens/settings/areas/hotkeys/generated/bindings.ts:62,80-81: "." / "," / BrowserNext → `playback.step-frame`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `./target/debug/deps/playback_engine-<hash> frame_step` (prebuilt) -> 5 passed, 0 failed (exit 0); rg caller enumerations |

## 1. One-paragraph verdict
The pure planning logic is small, correct, and honestly tested: direction sign, optional real frame period with a documented 24 fps fallback, non-positive periods replaced rather than trusted, auto-pause flag, and a relative SeekTarget that rides the existing seek resolution. All 5 tests pass (my run + baseline cargo-test-workspace.txt:4138-4142). But none of it is wired: no production code calls `plan_frame_step`, the CLI refuses the command typed, and the shipped UI binds three hotkeys to a command the engine answers with `UnsupportedForHost`. The module doc's "Execution rides the existing seek flow of [`crate::seek`]" describes the intended composition — no such composition exists in any shipped build. This is textbook UNREACHABLE: real code, correct, orphaned behind a catalog row and a UI hotkey.

## 2. What actually works (verified)
— verified by running the prebuilt lib test binary filtered on `frame_step`: `test result: ok. 5 passed; 0 failed` (exit 0).
— verified the fallback constant and non-positive replacement behavior from the passing tests (`unknown_period_falls_back_to_24fps` pins `FALLBACK_FRAME_PERIOD_NS == 41_666_666`; `non_positive_period_is_replaced_not_trusted` covers 0 and negative).
— verified the unserved path end-to-end shape: the CLI's own test `unserved_commands_refuse_typed` (dispatch.rs:228) pins the typed-refusal behavior for exactly this class of gap.

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/src/frame_step.rs:7-13 (vs apps/cli/src/dispatch.rs:81-155, ui/.../bindings.ts:80-81)
Code:
```
//! The **pure planning half** of a frame step: given the current
//! position, the media's frame period and a direction, produce the
//! [`SeekTarget`] the session should apply. Execution rides the
//! existing seek flow of [`crate::seek`] (a frame step IS a seek of
//! one frame's magnitude — doc 05 §2.4 says the command is class A,
```
What is wrong: "Execution rides the existing seek flow" — no execution exists. `plan_frame_step` has zero production callers (rg over crates/ and apps/: only frame_step.rs's own `#[cfg(test)]`). The wire command `playback.step-frame` is catalog-declared (class A, gen-bumping, in §4.2's closed mint list — spec.rs:983), exported into the generated TS bindings, and bound to three shipped UI hotkeys ("." / "," / BrowserNext, bindings.ts:62,80-81) — and the shipped CLI dispatch has no arm for it, answering the typed `UnsupportedForHost` refusal. A user pressing "." in the shipped desktop gets an error; the planning code that would make it work sits in the crate, exported (`lib.rs:122-123`), unused.
Why it matters: Feature-shaped surface (catalog row + hotkeys + generated types + this module) with zero served path — the exact "advertised but unserved" class the parity report's 0/252 aggregates. The module doc reads as if the composition exists.
What correct looks like: Either wire the dispatch arm (pause input when `needs_pause`, then the seek — the seek flow itself is served and tested), or mark the module and the catalog row as unserved-in-v0 and remove the shipped hotkey bindings.
Confidence: HIGH — Verification: rg (all callers enumerated: none) + read of dispatch.rs route table + bindings.ts.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FrameStepDirection (+sign) | 48-65 | REAL / UNREACHABLE | — | Correct signed multiple. Duplicates the wire `StepDirection` (types.rs:446-454) with no bridge — cross-file duplication already filed by W02-A07 as types.rs F-005; noted, not re-counted here. |
| FALLBACK_FRAME_PERIOD_NS | 67-70 | REAL / UNREACHABLE | — | 1e9/24 = 41,666,666 (integer division; test pins the value); [PROPOSAL] marked. |
| FrameStepPlan | 72-82 | REAL / UNREACHABLE | — | needs_pause + relative target — a clean seam for the (missing) executor. |
| plan_frame_step | 90-106 | REAL / UNREACHABLE | S3 | Correct: positive-period guard, fallback, no panic, no divide. F-001 is the reachability verdict. |
| 5 tests | 108-169 | REAL | — | All pass; each pins one behavior (sign, pause flag, fallback value, bad-period replacement). |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| next/previous_step_is_±_period_relative | YES | Exact SeekTarget equality both directions. |
| unknown_period_falls_back_to_24fps | YES | Value pinned (41_666_666). |
| non_positive_period_is_replaced_not_trusted | YES | 0 and negative inputs → fallback, not a zero-delta seek. |
| paused_step_needs_no_pause_input | YES | Flag pinned both ways across the tests. |
| (all) — but the suite cannot detect | — | Being orphaned: nothing tests that any executor consumes the plan (there is none). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Execution rides the existing seek flow of crate::seek" | module doc, lines 9-10 | FALSE (no such execution exists) | rg: zero callers; dispatch.rs:155 `other => Err(unserved(other))`. |
| "doc 05 §2.4 says the command is class A, generated" | module doc, lines 11-12 | REAL | spec.rs:240/950 (class A), command.rs:129-131. |
| "A non-positive period is a caller bug and also falls back — never a panic, never a divide" | lines 88-89 | REAL | Code 96-99; test 150-162. |
| "Clamping is truth, not failure" (target clamps to [0, duration]) | lines 22-26 | REAL-UNPROVEN here | The clamping lives in seek.rs's resolution (other agents' audited file); this module only emits the relative target. |
| UI hotkeys "." / "," / BrowserNext shipped for frame step | bindings.ts:62,80-81 | REAL (bindings exist) / FALSE (feature works) | The command the bindings send is refused `UnsupportedForHost` by the shipped host. |

## 7. Dead / duplicated / unreachable code
- The entire module is production-dead (F-001) while being `pub use`d at the crate root (lib.rs:122-123) — exported API with no consumer.
- `FrameStepDirection` vs wire `StepDirection` duplication: no conversion bridge; already filed from the types.rs side (W02-A07, types.rs.audit.md F-005); cross-referenced here.

## 8. Dependencies & flags
- Depends only on `crate::seek::SeekTarget`. Ships in the release lane (it's in the playback-engine lib) but no shipped entry point reaches it. No feature gates — it is unconditionally compiled and unconditionally uncalled.

## 9. Open questions for the next agent
- The `SeekKind::FrameStep` wire variant (types.rs:462-466) is the seek-completed payload for this command — with the command unserved, that variant is also producer-less; whoever audits seek.rs/event.rs's `SeekKind` mapping should confirm no fabricated `FrameStep` completions exist (none were observed in the events-contract audit).
- If the orchestrator wants one S0-candidate from this wave: the trio (catalog row + shipped UI hotkeys + typed refusal) is the strongest "advertised but unserved" pattern I found; I rate it S3 because the refusal is typed and documented (doc 05 §5.2 capability-gap behavior), not silent breakage.
