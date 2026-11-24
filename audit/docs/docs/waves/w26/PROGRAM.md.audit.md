# CLAIM AUDIT: docs/waves/w26/PROGRAM.md (credit: W19-A06, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Claims extracted | 8 |
| REAL 5 / REAL-UNPROVEN 0 / SHALLOW 1 / STUB 0 / PAPER-ONLY 1 / FALSE 0 / UNREACHABLE 0 / UNVERIFIED 2 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The program plan is structurally real and roughly half-executed with committed evidence:
the "20 agents per wave" claim is fully evidenced for exactly six waves (e, g, h, i, j, k —
20 artifacts each) and partially evidenced for a(14 logs), b(8), c(6), d(2), f(6), l(4);
wave m has zero artifacts. PROGRAM.md is a plan, not a completion report — but anyone
reading "260 agents" should know the repo carries evidence for ~130/260 lane outputs.

## 2. Claim ledger
| # | Claim (verbatim) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | 13 waves x 20 agents (260) | :1,:7-21,:23 | REAL structure / SHALLOW execution | table has exactly 13 wave rows a-m; 13x20=260 arithmetic correct; artifact census: logs a=14 b=8 c=6 d=2 f=6 i=20 k=20; findings e=20 g=20 l=4; ratings h=20; reviews j=20 -> ~130/260 evidenced |
| 2 | Task IDs `w26.<wave><nn>` (letters a-m) | :4 | REAL | ls docs/waves/w26/logs/ -> w26.a01..w26.k20 naming exactly matches |
| 3 | Every agent reads docs/waves/w26/CONTRACT.md first | :3-4 | REAL | CONTRACT.md:1-3 "binding for every w26 agent ... You received a Task ID like w26.a07" |
| 4 | After each wave the Lead builds, tests, integrates, commits and pushes to `main` | :4-5 | UNVERIFIED | requires git history; agents never run git (rulebook §2); no per-wave integration log found in logs/ census |
| 5 | verification waves file findings in docs/waves/w26/findings/ | :13 | REAL | findings/ holds 44 files: 20 `w26.e*`, 20 `w26.g*`, 4 `w26.l*` (ls + rg census) |
| 6 | 20 raters -> ratings/; 20 reviewers -> reviews/ | :16,18 | REAL | ratings/: 20 `w26.h<nn>` + h-scoreboard; reviews/: 20 `w26.j<nn>` + j-scoreboard |
| 7 | loop e->f->g repeats (g restarts f) until verifiers return clean | :23-24 | REAL | both e (20) and g (20) findings sets exist; wave-l findings w26.l05.md:65 shows residual P1/P2 red items — loop termination "clean" not yet reached |
| 8 | Lead re-seeds visual baselines only after wave d and wave l (W25 spec §4) | :24-25 | PAPER-ONLY / UNVERIFIED | docs/waves/w26/baselines/ holds only MATRIX-20261112.md (dated 2026-11-12 = mid-e-wave); no wave-l re-seed artifact exists (wave l is 4/20 done) |

## 3. Contradictions
None internal. Cross-file: none — CONTRACT.md and PROGRAM.md agree on lanes/IDs.

## 4. Fabrication / inflation indicators
"260 agents" is a plan total, not a results claim — but the file never states completion
status, leaving a reader to assume more execution than the ~130 evidenced artifacts.
Quotes byte-verbatim: `sed -n '1,5p;23,25p' docs/waves/w26/PROGRAM.md` (output above).
