# AUDIT: crates/video-engine/src/adjust.rs

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-22 |
| Lines / functions | 355 / AdjustError (+Display/Error), AdjustParams::{NEUTRAL, new, is_neutral, apply_rgba8}, check + 11 tests (196-355, 45%) |
| File verdict | **REAL code, UNREACHABLE** — a correct, well-tested CPU color-adjust model with zero production consumers (no host, no shader, no command constructs or applies it) |
| Reachable from shipped entry point? | NO. rg tree-wide for `AdjustParams|apply_rgba8|AdjustError` outside this file: only the lib.rs:184 re-export, a UI comment (ui/src/screens/settings/areas/effects.tsx:45), a worklog line, and feature-matrix.toml:1849's evidence string. No shipped app, no renderer-wgpu code, no command handler touches it. The module doc admits the production path is unbuilt ("wiring the uniforms into the wgpu pipeline is `[PROPOSAL]` follow-up", adjust.rs:10-12). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg (`AdjustParams|adjust::|apply_rgba8|AdjustError` across the workspace, incl. renderer-wgpu and apps/); sed verification of quotes. No cargo (static). |

## 1. One-paragraph verdict

The code that exists is good: typed range/finite validation per field with the offending name in the error, byte-exact neutral identity (short-circuit before any float math, proven by test), a fixed documented operator order (brightness → gamma → contrast around mid-gray → Rec.601 saturation → (B−Y,R−Y) hue rotation), no allocation, no panics, no unsafe, and 11 exact-value tests including boundary and adversarial (NaN/∞) cases. But it is a model without a consumer: `apply_rgba8` is only ever called by its own tests, the wgpu shader that is supposed to consume these values as uniforms does not exist (admitted `[PROPOSAL]`), no `video.set-adjust` command exists to set the values (feature-matrix.toml:1849 itself admits "ui pending sliders awaiting video.set-adjust"), and the UI's effects area only mentions the type in a comment. 355 lines of unreachable color science; the matrix row that cites this file as evidence is citing a parameter model, not a working adjust filter.

## 2. What actually works (verified)

- Validation (93-133): finite check then range check per field, first offender named; boundaries inclusive (test 258-261 pins 0.0/2.0/3.0/±180/0.1/10.0 accepted).
- Identity (150-154): neutral short-circuits before float conversion — byte-exact by construction, and the test (202-207) proves it on a 64-byte ramp.
- Math (161-191): verified by hand — `powf` only ever sees non-negative bases (inputs ÷255 × brightness ≥ 0, gamma applied before contrast can go negative), so no NaN path; clamp01 rounds, consistent with the pinned 128→129 contrast case ((128/255−0.5)×2+0.5 = 0.50392 → ×255 = 128.5 → 129).
- Alpha never touched (px[3] unwritten; test 294 pins it).
- `as_chunks_mut::<4>` tail safety: the length guard at 147-149 rejects non-multiple-of-4 buffers first, so the ignored remainder (`.0`) is always empty.
- Historical run: docs/gates/logs/phase7-exit-test.log:5755-5765 records all 11 tests passing.

## 3. Findings

### F-001 [S3] — the whole module is unreachable: no command, no host, no shader, no UI wiring
Where: crates/video-engine/src/adjust.rs:7-15 (the admission), corroborated by feature-matrix.toml:1849
Code (adjust.rs:7-12):
```rust
//! This is the **parameter model + CPU reference** for the adjust
//! stage. The production path is a renderer-wgpu shader consuming
//! these as uniforms (doc 03 §2.3 rule 3 — one pipeline, zero extra
//! copies); wiring the uniforms into the wgpu pipeline is
//! `[PROPOSAL]` follow-up (a bind-group layout change is structural —
```
What is wrong: rg across the workspace: `AdjustParams`/`apply_rgba8` appear in no shipped code path — not in apps/cli, apps/web, apps/desktop, renderer-wgpu, or any command handler; the only references are the re-export (video-engine/src/lib.rs:184), a UI comment (ui/src/screens/settings/areas/effects.tsx:45), and the feature-matrix evidence string. No `video.set-adjust` command exists in the commands catalog's served set. The "test oracle" role is also vacuous today: there is no GPU path for the CPU reference to be an oracle for.
Why it matters: the feature-matrix row `vfeat-colors-adjust` points at this file as its evidence — a reader would conclude a color-adjust filter exists; what exists is a validated struct nobody can instantiate from outside the crate in any shipped product. (The row's verdict belongs to wave 18; from this file's side the implementation is UNREACHABLE.)
What correct looks like: either the renderer shader + a served `video.set-adjust` command, or the matrix row's status reflecting "model only".
Confidence: HIGH — Verification: read (full-caller trace).
Severity: S3 (dead code with an honest admission), not S2 — unlike the governor/tracks cases there is no landed host-side feature being blocked; the entire stage never shipped and the file says so.

