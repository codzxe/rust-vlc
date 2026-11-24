# AUDIT: ui/src/screens/player/tools/video/HdrTool.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 79 / 1 (HdrTool) + HDR_MODES data |
| File verdict | SHALLOW (honestly-staged unwired control) |
| Reachable from shipped entry point? | YES (ToolsDialog.tsx:137 `<HdrTool />` — the tools dialog Video tab) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 0 |
| Commands run | rg video.set-hdr in generated protocol (ABSENT — exit 1); rg mounts; sed quote verification |

## 1. One-paragraph verdict
Fourth member of the staged-tools family: a six-mode HDR dropdown (Auto/Off/
HDR10/HDR10+/Dolby Vision/HLG) that keeps LOCAL state, never dispatches
anything, and confesses via data-staged-tool/data-pending-command="video.set-
hdr" plus a visible role=note. Unlike the CropTool case, `video.set-hdr` is
genuinely absent from the engine protocol (rg in generated/commands.ts: no
match), so the staging claim is TRUE here. Still a dead interactive control in
the shipped dialog: users can cycle six HDR modes and nothing anywhere changes.

## 2. What actually works (verified)
— rg: `video.set-hdr` absent from the CommandRequest union — the staged
marker names a command that truly does not exist.
— Dropdown wiring is real (shared primitive with full keyboard model);
useDropdownScrollportFlip handles the documented viewport-escape placement.
— Mount verified at ToolsDialog.tsx:137 (production) + video harness.

## 3. Findings
F-001 [S2]
Where: ui/src/screens/player/tools/video/HdrTool.tsx:62-76
Code:
```
      data-tool="hdr"
      data-staged-tool="hdr"
      data-pending-command="video.set-hdr"
```
and
```
      <p className="video-hdr__staged" role="note">
        {m("player.tools.video.staged")}
      </p>
```
What is wrong: A fully interactive six-option control with zero effect on
playback, shipped in the production tools dialog. The honesty marker is
present and truthful (command absent), but the user-facing cost stands: an
HDR selector that appears functional and is not. Sibling of the EQ/visualizer/
colors/essential findings — the tools dialog's Video tab is predominantly
staged theater.
Confidence: HIGH — Verification: read + rg protocol.

## 4. Per-function table
Not applicable (<100 LOC; 1 component + 1 data array, both REAL-as-written).

## 5. Test honesty
No tests in file; video tools tested elsewhere.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no HDR/tone-mapping engine stage or catalog command exists" | :15-16 | REAL | rg video.set-hdr in generated/commands.ts -> no match |

## 7. Dead / duplicated / unreachable code
None beyond the staged-control itself (F-001).

## 8. Dependencies & flags
components/menu Dropdown, useVideoMsg, useDropdownScrollportFlip, video.css.
No flags.

## 9. Open questions for the next agent
None (engine side: HDR has no command — engine auditors confirm whether HDR
output exists at all in the renderer).
