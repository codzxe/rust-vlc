# AUDIT: ui/src/components/menu/types.ts
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 79 / 0 functions — 4 types + 1 const (MenuDensity, MenuEntryBase, MenuSeparator, MenuEntry, identityResolveLabel) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — types consumed by MenuPopup/ContextMenu/ControlBar AppMenuButton (ControlBar.tsx:51 identityResolveLabel) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read; consumer references verified during ControlBar/ContextMenu audits |

## 1. One-paragraph verdict
Pure data-shape module for the menus-as-data architecture: i18n keys only (no hardcoded labels), accelerator strings from keymap data rendered verbatim, documented check-slot-wins interaction (J02-12), identity fallback resolver for dev/sandbox. Zero findings — nothing executable beyond the identity function.

## 2. What actually works (verified)
— verified by consumers (ControlBar.tsx:51/1081 identityResolveLabel on real menu mounts). Baseline vitest passed.

## 3. Findings
None. Checked: types are consumed; identityResolveLabel used in production ControlBar (pre-resolved labels — deliberate identity pattern).

## 4. Per-function table
| Export | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MenuDensity/MenuEntryBase/MenuSeparator/MenuEntry | 35-73 | REAL | — | data contracts |
| identityResolveLabel | 79 | REAL | — | dev default; production use documented in ControlBar |

## 5. Test honesty
Covered indirectly via MenuPopup/ControlBar tests (baseline). No dedicated test file needed for types.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a user rebinding updates menus automatically" (L8-9) | comment | FALSE today | hotkey overrides have no runtime consumer (controls.tsx audit F-001) — the TYPE supports it, the pipeline does not exist |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
IconName type only.

## 9. Open questions for the next agent
- None beyond the L8-9 comment already falsified via the controls.tsx finding.
