# AUDIT: ui/src/screens/playlist/playlist.css
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 640 / n.a. (pure stylesheet, 63 class selectors) |
| File verdict | REAL-UNPROVEN (presentational; wired; one dead compound-selector half) |
| Reachable from shipped entry point? | YES — imported by PlaylistScreen.tsx:58, PlaylistPanel.tsx:47, NewPlaylistDialog.tsx:28; /playlist is a lazy App route (App.tsx:319), PlaylistPanel renders inside PlayerScreen (App.tsx:303), NewPlaylistDialog in AppShell.tsx:852 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 dead-selector sweep (63 classes, 1 dead candidate, refuted/confirmed below); `sed -n '120,165p'`, `sed -n '400,640p'` quote checks; grep import chain (exit 0) |

## 1. One-paragraph verdict
Pure presentation for three surfaces (queue panel, /playlist library page, create dialog). Sound, token-driven, with one dead selector half: the rule pair styling the queue row's action overlay cites `.lumen-lrow--current`, a class no component ever emits — only `lumen-lrow--selected` (components/list/Row.tsx family, asserted in List.test.tsx:430) and `lumen-plrow--current` (PlaylistPanel.tsx:340) exist. Effect: the `--selected` half of the same rule does the real work, so impact is a missing visual nuance on the currently-playing row, not broken interaction.

## 2. What actually works (verified)
— verified by python3 sweep (63/63 class names appear in TSX except `lumen-lrow--current`, confirmed dead by `grep -rn "lrow--current" components/ screens/` → only PlaylistPanel's `lumen-plrow--current`); import chain grep exit 0.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/playlist/playlist.css:134-136
Code:
```
.lumen-playlist .lumen-lrow--selected .lumen-lrow__actions,
.lumen-playlist .lumen-lrow--current .lumen-lrow__actions {
  background: var(--color-surface-selected);
}
```
What is wrong: `.lumen-lrow--current` is never emitted by any component (Row.tsx emits `--selected`; PlaylistPanel emits `lumen-plrow--current`), so the second selector is dead. The currently-playing queue row's action overlay keeps the generic `--color-surface-hover` fill from L120-132 instead of the selected surface. What correct looks like: `.lumen-plrow--current` (as L152 correctly does for the title). Confidence: HIGH — Verification: read + grep (no emitter found anywhere in components/ or screens/).

## 4. Per-function table
n.a. — stylesheet. Selector families: `.lumen-playlist*` (queue), `.lumen-plpage*` (library page), `.lumen-plnew*` (create dialog), `.lumen-plo-*` (opened-playlist browse) — all backed by TSX usage (sweep above).

## 5. Test honesty
n.a. — CSS; PlaylistPanel/PlaylistScreen tests own the behavioral contracts named in the header comment.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every value is a semantic token. Raw px is structural only" | header L11-12 | REAL | read of full file: raw px values all carry `frame:` provenance comments |
| Contracts kept: "`.lumen-plrow--current` + data-current" | header L19 | REAL | L152 styles `.lumen-plrow--current`; PlaylistPanel.tsx:340 emits it |

## 7. Dead / duplicated / unreachable code
- `.lumen-lrow--current` selector half (F-001) — dead.
- Note (not a defect): `.lumen-plnew__thumb:disabled` styling exists because the create dialog's thumbnail picker ships disabled-honest (NewPlaylistDialog.tsx:148-153, "no thumbnail backend").

## 8. Dependencies & flags
Semantic tokens; list.css/Card/ContextMenu primitives; no feature flags.

## 9. Open questions for the next agent
None.
