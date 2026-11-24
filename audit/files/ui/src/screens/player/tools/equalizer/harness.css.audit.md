# AUDIT: ui/src/screens/player/tools/equalizer/harness.css
| Field | Value |
|---|---|
| Auditor | W15-R1 (crediting W15-A07) |
| Date | 2026-11-24 |
| Lines / functions | 21 / 2 rule blocks (body, .w26c1-stage) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — dev-only by design: imported solely by harness.tsx:15, which is imported solely by harness.test.tsx (rg exhaustive); never bundled into app routes |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg (importers, eslint guards, tokens) -> cited; file on design/figma/EQ.svg + renders/EQ.png -> 824px canvas / 900px render verified; baseline ui-test.txt:133 |

## 1. One-paragraph verdict
A 21-line dev-only stylesheet that does exactly what it says: reproduces the EQ.svg canvas geometry (824px width, 20px insets, 20px gap) so harness screenshots compare against the 900px figma render — and every number is verified against the actual frame (`design/figma/EQ.svg` width=824, `renders/EQ.png` 900x686, gap comment matches the y56→y76 arithmetic). Its one structural risk: it styles the global `body` element, and its exclusion from the product is enforced by nothing but comments.

## 2. What actually works (verified)
— verified by `file` command: EQ.svg is 824 wide (svg width attr, sed line 1), EQ.png render is 900x686 — the header's comparison-scale claim is frame-accurate.
— verified by rg: `--color-surface` and `--font-family-sans` exist in tokens.css (:54/:64 dark, :130 light) — the harness paints with real tokens, not literals.
— verified by baseline ui-test.txt:133 `✓ equalizer/harness.test.tsx (4 tests)` — the harness renders without crashing; harness.css is in the test's CSS_FILES inline list (harness.test.tsx:27).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/tools/equalizer/harness.css:8-10
Code: (verbatim)
```
body {
  margin: 0;
}
```
What is wrong: a global element selector in a file whose product-exclusion is convention-only — `rg harness ui/eslint.config.js` finds no import restriction, so nothing but discipline stops an app file from importing harness.css and silently zeroing the product body margin. Why it matters: the failure mode is a silent visual regression with no test guarding it. What correct looks like: scope under `.w26c1-stage`'s exported-HTML wrapper (the harness test owns the document), or add a no-restricted-imports lint rule for **/harness.css. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Rule | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| body | 8-10 | REAL | S4 | UA-margin kill for the standalone export; global scope (F-001) |
| .w26c1-stage | 12-21 | REAL | — | 824px canvas, 20px padding/gap — frame numbers verified; tokens real |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| equalizer/harness.test.tsx (4 tests, baseline :133) | PARTIAL | proves the harness renders + (with W26C1_HARNESS_OUT) exports HTML with this CSS inlined; does NOT assert the CSS values themselves — geometry fidelity is verified by the external screenshot loop (contract §5), which is outside the vitest run |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "DEV-ONLY harness layout (harness.tsx; never imported by app routes)" | harness.css:1-2 | REAL | rg: only importer harness.tsx:15 → only importer harness.test.tsx |
| "Mimics the EQ.svg canvas (824px wide … 20px insets)" | :2-3 | REAL | EQ.svg svg width=824 (sed line 1); 824−2×20 = 784 = the panel width the EqualizerTool header documents |
| "screenshots compare against the 900px figma render" | :3-4 | REAL | `file design/figma-renders/EQ.png` -> 900x686 |
| "gap: 20px; /* frame: bar bottom y56 → panel top y76 */" | :15 | REAL | arithmetic 76−56=20 consistent with the frame's documented geometry (frame y-coordinates themselves not independently measured — UNVERIFIED detail) |

## 7. Dead / duplicated / unreachable code
None. (Deliberately unreachable from the shipped bundle — its purpose.)

## 8. Dependencies & flags
Two semantic tokens (--color-surface, --font-family-sans) — both exist. No feature gates.

## 9. Open questions for the next agent
None. (If the contract §5 screenshot loop's outputs are archived anywhere, citing them would upgrade §5's fidelity verdict from PARTIAL to REAL — I found no archived loop output.)
