# AUDIT: crates/demux/src/asf/fixtures/asf_pkt8000.wmv
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-21 |
| Lines / functions | 16,899-byte binary / n/a (test asset) |
| File verdict | REAL (test asset: committed, sha-pinned, byte-reproducible from its in-repo recipe) |
| Reachable from shipped entry point? | N/A (test asset). Consumed by `fixture_family_walks_match_the_measured_truth` (truth entry asf/tests.rs:1020-1036) + ffprobe cross-check (tests.rs:1325) — executed ok in baseline (log 1567/1953). Consuming module UNREACHABLE from shipped entry points (guid.rs audit F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 (family findings filed in the asf_mjpeg.asf audit; cross-referenced in §3, not recounted) |
| Commands run | sha256sum (pin match) · ffmpeg 7.1.5 regeneration from tests.rs:1022 recipe incl. `-packet_size 8000` (byte-identical) · python3 audit/analysis/asf_parse.py (walk == truth entry) · ffprobe codec_name (`wmv2`,`wmav2` ✓) |

## 1. One-paragraph verdict
Genuine, sha-pinned, byte-exact-reproducible ffmpeg 7.1.5 output of the family's packet-size variant: same A/V inputs as `wmv_wmv2_wma_av.wmv` plus `-packet_size 8000`, which the bytes prove took effect (FPO min=max=8000 vs the family default 3200; Data Object packs the same 2 tracks into 2×8000-byte-class packets at 16,050 B vs the sibling's 3×3200-class at 9,650 B). Every typed truth fact verified by an independent walk; the A/V preroll-grid first-pts ([64, 0]) is asserted against ffprobe's own arithmetic. No file-specific defects; family findings cross-referenced.

## 2. What actually works (verified)
- sha256 `e9ef0bc063ccc02f41495e377eb30abd7a07a170ae14ea2f1cdd15fcfd5d2a35` == pin `e9ef0bc063ccc02f` (tests.rs:1493).
- Regeneration from tests.rs:1022 recipe (`… -c:v wmv2 -b:v 100k -c:a wmav2 -b:a 32k -packet_size 8000`; plan §1.2:66 "… same inputs … -packet_size 8000") under ffmpeg 7.1.5 → identical sha256.
- Independent walk == truth (tests.rs:1020-1036): HEADER 757 / 6 children (FPO, Header Extension 156, ECD 98, SP video 133, SP audio 114, Codec List 122) · FPO file_size 16899 == real size, 2 packets, preroll 3100, **min=max=8000** (the variant's whole point — non-default fixed packet size), flags 0x02 · DATA at 757 size 16050 count 2, reserved `01 01` · SIMPLE_INDEX at 16807 size 92 (757+16050=16807; +92=16899 exact) · video SP stream 1: WMV2, 64x48, data_size/biSize 44, extradata 4 · audio SP stream 2: WFX tag 0x0161, 1 ch, 8000 Hz, align 256, bits 16, cb_size 10, TSD 28 == 18+cb.
- ffprobe oracle: `wmv2`, `wmav2` == tests.rs:1035, in track order; stream count 2 == tracks.
- Baseline: differential + ffprobe cross-check ok (log 1567, 1953).

## 3. Findings
None specific to this file. Cross-references (filed and counted in the asf_mjpeg.asf audit):
- F-001 [S3, cross-ref]: both SP EC fields carry the real GUIDs (20FB5700-… video, BFC3CD50-…00AA00B4E220 audio), refuting guid.rs's two EC constants.
- F-002 [S4, cross-ref]: outside the xtask fixtures drift gate; in-test sha pin compensates.
- F-003 [S4, cross-ref]: missing committed copy regenerates into src/ via tests.rs:1100-1118.

## 4. Per-function table (asset checks)
| Check | Result | Notes |
|---|---|---|
| Exists / size sane | YES / 16,899 B | == FPO file_size; plan §4.4:458 row ✓ |
| Structure matches the truth table | YES (all facts) | Independent walk == tests.rs:1020-1036 |
| sha256 pin | MATCH | tests.rs:1493 |
| Recipe reproducibility | BYTE-EXACT | ffmpeg 7.1.5 + tests.rs:1022 → identical sha |
| Variant property verified in bytes | YES | FPO min_packet_size == max_packet_size == 8000 (vs family default 3200); 2 packets × 8000-class vs sibling's 3 × 3200-class |
| Referenced by code/docs | YES | tests.rs:1021, :1493; plan §1.2:66, §4.4:393+458 |
| Determinism / drift gate | TEST-TIME ONLY | sha pin in-test (cross-ref F-002) |
| Hostile value | N/A | Clean-file fixture; exercises the demuxer's fixed-non-default-packet-size acceptance (min==max passes; min!=max refuses typed, tests.rs:1577) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `fixture_family_walks_match_the_measured_truth` | YES | Includes the packet-size assertions `min==truth.fpo.3` AND `max==truth.fpo.3` (tests.rs:1202-1203) — break the FPO max-packet read and this file's 8000 assert fires. Plus the A/V facts: two-track table (:1185-1194), WFX sextuple on streams-find (:1208-1230, streams[0] is video — the find(|s| wave_format.is_some()) discipline), video VIH (:1232-1244), first_pts [64, 0] with monotonic stamps (:1292-1299), seek+resume (:1304-1320). |
| `fixture_family_cross_checks_against_live_ffprobe` | YES (ffprobe present) | Two-stream codec names + per-track packet counts vs `-count_packets`; hand re-run: `wmv2`,`wmav2` ✓. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "asf_pkt8000.wmv 16,899 e9ef0bc063ccc02f" + lane "packet-size variant" | plan §4.4:458 | REAL | sha256sum + FPO min=max=8000 in bytes |
| "`-packet_size` interaction; the pkt8000 fixture covers size variety" | plan §4.4:393 | REAL | The only family member with non-default packet size; asserted at tests.rs:1202-1203 |

## 7. Dead / duplicated / unreachable code
Alive test input; consumer module dead in the shipped product (guid.rs audit F-001).

## 8. Dependencies & flags
None (data file). cfg(test) consumption only.

## 9. Open questions for the next agent
None beyond the family ones (asf_mjpeg audit §9).
