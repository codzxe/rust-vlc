# AUDIT: docs/inventory/modules-linux.json
| Field | Value |
|---|---|
| Auditor | W18-A02 |
| Date | 2026-11-12 |
| Lines / data entities | 4356 lines / 502 module registrations in 56 capability groups |
| File verdict | REAL (DATA) — regenerable from raw dumps, byte-identical, all aggregates re-verified |
| Reachable from shipped entry point? | N/A (data file). Consumed by docs/feature-matrix.toml:27-28 (population source), docs/feasibility-report.md:27-29, docs/feasibility/sections/*.md denominators |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 3 |

## 1. One-paragraph verdict
This is the rare docs artifact that is actually what it claims to be: a machine-generated,
byte-reproducible census. I re-ran the in-repo parser
(`docs/inventory/raw/modules/parse-vlc-list.py`) against the committed raw dumps
(05/06) and the output is byte-identical to the committed JSON (`cmp` clean, exit 0).
All 502 registrations trace 1:1, in order, to the 502 module lines of dump 05; every
`dump_line` I checked (15/15) matches dump 06; sum(by_capability)=total=502; the
seven-category rollup reconciles exactly once the split rule (documented in the parser
and modules-summary.md §2, NOT in this JSON) is applied. The file is an honest, auditable
denominator for demux/codec coverage claims — its defects are a wrong example number in
its own counting_rule prose ("lua: 8" vs actual 11), a rollup whose bucket boundaries are
not derivable from this file alone, and downstream matrix join-keys that under-cover the
census long tail.

## 2. What actually works (verified)
- `python3 parse-vlc-list.py <dump06> <dump05> <out>` -> exit 0, prints
  `total 502 | rollup: {'demuxers': 57, 'decoders': 35, 'access': 57, 'filters': 110, 'subtitle': 32, 'playlist': 21, 'services-discovery': 26, 'other': 164} | sum 502`; `cmp` with committed JSON -> identical.
- dump 05 contains exactly 502 module-styled lines (511 total incl. 9 header/footer comment lines) — verified with a python3 line-class count.
- 1:1 name-order cross-check of all 502 by_capability entries (sorted by dump_line) vs dump 05 module lines -> PASS.
- dump_line spot-checks (15): ttml:207, webvtt:265, avi:1129, mp4:1156, flacsys:1124, wav:1112, ts:1114, mkv:1131, dtv:948, dummy:7, core:1287, playlist:1194, vaapi_filters:324, libbluray:875, lua:462 — all match dump 06 header lines.
- Internal arithmetic: per-group declared `count` == actual module-list length for all 56 groups (no mismatch); unique names 356; all 502 dump_lines unique, range 7..1287; `names_with_most_registrations` (playlist 15, lua 11, ddummy 6, stats 6, vaapi_filters 6, avcodec 5, export 5, avahi 4, dummy 4, mediadirs 4, udev 4, bridge 3) — every one re-computed and matched.
- `(none)` group: dump 06 line 550 `  export                 export` has no `c` capability line (child `c playlist export (0)` follows), same for bridge:755, playlist:1194, core:1287 — the "4 base/parent entries" claim is real.

## 3. Findings

F-001 [S3]
Where: docs/inventory/modules-linux.json:28-29
Code: (verbatim)
 "total": 502,
 "counting_rule": "One entry per module-capability registration printed by --list-verbose (the 502 header lines; every header carries at most one capability line). A single plugin binary can register several entries in different capabilities (avcodec: 5, lua: 8, stats: 6). ...
What is wrong: "lua: 8" is false. lua has 11 registrations (8 distinct capabilities). The same file's stats block says 11 (lines 141-143: `{"name": "lua", "registrations": 11}`), and the regenerating parser prints `top registrations [('playlist', 15), ('lua', 11), ...]` while embedding the wrong "lua: 8" prose (parse-vlc-list.py:143-145). Why it matters: the file contradicts itself in the very rule that defines how to count; a reader trusting counting_rule alone computes lua's multiplicity wrong. What correct looks like: "avcodec: 5, lua: 11, stats: 6" (units consistent) or "lua: 8 capabilities". Confidence: HIGH — Verification: ran (regeneration + re-count).

F-002 [S3]
Where: docs/inventory/modules-linux.json:30-133 (seven_category_rollup)
Code: (verbatim excerpt)
   "demuxers": {
    "count": 57,
    "capabilities": [
     "demux",
     "demux_filter"
    ]
   },
   ...
   "playlist": {
    "count": 21,
    "capabilities": [
     "(none)",
     "playlist export",
     "stream_filter"
    ]
   },
What is wrong: `stream_filter` is listed under BOTH "access" (lines 48-56) and "playlist" (81-87) bucket capabilities, and `(none)` under BOTH "playlist" and "other" (98-131) — the JSON carries no split rule, so the declared bucket counts (access 57, playlist 21, other 164) are NOT derivable from this file alone: naive capability-summing gives access 46, playlist 5, other 166 pools and no unique assignment reproduces the declared numbers without the external rule (11 of 26 stream_filter → access; 15 → playlist; (none) → playlist{playlist,export} / other{bridge,core}). The rule exists only in parse-vlc-list.py:66-84 and modules-summary.md:45-54. Why it matters: this file is the "single source" denominator for coverage claims; anyone recomputing coverage from it alone gets access/playlist/other wrong. What correct looks like: embed the split rule in the rollup note (e.g. "stream_filter: playlist-named + lua-playlist → playlist, rest → access; (none): playlist+export → playlist, bridge+core → other"). Confidence: HIGH — Verification: ran (numeric reconciliation + parser read + byte-identical regeneration).

F-003 [S3] — matrix join-key coverage gaps surfaced by this census (affects "X of VLC's modules" measurability)
Where: docs/feature-matrix.toml:541-548 (and 388-398), vs census
Code: (verbatim)
id = "codec-video-mjpeg"
category = "video-codec"
vlc_name = "Motion-JPEG decode"
...
vlc_modules = ["avcodec"]
What is wrong: three defects found by joining all 252 rows' `vlc_modules` (177 distinct tokens) against the census:
(a) codec-video-mjpeg maps to ["avcodec"] although VLC's M-JPEG decode is the `jpeg` module (census: video decoder "JPEG image decoder", score 1000, dump_line 253 — quoted §2-verified via dump 06 line 253); `jpeg` is cited by NO row;
(b) container-census-long-tail (feature-matrix.toml:391,394) says `description = "caf, voc, xa, tta, mpc, mod, sid, nsv, pva, ty, nuv, rawdv, rawvid, rawau and remaining census demuxers"` — "rawau" does not exist in the census (real name: "rawaud", demux, dump_line 1071); and its vlc_modules lists only 8 of the 14 names in its own description;
(c) 21 of 53 demuxer+demux_filter names and 9 of 29 decoder names are cited by no row's vlc_modules (avformat, demux_cdg, demux_chromecast, demux_stl, demuxdump, diracsys, dmxmus, image, linsys_sdi, mjpeg, noseek, nsc, nuv, pva, rawaud, rawdv, rawvid, stats, ty, vc1, vobsub; cdg, ddummy, oggspots, png, svgdec, vaapi_drm, xwd) — machine-computable coverage from vlc_modules alone undercounts; the tail lives in prose.
Also 26 cited tokens are not census names (dxva2, mmdevice, dshow, skins2, goom, …) — these are Windows/macOS skeleton or capability names (e.g. "inhibit", "addons finder" are capabilities, not modules), so a strict census-name join silently drops them.
Why it matters: the repo's coverage/parity claims are computed by joining the matrix to this census; the join key is incomplete and mixes name spaces. What correct looks like: full vlc_modules lists on the tail rows; `jpeg` on codec-video-mjpeg; "rawaud" not "rawau"; capability-name tokens moved to a separate field. Confidence: HIGH — Verification: ran (tomllib join over 252 rows).

F-004 [S4]
Where: docs/inventory/modules-linux.json:356-364
Code: (verbatim)
    "name": "vdpau_chroma",
    "capability": "video converter",
    "description": "VDPAU surface conversions"
   },
   {
    "name": "vdpau_chroma",
    "capability": "video converter",
    "description": "VDPAU surface conversions"
   },
What is wrong: hardware_related_modules duplicates (vdpau_chroma ×2, vaapi_filters video-filter ×5) with no distinguishing field — the stats entries carry no dump_line/shortcuts, so they are indistinguishable copy-paste-looking rows. They are NOT fabrications: both vdpau_chroma registrations are real (dump 06 lines 314 and 316; by_capability "video converter" lists both) and all 19 hw entries match the actual vaapi*/vdpau*/glconv* registrations 19:19. What correct looks like: carry dump_line in the stats block so duplicates are self-evidently distinct. Confidence: HIGH — Verification: ran (19:19 match + dump 06 lines 314/316).

