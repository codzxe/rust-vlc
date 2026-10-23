# Corpus-v4 smoke sweep — census (2026-10-24)

Card 7-d. Raw data: `docs/perf/raw/sweep-v4-smoke-20261024.log` (committed,
force-added; `docs/perf/raw/*.log` is gitignored). Tool: `lumen-sweep`
(subprocess-per-file isolation, 30 s timeout, 6-way outcome classification).

## Protocol

- Corpus: **corpus-v4 smoke** — 126 files regenerated from the committed
  manifest `docs/corpus/corpus-v4-smoke-manifest-20261023.json` via
  `gen_corpus.py --replay` (own seed), exact-126 file list passed as explicit
  args (~77.4 MB on disk; hls/0048 + hls/0067 carry extra segment/playlist
  files not counted here).
- Run: `lumen-sweep --timeout-secs 30 <explicit 126 paths>`, single pass,
  wall time per file recorded in field 15 of every `SWEEPLINE`.
- Field order: `SWEEPLINE class rel status stage kind code v_pkts/v_dec
  a_pkts/a_dec v_concealed/a_concealed demux_recovered tracks limbs container
  secs detail`.
- Load conditions: shared 5-agent sandbox; only this sweep ran during the
  pass (per-file wall times are all ≤ 0.63 s, dominated by process spawn).

## Headline tally (126 files)

| Outcome | Count |
|---|---|
| ok | 21 |
| unsupported | 103 |
| dirty-reject | 1 |
| typed-error | 1 |
| panic | **0** |
| timeout | **0** |

## Panic census: zero panics, zero timeouts

**0 panics and 0 timeouts across all 126 files, including 14 dirty /
21 garbage / 2 hls adversarial inputs.** Every file that would historically
have been a crash-or-hang risk resolved through a typed, bounded channel
instead. The would-be panics, and what caught them:

| Would-be panic risk | What actually happened | Disposition |
|---|---|---|
| `odd-chunk-avi` (media/0035): adversarial chunking drives concealment without bound — classic OOM/infinite-loop panic shape | typed-error `sweep.concealment.budget` at decode stage: `resource limit: memory (budget 1000, attempted 1000)` — v_pkts=1000, v_dec=0, v_concealed=1000 | **Budget fired as designed.** Engine handoff: verify the budget message semantics (1000 concealed, 0 decoded, `budget 1000, attempted 1000` reads as "attempted == budget", i.e. the *next* concealment is refused — confirm intent with engine owner) |
| `avi-magic-head-03` (garbage): truncated AVI header claims 425,516 B, file has 47 B — OOB-read panic shape | dirty-reject `demux.avi.truncated` (corrupt), typed + bounded, 0.03 s | Clean rejection; no action |
| 21 garbage `.bin` files (random bytes, all-ff, all-zero, empty, 1-byte, wrong-magic) | all bounded: 20 `unsupported` at probe (`probe.no-demuxer`, sniff shown in log), 1 dirty-reject above | No crash surface found |
| 12 dirty byte-op mutations on non-AVI containers (mkv/mp4/ts/wav/ogg/aac/flv/m3u8/flv) | all `unsupported` (`probe.no-demuxer` / `sweep.container.no_demuxer`) — the mutated container is refused before any parse depth | Honest, not a rejection; see coverage below |

No `Panic` outcome, no `crash_note`, no worker subprocess exit-code anomaly in
the raw log. **Zero engine handoffs from crashes.** The two handoffs below are
behavioral, not stability, findings.

## Damage census (dirty + garbage)

| Input class | Files | ok | unsupported | dirty-reject | typed-error |
|---|---|---|---|---|---|
| dirty/ (byte-op mutations) | 14 | 2 | 12 | 0 | 0 |
| garbage/ (magic/lie/junk) | 22 | 0 | 21 | 1 | 0 |
| hls/ (playlists) | 2 | 0 | 2 | 0 | 0 |

