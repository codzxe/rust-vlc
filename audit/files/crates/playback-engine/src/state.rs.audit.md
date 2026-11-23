# AUDIT: crates/playback-engine/src/state.rs
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-23 |
| Lines / functions | 980 / 19 production callables (incl. the 33-row TRANSITIONS table) + 2 tests (exhaustive coverage lives in tests/table.rs) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — lumen-cli main -> run::run -> dispatch -> CliHost::settle/advance_time -> Session::tick_step (crates/playback-engine/src/session.rs:2428) -> `apply_input`/`translate_pipeline_signal`/`close_plan` (session.rs:85 imports) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |

## 1. One-paragraph verdict
The machine is real, pure, and exhaustively pinned: 33 data rows, a total typed
rejection function, a driver guard (`translate_pipeline_signal`) that provably
yields only table-legal inputs, and a 23-test suite (tests/table.rs, all green
in baseline) that classifies every 8×13 pair, proves every state reachable from
Idle, and pins the wire-edge projection. No unreachable states. The hostile
question's other half is where the bodies are: the `Ended` state is a near-trap
— seek, play-restart and stop from Ended are all typed rejections — the
deferral's own justification (restart "once seek exists", card 3.A1) has since
been satisfied yet the edges are still missing, and the media-clock
`seek_back` machinery built for exactly this (backends.rs) is never wired. Two
stale row-count comments contradict the pinned table, and the `Primed` input's
doc claim is contradicted by the pipeline's producer-parked escape.

## 2. What actually works (verified)
- Baseline (audit/runs/baseline/cargo-test-workspace.txt:4550-4578):
  `Running tests/table.rs ... test result: ok. 23 passed; 0 failed` — including
  `every_state_input_pair_is_classified_exactly_by_the_table` (104 pairs),
  `every_state_is_reachable_from_idle`, `translate_never_yields_an_illegal_input`,
  `the_engine_edge_projection_is_exactly_the_wire_edge_set`.
- Reachability: session.rs:85 imports `apply_input, close_plan,
  translate_pipeline_signal`; tick_step (session.rs:2428-2664) applies at most
  one translated input per poll (verified by reading the signal cascade at
  :2608-2621).
- Signal-priority coherence verified by reading pipeline.rs: `starved` is
  suppressed at EOF (`report.starved = !self.eof && starved`, pipeline.rs:2077)
  and `drained` is only computed in Playing|Paused (pipeline.rs:1957), so the
  underrun-then-EOF corner cannot wedge a session in Buffering.

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/src/state.rs:837-841 (and module doc :24-30)
Code:
```
        (S::Ended, I::Play) => R::RestartNeedsSeek,
        (S::Ended, I::Seek) => R::EndedNeedsWireEdge,
        (S::Ended, I::Pause | I::Stop | I::SwitchTrack) => R::AlreadySettled,
```
What is wrong: `Ended` is exited only by `Close`. Doc 03 §4.6 specifies an
`Ended | Seek | Buffering` row; VLC-parity expects play-restart and stop after
end. The recorded justification is stale: module doc :24-26 says restart rides
"the Phase-3 seek flow, card 3.A1 — doc 05 §2.4 deliberately wires
`playback.play` as 'restart-from-ended' only once seek exists", but card 3.A1
landed (the seven Seek rows are in this very table, `tests/seek_flow.rs`
green in baseline) and the ended edges still do not exist. The exact cure —
media-clock's `MediaClock::seek_back` (re-arms paused at the new position) —
is implemented, tested, and called by no engine code (see
audit/files/crates/media-clock/src/backends.rs.audit.md F-001). Why it
matters: a real session that finishes cannot be restarted or seeked without
close+reopen; `stop` after end answers "the work is already done". What
correct looks like: the doc 05 §3.3 `ended→buffering` wire edge + a
`Ended+Seek` row wired to `seek_back`, or an honest product doc that says
v0 sessions are single-shot. Confidence: HIGH — Verification: read + baseline
table tests + rg (zero seek_back callers).

