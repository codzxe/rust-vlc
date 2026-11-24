# AUDIT: ui/src/screens/settings/areas/video.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 137 / 6 tests in 1 describe |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test-infra for shipped settings Video area; runs under vitest |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -n "areas/video.test" audit/runs/baseline/ui-test.txt` → `98: ✓ src/screens/settings/areas/video.test.tsx (6 tests) 455ms`; full file read |

## 1. One-paragraph verdict
Strong schema-truth tests: every seeded video entry must render bound to its real key with a per-key failure message, the ship-blocker blank-option regression is pinned by opening both dropdowns and asserting EVERY option's accessible name plus a real pick with dirty marking, the Picture rows are pinned as sliders (never bare inputs) with exact ffmpeg-domain ranges, Reset restores through the real store, and the area's no-engine-query contract is asserted by an empty transport.sent. Fixture transport mocks only the backend seam.

## 2. What actually works (verified)
- Committed run: baseline ui-test.txt L98 `✓ ... (6 tests) 455ms` (inside 1705 passed).
- Per-key loop assertions with descriptive failure messages (L31-35); option-label sweep L63-76; slider-domain pins L91-103; dirty/clean round-trip via keyboard commits L109-117; transport.sent empty pin L132-136.

## 3. Findings
None. Zero-findings checklist: tautology scan — every expect binds to DOM/roles/attributes produced by the real area over the real generated schema; no self-comparison. Mock-of-subject scan — no vi.mock; makeFixtureTransport mocks the transport seam only, and the test asserts what was NOT sent through it (honest direction). Skip scan — no it.skip; baseline 6 passed. Name-vs-body scan — verified per test. Mutation reasoning: unbinding a row from its key fails L33-34; blank-option regression fails L63-76; replacing sliders with inputs fails L86-88; breaking Reset fails L115-117; engine-query regression fails L135.

## 4. Per-function table (>100 LOC, test-file form)
| Test group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| frame sections/schema binding | 27-55 | REAL | — | all entries bound, toggle states, dropdown values |
| option labels never blank | 57-77 | REAL | — | full label sweep + pick + dirty |
| Picture inline sliders | 79-104 | REAL | — | data-kind=slider, no bare input, exact domains |
| Reset picture | 106-118 | REAL | — | dirty→clean round-trip through store |
| action row grammar | 120-130 | REAL | — | ResetPictureRow anatomy, enabled |
| no engine queries | 132-136 | REAL | — | transport.sent empty |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 6 | YES | schema-key-bound DOM assertions + behavior round-trips; verified green baseline L98 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the area never queries the transport" | header L5-6 | REAL | L132-136 asserts transport.sent === 0 |
| "the saturation/gamma ranges are the ffmpeg eq domains — 0..300 / 0..1000" | comment L95-97 | REAL-UNPROVEN | asserted on the DOM (L99-103); ffmpeg domain equivalence UNVERIFIED (engine not my file) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
@testing-library/react, vitest; real VideoArea/ResetPictureRow/SettingsProvider/generated schema; makeFixtureTransport backend-seam fixture.

## 9. Open questions for the next agent
- None.