Damage handling on AVI targets (the only containers with a demuxer):

- `riff-size-lie` → **ok**: 9/9 v-packets, 6/6 a-packets decoded, 0
  concealed, **1 demux recovery** — the size lie was detected and recovered,
  decode unaffected. 
- `dup-range-001-0063-avi-av` → **ok**: 18/18 v, 338/338 a, 0 concealed,
  **4 demux recoveries** — duplicated range detected and skipped, no decode
  damage.
- All 12 non-AVI dirty targets land `unsupported` *before any mutation is
  exercised* (no demuxer for that container), so the dirty-byte resistance of
  the AVI demuxer is exercised by exactly 2 of 14 dirty files. The other 12
  are honest coverage gaps, not passes.

## Per-class pass/fail matrix (media/ classes, 90 files)

First-vertical truth: only the AVI demuxer exists. `unsupported` therefore
dominates every non-AVI class and is reported as **coverage state**, not
failure.

| Class | Files | ok | unsup. | other |
|---|---|---|---|---|
| avi-video | 17 | 11 | 6 (demux.avi.no-supported-streams) | — |
| avi-av | 4 | 4 | 0 | — |
| audio-only | 14 | 0 | 14 | — |
| mkv-video / muxed-av | 5 | 0 | 5 | — |
| mp4-video / bframe-storm / meta-bomb-av / vbv-burst / packed-bs(mp4) | 8 | 0 | 8 | — |
| cover-audio | 3 | 0 | 3 | — |
| vfr-av | 3 | 0 | 3 | — |
| param-midchange / bit-drift(ts) | 4 | 0 | 4 | — |
| raw-video (h264) / 265 | 2 | 0 | 2 | — |
| flv-video / fmp4-video / mov-video / webm-video / ogv-video / ts-video | 9 | 0 | 9 | — |
| sr-chaos / dynres-video / fps-extremes / drift-av / multi-av / rot-display | 12 | 0 | 12 | — |
| chapter-bomb / still-24h / zeroframe-stream / m3u8 classes | 6 | 0 | 6 | — |
| multivideo-av | 2 | 1 | 1 | — |
| odd-dims | 2 | 1 | 1 (no-supported-streams) | — |
| hevc-in-avi | 1 | 1 | — | — (see findings) |
| maxtracks-avi | 1 | 1 | — | — |
| odd-chunk-avi | 1 | — | — | 1 typed-error |
| (garbage 22 + dirty 14 + hls 2 + master/hls-playlists) | 38 | 2 | 35 | 1 dirty-reject |

Unsupported reason breakdown (103): `probe.no-demuxer` 75,
`sweep.container.no_demuxer` 20 (mp4/m4a sniffed but no demuxer), 
`demux.avi.no-supported-streams` 8 (AVI parsed, no decodable stream).

## Interesting findings

1. **hevc-in-avi (media/0074) silently skips the video stream** — outcome
   `ok` with v_pkts/v_dec = **0/0**, a_pkts/a_dec = 7/7, 1 demux recovery,
   limbs = a. An HEVC-in-AVI file reports success while decoding only audio.
   This is the sweep's most consequential find: an `ok` that is not really
   ok. **Engine handoff**: a stream present in the container but with no
   decoder should surface as a visible warning/typed notice, not a silent
   skip (user hits it as "plays but no picture").
2. **odd-chunk-avi concealment budget fires** — first observed live
   activation of the `sweep.concealment.budget` resource limit (1000/1000,
   typed-error, no panic, 0.03 s). The *mechanism* works; the *message*
   wording deserves an engine-owner look (see panic census table).
3. **8 AVI files parse but have "no supported streams"** — 6 avi-video +
  packed-bs/0022 + odd-dims/0031. Codec-level coverage (not container-level)
   decides these; they are honest unsupported, feed Phase-5 codec work.
4. **103/126 unsupported is the honest container-coverage state** — AVI is
   the only demuxer in the first vertical; mp4-family sniffs (20 files) get
   as far as format identification before refusing. This number is the
   baseline the Phase-5 container work must move.