F-002 [S3]
Where: crates/playback-engine/src/state.rs:196-199 vs crates/playback-engine/src/pipeline.rs:1992-2007
Code (state.rs):
```
    /// Every enabled limb's packet queue reached its high watermark:
    /// enough buffered data to start (doc 03 §4.6 `Buffering | high
    /// watermark | Playing`).
    Primed,
```
What is wrong: the pipeline raises Primed in a second way the input doc does
not admit: `if limb.queue.watermark().high || producer_parked` — a demuxer
parked on a full queue counts as primed even when other pacing limbs are below
the high watermark (the documented 44.1 kHz wav livelock escape,
pipeline.rs:1986-1991). The state machine's own vocabulary lies about when its
input fires; a reader reconciling doc 03's `high watermark` row with observed
`buffering 1.00 → Playing` transitions on a parked producer gets no hint from
this file. Confidence: HIGH — Verification: read both files.

F-003 [S4]
Where: crates/playback-engine/src/state.rs:368-370 vs :972
Code:
```
/// The minimal-machine transition table (30 rows; see the module
/// docs). Data, not code — the exhaustive tests classify every
/// (state, input) pair against it.
```
What is wrong: the table has 33 rows and the pinned test says so
(`assert_eq!(TRANSITIONS.len(), 33)`, :972; the test comment :967-971 counts
26 of 2.C2 + 4 of 3.C1 + 3 of 3.A3). The "(30 rows)" header is stale.
Confidence: HIGH — Verification: read + counted rows in TRANSITIONS (:371-665).

F-004 [S4]
Where: crates/playback-engine/src/state.rs:775-776
Code:
```
    /// table row or a typed rejection, and the exhaustive tests pin all
    /// `8 × 12 = 96` classifications.
```
What is wrong: `SessionInput::ALL` has 13 entries (:222-236), so the pinned
classifications are 8 × 13 = 104 (and tests/table.rs:106 iterates ALL × ALL).
Stale count from before 3.A3's `SwitchTrack`. Confidence: HIGH.

F-005 [S4]
Where: crates/playback-engine/src/state.rs:940-943
Code:
```
        // In Buffering the drain is preempted by the short-media
        // start (EofReached fires first); the playing/paused cases
        // above are the only ones the driver raises.
        (S::Buffering, PipelineSignal::Drained) => None,
