# AUDIT: ui/src/screens/player/tools/visualizer/harness.css (quick check)
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 55 / 7 rule blocks (body, stage, caption, cell, viz-select width, frame-open, sibling gaps) |
| File verdict | REAL (dev-only vision harness skin — never in the app bundle) |
| Reachable from shipped entry point? | NO — imported only by visualizer/harness.tsx:17, which is rendered only by harness.test.tsx (static-HTML export via W26C3_HARNESS_OUT); no app route imports it (Grep across ui/src outside tools/ shows only *.test.tsx harness consumers). Excluded from the vite app graph. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read all 55 lines; Grep `harness.css\|w26c3-\|viz-select\|viz-tool` → harness.tsx:17/27/29/46/56/77 uses every w26c3-* class; .viz-select (VisualizerModeSelect.tsx, 2 refs) and .viz-tool (VisualizerTool.tsx, 8 refs) are the real component classes the harness re-skins; Grep harness importers outside tools/ → only .test.tsx files; tokens (--color-surface-deep, --font-family-sans, --font-size-caption, --color-content-secondary) verified present in tokens.css (read this session). Baseline: ui build #22 OK; harness.test green in ui-vitest (136 files). |

## 1. One-paragraph verdict
Dev-only frame-reproduction skin, correctly quarantined: imported solely by the harness component that the test file renders to static HTML, never by an app route, so it never enters the production bundle. All sizing is frame-measured and marked (406px canvas, 22px gutters, 362px selector, 360px tool cards, 20px state gaps, 288px open-frame min-height matching the 48+4+232+4 arithmetic in the comment), colors/fonts ride semantic tokens. The one hazard is the bare `body { margin: 0 }` at the top — an unscoped element selector in a component CSS file, safe only for as long as the quarantine holds.

## 2. What actually works (verified)
— verified by Grep: 5/5 w26c3-* classes consumed by harness.tsx; the descendant selectors target real component classes (.viz-select/.viz-tool) that exist in the visualizer tool sources.
— verified by Grep: zero non-test importers of any harness module → the file is outside the app entry graph (baseline vite build #22 OK confirms no interference).
— arithmetic: padding 22px on a 406px stage → content 362px = the selector's frame width; 48+4+232+4 = 288px ✓.

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/player/tools/visualizer/harness.css:7-9
Code:
```css
body {
  margin: 0; /* the standalone export has no app shell — kill UA margin */
}
```
What is wrong: An unscoped `body` selector in a component-adjacent stylesheet. Its safety depends entirely on the module never being imported by an app route — one stray import would silently strip the app's body margins (and the app's own global CSS would fight it by bundle order).
Why it matters: Latent global side effect in a file whose only guarantee is a comment ("never imported by app routes"). The dynamics twin (dynamics/harness.css) manages without a body rule — the margin reset belongs in the harness's static-HTML export template, not the component CSS.
What correct looks like: Scope it (`.w26c3-stage { margin: 0 }` is wrong — instead set the reset in the harness.test export wrapper, or `body:where(:has(.w26c3-stage))`).
Confidence: HIGH — Verification: read; import graph verified (no app consumers).

## 4. Per-function table
| Rule block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `body` | 7-9 | REAL | S4 | Unscoped element selector (F-001). |
| `.w26c3-stage` | 11-19 | REAL | — | 406px canvas, 22px gutters, surface-deep, tokens. |
| `.w26c3-caption` | 21-25 | REAL | — | Caption scale + secondary ink. |
| `.w26c3-cell` | 27-31 | REAL | — | Flex column. |
| `.w26c3-cell > .viz-select` | 34-36 | REAL | — | 362px frame width, scoped to the harness cell only. |
| `.w26c3-frame-open` + sibling gap | 39-45 | REAL | — | 288px min-height (arithmetic checks), 20px state gap. |
| `.w26c3-cell > .viz-tool` (+sibling) | 48-54 | REAL | — | 360px tool-card width, 20px stack gap. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| harness.test.tsx (renders the harness to static HTML for vision export) | Indirectly | It pins structure, not these styles; the vision export is human/CI-evaluated at the frame dimensions. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "DEV-ONLY vision harness styles … never imported by app routes" | harness.css:1-2 | REAL | Import graph verified (only harness.tsx → harness.test.tsx). |
| "the selector lands at its exact 362px frame width" | harness.css:2-3 | REAL | 406 − 2×22 = 362 ✓. |
| "Room for the open list below the second trigger (48 + 4 + 232 + 4)" | harness.css:38 | REAL | Sum = 288 = the min-height at :40. |

## 7. Dead / duplicated / unreachable code
None dead; unreachable from the shipped app by design (dev harness).

## 8. Dependencies & flags
Semantic tokens (surface-deep, font-family-sans, font-size-caption, content-secondary — all in the generated sheet). No flags.

## 9. Open questions for the next agent
None.
