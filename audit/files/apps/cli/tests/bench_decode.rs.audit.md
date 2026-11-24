# AUDIT: apps/cli/tests/bench_decode.rs
| Field | Value |
|---|---|
| Auditor | W13-A05 |
| Date | 2025-06-27 |
| Lines / functions | 169 / 6 tests + 3 helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | Test target `cargo test -p lumen-cli --test bench_decode` (executed by me); it drives the SHIPPED `lumen` binary's `decode-bench` subcommand via `CARGO_BIN_EXE_lumen` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `cargo test -p lumen-cli --test bench_decode` → 6 passed / 0 failed; `./target/debug/lumen decode-bench tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` → exit 0, `BENCH ... bytes=425516 v_frames=60 v_decoded=60 a_frames=60 a_decoded=60 wall_s=0.088 decode_s=0.087`; `./target/debug/lumen decode-bench tests/fixtures/generated/ivf/av1_64x48_30fps.ivf` → exit 0, `v_decoded=0` + stderr "no decoder in this build (first vertical: mjpeg + pcm)" |

## 1. One-paragraph verdict
A real subprocess-level CLI test suite: it spawns the actual `lumen` binary, pins the one-BENCH-line contract with exact fixture truths (bytes=425516, 60/60/60/60 demuxed+decoded for the class-A AVI), the first-of-kind two-audio note on stderr, and the typed failure ladder (garbage → exit 65 `[probe.unsupported]`; magic-broken derivative → 65; missing file → 66 `[input.inaccessible]`; grammar violations → 64). I ran all 6 tests (pass) and reproduced the happy-path BENCH line by hand. The suite is honest about what it tests — but it is blind to the one lie its subject binary tells: `decode-bench` refuses every codec except mjpeg/pcm with the message "no decoder in this build (first vertical: mjpeg + pcm)" (bench.rs:334), which is FALSE for this build — I proved the same binary's workspace decodes AV1 30/30 via the real provider (my containers::ivf run), yet `decode-bench` on that very fixture reports `v_decoded=0` + the false message, and no test in this file challenges it. Two smaller findings: the stdin BrokenPipe arm is unreachable dead code, and `wall_s` has no upper sanity bound.

## 2. What actually works (verified)
— `cargo test -p lumen-cli --test bench_decode` → `test result: ok. 6 passed; 0 failed`.
— Direct run: `./target/debug/lumen decode-bench tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` → exit 0, `BENCH file=... bytes=425516 v_frames=60 v_decoded=60 a_frames=60 a_decoded=60 wall_s=0.088 decode_s=0.087`, stderr `bench: video track#0 mjpeg — decoding; audio track#1 pcm_s16le — decoding (container avi)` — every assertion in `avi_fixture_reports_one_complete_bench_line` independently reproduced.
— The exit-code ladder (65/65/66/64) matches the sysexits contract; error strings pinned exactly.

