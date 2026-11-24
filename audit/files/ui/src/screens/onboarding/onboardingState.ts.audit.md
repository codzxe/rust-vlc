# AUDIT: ui/src/screens/onboarding/onboardingState.ts
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 156 / 8 (2 persistences, hydrate/commit, store hook, 3 actions) |
| File verdict | REAL state machine, PAPER-ONLY backend: privacy choices persist to webview localStorage ONLY — the engine config path (config.set-setting) exists in the codebase but onboarding never uses it (header admits the handoff is "recorded in ui/NEXT-STEPS.md") |
| Reachable from shipped entry point? | YES — OnboardingScreen.tsx:62-63 uses useOnboardingState/useOnboardingActions; App.tsx gates first-run routing on it (App.test.tsx covers the gate). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg consumers (App, OnboardingScreen); sed schema.ts:685-704 (privacy.telemetry default false, defaultRef telemetry.enabled); rg SettingsProvider default persistence (SettingsProvider.tsx:50 localStorageSettingsPersistence); baseline ui-test.txt:36 OnboardingScreen 28 tests PASS |

## 1. One-paragraph verdict
A correct, privacy-honest little store: opt-in-only flags defaulting false, corrupt/absent storage folds to OFF (never "unknown → on"), useSyncExternalStore wiring is proper, and finishing never mutates choices. But its backend story is a placeholder: `privacy.telemetry`/`privacy.update-checks` exist in the generated settings schema (schema.ts:687, default false, defaultRef telemetry.enabled) and the desktop settings screen writes them through the REAL engine bridge (config.set-setting via createConfigSettingsPersistence) — while onboarding writes only `localStorage["lumen.onboarding.v1"]`. The engine is never told what the user chose at first run. Because every default is false, the silent drop only bites on OPT-IN: a user who affirmatively enables telemetry during onboarding has that choice discarded (engine keeps false; the privacy settings screen later shows the mismatch). Adjacent: the wizard's own settings steps (theme/accent/playback) commit through a bare `<SettingsProvider>` (OnboardingScreen.tsx:767) whose default persistence is localStorage — so NO onboarding choice reaches the engine even in the desktop lane.

## 2. What actually works (verified)
— verified by read: hydrate() coerces every loaded value through `=== true` — corrupt stores cannot produce a truthy flag (the honest rule holds).
— verified by baseline: OnboardingScreen.test.tsx 28 tests PASS (opt-in defaults, choice persistence, finish-without-consent pinned).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/onboarding/onboardingState.ts:12-14 (with OnboardingScreen.tsx:767, SettingsProvider.tsx:50)
Code:
```
// there would be wiped by the next settings Apply. Handoff for the REAL
// config write path (config.set-setting over the client bridge, doc 05
// §2.11) is recorded in ui/NEXT-STEPS.md.
```
What is wrong: The handoff never happened. The engine write path EXISTS (client/config-settings.ts:138 `cmd: "config.set-setting"`; SettingsScreen.tsx:461 uses it in the desktop shell) but onboarding keeps localStorage — and its settings steps use the provider's default localStorage persistence too (SettingsProvider.tsx:50). First-run privacy choices and first-run settings never reach the engine in ANY lane; an affirmative telemetry opt-in is silently dropped (engine default false, schema.ts:690). The two stores can then disagree: onboarding says on, the privacy settings screen (engine-backed) says off. Honest comments keep this out of S0, but a user's explicit privacy decision not being delivered to the only layer that could act on it is a real wiring gap. What correct looks like: onboarding mounts under the same isDesktopShell()-selected persistence the settings screen uses, and choices map to privacy.telemetry/privacy.update-checks writes. Confidence: HIGH — Verification: ran rg on all three seams; read schema entry.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| localStorageOnboardingPersistence | 51-70 | REAL | — | try/catch both ways; corrupt → null |
| memoryOnboardingPersistence | 72-79 | REAL | — | honest test double |
| hydrate | 88-99 | REAL | — | strict `=== true` folds |
| commit | 101-105 | REAL | — | save + notify |
| subscribe/getSnapshot | 107-112 | REAL | — | |
| useOnboardingState | 115-117 | REAL | — | useSyncExternalStore with server snapshot |
| setOnboardingChoice | 122-125 | REAL | — | no-op on unchanged |
| completeOnboarding | 132-135 | REAL | — | idempotent; never touches choices |
| resetOnboardingForTests | 138-144 | REAL | — | seam swap + rehydrate + notify |
| useOnboardingActions | 147-156 | REAL | — | stable callbacks |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| OnboardingScreen.test.tsx (28, baseline PASS) | YES | pins defaults all-false, choice commits, finish/skip semantics |
| Not covered anywhere | — | the engine-write gap (F-001): no test asserts choices reach config.* — because they don't |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "privacy choices are opt-IN only and default OFF … never … by a corrupt/absent store" | onboardingState.ts:3-6 | TRUE | hydrate L88-99 |
| "Handoff for the REAL config write path … is recorded in ui/NEXT-STEPS.md" | onboardingState.ts:12-14 | TRUE (and still pending) | no config.* call in onboarding tree (rg) |

## 7. Dead / duplicated / unreachable code
None. (Separate storage key is justified: SettingsProvider.apply() rewrites the settings key wholesale — verified rationale.)

## 8. Dependencies & flags
React only; localStorage. No flags.

## 9. Open questions for the next agent
- ThemeProvider.tsx (my next file): which store does theme bootstrap read — localStorage overrides or engine config? If localStorage, a theme changed later via the desktop settings screen (engine) may be shadowed by the onboarding-era localStorage value.
