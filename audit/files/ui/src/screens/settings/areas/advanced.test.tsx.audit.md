# AUDIT: ui/src/screens/settings/areas/advanced.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / tests | 36 / 2 |
| File verdict | REAL — an honest tombstone test: the component was deleted; the one live behavior its slug still owns (redirect) is pinned end-to-end |
| Reachable from shipped entry point? | NO — vitest; baseline ui-test.txt:172 `✓ …advanced.test.tsx (2 tests) 158ms` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg baseline; rg SETTINGS_AREA_REDIRECTS in areas/index.tsx (exists, L275/290) |

## 1. One-paragraph verdict
The header states the history plainly: the Advanced AREA component was
DELETED in the w26.b02 registry fold (advanced=true entries render behind
in-flow disclosures now), and only the redirect survives — so the file tests
exactly that and nothing else: a real router mount from
/settings/advanced must land on the Performance screen (title announced via
findByText), and the registry constant must map advanced→performance. This
is what an honest deletion leaves behind: no zombie component tests, one
live-behavior pin.

## 2. What actually works (verified)
— Baseline ui-test.txt:172: 2 tests green in 158 ms.
— Registry exists: ui/src/screens/settings/areas/index.tsx:275
`export const SETTINGS_AREA_REDIRECTS: Readonly<Record<string,
SettingsAreaId>>` with L290 consuming lookup.
— Mutation reasoning: redirect removal fails L30 (the retired slug then
renders an unknown-area screen); registry drift fails L34.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/settings/areas/advanced.test.tsx:30
Code:
    expect(await screen.findByText("Performance")).toBeInTheDocument();
What is wrong: the end-to-end assertion keys on a visible text node — any
other screen element containing "Performance" (a nav link, a section title)
would satisfy it even if the redirect landed elsewhere; weaker than asserting
the active route/area id. The paired constant assertion (L34) backstops the
mapping, but the router-level proof is text-shaped.
Confidence: MEDIUM — Verification: read.

## 4. Per-function table
| Test | Lines | Verdict | Notes |
|---|---|---|---|
| redirect E2E | 17-31 | REAL | real router + SettingsScreen mount |
| registry constant | 33-35 | REAL | exact map |

## 5. Test honesty
Both fail on their named regression (§2); names match the tombstone reality
documented in the header.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the `advanced` slug redirects to `performance`" | file L3-4 | REAL | index.tsx:275 + E2E test |
| "The component-mount tests died with it" | file L5-6 | REAL | no AdvancedArea component import in file |

## 7. Dead / duplicated / unreachable code
None — the file IS the documentation that the old component is gone.

## 8. Dependencies & flags
RTL + react-router MemoryRouter + MockTransport (scaffold transport for the
screen mount); no network.

## 9. Open questions for the next agent
- None.
