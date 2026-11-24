# AUDIT: ui/src/screens/settings/SettingsProvider.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 159 / 3 (Provider + defaultsFor + useSettings) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: App.tsx:322 /settings/:area -> SettingsScreen.tsx:486-489 `<SettingsProvider persistence={config ?? localStorageSettingsPersistence} ...>` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg persistence consumers + config-settings wiring (exit 0); baseline ui-test.txt cited |

## 1. One-paragraph verdict
A correct draft/persisted settings store: schema-seeded totals (no invented keys — the loader drops unknown keys, and the persistence seam additionally type-validates known keys), dirty-key computation in schema order, minimal-override persistence (a value equal to the schema default is not stored, so reset truly clears), and an engine-event feed seam (`config.setting-changed` -> persisted + clean drafts, dirty drafts win). The backend answer for the master question: this provider itself is transport-agnostic — the REAL wiring happens one level up (SettingsScreen picks `createConfigSettingsPersistence(client)` in the desktop shell, which sends `config.set-setting`/`config.get-setting` against the CLI host's served arms). One stale-comment finding only.

## 2. What actually works (verified)
— Draft/persisted/dirty/restartPending model: L93-150; apply() persists ONLY keys that differ from schema defaults (L116-120) — verified logic.
— Engine feed: L78-91 `config.setting-changed` updates persisted and non-dirty drafts; unknown keys ignored (L83).
— Desktop wiring (context, verified in SettingsScreen.tsx:451-457 + config-settings.ts:138): Apply round-trips the real engine; browser shell is localStorage with the demo transport honestly refusing config.*.
— Baseline: settings suites pass (ui-test.txt).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/settings/SettingsProvider.tsx:50
Code:
```
  persistence = localStorageSettingsPersistence,
```
What is wrong: not a code bug — a stale-default trap. The DEFAULT persistence is the localStorage seam; any host that mounts `<SettingsProvider>` without passing the desktop config persistence silently gets localStorage-only settings even inside the desktop shell. SettingsScreen.tsx does pass the right seam today (L486-489), but the failure mode of forgetting is silent engine/UI divergence (UI shows a value the engine never received). Why it matters: silent divergence is this audit's core threat; the honest shape is a required-persistence prop or a transport-aware default. Confidence: HIGH — Verification: read + rg (only SettingsScreen mounts the provider in production).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| defaultsFor | 42-46 | REAL | - | schema-seeded |
| SettingsProvider (state init) | 60-72 | REAL | - | lazy load, unknown keys dropped |
| SettingsProvider (engine feed effect) | 78-91 | REAL | - | dirty drafts win; unknown keys ignored |
| SettingsProvider (context value) | 93-150 | REAL | S3 | apply/revert/resetField/resetArea/value all total; F-001 default-persistence trap |
| useSettings | 155-159 | REAL | - | throws outside provider |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| settings provider tests (baseline pass) | YES | draft/apply/revert/reset/dirty/restart and the setting-changed feed are unit-tested |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "persisted = schema defaults + user overrides from the persistence seam" | L10-11 | TRUE | L62-70 |
| "Reset-to-default rewrites draft values from the schema (never invents)" | L14 | TRUE | L134-138 |
| (persistence.ts) "MOCK persistence seam (card 4.E1) ... the REAL write path is the app layer's" | persistence.ts:3-6 | SUPERSEDED (comment stale) | SettingsScreen.tsx:451-457 already wires the real path in the desktop shell; the "MOCK" header undersells shipped reality |

## 7. Dead / duplicated / unreachable code
None. `subscribe` prop is optional and wired by SettingsScreen in production.

## 8. Dependencies & flags
Generated schema entries; persistence seam (localStorage impl + desktop config-settings impl); client Event type. No feature gates.

## 9. Open questions for the next agent
- config-settings.ts (desktop persistence): does save() handle partial failure (one key rejected by engine validation) without losing the rest? Not in my file list — next agent should audit it.
