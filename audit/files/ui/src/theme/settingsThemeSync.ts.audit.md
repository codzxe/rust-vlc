# AUDIT: ui/src/theme/settingsThemeSync.ts
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 130 / 4 (readPersistedThemeMode, readPersistedAccent, useSettingsThemeSync, isThemeMode/isAccentId guards) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — mounted in SettingsScreen.tsx:150 + OnboardingScreen AppearanceCards; bootstrap reads consumed by ThemeProvider (every route) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | read of schema.ts:87-90 + schemas/settings.toml:74-84; guard walk |

## 1. One-paragraph verdict
A clean, well-guarded seam: bootstrap reads (theme mode + accent) share the exact persistence store SettingsProvider hydrates from, unknown/corrupt values return null and never guess, the live-preview sync is idempotent and ThemeProvider-optional, and setAccent is guarded before every call (it throws outside the 9 generated ids). The two defects are documentation rot: the header claims the schema lacks "auto" (stale — the toml now enumerates it) and the module still reads only the localStorage seam, so in the desktop shell the FIRST-PAINT bootstrap may disagree with engine-hydrated settings for one pass.

## 2. What actually works (verified)
— verified by read: isThemeMode accepts dark/light/auto; isAccentId checks membership in the GENERATED ACCENTS (tokens stay the truth); effect guards `theme === null` (inert host) and never calls setAccent unguarded.
— verified: schemas/settings.toml:80 `enumeration = ["dark", "light", "auto"]` — "auto" IS in the schema (contradicting this file's header, see F-001).

## 3. Findings
F-001 [S4]
Where: ui/src/theme/settingsThemeSync.ts:22-27 (header comment)
Code:
```
// "auto"; the schema enumeration (schemas/settings.toml
// `interface.theme`) is still dark/light only — the REAL engine write path
// (config.set-setting validates against the embedded schema) will reject
```
What is wrong: STALE — schemas/settings.toml:76-80 now carries "auto" (the k12 Lead fix, whose comment cites this very seam). A reader today would believe System-mode Apply fails engine-side. Confidence: HIGH — Verification: read both files.

F-002 [S4]
Where: ui/src/theme/settingsThemeSync.ts:58-68, 78-88
What is wrong: `readPersistedThemeMode`/`readPersistedAccent` read `localStorageSettingsPersistence` directly — in the desktop shell the engine truth arrives via config-settings hydration (SettingsScreen), so the first-paint bootstrap can read a localStorage override that the engine has since changed (two stores again; the w26.f08 flash-fix only covers the localStorage half). Blast radius: one paint of the wrong theme/accent until the settings screen mounts. Confidence: MEDIUM — Verification: read; runtime flash UNVERIFIED.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| isThemeMode/isAccentId | 47-53 | REAL | - | total guards |
| readPersistedThemeMode | 55-68 | REAL | S4 | localStorage-only bootstrap (F-002) |
| readPersistedAccent | 70-88 | REAL | S4 | same |
| useSettingsThemeSync | 97-130 | REAL | - | idempotent, guarded, provider-optional |

## 5. Test honesty
settingsThemeSync tests exist (referenced in persistence.ts:68-70 — "settingsThemeSync's test"); content UNVERIFIED.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the schema enumeration ... is still dark/light only — the REAL engine write path ... will reject 'auto'" | header L22-25 | FALSE (stale) | toml:80 enumerates auto |
| "same persistence seam + storage key as SettingsProvider" | L60-61 | REAL | both use localStorageSettingsPersistence / lumen.settings.overrides.v1 |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
SettingsProvider, generated schema + tokens, ThemeContext. No env vars.

## 9. Open questions for the next agent
- Does the Rust config crate embed schemas/settings.toml (same enumeration incl. "auto")? If the engine schema lags, desktop Apply of System mode fails engine-side — the stale comment becomes a live bug report.
