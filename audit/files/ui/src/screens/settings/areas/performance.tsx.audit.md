# AUDIT: ui/src/screens/settings/areas/performance.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 101 / 1 (PerformanceArea) |
| File verdict | REAL-UNPROVEN (schema-driven live area; store wiring real; desktop Apply reaches engine) |
| Reachable from shipped entry point? | YES — /settings/performance via App.tsx:322 -> SettingsScreen -> areas registry -> PerformanceArea |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; grep schema entries (schema.ts:1787 performance.hw-decode; 15 `key: "performance."` rows) |

## 1. One-paragraph verdict
A genuinely live settings area — the counterpart to the staged areas: sections/entries come from the generated schema seed (15 performance rows verified in schema.ts), each renders through the real SettingField (audited: store-bound, dirty-tracked, desktop Apply -> config.set-setting bridge). The module adds nothing fake; the frame's "Rescan plugins" action is honestly omitted (no command in catalog, header L28-30), and hint-less rows render without ⓘ as the frame dictates. No findings.

## 2. What actually works (verified)
— verified by grep: 15 performance schema entries exist (schema.ts); SettingField is the audited live row renderer; partitionAdvanced/schemaSectionViews are shared helpers. Baseline UI tests pass.

## 3. Findings
None. Checked: schema-driven rendering (no hand-rolled controls), hint map keys all exist in schema (spot-check hw-decode/clock-jitter), advanced disclosure state local and correct, no fabricated rows, honest omission of the commandless action row.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PerformanceArea | 77-101 | REAL-UNPROVEN | — | schema views -> SettingField rows + advanced disclosure |

## 5. Test honesty
Area covered by settings screen tests (UNVERIFIED depth — generated.test.ts / SettingsScreen.test.tsx are other agents' files).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The screen stays schema-driven: sections/entries/order come from the generated seed" | header L14-16 | REAL | schemaSectionViews("performance") + zero hand rows |
| "Rescan plugins ... is an ACTION with no command ... not rendered" | header L28-30 | REAL | no such row in the module |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
generated schema, SettingField, AdvancedDisclosure, shared partition helpers, w26.k13 hint keys. `entry.featureExposed` gates apply via SettingField.

## 9. Open questions for the next agent
- Do the performance.* config keys actually affect engine behavior in the Rust config layer (engine lane)? UI half is real.
