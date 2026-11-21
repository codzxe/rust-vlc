# AUDIT: crates/theme-tokens/src/generated/tokens.rs
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 247 / 186 `pub const` (4 metadata: MODES, DEFAULT_MODE, ACCENTS, DEFAULT_ACCENT; 111 semantic incl. per-mode suffixed; 72 ramp = 9 accents × 8 roles) + 1 `use` |
| File verdict | REAL (generated artifact, byte-stable, diff-gated) — consumed by NOBODY (see F-001) |
| Reachable from shipped entry point? | NO: the crate `theme-tokens` has zero dependents (no workspace Cargo.toml lists it); the module compiles inside the crate (`#[path] pub mod generated`, lib.rs:40-42) but no engine/CLI/tauri code references any constant. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `grep -c "pub const"` (186); `grep -n ACCENT_BLUE_ON_ACCENT …` (cross-format value check vs tokens.css); `sed -n '38,173p'` section census (111 semantic + 72 ramp + 4 metadata); `python3 json.load(themes/tokens.json)` (source has primitive/alias/semantic layers — the 123/86/86 header claim's substrate exists); baseline #3 (diff.rs byte-equality test passes), #24 (`npm run tokens:check` PASS — the UI sibling copy is fresh) |

## 1. One-paragraph verdict
A model generated artifact: header carries provenance (generator, source, diff test), the 186 constants are sorted deterministically, per-mode values carry the `_DARK`/`_LIGHT` suffixes exactly where the CSS sheet has `[data-theme]` blocks (spot-checked COLOR_CONTENT_TERTIARY_DARK #828282, COLOR_CONTENT_ACCELERATOR_DARK #6b6b6b — both match my independently computed values from the menu.css/list.css audits), the per-accent ON_ACCENT ramps encode what tokens.css expresses as `[data-accent]` overrides (ACCENT_BLUE_ON_ACCENT #eaeaea, ACCENT_VIOLET_ON_ACCENT #ffffff — both verified), and the committed copy is proven byte-identical to regeneration by a passing diff test with a drift-sensitivity proof. The single defect is inherited from its generator (my emit.rs#part2 audit F-001): every one of these 186 constants is written for an "engine-side consumer" that does not exist — the artifact is maintained, freshness-enforced, and read by no production code.

## 2. What actually works (verified)
— verified by baseline #3 → `diff.rs::committed_outputs_match_regeneration` passes: this exact file equals `emit_rust(parse(themes/tokens.json))` byte-for-byte at HEAD.
— verified by value cross-checks against `ui/src/tokens/generated/tokens.css` (same source, other emitter): COLOR_ACCENT #d37918, TERTIARY_DARK #828282, ACCELERATOR_DARK #6b6b6b, ACCENT_BLUE_ON_ACCENT #eaeaea, ACCENT_VIOLET_ON_ACCENT #ffffff — all match; the two emitters agree on every spot-checked token (no cross-format drift; the f32 narrowing concern from emit.rs F-003 does not bite on current values).
— verified by `python3` source read → themes/tokens.json really has the primitive/alias/semantic layer structure the header counts (123/86/86).
— verified by `grep -c` → 72 ramp constants = 9 accents × 8 roles, matching the role table in the header (j18 F2's "all eight roles" fix).

## 3. Findings

### F-001 [S3]
Where: crates/theme-tokens/src/generated/tokens.rs:8-9 (the stamped claim) and the whole 186-const body
Code:
```
//! Semantic-layer constants for engine-side consumption (doc 01 §2.3,
//! doc 08 §3.1). Components and engine code consume SEMANTIC tokens
```
What is wrong: the header claims engine-side consumption; the tree has none. No crate depends on `theme-tokens` (Cargo.toml census in my emit.rs#part2 audit), no `use theme_tokens::` exists outside the crate, and not one of the 186 constants is referenced by any engine, CLI, or tauri code. The artifact is 247 lines of compile-checked, diff-enforced dead weight — the CSS sibling of the same pipeline drives the entire UI, making the asymmetry stark.
Why it matters: every regeneration cycle and CI diff assertion spends effort proving the freshness of an artifact nobody reads; the false present-tense claim in a GENERATED header (re-stamped on every regen) will mislead every future reader into assuming an engine consumer exists.
What correct looks like: an engine consumer (e.g. the software compositor's clear color, the EQ band colors COLOR_EQ_1..7 which are exactly the kind of value an audio visualization would need) or deleting the Rust emission until the consumer card lands.
Confidence: HIGH — Verification: consumer census greps (source + Cargo.toml) + this file's own diff-test pass; absence is proven, not inferred.
Cross-ref: emit.rs#part2 F-001 (same finding from the generator side — keep one canonical write-up, this one).

## 4. Per-function table (generated constants — grouped)
| Group | Lines (approx) | Verdict | Severity | Notes |
|---|---|---|---|---|
| `use crate::color::{Rgb8, Shadow}` | 27 | REAL | — | compiles inside theme-tokens |
| `MODES` / `DEFAULT_MODE` | 28-31 | REAL | S3 | `["dark","light"]` / "dark" — matches tokens.css `:root[data-theme="light"]` block set |
| `ACCENTS` / `DEFAULT_ACCENT` | 32-41 | REAL | S3 | 9 ids, `#[rustfmt::skip]` — order is the CSS `[data-accent]` order |
| Semantic: modeless (COLOR_ACCENT, COLOR_EQ_1..7, SPACE_*, RADIUS_*, OPACITY_*…) | 44-173 | REAL | S3 | sorted by name; values cross-verified vs CSS on 5 tokens; SHADOW_CARD/SHADOW_OVERLAY as Shadow literals (multiline — the rustfmt-stable form emit.rs documents) |
| Semantic: per-mode suffixed (*_DARK/*_LIGHT) | interleaved | REAL | S3 | suffix exactly where CSS has per-theme blocks; ON_ACCENT modeless here because per-ACCENT variants live in the ramps (design-consistent with tokens.css) |
| Ramp constants ACCENT_{ID}_{ROLE} ×72 | 174-247 | REAL | S3 | declared accent order (not sorted — intentional, documented); BLUE/VIOLET ON_ACCENT values verified |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| diff.rs committed-outputs test (baseline-passed) | YES | byte-equality vs regeneration catches ANY edit-by-hand or generator drift, with the mutation sensitivity test proving non-vacuity |
| (absence) no test consumes the constants semantically | — | nothing asserts e.g. COLOR_ACCENT == the CSS --color-accent at runtime — unnecessary given both derive from one source, and impossible for a consumer that doesn't exist |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Semantic-layer constants for engine-side consumption (doc 01 §2.3, doc 08 §3.1)" | tokens.rs:8-9 (generated header) | FALSE | zero consumers (F-001) |
| "Produced by `cargo xtask tokens` … Regeneration is idempotent; the committed copy is diff-tested" | tokens.rs:3-6 | REAL | diff.rs + baselines #3/#24 |
| "Source shape: 123 primitive / 86 alias / 86 semantic tokens" | tokens.rs:22-23 | REAL | tokens.json layer structure verified; counts are the parser's territory (parse.rs — other agent) |
| "Modes (2) … Accents (9)" | tokens.rs:13-16 | REAL | MODES/ACCENTS consts match the CSS sheet's block census |

## 7. Dead / duplicated / unreachable code
The entire file is unreachable-as-consumed (F-001): real, correct, maintained — and read by no production code.

## 8. Dependencies & flags
`crate::color::{Rgb8, Shadow}` only. No flags. Regeneration: `cargo run -p theme-tokens --bin tokens-gen` (xtask bridge).

## 9. Open questions for the next agent
- The EQ band colors (COLOR_EQ_1..7) look purpose-built for an audio-spectrum visualizer — if the player's equalizer/visualizer tools (ui/src/screens/player/tools/*, in this wave) hard-code the same hexes in CSS instead of consuming them, that's the two-sources drift risk noted in my emit.rs#part2 audit, question 1. My files 17/21 (visualizer/equalizer harnesses) will check the UI side.
