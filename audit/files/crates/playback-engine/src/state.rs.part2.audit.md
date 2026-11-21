# AUDIT: crates/playback-engine/src/state.rs (part2, lines 491–980 of 980)
> Chunk scope: part2 = lines 491–980 (assignments.tsv LOC 490/part × 2 =
> 980). Items: the `TRANSITIONS` table's second half (491–665: the Playing
> rows — Pause/Underrun/Drained/Seek/SwitchTrack×3/Stop/Failed — the Paused
> rows — Play/Pause/Primed/Drained/Stop/Failed — and the settled close exits),
> `RejectionReason` + Display (670–748), `TransitionRejected` + Display
> (750–766), `apply_input` (781–800), `rejection_reason` (802–884),
> `close_plan` + the two plan constants (886–897), `PipelineSignal` (903–929),
> `translate_pipeline_signal` (931–959), and `mod tests` (961–980, 2 tests).
> Part1 (1–490: module docs, `SessionState`, `SessionInput`, `Transition`,
> `Effect`, and the table's first rows 371–490) is W03-A05's. The 33-row table
> straddles the boundary at 491; rows are audited by the part that contains
> them.

| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2026-11-21 |
| Lines / functions | 490 (of 980) / 12 production units + 2 in-file tests |
| File verdict | REAL (pure data table + pure functions; executed end-to-end by every baseline session lifecycle and pinned by an exhaustive 23-test conformance suite that ran green) |
| Reachable from shipped entry point? | YES, EXECUTED — `apply_input`/`translate_pipeline_signal`/`close_plan` are called by the session executor (session.rs apply/tick paths) on every command and pipeline signal; every baseline open/play/pause/stop transcript (INDEX.md AVI lifecycle `Playing → Stopped → Idle`, sample-420.mp4 `Buffering → Playing → Stopped → Idle`) drives these rows. Suite evidence: `tests/table.rs` ran 23/23 green in the no-fail-fast baseline (orchrun log line 4196-4222: "running 23 tests … test result: ok"); playback_engine lib 83/83 (lines 3740-3743). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `sed -n '1,120p;120,490p;480,560p;560,760p;760,980p;920,980p' state.rs` (full part2 read + part1 context); `rg -n "^pub (const\|fn\|struct\|enum)\|^impl\|^const…" state.rs` (item map — exact lines in the scope note); `rg -n "must_use" state.rs` + `sed -n '125p' | od -c` (attribute integrity: file contains `#[must_use]` — the `#ust_use]` in sed output is a terminal ESC-eating artifact, NOT a source defect); `rg -n "96\|8 × 12" state.rs` (stale doc arithmetic at 776); `rg -n "#\[test\]\|fn \|ALL\|96\|104" tests/table.rs` (exhaustive suite: 104 pairs asserted at table.rs:10,133); `sed -n '4196,4222p' orchrun log` (table.rs 23/23 green, incl. `translate_never_yields_an_illegal_input`); `sed -n '561,581p;770,780p' state.rs` (quote verification); cross-refs to session.rs:2157 (switch clock pause) and tracks.rs F-001 (this wave) |

## 1. One-paragraph verdict
The second half of the state machine is data of unusual discipline: 33 pinned
transition rows (count asserted in-file), a rejection-reason function that is
total **without a wildcard** (new inputs force a compile error), a driver
guard whose every `Some` answer is proven table-legal by an exhaustive
conformance suite that ran green in the no-fail-fast baseline, and zero
allocation by construction (`&'static` plans, table as a const). The
SwitchTrack rows corroborate the wave's one significant cross-file defect:
their effect sets carry `BumpPipeline`+`SwitchPipeline` and no
`ResumeClock` anywhere (561-581) — the audio-switch clock freeze
(session.rs:2157, documented inside `Effect::SwitchPipeline`'s doc) has no
paired unfreeze in the table either; that defect is filed and counted against
tracks.rs F-001 (complete_switch) in this wave, not re-counted here. The one
defect native to this range is trivial but real: `apply_input`'s doc still
says the exhaustive tests pin "`8 × 12 = 96` classifications" when
`SessionInput::ALL` has had 13 variants since card 3.A3 and the test itself
asserts 104 (`8 states x 13 inputs`, table.rs:133).

## 2. What actually works (verified)
— Row-effect discipline — verified per row in 491-665: every state-leaving
row carries the pipeline-invalidating/teardown pair the docs mandate
(`BumpPipeline`+`TeardownPipeline` for Stop/Failed, `BumpPipeline`+
`DestroySession` for the settled close exits, `FreezeClock`+`EmitBufferingEnter`
for underrun/seek entry, `StopClock`+`EmitEndOfMedia` for Drained,
`ResumeClock`+`EmitBufferingExit` for Primed/EofReached from Buffering).
— The seek family's asymmetries — verified: Paused+Seek re-runs the flow
without `FreezeClock` (already frozen — no double-freeze), Buffering+Seek
re-runs without a second `EmitBufferingEnter` (the episode is already open —
no orphaned 0.0 bracket).
— `rejection_reason` totality — verified: the match covers every
`(state, input)` pair with explicit arms and ends in row-listing arms (no
`_` wildcard), so adding a 14th input or 9th state breaks compilation —
exactly what the doc promises.
— `close_plan` — verified: active states compose `Stop` then `Close`
(both table rows, both wire-conformant edges), settled/opening states close
directly, `Idle`'s self-row answers the no-session close; `&'static` slices —
zero allocation.
— `translate_pipeline_signal` — verified row-by-row: every `Some` answer
(`Failed` from Buffering/Playing/Paused, `Drained` from Playing/Paused,
`EofReached`/`Primed` from Buffering, `Primed` from Paused (the buffered-
pause bookend), `Underrun` from Playing) is a table row for that state; the
`None` arms are the documented absorptions (Buffering+Drained preempted by
short-media EofReached; Playing/Paused+Starved-absorptions). The exhaustive
test `translate_never_yields_an_illegal_input` ran green (orchrun log).
— The transition-table conformance chain — EXECUTED: `tests/table.rs`
23/23 green in the no-fail-fast baseline (line 4196ff), including
`every_state_input_pair_is_classified_exactly_by_the_table` (104 pairs,
`assert_eq!(illegal, 104 - TRANSITIONS.len(), "8 states x 13 inputs")` at
table.rs:133), `every_legal_row_is_wire_conformant`, and
`the_close_plan_composes_wire_conformant_trails_only`.

## 3. Findings

### F-001 [S4]
Where: crates/playback-engine/src/state.rs:776 (doc on `apply_input`)
Code:
```rust
/// transition is unreachable by construction: every pair is either a
/// table row or a typed rejection, and the exhaustive tests pin all
/// `8 × 12 = 96` classifications.
```
What is wrong: stale arithmetic. `SessionInput::ALL` (state.rs:222-240,
part1) has **13** variants — `SwitchTrack` joined with card 3.A3 — and the
exhaustive test classifies **104** pairs, asserting exactly that
(`tests/table.rs:10` "104 pairs", `:133` `"8 states x 13 inputs"`). The doc
still counts the pre-3.A3 vocabulary.
Why it matters: this file's whole value proposition is that its claims are
machine-checked; a doc line that the machine has already outgrown is exactly
the drift the exhaustive tests exist to prevent (the test comment and this
doc now disagree with each other).
What correct looks like: "`8 × 13 = 104` classifications" (or drop the
arithmetic and cite the test).
Confidence: HIGH — Verification: read + test source cited (table.rs:10,133).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `TRANSITIONS` rows 491-665 (Playing/Paused/settled) | 491-665 | REAL, EXECUTED | — | 18 of the 33 rows; effect sets verified in §2. SwitchTrack rows (561-581) corroborate tracks.rs F-001 (no ResumeClock paired to the audio-switch freeze). |
| `RejectionReason` + Display | 670-748 | REAL | — | 10 typed reasons; Display texts are full sentences, no id/kind confusion (contrast tracks.rs F-003). |
| `TransitionRejected` + Display | 750-766 | REAL | — | "{input} is not applicable in {state} ({reason})". |
| `apply_input` | 781-800 | REAL, EXECUTED | S4 | F-001 doc arithmetic. O(33) linear find — trivial, called per input application, not per tick slice. |
| `rejection_reason` | 802-884 | REAL | — | Total without wildcard (compile-enforced exhaustiveness); row-arms return a placeholder `SignalNotRaisedHere` documented as never-consulted — a wrong-but-unreachable answer if a future bug consults it (noted, not a finding). |
| `close_plan` | 886-893 | REAL, EXECUTED | — | Zero-alloc `&'static` plans; composition verified by `the_close_plan_composes_wire_conformant_trails_only` (green). |
| `STOP_THEN_CLOSE` / `CLOSE_ONLY` | 895-897 | REAL | — | The two plan constants. |
| `PipelineSignal` | 903-929 | REAL | — | 5 raw facts, docs distinguish pipeline observation from session meaning. |
| `translate_pipeline_signal` | 931-959 | REAL, EXECUTED | — | Driver guard; every Some is table-legal (exhaustive test green); Paused+Starved → None (consumption frozen) — sound under the v0 pause model. |
| `table_size_is_pinned` (test) | 966-974 | REAL | — | Pins 33; adding a row is a deliberate act. EXECUTED green (lib suite). |
| `state_display_uses_wire_names` (test) | 976-980 | REAL | — | Wire-name pinning. EXECUTED green. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `table_size_is_pinned` (in-file, 966) | YES | Any row addition/removal trips the count. EXECUTED green. |
| `state_display_uses_wire_names` (in-file, 976) | YES | Display drift trips it. EXECUTED green. |
| tests/table.rs (23 tests, EXECUTED green — orchrun log 4196-4222) | YES (the suite's design) | `every_state_input_pair_is_classified_exactly_by_the_table` walks ALL 104 pairs against the table and asserts the illegal count; `rejection_reason_agrees_with_apply_input_everywhere` pins the reason function; `every_row_is_walked_from_idle_through_legal_inputs_only` pins executability. A mutation in any part2 row, reason arm, plan, or translate arm breaks at least one of these. (Bodies not individually audited — test file is outside my assignment — but their assertions were read at the grep level and the pass is cited.) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the exhaustive tests pin all `8 × 12 = 96` classifications" | state.rs:774-776 | **STALE/FALSE** | 13 inputs since 3.A3; test asserts 104 (table.rs:10,133) — F-001. |
| "an illegal transition is unreachable by construction" | state.rs:772-773 | REAL | apply_input is the only mover (session executor the only caller — traced in session.rs apply paths); exhaustive test green. |
| "The pipeline tick applies at most one translated input per poll" | translate doc 924-928 | REAL (cross-file) | Enforced in the session tick's translate-then-apply loop (session.rs tick paths; priority order is a `[PROPOSAL]`). |
| "every `Some` answer is a table row for `state`" | translate doc 928-930 | REAL | `translate_never_yields_an_illegal_input` green (orchrun log 4221-4222). |
| SwitchTrack rows: "the wire state never leaving `playing`" | state.rs:557-560 + rows 561-581 | REAL | Rows are Playing→Playing/Paused→Paused/Buffering→Buffering — but see tracks.rs F-001: the audio flavor's direct clock freeze (session.rs:2157, `Effect::SwitchPipeline` doc) has no paired resume in these rows or anywhere. |
| "zero-allocation by construction — the plans are `&'static`" | close_plan doc 879-885 | REAL | Const slices at 895-897. |

## 7. Dead / duplicated / unreachable code
- Nothing dead in part2. The `rejection_reason` row-arms (869-884) return a
  placeholder value documented as never-consulted — unreachable by design.
- Cross-file note: the SwitchTrack machinery the rows drive is unreachable
  from shipped hosts (both refuse `tracks.set-*` — see tracks.rs F-004);
  the rows themselves are exercised by the table suite regardless.

## 8. Dependencies & flags
- `commands::PlaybackState` (the wire form) is the only external dep —
  consistent with the crate manifest. No feature gates in part2. No unsafe.
- Attribute integrity verified at byte level: the `#[must_use]` attributes
  (lines 125/240/801/885/930) are intact in the file — `sed` output showing
  `#ust_use]` is a terminal escape-eating artifact (checked with `od -c`).

## 9. Open questions for the next agent
- The `Effect::SwitchPipeline` doc (part1, ~356-365) says "The audio kind
  additionally freezes the clock" — part1's auditor should cross-link
  tracks.rs F-001 so the freeze-without-resume defect is visible from both
  halves of the machine.
- If the Lead ever adds the `ended→buffering` wire edge (EndedNeedsWireEdge,
  state.rs:698-704), `rejection_reason` and the close-plan family need a
  coordinated update — the no-wildcard totality will force it at compile
  time, which is the design working as intended.
