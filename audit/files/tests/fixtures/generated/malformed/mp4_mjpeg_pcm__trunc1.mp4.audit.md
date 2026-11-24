# AUDIT: tests/fixtures/generated/malformed/mp4_mjpeg_pcm__trunc1.mp4
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary, 64 bytes |
| File verdict | REAL (in-manifest; sha/size MATCH x3; prefix property verified) |
| Reachable from shipped entry point? | NO (test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256 + prefix check (exit 0) |

## 1. Verdict
Class-B truncation derivative INSIDE the drift gate: 64 bytes, sha256 0730b80c4b7010c8...
recomputed MATCH (expectation toml, MANIFEST.toml:158 block, CORPUS.md:93 —
`size_bytes = 64` records MATCH). VERIFIED exact byte prefix of the parent (64-byte head:
`ftyp isom` + start of the following box). Cut mid-moov-region per the family's structure
map (expectation documents "cut offset recorded").

## 2. Consumers
crates/codecs/tests/fuzz_survival.rs, recovery_matrix.rs (NEVER RAN in the committed
baseline — see wire_mode.rs audit F-001), media-probe fixtures_matrix (PACKAGE media-probe
exit_code=0 in baseline — that walk ran). docs/corpus jsonl records its refused outcome.

## 3. Findings
None. Integrity triple-verified; drift-gated.

## 4. Open questions
- recovery_matrix's pinned row for trunc1 is UNVERIFIED-run (baseline halt, systemic S2).
