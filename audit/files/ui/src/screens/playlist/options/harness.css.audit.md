# AUDIT: ui/src/screens/playlist/options/harness.css
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 52 / 7 rule blocks |
| File verdict | REAL — dev-only harness styling, exactly as labeled; zero product reach |
| Reachable from shipped entry point? | NO, by design and verified: imported only by harness.tsx:22, which is imported only by *.test.tsx files (5 test files repo-wide) — never by an app route; excluded from the vite app graph (main.tsx cannot reach it). The product-CSS claim in the header is true. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg `harness.css` importers (harness.tsx only); rg importers of harness.tsx (5 test files, zero production); rg `--color-surface:` (dark #090909 — the header's "mimics the app surface (#090909)" verified); rg `w26b10` in harness.tsx (21 class usages — all selectors consumed) |

## 1. One-paragraph verdict
Exactly what it claims: a 52-line dev-only staging sheet for the w26.b10 visual harness, never imported by app routes (verified by import-chain grep), using semantic tokens only, with every selector consumed by harness.tsx. Nothing to fix.

## 2. What actually works (verified)
— verified by import-chain sweep → harness.css ← harness.tsx ← 5 test files only; the app bundle cannot contain it.
— verified by rg → all 7 selectors (`w26b10-stage/cell/caption/frame/frame--menu/frame--media/anchor` + the `--media .lumen-plo-media` height pin) have producers in harness.tsx (21 class references).
— verified by token read → dark `--color-surface: #090909` matches the header's app-surface claim (tokens.css:54).
— verified: no raw hex, no tokens missing (all 7 referenced custom properties exist), no suppressions.

## 3. Findings
None. Checked and clean: selectors consumed (7/7), tokens resolve (7/7), dev-only claim true (import chain), no dead rules, no raw values beyond structural px (32/16/8/420/377/228 — each frame-measured and commented), no product leakage.

## 4. Per-function table (rule blocks)
| Selector(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.w26b10-stage` | 5-15 | REAL | — | neutral backdrop via token |
| `__cell` / `__caption` | 17-27 | REAL | — | |
| `__frame` (+`--menu`/`--media`) | 29-42 | REAL | — | 420px menu / 377px media.svg canvas, frame-cited |
| `__anchor` | 44-47 | REAL | — | |
| `.w26b10-frame--media .lumen-plo-media` | 50-52 | REAL | — | 228px height pin (all four frame rows visible) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| harness.test.tsx (options) | YES | reads this CSS by path (l.26) and pins its composition alongside rendering the harness — the dev-shots/visual-harness family; internals owned by the harness test's agent. UNVERIFIED: exact pin list (not my file). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "DEV-ONLY harness layout (harness.tsx; never imported by app routes)" | l.1-2 | REAL | import-chain grep: test files only |
| "Mimics the app surface (#090909)" | l.2-3 | REAL | dark --color-surface #090909 (tokens.css:54) |
| "Not part of the product CSS" | l.3 | REAL | unreachable from main.tsx |

## 7. Dead / duplicated / unreachable code
None (all rules consumed by the harness; the harness itself is intentionally dev-only).

## 8. Dependencies & flags
Tokens: color-surface, font-family-sans, font-size-caption, color-content-tertiary, color-surface-panel, color-stroke-border, radius-panel (all defined). Class contract with harness.tsx only. No feature flags.

## 9. Open questions for the next agent
- None for this file. Cross-note: the sibling harness sheets (subtitles, dynamics, video, dialogs) follow the same pattern — their audits should confirm the same test-only import chain.