## Honest limitations

- **Smoke corpus only**: 126 files (corpus-v4 smoke seed), *not* corpus-big
  (485 files, seed 58007…, ~17 min regen). The full corpus-big sweep remains
  open.
- **Single seed**: one draw of the smoke generator; no multi-seed variance.
- Dirty-byte resistance of the AVI demuxer is exercised by only 2 files
  (riff-size-lie, dup-range-avi) — the other 12 dirty targets are non-AVI.
- Parity (ffprobe/ffmpeg frame counts, PCM MD5) and speed re-measure are
  separate deliverables (parity-v4 / speed re-run), not covered here.

## Next actions

1. Parity: for the 21 ok-class files, ffmpeg-side comparison (ffprobe
   duration/track inventory, decoded frame counts, PCM MD5 where
   lossless-comparable) → `docs/perf/parity-v4-20261024.md`.
2. corpus-big (485-file) sweep once runtime budget allows.
3. Speed re-measure of the 3 canonical scenarios vs ffmpeg under the
   IDENTICAL 20261022 protocol.

---

## Addendum — corpus-big sweep (2026-10-24, card 7-h)

Closes the "corpus-big sweep remains open" limitation above. Raw data:
`docs/perf/raw/sweep-corpus-big-20261024.log` (force-added;
`docs/perf/raw/*.log` is gitignored).

### Protocol

- Corpus: corpus-big regenerated at seed 58007138736976908607880252061
  (`gen_corpus.py --preset standard --jobs 2`, exit 0; generator report
  "TOTAL 587 files, 606 MiB"; clean phase 493 files + 20 rejected
  draws; 1 ffprobe validation warning recorded in the manifest:
  media/0336-cover-audio.mp3).
- Run: `lumen-sweep --timeout-secs 30` over the four class dirs
  (media/dirty/garbage/hls) = **538 files** (whole-tree walk is 540
  including manifest.json + plan.json; the generator's 587 is its own
  plan accounting). Single pass, wall **57 s**.
- Binary: fresh release build from wip/w7-h @ main 171d0a7 — the
  compile log shows every engine crate compiled from this worktree
  (12.5 s), no stale shared-target rlib. Load conditions: idle sandbox
  (loadavg 0.08, no other cargo/ffmpeg running).

### Headline tally — big (538) vs smoke (126)

| Outcome | big | big % | smoke | smoke % |
|---|---|---|---|---|
| ok | 82 | 15.2% | 21 | 16.7% |
| unsupported | 446 | 82.9% | 103 | 81.7% |
| dirty-reject | 9 | 1.7% | 1 | 0.8% |
| typed-error | 1 | 0.2% | 1 | 0.8% |
| panic | **0** | 0% | **0** | 0% |
| timeout | **0** | 0% | **0** | 0% |

**Rates hold at 4.3× scale: 0 panics, 0 timeouts across 538 files**
(77 dirty + 22 garbage + 50 hls adversarial inputs). The unsupported
share is unchanged (82.9% vs 81.7%) — the honest AVI-only
container-coverage state; corpus-big surfaces no new crash shape.

Notes: (a) per-class: avi-av 25/25 ok; avi-video 30 ok / 43
unsupported; audio-only 0/43. (b) dirty-reject 1→9: the bigger dirty
sample reaches more AVI damage shapes (4 truncated, 2
walk-resync-failed, 2 riff-size-lie no-movi, 1 garbage truncated) —
all typed + bounded, consistent with the smoke's single case.
(c) typed-error 1 = the odd-chunk-avi concealment-budget shape
(media/0214, resource-limit). (d) slowest file media/0339-avi-video.avi
at **25.5 s** (ok) — ~10× the next-slowest (2.7 s) and near the 30 s
deadline; smoke max was 0.63 s. **Engine handoff**: profile 0339's
decode path before any larger corpus run.
