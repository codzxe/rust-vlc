# AUDIT: tests/fixtures/expectations/malformed/avi_mjpeg_pcm__noidx0.avi.toml
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | data contract, 34 lines |
| File verdict | REAL (sha/size/derivation all recomputed and MATCH) |
| Reachable from shipped entry point? | NO (test contract) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha + structure verification (exit 0) |

## 1. Verdict
Verified end-to-end against the bytes: sha256 5752528d... recomputed MATCH; size 423_588
MATCH; the fixture genuinely contains NO `idx1` (b'idx1' not found); the RIFF size field is
patched to 423580 (= 423588 − 8), exactly as the `[derivation]` documents ("idx1 dropped,
RIFF size patched"). Parent (425516 bytes, RIFF field 425508) minus the 1928-byte idx1 tail
= 423588 — the arithmetic closes.

## 2. Contract semantics
Union error contract `typed-error-or-bounded-recovery` with never-panic/hang/unbounded caps
(rss 64 MB, wall 5 s) — the contract class_b.rs/recovery_matrix.rs assert per row (per
baseline B1: first-vertical package exercises these; 2 failures there are in evidence.rs
transcript drift, not this contract).

## 3. Findings
None. Inside the 28-file drift gate (byte-compared by `cargo xtask fixtures --check`).

## 4. Open questions
None.