### F-002 [S4] — the gamma range carries an unresolved `[VERIFY]` tag
Where: crates/video-engine/src/adjust.rs:25 and 80
Code:
```rust
//! | gamma        | `0.1 ..= 10.0` [VERIFY exact VLC range] | `1.0`   |
```
What is wrong: the range table's own verification marker has never been resolved; the validation at line 131 enforces the guessed range. VLC's gamma slider runs 0.01–10 in different versions — the number is unverified by the code's own standard.
Why it matters: a `[VERIFY]` tag that survives into a "frozen" parameter contract is a doc debt that becomes an API constraint.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AdjustError + Display + Error | 39-66 | REAL (unreachable) | — | typed, field-named; Display tested |
| AdjustParams (struct) | 70-82 | REAL (unreachable) | — | five f32 fields, Copy |
| Default | 84-88 | REAL (unreachable) | — | = NEUTRAL |
| check | 93-101 | REAL (unreachable) | — | finite-then-range; inclusive bounds |
| AdjustParams::NEUTRAL | 106-112 | REAL (unreachable) | — | const identity |
| AdjustParams::new | 119-133 | REAL (unreachable) | — | validation order: brightness→contrast→saturation→hue→gamma (first offender wins) |
| AdjustParams::is_neutral | 136-138 | REAL (unreachable) | — | equality against NEUTRAL |
| AdjustParams::apply_rgba8 | 146-193 | REAL (unreachable) | S3 | the CPU reference; math verified (see §2); hoisted sin/cos; clamp+round at the end; F-001 |

## 5. Test honesty (11 tests, lines 196-355)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| neutral_params_leave_pixels_bit_unchanged (202) | YES | byte-exact identity on a 64-byte ramp |
| default_is_neutral_and_neutral_is_identity (210) | YES | Default==NEUTRAL |
| out_of_range_fields_are_rejected_by_name (216) | YES | five fields, both sides |
| non_finite_fields_are_rejected (244) | YES | NaN and ∞ |
| boundary_values_are_accepted (258) | YES | all five upper bounds + lower |
| brightness_scales_pixels_and_zero_is_black (264) | YES | mid-gray→white at 2.0; →black at 0.0 |
| saturation_zero_is_grayscale_but_alpha_survives (282) | YES | expected luma computed independently in the test |
| contrast_pushes_away_from_midgray_and_clamps (298) | YES | 128→129 rounding pinned; clamp both directions; 0-collapse to 128 |
| hue_180_rotates_red_chroma_to_blue_side (329) | YES | directional check (B>R, B>G) — weaker than exact but adequate for the rotation sign |
| bad_pixel_length_is_a_typed_error (342) | YES | 3-byte buffer rejected |
| error_display_names_the_field (351) | YES | string contains field + "range" |

Tests test dead code (F-001) but are themselves genuine. UNVERIFIED by run here; historical green in phase7-exit-test.log:5755-5765.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The production path is a renderer-wgpu shader consuming these as uniforms" | adjust.rs:8-9 | PAPER-ONLY | no shader code references AdjustParams (rg over renderer-wgpu) — the admission follows immediately in the same doc |
| "`apply_rgba8` is the byte-exact CPU reference the GPU path must match (and the test oracle)" | adjust.rs:14-15 | UNREACHABLE | no GPU path exists to match it |
| "Neutral parameters are the identity … bit-unchanged" | adjust.rs:27-29 | REAL | short-circuit + test 202 |
| "Out-of-range values are rejected at construction — no silent clamping" | adjust.rs:30-32 | REAL | check() + tests |
| ranges table incl. gamma `[VERIFY exact VLC range]` | adjust.rs:19-25 | UNVERIFIED | the [VERIFY] tag itself (F-002) |
| feature-matrix evidence "ui pending sliders awaiting video.set-adjust" | feature-matrix.toml:1849 | REAL (honest admission embedded in the evidence string) | no such command in the served set |

## 7. Dead / duplicated / unreachable code

- The entire public surface (`AdjustParams`, `AdjustError`, `apply_rgba8`): unreachable from shipped entry points (F-001). Not duplicated elsewhere (no other adjust implementation exists).

## 8. Dependencies & flags

- No dependencies beyond core/std (f32 math only) and testkit in tests. No features, no unsafe, no allocation. The lib.rs:184 re-export is the only external seam.

## 9. Open questions for the next agent

- renderer-wgpu owner (wave 8): confirm no shader consumes these uniforms (rg says none) — if one was planned, this model is its contract.
- Wave 15 (ui/src): effects.tsx:45 mentions AdjustParams in a comment — do the effects sliders render and write to nothing? (Dead-button audit.)
- Wave 18: matrix row `vfeat-colors-adjust` should be SHALLOW/UNREACHABLE, not whatever the row currently claims; the evidence string points at a parameter model with no consumer.
