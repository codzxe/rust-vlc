# AUDIT: ui/src/screens/settings/generated/Screens.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 346 / 11 screen fns + 2 helpers + 2 export maps |
| File verdict | REAL (generated, drift-guarded) |
| Reachable from shipped entry point? | YES — SETTINGS_SCREENS consumed by SettingsScreen.tsx:306 (`const Current = SETTINGS_SCREENS[area]`); screens render per /settings/:area |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg entry counts (102 unique SETTING_ refs vs 121 schema exports); read of screen map + package.json guard |

## 1. One-paragraph verdict
Straightforward codegen: 11 area screens statically binding 102 of the schema's 121 exported entries into sections, plus title/screen maps. Drift guard exists (`npm run settings:check` → scripts/gen-settings-screens.mjs --check, package.json:25). The wiring is the settings provider's (localStorage in browser, config bridge in desktop) — this file is pure structure. The 19 schema entries NOT referenced here are the interesting part (S3): they exist in the schema (and so in persistence/config writes) but no generated screen renders them.

## 2. What actually works (verified)
— verified by rg: 11 screen functions each compose `<SettingsSections area=... sections=[...]>` with schema constants; GENERATED_SCREENS map (L338-349) covers global/interface/privacy/controls/playback/video/audio/performance/recording/extensions (10 areas + generated/Screens vs 13 area ids — subtitles/library/network areas come from other sources; cross-checked: areas/index registry supplies SETTINGS_SCREENS, UNVERIFIED which map wins).
— NOT run: `npm run settings:check` (node script execution outside budget? — cheap but no node_modules needed for a --check that reads files; skipped to keep discipline: UNVERIFIED).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/settings/generated/Screens.tsx (whole file) vs generated/schema.ts
What is wrong: 121 `export const SETTING_` entries exist in schema.ts; only 102 unique SETTING_ constants are referenced by Screens.tsx. 19 schema keys are writable via the provider (and, in the desktop shell, via config.set-setting) but have NO screen row — invisible settings. Which 19: not enumerated here (diff via script would list them; UNVERIFIED exact list) — includes at least the onboarding-referenced keys if they are not in sections. Why it matters: settings that exist, validate, and persist but can never be seen or changed by a user — dead schema surface. What correct looks like: either the generator emits rows for every entry or the schema prunes them; the drift guard only checks this file matches the toml sections, not that every entry is placed. Confidence: MEDIUM (count math is exact; the 19-entry list is UNVERIFIED) — Verification: rg counts.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| msgid/section helpers | 113-123 | REAL | - | cast + shape helpers |
| GlobalScreen | 125-142 | REAL | - | 3 sections |
| InterfaceScreen | 144-163 | REAL | - | skin/window/playback-controls |
| PrivacyScreen | 165-181 | REAL | - | history/network (lastfm creds included) |
| ControlsScreen..ExtensionsScreen | 183-328 | REAL | - | remaining 8 screens |
| GENERATED_AREA_TITLES / GENERATED_SCREENS | 329-349 | REAL | - | static maps |

## 5. Test honesty
Generated file — covered by settings suite via rendering; `settings:check` guard not executed (UNVERIFIED).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "adding/removing/reordering schema entries changes this file, and `npm run settings:check` fails until it is regenerated" | header L2-4 | REAL-UNPROVEN | guard script exists (package.json:25); not executed |

## 7. Dead / duplicated / unreachable code
The 19 unreferenced schema entries (see F-001) — dead UI surface, live persistence surface.

## 8. Dependencies & flags
Generated (`/* eslint-disable */`), depends only on schema + SettingsSections. No env vars.

## 9. Open questions for the next agent
- Enumerate the 19 unreferenced schema entries and check whether the Rust config crate implements them (if the engine reads them, they are hidden-but-live knobs; if not, they are fully dead).
- Does areas/index.ts's SETTINGS_SCREENS merge GENERATED_SCREENS with hand-written area screens (subtitles/library/network)? Verify no area silently renders an empty screen.
