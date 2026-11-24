# AUDIT: ui/src/screens/settings/persistence.ts
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 88 / 3 (isValidValue, localStorageSettingsPersistence.load/.save, memorySettingsPersistence) |
| File verdict | REAL — the honestly-labeled localStorage seam (browser dev shell only) |
| Reachable from shipped entry point? | YES but only as the BROWSER fallback: SettingsScreen.tsx:490 `persistence={config ?? localStorageSettingsPersistence}` — the desktop shell uses the real config bridge (SettingsScreen.tsx:461 createConfigSettingsPersistence over config.get/set-setting) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read of full file; `rg -n "createConfigSettingsPersistence|localStorageSettingsPersistence" SettingsScreen.tsx` (L461, L490); config-settings.ts header read (audio.tsx audit) |

## 1. One-paragraph verdict
Exactly what its header says: "MOCK persistence seam (card 4.E1) ... Today it round-trips localStorage for development; the REAL write path is the app layer's — config.set-setting over the client bridge" — and the app layer really does own the shipped path (SettingsScreen chooses the config bridge when a client transport exposes config, localStorage otherwise). The mock itself is hardened: unknown keys dropped, per-key value-type validation on load (j12 P3-6), corrupt-store tolerated, quota failure tolerated. The test double is labeled "NOT a production seam". Zero findings.

## 2. What actually works (verified)
— verified by SettingsScreen.tsx:461/490 wiring (desktop = config bridge; browser = this file) and config-settings.ts's own header (round-trip discipline, onWriteResult never-swallowed). Baseline vitest 1705 passed.

## 3. Findings
None. Checked: load validation (L53-59), corrupt JSON path (L61-63), save quota path (L68-70), test-double labeling (L74-79).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| isValidValue | 30-47 | REAL | — | type+enum+list-element validation; min/max deliberately not enforced (rationale documented) |
| localStorageSettingsPersistence | 48-72 | REAL (dev seam) | — | honest header; validation on load |
| memorySettingsPersistence | 81-88 | STUB (labeled test double) | — | disclosed, 4 test-file consumers |

## 5. Test honesty
config-settings.test.ts + area tests cover both seams (baseline vitest). The mock's own validation has regression coverage per the j12 P3-6 comment.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "MOCK persistence seam ... the REAL write path is the app layer's — `config.set-setting` over the client bridge" (L3-8) | header | REAL | SettingsScreen.tsx:461/490 wiring verified |

## 7. Dead / duplicated / unreachable code
None — the mock is reachable in the browser dev shell by design.

## 8. Dependencies & flags
Generated SETTINGS_ENTRIES (schema). No flags.

## 9. Open questions for the next agent
- Does the desktop config bridge actually reach an engine-side config store (crates/commands config.*)? UI side verified; engine side is another domain. UNVERIFIED here.
