# AUDIT: crates/theme-tokens/src/color.rs
| Field | Value |
|---|---|
| Auditor | W11-A06 |
| Date | 2026-11-23 |
| Lines / functions | 358 / 10 (Rgb8::new, Rgb8::parse_hex, Rgb8::to_hex, Display, hex_digit, rgb_to_hsl, hue_to_rgb, hsl_to_rgb, contrast_ratio, relative_luminance, select_on_accent, derive_ramp + 5 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (build-time pipeline). Chain: `parse.rs:645 color::derive_ramp` → TokenSet → `emit::emit_css` (tokens-gen bin) → themes/generated/tokens.css → verbatim copy ui/src/tokens/generated/tokens.css → ui/src/main.tsx:27. Its computed VALUES ship in the UI; the functions themselves run only under `cargo xtask tokens` / tests. No runtime caller (`contrast_ratio` used only by color.rs + tests/pipeline.rs:174). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | python3 WCAG reimplementation: 9 accents × 4 candidates = 36 contrast ratios -> all match the crate's committed on_accent picks; python3 colorsys HSL reimplementation: 4 accents × 5 derived roles = 20 values -> ALL byte-equal to the committed ACCENT_* constants; baseline cargo-test-workspace.txt:6259-6264 (5 color::tests pass). |

## 1. One-paragraph verdict
The color math is correct, and I proved it independently: a from-scratch Python reimplementation of the documented HSL derivation formulas reproduces all 20 committed derived-ramp constants (red/blue/violet/gray × strong/tinted_dark/tinted_light/bright_light/icon_light) byte-for-byte, and a WCAG reimplementation reproduces the on-accent selection for all 9 accents. Hex parsing is strict and loud (exact `#rrggbb`, rejects 4/3/8-digit forms and non-hex digits). Alpha exists only in `Shadow` (via parse.rs rgba(), range-checked 0..=1) — the color model is deliberately opaque-RGB. The only defects are documentation-level: the header claims "add/mul/div only — bit-deterministic" while `relative_luminance` calls `powf(2.4)`, and the [PROPOSAL] calibration status means 8 of 9 ramps are formula-derived stand-ins for unmined Figma data.

## 2. What actually works (verified)
— verified by python3 (colorsys) recomputation of `hsl(h,s,l−0.09) / hsl(h,s,0.1196) / hsl(h,s×0.56,0.725) / hsl(h,s+0.02,l+0.045) / hsl(h,s−0.03,l−0.03)` for red/blue/violet/gray -> every result equals the committed `ACCENT_{ID}_{ROLE}` constant in crates/theme-tokens/src/generated/tokens.rs:184-247 (e.g. red strong `#c83a1d`, violet icon_light `#8536fc`, gray tinted_dark `#1e1e1e`). rgb↔hsl conversion is correct.
— verified by python3 WCAG-2.x ratio computation for all 9 accent primaries × 4 candidates -> the first candidate ≥ 4.5:1 matches the committed on_accent for every accent (orange #161616 5.617, blue #EAEAEA 4.924, violet #FFFFFF 4.802, red #161616 4.649...).
— tightest threshold margin is red/#161616 at 4.649 (0.149 above 4.5) — no selection sits near the boundary, so floating-point drift cannot flip a pick in practice.

## 3. Findings
F-001 [S4]
Where: crates/theme-tokens/src/color.rs:3-4 vs 167
Code:
```
//! Pure `f64` math (add/mul/div only — bit-deterministic on every platform we
//! target). All derivation rules are calibrated on the mined orange accent
```
and
```
            ((v + 0.055) / 1.055).powf(2.4)
```
What is wrong: The header's determinism argument ("add/mul/div only") is false — `relative_luminance` uses `powf`, whose results are libm-dependent across platforms/versions. Why it matters (limited): on_accent selection feeds the byte-stability contract (doc 08 §3.4); a powf discrepancy near the 4.5 threshold could flip a candidate and drift the committed CSS. Measured margin (0.149 minimum) makes an actual flip implausible, so this is a doc-accuracy defect, not a live bug. What correct looks like: either drop the "add/mul/div only" claim or implement the sRGB curve with integer/fixed-point exponentiation. Confidence: HIGH — Verification: read + ran (margin computed). 
F-002 [S4]
Where: crates/theme-tokens/src/color.rs:28-33
Code:
```
    /// Parse a `#rrggbb` hex string (case-insensitive, exactly 7 chars).
    pub fn parse_hex(s: &str) -> Result<Self, String> {
        let bytes = s.as_bytes();
        if bytes.len() != 7 || bytes[0] != b'#' {
            return Err(format!("expected `#rrggbb`, got `{s}`"));
```
What is wrong: Only `#rrggbb` is accepted — no `#rgb`, `#rrggbbaa`, `rgb()`/`hsl()` functional forms for plain color tokens (rgba() with alpha exists ONLY for shadow colors, parse.rs:519-547). This is a loud, documented-subset rejection (errors are actionable), not silent misparse — but a Figma export emitting `#rrggbbaa` or 3-digit hex will fail regeneration with "expected `#rrggbb`". Deliberate scope; hygiene note for the future re-pin. Confidence: HIGH — Verification: read + hex_roundtrip test (rejects `#0909`, `090909`, `#09090g`).
F-003 [S4]
Where: crates/theme-tokens/src/color.rs:4-6 and 230-242
Code:
```
//! family (doc 08 §1.2) and are `[PROPOSAL]` until the Figma variables export
//! arrives: re-pin per doc 08 §2/§3.3.
```
What is wrong: 8 of the 9 shipped accent ramps are formula-derived stand-ins for design values that were never supplied (doc 08 §3.3: "For the 8 alternate accents we only have the primary swatch [VERIFY-DESIGN]"). The shipped UI renders these computed colors as if they were design-pinned. Honestly labeled [PROPOSAL] in both code and doc — recorded here so the matrix row "9 accent selections" is not read as 9 designed ramps. Confidence: HIGH — Verification: read (doc 08 §3.3 + tokens.json primaries-only for alternates).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Rgb8::new | 24-26 | REAL | — | const fn for generated constants |
| Rgb8::parse_hex | 29-43 | REAL | S4 | strict 7-char form; F-002 scope |
| Rgb8::to_hex | 46-48 | REAL | — | lowercase canonical; roundtrip tested |
| Display | 51-55 | REAL | — | delegates to to_hex |
| hex_digit | 57-64 | REAL | — | 0-9a-fA-F, loud error |
| rgb_to_hsl | 92-116 | REAL | — | standard formula; achromatic epsilon guard prevents /0 (max==min==1 impossible in the l>0.5 branch); verified via colorsys cross-check |
| hue_to_rgb | 118-135 | REAL | — | single ±1 wrap sufficient (t inputs in [-1/3, 4/3]) |
| hsl_to_rgb | 137-151 | REAL | — | round+clamp per channel; s==0 achromatic fast path |
| contrast_ratio | 154-159 | REAL | — | symmetric WCAG ratio; matches my python reimpl for 36 pairs |
| relative_luminance | 161-171 | REAL | S4 | correct WCAG linearization; powf contradicts header (F-001) |
| select_on_accent | 184-201 | REAL | — | first-pass ≥ 4.5; empty list errors; best ratio in error msg |
| derive_ramp | 243-282 | REAL | — | implements exactly the 6 documented formulas (20/20 values verified); bright_dark := bright_light (w26 hover fix) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| tinted_dark_rule_reproduces_mined_value | YES | exact #371F06 + #161616 assertions against real mined values |
| contrast_of_mined_play_glyph_passes | YES | threshold assertion on the real pair |
| dark_accents_fall_through_to_light_glyphs | YES | exact picks for blue (#EAEAEA) and violet (#FFFFFF) — exercises the fall-through order |
| hex_roundtrip | YES | 5 roundtrips + uppercase normalize + 3 rejections |
| gray_accent_stays_achromatic | YES | asserts r==g==b on strong/tinted_light (does not assert bright/icon — minor coverage gap; sat_boost skip at s<0.05 covers it in code) |
All 5 pass in baseline (cargo-test-workspace.txt:6259-6264). Unit test binary not present in target/debug/deps; could not re-run locally — baseline cited.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Pure f64 math (add/mul/div only — bit-deterministic)" | color.rs:3-4 | FALSE (powf used) | color.rs:167; practical risk nil (margin 0.149 min, computed) |
| "tinted_dark = hsl(h, s, 0.1196) reproduces #371F06 exactly" | color.rs:234-236 | REAL | my colorsys reimpl → #371f06; test asserts it |
| "violet reaches 4.5:1 against none of the mined values" | color.rs:182-183 | REAL | python: violet vs #161616 3.769, #090909 4.147, #EAEAEA 3.991 — all < 4.5; #FFFFFF 4.802 passes |
| "on-accent glyphs need ≥ 4.5:1" (doc 08 §3.3) | color.rs:174-176 | REAL | all 9 committed on_accent picks verified ≥ 4.5 by reimplementation |
| ramps "[PROPOSAL] until the Figma variables export arrives" | color.rs:5-6 | REAL (honest gap) | alternates are derived, not mined (F-003) |

## 7. Dead / duplicated / unreachable code
- No dead code. `contrast_ratio` and `select_on_accent` are pub but consumed only inside the crate/tests — acceptable for a pipeline crate.

## 8. Dependencies & flags
- std-only (fmt). No features, no unsafe. Correctness depends on nothing external.

## 9. Open questions for the next agent
- When the Figma re-pin lands, will parse_hex need #rrggbbaa/#rgb support (F-002), or does the export guarantee 6-digit form?
- The tokens.css audit (file 4) should confirm the UI actually renders bright_dark distinctly from primary in dark mode (the w26 hover contract §2).
