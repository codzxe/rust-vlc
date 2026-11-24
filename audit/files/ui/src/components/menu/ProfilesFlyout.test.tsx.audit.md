# AUDIT: ui/src/components/menu/ProfilesFlyout.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / tests | 171 / 9 |
| File verdict | REAL — store contract + row grammar + real MenuPopup/MenuItem integration |
| Reachable from shipped entry point? | NO — vitest; baseline ui-test.txt:109 `✓ …ProfilesFlyout.test.tsx (9 tests) 137ms` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg baseline |

## 1. One-paragraph verdict
Honest suite with proper state hygiene (`afterEach(resetProfilesForTests)`):
the local profile store is pinned (seed, select persistence, unknown-id
no-op, quick-create name skipping taken names, empty-list-is-valid-user-state,
corrupt-store folds to seed without crash), the flyout row grammar is asserted
structurally (ids, selected flag, empty check slots, separator, +New), the
wire-up calls the real entry callbacks and asserts the host seams fired, and
the MenuPopup/MenuItem integration renders for real and asserts the
aria-current-not-checked grammar. Dirty-world cases (empty list, corrupt
store) are included — the honest-data posture.

## 2. What actually works (verified)
— Baseline ui-test.txt:109: 9 tests green in 137 ms.
— Mutation reasoning: unknown-id no-op (L41-44) fails if select fabricates a
selection; empty-list test (L64-68) fails if the seed resurrects over user
state; corrupt-store (L70-78) fails on crash or wrong fold; wire-up
(L118-127) fails if rows don't call the host seams.

## 3. Findings
F-001 [S4]
Where: ui/src/components/menu/ProfilesFlyout.test.tsx:105-123
Code:
    const first = entries[0] as { selected?: boolean };
    expect(first.selected).toBe(true); // frame row 1
What is wrong: repeated structural type-casts (`as { selected?: boolean }`,
`as { onSelect?: () => void }`) to read entry fields the real types evidently
don't expose — if the entry shape is refactored (fields renamed), the casts
silently produce undefined and the assertions fail with confusing "undefined
is not true" messages instead of compile errors. Hygiene-level.
Confidence: HIGH — Verification: read.

## 4. Per-function table (describe groups)
| Group | Lines | Verdict | Notes |
|---|---|---|---|
| store contract | 28-79 | REAL | 5 tests incl. dirty-world empty/corrupt |
| buildProfileFlyout grammar | 81-128 | REAL | ids/order/selected/separator/+New; wire-up |
| MenuPopup integration | 130-171 | REAL | aria-current + check slot; MenuItem grammar |

## 5. Test honesty
All 9 fail on their named regression (§2). Names match bodies.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "never a fabricated selection" | file L35 | REAL | unknown-id no-op asserted |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
RTL + vitest; real ProfilesFlyout store module + MenuPopup/MenuItem; state
reset in afterEach; no network.

## 9. Open questions for the next agent
- Is `lumen.profiles.v1` persistence ever wired to real storage (localStorage
  / config) or is memoryProfilesPersistence the only shipped adapter? (Not
  determinable from this file — UNVERIFIED.)
