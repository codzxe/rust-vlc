# AUDIT: crates/demux/src/asf/fixtures/wma_wmav1_st44k.wma
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-21 |
| Lines / functions | 10,102-byte binary / n/a (test asset) |
| File verdict | REAL (test asset: committed, sha-pinned, byte-reproducible from its in-repo recipe) |
| Reachable from shipped entry point? | N/A (test asset). Consumed by `fixture_family_walks_match_the_measured_truth` (asf/tests.rs:1127, truth entry 933-946) + ffprobe cross-check (tests.rs:1325) — executed ok in baseline (log 1567/1953). Consuming module UNREACHABLE from shipped entry points (guid.rs audit F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 (family findings filed in the asf_mjpeg.asf audit; cross-referenced in §3, not recounted) |
| Commands run | sha256sum (pin match) · ffmpeg 7.1.5 regeneration from tests.rs:935 recipe (byte-identical) · python3 audit/analysis/asf_parse.py (structural walk == truth entry) · ffprobe codec_name (`wmav1` ✓) · od of the audio SP EC field |

## 1. One-paragraph verdict
Genuine, byte-reproducible ffmpeg 7.1.5 output of its committed recipe, sha-pinned, and structurally consistent with every typed fact its `FixtureTruth` entry asserts — I verified each independently (walk + regeneration + live ffprobe). The name honestly encodes the content (wmav1, stereo, 44.1 kHz), unlike the adts family's misnamed derivatives. Nothing in or around this specific file is defective beyond the family-level findings already filed (guid.rs EC constants contradicted by this file's audio SP bytes — cross-ref; plan gate promises undelivered — cross-ref; tests writing regenerations into src/ — cross-ref).

## 2. What actually works (verified)
- sha256 `40e3e4b6eb3e707b7a048127bbd25a8ddb06fa48a1d590a084f4bfd0c059aeba` == pin `40e3e4b6eb3e707b` (tests.rs:1487) — sha256sum.
- Regeneration from the recipe `…sine=frequency=440:duration=2:sample_rate=44100 -ac 2 -c:a wmav1 -b:a 32k` (tests.rs:935; plan §1.2 quotes it verbatim at asf-selfbuild.md:56) under ffmpeg 7.1.5 → identical sha256 (audit/analysis/gen/).
- Independent walk == truth entry (tests.rs:933-946): HEADER 452 / 5 children (FPO 104, Header Extension 46, ECD 98, SP audio 108, Codec List 66) · FPO file_size 10102 == real size, 3 packets, preroll 3100, min=max=3200, flags 0x02 (not broadcast, seekable) · DATA at 452 size 9650 count 3, reserved `01 01` · 452+9650=10102 exact · NO Simple Index object (truth `None`; asserted absent at tests.rs:1174-1181) · SP stream 1, audio type GUID, WFX tag 0x0160, 2 channels, 44100 Hz, block_align 185, bits 16, cb_size 4, TSD length 22 == 18+cb_size (asserted tests.rs:1230).
- ffprobe oracle: `codec_name` = `wmav1` == tests.rs:945.
- Baseline: differential + ffprobe cross-check ok (log 1567, 1953).

## 3. Findings
None specific to this file's bytes, name, or truth entry. Cross-references (filed and counted in audit/files/crates/demux/src/asf/fixtures/asf_mjpeg.asf.audit.md):
- F-001 [S3, cross-ref]: this file's audio SP (object at 278) EC field at offset 318 is `50 cd c3 bf 8f 61 cf 11 8b b2 00 aa 00 b4 e2 20` = BFC3CD50-618F-11CF-8BB2-00AA00B4E220 — the audio-side evidence that guid.rs's `EC_AUDIO_SPREAD` constant (…00AAB4E220C0) contradicts the fixtures it claims to be "byte-verified" on.
- F-002 [S4, cross-ref]: not under the xtask fixtures drift gate (gate covers 55 files under tests/fixtures only); in-test sha pin compensates at test time.
- F-003 [S4, cross-ref]: a missing committed copy of this file would be regenerated INTO src/asf/fixtures/ by tests.rs:1100-1118.

## 4. Per-function table (asset checks)
| Check | Result | Notes |
|---|---|---|
| Exists / size sane | YES / 10,102 B | == FPO file_size; plan §4.4 row "10,102 40e3e4b6eb3e707b" ✓ |
| Structure matches the truth table | YES (all facts) | Independent walk (asf_parse.py) == tests.rs:933-946 |
| sha256 pin | MATCH | tests.rs:1487 |
| Recipe reproducibility | BYTE-EXACT | ffmpeg 7.1.5 + tests.rs:935 → identical sha |
| Name describes content | YES | wmav1 + stereo + 44.1k — all three verified in WFX bytes |
| Referenced by code/docs | YES | tests.rs:934 (truth), :1487 (pin); plan §1.2:56, §4.4:452 |
| Determinism / drift gate | TEST-TIME ONLY | sha pin in-test; not in xtask gate (cross-ref F-002) |
| Hostile value | N/A | Clean-file fixture by design |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `fixture_family_walks_match_the_measured_truth` | YES | 12+ typed facts for this entry incl. the WFX sextuple (tag/ch/rate/align/bits/cb — tests.rs:1214-1219), the TSD-length == 18+cb_size seam assert (:1230), no-Simple-Index assert (:1174-1181), 3-packet drain with first pts 0 and monotonic stamps (:1292-1299). Mutation: mis-parse block_align and :1217 fails. |
| `fixture_family_cross_checks_against_live_ffprobe` | YES (ffprobe present) | codec name + stream count + per-track packet counts vs `-count_packets`; I re-ran the codec oracle by hand (`wmav1` ✓). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "wma_wmav1_st44k.wma 10,102 40e3e4b6eb3e707b" + lane "wmav1 variant" | plan §4.4:452 | REAL | sha256sum + size + WFX tag 0x0160 |
| §1.2 recipe (plan:56) == tests.rs generate string | plan §1.2 / tests.rs:935 | REAL | Verbatim match; byte-exact regeneration performed |

## 7. Dead / duplicated / unreachable code
Alive test input; consumer module dead in the shipped product (guid.rs audit F-001).

## 8. Dependencies & flags
None (data file). cfg(test) consumption only.

## 9. Open questions for the next agent
None beyond the family ones (asf_mjpeg audit §9).
