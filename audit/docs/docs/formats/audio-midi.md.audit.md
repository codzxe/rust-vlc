# CLAIM AUDIT: docs/formats/audio-midi.md
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 3 |
| REAL 2 / UNVERIFIED 1 (crates.io facts) | |

## 1. Blunt verdict
A decision record that scrupulously separates the two matrix rows: SMF demux = honest not_started (no code exists — verified), synthesis = blocked-v1 with a named [PROPOSAL] path. The external crates.io license facts (oxisynth LGPL, rustysynth) are dated fetch claims, unverifiable offline.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "`container-midi-smf` (SMF demux) = later ... no `smf` module exists in `crates/demux/src/`, honest not_started" | Status | REAL | ls crates/demux/src/ -> no smf dir (grep count 0); matrix row container-midi-smf = not_started ×4 (grep) |
| 2 | "`codec-audio-midi-synth` (synthesis) = blocked v1 ... to match the matrix's blocked ×4 row" | Status | REAL | matrix row codec-audio-midi-synth = blocked ×4 (grep status line) |
| 3 | "oxisynth 0.1.0 ... LGPL-2.1 (all six releases since 2022-07-03 ...)" + rustysynth facts | Evidence | UNVERIFIED | external crates.io/GitHub fetches 2026-10-24; offline sandbox; no raw transcript committed here |

## 3. Contradictions
None (doc-vs-matrix statuses match exactly — the rare doc that reconciles both rows).

## 4. Fabrication / inflation indicators
None; the record self-corrects a doc-04 framing mismatch explicitly ("later, not blocked" -> blocked-v1-with-later-path) with the reasoning shown.
