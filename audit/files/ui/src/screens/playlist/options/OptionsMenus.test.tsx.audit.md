# AUDIT: ui/src/screens/playlist/options/OptionsMenus.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Lines / functions | 162 / 3 describes, 10 its |
| File verdict | REAL (executed in the committed baseline: 10/10 green) |
| Reachable from shipped entry point? | NO (vitest suite; components are shipped) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. One-paragraph verdict
Behavioral pinning of the playlist sorting/view options menus against the
design frames: exact frame copy in frame order (16 sort rows, 2 view rows),
default-checked rows, icon presence/absence, provider wiring (selecting
Title A-Z re-orders the shared entries; Grid View swaps rows -> the thumbnail
grid with rows gone), and the honest-degradation hints (meta-dependent rows
carry "Needs media metadata" only when no metadata seam is wired — all three
wiring shapes tested). No mocks of the components; the only seam is the
optional resolveMeta prop itself.

## 2. What actually works (verified)
- Baseline (ui-test.txt:106): `✓ src/screens/playlist/options/
  OptionsMenus.test.tsx (10 tests) 377ms` — green.
- Label-scoped assertions documented (j15 P3-11): row textContent carries the
  hint suffix, so frame-copy checks target `.lumen-menu__label` — the tests
  know their DOM; consistent with the sortLabels helper (:23-27).

## 3. Findings
None. Checked: fakeness (no constant-only asserts — order, copy, ARIA checked
state, and DOM shape all pinned), test honesty (a wrong sort comparator or a
lost provider re-render fails the waitFor row-order assertion), mock
discipline (only the metadata seam prop varies), coverage gaps (none for the
pinned behaviors; e2e visual covers pixels).

## 4. Per-function table
| Block | Lines | Verdict | Notes |
|---|---|---|---|
| openSorting/openView/sortLabels helpers | 13-27 | REAL | role-driven queries |
| SortingOptionsMenu frame data (4 its) | 29-78 | REAL | 16 rows/order/copy, default, no-icons, provider re-sort |
| metadata hint (3 its) | 80-126 | REAL | unwired / seam-wired / local-watched override — honest degradation |
| ViewOptionsMenu (3 its) | 128-162 | REAL | 2 rows + icons, default, grid/list body switch |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 10 | YES | exact ordered arrays, ARIA checked-state counts, DOM-shape swaps |
## 6. Doc claims: "frame data (16 rows / 2 rows, frame order, frame copy)" header — all asserted; REAL.
## 7. Dead code: none.
## 8. Dependencies: @testing-library/react, vitest, optionsTestSupport mount helper.
## 9. Open questions: none.
