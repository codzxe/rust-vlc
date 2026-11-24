# AUDIT: ui/src/components/button/index.ts
| Field | Value |
|---|---|
| Auditor | W15-R2 (credited: A07) |
| Date | 2026-11-23 |
| Lines / functions | 6 / 0 (barrel) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — main.tsx -> App.tsx -> multiple routed screens import this barrel (NotFoundScreen, PlaylistScreen/PlaylistPanel, NetworkStreamScreen, OnboardingScreen, AddSubtitlesDialog, NewPlaylistDialog, PlaylistOptionsMenu) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg export declarations in Button/IconButton/Toggle (all 9 symbols resolve); rg barrel consumers (8+ screen files, exit 0) |

## 1. One-paragraph verdict
Six-line barrel: 3 component re-exports + 6 type re-exports, all resolving to
real declarations — Button.tsx:47 (Button) + :20/:24/:26 (variant/size/props
types), IconButton.tsx:32 + :7/:9, Toggle.tsx:36 + :6. Eight-plus routed
screen files import through it. Zero logic, zero lies, zero dangling exports.
Button/Toggle are additionally exercised by their own test files (22 + 12
tests, both in the baseline passing run).

## 2. What actually works (verified)
— verified by `rg -n "export (const|function|type|interface) ..."` across the
  three component files -> all 9 re-exported symbols exist at cited lines.
— verified by `rg -l "components/button\""` across src/screens -> 8 consumer
  files on routed screens.
— Baseline ui-test.txt:86/:117 -> Button.test.tsx (22 tests) and
  IconButton.test.tsx (12 tests) both ✓.

## 3. Findings
None. Checked: dangling re-exports (none), name mismatch (none), dead barrel
(none), shadowing (none), circular imports (none — file imports nothing).
Note: useSwitchLabels/switchCore/states.md/button.css in the directory are
deliberately NOT re-exported (internal seams — switchCore consumed by
form/Switch.tsx via deep path, documented there).

## 4. Per-function table
Not applicable — barrel, < 100 LOC, zero functions.

## 5. Test honesty
No tests in this file; re-exported components have sibling test files executed
in baseline (cited above).

## 6. Doc claims touching this file
None — no comments in the file.

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
None. Plain re-export module.

## 9. Open questions for the next agent
None.
