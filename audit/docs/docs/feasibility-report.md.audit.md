# CLAIM AUDIT: docs/feasibility-report.md (120 lines — Phase 0 deliverable)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 8 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 5 REAL / 0 / 0 / 0 / 0 / 0 / 3 UNVERIFIED (sections' evidence, historical counts) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A 2026-10-19 conclusions document whose headline numbers reproduce exactly where the
tree is stable: 252 matrix rows (rg count = 252), 61 P0 rows (my parse = 61 EXACT),
502 modules (raw/linux/05 = 502), 1476 options / 58.3% advanced-folded (both verified in
my dump-14 audit), 8 menus/58 items/23 dialogs/111 hotkeys (ui-inventory.md:428, same
figures). The 33-blocked figure was true at authoring; 26 rows are blocked x4 today —
7 rows since moved (progress, not error: 13% of 252 = 32.8 ~ 33 arithmetic consistent).
The report's honesty framing ("Nothing here is verified in the 17 sense — no CI exists
yet") prevents doc-inflation; it defines a parity ceiling instead of claiming one.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Of 252 matrix rows: 61 are P0; 33 are blocked-with-reason (13%)" | §1 | REAL at date (P0 still exact) | 252 rows (rg); P0 = 61 today (parse); blocked x4 = 26 today vs 33 (rows unblocked since 2026-10-19). |
| 2 | "Modules: 502 registrations / 356 unique / 56 capabilities" | §2 | REAL (502 verified) | raw/linux/05-vlc-list.txt non-comment lines = 502. |
| 3 | "Options: 1,476 (100% dump coverage; 58.3% advanced-folded; 442 core ...)" | §2 | REAL — all verified | dump-14 audit: 1476 canonical records, core 442-443, 615+861 split. |
| 4 | "Formats: 82 families inventoried; 59 probe-verified ... 27 P0" | §2 | UNVERIFIED (sections' data) | inventory/formats.json not re-counted here. |
| 5 | "UI surface: 8 menus / 58 items / 23 dialogs; 111 hotkey actions" | §2 | REAL | ui-inventory.md:428 (a) item carries the same figures (verified in my earlier read). |
| 6 | "Opus placeholder crate is 1-byte empty — blocked v1" | §3 | SUPERSEDED (dated) | Opus is implemented today (matrix :738-741, ruopus 0.1.2 admitted 2026-11-09) — the report's blocked verdict was honest for Phase 0; history moved. |
| 7 | "Foundation pre-audit (18 crates): pass x 8, conditional-E2 x 7, exception-ADR-004 x 2, license-recheck x 1" | §6 | UNVERIFIED (preaudit doc) | docs/dependencies/preaudit-foundation.md exists (cited by ADR-004). |
| 8 | "This report claims nothing as verified; it defines what Phase 1+ must prove" | §8 | REAL framing | No false "passes" found in the document. |

## 3. Contradictions
None — the superseded verdicts (Opus, h264/HEVC software tracks) are dated Phase-0
conclusions that later Lead decisions overturned with their own records.
## 4. Fabrication / inflation indicators
None.
