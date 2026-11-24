# CLAIM AUDIT: docs/gates/phase5-records-audit.md
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Claims extracted | 9 |
| REAL / REAL-UNPROVEN / UNVERIFIED | 4 REAL / 2 REAL-UNPROVEN / 3 UNVERIFIED |
| Internal contradictions found | 0 |
| Commands run | `ls docs/formats/ | wc -l` -> 53; `python3 scripts/q2v_matrix_backfill.py --check` -> DRIFT, 10 problems, exit 1 |

## 1. Blunt verdict
A wave-9 point-in-time audit report whose artifact claims check out (the three subtitle records exist, the backfill reproducer script exists and its drift check really exits 1), but whose own §4 verification command is now RED on the current tree: the matrix has drifted past its db90dbc target by 10 cells (including two status DOWNGRADES on audio codecs). The report's "zero missing rows" verdict is historical truth at best; re-running its verification today fails. The 230-passed subtitles test claim could not be re-executed (build budget).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "`ls docs/formats/` = **40 records**" | L18 | was-REAL / stale | today `ls docs/formats/ | wc -l` -> 53 (grew; not fraud) |
| 2 | "sub-srt.md, sub-ass-ssa.md, sub-vtt.md" authored | L19-20 | REAL | all three exist under docs/formats/ |
| 3 | "scripts/q2v_matrix_backfill.py ... drift check, exit 1" | L10-14 | REAL | script exists; `--check` -> exit 1 (real failing check) |
| 4 | "38 rows backfilled ... --check to verify" | L109-110 | REAL-then / RED-now | `--check` today: "DRIFT vs main db90dbc ... 10 problem(s)", exit 1 |
| 5 | "Cell counts: not_started 842→723, implemented 9→87, partial 0→21, blocked 121→141" | L119-120 | UNVERIFIED | needs db90dbc-era tree; git forbidden |
| 6 | "`cargo test -p subtitles --all-features` ... 230 passed / 0 failed" | L149-153 | REAL-UNPROVEN | no built binaries in target/debug/deps (0 matches); compile > 2 min budget (cargo-build-budget) |
| 7 | "Verdict: zero missing rows." | L157 | historical | matrix evolved (claim 4 drift); current-state re-audit is other agents' lane |
| 8 | "no vlc binary in the sandbox" (oracle note) | L140-141 | REAL | sandbox has ffmpeg 7.1.5 (verified) and no vlc oracle claim anywhere contradicted |
| 9 | "Records authored cite only worklog Task IDs ... nothing transcribed from memory" | L154-155 | REAL-UNPROVEN | spot-check of docs/formats/sub-srt.md citations not done in this lane |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "codec-audio-aac-lc / -alac: implemented" (wave-9 backfill) | current matrix: `.linux` = partial (backfill --check output) | the CURRENT matrix is later data; the doc is stale, not lying — but the downgrade direction (implemented→partial) is unusual and worth a matrix-auditor's eyes |
| "codec-video-vp9: blocked" (wave-9) | current matrix: windows implemented / linux partial / macos implemented | later waves landed vp9; consistent with codecs default feature `codec-vp9 = ["dep:rusty_vp9"]` (crates/codecs/Cargo.toml) |

## 4. Fabrication / inflation indicators
None — the drift-check command it ships is exactly the one that exposes its own staleness. The report downgrades itself on re-run, which is the opposite of evidence inflation.

## 5. Findings
F-001 [S4] — the doc's live re-verification command has rotted.
Where: docs/gates/phase5-records-audit.md:147-148
Code:
```
- `python3 scripts/q2v_matrix_backfill.py --check` → OK (38 transitions,
  252 rows parse, rules hold).
```
What is wrong: the same command today exits 1 with 10 drift problems (matrix moved on: matroska implemented, vp9 landed, aac-lc/alac downgraded to partial on linux). The doc reports its session truth but is now a stale gate snapshot with no date-of-validity discipline. Confidence: HIGH — Verification: ran (`--check` -> DRIFT, exit 1).
F-002 [S3] — record-count headline ("40 records") is now 53 with no reconciliation note anywhere in this file.
Where: docs/gates/phase5-records-audit.md:18 vs `ls docs/formats/ | wc -l` -> 53.
What is wrong: 13 records landed after this audit without any addendum; a reader citing this audit as the records census is 13 records behind. Limited blast radius (superseded census). Confidence: HIGH — Verification: ran.

## 6. Open questions for the next agent
- Who owns reconciling the 10-cell drift the backfill script reports (especially the implemented→partial downgrades)? The script that can answer exists and is red — run it and read its output.
