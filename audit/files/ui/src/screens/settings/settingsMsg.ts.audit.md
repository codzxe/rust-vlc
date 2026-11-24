# AUDIT: ui/src/screens/settings/settingsMsg.ts
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 72 / 1 hook + 1 catalog const |
| File verdict | REAL (single lane-seam resolver; the "zero lane keys differ from the registered catalog" claim re-verified TODAY — still true) |
| Reachable from shipped entry point? | YES — consumed by SettingsScreen, SettingField, SettingsSections (verified import in my SettingsSections audit), SettingWidgets → /settings routes. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python drift check: 332 lane keys across the 9 files, all 332 registered in messages.ts, 0 value differences (claim re-verified); read |

## 1. One-paragraph verdict
A consolidation fix with a documented single precedence (lane catalog first — deliberately, so f03 retitles beat the registered catalog pre-registration — then useMsg, then ""). The header claims a k12-time script proved zero lane/central drift; I re-ran the equivalent check against the current tree: 332/332 lane keys registered, 0 differing values, so the lane-first shadowing is invisible today and the "never renders different copy in shell vs rows" guarantee holds. Zero findings.

## 2. What actually works (verified)
— python check (exit 0): every lane key from the 9 files exists in messages.ts with the identical string; the merge order (f03 last) is therefore currently moot but correctly future-proofed for retitle waves.

## 3. Findings
None. Checked: precedence soundness (lane value only wins while unregistered OR deliberately re-titled), the "" degradation path, consumer wiring (4 former private copies consolidated).

## 4. Per-function table
n/a (<100 LOC).

## 5. Test honesty
Settings suites (baseline PASS) render through this resolver.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "zero lane keys differ from an already-registered catalog value" (at k12 time) | settingsMsg.ts:35-37 | TRUE today too | python: 332/332 registered, 0 diffs |
| "once the Lead registers it both paths return the same literal" | settingsMsg.ts:33-34 | TRUE | all keys registered, identical |

## 7. Dead / duplicated / unreachable code
None (the lane catalogs are belt-and-braces by design, documented).

## 8. Dependencies & flags
i18n/messages + 9 lane key files. No flags.

## 9. Open questions for the next agent
- None.
