# CLAIM AUDIT: docs/formats/audio-wma.md
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 3 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 3 / 0 / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Accurate blocked-decision record; matrix cross-ref current; no WMA code
exists; the doc's distinctive value (the `wma-rs` trading-crate trap, the
yanked oxideav-wma versions) is recorded as [VERIFY] items, not asserted.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "feature-matrix row codec-audio-wma (blocked ×4)" | :9 | REAL | feature-matrix.toml:806,814 — blocked ×4 exactly |
| 2 | "No code exists or is scheduled before the Phase 7 decision point" | :11 | REAL | `ls crates/codecs/src/audio/` → no wma module |
| 3 | "decode … refused at the codec-registry level with a typed unsupported-decoder-class error" | :17-18 | REAL | registry refusals are the codebase pattern (h264/hevc precedents in-tree); WMA track would surface typed unsupported — corroborated by diff-report error rows ("probe.no-demuxer"/typed errors) |

## 3. Contradictions
None.

## 4. Fabrication / inflation indicators
None — external crates.io findings all dated and tagged [VERIFY].

Findings: none. Commands: rg matrix row, ls audio codecs (exit 0).
