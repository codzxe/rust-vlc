# Subtitle record — VobSub / DVD sub-picture (IDX/SUB, self-build)

Status: **implemented (win/linux/macos; web not_started)** — card 5.N2,
ADR-012 self-build decision (the CC0 `vobsub` 0.2.3 crate was
verified as a path but NOT taken: self-build keeps the damage-ledger
discipline and zero-dep stance; recorded here so R-10 can re-litigate).
Commits ab696b4 + 93a53ec. Feature-matrix row `subtitle-vobsub` was
backfilled to implemented by 5.Q2-V with the honest gap "NO
docs/formats record" (audit gap **G-1**) — this file closes it.
Front-matter keys per doc 04 §6:

- id = `subtitle-vobsub` (`docs/feature-matrix.toml`)
- status = implemented (idx + sub parse, SPU decode, palette render,
  compositing reduce; SVCD/CVD siblings NOT covered — see Deferred)
- phase = 5 (landed, wave 6 card 5.N2)
- fuzz_target = `fuzz_sub_vobsub` (in-crate harness
  `src/vobsub/fuzz.rs`; cargo-fuzz absent, Lead-owned wiring)
- crates = [] — zero external deps
- self_built = [`crates/subtitles/src/vobsub/**` (mod/idx/spu/fuzz)]
- last_audit = 2026-10-26 (G-1 record card; engine evidence 5.N2)

## Working subset (what "implemented" means)

- **`.idx` text parse**: palette, size, per-stream timestamp/filepos
  entries, delays, tolerances — typed `IdxDamage`, lenient
  whitespace/case.
- **SPU grammar** (`.sub` half): display-control command sequence
  (forced display, start/end dates, palette/chroma/alpha
  assignments, command-area crop), the nibble-RLE interlaced bitmap,
  the MPEG-PS PES walker; multi-PES SPUs accumulate exactly like the
  reference reconstruction buffer, bounded by the entry's byte span
  (distance to the next idx filepos) and a 128 KiB cap.
- **Pairing** (this module): per-entry PES accumulation, SPU decode,
  palette→RGBA with the reference's grayscale fallback when the idx
  lacks a palette, the transparent-border crop the reference applies,
  and the reduce to the 5.L1 [`CompositedCue`] contract.
- **Timing**: cue span = idx timestamp + SPU start/end dates
  (`(date << 10) / 90` ms, half-open `[t0, t1)` at the compositor);
  an SPU without an end date is open-ended (the next entry on the
  same stream implicitly replaces it — the reference's
  subtitle-queue semantics).
- **Carriage**: the PS demuxer already routes `0xBD` substream
  `0x20..=0x3F` as `"dvd_subtitle"` (landed with 5.N2's demux half).

## Evidence (what was checked, 5.N2 + this audit)

- **Unit suites**: 15 tests across mod/idx/spu (hand-built idx/sub
  pairs; RLE semantics cross-checked line-by-line against ffmpeg's
  `dvdsub` decoder tables).
- **Fuzz-lite survival** (`tests/dvb_vobsub_fuzz_survival.rs`, shared
  with DVB): seeded LCG mutation + truncation sweeps over
  synthetic-but-valid idx/sub pairs through `fuzz_sub_vobsub` — zero
  panics/hangs/unbounded allocations; the 128 KiB SPU cap never
  exceeded; every cue's RGBA buffer exactly `width × height × 4`.
- **No reference-encoder differential (honest)**: ffmpeg 7.1.5
  refuses text→bitmap transcoding (same wall as DVB), and no
  VobSub-bearing corpus lane exists in the sandbox `[VERIFY if a
  corpus lane ever lands]`.
- **Crate survey**: `vobsub` 0.2.3 (CC0) parses idx+spu — verified as
  an alternative path, deliberately not adopted (see Status).

## Deferred (honest)

**SVCD/CVD** siblings (`svcdsub`/`cvdsub` in VLC's module list) are
NOT implemented — separate grammars, tracked by the matrix row's
scope note; this record covers IDX/SUB only. `web` target
not_started. DVD-Video navigation (menu-driven sub-picture streams)
is blocked separately (platform-disc row). No real-`.idx` corpus
differential yet.

## Results / next steps

Gates at the 5.N2 card: 230/0 across `cargo test -p subtitles`;
current tree re-verified green at this G-1 record commit. Next:
(1) corpus lane with a real VobSub pair for the deferred
differential; (2) SVCD/CVD parsers if share data ever justifies them
(feasibility subtitles §1 row 11 covers VobSub only); (3) recheck
`vobsub` crate maintenance at Phase 7 R-10 — the CC0 option remains
open for a swap if the self-build ever lags.
