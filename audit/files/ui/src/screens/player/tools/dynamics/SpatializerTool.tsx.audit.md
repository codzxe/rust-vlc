# AUDIT: ui/src/screens/player/tools/dynamics/SpatializerTool.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 79 / 1 (SpatializerTool) + SPATIALIZER_ROWS data |
| File verdict | STUB (staged display ranges; interactive faders with no engine command) |
| Reachable from shipped entry point? | YES — ToolsDialog.tsx:37,128 mounts it in the player's Tools dialog (audio tab; reachable via canvas context menu / Ctrl+E) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | full read; grep DynamicsCard staging markers + mount chain |

## 1. One-paragraph verdict
Five frame-measured fader definitions (Size/Width/Wet/Dry/Damp with tick counts and toFixed formats) rendered through the shared DynamicsCard, which stamps the whole card `data-staged-tool` + `data-pending-command="audio.set-spatializer"` and prints the visible staged note (DynamicsCard.tsx:91-92, 138-139). The header admits the ranges are "staged display ranges". No `audio.set-spatializer` command exists (same finding as effects.tsx F-001), so the spatializer faders are draggable decorations: local values, zero engine effect, honestly labeled.

## 2. What actually works (verified)
— verified by grep: DynamicsCard carries the staged markers + role="note" line; ToolsDialog mounts the tool (L128); fader interaction via the audited slider engine (interaction.ts).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/tools/dynamics/SpatializerTool.tsx:5-6 and 74-78
Code:
```
 * of a 0..10 scale). Ranges are staged display ranges (types.ts note).
...
      pendingCommand="audio.set-spatializer"
```
What is wrong: the tool is display-only — faders accept drags and hold local state but `audio.set-spatializer` does not exist anywhere in the command surface, so nothing reaches the engine. Disclosure is real (visible staged note + data markers), consistent with the effects-area staging. Census: another interactive-looking dead surface in the master-question count. Confidence: HIGH — Verification: read + grep (no audio.set-spatializer in generated/commands.ts).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SpatializerTool | 73-79 | STUB | S3 | delegates to staged DynamicsCard |
| SPATIALIZER_ROWS | 11-68 | REAL-UNPROVEN | — | pure frame-measured data |

## 5. Test honesty
dynamics lane has tests (harness); UNVERIFIED depth (not my file).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Ranges are staged display ranges" | L5-6 | REAL (disclosure) | no command consumer exists |

## 7. Dead / duplicated / unreachable code
None beyond the staged nature (F-001).

## 8. Dependencies & flags
DynamicsCard, types; no feature flags.

## 9. Open questions for the next agent
- types.ts (dynamics lane): confirm DynamicsRowDef carries no dispatch seam at all.
