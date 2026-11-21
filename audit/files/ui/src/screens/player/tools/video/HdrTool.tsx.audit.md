# AUDIT: ui/src/screens/player/tools/video/HdrTool.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / functions | 79 / 1 (HdrTool) |
| File verdict | SHALLOW |
| Reachable from shipped entry point? | WITHIN WEB BUILD: YES (ToolsDialog.tsx:39 mounts HdrTool). ENGINE: NO — `video.set-hdr` absent from the catalog (full CommandRequest union verified: video.set-fullscreen/-aspect-ratio/-crop/-zoom/-delay only); matrix vfeat-hdr = not_started all platforms (feature-matrix.toml:1884, evidence ""). SHIPPED DESKTOP BINARY: NO (baseline #2; W14-A01) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg "video.set-hdr" in generated/commands.ts CommandRequest union (absent); sed feature-matrix.toml:1878-1889 (vfeat-hdr not_started); HdrTool.test.tsx survey 20-110 |

## 1. One-paragraph verdict
The smallest of the staged video tools: a single six-mode dropdown
(Auto/Off/HDR10/HDR10+/Dolby Vision/HLG) with local state, a staged note,
and a pendingCommand naming a command that does not exist anywhere. The
matrix is honest (not_started, empty evidence). Tests cover the dropdown
mechanics and the staged markers. Same family verdict as ColorsTool/
DynamicsCard/VideoCard: fully interactive, zero effect, disclaimed by
small print.

## 2. What actually works (verified)
— verified by baseline ui-vitest.log.txt (1705 passed): HdrTool.test.tsx
executes the closed trigger, the six-mode list (Auto selected, no icons),
click + keyboard selection with focus restoration, the jsdom-placement
fallback contract, and the staged-marker assertions (110+).
— verified by read: useDropdownScrollportFlip wired to the section ref
(55, 59); the frame deviation (no card shell, no tone-mapping rows) is
documented and matches the render (bare section + dropdown + note).
— verified by rg: catalog has no video.set-hdr — the header's claim
(15-16) is accurate, unlike VideoCard's over-general one (my VideoCard
F-002).

## 3. Findings
### F-001 [S3] Six-mode HDR selector with local state and no engine lane
Where: ui/src/screens/player/tools/video/HdrTool.tsx:52 and 64
Code:
```
  const [mode, setMode] = useState("auto"); // frame: "Auto" selected
```
```
      data-staged-tool="hdr"
      data-pending-command="video.set-hdr"
```
What is wrong: Mode selection writes local state; no dispatch, no command,
no HDR/tone-mapping stage exists (catalog verified). The staged note reuses
the video family's copy. Every mode switch is cosmetic.
Why it matters: HDR output mode is a user-facing expectation in an HDR-era
player; this panel can never affect tone mapping. Honesty chain complete
(note + markers + not_started matrix row), interaction surface real —
the same S3 family verdict as the sibling tools.
What correct looks like: Engine HDR stage + `video.set-hdr` command; until
then disabled-with-reason.
Confidence: HIGH — Verification: read + rg (catalog union enumerated).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| HdrTool | 50-79 | SHALLOW | S3 (F-001) | mechanics real (Dropdown composition + flip hook); `setMode` typed string not the mode union — type-weak but safe (options are the only source) |

## 5. Test honesty (HdrTool.test.tsx)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| closed trigger + frame mode (21-29) | YES | default "Auto" pinned |
| six modes, Auto selected, no icons (31-47) | YES | frame order pinned |
| click + keyboard selection (48-77) | YES | real key events, focus restore |
| placement fallback in jsdom (79-108) | YES | honest env-constrained contract (no layout → default placement, no escape markers) |
| staged markers (110+) | YES | data-staged-tool + data-pending-command asserted |
Mutation check: change default to "off" → test 21 fails; delete the note →
110+ fails. Honest suite.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "no HDR/tone-mapping engine stage or catalog command exists" | header 15-16 | REAL | catalog rg (no video.set-hdr) |
| vfeat-hdr not_started, evidence "" | feature-matrix.toml:1878-1889 | REAL (honest) | matches code reality |
| "Fields present only when the engine supports them (4.F3)" | matrix note 1887 | SHALLOW | the panel is present unconditionally today (no capability gating) |

## 7. Dead / duplicated / unreachable code
None in-file.

## 8. Dependencies & flags
Dropdown, useVideoMsg, types, useDropdownScrollportFlip, video.css. No
client import. No feature flags.

## 9. Open questions for the next agent
- `setMode` accepts any string (line 52, 70) — when the command lands, the
  value type should become the mode union to prevent typos in the wire args.
- The matrix note says fields appear "only when the engine supports them"
  (4.F3) — no capability query exists; who gates this panel when HDR lands?
