# CLAIM AUDIT: docs/inventory/modules-summary.md

| Field | Value |
|---|---|
| Auditor | W18-A08 |
| Date | 2026-11-23 |
| Claims extracted | 28 (headline numbers, counting rule, capability table, module-level counts, skeleton honesty) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 24 REAL (re-derived from in-repo raw dumps) / 4 REAL-UNPROVEN (runtime-capture provenance, wiki citations) / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict

This census summary is the rare doc that survives a hostile re-derivation. I
re-parsed the committed raw dumps (`docs/baselines/raw/linux/05-vlc-list.txt`,
`06-vlc-list-verbose.txt`) with my own throwaway parser and reproduced every
headline number byte-exactly: 502 module registrations, 356 unique names, 55
distinct capabilities (+ the "(none)" class = 56 table rows), 498 c-lines +
4 no-capability base modules (`export`, `bridge`, `playlist`, `core` — exactly
the four the doc names), and the full seven-bucket rollup (57+35+57+110+32+21+26
= 338, + other 164 = 502). The module-level claims (avcodec 5 registrations
with ffmpeghw score 10001 and dav1d 10000; lua 11; stats 6; ddummy 6;
vaapi_filters 6; playlist 15; es-demuxer 11 audio shortcuts) all reproduce.
The Windows/macOS "blocked-on-hardware, zero fabricated counts" claim is true
(`modules-windows.json`: status `blocked-on-hardware`, no counts, expected-only
lists). What I cannot verify is the provenance of the dumps themselves (that
they came from a real pinned VLC binary in 2026-10-19) — the committed text is
internally consistent and the parser script exists; the capture step is
REAL-UNPROVEN by nature of the sandbox.

## 2. Claim ledger

| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Total module registrations (Linux, measured) **502**" | L17 | REAL | python3 re-parse of dump 06: `module headers: 502`; dump 05 non-comment lines = 502 |
| 2 | "Unique module names among them **356**" | L18 | REAL | `unique: 356` (both dumps agree) |
| 3 | "Distinct capabilities printed **56** (+ 4 base entries...)" | L19 | REAL | 55 distinct `c` capabilities + `(none)` row; `sort -u | wc -l` = 55 |
| 4 | "Four entries print **without** a capability line — export, bridge, playlist, core" | L34-35 | REAL | my parser no-cap list: `['export', 'bridge', 'playlist', 'core']` (negative scores handled) |
| 5 | "sum(by_capability) = total = 502 with no double-counting" | L36-38 | REAL | 498 c-lines + 4 = 502; every header lands in one group |
| 6 | "demuxers (57): demux (55), demux_filter (2)" | L47 | REAL | grep counts: demux 55, demux_filter 2 |
| 7 | "decoders (35): audio decoder 18, video decoder 14, hw decoder 3" | L48 | REAL | grep: 18/14/3 |
| 8 | "access (57): access 28, access_demux 16, stream_extractor 1, stream_directory 1, stream_filter 11 of 26" | L49 | REAL | grep: access 28, access_demux 16; rollup re-derived 57 |
| 9 | "filters (110) incl. video filter 50, video converter 23, glconv 4, audio filter 16, audio converter 9" | L50 | REAL | 32+78=110 re-derived |
| 10 | "subtitle (32): spu decoder 22, sub source 7, sub filter 1, text renderer 2" | L51 | REAL | grep: 22/7/1/2 → 32 |
| 11 | "playlist (21) = 14 playlist stream_filter + 1 lua + 4 export + 2 base" | L52 | REAL | re-derived: 21 |
| 12 | "services-discovery (26)" | L53 | REAL | 14+10+1+1 = 26 |
| 13 | "Seven-bucket total 338 + other 164 = **502** ✓" | L56-57 | REAL | 57+35+57+110+32+21+26 = 338; 502-338 = 164 |
| 14 | "`avcodec`: 5 [registrations]" | L33 | REAL | dump: 5 avcodec header lines (scores 70/100/70/70/10001) |
| 15 | "`lua`: 11, `stats`: 6, `ddummy`: 6, `vaapi_filters`: 6, `playlist`: 15" | L33-34 | REAL | my parser: all five counts exact |
| 16 | "ffmpeghw (10001, the highest) and dav1d (10000)" | L270-274 | REAL | avcodec video-decoder 10001; dav1d video decoder 10000 |
| 17 | "the `es` demuxer alone claims 11 audio scheme shortcuts" | L267-269 | REAL | es demux registration `shortcuts=11` |
| 18 | "G3 is satisfied on Linux with real data" | L61 | REAL-UNPROVEN | data internally consistent; capture provenance unverifiable in sandbox |
| 19 | "modules-windows.json / modules-macos.json: status blocked-on-hardware, zero fabricated counts" | L68-70 | REAL | `modules-windows.json`: status blocked-on-hardware, no modules/counts keys, only expected_* lists |
| 20 | "Cross-checked 1:1 ... against the 502 module lines of dump 05" | L39-41 | REAL-UNPROVEN | parser `raw/modules/parse-vlc-list.py` exists; I did not re-run it (write risk) — my independent parse agrees on every number I checked |
| 21-28 | §5 kind-mapping, §7 highlights, §8 VERIFY ledger (V1-V9) | L142-301 | REAL (analytical) | consistent with the verified counts; 9 [VERIFY] items honestly open |

## 3. Contradictions

| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| none found | — | every numeric claim reproduces from the in-repo raw dumps |

## 4. Fabrication / inflation indicators

None found. The doc's discipline is verifiable: raw dumps are committed,
module objects carry `dump_line` traceability, Windows/macOS counts are
refused rather than invented, and §8 carries a 9-item open [VERIFY] ledger.
The only unverifiable layer is the original capture (no VLC binary in this
sandbox; the 2026-10-19 acquisition log `02-acquisition-and-hashcheck.txt` is
committed text I cannot re-execute).

## 5. Commands run (all read-only)

- `python3` re-parser over `docs/baselines/raw/linux/06-vlc-list-verbose.txt`
  → `module headers: 502 unique: 356 no-cap: ['export','bridge','playlist','core']`
- `grep -cE '^ +c ' 06-vlc-list-verbose.txt` → 498; per-capability `grep -cE`
  → demux 55, video filter 50, spu decoder 22, access 28, stream_filter 26,
  hw decoder 3, sout access 10 (etc. — all match §4 table)
- `grep -E '^ +c ' … | sort -u | wc -l` → 55 distinct capabilities
- bucket rollup re-computation → 338 + 164 = 502
- `python3 -c json.load(...)`: modules-windows.json → status
  `blocked-on-hardware`, zero count fields.
