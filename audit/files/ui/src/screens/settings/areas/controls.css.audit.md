# AUDIT: ui/src/screens/settings/areas/controls.css
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 63 / 6 selector blocks |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle — imported by controls.tsx:45; scoping attribute `[data-testid="controls-area"]` rendered at controls.tsx:183; all four `data-section` targets exist (keyboard-shortcuts/remotes hand-authored; skip-amounts/mouse from generated Screens.tsx:187,190). W14 caveat: no shipped desktop binary (build-desktop-tauri.log.txt exit 101). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | grep data-section values in controls.tsx + generated/Screens.tsx (all 4 verified); specificity math re-derived from the header's stated sheet baseline (29.5 = 22.5 + 7; margins 8.5/10.5 land 31/33 — arithmetic verified). |

## 1. One-paragraph verdict
A small, honest frame-correction skin: six scoped rules that re-pin the generated settings sheet's pitches to the §C8 frame measurements (48 vs 46, 44 vs 42, section-start 31/33 vs 29.5, the 34px Mouse→Remotes gap the global :last-child zeroing ate), each with its frame citation and a self-aware note about the −8/−42 drift it fixes. The one structural smell: the entire skin hangs off `[data-testid="controls-area"]` — a TEST attribute as the production styling scope.

## 2. What actually works (verified)
— All 6 selectors' targets render on the /settings/controls route (grep evidence). Arithmetic checks: 36+8=44 pitch ✓, 36+12=48 ✓, 22.5+8.5=31 ✓, 22.5+10.5=33 ✓, 24+10=34 ✓. Baseline: controls suites pass (ui-vitest.log.txt); `npm run build` OK (ui-build.log.txt).

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/settings/areas/controls.css:22 (and 30, 39-41, 45, 53, 57)
Code:
```
[data-testid="controls-area"] [data-section="remotes"] .settings-field:not(:last-child) .settings-field__row {
  margin-bottom: 8px; /* frame: 36 + 8 = 44 */
}
```
What is wrong: every rule scopes through `[data-testid="controls-area"]` — a test-only attribute carrying the production layout. data-testid is conventionally free for tests to change/rename; here renaming it silently drops ALL frame corrections (the area would render at the sheet's drifted pitches with zero build error).
Why it matters: styling coupled to test infrastructure; the attribute is doing double duty as a stable hook nobody owns.
What correct looks like: a semantic class (`settings-area--controls`) or the data-settings-area marker the i15 note says the wrapper deliberately does NOT carry (controls.tsx:14-16) — that decision funneled the CSS onto the testid.
Confidence: HIGH — read both files.

## 4. Per-function table (selector blocks)
| Selector block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| remotes row pitch | 22-24 | REAL | S4 (F-001) | 44 pitch |
| skip-amounts row pitch | 30-32 | REAL | S4 (F-001) | 48 pitch; last row keeps sheet 10 — documented |
| section-start 31 (kb/mouse/remotes) | 39-43 | REAL | S4 (F-001) | 8.5 margin |
| section-start 33 (skip-amounts) | 45-47 | REAL | S4 (F-001) | 10.5 margin |
| mouse section gap (section share) | 53-55 | REAL | S4 (F-001) | 24 of 34 |
| mouse last-row gap (row share) | 57-63 | REAL | S4 (F-001) | 10 of 34 |

## 5. Test honesty
No tests in this file. The colocated suites assert the testid exists (it must — the CSS depends on it), which accidentally guards F-001's coupling.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the shipped sheet landed the top two blocks pixel-exact but drifted −8 (Mouse) and −42 (Remotes)" | controls.css:6-10 | REAL-UNPROVEN | frame arithmetic internally consistent; Figma not accessible to re-measure |
| "Selectors carry .settings-sections + .settings-section so they outrank the sheet's :last-child zeroing (0,3,0)" | controls.css:14-16 | REAL | remotes rule (22) deliberately omits .settings-sections — the direct-child case, explained at 20-21 |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Depends on the shared settings sheet's class grammar (.settings-section/.settings-field/__row/__title) and the generated screens' section ids. No tokens consumed (all raw frame px — documented as the area's exception).

## 9. Open questions for the next agent
- settings.css owner: do OTHER area skins also scope via data-testid, or is this a one-off? A pattern would upgrade F-001 to S3.
