# AUDIT: ui/src/components/surface/surface.css
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 342 / 10 blocks (scrim, dialog, sr-title, sheet×4 sides + keyframes, pane, card, well, separator, section-header, hero-pill) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — imported by Surface.tsx:3; all grammars produced by the Surface primitives used across 12+ screens |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n "font-size-label|--shadow-overlay|--radius-window" src/tokens/generated/tokens.css` (token values); `sed -n '12,14p;19,21p'` comment quotes; full read |

## 1. One-paragraph verdict
Clean token-only stylesheet backing every Surface primitive; sheet animations correctly gated behind `prefers-reduced-motion: no-preference`; the `.lumen-pane` vs `.lumen-panel` name-collision decision is documented. The single defect is header-comment drift: the provenance block cites frame values ("r19–21 via --radius-window", "0 8px 15px rgb(0 0 0 / 46%) → --shadow-overlay", "0 0 12px ... → --shadow-card") that do not match the generated tokens actually consumed (`--radius-window: 14px`, `--shadow-overlay: 0px 4.76px 5.9px 0px rgba(0,0,0,0.7)`, `--shadow-card: 0px 4.76px 4.76px 0px rgba(0,0,0,0.38)` — tokens.css:103-105). The code is right (generated tokens are the source of truth); the comments are wrong.

## 2. What actually works (verified)
— verified by `rg` against src/tokens/generated/tokens.css: every `var(--...)` consumed here resolves in the generated sheet (spot-checked font-size-label=15px = font-size-section, so dialogs.css's and this file's 15px title claims agree). Baseline vitest/build pass (§13).

## 3. Findings
F-001 [S4]
Where: ui/src/components/surface/surface.css:12-14, 19-21
Code:
```
 *   * Dialog/sheet surfaces r19–21 via --radius-window, floating on a soft
 *     black shadow (Open network stream.svg / Keyboard Shortcuts.svg /
 *     Settings - global.svg: 0 8px 15px rgb(0 0 0 / 46%) → --shadow-overlay).
```
and
```
 *   * NO drop shadows except where a frame shows one: floating cards
 *     (Miniplayer.svg card: 0 0 12px rgb(0 0 0 / 38%) → --shadow-card) and
```
What is wrong: the provenance comments assert frame→token mappings that are false against the generated tokens (tokens.css:103-105: `--radius-window: 14px` not r19–21; `--shadow-overlay` is 4.76px/5.9px/0.7 not 8px/15px/0.46; `--shadow-card` is 4.76px/4.76px/0.38 not 0px/12px/0.38). Doc rot that misleads the next reader about fidelity. Why it matters: this repo's whole audit method leans on frame-citation comments; stale ones poison the trail. Confidence: HIGH — Verification: rg on tokens.css.

## 4. Per-function table (blocks)
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| .lumen-scrim | 27-41 | REAL | — | z-index 20 above menu layer (fix note); stylelint-disable w/ [PROPOSAL] scrim token |
| .lumen-dialog (+tone/focus) | 45-68 | REAL | — | 480×720 caps |
| .lumen-surface__sr-title | 71-81 | REAL | — | clip-path sr-only |
| .lumen-sheet sides | 85-202 | REAL | — | inner-corner radii, 160ms slide-ins behind no-preference guard |
| .lumen-pane | 212-235 | REAL | — | name-collision rationale L206-210 |
| .lumen-card variants | 239-266 | REAL | — | interactive hover/focus-within elevation |
| .lumen-well / separator | 270-298 | REAL | — | separator layered over theme base sheet, cascade-order independence argued |
| .lumen-section-header | 302-328 | REAL | — | 15px via --font-size-label (verified = 15px) |
| .lumen-hero-pill | 332-342 | REAL | — | color-mix ring |

## 5. Test honesty
Surface.test.tsx asserts classes/attributes (see Surface.tsx audit); CSS values themselves are asserted indirectly. Baseline vitest 1705 passed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "r19–21 via --radius-window" (L12) | comment | FALSE | tokens.css:103 `--radius-window: 14px` |
| "0 8px 15px rgb(0 0 0 / 46%) → --shadow-overlay" (L13-14) | comment | FALSE | tokens.css:105 `0px 4.76px 5.9px 0px rgba(0,0,0,0.7)` |
| "0 0 12px rgb(0 0 0 / 38%) → --shadow-card" (L20) | comment | FALSE | tokens.css:104 `0px 4.76px 4.76px 0px rgba(0,0,0,0.38)` |
| "Section headers 15px semibold" (L17) | comment | REAL | `--font-size-label: 15px` (tokens.css:69) |

## 7. Dead / duplicated / unreachable code
None — all selectors produced by Surface.tsx primitives.

## 8. Dependencies & flags
Generated token sheet only (src/tokens/generated/tokens.css — `cargo xtask tokens` product, diff-tested by crates/theme-tokens/tests/diff.rs per its header). No flags.

## 9. Open questions for the next agent
- Which is authoritative for the frame measurements — themes/tokens.json (generating 14px/4.76px) or the Figma frames cited as r19–21/8px/15px? If the frames, the TOKENS are wrong, not the comments — theme-tokens agent should adjudicate direction of the drift.
