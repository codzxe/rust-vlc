# CLAIM AUDIT: docs/13-agent-wave-program.md (chunks #part1-#part2, L1-972)
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 10 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 9 / 0 / 0 / 0 / 0 / 0 / 1 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
This planning doc's every quantitative claim reproduces exactly: 200 task-card
rows, 77 waves (74 headers, 3 of which cover two waves each), all nine
per-phase card counts (17/20/17/19/25/45/17/24/16), the [DECISION-PENDING]
2.B2/2.B3 pair, the [CONDITIONAL] 5.F2/5.G2/5.N1–5.N3 cards, the rework
arithmetic (77 × 1.25–1.5 = 96.25–115.5), and both cross-references it
extends (AGENTS.md §2 :18, §5 :91; MASTER-PROMPT §15 :296, §16 :323, §17 :351).
The one honest soft spot: it is a planning artifact — the wave cards are
intent, and whether the program's later phases (5–8) were actually executed
that way is not this doc's claim and cannot be settled here.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "**200 task cards in 77 waves**" | :6 | REAL | `awk '/^\| [0-9]\.[A-Z][0-9]/{n++} END{print n}'` -> 200; wave headers: `rg -o '^### Waves? ...'` -> 74 headers of which `5.A–5.B`, `5.C–5.D`, `5.F–5.G` each cover 2 waves -> 77. |
| 2 | Totals table per-phase card counts (17,20,17,19,25,45,17,24,16 = 200) | :857-866 | REAL | Per-wave `cut -c1-3 \| sort \| uniq -c` sums: 0=17, 1=20, 2=17, 3=19, 4=25, 5=45, 6=17, 7=24, 8=16; total 200. |
| 3 | "rework factor of 1.25–1.5× ... yields ~96–115 effective wave-units" | :870-871 | REAL | 77×1.25=96.25; 77×1.5=115.5 — "~96–115" is arithmetically faithful (floor/round). |
| 4 | "[DECISION-PENDING] pair 2.B2/2.B3 ... exactly one executes; Phase 2 runs 16 cards, program ~199" | :328-329, :876 | REAL | `rg -n 'DECISION-PENDING'` -> :328 `2.B2 ... [DECISION-PENDING — variant A] AVI demuxer`, :329 `2.B3 ... [DECISION-PENDING — variant B] MP4 demuxer`; math 17-1=16, 200-1=199 consistent. |
| 5 | "[CONDITIONAL] cards (5.F2, 5.G2, 5.N1–5.N3) execute only when their decision/ADR lands" | :534, :536, :590-592, :878 | REAL | `rg -n 'CONDITIONAL'` -> 5.F2 `[CONDITIONAL on ADR-001]` (:534), 5.G2 `[CONDITIONAL: executes only if the H.264 hw path landed]` (:536), 5.N1/5.N2/5.N3 `[CONDITIONAL]` (:590-592). |
| 6 | "extends AGENTS.md §5 (task-ID grammar) and §2 (assignment contract)" | :8-9 | REAL | AGENTS.md:18 `## 2. Assignment contract`; AGENTS.md:91 `## 5. Task ID scheme`. |
| 7 | "Authority: docs/MASTER-PROMPT.md §15 ... and §16 ... §17 defines done" | :4-5 | REAL | docs/MASTER-PROMPT.md:296 `## 15. Multi-agent protocol`, :323 `## 16. Roadmap and gates`, :351 `## 17. Reporting and definition of done`. |
| 8 | "planning-wave agents writing whole 6,000-word docs hit context limits (worklog 1.A2 timed out ...)" | :22-23 | REAL | docs/worklog/archive-2026-10.md:107-108: `Task ID: 1.A2` / `Agent: general-purpose (timed out before reporting; Lead verified output and appended this entry on its behalf)`. |
| 9 | "Status: planning artifact (Wave C, Task 1.C13)" | :3 | REAL | Doc contains only wave tables + binding prose; no execution/final-result claims of its own. |
| 10 | "2–6 agents may run in parallel" within a wave | :42, :866 | UNVERIFIED as executed practice | Rule text is coherent and consistent with totals table ("2–6 bound"); actual historical concurrency per wave would need worklog mining across all waves — out of scope here. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| none found | w26 PROGRAM.md and worklogs reference later wave numbering (w24+, w26) | Not a contradiction: doc 13 stops at phase 8 planning; later wave-program files (docs/waves/w26/PROGRAM.md) extend it. |

## 4. Fabrication / inflation indicators
None detected. Counts are modest and all reproduce. The doc even
self-corrects ("~199 executes", "no L card is launchable") — the opposite of
inflation. One caveat for downstream readers: R2's "[LEAD]-only" surface list
(:50-52) is a rule, not evidence of enforcement; enforcement lives in CI and
is not claimable here.

## 5. Matrix rows
None — not a matrix chunk. (Doc 13 is referenced by docs/waves/w26/PROGRAM.md
as the program origin, audited separately.)
