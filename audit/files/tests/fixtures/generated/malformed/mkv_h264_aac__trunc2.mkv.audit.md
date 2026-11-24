# AUDIT: tests/fixtures/generated/malformed/mkv_h264_aac__trunc2.mkv
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 526 / 6222265d59e6293f |
| File verdict | REAL (class-B B1 truncation) |
| Reachable? | NO (test fixture); consumers: crates/demux/tests/mkv_fixtures.rs:352 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## 1. Verdict
Class-B trunc2: byte-exact 526-B prefix of the valid MKV parent (26,105 B —
python3 verified). Cut deeper than trunc0 — inside the SegmentInfo/track
header region; a real mid-structure EOF case. Same no-committed-generator
gap as the mkv/webm families (F-001).
## 2. Verified
— python3: 526-B prefix of the parent.
— Consumer: crates/demux/tests/mkv_fixtures.rs:352 (trunc family);
expectation toml exists (expectations/malformed/mkv_h264_aac__trunc2.mkv.toml).
## 3. Findings
F-001 [S4] no committed generator for the mkv derivative family (same class
as mkv trunc0 / webm trunc0 audits). Confidence: HIGH.
