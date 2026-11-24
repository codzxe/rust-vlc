# CLAIM AUDIT: docs/feature-matrix.toml — rows 066–078 (chunk 2/2)

| Field | Value |
|---|---|
| Auditor | W18-A08 |
| Date | 2026-11-23 |
| Rows in chunk | 13 (matrix rows 066–078 of 252; ids subtitle-teletext … plfmt-pls) |
| Claim verdicts | REAL: 0 · honest-absence TRUE (not_started/blocked honest): 7 · UNREACHABLE (real code, no shipped entry point): 6 |
| Internal contradictions found | 2 (row 067/068 "implemented" vs product reachability; row 077 evidence digit) |
| Reachability method | dependency graph + call-chain + rg call-site search; orchestrator baseline test log |

## 1. Blunt verdict

Every "implemented" cell in this chunk is true of the LIBRARY and false of the
PRODUCT. `crates/subtitles` (cc, vobsub, srt, compositing: 267 unit tests +
11 cc acceptance + fuzz suites, all green in the baseline) and `crates/playlists`
(m3u/hls/pls/xspf: 46 unit + 18 + 14 + 14 suite tests, green) are real, tested
code — and **not one shipped binary links either crate**: `apps/cli/Cargo.toml`
(lines 48–63) has no `subtitles`, no `playlists`; `apps/desktop`,
`apps/desktop-tauri`, `apps/control-server` likewise (rg: 0 hits). The playback
engine's subtitle surface is a documented honest stub
(`playback-engine/src/tracks.rs:103-108` — "set-subtitle-track mints nothing").
So rows 067/068/072/074/077/078 describe capability that NO shipped entry point
can reach: a user of `lumen` today gets zero captions, zero vobsub, zero
sidecar detection, zero playlist import. The matrix's own status vocabulary
("implemented = code complete … evidence not yet complete") makes the cells
technically defensible — but nothing in these rows records the product-level
unreachability, and `parity-report` (0/252 verified on every platform) is the
corroborating truth. The 7 not_started/blocked rows are verified honest
absences.

## 2. Claim ledger (matrix rows)

