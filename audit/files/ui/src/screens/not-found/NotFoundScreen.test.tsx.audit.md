# AUDIT: ui/src/screens/not-found/NotFoundScreen.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A07) |
| Date | 2026-11-24 |
| Lines / functions | 60 / 4 tests + Where probe + render404 helper |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — vitest baseline ui-test.txt:164 (4 tests, 137ms); subject is the live catch-all route (App.tsx:310 `path="*"`, lazy-loaded :65) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | it-count 4 == baseline; App.tsx route rg |

## 1. One-paragraph verdict
Compact and complete for its scope: h1 copy, primary CTA class, decorative cone mark
(aria-hidden), the Solstice-retirement regression (dataset.backdrop untouched at mount AND
unmount — a real negative-space guard against the retired veil tier sneaking back), and the
CTA navigation back to /player through a router probe. All 4 pass in the baseline. Subject is
the shipped 404 route.

## 2. What actually works (verified)
— verified by citation audit/runs/baseline/ui-test.txt:164 `✓ src/screens/not-found/
NotFoundScreen.test.tsx (4 tests) 137ms` (1705-passed run); 4 it() blocks on disk match.

## 3. Findings
Zero numbered findings. Checked, and how: no tautologies (heading role+level, class, aria
attributes, dataset absence, router pathname all component-output-bound); no mocks of the
subject (MemoryRouter is the sanctioned router seam); no dead fixtures; no swallowed errors;
no timers. Mutation reasoning: removing the CTA navigate → :58 fails; reintroducing the
backdrop veil → :50/:52 fails.

## 4. Per-function table
| Test (line) | Verdict | Severity | Notes |
|---|---|---|---|
| quiet hero + one CTA (32) | REAL | — | level-1 heading pinned |
| cone decorative (41) | REAL | — | aria-hidden |
| Solstice veil retired (48) | REAL | — | negative guard, mount+unmount |
| CTA returns to player (55) | REAL | — | waitFor router probe |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 4 | YES | see mutation examples above |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "never a theatrical stage" | header 8-9 | REAL | :48-53's backdrop absence guard |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
In-repo + react-router-dom test seam. No i18n surface asserted (copy asserted raw — the screen
uses literal English today; if it migrates to useMsg these tests break loudly, which is
acceptable drift detection).

## 9. Open questions for the next agent
None material.
