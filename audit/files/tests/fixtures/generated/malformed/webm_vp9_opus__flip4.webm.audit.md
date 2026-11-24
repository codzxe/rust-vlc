# AUDIT: tests/fixtures/generated/malformed/webm_vp9_opus__flip4.webm
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 27,259 / fc9509a4181fabde |
| File verdict | REAL (class-B seeded bit-flip derivative) |
| Reachable? | NO (test fixture); consumers: crates/demux/tests/mkv_fixtures.rs:365 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |

## 1. Verdict
Class-B flip4 derivative: same size as the valid WebM parent (27,259 B),
NOT a prefix — 4 seeded single-bit XOR flips at recorded sites
(python3: size equal, byte-differs; construction class documented at
crates/demux/tests/mkv_fixtures.rs:358-367 and wav_aiff_fixtures.rs:38-40
"flips are seeded single-bit XORs at the recorded sites"). This is the
stronger mutation class: bytes INSIDE the parsed extent change, so the
demuxer must either recover or type-error.
## 2. Verified
— python3: same length as parent, distinct sha256; not a prefix → in-extent
mutation confirmed.
— Consumer: crates/demux/tests/mkv_fixtures.rs:365 (flip family); the
flip-site expectations are pinned in the expectations toml (derivative
family listed at :144 of scenario_09_corrupt.rs for AVI; mkv equivalent at
mkv_fixtures.rs:358-367).
## 3. Findings
None.
