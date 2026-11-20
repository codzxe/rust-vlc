# CLAIM AUDIT: docs/ROADMAP.md

| Field | Value |
|---|---|
| Auditor | W19-A01 |
| Date | 2026-11-16 |
| Claims extracted | 23 | REAL 12 | REAL-UNPROVEN 2 | SHALLOW 0 | STUB 0 | PAPER-ONLY 2 | FALSE 4 | UNVERIFIED 3 |
| Internal contradictions found | 2 |

## 1. Blunt verdict

The roadmap's phase-gate architecture is genuinely backed: every gate document it cites exists, the Phase 3/4 Lead sign-offs it claims are in those files at the claimed dates, Phase 7's "NOT PASSED → closed at wave-24 merge" story reconciles with phase7-exit.md and real commits, and the 200-cards/77-waves arithmetic checks. Its failures are concentrated and specific: one cited commit (2d06607, D-016) does not exist in this repository's history; the D-017 "61 commands / 23 events, pinned by catalog_shape.rs" decision text is contradicted by the live catalog (65) and by the two red workspace tests; the design-asset count (102 SVGs) is wrong against the tree; and the Phase 8 status ("complete in conduct") sits on top of a gate table whose own exit criteria (perf vs VLC on reference hardware, signed installers) the repo demonstrably does not meet. Plus the standard staleness: header last refreshed 2026-11-13, one-plus waves behind a repo at w31.6.

## 2. Claim ledger

