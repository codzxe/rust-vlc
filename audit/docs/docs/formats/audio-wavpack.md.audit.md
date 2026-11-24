# CLAIM AUDIT: docs/formats/audio-wavpack.md (49 lines — gap record)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 4 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 4 REAL / 0 / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Pure gap record, every claim verified: no wavpack anywhere in the workspace graph
(Cargo.toml/Cargo.lock rg -> 0), no .wv demuxer (crates/demux/src/ tree has none), matrix
row codec-audio-wavpack blocked x4 with the exact same blocked_reason text this record
carries (GPL-3.0-or-later symphonia-codec-wavpack 0.1.1 vs MIT repo, decision parked with
doc 11). Honest "Nothing implemented" self-assessment.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "status = blocked (v1) — license conflict candidate, no decode" | :9-10 | REAL | matrix :820-834 codec-audio-wavpack blocked x4 with matching reason. |
| 2 | "crates = [] — nothing admitted" | :12 | REAL — REPRODUCED | rg 'wavpack' Cargo.toml Cargo.lock -> 0. |
| 3 | "no .wv demuxer in crates/demux/" | parse § | REAL — REPRODUCED | ls crates/demux/src/ -> adts aiff asf avi es ivf mkv mp4 ogg ps ts wav (no wv). |
| 4 | "fuzz_target = none (no decoder admitted)" | :11 | REAL | No wavpack fuzz target in the 35-target census. |

## 3. Contradictions
None.
## 4. Fabrication / inflation indicators
None — a records-sweep gap record created for an already-blocked row; nothing overstated.
