# AUDIT: tests/fixtures/expectations/malformed/mp4_mjpeg_pcm__trunc0.mp4.toml
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Data fixture (expectation/1 TOML for the trunc0 derivative) |
| Size / sha256-prefix | 929 bytes / 06573c610d14dbd4 (verified locally) |
| File verdict | REAL — consumed and cross-checked by media-probe tests |
| Reachable from shipped entry point? | N/A — test asset |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256; read file; rg consumers |

## 1. Verdict
Real expectation TOML: pins the derivative's own sha256 (`1d514cd1...`, which
I independently verified against the actual .mp4 bytes — MATCH), records
derivation (`truncate`, `cut_offset = 28`) and an error CONTRACT
(typed-error-or-bounded-recovery, never panic/hang/unbounded alloc, RSS cap
64 MB, wall cap 5 s). Consumed by crates/media-probe/tests/fixtures_matrix.rs
(recorded_derivation + assert_verdict).

## 2. What actually works (verified)
- sha256 field == actual bytes of generated/malformed/mp4_mjpeg_pcm__trunc0.mp4 (both `1d514cd1aff19efd...`) — cross-verified locally.
- Consumer (verbatim, crates/media-probe/tests/fixtures_matrix.rs:401-406):
```
let derivation = recorded_derivation("expectations/malformed/mp4_mjpeg_pcm__trunc0.mp4.toml");
assert_eq!(derivation.op, "truncate");
assert_eq!(derivation.cut_offset, Some(28), "cut right after ftyp");
assert_verdict(
    "generated/malformed/mp4_mjpeg_pcm__trunc0.mp4",
```
- Ledger-integrity: the xtask `--check` byte-compares expectations/ (CORPUS.md:103-104).

## 3. Findings
F-001 [S3]
Where: tests/fixtures/expectations/malformed/mp4_mjpeg_pcm__trunc0.mp4.toml:24-27 ([error] block)
What is wrong: The error contract is a UNION — `expected = "typed-error-or-bounded-recovery"` — deliberately vague (comment: "v0: the union contract"); no per-flavor pin in the expectation itself. The precise outcome is pinned only in a different crate's test (fixtures_matrix asserts `Expected::Hit` brand detection; recovery_matrix pins Refused). rss_cap_mb/wall_cap_ms caps are recorded here but no automated check in this file's consumers enforces them.
Why it matters: the "expectation" alone cannot fail on a wrong-but-plausible outcome; caps are decorative at this layer.
What correct looks like: version the expectation schema with the exact observed flavor once conformance tables landed (the comment itself defers this to cards 2.B2/2.B3).
Confidence: HIGH — Verification: read file + consumers.
