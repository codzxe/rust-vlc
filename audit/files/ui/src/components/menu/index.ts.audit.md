# AUDIT: ui/src/components/menu/index.ts
| Field | Value |
|---|---|
| Auditor | W15-R1 (crediting W15-A07) |
| Date | 2026-11-24 |
| Lines / functions | 32 / 0 (barrel: 25 export statements — 11 value, 14 type) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — re-exported by components/index.ts:5 `export * from "./menu"`; direct importers include shipped screens (PlayerScreen.tsx, DiscoveryScreen.tsx, ConvertScreen.tsx, ControlBar.tsx, 5 settings areas) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg per-symbol export-existence (MenuBar/types/...) -> all exist; rg per-symbol external usage counts -> min 2 files each; rg barrel vs deep importers -> 17 vs 20 files; sed -n '12,20p' -> quote verified |

## 1. One-paragraph verdict
A clean, complete barrel: every one of its 25 exports resolves to a real declaration in the menu sources (verified symbol-by-symbol), every symbol has external consumers, and internal shared modules (MenuPopup.tsx, timings.ts) are correctly NOT re-exported — the public surface is deliberate. Two hygiene defects: a test-only reset helper ships through the production barrel, and the codebase bypasses the barrel (deep imports) more often than it uses it, so the barrel is a convenience, not an enforced seam.

## 2. What actually works (verified)
— verified by `rg "export (function|const|type|interface) <sym>" ui/src/components/menu/*` -> all 25 barrel symbols exist at their sources (MenuBar.tsx:40, Dropdown.tsx:118, Dropdown.tsx:434 `export const Select = Dropdown`, ContextMenu.tsx:33, Tooltip.tsx:31, MenuItem.tsx:42, ProfilesFlyout.tsx:142-189, types.ts:35-76).
— verified by external-usage rg: Dropdown 55 files, ContextMenu 21, MenuEntry 18, Select 12, MenuItem 12, MenuBar 9, Tooltip 8, ResolveLabel 7, MenuDensity 6, profile helpers 2-4 each — zero dead exports.

## 3. Findings
F-001 [S4]
Where: ui/src/components/menu/index.ts:14-20
Code: (verbatim, sed-verified)
```
export {
  buildProfileFlyout,
  createProfile,
  selectProfile,
  resetProfilesForTests,
  useMediaProfiles,
} from "./ProfilesFlyout";
```
What is wrong: `resetProfilesForTests` (ProfilesFlyout.tsx:174) is a test seam whose only external consumers are test files (ProfilesFlyout.test.tsx, ControlBar.test.tsx — verified by rg); it ships in the production barrel. Why it matters: production surface area exists only for tests; bundle includes a localStorage-resetting function reachable by anything importing the barrel. What correct looks like: export from the module file only, or gate under a test-only entry. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: ui/src/components/menu/index.ts (whole file) vs import discipline
What is wrong: 17 files import via the barrel but 20 files deep-import `components/menu/<module>` (e.g. ConvertScreen.tsx, SettingField.tsx, LocalePicker.tsx, MiniPlayer.tsx) — the majority bypasses the barrel. Why it matters: the barrel cannot serve as a stability seam when deep paths are equally common; renames break both ways. What correct looks like: one rule enforced by lint. Confidence: HIGH — Verification: read (rg -l counts).

## 4. Per-function table
Not applicable (barrel, 0 functions; every export individually verified above).

## 5. Test honesty
No tests in this file. Consumers' tests (MenuBar/Dropdown/ContextMenu/Tooltip/MenuItem/ProfilesFlyout .test.tsx) exercise the exported symbols via the barrel or deep paths — baseline ui-test.txt passes (e.g. Dropdown.test.tsx, MenuPopup.test.tsx all ✓).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Card 4.B3 — menu primitives barrel" | index.ts:1 | REAL | 6 component modules + types re-exported |
| "w26.i16 — the Profile flyout (Profiles.png §A10)" | index.ts:12 | REAL | ProfilesFlyout.tsx:189 buildProfileFlyout exists; ControlBar.tsx consumes useMediaProfiles |

## 7. Dead / duplicated / unreachable code
None. `Select = Dropdown` alias (Dropdown.tsx:434) is a deliberate API duplication (12 external consumers use Select) — noted, not dead. MenuPopup/timings intentionally unexported (internal sharing verified: MenuBar.tsx:3, ContextMenu.tsx:3, Tooltip.tsx:3).

## 8. Dependencies & flags
No imports — pure re-export module. No feature gates.

## 9. Open questions for the next agent
None blocking. (If a lint rule for barrel-only imports ever lands, F-002 resolves itself.)
