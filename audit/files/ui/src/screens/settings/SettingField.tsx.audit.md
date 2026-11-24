# AUDIT: ui/src/screens/settings/SettingField.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 434 / 9 (labelOf, numberStep, clampToSchema, secondsDuration, formatDurationMs, parseDurationMs, formatFloatValue, SettingInfoDot, SettingField) |
| File verdict | REAL-UNPROVEN (schema-driven rows wired to the real settings store; desktop Apply reaches the engine) |
| Reachable from shipped entry point? | YES — App.tsx:322 `/settings/:area` -> SettingsScreen -> SettingsProvider -> SettingsShell/SettingsSections -> SettingField per schema entry |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | sed quote verify (L331-343); grep i18n keys (L1927, L2731); read SettingsProvider.tsx + persistence.ts + SettingsScreen.tsx:449-500 for the persistence chain |

## 1. One-paragraph verdict
This is the REAL settings row renderer: every schema entry becomes a labeled, dirty-tracked control bound to the SettingsProvider store (`useSettings()` -> value/setValue/isDirty/resetField), with type-correct controls (bool->Switch, enum->Dropdown with honest raw-value fallback for unregistered msgids, numeric wells with schema clamping, draft-while-typing semantics). The master question resolves YES here for the desktop shell: Apply commits through `createConfigSettingsPersistence` (SettingsScreen.tsx:460-467) issuing real `config.set-setting` / `config.reset-setting` over the client bridge with `config.setting-changed` event sync; the browser lane stays on the localStorage mock seam (persistence.ts:4-9 says "MOCK persistence seam" openly, demo transport answers feature-not-exposed for config.*). The one dead button: every `path`-type row's Browse companion is permanently disabled.

## 2. What actually works (verified)
— verified by read chain: SettingField -> SettingsProvider (draft/persisted/isDirty) -> persistence seam; SettingsScreen.tsx:460-467 desktop bridge selection (`isDesktopShell() ? createConfigSettingsPersistence(client, SETTINGS_ENTRIES) : null`); persistence.ts localStorage fallback with per-key validation on load. Wave-0 baseline: UI tests pass.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/settings/SettingField.tsx:331-343
Code:
```
        <button
          type="button"
          className="settings-action settings-field__browse-action"
          disabled
          aria-describedby={browseNoteId}
          title={m("settings.capability-unavailable")}
        >
          {m("settings.field.browse")}
        </button>
```
What is wrong: every path-type row (global.log-file, interface.skin-file, recording.directory) renders a Browse button that is unconditionally `disabled` — "no file-dialog command in the catalog" (comment L314-317). Dead chrome on three rows; honest tooltip "Not available in this build" (messages.ts:1927) + sr note. Why it matters: the primary way users fill a path field (picking a file) is unavailable; typing full paths by hand is the only path. What correct looks like: a plugin:dialog open command seam like the OS window controls, or omit the button. Confidence: HIGH — Verification: read + grep.
F-002 [S4]
Where: ui/src/screens/settings/SettingField.tsx:60
Code:
```
const labelOf = (entry: SettingEntry): MessageKey => entry.description as MessageKey;
```
What is wrong: unchecked cast of the generated `description` to MessageKey; if a schema description msgid is unregistered the row label renders "" (settingsMsg.ts documents empty-string degradation). Enum OPTIONS have a raw-value fallback (L241) but row LABELS do not. Blast radius: cosmetic (blank label), schema msgids currently registered. Confidence: MEDIUM — Verification: read (no unregistered example found in current schema).

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| labelOf | 60 | REAL-UNPROVEN | S4 | cast, see F-002 |
| numberStep | 65-69 | REAL-UNPROVEN | — | float 0.05 / duration 50 / int 1 |
| clampToSchema | 72-76 | REAL-UNPROVEN | — | min/max clamp, ±Infinity safe |
| secondsDuration | 84 | REAL-UNPROVEN | — | key-prefix rule (controls.skip*) |
| formatDurationMs | 86-87 | REAL-UNPROVEN | — | "3 s" vs "5000 ms" |
| parseDurationMs | 89-95 | REAL-UNPROVEN | — | strips non-digits, NaN-guarded, unit-aware |
| formatFloatValue | 98-99 | REAL-UNPROVEN | — | one-decimal minimum |
| SettingInfoDot | 124-162 | REAL-UNPROVEN | — | real button, aria-expanded/controls/describedby; exported, reused by all areas |
| SettingField | 175-433 | REAL-UNPROVEN | S3 | full type->control matrix; feature-gated rows render null (L185); Browse dead button (F-001) |

## 5. Test honesty
SettingField.test.tsx exists (settings dir listing); not read in full — UNVERIFIED coverage depth. The store contract is tested in SettingsProvider.test.ts / persistence.test.ts (per-keystroke draft semantics, load validation documented in persistence.ts:19-27 comments from review w26.k12).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every option is bound to a REAL schema key; gated-but-unexposed features render nothing" | header L36-37 | REAL | L185 `if (!entry.featureExposed) return null;` + L188 compound-key guard |
| "The REAL write path is the app layer's — config.set-setting over the client bridge" (handoff) | persistence.ts:6-8 | REAL on desktop / MOCK in browser | SettingsScreen.tsx:460-467 bridge selection; config-settings.ts:138 `cmd: "config.set-setting"` |
| "an option msgid the catalog has not registered ... fall back to the RAW enum value" | L227-232 | REAL | L241 `labelKey: m(o.msgid as SettingsMsgKey) || o.value` |

## 7. Dead / duplicated / unreachable code
- Browse button (F-001) — dead on all path rows.
- No dead code otherwise; special-widget registry path (SPECIAL_FIELD_WIDGETS) is live.

## 8. Dependencies & flags
form/menu primitives, generated schema, SettingsProvider, settingsMsg lane catalog, specialWidgets, setting-field.css. Feature exposure via `entry.featureExposed` (product.toml [features] — engine lane owns truth).

## 9. Open questions for the next agent
- Does `config.set-setting` reach the engine's real config store in the Rust layer (config lane)? The UI half is verified here.
- Is there a file-dialog plugin command anywhere on the roadmap (Browse seam)?
