# AUDIT: tests/fixtures/generated/malformed/mp4_mjpeg_pcm__trunc3.mp4
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Binary fixture (class-B truncation at a structure boundary) |
| Size / sha256 | 213878 bytes / 8829e98002b6003789a1781d9a0a7c54c17ebf7c5a01d07c94c5ec5572ba61c2 (verified locally) |
| File verdict | REAL (generated, hash-locked, consumed) |
| Reachable from shipped entry point? | N/A — test asset |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256 (matches MANIFEST.toml:178), header sniff, rg consumers |

## 1. Verdict
Real class-B truncation: header is a clean `ftypisom` (first 32 bytes
uncorrupted), file cut at 213878 of the parent's 424452 bytes — mid-media.
sha256/size match MANIFEST.toml:174-179 exactly.

## 2. What actually works (verified)
- MANIFEST.toml:174-179 pins `sha256 = "8829e98002b6003789a1781d9a0a7c54c17ebf7c5a01d07c94c5ec5572ba61c2"`, `size_bytes = 213_878` — local hash matches.
- Consumer (verbatim, tests/integration/first-vertical/tests/recovery_matrix.rs:736-740):
```
fn matrix_mp4_trunc3_opens_then_lands_in_error_typed() {
    let checks = drive(Row {
        fixture: "generated/malformed/mp4_mjpeg_pcm__trunc3.mp4",
        outcome: Outcome::ErrorAfterOpenNoFlush {
            message_part: "demux.mp4.truncated",
```
- Also consumed by compat-matrix malformed_root walker and expectations/malformed/mp4_mjpeg_pcm__trunc3.mp4.toml.

## 3. Findings
None. Generated, pinned, consumed with a typed-error contract.
