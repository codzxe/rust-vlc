# AUDIT: ui/src/screens/settings/SettingsScreen.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | 387 / 26 it() tests, 2 describe blocks, 4 helpers |
| File verdict | REAL (26/26 pass in baseline; assertions bite incl. localStorage persistence) |
| Reachable from shipped entry point? | YES via test lane — baseline ui-test.txt line 40 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | full file read; rg count (26 == baseline 26, exact); sed quote verification (exit 0) |

## 1. One-paragraph verdict
Honest shell test over the real routed screen (MemoryRouter + /settings/:area + ClientProvider
with MockTransport). It pins: the 13-area registry, the focus trap using the component's OWN
exported FOCUSABLE selector (cannot drift from the trap's definition), apply/revert with REAL
localStorage persistence equality, restart-required badges, cross-area search + the
stale-search-on-navigation regression, the dirty navigation dialogs, per-field reset, and a
scrollIntoView spy with proper cleanup proving the fold-reveal fix. 26 `it(` == baseline 26.

## 2. What actually works (verified)
- Baseline: audit/runs/baseline/ui-test.txt line 40 `✓ src/screens/settings/SettingsScreen.test.tsx (26 tests) 1640ms`.
- Persistence proof (verified sed -n '196,206p'): `expect(JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? "{}")).toEqual({ "interface.keep-above": true, });` — would FAIL if apply persisted nothing/more.
- Revert proof: `expect(window.localStorage.getItem(STORAGE_KEY)).toBeNull();` (never persisted).
- Trap honesty (lines 92-110, read): imports `FOCUSABLE` from the component and applies the
  same aria-hidden filter — the test cannot pass with a drifted trap definition.
- scrollIntoView spy (lines 292-305, read): installs a spy, `afterEach` deletes it — no leak
  into other tests' jsdom.

## 3. Findings
F-001 [S4]
Where: lines 278-283 (`resetButtons[0]` click)
Code: `const resetButtons = screen.getAllByRole("button", { name: "Reset to default" });` …
`fireEvent.click(resetButtons[0] as HTMLElement);`
What is wrong: resets the FIRST reset button in DOM order while the toggled field is
"Keep window above other windows" — if DOM order changes, this clicks a different field's
reset and the subsequent assertion on the toggle's aria-checked could pass vacuously via
an unrelated default. Row-scoped lookup (`within(row).getByRole(...)`) would be honest.
Confidence: MEDIUM — read.
F-002 [S4]
Where: describe 2 (lines 291-387) has no `beforeEach(localStorage.clear())` (describe 1 has it)
What is wrong: the second describe relies on describe-1's cleanup not applying — localStorage
state leaks between describes within the same file run. Currently harmless (the second
describe's tests don't assert storage), but the asymmetry is a latent hazard.
Confidence: HIGH — read.

## 4. Per-function table
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| renderAt / field | 17-31 | REAL | — | real route mount; data-setting lookup |
| describe shell (card 4.E1) | 33-289 | REAL | S4 | 24 tests: nav, focus trap, dirty flows, search, reset |
| describe Advanced disclosure | 291-387 | REAL | S4 | scrollIntoView spy + cleanup, 2 tests |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| shows the apply bar when dirty; apply persists and clears it | YES | localStorage deep-equal on the exact key |
| clears the search query on sidebar navigation | YES | stale-results absence + clean screen assertions |
| discarding the dialog reverts edits and navigates | YES | applybar null + storage null after discard |
| codecs fold: expanding scrolls revealed rows | YES | spy length 1 + exact element id + block:nearest |
| redirects an unknown area to the first schema area | YES | aria-current on General after /settings/nope |
| resets a single field to its schema default | WEAK | F-001 first-button-by-order |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "SettingsScreen shell (card 4.E1) (24)" subgroup, 26 tests total | baseline ui-test.txt:40-41 | REAL | vitest lines |
| "the test imports the trap's OWN selector (FOCUSABLE) ... can never drift" (comment lines 92-96) | file itself | REAL | import at line 6 |

## 7. Dead / duplicated code
None — all helpers used; FOCUSABLE import justified.

## 8. Dependencies & flags
MockTransport (honest config-not-exposed answers); react-router; vitest. No timers. No
inline snapshots. jsdom localStorage used for real persistence checks — good.

## 9. Open questions
- Are the 13 area screens' CONTENT rows covered elsewhere (areas/*.test.tsx)? Yes per
  assignment (library.test, conflicts.test, updates-about.test, LocalePicker.test) — spot
  audits follow.
