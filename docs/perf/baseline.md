# Performance baseline — Phase-2/3 format set (card 3.F1)

Status: **baseline capture only — no targets claimed** (doc 12 §1.1: a
target before a baseline is theater; this document pins the first honest
measured record for the six metric families the card names).

| Field | Value |
|---|---|
| Card | 3.F1 (doc 13 wave program), per docs/12-performance-benchmarking.md §4.1 |
| Captured | 2026-10-23 07:59:46 UTC (full capture, one `baseline.py all` invocation, ~1 s wall) |
| Harness | `tools/perf/baseline.py` (Python 3.12, stdlib only) — see §8 |
| Raw data | `docs/perf/raw/` (committed: CSV/JSONL per family + `summary.json` + `environment.json`) |
| Binary | release build of `apps/cli` (`lumen`), 3 232 328 B, md5 `18e6b1ccdf8b16077b5f199d608856fe`, built from this branch's HEAD `a63fe22` (`CARGO_TARGET_DIR=/home/z/my-project/target cargo build --release -p lumen-cli` from `worktrees/w5-11` after touching every workspace `lib.rs`; all 17 `Compiling … (worktrees/w5-11)` lines verified — the shared-target stale-rlib quirk was checked and excluded; the rebuild is byte-identical to the predecessor agent's measured binary) |
| Corpus | the committed class-A fixture parents (byte-stable, `tests/fixtures/generated/valid/`): `avi_mjpeg_pcm.avi` (425 516 B, MJPEG+PCM, 2 streams, 60 frames/stream), `avi_mjpeg_2pcm.avi` (497 160 B, MJPEG+2×PCM, 3 streams, 20 frames/stream), `mp4_mjpeg_pcm.mp4` (424 452 B, MP4 MJPEG+PCM) |
| Comparisons to targets | **none** — Phase 3 claims none per the card |

Measurement posture (honest): every wall-clock number below is
**host-measured** (doc 12 §1.5) — stamped by the harness at
command-layer line receipt over the real `--control-stdio --json`
NDJSON session — because the engine's proposed `ts_ns` event-envelope
field (doc 12 §2.4 item 1) is not in the v1 catalog. Each number
therefore includes pipe transit and serde serialization on the observed
side of the pipe. No number in this document was produced by any run
other than the recorded one; medians and p95 are **nearest-rank** over
the committed raw rows (p95 = the ceil(0.95·N)-th smallest sample: with
N=20 that is rank 19, the second-worst; with N=12 or N=5 it is the
worst). Raw failures stay in the CSVs as rows — never dropped.

---

## 1. Hardware manifest (read from /proc this run)

| Field | Value |
|---|---|
| CPU model | Intel(R) Xeon(R) Processor |
| CPU family / model id | 6 / 173 |
| Logical cores | 2 |
| RAM | 4 138 564 kB (~3.95 GiB) |
| Kernel | 5.10.134-013.15.kangaroo.al8.x86_64 |
| Machine | shared 2-core sandbox (`c-6adae26f-145cf406-fea3e6817a12`); other wave agents may run concurrently — medians over N are the record, single rows are not |
| Clock | `time.perf_counter_ns` (monotonic) |

## 2. Process startup time (M-01 family)