F-005 [S4]
Where: docs/inventory/modules-linux.json:427-435 vs modules-summary.md:19
Code: (verbatim, summary)
| Distinct capabilities printed | **56** (+ 4 base entries with no capability line) | computed |
What is wrong: by_capability has exactly 56 keys INCLUDING "(none)" — the pseudo-group holding the 4 base entries. So "56 capabilities (+4 base entries)" counts the "(none)" bucket as a capability while simultaneously calling its members "no capability". Real capability strings printed = 55. Number-reconcilable but convention-confusing. Confidence: HIGH — Verification: ran (len(by_capability)=56).

F-006 [S4]
Where: docs/feature-matrix.toml:398 and modules-summary.md:155,264
Code: (verbatim, matrix)
notes = "51 unique demuxers in the census (57 registrations); wont-do-v1 list with reasons in matrix-seed-notes §5; re-opened per-format on demand with a record."
What is wrong: mixed scopes — 51 is the unique-name count of the `demux` capability only; the 57 registrations include `demux_filter`, whose 2 names (demux_chromecast, noseek) are not among the 51, union = 53. Same phrasing in summary "demuxer ← demux + demux_filter (57). 51 unique demuxer names." Related: audio-codecs.md:9-10 "18 registrations across 16 real decoders" — 18 registrations ✓, 17 unique names, "16" only if you silently drop ddummy (stats kept, criterion unstated). Confidence: HIGH — Verification: ran (unique-name counts 51/53/17).

