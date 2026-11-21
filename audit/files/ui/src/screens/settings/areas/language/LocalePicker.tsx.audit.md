# AUDIT: ui/src/screens/settings/areas/language/LocalePicker.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 52 / 2 (usePickerMsg factory const, LocalePicker) |
| File verdict | SHALLOW (honest single-option control wired to a dead end) |
| Reachable from shipped entry point? | YES (web build): specialWidgets.ts:60 maps `"global.language"` → LocalePicker (special widget registry consumed by the settings areas); test mounts under SettingsProvider. Baseline #22 build OK, #23 vitest aggregate green. No GUI binary (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `rg -ln "LocalePicker" src` (specialWidgets + test only); `rg -n "global.language" src --glob '!**/language/*'` (consumers: schema def, specialWidgets mount, generated.test, i18n message keys — NO runtime reader of the setting value); `rg -n "global.language" ../crates -l` (only crates/localization/src/fallback.rs, the orphaned crate); `rg -ln "lumen_localization" crates apps ui` (ZERO importers — orphan confirmed); `ls ../locales` (exactly one catalog: en-US.toml); read locales.ts, schema.ts:66-88 (SETTING_GLOBAL_LANGUAGE: enum, default en-US, restartRequired: true, enumeration=[en-US] only), SettingsProvider.tsx (draft/persist/restartPending machinery), persistence.ts (localStorage seam + enum-membership load guard), LocalePicker.test.tsx (full), fallback.rs (select_locale — setting → OS → default chain, uncalled); i18n key existence checks (settings.global.interface-language, settings.locale.en-us both exist). |

## 1. One-paragraph verdict
The "Interface language" row is a competently built dropdown that cannot change the interface language and is not read by anything that could. The intersection logic is exactly as documented (schema enumeration ∩ shipped catalogs = en-US, one option), the Dropdown is the real a11y combobox, and the value binds to the settings draft with a restart badge — but every layer downstream of the click is dead: the UI's i18n is a compile-time en-US string table with no runtime locale selection, the only code in the repository that maps `global.language` to a locale (the localization crate's `select_locale`) is orphaned with zero dependents, the persistence seam is a dev localStorage round-trip with no language consumer, and the restart the badge instructs would be performed by a host binary that cannot compile (W14). The component's own honesty is genuine (it never invents locales) — the surrounding claim "Interface language" is what's hollow. The binding test is a tautology in this single-locale world: delete the `onChange` wiring and it stays green.

## 2. What actually works (verified)
— Verified by `ls ../locales` + locales.ts read: exactly one catalog (en-US.toml); AVAILABLE_LOCALES=["en-US"] is the true state of the repo, and the picker renders exactly that (test: "offers exactly the real available locales" asserts length === 1).
— Verified by read of persistence.ts (isValidValue enum-membership guard) + SettingsProvider: a tampered non-en-US stored value is dropped on load, so `current` is always a valid enum member — the `String(value(...))` at :37 cannot produce "undefined" (draft is seeded with every schema key, SettingsProvider:95-97).
— Verified by rg: the intersection filter (:34-35) cannot offer a locale without a catalog (AVAILABLE_LOCALES gate) nor bind one the schema doesn't enumerate — the "never invented" claim holds structurally.
— Verified by key checks: `settings.global.interface-language` (w26.gfixB.ts:32, messages.ts:2732) and `settings.locale.en-us` (messages.ts:1785) both exist — the two msgid lookups this file performs resolve.
— Verified by baseline #23 aggregate: LocalePicker.test.tsx's 4 tests are in the green run (focus restoration, closed-trigger anatomy, option count are real assertions).

## 3. Findings

### F-001 [S2]
Where: ui/src/screens/settings/areas/language/LocalePicker.tsx:44-46
Code:
```tsx
        value={current}
        onChange={(next) => setValue(entry.key, next)}
```
What is wrong: the control's full chain dead-ends at every layer after the click. (a) One option: schema enumeration = [{en-US}] (schema.ts:76) ∩ AVAILABLE_LOCALES = ["en-US"] — a user opening "Interface language" can select only the language already active. (b) No reader: `rg "global.language"` over ui/src finds no runtime consumer of the value — ui/src/i18n/messages.ts is a hardcoded en-US table, no provider switches catalogs. (c) Orphaned engine-side: the only code mapping the setting to a locale is `select_locale` in crates/localization/src/fallback.rs:26 — `rg -ln "lumen_localization"` over crates/apps/ui returns ZERO importers. (d) The schema flags restartRequired: true (schema.ts:72), so an actual change would raise the "restart required" badge — a restart instruction for an effect no buildable binary implements (W14-A01; the tauri host is the would-be applier). (e) Persistence is the dev localStorage seam (persistence.ts header "MOCK persistence seam… the REAL write path is the app layer's config.set-setting") — which only the unbuildable shell provides (W11/W14).
Why it matters: this is the "Interface language" promise in the shipped settings screen — same write-only-settings family as my playback.tsx F-001 (S2) and W11's localization-orphaned finding; a user who understands the badge is told the product has a language subsystem it does not have.
What correct looks like: either a second catalog + a runtime locale switch in the i18n layer reading global.language, or the row rendered as an honest static "Language: English" (no picker affordance, no restart badge) until that exists.
Confidence: HIGH — Verification: all five legs traced by rg/read as listed in Commands run.

