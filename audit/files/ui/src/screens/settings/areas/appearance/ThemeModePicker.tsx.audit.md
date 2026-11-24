# AUDIT: ui/src/screens/settings/areas/appearance/ThemeModePicker.tsx (compact, 91 LOC — spec §13)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 91 / 2 (ThemeModePicker, isModeSetting) |
| File verdict | REAL (fully wired: schema entry -> settings provider dirty/apply -> persistence; live preview through ThemeContext) |
| Reachable from shipped entry point? | YES — appearance area of the settings screen; renders when schema entry `interface.theme` (schema.ts:87-103, screen "global"/section appearance) hits the special-widget branch |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg theme schemas/generated/settings-docs.md` -> :99 enum `dark, light, auto`; schema.ts:96 enumeration carries dark/light/auto; read of full file |

## 1. One-paragraph verdict
This picker is on the genuinely-wired side of the settings surface: the
segmented control writes through the settings provider's dirty/apply
contract (setValue -> persistence: engine config over lumen_dispatch in the
desktop shell, localStorage in the browser), and every value it can write
is a REAL engine enum member — the engine's `interface.theme` is
`dark, light, auto` (settings-docs.md:99) and the UI schema enumeration
already carries all three (schema.ts:96), so nothing dispatches an invented
value. The live-preview effect pushes draft values into the ThemeProvider
(:76-79), covering "auto" beyond the shell-level sync. The only defect is
dead defensive code: the "system" cell-value branch can never trigger
because the schema/engine vocabulary is "auto".

## 2. What actually works (verified)
— Wire chain verified by reading: SegmentedControl onChange -> setValue
(entry.key) -> SettingsProvider -> persistence seam (config-settings.ts
audit) -> `config.set-setting` over lumen_dispatch. Key exists engine-side
(settings-docs.md:99 + CLI flag --interface-theme).
— Unknown persisted values render NO active cell (:71) instead of guessing
— honest unknown handling.
— Covered by the settings/vitest suites (baseline 1705 passed).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/settings/areas/appearance/ThemeModePicker.tsx:36-37,55
Code:
  system: "onboarding.appearance.theme.system",
  auto: "onboarding.appearance.theme.system",
...
  const systemValue = entry.enumeration.some((o) => o.value === "system") ? "system" : "auto";
What is wrong: the schema enumeration (schema.ts:96) and the engine
(settings-docs.md:99) use `auto`, never `system` — the "system" branch of
systemValue and the `system:` fallback row are unreachable defensive code
carrying vocabulary from a different lane (onboarding). Harmless but
misleading to the next reader. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table (compact)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| isModeSetting | 41-42 | REAL | guards the preview effect |
| ThemeModePicker | 44-91 | REAL | wired control; F-001 dead branch |

## 5. Test honesty
No dedicated test file for this picker (special-widget area); the segmented
control itself is test-covered and the settings screens are generated +
checked (`settings:check`). UNVERIFIED: no direct mutation test found for
the preview effect — a broken setModeSetting call would not fail an
obvious test.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the System cell writes the ThemeProvider's own 'auto' mode-setting id ... until the seed lane grows the enum" | :21-23 | STALE | the seed enum ALREADY carries auto (schema.ts:96) — the workaround description lags the schema |
| "never invented UI state" | :23-24 | REAL | all three values are engine enum members (verified) |

## 7. Dead / duplicated / unreachable code
F-001 ("system" branch). No duplication.

## 8. Dependencies & flags
SegmentedControl, useSettings (provider), ThemeContext,
useSettingsThemeSync, i18n w26.f03. No flags.

## 9. Open questions for the next agent
- SegmentedControl auditor: does it render `value=""` (the unknown-value
  case, :86) as no-active-cell, or does "" match an option id and light a
  wrong cell?
