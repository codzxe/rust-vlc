# AUDIT: ui/src/screens/settings/generated/schema.ts
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 2919 / 0 functions (121 data consts + 4 type ifaces + 2 aggregates) |
| File verdict | REAL (generated data, drift-guard-clean) — but the surface it describes is mostly write-only downstream |
| Reachable from shipped entry point? | YES (lumen-tauri lane: main.tsx -> App -> SettingsScreen -> SETTINGS_AREAS/SETTINGS_ENTRIES; verified importer list below) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `node scripts/gen-settings-screens.mjs --check` -> exit 0 "OK (10 ui areas, 102 ui entries, 121 total)"; `node scripts/l10n-check.mjs` -> exit 0 "missing 0, duplicates 0, orphans 0"; python set-diff of consts vs SETTINGS_ENTRIES vs SETTINGS_AREAS; rg importer scan (exit 0) |

## 1. One-paragraph verdict
Generated constant data (no logic): 121 `SettingEntry` consts across 11 `screen` values, assembled into `SETTINGS_ENTRIES` (102) and `SETTINGS_AREAS` (10 areas/37 sections). The file is honestly labeled GENERATED, is byte-consistent with its declared source of truth (`node scripts/gen-settings-screens.mjs --check` passes, 121 total), and every msgid it references exists in the i18n catalog (l10n-check: missing 0). Reachability is real: 20+ production files import it (SettingsScreen.tsx:18, persistence.ts:1-2, config-settings.ts:32, settingsThemeSync.ts:44, areas/*.tsx). The problem is not this file but what it feeds: the desktop shell's `config.set-setting` lands in an in-memory `BTreeMap` in apps/cli/src/host.rs:181/1404 that NO engine subsystem reads (see F-001), and 19 of the 121 entries are exported-but-unrendered dead data (F-002). The schema itself is the best-quality file in my assignment.

## 2. What actually works (verified)
— verified by `node scripts/gen-settings-screens.mjs --check` -> `gen-settings-screens --check: OK (10 ui areas, 102 ui entries, 121 total)` (exit 0): no drift from schemas/settings.toml + product.toml.
— verified by `node scripts/l10n-check.mjs` -> `PASS l10n:check — 1448 MessageKeys ... missing 0, duplicates 0, orphans 0` (exit 0): every `description`/`enumeration[].msgid` in this file resolves.
— verified by python set-diff: `consts = 121`, `SETTINGS_ENTRIES = 102`, `SETTINGS_AREAS = 102`, areas set == entries set; spot-checked `engine.tick-interval-ms`, `privacy.lastfm-password`, `playback.start-paused` exist in schemas/settings.toml (sed L578/653/1339).

## 3. Findings
F-001 [S2]
Where: ui/src/screens/settings/generated/schema.ts:1-4 (file contract) vs apps/cli/src/host.rs:1366-1406
Code:
```
// GENERATED FILE — do not edit (card 4.E1, doc 07 §5.3 generator G1).
// Source of truth: schemas/settings.toml (card 1.B2) with default_ref
// targets and feature exposure resolved from product.toml.
```
What is wrong: not this file — the effect chain it documents. 102 rendered settings round-trip `config.set-setting` into `self.settings.insert(full, ...)` (host.rs:1404) whose ONLY reader is `setting_get` (host.rs:1372). No playback/audio/video/demux code path reads the map. `restartRequired: true` (18 entries) implies persisted-across-restart semantics; the desktop shell has no settings persistence at all (in-memory BTreeMap, process lifetime). Why it matters: the settings UI is "wired to a real backend" only in the transport sense — the backend is a write-only echo store. What correct looks like: engine consumption of the validated store or an honest "no effect in this build" marker in the schema/UI. Confidence: HIGH — Verification: read (host.rs:1366-1406 via Read tool; rg `self.settings` -> exactly 3 hits: decl/init/get/insert).
(Owner: apps/cli — cross-file, flagged for that wave's owner.)

F-002 [S3]
Where: ui/src/screens/settings/generated/schema.ts — 19 consts, e.g. SETTING_ENGINE_LOG_LEVEL (L2026), SETTING_INTERFACE_WINDOW_WIDTH (L606), SETTING_EXTENSIONS_CRASH_RESTART_LIMIT (L2206), SETTING_PRIVACY_TELEMETRY (L686)
Code:
```
export const SETTING_PRIVACY_TELEMETRY: SettingEntry = {
  key: "privacy.telemetry",
  ...
  featureGate: "",
  featureExposed: true,
```
What is wrong: 19 consts (all 6 engine.*, 5 extensions.crash-*, 5 interface.window-*/start-maximized, privacy.telemetry/crash-reports/usage-stats) are exported with `featureExposed: true` but are NOT in `SETTINGS_ENTRIES`/`SETTINGS_AREAS` — python diff `consts - entries = 19`. rg finds ZERO production importers for these 19 names (only tests). They are unreachable data: no screen renders them, `createConfigSettingsPersistence(client, SETTINGS_ENTRIES)` (SettingsScreen.tsx:461) never hydrates them. Why it matters: dead 700+ LOC of generated surface; "exposed" flag lies for these rows. Confidence: HIGH — Verification: ran (python diff + rg scan, both exits 0).

