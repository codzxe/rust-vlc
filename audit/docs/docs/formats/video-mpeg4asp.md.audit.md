# CLAIM AUDIT: docs/formats/video-mpeg4asp.md
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 4 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 3 / 0 / 0 / 0 / 0 / 0 (1 stale-ref) | 0 |
| Internal contradictions found | 1 (dated cross-ref) |

## 1. Blunt verdict
Honest blocked-decision record: no code exists (no ASP module in
crates/codecs/src/video), the candidate dep is recorded as
purity-prechecked but NOT admitted, and the matrix row's status has since
moved from not_started (as the record cites) to blocked-with-reason — the
record's own verdict direction. Its one defect is the dated line-number
cross-reference.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "matches docs/feature-matrix.toml:526, which carries status = not_started ×4 with an empty evidence" | :6-9 | STALE-REF | today matrix:527-537 → id at 527, status blocked ×4, evidence "" — row moved not_started→blocked after the record (5.Q2-V backfill note present) |
| 2 | "crates = [] — none admitted; oxideav-mpeg4video 0.1.8 is the recorded candidate" | :13-15 | REAL | no mpeg4 module in `ls crates/codecs/src/video/`; records/oxideav-mpeg4video-0.1.8.md exists; not in Cargo.toml |
| 3 | "no VA-API driver class exposes ASP … in-box Media Foundation on Windows only" | :32 | REAL (repo-internal) | matrix:537 notes say the same; external-world claim [not re-verifiable — accepted as repo-consistent] |
| 4 | "18 of 143 corpus-v4 lanes (12.6%)" | :35 | REAL-UNPROVEN | arithmetic 18/143 = 12.587% ✓ rounds to 12.6; corpus manifest not re-tabulated |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| matrix row "not_started ×4" (cited line 526) | current row: blocked ×4 | matrix true; record predates the flip — its own verdict ("blocked for v1") is what landed |

## 4. Fabrication / inflation indicators
None — statuses explicitly unchanged by the record ("No code changed").

Findings: F-001 [S4] dated line-number cross-ref (:6-9) now mis-states the
row's status cells.
Commands: rg/sed on feature-matrix.toml, ls codecs tree (exit 0).
