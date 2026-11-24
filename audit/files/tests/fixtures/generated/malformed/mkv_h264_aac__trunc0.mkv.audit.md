# AUDIT: tests/fixtures/generated/malformed/mkv_h264_aac__trunc0.mkv
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 40 / 58e60510997b00b3 |
| File verdict | REAL (class-B B1 truncation) |
| Reachable? | NO (test fixture); consumers: crates/demux/tests/mkv_fixtures.rs:350 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## 1. Verdict
Class-B trunc0: byte-exact 40-B prefix of the valid MKV parent
(tests/fixtures/generated/valid/mkv_h264_aac.mkv, 26,105 B — python3
verified). EBML magic + header, nothing else: maximal early cut. Same
no-committed-generator gap as the webm family (F-001).
## 2. Verified
— python3: 40-B prefix of the parent (sha256 parent 42ece741f5d2986c).
— Consumer: crates/demux/tests/mkv_fixtures.rs:350 (trunc family).
## 3. Findings
F-001 [S4] no committed generator for the mkv derivative family (xtask
fixtures.rs generates avi/mp4 only — rg verified). Confidence: HIGH.
