# AUDIT: apps/cli/tests/session_script.rs
| Field | Value |
|---|---|
| Auditor | W13-A06 |
| Date | 2026-11-24 |
| Lines / functions | 538 / 3 helpers + 21 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test-only; runs via `cargo test -p lumen-cli`, spawns the SHIPPED `lumen` binary through CARGO_BIN_EXE_lumen — the tested surface IS the shipped entry point) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `./target/debug/deps/session_script-871be5754d608fa7 --test-threads=1` -> `21 passed; 0 failed` exit 0, 4.45s; sed quote verification |

## 1. One-paragraph verdict
This is what an acceptance suite should look like: 21 tests drive the REAL
binary (subprocess, piped stdio — the scripted-CLI surface), assert ordered
state-machine trails, exact landing times (266 667 ns vs the requested 250 ms),
generation/epoch continuity (one open, gen=1), teardown ordering, typed
refusals, and byte-identical determinism. I executed all 21: pass in 4.45s.
No tautologies, no mocks, no sleeps. The only debt is documentation rot in the
two rewritten 3.C1 machine-mode tests (names and doc comments describe a
refusal+settle+re-drive flow the bodies explicitly assert no longer exists)
and one copy-paste duplicate assertion.

## 2. What actually works (verified)
— verified by `./target/debug/deps/session_script-871be5754d608fa7 --test-threads=1` ->
`test result: ok. 21 passed; 0 failed; 0 ignored` (full list: scripted_session,
deterministic, runs_to_end, start_time, start_paused, pause_freezes, toggle,
close/chapter refusals, seek from playing/ended/backward-below-start-time,
4 teardown tests, 4 machine-mode tests). Determinism test compares two full
(stdout, stderr, exit) tuples — any wall-clock leak would fail it.

## 3. Findings
F-001 [S4]
Where: apps/cli/tests/session_script.rs:449-455 vs 458-462
Code (doc comment):
```
/// session). The assertions below are deliberately unchanged from
/// the pre-re-drive pin.
```
vs body comment:
```
    // Post-3.C1 the Buffering+Close composition is a table row (the
    // executor macro): EOF during Buffering tears down DIRECTLY — no
    // round-0 refusal, no settle window needed.
```
What is wrong: the doc comment claims the assertions are "deliberately
unchanged from the pre-re-drive pin" while the body says the pinned behavior
was REPLACED post-3.C1 (no refusal at all). Self-contradiction inside one
test; the test now asserts only exit 0 + absence of a refusal string.
Confidence: HIGH — Verification: read + ran.

