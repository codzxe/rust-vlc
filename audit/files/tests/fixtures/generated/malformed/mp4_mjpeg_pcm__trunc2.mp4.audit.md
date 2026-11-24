# AUDIT: tests/fixtures/generated/malformed/mp4_mjpeg_pcm__trunc2.mp4
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary, 2208 bytes |
| File verdict | REAL (in-manifest; sha/size verified vs 3 records; prefix property verified) |
| Reachable from shipped entry point? | NO (test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256 batch + prefix check (exit 0) |

## 1. Verdict
Class-B truncation derivative INSIDE the 28-file manifest drift gate: sha256
7216c90ce930746b... recomputed MATCH (expectation toml, MANIFEST.toml:166 block, CORPUS.md:94).
size 2208 == `size_bytes = 2_208`. VERIFIED exact byte prefix of the parent
generated/valid/mp4_mjpeg_pcm.mp4. `ftyp isom` head intact.

## 2. Consumers
crates/codecs/tests/fuzz_survival.rs (whole-corpus no-panic sweep), recovery_matrix.rs
(NEVER RAN in baseline — see wire_mode.rs audit F-001), CORPUS.md:94 documents
"cut offset recorded in the expectation file" (expectation: trunc2 cut at a structure
boundary — mid-moov region per the 2208-byte cut).

## 3. Findings
None. Protected by the drift gate; integrity cross-verified against 3 independent records.

## 4. Open questions
- recovery_matrix never ran in the committed baseline (S2 systemic) — its pinned outcome for
  this file is UNVERIFIED-run.
