# AUDIT: crates/playback-engine/src/state.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 981 / 22 production items (3 enums + impls, `TRANSITIONS` 33-row const table, `Effect`, 5 pure fns) + 2 in-file tests |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | **YES** — `lumen <any transport command>` → `lumen_dispatch` → `Engine`/session executor, which imports and drives this table: crates/playback-engine/src/session.rs:83-85 (`use crate::state::{… apply_input, close_plan, translate_pipeline_signal …}`), applied at session.rs:1550 (`apply_input`), 1252/1874 (`close_plan`), 2623 (`translate_pipeline_signal`). The engine is constructed by apps/cli/src/host.rs:984 (`self.engine.open(config)`). |
| Findings | S0:0 S1:0 S2:1 S3:1 S4:3 |
| Commands run | rg for consumers (`apply_input`, `close_plan`, `translate_pipeline_signal` → session.rs); sed quote verification; `grep -c "Transition {"` → 34 (33 rows + 1 struct def occurrence at 288); cross-check of phase3-exit.md claims |

## 1. One-paragraph verdict

This is the real session state machine: 8 wire states, 13 inputs, a 33-row data table (`TRANSITIONS`), a total typed rejection function, a close-composition macro (`close_plan`), and a pipeline-signal guard (`translate_pipeline_signal`) — all pure, all consumed by the session executor, which is the single writer. The machine is honestly documented as the *minimal* 8-state subset of doc 03 §4.6's 10-state sketch (Probing/Demuxing folded into Opening), and every deliberate refusal carries a typed reason instead of a silent drop. The design is sound. The defects: two of those typed refusals fence off behavior users actually need — **seek-from-Ended and play-from-Ended are refused**, while the clock-side machinery built to enable exactly that (`MediaClock::seek_back`, card 3.B2 in media-clock/src/backends.rs) sits unreachable in another crate (see my backends.rs audit F-001): the two halves of the ended-restart feature were built and never joined. Plus stale self-descriptions (row count, classification count, a rejection reason that contradicts the engine's implemented replacement).

## 2. What actually works (verified)

