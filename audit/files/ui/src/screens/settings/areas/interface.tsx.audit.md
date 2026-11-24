# AUDIT: ui/src/screens/settings/areas/interface.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 88 / 1 component (+1 msg hook, 1 hint map) |
| File verdict | REAL renderer for 13 setting rows that are wired to NOTHING — no UI consumer and no engine consumer reads any of them back |
| Reachable from shipped entry point? | YES (App /settings/interface -> SettingsScreen -> InterfaceArea) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 0 S4: 0 |
| Commands run | rg all 13 interface.* setting keys across src (exit 0 — ONLY i18n key-declaration hits); rg `value\("interface.|value\("global.` (exit 0 — zero call sites); read settingsThemeSync.ts + ThemeProvider.tsx:46-76 + SettingsScreen.tsx:461-494 |

## 1. One-paragraph verdict
The renderer itself is clean framework-driven composition (schemaSectionViews + SettingField, frame-faithful hint map, honest header-only Notifications section). The problem is what it renders: the interface area's 13 rows (skin, window, playback-controls sections) have ZERO consumers anywhere. rg across all non-test, non-schema sources for each of the 13 `interface.*` keys returns only i18n message-key declarations — no component reads them, and the engine side is the write-only `host.settings` BTreeMap (apps/cli/src/host.rs:181/1404, read-only via get-setting). Changing "Keep window on top", "Window decorations", "Auto-hide controls", "Hide after", "Start in compact mode" etc. changes NOTHING — in either shell. This is the checklist's exact "UI settings wired to nothing" pattern, at 13-for-13 density. The cross-file S2 (theme/accent bootstrap reading the wrong persistence seam in the desktop shell) is recorded here because this audit is where the settings-wiring question was answered end-to-end.

## 2. What actually works (verified)
— verified by read: flat section rendering in schema order; hint map covers exactly the 11 frame-circle rows and honestly omits 2 (L47-62 vs frame analysis); the Notifications header-only section renders no fabricated rows (L80-85).
— verified by rg: only `interface.theme` (global-area appearance row, not this area) and `global.accent-color` have any consumer in the entire UI (settingsThemeSync.ts:49-53) — see F-002.

## 3. Findings
F-001 [S2]
Where: ui/src/screens/settings/areas/interface.tsx:69-79 (all rows) — keys defined in generated/schema.ts:326-684
Code:
```
      {schemaSectionViews("interface").map(({ section, visible }) => {
        ...
            {visible.map((entry) => (
              <SettingField key={entry.key} entry={entry} hint={INTERFACE_HINTS[entry.key]} />
            ))}
```
What is wrong: every rendered row round-trips Apply into `config.set-setting` -> `host.settings.insert` (host.rs:1404), a map whose only reader is `config.get-setting` (host.rs:1372). No window/controls/skin behavior anywhere reads these 13 keys: rg for each key finds only i18n declarations; the tauri shell builds the window from product.toml at startup (apps/desktop-tauri window_config) and never queries user overrides. The user toggles "Keep window on top" and the window stays put; "Hide after: 500ms" and controls never auto-hide. Why it matters: the entire area is a control panel for nothing — the master question's answer for this screen is NO. What correct looks like: consumers (window chrome via tauri, control auto-hide in ControlBar) reading the settings store, or the rows staged/disabled until they exist. Confidence: HIGH — Verification: ran rg (13/13 keys, zero functional consumers) + read of both persistence paths.
F-002 [S2] (cross-file; owner: theme/settingsThemeSync.ts)
Where: ui/src/theme/settingsThemeSync.ts:74, 90 vs SettingsScreen.tsx:490
Code:
```
  const persisted = localStorageSettingsPersistence.load()[THEME_SETTING_KEY];
```
What is wrong: the ONLY two settings with real UI effect (interface.theme, global.accent-color — consumed via useSettingsThemeSync + ThemeProvider bootstrap) bootstrap from localStorage, but in the desktop shell the settings screen persists to the ENGINE store (`persistence={config ?? localStorageSettingsPersistence}`, SettingsScreen.tsx:490). Live preview works (draft sync), yet after app restart ThemeProvider falls back to dark/orange: the user's theme/accent never survives restart in the product shell because the bootstrap reads a seam the desktop never writes. Also the "auto" mode the picker writes is rejected by the engine's schema validation (settingsThemeSync.ts:26-29 admits it). Confidence: HIGH — Verification: read (all three files; no other bootstrap path exists — rg readPersisted* -> ThemeProvider only).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useInterfaceMsg | 45 | REAL | — | 5 lane key sets merged |
| INTERFACE_HINTS | 50-62 | REAL | — | frame-verbatim 11-of-13 |
| InterfaceArea | 64-88 | REAL renderer / dead controls | S2 | F-001 |

## 5. Test honesty
interface area tests (lane) assert rendering + schema coverage — they would pass unchanged if the settings did nothing, because they assert nothing about effects. No test asserts any interface.* value changes behavior. Baseline vitest covers rendering only.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "special widgets ..., type controls, dirty/restart contracts all stay framework-driven" (L19-21) | header | REAL | SettingField framework (sibling files) |
| "Honest absence: no fabricated rows" (L81-82) | comment | REAL | verified L83-85 |

## 7. Dead / duplicated / unreachable code
The 13 rendered rows are functionally dead (F-001) — UI-reachable, effect-unreachable.

## 8. Dependencies & flags
SettingField, shared area layer, 5 i18n lane key modules. No feature flags.

## 9. Open questions for the next agent
- settingsThemeSync.ts owner: fix the bootstrap seam (read the effective persistence — engine store in desktop) or mirror writes to localStorage; today theme/accent are session-only in the product shell (F-002).
- Who owns ControlBar auto-hide? "Hide after"/"Auto-hide controls" are natural first consumers for F-001.