## 4. Data-structure inventory (per-group table; "per-function" equivalent for a DATA file)
| Capability group | Declared count | Re-counted | Notes |
|---|---|---|---|
| demux | 55 | 55 ✓ | 51 unique names; scores range 0..240 (mp4 240, avi 212, asf 200) |
| video filter | 50 | 50 ✓ | vaapi_filters ×5, all score 0 |
| access | 28 | 28 ✓ | incl. avio score -1 |
| stream_filter | 26 | 26 ✓ | 14 playlist-named + lua + 11 generic |
| packetizer | 25 | 25 ✓ | |
| sout stream | 23 | 23 ✓ | |
| video converter | 23 | 23 ✓ | vdpau_chroma ×2, i420_yuy2_sse2 250 |
| spu decoder | 22 | 22 ✓ | |
| encoder | 21 | 21 ✓ | png/jpeg score 1000 |
| audio decoder | 18 | 18 ✓ | 17 unique (ddummy ×2) |
| access_demux / audio filter / interface | 16/16/16 | ✓ | |
| services_discovery / video decoder / vout display | 14/14/14 | ✓ | dav1d 10000, avcodec ffmpeghw 10001 |
| sout mux / services probe / sout access | 11/10/10 | ✓ | |
| audio converter / sub source / audio output / (none) / audio resampler / glconv / keystore / logger / playlist export / vout window | 9/7/5/4/4/4/4/4/4/4 | ✓ | (none) = export, bridge, playlist, core |
| addons finder / hw decoder / opengl | 3/3/3 | ✓ | |
| art finder / audio volume / demux_filter / inhibit / meta reader / opengl es2 / text renderer / vod server | 2 each | ✓ | |
| 17 remaining singleton groups | 1 each | ✓ | addons storage … xml reader |
Declared `total`=502 = sum ✓; unique_module_names=356 ✓; rollup sum_check=502 ✓ (57+35+57+110+32+21+26+164).

