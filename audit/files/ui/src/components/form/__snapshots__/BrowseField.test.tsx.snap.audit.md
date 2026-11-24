# AUDIT: ui/src/components/form/__snapshots__/BrowseField.test.tsx.snap
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 29 / 1 snapshot |
| File verdict | REAL (committed vitest snapshot) |
| Reachable from shipped entry point? | N/A (test artifact) — pins BrowseField, the path-combo used by settings rows |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read; rg BrowseField.test.tsx case list (10 cases incl. disabled/a11y/geometry) |

## 1. One-paragraph verdict
One DOM snapshot of the path combo (label "Skin file" + text input + Browse
button). Pins the label/for association and the lumen-browse structure. The
snapshot itself shows an ENABLED Browse button — the honest stubbing of
Browse happens at the HOST level (convert screen's disabled-from-data,
settings' staged path rows), not in this primitive, which is the correct
separation. No findings.

## 2. What actually works (verified)
- Label→input association pinned (for="_r_6_" ↔ id="_r_6_").
- The component test suite around it is behavioral (onAction fires, disabled
  blocks both controls, aria-invalid/error wiring, hideLabel mechanism,
  ref forwarding, frame geometry) — the snapshot is one of 10 cases, not the
  whole coverage.

## 3. Findings
None. Checked: staleness, over-pinning (useId anchors stable), honesty of
what it pins.

## 4. Per-function table (<100 LOC)
| Snapshot | Lines | Verdict | Notes |
|---|---|---|---|
| BrowseField DOM snapshot 1 | 3-29 | REAL | |

## 5. Test honesty
See §2 — behavioral cases dominate; snapshot adds structure pinning.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "(settings path combo, wave 26)" | snap:3 | REAL | consumed by settings rows (SettingWidgets chain) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Vitest snapshot format v1.

## 9. Open questions for the next agent
- None.