| Row id | Status claimed (line) | Evidence claimed | Code found | Executed? | Verdict |
|---|---|---|---|---|---|
| 066 subtitle-teletext | blocked x4 (L1017) | "" + blocked_reason zvbi C-forbidden (L1019) | NONE — rg `zvbi\|teletext` over crates+apps: 0 hits | n/a | HONEST |
| 067 subtitle-cc-608-708 | implemented w/l/m (L1030) | cc/** FULL EIA-608 + staged 708, GA94 SEI + SCTE-20, "26 unit + 11 cc_acceptance tests, all green" (L1031) | `crates/subtitles/src/cc/` (cc_data.rs, eia608.rs, cea708.rs, render.rs) — real; numbers verified: baseline has exactly 26 `test cc::` unit tests + `cc_acceptance.rs` 11 tests, all ok. NOT linked by any shipped app | baseline L6022-6038: 11/11 ok; 26 unit ok | UNREACHABLE (library REAL; no shipped entry point links `subtitles`) |
| 068 subtitle-vobsub | implemented w/l/m (L1043) | vobsub/** idx/spu parse, 128 KiB cap, 230/0 gates (L1044) | `crates/subtitles/src/vobsub/` (idx.rs, spu.rs) — real; baseline: 15 `test vobsub::` unit tests + dvb_vobsub_fuzz_survival ok | baseline L6060ff: 2/2 fuzz-survival ok | UNREACHABLE (same as 067) |
| 069 subtitle-kate | blocked x4 (L1056) | "" + blocked_reason (L1058) | NONE — no kate code | n/a | HONEST |
| 070 subtitle-forced | not_started x4 (L1068) | "" | NONE — "forced" hits are prose/PGS fields, no forced-track policy code | n/a | HONEST |
| 071 subtitle-autoload | not_started x4 (L1080) | "" | NONE — no sidecar/autodetect logic in apps/crates | n/a | HONEST |
| 072 subtitle-encoding-detection | partial w/l/m (L1092) | srt/encoding.rs BOM matrix (L1093) | `crates/subtitles/src/srt/encoding.rs` — real; does MORE than the row cites (Windows-1252 8-bit fallback with typed damage, documented `[VERIFY]` heuristic); note honestly limits scope ("BOM-less UTF-16 deliberately NOT sniffed") | baseline: 9 `test srt::encoding::` tests ok (incl. `bom_detection_matrix`) | UNREACHABLE (honest partial; library only) |
| 073 subtitle-census-tail | not_started x4 (L1105) | "" | NONE — only a descriptor string `b"tx3g" | b"text" => Some("mov_text")` in `demux/src/mp4/mod.rs:467`, no parser | n/a | HONEST |
| 074 textrender-subtitle-rasterizer | partial w/l/m (L1123) | compositing/** + model + 5.M2b placeholder glyph metrics (L1124) | `compositing/**` (bitmap/layout/compose/timing) + `model/**` + `font/` seam — real; `font/mod.rs:8-9` concedes placeholder block glyphs | baseline: 34 compositing/font tests + compositing_ac/alloc suites ok | UNREACHABLE (honest partial; product renders placeholder blocks only, and only if wired — it is not) |
| 075 textrender-osd | not_started x4 (L1135) | "" | NONE — rg `\bosd\b` in apps/cli, playback-engine, video-engine: 0 code hits | n/a | HONEST |
| 076 textrender-subsource-overlays | not_started x4 (L1148) | "" | NONE — no marq/rss/logo/mosaic producers ("logo" hits = brand logo path only) | n/a | HONEST |
| 077 plfmt-m3u-m3u8 | partial w/l/m (L1166) | m3u.rs; "EXTINF sums vs ffprobe 9.046543/9.046539, RFC 3986 join 23+19 pinned, 10k fuzz runs; 38/0" (L1167) | `crates/playlists/src/m3u.rs` + `hls.rs` — real. Numbers: 9.046543 verified (`hls_attrs.rs:15,93-97`); RFC 3986 23+19 case lists verified (hls_attrs.rs:616-660); 10k fuzz verified (`fuzz_lite_random_bytes_10k_no_panic ... ok`); "9.046539" NOT in tree (fMP4 fixture pins 9.046439, hls_attrs.rs:17) — one-digit wrong; "38/0" stale vs today's 46+18+20 gates | baseline: m3u_suite 18 ok, hls_attrs 20 ok | UNREACHABLE (honest partial; `playlists` not linked by any shipped app; `streaming` uses it but no shipped app reaches `remote_playlist`/`hls`) |
| 078 plfmt-pls | implemented w/l/m (L1179) | pls.rs golden truth-sets, 46/0 (L1180) | `crates/playlists/src/pls.rs` — real; baseline pls_suite 14 ok incl. `pls_bridges_losslessly_through_m3u`; "46/0" = playlists crate's 46 unit tests (baseline L4675: 46 passed) | baseline L4753-4770: 14/14 ok | UNREACHABLE (library REAL; no shipped consumer of .pls) |

## 3. Contradictions

| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Rows 067/068 "implemented" on win/linux/macos | `apps/cli/Cargo.toml:48-63` has no `subtitles` dep; `playback-engine/src/tracks.rs:103-108`: "set-subtitle-track mints nothing … What is stub today" | The library code exists and is tested; the shipped product displays no captions of any kind. Both true; the matrix never records the product gap |
| Row 077 evidence: "ffprobe 9.046543/9.046539" | Tree pins 9.046543 and 9.046439 (`hls_attrs.rs:15,17`) | The second number in the evidence string is a transcription error (539 vs 439). Trivial but it is a wrong digit in a "verified evidence" string |
| Row 067 notes: "docs/formats record owed (G-1 class gap)" | `docs/formats/sub-cc.md` exists (85 lines, card 8.G1 consolidation) | The record has since landed; the note is STALE, not false |

## 4. Fabrication / inflation indicators

1. No fabricated code found in this chunk: every evidence path that names a
   module (`cc/**`, `vobsub/**`, `srt/encoding.rs`, `compositing/**`, `m3u.rs`,
   `pls.rs`) exists and its claimed tests run green in the orchestrator baseline.
2. The inflation is structural, not textual: "implemented" cells silently mean
   "library implemented", never saying that no shipped binary links the crate.
   A reader of the matrix alone would believe lumen renders CEA-608 captions
   and opens PLS playlists. It does neither.
3. Count drift: "38/0" (row 077) and "230/0 gates" (row 068) are card-time
   gate tallies; current suites are 46/18/20 and 15+2 respectively. Stale
   counts, direction inconsistent (some lower, some higher than today).

## 5. Key verbatim evidence (sed-verified)

Matrix (docs/feature-matrix.toml):
- L1030-1031 (row 067): `status = { windows = "implemented", linux = "implemented", macos = "implemented", web = "not_started" }` / `evidence = "crates/subtitles/src/cc/** ... 26 unit + 11 cc_acceptance tests, all green in this tree 2026-10-26)"`
- L1179-1180 (row 078): `status = { windows = "implemented", ... }` / `evidence = "crates/playlists/src/pls.rs (card 5.O1b: ... 46/0 playlists gates)"`
Code:
- apps/cli/Cargo.toml:48-63 — dependency list contains `codecs`, `demux`, `playback-engine`, … and NO `subtitles`, NO `playlists` (grep count 0, verified).
- crates/playback-engine/src/tracks.rs:103-108: "The class-S subtitle seam — and its honest stubs … `tracks.set-subtitle-track` mints **nothing**"
- crates/subtitles/src/font/mod.rs:8-10: "Today the ASS render path rasterizes placeholder block glyphs (`ass::render`'s monospace-widest-char metrics — the G-5 gap ...)"
- crates/demux/src/mp4/mod.rs:467: `b"tx3g" | b"text" => Some("mov_text"),`
- crates/streaming/src/remote_playlist.rs:178: `let parsed = playlists::parse_bytes(&response.body)...` — the only production consumer of `playlists`, itself unreached by shipped apps.
Baseline (audit/runs/baseline/cargo-test-workspace.txt):
- L6022-6038: cc_acceptance 11 tests `test result: ok. 11 passed; 0 failed`
- L6010: subtitles 267 unit `test result: ok. 267 passed; 0 failed`
- L4675/4753/4771: playlists 46 passed; pls_suite 14 passed; xspf 14 passed.
- L3158: `test ivf_av1_family_lane ... ok` (row 026 corroboration lives in chunk-1 ledger).

## 6. Row-level findings

- F-005 [S2] Rows 067/068 (and 072/074): "implemented" cells record library
  capability that no shipped entry point reaches (subtitles crate unlinked from
  every shipped binary; engine subtitle seam is a documented stub). The matrix
  has no field carrying "product-reachable: NO" — the single most misleading
  structural gap in this chunk.
- F-006 [S4] Row 077 evidence digit: "9.046539" should be 9.046439
  (hls_attrs.rs:17). One wrong digit in a pinned-truth evidence string.
- F-007 [S4] Rows 067 notes ("record owed") and 077 ("38/0") are stale vs the
  tree (sub-cc.md exists; suites renumbered). No status cell affected.
