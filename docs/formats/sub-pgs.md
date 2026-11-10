# Subtitle record — PGS (HDMV / Blu-ray) parse + decode

Records-consolidation entry (card 8.G1, issue #58 — the format-records
audit found the implemented `subtitle-pgs` matrix row had no
docs/formats record; engine evidence from card 5.N1, ADR-012
self-build). Front-matter keys per doc 04 §6:

- id = `subtitle-pgs` (matches `docs/feature-matrix.toml`; row
  status `implemented` on win/linux/macos, `not_started` on web, with
  evidence `crates/subtitles/src/pgs/**` — this record consolidates
  it)
- status = **implemented** (segment grammar, epoch state machine, RLE
  + palette decode to straight-alpha RGBA, reduce to the compositing
  contract — the ADR-012 self-build, zero dependencies)
- phase = 5 (card 5.N1; in tree by the 5.R2-V Phase-5 exit `3116aec`)
- fuzz_target = `fuzz_sub_pgs` (in-crate survival harness
  `crates/subtitles/src/pgs/fuzz.rs` + the integration survival
  suite `crates/subtitles/tests/pgs_fuzz_survival.rs`)
- crates = [] — self-built, std-only
- self_built = [`subtitles::pgs` (`segment`/`palette`/`rle` + the
  epoch state machine in `mod.rs`)]
- last_audit = 2026-11-10 (this consolidation; engine evidence from
  card 5.N1, re-run this session — below)

## Parse / Decode / Render

- **Parse + decode (landed)**: PCS/WDS/PDS/ODS/END segments in BOTH
  framings (`.sup` PTS-prefixed and Matroska-stripped); the epoch
  state machine (acquisition/epoch-start flush, object and palette
  banks, multi-fragment ODS assembly); the PGS RLE codes (literal
  pixels, `0x00` escapes: EOL, 1–63 runs, 16-bit long runs) with
  typed clip/snap damage; PDS 16-bit YCbCrA → straight-alpha RGBA8
  with BT.601-vs-BT.709 selection by video height (the reference
  decoder's rule).
- **Render**: reduce to `compositing::CompositedCue` — the 5.L1
  compositing contract. The **renderer overlay contract is still
  missing** (R-27): compositing proof onto live video is pending,
  which is why ADR-012's landing tracker stays open (see
  `docs/KNOWN-LIMITS.md` §3).

## Evidence

- Module: `crates/subtitles/src/pgs/` — the module header carries
  the ownership map and the epoch lifecycle model.
- Matrix row evidence quotes: "32 unit tests + fuzz_sub_pgs survival
  harness, all green in this tree 2026-10-26" (5.R2-V exit).

## Gates (this consolidation, exact commands, run 2026-11-10)

Shared-target worktree `wt/w24e` @ `c339ee8`, 2-core Linux sandbox:

- `cargo test --locked -p subtitles --lib pgs` — **32 passed /
  0 failed**.
- `cargo test --locked -p subtitles --test pgs_fuzz_survival` —
  **2/0**.

## Known gaps / honest limits

- No renderer-side overlay proof yet (R-27; ADR-012) — parse/decode
  to composited cues is landed, on-screen compositing is not.
- Linux-only evidence (program-wide limit, `docs/KNOWN-LIMITS.md`
  §1); web column `not_started`.
