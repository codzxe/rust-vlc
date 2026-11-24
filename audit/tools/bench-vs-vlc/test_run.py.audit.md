# CLAIM AUDIT: tools/bench-vs-vlc/test_run.py (544 lines, source+tests)
| Field | Value |
|---|---|
| Auditor | W19-A08 |
| Lines / functions | 544 / 30 tests |
| File verdict | REAL (executed) |
| Reachable from shipped entry point? | N/A (dev/test tool; executed directly) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. One-paragraph verdict
A genuine, honest test suite: 30 tests, all passing (executed: "Ran 30 tests in 0.697s —
OK"). The tests are mutation-resistant by design — single-byte divergence is found
(test_single_byte_difference_is_found), truncation offset reported, drift aborts both via
exception and exit-3, absent oracle exits 2, mock labels mechanically asserted on every
digest row, pending mode asserts zero oracle digests. Test names match their bodies.

## 2. What actually works (verified)
— `python3 -m unittest discover -s tools/bench-vs-vlc -p 'test_run.py'` -> Ran 30 tests, OK (exit 0).
— `rg -n 'def test_'` -> 30 named tests matching the README's claimed coverage list exactly.

## 3. Findings
None at S0-S4. Honest [VERIFY] boundary in README (real-oracle confirmation) is the only
open item and it is documented, not hidden.

## 4. Per-function table
| Test | Lines | Verdict | Notes |
|---|---|---|---|
| test_comparable_families / non_comparable_* (8) | 59-125 | REAL | classification coverage incl. undumpable PCM, BE-wire, lossy, corrupt exclusion |
| test_identical/single_byte/prefix/empty/md5 hashlib (5) | 137-178 | REAL | comparison logic proven on synthetic PCM — would fail if digest logic broke |
| test_version_pin/drift/absent (5) | 182-261, 515 | REAL | both drift paths (exception + exit 3) and exit-2 absence |
| test_dump_* (3) | 200-250 | REAL | determinism, echo hook, documented flag shape |
| test_pending/mock/lumen_refused/manifest/files (6) | 309-506 | REAL | report-writer invariants, end-to-end through the subprocess seam |

## 5. Test honesty
Would these fail if the code were broken? Yes — divergence, drift, and label invariants are
all asserted positively and negatively (mock mismatch path labeled too, L359).

## 6-9. Doc claims / dead code / deps / open questions
Covered in README audit (same dir). None.
