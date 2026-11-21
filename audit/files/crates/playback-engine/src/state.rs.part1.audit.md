# AUDIT: crates/playback-engine/src/state.rs#part1 (lines 1-490)
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-21 |
| Lines / functions | 490 (chunk of 981) / 10 items: SessionState (+ALL/as_wire/Display), SessionInput (+ALL/is_command/Display), Transition, Effect (13 variants), TRANSITIONS rows 1-12 of 33 |
| Chunk boundaries | part1 = 1-490, ending mid-table at the `Buffering + Failed` row (485-490); part2 (A08) = 491-980: remaining 21 rows, RejectionReason/TransitionRejected, apply_input/rejection_reason/close_plan, PipelineSignal/translate, in-file tests |
| File verdict (chunk) | REAL |
| Reachable from shipped entry point? | YES. `lumen` CLI → apps/cli/src/host.rs → playback-engine session executor → `apply_input`/`TRANSITIONS` (session.rs:176 maps SessionCommandKind to SessionInput; session.rs applies the table). Executed: tests/table.rs (23 passed, baseline orchrun log:4196+), tests/model_check.rs, contract suites — all ok in the no-fail-fast baseline (4534/7/49; none of the 7 failures is playback-engine's table lane). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | grep -c "Transition {" → 33 rows · Read commands/src/catalog/types.rs:290-339 (PlaybackState order + can_transition_to) — hand-checked all 12 chunk rows legal · Read tests/table.rs header + edge-projection test (188-210) · rg `with_backends\|seek_back\|mark_ended\|ClockBackends` repo-wide (production callers: NONE; only media-clock's own tests; apps/cli/src/pipeline_host.rs:1605/1630 uses audio_master/system_monotonic only) · sed -n quotes verified · baseline orchrun log: table.rs `test result: ok. 23 passed; 0 failed` |

## 1. One-paragraph verdict
The state machine is real, data-driven, and provably wire-conformant: 33 table
rows (12 in this chunk, each hand-checked against
`PlaybackState::can_transition_to`), an exhaustive 104-pair classification
suite that passed in the baseline, and an edge-projection proof that the
engine's edges are *exactly* the wire's 21 edges. The defects are two stale
doc-count claims ("30 rows", "strict subset") and one cross-crate dead seam
the module doc papers over: restart-from-ended is said to ride "3.B2's
ended-seek-back scope", but media-clock's entire 3.B2 surface (seek_back,
mark_ended, with_backends) has zero production callers — the shipped engine
builds only single-master clocks, so after `Ended` the only exit is `Close`,
and the clock-side machinery that would enable seek-back is dead code.

## 2. What actually works (verified)
— Wire mapping: `SessionState::as_wire` (126-137) is the identity; `SessionState::ALL` (111-120) order matches `commands::PlaybackState` declaration order exactly (Idle, Opening, Buffering, Playing, Paused, Stopped, Ended, Error — types.rs:293-310). Pinned by tests/table.rs `engine_states_map_one_to_one_onto_the_wire_states` (ok, baseline).
— All 12 chunk rows hand-verified legal against can_transition_to (types.rs:321-338): Idle→Opening (open), Idle→Idle (close, C8), Opening→{Buffering (open-ready), Error (failed), Idle (close), Stopped (stop)}, Buffering→{Playing (primed), Playing (eof-reached), Buffering (play, C8), Paused (pause), Stopped (stop), Error (failed)}. Plus the exhaustive suite: `every_legal_row_is_wire_conformant`, `every_state_input_pair_is_classified_exactly_by_the_table` (104 pairs: 33 legal / 71 typed rejections) — ok in baseline.
— Effect vocabulary is data-only; the executor (session.rs) interprets — verified the resume-path episode bookend the Buffering+Pause comment (459-461) defers to "the executor's episode discipline": session.rs:1703-1715 emits the closing 1.0 on `Paused --Play--> Playing` when an episode is open. The claim is REAL, not hand-waving.
— Baseline: tests/table.rs 23/23 ok (orchrun log 4196+); the exhaustive-model lane (model_check.rs) ran ok; none of the 7 workspace failures touches this table.

## 3. Findings

