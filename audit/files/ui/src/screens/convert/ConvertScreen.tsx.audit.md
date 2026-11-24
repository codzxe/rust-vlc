# AUDIT: ui/src/screens/convert/ConvertScreen.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 406 / 4 (component + hook + 2 handlers) |
| File verdict | SHALLOW (real probe, honest gating; the feature itself is dead in every build) |
| Reachable from shipped entry point? | YES: App.tsx:309 `<Route path="/convert" element={<ConvertScreen />} />` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg over generated/commands.ts + apps/cli/src/host.rs (exit 0); baseline ui-test.txt cited |

## 1. One-paragraph verdict
The screen's ONE real backend touch is a negative probe: it sends `system.list-commands` (served by the CLI/desktop host, apps/cli/src/dispatch.rs:139-143) and filters for `convert.*`/`record.*` ids (convertQueue.ts:164-168). No such command exists anywhere — not in the generated CommandRequest union (ui/src/generated/commands.ts has zero convert/record cmds), not in the CLI host's listings — so `laneGate` settles true in EVERY build and the Start button renders permanently `disabled` with an aria-describedby reason. Combined with the Browse button (disabled, `data-wiring-stub="native file picker — no catalog command exposed"`, L235-243) and a queue that can never gain a row (enqueue is gated behind the same lane), this is a 406-line dead surface: the only functional controls are the Back navigation and local-state form fields. It is honestly dead (disabled-from-data, visible gate note, no simulated progress), and the underlying engine capability genuinely exists (crates/transcode/src/{convert,job,graph}.rs), but nothing on this screen is wired to a real backend.

## 2. What actually works (verified)
— The probe is real: `client.send({ cmd: "system.list-commands", ... })` (L125-126) against the transport; CLI host serves it (dispatch.rs:139-143 `Command::SystemListCommands`).
— Honest disabled affordances: Start `disabled={laneGate}` + `aria-describedby="convert-lane-note"` (L299-310); Browse `disabled` + gate line (L235-248).
— Baseline vitest: ConvertScreen.test.tsx + convertQueue.test.ts pass (audit/runs/baseline/ui-test.txt — "src/screens/convert/" suite present).
— The reducer is pure/total with a bound (MAX_CONVERT_QUEUE_JOBS = 8) and clamp; no progress simulation exists in the reachable path.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/convert/ConvertScreen.tsx:153-182 (gate) + 299-310 (Start) + 235-243 (Browse)
Code:
```
    if (laneGate) return;
```
```
            disabled={laneGate}
            aria-describedby={laneGate ? "convert-lane-note" : undefined}
            data-wiring-stub={
              laneGate ? "conversion lane — no catalog command exposed (card 7.E3)" : undefined
            }
```
What is wrong: the gate can never open. The lane test is `listing.id.startsWith("convert.") || listing.id.startsWith("record.")` (convertQueue.ts:165-167), but the UI's own generated command union (ui/src/generated/commands.ts:24) contains no convert/record command, and the CLI host's `command_listings()` (apps/cli/src/host.rs:1665) serves none. The screen therefore ships a full conversion UI (source picker, target picker, job queue with progress/cancel chrome) whose every action is inert — permanently, in the desktop shell AND the browser demo. Why it matters: dead surface on a shipped route; the engine work it fronts (crates/transcode, real) is unreachable from the product; even the hypothetical lane-available branch (L169-181) enqueues a LOCAL job and dispatches nothing ("this is the single branch that changes" — a job would sit `queued` forever). What correct looks like: either land convert/record catalog commands end-to-end or pull the route until they exist. The honesty markers earn S3 not S2. Confidence: HIGH — Verification: read + rg (generated union, host listings, catalog).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useConvertMsg | 83-89 | REAL | - | lane i18n fallback |
| LaneState/STATUS_KEY consts | 91-106 | REAL | - | total status map |
| ConvertScreen (probe effect) | 122-142 | REAL | - | negative probe, cancellation-guarded, idempotent per transport |
| ConvertScreen (gate + form) | 144-312 | SHALLOW | S3 | F-001: gate permanently closed; Browse disabled; target dropdown local-only |
| startJob | 153-182 | UNREACHABLE-armed | S3 | gated; hypothetical branch dispatches nothing |
| cancelJob | 184-186 | UNREACHABLE | S3 | queue can never hold a job |
| queue render | 314-403 | REAL (rendering of impossible states) | S3 | progress/cancel chrome for jobs that cannot exist |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ConvertScreen.test.tsx:39 stubs "audio.set-volume" etc. — asserts the lane gate + disabled Start against a fake listing set | YES for the gating logic | it proves the disabled-from-data behavior, and correctly does NOT fake a lane |
| convertQueue.test.ts (pure reducer) | YES | lifecycle/totality pinned (baseline pass) |
| No test can exercise a real conversion — none is possible | n/a | the feature has no backend |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no capability shown that is not implemented" (acceptance criterion) | L21-22 | TRUE | Start/Browse disabled; targets note honest |
| "The UI NEVER simulates progress or completion" | L36-37 | TRUE | no progress source exists in reachable code |
| "the queue is driven by the ENGINE-BINDING SEAM below: the screen probes the command catalog" | L27-29 | MISLEADING | the probe is real but can only ever prove absence; "driven by" suggests a working lane |

## 7. Dead / duplicated / unreachable code
- startJob's enqueue branch, cancelJob, and the entire queue rendering (L335-401) are unreachable in every build (gate permanently closed).
- Browse button (L235-243): dead by stub, marked.

## 8. Dependencies & flags
No feature gates in UI. Depends on generated command types (no convert/record ids — the structural reason the gate can't open), Dropdown/Button/Card primitives, convertQueue pure model.

## 9. Open questions for the next agent
- Wave 23 audited crates/transcode: is the engine's WAV/MP4-remux path REAL and tested? If so, the gap is purely the command catalog (Lead-owned) — a one-line owner for a P1 feature gap.
