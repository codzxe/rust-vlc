# CLAIM AUDIT: docs/reviews/wave24-followup-ledger.md
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A02; doc authored by W24-AC) |
| Date | 2026-11-24 |
| Claims extracted | 14 |
| REAL 9 / REAL-UNPROVEN 1 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 4 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A dated (2026-11-11) read-only synthesis of wave 24. Sampled claims verify against today's
tree; its arithmetic is self-consistent (2+12+24+53+10=101 findings; 71+7+5+15+3=101
dispositions). Its git-history claims (branch tips, SHAs, merge states) are UNVERIFIABLE here
(no git allowed). Three sampled OPEN rows have since been closed in-tree — the doc is a stale
snapshot, not a false one. No fabrication found.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "101 findings" total | :266 | REAL | python3 recount 2+12+24+53+10=101; 71+7+5+15+3=101 |
| 2 | ".gitignore target-xtask/ duplicated" IN-9 F+V | :166 | REAL | `rg -n "target-xtask" .gitignore` -> single entry (line 5) + note (line 112) |
| 3 | "four review side files under docs/worklog/" IN-10 OPEN | :167 | REAL | ls docs/worklog/ -> w24-{r1,r3,r4,r6}-review.md all present |
| 4 | "full_gate.sh step-10 hole... stale comment (9->11)" IN-2 OPEN | :159 | FALSE (today) | `sed -n '70p' scripts/full_gate.sh` -> "=== 10. web-budget --check ... ===" exists; snapshot-aging, was true 2026-11-11 |
| 5 | honesty note "false non-runnability claim" R11 P1 OPEN | :189,291 | FALSE (today) | ui/scripts/web-matrix-run.mjs:168-177 now: "the matrix re-runs whenever that executable exists" — corrected in tree |
| 6 | '"735 lines" literal drifted' R11 P3-3 OPEN | :193 | FALSE (today) | web-matrix-run.mjs:152-155 comment "R11 P3-3 — a stale \"735 lines\" was filed" + derived count now |
| 7 | OF-5 "MF claim was false... retracted" F+V | :58 | REAL | docs/feature-matrix.toml:416 "Media Foundation (Windows) NOT IMPLEMENTED... the Media Foundation claim was false" |
| 8 | "every run dies at 0 steps" CI billing blocker | :336 | REAL-UNPROVEN | KNOWN-LIMITS.md:86 "CI billing blocked... every lane re-verified locally"; matches W17 gates-theatre baseline |
| 9 | "R15... the phase7-exit §8 re-run battery remains unexecuted" | :32 | UNVERIFIED | phase7-exit.md exists (325 lines); battery execution state needs git history |
| 10 | "w24/x... pushed, unmerged" merge states | :219,238,247 | UNVERIFIED | git forbidden by rulebook; branch/merge states not re-checkable |
| 11 | "71 FIXED + VERIFIED" | :282 | UNVERIFIED | each row cites a verifying report; not re-runnable here; sampled rows (2,3,7) hold |
| 12 | ADR-010 "proposed — 24-item counsel ledger" | :340 | REAL | docs/adr/ADR-010-codec-patent-shipping.md exists on disk |
| 13 | "This ledger is a read-only synthesis: no battery was re-run" | :349 | REAL | doc method statement; consistent with sampled content |
| 14 | "P1 (12): 9 F+V... 1 F(pend)... 2 OPEN" | :259 | REAL | python3 9+1+2=12; P2 items recount 4+3+1+3+2+1+1+2+3+1+1+1+1=24 |

## 3. Contradictions
| This doc says | Other source says | Which is true |
|---|---|---|
| IN-2 step 10 "never landed" (2026-11-11) | scripts/full_gate.sh:70 has step 10 web-budget | Both, at their dates; today the doc is stale (S3 snapshot-aging) |
| R11 P1 honesty note OPEN | web-matrix-run.mjs:168-177 corrected prose | Fixed since; doc stale |

## 4. Fabrication / inflation indicators
None found in sampling: honest "Not filed / not invented" section (:31-35), F(pend) bucket
disclosed (:283), explicit method-limits section (:344-350). Statuses drift in the CONSERVATIVE
direction (OPEN items later closed).

## 5. Notes for next agents
- Two side files from IN-10 are in this batch: docs/worklog/w24-r1-review.md, w24-r6-review.md.
- Cross-wave: OF-5/IN-5 web-budget gate now exists at full_gate.sh:70 but baseline says
  web-budget section RED (GATE RESULT FAIL) — the landed gate still fails.
