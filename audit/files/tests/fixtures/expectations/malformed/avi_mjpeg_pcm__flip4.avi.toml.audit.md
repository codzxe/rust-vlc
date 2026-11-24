# AUDIT: tests/fixtures/expectations/malformed/avi_mjpeg_pcm__flip4.avi.toml
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 1,036 bytes / d30069f6e2688dff9c391011a0e6cdbcde74003c9fb12e9f3abcffcf7411cbad |
| File verdict | REAL (verified against bytes) |
| Reachable from shipped entry point? | NO — test expectation data |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | sha256 + python3 XOR diff vs parent; rg consumers |

## 1. Verdict
Pins `avi_mjpeg_pcm__flip4.avi` (sha256 004ba5af... — MATCHES computed digest of the fixture). Derivation: 4 seeded bit-flips within the header extent 64. python3 vs parent: 4 differing bytes at [7, 18, 58, 63], XOR masks [0x02, 0x10, 0x40, 0x01] — each exactly one bit, all inside the 64-byte header extent. The TOML's claims are exactly true in the bytes. Error contract: union `typed-error-or-bounded-recovery`, never panic/hang/unbounded alloc, rss 64MB / wall 5s. Consumer: crates/demux/tests/fixtures_class_b.rs (`per-file flavors are PINNED here` header L6-8; asserts `demux.avi.*` typed flavors).
## 2. Findings
F-001 [S3]
Where: :22-25 `expected = "typed-error-or-bounded-recovery"`
What is wrong: union contract — the TOML alone does not pin WHICH of the three flavors occurs; the specific flavor lives in fixtures_class_b.rs assertions. The expectation file under-specifies (documented as v0, admitted in comment L22-24). Confidence: HIGH — Verification: read.
## 3. What actually works (verified)
sha pin match + bitwise derivation match (python3 output above); baseline fixtures_class_b ok (cargo-test-workspace.txt:2242).
