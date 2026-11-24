# AUDIT: ui/src/screens/convert/ConvertScreen.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | 237 / 1 describe, 9 tests, CatalogTransport double + 3 helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (vitest; baseline ui-test.txt:67 "ConvertScreen.test.tsx (9 tests)" ✓ PASS) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg suite in baseline ui-test.txt |

## 1. One-paragraph verdict
The capability-honesty test for the convert screen, covering BOTH branches of the capability seam: (a) with the real today-catalog (no convert.* commands) the probe fires exactly once, the honest banner renders, the primary action is disabled-from-data with an aria-describedby gate and data-wiring-stub marker, and a filled source + direct form submit STILL enqueues nothing (the anti-ritual pin — transport.sent stays exactly ["system.list-commands"]); (b) with a future-lane catalog the action enables, jobs queue, the 8-job bound refuses the 9th honestly, queued is cancellable and terminal rows lose the cancel affordance. The target picker is pinned to EXACTLY the two real operations. No mock-of-subject; CatalogTransport is the sanctioned transport-port double.

## 2. What actually works (verified)
- Baseline ui-test.txt:67: `✓ src/screens/convert/ConvertScreen.test.tsx (9 tests) 322ms`.
## 3. Findings
Zero findings. Checked for: tautologies (none — the anti-ritual test asserts the exact command log), mocks-of-subject (none — real screen, real router), ignored tests (none), vacuous greens (none — disabled-from-data, honest notes and bound refusals all asserted).
## 4. Per-function table (test blocks)
| Test | Lines | Verdict | Notes |
|---|---|---|---|
| backdrop retirement | 98-114 | REAL | mount + unmount assertions |
| two real targets + honest notes | 116-137 | REAL | exactly 2 options, selected default, per-target note flips on switch |
| Browse disabled stub | 139-149 | REAL | toBeDisabled + data-wiring-stub + gate wired via aria-describedby + no non-probe command |
| catalog probe once + banner | 151-161 | REAL | exactly one system.list-commands in the log |
| no-lane: disabled-from-data, no ritual | 163-183 | REAL | submit with filled source enqueues nothing; sent == ["system.list-commands"] exactly |
| empty source refusal (lane present) | 185-192 | REAL | honest error + no enqueue |
| 9th job bound refusal | 194-211 | REAL | 8 listitems then "queue is full", count stays 8 |
| lane present: queued + cancellable + terminal no-cancel | 213-229 | REAL | status transitions Queued -> Cancelled via the real cancel affordance |
| Back to player exits | 231-236 | REAL | route change asserted, no command |
## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| no-lane anti-ritual | YES | a fabricated enqueue attempt would show in transport.sent / a listitem |
| 9th job bound | YES | exact listitem count 8 + full-queue text |
| cancel lifecycle | YES | status text + cancel affordance presence/absence per state |
| target picker | YES | exact option set — an invented MP3/MKV target fails |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the enqueue→blocked ritual is gone: no job, no fabricated request" | header :6-8 | REAL | asserted :163-183 |
| "The full lifecycle arithmetic … is pinned by convertQueue.test.ts against the pure reducer" | header :15-16 | REAL-UNPROVEN | convertQueue.test.ts not in my assignment (other agent) |
## 7. Dead / duplicated / unreachable code
None.
## 8. Dependencies & flags
@testing-library/react, MemoryRouter; CommandListing shape imported from generated/commands. No feature flags.
## 9. Open questions for the next agent
- The future-lane branch relies on a transport that ANSWERS convert.start with a canned unit completion but the queued/cancelled states are driven by the local store — the engine-driven lifecycle (progress/complete events) is only covered by the reducer suite, not this screen test (acceptable split, noted in header).
