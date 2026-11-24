# AUDIT: ui/src/screens/settings/areas/codecs.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 110 / 2 components |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: areas registry -> CodecsFold (video.tsx:163-170) renders CodecsArea below /settings/video |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg featureExposed/product.toml gate data (exit 0); baseline ui-test.txt cited |

## 1. One-paragraph verdict
Honest capability gating done right: the hw-decode policy control is bound to the REAL schema key (`playback.hw-decode-policy`), renders visible, and is disabled with a gate note because the generated schema carries `featureExposed: false` (schema.ts:975) which mirrors product.toml:168 `hw-decode = false` ("ADR-001 accepted ... implementation is Phase 7 (7.B) — rule F2 keeps this false until then"). The gate is DATA, not a hard-coded boolean. The two cache keys are real schema entries wired through the settings store (draft -> Apply -> engine config in the desktop shell). The inert `onChange` (L75-77) is belt-and-braces under a disabled control, not a dead click.

## 2. What actually works (verified)
— Gate data chain: codecs.tsx:51 `const disabled = !entry.featureExposed;` <- schema.ts:975 `featureExposed: false` <- product.toml:168. The visible reason row renders when disabled (L82-88).
— Cache keys: SETTING_PERFORMANCE_CACHE_DEMUX_MIB / CACHE_NETWORK_MIB rendered via SettingField -> settings store (same real Apply path as video.tsx).
— Baseline: settings suites pass (ui-test.txt).

## 3. Findings
None. Checked: no dead controls (the one disabled control carries `data-gated-disabled`, a role=note reason, and a data-gate-reason attribute); no fabricated codec toggles ("NO schema entries -> NOTHING" holds — no invented rows); i18n through the lane seam.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useAreaMsg | 39 | REAL | - | shared factory |
| HwDecodePolicyControl | 47-91 | REAL | - | data-gated disabled + reason; inert onChange under disabled control |
| CodecsArea | 94-110 | REAL | - | two schema-backed sections |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| codecs/settings tests (baseline pass) | YES | gate rendering + cache rows asserted |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "product.toml keeps `features.hw-decode = false` under rule F2" | L7-8 | TRUE | product.toml:168 |
| "DISABLED WITH THE HONEST REASON ... never a dead control that looks alive" | L9-11 | TRUE | L51-59, L82-88 |
| "cache ... schema placement currently also renders them on the generated 'performance' screen" | L16-19 | UNVERIFIED (known-duplication, flagged in NEXT-STEPS by the authors) | would need the performance screen render; flagged not hidden |

## 7. Dead / duplicated / unreachable code
None in-file. (The schema-side duplicate placement of the cache keys on the performance screen is admitted in the header and filed to the schema owner.)

## 8. Dependencies & flags
Generated schema consts (featureExposed data), Dropdown, SettingField, settings store. No UI-side flags — gating comes from generated data.

## 9. Open questions for the next agent
- Does the generated /settings/playback screen ALSO render the hw-decode row (schema screen: "playback", section: "tracks")? If yes, the same gated control exists twice; if SettingField honors featureExposed there too, it is consistent — verify.
