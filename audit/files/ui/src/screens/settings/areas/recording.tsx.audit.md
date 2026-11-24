# AUDIT: ui/src/screens/settings/areas/recording.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 159 / 2 (RecordingArea, RECORDING_EXPOSED const) |
| File verdict | REAL — 100% staged-inert by a real feature gate |
| Reachable from shipped entry point? | YES — route /settings/recording (System group) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -n "recording =" product.toml` (L164: `false # Phase 7`); `rg -n "it\(" recording.test.tsx` (6 tests); full read |

## 1. One-paragraph verdict
The entire area is honest dead surface, correctly gated: every one of the frame's 10 rows renders disabled with the staged reason (Recording section conditionally — only while the `recording` feature gate is off, product.toml:164 `recording = false # Phase 7`; Casting/Streaming permanently until schema keys exist), the Advanced bar is an inert disclosure, and the generated screen (the only live path for the seeded rows) is embedded so exposure flipping needs no code change. Tests pin the gating, the staged rows, and the ⓘ census. Zero findings — the staging is the product.

## 2. What actually works (verified)
— verified by recording.test.tsx:39-114 (routing, generated embed, schema gate truth, gated rows hidden+inert, staged frame-verbatim rows, ⓘ census 4/10 operable) and product.toml:164. Baseline vitest 1705 passed.

## 3. Findings
None. Checked: all controls disabled via StagedRow (see staged.tsx audit); no enabled control anywhere in this file; the conditional Recording section cannot double-render with the generated screen (the `!RECORDING_EXPOSED` guard).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| RECORDING_EXPOSED | 38-40 | REAL | — | compile-time from generated schema featureExposed |
| RecordingArea | 47-159 | REAL (staged) | — | 10 inert rows + inert advanced bar; yield rule documented |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| :62 schema gate truth | YES | asserts seed rows + gate |
| :72 gated rows hidden+inert while off | YES | the yield rule |
| :83 staged rows never live bindings | YES | disabled + reason attributes |
| :114 ⓘ census 4/10 | YES | frame-fidelity regression |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The seeded rows are feature-gated (`recording` = false, product.toml Phase 7, rule F2)" (L6-7) | header | REAL | product.toml:164 verified |
| "the moment exposure flips, GENERATED_SCREENS.recording renders them live ... no double render" (L13-17) | header | REAL | test :72 + the !RECORDING_EXPOSED guard |

## 7. Dead / duplicated / unreachable code
All 10 rows are inert-by-design (quantified: this is 100% of the area's interactive surface). Marked data-staged; the honest-dead pattern per staged.tsx.

## 8. Dependencies & flags
Generated Screens + schema; product.toml feature gate `recording` (Phase 7). The generated-vs-handwritten boundary is respected (staged rows reference seeded keys by string, live rows only via the generated screen).

## 9. Open questions for the next agent
- None. Recording backend (if any) is Phase 7 per product.toml — engine-side audit out of scope.
