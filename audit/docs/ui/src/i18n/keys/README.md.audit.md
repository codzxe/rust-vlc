# CLAIM AUDIT: ui/src/i18n/keys/README.md
| Field | Value |
|---|---|
| Auditor | W15-R2 (credited: A04) |
| Date | 2026-11-23 |
| Claims extracted | 5 |
| REAL: 3 | REAL-UNPROVEN: 1 | SHALLOW: 0 | STUB: 0 | PAPER-ONLY: 0 | FALSE: 0 | UNREACHABLE: 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
This 12-line README describes the lane-catalog pattern and every mechanically
checkable claim is TRUE at 100%: I extracted all key/value pairs from all 53
`w26.*.ts` lane files and compared them against the central catalog — 815/815
lane keys registered in messages.ts, 0 missing, 0 value drift, 0 duplicate keys
across lanes, all present in the MessageKey union. The one process claim
("Agents never edit messages.ts directly") is UNVERIFIABLE without git history
but is not contradicted by anything in the tree. This README is honest
documentation of a discipline the tree actually follows.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "each lane adds genuinely NEW message keys in its own file here (`<task-id>.ts`)" | README.md:3-4 | REAL | All 54 files in dir match `w26.*.ts` (ls); zero cross-lane duplicate keys (python3 extraction: no LANE-DUP/LANE-CONFLICT output) |
| 2 | "exporting `export const keys = { ... } as const`" | README.md:4-8 | REAL | `grep -c 'export const keys' w26.*.ts` -> every file exactly 1; grep -L -> no file missing the pattern |
| 3 | "The Lead registers these into ui/src/i18n/messages.ts (MessageKey union + catalog)" | README.md:10-11 | REAL | python3 lane-vs-central diff: 815/815 ok, 0 missing, 0 drift; 0 keys missing from union |
| 4 | "Agents never edit messages.ts directly" | README.md:11-12 | UNVERIFIED | Process claim; needs git authorship history (forbidden this run). Nothing in tree contradicts it |
| 5 | "Reuse existing keys whenever the copy already exists — grep first" | README.md:12 | REAL | Cross-lane duplicate values would show LANE-DUP in the extraction; none printed; lane headers (w26.f05, w26.c1, w26.a18) document reuse explicitly |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| — (none found) | — | — |

## 4. Fabrication / inflation indicators
None. The README undersells if anything: it does not mention that registration
is ALREADY complete and drift-free for every lane (815/815), which is stronger
than the "after each wave merge" future tense implies.

## 5. Matrix rows
Not a matrix file — no rows apply.

### Verification commands
python3 extraction script (run this audit): for each of 53 lane files, regex
`"key": "value"` pairs; compared against `messages.ts` catalog dict and
MessageKey union. Output tail: `TOTAL lane keys=815 ok=815 missing=0 drift=0`;
`keys missing from MessageKey union: none`.
