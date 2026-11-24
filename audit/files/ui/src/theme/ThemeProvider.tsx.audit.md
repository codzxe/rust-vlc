# AUDIT: ui/src/theme/ThemeProvider.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 129 / 4 (provider + 3 helpers) |
| File verdict | REAL provider with a S2 split-brain persistence flaw: it bootstraps theme/accent from localStorage ONLY, while the desktop settings screen writes theme/accent to the ENGINE config only — desktop theme changes cannot survive reload |
| Reachable from shipped entry point? | YES — main.tsx:60 `<ThemeProvider>` wraps the entire app in both lanes. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | read settingsThemeSync.ts (bootstrap seam, in full); rg persistence.save in SettingsProvider.tsx:121; sed schema.ts:87-99 (interface.theme enum now includes auto); rg config.get-setting readers (only SettingsScreen + config-settings); baseline ui-test.txt theme/settings suites PASS |

## 1. One-paragraph verdict
The provider itself is correct and disciplined: lazy-init from a total, never-guessing bootstrap read; attribute-only theming in a layout effect (no first-frame flash); OS preference tracked only while "auto"; unknown accents throw. The defect is architectural and lands on this file's bootstrap choice: `readPersistedThemeMode`/`readPersistedAccent` read `localStorageSettingsPersistence` — HARDCODED (settingsThemeSync.ts:43,74,90) — but in the desktop shell the settings screen's Apply writes `interface.theme`/`global.accent-color` through the ENGINE persistence (SettingsScreen.tsx:490-491 `persistence={config ?? …}`; SettingsProvider.tsx:121 `persistence.save(overrides)` — one seam, no mirror). Result in the shipped desktop lane: change theme to Light in Settings → engine config updated, live preview works, reload → ThemeProvider bootstraps from stale/empty localStorage → boots Dark again. The browser dev shell is self-consistent (localStorage both ways). Nobody bridges engine config to the bootstrap: rg shows `config.get-setting` consumed only by SettingsScreen's hydrate.

## 2. What actually works (verified)
— verified by read: layout-effect attribute application (L102-106) runs before paint; matchMedia listener only in auto (L81-93); setAccent validates against the generated ACCENTS and throws otherwise (L112-117); bootstrap reads are total (null → defaults, never throws).
— verified by baseline: ThemeProvider-driven suites (theme lane + settings suites incl. appearance pickers) PASS.

## 3. Findings
F-001 [S2]
Where: ui/src/theme/ThemeProvider.tsx:69-77 (bootstrap) with ui/src/theme/settingsThemeSync.ts:74,90 and ui/src/screens/settings/SettingsScreen.tsx:490-491
Code:
```
  const [modeSetting, setModeSettingState] = useState<ThemeModeSetting>(
    () => readPersistedThemeMode() ?? "dark",
  );
```
```
  const persisted = localStorageSettingsPersistence.load()[THEME_SETTING_KEY];
```
What is wrong: Read seam and write seam point at DIFFERENT stores in the desktop lane. Bootstrap (this file, via settingsThemeSync) reads localStorage `lumen.settings.overrides.v1`; the desktop settings screen persists through `createConfigSettingsPersistence` (engine `config.set-setting`) with no localStorage mirror. Every theme/accent change a desktop user commits evaporates on the next launch; meanwhile the engine config holds a value the UI ignores at boot. The engine is wired; the provider is not wired to the engine — a textbook answer to the wave's master question. Why it matters: theme is a first-class setting with dedicated pickers in settings AND onboarding; silent revert on restart is user-visible wrong behavior in realistic use. What correct looks like: the bootstrap read goes through the same isDesktopShell()-selected persistence the settings screen uses (hydrate engine overrides before first paint), or Apply mirrors to localStorage. Confidence: HIGH on the code paths (both seams read; no bridge exists — rg config.get-setting); reload behavior REASONED, not executed (UNVERIFIED: no desktop runtime in sandbox). Severity S2, not S1: cosmetic setting, no data loss.
F-002 [S4]
Where: ui/src/theme/settingsThemeSync.ts:26-29 (this provider's consumption seam)
Code:
```
 * PICKER already renders the third cell (Light · Dark · System, f03 lane)
 * writing "auto"; the schema enumeration (schemas/settings.toml
 * `interface.theme`) is still dark/light only — the REAL engine write path
```
What is wrong: Stale — the generated schema's `interface.theme` enumeration now INCLUDES "auto" (schema.ts:96 `enumeration: [{ dark }, { light }, { auto }]`). The "will reject auto until the schema owner adds it" warning no longer describes reality (generated side). Rust-side schema agreement UNVERIFIED (crates lane). Confidence: HIGH on the generated side — Verification: sed schema.ts:87-99.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| systemPrefersLight | 56-61 | REAL | — | no-signal → dark (documented default) |
| ThemeProvider | 63-125 | REAL | S2 | F-001 bootstrap seam |
| ↳ lazy inits | 69-77 | REAL | S2 | localStorage-only bootstrap |
| ↳ auto-mode matchMedia effect | 81-93 | REAL | — | correct subscribe/cleanup |
| ↳ resolvedMode | 95 | REAL | — | |
| ↳ layout effect | 102-106 | REAL | — | pre-paint attribute set |
| ↳ setModeSetting/setAccent | 108-117 | REAL | — | accent validated; mode typed |
| ↳ context memo | 119-122 | REAL | — | |
| re-exports | 53-54 | REAL | — | compat barrel, documented |

## 5. Test honesty
Theme/settings suites PASS in baseline; none of them covers the desktop write-path→bootstrap read-path round trip across a reload (jsdom cannot; the gap is exactly where the tests are silent).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every route starts on the user's saved mode" | ThemeProvider.tsx:13-15 | TRUE in browser lane, FALSE in desktop lane | F-001: desktop saves go to engine; bootstrap reads localStorage |
| "the same persistence seam + storage key as SettingsProvider" | settingsThemeSync.ts:14-15 | HALF-TRUE | true only for the default (localStorage) persistence; SettingsScreen overrides it with the engine seam on desktop |
| schema enum "still dark/light only" | settingsThemeSync.ts:26-29 | FALSE (stale) | schema.ts:96 includes auto |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
tokens (ACCENTS/TokenMode), lumen.css, context.ts, settingsThemeSync. No flags.

## 9. Open questions for the next agent
- Rust side: does crates' embedded settings schema accept "auto" for interface.theme (the generated one does)? If not, desktop Apply of "System" is rejected by the engine while the UI live-previews it — worse than F-001.
- Does anything in the desktop host (crates/desktop) push engine config into the webview at boot (injected JS/localStorage seeding) that would mask F-001? If yes, the split-brain is healed externally — verify before final severity.
