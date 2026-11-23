# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_13_long_media.rs
| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-23 |
| Lines / functions | 127 / 1 test |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test) — module of `tests/scenarios_b/main.rs`; drives the real Engine over a scripted on-demand `timeline://` demuxer (host.rs:377-446 TimelineDemuxer, synthesized packets, bounded producer memory) on the counting-allocator binary |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/scenarios_b-bee4b59599013918 scenario_13` -> 1 passed, 0 failed, exit 0 (0.02s — virtual clock, no wall-clock waiting) |

## 1. One-paragraph verdict
The long-media scenario is a real bounded-soak with exact accounting: a 4-hour-equivalent audio timeline (14_400 one-second packets, synthesized on demand — producer memory bounded by construction, verified in host.rs:372-416) plays to `Ended` inside a hard pump bound (6_144), with every packet delivered (14_400/14_400 asserted), exactly one end-of-media, zero error events, the last frame's pts pinned to the exact final-packet value (4 h minus 1 s), clean feeding counters (no starve episodes, no outbox drops on either side), and a MEASURED memory plateau: the counting allocator's live bytes at the halfway point must be ≥ the reading at `Ended` — a zero-growth pin over the second half of a 4 h-equivalent play that fails on any per-packet or per-frame retention. All bounds are hard for-loops; a hang is a bound breach that fails the test. The scope reduction versus doc 10's row — audio-only single track, virtual clock, in-process allocator bytes with a non-retaining sink instead of the nightly RSS slope — is disclosed in the module doc, which itself says the 5 MB/h budget is "a nightly-tier proposal" and this pin is "the stronger zero-growth claim for the engine's steady state".

## 2. What actually works (verified)
- `./target/debug/deps/scenarios_b-bee4b59599013918 scenario_13` -> `test result: ok. 1 passed; 0 failed` (exit 0).
- Mutation honesty: a per-packet leak fails `end <= mid` (112-116); a dropped frame fails `frames == PACKETS` (83); a duplicate/missing EOM fails the count (88-92); a stall fails the pump bound via the terminal assert (76-81); a wrong final pts fails (100-104); outbox overflow fails the counter asserts (122-123).
- Producer boundedness verified by reading the timeline demuxer (host.rs:399-416): packets are synthesized per call (`from_static` payload), `remaining -= 1` guarded by the `remaining > 0` check at 433 — no underflow path.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_13_long_media.rs:6-21 (module doc) + 42-43 (test)
Code:
```
//! The doc 10 row names the 4 h synthetic stream (soak tier). This
//! suite's shape (per the card: "long = synthetically scripted long
//! timeline on virtual clock (NOT wall-clock waiting)"): a scripted
//! demux stage producing a **4-hour audio timeline on demand** —
```
What is wrong: coverage scope, disclosed but narrower than the row it answers. (a) The timeline is AUDIO-ONLY, single track — the doc-10 row's "long media" says nothing about audio-only; a long A/V media exercises per-video-frame queues, the presentation scheduler and (in the CLI host) the video hold ring — none of that is soaked here. (b) The memory evidence is in-process allocator live bytes with a sink that "retains nothing" (11-12) — the nightly RSS-slope soak tier the row's 5 MB/h budget belongs to does not exist in this repo (the doc says "nightly-tier proposal"), so any feature-matrix citation of row 13's memory criterion resolves to this narrower pin. Both halves are honestly labeled in-file. Confidence: HIGH — Verification: read + ran.
Why it matters: a reader of the row believes long-media memory behavior is proven generally; it is proven for one audio limb on a virtual clock. What correct looks like: an A/V variant of the timeline, or a doc-10 annotation naming the audio-only v0 shape.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| four_hour_timeline_ends_with_bounded_memory | 42-127 | REAL | S4 | the whole scenario (ran, passes); scope note F-001 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| four_hour_timeline_ends_with_bounded_memory | YES | exact packet accounting, EOM count, error emptiness, exact last pts, measured zero-growth plateau, clean counters, hard pump bounds (ran) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "14_400 packets of 1 s each ... the run takes ~2_000 pumps, not wall-clock time" (9-13) | module doc | REAL | ran in 0.02s; BOUND arithmetic documented at 35-38 |
| "live-bytes reading at the timeline's halfway point equals the reading at Ended" (16-17) | module doc | REAL (assert is `end <= mid` — at-or-below, the doc's "equals" is the plateau reading; decrease allowed as warm-up retirement, 108-110) | assert 112-116 |
| "§6.5's 5 MB/h budget is a nightly-tier proposal; this pins the stronger zero-growth claim" (19-21) | module doc | REAL | the nightly tier itself is absent from the repo (F-001b) |

## 7. Dead / duplicated / unreachable code
None. (The `timeline://` scheme's demuxer lives in host.rs — shared with scenario 14, not duplicated.)

## 8. Dependencies & flags
commands (Command/SourceSpec), testkit (Must, serialize_tests, counting allocator via crate::alloc_state), crate::host. No env knobs, no #[ignore].

## 9. Open questions for the next agent
- scenario_14_short_media (same TimelineDemuxer at small counts) is not my file — its audit should confirm the zero-packet edge (timeline_track(None) → UNKNOWN span) behaves per doc 03 §4.6's Ended-vs-Buffering race.
- Does any feature-matrix row cite scenario_13 for the 5 MB/h budget? If so the citation overstates (in-proc audio-only plateau ≠ RSS slope) — cross-ref Wave 2 parity findings (0/252 verified).
- A long A/V soak would also cover the CLI host's video-hold ring (pipeline_host.rs:1357-1380's KNOWN DEFECT comment says a longer media would fatal `ring_full` under a one-quantum-per-pump bound — this scenario's audio-only shape cannot catch that class).
