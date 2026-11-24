# AUDIT: ui/src/screens/settings/areas/index.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 310 / 6 (GlobalAreaScreen, VideoAreaScreen, AudioAreaScreen, resolveSettingsAreaSlug + 4 exported registries + module-load guard) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (SettingsScreen.tsx:13-16 imports SETTINGS_AREA_IDS/SETTINGS_SCREENS/resolveSettingsAreaSlug; route /settings/:area -> SettingsScreen.tsx:292,306) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg GENERATED_AREA_IDS (10 areas, all covered); rg shell consumption; sed quote verification |

## 1. One-paragraph verdict
A working registry, not a facade: 13 areas, a module-load contract guard that
THROWS if a schema-backed area lacks a registry row (209-216), retired-slug
redirects so nothing 404s (275-281), and a resolver consumed by the shell.
Type-level Record<SettingsAreaId, ...> completeness plus the runtime guard
make silent area loss impossible. The hand-authored compositions it registers
delegate to sibling files — where the dead-UI risk actually lives (Remotes,
Casting, Streaming out, Extensions list are hand sections whose seams are NOT
verified here; flagged to their owners).

## 2. What actually works (verified)
— rg: GENERATED_AREA_IDS = 10 ids (Screens.ts:318); all 10 present in
SETTINGS_AREA_IDS (189-203) — guard passes with real coverage, not vacuously.
— Shell wiring: SettingsScreen.tsx:292 `resolveSettingsAreaSlug(area) ??
SETTINGS_AREA_IDS[0]` and :306 `SETTINGS_SCREENS[area]` — registry is the
single source of the area routes.
— GlobalAreaScreen composition verified (schema sections + AdvancedDisclosure
+ About behind disclosure, 117-144).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/settings/areas/index.tsx:271-273
Code: ` * the first area. [Lead: confirm `advanced` → `performance`; the Advanced area
 * dissolves into the per-area in-flow disclosure, so `performance`
 * (Decoding/Smooth Playback/Developer) is the nearest home.]`
What is wrong: An open Lead-confirmation bracket has been shipping in the
redirect table's comment — the `advanced -> performance` mapping is asserted as
"nearest home" but never confirmed. Redirect still functions; the debt is the
unconfirmed product decision frozen in code.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| GLOBAL_HINTS map | 87-98 | REAL | - | 10 hint keys, i13Keys-typed |
| GlobalAreaScreen | 117-144 | REAL | - | frame-true column order documented |
| VideoAreaScreen / AudioAreaScreen | 149-168 | REAL | - | composition folds (codecs/effects) |
| MISSING_GENERATED_AREAS guard | 209-216 | REAL | - | throws at load — real safety |
| SETTINGS_AREA_TITLES / SETTINGS_SCREENS | 218-263 | REAL | - | Record type forces completeness |
| SETTINGS_AREA_REDIRECTS + resolveSettingsAreaSlug | 275-291 | REAL | F-001 | no-404 contract |

## 5. Test honesty
areas.test.tsx referenced (AC-asserted special-widget keys) — owned by another
agent; not re-run.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a seeded screen without a registry row would be an unreachable route" | 206-208 | REAL | guard throws; Generated ids all present |
| "NOTHING may 404" (retired slugs) | 266-267 | REAL | resolver fallback to first area (SettingsScreen.tsx:292) |

## 7. Dead / duplicated / unreachable code
None in this file. Risk transferred: sibling areas (controls/recording/
extensions/library hand sections) own their own dead-UI exposure — see my
controls.css / playlist-services.tsx / updates-about.tsx audits.

## 8. Dependencies & flags
generated/Screens (GENERATED_*), sibling area modules, specialWidgets leaf,
SettingField, i18n w26.i13 keys. No flags.

## 9. Open questions for the next agent
ControlsArea/RecordingArea/ExtensionsArea/LibraryArea owners: do the hand
sections (Remotes, Casting, Streaming out, add-on list) render real seams or
staged markers? Registry cannot tell.
