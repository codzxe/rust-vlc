# AUDIT: ui/src/components/button/__snapshots__/Toggle.test.tsx.snap
| Field | Value |
|---|---|
| Auditor | W15-R1 (crediting W15-A03) |
| Date | 2026-11-24 |
| Lines / functions | 53 / 2 snapshot entries (toggle-off, toggle-on) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (test artifact: vitest consumes it via Toggle.test.tsx:87-92; the component it pins is production-reachable: Toggle.tsx -> components/index.ts:2 -> ColorsTool.tsx:34) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg (Toggle in ui-test.txt, importers of Toggle) -> hit; sed -n '3,9p;20,25p' snap -> verified quotes |

## 1. One-paragraph verdict
Legitimate vitest snapshot pinning the full DOM of `<Toggle>` in both states. It pins exactly the a11y contract the component doc claims: `role="switch"`, `aria-checked` bound to the `checked` prop, native `<button type="button">`, three decorative (`aria-hidden="true"`) micro-label spans, and — critically — the i18n seam: the pinned texts are "AN"/"AUS", which exist only in the test's mocked catalog (Toggle.test.tsx:15-18), so the snapshot transitively proves the labels flow through `useMsg()` and are not JSX literals.

## 2. What actually works (verified)
— verified by `rg -n "Toggle" audit/runs/baseline/ui-test.txt` -> line 84: `✓ src/components/button/Toggle.test.tsx (17 tests) 171ms`, and line 179 `Tests  1705 passed | 1 skipped` — snapshot matched the live component output at baseline.

## 3. Findings
None. Checked: stale-entry drift (snap matches current component DOM — baseline run passed after the component's current shape), auto-regeneration abuse (entries correspond 1:1 to the two `toMatchSnapshot` calls at Toggle.test.tsx:89,91 — no orphan entries), pinned a11y attributes (role/aria-checked/aria-hidden all present in both entries), hardcoded-literal bypass (impossible: "AN"/"AUS" are mock-only strings, Toggle.test.tsx:11-17).

## 4. What the snapshot pins (mutation reasoning)
| Component change | Snapshot fails? | Why |
|---|---|---|
| `role` removed or changed from `switch` | YES | literal attribute at lines 7, 34 |
| `aria-checked` not bound to `checked` prop | YES | `false`/`true` differ across the two entries (lines 5, 31) |
| class grammar renamed (`lumen-toggle*`) | YES | pinned at lines 6, 12, 18, 22 |
| micro labels hardcoded "ON"/"OFF" | YES | pinned "AN"/"AUS" (lines 14, 24, 40, 50) resolve only via mocked `useMsg` |
| labels moved into a11y tree | YES | `aria-hidden="true"` pinned on all three spans |
| extra/missing DOM child | YES | full-tree serialization |
| disabled styling, focus ring, knob geometry | NO | CSS/attribute-driven, outside snapshot scope (covered by the CSS-regex tests at Toggle.test.tsx:95-184) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| Toggle.test.tsx:87-92 (produces this snap) | YES | full-DOM diff against the real render; the mock guarantees the seam path is exercised, so a broken seam or renamed class cannot regenerate to a silent pass without an explicit `--update`. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "role=switch + aria-checked (doc 08 §4.5)" | snapshot title line 3 | REAL | pinned attrs at lines 5-8 (sed-verified) |
| labels "decorative ... but localized (useSwitchLabels → i18n keys, j01-4)" | Toggle.tsx:31-32 | REAL | "AN"/"AUS" in snap come only from the mocked useMsg catalog |

## 7. Dead / duplicated / unreachable code
None — both entries are live (each maps to one `toMatchSnapshot` call).

## 8. Dependencies & flags
None. Plain vitest v1 snapshot format (header line 1); no env/config keys.

## 9. Open questions for the next agent
None.
