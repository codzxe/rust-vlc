# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_13_long_media.rs

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-22 |
| Lines / functions | 127 / 1 `#[test]` |
| File verdict | **REAL** — a 4-hour-equivalent virtual-clock soak with exact accounting and a measured memory plateau |
| Reachable from shipped entry point? | Subject: the real engine/pipeline/queue machinery; the demuxer is the suite's *named* scripted fake (`TimelineDemuxer`, host.rs:53-57) generating a 14 400×1 s audio timeline on demand — the fakeness is documented in the header (7-13) and sanctioned by the card ("long = synthetically scripted long timeline on virtual clock"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read of the full file; cross-check of TimelineDemuxer's naming in scenarios_b/host.rs. No cargo (static). |

## 1. One-paragraph verdict

An honest long-media scenario: 14 400 one-second packets (exactly 4 h) driven on the virtual clock in ~2 000 pumps with an explicit hang bound (6 144, with the arithmetic shown at 35-37), asserting every packet decoded and delivered (83), exactly one end-of-media (88-92), zero error events (93-97), the last delivered pts equal to the final packet's start (100-104), zero starvation episodes and zero outbox overflows (120-123), and the strongest check — a measured memory plateau: the counting allocator's live bytes at the end of the second 2-hour-equivalent half must be at or below the halfway snapshot (111-116), with the direction of the inequality justified in a comment (growth is the failure mode). The fake demuxer is not smuggled: the header says what it is and why.

## 2. What was checked

- The plateau assertion's polarity: `end <= mid` (113) — correct; a decrease is explained (warm-up retirement), growth fails.
- The halfway predicate (`audio_accepted() >= half` inside `pump_pipeline_until_within`, 67-72) decouples the measurement point from pump-count assumptions.
- Exact-value asserts throughout (14 400 delivered; pts = 14 399 s in ns at 100-104 — the u64 conversion is explicit).
- The header honestly downgrades the relationship to the nightly tier: "§6.5's 5 MB/h budget is a nightly-tier proposal; this pins the stronger zero-growth claim for the engine's steady state on a virtual clock" (18-21) — no claim inflation.
- Starvation/outbox counters come from the host's real engine stats (120-123).
- Audio-only timeline: no A/V-sync claim is made (scenario 07 owns sync) — no overclaiming.

## 3. Findings

None. (Checked: mock-of-the-subject — the demuxer fake is named and scenario-appropriate; the engine/queue/scheduler under test are real. Tautologies — none. Bound-hangs — prevented by design.)

## 4. Per-function table

| Item | Lines | Verdict | Notes |
|---|---|---|---|
| four_hour_timeline_ends_with_bounded_memory | 43-127 | REAL | §1; accounting + plateau + counters triple |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| four_hour_timeline_ends_with_bounded_memory | YES | delivery count, EoM uniqueness, pts truth, memory-growth direction, starvation and overflow counters — six independent failure classes |

UNVERIFIED by run (no cargo); W00 baseline consistency applies.

## 6-9

- Doc claim "memory growth within budget" (4): the body pins the *stronger* zero-growth claim and says so — honest.
- No dead code.
- Open question for wave 16: the nightly-tier RSS-slope soak (5 MB/h) does not exist in this repo — the scenario says it is the "in-proc stand-in"; the nightly tier itself is PAPER-ONLY unless another file owns it (none found by this agent — flag for the claim ledger).
