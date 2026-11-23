# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_11_interrupted_network.rs
| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-23 |
| Lines / functions | 292 / 2 helpers + 3 tests |
| File verdict | REAL (with honestly-disclosed v0 scope reduction) |
| Reachable from shipped entry point? | N/A (test) — module of `tests/scenarios_b/main.rs` (16-scenario binary); the harness drives the REAL production stack (host.rs:1060 `engine: Engine::new()`, real AVI demuxer + MJPEG/PCM decoders, command-layer dispatch per main.rs:36-40) with a scripted access source as the only fake |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/scenarios_b-bee4b59599013918 scenario_11` -> 3 passed, 0 failed, exit 0 (binary 19:26 newer than source 17:35) |

## 1. One-paragraph verdict
The "interrupted network" scenario row is tested against reality, and the first test pins the production truth the whole repo keeps admitting: real `https://` sources refuse with the typed `input/unsupported-for-host` error naming Phase 6 — there is NO network streaming in this build, and the test makes that un-fakeable. The two data-path arms drive the real engine/demux/decoders over a scripted local source: a mid-stream truncate (documented offset 212_512, the parent AVI's frame-30 chunk) ends CLEANLY with exactly one end-of-media and exactly one coalesced non-fatal damage event; a mid-read reset lands a typed FATAL (`input.inaccessible`) with a bounded state trail and a full close-reopen recovery. Fault arming is guarded by honest assertions (the demuxer provably sits before the cut; the park provably held playback mid-stream), and every wait is a hard pump bound that fails the test on breach. The scope reductions — no `Buffering → resume` rebuffer machinery, and a dual-from-state acceptance in the reset arm — are both disclosed in comments with owners named.

