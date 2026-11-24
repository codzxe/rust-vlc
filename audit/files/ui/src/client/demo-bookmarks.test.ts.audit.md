# AUDIT: ui/src/client/demo-bookmarks.test.ts
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | 136 / 1 describe, 5 tests, completed/raw narrowing helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (vitest; baseline ui-test.txt:134 "demo-bookmarks.test.ts (5 tests)" ✓ PASS) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg suite in baseline ui-test.txt |

## 1. One-paragraph verdict
A clean contract test of the DemoTransport bookmark simulation: the capability listing (trio present in system.list-commands), id assignment from 1 with non-renumbering removal, typed not-found / invalid-argument refusals (asserted on the raw envelope shape, not the happy narrowing), and per-media reset with numbering restart after a fresh session.open. The subject is the real DemoTransport class — no mocks at all.

## 2. What actually works (verified)
- Baseline ui-test.txt:134: `✓ src/client/demo-bookmarks.test.ts (5 tests) 6ms`.
## 3. Findings
F-001 [S4]
Where: demo-bookmarks.test.ts:3-4 (header claim) vs the suite body
Code:
```
// the engine-side BookmarkList contract: creation-ordered ids from 1, no
// id reuse after delete, per-media reset on open, typed refusals for
```
What is wrong: "no id reuse after delete" is claimed but never tested — the removal test (:67-77) asserts the list keeps id 2 but never adds a third bookmark afterwards to prove the next id is 3 (not 1). The per-media reset test proves a DIFFERENT fact (fresh numbering after open). Why it matters: a DemoTransport that recycles the freed id 1 would pass this suite green while violating the documented contract. Confidence: HIGH — Verification: read.
## 4. Per-function table (test blocks)
| Test | Lines | Verdict | Notes |
|---|---|---|---|
| trio listed | 29-42 | REAL | capability-gate pin |
| ids from 1, no renumber | 44-77 | REAL | labels incl. null; remove keeps id 2 |
| unknown id not-found | 79-89 | REAL | raw envelope, typed code |
| negative position invalid-argument | 91-101 | REAL | raw envelope, typed code |
| per-media reset | 103-135 | REAL | second open empties; next id 1 |
## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all five | YES | concrete ids, list contents, typed error codes; the narrowed/un-narrowed split prevents a refusal masquerading as completed |
| (the claimed no-reuse case) | NO TEST EXISTS | see F-001 |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no id reuse after delete" | :3-4 | UNVERIFIED | no test exercises add-after-delete (F-001) |
| "the simulation mirrors the engine-side BookmarkList contract" | :1-2 | REAL-UNPROVEN | engine-side contract not re-audited here (crates/…/bookmarks out of my assignment) |
## 7. Dead / duplicated / unreachable code
None.
## 8. Dependencies & flags
vitest only; DemoTransport imported directly. No feature flags.
## 9. Open questions for the next agent
- Add the add-after-delete case (expect id 3) to close F-001; confirm the engine-side BookmarkList really never reuses ids (cross-crate claim).
