# AUDIT: ui/src/screens/player/dialogs/BookmarksDialog.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Lines / tests | 370 / 11 `it` blocks |
| File verdict | REAL — executed, 11/11 passing in baseline |
| Reachable from shipped entry point? | N/A — vitest suite (baseline ui-test.txt:46) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; baseline citation |

## 1. One-paragraph verdict
A high-quality wire-seam test. The `BookmarkTransport` mock implements
engine-side semantics (creation-ordered ids with no reuse, list mutation,
typed error responses, transport-level rejection), and the assertions pin
wire payloads, units, ordering, focus, and the three failure paths. The
centerpiece is the j09-P1 unit regression pin: a bookmark added at a
NONZERO position (90 s) must arrive on the wire as 90 000 000 µs — the
historical ms/µs 1000× bug was invisible at position 0, and this test
closes the loop (seam seconds → wire µs → engine echo → rendered timecode
→ jump → absolute-µs seek).

## 2. What actually works (verified)
- Baseline: audit/runs/baseline/ui-test.txt:46 (verbatim):
```
 ✓ src/screens/player/dialogs/BookmarksDialog.test.tsx (11 tests) 377ms
```
- Unit regression pin (lines 216-220, 227-233):
```
      // 90 s → 90 000 000 µs on the wire ($defs/Timestamp), NOT 90 000.
      expect(add?.args).toEqual({ time: 90_000_000, label: "Chorus" });
```
and
```
      expect(seek?.args).toEqual({ target: { kind: "absolute", time: 90_000_000 } });
```
- Digit-aware natural-order pin with scrambled seed (284-307): exact
  aria-label array equality including the "Part 2" before "Part 10" tie.
- Failure honesty (j09 P2): failed add KEEPS the typed label (324), failed
  load does NOT show the misleading empty copy (335), failed delete keeps
  the row (354) — each verified by negative query.

## 3. Findings
None. Zero findings justification: mocked seam is the transport (correct
layer — the dialog under test is the real component); every assertion is
an exact payload/array/ARIA equality; failure paths are asserted with
negative presence checks; the one soft spot candidate (line 252, `time: 0`
"unit-invariant") is explicitly documented as such by the authors and
covered by the nonzero test.

## 4. Per-function table (test-file units)
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| BookmarkTransport mock | 30-125 | REAL | engine semantics: id allocation, typed errors, list mutation |
| renderDialog | 127-135 | REAL | |
| focus test | 138-147 | REAL | initial focus on label input, not ✕ |
| mount/render test | 149-173 | REAL | exact request count + empty-then-rows |
| add-at-0 test | 175-189 | REAL | exact args `{ time: 0, label }` |
| µs unit regression | 191-234 | REAL | the file's core value |
| jump absolute test | 236-256 | REAL | wire shape + dialog-stays-open |
| delete adopts list | 258-278 | REAL | exact `{ id: 1 }` |
| natural order | 280-308 | REAL | exact aria-label array |
| 3 failure tests | 310-355 | REAL | inline errors + preserved user state |
| Escape closes | 357-369 | REAL | `toHaveBeenCalledWith("escape")` |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| All 11 | YES | exact payload equality (µs magnitudes, ids, absolute-target shapes), exact rendered-name arrays, negative queries on fabricated rows |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "j09 review's zero-only adds ... hid a 1000× ms/µs bug" | file header :6-8 | REAL | the 90 s test is written to fail under that bug |

## 7. Dead / duplicated code
None.

## 8. Dependencies & flags
@testing-library/react, vitest, generated commands types, PlayerControlsProvider seam. No gates.

## 9. Open questions
None.
