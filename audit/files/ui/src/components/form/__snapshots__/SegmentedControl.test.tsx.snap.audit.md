# AUDIT: ui/src/components/form/__snapshots__/SegmentedControl.test.tsx.snap
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 44 / 1 snapshot |
| File verdict | REAL (committed vitest snapshot) |
| Reachable from shipped entry point? | N/A (test artifact) — pins SegmentedControl, which renders the live Theme picker (settings interface area) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read; rg toMatchSnapshot usage |

## 1. One-paragraph verdict
One DOM snapshot of the Theme segmented control (Light/Dark/System). It pins
the a11y contract precisely: role=radiogroup with aria-label, radio segments
with aria-checked, and ROVING tabindex (checked segment tabindex=0, others
-1). A regression in any of these fails the test. No findings.

## 2. What actually works (verified)
- Roving tabindex captured correctly (lines 15, 26, 37) — the keyboard model
  is pinned, not just markup.
- Snapshot is structure+aria only (no volatile styling), so it fails on real
  regressions, not churn.

## 3. Findings
None. Checked: staleness (matches the component's documented contract),
over-pinning (none — no transient ids beyond useId anchors, which are stable
per mount in the snapshot context).

## 4. Per-function table (<100 LOC)
| Snapshot | Lines | Verdict | Notes |
|---|---|---|---|
| SegmentedControl DOM snapshot 1 | 3-44 | REAL | radiogroup + roving tabindex pinned |

## 5. Test honesty
Snapshot tests catch structural/aria drift; would fail if the radiogroup
semantics or tabindex model broke. Honest use of the mechanism.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "(Values.svg, wave 26)" grammar | snap:3 | UNVERIFIED | frame not inspected |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Vitest snapshot format v1.

## 9. Open questions for the next agent
- None.
