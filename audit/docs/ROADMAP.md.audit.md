# CLAIM AUDIT: docs/ROADMAP.md (L1-241)
| Field | Value |
|---|---|
| Auditor | W19-A09 |
| Date | 2026-11-24 |
| Claims extracted | 12 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 8 / 0 / 0 / 0 / 0 / 0 / 4 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The ROADMAP is a disciplined program-state document: every phase-gate record it cites exists on disk, the phase-7 "NOT PASSED at c339ee8 … closed at wave-24 square-up" narrative is corroborated verbatim by docs/gates/phase7-exit.md, the phase-6 PENDING verdict matches its draft ledger, and the decision register (D-001..D-018) records conflicts with resolutions rather than hiding them. Defects are small: the "102 pinned SVG exports" count is wrong (105 .svg files under design/figma, 101 top-level), the header date (2026-11-13, wave-26) trails the README's status (2026-11-15, wave 30) with no note, and it defers to the register for the ADR-007..009 [PROPOSAL]s it cannot number yet (honest). Commit hashes cited throughout are UNVERIFIABLE (git forbidden).

## 2. Claim ledger
| # | Claim (verbatim, ≤25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Phase 0 — COMPLETE (`docs/gates/phase0-exit-report.md`)" | :7 | REAL | File exists (ls docs/gates/). |
| 2 | Phase gate records exist for 1–7 (phase{1..7}-exit.md) | :9-42 | REAL | All present in docs/gates/ (phase3-exit-draft.md also kept). |
| 3 | Phase 6 "verdict PENDING LEAD DECISION… gate-line runs… remain owed at gate time" | :42-45 | REAL | docs/gates/phase6-exit.md: "Status: **DRAFT — PENDING LEAD DECISION**… Gate-line runs… PENDING"; parity snapshot 60/252 at wave 13 (baseline now 61-66/252 — consistent drift, no fabrication). |
| 4 | Phase 7 "gate ran NOT PASSED at `c339ee8` with 5 bounded findings (OF-1..OF-5)… closed at the wave-24 integration merge `903d618c`" | :46-52 | REAL (gate-record side) | docs/gates/phase7-exit.md:5-8 "**NOT PASSED at `c339ee8` — 5 open findings (OF-1…OF-5)**" + closure path matching. Commit hashes themselves UNVERIFIED (no git). |
| 5 | D-017: "command catalog count is 61 commands / 23 events… `catalog_shape.rs` pins the exact counts" | §3 D-017 | UNVERIFIED (file exists, count not re-derived) | crates/commands/src has catalog_shape.rs (rg-located); a full count would need compiling the crate — skipped (cargo budget). |
| 6 | D-002: "Settings areas = 13" | §3 | REAL | ui/src/screens/settings/areas/index.tsx registry implements the 13-area model (verified in matrix rows-183-195 audit, row 195). |
| 7 | D-007: "Baseline artifacts live in `docs/baselines/` (plural)… standardizes doc 02 (`docs/baseline/`)" | §3 | REAL | docs/baselines/ is the actual dir — the drift is REGISTERED as a decision, not hidden. |
| 8 | ADR index: ADR-001..006,010,011,012 files exist; ADR-007/008/009 [PROPOSAL], "no ADR-007 file exists in docs/adr/ yet" | §6 | REAL | docs/adr/ listing matches exactly (13 files + README); the un-numbered proposals are disclosed. |
| 9 | Doc index: "design/ (102 pinned SVG exports + inventory)" | §7 | FALSE (count) | 105 .svg files under design/figma (101 top-level + 4 in "Add Subtitles to Playlist/"); README.md inventory exists. Off-by-3 stale count. |
| 10 | "worklog.md — live log at the repository root" | §7 | REAL | worklog.md exists at repo root. |
| 11 | §8 historical snapshot kept verbatim, marked "[HISTORICAL SNAPSHOT — do not update in place]" | :181+ | REAL | Good-faith versioning: superseded state explicitly labeled. |
| 12 | Header: "last refreshed: 2026-11-13 (wave-26…)" vs README "2026-11-15, wave 30" | :1-2 / README:16-17 | UNVERIFIED (drift) | The two status pages carry different refresh dates; ROADMAP does not state which is newer — minor doc-currency drift, both plausible. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "102 pinned SVG exports" | filesystem: 105 .svg in design/figma | The filesystem; the ROADMAP count is stale. |
| Phase 6 parity "60/252 implemented-or-verified" (gate draft, wave 13) | baseline parity-report (2026-11-23): 61-66/252 | Both true at their own dates; the ROADMAP cites the gate-time snapshot, not present state — no false claim, but a reader must know which is current. |

## 4. Fabrication / inflation indicators
None found. Failures are recorded as failures (Phase 7 NOT PASSED, Phase 6 PENDING, netfuzz "HONEST: not in main" in the phase-6 ledger). The stale SVG count is sloppiness, not inflation.
Commands run: ls docs/gates/, design/figma counts (101/105), worklog.md existence, rg catalog_shape.rs, head of phase6/phase7 exit files. Exit 0.
Findings: S2: 0 · S3: 0 · S4: 2 (F-017 stale "102 SVG exports" count; F-018 ROADMAP/README refresh-date drift undocumented).
