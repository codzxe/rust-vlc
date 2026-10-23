# tools/sweep — card 7-d session breadcrumbs

Session 4 (relaunch 5) — CARD COMPLETE (see worklog 7-d closure entry):

- Merged main first (Lead's ce9cc4a: scalar-IDCT default restored +
  V-1/V-3/input-path memory fixes + ui scaffold) so the speed
  re-measure runs the shipped engine, not the stale branch.
- PARITY DOC DONE: docs/perf/parity-v4-20261024.md.
  * Frame counts 19/19 mjpeg streams EXACT (17 clean + 2 dirty).
  * Duration: 12/19 µs-exact; deltas cluster on a+v audio-tail files
    (max +130 ms, 0066); 0029 ffprobe=N/A where Lumen answers.
  * PCM MD5 8/9 EXACT; 1 honest MISMATCH (dup-range dirty: −2 packets
    = 4,524 B payload dropped after 4 demux recoveries — demux
    handoff, recorded not hidden).
  * 0039 f32 packet-granularity dispute settles byte-exact on MD5.
  * 0076 inventory truth: 29 ffprobe streams (9v+18a+2 unknown strl),
    not 27; Lumen surfaces 2.
- SPEED RE-MEASURE DONE (identical decode-to-null protocol, best-of-2,
  fresh process, warmup discarded; release lumen-cli from merged HEAD;
  load 0.47→0.14; raw in docs/perf/raw/speed-v4-20261024-*):
  * avi-av (8 files): ffmpeg med 81.7 ms vs lumen 73.6 ms → 1.11×
    (was 1.18×).
  * uhd4k-mjpeg (0001, 133 f): 1.220 s vs 1.779 s → 0.686× (was 0.75×).
  * heavy-mjpeg (0127, 219.5 MB, 1338 f): 3.764 s vs 12.875 s →
    **0.292×** (was 0.16× — ≈1.8× better; Lead's V-1/input-path +
    W7-b V-3, NOT SIMD).
- BUILD FIX (flagged to 3.E3 owner): tools/bench-vs-vlc/Cargo.toml now
  enables input's `contract` feature — the Lead's V-1 gating of
  `SyntheticStream` broke the lumen_pcm digest test at merge; one-line
  fix, no behavior change.
- Gotcha for successors: nohup'd `cargo test` batches die silently
  between tool calls — use setsid; `tee | head` SIGPIPEs truncates
  files (durations.tsv needed a rerun); speed_bench needs manifest
  entries WITH a `bytes` field (synthetic mini-corpus manifest must
  copy real entries, not invent them).

Session 3 (relaunch 4):
- Clippy error FIXED (worker.rs test): `assert!(false, …)` → test now
  returns `Result<(), String>` and fails via typed `Err(format!(…))`.
  Chose typed-Err over `unreachable!`/`panic!` so BOTH clippy
  (`assertions_on_constants`) and the F-5 panic-surface scan stay green.
  Verified: `cargo clippy -p lumen-sweep --all-targets` clean, 15/15
  tests green. Committed fadd570 (with census doc).
- CENSUS DOC DONE: docs/perf/corpus-v4-sweep-20261024.md (panic census
  0/0, damage census, per-class matrix, findings, limitations).
