# AUDIT: crates/video-engine/src/adjust.rs
| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2025-06-27 |
| Lines / functions | 355 / 7 (+12 unit tests) |
| File verdict | UNREACHABLE (real, well-validated parameter model + CPU reference with zero production callers — no engine wiring, no `video.set-adjust` command, GPU path is an admitted [PROPOSAL], UI sliders inert) |
| Reachable from shipped entry point? | NO. rg repo-wide (excluding audit/): `AdjustParams`/`apply_rgba8` appear only in adjust.rs itself, the lib.rs:184 re-export, a UI comment (ui/src/screens/settings/areas/effects.tsx:45), and the feature-matrix evidence string. No engine, renderer, command, or app code constructs it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `sed -n` quotes (exit 0); `rg 'AdjustParams|apply_rgba8|adjust::'` repo-wide excluding audit/ (exit 0 — no production caller); baseline cargo-test-workspace.txt:6458-6469 — all 12 adjust::tests pass |

## 1. One-paragraph verdict
Hostile question 1, adjust half answered: this is honest scaffolding, and it is UNREACHABLE — the module doc itself says "wiring the uniforms into the wgpu pipeline is `[PROPOSAL]` follow-up", the feature-matrix row honestly marks the feature "partial" with "video.set-adjust catalog command (D-012) pending" and "UI sliders render inert-with-reason until the command lands", and rg proves no shipped code path ever calls `AdjustParams::new` or `apply_rgba8`. What exists is a genuinely well-tested parameter model (typed range/finite validation with ranges living only at the `new` call sites, byte-exact neutral identity short-circuit, no per-pixel allocation, clamped write-back). The two real defects: the hue rotation is mathematically approximate (the G channel is never recomputed from the rotated chroma, and the chroma pair is unnormalized — no BT.601 inverse coefficients), and this approximation is declared "the byte-exact CPU reference the GPU path must match", so the day the shader lands it will enshrine the approximation; and the shipped range table carries an unresolved `[VERIFY exact VLC range]` marker on gamma.

## 2. What actually works (verified)
— All 12 unit tests pass: baseline cargo-test-workspace.txt:6458-6469.
— Validation is exact and typed: `out_of_range_fields_are_rejected_by_name` pins every field's rejection; `non_finite_fields_are_rejected` covers NaN/∞; `boundary_values_are_accepted` pins the inclusive bounds.
— The neutral identity is byte-exact by short-circuit (lines 150-154), pinned by `neutral_params_leave_pixels_bit_unchanged` over a 16-pixel sweep.
— `contrast_pushes_away_from_midgray_and_clamps` pins the composite arithmetic including the documented 128→129 rounding artifact; `saturation_zero_is_grayscale_but_alpha_survives` recomputes the expected Rec.601 luma independently.

## 3. Findings
F-001 [S3]
Where: crates/video-engine/src/adjust.rs:5-15 (whole module)
Code:
```
//! This is the **parameter model + CPU reference** for the adjust
//! stage. The production path is a renderer-wgpu shader consuming
//! these as uniforms (doc 03 §2.3 rule 3 — one pipeline, zero extra
//! copies); wiring the uniforms into the wgpu pipeline is
//! `[PROPOSAL]` follow-up
```
What is wrong: no shipped entry point reaches this code. The `video.set-adjust` catalog command does not exist; no renderer-wgpu code consumes the params; the UI's effects area renders inert sliders (ui/src/screens/settings/areas/effects.tsx:43-45 documents the pending stage). The feature-matrix row (docs/feature-matrix.toml:1849) is honest about this ("Partial: parameter model + CPU reference only"), so this is not fraud — but per the audit's verdict scale the code is UNREACHABLE: 355 lines of validated engine-side model with zero runtime effect. Why it matters: the `REAL / 252` matrix arithmetic must not count this row as working; any doc claiming color-adjust "works" on any platform is false. What correct looks like: land the command + shader wiring, or keep the row "not_started" until then. Confidence: HIGH — Verification: rg repo-wide + read of matrix row and UI comment.

