# CLAIM AUDIT: docs/02-phase0-audit-plan.md
| Field | Value |
|---|---|
| Auditor | W19-A09 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Claims extracted | 12 (artifact-path claims, G1-G13) |
| REAL 7 / REAL-UNPROVEN 0 / SHALLOW 0 / STUB 0 / PAPER-ONLY 1 / FALSE 0 / UNREACHABLE 0 | |
| Internal contradictions found | 0 (path drifts only) |

## 1. Blunt verdict
A plan whose exit-gate artifacts all exist today. Every defect is path aging, not
phantom work: three G-rows name files that live elsewhere (baseline→baselines; exit report
under docs/gates/; hotkeys.toml became keymaps/default.toml), and one ADR filename differs
by a suffix. The gate itself closed: docs/gates/phase0-exit-report.md ticks 13/13 rows
with evidence links and an honest [VERIFY-blocked-on-hardware] class. This ledger CONFIRMS
the dead agent's self-correction (the gate is closed; its first-pass "never created" was
wrong) and confirms the drift class instead.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (command + result) |
|---|---|---|---|---|
| 1 | G1: "`docs/baseline/vlc-3.0.23.md` + `docs/baseline/vlc-3.0.23.json`" | :45 | SHALLOW (path drift) | ls: docs/baseline/ MISSING; docs/baselines/vlc-3.0.23-{linux,windows,macos}.md exist |
| 2 | G2: "`docs/inventory/raw/` + `docs/inventory/raw/manifest.json`" | :46 | SHALLOW (path drift) | raw/ exists; manifests live at raw/modules/manifest.json + raw/options/manifest.json, none at raw/ root |
| 3 | G3: modules-{windows,linux,macos}.json + modules-summary.md | :47 | REAL | all present in docs/inventory/ listing; modules-linux.json total=502 verified by me |
| 4 | G4: options-{windows,linux,macos}.json + options-summary.md | :48 | REAL | present; options-linux.json = 1,476 records (python3 count); summary's D7 0-difference corroborated |
| 5 | G5: formats.json + formats.md | :49 | REAL | both exist |
| 6 | G6: "`docs/inventory/ui-inventory.md` + `docs/inventory/hotkeys.toml`" | :50 | SHALLOW | ui-inventory.md EXISTS; hotkeys.toml MISSING — data landed as keymaps/default.toml (exit report row 4 cites it) |
| 7 | G7: "`docs/feasibility-report.md`" | :51 | REAL | exists |
| 8 | G8: "`docs/adr/ADR-001-hardware-decoding.md`" | :52 | SHALLOW | actual file: docs/adr/ADR-001-hardware-decode.md (suffix drift) |
| 9 | G11: matrix "Parses as TOML", seeded | :54 | REAL | baseline xtask-feature-audit.txt: "rows: 252", parses; per-status census printed |
| 10 | G12: "`docs/inventory/corpus-plan.md`" | :55 | REAL | exists |
| 11 | G13: "The Lead writes `docs/phase0-exit-report.md` ticking each row with links." | :39,55 | SHALLOW (path drift) | exists at docs/gates/phase0-exit-report.md; 13/13 rows ticked; "Gate verdict: PASS" |
| 12 | "A phase is done only when its gate passes with recorded evidence" (MP §16 quote) | :39 | REAL | the exit report carries evidence links per row |

## 3. Contradictions
None material. The plan's paths vs disk: 5 drifted paths, all resolvable to existing
artifacts (see ledger).

## 4. Findings
F-001 [S4] Five G-row artifact paths in this doc point at paths that do not exist as
written (baseline/ vs baselines/; raw/manifest.json vs per-dir manifests; hotkeys.toml vs
keymaps/default.toml; ADR-001 filename; phase0-exit-report.md vs docs/gates/). Any script
or agent following the doc's paths verbatim fails. Confidence: HIGH — Verification: ran
(ls loop, 10 paths probed).
F-002 [S4] hotkeys.toml is the one genuinely renamed artifact (PAPER-ONLY as written);
its consumer form is keymaps/default.toml — the doc was never updated. Confidence: HIGH.

## 5. Prior-audit reconciliation
Dead W19-A09 audited this file, logged "G13 never created (S3)", then self-corrected
(CORRECTION-2 in its run log: docs/gates/phase0-exit-report.md EXISTS; reclassified to
path drift S4). My independent re-check confirms the correction: the gate is CLOSED (PASS,
13/13 ticked). Remaining value-add of this recovery ledger: the other four path drifts
(G1/G2/G6/G8) are enumerated and verified above.
