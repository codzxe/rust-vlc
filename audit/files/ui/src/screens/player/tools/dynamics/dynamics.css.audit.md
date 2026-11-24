# AUDIT: ui/src/screens/player/tools/dynamics/dynamics.css
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 300 / 17 rule blocks (card/toggle/rows/slider/ticks/motion/light-theme) |
| File verdict | REAL — token-pure presentational sheet for honestly-STAGED tools |
| Reachable from shipped entry point? | YES (DynamicsCard/CompressorTool/SpatializerTool/StereoWidenerTool import it via tools/dynamics; tools dialog reachable from PlayerScreen) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg token vars in src/tokens/generated/tokens.css (all 20 used vars resolve, exit 0); rg `dynamics-tool|staged` tools/*.tsx (exit 0) |

## 1. One-paragraph verdict
Clean, frame-measured stylesheet: every color/typography/spacing value is a semantic token (I verified all 20 custom properties used here exist in src/tokens/generated/tokens.css — 20/20 resolve); raw px values are structural dimensions documented per-line against the source SVG frames. The sheet styles the Compressor/Spatializer/Stereo-Widener tools, which are STAGED surfaces: their TSX carries `data-staged-tool` markers and "staged" notes (tests assert them — SpatializerTool.test.tsx:113-119), because NO audio-effect command exists in the catalog at all (no compressor/spatializer/widener ids in ui/src/generated/commands.ts CommandRequest union — verified by reading the union). So this CSS is real styling for honestly-labeled dead-weight UI: the sliders are local-state-only until an engine seam exists.

## 2. What actually works (verified)
— verified by rg: all 20 token vars resolve in src/tokens/generated/tokens.css (counts 1-18 each, none 0). Reduced-motion honored (L253-257). Light-theme remap rules are coherent (L276-300, no orphan overrides).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/tools/dynamics/dynamics.css:111-117
Code:
```
/* Staged note — the honest-unbound marker (effects-area pattern): quiet
 * caption line, never styled like an engine status. */
.dynamics-tool__staged {
```
What is wrong: nothing in the CSS — the note it styles is the honesty marker for tools that have no backend command (staged display ranges only). The entire sheet styles controls whose only effect is local React state; that is disclosed (markers + tests), so this is recorded as scope debt, not theater: ~300 LOC of shipped styling for zero engine capability. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table (rule blocks)
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| .dynamics-tool / __toggle / --open | 18-76 | REAL | — | tokens only; hover/focus-visible |
| __chevron | 88-92 | REAL | — | frame-measured offset |
| __body / __rows / __staged | 96-117 | REAL | S4 | styles the staged marker (F-001) |
| .dynamics-row* | 124-161 | REAL | — | pitch math documented |
| .dynamics-slider track/fill/thumb | 168-209 | REAL | — | flush-thumb formula documented w/ frame coordinates |
| __ticks / --covered | 225-249 | REAL | — | color-mix token composition (Δ≤1/255 documented) |
| reduced-motion | 253-257 | REAL | — | |
| light-theme block | 276-300 | REAL | — | contrast ratios documented (5.44:1/4.79:1) |

## 5. Test honesty
No CSS tests here; harness.test.tsx + per-tool tests assert staged markers and slider ARIA (baseline vitest covers). They would fail if markers were dropped — honest.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only (stylelint + eslint enforce); ... Raw px below are structural" (L12-14) | header | REAL | 20/20 vars resolve; px values are dimensions with frame citations |
| "the honest-unbound marker" (L111) | comment | REAL | the tools are indeed unbound (no catalog command exists) |

## 7. Dead / duplicated / unreachable code
None in-file. The styled surfaces are staged (no engine seam) — see F-001.

## 8. Dependencies & flags
Consumes tokens from src/tokens/generated/tokens.css; depends on the Slider primitive's `--lumen-slider-fill/--lumen-slider-thumb` custom props. No feature flags. UNVERIFIED: lint-css.mjs execution (script hung in sandbox beyond 120s budget — token-existence check above substitutes).

## 9. Open questions for the next agent
- When an audio-effects command lands in the catalog, who un-stages these cards? The `.dynamics-tool__staged` note + data-staged-tool markers must be removed in the same change.
