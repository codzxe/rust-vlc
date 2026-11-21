# AUDIT: ui/src/screens/settings/generated/Screens.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 346 / 13 (10 screen fns + 2 helpers + 3 exported records/types) |
| File verdict | REAL (generated artifact, in sync with schemas/settings.toml per baseline run #24; 7 of 10 screen functions dead at runtime — replaced records) |
| Reachable from shipped entry point? | NO (ui-wide: no GUI binary at HEAD). SPA-internal: areas/index.tsx:6-8 imports GENERATED_AREA_IDS/TITLES/SCREENS; SETTINGS_SCREENS registry → SettingsScreen → /settings/:area (App.tsx:322). Live consumers of the generated SCREEN functions: ControlsArea (areas/controls.tsx:49), RecordingArea (:35), ExtensionsArea (:52) — the other 7 records are overridden by hand-authored areas (verified). Executed under vitest (baseline run #23). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | Read complete; `rg -n "GENERATED_SCREENS" areas/*.tsx areas/index.tsx` (live consumers: controls/recording/extensions embed; global/interface/privacy/playback/video/audio/performance render via `schemaSectionViews` instead — so 7 generated screen fns are dead records); python3 cross-check: all 28 section-title + 10 area-title keys present in messages.ts (none missing); read gen-settings-screens.mjs :580-600 (msgid seam rationale) + :610-635 (--check drift mode); l10n-check.mjs error-rule inventory (catalog-internal only — no schema cross-check); read areas/index.tsx:205-217 (MISSING_GENERATED_AREAS contract guard). |

## 1. One-paragraph verdict
A faithful, drift-guarded codegen artifact (baseline run #24: `settings:check` PASS = byte-sync with schemas/settings.toml), wired into a registry with a real contract guard that THROWS if a schema area lacks a registry row. Two structural truths a hostile auditor must record: the generator's `msgid()` seam is a blind `as MessageKey` cast with NO gate cross-checking schema title keys against the i18n catalog (today all 38 keys are registered — verified — but a schema edit + missing Lead registration silently renders blank section titles), and 7 of the 10 generated screen functions are dead at runtime because the registry replaces their records with hand-authored area components that re-derive sections from `schemaSectionViews` (same schema, different code path).

## 2. What actually works (verified)
— Sync guarantee: baseline run #24 (`npm run settings:check`) PASS — the committed file equals the generator's output for the committed schema.
— Registry guard: areas/index.tsx:209-217 throws on any GENERATED_AREA_ID missing from SETTINGS_AREA_IDS — unreachable schema screens cannot appear silently (read, verified).
— Key registration TODAY: python3 check — 28 section titles + 10 area titles all present in messages.ts (e.g. "settings.section.global.appearance" messages.ts:2479, "settings.section.controls.skip-amounts" :2283).
— Live consumption: ControlsArea/RecordingArea/ExtensionsArea embed the generated screens (controls.tsx:49, recording.tsx:35, extensions.tsx:52), so schema rows stay reachable for those areas.

## 3. Findings

### F-001 [S3]
Where: ui/src/screens/settings/generated/Screens.tsx:115
Code:
```
const msgid = (id: string): MessageKey => id as MessageKey;
```
What is wrong: A blind cast that defeats the MessageKey type system. The generator emits `title: msgid(title)` for schema-derived titles; nothing validates those strings against the i18n catalog: `settings:check` only compares generated-file bytes vs schema (gen-settings-screens.mjs:625-635), and `l10n:check` validates only the catalog's internal consistency (rule inventory: parse/duplicate/missing/orphan/grammar/empty/placeholder/allowlist/dead-keys — no schema cross-check). If schemas/settings.toml gains a section whose `description` msgid the Lead never registers, `useMsg` returns `undefined` (messages.ts:3015 `catalog[key] ?? messages[key]`) and the section header renders EMPTY — no type error, no gate failure.
Why it matters: The generator's own comment admits the window ("titles are registered into ui/src/i18n/messages.ts only POST-WAVE", gen-settings-screens.mjs:588-589); the entire safety story is process discipline. Currently safe (all 38 keys verified registered) but one schema edit away from a silent blank-title UI.
What correct looks like: A generator/l10n-check rule asserting every schema description msgid ∈ catalog (the isMessageKey runtime guard at messages.ts:3004 already exists — the gate just needs to call it over the schema model).
Confidence: HIGH — Verification: read + script cross-check + gate rule inventories.

### F-002 [S4]
Where: ui/src/screens/settings/generated/Screens.tsx:125-316 (records overridden at areas/index.tsx:233-261)
Code:
```
export const GENERATED_SCREENS: Record<SettingsAreaId, () => ReactElement> = {
  global: GlobalScreen,
  interface: InterfaceScreen,
  privacy: PrivacyScreen,
```
What is wrong: 7 of the 10 generated screen functions (global, interface, privacy, playback, video, audio, performance) are dead as registry records: areas/index.tsx replaces every one of them with hand-authored screens that re-render the same schema rows via `schemaSectionViews(area)` (interface.tsx:69, privacy.tsx:116, playback.tsx:87, performance.tsx:80, video.tsx:123, audio.tsx:250, index.tsx:117-120). Only controls/recording/extensions records are consumed, via sibling embedding.
Why it matters: ~150 lines of generated dead code plus dozens of dead SETTING_* imports that load at module scope; a reader auditing "which rows render on /settings/video" finds TWO code paths and must know the registry override. The mechanism is honestly documented (areas/index.tsx:56-58 "hand-authored areas … win over the generated screen records").
What correct looks like: The generator emitting records only for areas without a hand-authored override (or the registry importing only the generated screens it uses and the generator pruning the rest).
Confidence: HIGH — Verification: read + rg of every area file's composition source.

### F-003 [S4]
Where: ui/src/screens/settings/generated/Screens.tsx:5
Code:
```
/* eslint-disable */
```
What is wrong: Blanket lint disable over the whole file. Standard for codegen, but it also silences rules that would catch generator OUTPUT bugs (e.g. an import typo), not just hand-edit noise.
Why it matters: Minor; the drift gate substitutes for lint on this file.
What correct looks like: Scoped disables (or none — generated code that passes lint needs none).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `msgid` | 115 | SHALLOW | F-001 | Blind cast; see finding. |
| `section` | 118-122 | REAL | — | Descriptor factory. |
| `GlobalScreen` | 125-142 | UNREACHABLE (record overridden; rows reachable via schemaSectionViews) | F-002 | Dead record. |
| `InterfaceScreen` | 144-162 | UNREACHABLE (record) | F-002 | Dead record. |
| `PrivacyScreen` | 164-179 | UNREACHABLE (record) | F-002 | Dead record. |
| `ControlsScreen` | 181-196 | REAL (embedded by ControlsArea:49) | — | Live. |
| `PlaybackScreen` | 198-222 | UNREACHABLE (record) | F-002 | Dead record. |
| `VideoScreen` | 224-245 | UNREACHABLE (record) | F-002 | Dead record. |
| `AudioScreen` | 247-265 | UNREACHABLE (record) | F-002 | Dead record. |
| `PerformanceScreen` | 267-288 | UNREACHABLE (record) | F-002 | Dead record. |
| `RecordingScreen` | 290-302 | REAL (embedded by RecordingArea:35) | — | Live. |
| `ExtensionsScreen` | 304-316 | REAL (embedded by ExtensionsArea:52) | — | Live. |
| `GENERATED_AREA_IDS` / `SettingsAreaId` / `GENERATED_AREA_TITLES` / `GENERATED_SCREENS` | 318-346 | REAL | — | Consumed by areas/index.tsx registry + contract guard; titles verified registered. |

## 5. Test honesty
No tests in this file. Coverage is indirect: areas tests (areas.test.tsx + per-area tests) render the registry screens; baseline run #23 passed them. The drift-detection test mentioned in gen-settings-screens.mjs:16 guards regeneration. Mutation note: deleting a SETTING_* row from this file would fail `settings:check` (byte drift) — the sync gate is real.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "adding/removing/reordering schema entries changes this file, and `npm run settings:check` fails until it is regenerated" | :1-4 | REAL | Baseline run #24 PASS (in sync); generator --check mode read (:625-635). |
| "titles are registered into messages.ts only POST-WAVE" (cast rationale) | gen-settings-screens.mjs:588 | REAL — and currently fully registered | All 38 keys verified present (script). |
| 13-area registry ("Exactly the 13 Figma areas") | areas/index.tsx:186-201 | REAL | SETTINGS_AREA_IDS length 13, matches GENERATED_AREA_IDS ⊂ registry via guard. |

## 7. Dead / duplicated / unreachable code
7 dead generated screen records (F-002) — rows still reachable through schemaSectionViews (verified per area file), so no user-facing loss, only code bloat and dual paths.

## 8. Dependencies & flags
schema.ts (generated schema module — another agent), SettingsSections (consumer), messages.ts. `/* eslint-disable */` only. Generator: ui/scripts/gen-settings-screens.mjs (not my assignment — audited only for the cast/drift mechanics).

## 9. Open questions for the next agent
- Who audits schemas/settings.toml + generated/schema.ts? The msgid-cast exposure (F-001) starts there.
- W11's "settings never persist" prior: does SettingsProvider (my file 12) dispatch config.set-setting on the demo transport only, or is the engine-side store the dead end? Cross-check with the config-settings client owner.
- The areas/index.tsx registry itself (SETTINGS_SCREENS overrides) is unaudited by me — its 40-line comment block carries the whole area-merge history; worth its own audit pass (likely A09/A10 territory).