## 5. Ten-module spot-check vs feature-matrix rows (census ↔ matrix)
| # | Census module (cap, score, dump_line) | Matrix row | vlc_modules cited | Status (linux) | Evidence dir exists? | Verdict |
|---|---|---|---|---|---|---|
| 1 | avi (demux, 212, 1129) | container-avi | ["avi"] | implemented | crates/demux/src/avi/ EXISTS | consistent |
| 2 | mp4 (demux, 240, 1156) | container-mp4-mov-3gp | ["mp4"] | implemented | crates/demux/src/mp4/ EXISTS | consistent |
| 3 | mkv (demux, 50, 1131) | container-matroska-webm | ["mkv"] | implemented | crates/demux/src/mkv/ EXISTS | consistent |
| 4 | ogg (demux, 50, 1154) | container-ogg | ["ogg"] | implemented | crates/demux/src/ogg/ EXISTS | consistent |
| 5 | wav (demux, 142, 1112) | container-wav | ["wav"] | implemented | crates/demux/src/wav/ EXISTS | consistent |
| 6 | flacsys (demux, 155, 1124) | container-flac-native | ["flacsys"] | not_started | evidence EMPTY | honest absence |
| 7 | es (demux, 155, 1177, 11 shortcuts) | container-mp3-raw / container-adts-aac | ["es","mpegaudio"] / ["es","mpeg4audio"] | implemented | crates/demux/src/es/, adts/ EXIST | consistent |
| 8 | mad (audio decoder, 99, 47) | codec-audio-mp3 / codec-audio-mp2 | ["mpg123","mad"] | implemented | crates/codecs/src/audio/mp3/ EXISTS | consistent |
| 9 | araw (audio decoder, 100, 221) | codec-audio-pcm | ["araw","lpcm"] | partial (linux) | crates/codecs/src/audio/pcm/ EXISTS | consistent |
| 10 | faad (audio decoder, 100, 120) | codec-audio-aac-lc | ["faad"] | partial (linux) | crates/codecs/src/audio/aac/ EXISTS | consistent |
| +1 | jpeg (video decoder, 1000, 253) | codec-video-mjpeg | ["avcodec"] — NOT "jpeg" | implemented | — | MAPPING DEFECT (F-003a) |
| +2 | mjpeg (demux, 5, 1116) | none | — | — | — | uncovered by any row |
All 12 census-side facts verified against dump 06 (§2) and the regenerated file. Matrix statuses are `implemented`/`partial`, never `verified` — matches the orchestrator baseline (parity-report 0/252 verified on every platform).

