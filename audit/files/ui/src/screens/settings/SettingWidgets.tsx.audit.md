# AUDIT: ui/src/screens/settings/SettingWidgets.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 338 / 8 (4 exported widgets/helpers + 1 private input + specs/probes) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle — /settings/:area → SettingField consults SPECIAL_FIELD_WIDGETS (specialWidgets.ts:61-63) which registers InvertedSwitchField (playback.start-paused), StartupVolumeField (audio.startup-volume) and SettingsSliderField (8 slider keys). W14 caveat: no shipped desktop binary (build-desktop-tauri.log.txt exit 101). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | rg keys in generated/schema.ts + sed ranges 547-560/991-1020/1327-1425/1547-1605 (all 10 widget keys exist; slider min/max match schema — one documented exception F-004); cat specialWidgets.ts (registry consumers); rg -ln 'useSettings' outside settings/ (only settingsThemeSync + onboarding — NO player consumer); read useRadiogroupKeyboard.ts (no event-target guard → F-001). |

## 1. One-paragraph verdict
Real, carefully-built settings widgets: every key is a real schema key, every slider spec's min/max matches the generated schema (verified key-by-key), the inverted switch's polarity math is correct, and the Values-1 composite implements the roving-radiogroup pattern. Two real defects: the radiogroup keydown handler is attached to a host containing a nested editable text input, so arrow keys pressed while editing the percent field hijack the caret and flip the radio selection; and nothing in the UI ever reads the values these widgets write — no player code consumes settings, so in the only executable configuration (browser demo) the controls are inert past localStorage, and the desktop-shell engine bridge that would consume them cannot build at HEAD (W14). This file is where the W11 "settings never persist" claim lands in the widget layer: writes are real, consumers are not.

