# AUDIT: ui/src/screens/onboarding/OnboardingScreen.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 964 / 18 (12 components/fns + 6 helper const-blocks) |
| File verdict | REAL-UNPROVEN (renders real; bridge call is real but untested against a live engine by me) |
| Reachable from shipped entry point? | YES — App.tsx:11 `import { OnboardingEntry, RequireOnboarding } from "./screens/onboarding/OnboardingScreen"`; /onboarding route + app-wide first-run gate (App.tsx:76-88) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 1 S4: 1 |
| Commands run | `rg -n` function/claim locators; `sed -n` quote verification; read of onboardingState.ts, SettingsProvider.tsx, persistence.ts, open-path.ts |

## 1. One-paragraph verdict
A genuinely implemented, well-tested wizard: 8 steps, real routing gates, real generated-keymap data, and ONE real backend call — the library step's `client.send(openPathRequest(...))` (L656-657) which reaches `session.open`/`playlist.load` over the bridge. Everything else the wizard writes (theme, accent, playback, subtitle, privacy flags) lands in **localStorage only** — `lumen.settings.overrides.v1` / `lumen.onboarding.v1` — and never crosses the bridge. The header's honesty manifesto (L13-27) is accurate about opt-in defaults and keycap data, but its "does something real" claim quietly means "writes a webview-local store", not "configures the engine".

## 2. What actually works (verified)
— verified by reading the call chain: OnboardingScreen L656-657 `client.send(openPathRequest(source, requestId))` → open-path.ts `openPathRequest` → classifyOpenSource → `session.open` (media) or `playlist.load` (playlist) over the shared transport. Reachability chain: App.tsx:11 → routes/gate → OnboardingScreen → LibraryStep. NOT executed against a live engine by me (no dev server run) — REAL-UNPROVEN at runtime, REAL by code path.
— UI tests exist and are behavioral (OnboardingScreen.test.tsx, 543 LOC, 29 `it(` blocks incl. "library step opens a pasted path through the one funnel (session.open)" L204) — but they run against a mock transport, proving UI logic, not engine wiring.

## 3. Findings
F-001 [S2]
Where: ui/src/screens/onboarding/OnboardingScreen.tsx:16-24 (header), 392-511 (PlaybackControls/SubtitleControls)
Code:
```
//   · every control on every step does something real — the appearance
//   ...
//     the playback/subtitles steps write the REAL settings schema
//     (SettingsProvider → the shared overrides store), the media step opens
```
What is wrong: "the REAL settings schema ... the shared overrides store" is localStorage, not the engine. SettingsProvider.apply (SettingsProvider.tsx:105-124) ends in `persistence.save(overrides)`; persistence.ts:1-4 admits: "MOCK persistence seam (card 4.E1)... Today it round-trips localStorage for development; the REAL write path is ... `config.set-setting` over the client bridge". `playback.start-paused`, `playback.default-rate`, `playback.subtitle-mode`, `playback.preferred-subtitle-languages` never reach the Rust engine. Why it matters: the user finishes onboarding believing playback behavior is configured; the engine still runs its own defaults. What correct looks like: Apply() also issuing config.set-setting commands (as persistence.ts itself documents). Confidence: HIGH — Verification: read (SettingsProvider.tsx:91-124, persistence.ts:1-4).

F-002 [S2]
Where: ui/src/screens/onboarding/OnboardingScreen.tsx:843-861 (privacy step Switches → onboardingState)
Code:
```
<Switch
  checked={choices[row.key]}
  onChange={(value) => setChoice(row.key, value)}
```
What is wrong: privacy flags (telemetry, updateChecks, onlineSuggestions) persist only under localStorage key `lumen.onboarding.v1` (onboardingState.ts:47, `const STORAGE_KEY = "lumen.onboarding.v1"`). onboardingState.ts:15-17 states: "Handoff for the REAL config write path (config.set-setting over the client bridge, doc 05 §2.11) is recorded in ui/NEXT-STEPS.md." The engine has no way to learn a user opted out of telemetry. Why it matters: privacy choices are precisely the flags that must be enforced engine-side; a webview-local flag enforces nothing. Confidence: HIGH — Verification: read (onboardingState.ts:9-17, 47-71).

F-003 [S3]
Where: ui/src/screens/onboarding/OnboardingScreen.tsx:633-745 (LibraryStep) — no Browse dialog
What is wrong: the submit button is labeled "Browse" (`msg("onboarding.library.browse")`, L687) but there is no file dialog anywhere — it merely submits the typed text field. A "Browse" button that does not browse is a name that lies (spec 6.5). Confidence: HIGH — Verification: read (no dialog/fetch call in the component; only parseSourceInput + openPathRequest on the text value).

