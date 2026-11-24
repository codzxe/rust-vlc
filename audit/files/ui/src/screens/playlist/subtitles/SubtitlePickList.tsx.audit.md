# AUDIT: ui/src/screens/playlist/subtitles/SubtitlePickList.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 190 / 1 component (+5 inner fns) |
| File verdict | REAL (APG-correct roving-tabindex listbox) — but the list body is production-unreachable: the only production mount feeds it an always-empty candidate array, so users always see the empty state |
| Reachable from shipped entry point? | YES via AddSubtitlesDialog.tsx:282-287 — which receives `candidates=[]` from both PlaylistScreen mounts (see that audit). The listbox rows, roving focus, and multi-select logic execute only in the visual harness (subtitles/harness.tsx) and tests. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | read (component + parent mounts, prior audit); baseline ui-test.txt subtitles suites PASS (matching.test.ts 14 tests, AddSubtitlesDialog suite) |

## 1. One-paragraph verdict
Well-built listbox: roving tabindex only (aria-activedescendant deliberately removed, APG one-model rule cited), host-order rendering (natural sort moved to the producing seam, with the regression that motivated it documented), content-signature reset that avoids identity-churn focus theft, and a first-focus pass that never yanks the caret out of the Folder-3 search field. Two edge nits: duplicate candidate names collide as React keys and conflate selection (both rows toggle together); the length+first+last signature can miss a middle-only content change, briefly focusing a removed row index. Master-question reality: in production this list NEVER renders rows — the candidate seam is unsupplied, so the empty state is the only shipped face.

## 2. What actually works (verified)
— verified by read: keyboard model complete (Arrows/Home/End/Enter/Space, all terminal; Tab intentionally left to the dialog trap, documented at L142).
— verified by baseline: subtitles suites (matching 14 tests, dialog suite) PASS — the dialog-level tests cover this list's selection semantics.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/playlist/subtitles/SubtitlePickList.tsx:166-168
Code:
```
      {ordered.map((name, index) => (
        <SubtitlePickRow
          key={name}
```
What is wrong: Duplicate names (a host seam can produce them — two files with the same basename in different folders) collide as React keys and share selection state (`isSelected` matches by name, toggle filters by name). Cosmetic in the current always-empty production reality. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: ui/src/screens/playlist/subtitles/SubtitlePickList.tsx:88
Code:
```
  const signature = `${ordered.length}|${ordered[0] ?? ""}|${ordered[ordered.length - 1] ?? ""}`;
```
What is wrong: A candidate swap that changes only middle items (same length, same first/last) does not reset activeIndex; the focus effect's `rowRefs.current[activeIndex]?.focus()` then no-ops or focuses a different row than intended until the next keypress. Narrow edge; the documented repros (identity churn, filter reset) are handled. Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SubtitlePickList | 45-190 | REAL | — | empty-state early return |
| ↳ focus effect | 74-81 | REAL | — | first-pass + subtree-ownership guard (correct) |
| ↳ signature reset effect | 88-94 | REAL | S4 | F-002 |
| ↳ toggle | 96-104 | REAL | S4 | name-keyed (F-001) |
| ↳ move | 106-109 | REAL | — | modulo wrap; empty guard |
| ↳ onKeyDown | 111-143 | REAL | — | complete APG set |
| ↳ render | 154-189 | REAL | — | role=listbox, aria-multiselectable only when multiple |

## 5. Test honesty
Covered via AddSubtitlesDialog.test.tsx and the subtitles harness (baseline PASS). Would fail on keyboard/selection breakage.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "aria-activedescendant was REMOVED — the APG lets a listbox pick … never both" | SubtitlePickList.tsx:5-7 | TRUE | no activedescendant in render; tabIndex roving at L178 |
| "candidates render in HOST order" | SubtitlePickList.tsx:9-14 | TRUE | `const ordered = candidates;` L67 |

## 7. Dead / duplicated / unreachable code
No dead code in-file. The list-row path is production-unreachable (empty candidates seam) — dead surface in effect, exercised only by harness/tests.

## 8. Dependencies & flags
SubtitlePickRow, useSubtitlesMsg. No flags.

## 9. Open questions for the next agent
- SubtitlePickRow (not assigned): confirm it forwards optionProps without overriding tabIndex.
