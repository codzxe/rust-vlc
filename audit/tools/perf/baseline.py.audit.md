# CLAIM AUDIT: tools/perf/baseline.py (L1-967, chunks part1-2)
| Field | Value |
|---|---|
| Auditor | W19-A09 |
| Date | 2026-11-24 |
| Claims extracted | 15 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 11 / 2 / 0 / 0 / 0 / 0 / 2 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
This is a genuinely honest measurement harness and its recorded-run artifacts verify end-to-end: I executed `python3 tools/perf/baseline.py summary` against the committed raw CSVs and the recomputed medians/p95 match `docs/perf/baseline.md`'s tables exactly (startup ready 0.546/0.694 n=25; probe 1.074/2.347 and 1.034/1.081 n=20), and the recomputed summary.json is byte-identical to the committed one — the "CSVs are the truth" claim is reproducible. The docstring's self-scoped limitations (cold = upper bound; probe includes pipe transit + serde; per-frame TTF NOT measurable in this build; counters NOT reachable → DNF) are exactly what a methodology audit wants: claims the method can support, admissions where it cannot. Two real defects: the memory family's VmHWM "tight-poll" is a busy-spin /proc reader that competes for CPU with the measured child on this 2-core sandbox (measurement interference, undisclosed), and the `stats` family hard-codes only two probes with no N/repetition (single-shot evidence). One process slip by this auditor: `cmd_summary` writes into the raw dir by default — my verification run rewrote docs/perf/raw/summary.json (byte-identical deterministic output; diff-verified against a recomputation; incident logged).

## 2. Claim ledger
| # | Claim (verbatim, ≤25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "MEASURES (nothing here is a target — Phase 3 claims none)" | :4-5 | REAL | No target/budget numbers in the harness; medians/p95 only. |
| 2 | startup: "N>=20, median + p95" plus cold variant | :7-10 | REAL | FAMILY_DEFAULT_RUNS startup=25; cold_runs=5 (cmd_all :858-871); startup.csv has n=25 warm rows (recomputed summary n=25). |
| 3 | cold = "best-effort… posix_fadvise DONTNEED… cold rows are an UPPER bound" | :10-11,265-278 | REAL | evict_binary_pages implements fadvise only; honest bound disclosed. Recomputed cold median 6.706 vs warm 0.537 (summary.json). |
| 4 | probe "Host-measured at command-layer receipt… includes pipe transit + serde" | :16-18 | REAL | probe_once stamps at line receipt (:336-352) — the limitation (inclusion of transport) is disclosed, method supports the claim. |
| 5 | "The MP4 fixture measures the probe's TYPED REFUSAL latency… honest refusal-timing row" | :19-21 | REAL | Recomputed summary: `mp4_mjpeg_pcm.mp4/refusal_response_ms` n=20 median 0.52 p95 0.548; decode/memory rows keep `[exit65-refusal]` tagged. |
| 6 | decode: "a per-frame TTF is NOT measurable through the contract in this build" | :22-27 | REAL | per_frame_ms_derived labeled "derived, labeled as such" (:812-820); bench fields come from the BENCH line. |
| 7 | memory: VmHWM /proc sampling because "ru_maxrss is UNUSABLE from a Python parent… measured: wait4(/bin/true)=~13 MiB vs real ~2 MiB" | :30-36,606-612 | REAL (reasoned) | peak_rss_once samples /proc/<pid>/status VmHWM (:613-650). The wait4-contamination measurement itself is historical — UNVERIFIED (no re-run of the wait4 comparison; plausible and well-reasoned). |
| 8 | stats: "counters are NOT reachable through the command layer in this build (DNF, honest)" | :39-43 | REAL | docs/perf/raw/stats-probe.jsonl: `system.get-stats` → exit64 (unknown cmd); `system.get-state-snapshot` → cataloged (commands/src/catalog/ids.rs:170) but served error "unsupported-for-host". Both recorded verbatim. |
| 9 | "every number in docs/perf/baseline.md comes from a raw file written by THIS script" | :45-47 | REAL (executed) | Ran `python3 tools/perf/baseline.py summary --binary /bin/true` → medians/p95 match baseline.md:57 (0.546/0.694), :79-80 (1.074/2.347, 1.034/1.081); recomputed summary.json byte-identical to committed one (diff → IDENTICAL). |
| 10 | "medians/p95 are nearest-rank over the raw rows;… no warmup samples are discarded" | :47-49 | REAL | median/p95 nearest-rank implementations :128-144; warmups never discarded in any family loop. |
| 11 | ready-signal bugfix: predecessor "returned an int… every ready row was mislabeled bad-response and silently dropped" | :294-298 | REAL-UNPROVEN | Current code returns `bool(ok and code == 0)` (:299); predecessor behavior UNVERIFIABLE (no git history access). |
| 12 | "p95… N=12 -> rank 12 (worst). Nothing is interpolated." | :136-141 | REAL | Recomputed p95 for n=5 rows equals max (e.g. cold-fadvise p95 7.538 = max 7.538). |
| 13 | timing_note: "shared 2-core sandbox; other wave agents may run concurrently — medians+raw are the record" | :214-217 | REAL | Honest environmental caveat in environment.json. |
| 14 | (implicit) memory-family sampling does not perturb the measured process | :30-36 | FALSE-adjacent → methodology defect (S3) | peak_rss_once spins `while True: open(/proc/pid/status)` as fast as /proc answers (:625-650) on a 2-core box — the sampler occupies a core the decode-bench child could use; wall_ms and even completion ordering can be perturbed. Not disclosed in the docstring's honesty rules. |
| 15 | cmd_summary: "the CSVs are the truth, this is a convenience view" | :742-744 | REAL | Verified by execution (see #9); summary.json deterministically reproducible. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Honesty rules cover measurement integrity | (none found) | — |

## 4. Fabrication / inflation indicators
None. Numbers reconcile from raw to summary to doc. The one methodology weakness (busy-spin sampler interference) is an omission, not a fabrication.
INCIDENT NOTE (auditor, 2026-11-24): running `baseline.py summary` with default `--raw-dir` rewrote docs/perf/raw/summary.json (the command writes its output into the raw dir). Diff-verified byte-identical to the deterministic recomputation from the committed CSVs; no data changed. Lesson: redirect `--raw-dir` under audit/ when executing repo tools.

## 5. What actually works (verified)
- `python3 -m py_compile tools/perf/baseline.py` → clean (the earlier "corruption" observation was a display artifact; retracted).
- `python3 tools/perf/baseline.py summary --binary /bin/true` → recomputed summary matching baseline.md tables and committed summary.json byte-for-byte.
Commands run (exit 0): py_compile, summary (twice; once redirected), diff, python json dumps of raw CSVs. UNVERIFIED: full `all` capture (needs the lumen release binary; none in tree — cargo-build budget forbids).
Findings: S2: 0 · S3: 1 (F-010 busy-spin VmHWM sampler competes with the measured child on 2 cores, undisclosed) · S4: 1 (F-011 stats family single-shot, no N).
