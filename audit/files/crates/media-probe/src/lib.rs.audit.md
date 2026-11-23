# AUDIT: crates/media-probe/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-23 |
| Lines / functions | 445 / 12 (2 production fns + 10 test fns) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/demux_host.rs:446 (`media_probe::probe_stream(&registry, &mut *stream, &config)`) <- CLI open path (demux_host.rs:21 documents the chain). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg (callers, baseline, features), sed -n quote verification. No cargo runs (baseline cited). |

## 1. One-paragraph verdict
The crate root is small and real: `probe_stream` orchestrates (seek check -> meta -> bounded window fill -> position restore -> pure scoring) and `fill_window` is a correctly bounded reader (per-request budget accounting, defensive clamp against over-serving sources, EOS stop, no retry spin — all directly asserted by the in-file tests using the instrumented `CountingSource`). The test module is honest and hostile: exact window boundaries (36/35/12/11 bytes pinning the AVI 90/80/70/no-hit ladder), 8 window sizes up to 300 KB, short reads, non-seekable refusal with zero bytes served, scripted read failures, garbage windows, and non-zero start positions. The one contract violation is on the error path: the crate doc promises "the probe never consumes stream bytes... the position is restored afterwards" unconditionally, but a failing `stream.read` inside `fill_window` propagates with the position left mid-window, unrestored.

## 2. What actually works (verified)
— verified by baseline cargo-test-workspace.txt (PACKAGE media-probe, lines 3573-3708, exit_code=0): unittests `80 passed; 0 failed`, doc-tests `1 passed` (the doctest at lib.rs:85-157 executes the full probe+restore+replay and is pinned by `doctest_shape_probes_like_the_instrumented_source` at 247-266).
— read: fill_window loop-termination reasoning — `want > 0` inside the loop, non-empty chunk implies `take > 0`, so the window strictly grows; no spin, no unbounded capacity (`with_capacity(hint)` where `hint <= max`).

## 3. Findings
F-001 [S3]
Where: crates/media-probe/src/lib.rs:203-209 vs doc claim at 44-46
Code (verbatim, 203-209):
```
    let start = stream.position();
    let window = fill_window(stream, config.max_lookahead)?;
    // Doc 03 §4.2(e): the probe never consumes stream bytes. Restore
    // the pre-probe position (for input's buffered streams this is a
    // read-ahead hit, not fresh I/O).
    if stream.position() != start {
        stream.seek(SeekOrigin::Start(start.0))?;
    }
```
What is wrong: the restore only happens on the success path. If `fill_window`'s `stream.read(want)?` (line 226) fails mid-window, the `?` propagates the error while the stream position sits somewhere inside the consumed window — the crate doc's unconditional guarantee ("The probe never consumes stream bytes ... the position is restored afterwards", lines 44-46) is false on that path. Why it matters: any caller that retries after a transient read error (a plausible engine-level recovery) re-probes from mid-stream and gets garbage; the in-file test `read_failures_propagate_from_the_stream` (392-403) asserts only the error code/stage, never the position. What correct looks like: restore (best-effort) before propagating, or scope the doc claim to the success path and add an error-path test. Blast radius today: low — the sole shipped caller (demux_host.rs:446) aborts the open on probe error — hence S3, not S2. Confidence: HIGH — Verification: read; error-path restore untested confirmed by rg (no test asserts position after a failed read).
F-002 [S4]
Where: crates/media-probe/src/lib.rs:208
Code (verbatim):
```
        stream.seek(SeekOrigin::Start(start.0))?;
```
What is wrong: a seek failure during restore propagates the raw source error (stage Open per plugin-api), with no probe context wrapper — the operator sees an access error where the actual failure is "probe could not un-consume its window". Documented as "propagated unchanged" for stream errors (189-190), so this is a context-quality defect, not a correctness one. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| probe_stream | 193-211 | REAL | S3 (error path) | seek-check-first ✓ (tested: 0 bytes served); restore-on-success ✓; error path leaves position moved (F-001) |
| fill_window | 216-236 | REAL | — | bounded requests, defensive clamp (232), EOS break, terminates; hint capped by `max` |
| tests::doctest_shape_probes_like_the_instrumented_source | 247-266 | REAL | — | pins doctest == instrumented source |
| tests::ok / avi_header | 268-280 | REAL | — | helpers |
| tests::probe_stream_scores_and_restores | 282-297 | REAL | — | replay proof (same bytes again) |
| tests::window_bound_is_exact | 299-326 | REAL | — | 36/35/12/11-byte ladder, 90/80/70/no-hit pinned |
| tests::depth_bound_holds_for_every_window_size | 328-361 | REAL | — | 8 bounds incl. 262_144/300_000; EOS request count pinned (2) |
| tests::short_reads_are_robust | 363-375 | REAL | — | 7-byte chunk limit; request sizes ≤ bound |
| tests::non_seekable_streams_are_a_typed_refusal | 377-390 | REAL | — | zero bytes served before refusal |
| tests::read_failures_propagate_from_the_stream | 392-403 | REAL | — | code+stage pinned; position NOT asserted (F-001) |
| tests::garbage_windows_are_low_confidence_rejections | 405-424 | REAL | — | 5 hostile windows incl. off-by-one EBML and RIFX |
| tests::probing_from_a_nonzero_position_scores_that_window | 426-444 | REAL | — | position-honesty, bytes_examined == 24 |

## 5. Test honesty
All 10 in-file tests assert exact values (codes, stages, confidences, byte counts, request counts, positions); every one would fail on the corresponding breakage. The single blind spot is the F-001 error-path position. No tautologies; no #[ignore] in this file (the crate's one ignored test is elsewhere in the baseline output).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Never probes unbounded data... no code path that reads past it" | lib.rs:6-8 | REAL | fill_window 224-234 + depth test 328-361 |
| "The probe never consumes stream bytes... position is restored afterwards" | lib.rs:44-46 | SHALLOW (success path only) | restore at 207-209; skipped on fill error (F-001) |
| "a valid AVI named .mp4 probes as AVI (tested)" | lib.rs:57-59 | REAL-UNPROVEN (in this file) | tested in signature/registry suites, not here — not re-verified by this audit (other agents' files) |
| "contract module... compiled under cfg(any(test, feature))" | lib.rs:30-33 | REAL | lib.rs:160-161 + Cargo.toml [features] contract=[]; release build (baseline cargo-build-release-lumen-cli.txt:206) compiles media-probe without the feature |
| doctest asserts format "avi", confidence 90, position restored, replay RIFF | lib.rs:85-157 | REAL | baseline Doc-tests media_probe: 1 passed |

## 7. Dead / duplicated / unreachable code
None found. The `contract` module is correctly cfg-gated out of release (verified Cargo.toml + baseline release build compiles the crate).

## 8. Dependencies & flags
media-types + plugin-api only; std-only; no unsafe. Feature `contract = []` never enabled by release lane (baseline release build log shows plain compile). House rules hold — no unwrap/panic in the two production functions.

## 9. Open questions for the next agent
- signature/ and phase5.rs auditors: confirm the scoring ladders mirror the demuxers' own probe gates (the crate doc claims "mirror-pinned... by the round-trip suite" — lib.rs:25-26).
- Engine auditors: does any recovery path retry `probe_stream` after a read error (would trip F-001)?
