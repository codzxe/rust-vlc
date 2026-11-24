# AUDIT: ui/src/screens/playlist/subtitles/subtitles.css
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 486 / 41 selector groups |
| File verdict | REAL (stylesheet for Add-Subtitles overlays; all selectors live, all vars resolve; one no-op motion guard) |
| Reachable from shipped entry point? | YES — imported by AddSubtitlesDialog.tsx and settings/areas/subtitles.tsx; the dialog mounts from the playlist subtitles flow. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python class/var sweep (41 classes, dead=NONE; 22 vars, missing=NONE); rg transition scan (exit 0 — single hit at 484); rg importers (exit 0) |

## 1. One-paragraph verdict
Real frame-measured stylesheet for the "Add Subtitles to Playlist/Folder" overlays: window deck, header band arithmetic (14+28+11+34+9=96), control pills, candidate rows, dotted-leader pair rows, footer band with a documented min-height fix. All 41 classes are referenced by TSX; all 22 custom properties resolve; the two LEAD-RULING token-vs-frame deltas are documented rather than silently applied. One hygiene defect: the prefers-reduced-motion guard disables `transition` on two selectors that define no transition anywhere in this sheet — a no-op rule.

## 2. What actually works (verified)
— verified by python sweep: 41/41 selectors live (`.lumen-plsubs*`, `.lumen-plsubs-*`); 22/22 vars resolve.
— verified by rg importers: AddSubtitlesDialog.tsx, settings/areas/subtitles.tsx both `import` this sheet.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/playlist/subtitles/subtitles.css:481-486
Code:
```
@media (prefers-reduced-motion: reduce) {
  .lumen-plsubs-row,
  .lumen-plsubs-pair {
    transition: none;
  }
}
```
What is wrong: Neither `.lumen-plsubs-row` (L253-264) nor `.lumen-plsubs-pair` (L338-347) sets any `transition` — rg shows the file's only occurrence of the property is line 484 itself, and `transition` is not inherited. The guard is a no-op; copy-pasted motion hygiene. What correct looks like: delete it or add the transitions it thinks it guards. Confidence: HIGH — Verification: ran rg "transition" on the file (1 hit).

## 4. Per-function table (file >100 LOC — selector groups)
| Selector group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| .lumen-dialog.lumen-plsubs window + __window | 30-54 | REAL | — | min(1048/800) frame caps |
| __head/__titlebar/__title/__close | 58-91 | REAL | — | band arithmetic documented |
| __controls + pill overrides (shared Dropdown) | 93-129 | REAL | — | equal-specificity compound overrides |
| __crumb + __search family | 131-196 | REAL | — | focus-within ring |
| .lumen-plsubs-match dropdown overrides | 198-224 | REAL | — | scoped label-tone override documented |
| __body + .lumen-plsubs-list/-row family (+selected/multiple/cycle/check) | 228-320 | REAL | — | |
| .lumen-plsubs-pairs/-pair family (+__side fixed 472px, __leader dot pitch 8.15px) | 322-391 | REAL | — | |
| .lumen-plsubs-empty family | 395-413 | REAL | — | |
| __foot (+ per-theme cancel/disabled-save) | 415-477 | REAL | — | raw #1b1b1b suppressed with citation |
| @media reduced-motion | 481-486 | REAL | S4 | F-001 no-op |

## 5. Test honesty
No tests here; matching.test.ts (14 tests, baseline PASS) covers the matching logic the dropdown drives.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens ONLY; raw px appears solely for frame-measured structural dimensions" | subtitles.css:12-14 | TRUE | single raw hex (#1b1b1b, L452) carries citation + lint suppression |
| "LEAD RULING: keep the token (WCAG wins); documented in the rules below, not changed" | subtitles.css:17-19 | TRUE | rules at L282-285, L353-356 keep --color-content-tertiary |

## 7. Dead / duplicated / unreachable code
The reduced-motion block (F-001) is effectively dead. No dead selectors.

## 8. Dependencies & flags
Shared Dropdown grammar (menu.css), Button (button.css), Dialog surface. No flags.

## 9. Open questions for the next agent
- AddSubtitlesDialog.tsx is in my queue — verify the dialog's Save actually persists anything (playlist subtitles on a real engine).
