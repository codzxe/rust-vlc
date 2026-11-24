# CLAIM AUDIT: docs/audits/error-surface-fixes-20261024.md
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 5 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 5 / 0 / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Every one of the five fix claims is present in the current tree exactly as
described: the production expect is gone (WiredKind/UnwiredKind in place),
both contract modules are feature-gated, the queue limits are consts, the
recycle-path regression test exists, and the check-panics gate with its
3-entry CITED_INVARIANTS table exists. This is a fix ledger that checks out.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | F-1: "production expect retired via WiredKind + total QueuePlan::for_kind" | table row 1 | REAL | crates/playback-engine/src/pipeline.rs:112,374-379 WiredKind; `rg 'expect\(' pipeline.rs` → 0 hits |
| 2 | F-2: contract modules `#[cfg(any(test, feature = "contract"))]` | table row 2 | REAL | crates/input/src/contract.rs:5, crates/media-probe/src/contract.rs:5 verbatim |
| 3 | F-3: packet-queue cap() literals are `const` items, runtime abort arm deleted | table row 3 | REAL | crates/packet-queue/src/limits.rs:40-47 NonZeroUsize consts |
| 4 | F-4: regression `parked_codec_rides_the_recycle_path` added | table row 4 | REAL | rg → crates/playback-engine/src/pipeline.rs hit |
| 5 | F-5: `check-panics` gate + CITED_INVARIANTS (3 residuals) | table row 5 | REAL | tools/xtask/src/cmd/check_panics.rs:107 `CITED_INVARIANTS: [(&str,&str); 3]` exactly 3 |

## 3. Contradictions
None. Doc's self-report of the predecessor gate bug (cited-invariant never
matching) is corroborated by the raw-line matching design in check_panics.rs.

## 4. Fabrication / inflation indicators
None — includes a seeded-violation FAIL transcript (proof the gate bites)
and discloses its own handoff bug honestly.

Findings: F-001 [S4] the cited `git show` commit hashes and the four-crate
test runs are branch-state claims — not re-verifiable without git; the
code endstates all verify.
Commands: rg/sed across the five crates (exit 0 each).
