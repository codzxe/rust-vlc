# tools/sweep — card 7-d session breadcrumbs

Session 1 (relaunch 2) committed the skeleton: supervisor + worker,
subprocess-per-file isolation, timeout, 6-way classification,
per-class tallies, 15 unit tests green, smoke-validated on the
orphaned /tmp/corpus-big leftover (ok / dirty-reject / unsupported
all firing; 0 panics, 0 timeouts on that partial set — NOT official
census data).

## Remaining for the card (in order)

1. Regenerate corpus-v4 smoke (126 files):
   `python3 tools/corpus/gen_corpus.py --help` FIRST (find the
   manifest flag), manifest
   `docs/corpus/corpus-v4-smoke-manifest-20261023.json`, out to
   /tmp NOT the repo.
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