## 2. What actually works (verified)
- `./target/debug/deps/scenarios_b-bee4b59599013918 scenario_11` -> `test result: ok. 3 passed; 0 failed` (exit 0).
- Mutation honesty: if truncate leaked a fatal, `fatal.is_empty()` (204) fails; if damage coalescing broke (0 or 2+ events), `damage.len() == 1` (195-199) fails; if the cut did not bite (all 120 frames), `frames < 120` (216-218) fails; if the reset hung, `run_until_terminal(512)` returns non-Error state and the assert at 242-247 fails; if a poisoned engine survived the fatal, the reopen at 285 fails.
- The parked-open guards (95-105) prove the fault arming is real: `bytes < CUT` (the demuxer is genuinely mid-file) and `before_frames < 120` (playback genuinely held) — the test cannot accidentally test an already-drained stream.
- Harness verification: host.rs:1156 `run_until_terminal(bound)` is a hard for-loop bound returning `(spent, state)` — a hang is a bound breach that fails the terminal assert, never a silent pass.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_11_interrupted_network.rs:1-5 (module doc) vs 23-39 (v0 mapping)
Code:
```
//! Scenario 11 — **interrupted network source** (doc 10 §6.4 row
//! 11), driven through the command layer.
//!
//! Pass criteria: `Buffering` → resume or typed error; no hang
//! (deadlines fire).
```
What is wrong: the doc-10 pass criteria's "`Buffering` → resume" half is not exercised — the truncate arm maps "resume" to "bounded play-out to a clean end" (26-34) because a local-source cut is final and no rebuffer machinery exists. The mapping is disclosed three times in this file (7-12, 23-34, 41-45) with owners named (3.C3, 6.D2), so this is honest incompleteness, not fraud — but any feature-matrix row citing "scenario 11" as evidence of network-interruption handling is citing a typed refusal plus a local-file simulation. Cross-ref: parity-report already shows 0/252 verified (baseline xtask-parity-report.txt). Confidence: HIGH — Verification: read + ran.
Why it matters: the row's name promises a network path the binary does not contain. What correct looks like: the streaming crate's own scenario when Phase 6 lands (the file's own handoff note says exactly this).

F-002 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_11_interrupted_network.rs:256-274
Code:
```
    // The fatal error event is ordered BEFORE the state change to
    // Error (doc 05 §3.3) — the state trail ends with → Error. The
    // from-state is the buffering phase the reset happened to land
    // in (the 6.FIX-pcm queue-admission change shifted the episode
    // timing: the resumed session sits in a rebuffer when the failed
    // read hits — `Buffering → Error`; a reset landing mid-play
    // answers `Playing → Error`. Both are the D-4 typed fatal; the
    // contract is the terminal, not the phase it interrupted).
```
What is wrong: the assertion accepts `Playing → Error` OR `Buffering → Error` — a widened tolerance (the episode timing is nondeterministic with respect to which phase the reset interrupts). The comment documents the cause (a queue-admission change shifted timing) and the rationale (the contract is the terminal), and the test still pins the fatal-before-error ordering and the exact typed code. Acceptable, but a future regression that breaks the Playing→Error path specifically would pass this test. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open_scripted_parked | 72-106 | REAL | — | tight caps + immediate pause; honest arming guards (bytes < CUT, frames < 120) |
| open_scripted | 108-129 | REAL | — | settle to Playing (recovery-path helper) |
| url_sources_refuse_the_phase_named | 133-164 | REAL | — | pins the typed refusal + scheme detail + "Phase 6" message + no session + Idle (ran) |
| truncated_midstream_source_ends_cleanly_within_deadline | 172-224 | REAL | S4 | clean end, one EOM, one coalesced damage, cut-bit check, visible_len truth (ran) |
| reset_midstream_source_fails_typed_within_deadline | 230-292 | REAL | S4 | typed fatal input.inaccessible, bounded, dual-from-state acceptance (F-002), full recovery cycle (ran) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| url_sources_refuse_the_phase_named | YES | exact stage/code/scheme/message asserts; an Ok would hit unreachable! (ran) |
| truncated_midstream_source_ends_cleanly_within_deadline | YES | terminal == Ended, damage == 1, fatal empty, EOM == 1, frames in (before, 120), visible_len == CUT (ran) |
| reset_midstream_source_fails_typed_within_deadline | PARTIAL | pins fatal == 1, code, trail terminal, recovery; the from-state is deliberately dual-accepted (F-002) — a Playing-specific regression hides |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "interrupted network source ... driven through the command layer" (1-2) | module doc | PARTIAL | real URLs refuse typed (test 1, ran); data path is a scripted LOCAL source (disclosed at 13-21) |
| "truncate → ends CLEANLY ... exactly one end-of-media, exactly one NON-FATAL playback.error" (26-29) | module doc | REAL | asserts 186-209 (ran) |
| "reset → typed FATAL `Playing → Error` ... stage input, code inaccessible" (35-39) | module doc | REAL with tolerance | fatal code pinned; from-state dual (F-002) |
| "no hang: the terminal arrives within the pump bound" (30-31) | module doc | REAL | run_until_terminal(512) hard bound, assert on terminal |

## 7. Dead / duplicated / unreachable code
None in this file. (CUT = 212_512 duplicates the trunc3 derivative's offset — by design, documented at 58-61.)

## 8. Dependencies & flags
commands (Command/SourceSpec/Stage/ErrorCode), testkit (Must, serialize_tests — the binary-wide counting-allocator discipline), crate::host (ScenarioHost — real engine + real demux/decoders + scripted access source, host.rs:1060/1156). No env knobs, no #[ignore].

## 9. Open questions for the next agent
- host.rs (2308 loc, not assigned to me): the `script://` scheme's ScriptHandle (shrink_to/fail_next_read, host.rs:170/177) is the only fake in the stack — its faithfulness to "a stalled download" is asserted structurally (read_stats/visible_len) but the access-source semantics are host.rs's to prove.
- doc 10 §6.4 row 11 evidence in the feature matrix (docs/feature-matrix.toml) — if it cites this scenario, the citation is a typed-refusal + local simulation, not network handling (cross-ref Wave 2 parity findings).
