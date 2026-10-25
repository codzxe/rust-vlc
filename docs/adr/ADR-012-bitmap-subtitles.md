# ADR-012: Bitmap + closed-caption subtitle strategy (N-wave unlock)

- **Status:** ACCEPTED — Lead signature 2026-10-24 (wave-9 N-cards authorized; the original draft framing
  unblocks the `[CONDITIONAL — bitmap/CC decision]` gate on doc 13 wave 5.N
  cards 5.N1–5.N3)
- **Owner:** Lead decision. Drafted by P5-NADR (card 5.N-ADR, D-10,
  investigation — no bitmap code).
- **Evidence:** live crates.io API checks (curl, 2026-10-24: exact-name,
  family searches, per-version `/dependencies`, licenses, download counts —
  all quoted below); in-tree carriage verified by file:line; doc 04 §5 rows
  9–12 + §5 "bitmap compositing dependency" + §6 phase-6 row;
  `docs/feasibility/sections/subtitles.md` §1 rows 9–12, §3 (bitmap
  compositing), §4 (CC extraction); doc 13 §wave 5.N (rows 5.N1–5.N3);
  feature-matrix rows `subtitle-pgs` / `subtitle-dvb-subs` /
  `subtitle-cc-608-708` / `subtitle-vobsub`.

## The gate

Doc 13's wave 5.N (N1 PGS, N2 DVB+VobSub, N3 CC-608/708) is `[CONDITIONAL]`
on this decision. The question: self-build all three decoders, or hold them
blocked pending crate ecosystem maturation?

## What has already landed (the sunk-cost half)

- **Compositor contract (5.L1):** `CueBitmap` — validated straight-alpha
  RGBA8 + baseline (`crates/subtitles/src/compositing/bitmap.rs:87`,
  `from_rgba` stride/baseline checks) — and `CompositedCue` (bitmap +
  `CueLayout` + z + ns-converted span, `bitmap.rs:190`). SRT/ASS already
  reduce to this; bitmap formats reduce to the *same* type. This is the
  integration point for all of N1–N3.
- **VobSub carriage:** PS demuxer routes private-stream-1 substream
  `0x20..=0x3F` → `PrivateKind::Spu` → `"dvd_subtitle"` `MediaKind::Subtitle`
  (`crates/demux/src/ps/mod.rs:151,171`; test-pinned `ps/tests.rs:250`).
- **PGS-in-TS carriage:** stream type `0x90` → `"pgssub"` subtitle
  (`crates/demux/src/ts/mod.rs:364,381`); DVB subs ride `0x06`
  `"ts-private-pes"` (`ts/mod.rs:354`).
- **H.264 SEI framework (5.F1):** typed `user_data_unregistered` (type 5)
  parse; every other payload type — including type 4
  `user_data_registered_itu_t_t35`, where ATSC captions live — is captured
  as `Unparsed{type,size}` and skipped intact, never dropped
  (`crates/codecs/src/video/h264/parser/sei.rs:24-26,148`).
- **Not landed (honest):** MKV codec map lacks `S_HDMV/PGS`, `S_VOBSUB`,
  `S_DVBSUB` (falls to raw codec-id, `demux/src/mkv/header.rs:784-787`);
  MPEG-2 picture-header `user_data` has no parser (grep: zero hits in
  `codecs/src/video/mpeg12`); VBI/line-21 decode is absent (out of v1 scope —
  no analog input path exists).

## Per-format evidence

### PGS (HDMV, Blu-ray) — self-build M

- **Spec:** no free ISO text, but the segment grammar (PCS/WDS/ODS/END +
  RLE object data) is publicly documented in mkvtoolnix / mpv / ffmpeg
  sources; doc 04 row 9 already rates parse "S". Open item: palette is
  YCbCr in some streams → RGB conversion [VERIFY — matrix note].
