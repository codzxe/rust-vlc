# AUDIT: ui/src/screens/settings/areas/areas.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A05) |
| Date | 2026-06-14 |
| Lines / functions | 269 / 2 helpers + 2 describe blocks (18 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test lane — baseline ui-test.txt:57 `✓ src/screens/settings/areas/areas.test.tsx (18 tests) 648ms`; subject (areas/index.tsx registry + SettingsScreen) ships on every /settings/:area route |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `grep -n areas.test audit/runs/baseline/ui-test.txt` -> :57; `ls ui/src/screens/settings/areas/` -> controls/recording/extensions/library .tsx all present; `sed -n 1,55p` index.tsx (imports verified) |

## 1. One-paragraph verdict
Registry-integrity suite with real teeth: it pins the exact 13-area id order, that every special widget key exists in the real schemas/settings.toml-derived SETTINGS_ENTRIES, retired-slug redirect data, per-area DOM binding of every schema entry, and frame-accurate ⓘ-hint counts (global 6→10, interface 11-of-13 with two honest absences, playback 12-of-13, video 8-of-13, audio 10-of-11). The only defect is a stale trailing note that claims module resolution fails — contradicted by the passing baseline and the on-disk sibling files.

## 2. What actually works (verified)
— verified by baseline ui-test.txt:57 (18/18 pass; lane 1705 passed :178-179). Mutation reasoning: dropping an area id breaks :45-49 equality; a special widget key not in the schema fails :57-61; a hint-count regression fails :141-143/:148-157.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/settings/areas/areas.test.tsx:264-268
Code:
```
// NOTE (w26.b02): this file imports ./index, which imports the sibling area
// components ./controls, ./recording, ./extensions, ./library under the
// wave-B fixed names. Until lanes b07/b08 land those files, module
// resolution fails HERE (and in SettingsScreen.test.tsx) — expected
// parallel-lane gap, documented in docs/waves/w26/logs/w26.b02.md.
```
What is wrong: the note asserts the suite cannot resolve modules — false in the current tree: baseline ui-test.txt:57 shows 18/18 passing and `ls` shows controls.tsx, recording.tsx, extensions.tsx, library.tsx all on disk (index.tsx:49-51 imports them). Why it matters: stale "expected failure" prose teaches future readers to distrust a green suite and to excuse real resolution failures as the documented gap. What correct looks like: delete the note or convert to a historical remark with the merge marker. Confidence: HIGH — Verification: ran (baseline citation) + read.

## 4. Per-function table (file >100 LOC)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| routed()/routes | 30-41 | REAL | — | real SettingsScreen mount under MockTransport + router |
| 13-area registry test | 44-54 | REAL | — | exact-order equality, screen+title presence |
| special-widget keys test | 56-61 | REAL | — | cross-checks against real schema entries |
| area titles test | 63-71 | REAL | — | catalog or lane-key seam |
| retired-slug map test | 73-85 | REAL | — | redirect targets in registry, retired ids out |
| interface render test | 87-101 | REAL | — | every non-advanced AND advanced entry bound by data-setting |
| language screen test | 103-113 | REAL | — | dropdown trigger carries current locale |
| global frame-true test | 115-133 | REAL | — | Advanced bar last, About behind disclosure, version = UI_PACKAGE_VERSION |
| ⓘ count tests | 135-209 | REAL | — | 6 areas pinned incl. honest absences |
| codecs/effects fold tests | 211-225 | REAL | — | merged areas render behind disclosure |
| nav + theme picker tests | 227-251 | REAL | — | aria-current nav; live dataset.theme flip |
| locales test | 254-261 | REAL | — | every shipped locale bindable via enumeration |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| registry :44-54 | YES | toEqual exact array |
| special widget keys :56-61 | YES | Set membership against real SETTINGS_ENTRIES |
| hint counts :135-209 | YES | exact length assertions against DOM |
| stale NOTE :264-268 | n/a | comment, not code — see F-001 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "module resolution fails HERE" | file comment :266-267 | FALSE (stale) | baseline :57 18 tests pass; sibling files present (ls) |
| 18 tests run | baseline ui-test.txt:57 | REAL | `✓ ... (18 tests) 648ms` |

## 7. Dead / duplicated / unreachable code
None beyond F-001's stale comment. Unused imports: none (SETTING_INTERFACE_THEME used :246, AVAILABLE_LOCALES :256-259, laneKeys :67).

## 8. Dependencies & flags
All imports resolve (verified against disk: index.tsx, schema, locales, messages, w26.b02 keys).

## 9. Open questions for the next agent
None.
