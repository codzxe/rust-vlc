# CLAIM AUDIT: docs/ROADMAP.md (241 lines, program-level plan & decision register)
| Field | Value |
|---|---|
| Auditor | W19-A09 — RECOVERED by W19-R2 |
| Claims extracted | 14 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 11 / 0 / 0 / 0 / 1 (drift) / 0 / 2 |
| Internal contradictions found | 0 (1 count-drift vs tree, logged as finding) |

## 1. Blunt verdict
The roadmap's authority claims are checkable and check out: every gate record it cites
exists (phase0–7 exit files; phase8-exit absent exactly as stated), the dated Lead
square-ups (2026-11-13) are verbatim in phase3/phase4-exit.md, the decision register's
hard numbers (61/23 catalog, ureq 3.5.0 + ring 0.17.14, 13 settings areas, 47/47 phase-5
cards, 200 cards / 77 waves) reproduce against code, Cargo.lock and the design inventory.
It also discloses its own governance gaps honestly (ADR-007/008/009 "no file exists").
One stale count: "102 pinned SVG exports" vs 101 actual .svg files in design/figma.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Phase 3 "squared up with a dated Lead sign-off on 2026-11-13 (issue #59)" | :16-18 | REAL | docs/gates/phase3-exit.md:254-256: "## Lead sign-off (the owed square-up, 2026-11-13)" / "**Verdict: PASS (Lead-signed 2026-11-13).**" — byte-verified. |
| 2 | Phase 4 "signed by the Lead on 2026-11-13 (issue #59)" | :20-21 | REAL | phase4-exit.md tail: "## Lead sign-off (the owed square-up, 2026-11-13) … **Verdict: PASS (Lead-signed 2026-11-13).** … issue #59 square-up." |
| 3 | Phase 5 "47/47 parent cards resolved" | :25-26 | REAL | phase5-exit.md:5 "= 47 parent cards, all resolved"; :21 "## 2. Card ledger (47 resolved)". |
| 4 | Phase 6 exit "verdict **PENDING LEAD DECISION** … drafted at docs/gates/phase6-exit.md" | :36-38 | REAL | phase6-exit.md:3 "Status: **DRAFT — PENDING LEAD DECISION**." |
| 5 | Phase 7 "ran NOT PASSED at `c339ee8` with 5 bounded findings (OF-1..OF-5)" | :43-44 | REAL | phase7-exit.md:10 "**NOT PASSED at `c339ee8` — 5 open findings (OF-1…OF-5)**" (:5/:73 cite c339ee8). Merge 903d618c itself UNVERIFIED (no git). |
| 6 | Phase 8 record "lands at `docs/gates/phase8-exit.md` when the release lane is green" | :60-62 | REAL | `ls docs/gates/` -> phase0-exit-report … phase7-exit.md only — phase8-exit.md absent, exactly as the doc conditions it. |
| 7 | D-002 "Settings areas = 13 (global, interface, …, extensions)" | :99 | REAL | design/README.md:20 lists the identical 13 areas + "count confirmed by decision D-002". |
| 8 | D-011 "ureq 3.5.0 admitted with ring 0.17.14 … `docs/adr/ADR-011-tls-https.md`" | :108 | REAL | Cargo.lock:749-750 `name = "ring" / version = "0.17.14"`, :1214-1215 `name = "ureq" / version = "3.5.0"`; ADR file exists. |
| 9 | "ADR-007 … **no ADR-007 file exists in `docs/adr/` yet**" | :160-161 | REAL | `ls docs/adr/` -> 001,002×2,003,004,005,006,010,011,012,013,README — no 007 (nor 008/009, matching the PROPOSAL notes). Honest absence disclosure. |
| 10 | D-017 "command catalog count is 61 commands / 23 events … `catalog_shape.rs` pins the exact counts" | :114 | REAL | crates/commands/tests/catalog_shape.rs:4-5: "61 command entries (+ the issue #32 bookmark trio = 64) / 26 event entries (23 …" — pinned invariants present. |
| 11 | D-009 "Repository license = MIT … `LICENSE`" | :106 | REAL | LICENSE:1 "MIT License". |
| 12 | "design/ (102 pinned SVG exports + inventory)" | :203 | FALSE (stale count, S4) | `ls design/figma/*.svg | wc -l` -> **101**; design/README.md:10 says "Inventory (102 files, ~77 MB)" — off-by-one drift inherited from the inventory doc. No file was added since to reconcile it. |
| 13 | "200 task cards in 77 waves (0:17, 1:20, 2:17, 3:19, 4:25, 5:45, 6:17, 7:24, 8:16)" | :89 | REAL | Arithmetic recomputed: 17+20+17+19+25+45+17+24+16 = 200; wave letters 0.A–0.D … 8.A–8.H = 4+6+5+6+9+18+9+12+8 = 77. |
| 14 | "git-always + salvage protocol … 100% of cut work (11-for-11)" + commit hashes 2d06607/5a411ba/9aac5ca | :113,:241 | UNVERIFIED | No-git rule for auditors; salvage/commit claims not re-derivable here. |

## 3. Contradictions
None material. The 102-vs-101 SVG count is drift, not a contradiction between two
assertions of the current tree (the ROADMAP copies README's stale number).

## 4. Fabrication / inflation indicators
None. Phase statuses carry their own bad news in-line: Phase 6 "PENDING LEAD DECISION"
with named debts (6.H1 security finding, 0/252 parity), Phase 7's original "NOT PASSED"
verdict is preserved rather than papered over, and the 2026-11-09 "Phases 7–8 NOT
STARTED" error is kept and explicitly superseded (:64-67) instead of silently rewritten.
