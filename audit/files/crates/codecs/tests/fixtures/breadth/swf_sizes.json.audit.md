# AUDIT: crates/codecs/tests/fixtures/breadth/swf_sizes.json (JSON data asset)
| Field | Value |
|---|---|
| Auditor | W06-A03 (recovery: executed by W06-R1, 2026-11-23) |
| Date | 2026-11-23 |
| Type / size / sha256 | JSON text: flat array `[4101 × 9]` / 54 bytes / 2ef7e315617a2f3e4ae54148f556c8743204e1b935e63f17fddfe1846e914075 |
| File verdict | REAL — packet-shape pin for the adpcm_swf breadth lane; geometry self-consistent and exercised by a passing suite |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `sha256sum`; python3 strict `json.load` -> 9 entries, all 4101, sum 36909 == `stat` swf_payload.bin 36909; `./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b` -> 4 passed / 0 failed (exit 0, 1.20s) |

## 1. Verdict
The ffprobe-captured packet-size record for the `adpcm_swf` stereo 22050 Hz
lane of the ADPCM breadth differential: 9 packets × 4101 bytes, summing
EXACTLY to the sibling payload's 36,909 bytes (python-verified). The
consuming suite parses it (`parse_sizes`, adpcm_breadth_differential.rs:133)
and drives the decode packet-by-packet through those shapes requiring the
pinned reference MD5 `9aac930510b6ef482b7303db62a7b200` (test header :19 and
Case :70-79); the geometry guard also pins the tuple
`("swf_payload.bin", 9, 4101)` (:304). All four suite tests ran green on the
prebuilt binary this session, including
`breadth_corpus_is_bit_exact_against_ffmpeg_reference`, which is the test
that actually walks these 9 shapes. Consistent with the worklog record
(docs/worklog/archive-2026-10.md:9338 — same 9×4101, same MD5). No
inflation.

## 2. Test usage (verified by execution)
`./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b` ->
**4/4 pass** (bit-exact corpus incl. the swf lane, geometry shapes, fuzz-lite,
RFC-1321 MD5 self-check). A stale payload/sizes pair fails loudly by design
(the geometry guard at :299-309).

## 3. Matrix evidence
Feature-matrix row `codec-audio-adpcm` (docs/feature-matrix.toml:793-803)
cites "8/8 breadth bit-exact" — this file is 1 of those 8 lanes. The
committed numbers match the bytes on disk and the worklog's authored record.
