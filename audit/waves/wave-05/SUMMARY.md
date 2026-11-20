# Wave 05 SUMMARY — Demuxers II: MP4/MOV (priority wave)

Agents: W05-A01 (subagent, mp4/mod.rs 2225 LOC, 3 chunks), W05-A02 (subagent, tests/test_support/error/fuzz), W05-O (orchestrator, demux/src/lib.rs).
Coverage deviation: mkv/ts/ps/mpegts demuxer files not audited at depth this run (deferred; recorded in the final report).

## Files audited (6)
| File | Auditor | Verdict | Findings |
|---|---|---|---|
| crates/demux/src/mp4/mod.rs | W05-A01 | REAL | S1:3 S2:4 S3:12 S4:5 |
| crates/demux/src/mp4/tests.rs | W05-A02 | REAL | S2:2 S3:6 S4:8* |
| crates/demux/src/mp4/test_support.rs | W05-A02 | REAL | (within counts above) |
| crates/demux/src/mp4/error.rs | W05-A02 | REAL | S2:1 (phantom constructor) |
| crates/demux/src/mp4/fuzz.rs | W05-A02 | REAL-UNPROVEN | S3:1 (never-executed campaign) |
| crates/demux/src/lib.rs | W05-O | REAL (doc STALE) | S3:1 S4:1 |

Totals: S0:0 S1:3 S2:7 S3:19 S4:14 (43).

## The MP4 picture after this wave (all executed/verified 2026-11-16)
1. The demuxer is REAL: a real 3s H.264+AAC MP4 opens through it (tracks, duration, seekable) with a clean state-machine walk (Wave 0 litmus). Fragmented MP4 (moof/trex/trun/tfdt) is genuinely implemented and unit-tested.
2. What actually breaks the user experience is downstream: the shipped build registers decoders `mjpeg, pcm_s16le` only (engine-info self-report) — 0 frames of H.264 and 0 AAC frames decode. The MP4 "cannot be opened properly" complaint is, at HEAD, a DECODER absence, not a demux failure (demux defects below are real but secondary for this file).
3. Demux defects found (S1): track-id space split (chapter-trak files deliver packets on unlisted TrackIds, mod.rs:833-839 vs :1882); backward seek does not reset order_cursor (mod.rs:1571-1577, :2129-2141) — returns right TimeSpan then wrong packets; stts/ctts loops iterate file-declared counts unclamped by body length (mod.rs:1253-1266, :1314-1332) — CPU DoS inside start() on a ~16 MiB crafted moov.
4. Test suite is genuinely good (35/38 real, 0 tautologies, 0 circular) — BUT tests.rs:196 PINS the elst empty-edit units bug (asserts the unconverted 1_000; fails-if-fixed, mod.rs:1090-1094 stores movie-timescale into a media-tick field), and the absent_stss test passes vacuously on error exit.
5. depth_exceeded (error.rs:81) is dead code: zero call sites; MAX_BOX_DEPTH never referenced; the module doc's "depth guard" claim is FALSE.

## Worst 10 findings (citations verified with sed -n by the orchestrator)
1. [S1] mod.rs:1253-1266 stts count unclamped vs body length — crafted-moov CPU DoS.
2. [S1] mod.rs:833-839 vs :1882 — TrackId space split; chapter-trak files mis-address packets.
3. [S1] mod.rs:1571-1577/:2129-2141 — backward seek serves stale order_cursor.
4. [S2] mod.rs:1090-1094 elst empty_edit stores movie-ts in a media-ticks field (48x off) — and tests.rs:196-197 pins it.
5. [S2] mod.rs:779-813 mixed progressive+fragmented silently drops progressive samples.
6. [S2] error.rs:81-90 depth_exceeded dead + doc claims the guard exists (FALSE doc claim).
7. [S2] mod.rs:1351-1359 stsc O(chunks×runs).
8. [S3] tests.rs:60 while-let-Ok swallows typed errors; vacuous pass.
9. [S3] fuzz.rs:45 registered, 4 seeds, campaign never run anywhere ("NO libFuzzer runs" in the archive).
10. [S3] mod.rs:126 + module doc — MAX_BOX_DEPTH dead while doc claims it.

## Anti-laziness gate (§7.3 step 4)
Orchestrator re-checked 4 citations (mod.rs:833-839, 1253-1258, tests.rs:196-197, error.rs:81-84) against the real source — all verbatim-faithful. PASS. No fabrications (audit/verify/fabrications.md empty).

## Cross-agent consistency check
W05-A01 (source side) and W05-A02 (test side) independently found the elst units bug from opposite directions — CONFIRMED by convergence, recorded as dual-verified.

## Agents' blocked items
- moov-at-end execution proof: no test builds a trailing-moov file; corpus file has moov first. Un-tested code path (handled in code per :704-759). Suggested to W20-A05.
- mkv/ts/ps sibling demuxers: deferred (not this wave's files).