F-002 [S3]
Where: crates/video-engine/src/adjust.rs:182-190
Code:
```
            // Hue: rotate the (B−Y, R−Y) chroma pair by the hue angle.
            let cb = b - luma;
            let cr = r - luma;
            let r = luma + cr * cos_h - cb * sin_h;
            let b = luma + cb * cos_h + cr * sin_h;
            let clamp01 = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round();
```
What is wrong: the hue rotation recomputes only R and B; G keeps its pre-rotation value (line 180's `g` is written at line 189 untouched by the rotation). A YPbPr hue rotation must re-derive all three channels from the rotated (Y, Pb, Pr) — G = Y − 0.344·Pb' − 0.714·Pr' for BT.601 — otherwise the rotated pixel's luma/color drifts arbitrarily with hue. The chroma pair is also unnormalized (no 1.402/1.772 inverse coefficients), so rotation strength differs per axis. The module doc's "YPbPr-ish" (line 167) concedes approximation, but header lines 14-15 declare `apply_rgba8` "the byte-exact CPU reference the GPU path must match (and the test oracle)" — enshrining a wrong hue model as the product's ground truth. The test (`hue_180_rotates_red_chroma_to_blue_side`, lines 329-339) only asserts direction (`px[2] > px[0]`), not exactness — the weakest oracle in the file. What correct looks like: rotate in a proper normalized chroma plane and recompute G; pin an exact expected triple. Confidence: HIGH — Verification: read + arithmetic (pure red at 180°: R'=luma−cr clamps to 0, B'=luma−cb=0.598→152, G stays 0 — luma of the result is no longer 0.299).

F-003 [S4]
Where: crates/video-engine/src/adjust.rs:78
Code:
```
    /// Hue rotation in degrees (`0.0` = unchanged; ±180 full turn).
```
What is wrong: ±180° is a HALF turn; a full turn is 360°. The intended meaning ("the −180..180 range spans a full turn") is guessable but the sentence as written is false. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/video-engine/src/adjust.rs:25 and 80
Code:
```
//! | gamma        | `0.1 ..= 10.0` [VERIFY exact VLC range] | `1.0` |
...
    /// Gamma exponent (`1.0` = unchanged). [VERIFY] exact upper range.
```
What is wrong: an unresolved verification marker ships inside the "VLC-corroborated" range table — the table's own header claims corroboration while one of its five ranges is admitted unverified. Honest, but it is an open TODO in a shipped contract. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AdjustError Display/Error | 53-66 | REAL | — | typed, names the field |
| AdjustParams::default | 84-88 | REAL | — | NEUTRAL |
| check | 93-101 | REAL | — | ranges live at call sites only — good discipline |
| AdjustParams::new | 119-133 | REAL | — | finite + inclusive-range validation, no clamping |
| AdjustParams::is_neutral | 136-138 | REAL | — | exact equality vs NEUTRAL consts (safe: no clamping) |
| AdjustParams::apply_rgba8 | 146-193 | REAL (code) / UNREACHABLE (prod) | S3 | neutral short-circuit exact; hue approximation F-002 |
| 12 tests | 201-354 | REAL | S4 | all exact except the directional-only hue oracle |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| neutral_params_leave_pixels_bit_unchanged | YES | byte-equality over a 16-pixel sweep |
| default_is_neutral_and_neutral_is_identity | YES | exact equality |
| out_of_range_fields_are_rejected_by_name | YES | typed equality per field, both hue bounds |
| non_finite_fields_are_rejected | YES | NaN and ∞ |
| boundary_values_are_accepted | YES | inclusive bounds both ends |
| brightness_scales_pixels_and_zero_is_black | YES | exact 255/0 triples |
| saturation_zero_is_grayscale_but_alpha_survives | YES | independently recomputed Rec.601 luma |
| contrast_pushes_away_from_midgray_and_clamps | YES | 128→129 pinned + clamps + collapse-to-midgray |
| hue_180_rotates_red_chroma_to_blue_side | PARTIAL | direction only — would not catch the G-channel omission or unnormalized chroma (F-002) |
| bad_pixel_length_is_a_typed_error | YES | 3-byte buffer |
| error_display_names_the_field | YES | substring |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the byte-exact CPU reference the GPU path must match (and the test oracle)" | adjust.rs:14-15 | SHALLOW | exact for brightness/contrast/saturation; hue is an unnormalized partial rotation (F-002) |
| "Ranges ... VLC-corroborated" | adjust.rs:17 | SHALLOW | gamma range self-marked [VERIFY] (F-004) |
| "Out-of-range values are rejected at construction — no silent clamping" | adjust.rs:30-32 | REAL | check() + tests |
| "Allocation per pixel, panics, unsafe" — Never | adjust.rs:34-36 | REAL | as_chunks_mut, no alloc, no panic path (length checked), no unsafe |
| matrix row vfeat-colors-adjust status "partial", "parameter model + CPU reference only" | feature-matrix.toml:1848-1850 | REAL (honest) | rg confirms zero production callers |

## 7. Dead / duplicated / unreachable code
- The entire module is production-unreachable today (F-001) — exported at lib.rs:184 but never imported by engine, renderer, commands, or apps.

## 8. Dependencies & flags
Std-only within the std-only video-engine crate; no deps, no unsafe, no feature gates. Tests run in the default lane (baseline :6458-6469).

## 9. Open questions for the next agent
- When the D-012 `video.set-adjust` command lands, re-audit F-002 before the shader copies the reference — the G-channel omission will become user-visible once hue is wired.
- The UI effects area (ui/src/screens/settings/areas/effects.tsx) renders inert sliders "with reason" — verify the reason string actually renders (an inert control without an explanation is a UI lie) — that file belongs to the UI owner.
