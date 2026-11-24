# AUDIT: tests/fixtures/generated/malformed/webm_vp9_opus__garbage0.webm
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 31,355 / 2415f73977bcfb89 |
| File verdict | REAL (class-B garbage-append derivative) |
| Reachable? | NO (test fixture); consumers: crates/demux/tests/mkv_fixtures.rs:367 |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:0 |

## 1. Verdict
Class-B garbage0 derivative of the WebM parent: byte-exact copy of
tests/fixtures/generated/valid/webm_vp9_opus.webm (27,259 B) with exactly
4,096 seeded garbage bytes appended past the declared EBML extent
(python3 prefix-compare verified; Δ = 4,096). Same construction class as the
AVI garbage0 audit (F-001 there): a size-field-respecting parser never reads
the appended bytes, so the mutation is weak by construction — the repo's own
fixtures_matrix.rs:28 admits "garbage0 is an append, so the honest verdict is
the parent's".
## 2. Verified
— python3: prefix == valid parent; +4,096 B; parent exists (27,259 B,
sha256 8b5c6ba741cb843b).
— Consumer: crates/demux/tests/mkv_fixtures.rs:367 lists it in the class-B
derivative set; expectation file
tests/fixtures/expectations/malformed/webm_vp9_opus__garbage0.webm.toml
exists on disk (ls).
## 3. Findings
F-001 [S3] invisible-mutation weakness (same as AVI garbage0): append past
extent exercises size accounting, not parser recovery. Confidence: HIGH.
