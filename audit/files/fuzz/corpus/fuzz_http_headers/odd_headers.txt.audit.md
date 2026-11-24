# AUDIT: fuzz/corpus/fuzz_http_headers/odd_headers.txt
| Field | Value |
|---|---|
| Auditor | W16-A10 | Date | 2026-11-24 |
| Size / sha256 | 40 B / c11c91ca91996b4c6ed2fda2159d76bc539cc4a1cb79dd3b2bf70f2a36ad28b4 |
| File verdict | REAL (as data); the corpus copy itself is never read |
| Reachable from shipped entry point? | NO — fuzz corpus seed; byte-identical inline literal executed by the netfuzz sweep |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## 1. Verdict (corpus-seed audit)
Hand-authored hostile HTTP response head (40 bytes, CRLF): `HTTP/1.1 200` with no reason phrase, an empty-value header `X-Empty:`, and an `X-Two: a`/` b` obs-fold (continuation line) — exactly the odd shapes a header-grammar fuzzer needs. Its exact bytes are embedded INLINE in the stable netfuzz sweep: crates/streaming/tests/netfuzz_sweep.rs:148 `b"HTTP/1.1 200\r\nX-Empty:\r\nX-Two: a\r\n b\r\n\r\n"` — byte-for-byte the file's content. The corpus file itself is never read from fuzz/corpus.

## 2. What was checked and how
- batch.py: 40 B, 6 text lines, full content in batchmeta.txt.
- Byte-equality with the netfuzz inline literal: verified by reading both (file head == the b"" literal at netfuzz_sweep.rs:148).
- Twin execution: netfuzz_sweep passes in baseline (audit/runs/baseline/cargo-test-workspace.txt — netfuzz suite ok).
- Registry gate (existence only) run by me: exit 0.

## 3. Findings
F-001 [S3] Corpus-lane gap (shared): the seed's only fuzz-corpus consumers are the existence check and the unwired nightly campaign; the executed copy is a hardcoded literal. If someone edits the corpus file, nothing notices — the inline literal is the real seed of record. Verification: ran (rg) + read (byte compare).
