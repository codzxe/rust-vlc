# AUDIT: ui/src/components/menu/Tooltip.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A05) |
| Date | 2026-11-24 |
| Lines / functions | 163 / 8 tests |
| File verdict | REAL (tests) — SUBJECT UNREACHABLE |
| Reachable from shipped entry point? | Test lane YES (baseline ui-test.txt:114, 8 tests). Component under test: NO — `Tooltip` is exported (menu/index.ts:8) and mounted by nobody |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 identifier scan (zero `<Tooltip` mounts in non-test sources), it-count 8 == baseline |

## 1. One-paragraph verdict
An excellent suite in a vacuum: hover-intent show/hide delays with off-by-one boundary
assertions against the real timing constants, keyboard-focus immediate show, Escape dismissal
with correct scoping (open tooltip consumes Escape so a host dialog does not close; closed
passes through — j02 J02-13), aria-describedby riding the FOCUSABLE child not the wrapper
(J02-1), and merge-not-clobber of an existing describedby. All 8 pass in the baseline. But the
subject is dead code: exhaustive identifier scan over src/**/*.tsx|ts (non-test) finds `Tooltip`
only in its own file, the barrel export, the timing constants' comments, and one
"the pattern" comment in SettingField.tsx:114 — no JSX mount anywhere.

## 2. What actually works (verified)
— verified by citation audit/runs/baseline/ui-test.txt:114 `✓ src/components/menu/Tooltip.test.tsx
(8 tests) 188ms` (1705-passed run); 8 it() blocks on disk match.

## 3. Findings
F-001 [S3]
Where: ui/src/components/menu/index.ts:8 (and the absence of any mount)
Code:
export { Tooltip } from "./Tooltip";
export type { TooltipProps } from "./Tooltip";
What is wrong: `Tooltip` is exported and never used — no `<Tooltip` in any non-test module
(python3 scan: zero hits). The 8 passing tests verify a component the shipped app cannot
render; hover-intent delays (timings.ts:22-25, their own [PROPOSAL] markers) are likewise
unconsumed by the product. Why it matters: dead primitive + dead test mass, and doc 08's
primitives list (§259, per Tooltip.tsx:20) reads as if shipped. What correct looks like: mount
it on icon-only controls (ControlBar buttons lack text labels — the classic use) or delete.
Confidence: HIGH — Verification: read (scan ran).
No S0-S2: no tautologies — every assertion binds to role/aria/timer behavior of the real
component.

## 4. Per-function table
| Test (line) | Verdict | Severity | Notes |
|---|---|---|---|
| show after delay (18) | REAL | — | SHOW_DELAY-1 boundary |
| hide after leave delay (38) | REAL | — | HIDE_DELAY-1 boundary |
| keyboard focus immediate (62) | REAL | — | no timers |
| describedby on child (75) | REAL | — | J02-1 regression |
| merge describedby (91) | REAL | — | external-hint preserved |
| Escape hides (109) | REAL | — | |
| Escape scoping (122) | REAL | — | J02-13: host not called when open; passes when closed |
| custom delays honored (144) | REAL | — | props override constants |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 8 | YES | boundary assertions fail if delays change; scoping test fails if stopPropagation is dropped (hostKeyDown would be called twice) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Timer callbacks setState outside React's event flow, so advances are wrapped in act" | header 9-10 | REAL | every advance is act-wrapped |

## 7. Dead / duplicated / unreachable code
F-001 (subject dead). Fixture duplication: none.

## 8. Dependencies & flags
In-repo (Tooltip, timings). Fake timers only within the two delay tests; restored in afterEach.

## 9. Open questions for the next agent
Same family question as MenuBar.test.tsx: was the menu primitives barrel (MenuBar, Tooltip)
superseded wholesale by MenuPopup/ContextMenu + CSS title attributes? If yes, both components
and both suites are dead weight to delete, not maintain.
