# AUDIT: tests/fixtures/expectations/malformed/avi_mjpeg_pcm__trunc1.avi.toml
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 925 bytes / 932ab7ff283186b515bfa55b884d2f09efddf2e85543135105299d3576855b0e |
| File verdict | REAL (verified against bytes) |
| Reachable from shipped entry point? | NO — test expectation data |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | sha256 batch; python3 prefix check; rg consumers |

## 1. Verdict
Pins `avi_mjpeg_pcm__trunc1.avi`: sha256 71fe311a... (MATCHES computed digest), size 52, derivation `truncate`, `cut_offset = 52`, note "cut mid-avih (main AVI header)" — verified true (python3: file is exact 52-byte prefix of parent; bytes 44-51 sit inside the 56-byte `avih` payload). Error contract: union `typed-error-or-bounded-recovery`, `never = "panic, hang, unbounded allocation"`, rss_cap_mb 64 / wall_cap_ms 5000. Consumers: fixtures_class_b.rs (typed truncation boundary assertions — `assert_truncated` pins `ErrorKind::Corrupt{Truncated{expected, found}}` + `demux.avi.truncated`), first-vertical recovery_matrix.rs:714.
## 2. Findings
F-001 [S3]
Same union-contract under-specification as the flip4 TOML (flavor pinned by the test suite, not the expectation file). Confidence: HIGH — Verification: read.
## 3. What actually works (verified)
sha + cut-offset + prefix property all verified by python3; baseline ok (cargo-test-workspace.txt:2242).
