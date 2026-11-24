# AUDIT: ui/src/screens/settings/areas/hotkeys/generated/bindings.ts
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 184 / 2 (HotkeyBinding interface, HOTKEY_BINDINGS const — 141 data rows, no logic) |
| File verdict | REAL (data in sync — verified by executing the drift guard) but its consumer story is SHALLOW (editor data the runtime ignores) |
| Reachable from shipped entry point? | YES — imported by PlayerScreen.tsx (accelerator hints), ControlBar.tsx:103, HotkeysEditor (settings/controls dialog), ShortcutsScreen |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 0 |
| Commands run | `node scripts/gen-hotkeys-data.mjs --check` (ui/) -> "OK (141 bindings, 9 platform overrides)" exit 0; python3 id-set diff vs keymaps/default.toml; read of keymap.ts/variantKeymap.ts/PlayerScreen.tsx dispatch chain |

## 1. One-paragraph verdict
Generated data, provably in sync with keymaps/default.toml (I ran the repo's own drift guard — exit 0; the 9 missing `-macos` rows are the generator's documented `platform_overrides` exclusion). But this file is the display data for a hotkeys editor and shortcut hints whose subject — the live keymap — does not exist: the runtime executes a hard-coded [PROPOSAL] seed of ~5 bindings (keymap.ts:68-76 DEFAULT_KEYMAP: F/Esc/I; variantKeymap.ts: B/G) plus dialog openers (Ctrl+T/bookmarks/Ctrl+E/Ctrl+A), while `onBinding` — the seam that would receive all other forwarded bindings — is never passed by any caller (App.tsx:195-208 PlayerRoute passes only variant+onVariantChange). Pressing Space, N/P, Up/Down, M, Ctrl+O, Ctrl+Q etc. does nothing in the shipped app even though this table lists them as the product's keymap.

## 2. What actually works (verified)
— verified by RAN command: `node scripts/gen-hotkeys-data.mjs --check` -> "gen-hotkeys-data --check: OK (141 bindings, 9 platform overrides)", exit 0. python3 set-diff: 141/141 ids match keymaps/default.toml rows minus platform_overrides.

## 3. Findings
F-001 [S2]
Where: ui/src/screens/settings/areas/hotkeys/generated/bindings.ts:37-184 (data) vs ui/src/screens/player/keymap.ts:68-76 (runtime)
Code (runtime seed, keymap.ts):
```
export const DEFAULT_KEYMAP_TOML = `# [PROPOSAL] excerpt of keymaps/default.toml (card 4.C1 shell seam).
[player-fullscreen]
fullscreen = { key = "F", recipe = "fullscreen-toggle", mode = "player-focused" }
exit-fullscreen = { key = "Esc", recipe = "exit-fullscreen-close-dialog", mode = "player-focused" }
show-controls-fullscreen = { key = "I", ui_action = "show-controls-fullscreen", mode = "player-focused" }
`;
```
What is wrong: the 141-row table (and the HotkeysEditor that edits it, and the ShortcutsScreen that displays it) describes a keymap the app does not run — the "full keymap arrives with the loader" note in keymap.ts is the giveaway that the loader never landed; `onBinding?.(fwd)` (PlayerScreen.tsx:854) is a no-op because no caller wires onBinding. Why it matters: master-question census — the entire keyboard-shortcut surface (editor + overrides persistence + hints) operates on data with no runtime consumer; users can "rebind" keys that were never bound. What correct looks like: parse keymaps/default.toml + overrides at runtime (the documented loader card) or mark the editor data as pending. Confidence: HIGH — Verification: read + grep (no onBinding= anywhere outside tests).
F-002 [S3]
Where: bindings.ts audio/subtitles rows (e.g. L66 `audio.audio-track-cycle` key "B"; subtitles.subtitle-delay-down key "G") vs variantKeymap.ts:18-19
Code (variantKeymap.ts:18-19):
```
miniplayer-toggle = { key = "B", recipe = "miniplayer-toggle", mode = "player-focused" }
music-mode-toggle = { key = "G", recipe = "music-mode-toggle", mode = "player-focused" }
```
What is wrong: display/runtime contradiction — the editor table documents B as audio-track-cycle and G as subtitle-delay-down, while the RUNTIME maps B to miniplayer toggle and G to music-mode toggle. A user consulting the shortcuts table is told the wrong thing for at least these two keys. Confidence: HIGH — Verification: read of both files.

## 4. Per-function table
n.a. — generated data file (interface + const). Drift guard proves sync.

## 5. Test honesty
hotkeysData.test.ts + HotkeysEditor.test.tsx exist (not my files); they assert data shape/ordering — they cannot catch F-001 (nothing tests that runtime keys match the table).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Source of truth: keymaps/default.toml ... Drift guard: npm run hotkeys:check" | header L1-3 | REAL | ran the guard: exit 0 |
| "A table of bindings from the REAL keymap data" | HotkeysEditor.tsx:13-14 | MISLEADING | the data is real; the runtime keymap is a 5-row seed that ignores it (F-001) |

## 7. Dead / duplicated / unreachable code
The 141 rows are reachable (editor/hints) but describe unimplemented behavior; 9 platform_overrides rows intentionally excluded by the generator (documented, gen-hotkeys-data.mjs:40).

## 8. Dependencies & flags
None (pure data module, eslint-disabled header as generated).

## 9. Open questions for the next agent
- HotkeysEditor overrides.ts persistence seam: does ANY runtime code read the persisted overrides? (expected: no — same loader gap).
- conflicts.ts (my next file) handles duplicate combos inside the data — check its honesty.
