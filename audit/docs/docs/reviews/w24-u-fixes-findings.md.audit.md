# CLAIM AUDIT: docs/reviews/w24-u-fixes-findings.md
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A02; doc authored by W24-R13) |
| Date | 2026-11-24 |
| Claims extracted | 9 |
| REAL 6 / REAL-UNPROVEN 0 / FALSE 0 / UNVERIFIED 3 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A 2026-11-11 fix audit of w24/u (build-msi.ps1 escaping + parse repair + -SelfTest),
verdict PASS merge-ready. Every code-level claim verifies in today's tree: XmlEscape chain
(& first), New-MsiWxs single generation path, -SelfTest switch, and all three LOW
follow-ups (F-U1 raw-text pin, F-U2 README docs, F-U3 gate wiring) have since LANDED —
each in the exact shape the doc recommended. Its pwsh-execution proofs (parse 4→0,
red/green mutants, cross-twin byte diff) are not re-runnable (no pwsh here). No fabrication.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "XmlEscape... full set & < > " ' as &apos;... '&' replaced FIRST" | :51-63 | REAL | scripts/packaging/build-msi.ps1:101-102 `function XmlEscape` + `.Replace('&', '&amp;')` first |
| 2 | "-SelfTest block calls New-MsiWxs — the SAME function the main flow invokes" | :74-76 | REAL | build-msi.ps1:81 `[switch]$SelfTest`; :107-119 "The ONE generation path... `function New-MsiWxs`" |
| 3 | "F-U2: README predates -SelfTest... credits only gen-wxs.sh" | :161-164 | REAL at date; fixed | docs/packaging/README.md:69 "proven by BOTH twins' self-tests (`gen-wxs.sh --self-test` + `build-msi.ps1 -SelfTest`...)"; :153-155 "added in W24-U" |
| 4 | "F-U3: gate.sh wires NEITHER self-test" | :165-170 | REAL at date; fixed | scripts/packaging/gate.sh:30-36 "7. BOTH .wxs-generator self-tests run (audit R13 F-U3)" |
| 5 | "F-U1: gen-wxs.sh --self-test lacks the raw-text escape-set pin" | :156-160 | REAL at date; fixed | scripts/release/gen-wxs.sh:6 "raw-text escape-set pins added in W24-Y, audit R13"; :474 "escape SET pinned on the raw text" |
| 6 | "the pre-fix script did not PARSE AT ALL (4 ParserErrors from bash-style \")" | :8-10,33-41 | REAL-UNPROVEN | current build-msi.ps1 parses (structure consistent, backtick idiom); pre-fix revision needs git (forbidden) |
| 7 | "pwsh 7.7.0 in /tmp reused... red/green mutants reproduced" | :85-98 | UNVERIFIED | no pwsh on PATH here; /tmp/pwsh-test absent in audit sandbox |
| 8 | "cross-twin output diff... differ ONLY in 4 provenance-comment lines" | :128 | UNVERIFIED | requires executing both generators; not re-run |
| 9 | "gate.sh → PASS (51 vars); lint-literals 0 violations" | :132-137 | UNVERIFIED | cargo/gate runs forbidden by load discipline; W17 baseline consistent (lint-literals allowlist exists) |

## 3. Contradictions
None. The wave24-followup-ledger (§1.15) records the same F-U1..3 statuses (fixed on
w24/y, then-merged-later); current tree confirms all three landed.

## 4. Fabrication / inflation indicators
None. The auditor discloses its verification tooling (borrowed pwsh), names what it
executed vs asserted, and the follow-ups it filed were confirmed real by the landed fixes.

## 5. Notes for next agents
- build-msi.ps1 -SelfTest requires pwsh; gate.sh runs it only "when pwsh is on PATH" —
  in this audit sandbox that leg is skipped by design (gate step 7 condition).
- The twins rule (gen-wxs.sh .sh + build-msi.ps1 change-together) is enforced by shared
  fixtures — cite gen-wxs.sh:329 when auditing either twin.
