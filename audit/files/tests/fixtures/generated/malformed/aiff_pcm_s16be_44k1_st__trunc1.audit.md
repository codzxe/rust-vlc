# AUDIT: tests/fixtures/generated/malformed/aiff_pcm_s16be_44k1_st__trunc1

| Field | Value |
|---|---|
| Auditor | W16-RR5 (orchestrator-completed; orig W16-A08) |
| Date | 2026-11-24 |
| File verdict | REAL (malformed binary fixture) |
| Reachable from shipped entry point? | N/A (test fixture; the CLI aiff.rs container cites this family) |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:0 |
| Commands run | sha256sum (bede6b917e031b31), cmp vs valid parent (prefix TRUE: 24 of 35334 bytes), rg consumer -> apps/cli/src/containers/aiff.rs |

## 1. One-paragraph verdict
A 24-byte truncation of the valid AIFF parent (verified byte-prefix).
Named "trunc1" but contains 24 bytes, i.e. truncation-to-24B not
"truncate 1 byte" — the family naming scheme (truncN = keep N+? bytes)
is inconsistent with the W4-documented prefix scheme. Consumed by the
aiff container fuzz/attack tests.

## 3. Findings
F-001 [S3] The naming convention ambiguity: trunc0 keeps 12B, trunc1
keeps 24B (this file) — the N is an index, not a byte count or a
"drop-N" count; nothing in fixtures.rs documents this. Misleading name.
Confidence: HIGH — Verification: read+cmp.
