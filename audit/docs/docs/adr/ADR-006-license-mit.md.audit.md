# CLAIM AUDIT: docs/adr/ADR-006-license-mit.md
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A07) |
| Date | 2026-11-24 |
| Claims extracted | 8 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 7 / 0 / 0 / 0 / 0 / 0 / 1 |

## 1. Blunt verdict
Clean decision record; every cross-reference it makes resolves in the tree
(LICENSE text, R-05 closure row, D-009, ADR-010 pending status, the §8.2
brief it supersedes). No contradictions found. The move note is accurate
(docs/decisions/ no longer exists).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "licensed under the **MIT License** (see `LICENSE`)" | :20 | REAL | LICENSE:1-3 "MIT License / Copyright (c) 2026 Lumen Media Player Contributors" byte-exact |
| 2 | "Copyright line: 'Lumen Media Player Contributors' (working name per D-006)" | :21 | REAL | LICENSE:2 matches verbatim; product.toml:2 "working name" |
| 3 | "Supersedes ... risk R-05 (`docs/14-risk-register.md`)" + "closes risk R-05" | :5,57 | REAL | 14-risk-register.md:96 R-05 row -> "closed (2026-10-19: **MIT** via ADR-006 ...) / closed — no further review" |
| 4 | "Related: D-009 (`docs/ROADMAP.md` §3)" | :6 | REAL | ROADMAP.md:106 "D-009 | **Repository license = MIT** (user decision 2026-10-19; LICENSE; docs/adr/ADR-006 ... closes risk R-05)" |
| 5 | "ADR-010 (patent matrix, pending)" | :6 | REAL | docs/adr/ADR-010-codec-patent-shipping.md exists; adr/README.md:20 status "**proposed** ... Decision due before any binary-distribution plan" |
| 6 | "open question in `docs/11-...md` §8.2" | :5 | REAL | doc 11 line 569: "### 8.2 ADR-006 — this repository's license (the brief, not the decision)" |
| 7 | "moved from `docs/decisions/` to `docs/adr/`" (card 3.DOC-1) | :60-62 | REAL | `ls docs/decisions/` -> ENOENT (old path gone); all 13 ADRs in docs/adr/ |
| 8 | "user's decision delivered 2026-10-19 ('mit licence')" | :56 | UNVERIFIED | the user utterance itself is not in any tree artifact I can cite; downstream registers all agree on the date |

## 3. Contradictions
None within this file's claims. (Adjacent observation, not this doc's claim:
docs/adr/README.md:27 says "ADR-007, ADR-008, ADR-009 do not exist" while
ROADMAP D-008 references an "ADR-007 identity" decision — a register-vs-ROADMAP
tension outside ADR-006's scope.)

## 4. Fabrication / inflation indicators
None — the decision is modestly scoped and its trade-off (no patent grant)
is stated, not hidden. Findings: none.
