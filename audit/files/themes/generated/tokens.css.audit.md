# AUDIT: themes/generated/tokens.css
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 262 / 18 blocks (86 + 24 + 8×6 + 8×3 declarations) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES, transitively: ui/src/main.tsx:27 imports ui/src/tokens/generated/tokens.css — the VERBATIM copy of this file (byte-identical, diff-verified) — so every UI route consumes this token set. The crate-side copy is consumed by crates/theme-tokens/src/generated/tokens.rs consumers. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |
| Commands run | diff themes/generated/tokens.css ui/src/tokens/generated/tokens.css (IDENTICAL); python declaration-count per block (86/24/6×8/3×8); rg consumer census for 20 token names across ui/src; tokens.json provenance reads (content.bright, shadow.overlay); crates/theme-tokens/tests/diff.rs read in full. |

## 1. One-paragraph verdict
A GENERATED file (header: "DO NOT EDIT", produced by `cargo xtask tokens` from themes/tokens.json) — and the generation contract is enforced, not just claimed: crates/theme-tokens/tests/diff.rs re-parses the committed source, re-emits CSS+Rust, and byte-compares BOTH committed copies (themes/generated and the ui bridge copy) — and a second test mutates one primitive value to prove the gate would detect drift (non-vacuous). That test ran in the baseline workspace test pass (INDEX.md row 3: 1681 passed), so the committed file provably equals regeneration. Structure matches the header exactly: 86 semantic tokens in :root (counted), 24 light-mode overrides, 9 accents × (6 dark + 3 light) declarations, 18 blocks. No findings against this file itself; three consumer-side observations are recorded as cross-refs (the font-family-time≡sans degeneracy already filed at dialogs.css F-001; a light-mode role collapse disclosed in source; 6 semantic tokens with zero UI consumers — supply-side, belongs to the tokens.json audit).

