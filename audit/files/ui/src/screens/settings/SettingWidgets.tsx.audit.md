# AUDIT: ui/src/screens/settings/SettingWidgets.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 338 / 6 (isSliderFieldKey, isValuesFieldKey, SettingsSliderField, InvertedSwitchField, StartupVolumeInput, StartupVolumeField) + 2 exported key sets |
| File verdict | REAL (backend-wired via settings store in desktop shell) |
| Reachable from shipped entry point? | YES (specialWidgets.ts:7-9 imports the three fields -> SettingField renders them in the standard row inside SettingsScreen, routed /settings/:area) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg specialWidgets registry; rg SettingsProvider persistence wiring; sed/od quote verification |

## 1. One-paragraph verdict
Real special-case settings widgets (frame-measured sliders with ticks/scale
labels, an inverted switch, a Values-1 radiogroup with nested percent input).
Crucially for the master question, these controls ARE backend-wired in the
desktop shell: setValue() writes the draft, Apply() hands overrides to the
persistence seam, and SettingsScreen.tsx:460-461 constructs that seam as
`createConfigSettingsPersistence(client, ...)` — real `config.set-setting` /
`config.reset-setting` commands over the client bridge (config-settings.ts:138),
with a hydration gate before first render. The browser dev shell honestly
falls back to localStorage (demo transport answers feature-not-exposed for
config.*). Findings: the nested percent input commits on every keystroke into a
reformatted controlled value (UX hazard), and slider specs deliberately cap
below the schema domain (documented).

## 2. What actually works (verified)
— rg wiring chain: SettingWidgets -> specialWidgets.ts:7-9 -> SettingField ->
areas; SettingsScreen.tsx:490 `persistence={config ?? localStorageSettingsPersistence}`
with config = createConfigSettingsPersistence(client, SETTINGS_ENTRIES) when
isDesktopShell() (SettingsScreen.tsx:460-461).
— config-settings.ts:138 dispatches `cmd: "config.set-setting"` — real bridge
command present in src/generated/commands.ts (verified in the generated union).
— od -c on SettingsScreen.tsx:464 proves `const [hydrated, setHydrated]` is
intact (display artifact suspected earlier — NOT a defect).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/settings/SettingWidgets.tsx:242-254
Code:
```
      value={`${volume}%`}
      disabled={disabled}
      aria-label={label}
      onChange={(event) => {
        const next = Number(event.target.value.replace(/[^0-9]/g, ""));
        if (Number.isFinite(next) && event.target.value.trim() !== "") {
          onCommit(Math.min(300, Math.max(0, next)));
        }
      }}
```
What is wrong: The controlled input re-renders as `${volume}%` after EVERY
keystroke commit — typing "150" commits 1, then 15, then 150, each reformat
moving the caret behind the appended "%" (classic controlled-reformat hazard);
typing a leading digit past the "%" text selects nothing, and intermediate
drafts persist to the settings draft store per keystroke. No commit-on-blur or
debounce. Clamping to 300 is silent. Functional but hostile to typing.
What correct looks like: local string state while focused; commit parsed value
on blur/Enter only.
Confidence: MEDIUM — Verification: read (no browser run available); caret
behavior is standard controlled-input behavior.
F-002 [S4]
Where: ui/src/screens/settings/SettingWidgets.tsx:66-69
Code: ` * the frame's slider spans 0.25–2 even though the schema
 * validation domain is 0.25–4; committed values stay inside the schema
 * range, g11-P2-3).`
What is wrong: The UI caps default-rate at 2x while the schema allows 4x —
documented frame-fidelity decision; users cannot set 2.5-4x from this control.
Deliberate, cited, tracked. Debt note only.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SLIDER_SPECS table | 51-108 | REAL | - | frame-measured ranges/ticks, per-key |
| isSliderFieldKey/isValuesFieldKey | 112-114 | REAL | - | row-kind probes |
| WIDGET_GROUPED_KEYS | 116-122 | REAL | - | compound-key filtering contract |
| SettingsSliderField | 129-207 | REAL | - | honest schema-range fallback for unregistered keys (141-152) |
| InvertedSwitchField | 215-226 | REAL | - | documented negative-semantics key |
| StartupVolumeInput | 230-257 | REAL | F-001 | per-keystroke commit + reformat |
| StartupVolumeField | 272-338 | REAL | - | WAI-ARIA radiogroup + roving tabindex (280-287) |

## 5. Test honesty
SettingWidgets.test.tsx covers the radiogroup keyboard/click paths (lines
28-82). No test exercises the nested input's typing behavior — F-001 would
survive the suite. Would the existing tests fail if code broke? YES for radio
roving/commit; NO for input caret/commit-on-keystroke (untested).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Controlled: commits through the settings store on release/step" | SettingWidgets.tsx:127 | REAL | onCommit -> setValue (draft) at 150/171 |
| "The REAL write path is ... `config.set-setting` over the client bridge" | persistence.ts:7-9 | REAL | SettingsScreen.tsx:460-461 + config-settings.ts:138 |

## 7. Dead / duplicated / unreachable code
None — all three fields registered and rendered; both exported key sets
consumed (SettingField + schemaSectionsOf filtering, areas/index.tsx:307-310).

## 8. Dependencies & flags
components/slider/Slider, components/form/Switch, useRadiogroupKeyboard,
settingsMsg lane seam, generated schema (SETTINGS_ENTRIES), SettingsProvider.
No feature flags; desktop-vs-browser split is runtime (isDesktopShell()).

## 9. Open questions for the next agent
SettingsScreen.tsx owner: verify isDesktopShell() is true in the shipped
desktop build (if it can be false there, ALL settings silently degrade to
localStorage — the single point that decides backend wiring for ~100 rows).
