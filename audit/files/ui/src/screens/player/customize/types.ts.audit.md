# AUDIT: ui/src/screens/player/customize/types.ts
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 114 / 0 runtime (5 exported types + 2 helper type aliases) |
| File verdict | REAL (type model consumed by 5 live modules; no runtime code) |
| Reachable from shipped entry point? | YES — toolbarLayout.ts, CustomizePanels.tsx, toolbarItems.tsx, panels.ts, index.ts all import it (rg); CustomizePanels renders from the player screen's customize surface |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg -ln "CustomizeTile\|ToolbarLayout" ui/src/screens/player/customize/` -> 7 files; `rg -in "pip\|hdr" crates/commands/src/catalog/ids.rs` -> no matches (forward-declared tiles are gated off) |

## 1. One-paragraph verdict
A clean types-only data model for the toolbar-customize surface. The design
splits honestly along the seam that matters for this audit: tiles with a
`command` are capability-gated on the engine's real
`system.list-commands` listing and dispatch real payloads; `kind: "ui"`
tiles are declared inert ("its click is inert", :81-82) — a written
admission in the type system that some toolbar items never touch the
engine. The one wart is the [PROPOSAL] arg stand-ins for PiP/HDR commands
that do not exist in the wire union — verified absent from the catalog, so
those tiles can never render in any shipped build.

## 2. What actually works (verified)
— Type consumption verified by rg (5 production modules + test file).
— The claimed gating split is implemented downstream (capabilities.ts,
  toolbarItems.tsx — other agents' files); this module only declares it.
No runtime behavior to execute here.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/customize/types.ts:46-49
Code:
 * Args payload for a tile's command — the `args` fragment of a
 * CommandRequest. Forward-declared commands (PiP, HDR) are not in the 1.D4
 * wire union yet; their arg keys are [PROPOSAL] stand-ins reconciled on
 * catalog regeneration (ui/NEXT-STEPS.md).
What is wrong: tiles for PiP/HDR carry invented arg shapes for commands
that do not exist (rg over catalog ids.rs -> no pip/hdr command). The
capability gate keeps them unrendered, so the stand-ins are invisible dead
data — but any consumer trusting the type's `CommandArgs` as wire truth is
trusting fiction. Reconciliation is promised to a NEXT-STEPS doc, not a
test. Confidence: HIGH — Verification: read + rg.

## 4. Per-type table
| Type | Lines | Verdict | Notes |
|---|---|---|---|
| CustomizeLabelKey | 40-43 | REAL | MessageKey + 2 lane key unions |
| CommandArgs | 51 | REAL | loose record — see F-001 |
| CustomizeOption | 54-58 | REAL | id + args |
| CustomizeControl (4 kinds) | 60-82 | REAL | action / toggle-action / cycle / ui — ui kind is honestly inert |
| CustomizeTile | 85-99 | REAL | command: null = UI-owned |
| ToolbarLayout | 110-113 | REAL | order + band; persisted UI-owned state |

## 5. Test honesty
No tests in this file (types only). CustomizePanels.test.tsx exercises the
model's consumers (in vitest baseline).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "engine-bound tiles ... render ONLY when the engine's system.list-commands listing includes that command" | :19-21 | REAL-UNPROVEN | declared here; enforcement lives in capabilities.ts/toolbarItems.tsx (other agents) |
| "cycle tiles ... dispatch the NEXT option only from engine-confirmed state — never optimistic" | :26-27 | REAL-UNPROVEN | enforced in renderer, not here |
| "Forward-declared commands (PiP, HDR) are not in the 1.D4 wire union yet" | :47-48 | REAL | rg catalog ids -> absent |

## 7. Dead / duplicated / unreachable code
PiP/HDR arg stand-ins are dead data until the catalog grows (F-001).

## 8. Dependencies & flags
icon IconName, i18n MessageKey + two lane key files. No flags, no runtime
imports (pure `import type` — zero bundle cost).

## 9. Open questions for the next agent
- toolbarItems.tsx auditor: enumerate which tiles have command=null vs
  real commands — the ratio is the honest "inert click" count for the
  customize surface, directly relevant to the dead-button census.
