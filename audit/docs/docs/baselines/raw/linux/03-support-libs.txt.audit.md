# CLAIM AUDIT: docs/baselines/raw/linux/03-support-libs.txt
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 3 |
| REAL 3 | |

## 1. Blunt verdict
A raw acquisition log that records its own failed first round verbatim ("HONEST FAILURE: apt-get download is all-or-nothing, it aborted ... and downloaded nothing") before the successful rounds. The arithmetic is exact: 30 round-2 packages + 2 round-3 = 32 support libs; the hash list has exactly 32 entries; the final scan says PLUGINS=372 MISSING=0. This file is the ground truth that exposes the vlc-3.0.23-linux.md §1 summary sentence ("every downloaded .deb ... cross-checked MATCH") as an overclaim — the 32 support debs here are hash-COMPUTED only, not index-cross-checked.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Round 1 ... HONEST FAILURE: apt-get download is all-or-nothing, it aborted with these errors and downloaded nothing" | :10-19 | REAL | verbatim E: lines committed (8 error lines) — failure evidence preserved, not cleaned |
| 2 | "Round 2 ... all OK (30 packages)" + "Round 3 ... 2 more" = 32 support libs | :29-34 | REAL | hash list lines 43-74 = exactly 32 entries (recounted); matches linux-baseline's "32 additional trixie library .debs" |
| 3 | "Final plugin dependency scan ... PLUGINS=372 MISSING=0 (was: 372/43 missing before support libs); TRANSITIVE_MISSING=0" | :36-38 | REAL | consistent with dump 03's summary consumed by the pin record (my earlier verification of the same numbers) |

## 3. Contradictions
None in-file. Feeds the cross-doc finding: linux pin record §1 "every downloaded .deb ... MATCH" vs this log's computed-only SHA256s for the 32 support libs (8/40 actually cross-checked).

## 4. Fabrication / inflation indicators
None — a mechanical raw log; failure rounds committed verbatim.
