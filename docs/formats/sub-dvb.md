# Subtitle record — DVB bitmap subtitles (ETSI EN 300 743, self-build)

Status: **implemented (win/linux/macos; web not_started)** — card 5.N2,
ADR-012 self-build decision (no crate: ffmpeg/dvbsubstr-closed, C —
forbidden by §9). Commits ab696b4 + 93a53ec. Feature-matrix row
`subtitle-dvb-subs` was backfilled to implemented by 5.Q2-V with the
honest gap "NO docs/formats record" (audit gap **G-1**) — this file
closes it. Front-matter keys per doc 04 §6:

- id = `subtitle-dvb-subs` (`docs/feature-matrix.toml`)
- status = implemented (parse + decode + compositing reduce; no PMT
  descriptor plumbing of its own — pairs with the TS demuxer's
  5.I1/5.REC-1 track plumbing, which owns PMT walk + stream routing)
- phase = 5 (landed, wave 6 card 5.N2)
- fuzz_target = `fuzz_sub_dvb` (in-crate harness `src/dvb/fuzz.rs`;
  cargo-fuzz absent, Lead-owned wiring)
- crates = [] — zero external deps
- self_built = [`crates/subtitles/src/dvb/**` (mod/segment/rle/clut/
  fuzz)]
- last_audit = 2026-10-26 (G-1 record card; engine evidence 5.N2)

## Working subset (what "implemented" means)

- **Segments** (EN 300 743 §7.2): 6-byte header walk — page
  composition (§7.2.2), region composition (§7.2.3), CLUT definition
  (§7.2.4), object data (§7.2.5), display definition (§7.2.1),
  end-of-display-set, stuffing; unknown type bytes skipped + counted.
- **Pixel RLE** (§7.2.5): 2/4/8-bit tables with the map-table
  translation for cross-depth objects and the non-modifying colour
  rule; line-based sub-blocks (0x10/0x11/0x12 data types, `0xF0`
  two-row field interleave).
- **CLUTs**: versioned banks, BT.601-headroom YCbCr→RGBA conversion
  (integer-identical to the reference decoders), 2/4/8-bit default
  banks as the `BadClutId` fallback.
- **Page state machine** (§7.3): page versions; `page_state` 1
  (acquisition) / 2 (mode change) reset regions/objects/CLUTs;
  repeated version = no-op; composition at end-of-display-set (or
  once page+region+object are all seen when the stream omits it);
  empty page composes to zero regions (display clears).
- **Contract**: [`DvbDecoder::decode_pes`] takes one subtitle PES
  payload + its 90 kHz PTS → [`DvbPageUpdate`] (region images for
  THIS display set, page timeout, capped typed damage ledger);
  reduces to the 5.L1 [`CompositedCue`] contract. Zero panics;
  bounded: damage ledger 64 events (overflow counted), region pixel
  buffers 4 MiB.

## Evidence (what was checked, 5.N2 + this audit)

- **Unit suites**: 38 tests across mod/segment/rle/clut (hand-built
  bit vectors; bit tables cross-checked line-by-line against
  ffmpeg's `dvbsub_read_{2,4,8}bit_string` RLE semantics).
- **Fuzz-lite survival** (`tests/dvb_vobsub_fuzz_survival.rs`):
  seeded LCG mutation + truncation sweeps over synthetic-but-valid
  display sets through `fuzz_sub_dvb` — zero panics/hangs/unbounded
  allocations; ledger cap respected; every RGBA buffer exactly
  `width × height × 4`.
- **No reference-encoder differential (honest)**: ffmpeg 7.1.5
  carries `dvbsub` but refuses text→bitmap transcoding ("text to
  text or bitmap to bitmap"), and the sandbox has no
  DVB-bearing corpus lane — the differential is NOT generatable
  `[VERIFY if a corpus lane ever lands]`.
- **PMT plumbing**: owned by the MPEG-TS demuxer (5.I1; multi-program
  TS = synthetic-only per its record) — not re-tested here.

## Deferred (honest)

`web` target not_started (no wasm lane yet); no live-TS integration
test (demux↔subtitle seam is the Lead's wiring); object clipping at
region edges is clip-and-count (no wrap); timeouts beyond the page
timeout (PTS discontinuity policy) follow the compositing clock, not
this parser.

## Results / next steps

Gates at the 5.N2 card: 230/0 across `cargo test -p subtitles`
(incl. the shared dvb_vobsub survival file); current tree re-verified
green at this G-1 record commit. Next: (1) corpus lane with a real
`.ts` + DVB track for the deferred differential; (2) Lead wires
`DvbDecoder` into the TS track path (PES assembly already exists
demux-side); (3) recheck crates.io at Phase 7 R-10 — currently zero
pure-Rust EN 300 743 decoders, so the self-build stands.
