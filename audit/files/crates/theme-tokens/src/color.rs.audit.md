# AUDIT: crates/theme-tokens/src/color.rs
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 358 / 16 items (5 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO shipped binary — dev pipeline crate: `cargo xtask tokens` (bin/tokens_gen.rs) → themes/generated/tokens.css → ui tokens:check (baseline #24 PASS); NO shipped crate depends on theme-tokens (rg over every Cargo.toml: only its own). Unit tests EXECUTED in baseline #3 (`cargo test --workspace` 1681 passed; test-workspace.log.txt:9 compiles theme-tokens) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read of the full file + hand-trace of every algorithm (HSL round-trip, hue_to_rgb t-normalization, WCAG luminance coefficients) · `rg -n "derive_ramp\|select_on_accent\|contrast_ratio\|parse_hex\|Shadow" crates/theme-tokens/src --glob '!color.rs'` (consumers: parse.rs:379/476/520/645, model.rs:6-63, lint.rs:253, generated/tokens.rs:23) · `rg -n "theme-tokens" --glob '**/Cargo.toml'` (no dependents outside the crate) · `rg -n "bright_dark\|bright_light" emit.rs` (:226-227, :457-458) + spot-check of emitted values against themes/generated/tokens.css (orange #e8851a dark-bright, gray tinted #1e1e1e, gray strong #989898 — all match the derivations) · `ls crates/theme-tokens/tests/` (diff.rs drift test exists — pins committed output to regeneration) · baseline: test-workspace.log.txt (#3, 1681 passed), ui-tokens_check.log.txt (#24 EXIT=0) |

## 1. One-paragraph verdict
This is the color-math foundation of the token pipeline, and it is correct where it matters: the HSL round-trip is the standard algorithm (verified by hand, including the tie-breaking `match (max == r, max == g, max == b)` and the t-normalization in `hue_to_rgb`), the WCAG relative-luminance/contrast functions use the right coefficients and thresholds, hex parsing is strict and loud, and `derive_ramp`'s formulas verifiably reproduce the committed tokens.css values (spot-checked orange and gray ramps). The tests are mutation-sensitive — the tinted-dark test pins the exact mined #371F06 (a 0.12-vs-0.1196 lightness change fails it by one green step, per the comment's own arithmetic), and the fall-through test pins the violet→white on-accent decision. The whole chain (this file → emit → committed tokens.css) is diff-tested (tests/diff.rs) and the UI side is drift-checked (baseline #24). The two findings are documentation-level: the header claims "add/mul/div only — bit-deterministic" while the contrast path uses `powf` (not bit-deterministic across libm implementations, mitigated by being decision-only), and `bright_dark` is a field that can never differ from `bright_light` by construction.

## 2. What actually works (verified)
— Emission-path determinism holds: rgb_to_hsl / hsl_to_rgb / derive_ramp use only add/mul/div/round/clamp — the committed tokens.css is byte-reproducible, and tests/diff.rs pins it (crate test suite ran green in baseline #3, 1681 passed).
— Derivation correctness spot-verified against output: gray accent (s=0) → strong #989898, tinted_dark hsl(h,0,0.1196) → #1e1e1e, orange bright #e8851a — all present in themes/generated/tokens.css with the predicted values; `--color-content-on-accent: #ffffff` for violet and `#eaeaea` for blue match the fall-through test's assertions.
— parse_hex strictness: 7-char `#rrggbb` only; `#0909`, `090909`, `#09090g` all rejected (test :342-344); `as_chunks::<2>()` on exactly-6 bytes is remainder-free by construction.
— select_on_accent: first-pass-wins over the declared candidate order (documented dark-glyph-first), loud error carrying the best ratio when nothing passes; the empty-candidates case errors rather than defaulting.
— Every public item has an in-crate consumer (parse.rs, model.rs, lint.rs, emit.rs, generated/tokens.rs — rg-verified); nothing is dead.

## 3. Findings

### F-001 [S4]
Where: crates/theme-tokens/src/color.rs:3-4 (claim) vs :167 (code)
Code:
```
//! Pure `f64` math (add/mul/div only — bit-deterministic on every platform we
//! target). All derivation rules are calibrated on the mined orange accent
```
```
            ((v + 0.055) / 1.055).powf(2.4)
```
What is wrong: the header's "add/mul/div only — bit-deterministic" is false for this file: `relative_luminance` uses `powf(2.4)`, and `f64::powf` is not guaranteed bit-identical across libm implementations/platforms (it is not a correctly-rounded operation in general).
Why it matters: the claim is the crate's stated guarantee for regenerating committed outputs; anyone extending the code on the assumption that "everything here is add/mul/div" could route emission values through the contrast path and inherit cross-platform drift. Today the exposure is bounded — `powf` feeds only `contrast_ratio`/`select_on_accent` THRESHOLD decisions (≥4.5:1 with wide margins), never emitted color values — so committed tokens remain reproducible.
What correct looks like: scope the claim: "the emission path (ramp derivation) is add/mul/div/round only and bit-deterministic; the contrast path uses powf and is decision-only (thresholds, not emitted values)."
Confidence: HIGH — Verification: read; the mitigation (decision-only use) verified by tracing contrast_ratio's callers (select_on_accent threshold + lint).

### F-002 [S4]
Where: crates/theme-tokens/src/color.rs:222-223 (doc) and :278 (construction)
Code:
```
    /// Hover-bright accent in DARK mode (w26 Lead: the contract §2 hover —
    /// dark hover previously equaled the primary, an invisible hover state).
    /// Same formula as bright_light: `hsl(h, s + 0.02, l + 0.045)`.
```
```
        bright_dark: bright_light,
```
What is wrong: `bright_dark` and `bright_light` are two public fields that are always identical by construction — the model carries a distinction the data can never express. emit.rs (:226-227) consumes them per-mode, so the two emitted CSS values can never diverge until someone edits one formula; the "distinction" is purely nominal today.
Why it matters: a maintainer tuning dark-mode hover (the very reason the field exists) must know the values are currently aliased, or a "fix" to one silently changes both modes; conversely a test asserting they differ can never pass.
What correct looks like: either compute `bright_dark` with its own expression (even if currently the same formula, spelled out) so the fields have independent definitions, or document at the construction site that they are deliberately aliased and the seam activates in a future lane.
Confidence: HIGH — Verification: read + emit.rs:226-227/:457-458 consumer trace.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Rgb8 (struct + const new) | 11-26 | REAL | — | trivial, correct |
| Rgb8::parse_hex | 29-43 | REAL | — | strict 7-char; as_chunks remainder-free; loud errors |
| Rgb8::to_hex | 46-48 | REAL | — | lowercase canonical form; round-trip test-pinned |
| Display for Rgb8 | 51-55 | REAL | — | delegates to to_hex |
| hex_digit | 57-64 | REAL | — | case-insensitive, loud error |
| Shadow (struct) | 68-82 | REAL | — | plain data; consumed by parse.rs:476/509, model.rs |
| Hsl (private struct) | 85-90 | REAL | — | h degrees, s/l 0..1 |
| rgb_to_hsl | 92-116 | REAL | — | standard algorithm, hand-verified incl. tie-break arm and achromatic epsilon branch; h ∈ [0,360) |
| hue_to_rgb | 118-135 | REAL | — | t-normalization (t<0 → +1, t>1 → −1) verified for the h±⅓ channel offsets |
| hsl_to_rgb | 137-151 | REAL | — | grayscale early-return on exact s==0 (holds: derived s is exactly 0 for achromatic input); round+clamp per channel |
| contrast_ratio | 154-159 | REAL | — | WCAG (hi+0.05)/(lo+0.05); order-independent |
| relative_luminance | 161-171 | REAL | S4 | correct coefficients (0.2126/0.7152/0.0722) and 0.04045 threshold; powf → F-001 |
| MIN_ON_ACCENT_CONTRAST | 174 | REAL | — | 4.5 per doc 08 §3.3 |
| select_on_accent | 184-201 | REAL | — | first-pass-wins over declared order (documented); error carries best ratio |
| AccentRamp (struct) | 206-228 | REAL | S4 | F-002 bright_dark ≡ bright_light |
| derive_ramp | 243-282 | REAL | — | formulas verified against committed tokens.css (orange/gray spot-checks); sat-boost guard keeps gray achromatic (test-pinned) |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| tinted_dark_rule_reproduces_mined_value (:291) | YES | pins exact #371F06 AND on_accent #161616 — a one-step lightness change (0.12) or candidate-order change fails it |
| contrast_of_mined_play_glyph_passes (:306) | YES | fails if luminance math or the 4.5 constant regresses below threshold |
| dark_accents_fall_through_to_light_glyphs (:317) | YES | pins blue→#EAEAEA and violet→#FFFFFF decision boundaries |
| hex_roundtrip (:332) | YES | covers case normalization + three malformed-input rejections |
| gray_accent_stays_achromatic (:349) | YES | fails if the sat-boost guard is removed (gray would gain a chromatic bright) |
All executed: baseline #3 (cargo test --workspace, 1681 passed; theme-tokens compiled at test-workspace.log.txt:9). The crate-level tests/diff.rs additionally pins the committed themes/generated/tokens.css to regeneration.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Pure f64 math (add/mul/div only — bit-deterministic on every platform we target)" | file header :3-4 | FALSE for the file / REAL for the emission path | powf at :167 (F-001); emission path (derive_ramp) is add/mul/div/round — verified |
| "tinted_dark = hsl(h, s, 0.1196) … reproduces #371F06 exactly" | :234-236 | REAL | test :300 asserts equality; committed tokens.css --color-accent-tinted #371f06 |
| "the violet accent reaches 4.5:1 against none of the mined values" (white fallback rationale) | :181-183 | REAL | test :325-327 pins violet→#FFFFFF |
| "same formula as bright_light" (bright_dark) | :222-223 | REAL (and the problem — see F-002) | :278 aliases the field |

## 7. Dead / duplicated / unreachable code
- None dead: every public item consumed in-crate (rg trace). F-002's aliasing is the only redundancy.
- Not shipped: no runtime crate links theme-tokens (dev pipeline only) — by design (boundaries comment in Cargo.toml: "xtask calls crates/theme-tokens").

## 8. Dependencies & flags
- std only (no crates.io deps — consistent with a foundation crate). No feature flags. `as_chunks` requires the repo's pinned modern toolchain (Rust 1.99.1 per baseline INDEX) — fine.

## 9. Open questions for the next agent
- emit.rs (W15-A10 has part1): the mined-vs-derived mix — orange light-mode bright #b35f0d in tokens.css is NOT derive_ramp's bright_light (which is #e8851a); confirm tokens.json declares the light-mode override so the pipeline's precedence is explicit.
- lint.rs / semantic_lint.rs owners: contrast_ratio's powf also feeds lint decisions — same F-001 mitigation applies (decision-only).
