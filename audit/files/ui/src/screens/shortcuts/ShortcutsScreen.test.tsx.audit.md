# AUDIT: ui/src/screens/shortcuts/ShortcutsScreen.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A03 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / tests | 289 / 14 (1 describe) |
| File verdict | REAL (baseline-verified green; behavior-level, mutation-hard) |
| Reachable from shipped entry point? | YES — tests ShortcutsScreen.tsx (511 LOC), the /keyboard-shortcuts route screen in App.tsx; visual baseline keyboard-shortcuts-1280x720-1x-light.png corroborates |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg baseline ui-test.txt:52-53; rg source exports |

## 1. Verdict
A behavior contract suite, not a render smoke test: friendly-keycap mapping (no raw
"VolumeMute"/"MediaNext" spill), numeric bookmark ordering 1..10 (kills lexical sort bugs),
curated row order, three-way search scoping, focus-trap wrap semantics both directions,
scrim/✕/Escape dismissal through the router, and a pure-function test of filterGroups.
Baseline green: ui-test.txt:52 `✓ …ShortcutsScreen.test.tsx (14 tests) 2089ms`.

## 2. What actually works (verified)
- Baseline vitest 14/14 pass (cite above).
- Mutation reasoning: lexical bookmark sort fails (:147-154); raw media-key spill fails
  (:95-110); broken focus trap fails (:240-257); Escape-while-listbox-open leaking to window
  close fails (:277-281); search scope ignoring the filter fails (:200-211).
- Export surface under test exists: filterGroups (ShortcutsScreen.tsx:326),
  buildShortcutGroups (:238), ShortcutsScreen (:349).
- Routed dismissal tests use a real /player sibling route, proving navigation, not just
  unmount (:29-37, 266-287).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/shortcuts/ShortcutsScreen.test.tsx:214-226
Code:
    const labelOf = (key: { toString(): string }) => key.toString();
    const comboOnly = filterGroups(groups, labelOf as never, "Ctrl+O", "hotkey");
What is wrong: the pure-function test injects a stub labeler via an `as never` type-cast — the
REAL label function (whatever ShortcutsScreen passes at runtime) is never exercised in the
action/all scope assertions; the cast also defeats the compiler's check that the labeler has
the right shape. Scope mechanics are covered; label-matching is only covered through the DOM
tests above.
What correct looks like: export/replicate the real labeler or type the stub properly.
Confidence: HIGH — Verification: read.
F-002 [S4]
Where: ui/src/screens/shortcuts/ShortcutsScreen.test.tsx:47-49, 59-63, 228-231
Code:
    "renders the four reference groups as sections of the shortcuts window",
    { timeout: 30_000 },
What is wrong: three tests carry 6x timeout bumps justified by a shared-20-lane load comment
(:61-62 "load avg >10 observed"). Not a defect of the screen — a capacity smell that this
suite's renders (146 rows in jsdom) flirt with the default 5s budget on loaded runners. The
assertions themselves are unchanged, so no honesty loss.
Confidence: HIGH — Verification: read.

## 4. Per-test table (tests >100 LOC)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| four reference groups | YES | 4 headings + non-empty rows |
| window grammar | YES | dialog role + aria-modal + disabled Save |
| friendly keycaps | YES | 12 raw ids must be absent, 4 friendly present, >50 caps |
| one keycap per combo | YES | exact combo strings + per-row keycap count |
| edit pencil link | YES | href + aria-label exact |
| numeric bookmark order | YES | [1..10] exact arrays |
| curated order | YES | slice equality |
| search filters | YES | group presence/absence + empty-result copy |
| scoped search | YES | All Fields/Action/Hotkey discrimination |
| filterGroups pure | YES (mechanics) | combo-only vs action scope counts |
| trapped modal | YES | 4 focus moves asserted |
| scrim/✕/Escape close | YES | player-home testid after navigation |

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "pins the behavior contracts that survive the reskin EXACTLY as before" | :1-14 | REAL | the contracts are asserted, not just named |

## 6. Dead / duplicated / unreachable code
None. All helpers used.

## 7. Dependencies & flags
@testing-library/react + react-router; no mocks of the component itself; no snapshots.

## 8. Open questions for the next agent
Does ShortcutsScreen.tsx's real labeler match the DOM-tested behavior (F-001 gap)?