- The executor wiring chain above (read hop by hop).
- W00 baseline proves the machine runs in a real scripted session: audit/runs/baseline/advance-test.txt shows the exact table edges firing — `Idle → Opening`, `Opening → Buffering`, `Buffering → Playing` (the `Primed` row), `Playing → Ended` (the `Drained` row), `Ended → Idle` (close at teardown). That is 5 distinct table rows observed executing on the shipped binary (UNVERIFIED by my own run — cited from W00 output).
- `rejection_reason` (802-874) is total without a wildcard — adding an input or state without classifying it is a compile error (862-872 comment). Good discipline.
- The exhaustive classification tests exist outside this file: `tests/table.rs`, `tests/model_check.rs`, `tests/model_walk.rs`, `tests/model/oracle.rs` + `tests/model/expected_table.toml` (a TOML oracle — other agents' assignments).

## 3. Findings

### F-001 [S2] — Seek-from-Ended and play-from-Ended are refused; the intended enabler (`MediaClock::seek_back`) was built in another crate and never wired

Where: crates/playback-engine/src/state.rs:837-841; refusal vocabulary at 687-705.

Code (837-841, verbatim):
```rust
        // Ended: retained for seek-back, but the wire edge is missing
        // (the doc 03 §4.6 row vs doc 05 §3.3's list — Lead flag).
        (S::Ended, I::Play) => R::RestartNeedsSeek,
        (S::Ended, I::Seek) => R::EndedNeedsWireEdge,
        (S::Ended, I::Pause | I::Stop | I::SwitchTrack) => R::AlreadySettled,
```

What is wrong: doc 03 §4.6 specifies the row `Ended | Seek | Buffering`; the code refuses it because doc 05 §3.3's wire-conformance list lacks the `ended→buffering` edge ("Lead-owned change"). Meanwhile card 3.B2 implemented the clock-side Ended latch + `seek_back` re-arm in crates/media-clock/src/backends.rs:869-912 specifically for "the play-from-ended restart" — and nothing in playback-engine constructs a backends clock (my backends.rs audit F-001: zero callers). The two halves of one feature shipped in two crates and never met; the user gets a typed refusal.

Why it matters: after any media ends, `lumen`'s `seek 30s` and `play` both fail; the realistic case (watch to end, rewind, watch again) is broken by a documentation reconciliation that nobody performed. Under §4 this is wrong behavior in realistic cases (the doc admits the gap, which keeps it out of S0/S1 fraud territory — the code never claims otherwise).

What correct looks like: add the `ended→buffering` wire edge to `commands::PlaybackState::can_transition_to`, add the `Ended + Seek → Buffering` row, drive `MediaClock::seek_back` (or the v0 reopen-seek) at the executor.

Confidence: HIGH — Verification: read (table + cross-crate grep); W00 transcript shows `Playing → Ended` reached in a real session.

### F-002 [S3] — `SourceReplacementPhase3` rejection text and the CLI's copy are both stale: the engine now implements replacement, the host still blocks it

Where: crates/playback-engine/src/state.rs:691-693; apps/cli/src/host.rs:896-901.

Code (state.rs:691-693, verbatim):
```rust
    /// `open` on a live session is source replacement (card 3.A2);
    /// open a new engine session after closing this one.
    SourceReplacementPhase3,
```

Code (apps/cli/src/host.rs:899-901, verbatim):
```rust
                ErrorCode::NotReady,
                "a session is already open — close it first (source replacement is card 3.A2)",
```

What is wrong: `Engine::open` (session.rs:1071-1073) handles a live session by calling `self.replace_live_session()?` — replacement R2-R8 is implemented in the engine (documented at session.rs:1040-1070, exercised by `tests/source_swap.rs` and `tests/replacement.rs`). But the CLI host refuses a second `open` before the engine ever sees it, citing "card 3.A2" as future work. The engine capability is unreachable from the shipped CLI, and both error strings tell the user a merged card is still pending.

Why it matters: misleading diagnostics at the exact seam a user hits; a shipped capability fenced off by a stale guard. (The phase3-exit gate lists "source replacement" as MET — true at engine level, false at CLI level; the two docs disagree with each other through this message.)

What correct looks like: the host forwards the open to the engine (which replaces), or the message names the real (host-side) reason.

Confidence: HIGH — Verification: read (both sites + `Engine::open`).

### F-003 [S4] — Stale row-count comment: "(30 rows" vs the pinned 33

Where: crates/playback-engine/src/state.rs:368-369 vs 966-973.

Code (368-369, verbatim):
```rust
/// The minimal-machine transition table (30 rows; see the module
/// docs). Data, not code — the exhaustive tests classify every
```

Code (966-973, verbatim):
```rust
        // 33 rows (26 of 2.C2 + 3.C1's four: Opening+Stop,
        // Buffering+Pause, Buffering+Stop, Paused+Primed + 3.A3's
        // three SwitchTrack work-carriers); adding one is a
        // deliberate act (worklog + doc 03 §4.6 reference in the
        // row's comment).
        assert_eq!(TRANSITIONS.len(), 33);
```

What is wrong: the doc comment says 30, the test pins 33, the file contains 33 rows. Someone added 3.A3's three rows and updated the test but not the comment. §3.4 stale comment.

Confidence: HIGH — Verification: read + grep count (34 `Transition {` occurrences = 33 rows + the struct definition).

### F-004 [S4] — Stale classification count: "8 × 12 = 96" with 13 inputs

Where: crates/playback-engine/src/state.rs:776 vs 222.

Code (774-776, verbatim):
```rust
/// transition is unreachable by construction: every pair is either a
/// table row or a typed rejection, and the exhaustive tests pin all
/// `8 × 12 = 96` classifications.
```

Code (222, verbatim):
```rust
    pub const ALL: [SessionInput; 13] = [
```

What is wrong: 8 states × 13 inputs = 104 classifications; the comment predates `SessionInput::SwitchTrack`. Whether the external exhaustive tests actually walk `ALL` × `ALL` is their business (tests/table.rs, another agent), but this file's own math is stale.

Confidence: HIGH — Verification: read (count of enum variants at 161-218 = 13).

### F-005 [S4] — The two in-file tests are trivia (table size, display names); all real verification is external

Where: crates/playback-engine/src/state.rs:965-980.

What is wrong: nothing dishonest — the tests pin the row count (a change-detector, which is their stated purpose) and two Display strings — but a reader of this file alone would see "exhaustive tests" promised three times in doc comments with only trivia in-file. The exhaustive suites exist (tests/table.rs, model_check.rs, model_walk.rs, oracle.rs — verified by directory listing) but live outside. Hygiene: move or duplicate the exhaustive classification here, or say where it lives (the module docs do not name the files).

Confidence: HIGH — Verification: read + `ls crates/playback-engine/tests/`.

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (design contract) | 1-79 | REAL-UNPROVEN | — | honest about minimal machine + refusals |
| `SessionState` (enum) | 86-107 | REAL-UNPROVEN | — | 8 wire states |
| `SessionState::ALL` | 111-120 | REAL-UNPROVEN | — | wire-catalog order |
| `SessionState::as_wire` | 126-137 | REAL-UNPROVEN | — | identity mapping, pinned by test elsewhere |
| `Display for SessionState` | 140-154 | REAL-UNPROVEN | — | wire names |
| `SessionInput` (enum) | 161-218 | REAL-UNPROVEN | — | 13 inputs; payloads ride beside (pure-data doctrine) |
| `SessionInput::ALL` | 222-236 | REAL-UNPROVEN | — | 13 (F-004 stale count in docs) |
| `SessionInput::is_command` | 241-252 | REAL-UNPROVEN | — | 7 command inputs |
| `Display for SessionInput` | 255-274 | REAL-UNPROVEN | — | wire names |
| `Transition` (struct) | 288-298 | REAL-UNPROVEN | — | from/input/to/effects |
| `Effect` (enum) | 303-366 | REAL-UNPROVEN | — | 14 effects, executor-interpreted |
| `TRANSITIONS` (const table) | 371-665 | REAL-UNPROVEN | — | 33 rows; seek rows 518-554; switch rows 555-584; close rows 646-664; F-003 comment |
| `RejectionReason` (enum) + Display | 670-746 | REAL-UNPROVEN | S3 | F-002 stale `SourceReplacementPhase3` text |
| `TransitionRejected` + Display | 749-767 | REAL-UNPROVEN | — | typed rejection |
| `apply_input` | 781-796 | REAL-UNPROVEN | — | linear scan over 33 rows — O(33), called per input; fine |
| `rejection_reason` | 802-874 | REAL-UNPROVEN | S2 | total, no wildcard; F-001 Ended refusals at 839-841 |
| `close_plan` + consts | 886-897 | REAL-UNPROVEN | — | stop-then-close for active states; static slices |
| `PipelineSignal` (enum) | 902-917 | REAL-UNPROVEN | — | 5 raw facts |
| `translate_pipeline_signal` | 931-959 | REAL-UNPROVEN | — | driver guard; priority order is executor-side (doc §925); Paused+Starved absorbed (correct) |
| tests: `table_size_is_pinned`, `state_display_uses_wire_names` | 965-980 | SHALLOW | S4 | F-005 change-detectors only |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `table_size_is_pinned` (965-973) | Only for added/removed rows | change-detector by design; a wrong *row content* passes |
| `state_display_uses_wire_names` (975-979) | Only for renamed states | string equality on 2 of 8 names |

The real suites are external: tests/table.rs, tests/model_check.rs, tests/model_walk.rs, tests/model/oracle.rs + expected_table.toml — not in my assignment; their quality is another agent's call. From names alone they promise exhaustive classification and a TOML oracle (UNVERIFIED).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| doc 03 §4.6's 10-state `SessionState` (with `Probing`, `Demuxing`) | docs/03-engine-plan.md §4.6 (referenced at state.rs:7-14) | PAPER-ONLY (in this crate) | the code implements the 8-state minimal machine and says so |
| "The table below is a strict subset of the doc 05 §3.3 wire conformance list … proven row-by-row" | state.rs:16-19 | REAL-UNPROVEN | proof lives in tests/table.rs + oracle (not executed by me) |
| "Card 3.C1 completed … `Stop` during `Opening`/`Buffering` and `Pause` during `Buffering` are rows" | state.rs:39-41 | REAL | rows at 426-431, 462-467, 475-484 verified by reading |
| "seek (v0: reopen at the target through the engine's real transitions)" — CLI help for `seek` | apps/cli/src/help.rs (W00 baseline lumen-help.txt) | cross-file — see my seek.rs audit | the table's `Seek` rows exist; the "reopen" wording is the CLI's |
| phase3-exit "source replacement … MET" | docs/gates/phase3-exit.md:31 | REAL at engine level / FALSE at CLI level | `Engine::open` replaces (session.rs:1071-1073); CLI host blocks it (host.rs:896-901) — F-002 |

## 7. Dead / duplicated / unreachable code

None in this file — every item is consumed by session.rs (verified by grep: `apply_input`, `close_plan`, `translate_pipeline_signal`, `TRANSITIONS`, `SessionState`, `SessionInput`, `Effect`, `PipelineSignal`, `RejectionReason` all appear in session.rs/engine-facing code or tests).

## 8. Dependencies & flags

Depends only on `commands::PlaybackState` (the wire vocabulary). No feature gates. Always compiled into playback-engine.

## 9. Open questions for the next agent

- Does `commands::PlaybackState::can_transition_to` (the wire conformance list) really lack `ended→buffering`? That crate is wave-2 territory; the F-001 fix is blocked on it.
- tests/model/expected_table.toml — is the oracle generated from the table (circular proof) or hand-written? tests/ audit (wave 16).
- The executor's signal priority (`Fatal > Drained > …`, state.rs:924-926 `[PROPOSAL]`) is enforced in session.rs's tick — verify the at-most-one-input-per-poll claim there (session.rs:2623 area, another agent's file).