## 2. What actually works (verified)
— Schema cross-check: all 10 SLIDER_SPECS/VALUES keys exist in generated/schema.ts with matching ranges (video.hue -180..180, brightness/contrast 0..200, saturation 0..300, gamma 0..1000, hide-controls-after 0..10000, maximum-volume 100..300, startup-volume 0..300 with clamp at SettingWidgets.tsx:252 matching). Registry wiring confirmed (specialWidgets.ts:61-63). Baseline: SettingWidgets suites pass (ui-vitest.log.txt); `tsc --noEmit` clean (ui-build.log.txt).

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/settings/SettingWidgets.tsx:280-294 (+ useRadiogroupKeyboard.ts:28-38)
Code:
```
  const onGroupKeyDown = useRadiogroupKeyboard(
    [
      { value: "remember-last" },
      { value: "always" },
    ],
    mode,
    (next) => setValue(SETTING_AUDIO_STARTUP_VOLUME_MODE.key, next),
  );
```
```
    const vertical = event.key === "ArrowDown" || event.key === "ArrowUp";
    const horizontal = event.key === "ArrowRight" || event.key === "ArrowLeft";
    if (!vertical && !horizontal) return;
```
What is wrong: the roving-arrow handler has NO event-target guard, and the group host (SettingWidgets.tsx:294 `onKeyDown={onGroupKeyDown}`) contains the nested editable `StartupVolumeInput` (242-256), enabled exactly when "always" is selected. A keyboard user focusing the percent input and pressing ArrowLeft/ArrowRight to move the caret gets: (1) the mode flipped to "remember-last" (two options, wrap-around — any arrow from "always" selects the other), which DISABLES the input mid-edit, and (2) `event.preventDefault()` (useRadiogroupKeyboard.ts:38), so the caret never moves. The input cannot be edited with arrows at all.
Why it matters: realistic keyboard path breaks a control and flips state the user did not ask for; the sibling primitives (RadioGroup/SegmentedControl/SwatchGroup) have no nested focusable so only this composite is affected.
What correct looks like: guard in the hook (`if (event.target !== event.currentTarget && isEditableTarget(event.target)) return;`) or attach the handler to the two radio buttons instead of the group host.
Confidence: HIGH — code path traced end-to-end; not executed (would need jsdom focus test).
### F-002 [S2]
Where: ui/src/screens/settings/SettingWidgets.tsx:131,171,223,286,332 (every `setValue`) — consumer gap proven by rg
Code:
```
  const { value, setValue } = useSettings();
```
What is wrong: NOTHING in the UI reads these values back. rg for `useSettings`/`SettingsProvider` outside settings/onboarding returns only theme/settingsThemeSync (interface.theme/accent keys — not this file's keys). The player, transport, and controls never consult `audio.startup-volume`, `audio.maximum-volume`, `playback.default-rate`, `video.hue/brightness/contrast/saturation/gamma`, `playback.start-paused` or `interface.hide-controls-after`. The store's Apply writes localStorage (browser — the only executable shell today) or the engine bridge (desktop shell, SettingsScreen.tsx:459-464 — real code via config.set-setting, but its host binary fails to compile at HEAD, W14 build-desktop-tauri.log.txt exit 101; engine-side effect UNVERIFIED).
Why it matters: ten controls whose effect is invisible in every currently-executable configuration — the W11 "settings never persist" finding expressed at widget level: writes are real, consumers are absent.
What correct looks like: either the player consumes the store (startup volume, default rate, start-paused on session.open) or the settings screen states plainly that these are engine-side preferences requiring the desktop shell.
Confidence: HIGH for the UI-side absence (rg exhaustive); engine-side effect UNVERIFIED (cross-wave).
### F-003 [S4]
Where: ui/src/screens/settings/SettingWidgets.tsx:278
Code:
```
  const volume = typeof volumeRaw === "number" ? volumeRaw : 20;
```
What is wrong: the defensive fallback is 20, but the schema default for `audio.startup-volume` is 200 (generated/schema.ts:1587-1595, defaultValue: 200). If the branch ever fired (store guarantees it shouldn't), the input would show 20% instead of the schema's 200%.
Why it matters: inconsistent fallback constant — a lie if reached, dead if not.
What correct looks like: `?? entry.defaultValue` (the number it already has in scope).
Confidence: HIGH — read both sites.
### F-004 [S4]
Where: ui/src/screens/settings/SettingWidgets.tsx:70-73
Code:
```
  "playback.default-rate": {
    min: 0.25,
    max: 2,
```
What is wrong: the slider caps at 2× while the schema validates 0.25..4 (generated/schema.ts:1007+, max: 4) — documented as the frame's span (g11-P2-3), so the UI silently cannot express schema-legal values 2.25–4×.
Why it matters: schema/UI range drift; a value set to 3 by the engine or CLI displays fine on... actually it would clamp display? No — the Slider receives value=3 with max=2: a controlled slider outside its range. Edge behavior depends on the Slider primitive.
What correct looks like: slider max 4 with the frame's 0.25–2 scale labels confined to the frame span, or a schema max of 2.
Confidence: MEDIUM — the out-of-range display behavior depends on Slider (another agent's file); the drift itself is HIGH (both files read).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ScaleMark` / `SliderSpec` (types) | 31-46 | REAL | — | |
| `numberLabel` | 48 | REAL | — | |
| `SLIDER_SPECS` (const) | 51-108 | REAL | S4 (F-004) | ranges match schema except default-rate max; scale math (at−0.25)/1.75 verified |
| `isSliderFieldKey` / `isValuesFieldKey` | 112-114 | REAL | — | consumed by SettingField.tsx:212 |
| `WIDGET_GROUPED_KEYS` | 122 | REAL | — | startup-volume-mode never standalone |
| `SettingsSliderField` | 129-207 | REAL | S2 (F-002) | unregistered-key fallback uses schema range — honest |
| `InvertedSwitchField` | 215-226 | REAL | S2 (F-002) | inversion math correct (checked !== true, persists !checked) |
| `StartupVolumeInput` | 230-257 | REAL | — | digit-strip + clamp 0..300 matches schema; empty-input no-commit correct |
| `StartupVolumeField` | 272-338 | REAL | S3 (F-001) | roving tabindex correct; arrows hijack the nested input |

## 5. Test honesty
SettingWidgets.test.tsx belongs to another agent. Mutation reasoning: break the inversion (`checked={value(entry.key) === true}`) → InvertedSwitch tests must fail; break the clamp → StartupVolumeInput tests must fail; the F-001 arrow-hijack requires a focused-input test — if the suite only fires keydown on the group host, it will NOT catch the bug (the handler works there by design).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Controlled: commits through the settings store on release/step" | SettingWidgets.tsx:126-127 | REAL | setValue → SettingsProvider draft; Apply → persistence |
| "the persisted key keeps its negative semantics … Clicking ON→OFF persists start_paused = true" | SettingWidgets.tsx:210-213 | REAL | lines 222-223 |
| "Binds BOTH schema keys: the mode enum drives the selection, the int drives the input" | SettingWidgets.tsx:262-264 | REAL | 275-287, 332 |
| schema keys are real ("every key MUST be a real schemas/settings.toml key") | specialWidgets.ts:26-27 | REAL | all 10 verified in generated/schema.ts |

## 7. Dead / duplicated / unreachable code
No dead code in this file. The values it writes are unconsumed (F-002) — dead data flow, not dead code.

## 8. Dependencies & flags
Slider, Switch, useRadiogroupKeyboard (F-001), useSettingsMsg, generated schema + SETTING_AUDIO_STARTUP_VOLUME_MODE, SettingsProvider. All imports exist.

## 9. Open questions for the next agent
- useRadiogroupKeyboard owner (asset-group w15-g003, my next audit): does ANY other consumer nest a focusable/editable inside the group host? (RadioGroup/SegmentedControl/SwatchGroup: no, verified by usage — but the hook should defend anyway.)
- Slider primitive owner: what does a controlled value > max render (F-004's 3× case)?
- Engine side: does the Rust config store apply video.hue/gamma etc. to the pipeline, or is config.set-setting write-only there too? (Cross-wave; determines whether F-002 upgrades to S0-class "settings do nothing" forever.)
