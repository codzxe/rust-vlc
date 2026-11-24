# AUDIT: ui/src/screens/settings/SettingField.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 180 / 9 tests across 3 describes |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test-infra for shipped settings UI (SettingField/SettingInfoDot/SettingsProvider); runs under vitest |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -n "SettingField.test" audit/runs/baseline/ui-test.txt` → `82: ✓ src/screens/settings/SettingField.test.tsx (9 tests) 224ms`; full file read |

## 1. One-paragraph verdict
Honest row-anatomy tests over the real provider-wrapped component: closed dropdowns (explicitly asserting NO stacked radiogroup), option-selection with dirty marking, the raw-value fallback floor for unregistered msgids (constructed probe entry — the honest way to pin a catalog gap), label-left grammar for text/duration rows with the ms round-trip asserted through the real store via a probe component, and the ⓘ affordance with both honest-absence and full aria-describedby/aria-controls association checks. No mocks of the subject.

## 2. What actually works (verified)
- Committed run: baseline ui-test.txt L82 `✓ ... (9 tests) 224ms` (inside 1705 passed).
- Real seams: SettingsProvider-mounted component (L22); generated schema entries imported (L14-20); duration round-trip "3 s" → input → `latest === 10000` via useSettings probe (L99-124).
- The fallback probe (L59-66) spreads the real SETTING_VIDEO_ASPECT_RATIO and overrides one enumeration entry's msgid to an unregistered one — tests the subject's own fallback, not a mock of it.

## 3. Findings
None. Zero-findings checklist: tautology scan — every expect binds to rendered roles/text/attributes or store values read through the provider hook; the double-negative guards (`.lumen-radiogroup` absent L34-36, blank-option guards L71-75) are absence assertions on the subject's real output. Mock-of-subject scan — no vi.mock; only an inline Probe component reading the store. Skip scan — no it.skip; baseline 9 passed. Name-vs-body scan — verified per test. Mutation reasoning: blank-option regression (h09) fails L71-74; duration parse regression fails L123; hint-a11y regression fails L143-155; glyph size regression fails L170-171.

## 4. Per-function table (>100 LOC, test-file form)
| Test group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| enum rows = closed dropdowns | 24-86 | REAL | — | haspopup/expanded, selection+dirty, raw fallback floor, frame labels e2e |
| label-left grammar | 88-125 | REAL | — | text row + duration row with ms round-trip via real store |
| ⓘ hint affordance | 127-179 | REAL | — | honest absence, reveal on demand with full a11y wiring, glyph geometry pins |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 9 | YES | roles/attributes/store round-trips asserted on the real component; verified green baseline L82 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the schema seed carries no hint text today" | comment L4-5 | REAL | L128-131 asserts absence for a real seeded entry |
| "aria-label used to win over `title` ... it now rides as the accessible DESCRIPTION" | comment L139-142 | REAL | L143-146 asserts aria-describedby twin exists with text + .settings-vh class |
| "the shared 'info' sheet ... at size 18 lands exactly on those frame metrics" | comment L161-166 | REAL-UNPROVEN | asserts width/height 18 (L170-171); the r7.5/sw1.125 equivalence itself is SVG geometry UNVERIFIED (icon source not my file) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
@testing-library/react, vitest; real SettingField/SettingInfoDot/SettingsProvider/generated schema. No mocks.

## 9. Open questions for the next agent
- None.
