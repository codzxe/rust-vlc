# AUDIT: tests/fixtures/generated/malformed/avi_mjpeg_pcm__trunc1.avi
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 52 bytes / 71fe311aefeea1d5032eb75868ee7cb5691a225978ecaf962c3f5ec0357b9cc9 |
| Type | RIFF AVI header fragment (cut mid-avih) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 prefix check; rg consumers |

## 1. Verdict
52-byte exact prefix of parent `avi_mjpeg_pcm.avi` (python3: `t1 == p[:52]`), cut mid-`avih` (main AVI header). MANIFEST.toml sha match. Expectation `tests/fixtures/expectations/malformed/avi_mjpeg_pcm__trunc1.avi.toml` pins `cut_offset = 52`, union error contract `typed-error-or-bounded-recovery`, `never = "panic, hang, unbounded allocation"`, rss_cap 64MB / wall_cap 5s. Consumers: crates/demux/tests/fixtures_class_b.rs (typed truncation boundary assertions — `assert_truncated` checks `ErrorKind::Corrupt{Truncated{expected,found}}` + context `demux.avi.truncated`); tests/integration/first-vertical/tests/recovery_matrix.rs:714. Baseline ok (cargo-test-workspace.txt:2242, 2353-2368 for related suites).
## 2. What actually works (verified)
python3 prefix check; expectation TOML sha256 field equals computed digest; baseline runs cited.