Full process wall (spawn → exit) per invocation, N runs; plus the
doc-12 M-01 signal proper: spawn → first `system.get-engine-info`
response over `--control-stdio`. The cold variant is best-effort
(`posix_fadvise(DONTNEED)` over the binary before each run — the
loader/shared-library pages cannot be evicted from userspace, so cold
rows are an *upper* bound on warmth; doc 12 §2.2's caveat applies).

| Metric | Variant | N | median (ms) | p95 (ms) | Raw |
|---|---|---|---|---|---|
| `lumen --help` wall | warm | 25 | 1.138 | 1.350 | `raw/startup.csv` |
| `lumen --version` wall | warm | 25 | 0.537 | 0.605 | `raw/startup.csv` |
| `lumen --version` wall | cold (fadvise) | 5 | 6.706 | 7.538 | `raw/startup.csv` |
| spawn → engine-info response | warm | 25 | 0.546 | 0.694 | `raw/startup.csv` |

Harness note (relaunch fix): the predecessor's draft mislabeled every
ready-signal row `bad-response` (an int `0` success code read as
falsy), which also silently dropped those rows from the summary. The
24 previously recorded wall values were discarded with the predecessor's
raw set; every number above is from this run.

## 3. Probe latency (M-02 family) — open → `media.info-ready`

Fresh `--control-stdio --json` process per run; harness writes
`session.open` (file source) and stamps the moment the engine's
`media.info-ready` event line is read back. Doc 12 §2.1's M-02 signal
proper is the proposed `session.probe-completed` event (unlanded); the
closest landed surface is `media.info-ready`, and that is what is
measured — labeled as such. The MP4 row measures the probe's
**typed-refusal** latency (the signature registry identifies `mp4`,
D-018 ships no demuxer for it) — an honest refusal-timing row, not a
media-info row.

| Fixture | Signal | N | median (ms) | p95 (ms) | Raw |
|---|---|---|---|---|---|
| `avi_mjpeg_pcm.avi` | open→`media.info-ready` | 20 | 1.074 | 2.347 | `raw/probe.csv` |
| `avi_mjpeg_2pcm.avi` | open→`media.info-ready` | 20 | 1.034 | 1.081 | `raw/probe.csv` |
| `mp4_mjpeg_pcm.mp4` | open→typed refusal | 20 | 0.520 | 0.548 | `raw/probe.csv` |

## 4. Time-to-first-frame / first-sound (M-03/M-04 family)

**What is honestly measurable in this build** (the card's prescribed
method): `decode-bench` repeated runs — `wall_s` is the true process
wall over open + full-file decode-to-null (both streams), `decode_s` is
the packet loop after open. The per-frame first-frame / first-sound
signals are **not measurable through the command contract in this
build**: the v1 catalog has no `video.first-frame-presented` event and
no `audio.first-sound` counter (doc 12 §2.4 items 2–3 unlanded), and
the CLI host advances the engine only at command boundaries (poll
model — no wall-clock playback), so a session-level TTF would measure
pump scheduling, not a user-visible latency. Per-frame mean is derived
(whole-file decode ÷ decoded frames) and labeled derived. The BENCH
line carries 1 ms resolution (`{:.3}` s) — the decode rows inherit it.

| Fixture | Signal | N | median | p95 | Raw |
|---|---|---|---|---|---|
| `avi_mjpeg_pcm.avi` | `wall_s` (open+decode all, 60+60 frames) | 12 | 8 ms | 8 ms | `raw/decode.csv` |
| `avi_mjpeg_pcm.avi` | `decode_s` (packet loop) | 12 | 7 ms | 8 ms | `raw/decode.csv` |
| `avi_mjpeg_pcm.avi` | per-frame mean (derived) | 12 | 0.117 ms | 0.133 ms | `raw/decode.csv` |
| `avi_mjpeg_2pcm.avi` | `wall_s` (20+20 frames) | 12 | 3 ms | 3 ms | `raw/decode.csv` |
| `avi_mjpeg_2pcm.avi` | `decode_s` (packet loop) | 12 | 3 ms | 3 ms | `raw/decode.csv` |
| `avi_mjpeg_2pcm.avi` | per-frame mean (derived) | 12 | 0.150 ms | 0.150 ms | `raw/decode.csv` |
| `mp4_mjpeg_pcm.mp4` | — | — | **DNF** (typed refusal `probe.unsupported`, exit 65: no MP4 demuxer, D-018) | — | `raw/decode.csv` |

TTFS specifically: PCM decode is effectively memcpy-class and the
whole-audio-stream decode is inside `decode_s` above; no finer
first-sample signal exists on the contract surface this build (same
§2.4 gap). Recorded as the honest limitation it is.

## 5. Seek latency (M-05 family) — seek command → `timing.seek-completed`

Fresh `--control-stdio --json` session per run: `session.open` (auto
settle), then `playback.seek {percent 0.5}` (first in-session seek),
then `playback.seek {percent 0.25}` (second, in-session "warm"). The
CLI's seek dispatch awaits the terminal internally, so the wire-visible
response and the post-response `timing.seek-completed` event line both
cover the whole native seek flow (queue flush, reanchor, pre-roll).
Doc 12 §2.3's M-05 definition is seek `Accepted` → `seek-completed`;
this build's accepted-response and seek-completed lines are stamped
separately below, both from the request write.

| Fixture | Signal | N | median | p95 | Raw |
|---|---|---|---|---|---|
| `avi_mjpeg_pcm.avi` | seek#1 write→response | 20 | 6.983 ms | 7.312 ms | `raw/seek.csv` |
| `avi_mjpeg_pcm.avi` | seek#1 write→`seek-completed` event | 20 | 6.992 ms | 7.320 ms | `raw/seek.csv` |
| `avi_mjpeg_pcm.avi` | seek#2 write→response | 20 | 5.804 ms | 5.988 ms | `raw/seek.csv` |
| `avi_mjpeg_pcm.avi` | seek#2 write→`seek-completed` event | 20 | 5.810 ms | 6.002 ms | `raw/seek.csv` |
| `avi_mjpeg_2pcm.avi` | seek#1 write→response | 20 | 5.339 ms | 5.543 ms | `raw/seek.csv` |
| `avi_mjpeg_2pcm.avi` | seek#1 write→`seek-completed` event | 20 | 5.351 ms | 5.550 ms | `raw/seek.csv` |
| `avi_mjpeg_2pcm.avi` | seek#2 write→response | 20 | 5.346 ms | 5.399 ms | `raw/seek.csv` |
| `avi_mjpeg_2pcm.avi` | seek#2 write→`seek-completed` event | 20 | 5.352 ms | 5.407 ms | `raw/seek.csv` |
| `mp4_mjpeg_pcm.mp4` | — | — | **DNF** (open refusal, D-018) | — | `raw/seek.csv` |

## 6. Peak memory per session (M-08 family)

Peak RSS via `/proc/<pid>/status` `VmHWM` tight-poll sampling — the
kernel's high-water mark of the process's **own** image. (`os.wait4`'s
`ru_maxrss` is unusable from a Python parent: the fork's pre-exec
Python image contaminates the child's `signal->maxrss` — measured on
this machine, `wait4(/bin/true)` reports ~13 MiB where the child's real
peak is ~2 MiB; the harness module docs carry the experiment.
`/usr/bin/time` does not exist on this machine.) The sampling loop
reads as fast as `/proc` answers (~0.03 ms per read); the raw rows
carry the per-run sample count (`hwm_samples`). Scenarios: the
decode-bench run, a full scripted control-stdio session (open + two
seeks + stop + close), and `--help` as the process floor.

| Scenario | N | median peak RSS (KiB) | max peak RSS (KiB) | Raw |
|---|---|---|---|---|
| `--help` (process floor) | 5 | 3 224 | 3 348 | `raw/memory.csv` |
| `decode-bench avi_mjpeg_pcm.avi` | 5 | 3 532 | 3 676 | `raw/memory.csv` |
| `decode-bench avi_mjpeg_2pcm.avi` | 5 | 3 380 | 3 548 | `raw/memory.csv` |
| `decode-bench mp4_mjpeg_pcm.mp4` (refusal path, exit 65) | 5 | 3 000 | 3 124 | `raw/memory.csv` |
| session `avi_mjpeg_pcm.avi` (open+2 seeks+stop) | 5 | 12 356 | 12 440 | `raw/memory.csv` |
| session `avi_mjpeg_2pcm.avi` (open+2 seeks+stop) | 5 | 11 796 | 11 868 | `raw/memory.csv` |
| session `mp4_mjpeg_pcm.mp4` (refusal path) | 5 | 3 160 | 3 272 | `raw/memory.csv` |

The session scenarios peak ~8.5 MiB above the decode-bench scenarios
(same engine, session/pipeline path); this run's per-scenario spreads
are tight (≤ 392 KiB). The predecessor's draft run had shown one
4-KiB `--help` artifact (a sampler race on a very short-lived process);
this run's rows are anomaly-free, and the median-of-N rule plus the
committed `hwm_samples` column keep any future recurrence visible
rather than hidden.

## 7. Dropped frames / underruns (M-09/M-10 family) — DNF, with evidence

`system.get-stats` is **not in the v1 wire catalog** and the nearest
cataloged surface, `system.get-state-snapshot`, is **not served by this
host** — both probed on a live session during the recorded run and
answered with typed refusals, recorded verbatim in
`raw/stats-probe.jsonl`:

- `system.get-stats` → `error: [control.invalid-argument] unknown
  variant 'system.get-stats', expected one of …` (the full 60-variant
  command enum is in the raw record; exit 64)
- `system.get-state-snapshot` → wire response
  `{"error":{"stage":"control","code":"unsupported-for-host","message":"…
  not served by this build's command-line host (not in the first
  vertical's served set)"}}` (exit 0 — the session survives the refusal)

The engine's internal counters exist (`video_engine::SchedulerStats`'
`dropped_late` / `dropped_stale`, the audio ring's underrun accounting)
but are not reachable through the command layer, and the anti-gaming
rules (doc 12 §3.9.1) forbid the harness from reaching past the
contract. **Handoff: the stats command + counters are the prerequisite
for this family.**

## 8. Reproducing the run

```bash
# 1. build the release binary (shared target dir per wave brief);
#    verify the compile lines name YOUR worktree, then copy the binary
#    aside so a concurrent shared-target rebuild cannot swap it mid-run
export CARGO_TARGET_DIR=/home/z/my-project/target
cargo build --release -p lumen-cli
cp "$CARGO_TARGET_DIR/release/lumen" /tmp/lumen-perf

# 2. full capture (each family's documented N; raw lands in docs/perf/raw/)
python3 tools/perf/baseline.py --binary /tmp/lumen-perf all

# families individually (common flags accept both positions:
# `--binary X startup` and `startup --binary X`)
python3 tools/perf/baseline.py startup --runs 25
python3 tools/perf/baseline.py probe   --runs 20
python3 tools/perf/baseline.py decode  --runs 12
python3 tools/perf/baseline.py seek    --runs 20
python3 tools/perf/baseline.py memory  --runs 5
python3 tools/perf/baseline.py stats
python3 tools/perf/baseline.py environment
python3 tools/perf/baseline.py summary   # recompute medians/p95 from CSVs
```

Every table row above links its raw file; the CSVs are the truth, the
tables are the nearest-rank medians/p95 of those rows. Failures stay in
the CSVs as rows (`status`/`outcome`/`refusal` columns) — never dropped.
`py_compile` clean (Python 3.12.15, stdlib only, no third-party imports).

## 9. Known limitations of this capture

- Host-measured latencies (no `ts_ns` engine stamps) — doc 12 §1.5's
  weaker, labeled claim; includes pipe transit + serde.
- Shared 2-core sandbox: concurrent wave agents are plausible noise;
  medians+raw are the record (the decode family's 1 ms BENCH
  resolution further floors that family's precision).
- Cold startup is best-effort page-cache eviction only (upper bound on
  warmth).
- MP4 rows measure the D-018 refusal path, not playback.
- M-03/M-04 per-frame TTF/TTFS and M-09/M-10 counters are not on the
  contract surface this build (see §4 and §7) — DNF with evidence, not
  silent omission.
- corpus-big is regenerating in the main tree and was **not** used —
  the committed class-A parents are the Phase-2 format set of record.
