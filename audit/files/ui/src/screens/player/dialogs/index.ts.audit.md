# AUDIT: ui/src/screens/player/dialogs/index.ts
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 44 / barrel (18 export statements) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: consumed by PlayerScreen.tsx (ToolsDialogHost, openToolsDialog, BookmarksDialog, GotoTimeDialog) and ControlBar.tsx |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg export-source existence checks (exit 0) |

## 1. One-paragraph verdict
A clean frozen-surface barrel: every export maps to a real symbol in its source file (verified: ToolsDialog.tsx exports openToolsDialog/ToolsDialogHost/TOOLS_DIALOG_EVENT/TOOLS_DIALOG_TABS at L48-64; naturalSort has both comparators; useDialogsMsg carries the consolidated COPY constants; ToolsDialogShell exports the four card primitives + types). Consumers are PlayerScreen and ControlBar. No re-exports of nonexistent names, no side effects.

## 2. What actually works (verified)
— All 18 export statements resolve (rg per-source export counts + symbol spot checks).
— The `lumen:open-tools` CustomEvent seam (TOOLS_DIALOG_EVENT, ToolsDialog.tsx:48) is the real trigger wired from ControlBar and Ctrl+E (PlayerScreen.tsx:802-809).

## 3. Findings
None. Checked: export/source agreement, no circular imports (sources import from siblings, barrel only re-exports), no dead re-exports.

## 4. Per-function table
N/A (<100 LOC barrel): each export verified.

## 5. Test honesty
The barrel's modules carry their own tests (BookmarksDialog/GotoTimeDialog/ToolsDialog/ToolsDialogShell/naturalSort/parseTimeInput — all in baseline pass).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "all eight wave-c tools behind Audio/Video tabs, the `lumen:open-tools` trigger seam" | L3-5 | TRUE | TOOLS_DIALOG_TABS audio/video (ToolsDialog.tsx:54) + event seam L48 |

## 7. Dead / duplicated / unreachable code
None found in the barrel. (The ToolsDialog's tools themselves are staged-local — see the dynamics/video audits; the shell is real.)

## 8. Dependencies & flags
None.

## 9. Open questions for the next agent
- None.
