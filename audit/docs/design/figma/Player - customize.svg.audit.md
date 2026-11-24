# CLAIM AUDIT: design/figma/Player - customize.svg
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 4 |
| REAL / REAL-UNPROVEN / FALSE / UNVERIFIED | 3 / 1 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Valid Figma export, 1920x1080 base customize-interface screen (448 elements,
151 paths, XML parses clean, includes topbar groups). Cited by docs/08:158 as
the `player.customize` base and by docs/inventory/ui-inventory.md:386 as the
"Tools → Customize Interface" design source. The 9-file family exists intact.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Valid SVG design asset | :1 | REAL | ElementTree parse OK; root 1920x1080; `file` -> SVG ASCII. |
| 2 | docs/08:158 — `player.customize` = "`Player - customize.svg`, `-1..8`"; "control-bar editor ... direct input for the customize panel's data model" | docs/08-ui-figma-fidelity.md:158 | REAL | All 9 files exist (`ls \| rg customize`); group content is button/toggle-rich (151 paths, `id="topbar"` + Frame groups verified in diff vs -3). |
| 3 | ui-inventory.md:386 — customize screen lists buttons (Playback Speed, ±10s/±15s, Stop, Repeat, Shuffle, Record, Snapshot, PiP, HDR, Zoom, ...) | docs/inventory/ui-inventory.md:386 | REAL-UNPROVEN | File carries the button geometry; per-button identification (which path is "Snapshot") would need render/label extraction — the w26.g07.md:8 audit already used `Player_-_customize.png` as frame evidence; not re-derived here. |
| 4 | w26.g07.md:8 — "base state) vs `Player_-_customize.png` (`design/figma/Player - customize.svg`, 1920×1080)" | docs/waves/w26/findings/w26.g07.md:8 | REAL | This SVG is 1920x1080 (root attrs) as stated. |

## 3. Contradictions / Fabrication
None. Design asset with no prose claims.