## 3. Findings
F-001 [S3]
Where: apps/cli/tests/bench_decode.rs:2-3 (contract doc) + 67-91 (the only decode happy paths); locus of the lie: apps/cli/src/bench.rs:334
Code: (verbatim)
```
//! one machine-parseable BENCH line on stdout and exit 0 for a
//! decodable file; typed stage-tagged errors + sysexits-class exits
```
What is wrong: "a decodable file" is only ever exercised as mjpeg+pcm. The bench binary hard-codes its decoder selection (`fact.codec == "mjpeg"` / `starts_with("pcm_")`, bench.rs:295-300) and tells every other codec "no decoder in this build (first vertical: mjpeg + pcm)" — a false statement about the build: `./target/debug/lumen decode-bench tests/fixtures/generated/ivf/av1_64x48_30fps.ivf` → `v_decoded=0` + that message, while the identical workspace build decodes the identical file 30 frames through the real provider (my `cargo test -p lumen-cli --lib containers::ivf` run: `av1_lanes...` / `generated_av1_lane_decodes_the_full_stream` pass with 30 decoded frames). The test suite pins the mjpeg/pcm lane and the failure ladder but never asserts that a decodable AV1/VP9/FLAC file decodes, so the false capability message ships test-green — and the rc-smoke gate ("headless-decode ... every frame must decode") only ever feeds it the AVI fixture. Why it matters: the bench's output is the project's own throughput-evidence surface; it silently measures nothing for every modern codec while blaming "this build". What correct looks like: bench uses the real `HostDecoderProvider` (like the container hosts' tests do), or the message says "decode-bench benches only the first vertical (mjpeg/pcm)"; a test in this file benches one AV1 fixture and asserts v_decoded>0. Confidence: HIGH — Verification: ran (both commands, outputs above). bench.rs itself is another agent's file — the finding lands there as the fix site.

F-002 [S4]
Where: apps/cli/tests/bench_decode.rs:32-38
Code: (verbatim)
```
        if let Err(err) = stdin.write_all(b"") {
            assert_eq!(
                err.kind(),
                std::io::ErrorKind::BrokenPipe,
                "unexpected stdin write failure: {err}"
            );
        }
```
What is wrong: `write_all` with an EMPTY buffer returns `Ok(())` without performing any I/O (std's loop is `while !buf.is_empty()`), so the `Err` arm — and its "a closed pipe is the expected shape" rationale in the comment — is unreachable dead code; the block never probes the pipe at all. Harmless, but it is test theater: it looks like broken-pipe handling and can never execute. Confidence: HIGH — Verification: read (std semantics; not empirically distinguished — UNVERIFIED at the syscall level, but Rust's write_all contract makes the arm unreachable).

F-003 [S4]
Where: apps/cli/tests/bench_decode.rs:84-88
Code: (verbatim)
```
    assert!(number(&fields, "wall_s") >= 0.0, "{line}");
    assert!(
        number(&fields, "decode_s") <= number(&fields, "wall_s") + 0.002,
```
What is wrong: `wall_s >= 0.0` is a field-presence check only (missing key parses to the sentinel −1.0, so it does catch absence) with no upper bound — a 1000× decode regression passes; and the `decode_s ≤ wall_s + 0.002` tolerance is generous enough to hide the decode loop exceeding the wall by 2 ms per run. Acceptable for a contract smoke, weak as a perf guard. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixture / run_bench / bench_fields / number | 12-61 | REAL | — | subprocess harness; `number`'s −1.0 sentinel doubles as a presence check |
| test avi_fixture_reports_one_complete_bench_line | 66-91 | REAL | S3 | exact truths; only mjpeg/pcm lane (F-001) |
| test two_audio_stream_fixture_benches_the_first_stream | 95-123 | REAL | — | first-of-kind note pinned |
| test garbage_file_is_a_typed_probe_refusal | 127-136 | REAL | — | exit 65, exact code |
| test magic_broken_derivative_is_a_typed_probe_refusal | 141-147 | REAL | — | committed malformed derivative |
| test missing_file_is_inaccessible | 150-155 | REAL | — | exit 66, exact code |
| test grammar_failures_are_usage_errors | 158-169 | REAL | — | 0-arg and 2-arg → 64 (unknown flags land here too — treated as paths, verified by my run) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| avi_fixture_reports_one_complete_bench_line | YES | exact byte/frame counts, single-line stdout, stderr note (reproduced by hand) |
| two_audio_stream_fixture_benches_the_first_stream | YES | decoded==frames equalities + note |
| garbage / magic-broken / missing / grammar | YES | exact exit codes + error strings; no-BENCH-on-failure pinned |
| (absent) any non-mjpeg/pcm decode lane | — | F-001: the false "no decoder in this build" message cannot fail any test in this file |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "one machine-parseable BENCH line on stdout and exit 0 for a decodable file" | L1-3 | SHALLOW | true only for mjpeg/pcm files; a decodable AV1 file gets a BENCH line with zeros + exit 0 (my run) — "decodable" is doing undocumented work |
| "typed stage-tagged errors + sysexits-class exits (never a panic, never a BENCH line) for garbage, missing files and grammar failures" | L3-5 | REAL | 4 tests, executed |
| "the adversarial-corpus contract" | L5 | REAL (scoped) | the three named failure classes are pinned; the undecodable-codec class is not |

## 7. Dead / duplicated / unreachable code
The stdin BrokenPipe arm (F-002) is unreachable. Nothing else dead.

## 8. Dependencies & flags
testkit only; uses `CARGO_BIN_EXE_lumen` (the real bin). No feature gates, no unsafe. Fixtures from tests/fixtures/generated (committed).

## 9. Open questions for the next agent
- bench.rs owner: the hard-coded `fact.codec == "mjpeg"` / `pcm_` selection (bench.rs:295-300) + the false "no decoder in this build" note (bench.rs:334) — the S3's fix site; also reconciles with the environment-fact thread "trace which registry lies" (the hevc error's "landed:" list vs this message are two contradictory capability claims in the same binary).
- rc-smoke's "headless-decode (every frame must decode)" gate only feeds the AVI fixture — widening it would have caught F-001.
