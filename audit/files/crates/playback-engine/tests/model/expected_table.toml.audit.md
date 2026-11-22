# AUDIT: crates/playback-engine/tests/model/expected_table.toml

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 606 / 104 `[[pair]]` entries (33 legal, 71 typed rejections) |
| File verdict | REAL — the human-review transition artifact; counts verified (8 states × 13 inputs = 104 exactly), spot-checked rows agree with `state.rs::TRANSITIONS` (33 rows pinned by state.rs:972) |
| Reachable from shipped entry point? | N/A (test artifact, loaded by `tests/model_check.rs` — another agent's file — for the three-way agreement proof) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read/Grep/awk counts (verified with unfiltered tools) |

## 1. One-paragraph verdict

A data artifact, not code: one TOML entry per (state, input) pair of
the session state machine — 104 entries exactly, as the header claims
(counted: 104 `[[pair]]`, 33 `to` rows, 71 `reject` rows, matching
`state.rs`'s pinned `TRANSITIONS.len() == 33`). Spot-checked rows
(idle+open, playing+pause/stop/seek/switch-track, the
`paused + primed` bookend, the `settle-first` close-from-active trio)
agree with the transition table excerpts verified in the pipeline.rs
audit. The rejections are typed with kebab reasons, and the
pipeline-only signals (`primed`/`underrun`/`eof-reached`/`drained`/
`failed`) are correctly `signal-not-raised-here` in states where
`translate_pipeline_signal` absorbs them (state.rs:937-958) — the
artifact encodes the same guard the code implements.

## 2. What actually works (verified)

— Counts (rg): `[[pair]]` = 104, legal `to =` = 33, `reject =` = 71 —
matching "8 states x 13 inputs = 104 entries, exactly" (header :2-3)
and `TRANSITIONS.len() == 33` (state.rs:967-972).
— Cross-checked rows against the code:
  - `playing + pause → paused, ["freeze-clock"]` — matches the
    transition read in the pipeline audit (state.rs:502-504 family).
  - `playing + seek → buffering, ["freeze-clock", "bump-pipeline",
    "seek-pipeline", "emit-buffering-enter"]` — matches seek.rs's
    seven-step flow and the Underrun row (state.rs:506-511).
  - `paused + primed → paused, ["emit-buffering-exit"]` — the 3.C1
    buffered-pause bookend, matching state.rs:950.
  - `playing + close → reject "settle-first"` — the header's own note
    (:35-37) that the wire has no active→idle edge and the executor
    composes stop-then-close (`close_plan`, session.rs:1245-1258).
— The consumer exists: `tests/model_check.rs` loads this file
(`#[path = "model/oracle.rs"] mod oracle;`, model_check.rs:31-34) and
proves the three-way agreement per its docs.

## 3. Findings

### F-001 [S4] Header carries a stale historical row count next to the current one
Where: expected_table.toml:31-32 vs the counted 33/71

Code:
```
# Row count after card 3.C1: 30 legal rows, 66 typed rejections.
# Card 3.C1's four new rows (vs the 2.C2 baseline of 26/70):
```
What is wrong: the file now contains 33 legal and 71 rejection rows
(3.A3's three work-carriers landed after that sentence was written);
the header's "Row count after card 3.C1" is historical, and the
current count is only derivable by counting. A reviewer reading top to
bottom sees two different numbers (104 total, 30+66=96) with no
current breakdown.
What correct looks like: "Row count now: 33 legal, 71 rejections
(3.C1's four + 3.A3's three work-carriers over the 2.C2 baseline)".
Confidence: HIGH — Verification: counted with rg (104/33/71).

## 4. Per-function table (per-state sections)

| Section | Lines (approx) | Verdict | Notes |
|---|---|---|---|
| header/grammar | 1-38 | REAL | S4 | stale historical count (F-001) |
| idle (13 entries) | 39-107 | REAL | no-session/signal rejections; open→opening; close→idle |
| opening (13) | 108+ | REAL | source-replacement-phase3 refusal; open-ready→buffering |
| buffering (13) | — | REAL | pause/stop rows (3.C1); primed→playing; eof-reached→playing (short media) |
| playing (13) | spot-checked | REAL | pause/stop/seek/switch rows verified against code |
| paused (13) | spot-checked | REAL | primed self-row (bookend); drained→ended |
| stopped / error / ended (39) | — | REAL | terminal-state rejections consistent with translate's `_ => None` |

## 5. Test honesty

The artifact IS the expected data; its honesty is the three-way
agreement test (model_check.rs — another agent) and my model_walk.rs
audit (next file). No circularity found: the table is hand-reviewable
TOML, not generated from the code it is compared against (the header
says parsed by a hand-rolled std-only reader, :14-16).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "8 states x 13 inputs = 104 entries, exactly" | :2-3 | REAL | counted 104 |
| "the close-from-active trio … typed `settle-first`" | :35-37 | REAL | playing+close row + session.rs close_plan |
| "Row count after card 3.C1: 30 legal rows, 66 typed rejections" | :31 | STALE | now 33/71 (F-001) |

## 7. Dead / duplicated / unreachable code

None (data artifact).

## 8. Dependencies & flags

None (TOML parsed by the test's own reader — C5 zero-dep contract).

## 9. Open questions for the next agent

- Whoever holds `tests/model_check.rs` and `tests/model/oracle.rs`:
  verify the agreement test actually iterates all 104 pairs (not a
  subset) and that the oracle cites doc 03 §4.6 per row.
