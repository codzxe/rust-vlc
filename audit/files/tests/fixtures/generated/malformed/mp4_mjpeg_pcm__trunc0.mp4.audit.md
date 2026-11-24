# AUDIT: tests/fixtures/generated/malformed/mp4_mjpeg_pcm__trunc0.mp4
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Binary fixture (class-B truncation, 28 bytes = ftyp only) |
| Size / sha256 | 28 bytes / 1d514cd1aff19efd8f5b0f0e537c0f39c71ee0a4036a64dfccfb7aa0a6b0673d (verified locally) |
| File verdict | REAL (generated, hash-locked, consumed) |
| Reachable from shipped entry point? | N/A — test asset |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256 (matches MANIFEST.toml:154), rg consumers |

## 1. Verdict
Real class-B truncation: exactly the 28-byte ftyp box
(`\x00\x00\x00\x1cftypisom\x00\x00\x00\x00isomiso2mp41`), everything after
cut. sha256/size match MANIFEST.toml:150-155. This file is also the subject
of the assigned expectation TOML (audited separately).

## 2. What actually works (verified)
- Consumer (verbatim, crates/media-probe/tests/fixtures_matrix.rs:399-406):
```
// trunc0: 28 bytes — the writer's complete ftyp (isom + version +
// isom/iso2/mp41). Brand-rung only.
let derivation = recorded_derivation("expectations/malformed/mp4_mjpeg_pcm__trunc0.mp4.toml");
assert_eq!(derivation.op, "truncate");
assert_eq!(derivation.cut_offset, Some(28), "cut right after ftyp");
assert_verdict(
    "generated/malformed/mp4_mjpeg_pcm__trunc0.mp4",
```
- Also consumed at tests/integration/first-vertical/tests/recovery_matrix.rs:701 and by the compat-matrix walker.

## 3. Findings
None. Smallest possible honest truncation case, pinned by hash and
cross-checked against its expectation TOML (cut offset 28 verified above).
