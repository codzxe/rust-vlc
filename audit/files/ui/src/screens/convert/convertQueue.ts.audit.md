# AUDIT: ui/src/screens/convert/convertQueue.ts
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 192 / 8 (cancellable, canEnqueue, clampFraction, nextJobId, convertQueueReduce, laneCommandsOf, formatProgressLabel, sourceBasename) |
| File verdict | REAL (pure model, correct, tested) — but the lifecycle it models is UNREACHABLE in every shipped build by design |
| Reachable from shipped entry point? | YES as code: ConvertScreen.tsx:62-64,134,176-185 (route /convert). NO as behavior: enqueue is gated (ConvertScreen.tsx:147 `laneGate`, :160 `if (laneGate) return;`) and the catalog has zero convert./record. commands (`rg -c "convert\.|record\." crates/commands/src/catalog/ids.rs` -> rc 1, no matches) — the queue can never hold a job in any build. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg over catalog ids (no matches); `sed -n '120,200p' ConvertScreen.tsx`; test inventory rg (13 cases) |

## 1. One-paragraph verdict
A genuinely well-built pure reducer: total (illegal transitions return the
same array reference), clamped fractions, bounded queue, monotonic ids —
and every rule is pinned by a test. The module's honesty claims check out
byte-for-byte against the Rust side: crates/commands has NO convert/record
commands, so `laneCommandsOf` over `system.list-commands` (a REAL
lumen_dispatch round-trip the screen performs) always returns [] and the
screen disables the action ("no conversion lane -> no job, no ritual",
ConvertScreen.tsx:159). The cost of that honesty: the entire lifecycle
(queued->running->progress->complete/fail/cancel, and even the `blocked`
state, retired by w26.i18) is dead code in production — 147 lines of
state machine that can never run past `enqueue`, which itself can never
run. The convert screen is a capability probe plus a disabled button.

## 2. What actually works (verified)
— Reducer logic verified by reading + 13 tests in convertQueue.test.ts
(lifecycle :32-119, bound :121-142, probe :144-177 — including :155
"returns empty for the REAL 64-command engine catalog namespaces — the
honest today"), all in the vitest baseline (1705 passed). Mutation
reasoning: break clamping -> :103 fails; break the bound -> :122 fails;
break no-op identity -> :88 fails.
— The screen's `system.list-commands` probe is REAL backend wiring (the
one command this screen ever sends; catalog id confirmed, ids.rs:171).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/convert/convertQueue.ts:107-146 (cross ConvertScreen.tsx:159-186)
Code:
  switch (event.kind) {
    case "enqueue": {
      if (!canEnqueue(queue)) return queue;
      ...
    case "start":
      return mapJob(event.id, (job) =>
        job.state === "queued" ? { ...job, state: "running", progress: 0 } : job,
      );
What is wrong: every transition after `enqueue` — and `enqueue` itself — is
unreachable in every shipped build: the catalog exposes no convert./record.
command (verified) and ConvertScreen gates the action on the probe
(ConvertScreen.tsx:147,160). The `blocked` state (:142-145) is doubly dead
(the enqueue->blocked ritual was removed, screen comment :36). Why it
matters: ~75% of this module is a shipped state machine with no producer;
the risk is drift — when a Lead lands catalog commands the reducer runs for
the first time in production. What correct looks like: fine as a seam, but
the dead states should be labeled unreachable-by-probe in the diagram
comment (:7-12) so no one mistakes them for exercised code.
Confidence: HIGH — Verification: read + rg.
F-002 [S4]
Where: ui/src/screens/convert/convertQueue.ts:188-191
Code:
export function sourceBasename(path: string): string {
  const cut = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return cut === -1 ? path : path.slice(cut + 1);
}
What is wrong: a trailing separator ("D:\\media\\") yields the empty string
as the display name. Cosmetic-only (title attribute keeps the full path).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| cancellable | 68-70 | REAL | — | queued/running only |
| canEnqueue | 73-75 | REAL | — | bound 8 |
| clampFraction | 78-81 | REAL | — | NaN->0 |
| nextJobId | 84-86 | REAL | — | max+1, O(n) fine |
| convertQueueReduce | 93-147 | REAL (unreachable in prod) | S3 | F-001 |
| laneCommandsOf | 164-168 | REAL | — | probe; [] today, verified |
| formatProgressLabel | 177-180 | REAL | — | computed percent |
| sourceBasename | 188-191 | REAL | S4 | F-002 |
| LIST_COMMANDS_REQUEST_ID | 171 | REAL | — | used, ConvertScreen.tsx:117 seeds counter |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| lifecycle happy path :49 | YES | real fold asserts |
| total/no-op identity :88 | YES | asserts same reference |
| clamp :103 | YES | dirty fractions |
| bound :122 | YES | MAX+1 refused |
| probe :155 (real catalog namespaces) | YES | fixture mirrors the true catalog — and I verified the fixture's claim against ids.rs (no convert./record.) |
Tests are honest; the dishonesty risk is narrative: a green convert suite
can be cited as "conversion tested" while no build can enqueue a job.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the COMMAND CATALOG carries no convert/record commands yet" | :14-18 | REAL | rg over catalog/ids.rs -> no matches |
| "never a simulated progress bar, never a fake 'completed' file" | :20-21 | REAL | gate at ConvertScreen.tsx:160; no timer/fake progress anywhere in the screen |
| "Today this returns an empty list for both the demo transport and the full 64-command engine catalog" | :157-159 | REAL | catalog verified; demo side asserted by test :174 |

## 7. Dead / duplicated / unreachable code
F-001 (lifecycle unreachable in prod). No duplication. The pure functions
are all consumed by ConvertScreen or tests.

## 8. Dependencies & flags
generated/commands (CommandListing), convertTargets (ConvertTargetId).
No flags. The one backend touch is system.list-commands via client.send.

## 9. Open questions for the next agent
- ConvertScreen auditor: does the demo transport answer system.list-commands
  with the full 64-row catalog (making the gate flip-flop mid-session if it
  ever includes convert.*)? Test :155 implies the fixture matches the real
  catalog — confirm demo-transport's listing source.
