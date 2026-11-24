# AUDIT: ui/src/screens/settings/areas/hotkeys/conflicts.test.ts
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | 44 / 7 it() tests, 1 describe, 1 helper |
| File verdict | REAL (7/7 pass in baseline; pure-function semantics fully pinned) |
| Reachable from shipped entry point? | YES via test lane — baseline ui-test.txt line 173 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full file read; baseline cite |

## 1. One-paragraph verdict
Dense, honest unit test of `detectConflicts`: duplicated (mode, combo) groups mark EVERY row
(both sides warn, not just the second); normalization before comparison (Esc ≡ esc, case,
spacing); draft overrides participate (not just defaults); activation modes never collide;
unparseable spellings NEVER guess (Hyper+X twice -> no conflict); modifier order is
irrelevant (Ctrl+Shift+P ≡ Shift+Ctrl+P); and resolution clears the set. Every branch of the
function's decision surface is pinned by a distinct test.

## 2. What actually works (verified)
- Baseline: ui-test.txt line 173 `✓ src/screens/settings/areas/hotkeys/conflicts.test.ts (7 tests) 5ms`; count 7 == 7.
- Read-verified strongest rows: `expect([...ids].sort()).toEqual(["a", "b"]);` (both-sides
  warning) and `expect(ids.size).toBe(0);` (no-guess on unparseable + mode separation +
  resolution clearing).

## 3. Findings
None. Checked: input shapes (defaults + overrides map), normalization axes, set semantics.

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| B helper | 8 | REAL | binding factory with default mode |
| 7 tests | 10-44 | REAL | each pins one semantic branch |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| marks every row of a duplicated group | YES | exact sorted id pair |
| never conflicts on unparseable spellings | YES | size 0 on identical garbage |
| honors draft overrides | YES | override map produces the pair |
| clears when the duplicate is resolved | YES | size 0 after override change |

## 6-9. Doc claims / dead code / deps / open questions
Baseline 7 == 7. None / vitest only / none.