### F-001 [S3] — "riding 3.B2's ended-seek-back scope" points at dead code: restart-from-ended is impossible in the shipped engine and the clock-side enabler has zero production callers
Where: crates/playback-engine/src/state.rs:27-30
Code:
```rust
//! - `Seek` from `Ended` (doc 03 §4.6 has the row; the doc 05 §3.3
//!   conformance list has no `ended→buffering` edge — the
//!   reconciliation stays Lead-owned, riding 3.B2's ended-seek-back
//!   scope).
```
What is wrong: The module doc frames the Ended-seek gap as riding media-clock
card 3.B2's `seek_back` scope — and that scope exists
(media-clock/src/backends.rs:881 `seek_back`, with mark_ended, the Ended
latch, and a 7-test suite I audited in this wave) but **nothing calls it**.
Repo-wide rg for `with_backends|ClockBackends|on_master_lost|switch_master|
observe_video|mark_ended|seek_back|take_master_change`: production callers =
none; matches are media-clock's own tests (src/backends.rs test module,
tests/zero_alloc_backends.rs) and doc comments. The shipped CLI constructs
clocks only via `MediaClock::audio_master` (apps/cli/src/pipeline_host.rs:1605)
and `MediaClock::system_monotonic` (pipeline_host.rs:1630). Combined with the
table (no Ended+Play, no Ended+Seek rows; the only Ended exit is Close at
647-652, part 2), the user-visible behavior is: **after a media ends, the
session cannot be resumed or seeked — only closed**, and the ~1000-line 3.B2
multi-backend surface (master-loss demotion, video-master steering, Ended
latch, seek_back) is unreachable from both shipped entry points.
Why it matters: this is the exact "claim vs wiring" class the devil audit
hunts: doc language implies a capability path ("riding 3.B2's … scope") whose
entire engine-side half was never built. Dead code this large also rots: my
media-clock audits found the await-anchor latch bug precisely in this
unreachable surface.
What correct looks like: either the engine wires 3.B2 (an executor path that
maps Ended+Seek to `seek_back` + re-anchor) and the doc's claim becomes true,
or the doc says plainly "restart-from-ended is unsupported in v0; the 3.B2
clock surface is built but unwired" and the dead surface is flagged in
KNOWN-LIMITS.md.
Confidence: HIGH — Verification: read + rg (caller enumeration above);
baseline engine-info features (`first-vertical:avi-mjpeg-pcm`) contain no
backend-switching feature; UNVERIFIED: nothing (the absence of callers is
exhaustive over the repo).

### F-002 [S4] — stale row count: "30 rows" vs the actual 33
Where: crates/playback-engine/src/state.rs:368
Code:
```rust
/// The minimal-machine transition table (30 rows; see the module
/// docs). Data, not code — the exhaustive tests classify every
/// `(state, input)` pair against it.
```
What is wrong: `grep -c "Transition {"` counts 33 rows; the in-file test
`table_size_is_pinned` (state.rs:972) asserts `TRANSITIONS.len() == 33` and
its comment explains the arithmetic (26 of 2.C2 + 4 of 3.C1 + 3 of 3.A3 =
33). The doc comment still says 30 — it was not updated when 3.A3's three
SwitchTrack rows landed (30 = 26+4 exactly).
Why it matters: a reader reconciling doc against table wastes time or, worse,
trusts the comment over the code.
What correct looks like: "33 rows" (or drop the count and cite the pinned
test).
Confidence: HIGH — Verification: read + grep count + the pinned test.

