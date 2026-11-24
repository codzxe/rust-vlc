# AUDIT: tests/fixtures/generated/malformed/webm_vp9_opus__trunc0.webm
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 36 / 52abd50f38424d8f |
| File verdict | REAL (class-B B1 truncation) |
| Reachable? | NO (test fixture); consumers: crates/demux/tests/mkv_fixtures.rs:359 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |

## 1. Verdict
Class-B trunc0: the first 36 bytes of the valid WebM parent — the EBML
header magic + header size + a truncated first element (python3:
byte-exact 36-B prefix of the 27,259-B parent). Maximal early truncation:
the demuxer must refuse with a typed error before any track parsing. Cut
offset class documented at tools/xtask/src/cmd/fixtures.rs:2514-2515
("B1 truncation: the parent's first `cut` bytes") — but see F-001.
## 2. Verified
— python3: 36-B prefix of the parent; EBML magic 1a 45 df a3 in head.
— Consumer: crates/demux/tests/mkv_fixtures.rs:359 (trunc family).
## 3. Findings
F-001 [S4] provenance: the webm/mkv derivative family shares the
no-committed-generator gap with the aiff family (xtask generates only
avi/mp4 — see aiff audit F-001); mkv_fixtures.rs pins behavior but no tool
re-creates these bytes. Confidence: HIGH — Verification: rg over tools/ and
scripts/.