- PARITY RAW DATA (start of deliverable #2), corpus regenerated from
  the manifest (`--replay`, 126 files verified):
  * docs/perf/raw/parity-v4-smoke-ffprobe-20261024.log — ffprobe
    (-count_frames) track inventory + decoded frame counts + duration
    for all 21 ok-class files.
  * docs/perf/raw/parity-v4-smoke-audio-pkts-20261024.log — ffprobe
    -count_packets audio-side for the 9 audio-bearing ok files.
  Headline parity findings (write up in parity doc next session):
  * mjpeg video frame counts MATCH ffmpeg exactly on every file where
    the mjpeg stream was decoded: 9,18,47,66,120,33,36,36,37,904,93,18,
    4,51,8,221,18,16,76 — all equal (17 streams).
  * silent-skip family (engine handoffs): hevc-in-avi/0074 (ffmpeg 11
    hevc frames, Lumen 0 v), 0066-avi-av (ffmpeg 8 mpeg4 frames, Lumen
    v=0/0 — mpeg4 stream skipped), 0021-multivideo-av (ffmpeg has BOTH
    mpeg4(60f) and mjpeg(120f) streams; Lumen decodes only mjpeg 120).
  * maxtracks-avi/0076: ffmpeg sees 27 streams (9 mjpeg v + 18 pcm_u8
    a); Lumen surfaces 2 tracks (first v + first a), v=8/8 a=8/8.
  * 0039-avi-av: pcm_f32le — ffprobe counts 41,377 packets vs Lumen
    a_pkts=41 (packetization granularity differs; compare duration +
    PCM MD5, not packet counts).
  * dup-range-001 (dirty): a 338 vs ffmpeg 340 packets — 2-packet loss
    after the 4 demux recoveries; riff-size-lie: exact 9/9 + 6/6.
  NEXT: PCM MD5 comparison (ffmpeg -f s16le pipe vs Lumen digest infra
  in tools/bench-vs-vlc/; 35 rows already in
  docs/corpus/vlc-diff-report-20261023.md), duration deltas, then
  docs/perf/parity-v4-20261024.md.

Session 2 (relaunch 3) ran the FIRST REAL corpus-v4 smoke sweep:

- corpus regenerated to `/tmp/corpus-v4-smoke` via `--replay` (all
  126 manifest files verified on disk, 77.4 MB; hls/0048 + hls/0067
  carry extra segment/playlist files NOT in the 126 — don't walk
  the corpus root).
- exact-126 file list extracted from the manifest `rel` fields
  (`/tmp/v4smoke-files.txt`, still there) and passed as explicit
  args; binary at /home/z/my-project/target/release/lumen-sweep.
- RAW LOG COMMITTED: docs/perf/raw/sweep-v4-smoke-20261024.log
  (force-add: docs/perf/raw/*.log is gitignored).
  TALLY: 126 files — 21 ok / 103 unsupported / 1 dirty-reject /
  1 typed-error / **0 panics / 0 timeouts**.
- Notable lines for the census doc:
  * odd-chunk-avi → typed-error `sweep.concealment.budget`
    (1000/1000 concealment budget, v=1000 concealed) — check the
    budget is doing what doc intends.
  * avi-magic-head-03 (garbage) → dirty-reject `demux.avi.truncated`
    (expected 425516 B, found 47) — typed/bounded, good.
  * hevc-in-avi 0074 → "ok" but 0 video frames / 7 audio frames —
    HEVC stream silently skipped, audio-only decode. First-vertical
    truth; census should call this out as coverage, maybe handoff.
  * dirty byte-op classes on mp4/mkv targets land `unsupported`
    (no demuxer for those containers) — honest, not rejection.

Session 1 (relaunch 2) committed the skeleton: supervisor + worker,
subprocess-per-file isolation, timeout, 6-way classification,
per-class tallies, 15 unit tests green, smoke-validated on the
orphaned /tmp/corpus-big leftover (ok / dirty-reject / unsupported
all firing; 0 panics, 0 timeouts on that partial set — NOT official
census data).

## Remaining for the card (in order)

1. Regenerate corpus-v4 smoke (126 files):
   there is NO `--manifest` flag — regeneration of a committed
   manifest goes through `--replay` (re-runs the manifest's seed
   verbatim):
   `python3 tools/corpus/gen_corpus.py --replay
   docs/corpus/corpus-v4-smoke-manifest-20261023.json --out /tmp/<corpus-out> --jobs 8`
   (~45 s, ~66 MiB for the smoke manifest).
2. `cargo run -p lumen-sweep -- --timeout-secs 30 /tmp/<corpus-out>`
   → save stdout as the raw log (docs/perf raw data).
3. Census tables (panic census: any Panic outcome's crash_note is
   the handoff material; damage census: v_concealed/a_concealed/
   demux_recovered columns).
4. Parity vs ffmpeg CLI (ffprobe/ffmpeg: duration, track inventory,
   frame counts, PCM MD5, seek accuracy per doc 10 §6.6) →
   docs/perf/parity-v4-20261024.md.
5. Speed re-measure the 3 canonical scenarios (avi-av 1.18x,
   uhd4k-mjpeg 0.75x, heavy-mjpeg 0.16x; baseline
   docs/perf/speed-baseline-20261022.md) under IDENTICAL protocol.
6. docs/perf/corpus-v4-sweep-20261024.md + worklog entry (only when
   the whole card is done or hard-blocked).

## Known gaps / notes

- Class derivation is filename-based (`NNNN-<class>.<ext>`,
  `outcome::class_of`). The v3-named media in the leftover corpus
  yields compound classes (`dup-range-006-0403-dynres-video`); the
  v4 smoke corpus naming is clean, but consider adding a
  `--manifest` flag that reads the manifest's `class` field
  directly (1:1 with the generator) before writing the report.
- Non-AVI containers honestly land in `unsupported`
  (sweep.container.no_demuxer / probe.no-demuxer) — first-vertical
  truth, expected to dominate most classes; the census reports it
  as coverage, not failure.
- Worker protocol: 15 tab-separated fields after `SWEEP`
  (see outcome::parse_worker_line); supervisor lines are
  `SWEEPLINE` + 15 fields (see sweepline()).
- boundaries.toml has no [crates.lumen-sweep] entry (Lead union
  handoff, same as mem-audit's was).