| # | Claim (verbatim ≤25 words) | Location (path:line) | Verdict | Evidence you gathered |
|---|---|---|---|---|
| 1 | "Phase 0 — COMPLETE (`docs/gates/phase0-exit-report.md`)" | docs/ROADMAP.md:10 | REAL | File exists; its row 3 records the 252/252 default-agreement cross-check I verified against options-linux.json (totals 1476). |
| 2 | "Phase 1 — COMPLETE — PASS 2026-10-20, local-evidence; 3-OS CI green-run links PENDING first push (`docs/gates/phase1-exit.md`)" | docs/ROADMAP.md:11-12 | REAL | phase1-exit.md:6-8: "honestly PENDING first push … actionlint 1.7.12 clean" — exactly as cited. |
| 3 | "Phase 2 — COMPLETE — PASS 2026-10-21, local-evidence (`docs/gates/phase2-exit.md`)" | docs/ROADMAP.md:13-14 | REAL | File exists; D-018 first-vertical record consistent with phase2-exit content cited elsewhere. |
| 4 | "Phase 3 … the PENDING LEAD SIGNATURE line was squared up with a dated Lead sign-off on 2026-11-13 (issue #59)" | docs/ROADMAP.md:15-19 | REAL | phase3-exit.md:254-256: "## Lead sign-off (the owed square-up, 2026-11-13) … Verdict: PASS (Lead-signed 2026-11-13)". |
| 5 | "Phase 4 — COMPLETE — PASS … signed by the Lead on 2026-11-13 (issue #59)" | docs/ROADMAP.md:20-23 | REAL-UNPROVEN | phase4-exit.md:255-257 carries the same "Lead-signed 2026-11-13" square-up block; the underlying phase-4 evidence (screenshot diffs, 13 settings areas) not re-run in this audit. |
| 6 | "Phase 5 — COMPLETE — PASS WITH REGISTERED CAVEATS, Lead sign-off 2026-10-24 …; 47/47 parent cards resolved" | docs/ROADMAP.md:24-26 | REAL | phase5-exit.md:3-5: "Status: PASS WITH REGISTERED CAVEATS — Lead sign-off 2026-10-24 … 47 parent cards, all resolved"; caveat register §4 present. |
| 7 | "Phase 6 … 11 orchestrated waves landed the streaming program today (27 of 34 tracker issues resolved…)" | docs/ROADMAP.md:27-30 | UNVERIFIED | Issue tracker is GitHub-side; in-repo: merge commits with issue refs exist, and phase6-exit.md's issue ledger lists #6–#15 with one honestly marked "not in main". The 27/34 tally is not derivable from the tree. |
| 8 | "The exit-gate ledger is drafted at `docs/gates/phase6-exit.md` — verdict PENDING LEAD DECISION … Open tails: #38, #39" | docs/ROADMAP.md:36-40 | REAL | phase6-exit.md:3 "Status: DRAFT — PENDING LEAD DECISION"; #8/#15 tails recorded there. |
| 9 | "Phase 7 — COMPLETE … the gate ran NOT PASSED at `c339ee8` with 5 bounded findings (OF-1..OF-5) … all five closed at the wave-24 integration merge `903d618c`" | docs/ROADMAP.md:41-47 | REAL | phase7-exit.md:10 "NOT PASSED at `c339ee8` — 5 open findings (OF-1…OF-5)"; commits c339ee8b and 903d618c ("merge wave 24: … phase-7 gate (#51) …") both exist in git. |
| 10 | "the per-card Phase-7 substance … is on main and green in the hosted CI set" | docs/ROADMAP.md:47-50 | UNVERIFIED | Substantively on main (merge commits verified); "green in the hosted CI set" is GitHub-side and unverifiable from the tree. |
| 11 | "Phase 8 — … complete in conduct; exit gate 8.H1-V in the Lead's final pass (issue #59) … first real 3-OS release dry-runs executed 2026-11-13 — macOS lanes green …" | docs/ROADMAP.md:51-59 | UNVERIFIED | Dry-run claims are GitHub-side. "Complete in conduct" is unsupported against the file's own §1 gate table (see contradictions). |
| 12 | "The program-exit record lands at `docs/gates/phase8-exit.md` when the release lane is green and the first tagged release ships (#66)" | docs/ROADMAP.md:60-62 | FALSE (stale) | Tags v0.1.0–v0.1.3 exist in local git (shipped per RELEASE.md/README/worklog w30.10), yet phase8-exit.md does not exist (docs/gates/ has no phase8 file) — the header was refreshed 2026-11-13, before the releases, and predates the state it describes. |
| 13 | "Header last refreshed: 2026-11-13 (wave-26 orchestration …)" | docs/ROADMAP.md:64-67 | REAL | The date is self-consistent — and demonstrably stale: worklog.md:2146-2149 records that the 12-agent audit wave maintained a "doc-staleness register S-1..S-6" against this roadmap, and w31.x landed after. |
| 14 | "**200 task cards in 77 waves** (per-phase: 0:17, 1:20, 2:17, 3:19, 4:25, 5:45, 6:17, 7:24, 8:16)" | docs/ROADMAP.md:89 | REAL | doc 13:6 "200 task cards in 77 waves" + :866 "Total 77 waves 200"; per-phase sum 17+20+17+19+25+45+17+24+16 = 200 ✓. |
| 15 | D-006: "trademark collision search pending [VERIFY …]" | docs/ROADMAP.md:103 | REAL-UNPROVEN | Open [VERIFY], honestly marked; no trademark search artifact exists in-tree. |
| 16 | D-016: "… commit `2d06607`" (Lead batch-flip) | docs/ROADMAP.md:113 | FALSE | `git log -1 2d06607` → exit 128 "unknown revision or path not in the working tree". The cited commit does not exist in this repository's history. |
| 17 | D-017: "The command catalog count is 61 commands / 23 events — doc 05's … tables are the authority …; `catalog_shape.rs` pins the exact counts" | docs/ROADMAP.md:114 | FALSE | The live catalog lists 65 commands — the two failing workspace tests are exactly "catalog lists 65 commands vs test pinning 64" (executed cargo test, 2 failed); catalog_shape.rs itself pins "61 command entries (+ the issue #32 bookmark trio = 64) / 26 event entries". Three numbers, none matching reality; the "authority" doc drifted and the tests prove it. |
| 18 | ADR index: ADR-001/002/003 accepted; ADR-007/008/009 PROPOSAL, "no ADR-007 file exists in `docs/adr/` yet"; ADR-010 proposed; ADR-011 executed; ADR-012 accepted | docs/ROADMAP.md:152-181 | REAL | docs/adr/ listing: ADR-001…006, 010, 011, 012, 013 files present; no ADR-007/008/009 files — exactly as stated; ADR-010:3 "Status: proposed"; ADR-012 accepted (file present). |
| 19 | "design/ (102 pinned SVG exports + inventory)" | docs/ROADMAP.md:203 | FALSE | Actual tree: 101 SVG files in design/figma/ (105 under design/ overall). Neither 101 nor 105 equals 102. |
| 20 | "worklog.md — live log at the **repository root** (append-only); prior entries archived under `docs/worklog/`" | docs/ROADMAP.md:204 | REAL | worklog.md at root; docs/worklog/archive-2026-10.md exists. |
| 21 | "§8 … [HISTORICAL SNAPSHOT — do not update in place.] … Its 'Phase 3 IN PROGRESS' state is superseded…" | docs/ROADMAP.md:206-213 | REAL | The snapshot block is explicitly labeled historical with supersession note — honest archiving. |
| 22 | Phase-gate table, Phase 8 row: "final perf report vs VLC 3.0.23 on reference hardware; installers signed/launch-tested; `parity-report` states verified/partial/unsupported/blocked" | docs/ROADMAP.md:85 | PAPER-ONLY | As exit criteria they are unmet by the repo's own artifacts: no VLC wall-clock timing exists (PERF-VS-VLC.md:9), reference-hardware matrix BLOCKED (PERF-VS-VLC.md:20), binaries unsigned (RELEASE.md:138-144). |
| 23 | "77 nominal waves, ~96–115 with rework factor" | docs/ROADMAP.md:91 | PAPER-ONLY | Projection; the tree has passed 31+ waves with the worklog recording rework — the estimate is planning arithmetic, unfalsifiable. |