F-002 [S4]
Where: apps/cli/tests/session_script.rs:488-493
Code:
```
fn machine_mode_eof_transient_refusal_settles_then_tears_down() {
    // Post-3.C1: the close composition is a table row, so the EOF
    // teardown from Buffering lands DIRECTLY (the settle/re-drive
```
What is wrong: the NAME (and its doc block, 479-486, describing "round-0
refusal", "settle window", "second stop → close drive") promises a two-drive
transient-refusal scenario; the body asserts NO refusal and a one-drive
teardown (:501-503 `!err.contains("stop-or-close from Buffering")`). The name
lies about what is pinned — a reader auditing refusal handling is misled.
What correct looks like: rename (e.g. machine_mode_eof_tears_down_directly)
and delete the stale doc block. Confidence: HIGH — Verification: read + ran.

F-003 [S4]
Where: apps/cli/tests/session_script.rs:68-71
Code:
```
    // play is an idempotent self-confirmation while playing (C8).
    // pause lands: Playing → Paused.
    let paused_at = out.find("state-changed Playing → Paused").must();
    assert!(out.contains("ok: accepted session=1"), "{out}");
```
What is wrong: duplicate assertion — "ok: accepted session=1" is already
asserted at :63; the comment promises a play/pause check but re-asserts the
open receipt. The real pause pin is the `must()` find above it. Copy-paste
residue. Confidence: HIGH — Verification: read.

## 4. Per-function table (test inventory)
| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| run / parent helpers | 19-46 | REAL | — | subprocess + fixture path (exists: tests/fixtures/generated/valid/avi_mjpeg_pcm.avi) |
| scripted_session_open_play_pause_seek_stop | 55-131 | REAL, honest | S4 | full trail + exact landings; F-003 dup assert |
| scripted_session_is_deterministic | 136-141 | REAL, honest | — | byte-identical twin runs |
| scripted_session_runs_to_end | 147-155 | REAL, honest | — | end-of-media + exact 2.000000s |
| start_time_opens_at_the_target | 161-169 | REAL | — | position-only pin (content correctness follows from AVI's per-packet stamps; would NOT catch an ogg-style stamping gap) |
| start_paused_opens_then_pauses | 173-179 | REAL | — | |
| pause_freezes_the_position | 183-189 | REAL | — | advance-while-paused moves nothing |
| toggle_flips_play_pause | 193-200 | REAL | — | |
| close_then_snapshot_is_not_ready | 207-214 | REAL | — | typed not-ready |
| chapter_seek_refuses_typed | 218-224 | REAL | — | |
| seek_from_playing_rebuffers_in_pipeline | 236-267 | REAL, honest | — | rebuffer episode + one-open + gen=1 |
| seek_reports_the_actual_not_the_requested | 274-290 | REAL, honest | — | 266667 landing vs 250ms ask — the best pin in the file |
| seek_from_ended_refuses_typed | 297-310 | REAL, honest | — | no fabricated terminal asserted |
| backward_seek_below_start_time_lands | 317-334 | REAL, honest | — | pins the reanchor watermark clear (SelectDemuxer) |
| quit_during_play_tears_the_session_down | 348-362 | REAL, honest | — | causal order + transcript tail |
| eof_during_play_tears_the_session_down | 367-376 | REAL | — | |
| quit_while_paused_still_tears_down | 382-389 | REAL | — | |
| quit_after_close_is_a_no_op | 395-409 | REAL, honest | — | idempotence counts |
| machine_mode_eof_during_play_tears_down | 419-439 | REAL, honest | — | NDJSON trail, last line pinned |
| machine_mode_eof_refusal_is_bounded_and_honest | 457-477 | REAL, weak | S4 | F-001; 2 asserts only |
| machine_mode_eof_transient_refusal_settles_then_tears_down | 488-512 | REAL | S4 | F-002 name lies |
| machine_mode_malformed_request_still_tears_down | 520-538 | REAL, honest | — | exit 64 + teardown-before-exit pinned |

## 5. Test honesty
All 21 would fail if the corresponding engine/runner behavior broke (state
strings, orderings, exact times, counts are pinned). Mutation check: breaking
native seek (reopen instead of in-pipeline) fails the one-open count asserts;
breaking teardown ordering fails the find-order asserts; breaking determinism
fails the twin-run equality. The two 3.C1-rewritten machine-mode tests are the
weak pair (F-001/F-002) but still pin exit codes and teardown tails.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "scripted CLI session passes (open fixture → play →" + "pause → seek → stop) incl. the unsupported-file error path" | :2-3 (quoting doc 13) | REAL | my run: 21/21 pass |
| "so two runs must be byte-identical" + "(pinned by test)." | :9-10 | REAL | deterministic test passed |
| teardown-refusal scenario pins "round-0 refusal ... settle window ... second drive" | :441-455, 479-486 | FALSE (stale) | bodies assert NO refusal post-3.C1 (F-001/F-002) |

## 7. Dead / duplicated / unreachable code
The duplicated assert at :71 (F-003). No dead tests; all 21 run (verified).

## 8. Dependencies & flags
testkit::Must, std::process. No features. Uses only the AVI MJPEG+PCM
first-vertical fixture — other containers are covered by sibling suites
(container_conformance.rs, bench_decode.rs — not this file's scope).

## 9. Open questions for the next agent
- Does any suite pin the machine-mode round-0 refusal families that STILL
  exist (the body comment names "transient stop refusals")? If none, the
  runner's settle/re-drive machinery is untested residue.
