# CLAIM AUDIT: docs/formats/audio-musepack.md
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 2 |
| REAL 2 | |

## 1. Blunt verdict
A pure gap record for a codec that does not exist in the tree — and says exactly that, with the matrix row matching (blocked ×4, empty evidence) and no code contradicting it.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "status = blocked (v1) — no pure-Rust decoder exists ... Nothing implemented; no MPC parser or decoder exists in-tree" | front-matter + §P/D/E | REAL | matrix row codec-audio-musepack: blocked ×4, evidence "" (grep); no musepack/mpc code anywhere in crates/ (demux/src listing has none) |
| 2 | "blocked_reason ... No pure crate (musepack NOT_FOUND 2026-10-19)" | Evidence | UNVERIFIED-as-cited / REAL-as-recorded | the crates.io NOT_FOUND survey is external; the matrix row carries the identical reason string (verified), i.e., record and ledger agree |

## 3. Contradictions
None.

## 4. Fabrication / inflation indicators
None — a 46-line record that exists only to close a documentation gap for a blocked row; nothing is claimed to work.
