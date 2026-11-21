# AUDIT: crates/demux/src/asf/fixtures/asf_adpcm_ms.wma
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-21 |
| Lines / functions | 6,936-byte binary / n/a (test asset) |
| File verdict | REAL (test asset: committed, sha-pinned, byte-reproducible from its in-repo recipe) |
| Reachable from shipped entry point? | N/A (test asset). Consumed by `fixture_family_walks_match_the_measured_truth` (truth entry asf/tests.rs:1051-1064) + ffprobe cross-check (tests.rs:1325) — executed ok in baseline (log 1567/1953). Consuming module UNREACHABLE from shipped entry points (guid.rs audit F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 (family findings filed in the asf_mjpeg.asf audit; cross-referenced in §3, not recounted) |
| Commands run | sha256sum (pin match) · ffmpeg 7.1.5 regeneration from tests.rs:1053 recipe (byte-identical) · python3 audit/analysis/asf_parse.py (walk == truth entry) · ffprobe codec_name (`adpcm_ms` ✓) · od -A d -t x1 -j 318 -N 16 (the audio SP EC field — F-001 audio-side evidence) |

## 1. One-paragraph verdict
Genuine, sha-pinned, byte-exact-reproducible ffmpeg 7.1.5 output of the family's ADPCM member: the smallest committed asset (6,936 B), audio-only, no Simple Index, carrying the demuxer's ADPCM decoder-config seam in bytes — a WAVEFORMATEX TSD of exactly 18+32 bytes with tag 0x0002, 4 bits/sample, block_align 1024 and the 32-byte MS-ADPCM coefficient tail (cbSize 32), which the plan correctly identifies as the blob `AdpcmDecoder::parse_waveformat` consumes. Every typed truth fact verified by an independent walk; live ffprobe names the stream `adpcm_ms`. No file-specific defects — the plan's "decodable audio (ADPCM)" lane label is demux-only in the landed tests (family F-002 cross-ref), and this file's audio SP EC field is the byte-evidence that guid.rs's `EC_AUDIO_SPREAD` constant is wrong (family F-001 cross-ref).

## 2. What actually works (verified)
- sha256 `02ca9cfcf394078b6e040a175218f16fe94c436da79f3b3536af918eb7440826` == pin `02ca9cfcf394078b` (tests.rs:1495).
- Regeneration from tests.rs:1053 recipe (`sine=frequency=440:duration=1:sample_rate=8000 -ac 1 -c:a adpcm_ms -b:a 32k`; plan §1.2:70 verbatim) under ffmpeg 7.1.5 → identical sha256.
- Independent walk == truth (tests.rs:1051-1064): HEADER 486 / 5 children (FPO 104, Header Extension 46, ECD 98, SP audio 136, Codec List 72) · FPO file_size 6936 == real size, 2 packets, preroll 3100, min=max=3200, flags 0x02 · DATA at 486 size 6450 count 2, reserved `01 01` (486+6450=6936 exact) · NO Simple Index (truth `None`) · SP stream 1, audio type GUID, WFX tag 0x0002, 1 ch, 8000 Hz, block_align 1024, bits_per_sample 4, cb_size 32, TSD length 50 == 18+cb_size (the seam assert, tests.rs:1230).
- ffprobe oracle: `codec_name` = `adpcm_ms` == tests.rs:1063.
- Baseline: differential + ffprobe cross-check ok (log 1567, 1953).

## 3. Findings
None specific to this file. Cross-references (filed and counted in the asf_mjpeg.asf audit):
- F-001 [S3, cross-ref]: this file's audio SP (object at 278) EC field at offset 318 is `50 cd c3 bf 8f 61 cf 11 8b b2 00 aa 00 b4 e2 20` = BFC3CD50-618F-11CF-8BB2-00AA00B4E220 — od-verified; the audio-side evidence that guid.rs's `EC_AUDIO_SPREAD` (…00AAB4E220C0) contradicts the fixtures it claims to be "byte-verified" on.
- F-002 [S4, cross-ref]: the plan's "decodable audio (ADPCM)" label (§4.4:460) and its "decode-exact lanes for pcm/adpcm/mjpeg" gate promise (§4.4:468) never landed — this fixture is demuxed only; no test connects its TSD blob to `AdpcmDecoder` (the adpcm corpus differentials exercise the WAV lane with their own files). Also outside the xtask fixtures drift gate; in-test sha pin compensates.
- F-003 [S4, cross-ref]: missing committed copy regenerates into src/ via tests.rs:1100-1118.

## 4. Per-function table (asset checks)
| Check | Result | Notes |
|---|---|---|
| Exists / size sane | YES / 6,936 B | == FPO file_size; smallest family member; plan §4.4:460 row ✓ |
| Structure matches the truth table | YES (all facts) | Independent walk == tests.rs:1051-1064 |
| sha256 pin | MATCH | tests.rs:1495 |
| Recipe reproducibility | BYTE-EXACT | ffmpeg 7.1.5 + tests.rs:1053 → identical sha |
| Decoder-config seam in bytes | YES | TSD 50 B = WAVEFORMATEX 18 + cbSize 32 (MS coefficient tail); tag 0x0002/4-bit/align 1024 == plan §2.3:184-186's measured facts |
| Referenced by code/docs | YES | tests.rs:1052, :1495; plan §1.2:70 + §1.3:96, §2.3:184-186, §4.4:460 |
| Determinism / drift gate | TEST-TIME ONLY | sha pin in-test (cross-ref F-002) |
| Hostile value | N/A | Clean-file fixture by design |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `fixture_family_walks_match_the_measured_truth` | YES | WFX sextuple asserted (tests.rs:1214-1219 — incl. bits_per_sample 4 and block_align 1024, the ADPCM-specific fields) + the TSD-length seam assert `18 + cb` (:1230) + no-Simple-Index assert (:1174-1181) + 2-packet drain, first pts 0, monotonic (:1292-1299). Note: the seek lane (tests.rs:1304) is skipped for this file (indexless by design) — indexless seek is covered by the synthesized `indexless_files_seek_by_bounded_linear_scan`, not by this asset. Mutation: mis-read cb_size and :1219/:1230 fail. |
| `fixture_family_cross_checks_against_live_ffprobe` | YES (ffprobe present) | Codec name + per-track packet counts vs `-count_packets`; hand re-run: `adpcm_ms` ✓. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "asf_adpcm_ms.wma 6,936 02ca9cfcf394078b" | plan §4.4:460 | REAL | sha256sum + size |
| "tag 0x0002, align 1024, bits 4, cbSize 32 (the MS coefficient tail) — exactly the blob AdpcmDecoder::parse_waveformat consumes" | plan §2.3:184-186 | REAL (blob shape) / UNVERIFIED (the consumption link) | Blob shape verified in bytes (TSD 50 == 18+32); no landed test feeds THIS fixture's blob to AdpcmDecoder (cross-ref F-002) — the claim rests on the WAV-lane corpus differentials |
| Lane "**decodable audio (ADPCM)**" | plan §4.4:460 | SHALLOW | Demux-only in landed tests; ASF lane unreachable in the shipped binary (and engine-info's hard-coded shipped decoder list names mjpeg+pcm only, per baseline INDEX correction) |
| "adpcm_ms → AdpcmDecoder::configure_wav parses the WAVEFORMATEX blob from extra_data" | plan §1.3:96 | REAL-UNPROVEN (via this lane) | The decoder exists and is corpus-tested (codecs/tests/adpcm_*); the ASF→decoder wiring has no executed path |

## 7. Dead / duplicated / unreachable code
Alive test input; consumer module dead in the shipped product (guid.rs audit F-001).

## 8. Dependencies & flags
None (data file). cfg(test) consumption only.

## 9. Open questions for the next agent
- If the family decode-exact lane is ever landed (F-002's fix), this file + `asf_pcm_s16le.wma` (A01) are the two audio candidates; the `-f md5` oracle command shape is already in use in the wav/ivf corpus tests.
