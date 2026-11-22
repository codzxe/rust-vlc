AUDIT: apps/cli/tests/session_script.rs
=======================================

| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-17 |
| Lines / functions | 538 / 23 (21 tests + 2 helpers) |
| File verdict | REAL (the CLI's strongest evidence file; two hygiene defects) |
| Reachable from shipped entry point? | NO — test-only; drives the REAL shipped binary (`CARGO_BIN_EXE_lumen`, line 29) over piped stdio, which is exactly the surface doc 05 §5.3 defines. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |

Commands run (static; no cargo per adaptations):
- Read file complete (single pass)
- `sed -n '449,473p;279,290p' apps/cli/tests/session_script.rs` (quote verification, passed)
- `ls tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` → EXISTS (the single fixture every test uses is committed)
- W00 baseline cross-refs: `mp4-smoke3-stdout.txt` (same transcript shapes: `ok: accepted session=1 gen=0`, `ok: time 0.000000s (0.0%)`), `advance-test.txt` (`advance` driving Ended/100% — the same behavior `scripted_session_runs_to_end` pins).

## 1. One-paragraph verdict

This is the file that proves the CLI's session machinery is real: 21 tests spawn the actual `lumen` binary, feed it scripted stdin, and assert exact transcript lines — state trails, seek terminals with µs-exact landings, no-reopen counts, teardown ordering, machine-mode NDJSON shapes, and byte-identical determinism across two runs. The assertions are precise enough to fail on any regression they describe (e.g. `Timestamp(266667)` landings, "one open only" counts, transcript-suffix checks). Two defects: one test's outer doc-comment claims its assertions are "deliberately unchanged" when the body pins the opposite (post-3.C1) behavior — a lie in the comment layer; and the file's own pins document a 1 µs inconsistency between the seek terminal (rounded) and the time snapshot (truncated) for the same landing. Note also what this file codifies: the frozen-clock poll model ("pipeline-only settles never advance media time; only `advance` does") is pinned as ACCEPTANCE behavior here — the suite proves the model, and the model is the run.rs S0.

## 2. What actually works (verified)

— verified by reading all 21 tests + W00 baseline transcripts: `scripted_session_open_play_pause_seek_stop` (54-131) asserts the full AC chain with ordering (`info_at > paused_at`, `stop_at > seek_at`), the seek terminal, exactly-one-open, and no session-2; `scripted_session_is_deterministic` (135-141) runs the binary twice and asserts equality; `scripted_session_runs_to_end` (146-155) pins `advance 512` → `Playing → Ended`, `2.000000s (100.0%)` — matching W00's advance-test.txt behavior on the same class of fixture; the native-seek trio (236-334) pins in-pipeline rebuffers, actual-vs-requested landings, typed Ended refusals with no fabricated terminal, and backward-seek-below-start-time; the M17 teardown sextet (348-538) pins stop→close ordering, transcript-suffix closure, paused-quit, idempotent quit-after-close, machine-mode NDJSON teardown, and the malformed-request teardown-before-exit-64.
— UNVERIFIED at runtime here (no cargo); the W00 baseline's identical transcript shapes (mp4-smoke3, advance-test) corroborate the formats.

## 3. Findings

### F-001 [S3] — stale outer comment contradicts the test body it describes ("assertions deliberately unchanged" — they are changed)
Where: apps/cli/tests/session_script.rs:449-456 (comment) vs 469-473 (body)
Code:
```rust
/// the session). The assertions below are deliberately unchanged from
/// the pre-re-drive pin.
#[test]
fn machine_mode_eof_refusal_is_bounded_and_honest() {
```
```rust
    assert!(
        !err.contains("[session.not-ready]"),
        "3.C1 landed the close composition — no refusal expected: {err}"
    );
```
What is wrong: the outer doc-comment (441-455) still narrates the pre-3.C1 world — "the engine's v1 table refuses the close from the transient state … The assertions below are deliberately unchanged from the pre-re-drive pin" — but the body asserts the ABSENCE of that refusal (`!err.contains("[session.not-ready]")`), i.e. the assertions were changed when 3.C1 landed and the outer comment was left behind contradicting them. A reader trusting the outer comment believes this test still pins a round-0 refusal.
Why it matters: comment-layer falsehood in an evidence file; anyone diffing the history or reading the test to learn the close-composition behavior gets the wrong answer from the doc layer while the code layer is correct.
What correct looks like: delete or rewrite the outer comment to match the post-3.C1 pin (the inner comment at 458-462 already does).
Confidence: HIGH — Verification: read (sed-verified both blocks).

### F-002 [S4] — same seek landing reported as 266667 µs (terminal) and 266666 µs (snapshot): two wire surfaces disagree by 1 µs
Where: apps/cli/tests/session_script.rs:279-290
Code:
```rust
    assert!(
        out.contains(
            "event: TimingSeekCompleted { position: Timestamp(266667), \
             seek_kind: Command }"
        ),
        "the terminal carries the demuxer's landing:\n{out}"
    );
    assert!(
        out.contains("ok: time 0.266666s (13.3%)"),
        "the snapshot reports the actual (ns → µs truncation):\n{out}"
    );
```
What is wrong: the seek terminal reports the landing as 266667 µs and the very next `time` snapshot reports the same landing as 266666 µs — one surface rounds (ns→µs), the other truncates. The test pins both, so it is honest, but two consumers of the same fact get different numbers.
Why it matters: unit-conversion inconsistency between wire surfaces; invisible at µs, but a template for the ms/µs/ns drift class the audit hunts (§3.2).
What correct looks like: one conversion rule for position reporting across terminal + snapshot.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parent` | 19-24 | REAL | — | committed fixture (verified on disk). |
| `run` | 28-46 | REAL | — | spawns real binary; `wait_with_output` closes stdin → EOF after script (the eof tests depend on it — std semantics, correct). |
| `scripted_session_open_play_pause_seek_stop` | 54-131 | REAL | — | full AC; ordering + counts + exact values; would fail on any described regression. |
| `scripted_session_is_deterministic` | 135-141 | REAL | — | two runs byte-identical. |
| `scripted_session_runs_to_end` | 146-155 | REAL | — | advance 512 → Ended 100%; cross-checked W00 advance-test. |
| `start_time_opens_at_the_target` | 160-169 | REAL | — | 1.000000s then 2.000000s. |
| `start_paused_opens_then_pauses` | 172-179 | REAL | — | Paused at 0.0%. |
| `pause_freezes_the_position` | 182-189 | REAL | — | advance-while-paused is a no-op (poll model pinned). |
| `toggle_flips_play_pause` | 192-200 | REAL | — | both transitions. |
| `close_then_snapshot_is_not_ready` | 206-214 | REAL | — | not-ready after close. |
| `chapter_seek_refuses_typed` | 217-224 | REAL | — | `[session.not-found]` "no chapter 3". |
| `seek_from_playing_rebuffers_in_pipeline` | 235-267 | REAL | — | in-pipeline rebuffer + no-reopen + gen=1. |
| `seek_reports_the_actual_not_the_requested` | 273-290 | REAL | S4 | F-002 1 µs split. |
| `seek_from_ended_refuses_typed` | 296-310 | REAL | — | refusal + no fabricated terminal + position stands. |
| `backward_seek_below_start_time_lands` | 316-334 | REAL | — | watermark cleared at re-anchor; plays to exact end. |
| `quit_during_play_tears_the_session_down` | 347-362 | REAL | — | stop→close order + transcript suffix. |
| `eof_during_play_tears_the_session_down` | 366-376 | REAL | — | empty script. |
| `quit_while_paused_still_tears_down` | 381-389 | REAL | — | Paused → Stopped → Idle. |
| `quit_after_close_is_a_no_op` | 394-409 | REAL | — | idempotent (no echo). |
| `machine_mode_eof_during_play_tears_down` | 418-439 | REAL | — | NDJSON trail, last line idle. |
| `machine_mode_eof_refusal_is_bounded_and_honest` | 456-477 | REAL | S3 | F-001 stale comment; body pins post-3.C1 direct teardown. |
| `machine_mode_eof_transient_refusal_settles_then_tears_down` | 487-512 | REAL | — | post-3.C1 single-drive pin. |
| `machine_mode_malformed_request_still_tears_down` | 519-538 | REAL | — | exit 64 + teardown-before-exit ordering. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 21 | YES | exact transcript substrings/orderings/counts against a real child process on a committed fixture. The determinism test additionally fails on any nondeterminism (ids, addresses, wall-clock leakage). |

No tautologies, no mocks-of-the-subject, no ignored tests in this file. The only soft spot: everything runs against ONE fixture (MJPEG+PCM AVI) — MP4 session behavior through the binary is covered only by ttf_seek (corpus-gated, see that audit).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| doc 13: "scripted CLI session passes (open fixture → play → pause → seek → stop) incl. the unsupported-file error path" | session_script.rs:1-3 | REAL (this half) | tests 54-131 pin the full chain; the unsupported-file half lives in unsupported_path.rs (next audit). UNVERIFIED at runtime here. |
| "two runs must be byte-identical (pinned by test)" | session_script.rs:9-10 | REAL | test 135-141. |
| "pipeline-only settles never advance media time; only `advance` does" | session_script.rs:128-129 | REAL | pinned by 130; this is the run.rs S0 model being codified as correct — cross-ref run.rs F-001. |

## 7. Dead / duplicated / unreachable code

None. No duplication; no ignored tests.

## 8. Dependencies & flags

- Integration test; needs the built binary + the committed AVI fixture. No features. No corpus dependency (unlike ttf_seek) — this file runs everywhere, which is why it is the CLI's most trustworthy evidence.

## 9. Open questions for the next agent

1. host.rs seek dispatch — the `Timestamp(266667)` vs `0.266666s` split (F-002): which surface rounds, which truncates, and where in host.rs the two conversions live.
2. The 3.C1 close-composition landing: the two machine-mode refusal tests' comments (441-462, 489-493) tell a three-act history; whoever audits doc 03 §6 / card 3.C1 should verify the CURRENT engine table matches the "direct teardown" pin.
3. No scripted MP4 session exists anywhere in apps/cli tests — Wave 20's MP4 trace (A05) should note that binary-level MP4 session evidence is corpus-gated only (ttf_seek) or absent (this file).
