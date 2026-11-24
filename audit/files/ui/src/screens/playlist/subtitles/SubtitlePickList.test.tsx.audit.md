# AUDIT: ui/src/screens/playlist/subtitles/SubtitlePickList.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A01) |
| Date | 2026-11-24 |
| Lines / functions | 333 / 17 tests + 3 host helpers (StatefulPickList, SearchFilterHost, mount) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — vitest lane executed it: baseline ui-test.txt:61 `✓ src/screens/playlist/subtitles/SubtitlePickList.test.tsx (17 tests) 282ms`. Subject mounted by AddSubtitlesDialog.tsx + harness.tsx, exported via subtitles/index.ts:26 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg (it-count 17 == baseline; consumers), sed -n '15,29p' (quote verify) |

## 1. One-paragraph verdict
Real behavioral suite for the APG listbox: render anatomy, HOST-order preservation, single vs
multi selection semantics, full roving-keyboard model (arrows wrap, Home/End, Enter/Space), and
two regression repros (caret theft on filtered arrays; identity-vs-content reset policy). 17/17
tests passed in the orchestrator baseline (ui-test.txt:61; 17 it() blocks on disk match). One
hygiene defect: a verbatim-duplicated fixture constant. No mocks of the subject anywhere — hosts
are genuine stateful React wrappers.

## 2. What actually works (verified)
— verified by rg -c "it(" -> 17, matching baseline ui-test.txt:61 `(17 tests) 282ms` (PASS in the
1705-passed run).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/playlist/subtitles/SubtitlePickList.test.tsx:15-29
Code:
const CANDIDATES = [
  "The_Wand_S01E01_English.srt",
  "The_Wand_S01E01_Spanish.srt",
  "The_Wand_S01E01_Japanese.srt",
];
...
const ORDERED_CANDIDATES = [
  "The_Wand_S01E01_English.srt",
  "The_Wand_S01E01_Spanish.srt",
  "The_Wand_S01E01_Japanese.srt",
];
What is wrong: ORDERED_CANDIDATES is a byte-identical copy of CANDIDATES — two names for one
value; the doc-comment frame (Folder-2 host order) applies to both.
Why it matters: if the fixture ever changes, one copy drifts and the "ordering" tests silently
test a different list than the anatomy tests. What correct looks like:
`const ORDERED_CANDIDATES = CANDIDATES;` (or use CANDIDATES directly).
Confidence: HIGH — Verification: read (sed verified).
No S0-S3: no tautologies; assertions bind to component output (roles, aria, classes, callbacks'
called-with payloads, focus).

## 4. Per-function table
| Test group (describe) | Tests | Verdict | Notes |
|---|---|---|---|
| SubtitlePickRow anatomy (53-89) | 3 | REAL | selected class+check toggle, glyph, optionProps spread |
| render + ordering (91-126) | 3 | REAL | GIVEN order preserved (E10 before E2), labelled multiselect, honest empty state |
| selection states (128-183) | 3 | REAL | multi append/remove, single replace |
| keyboard model (185-250) | 5 | REAL | roving start, wrap arrows, Home/End, Space/Enter toggle via StatefulPickList |
| candidate changes (252-333) | 3 | REAL | caret-steal repro, identity vs content, legit reset |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 17 | YES | Each asserts DOM/aria/focus/callback state produced by the component; the hosts (StatefulPickList/SearchFilterHost) wrap the REAL component, not a mock of it. Mutation example: remove the wrap-at-ends logic -> line 215-218 fails. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "renders candidates verbatim; hosts sort at their seam" | header 21-24 | REAL | test 92-102 asserts E10-before-E2 order |
| "NO aria-activedescendant — roving-tabindex only" | comment 116-118 | REAL | test asserts not.toHaveAttribute (line 118) |

## 7. Dead / duplicated / unreachable code
F-001 duplication only. `cleanup` afterEach (13) is redundant under vitest globals but harmless
(explicit, not wrong).

## 8. Dependencies & flags
In-repo subjects only + @testing-library/react + vitest; no new deps, no gates.

## 9. Open questions for the next agent
Does AddSubtitlesDialog pass `multiple` differently in single-file contexts? Suite covers both
props; integration-level proof lives in AddSubtitlesDialog.test.tsx (not this file).
