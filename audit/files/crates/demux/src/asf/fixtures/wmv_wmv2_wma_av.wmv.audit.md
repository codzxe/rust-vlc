# AUDIT: crates/demux/src/asf/fixtures/wmv_wmv2_wma_av.wmv
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-21 |
| Lines / functions | 10,499-byte binary / n/a (test asset) |
| File verdict | REAL (test asset: committed, sha-pinned, byte-reproducible from its in-repo recipe) |
| Reachable from shipped entry point? | N/A (test asset). Consumed by `fixture_family_walks_match_the_measured_truth` (truth entry asf/tests.rs:1003-1019) + ffprobe cross-check (tests.rs:1325) — executed ok in baseline (log 1567/1953). Consuming module UNREACHABLE from shipped entry points (guid.rs audit F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 (family findings filed in the asf_mjpeg.asf audit; cross-referenced in §3, not recounted) |
| Commands run | sha256sum (pin match) · ffmpeg 7.1.5 regeneration from tests.rs:1005 recipe (byte-identical) · python3 audit/analysis/asf_parse.py (walk == truth entry) · ffprobe codec_name (`wmv2`,`wmav2` ✓) |

## 1. One-paragraph verdict
Genuine, sha-pinned, byte-exact-reproducible ffmpeg 7.1.5 output of the family's A/V member: two Stream Properties objects (video stream 1: WMV2 64x48 with 4 bytes of extradata, TSD 55 B; audio stream 2: wmav2 WFX 0x0161/1 ch/8000 Hz/align 256/16/10, TSD 28 B), 6 header children, 3 data packets, and a Simple Index — every typed truth fact verified by an independent walk, both tracks' identities verified against live ffprobe, and the measured preroll-grid first-pts ([64, 0]) asserted against ffprobe's own arithmetic. No file-specific defects; family findings cross-referenced.

## 2. What actually works (verified)
- sha256 `5bd8c76fa14e31aa2f2ddcfaa7acfd84394419e1316c809d959335b8129d44be` == pin `5bd8c76fa14e31aa` (tests.rs:1492).
- Regeneration from tests.rs:1005 recipe (`testsrc 64x48@10 1s` + `sine 440 8 kHz 1s`, `-c:v wmv2 -b:v 100k -c:a wmav2 -b:a 32k`; plan §1.2:64-67 family) under ffmpeg 7.1.5 → identical sha256.
- Independent walk == truth (tests.rs:1003-1019): HEADER 757 / 6 children (FPO 104, Header Extension 156, ECD 98, SP video 133, SP audio 114, Codec List 122) · FPO file_size 10499 == real size, 3 packets, preroll 3100, min=max=3200, flags 0x02 · DATA at 757 size 9650 count 3, reserved `01 01` · SIMPLE_INDEX at 10407 size 92 (757+9650=10407; +92=10499 exact) · video SP stream_number 1: WMV2, 64x48, data_size/biSize 44, extradata 4, TSD 55 == the value cited in the tests.rs:1224 comment ("streams[0] tsd 55 = VIDEOINFOHEADER") · audio SP stream_number 2: WFX tag 0x0161, 1 ch, 8000 Hz, align 256, bits 16, cb_size 10, TSD 28 == 18+cb.
- ffprobe oracle: `wmv2`, `wmav2` == tests.rs:1018 in order; 2 streams == 2 tracks.
- Baseline: differential + ffprobe cross-check ok (log 1567, 1953).

## 3. Findings
None specific to this file. Cross-references (filed and counted in the asf_mjpeg.asf audit):
- F-001 [S3, cross-ref]: both SP EC fields carry the real GUIDs (video 20FB5700-…, audio BFC3CD50-…00AA00B4E220), refuting guid.rs's two EC constants.
- F-002 [S4, cross-ref]: outside the xtask fixtures drift gate; in-test sha pin compensates.
- F-003 [S4, cross-ref]: missing committed copy regenerates into src/ via tests.rs:1100-1118.

## 4. Per-function table (asset checks)
| Check | Result | Notes |
|---|---|---|
| Exists / size sane | YES / 10,499 B | == FPO file_size; plan §4.4:457 row ✓ |
| Structure matches the truth table | YES (all facts) | Independent walk == tests.rs:1003-1019 |
| sha256 pin | MATCH | tests.rs:1492 |
| Recipe reproducibility | BYTE-EXACT | ffmpeg 7.1.5 + tests.rs:1005 → identical sha |
| A/V property verified in bytes | YES | Two SPs (streams 1+2), two-track truth, first_pts [64, 0] — the only A/V member at default packet size (sibling asf_pkt8000 varies it) |
| Referenced by code/docs | YES | tests.rs:1004, :1224 (comment), :1492; plan §4.4:457 |
| Determinism / drift gate | TEST-TIME ONLY | sha pin in-test (cross-ref F-002) |
| Hostile value | N/A | Clean-file fixture; exercises multi-payload packet walk on real muxer output |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `fixture_family_walks_match_the_measured_truth` | YES | Two-track table with per-stream numbers (:1185-1194: video=1, audio=2 — my SP-flag walk confirms), WFX found on the AUDIO record not streams[0] (:1208-1230 — the streams[0]-is-video discipline), VIH on the video record (:1232-1244), first_pts [64, 0] (:1294-1298 — the preroll-offset video grid), monotonic per-track stamps (:1279-1284), seek+resume (index present, :1304-1320). Mutation: swap the SP stream-number read and :1190-1193 fails. |
| `fixture_family_cross_checks_against_live_ffprobe` | YES (ffprobe present) | Two-stream codec names + per-track packet counts vs `-count_packets`; hand re-run: `wmv2`,`wmav2` ✓. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "wmv_wmv2_wma_av.wmv 10,499 5bd8c76fa14e31aa" + lane "A/V multi-payload" | plan §4.4:457 | REAL | sha256sum + 2 SPs / 2 tracks verified in bytes |
| "wmv_wmv2_wma_av: streams[0] tsd 55 = VIDEOINFOHEADER" | tests.rs:1223-1224 (comment) | REAL | Video SP TSD length 55 in bytes (walk) |
| wmav2/wmv2 codecs not in the shipped decoder set → "demux + typed cli.codec_unavailable" | plan §1.3:98 | REAL-UNPROVEN (as to this crate) | Demux side proven; the typed codec-unavailable refusal on the ASF lane is unreachable today (module unwired; engine-info ships mjpeg+pcm only) |

## 7. Dead / duplicated / unreachable code
Alive test input; consumer module dead in the shipped product (guid.rs audit F-001).

## 8. Dependencies & flags
None (data file). cfg(test) consumption only.

## 9. Open questions for the next agent
None beyond the family ones (asf_mjpeg audit §9).
