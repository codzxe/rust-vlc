# AUDIT: tests/integration/first-vertical/src/digest.rs
| Field | Value |
|---|---|
| Auditor | W16-A03 |
| Date | 2026-11-24 |
| Lines / functions | 176 / 4 items (K, H0, compress, sha256, sha256_hex) + 3 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES from the test lane: `cargo test -p first-vertical` (lib); consumed by harness.rs (crate::digest::sha256_hex) for transcript PNG digests |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read; python3 hashlib cross-check of every pinned vector; rg baseline |

## 1. One-paragraph verdict
A correct, self-contained std-only SHA-256. I cross-verified every pinned
test vector against python3 hashlib — all match: the three NIST vectors AND
the 55/56/64-byte padding-boundary vectors (9f4390f8…, b35439a4…,
ffe054fe…). The committed baseline ran its tests green
(audit/runs/baseline/cargo-test-workspace.txt:2729-2732 `test
digest::tests::nist_vectors ... ok`, `padding_boundaries ... ok`,
`digests_separate_inputs ... ok`). The single `unreachable!` (:97) is
provably dead by the padding construction and carries an in-file proof
note. The implementation is also indirectly cross-validated in production:
the transcript artifacts it digested (pause.json final_png sha256) match my
independent hashlib run on the same bytes.

## 2. What actually works (verified)
— python3 hashlib: sha256('a'*55/56/64) == the three pinned boundary
digests; NIST empty/abc/long-abc == pinned values (command output above).
— Baseline: :2729-2732 all three digest tests ok.
— Consumer: harness.rs:43 `use crate::digest::sha256_hex` → png_json (:705)
writes the digest column of the evidence ledger.

## 3. Findings
F-001 [S4]
Where: :79-81
Code: `let bit_len = u64::try_from(data.len()).unwrap_or(u64::MAX).wrapping_mul(8);`
What is wrong: for an input ≥ 2^61 bytes the length field saturates
(silently wrong digest); practically unreachable (2 EiB input), and the
`saturating` behavior is undocumented at the site.
Confidence: HIGH — Verification: read.
(Praise-note: :142-146 honestly records that the first draft's from-memory
boundary values were WRONG and the vectors caught them — the test earned
its keep.)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| K / H0 consts | 12-26 | REAL | — | FIPS 180-4 §4.2.2/§5.3.3 values |
| compress | 29-73 | REAL | — | schedule + compression; wrapping adds; rotate_right(7/18/3, 17/19/10, 6/11/25, 2/13/22) all correct |
| sha256 | 77-107 | REAL | S4 | padding always-one-block; unreachable! at :97 is provably dead with documented proof |
| sha256_hex | 111-119 | REAL | — | 64-hex lowercase; `let _ = write!` on String (infallible) |
| tests::nist_vectors | 127-140 | REAL | — | 3 NIST vectors, all hashlib-verified |
| tests::padding_boundaries | 148-167 | REAL | — | 55/56/64 boundaries, all hashlib-verified |
| tests::digests_separate_inputs | 171-174 | REAL | — | non-vacuity |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| nist_vectors | YES | exact digest equality on 3 known answers (and they are CORRECT — I re-derived them with hashlib) |
| padding_boundaries | YES | catches off-by-one padding at both boundaries; historically caught a real from-memory error (comment :142-146) |
| digests_separate_inputs | YES | non-vacuity guard |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the FIPS 180-4 test vectors below pin the implementation … nothing here is 'observed and blessed'" | :8-9 | REAL | vectors verified against hashlib + baseline green |

## 7. Dead / duplicated / unreachable code
The :97 `unreachable!` is the only unreachable path — justified and
documented (see F-001 note). No duplication (the tree's other SHA-256s:
xtask fixtures.rs + testkit each have their own — cross-referenced at
baseline :6649/:6933 `sha256_matches_nist_vectors`, consistent pattern).

## 8. Dependencies & flags
std-only, no deps (C5 rationale documented :4-7). No features.

## 9. Open questions for the next agent
None.
