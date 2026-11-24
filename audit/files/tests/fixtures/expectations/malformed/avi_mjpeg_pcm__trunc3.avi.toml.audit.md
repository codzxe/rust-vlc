# AUDIT: tests/fixtures/expectations/malformed/avi_mjpeg_pcm__trunc3.avi.toml
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | data contract, 30 lines |
| File verdict | REAL (sha/size/derivation recomputed and MATCH; prefix property verified) |
| Reachable from shipped entry point? | NO (test contract) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha + prefix verification (exit 0) |

## 1. Verdict
Verified: sha256 5c1a6d05... recomputed MATCH; size 212_512 MATCH; the fixture is an EXACT
byte prefix of the parent (python3 `p[:212512] == t` -> True), matching op = "truncate",
cut_offset = 212_512 ("cut mid-payload of the frame-30 00dc chunk"). Same union error
contract as noidx0 (typed-error-or-bounded-recovery, never panic/hang/unbounded).

## 2. Consumers
tests/integration/first-vertical/tests/recovery_matrix.rs (per-row wall-cap measurement),
class_b.rs union-contract assertions; soak scenario lists rotate it
(tests/soak/scenarios.toml:102). Drift-gated via `cargo xtask fixtures --check`.

## 3. Findings
None. Contract internal arithmetic consistent; fixture derivation byte-verified.

## 4. Open questions
None.
