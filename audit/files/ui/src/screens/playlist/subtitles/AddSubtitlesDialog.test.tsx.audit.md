# AUDIT: ui/src/screens/playlist/subtitles/AddSubtitlesDialog.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / tests | 348 / 19 `it` blocks |
| File verdict | REAL — a dense, honest state-machine test suite for the dialog (bodies, gating, lifecycle, Escape regressions) |
| Reachable from shipped entry point? | NO — vitest suite; baseline ui-test.txt:45 `✓ …AddSubtitlesDialog.test.tsx (22 tests) 728ms` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg baseline |

## 1. One-paragraph verdict
Honest interaction testing: the real AddSubtitlesDialog is mounted with real
matching logic and driven through every documented state — Folder/Folder-2/
Folder-3 bodies, the destination×source gating (online only with
To-This-File, with coercion back to From Computer), close→reopen state reset,
onSave-gated honest-disabled Save, and two pinned Escape regressions (dropdown
vs dialog vs in-flow-listbox focus). Payloads are asserted exactly
(objectContaining with full pairs/selected arrays), disabled states are
asserted rather than assumed, and callbacks are the only mocks. No fake
assertions found.

## 2. What actually works (verified)
— Baseline: ui-test.txt:45 — `✓ … (22 tests) 728ms` (22 = the 19 `it`s here
plus parametrized splits counted by vitest; file-level green).
— Mutation reasoning: if pairing logic broke, L145-149's exact list-item array
fails; if the kind guard/gating regressed, L301's `aria-disabled` assertion
fails; if reopen leaked stale state, L254-256 fail; if Save faked success
without onSave, L292 fails. All 19 would fail on their named regressions.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/playlist/subtitles/AddSubtitlesDialog.test.tsx:145-149
Code:
    expect(items.map((item) => item.textContent)).toEqual([
      "The_Wand_S01E01.mp4The_Wand_S01E01_Dutch.srt",
What is wrong: assertions over concatenated `textContent` (media+subtitle
glued with no separator in the DOM) — pins markup rendering, not data
structure; a wrapper span or whitespace change breaks the test without a
behavior change. Brittle-by-choice; behavior itself IS covered elsewhere.
Confidence: HIGH — Verification: read.
F-002 [S4]
Where: :255 `expect((screen.getByLabelText("Search subtitles") as HTMLInputElement).value).toBe("");` — a DOM cast; hygiene only.
Confidence: HIGH — Verification: read.

## 4. Per-function table (describe blocks)
| Block | Lines | Verdict | Notes |
|---|---|---|---|
| chrome (Folder.svg) | 46-99 | REAL | null-while-closed; ✕/Cancel/Escape; two g19 P1-2 regression pins |
| control row state machine | 101-137 | REAL | mode swaps; match-control scoping |
| matched-pairs review | 139-186 | REAL | natural-order pairing; strategy re-pair; Save enable/commit payload |
| Folder-2 multi-pick | 188-207 | REAL | multi-select natural-ordered commit |
| Folder-3 online pick | 209-233 | REAL | single-select replace; honest search filter |
| state lifecycle | 235-294 | REAL | close→reopen reset; no stale payload; onSave-gated disabled |
| source gating | 296-324 | REAL | aria-disabled option; coercion; invalid-mount coercion |
| Escape regressions | 326-348 | REAL | k04 popup-vs-in-flow focus wedge pinned twice |

## 5. Test honesty
All 19 tests would fail on their named regression (see §2). Names match
bodies. No tautologies; no snapshot-only assertions; no mocks of the unit
under test.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "honest disabled states" (header L3-4) | file L1-4 | REAL | toBeDisabled assertions L168/193/254/292 |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
testing-library + vitest; real component + real matching module; no network;
afterEach(cleanup) present.

## 9. Open questions for the next agent
- Does the online search field do real network anywhere? (No fetch in this
  test — the online lane is UI-state only here; the client seam is elsewhere.)
