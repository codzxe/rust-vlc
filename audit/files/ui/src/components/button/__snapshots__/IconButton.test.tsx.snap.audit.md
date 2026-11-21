# AUDIT: ui/src/components/button/__snapshots__/IconButton.test.tsx.snap
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 19 / 1 snapshot entry |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A — test evidence artifact, exercised by `npm test` (baseline: 1705 passed / 1 skipped, ui-vitest.log.txt). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | cross-check against IconButton.tsx (button type + aria-label + dual class `lumen-icon-button lumen-icon-host`; glyph passed as raw svg children — no Icon primitive in this test, hence attribute-free svg). |

## 1. One-paragraph verdict
A minimal, honest snapshot: one IconButton (label "Snapshot", a play-triangle glyph) pinning the button contract — type=button, aria-label, the host class pair, aria-hidden decorative svg. No generated ids are pinned (unlike the SegmentedControl snapshot), so no framework-format brittleness. Zero findings: checked the pinned attributes against the component's real output, the glyph path is a fixture, and the entry is exercised by the baseline run.

## 2. What actually works (verified)
— Structure matches IconButton's render (class pair verified in the AppShell part1 pass — the shell's window controls mount IconButton with these classes). Baseline includes it: ui-vitest.log.txt.

## 3. Findings
None. (Checked: attribute set, class names, aria wiring, fixture path, snapshot header format, no ids pinned — all consistent with the component and the baseline pass.)

## 4. Per-function table
| Entry | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `IconButton > DOM snapshot 1` | 3-19 | REAL | — | pins button type/aria/class + decorative svg |

## 5. Test honesty
Would it fail if the code were broken? YES — dropping `type="button"` (form-submit regression), the aria-label, or the host class breaks the snapshot.

## 6. Doc claims touching this file
None (test artifact).

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Vitest snapshot v1; belongs to IconButton.test.tsx (other agent).

## 9. Open questions for the next agent
- IconButton.test.tsx owner: does any test pin the DISABLED and focus-visible states (the snapshot only covers the default state)?
