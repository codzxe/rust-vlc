# AUDIT: crates/codecs/tests/fixtures/breadth/yamaha_st_sizes.json (binary asset / data)
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Type / size / sha256 | JSON text: flat array `[1024 × 35]` / 210 bytes / 909199b29c6a5cadfebca978b64a636751b77a9d09001d9aab4a4ab5dbbe84c0 |
| File verdict | REAL (packet-shape pin, consistency-verified and exercised by a passing suite) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `sha256sum`; `cat` -> 35 × 1024; `stat` sibling payload -> 35840 bytes == 35×1024 exactly; `./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b` -> 4 passed / 0 failed (ran earlier this session, includes the yamaha_st lane this file shapes) |

## 1. Verdict
The ffprobe-captured packet-size record for the `yamaha_st_payload.bin` lane of the ADPCM breadth differential: 35 packets × 1024 bytes, summing exactly to the sibling payload's 35840 bytes. The consuming suite parses it (`parse_sizes`, adpcm_breadth_differential.rs:131+) and drives the decode packet-by-packet through those shapes, requiring the bit-exact MD5 `fcf6aecad5c50e142a74235c43624f55`; the geometry test also pins the shape tuple `(yamaha_st_payload.bin, 35, 1024)` — both ran green when I executed the suite. Consistent, live-used, no inflation.

## 2. Test usage (verified by execution)
`./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b` -> **4/4 pass** (bit-exact corpus vs ffmpeg references, geometry shapes incl. this file's 35×1024 pin, fuzz-lite, RFC-1321 MD5 self-check). A stale payload/sizes pair fails loudly by design (the geometry guard).

## 3. Matrix evidence
Part of the adpcm breadth lane's evidence set (worklog table row "adpcm_yamaha stereo 22050 | yamaha_st_payload.bin (35 × 1024)"). The committed numbers match the bytes on disk.
