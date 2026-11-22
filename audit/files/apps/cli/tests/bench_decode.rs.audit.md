# AUDIT: apps/cli/tests/bench_decode.rs

| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-22 |
| Lines / functions | 169 / 4 helpers + 6 tests |
| File verdict | REAL-UNPROVEN (real-binary tests, committed fixtures, no corpus gating; never ran in the W00 baseline) |
| Reachable from shipped entry point? | N/A (test). Spawns the REAL `lumen` binary via `CARGO_BIN_EXE_lumen` with argv `decode-bench`. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 0 |
| Commands run | `ls` fixture verification (all present, incl. `generated/malformed/avi_mjpeg_pcm__flip16.avi` and `generated/valid/avi_mjpeg_2pcm.avi`); quotes verified with `sed -n`. NO cargo (adaptation). |

## 1. One-paragraph verdict

A well-built real-binary test of the `decode-bench` surface: exit-code taxonomy (0/64/65/66), one-BENCH-line stdout discipline, exact frame counts against MANIFEST-pinned fixture truth, stderr-only coverage notes, and adversarial inputs (garbage, magic-flipped derivative, missing file, grammar failures). No corpus gating — all six tests run anywhere. Two problems: the suite has no recorded execution (the W00 baseline fail-fast hole), and it silently codifies the bench's first-vertical-only scope — every happy path is mjpeg+pcm, the `no decoder in this build` refusal path (bench.rs:334, the FALSE message documented in my bench.rs audit) is never exercised by any test, which is exactly how a lying diagnostic survives a green suite.

## 2. What actually works (verified)

- Real binary, piped stdio, `code().unwrap_or(-1)` — signal kills surface as -1.
- All fixtures exist (verified `ls`): `generated/valid/avi_mjpeg_pcm.avi`, `generated/valid/avi_mjpeg_2pcm.avi`, `generated/malformed/avi_mjpeg_pcm__flip16.avi`.
- Assertions are exact where they can be: `bytes=425516` (MANIFEST pin), `v_frames=60/v_decoded=60`, `a_frames=60/a_decoded=60` (structure-map counts), exit codes 64/65/66 with stage-tagged error prefixes.
- The two-stream test pins the honest first-of-kind coverage note (`first-of-kind already benched`, :119-122).

## 3. Findings

### F-001 [S2] — No recorded run

Where: whole file; evidence: audit/runs/baseline/cargo-test.log (116 binaries, `bench_decode` absent)

Code (bench_decode.rs:67-70, verbatim):
```rust
fn avi_fixture_reports_one_complete_bench_line() {
    let path = fixture("generated/valid/avi_mjpeg_pcm.avi");
    let (out, err, code) = run_bench(&[&path]);
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
```

What is wrong: the W00 baseline never executed this binary (fail-fast stop at `first-vertical`, cargo-test.log:2499). The exit-code taxonomy and the exact-count pins are unexecuted against the current tree.
Confidence: HIGH — Verification: baseline binary inventory.

### F-002 [S3] — The suite pins the bench's 2-codec scope and never tests the refusal message that lies

Where: apps/cli/tests/bench_decode.rs:93-123 (the only multi-codec-shaped test is still mjpeg+pcm); bench.rs:334 (the untested false claim)

Code (bench_decode.rs:119-122, verbatim):
```rust
    assert!(
        err.contains("first-of-kind already benched"),
        "the unbenched second audio stream is named:\n{err}"
    );
```

What is wrong: every happy path benches only mjpeg+pcm; the third refusal reason — `"no decoder in this build (first vertical: mjpeg + pcm)"` (bench.rs:334, proven FALSE in my bench.rs audit: the build dispatches 14 codecs) — appears in no test. A test feeding the committed `mp4_h264_aac.mp4` fixture through `decode-bench` would either have pinned the message (enshrining the lie) or caught the divergence between the bench's allowlist and the provider; neither happened. The suite's green therefore says nothing about the bench's codec coverage story.
Why it matters: §3.4 — the untested arm is the one carrying a false capability claim that already corrupted the W00 baseline narrative.
What correct looks like: a `h264_fixture_refuses_or_benches_honestly` test pinning whatever the truthful behavior is post-fix.
Confidence: HIGH — Verification: read + bench.rs audit cross-reference.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `fixture` | 12-17 | trivial-correct | — | |
| `run_bench` | 20-46 | REAL-UNPROVEN | — | Real binary; BrokenPipe tolerated. |
| `bench_fields` / `number` | 49-61 | trivial-correct | — | Missing/unparseable → -1.0 sentinel (asserts then fail loudly). |
| `avi_fixture_reports_one_complete_bench_line` | 66-91 | REAL-UNPROVEN | S2 (F-001) | Exact pins incl. MANIFEST bytes. |
| `two_audio_stream_fixture_benches_the_first_stream` | 95-123 | REAL-UNPROVEN | S3 (F-002) | First-of-kind coverage pin. |
| `garbage_file_is_a_typed_probe_refusal` | 127-136 | REAL-UNPROVEN | S2 (F-001) | Exit 65 + `[probe.unsupported]`. |
| `magic_broken_derivative_is_a_typed_probe_refusal` | 141-147 | REAL-UNPROVEN | S2 (F-001) | Adversarial committed derivative. |
| `missing_file_is_inaccessible` | 150-155 | REAL-UNPROVEN | S2 (F-001) | Exit 66. |
| `grammar_failures_are_usage_errors` | 158-169 | REAL-UNPROVEN | S2 (F-001) | Exit 64, both arities. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 6 | YES | Exact counts/codes/messages against the real binary. (All runtime-UNPROVEN — F-001.) |
| Coverage gap | — | No test exercises the not-benched-refusal arm (F-002) — the false message is unpinned. |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "one machine-parseable BENCH line on stdout and exit 0 for a decodable file" | :1-3 | REAL-UNPROVEN | Pinned by the happy-path test; unexecuted (F-001). |
| "typed stage-tagged errors + sysexits-class exits (never a panic, never a BENCH line)" | :3-5 | REAL-UNPROVEN | Refusal tests pin 64/65/66 + no-BENCH-on-failure. |

## 7. Dead / duplicated / unreachable code

- None. No corpus gating.

## 8. Dependencies & flags

- Integration test against the bin target (`CARGO_BIN_EXE_lumen`); no feature gates.

## 9. Open questions for the next agent

- After bench.rs F-001's fix (shared provider), this suite must grow the h264/aac bench lanes and re-pin the coverage notes.
