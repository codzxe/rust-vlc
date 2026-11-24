# AUDIT: tests/fixtures/generated/malformed/mp4_3gp_h263__trunc0.3gp
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary, 9000 bytes |
| File verdict | REAL (derivation verified byte-level) |
| Reachable from shipped entry point? | NO (test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 prefix + box walk (exit 0); rg consumer (exit 0) |

## 1. Verdict
Genuine 3GP truncation derivative: byte head `0000001c66747970 336770 34` = `ftyp 3gp4`
(size 28). VERIFIED: exact 9000-byte prefix of parent generated/valid/mp4_3gp_h263.3gp
(9520 bytes; parent boxes ftyp 28 / free 8 / mdat 8784 / moov 700 — cut lands inside mdat,
520 bytes before moov). sha256 02b78c2b67facdd7... (recomputed).

## 2. Consumers
crates/demux/tests/mp4_fixtures.rs:234 (malformed walk asserting typed refusal/no panic at
open) and :92 pins the parent's truth (h263 video + audio track facts). docs/corpus jsonl
records an outcome row.

## 3. Findings
F-001 [S3] — systemic manifest-gap: outside the 28-file drift gate; no generator row in
tools/xtask/src/cmd/fixtures.rs (the 3gp family is hand/agent-authored). CORPUS.md:10-11's
tree-wide authorship claim FALSE here. Confidence HIGH — ran.

## 4. Open questions
None.
