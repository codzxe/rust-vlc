# AUDIT: ui/src/screens/onboarding/OnboardingScreen.tsx#part2 (lines 483–964)
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 482 (chunk 483–964 of 965) / 7 components + 3 inner handlers |
| File verdict | SHALLOW (chunk-level: the wizard's engine-facing claims; the appearance step and open funnel are REAL) |
| Reachable from shipped entry point? | YES in the UI bundle: App.tsx:11 imports OnboardingEntry/RequireOnboarding; /onboarding route + the /player first-run gate (App.tsx:319 area) mount this screen on every fresh profile. NO shipped binary mounts it (W14: desktop-tauri E0425 — no GUI binary exists at HEAD). |
| Findings | S0:0 S1:0 S2:1 S3:1 S4:1 |
| Commands run | rg census of SETTING_PLAYBACK_* consumers repo-wide (ui/src); sed read of SettingsProvider.tsx:95-135 + :49-51, persistence.ts:1-27, config-settings.ts (full), SettingsScreen.tsx:455-495, settingsThemeSync.ts (full), onboardingState.ts (full), open-path.ts:1-60; binding-id verification against settings/areas/hotkeys/generated/bindings.ts (18/18 exist); sed quote verification; onboarding.css hidden-class check. |

## 1. One-paragraph verdict
Part 2 contains the appearance cards/accents (real: ThemeProvider live preview + localStorage bootstrap — verified end-to-end), the library step (real: `openPathRequest` → `session.open`/`playlist.load` over the client transport, with a correct supersede guard), the screen shell (progress dots, live announcer, footer), and the route gates. The shortcuts tour renders the REAL generated keymap (all 18 binding ids verified present in bindings.ts). The rot: `<SettingsProvider>` at :767 is mounted with NO `persistence` prop, so the wizard hard-wires the localStorage MOCK even on the desktop shell where the real `config.set-setting` bridge exists and SettingsScreen uses it — the playback/subtitles picks write a store whose only consumers are the settings screens' own display (and on desktop, Settings hydrates from the engine, so the picks are invisible there too). The module's honesty charter ("every control on every step does something real") is therefore false for those steps: first-run controls that toggle, "persist", and change nothing. Privacy switches are the same disease in the safe (opt-in-off) direction.

## 2. What actually works (verified)
- Appearance step end-to-end: `settings.setValue(THEME_SETTING_KEY/ACCENT_SETTING_KEY, …)` (:534, :615, :580) → AutoApplySettings effect (:341-347, part1) → `persistence.save(overrides)` (SettingsProvider.tsx:114-126) → localStorage `lumen.settings.overrides.v1` (persistence.ts:16) → `readPersistedThemeMode()/readPersistedAccent()` bootstrap every route (settingsThemeSync.ts:72-92). The "pick survives reload" claim (g02 P1-2) is REAL and test-covered (OnboardingScreen.test.tsx:312-345, "a fresh load hydrates the wizard's persisted appearance picks").
- Accent roving tabindex: arrows move selection AND focus with wrap-around (:570-586); radio semantics single tab stop; verified by test (:290-307, asserted aria-checked/tabindex/focus).
- Library step funnel: `client.send(openPathRequest(source, requestId))` (:656-657) → real `session.open`/`playlist.load` commands (open-path.ts:50-60+, generated/commands.ts:24); one-in-flight guard (:645), supersede-by-request-id guard (:661, :665) — the stale-response bug it cites (j16 P3-9) is actually fixed; failure surfaced honestly via aria-live notice (:699-702).
- Shortcuts tour: `binding(id)` lookups against HOTKEY_BINDINGS — all 18 ids in SHORTCUT_ROWS + the library-step hints verified present in generated/bindings.ts (sed grep, 18/18); missing ids render nothing rather than fake caps (:871).
- Column math: `SHORTCUT_COLUMN_COUNT = ceil(12/6) = 2`; a 13th row would add a column (comment's claim holds).
- Route gates: OnboardingEntry redirects completed users (:952-956); RequireOnboarding gates /player (used at App.tsx:11) — both read the same useSyncExternalStore (onboardingState.ts:115-117).
- Baseline: vitest 1705 passed (INDEX.md row 23) includes OnboardingScreen.test.tsx (~40 cases, behavioral: clamping, separator-typing, roving, persistence).

## 3. Findings
### F-001 [S2]
Where: ui/src/screens/onboarding/OnboardingScreen.tsx:762-768 (prop omission at :767)
Code:
```
       * (the keys ThemeProvider bootstraps from on every fresh load, so the
       * first-run answer survives reload — g02 P1-2). */}
      <SettingsProvider>
        <AutoApplySettings>
```
What is wrong: `<SettingsProvider>` passes no `persistence` prop, so it always uses the default `localStorageSettingsPersistence` (SettingsProvider.tsx:49-51) — including on the desktop shell, where the REAL write path exists and is used by the sibling Settings screen (SettingsScreen.tsx:460-461: `isDesktopShell() ? createConfigSettingsPersistence(client, SETTINGS_ENTRIES)`; config-settings.ts:5-7: "This replaces the localStorage MOCK (persistence.ts) in the desktop shell"). The wizard's playback/subtitles picks (`playback.start-paused`, `playback.default-rate`, `playback.subtitle-mode`, `playback.preferred-subtitle-languages`) therefore never reach the engine, and no UI runtime consumes them either: rg over ui/src shows their only consumers are OnboardingScreen (write), the settings screens (display), and tests — settingsThemeSync.ts consumes only `interface.theme` + `global.accent-color` (:49-50). On desktop the Settings screen hydrates from the ENGINE (config.get-setting), not localStorage, so the onboarding picks are invisible even there: a user who toggles "Start paused" in the first-run wizard then opens Settings sees the default (off).
Why it matters: This is the first-run experience of the product; the module's own honesty charter (lines :16-17, part1: "every control on every step does something real") is false for two of the wizard's steps — enabled controls that persist into a void. Same disease family as W8's `audio.set-eq` and W11's never-persisted settings, now at the onboarding seam. The test suite locks in the wrong behavior ("playback step writes the real settings overrides store", OnboardingScreen.test.tsx:357 — `readSettings()` reads localStorage).
What correct looks like: Mirror SettingsScreen: probe `isDesktopShell()` and pass `createConfigSettingsPersistence(client, SETTINGS_ENTRIES)` (with its awaited hydrate gate), falling back to localStorage only in the browser/demo shell — or render the playback/subtitles controls disabled with an honest reason until the engine path is wired.
Confidence: HIGH — Verification: read (all five files cited above, line-verified); reachability of consumers by rg.

### F-002 [S3]
Where: ui/src/screens/onboarding/OnboardingScreen.tsx:845-850 (store at onboardingState.ts:49)
Code:
```
                        <Switch
                          checked={choices[row.key]}
                          onChange={(value) => setChoice(row.key, value)}
                          aria-label={msg(row.labelKey)}
                          data-testid={`onboarding-choice-${row.key}`}
                        />
```
What is wrong: The three privacy switches persist only to the separate `lumen.onboarding.v1` localStorage key (onboardingState.ts:49); the comment "Maps to schema key `privacy.telemetry`" (onboardingState.ts:20) is aspirational — nothing maps it, no engine write happens (onboardingState.ts:12-14 discloses the handoff as open), and no telemetry/update-check/suggestion feature exists anywhere to be gated (rg `telemetry` in ui/src → settings schema display only).
Why it matters: Dead controls in the first-run flow — opting in enables a feature that does not exist. Safe direction (opt-in, default off, opting in enables nothing rather than opting out disabling nothing), and the seam file is honest about the gap, which keeps this at S3 rather than S2.
What correct looks like: Either write the choices through the settings schema/engine seam when it lands, or annotate the switches as staged (the pattern the EQ tool uses: disabled with reason).
Confidence: HIGH — Verification: read + rg.

### F-003 [S4]
Where: ui/src/screens/onboarding/OnboardingScreen.tsx:605-617 (esp. :610-611)
Code:
```
          role="radio"
          aria-checked={theme ? accent === id : undefined}
```
What is wrong: On hosts without a ThemeProvider, every accent button renders `role="radio"` with `aria-checked` omitted — ARIA requires a checked state on the radio role; an unchecked-or-unknown radio is a spec violation, and the whole group also loses its single tab stop (`tabIndex={theme !== null && accent === id ? 0 : -1}` makes ALL buttons -1).
Why it matters: Only reachable in harness/test mounts (the app always wraps ThemeProvider — main.tsx:60), so blast radius is the dev/visual harness's a11y reports; still an invalid ARIA tree the axe lane would flag.
What correct looks like: On inert hosts render `aria-checked={accent === id}` regardless (the state exists — it just has no visual effect), or drop the radio role when disabled.
Confidence: HIGH — Verification: read; ThemeProvider presence at main.tsx:60.

## 4. Per-function table (chunk 483–964)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SubtitleControls (render tail) | 483-496 | REAL | — | SegmentedControl options from generated schema enumeration; TextField draft-on-blur commit (separator fix verified by test) |
| AppearanceCards | 502-543 | REAL | — | aria-pressed toggle buttons; `disabled={!theme}` inert hosts; live preview via useSettingsThemeSync (:518) |
| AppearanceAccents | 553-621 | REAL | — | Radiogroup + roving tabindex; F-003 ARIA gap on inert hosts only |
| └ onAccentKeyDown | 570-586 | REAL | — | Wrap-around math correct for indexOf ≥ 0; preventDefault before inert-return (key consumed, no effect — documented); typed `next === undefined` guard defensive |
| LibraryStep | 633-725 | REAL | — | Real open funnel; F-001-adjacent none; see §2 |
| └ openMediaPath | 641-668 | REAL | — | One-in-flight + supersede guards correct; catch → honest "failed" notice (aria-live) |
| OnboardingScreen | 731-948 | SHALLOW | S2 | F-001: the SettingsProvider mount hard-wires the mock persistence; step machine (next/back clamps :740-741), progress dots (aria-current), live announcer (:803-805), footer (Back visibility:hidden keeps geometry — onboarding.css:643), Skip/Get-started share one handler (defaults-unchanged rule holds: handler never touches choices) — all correct |
| OnboardingEntry | 952-956 | REAL | — | Redirect-on-complete; gate honest |
| RequireOnboarding | 960-964 | REAL | — | /player gate; consumed at App.tsx:11 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "appearance choices persist through the settings overrides store" (test :312-336) | YES for the localStorage path | Asserts store contents after clicks (real behavior) |
| "a fresh load hydrates the wizard's persisted appearance picks" (:338-353) | YES | Seeds localStorage, renders ThemeProvider, asserts dataset.theme/accent — the actual reload contract |
| "playback step writes the real settings overrides store" (:357-362) | YES for localStorage, NO for the defect that matters | Asserts `readSettings()` (localStorage) — would still pass if the engine path were wired or unwired; F-001 is invisible to it. Test NAME says "real settings" while asserting the MOCK store (persistence.ts:2 calls itself a MOCK) |
| default-rate clamp suite (:404-420) | YES | Behavioral: 9→4 clamp, empty-commit suppression, blur snap-back |
| accent roving suite (:283-307) | YES | Asserts aria-checked/tabindex/focus per keypress |
| Skip/defaults tests (per :742-746 comment) | YES | Per-button assertions that choices stay untouched |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every control on every step does something real" | OnboardingScreen.tsx:16-17 (part1 header, chunk cross-ref) | FALSE for playback/subtitles steps | F-001 — store write only, zero consumers |
| "the appearance step … persists its picks through the same SettingsProvider seam" | :17-20 | REAL | settingsThemeSync bootstrap; tests :312-353 |
| "the media step opens through the ONE open-path funnel" | :22-23 | REAL | openPathRequest → session.open/playlist.load (open-path.ts:50+); LibraryStep :656 |
| "the shortcut tour's keycaps are the REAL keymap … data, never hand-typed copy" | :23-25 | REAL | 18/18 binding ids verified in generated/bindings.ts |
| "Skip completes the wizard with the defaults UNCHANGED" | :14-15 | REAL | completeAndExit (:747-750) never touches choices; onboardingState.completeOnboarding preserves state |
| progress label "one localizable string" (j16 P3-14) | :386-387 (part1), used :752-756 | REAL | template.replace; visually-hidden dot numerals are number-suffixed labels, acceptable |

## 7. Dead / duplicated / unreachable code
- The playback/subtitles settings writes are effectively dead-in-effect (F-001) — not dead code, dead DATA: values with no reader on the desktop host.
- Nothing else: all subcomponents are mounted by the step machine; bindings/keys all resolve.

## 8. Dependencies & flags
React 19 (useSyncExternalStore, Fragment), react-router (Navigate/useNavigate), generated schema keys (settings/generated/schema.ts), generated hotkey bindings, client/open-path, theme/settingsThemeSync, onboardingState (localStorage `lumen.onboarding.v1`), SettingsProvider (default localStorage persistence — F-001). No feature flags. i18n: every string via useOnboardingMsg (verified — no literals in chunk).

## 9. Open questions for the next agent
- Part1 (W15-A04) owns the header honesty rules (:13-27) and PlaybackControls/SubtitleControls bodies — my F-001 lives at the :767 mount in THIS chunk; cross-count carefully (one finding, cite whichever chunk you audit from).
- Whoever audits SettingsScreen.tsx / config-settings.ts / settingsThemeSync.ts: the MIRROR defect — desktop Settings writes engine-only, never localStorage, but ThemeProvider bootstraps from localStorage (settingsThemeSync.ts:72-92), so desktop settings-screen theme picks may not survive reload. That is outside my chunk's files but adjacent and real; rg-verified seam, not asserted here as a filed finding.
- OnboardingScreen.test.tsx is assigned to no one in w15 — its "real settings" naming drift is noted in §5 for the wave summary.
