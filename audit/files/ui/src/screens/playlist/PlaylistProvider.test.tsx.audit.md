# AUDIT: ui/src/screens/playlist/PlaylistProvider.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A03 |
| Date | 2026-11-24 |
| Lines / functions | 299 / 2 describes, 15 `it` blocks, 1 fake transport class + 4 helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — vitest lane (`vitest run`); baseline executed |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read; rg `\bit\(` → 15; rg baseline → pass line |

## 1. One-paragraph verdict
A strong provider-contract suite: every action is asserted through a
dispatch spy (`transport.sent`) with EXACT catalog args ({id:3}, {n:-2},
{id:3,toIndex:0}, {enabled:true}, {mode:"one"}), engine state arrives only
via emitted events/snapshot, and the suite contains two genuinely hard
regression tests — a mid-flight `moved` event between enqueue and move
(w26.k14 j14 #5, :218-233) and action-identity stability across queue
changes (no context churn, :238-248). The not-exposed path is tested to
stay null rather than fabricate defaults (:289-298). Ran green in the
committed baseline. The one wrinkle: the fake transport's `default:` arm
(:74-75) answers any unknown command with `completed: unit` — convenient,
but it means a typo'd command name in the provider would pass this suite's
spy-only checks for commands whose args are not asserted.

## 2. What actually works (verified)
— Baseline: audit/runs/baseline/ui-test.txt:51 `✓ src/screens/playlist/PlaylistProvider.test.tsx (15 tests) 619ms` — matches rg count.
— Mutation reasoning: if `playNext` computed the move index from a stale
closure, :232's `toIndex === 3` fails (the documented pre-fix value was 2);
if actions were rebuilt per change, :244-247 identity checks fail; if the
provider faked not-exposed state, :294-296 fail.
— Fake transport is a boundary mock (implements catalog commands over a tiny
model) — the correct mocking level for a provider test.

## 3. Findings
F-001 [S4]
Where: :74-75
Code: `default: return { gen, requestId, result: { completed: { kind: "unit" } } };`
What is wrong: the fake answers ANY command with success — combined with
spy-only assertions for some actions (e.g. `playlist.clear` checked only by
existence, :161), a misspelled command would still "succeed" in tests.
Why it matters: low blast radius (the catalog is typed in the real transport),
but the fake's default arm weakens the spy contract for commands whose args
aren't asserted. What correct looks like: default arm returns
feature-not-exposed like the exposed=false path.
Confidence: HIGH — Verification: read.

## 4. Per-function table (every test)
| Describe (lines) | Tests | Verdict | Severity | Notes |
|---|---|---|---|---|
| catalog command dispatch (126-256) | 11: snapshot seeds summary; play-item exact args; next/prev/skip; remove/clear/move args; set-shuffle/set-repeat; play-later = enqueue only (no move asserted absent :181); play-next = enqueue+move toIndex 2; play-next no-current appends (toIndex 3); play-next mid-flight moved (toIndex 3, regression w26.k14); action identities stable; playlist.load args | REAL | S4 | F-001 default-arm note |
| engine events authoritative (258-299) | 4: items-added honest null fields; items-removed/moved/cleared; shuffle/repeat/current set; feature-not-exposed → nulls, entries stay | REAL | — | |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| play-next mid-flight (218-233) | YES | exact toIndex from live queue state |
| identities survive changes (238-248) | YES | referential-equality assertions |
| not-exposed flags (289-298) | YES | nulls asserted, no fabricated defaults |
| all 15 | YES | every test asserts dispatched args or event-applied state |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every action must dispatch exactly the catalog command with the catalog args" | :1-3 | REAL for asserted commands; WEAK for existence-only ones (clear) | spy assertions throughout |

## 7. Dead / duplicated / unreachable code
None; no skip/only/todo.

## 8. Dependencies & flags
First-party seams only (ClientProvider, transport types, PlaylistProvider).

## 9. Open questions for the next agent
None.
