# AUDIT: ui/src/screens/settings/areas/index.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 310 / 2 components + 5 exported registries/helpers + 1 load-time guard + 2 re-exports |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle: SettingsScreen.tsx:16 imports resolveSettingsAreaSlug, :292 `?? SETTINGS_AREA_IDS[0]`, :306 `SETTINGS_SCREENS[area]`, :116/:371/:393 SETTINGS_AREA_TITLES — /settings/:area routes render this registry's screens. No shipped binary (W14 E0425). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | rg message-key existence (5 area titles + 10 global hints in messages.ts / w26.i13.ts); rg re-exported symbols (SPECIAL_FIELD_WIDGETS, WIDGET_GROUPED_KEYS); SettingsScreen consumer seams read; areas.test.tsx describe/it census. |

## 1. One-paragraph verdict
The 13-area settings registry — routing, titles, screen composition, retired-slug redirects. This is real, total, and guarded: SETTINGS_AREA_IDS is exactly the 13 Figma areas in sidebar order (counted); the load-time MISSING_GENERATED_AREAS guard (:209-216) fails loudly if a seeded schema area lacks a registry row; SETTINGS_AREA_TITLES/SETTINGS_SCREENS are TypeScript-total Records (baseline `tsc --noEmit` passed, INDEX.md row 22 — a missing key would not compile); resolveSettingsAreaSlug is consumed by the shell exactly as the doc comment promises (SettingsScreen.tsx:292); all 5 hand-registered title msgids and all 10 global-hint msgids exist in the i18n catalog (rg-verified). areas.test.tsx pins the registry order, the special-widget-keys-are-real-schema-keys AC, the retired-slug map, and per-area hint wiring — behavioral tests in the baseline vitest pass. Two S4s: a stale wave-era comment describing import errors that "are EXPECTED during the wave" (the siblings landed long ago — the comment describes a state that no longer exists), and a shipped "[Lead: confirm advanced → performance]" open decision on a redirect target.

## 2. What actually works (verified)
- Registry totality: 13 ids (:189-203), spread overrides in SETTINGS_SCREENS cover all 13 (global, playback, interface, privacy, controls, recording, extensions, video, audio, subtitles, library, network, performance); Record typing enforces completeness at compile time.
- Load-time contract guard (:209-216): a schema-seeded area missing from the registry throws at import — "unreachable route" made impossible by construction; this is the fail-loud pattern, not a crash risk (drift only).
- Redirect map (:275-281): 5 retired slugs (hotkeys→controls, codecs→video, about→global, advanced→performance, effects→audio) — each matches the documented merge decisions (:54-67); consumed at SettingsScreen.tsx:292; tested (areas.test.tsx:73 "maps every retired slug to its merged home and off the 13-area registry").
- i18n: "settings.area.video/audio/subtitles" (messages.ts:771/:775/:776/:2253-2254), "library" (:1935), "network" (:1936) — the "registered (messages.ts, b19 merge)" comments are true; all 10 GLOBAL_HINTS msgids exist (w26.i13.ts:16-27, catalog type at messages.ts:1347-1348).
- Re-exports: SPECIAL_FIELD_WIDGETS (specialWidgets.ts:57) and WIDGET_GROUPED_KEYS (SettingWidgets.tsx:122) exist and are re-exported under frozen names — the module-cycle fix (j12 P2-5) holds (SettingField imports from the leaf, not from here).
- GlobalAreaScreen composition: schema sections + AdvancedDisclosure LAST + About behind the disclosure (:141 `{advancedOpen ? <AboutArea /> : null}`) — matches its h08-defect fix documentation; hint wiring tested (areas.test.tsx:135: "6 + the 4 advanced rows").
- Baseline: tsc --noEmit + vite build OK (row 22); areas.test.tsx suites in the 1705-test vitest pass (row 23).

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/settings/areas/index.tsx:24-29
Code:
```
// w26.b02 — sibling area components under the FIXED module names (wave-B
// lanes b07/b08 land them in parallel). If these imports show unresolved-
// module errors, the sibling files have not landed yet — EXPECTED during the
```
What is wrong: The comment instructs the reader that unresolved-module errors (tsc TS2307, vite import-analysis failures "in every suite that mounts SettingsScreen") are EXPECTED — a description of a transient wave-parallel-development state. All four sibling files exist (controls/recording/extensions/library import cleanly; baseline tsc+build+vitest all pass), so the comment describes errors that can no longer occur and will mislead a contributor who sees a REAL import error into dismissing it as the expected transient state.
Why it matters: A comment that pre-authorizes a class of build failure suppresses investigation of that failure when it genuinely happens.
What correct looks like: Delete the transient-state paragraph (or convert to history: "siblings landed w26.b07/b08").
Confidence: HIGH — Verification: sibling files present; baseline row 22/23 clean.
### F-002 [S4]
Where: ui/src/screens/settings/areas/index.tsx:271-273
Code:
```
 * first area. [Lead: confirm `advanced` → `performance`; the Advanced area
 * dissolves into the per-area in-flow disclosure, so `performance`
 * (Decoding/Smooth Playback/Developer) is the nearest home.]
```
What is wrong: A shipped open decision: the `advanced → performance` redirect target was never confirmed by the Lead (bracketed TODO in the shipped registry). Every retired slug has a merge rationale except this one, whose rationale is explicitly provisional ("nearest home").
Why it matters: Deep links to /settings/advanced land on a provisionally-chosen screen; if the Lead ever rules differently (e.g. global, which carries the developer-mode/log rows), the redirect silently diverges from the decision with no test pinning the intent (the test pins the current mapping, not the confirmed one).
What correct looks like: Resolve the bracket (Lead decision recorded) or track it as an issue link instead of an inline TODO.
Confidence: HIGH — Verification: read; areas.test.tsx:73 pins current behavior only.