- **crates.io (2026-10-24):** `pgs` = **name collision** (Pokémon Simulator
  save files, 2022, 1,787 dl). Real candidates, none above the feasibility
  §1.3 adoption bar today: `libpgs` 0.6.0 (MIT OR Apache-2.0, 2026-04-17,
  4,031 dl, extraction+encoding+roundtrip, dep `flate2 ^1` — pure only if
  miniz_oxide backend, must be verified); `pgs-parse` 0.1.2 (MIT, 2024-09-23,
  1,282 dl, parse-only, dep `log`); `pgs-rs` 0.1.1 (MIT, 2025-11-28, 598 dl,
  deps struple/thiserror/winnow/yuv); `oxideav-sub-image` 0.0.9 (MIT,
  2026-04-17, 1,121 dl — "Pure-Rust bitmap-subtitle decoders: PGS, DVB,
  VobSub", dep `oxideav-core`; the oxideav 0.0.x family has a scaffold
  history per 5.K3-I/5.P2a-I); `libbitsub` 1.13.0 (130 dl, WASM renderer,
  young).
- **Decode:** RLE→bitmap + palette→RGBA mapped 1:1 onto `CueBitmap::from_rgba`
  + epoch state machine (PCS-driven display sets, window composition).
  Effort M; the actual risk is timing correctness across seeks (epoch
  reconstruction), already named by feasibility §3.3.

### DVB subtitles (ETSI EN 300 743) — self-build M

- **Spec:** EN 300 743 is a free ETSI download; page/region/object model +
  RLE inside subtitle PES; needs PMT descriptor/subtitle-PID plumbing that
  pairs with the landed TS demuxer.
- **crates.io (2026-10-24):** `dvbsub` NOT_FOUND. NEW since the 2026-10-19
  survey: `dvb-subtitle` 0.6.0 (MIT OR Apache-2.0, created 2026-06-19,
  8 releases, latest 2026-10-17, 476 dl, no-std, "DVB subtitling (bitmap)
  segment parser — ETSI EN 300 743", repo fishloa/rust-broadcast; deps
  `broadcast-common ^10` + `thiserror` + optional serde — parse-only).
  Active but young and parse-only: below the adoption bar, above the
  recheck trigger.
- **Decode:** same RLE mechanics as PGS (shared core), plus region
  composition/CLUT and page-acquisition-time-vs-PTS subtleties. Effort M.

### VobSub (IDX/SUB) — self-build S–M (carriage already landed)

- **Spec:** de-facto format documented in ffmpeg/mkvtoolnix; `.idx` carries
  palette + timestamps, `.sub` is MPEG-PS PES with RLE SPU segments — the
  PS demuxer already routes those PES payloads (above).
- **crates.io (2026-10-24):** `vobsub` 0.2.3 re-confirmed (CC0-1.0, 2017,
  47,369 dl, emk/subtitles-rs) — but deps are 2017-era (`error-chain 0.10`,
  `nom 2`, `image 0.13`, `regex 0.2`, `safemem 0.2`, `log 0.3`): pure yet
  archaeic, unmaintained 9 years; doc 04 row 11's "S (harden)" understates
  the dependency drag of vendoring that stack. `subtile-ocr` is the OCR
  path (different problem). `gst-plugin-vobsub` = placeholder.
- **Decode:** control-sequence + RLE→RGBA; palette from IDX. Smallest of
  the three (SPU packet grammar is compact). Effort S–M.

### CC 608/708 (CEA/CTA captions) — self-build M (608) then M–L (708), staged

- **Specs:** CTA-608-E / CTA-708-D are purchasable, not free full-text
  [VERIFY storefront — feasibility §4]; ATSC A/53 Part 4 (MPEG-2 user_data
  carriage) and H.264 SEI grammar are free. Clean-room legal either way.
- **Extraction surface:** (a) H.264 SEI type 4 — parser framework landed,
  T.35/ATSC GA94 decode is the add; (b) MPEG-2 picture `user_data` — needs
  a small parser in `mpeg12` (5.H1 landed the sequence layer only);
  (c) VBI — out of v1 scope. NEW (2026-10-24): `cc-data` 0.7.0 (MIT OR
  Apache-2.0, 2026-06-27, 1,235 dl, fishloa/rust-broadcast — "cc_data()
  carriage (CEA-608/708) per ETSI TS 101 154 Table B.9"; deps
  `broadcast-common` + `thiserror`) covers the carriage half only — no
  608/708 decode exists there or anywhere else found.
