# CLAIM AUDIT: docs/audit/docs-sweep-20261110.md
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 6 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 5 / 0 / 0 / 0 / 0 / 0 / 1 |
| Internal contradictions found | 0 (one post-dated flip explains the only count drift) |

## 1. Blunt verdict
The 8.G1 docs-sweep's arithmetic is reproducible and correct at its date:
73 format rows, 53 records in docs/formats/, 14/6/9/8 claimed split. I
recomputed all of it from the current matrix; the only delta is one row
(codec-video-h264-software: blocked at sweep → implemented 2026-11-15 via
the Lead-admitted rust_h264 0.6.1) — a dated flip after the sweep, not a
sweep error. The flip's cited evidence itself resolves in-tree.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "73 format rows = 21 container + 16 video-codec + 19 audio-codec + 17 subtitle-format" | :26 | REAL | python3 recount: exactly 21+16+19+17=73 |
| 2 | "37 claimed on desktop (implemented or partial: 14 / 6 / 9 / 8)" | :26-27 | REAL (at date) | recount today: 38 (14/7/9/8) — the +1 is codec-video-h264-software, Lead-admitted 2026-11-15 per matrix:427-429; rust_h264=0.6.1 + sw.rs + h264_sw_differential.rs + record all exist in-tree |
| 3 | "0 claimed on web" | :27 | REAL | format rows' web column is not_started throughout (matrix:427 etc.) |
| 4 | "docs/formats/ holds 53 records" | :27 | REAL | `ls docs/formats/*.md | wc -l` → 53 |
| 5 | blocked rows' decision records exist (video-hevc.md, video-h264-software.md, video-screen-codecs.md) | :60-66 | REAL | ls verified in docs/formats/ |
| 6 | F-1/F-2 defects "fixed on lane A's branch (unmerged at sweep time)" | :9 | UNVERIFIED | branch state not inspectable (no git) |

## 3. Contradictions
None that indict the sweep — the only delta is a post-dated matrix flip
documented by its own admission record.

## 4. Fabrication / inflation indicators
None — records two out-of-scope code defects instead of hiding them, and
explicitly refuses to change statuses ("the parity tool's business").

Findings: F-001 [S4] sweep is a snapshot; one row it filed as blocked is
now implemented — consumers reading it today get a stale blocked story.
Commands: python3 matrix recount (exit 0), ls counts (exit 0).
