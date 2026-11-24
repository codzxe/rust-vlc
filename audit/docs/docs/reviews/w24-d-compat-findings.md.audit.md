# CLAIM AUDIT: docs/reviews/w24-d-compat-findings.md
| Field | Value |
|---|---|
| Auditor | W19-A02 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Claims extracted | 8 |
| REAL 5 / REAL-UNPROVEN 2 / SHALLOW 1 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNREACHABLE 0 | |
| Internal contradictions found | 1 (review's "exactly" vs the gap-triage transcription error) |

## 1. Blunt verdict
A review-scout report that holds up: its headline verdict (0 P0 / 1 P1 / 2 P2 / 10 P3 —
13 numbered findings present in the doc) matches its own body; its P1-1 (missing worklog
append) is proven TRUE by the fix trail (W24-P appended the entry retroactively with a
disclosure at worklog.md:821-822); its P2-1 (refusals hidden inside claimed-implemented
rows) is corroborated by the compat-matrix machine record's own POST-RUN EDITORIAL NOTE
that cites P2-1 verbatim. One overstatement: "the gap triage's five flips match the run's
evidence exactly" — the flips match, but the gap-triage's worst-row figure is a wrong
transcription (4.94e-1 vs the measured 5.42e-1 in vlc-f32-tolerance json), which this
review missed and the W19-A02 recovery audit later caught.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "0 P0 / 1 P1 / 2 P2 / 10 P3" verdict | :21 | REAL | rg '^### P' -> 13 sections (1 P1 + 2 P2 + 10 P3), count matches |
| 2 | P1-1: "the 9-commit lane shipped with zero worklog entries" | :34-55 | REAL | worklog.md:821-822 — W24-D entry appended 2026-11-10 by W24-P "per W24-L finding P1-1 … content below is reconstructed" — the fix trail proves the finding |
| 3 | P2-1: "4 valid-class corpus refusals … counted but never surfaced" | :55 | REAL | compat-matrix-20261110.json POST-RUN EDITORIAL NOTE cites "finding P2-1 … surface the four in-row codec-level refusals (denominators 43/10)" — 4 refusals, cross-verified |
| 4 | "the status transitions were genuinely applied … zero hand-edited statuses" | :26-27 | REAL | applied jsonl applied_by "cargo xtask compat-matrix apply-transitions", 6 entries/5 rows; matrix rows now partial (my tomllib check) |
| 5 | "the gap triage's five flips match the run's evidence exactly" | :27 | SHALLOW | 5 distinct rows (asf/aac-lc/alac/vp9/pcm) DO match the applied ledger; but the gap-triage's own reason text transcribes the worst-row peak as 4.94e-1 while vlc-f32-tolerance-20261109.json measures 0305-avi-av.avi peak_dev=0.5424160254187882 (5.42e-1) — the flips match, the numbers do not ("exactly" overstates) |
| 6 | "the real 166-file run … 84 decoded / 22 decoded-partial / 6 refused / 0 demux-fatal / 54 malformed" | worklog quote :828 | REAL | sums to 166; matches compat-matrix counts block exactly (my reconciliation: 84+22+2+4+54=166) |
| 7 | git claims (25 files, +14,485/−15, 9 commits, branch refs) | :8-13 | REAL-UNPROVEN | no git allowed; commit hashes cited consistently downstream |
| 8 | P2-2: wasm "cell text is not derived from the step outcomes it cites" | :99 | REAL-UNPROVEN | code-level claim against the renderer; consistent with the recorded-steps lane design (not re-executed here) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "gap triage's five flips match the run's evidence exactly" | gap-triage json reason: "peak 4.94e-1"; f32-tolerance json: peak_dev 0.5424 | flips exact; worst-row figure transcribed wrong — the review's "exactly" is overbroad |

## 4. Findings
F-001 [S4] The review verified the five FLIP targets but not the numbers inside the
gap-triage reasons — the 4.94e-1 transcription error (vs measured 5.42e-1) slipped
through this review and was caught only in the W19-A02 recovery audit of the gap-triage
json. Reviews claiming "exactly" must re-derive the numbers, not just the row IDs.
Confidence: HIGH — Verification: ran (python3 json cross-check).

## 5. Prior-audit reconciliation
Dead W19-A02's own log corroborates this file's P1/P2 substance ("gap-triage worst-row
peak transcribed WRONG (4.94e-1 vs actual 5.42e-1) and propagated into matrix note L636 —
F-001 S3"); my re-derivation confirms both the review's valid findings and its one
overstatement.
