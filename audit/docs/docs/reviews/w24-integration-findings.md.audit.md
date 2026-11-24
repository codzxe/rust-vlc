# CLAIM AUDIT: docs/reviews/w24-integration-findings.md
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 8 (finding-level, current-state re-checked) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 5 / 0 / 0 / 0 / 0 / 0 / 3 |
| Internal contradictions found | 1 temporal (its own live-tip notes now stale) |

## 1. Blunt verdict
A wave-24 integration review whose sharpest finding (IN-1: committed merge
markers in ui/src/App.tsx breaking the whole UI build) is corroborated by
the gate report on disk and was genuinely resolved afterward — today's
App.tsx has zero markers. Its "live tip" notes are themselves partly stale
now: IN-2 said "step 10 does not exist anywhere / still 9 → 11" — today
scripts/full_gate.sh HAS step "10. web-budget --check" (issue #49), so
that finding closed after the report. IN-3/IN-4 remain partially open in
today's tree (7.w24b/7.w24d worklog entries absent; the dangling
`docs/soak/regression-guards.md` citation it flags is still unresolved).
Git-lineage claims (SHAs, ancestor checks) are UNVERIFIABLE for me (git
forbidden).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (current tree) |
|---|---|---|---|---|
| 1 | IN-1: committed conflict markers in `ui/src/App.tsx` (OF-1); UI typecheck/lint FAIL (TS1185/TS1005) | :26-46 | REAL (then) / resolved now | Gate report on disk: docs/gates/phase7-exit.md:59 `16 \| UI npm run typecheck / lint \| **FAIL** (TS1185/TS1005 parse errors; eslint 1 error) — OF-1`; today `rg -c '<<<<<<<\|>>>>>>>' ui/src/App.tsx` -> NONE (resolved as its live-tip note predicted). |
| 2 | IN-1 live-tip: "`App.tsx` at the tip has 0 markers" | :41-42 | REAL | Verified today: zero conflict markers in the whole worktree (only worklog prose + scripts' regexes mention them). |
| 3 | IN-2: "`scripts/full_gate.sh` … steps are 1–9 then 11 … `rg "web-budget"`: 0 hits … Live tip: unchanged — still 9 → 11" | :48-65 | REAL-then / RESOLVED now | Today `sed -n '70,76p' scripts/full_gate.sh` -> `=== 10. web-budget --check (issue #49 / doc 09 §2.4 … card 7.K1) ===` — step 10 landed post-report; the report's "unchanged" live-tip is now stale. |
| 4 | IN-3: lanes B/D/G lack worklog entries; live tip: `7.w24g` present, W24-B and W24-D "still missing" | :67-87 | REAL (still partially open) | `rg 'Task ID: 7\.w24' worklog.md` -> only `:878 Task ID: 7.w24g`; no 7.w24b / 7.w24d entries today — matches its open-items state. |
| 5 | IN-4: "`tests/soak/scenarios.schema.json:5` … cites `docs/soak/regression-guards.md`, which does not exist" | :88-99 | REAL (still open) | `ls docs/soak/regression-guards.md` -> ABSENT today; dangling reference persists. |
| 6 | Method: "path-reference resolution across 76 wave-24 docs (238 unique referenced paths)" | :18-20 | UNVERIFIED | Methodology claim; no committed artifact of the sweep itself. |
| 7 | Severity summary: 1 P0 + 3 P1 + 3 P2 + 3 P3 = 10 findings | :24-31 | REAL | The file's sections enumerate IN-1..IN-10 with matching severities (33 IN- mentions, 10 distinct IDs). |
| 8 | Snapshot lineage: "`286bacd` … 17 merges … diff `origin/main...286bacd` (126 files, +37,333/−228)" | :6-9 | UNVERIFIED | git forbidden to auditors (AGENT-SPEC §2) — cannot re-derive; internally consistent with other w24 review docs. |

## 3. Contradictions
| This doc says | Today's tree says | Truth |
|---|---|---|
| IN-2 live-tip "still 9 → 11" | step 10 (web-budget) exists in full_gate.sh | Tree moved after the report — the report correctly described its snapshot; its live-tip froze. |

## 4. Fabrication / inflation indicators
None — the report re-verified its own findings against a moving tip and
marked the P0 as resolved-in-flight. The residual value today is IN-3/IN-4
open items, which I confirm are STILL open (missing worklog entries +
dangling regression-guards.md path).
