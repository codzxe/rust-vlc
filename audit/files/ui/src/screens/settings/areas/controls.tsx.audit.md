# AUDIT: ui/src/screens/settings/areas/controls.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 244 / 4 (ActionRow, RemotesSection, ControlsArea, useAreaMsg factory) |
| File verdict | REAL (with honestly-staged dead rows) |
| Reachable from shipped entry point? | YES — route /settings/controls; retired /settings/hotkeys redirects here (test :58) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 0 |
| Commands run | `head -20` generated/Screens.tsx; `rg -n "it\(" controls.test.tsx` (8 tests); `rg -n "control-server" product.toml` (L167: `false # Phase 5+ [PROPOSAL]`); full read |

## 1. One-paragraph verdict
Real area screen: two live action rows (hotkeys editor dialog; reset clears the overrides store and remounts the editor via epoch key), schema-seeded Skip Amounts + Mouse rows rendered by the GENERATED screen (generated/Screens.tsx — "do not edit", drift-guarded by settings:check), and a Remotes section that is honest staged theater: "Web remote control"/"Port" render disabled with the staged reason in tooltip + sr-only twin, gated by the REAL product.toml flag `control-server = false # Phase 5+ [PROPOSAL]`. Nothing pretends. The only limitation worth recording: the hotkey overrides persist to localStorage only (local-device data), never to the engine config.

## 2. What actually works (verified)
— verified by controls.test.tsx:43-135 (routing, one area root, generated-screen embed with every seeded row bound, action rows, dialog mount/close, Reset clears store + re-reads keymap defaults, staged inert rows) and product.toml:167 flag check. Baseline vitest 1705 passed.

## 3. Findings
F-001 [S2]
Where: ui/src/screens/settings/areas/controls.tsx:193-206 (editor entry) + 175-178 (reset); runtime gap in src/screens/player/keymap.ts:6-11
Code (keymap.ts:6-11, the disclosure):
```
// keymaps/default.toml at repo root (binding-id keyed
// inline tables: `fullscreen = { key = "F", recipe = "fullscreen-toggle",
// mode = "player-focused", … }`). The keymap LOADING pipeline has not landed
```
and (controls.tsx:175-178):
```
  const resetAllShortcuts = (): void => {
    localStorageHotkeyOverrides.save({});
    setHotkeysEpoch((epoch) => epoch + 1);
  };
```
What is wrong: the "Edit Keyboard Shortcuts" dialog and its Apply bar commit hotkey overrides to localStorage — and NOTHING at runtime consumes them. The player's live dispatch (PlayerScreen.tsx:786-862) matches only DIALOG_KEYMAP / CUSTOMIZE_DIALOG_KEYMAP / PLAYER_SHELL_KEYMAP, all derived from DEFAULT_KEYMAP_TOML (keymap.ts:68 — a hardcoded [PROPOSAL] excerpt of keymaps/default.toml); `rg -ln "localStorageHotkeyOverrides|hotkeys/overrides"` returns exactly 4 files (controls.tsx, its test, HotkeysEditor.tsx, overrides.ts) — no player, no App, no ShortcutsScreen consumer. ShortcutsScreen (/keyboard-shortcuts) also renders only the generated defaults. Why it matters: a user who customizes any of the 141 bindings and clicks Apply gets zero effect anywhere — the editor is persistence theater over write-only storage, with no in-UI disclosure (the disclosure lives only in keymap.ts comments and NEXT-STEPS.md). What correct looks like: the keymap loading pipeline (recorded [PROPOSAL]) merging overrides into the runtime maps, or the editor disabled/staged like the Remotes rows until it lands. Confidence: HIGH — Verification: read + rg consumer sweep (both quoted commands).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ActionRow | 64-89 | REAL | — | data-controls-action markers |
| RemotesSection | 100-160 | REAL (staged) | — | disabled Switch/TextField, aria-describedby reason, data-staged="control-server" |
| ControlsArea | 166-244 | REAL | — | epoch-remount reset; Dialog-hosted HotkeysEditor |
| useAreaMsg | 57 | REAL | — | 5-file seam merge (registry/shell/lane/fixer/i15) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| :49 exactly ONE area root | YES | regression for duplicate data-settings-area |
| :66 generated embed rows bound | YES | seed-order binding |
| :119 Reset clears + defaults | YES | asserts store cleared |
| :135 staged inert rows | YES | disabled + reason attributes |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no controls.web-remote / controls.port [[setting]] keys exist and the control server is Phase 5+ (product.toml [features] control-server = false)" (L94-96) | comment | REAL | product.toml:167 verified |
| "the retired /settings/hotkeys slug redirects here" (L8-9) | comment | REAL | test :58 |

## 7. Dead / duplicated / unreachable code
Remotes rows are permanently-inert by config (honest, gated, tested) — flagged as the intended dead surface, not a defect to fix.

## 8. Dependencies & flags
GENERATED_SCREENS.controls (generated, drift-guarded). product.toml feature gate. localStorage overrides store.

## 9. Open questions for the next agent
- Does the player's live keymap (screens/player/keymap.ts) actually consult localStorageHotkeyOverrides at dispatch time? If not, custom hotkeys are editable but inert — that would be S0-worthy; keymap.ts is outside my assignment. UNVERIFIED.