```
What is wrong: the rationale names the wrong mechanism. The driver never
raises Drained in Buffering at all — pipeline.rs:1957 computes `drained` only
for `Playing | Paused` — so this arm is unreachable defensive data, not a
preemption bet. Harmless (dead arm, consistent), but the comment teaches a
false model of the guard. Confidence: HIGH — Verification: read
pipeline.rs:1954-1959.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SessionState (enum) | 86-107 | REAL | — | 8 states, all reachable (test-proven) |
| SessionState::ALL | 111-120 | REAL | — | wire order |
| SessionState::as_wire | 126-137 | REAL | — | identity mapping, pinned by test |
| Display for SessionState | 140-154 | REAL | — | wire names |
| SessionInput (enum) | 161-218 | REAL | — | 13 inputs, 7 commands + 6 signals |
| SessionInput::ALL | 222-236 | REAL | — | exhaustive-test input |
| SessionInput::is_command | 241-252 | REAL | — | |
| Display for SessionInput | 255-274 | REAL | — | |
| Transition (struct) | 288-298 | REAL | — | pure data |
| Effect (enum) | 303-366 | REAL | — | 14 effects; doc 03 side-effect column pinned by test |
| TRANSITIONS | 371-665 | REAL | S4 | 33 rows; header says 30 (F-003) |
| RejectionReason (enum) | 670-716 | REAL | S3 | Ended rejections (F-001) |
| Display for RejectionReason | 718-746 | REAL | — | actionable text |
| TransitionRejected + Display | 749-767 | REAL | — | typed, tested rendering |
| apply_input | 781-796 | REAL | — | pure lookup; only mutation path is the executor |
| rejection_reason | 802-874 | REAL | S4 | total, no wildcard (compile-forced); stale 96 count (F-004) |
| close_plan (+2 consts) | 886-897 | REAL | — | active states settle via Stop first; tested |
| PipelineSignal (enum) | 902-917 | REAL | — | raw pipeline facts |
| translate_pipeline_signal | 931-959 | REAL | S4 | guard; F-005 comment; priority honored in session.rs:2608-2621 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| every_state_input_pair_is_classified_exactly_by_the_table (tests/table.rs:102) | YES | 104-pair totality: every pair is exactly one row or one rejection |
| every_state_is_reachable_from_idle (table.rs:505) | YES | BFS over rows — answers the hostile question directly |
| translate_never_yields_an_illegal_input (table.rs:432) | YES | guards the driver guard |
| the_engine_edge_projection_is_exactly_the_wire_edge_set (table.rs:188) | YES | wire conformance of every row |
| the_close_plan_composes_wire_conformant_trails_only (table.rs:381) | YES | close never invents an edge |
| table_size_is_pinned (state.rs:966) | YES | row-count change is a deliberate act |
In-file tests are only 2; the muscle is in tests/table.rs (23 tests, all pass,
baseline :4550-4578). Mutation reasoning: breaking any row/reason flips a
classification assert.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "10-state sketch; card 2.C2 implements the minimal 8-state machine" | state.rs:5-14 | REAL | 8 states; doc 03's Probing/Demuxing folded into Opening |
| "doc 05 §2.4 wires playback.play as restart-from-ended only once seek exists" | state.rs:24-26 | STALE/FALSE-now | seek exists (3.A1 rows in table; tests/seek_flow.rs green in baseline) yet Ended+Play still rejected |
| "Primed = every enabled limb reached its high watermark" | state.rs:196-199 | FALSE (incomplete) | producer_parked escape, pipeline.rs:1999 |
| "30 rows" | state.rs:368 | FALSE | 33 rows, pinned :972 |
| "8 × 12 = 96 classifications" | state.rs:776 | FALSE | 13 inputs = 104; table.rs:106 iterates ALL |
| Baseline litmus "walks Idle→Opening→Buffering→Playing→Stopped→Idle in 19ms" | audit/runs/baseline/mp4-smoke.md:9-19 | REAL but weak | the walk never enters Paused/Ended/Error and decodes zero frames (v_frames=0, :38) — it is open+settle+teardown of undecodable h264/aac media, not playback |

## 7. Dead / duplicated / unreachable code
- `(S::Buffering, PipelineSignal::Drained) => None` arm: unreachable from the
  driver (pipeline never reports drained in Buffering) — defensive data (F-005).
- The final catch-all arms of `rejection_reason` (:862-872) return
  `SignalNotRaisedHere` for pairs that ARE table rows — unreachable via
  apply_input (consulted only on miss); the comment admits it. Acceptable
  totality device, semantically dead labels.

## 8. Dependencies & flags
Depends only on `commands::PlaybackState` (wire enum) — clean layering. No
features, no unsafe, no allocation on the transition path (`&'static` rows and
plans).

## 9. Open questions for the next agent
- session.rs: does the executor's pre-table branch handle `Open` on a live
  session by close+reopen (the SourceReplacementPhase3 rejection suggests the
  table refuses; tests/source_swap.rs and replacement.rs are green — check
  which path they exercise)?
- Who applies `Stop` in the litmus teardown — graceful_exit (apps/cli/src/
  run.rs) composing close_plan, or a direct Stop? The baseline trail
  Playing→Stopped→Idle implies the close_plan composition; confirm in run.rs's
  graceful_exit.
- The `EmitBufferingExit` ordering on Buffering+Stop (state.rs:479-483) puts
  the 1.0 bookend after teardown — does the events contract suite pin the
  event order (episode-close before state-changed)? (events_contract.rs is
  another agent's file.)
