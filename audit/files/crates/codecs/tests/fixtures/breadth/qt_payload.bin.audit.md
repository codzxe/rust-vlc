# AUDIT: crates/codecs/tests/fixtures/breadth/qt_payload.bin (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-R3 (credit W06-A08) |
| Date | 2026-11-23 |
| Size / type | 11 764 bytes; `file` -> "data" (raw ADPCM packet payload, not a container) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture); consumed by crates/codecs/tests/adpcm_breadth_differential.rs (Case `adpcm_ima_qt`, :79-84) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` -> data; `md5sum` -> 34ac2062c53befaa7882c59e5d18c7b7 (file integrity); `od` head (IMA-QT-style per-packet state bytes `00 00 70 77 ...` + ADPCM nibble body); `./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b --test-threads=1` -> 4 passed, exit 0 |

## 1. One-paragraph verdict
The `adpcm_ima_qt` stereo-22050 payload of the ADPCM breadth differential: ffmpeg-encoded (`-c:a adpcm_ima_qt`), MOV-carried, then extracted as 173 raw packets of 68 bytes (11 764 = 173 × 68 — verified by byte count and by the consuming geometry assert, which passed in my run). Its lane (`Case { descriptor: "adpcm_ima_qt", payload: "qt_payload.bin", sizes: Some("qt_sizes.json"), channels: 2, rate: 22_050, expected_md5: "a2cb46ce5b9ad2da111403ad6195bf3b" }`) executed green in my prebuilt-binary run (4/4, exit 0), i.e. this exact file decodes bit-exact to the pinned ffmpeg-7.1.5 reference MD5.

## 2. What actually works (verified)
- Geometry: `ls` size 11 764 = 173 × 68; `corpus_geometry_matches_the_reference_shapes` passed in my run (`./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b --test-threads=1` -> `4 passed; 0 failed`).
- Bit-exact lane: `breadth_corpus_is_bit_exact_against_ffmpeg_reference` passed — this fixture's decode MD5 equals the pinned `a2cb46ce5b9ad2da111403ad6195bf3b` (recorded from a documented ffmpeg 7.1.5-0+deb13u1 session, adpcm_breadth_differential.rs:9).
- Companion files present: `qt_sizes.json` (173-entry flat array, all 68 — head verified) pairs with this payload as the packet-size capture.

## 3. Findings
None. Checked: type (`file`: data), size vs the consumer's geometry table (173 × 68 = 11 764 exact), md5sum of the asset for this audit record (34ac2062c53befaa7882c59e5d18c7b7), consuming test executed green, doc/matrix citations present.

## 4-9. (binary asset — sections 4/5/7/8/9 not applicable)
Doc/matrix citations: docs/worklog/archive-2026-10.md breadth table row (`adpcm_ima_qt` stereo 22 050 | qt_payload.bin | 173 × 68 | MOV enc→dec | a2cb46ce...); feature-matrix.toml:802 `codec-audio-adpcm` evidence "8/8 breadth bit-exact" — this fixture is one of the 8 lanes (the claim's executed proof: my 4/4 run + baseline :919-928). Note: the pinned MD5 is a past-session record, not a live ffmpeg re-check (no ffmpeg spawn in this test — pin-only oracle; ffmpeg IS present in this sandbox but the original MOV is not committed, only the extracted payload, so the pin is not independently re-derivable without regenerating the container — carried in the consuming test's existing audit).
