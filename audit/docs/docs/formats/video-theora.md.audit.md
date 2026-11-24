# CLAIM AUDIT: docs/formats/video-theora.md
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 3 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 3 / 0 / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Accurate decision record: the matrix row is blocked ×4 exactly as cited,
no Theora decoder exists in the codecs tree, and the doc claims no code
changed — true.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | matrix row carries "status = blocked on all four platforms with empty evidence" | :8-10 | REAL | feature-matrix.toml:507 `status = { windows = "blocked", … web = "blocked" }`, evidence "" |
| 2 | "crates = [] — none admitted … no theora engine" | :14-15 | REAL | `ls crates/codecs/src/video/` → av1 dnx h264 hevc mjpeg mpeg12 vp9 (no theora) |
| 3 | "Ogg carriage is landed" | :11 | REAL | docs-sweep table + matrix container-ogg row is claimed/implemented (container-ogg record exists in docs/formats/) |

## 3. Contradictions
None.

## 4. Fabrication / inflation indicators
None — a defer-with-promotion-path record; explicitly rejects both
hard-blocked and self-build-now framings.

Findings: none. Commands: rg matrix row, ls codecs (exit 0).
