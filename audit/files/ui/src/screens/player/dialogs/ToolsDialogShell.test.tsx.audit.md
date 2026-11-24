# AUDIT: ui/src/screens/player/dialogs/ToolsDialogShell.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / tests | 109 / 7 |
| File verdict | REAL — dialog chrome + collapsible-card skeleton grammar, close-reason asserted |
| Reachable from shipped entry point? | NO — vitest; baseline ui-test.txt:141 `✓ …ToolsDialogShell.test.tsx (7 tests) 177ms` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg baseline |

## 1. One-paragraph verdict
Honest chrome tests for the shared tools-dialog shell: role/labelled heading,
body/subtitle/footer slots, and the three close paths each asserting the
close REASON argument ("escape" / "outside"), which is stronger than
call-count assertions; the collapsible ToolsCard pins collapsed-by-default,
hidden-but-wired body (aria-controls id resolution), toggle round-trip, and
defaultOpen; the row grammar pins label/hint/chip title. Copy flows through
TOOLS_DIALOG_COPY (the w26.k09 centralization), so the assertions track the
real copy module rather than inline strings.

## 2. What actually works (verified)
— Baseline ui-test.txt:141: 7 tests green in 177 ms.
— Mutation reasoning: scrim-close regression fails L53 (reason "outside");
focus-trap Escape regression fails L44; a ToolsCard that rendered its body
while collapsed fails L79; unwired aria-controls fails L77-78.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/dialogs/ToolsDialogShell.test.tsx:50-52
Code:
    const dialog = screen.getByRole("dialog");
    const scrim = dialog.parentElement as HTMLElement;
    fireEvent.click(scrim);
What is wrong: the scrim is located by structural position
(dialog.parentElement) with a cast — a wrapper refactor (one extra div)
breaks the test without a behavior change, and the cast can throw on null
parent silently into a wrong-element click.
Confidence: HIGH — Verification: read.
Zero S0-S3 findings; checked: all close-path assertions assert the reason
argument; all aria wiring resolved through real ids; no mocks of the unit
under test (only onClose); copy imported from the central module.

## 4. Per-function table
| Group | Lines | Verdict | Notes |
|---|---|---|---|
| dialog chrome | 29-61 | REAL | 3 close reasons asserted |
| ToolsCard skeleton | 63-95 | REAL | hidden-but-wired pin |
| row grammar | 97-109 | REAL | hint + chip title |

## 5. Test honesty
All 7 fail on their named regression (§2).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "collapsed by default per the frames" | file L4 | REAL | L75 aria-expanded false + hidden |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
RTL + vitest; real ToolsDialogShell exports + TOOLS_DIALOG_COPY.

## 9. Open questions for the next agent
- Does harness.test.tsx (also mine) duplicate this chrome? (Checked next.)
