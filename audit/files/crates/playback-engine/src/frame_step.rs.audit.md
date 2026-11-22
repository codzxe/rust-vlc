# AUDIT: crates/playback-engine/src/frame_step.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 170 / 4 production items + 5 `#[test]` fns |
| File verdict | UNREACHABLE (correct, honest pure-planning helper with no driver) |
| Reachable from shipped entry point? | **NO** — `plan_frame_step` is consumed only by lib.rs's re-export (crates/playback-engine/src/lib.rs:123). The `playback.step-frame` command exists in the catalog (crates/commands/src/catalog/command.rs:129-131) with **no executor** anywhere (grep over playback-engine, apps/cli, apps/desktop-tauri: catalog definitions only; the UI's step-frame handler is demo-transport, per the matrix row's own evidence string). |
| Findings | S0:1 S1:0 S2:0 S3:0 S4:0 |
| Commands run | rg for `plan_frame_step`/`StepFrame` consumers across crates+apps; matrix row read (docs/feature-matrix.toml:1534-1543); full-file read |

## 1. One-paragraph verdict

Seventy lines of correct pure planning: pick the frame period (real when the host knows it, non-positive or unknown → the documented 24 fps fallback — never a divide, never a panic), emit `needs_pause` (auto-pause-before-step, VLC parity) and a signed one-frame `SeekTarget::Relative` that rides the existing seek flow's clamping. Five tests pin every rule including the non-positive-period defense. And like bookmarks: no shipped binary can reach it. The `playback.step-frame` command has no executor; the matrix row claims **"partial" on all four platforms** with this file as evidence, while the row's own evidence string admits every other listed piece "pre-existed" as catalog/keymap/demo-transport artifacts — none of which execute a frame step against the engine either.

## 2. What actually works (verified)

- The planning math (91-106): `Some(ns) if ns > 0` guard, fallback const `1_000_000_000/24 = 41_666_666` (pinned by test at 146), signed delta via `direction.sign() * period`.
- The v0 honesty notes (36-43): decode-exact landing is explicitly out of scope — a step lands on the container's keyframe-safe position, not a decoded frame boundary. This is the correct engineering call, stated plainly.

## 3. Findings

### F-001 [S0] — `playback-frame-step` row claims "partial" on all platforms; the feature has no executor

Where: docs/feature-matrix.toml:1541-1542; the unreachable helper at crates/playback-engine/src/frame_step.rs:91-106.

Code (docs/feature-matrix.toml:1541-1542, verbatim):
```toml
status = { windows = "partial", linux = "partial", macos = "partial", web = "partial" }
evidence = "crates/playback-engine/src/frame_step.rs (FrameStepPlan/plan_frame_step + tests); commands catalog playback.step-frame (next/previous) pre-existed; keymaps/default.toml frame-next '.'/frame-prev ',' pre-existed; ui demo-transport step-frame handler pre-existed"
```

What is wrong: `plan_frame_step` has zero non-test callers. The `playback.step-frame` command (catalog command.rs:129-131) has no handler in any engine session executor or host dispatch (repo-wide grep). The keymap binds '.'/',' to actions nothing processes; the UI handler is demo-transport. Status "partial" on four platforms asserts shipped behavior that does not exist on any platform. Same pattern as `playback-bookmarks` (my bookmarks.rs audit F-001) and `afeat-volume` (my effects.rs audit F-001).

Why it matters: §4 S0 — the claim is false; a user pressing '.' or 'e' in any shipped surface steps nothing.

What correct looks like: a session executor owns `playback.step-frame` (pause-if-playing + submit the planned `SeekTarget` through the existing seek path — the module was designed for exactly that), or the row says not_started.

Confidence: HIGH — Verification: read + exhaustive grep.

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-43 | REAL (honest about v0 limits) | — | decode-exact landing explicitly deferred |
| `FrameStepDirection` + `sign` | 49-65 | UNREACHABLE | — | Next/Previous |
| `FALLBACK_FRAME_PERIOD_NS` | 70 | UNREACHABLE | — | 24 fps, [PROPOSAL] labeled |
| `FrameStepPlan` (struct) | 75-82 | UNREACHABLE | — | needs_pause + Relative target |
| `plan_frame_step` | 91-106 | UNREACHABLE | S0 | F-001; correct math, defensive fallback |
| tests (5) | 112-168 | REAL-UNPROVEN | — | §5 |

## 5. Test honesty (5 `#[test]`)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `next_step_is_plus_one_period_relative` (113-122) | YES | exact delta + needs_pause |
| `previous_step_is_minus_one_period_relative` (125-134) | YES | negative delta + no pause |
| `unknown_period_falls_back_to_24fps` (137-147) | YES | fallback const pinned by value |
| `non_positive_period_is_replaced_not_trusted` (150-162) | YES | 0 and negative both replaced |
| `paused_step_needs_no_pause_input` (165-168) | YES | flag semantics |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| status partial (all platforms) | docs/feature-matrix.toml:1541 | FALSE | F-001 |
| "Engine seam landed: auto-pause flag + signed one-frame Relative SeekTarget, 24 fps fallback" | docs/feature-matrix.toml:1543 | TRUE as a library, FALSE as a seam | the seam's consumer never landed |
| "decode-exact landing … is the honest v0 limit" | frame_step.rs:41-43 / matrix notes | REAL | honest |

## 7. Dead / duplicated / unreachable code

The whole module (F-001). Re-exported at lib.rs:123 for a driver that does not exist.

## 8. Dependencies & flags

Depends on `crate::seek::SeekTarget` only. No feature gates; dead code in every shipped binary.

## 9. Open questions for the next agent

- `media_types::Track` still carries no frame-rate field (module docs 28-31) — the fallback stays [PROPOSAL] until it does; media-types is wave-2 territory.
- The keymap rows for frame-next/frame-prev (keymaps/default.toml) — wave 11 config audit: do they bind to a catalog command with no executor (dead keybinding)?