## 2. What actually works (verified)
- Generation/drift gate: diff.rs:80-107 `committed_outputs_match_regeneration` byte-compares this file AND ui/src/tokens/generated/tokens.css against fresh emission; `the_comparison_detects_drift` (:110-130) mutates gray.700 and asserts output differs — the equality test is provably sensitive. Baseline row 3 (cargo test --workspace, 1681 passed) executed it — the committed copy IS the regenerated output.
- Dual-copy contract: `diff` of the two committed tokens.css copies → byte-identical; diff.rs:14-15 documents the bridge copy is written by ui/scripts/gen-tokens.mjs and gates staleness "from the Rust side of the fence" (ui.yml has no tokens step — the gap is closed here).
- Structure claims: header says "Tokens: 123 primitive / 86 alias / 86 semantic" — the 86 semantic emission counted in :root (29-114); "Modes (2) / Accents (9)" — light block + 8 non-default accent blocks present, each with its per-light sub-block (18 blocks total, counted).
- Values are frame-mined per source descriptions (spot-read: content.bright dark=gray.100/light=gray.750 with measured-light rationale; shadow.overlay 0/4.76/5.9/0.7 "floating-window shadow — neutral black, measured").
- Consumer breadth: 20-token spot census — space-track-inset ×22, radius-hairline ×10, space-item-pitch ×7, font-size-sidebar ×5 consumers etc.; the accent ramp drives ThemeProvider data-accent switching (ThemeProvider consumes ACCENTS from the ui bridge's tokens.ts).

## 3. Findings
None against this file. It is a faithful, drift-gated emission. Cross-refs (owned elsewhere):
1. **--font-family-time ≡ --font-family-sans** (:64-65, byte-identical Inter stacks) — filed as dialogs.css F-001 (my file 2); the ROOT is themes/tokens.json:376's "timecode face — same UI stack" design decision — belongs to the tokens.json audit (W15-A01 parts 1/2).
2. **Light-mode role collapse**: --color-content-bright ≡ --color-content-primary (#161616, :122-123) while dark keeps them distinct (#eaeaea vs #c6c6c6). Disclosed in source ("light value measured (#161616 emphasis glyphs, Player - Light.svg)") — the emphasis hierarchy vanishes in light mode by measurement, not by accident. tokens.json audit's call.
3. **6 semantic tokens with ZERO ui/src consumers**: --font-size-micro (6px), --radius-app, --stroke-window, --motion-duration-slow, --motion-duration-step, --opacity-scrim-subtle (census: 0 hits each). The semantic layer is "the only consumable layer" (doc 08 §3.2) — 7% of the supply is unconsumed. Supply-side question for the tokens.json audit; NOT a defect of this emission.
4. **surface.css shadow citation drift** (consumer-side, cross-ref to my file 7): surface.css:12-14 cites "0 8px 15px rgb(0 0 0 / 46%) → --shadow-overlay" for dialog surfaces, but the token is 0px 4.76px 5.9px rgba(0,0,0,0.7) (this file :105, source-measured). The comment maps a different measurement onto the token — noted as an addendum in the surface.css audit.

## 4. Per-function table (block layout — generated CSS has no functions)
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| :root (default dark + orange) | 28-115 | REAL | — | 86 semantic declarations (counted = header claim) |
| :root[data-theme="light"] | 117-142 | REAL | — | 24 overrides; content roles collapse bright≡primary (cross-ref §3.2) |
| 8 × :root[data-accent="…"] | 144-256 (interleaved) | REAL | — | 6 declarations each: accent/bright/icon/strong/tinted/on-accent; violet on-accent #ffffff (the 4.5:1 fallback candidate per tokens.json accent-derivation note) |
| 8 × :root[light][accent] | interleaved | REAL | — | 3 declarations each: bright/icon/tinted |
| Header comment | 1-26 | REAL | — | Count/mode/accent claims verified by counting |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| crates/theme-tokens/tests/diff.rs `committed_outputs_match_regeneration` | YES | Byte equality on both committed copies vs fresh emission; ran in baseline row 3 |
| `the_comparison_detects_drift` | YES (meta) | Mutation-sensitivity proof — the equality test is not vacuous |
| ui/src/tokens/tokens.test.ts (ui bridge) | cross-ref | Runs in vitest pass (row 23) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "GENERATED FILE — DO NOT EDIT … diff-tested by crates/theme-tokens/tests/diff.rs" | :1-6 | REAL | diff.rs exists, wired into cargo test; baseline executed |
| "Tokens: 123 primitive / 86 alias / 86 semantic" | :24-25 | REAL (86 semantic counted) | python count: 86 :root declarations; 123/86 are source-side counts (tokens.json audit to confirm) |
| "Modes (2): dark (default), light. Accents (9)" | :22-23 | REAL | 18 blocks counted |
| "The primitive and alias layers are never emitted here" | :9-10 | REAL | All 86 names are semantic-layer names (spot-verified against alias paths in tokens.json) |
| ui copy is "the verbatim UI-side bridge copy" | diff.rs:14-15 | REAL | diff → byte-identical |

## 7. Dead / duplicated / unreachable code
The dual physical copies are intentional and gate-enforced (not duplication drift). 6 zero-consumer semantic tokens listed in §3.3 — supply-side, cross-ref.

## 8. Dependencies & flags
Source: themes/tokens.json (W15-A01 parts 1/2). Generator: crates/theme-tokens (emit.rs part1 = W15-A10). Bridge: ui/scripts/gen-tokens.mjs (npm run tokens:generate; tokens:check = the ui-side gate, baseline row 24 PASS). No flags. This file must never be hand-edited — findings against VALUES route to tokens.json; findings against EMISSION route to emit.rs.

## 9. Open questions for the next agent
- tokens.json audit (A01): confirm the 123/86 counts and rule on the 6 unconsumed semantic tokens and the light-mode bright≡primary collapse.
- surface.css audit addendum (my file 7): the shadow-citation drift cross-ref (§3.4 here).
- --font-size-micro: 6px — currently unconsumed (good; a 6px text token is an a11y hazard if ever adopted). tokens.json audit should consider deleting it.