## 4. Per-function table
| Function/const | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| GLOBAL_HINTS | 87-98 | REAL | — | 10 msgids verified; consumed as SettingField hint prop (:128/:135) |
| GlobalAreaScreen | 117-144 | REAL | — | Frame-true column (Advanced LAST); About behind disclosure; tested |
| VideoAreaScreen | 149-156 | REAL | — | VideoArea + CodecsFold composition (codecs→video merge); tested (:211) |
| AudioAreaScreen | 161-168 | REAL | — | AudioArea + EffectsArea (effects→audio merge); tested (:221) |
| SettingsAreaId type | 176-182 | REAL | — | Generated ids ∪ 5 hand ids |
| SETTINGS_AREA_IDS | 189-203 | REAL | — | 13 in sidebar order (App 4 · Media 5 · System 4 — counted); tested (:44) |
| MISSING_GENERATED_AREAS guard | 209-216 | REAL | — | Load-time throw; fail-loud by design |
| SETTINGS_AREA_TITLES | 218-225 | REAL | — | Total Record; all 5 hand msgids exist |
| SETTINGS_SCREENS | 227-263 | REAL | — | Total Record; spread+overrides; siblings own replaced ids |
| SETTINGS_AREA_REDIRECTS | 275-281 | REAL | S4 | F-002 unconfirmed `advanced` target |
| resolveSettingsAreaSlug | 287-291 | REAL | — | Pure; consumed at SettingsScreen:292 |
| SPECIAL_FIELD_WIDGETS / WIDGET_GROUPED_KEYS re-exports | 301, 310 | REAL | — | Frozen public names; leaf sources verified |
| (module comment) | 24-29 | — | S4 | F-001 stale transient-state prose |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "is exactly the 13 Figma areas in sidebar order" (:44) | YES | Exact array equality |
| "every special widget key IS a real schemas/settings.toml key" (:56) | YES | Cross-checks registry keys against generated schema (the AC) |
| "maps every retired slug…" (:73) | YES | Enumerates the redirect map |
| "renders the global area frame-true: the Advanced bar is LAST…" (:115) | YES | DOM-order assertions |
| per-area ⓘ wiring suites (:135-:197) | YES | Count + presence of hint affordances incl. honest absences |
| "navigates between merged areas from the settings nav" (:227), "special theme picker replaces interface.theme… (live wiring)" (:235) | YES | Interaction-level |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the 13 Figma Settings areas … are THE registry" | :54-56 | REAL | 13 counted; order matches App/Media/System grouping |
| Merge decisions (hotkeys→controls, codecs→video, playlist-services→library, about→global, effects→audio) | :57-67 | REAL | Redirect map + screen compositions + tests |
| "the dead [advanced area] component was deleted w26.k13" | :64-65 | REAL | No advanced.tsx in areas/ (ls); slug redirects instead |
| "messages.ts registers all three" (video/audio/subtitles titles) | :77-82 | REAL | messages.ts:771/:775/:776 + :2253-2254 |
| "The shell (SettingsScreen) imports this module, never generated/Screens directly" | :74-75 | REAL | SettingsScreen.tsx:12-16 imports from "./areas" |
| "NOTHING may 404" (retired slugs) | :266-268 | REAL | resolveSettingsAreaSlug + first-area fallback; tested |
| "[Lead: confirm advanced → performance]" | :271-273 | UNRESOLVED | F-002 |

## 7. Dead / duplicated / unreachable code
None. Every export has a consumer (SettingsScreen or tests); the re-exports exist for the frozen registry API.

## 8. Dependencies & flags
generated/Screens (GENERATED_AREA_IDS/TITLES/SCREENS), sibling area modules (13 leaf files), SettingField/SettingsSections/specialWidgets/SettingWidgets, i18n (messages + w26.i13 lane keys). No feature flags. Module-load throw guard is intentional. (Engine-side persistence gap is W11's finding — this registry only renders rows; the SettingsProvider seam was audited in my OnboardingScreen#part2 audit, cross-ref F-001 there.)

## 9. Open questions for the next agent
- The Lead bracket in F-002 should be surfaced to the wave gate — it's a product decision, not a code fix.
- codecs.tsx (CodecsFold) is MY file 15 — the fold's own honesty audited there; this registry correctly composes it.
- SettingsScreen.tsx belongs to another agent — my reachability citations are from its seams only.