### F-002 [S3]
Where: ui/src/screens/settings/areas/language/LocalePicker.test.tsx:66-83 ("binds to the global.language setting key (select → draft value + dirty)")
Code:
```tsx
    expect(latest).toBe(SETTING_GLOBAL_LANGUAGE.defaultValue); // en-US
    // Single-locale world: the option is already selected; the binding is
    // proven by the trigger carrying the schema key's current value and the
    // selected option staying bound after re-selecting it.
```
What is wrong: the test that names itself "binds to the global.language setting key" asserts `latest === "en-US"` BEFORE the click and `"en-US"` AFTER the click — the same value both times. Mutation check (reasoned, not run): delete `onChange={(next) => setValue(entry.key, next)}` from LocalePicker.tsx entirely — every assertion still passes (trigger text comes from value→label resolution, which doesn't touch setValue; focus restoration is Dropdown's own close behavior). The binding is not proven; the comment admits it ("Single-locale world") but the test name and suite role (the only "binds" test) promise more than the body delivers.
Why it matters: this is the single test standing between the picker and a silent un-wiring; a future refactor that drops the onChange prop ships green. The checklist bar — would it FAIL if the code were broken? — is answered NO for the file's one essential wiring.
What correct looks like: assert a dirty-state change (e.g. probe `dirtyKeys` via a second hook consumer) after selecting a DIFFERENT option — impossible until a second schema-legal locale exists, which is itself the finding: until then, rename/mark the test as pinning trigger/focus behavior only, not "binding".
Confidence: HIGH — Verification: read of both files; mutation reasoned through the exact assertion set.
Note: finding recorded against this file's audit because the test file is this file's companion and not separately assigned.

### F-003 [S4]
Where: ui/src/screens/settings/areas/language/LocalePicker.tsx:42
Code:
```tsx
        label={m(entry.description as MessageKey)}
```
What is wrong: another instance of the tree-wide `as MessageKey` cast pattern (ToolsDialog F-003, MusicMode, my StateRouter F-002): `entry.description` is a schema string, cast blind into the i18n key union. Today it resolves (`settings.global.interface-language` exists, verified) — but a schema regeneration that renames the msgid compiles green and renders a raw key at runtime.
Why it matters: the schema→i18n seam is type-checked nowhere; this file's label depends on a convention, not a compiler guarantee.
What correct looks like: schema msgid fields typed as MessageKey at the generated-schema source (single cast point), not per-consumer.
Confidence: HIGH — Verification: read + key-existence rg (messages.ts:2732, w26.gfixB.ts:32).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `usePickerMsg` (makeAreaMsg(fixerKeys)) | 26 | REAL | — | Standard shared area-msg factory (j13 dedup); keys verified present. |
| `LocalePicker({entry})` | 28-52 | SHALLOW | F-001, F-003 | Intersection filter correct + structurally honest; String(value()) total (draft pre-seeded); no ignored params. The `labelKey: o.value` trick (option label key = locale tag) is resolved through localeLabel's msgid map — raw-tag fallback for unknown locales (test-pinned "fr-FR" → "fr-FR"). |

## 5. Test honesty (LocalePicker.test.tsx, green in baseline #23 aggregate)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "renders a closed dropdown trigger showing the current locale" | YES | Combobox anatomy + closed state + "English" text — real Dropdown behavior pinned. |
| "offers exactly the real available locales when opened" | YES | Option count === AVAILABLE_LOCALES.length === 1 — pins the intersection against schema drift (a schema enum growth without a catalog changes the count). |
| "binds to the global.language setting key" | NO | F-002 — tautology in the single-locale world; survives deletion of the onChange wiring. |
| "labels the reference locale through the i18n catalog" | YES | localeLabel map + raw-tag fallback both pinned directly. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Options stay the INTERSECTION of the schema enumeration and the localization crate's real available locales (locales/ dir) — real locales only, never invented" (:18-22) | this file | REAL | Filter at :34-35; locales/ has exactly one catalog; test pins count 1. |
| "the value binds to the settings draft (restart badge via the framework, the entry is schema-flagged restart_required)" (:20-22) | this file | SHALLOW | The binding and badge machinery exist (SettingsProvider:124), but the restart applies nothing — F-001 legs (b)-(e). |
| "today that directory contains exactly one catalog — the reference locale en-US" (locales.ts:6-8) | locales.ts | REAL | `ls locales` → en-US.toml only. |
| locales/ "the crate's source of truth" (locales.ts:14) | locales.ts | FALSE (as wiring) | The crate that scans locales/ is orphaned — zero dependents; nothing at runtime reads that directory on any shipped path. |
| "Interface language" (the row label itself) | w26.gfixB.ts:32 / schema | FALSE (as capability) | No language can be picked (one option) and no layer would apply it — F-001. |

## 7. Dead / duplicated / unreachable code
- Not dead in the UI sense (mounted via specialWidgets when the global/appearance area renders); dead in the EFFECT sense (F-001).
- The `labelKey: o.value` + `resolveLabel` two-step is mildly redundant (Dropdown's resolveLabel already receives the tag) but is the seam localeLabel needs — not a finding.
- Cross-ref: W11's "localization crate orphaned, UI hardcodes en-US" — this file is that finding's UI endpoint; my playback.tsx F-001 records the sibling pattern (write-only schema settings).

## 8. Dependencies & flags
- Dropdown (audited this wave: REAL, a11y-solid), i18n keys w26.gfixB (verified), schema entry (generated), SettingsProvider (framework, real), locales.ts (real data). No flags. No engine dependency at all — which is the problem (F-001).

## 9. Open questions for the next agent
- If a second locale pack ever lands: AVAILABLE_LOCALES is a hand-maintained literal (locales.ts:15) while the schema enumeration is generated — the intersection silently hides a schema locale without a catalog row here. The file's own comment promises "grow this list from the same directory scan… a generated list is the follow-up" — that follow-up is the debt.
- The i18n lane owner should state whether ANY runtime locale-selection design exists beyond the orphaned crate (I found none in ui/src/i18n — messages.ts, keys/, pseudo.ts only).
