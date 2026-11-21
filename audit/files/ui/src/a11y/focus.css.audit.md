# AUDIT: ui/src/a11y/focus.css
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 24 / 1 rule block |
| File verdict | REAL — verified focus floor with correct light-mode layering; one stale cited ratio |
| Reachable from shipped entry point? | YES in the app graph: main.tsx:36 imports it unconditionally before render. As product: unreachable (desktop-tauri cannot compile, W14); live in the browser dev build. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read (whole file); rg `data-theme="light"\] :focus-visible` in theme/lumen.css (:202-204 — the override exists as claimed); python WCAG recompute: #824505 on #ababab = 3.26 (claim "≥3.26:1 on every light surface" — chrome is the worst case; surface/panel/elevated measure 4.21/4.39/4.52); #e15032 (light accent) on #ababab = 1.70 vs the comment's "1.40:1" (= the DARK accent #d37918's ratio) |

## 1. One-paragraph verdict
A 24-line global keyboard-focus floor that does exactly what its header promises, verified end-to-end: the `:focus-visible` ring rule is additive over the primitives' identical declarations, the light-mode color override lives one layer up in theme/lumen.css:202-204 (real, `outline-color` only, (0,3,0) beats this floor's (0,1,0) regardless of import order — the composition argument is correct), and the replacement ring's contrast claim is EXACT: #824505 on light chrome measures 3.26:1 with every other light surface higher (4.21-4.52), so "≥3.26:1 on every light surface" is true with chrome as worst case. The single defect is a stale number in the rationale: "the accent ring measures 1.40:1 on light chrome" is the DARK accent's (#d37918) ratio — the light accent (#e15032) actually measures 1.70:1. The conclusion (far under WCAG 2.4.11's 3:1, swap needed) is unaffected.

## 2. What actually works (verified)
— verified by python WCAG → focus-ring #824505: chrome #ababab 3.26 / surface #c2c2c2 4.21 / panel #c6c6c6 4.39 / elevated #c9c9c9 4.52 — the "≥3.26:1 on every light surface" claim holds with chrome as the floor.
— verified by rg + read → lumen.css:202-204 `:root[data-theme="light"] :focus-visible { outline-color: var(--color-focus-ring); }` — the override exists, keeps this floor's width/offset, and wins on specificity as documented.
— verified by reading → the floor covers focusables outside the B1 primitives (the stated gap: hyperlinks, custom click targets); primitives keep their own identical rules (duplication is deliberate and documented, l.8-10).
— verified: `--color-focus-ring` defined in the light token block (tokens.css:127 #824505 — matches the comment's hex).

## 3. Findings
### F-001 [S4]
Where: ui/src/a11y/focus.css:13-15
Code:
```
 * :focus-visible` in theme/lumen.css (theme lane — h05 F1, WCAG 2.4.11:
 * the accent ring measures 1.40:1 on light chrome, light swaps to the
 * calibrated --color-focus-ring #824505, ≥3.26:1 on every light surface).
```
What is wrong: The motivating number is stale or mislabeled: 1.40:1 is the DARK accent (#d37918) on light chrome; the light accent (#e15032) — the one a light-mode accent ring would actually paint — measures 1.70:1. The comment cites the wrong member of the accent pair for a light-mode rationale.
Why it matters: The a11y justification is directionally right but numerically wrong; the next recalibration inherits a bad baseline. (The fix's own number, 3.26, is exact — the error is only in the problem statement.)
What correct looks like: "the light accent #e15032 measures 1.70:1 on light chrome" (or a generated ratio comment).
Confidence: HIGH — Verification: python WCAG on both accent values against #ababab.

## 4. Per-function table (rule blocks)
| Selector(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `:focus-visible` floor | 21-24 | REAL | — | 2px accent + 2px offset, token-only; additive over primitives (documented); light color owned by lumen.css (verified :202-204) |
| header comment | 1-19 | REAL | S4 | F-001 stale ratio; layering/specificity argument verified correct |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| (no direct test file) | — | the floor is asserted indirectly: component suites pin their own `:focus-visible` rules (Button/IconButton/Slider tests seen in earlier audits), and lint:css (ui.yml:111) governs token usage. A regression of THIS rule (deleting the global floor) would break no test — UNVERIFIED whether any suite pins it; the custom-focusable coverage (hyperlinks) is the untested surface. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "light swaps to the calibrated --color-focus-ring #824505, ≥3.26:1 on every light surface" | l.14-15 | REAL | computed: 3.26 (chrome, worst case) / 4.21 / 4.39 / 4.52 |
| "the accent ring measures 1.40:1 on light chrome" | l.13-14 | FALSE (number) / TRUE (conclusion) | 1.40 = dark accent #d37918; light accent #e15032 = 1.70 — F-001 |
| "its (0,3,0) specificity wins over this floor regardless of import order" | l.17-18 | REAL | :root[data-theme] (0,2,0) + :focus-visible (0,1,0) = (0,3,0) vs (0,1,0) — cascade arithmetic |
| "Screens occasionally mark up custom focusables outside those primitives" | l.5-7 | REAL-UNPROVEN | plausible (about-text hyperlinks); no enumeration run — the floor is cheap insurance either way |

## 7. Dead / duplicated / unreachable code
Duplication is deliberate and documented (floor + primitives' identical rules — the "floor, not a replacement" doctrine). No dead rules. Unreachable as product (W14).

## 8. Dependencies & flags
Tokens: --color-accent (dark default), --color-focus-ring (light, via lumen.css). Imported by main.tsx:36 before render. No feature flags.

## 9. Open questions for the next agent
- theme/lumen.css owner: the light focus-ring block at :202-204 is the theme lane's — confirm its own audit files the 3.26 computation (my recomputation confirms it).
- Are there custom focusables whose authors ALSO added their own focus rules that now fight the floor (higher-specificity outline: none)? rg `outline: none` outside slider.css's compensated case — a candidate sweep for the a11y agent.