### F-003 [S4] — "strict subset" claim is stale: the engine's edge projection is exactly the wire's edge set
Where: crates/playback-engine/src/state.rs:16-19
Code:
```rust
//! The table below is a strict **subset** of the doc 05 §3.3 wire
//! conformance list (`commands::PlaybackState::can_transition_to`) —
//! proven row-by-row by the exhaustive transition-table tests, so no
//! engine-side transition can ever surface on the wire as a
//! conformance failure.
```
What is wrong: tests/table.rs:188-210
(`the_engine_edge_projection_is_exactly_the_wire_edge_set`) asserts
`assert_eq!(engine_edges, wire_edges)` and `engine_edges.len() == 21` — the
engine's distinct (from,to) edges are **equal** to the wire conformance
list's, not a strict subset of it. The comment was true before card 3.C1
added the opening→stopped, buffering→stopped and buffering→paused edges (the
test's own note: "18 before 3.C1's …"), and nobody updated the module doc.
Why it matters: the doc understates the shipped invariant; stale "subset"
language invites an auditor to look for wire edges the engine cannot perform —
there are none.
What correct looks like: "exactly covers (and is proven row-by-row within)
the doc 05 §3.3 wire conformance list".
Confidence: HIGH — Verification: read (test body + module doc; both quoted).

## 4. Per-function table (chunk)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-79 | REAL | S3/S4 | F-001 (3.B2 dead-seam claim), F-003 (stale subset claim); the C8 and pipeline-guard claims verified via tests/table.rs + session.rs |
| enum SessionState | 85-107 | REAL | — | 8 states, matches wire enum 1:1 |
| SessionState::ALL | 109-120 | REAL | — | wire-catalog order verified vs types.rs:293-310 |
| SessionState::as_wire | 122-137 | REAL | — | identity mapping; exhaustively pinned by table.rs test |
| Display for SessionState | 140-154 | REAL | — | lowercase wire names; pinned by in-file test (part 2) |
| enum SessionInput | 160-218 | REAL | — | 7 commands + 6 pipeline signals; payload-beside-the-table doctrine documented |
| SessionInput::ALL | 220-236 | REAL | — | 13 inputs; 8×13=104 exhaustive-pair basis |
| SessionInput::is_command | 238-252 | REAL | — | matches the 7 command variants |
| Display for SessionInput | 255-274 | REAL | — | kebab names |
| struct Transition | 288-298 | REAL | — | pure data row |
| enum Effect (13 variants) | 303-366 | REAL | — | each variant documented with doc refs; executor interprets (session.rs verified for EmitBufferingEnter/Exit paths) |
| TRANSITIONS rows 1-12 | 371-490 | REAL | — | all 12 hand-verified wire-legal; effect sets match the row comments' doc citations; count claim S4 (F-002) |

## 5. Test honesty
No in-file tests in this chunk (tests at 961-980 are part 2's). The file's
proofs live in tests/table.rs (23 tests, all ok in baseline orchrun log):
- `every_state_input_pair_is_classified_exactly_by_the_table` — 104 pairs vs
  the table: would fail on any row add/remove/typo. YES it would fail if broken.
- `every_legal_row_is_wire_conformant` + edge-projection equality — would
  fail on any non-conformant row. YES.
- `table_size_is_pinned` (in-file, part 2) — catches accidental row changes.
- `self_transitions_are_the_idempotent_confirmations_and_seek_work_rows` —
  pins the C8 set exactly. YES.
The table is data; the tests treat it as the subject (not a mock of it) —
this is the correct orientation.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "strict subset of the doc 05 §3.3 wire conformance list" | state.rs:16-17 | FALSE (stale) | table.rs:188-210 asserts edge-set equality (21 edges) — F-003 |
| "proven row-by-row by the exhaustive transition-table tests" | state.rs:18-19 | REAL | table.rs tests ok in baseline |
| "The minimal-machine transition table (30 rows…)" | state.rs:368 | FALSE (stale count) | 33 rows; pinned by state.rs:972 — F-002 |
| "restart-from-ended … riding 3.B2's ended-seek-back scope" | state.rs:27-30 | FALSE (as a capability path) | 3.B2 surface has zero production callers (rg); apps/cli builds only single-master clocks — F-001 |
| "a play that resumes before priming must not strand a 0.0 with no closing 1.0" (executor discipline) | state.rs:459-461 | REAL | session.rs:1703-1715 emits the 1.0 on Paused→Playing when episode_open |
| "Illegal transitions are therefore not just rejected — they are unreachable through any other path" | state.rs:53-56 | REAL | single-writer executor + translate guard; `translate only yields table-legal inputs` proven by table.rs guard test (ok, baseline) |
| "Card 3.C1 completed the doc 03 §4.6 'any active | Stop' mandate" | state.rs:39-40 | REAL | Opening+Stop (426-431), Buffering+Stop (475-484), Buffering+Pause (462-467), Paused+Primed (619-624) rows present; `the_card_3c1_completion_rows_are_pinned` ok in baseline |

## 7. Dead / duplicated / unreachable code
None inside this chunk. Cross-chunk (F-001): the 3.B2 clock surface this
module's docs lean on is dead code from the shipped engine's perspective —
see media-clock/src/backends.rs audits (this wave) for the surface's own
state.

## 8. Dependencies & flags
Depends on `commands` (PlaybackState) only — no external crates, no
features, no unsafe. All code ships in the default CLI build.

## 9. Open questions for the next agent
- part 2 (A08): does `close_plan`/`translate_pipeline_signal` hold the same
  exhaustive-proof standard? (table.rs suggests yes — verify the in-file
  tests are not weaker than the integration suite.)
- Engine owner: decide F-001 — wire 3.B2's seek_back into the executor or
  mark the surface unwired. The current state (docs leaning on dead code) is
  the worst of both worlds.
- session.rs auditor: the executor's episode-discipline half
  (session.rs:1703-1715) is outside the table — confirm the contract suite
  pins the paused-mid-episode resume bookend (a stranded 0.0 would be an
  EpisodeUnclosedAtExit violation the events guard would catch).
