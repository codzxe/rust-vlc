# AUDIT: crates/demux/src/asf/fixtures/wmv_wmv1_64x48.wmv
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-21 |
| Lines / functions | 7,119-byte binary / n/a (test asset) |
| File verdict | REAL (test asset: committed, sha-pinned, byte-reproducible from its in-repo recipe) |
| Reachable from shipped entry point? | N/A (test asset). Consumed by `fixture_family_walks_match_the_measured_truth` (truth entry asf/tests.rs:961-974) + ffprobe cross-check (tests.rs:1325) — executed ok in baseline (log 1567/1953). Consuming module UNREACHABLE from shipped entry points (guid.rs audit F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 (family findings filed in the asf_mjpeg.asf audit; cross-referenced in §3, not recounted) |
| Commands run | sha256sum (pin match) · ffmpeg 7.1.5 regeneration from tests.rs:963 recipe (byte-identical) · python3 audit/analysis/asf_parse.py (walk == truth entry) · ffprobe codec_name (`wmv1` ✓) |

## 1. One-paragraph verdict
Genuine, sha-pinned, byte-exact-reproducible ffmpeg 7.1.5 output of the family's wmv1 member: video-only, 64x48, fourcc `WMV1`, and the family's no-extradata video shape (BITMAPINFOHEADER data_size/biSize 40, extradata 0 — the contrast case to wmv2's 44/4, which the demuxer's `parse_video_info` treats as its two verified shapes). Every typed truth fact verified by an independent walk; ffprobe agrees on the codec name. No file-specific defects; family findings cross-referenced.

## 2. What actually works (verified)
- sha256 `3cfac4e86fee00fcb666b20af8e078d2f9e31c2e4f43d98c3a5a1d8f31777627` == pin `3cfac4e86fee00fc` (tests.rs:1489).
- Regeneration from tests.rs:963 recipe (`testsrc=size=64x48:rate=10:duration=1 -c:v wmv1 -b:v 100k`; plan §1.2:59 verbatim) under ffmpeg 7.1.5 → identical sha256.
- Independent walk == truth (tests.rs:961-974): HEADER 583 / 5 children (FPO 104, Header Extension 156, ECD 98, SP video 129, Codec List 66) · FPO file_size 7119 == real size, 2 packets, preroll 3100, min=max=3200, flags 0x02 · DATA at 583 size 6450 count 2, reserved `01 01` · SIMPLE_INDEX at 7033 size 86 (583+6450=7033; +86=7119 exact) · SP stream 1 video: 64x48, data_size/biSize 40, bit_count 24, fourcc `WMV1`, size_image 9216, extradata 0, TSD 51.
- ffprobe oracle: `codec_name` = `wmv1` == tests.rs:973.
- Baseline: differential + ffprobe cross-check ok (log 1567, 1953).

## 3. Findings
None specific to this file. Cross-references (filed and counted in the asf_mjpeg.asf audit):
- F-001 [S3, cross-ref]: this file's video SP EC field carries the real no-error-correction GUID (20FB5700-5B55-11CF-A8FD-00805F5C442B), refuting guid.rs's all-zero `EC_NONE` "observed" claim.
- F-002 [S4, cross-ref]: outside the xtask fixtures drift gate; in-test sha pin compensates.
- F-003 [S4, cross-ref]: missing committed copy regenerates into src/ via tests.rs:1100-1118.

## 4. Per-function table (asset checks)
| Check | Result | Notes |
|---|---|---|
| Exists / size sane | YES / 7,119 B | == FPO file_size; plan §4.4:454 row ✓ |
| Structure matches the truth table | YES (all facts) | Independent walk == tests.rs:961-974 |
| sha256 pin | MATCH | tests.rs:1489 |
| Recipe reproducibility | BYTE-EXACT | ffmpeg 7.1.5 + tests.rs:963 → identical sha |
| No-extradata shape verified | YES | data_size=biSize=40, extradata 0 (vs wmv2 sibling's 44/4) — one of `parse_video_info`'s two verified shapes (header.rs:810-815) |
| Referenced by code/docs | YES | tests.rs:962, :1489; plan §1.2:59, §4.4:454 |
| Determinism / drift gate | TEST-TIME ONLY | sha pin in-test (cross-ref F-002) |
| Hostile value | N/A | Clean-file fixture; wmv1 not a shipped decoder (demux-only lane by design, plan §1.3:98) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `fixture_family_walks_match_the_measured_truth` | YES | Typed facts incl. fourcc `WMV1` (tests.rs:1240), extradata 0 (:1243), Simple Index offset/size (:1164-1173), first_pts [0] + monotonic stamps (:1292-1299), seek+resume (:1304-1320). Mutation: make `parse_video_info` accept only biSize 44 and :1240/:1243 fail for this file. |
| `fixture_family_cross_checks_against_live_ffprobe` | YES (ffprobe present) | Codec name + packet count oracles; hand re-run: `wmv1` ✓. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "wmv_wmv1_64x48.wmv 7,119 3cfac4e86fee00fc" + lane "wmv1 variant" | plan §4.4:454 | REAL | sha256sum + size |
| "Measured `data_size`/`biSize` pairs: 40/40 (wmv1, msmpeg4v3, mjpeg — no extradata)" | header.rs:810-812 (comment) | REAL | This file's bytes: data_size 40, biSize 40, extradata 0 |

## 7. Dead / duplicated / unreachable code
Alive test input; consumer module dead in the shipped product (guid.rs audit F-001).

## 8. Dependencies & flags
None (data file). cfg(test) consumption only.

## 9. Open questions for the next agent
None beyond the family ones (asf_mjpeg audit §9).
