# tools/sweep — card 7-d session breadcrumbs

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