## 6. Claim ledger (docs claims measured against THIS file)
| # | Claim (verbatim, <=25 words) | Source | Verdict | Proof |
|---|---|---|---|---|
| 1 | "Total module registrations (Linux, measured) 502" | modules-summary.md:17 | REAL | dump 05 = 502 module lines; regeneration byte-identical |
| 2 | "Unique module names among them 356" | modules-summary.md:18 | REAL | re-count = 356 |
| 3 | "Sum of per-capability counts 502 = total" | modules-summary.md:20 | REAL | python3 sum check |
| 4 | "502 registrations / 356 unique / 56 capabilities (Linux census…)" | feasibility-report.md:27 | REAL (with F-005 counting caveat) | all three re-verified |
| 5 | "Seven-category split: demux 57, decode 35, access 57, filters 110, subtitle 32, playlist 21, services-discovery 26, other 164" | feasibility-report.md:28-29 | REAL | regeneration prints identical rollup |
| 6 | "avcodec: 5, lua: 8, stats: 6" (registration example) | modules-linux.json:29 counting_rule | FALSE (lua component) | actual lua = 11 (file's own stats block; parser output) |
| 7 | "VLC Linux census = 57 demuxer registrations (55 demux + 2 demux_filter…)" | feasibility/sections/subsystems.md:21 | REAL | 55+2 verified |
| 8 | "audio decoder capability = 18 registrations across 16 real decoders … 35 total (18+14+3)" | feasibility/sections/audio-codecs.md:8-9 | REAL with unstated exclusion (ddummy dropped, stats kept; 17 unique) | re-count |
| 9 | "this census is regenerable with it" (parser claim) | modules-linux.json:8 parser field | REAL | ran parser → cmp identical, exit 0 |
| 10 | "cross-checked 1:1, in order, against the 502 module lines of dump 05" | modules-linux.json:29 | REAL | I re-ran the equivalent 1:1 name-order check: PASS |
| 11 | "51 unique demuxers in the census (57 registrations)" | feature-matrix.toml:398, modules-summary.md:155 | SHALLOW (mixed scope; union = 53) | unique-name counts |
| 12 | "v1 = 3 capabilities out of VLC's 46-module family (~7% by count)" | feasibility/sections/subsystems.md:525-526 | REAL | sout stream 23 + sout mux 11 + sout access 10 + vod server 2 = 46 |
| 13 | "The two highest scores … are ffmpeghw (10001) and dav1d (10000)" | modules-summary.md:272-274 | REAL | census: avcodec/ffmpeghw 10001 (dump_line 190), dav1d 10000 (217) |

## 7. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| counting_rule: "lua: 8" (json:29) | same file stats: "lua", 11 registrations (json:141-143); parser output "lua: 11" | 11 is true (re-count + dump 06 has 11 lua headers with c-lines) |
| rollup note: "Every registration lands in exactly one bucket" (json:31) | bucket capabilities list stream_filter under access AND playlist; (none) under playlist AND other | both true only with the parser's hidden split rule (parse-vlc-list.py:66-84); not decidable from this file alone |

## 8. Fabrication / inflation indicators
NONE in this file. Every number is dump-derived and reproducible; duplicates that look like copy-paste (vdpau_chroma, vaapi_filters) are genuine multiple registrations (dump 06 lines 314/316, 324-337). The scope is honestly narrowed (plugin_packages + excluded_debs_headless fields, json:12-27); DR-11 (docs/baselines/vlc-3.0.23.md:153) correctly warns "must not treat 502 as the complete Debian build". Inflation risk sits downstream: any "of VLC's modules" claim using 502/57/35 as denominator silently means "Linux headless 2-package subset".

## 9. Measurability assessment (task question)
"X of VLC's modules"-style claims ARE measurable from this file for the Linux-headless scope: I computed coverage joins programmatically. Boundaries:
1. Denominator scope: 2 plugin packages, 10 excluded debs (json:12-27) — VLC full module set is larger; Windows/macOS have zero measured counts (blocked-on-hardware skeletons, modules-summary.md:21).
2. Join key: feature-matrix vlc_modules — works for name-level rows (my 10-module spot-check: 10/10 correct) but under-covers the tail (F-003c: 21/53 demuxer + 9/29 decoder names uncited) and mixes module/capability/shortcut token spaces (26 non-census tokens).
3. Units: claims must pick registrations (502/57/35) vs unique names (356/53/29) vs "real" modules (16 decoders after unstated exclusions) — the repo mostly does this correctly but not uniformly (F-006).

## 10. Dead / duplicated / unreachable code
N/A (data). The duplicated-looking stats rows are real registrations (F-004). No unreferenced fields: census/task/generated_from/parser/acquisition/vlc_version/plugin_packages/excluded_debs_headless/total/counting_rule/seven_category_rollup/stats/by_capability — all consumed by modules-summary.md and/or the matrix.

## 11. Dependencies & flags
None (pure JSON data). Inputs verified present: docs/baselines/raw/linux/05-vlc-list.txt (511 lines), 06-vlc-list-verbose.txt (1288 lines), 02-acquisition-and-hashcheck.txt, docs/inventory/raw/modules/parse-vlc-list.py (172 lines, read in full), docs/inventory/raw/modules/manifest.json. Pin cross-check: vlc_version "3.0.23-2-0-g79128878dd" == docs/baselines/raw/linux/04-vlc-version.txt:7 "VLC version 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)".

## 12. Open questions for the next agent
1. F-001/F-002 are fixable in the parser prose + rollup note — verify the fix lands byte-identical otherwise (regeneration is the gate).
2. F-003's matrix join gaps: whoever audits docs/feature-matrix.toml rows 388-398 / 541-548 should fix `jpeg` mapping and "rawau"→"rawaud" and complete long-tail vlc_modules; the R8 "inventory completeness" tool is admitted OWED debt in the matrix header (feature-matrix.toml:13).
3. modules-windows.json / modules-macos.json (blocked-on-hardware skeletons) not audited here — the 26 non-census vlc_modules tokens trace there.
4. Line count: task metadata says 4357 loc; actual file is 4356 lines.
