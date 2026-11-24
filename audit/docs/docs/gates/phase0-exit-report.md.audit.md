# CLAIM AUDIT: docs/gates/phase0-exit-report.md
| Field | Value |
|---|---|
| Auditor | W17-A08 (recovered by W17-R1) |
| Date | 2026-11-24 |
| Claims extracted | 13 |
| Verdict counts | TRUE: 9 (several EXACT number matches against cited evidence) TRUE-AT-TIME/RESOLVED: 1 UNVERIFIED: 3 |
| Internal contradictions found | 0 (1 citation-path defect) |

## 1. Blunt verdict
The best number-hygiene of any gate doc in this recovery batch: every count
I re-derived matches its cited evidence EXACTLY — 252 matrix rows (rg count
252) and 21 categories (awk distinct = 21), 82 families / 59 probed
(formats.md total row 82/59/23), 1,476 option records (options-summary:21),
252/252 default agreement (options-summary D4 row "0 / 252 ... all 252
agree"), 8 menus / 58 items / 23 dialogs / 111 actions (ui-inventory.md:428
G6 claim, byte-matching), DR-01..DR-16 (16 DR hits in vlc-3.0.23.md), and
schema_version = 1 frozen 2026-10-19 (feature-matrix.toml:35). All 13 rows'
cited evidence files exist in the tree (28/30 verified by direct ls; the 2
"misses" resolve under different parents — one is a path-citation defect,
below). ADR statuses match the register, and D-011's "ADR-011 opened as
proposed" resolved exactly as predicted (register: accepted 2026-11-09).
Unverifiable: the two network claims (Launchpad, keyserver fingerprints)
and the git tag.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Gate verdict: PASS (with [VERIFY-blocked-on-hardware] rows explicitly carried, not hidden)" | :4-6 | TRUE-consistent | hardware-template.md + [VERIFY] markers across the inventory docs; runbook posture matches row 1 |
| 2 | Row 1: "VLC 3.0.23 pinned per platform ... per-platform records + raw/; DR-01..DR-16 discrepancy register" | :10 | TRUE | docs/baselines/: vlc-3.0.23.md + .json + -linux/-macos/-windows.md + raw/ + hardware-template.md all present; `rg -c 'DR-0' vlc-3.0.23.md` -> 16 |
| 3 | Row 2: "modules-{linux,windows,macos}.json, modules-summary.md" | :11 | TRUE | all four files exist (ls) |
| 4 | Row 3: "1,476 records, 100% coverage, 252/252 default agreement" | :12 | TRUE (EXACT) | options-summary.md:21 "**1476**"; :95 D4 "**0 / 252** ... all 252 options declared in both agree"; :13 "100% of Linux data comes from observed runtime" |
| 5 | Row 4: "8 menus/58 items/23 dialogs; 111 actions → keymap seed" | :13 | TRUE (matches cited evidence) | ui-inventory.md:428 G6 claim "8 menus, 58 leaf items ... 23 dialogs ... all 111 VLC 3.0.23 hotkey actions"; keymaps/default.toml exists. NOTE: ui-inventory.md:195 itself lists a **10-menu** bar — internal tension inside the cited evidence, not this report (flagged for that file's owner) |
| 6 | Row 5: "82 families; 59 probed ... raw/linux/17-format-probes.txt" | :14 | numbers TRUE / path S4-defective | formats.md:17-25 total row "**82** | **59** | **23**"; but `raw/linux/17-format-probes.txt` resolves to NO file — the probe log actually lives at docs/baselines/raw/linux/17-format-probes.txt (rg --files) |
| 7 | Row 6: "252 rows / 21 categories; schema_version 1 frozen" | :15 | TRUE (EXACT) | `rg -c '^id = ' docs/feature-matrix.toml` -> 252; `awk ... category` distinct -> 21; feature-matrix.toml:35 `schema_version = 1  # frozen 2026-10-19` |
| 8 | Rows 7-13 cited evidence files | :16-22 | TRUE | feasibility-report.md + feasibility/sections/{video,audio,subtitles,subsystems}.md; ADR-001/002/003 docs; audit-method/allowlist-format/preaudit-foundation/records/wasmi.md/ADR-002-evidence.md; platform-{windows,linux,macos}.md + platform-synthesis.md; security-phase0.md; docs/inventory/corpus-plan.md — ALL present (ls; only the row-5 probe path is misplaced, see #6) |
| 9 | Rows 8-10: ADR-001/002/003 "✅ accepted" | :17-19 | TRUE | adr/README.md register rows: ADR-001 accepted, ADR-002 accepted, ADR-003 "accepted (per-OS final confirmation gated on Phase 2 prototypes)" — all 2026-10-19 (Phase 0 gate) |
| 10 | D-011: "ADR-011 (TLS/HTTPS) opened as *proposed*; decision due ≤ Phase 5" | :40-41 | TRUE-AT-TIME / RESOLVED | register: ADR-011 "accepted | 2026-11-09 (activates issue #39)" — resolved exactly within the stated horizon |
| 11 | R-1: "Debian-trixie ... Ubuntu 24.04–25.10 don't carry it — Launchpad-verified" / R-2: "two keyserver anchors agree" | :26-30 | UNVERIFIED | network claims (Launchpad + keyserver), no egress in sandbox |
| 12 | R-5: "G1 ticked complete-with-hardware-pending as recorded in baselines/vlc-3.0.23.md §G1" | :35-36 | TRUE | vlc-3.0.23.md:114 "## 6. G1 evidence checklist (doc 02 §1.2 row G1)" section present |
| 13 | "Phase 0 is complete. Tag: phase-0-gate." | :64 | UNVERIFIED (tag) / doc-state TRUE | git tags not checkable from sandbox; every checklist row's artifact is present in-tree |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "8 menus" (:13, from ui-inventory's G6 claim) | ui-inventory.md:195 "Menu bar has **10 menus**" (VLC, File, Edit, View, Playback, Audio, Video, Subtitles, Window, Help) | The evidence file is internally inconsistent (8 vs 10 — likely two counting conventions: G6's transcription vs the full bar listing). This report faithfully mirrors its cited source; the defect belongs to ui-inventory.md, flagged for that file's owner |

## 4. Fabrication / inflation indicators
NONE. The opposite: blocked-on-hardware rows are carried explicitly
(:4-6), and every spot-checkable number reconciles exactly with its
evidence file. The single defect is the row-5 evidence path (S4).

## 5. Matrix rows
N/A — gate report, not a matrix chunk. (Cross-ref: the 252-row matrix the
report froze is the same one whose evidence-audit today reports 2 missing
evidence paths — baseline xtask-feature-audit.txt:10-12 — a LATER rot, not
a phase-0 defect.)