F-003 [S3]
Where: ui/src/screens/settings/generated/schema.ts:48-49, 82 (`cliFlag`) and all 121 entries
Code:
```
  readonly cliFlag: string;
```
What is wrong: `cliFlag` is populated for all 121 entries but consumed by NOTHING outside this generated file (rg `cliFlag` in src excluding generated/tests -> 0 hits). The flags it names (e.g. `--interface-hide-controls-after`) do not exist as lumen-cli flags — they are VLC-schema-heritage strings. Dead data field shipped in every entry. Confidence: HIGH — Verification: ran rg (exit 0, no hits).

F-004 [S4]
Where: ui/src/screens/settings/generated/schema.ts:866-884
Code:
```
export const SETTING_PRIVACY_LASTFM_PASSWORD: SettingEntry = {
  key: "privacy.lastfm-password",
  type: "string",
```
What is wrong: a password is typed as plain `string` with no secret/masking marker in the type vocabulary (`SettingValueType` L9-19 has no secret kind), so any generic well renders it unmasked and localStorage persistence (browser seam) stores it in cleartext. Confidence: MEDIUM (rendering behavior lives in the privacy area file, not audited by me) — Verification: read.

F-005 [S4]
Where: ui/src/screens/settings/generated/schema.ts:986-989
Code:
```
export const SETTING_PLAYBACK_START_PAUSED: SettingEntry = {
  key: "playback.start-paused",
  type: "bool",
  description: "settings.playback.start-playing",
```
What is wrong: key says start-**paused**, msgid says start-**playing** (inverted polarity in the label contract; schemas/settings.toml:653 carries the same mismatch, so this faithfully reproduces a source-of-truth defect). A user reading "Start playing: false" cannot tell which polarity applies. Confidence: HIGH — Verification: read + sed on schemas/settings.toml:653.

## 4. Per-function table (no functions — data inventory instead)
| Export | Verdict | Severity | Notes |
|---|---|---|---|
| SettingScalarValue/SettingValue/SettingValueType (L7-19) | REAL | — | type vocabulary, no secret kind (F-004) |
| SettingEnumOption/SettingEntry/SettingsSection/SettingsArea (L21-64) | REAL | — | shape consumed by 20+ files |
| 121 x SETTING_* consts (L66-2723) | REAL data | S3 | 102 rendered, 19 dead (F-002); cliFlag dead field (F-003) |
| SETTINGS_ENTRIES (L2487) | REAL | — | 102 entries; hydration bound for config-settings |
| SETTINGS_AREAS (L2592) | REAL | — | 10 areas, 37 sections; drives SettingsScreen nav |
| i18n msgid export list (tail) | REAL | — | verified complete by l10n-check |

## 5. Test honesty
No tests in file. Downstream: areas.test.tsx / search.test.ts assert every SETTINGS_ENTRIES row has a msgid etc. — verified by baseline vitest 1705 passed (orchestrator baseline §13; not re-run).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Source of truth: schemas/settings.toml ... resolved from product.toml" (L2-3) | file header | REAL | `--check` exit 0, 121 total |
| "Regenerate: npm run settings:generate Drift guard: npm run settings:check" (L4) | file header | REAL | package.json:24-25 defines both; check ran clean |
| featureExposed resolved from product.toml [features] | L40-41 | FALSE for 19 rows | featureExposed:true but no area/entry placement, 0 importers (F-002) |

## 7. Dead / duplicated / unreachable code
19 dead consts (F-002); `cliFlag` dead field (F-003); `since` field — consumed nowhere in production (same rg scan) — S4-grade dead data (not separately numbered).

## 8. Dependencies & flags
No imports. Consumers: 20+ files (list in §1). Feature gates appearing in data: `recording` (2 entries, featureExposed:false), `hw-decode` (1, false), `wasm-extensions` (1, false) — consistent with product.toml features; those 4 rows are correctly hidden.

## 9. Open questions for the next agent
- privacy-network.tsx owner: is `privacy.lastfm-password` rendered masked? (F-004 follow-up)
- apps/cli owner: F-001 — is any engine consumer of `host.settings` planned/landed since Wave 2? (rg says no readers.)
- Who renders `restartRequired` honesty in the UI (Apply banner)? If it claims persistence, that claim is false in the desktop shell.