- **crates.io (2026-10-24):** `cc608`/`eia608`/`cc708`/`cea708`/
  `ccextractor` NOT_FOUND (`ccextractor` is a C project — bindings would be
  forbidden anyway); `cea608` 0.0.1 re-confirmed as a **builder** (53 dl);
  `filmcraft-captions` 0.9.0 (2026-10-23, 9 dl, SCC/MCC *import* — text-side,
  one day old) = recheck trigger only. ⇒ the decode self-build stands
  regardless of any admission.
- **Decode:** 608 = NTSC 15×32 caption grid, CC1–CC4/T1–T4 + XDS, pop-on
  buffer swap / paint-on / roll-up history, PAC/mid-row attributes →
  M, rendering as text-grid cues onto `CueBitmap` + timed events. 708 =
  window model (create/define/toggle/setattributes/setpen, per-window
  addressing, borders/word-wrap, G0–G3 multi-plane glyph encoding) —
  terminal-emulator-class M–L, staged partial behind 608 per matrix note
  (row `subtitle-cc-608-708`, P1, phase 6).

## Options

1. **Self-build all three (RECOMMENDED).** One shared paletted-RLE→RGBA
   core in `crates/subtitles` feeding the landed 5.L1 `CueBitmap`/
   `CompositedCue` pipeline (no renderer changes — feasibility §3.1's "one
   compositing code path"); three thin segment grammars (PGS epochs, DVB
   page/region, VobSub SPU) + the CC extraction/decode pair. Zero new
   dependencies (RLE is bit-twiddling, no unsafe). Effort: N1 M, N2 M
   (reuses N1's core; carriage landed), N3 M with 708 staged M–L partial.
   Optional cheap accelerators recorded as **validate-then-admit S spikes,
   not dependencies**: `libpgs` (PGS conformance vectors + flate2 backend
   purity check), `dvb-subtitle` + `cc-data` (fishloa family, same author,
   active — differential oracle candidates), mirroring the wave-7
   ruopus/rusty_vp9 pattern. `vobsub` 0.2.3 is reference/oracle material
   only (its 2017 dep stack stays out of our tree).
2. **Blocked-mixed.** Keep all four matrix rows blocked pending crate
   maturation. Honest but strands the P1 CC-608/708 row (no decoder crate
   exists to wait for — the wait has no exit), wastes the landed carriage
   (PS SPU, TS 0x90, SEI framework), and the three bitmap formats share a
   core that no single crate covers across all three.

## Decision (draft — for Lead signature)

**Option 1: self-build all three, zero new dependencies, CC staged 608 →
708.** Upon signature, wave 5.N launches: N1 PGS, N2 DVB+VobSub, N3 CC
(608 + extraction; 708 partial-honest). The Lead may attach the
validate-then-admit S spikes to N1/N2 kickoff as conformance oracles.

## Risks & obligations this decision carries

- **Timing correctness is the actual risk** (feasibility §3.3): PGS epoch
  reconstruction across seeks, DVB page-acquisition vs PTS, VobSub idx
  sync. Mitigation (already planned): CPU reference compositor
  differential before any bitmap sub lands in Phase 6.
- **PGS YCbCr palette conversion** [VERIFY] (matrix note stands).
- **CTA-608-E/708-D purchase friction** — legal/specs channel, not code.
- **Small in-tree gaps to fold into N-cards:** MKV `S_HDMV/PGS`/
  `S_VOBSUB`/`S_DVBSUB` codec-map rows (S); H.264 SEI type-4 T.35/GA94
  decode (N3); MPEG-2 `user_data` parser (N3).
- **Doc drift for the Lead (flagged, not edited here):** doc 04 §5 rows and
  feasibility §1/§4 ecosystem checks predate 2026-10-24 and miss
  `dvb-subtitle`, `cc-data`, `libpgs`, `pgs-rs`, `oxideav-sub-image`,
  `filmcraft-captions`; matrix evidence backfill happens at Lead merge.

## Consequences

- No new Cargo dependencies; `crates/subtitles` grows `pgs/`, `dvb/`,
  `vobsub/`, `cc/` modules per doc 13's target paths; fuzz targets
  `fuzz_sub_pgs` / `fuzz_sub_dvb` / `fuzz_sub_vobsub` / `fuzz_sub_cc`
  per doc 04 §5.
- The `[CONDITIONAL]` marker on 5.N1–5.N3 is discharged by signature;
  doc 13 needs no edit (gate language already points here).
