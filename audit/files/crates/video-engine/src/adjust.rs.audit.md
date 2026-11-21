# AUDIT: crates/video-engine/src/adjust.rs
| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2026-11-21 |
| Lines / functions | 355 / 8 production units + 11 tests |
| File verdict | REAL (complete, tested parameter model + CPU reference) but UNREACHABLE from shipped entry points (no production caller; GPU wiring and the `video.set-adjust` command are both pending, per the module's own docs and the feature matrix) |
| Reachable from shipped entry point? | NO. `rg -ln "AdjustParams\|AdjustError" crates/ apps/` → only `crates/video-engine/src/adjust.rs` and its re-export `crates/video-engine/src/lib.rs:184`. No renderer-wgpu use (the "production path is a renderer-wgpu shader" — module doc 8-11 — is `[PROPOSAL]`), no playback-engine/app use, and the CLI refuses the whole `video.*` family (`apps/cli/src/dispatch.rs`: "video output control joins with the windowed renderer (2.D3)"). The UI sliders are inert-with-reason (feature matrix row `vfeat-colors-adjust` notes at docs/feature-matrix.toml:1850). Only the test suite executes this code — video_engine lib 48 passed / 0 failed in the no-fail-fast baseline (orchrun log, `deps/video_engine` binary). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | full read via Read tool (355 lines); `rg -ln "AdjustParams\|adjust::" crates/ apps/` (callers: none outside video-engine); `rg -rn "AdjustParams\|AdjustError" crates/ apps/ ui/src --glob '!crates/video-engine/**'` (zero hits); `rg -n "adjust" docs/feature-matrix.toml` (row `vfeat-colors-adjust` at 1840-1850: honestly "Partial: parameter model + CPU reference only", gamma range `[VERIFY]`); hue math verified by hand-computation (see F-002); baseline: video_engine lib 48/48 green (orchrun log `deps/video_engine`) |

## 1. One-paragraph verdict
A disciplined little module: typed validation with named fields, a
bit-exact identity short-circuit, hoisted per-buffer math, alpha untouched,
and eleven mutation-sensitive tests that ran green in the baseline. Its two
defects are structural, not cosmetic. First, the doc promises
"`AdjustParams` carries the validated, clamped-by-contract values" — but all
five fields are `pub f32`, so `AdjustParams { brightness: f32::NAN, .. }`
compiles and sails past the very `NotFinite` check the type exists to
enforce (no panic results — NaN clamps to 0 — but the invariant is
convention, not type). Second, the hue "rotation" only rotates the (B−Y,
R−Y) pair and **never touches the green channel** — pure red at 180°
becomes blue (the in-file test asserts exactly this) where a true 180°
rotation yields cyan; pure green at 90° becomes yellow where truth is cyan.
As the self-declared "byte-exact CPU reference the GPU path must match",
this would bake a wrong colorimetric axis into the shader. Both are
currently unreachable from shipped binaries (no caller, unwired GPU,
refused `video.*` commands, inert UI) — which is also why nobody has
noticed the hue axis.

## 2. What actually works (verified)
— Validation — verified: `check` (93-101) refuses non-finite before range;
`new` (119-133) validates all five fields with the documented ranges
(0..2, 0..2, 0..3, −180..180, 0.1..10); first offending field named. Tests
pin rejections (216-255) and boundary acceptance (258-261). EXECUTED green.
— Bit-exact identity — verified: `apply_rgba8` short-circuits on
`is_neutral()` before any float conversion (150-154);
`neutral_params_leave_pixels_bit_unchanged` (202-207) pins it. EXECUTED.
— Buffer contract — verified: length-multiple-of-4 typed refusal (147-149),
`as_chunks_mut::<4>()` consumes the whole buffer (remainder provably empty),
alpha never written (px[3] untouched; pinned by
`saturation_zero_is_grayscale_but_alpha_survives`, 294).
— Per-buffer hoisting — verified: sin/cos computed once (157-160); no
allocation in the loop (module "Never" list honored).
— Clamp/cast safety — verified: `clamp01` clamps before `*255.0.round()`;
the `as u8` cast input is always in [0, 255] (or NaN→0 — see F-001); no
panic path exists in the file (no unwrap/expect/index arithmetic).
— Brightness/contrast/saturation math — verified against the tests by
trace: channel multiply ≡ luma multiply (linear), contrast around 0.5 with
the documented 128→129 rounding, saturation toward Rec.601 luma with 0 =
grayscale (test computes the same luma constant, 289).
— Executed: video_engine lib 48/48 green in the no-fail-fast baseline
(orchrun log `deps/video_engine` line 5915+); the feature-matrix evidence
string's "48/0 crate tests at that commit" is consistent with today's
baseline.

## 3. Findings

### F-001 [S3]
Where: crates/video-engine/src/adjust.rs:70-82 (struct) vs module doc 12-14; enforcement gap at apply_rgba8 146-154
Code:
```rust
pub struct AdjustParams {
    /// Multiplier on luma (`1.0` = unchanged).
    pub brightness: f32,
```
(the module doc's promise:)
```rust
//! Until then [`AdjustParams`]
//! carries the validated, clamped-by-contract values and
```
What is wrong: the struct's five fields are public, so the "validated"
invariant is unenforceable — `AdjustParams { brightness: f32::NAN, ..Default::default() }`
bypasses `AdjustParams::new` entirely, and `apply_rgba8` performs **no**
re-validation before using the values. The doc on `AdjustError::NotFinite`
says "NaN/∞ must never reach a shader" — with pub fields, nothing stops it.
Why it matters: today the failure mode is benign (NaN clamps to 0 — black
pixels; verified by trace of `clamp01`: `f32::clamp` propagates NaN,
`.round()` keeps it, `NaN as u8` = 0), but this struct's stated purpose is
to be the *uniform source* for a future GPU path, where a NaN uniform is
exactly the "must never reach a shader" event the typed error exists for.
A default-constructed or struct-literal-constructed value silently voids
the contract.
What correct looks like: private fields + getters, or a `validate()` call
at the top of `apply_rgba8` (and at the future uniform-upload site), or a
`#[doc(hidden)]`/test-only constructor escape hatch.
Confidence: HIGH — Verification: read (no re-validation anywhere in
apply_rgba8; `Default` returns NEUTRAL so only explicit literals can break
it — but they can, and pub-ness makes that a public-API fact).

### F-002 [S3]
Where: crates/video-engine/src/adjust.rs:182-186
Code:
```rust
            // Hue: rotate the (B−Y, R−Y) chroma pair by the hue angle.
            let cb = b - luma;
            let cr = r - luma;
            let r = luma + cr * cos_h - cb * sin_h;
            let b = luma + cb * cos_h + cr * sin_h;
```
What is wrong: the hue stage rotates only the red and blue chroma
components; **green is never reassigned** in the hue block. Consequences,
verified by hand-computing the pipeline: pure red (255,0,0) at hue 180° →
(0, 0, 152) — blue, where a true 180° hue rotation yields cyan (equal G/B
high); pure green (0,255,0) at hue 90° → (255, 255, 0) — yellow, where
truth is cyan; a green pixel's own G channel is immune to hue at every
angle. The in-file test `hue_180_rotates_red_chroma_to_blue_side` (329-339)
asserts the wrong outcome as correct ("and not toward green" — a true
rotation would move toward green as part of cyan).
Why it matters: this is the module's self-declared "byte-exact CPU
reference the GPU path must match" (module doc 14-15) — the approximation
("YPbPr-ish", comment 167) would be frozen into the shader contract and the
test oracle. A user sliding hue would see colors rotate along a visibly
wrong axis (nothing ever becomes or leaves green through G itself). The
ranges are "VLC-corroborated" (module doc 17) but the rotation model is
not VLC's.
What correct looks like: either a full 3-channel hue rotation (rotate in a
proper YPbPr/YCbCr space and re-derive R, G, B from rotated Pb/Pr), or an
explicit documentation of the 2-plane approximation as a deliberate
artistic choice with the green-immunity consequence stated — not a silent
"ish".
Confidence: HIGH — Verification: read + hand-computation (both example
pixels traced through lines 161-191; test body read at 329-339).

### F-003 [S4]
Where: crates/video-engine/src/adjust.rs:25 and 80
Code:
```rust
//! | gamma        | `0.1 ..= 10.0` [VERIFY exact VLC range] | `1.0`   |
```
(and the field doc at 80: "[VERIFY] exact upper range.")
What is wrong: an unresolved verification marker shipped in two doc
locations and echoed into the feature matrix (docs/feature-matrix.toml:1850
"gamma upper range 10.0 [VERIFY exact VLC range]"). The claim
"VLC-corroborated" (module doc 17) is therefore only 4/5 corroborated.
Why it matters: tracked honestly, but a shipped range that its own author
flags as unverified is exactly the class of claim this audit exists to
close out; VLC's actual gamma range is 0.01..=10.0, so the lower bound
0.1 is likely wrong too.
What correct looks like: verify against VLC's adjust filter (gamma range
0.01–10.0) and delete the marker.
Confidence: HIGH for the marker's existence (read); MEDIUM for VLC's exact
range (from general knowledge of VLC's adjust filter, not verified against
VLC source in this audit — hence S4, not a stronger claim).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `AdjustError` + Display + Error | 39-66 | REAL | — | Typed, field-named; test-pinned Display. |
| `AdjustParams` (struct) | 70-82 | REAL | S3 | F-001: pub fields void the validated-invariant claim. |
| `Default for AdjustParams` | 84-88 | REAL | — | Returns NEUTRAL. |
| `check` | 93-101 | REAL | — | Finite-before-range; single validation point. |
| `AdjustParams::NEUTRAL` | 106-112 | REAL | — | Const identity. |
| `AdjustParams::new` | 119-133 | REAL | — | Ranges only here (anti-drift comment 90-92 honored); first-offender order brightness→gamma. |
| `AdjustParams::is_neutral` | 136-138 | REAL | — | Exact equality against the const. |
| `AdjustParams::apply_rgba8` | 146-193 | REAL | S3 | F-001 (no re-validation), F-002 (hue leaves G untouched); identity short-circuit, hoisted sin/cos, safe clamp/cast, alpha preserved — the rest verified clean. |
| 11 tests | 201-355 | REAL, EXECUTED | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `neutral_params_leave_pixels_bit_unchanged` (202) | YES | Byte-exact compare over 16 pixels; any float round-trip breaks it. EXECUTED green. |
| `default_is_neutral_and_neutral_is_identity` (210) | YES | Const/Default equivalence. EXECUTED green. |
| `out_of_range_fields_are_rejected_by_name` (216) | YES | Five vectors incl. both hue signs. EXECUTED green. |
| `non_finite_fields_are_rejected` (244) | YES | NaN and +∞ vectors. EXECUTED green. |
| `boundary_values_are_accepted` (258) | YES | Both inclusive ends of every range. EXECUTED green. |
| `brightness_scales_pixels_and_zero_is_black` (264) | YES | 2.0 clamps to white, 0.0 to black. EXECUTED green. |
| `saturation_zero_is_grayscale_but_alpha_survives` (282) | YES | Luma recomputed in the test independently. EXECUTED green. |
| `contrast_pushes_away_from_midgray_and_clamps` (298) | YES | Rounding (128→129), clamp both ends, 0 → mid-gray. EXECUTED green. |
| `hue_180_rotates_red_chroma_to_blue_side` (329) | YES for the implemented model — but it ENSHRINES F-002's wrong axis | The assertion `px[2] > px[1]` ("not toward green") is only true because G never participates; a correct cyan-producing rotation would fail this test. |
| `bad_pixel_length_is_a_typed_error` (342) | YES | 3-byte buffer refused typed. EXECUTED green. |
| `error_display_names_the_field` (351) | YES | Display text pinned. EXECUTED green. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Neutral parameters are the identity … bit-unchanged (short-circuits before any float conversion)" | module doc 27-30 | REAL | Code 150-154 + test 202. EXECUTED. |
| "Out-of-range values are rejected at construction — no silent clamping of a control value" | module doc 30-32 | REAL | check/new/tests. EXECUTED. |
| "`AdjustParams` carries the validated, clamped-by-contract values" | module doc 12-14 | **SHALLOW** | True only for values built through `new`; pub fields make it unenforceable (F-001). |
| "ranges … VLC-corroborated" | module doc 17 | REAL for 4 fields, UNVERIFIED for gamma | F-003's [VERIFY] markers. |
| "the production path is a renderer-wgpu shader consuming these as uniforms … [PROPOSAL] follow-up" | module doc 8-12 | REAL (honest) | No renderer-wgpu reference exists (`rg` zero hits); matrix row says the same. |
| Feature row `vfeat-colors-adjust` evidence: "parameter model + CPU reference only — GPU/shader-uniform wiring [PROPOSAL] and the video.set-adjust catalog command (D-012) pending" | docs/feature-matrix.toml:1850 | REAL (honest partial) | Verified: zero production callers, CLI refuses `video.*`, UI sliders inert. The row's status is not inflated. |
| "hue rotation in a YPbPr-ish plane" | code comment 167 | REAL but understates | F-002: the "ish" is a wrong rotation axis (G never moves). |

## 7. Dead / duplicated / unreachable code
- The whole module is unreachable from shipped entry points (Field row);
  live only through the lib re-export (lib.rs:184) and the test suite.
- No duplication found.

## 8. Dependencies & flags
- No dependencies at all (pure std math + testkit in tests) — consistent
  with the video-engine manifest (W01-A01). No feature gates, no unsafe.

## 9. Open questions for the next agent
- renderer-wgpu's auditor (W03-A03/A06/A07 have renderer files): confirm no
  adjust uniforms exist yet anywhere in the shader pipeline (I verified no
  `AdjustParams` reference; a uniform named differently would need a
  shader-side look).
- If D-012 (`video.set-adjust`) lands, F-001 becomes reachable the moment
  the command maps host input into the struct — fix the fields first.
- VLC's actual adjust gamma range (0.01–10.0?) vs the shipped 0.1–10.0 —
  close F-003 with a source citation.
