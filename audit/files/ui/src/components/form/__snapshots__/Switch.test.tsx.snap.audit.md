# AUDIT: ui/src/components/form/__snapshots__/Switch.test.tsx.snap (generated test artifact — short audit per spec §12)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines | 27 (1 snapshot: `Switch (Toggle.svg, doc 08 §4.5) > DOM snapshot 1`) |
| File verdict | REAL (snapshot pinning the switch's seam-proven DOM) |
| Reachable from shipped entry point? | N/A (test artifact; Switch ships via settings rows and toggles) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read of full file + Switch.test.tsx:15-17,44-55; `rg switch.on/off messages.ts` -> :2974-2975 "ON"/"OFF" |

## 1. Blunt verdict
The "AN"/"AUS" strings in this snapshot are NOT shipped copy and NOT
German-by-default UI — they are the test's deliberate catalog mock
(Switch.test.tsx:15-17 replaces switch.on/switch.off with "AN"/"AUS").
That is unusually honest test design: by mocking the catalog with strings
that differ from the component's likely hard-code temptation, the test
PROVES the micro labels flow through the i18n seam. A component that
hard-coded "ON"/"OFF" would fail this snapshot and the :47-55 assertions.
The real catalog carries "ON"/"OFF" (messages.ts:2974-2975). The snapshot
pins the full switch anatomy: role="switch" button, aria-checked, both
aria-hidden micro-label spans, fill span — structural drift fails.

## 2. What actually works (verified)
— Mock seam proof (above): mutation reasoning — hard-coding the labels
  breaks this snapshot. This is the anti-fabrication pattern other suites
  should copy.
— a11y structure pinned (role, aria-checked, aria-hidden on decorative
  text/fill).

## 3. Findings
None. (Contrast NumberField.test.tsx.snap's useId embedding — this
snapshot has no generated ids, so no re-pin risk.)

## 4. Cross-references
- states.md claim 4 (switch micro labels localized via useSwitchLabels) —
  CONFIRMED end-to-end: hook (useSwitchLabels.ts:18) -> catalog
  (:2974-2975) -> seam-proven by this snapshot.
