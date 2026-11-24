# CLAIM AUDIT: docs/formats/container-mp4.md
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 4 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 3 / 1 / 0 / 0 / 0 / 0 (1 stale-ref) | 0 |
| Internal contradictions found | 1 (dated cross-ref) |

## 1. Blunt verdict
The record's implementation claims are real code (crates/demux/src/mp4
exists with the module layout), and its "matrix row still reads
not_started — this record is that evidence" framing is exactly the
honest-gap pattern; the row has since been flipped to implemented ×3 with
this record as the evidence string (the flip it asked for happened).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Feature-matrix row container-mp4-mov-3gp still reads not_started with empty evidence" | :6-8 | STALE-REF (true at date) | today matrix:150-151 implemented ×3 + evidence citing this very record ("5.REC-1: 30-lane corpus_mp4 table") — the Lead amendment landed |
| 2 | "self_built = crates/demux/src/mp4/** … mp4parse NOT used" | :17-18 | REAL | `ls crates/demux/src/mp4` → mod.rs, error.rs, fuzz.rs, tests.rs + stbl boxes; no mp4parse in Cargo.toml (demux is std-only) |
| 3 | probe "EXTENSIONS = [...mp4, mov, qt, 3gp…] (mp4/mod.rs:138, verified this audit)" | :25-26 | REAL | crates/demux/src/mp4/mod.rs:137-139 `pub const EXTENSIONS: &[&str] = &["mp4","m4v","m4a","mov","qt","3gp","3g2","3ga","3gpp"]` byte-verified |
| 4 | "Corpus evidence (5.C1, corpus-v4, ffmpeg 7.1.5 — 30/30)" | :49-50 | REAL-UNPROVEN | corpus-v4 media absent from sandbox; diff evidence not re-runnable; baseline rc-smoke MP4 litmus (Idle→…→Stopped, exit 0) corroborates the demux path works |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| matrix row "still reads not_started" | matrix:150 implemented ×3 today | matrix current — record predates its own demanded amendment |

## 4. Fabrication / inflation indicators
None — self-labels as docs-only evidence for a Lead amendment rather
than self-flipping the matrix (correct MP §15 discipline).

Findings: F-001 [S4] dated cross-ref (row since flipped).
Commands: rg matrix row, ls mp4 module (exit 0).
