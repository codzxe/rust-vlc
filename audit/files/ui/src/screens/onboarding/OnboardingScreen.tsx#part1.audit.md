# AUDIT: ui/src/screens/onboarding/OnboardingScreen.tsx#part1
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 1-482 of 964 (chunk part1; part2 = 483-964 is W15-A05's) / 16 items |
| File verdict | REAL (chunk) — with one S2 dead-end wiring defect on the settings steps |
| Reachable from shipped entry point? | YES as UI code — /onboarding route, and the first-run gate (App.tsx RequireOnboarding) redirects EVERY route there until complete; this is the first screen a user sees. UI build + tests green (baseline #22/#23). Desktop host binary itself cannot compile at HEAD (W14-A01 F-001) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `rg -n "export function useMsg" -A 6 ui/src/i18n/messages.ts` (:3011-3016 — unknown key → undefined, so the lane `??` fallback is sound) · `rg -n "SettingsProvider" OnboardingScreen.tsx` (:767 bare mount, default localStorage persistence) · Read SettingsProvider.tsx (159 lines — apply() writes ONLY the persistence seam :108-125) · `rg -ln "createConfigSettingsPersistence" ui/src --glob '!*.test.*'` (ONLY SettingsScreen.tsx:461 — the engine bridge is never used by onboarding) · `rg -n "SETTING_PLAYBACK_DEFAULT_RATE|SETTING_PLAYBACK_START_PAUSED|SETTING_PLAYBACK_SUBTITLE_MODE" ui/src --glob '!*.test.*'` (no consumer outside OnboardingScreen + generated display screens) · `rg -n "privacy.telemetry|privacy.update-checks" schemas/settings.toml` (telemetry exists ui=false :469-475; privacy.update-checks does NOT exist — schema has global.update-checks :129) · `rg -n "choices\." ui/src --glob '!*.test.*'` (no reader of privacy choices outside onboardingState.ts) · per-id `rg -c "id: \"$id\"" bindings.ts` — all 16 SHORTCUT_ROWS binding ids exist (1 hit each) · `rg -n "describe|it\(" OnboardingScreen.test.tsx` (25 tests) · baseline: ui-vitest.log.txt (1705 passed), W11-A02 log (engine config = dead in-memory BTreeMap, S2), W14-A01 app.rs audit (no GUI binary) |

## 1. One-paragraph verdict
Part1 is the onboarding wizard's data layer and the settings-backed step controls, and most of it is genuinely disciplined: the privacy switches are opt-in-only with all-OFF defaults that survive corrupt stores, the shortcut tour renders the REAL generated keymap (all 16 binding ids verified present), the rate/languages fields carry documented draft-state fixes for the controlled-input bugs that plagued them, and the whole flow is pinned by 25 tests green in baseline. The defect that matters: the playback/subtitles steps write `playback.start-paused`, `playback.default-rate`, `playback.subtitle-mode` into a SettingsProvider mounted with the DEFAULT localStorage persistence — a store whose only consumers are the Settings screen's display fields. No playback path reads these keys, the wizard never uses the engine-backed persistence the Settings screen uses (`createConfigSettingsPersistence` is imported by exactly one file, SettingsScreen.tsx:461), and even that bridge dead-ends in the engine's non-persisted in-memory config map (W11-A02, S2). The first screen every user sees asks them to configure playback defaults that cannot affect playback. Secondarily, the privacy choices persist to a parallel store nothing ever reads, and the in-code mapping comments are half false (`privacy.update-checks` does not exist in the schema).

## 2. What actually works (verified)
— Shortcut tour is real data: all 16 binding ids in SHORTCUT_ROWS (:259-287) exist in generated HOTKEY_BINDINGS (per-id rg = 1 hit each); keycap rendering derives from `binding.key.split("+")` (:238-239), never hand-typed copy.
— The lane-i18n fallback is sound: useMsg returns `catalog[key] ?? messages[key]` (messages.ts:3016), i.e. `undefined` for unregistered keys, so `m(key as MessageKey) ?? laneCatalog[key]` (:93) resolves lane copy while unregistered; all onboarding lane keys checked ARE registered (messages.ts:485, :677, :1297, :490-493) — currently a no-op seam.
— Draft-input discipline on the rate field: local draft state + clamped commits + onBlur snap (:407-435), with the w26.j16 bug history documented in-comment (:398-406) and pinned by tests (:373 "keeps separators", :405 "commits clamp inside the schema range 0.25–4, never writes out-of-schema") — green in baseline ui-vitest.log.txt.
— Privacy opt-in honesty: ONBOARDING_DEFAULTS all-false (onboardingState.ts:39-42), hydrate folds corrupt/absent stores to OFF (:88-99), skip never mutates choices (:132-135) — tests :74-158 cover fresh-run OFF, skip semantics, opt-in round-trip, corrupt store.
— All 25 OnboardingScreen tests pass in baseline #23 (ui-vitest.log.txt, 1705 passed / 1 skipped).

## 3. Findings

### F-001 [S2]
Where: ui/src/screens/onboarding/OnboardingScreen.tsx:341-347 (AutoApplySettings), :392-440 (PlaybackControls), :445-482 (SubtitleControls); mount :767 (part2 range — cited as the screen's own root)
Code:
```
function AutoApplySettings({ children }: { children: ReactNode }): ReactElement {
  const settings = useSettings();
  useEffect(() => {
    if (settings.dirtyCount > 0) settings.apply();
  }, [settings]);
  return <>{children}</>;
}
```
```
          onChange={(checked) => settings.setValue(SETTING_PLAYBACK_START_PAUSED.key, checked)}
```
What is wrong: the playback/subtitles steps auto-apply settings into a SettingsProvider whose `apply()` (SettingsProvider.tsx:108-125) writes ONLY the persistence seam — and the wizard mounts it bare (`<SettingsProvider>` :767), i.e. the DEFAULT `localStorageSettingsPersistence`. Repo-wide rg shows no consumer of `playback.start-paused` / `playback.default-rate` / `playback.subtitle-mode` outside OnboardingScreen itself and the generated Settings screens that merely display them. The engine-backed persistence (`createConfigSettingsPersistence`, config.set-setting over the bridge) is imported by exactly one file — SettingsScreen.tsx:461 — never by onboarding; and even that path terminates in the engine's dead in-memory config BTreeMap with zero persistence (W11-A02 F-001, S2).
Why it matters: this is the FIRST screen every user sees (the first-run gate redirects all routes to /onboarding). The wizard asks "Start playing as soon as a file opens", default rate, subtitle mode — every choice is stored to a localStorage key that no playback code reads. The user-visible contract (onboarding configures the player) is broken end to end; the values only resurface as Settings-screen display state.
What correct looks like: either the onboarding provider uses the desktop config bridge (as SettingsScreen does) with a hydration gate, or the step copy honestly scopes the fields ("appearance only; playback settings are not yet wired to the engine") — and the engine side needs a real settings surface (W11) for either to matter.
Confidence: HIGH — Verification: read of OnboardingScreen/SettingsProvider/config-settings + repo-wide rg for consumers; engine-side dead-end cited from W11-A02 audit (baseline-tested engine code).

### F-002 [S3]
Where: ui/src/screens/onboarding/OnboardingScreen.tsx:139-159 (PRIVACY_ROWS); mapping claims at onboardingState.ts:20-29; store onboardingState.ts:49/122-125
Code:
```
const PRIVACY_ROWS: ReadonlyArray<{
  key: OnboardingChoiceKey;
  labelKey: MessageKey;
  descriptionKey: MessageKey;
}> = [
  {
    key: "telemetry",
    labelKey: "onboarding.privacy.telemetry.label",
```
```
  /** Maps to schema key `privacy.telemetry` (doc 07 §5.4). Default false. */
  readonly telemetry: boolean;
  /** Maps to schema key `privacy.update-checks`. Default false. */
  readonly updateChecks: boolean;
```
What is wrong: the three privacy switches persist to `lumen.onboarding.v1` (onboardingState.ts:49, :122-125) — a parallel store no code outside the wizard reads (rg: `.choices` consumers are all in onboardingState.ts). The choices are never transferred into the settings overrides store, so the "maps to schema key" comments are aspirational: `privacy.telemetry` exists but is ui=false (settings.toml:469-475) and is never written by the wizard; `privacy.update-checks` DOES NOT EXIST in the schema (the real key is `global.update-checks`, settings.toml:129); `onlineSuggestions` is admitted [PROPOSAL].
Why it matters: the user's privacy decisions are write-only data — no telemetry system, updater, or settings surface ever reads them (none exists). The opt-in/OFF default is honest, but a user who opts IN has consented to a system that cannot act on the consent, and a maintainer following the mapping comments will look for schema keys that aren't there.
What correct looks like: the wizard's finish handler writes the choices through the (engine) settings seam, or the comments state plainly "no consumer yet — persisted for the future telemetry lane".
Confidence: HIGH — Verification: read + rg over schemas/settings.toml, ui/src.

### F-003 [S4]
Where: ui/src/screens/onboarding/OnboardingScreen.tsx:93
Code:
```
    (key: MessageKey | LaneKey): string => m(key as MessageKey) ?? laneCatalog[key],
```
What is wrong: the `key as MessageKey` cast (same seam family as the generated settings screens) disables compile-time verification that lane keys are catalog keys. It currently works (useMsg returns undefined for unknown keys, messages.ts:3016, and all onboarding lane keys are registered — verified), so the fallback is live only in the unregistered window the comments describe.
Why it matters: a NEW lane key that never gets registered type-checks fine and silently renders lane copy — the drift gate is only the vitest/l10n checks, not tsc.
What correct looks like: once keys are registered (they are), type the resolver over the union and drop the cast, or keep the seam but add a test asserting every `LaneKey` is either registered or in laneCatalog (that test effectively exists via generated.test-style coverage only for settings, not onboarding).
Confidence: HIGH — Verification: read + messages.ts:3011-3016.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| laneCatalog / useOnboardingMsg | 67-96 | REAL | S4 | 3 lane key modules merged; fallback sound (see F-003) |
| STEPS / STEP_TITLES / STEP_BODIES / CENTERED_STEPS | 102-133 | REAL | — | 8 steps; all title/body msgids registered (spot-verified :485/:677/:1297) |
| PRIVACY_ROWS | 139-159 | REAL | S3 | F-002 — keys reference write-only choice store |
| THEME_CARDS / ACCENT_LABEL_KEYS / ACCENT_ORDER | 165-200 | REAL | — | 3 theme cards, 9 accents in frame order; label keys registered |
| isThemeModeDraft / isAccentDraft | 204-208 | REAL | — | correct unknown-value guards (never guess) |
| BINDING_BY_ID / binding | 215-219 | REAL | — | Map over generated bindings; all ids verified present |
| KEYCAP_GLYPHS / keycapParts / isSymbolKeycap / keycapClass | 228-252 | REAL | — | arrow/SHIFT glyphs are documented cap artwork; split on "+" |
| SHORTCUT_ROWS / SHORTCUT_COLUMN_LENGTH / COUNT | 259-294 | REAL | — | 12 rows; column count derives from census (13th row adds a column) |
| ConeMark | 304-328 | REAL | — | static frame-measured SVG, aria-hidden/focusable=false |
| AutoApplySettings | 341-347 | REAL | S2 | F-001 — applies into the localStorage-only provider |
| SettingsRow | 351-368 | REAL | — | label+control row; htmlFor wiring correct |
| clampToSchema | 372-373 | REAL | — | correct min/max clamp with ±Infinity defaults |
| parseLanguageList | 377-381 | REAL | — | split/trim/drop-empties; correct |
| formatStepLabel | 386-387 | REAL | — | template replace; single-occurrence tokens, fine |
| PlaybackControls | 392-440 | REAL | S2 | F-001 — writes start-paused/default-rate; draft-state input discipline correct |
| SubtitleControls | 445-482 (continues in part2) | REAL | S2 | F-001 — writes subtitle-mode/languages; enumeration from schema data |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :66-158 privacy suite (fresh OFF, skip semantics, opt-in round-trip, corrupt store) | YES | asserts switch states, store contents after skip/opt-in, re-hydration |
| :192-243 library step (open through the one funnel, empty-path rejection) | YES | asserts session.open request emitted via the funnel |
| :244-356 appearance (live preview through ThemeProvider, persistence through overrides store, hydration) | YES | theme keys DO have a consumer (ThemeProvider bootstraps from the store) — this step is genuinely wired |
| :357-421 settings steps ("writes the real settings overrides store", clamp, separators) | YES for what it asserts — but it asserts ONLY the store write, not any playback effect; it cannot catch F-001 (no consumer exists to observe) |
| :434-455 shortcuts tour (real keymap + full-list note) | YES | would fail if a binding id vanished (row renders empty) |
| :457-478 first-run gate redirects | YES | route-level assertions on RequireOnboarding |
All green: baseline ui-vitest.log.txt (1705 passed / 1 skipped).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the three privacy choices are opt-IN switches, all OFF, none pre-checked" | file header :13-15 | REAL | onboardingState.ts:39-42 + tests :74-116 |
| "every control on every step does something real — … the playback/subtitles steps write the REAL settings schema" | file header :16-22 | SHALLOW | they write schema KEYS to a store with no playback consumer (F-001); "does something real" is only locally true |
| "usage-stats opt-in-off persisted by onboarding" | docs/feature-matrix.toml:2084 note | REAL (narrow) | persisted to lumen.onboarding.v1 — but write-only (F-002) |
| "first-run flow live, privacy defaults opt-in-off" | feature-matrix row ui-screen-onboarding :2726-2735 | REAL | flow + gate + tests verified |

## 7. Dead / duplicated / unreachable code
- No dead code in the chunk itself. The playback/subtitles settings values are the dead END (F-001) — the code is live, its data has no reader.
- `KEYCAP_GLYPHS.Shift → "SHIFT"` is intentional artwork, not drift (comment :233-235).

## 8. Dependencies & flags
- React hooks, react-router (Navigate/useNavigate — part2 usage), form components, settings schema constants (generated), theme tokens ACCENTS, onboardingState store, keymap bindings (generated). No feature flags in this chunk. The whole UI cannot ship (W14-A01) but builds+tests green (baseline #22/#23).

## 9. Open questions for the next agent
- W15-A05 (part2): the privacy step RENDER (:483+) and the wizard frame; the `<SettingsProvider>` bare mount :767 is in your range — F-001's mount site; please cross-link.
- Whoever owns ui/src/client/config-settings.ts: verify the bridge against the engine's dead config map (W11-A02 F-001) — the UI side is real, the engine side is the black hole.
- Theme keys (interface.theme/global.accent-color) DO have a real consumer (ThemeProvider) — the appearance step is the one fully-wired settings step; no finding there.
