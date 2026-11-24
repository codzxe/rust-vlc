# AUDIT: tools/perf/baseline.py
| Field | Value |
|---|---|
| Auditor | W19-A09 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Lines / functions | 968 / 30 (incl. 2 nested) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — dev-side measurement harness (docs/perf/baseline.md's capture tool); not compiled into the release lane, invoked manually |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | python3 re-implementation of cmd_summary vs committed summary.json -> FULL MATCH True (exit 0); row censuses of 5 CSVs |

## 1. One-paragraph verdict
A genuinely honest measurement harness: every family records failures as failures
(decode refusal kept :451-453, memory refusal rows kept-and-tagged :853-858), the docstring
is a treatise on what it CANNOT measure (TTF not measurable through the contract, cold is an
upper bound, wait4's ru_maxrss unusable — with the measured reason), and the summary is
recomputable from the committed raw rows. I re-implemented cmd_summary's grouping/statistics
in an independent python3 pass (non-mutating) and reproduced docs/perf/raw/summary.json
EXACTLY across all five families (startup/probe/decode/seek/peak_rss). Defects are
methodology-level, not integrity-level: a busy-spin RSS poller that competes with the
measured child on this exact 2-core box, and a mislabeled timeout outcome.

## 2. What actually works (verified)
- python3 re-implementation of the summary logic (same grouping keys, median/p95/min/max
  rounding) -> byte-equal dict vs committed summary.json: FULL MATCH True (5/5 families).
- Row censuses: startup 80 (= 25+25+5+25, exactly the code's default runs), probe 60,
  decode 36, seek 60, memory 35 (= 5 help + 3 fixtures x 2 scenarios x 5) — all consistent
  with cmd_* loop structure.

## 3. Findings
F-001 [S3] Where: tools/perf/baseline.py:599-652 (peak_rss_once).
Code (verbatim, :607-610): "The VmHWM of the post-exec image is the process's own kernel-truth
peak; the sampling loop reads it as fast as /proc answers (a read is ~0.03 ms; the committed
raw rows carry the sample count), with one final read after exit-detection — the only race
is peak growth inside that last sub-millisecond window."
What is wrong: "as fast as /proc answers" is an unthrottled busy-spin that runs on the SAME
2 cores as the measured child; the poller is scheduled against the child, inflating the
memory rows' own wall_ms and perturbing any concurrent family. The docstring discloses the
sample rate, not the interference. What correct looks like: a fixed-interval sampler (e.g.
1-5 ms) or a cgroup/kernel-counter read after exit. Confidence: HIGH — Verification: read.
F-002 [S4] Where: :348-381 (probe_once). The 30 s deadline expiring with no response line
leaves `outcome` at its initializer "open-error" — a TIMEOUT is recorded as an open error;
consumers cannot distinguish hang from refusal. Confidence: HIGH — Verification: read.
F-003 [S4] Where: :900-963 (main). `main()` returns 0 unconditionally; a family whose every
row failed still exits 0 (§6.6 "steps that cannot fail"). Failures are visible in the CSVs,
not in the exit code. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| resolve_default_binary | 89-106 | REAL | — | env override + worktree heuristic; no error if absent (deferred to main) |
| now_ms / median / p95 | 125-144 | REAL | — | median even-N avg; p95 nearest-rank ceil, docstring exact |
| stats_row | 147-154 | REAL | — | round(3) everywhere; reproduced committed values |
| write_csv | 157-163 | REAL | — | prints raw path + row count |
| append_jsonl | 166-169 | REAL | — | sort_keys canonical |
| run_status_text | 172-173 | REAL | — | exit{code} for non-zero |
| read_first | 180-190 | REAL | — | exact-field /proc parser; OSError -> "?" |
| environment_snapshot | 193-238 | REAL | — | md5 of binary, git_head via try/except, honest timing_note (:234-237) |
| cmd_environment | 241-249 | REAL | — | writes environment.json |
| time_argv | 256-262 | REAL | — | DEVNULL in/out; wall only |
| evict_binary_pages | 265-276 | REAL | — | fadvise DONTNEED best-effort; upper-bound honesty in docstring |
| time_ready_signal | 279-306 | REAL | — | bool-fix comment :298-302 documents a real past bug (rows silently dropped) — honest history |
| cmd_startup | 309-335 | REAL | — | warm/cold/ready rows; 80-row census consistent |
| probe_once | 348-396 | REAL | S4 | timeout mislabeled "open-error" (F-002) |
| cmd_probe | 399-414 | REAL | — | warns on non-(media-info/refusal) outcomes |
| decode_once | 421-454 | REAL | — | keeps typed refusal first line :451-453 |
| cmd_decode | 457-473 | REAL | — | prints every failed run |
| seek_once | 491-573 | REAL | — | interleaved write/read state machine; seek-failed-event handled; honest "incomplete" default |
| cmd_seek | 576-592 | REAL | — | warns on non-completed |
| peak_rss_once | 599-662 | REAL | S3 | busy-spin poller (F-001) |
| cmd_memory | 665-689 | REAL | — | help floor + decode-bench + scripted session scenarios |
| cmd_stats | 696-756 | REAL | — | two fresh sessions; verbatim stdout+stderr capture; unlinks prior output first |
| load_csv | 764-768 | REAL | — | missing file -> [] |
| numeric | 771-778 | REAL | — | ""/None -> None; ValueError guarded |
| cmd_summary | 781-869 | REAL | — | refusal-tagged memory keys (:853-858); reproduced exactly by me |
| cmd_all | 881-893 | REAL | — | documented default run counts; reproducible one-command record |
| main (+common_flags, add) | 900-963 | REAL | S4 | SUPPRESS-defaults trick documented; always exit 0 (F-003) |

## 5. Test honesty
No unit tests for this file (tools/perf/ contains baseline.py + r1_ff_lanes.py only).
The harness's test is reproducibility: my independent recomputation of summary.json from the
committed CSVs matches exactly — the CSV->summary pipeline is mutation-sensitive (a wrong
median/p95 would show as a mismatch).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every number in docs/perf/baseline.md comes from a raw file written by THIS script" | :52-55 docstring | REAL | summary.json reproduced from CSVs exactly |
| "medians/p95 are nearest-rank over the raw rows" | :54 | REAL | re-implementation used nearest-rank; FULL MATCH |
| "failures are recorded as failures, never dropped" | :54-55 | REAL | decode refusal column; memory [exit-refusal] tagged keys |
| stats "counters are NOT reachable … (DNF, honest)" | :44-50 | REAL | cmd_stats captures the verbatim refusals; W19-A02 corroborated via stats-probe.jsonl |

## 7. Dead / duplicated / unreachable code
None found. All families share the CSV/jsonl plumbing; no copy-paste blocks.

## 8. Dependencies & flags
stdlib only (argparse/csv/json/os/platform/subprocess/sys/time/datetime/hashlib). No env-var
leaks in output (LUMEN_PERF_BINARY read, never printed). Writes confined to --raw-dir
(default docs/perf/raw) — running `summary` rewrites summary.json (deterministic; I avoided
it by re-implementing read-only; dead agent A09 ran it and verified byte-identical).

## 9. Open questions for the next agent
- Quantify F-001: compare memory-row wall_ms under the poller vs a sleep-sampled control.
- tools/perf/r1_ff_lanes.py is unaudited (not in this batch's assignment).
