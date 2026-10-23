# NEXT-STEPS — Task 7-h: bench-vs-vlc verify + corpus-big sweep (2026-10-24)

Branch `wip/w7-h` (from main @ 171d0a7). Commit ca5db91 = bench pin fix.

## Item 1 — bench-vs-vlc contract-feature verification: DONE, GREEN

- 7-d's `features = ["contract"]` wiring on the `input` dep
  (tools/bench-vs-vlc/Cargo.toml:52) **HOLDS**: the standalone crate
  compiles and the `SyntheticStream` fixtures drive (V-1 gating is not
  the problem).
- First run on this base: 8 passed / **1 FAILED** / 1 ignored. The
  failure was a stale pre-W5-14 expectation in
  `filtered_video_stream_does_not_touch_the_audio_digest` (pinned
  2 Recovered events + 32 skip bytes). W5-14 (3b1a9e2, merge 90fb182)
  has since landed: filtered-stream idx1 entries are a SILENT skip, so
  this build emits **1** Recovered (codec-filter decision, skipped 0)
  and **0** skip bytes. Test aligned (digest assertions untouched);
  corroborated by reading the engine's own pin
  `crates/demux/tests/idx1_stream_scope.rs::
  filtered_stream_idx1_entries_are_silent` (read, not executed).
- **Final: 9 passed / 0 failed / 1 ignored** (`corpus_batch_via_env`
  is env-driven via tools/bench-vs-vlc/run.py — ignored by design);
  0 doc-tests. Freshness: `Compiling bench-vs-vlc
  (worktrees/w7-h/tools/bench-vs-vlc)` — no stale shared-target rlib.
- No bin mode (lib-only crate) — test suite is the smoke surface.
- **Handoff: 3.E3 owner should re-bless the W5-14 expectation update.**

## Item 2 — corpus-big generation: COMPLETE (this session, verified)

Command: `python3 tools/corpus/gen_corpus.py --preset standard --out
/tmp/corpus-big --seed 58007138736976908607880252061 --jobs 2`

- A prior session had 493 clean files in place; the resume pass
  finished in 6 s → exit 0, tool report: `TOTAL 587 files, 606 MiB,
  wall 6s`, clean phase 493 files + 20 rejected draws.
- 1 ffprobe validation warning (recorded in manifest):
  `media/0336-cover-audio.mp3`.
- On-disk recursive walk: **540 files** (top-level: media 391, dirty
  77, garbage 22, hls 50, plus `manifest.json`, `plan.json`, `_tmp/`).
  The 587-vs-540 delta = tool-side plan counting; the sweep's own
  discovered-file TOTAL (its summary block) is the census denominator.
- Background-proc reaping (7-g's finding) reconfirmed: `setsid nohup`
  gen died at tool-call end; foreground resume completed it.

## Item 3 — corpus-big sweep: see below / successor

Attempt launched from this session after this file was committed. If
no follow-up commit updates this section, the attempt died with the
session — re-run (corpus in /tmp persists across sessions here):

```
cd /home/z/my-project/worktrees/w7-h
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release -p lumen-sweep        # shared target; watch Compiling lines
/home/z/my-project/target/release/lumen-sweep --timeout-secs 30 /tmp/corpus-big \
  > docs/perf/raw/sweep-corpus-big-20261024.log 2>&1
git add -f docs/perf/raw/sweep-corpus-big-20261024.log   # *.log gitignored
```

Then: short addendum section in `docs/perf/corpus-v4-sweep-20261024.md`
(smoke 126 vs big census rates), and the 7-h worklog entry (both items
then complete). Load conditions for the census: record `uptime` +
running cargo/ffmpeg processes alongside the run.
