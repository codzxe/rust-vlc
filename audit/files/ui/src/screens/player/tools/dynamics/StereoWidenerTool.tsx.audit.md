# AUDIT: ui/src/screens/player/tools/dynamics/StereoWidenerTool.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 68 / 1 (StereoWidenerTool) + STEREO_WIDENER_ROWS data |
| File verdict | STUB (staged display ranges; interactive faders, no engine command) |
| Reachable from shipped entry point? | YES — ToolsDialog.tsx:37,128-family mounts it (player Tools dialog audio tab) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | full read; DynamicsCard staging markers verified in my SpatializerTool audit (DynamicsCard.tsx:91-92,138-139) |

## 1. One-paragraph verdict
Identical pattern to SpatializerTool: four frame-measured fader definitions (Delay ms + Feedback/Crossfeed/Dry-Mix percent) through the shared staged DynamicsCard with `data-pending-command="audio.set-stereo-widener"` — a command that exists nowhere in the catalog. Draggable decoration with the visible staged note; zero engine effect.

## 2. What actually works (verified)
— verified by read + the DynamicsCard marker greps from the SpatializerTool audit; mount chain via ToolsDialog.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/tools/dynamics/StereoWidenerTool.tsx:63-67
Code:
```
    <DynamicsCard
      tool="stereo-widener"
      nameKey="player.tools.dynamics.stereo-widener"
      pendingCommand="audio.set-stereo-widener"
```
What is wrong: same as SpatializerTool F-001 — staged display-only faders; `audio.set-stereo-widener` does not exist in the command surface, so nothing reaches an engine stage. Honest markers, dead effect. Confidence: HIGH — Verification: read + grep.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| StereoWidenerTool | 61-67 | STUB | S3 | delegates to staged DynamicsCard |
| STEREO_WIDENER_ROWS | 15-57 | REAL-UNPROVEN | — | pure frame-measured data |

## 5. Test honesty
dynamics lane tests (UNVERIFIED depth).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The gain ranges match the widener semantics the frame encodes" | L10-11 | REAL-UNPROVEN | frame data only; no engine semantics exist to match |

## 7. Dead / duplicated / unreachable code
None beyond the staged nature.

## 8. Dependencies & flags
DynamicsCard, types. None.

## 9. Open questions for the next agent
None.
