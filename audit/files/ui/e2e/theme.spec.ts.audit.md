# AUDIT: ui/e2e/theme.spec.ts
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 124 / 21 tests (3 hand + 18 generated matrix) |
| File verdict | REAL-UNPROVEN (current bytes not re-run; the test SET has a committed passing run at an older revision) |
| Reachable from shipped entry point? | YES — e2e of the shipped ThemeProvider/token cascade; wired into CI via ui.yml "5. e2e smoke (playwright, chromium)" → `npm run test:e2e` (playwright.config.ts testDir ./e2e) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n "theme.spec" docs/gates/logs/*.log` → phase4-exit-playwright-postfix.log: `✓ 18-20 e2e/theme.spec.ts:8/14/40` + `✓ 21-38 ... matrix: ...` and `38 passed (40.2s)`; Grep repo-wide for theme.spec (committed logs found); ui/playwright.config.ts read |

## 1. One-paragraph verdict
A real-browser computed-style e2e: the default-theme attributes on the product root, mode/accent cascade swaps with exact RGB values on the /dev/tokens sandbox, auto-mode following emulated color scheme, and a generated 9×2 accent matrix asserting BOTH --color-accent and --color-accent-bright per combination — proving both axes of the cascade. The header honestly documents a corrected WCAG falsehood (the old "4.2:1" light-accent claim was false; 2.59:1/2.70:1 recorded). The one caveat: the committed passing run (docs/gates/logs/phase4-exit-playwright-postfix.log, 38 passed) predates the current file bytes (log cites theme.spec.ts:8/14/40/78; current tests live at L19/25/55/96 after the 2026-11-12 W25/W26 re-pin header grew the file).

## 2. What actually works (verified)
- Committed run evidence: docs/gates/logs/phase4-exit-playwright-postfix.log → `✓ 18 e2e/theme.spec.ts:8:1 › default theme lands on <html> as dark + orange`, `✓ 19 ... switching mode + accent swaps the CSS variable cascade`, `✓ 20 ... auto mode follows the emulated OS color scheme`, `✓ 21-38 ... matrix: <accent> × <mode> resolves the accent cascade`, final `38 passed (40.2s)`.
- CI wiring: .github/workflows/ui.yml step 5 runs `npm run test:e2e`; ui/playwright.config.ts testDir `./e2e`, excludes visual/web lanes (documented snapshotDir trap), baseURL 127.0.0.1:5173, webServer budget 240s with a cited real CI timeout incident (run 43370495290).
- Matrix math: ACCENT_MATRIX rows (L77-87) each carry 5 hex values; hexToRgb (L89-92) converts; both swatch assertions per cell (L110-120).

## 3. Findings
F-001 [S4]
Where: ui/e2e/theme.spec.ts (whole file) vs docs/gates/logs/phase4-exit-playwright-postfix.log
What is wrong: the newest committed playwright gate log cites theme.spec.ts line numbers (8/14/40, matrix at 78) that no longer exist in the current file (tests at 19/25/55, matrix at 96) — the spec was re-pinned 2026-11-12 (header L8) with no newer committed e2e gate log for the current bytes. The tests themselves are the same set (names match one-for-one with the log), so this is evidence staleness, not fabrication. Why it matters: a future re-pin (values only) could silently invalidate the last committed green run. What correct looks like: re-run the phase-gate log on each re-pin, or record the commit alongside the log.
Confidence: HIGH — Verification: read of both files (log grep output above).

## 4. Per-function table (>100 LOC, test-file form)
| Test group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| default theme | 19-23 | REAL | — | html data-theme/data-accent on product root |
| mode+accent swap | 25-53 | REAL | — | exact RGB cascade values, independent axes |
| auto mode | 55-63 | REAL | — | emulated colorScheme both directions |
| accent matrix | 77-123 | REAL | — | 9 accents × 2 modes, accent + accent-bright per cell |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 21 | YES | computed-style equality per cell and attribute pins; the committed log shows the set green (38 passed incl. these) |
Runtime for CURRENT bytes: UNVERIFIED (no playwright run in this sandbox; no npm per load discipline).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "asserted through computed styles on the /dev/tokens sandbox (card 4.A2 dev fixture, not a product screen)" | header L5-6 | REAL | L26-52, L99-120 use /dev/tokens swatches |
| "it measures 2.59:1 on #c2c2c2 / 2.70:1 on #c6c6c6, the old '4.2:1' claim was false" | header L13-15 | REAL-UNPROVEN | contrast ratios not re-measured here (UNVERIFIED: no browser); the honesty of the correction itself is notable |
| "Truth source: themes/generated/tokens.css (pipeline output of themes/tokens.json)" | header L17 | REAL-UNPROVEN | expected values transcribed, not derived at runtime; tokens pipeline not my file |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
@playwright/test; runs against the vite dev server (baseURL 127.0.0.1:5173); excluded from visual/web lanes via config testIgnore.

## 9. Open questions for the next agent
- The /dev/tokens sandbox route: is it shipped in the production build or dev-only routing? (Not my file; if dev-only, the matrix proves the CSS pipeline, not product reachability of the tokens.)