## 3. Contradictions

| This doc says | Other source says (path:line) | Which is true (proof) |
|---|---|---|
| Phase 8 status: "complete in conduct; exit gate … in the Lead's final pass" (ROADMAP.md:51-52) | ROADMAP.md:85 (its own gate table): "final perf report vs VLC 3.0.23 on reference hardware; installers signed/launch-tested" vs PERF-VS-VLC.md:9 ("No wall-clock Lumen-vs-VLC timing number exists anywhere in this repo") + RELEASE.md:138-144 ("Binaries are unsigned") | The gate table's criteria are unmet — "complete in conduct" is a status inflation against the roadmap's own exit definition. |
| "The program-exit record lands at phase8-exit.md when the release lane is green and the first tagged release ships (#66)" (ROADMAP.md:60-62) | RELEASE.md:189-195 "First real run (v0.1.0, 2026-11-13)"; README.md:28-31 "Releases v0.1.0, v0.1.1 and v0.1.2 are published"; local git tags v0.1.0–v0.1.3 | If the releases happened, the roadmap's own trigger fired and no phase8-exit.md exists; if they did not, RELEASE.md is fabricated. Either way the roadmap block is not current. |
| D-017: "catalog count is 61 commands / 23 events … catalog_shape.rs pins the exact counts" (ROADMAP.md:114) | Executed cargo test at HEAD: 2 failed — "catalog lists 65 commands vs test pinning 64"; catalog_shape.rs:7 "61 command entries (+ the issue #32 bookmark trio = 64) / 26 event entries" | The live catalog (65) — the drift between the decision text, the pin, and the code is proven by the failing tests. |
| D-016 commit `2d06607` (ROADMAP.md:113) | git: "fatal: ambiguous argument '2d06607': unknown revision" | The git history — the cited commit is absent. |

## 4. Fabrication/inflation indicators

1. **Phantom commit**: D-016 cites `2d06607` as the batch-flip commit; no such revision exists. Either the hash is fabricated, mistyped beyond resolution, or refers to a squashed-away commit — all three possibilities indict the decision register's auditability.
2. **"Complete in conduct"**: Phase 8's status phrase borrows the vocabulary of completion while the same file's gate table demands artifacts the repo admits it does not have (VLC wall-clock perf, signed installers). Status inflation through soft wording.
3. **Stale refresh discipline**: the header advertises "last refreshed 2026-11-13" yet the phase-8 block still speaks of the first tagged release as future work — the file was refreshed the day BEFORE v0.1.0 shipped and never again.
4. **Count drift**: "102 pinned SVG exports" vs 101/105 on disk — a small, checkable number stated wrongly, the kind of unforced error that distinguishes transcribed from verified facts.