F-004 [S4]
Where: ui/src/screens/onboarding/OnboardingScreen.tsx:90-99
Code:
```
function useOnboardingMsg() {
  const m = useMsg();
  return useCallback(
    (key: MessageKey | LaneKey): string => m(key as MessageKey) ?? laneCatalog[key],
```
What is wrong: `m(key) ?? laneCatalog[key]` — if useMsg returns `""` for a missing key (rather than undefined) the fallback never fires; behavior depends on useMsg's missing-key contract (unchecked here). Minor robustness debt. Confidence: LOW — Verification: read; useMsg contract not audited (other lane).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useOnboardingMsg | 90-99 | REAL | - | i18n fallback; see F-004 |
| STEPS/STEP_TITLES/STEP_BODIES/CENTERED_STEPS | 105-151 | REAL | - | static flow data |
| PRIVACY_ROWS/THEME_CARDS/ACCENT_LABEL_KEYS/ACCENT_ORDER | 158-201 | REAL | - | static |
| isThemeModeDraft/isAccentDraft | 205-210 | REAL | - | total guards |
| BINDING_BY_ID/binding | 213-216 | REAL | - | map over generated HOTKEY_BINDINGS |
| keycapParts/isSymbolKeycap/keycapClass | 255-271 | REAL | - | glyph mapping |
| SHORTCUT_ROWS/COLUMN_* | 274-317 | REAL | - | derives column count from row census |
| ConeMark | 304-339 | REAL | - | pure SVG art, aria-hidden |
| AutoApplySettings | 341-348 | REAL | S2 | effect auto-calls settings.apply → localStorage only (F-001) |
| SettingsRow | 351-368 | REAL | - | layout |
| clampToSchema/parseLanguageList/formatStepLabel | 373-392 | REAL | - | pure helpers, tested |
| PlaybackControls | 392-443 | SHALLOW (backend-wise) | S2 | writes draft+clamped values; localStorage seam only |
| SubtitleControls | 445-511 | SHALLOW (backend-wise) | S2 | draft/blur commit; localStorage seam only |
| AppearanceCards | 513-551 | REAL | S2 | settings.setValue(THEME_SETTING_KEY) → localStorage; live preview via useSettingsThemeSync is real |
| AppearanceAccents | 553-631 | REAL | - | ARIA radiogroup + roving tabindex, wrap-around, focus-follows-selection |
| LibraryStep | 633-728 | REAL-UNPROVEN | - | the ONE bridge call; request-id supersede guard |
| OnboardingScreen | 731-950 | REAL | - | step machine, gates |
| OnboardingEntry/RequireOnboarding | 952-964 | REAL | - | route gates used by App.tsx |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| OnboardingScreen.test.tsx L204 "opens a pasted path through the one funnel (session.open)" | YES for UI logic | asserts the request shape on a mock transport; would NOT catch the engine never implementing session.open |
| L74-150 privacy suite | YES for defaults | real behavioral asserts incl. corrupt-store fold-back |
| L312/357 "persist through the settings overrides store" | YES, but only for localStorage | asserts the mock seam, so "real settings" claim is tested at the wrong seam |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every control on every step does something real" | file header L16 | SHALLOW | real for open-path + preview; settings/privacy writes stop at localStorage (F-001/F-002) |
| "the media step opens through the ONE open-path funnel" | header L22-23 | REAL | L656-657 + open-path.ts |
| "keycaps are the REAL keymap (keymaps/default.toml via the generated bindings)" | header L24-25 | REAL-UNPROVEN | HOTKEY_BINDINGS import L45; bindings.ts header says generated from keymaps/default.toml with drift guard `npm run hotkeys:check` (not run) |

## 7. Dead / duplicated / unreachable code
None found — every component is rendered by the step machine; no unreachable branches beyond the documented `unreachable (accent ids are total)` typed guard (L599).

## 8. Dependencies & flags
react-router-dom (Navigate/useNavigate), ThemeProvider, SettingsProvider, generated bindings/schema/tokens — all eager, no feature gates. No env vars.

## 9. Open questions for the next agent
- Does the desktop shell read `lumen.settings.overrides.v1` from localStorage on boot and push it into the engine? If not, F-001 is S1-adjacent. Check apps/desktop + config-settings.ts (W15-A01).
- Does any engine-side component read `lumen.onboarding.v1`? (Expect NO.)
