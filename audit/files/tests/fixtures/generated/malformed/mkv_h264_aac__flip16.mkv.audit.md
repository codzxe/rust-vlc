# AUDIT: tests/fixtures/generated/malformed/mkv_h264_aac__flip16.mkv
| Field | Value |
|---|---|
| Auditor | W16-A05 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Kind | Binary/fixture data audit |
| File verdict | REAL (bytes real, consumed by a green baseline test) |
| Reachable from shipped entry point? | N/A — test/corpus asset |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:0 |
| Commands run | file -b; python3 sha256 (audit/runs/W16-RR1.datahashes.tsv); rg consumers; baseline citations |

## 1. Verdict & evidence
Identity: EBML (Matroska), flip16 derivative of the mkv_h264_aac parent (size=26105, sha256=e16ef19d1be49119b7bd53c0c519c0ab7df2def72e0b15ef73ea0c6d26773a7f — batch table this run).
Consumer: crates/demux/tests/mkv_fixtures.rs:345 `malformed_derivatives_are_typed_or_recover_never_panics` (name listed at :357) — baseline cargo-test-workspace.txt:2358 `ok`, binary 9 passed :2366.
Generated-vs-handmade: derivative under generated/malformed/ (flip16 = 16-byte bit-flip op) of the ffmpeg-built parent — the dir convention per CORPUS.md, but the mkv family is NOT in the registered 28-file set.

## 2. Findings
F-001 [S3]
Where: tests/fixtures/generated/MANIFEST.toml (read in full, 28 rows)
What is wrong: this flip16 derivative is NOT registered in the manifest — no sha256 ledger
row, no drift-gate coverage; provenance rides on the naming convention + the passing test.
Same shape as W16-A02's cross-cutting unregistered-corpus finding.
Confidence: HIGH — Verification: read.
(tests/fixtures/generated/MANIFEST.toml read in full: 28 rows, no row for this file;
fuzz/README.md:99-119,165-168; audit/runs/baseline/full_gate.txt:56-58).
Checked otherwise: type via file, size/sha via python3, byte-level header inspection,
consumer rg across tests/ crates/ fuzz/ tools/ .github/, baseline test-result citations.
Nothing executes from this file itself in this recovery run.
