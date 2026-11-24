# AUDIT: ui/src/screens/player/tools/dynamics/CompressorTool.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 93 / 1 (CompressorTool) + COMPRESSOR_ROWS data |
| File verdict | SHALLOW (honestly-staged data card) |
| Reachable from shipped entry point? | YES — tools dialog + effects settings area (via dynamics/index.ts) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg `audio.set-compressor` in generated/commands.ts (absent); read |

## 1. One-paragraph verdict
Pure data: 7 VLC-semantics compressor rows (RMS/peak, attack, release, threshold, ratio, knee, makeup — defaults match VLC's compressor defaults, frame-measured ranges) mounted through the staged DynamicsCard with `pendingCommand="audio.set-compressor"` — a command that does not exist in the 65-command wire union (audio.* is set-volume/get-volume/set-mute/list-devices/set-device/set-delay only). The card works, the sliders drag, the audio is untouched; the staged note says so.

## 2. What actually works (verified)
— verified by rg: `audio.set-compressor` absent from generated/commands.ts L24 union — the pendingCommand is a forward declaration, and the card displays it as a data attribute.
— verified by read: row defs are coherent (min/max/step/initial/format all sane; threshold −30..0 dB, ratio 1..10:1).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/tools/dynamics/CompressorTool.tsx:88
Code:
```
      pendingCommand="audio.set-compressor"
```
What is wrong: names a command that will never arrive (no such command in the catalog; no engine compressor stage per the DynamicsCard header). The `data-pending-command` marker is honest metadata, but the user-facing surface is a full 7-knob compressor that processes nothing. Same family as EQ F-001 / video tools. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| COMPRESSOR_ROWS | 11-83 | REAL (data) | VLC-calibrated |
| CompressorTool | 86-93 | SHALLOW (staged) | data-only composition |

## 5. Test honesty
Lane tests cover rendering/markers; cannot catch "no engine target" (by design — it's admitted).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Ranges are staged display ranges calibrated to the frame's measured thumb positions" | header L9-10 | REAL-UNPROVEN | frame calibration not re-measured |

## 7. Dead / duplicated / unreachable code
None beyond the staged surface itself.

## 8. Dependencies & flags
DynamicsCard. No env vars.

## 9. Open questions for the next agent
- Does any Rust crate implement a compressor audio filter? (If the engine has one but no command, the fix is one command card away.)
