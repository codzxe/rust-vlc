# AUDIT: ui/src/screens/settings/areas/appearance/ThemeModePicker.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 91 / 4 (laneCatalog, LABEL_FALLBACK_KEYS, isModeSetting, ThemeModePicker [cells build + preview effect + render]) |
| File verdict | REAL (behaves correctly; header describes a transitional state that ended — three stale claims, one redundant effect) |
| Reachable from shipped entry point? | YES in the web UI (specialWidgets.ts:58 registers ThemeModePicker for `interface.theme`; schema.ts:96-100 places the key on the GLOBAL screen's appearance section, featureExposed — so /settings/global renders it through SettingField's special-widget leaf). NO desktop binary (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read theme/context.ts (ThemeModeSetting = dark/light/auto; nullable context) + theme/settingsThemeSync.ts:1-121 (the sync forwards "auto" and bootstraps persisted mode+accent — w26.f08 e19-F5); sed schemas/settings.toml:74-92 (enumeration = ["dark","light","auto"] — the k12 fix comment) + generated/schema.ts:86-100 (same three-value enum, msgid settings.value.interface-theme.auto); Read crates/settings-schema/src/runtime.rs:16-19 (EMBEDDED_SOURCE = include_str! schemas/settings.toml — the engine validates against the SAME file, so config.set-setting accepts "auto"); rg interface-theme in messages.ts (:2628 "settings.value.interface-theme.auto": "System" — the third cell labels "System", frame-true); Grep ThemeModePicker in ui/src (specialWidgets registration + test); Read ThemeModePicker.test.tsx:23-70 (4 behavioral tests). Baseline: ui-vitest 136 files / 1705 passed. |

## 1. One-paragraph verdict
The picker does the right things: three frame cells (Light/Dark/System) as a SegmentedControl radiogroup, cell clicks write real schema values into the settings draft (System → "auto", which the schema enumeration NOW carries and the engine's compile-time-embedded schema validates — verified through runtime.rs's include_str! of the very settings.toml that lists "auto"), unknown persisted values select no cell instead of guessing, and the live preview pushes the draft into ThemeProvider so data-theme follows immediately (test-pinned through document.documentElement.dataset.theme). Theme is one of the few settings that genuinely works end-to-end in the web product (localStorage persistence + bootstrap read — W15-A02's exception to the engine's never-persist). The defects are all staleness: the header claims the schema carries "dark/light today", that the System cell writes a ThemeProvider-only id "until the seed lane grows the enum", and that the shell sync "only carries dark/light" — all three were true once and are false now (k12 grew the enum; e19-F5 grew the sync), and on the strength of the third stale claim the file keeps a local preview effect that duplicates the sync hook it itself mounts.

## 2. What actually works (verified)
— verified by ThemeModePicker.test.tsx:24-70 (three cells Light/Dark/System in a radiogroup with the segmented grammar; no stale hint paragraph; click binding: Light→draft "light" + data-theme "light", System→draft "auto", Dark→"dark"; arrow-key roving) + baseline ui-vitest (136 files, 1705 passed).
— verified by reading the full chain: schema.ts:96 (enum dark/light/auto) → settings.toml:80 (same) → settings-schema runtime.rs:18-19 (embedded via include_str! of that file) → apps/cli host.rs:1390-1407 (setting_set validates through that schema) — an Apply of "auto" is engine-valid, no InvalidArgument.
— verified by reading settingsThemeSync.ts:99-121: the mounted sync pushes draft dark/light/auto AND the accent draft into ThemeProvider, and readPersistedThemeMode() bootstraps every route from the same localStorage seam — the picker's preview is covered twice, harmlessly.

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/settings/areas/appearance/ThemeModePicker.tsx:20-27 (header)
Code:
```ts
// Values: the schema enumeration is the truth for the cells it carries
// (dark/light today); the System cell writes the ThemeProvider's own
// "auto" mode-setting id (theme/context.ts) until the seed lane grows the
// enum — the value is still a real theme-system setting, never invented
// UI state.
```
What is wrong: Three claims, all stale. (1) "dark/light today": the schema enumeration is ["dark","light","auto"] (schemas/settings.toml:80 — its own comment documents the k12 fix; generated schema.ts:96 mirrors it). (2) "the System cell writes the ThemeProvider's own 'auto' mode-setting id": "auto" is now a first-class SCHEMA value with its own msgid (settings.value.interface-theme.auto = "System"), not a ThemeProvider-only id. (3) "until the seed lane grows the enum": it grew. The same staleness is copy-pasted in settingsThemeSync.ts:23-28 ("the REAL engine write path … will reject 'auto' until the schema owner adds it" — false: the engine embeds the fixed file).
Why it matters: The header tells the next developer the engine will reject "auto" on Apply; it will not. Anyone "fixing" the picker against this comment would be fixing a solved problem.
What correct looks like: "The schema enumeration (dark/light/auto) is the truth for all three cells; System writes 'auto', which the engine schema validates (k12)."
Confidence: HIGH — Verification: schema.toml, generated schema, runtime.rs include_str!, host.rs validation path all read.
### F-002 [S4]
Where: ui/src/screens/settings/areas/appearance/ThemeModePicker.tsx:76-79 (preview effect) vs settingsThemeSync.ts:99-121
Code:
```ts
  useEffect(() => {
    if (theme === null || !isModeSetting(preview)) return;
    theme.setModeSetting(preview);
  }, [preview, theme]);
```
What is wrong: Redundant with the sync hook this component itself mounts at :49. useSettingsThemeSync already pushes every valid draft value — "auto" included (its own doc: "'auto' … is now a first-class value on this seam — the sync forwards it", settingsThemeSync.ts:14-16) — into ThemeProvider. The local effect's only addition is the system→auto mapping at :72, which can never fire: the draft can only hold cell values (light/dark/auto), never "system" (no cell writes it, no schema value carries it). The header's justification ("the shell's useSettingsThemeSync only carries dark/light; this picker extends the preview to 'auto' locally") is the stale claim F-001 documents. The sync's own doc (:33-35) blesses the picker effect staying mounted ("idempotent per provider pair"), so this is documented duplication, not a bug.
Why it matters: Two effects setModeSetting on the same change — harmless today (same value), but a second place to drift when mode semantics change, and dead-weight justification in the header.
What correct looks like: Delete the local effect + the preview/systemValue mapping that only serves it (keep systemValue for the cell value), OR delete the local useSettingsThemeSync mount and keep the local effect — one owner of the preview.
Confidence: HIGH — Verification: both files read; no cell or schema value produces "system" (rg over schema enum + cells array).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `laneCatalog` | 30 | REAL | — | Spread copy of lane keys (w26.f03 seam). |
| `LABEL_FALLBACK_KEYS` | 33-38 | REAL | — | Fallback msgids incl. system/auto → "System"; schema msgid takes priority when the option exists (it does — label renders "System"). |
| `isModeSetting` | 41-42 | REAL | — | Type guard matching ThemeModeSetting. |
| `ThemeModePicker` | 44-91 | REAL | S4 | Draft binding via useSettings().setValue (dirty/apply contract); unknown-value no-selection honest (:71); live preview duplicated (F-002); header stale (F-001). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :24 three frame cells in a radiogroup | YES | Role/name/count assertions. |
| :34 no stale "applies instantly" hint | YES | Negative assertion. |
| :39 cell clicks bind draft + preview live | YES | Clicks → draft value probe + documentElement.dataset.theme — pins the System→"auto" write and the live preview. |
| :66 arrow-key roving | YES | Keyboard interaction assertion. |
| Gap: no test pins the no-selection state for an out-of-enum persisted value (the :71 guard is unpinned). | | |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the schema enumeration is the truth for the cells it carries (dark/light today)" | ThemeModePicker.tsx:20-21 | FALSE (stale) | schema.toml:80 / schema.ts:96 carry auto. |
| "the System cell writes the ThemeProvider's own 'auto' mode-setting id … until the seed lane grows the enum" | ThemeModePicker.tsx:21-24 | FALSE (stale) | auto is a schema value with msgid (schema.ts:96, messages.ts:2628). |
| "the shell's useSettingsThemeSync only carries dark/light; this picker extends the preview to 'auto' locally" | ThemeModePicker.tsx:25-27 | FALSE (stale) | settingsThemeSync.ts:14-16, :108-113 forwards auto. |
| "The draft binds through the settings provider (dirty/apply contract)" | ThemeModePicker.tsx:24-25 | REAL | setValue via useSettings; test :39-63. |
| (cross-file) "the REAL engine write path (config.set-setting …) will reject 'auto' until the schema owner adds it" | settingsThemeSync.ts:26-28 | FALSE (stale) | runtime.rs:18-19 embeds the fixed settings.toml; host.rs:1400-1402 validates through it. |

## 7. Dead / duplicated / unreachable code
Duplicated: the local preview effect (F-002). Dead-in-practice: the `current === "system"` mapping at :72 (no producer of "system"). Unreachable: none.

## 8. Dependencies & flags
SegmentedControl (form primitive), useMsg + w26.f03 lane keys, generated schema (SettingEntry), SettingsProvider (useSettings), ThemeContext, useSettingsThemeSync. Registered in specialWidgets.ts:58 keyed by the schema's theme entry. No feature flags. Engine-side acceptance of "auto" verified through the embedded schema.

## 9. Open questions for the next agent
- The stale "engine will reject auto" comment in settingsThemeSync.ts:23-28 should be fixed by its owner together with this file's header (F-001) — same fix, two files.
- If the local effect is deleted (F-002), the `display`/`preview` split collapses too — one reviewer should own both.
