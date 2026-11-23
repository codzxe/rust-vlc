# AUDIT: crates/codecs/tests/fixtures/breadth/swf_payload.bin (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Type / size / sha256 | raw ADPCM-SWF audio payload (ffmpeg `-c:a adpcm_swf`, FLV-carried, `-f data` extract; first bytes `80 00 00 00 00 01 dd dd…` = the SWF ADPCM packet header shape) / 36909 bytes = 9 × 4101 + 0 per the recorded packetization / 77def57669a827804d155c58be6bb0ab15a1178b4ccd1e0cd745b6873f415799 |
| File verdict | REAL (differential decode evidence, executed) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` -> data; `sha256sum`; `od` header bytes; `./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b` -> 4 passed / 0 failed, 1.61s (incl. `breadth_corpus_is_bit_exact_against_ffmpeg_reference` and `corpus_geometry_matches_the_reference_shapes`) |

## 1. Verdict
Real ffmpeg-authored ADPCM-SWF stereo 22050 payload, one case of the eight-lane ADPCM breadth differential (`crates/codecs/tests/adpcm_breadth_differential.rs`, Case at :71-78, shapes table at :301-304). Not decorative: the suite decodes it packet-by-packet (9 × 4101 per the companion swf_sizes.json) through the in-house `adpcm_swf` decoder and requires bit-exact s16le MD5 `9aac930510b6ef482b7303db62a7b200` against the ffmpeg reference — both the corpus test and the geometry guard passed when I ran the suite.

## 2. Test usage (verified by execution)
`breadth_corpus_is_bit_exact_against_ffmpeg_reference` + `corpus_geometry_matches_the_reference_shapes` + `fuzz_lite_never_panics_and_stays_bounded` + `md5_implementation_matches_rfc1321_vectors` — **4/4 passed, 1.61s**. The geometry test explicitly pins this file's shape `(swf_payload.bin, 9, 4101)`, guarding against a stale fixture/sizes mismatch. 36909 = 9 × 4101 exactly, consistent.

## 3. Matrix evidence
Cited in the worklog differential table (docs/worklog/archive-2026-10.md:9338, "adpcm_swf stereo 22050 | swf_payload.bin (9×4101) | FLV enc→dec | 9aac9305…"). The committed MD5 and the live test agree; the fixture is the evidence behind the adpcm-swf breadth row, and the test I ran is the proof, not the string.
