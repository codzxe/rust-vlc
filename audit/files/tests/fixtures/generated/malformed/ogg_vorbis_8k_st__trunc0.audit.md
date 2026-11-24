# AUDIT: tests/fixtures/generated/malformed/ogg_vorbis_8k_st__trunc0

| Field | Value |
|---|---|
| Auditor | W16-RR5 (orchestrator-completed; orig W16-A08) |
| Date | 2026-11-24 |
| File verdict | REAL (malformed binary fixture) |
| Reachable from shipped entry point? | N/A (test fixture) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | sha256sum (80e46d0d62e25a5a), cmp vs valid parent (prefix TRUE: 12 of 3245 bytes) |

## 1. One-paragraph verdict
12-byte prefix of the valid OGG/Vorbis parent — a truncated-at-12-bytes
capture page fragment (page header only). Feeds the ogg demuxer
adversarial tests; the whole ogg malformed family sits OUTSIDE the
xtask 28-row MANIFEST drift gate (W16 systemic finding).

## 3. Findings
F-001 [S4] Outside fixtures --check manifest (no hash pin); regeneration
would silently change bytes undetected.
Confidence: HIGH — Verification: read (manifest rg).
