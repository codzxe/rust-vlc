# CLAIM AUDIT: docs/corpus/vlc-diff-report-20261023.lumen.fixtures.tsv
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 2 (evidence-artifact integrity) |
| REAL 1 / FALSE 0 / UNVERIFIED 1 (machine-bound paths) | |

## 1. Blunt verdict
This is the harness-generated fixture list (run.py `run_lumen_digests` writes `out.with_suffix(".fixtures.tsv")`) for the 2026-10-23 differential run: 500 rows, exactly `rel<TAB>abs_path`, all rows NF==2 (awk-verified), 500 unique rels, 500 unique abs paths. Well-formed and machine-generated — but every absolute path points at `/home/z/my-project/corpus-big/...`, which DOES NOT EXIST in this sandbox (`ls /home/z/my-project/corpus-big` -> No such file or directory). The committed evidence is replayable only on the original machine; the digests it produced live in the sibling `vlc-diff-report-20261023.lumen.json` (not in my assignment).

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | 500 fixture rows rel+abs path pairs, consumed by the lumen_pcm batch test | whole file | REAL | `wc -l` -> 500; `awk NF!=2` -> zero malformed rows; format matches run.py:503-506 writer |
| 2 | Fixtures exist at the recorded absolute paths | col 2 | UNVERIFIED (absent) | `ls /home/z/my-project/corpus-big` -> "No such file or directory"; corpus is an external, non-tracked corpus (diff-report's own header: "19/35 of its inputs are lost (runtime-random lavfi sources)") |

## 3. Contradictions
| TSV says | Reality | Proof |
|---|---|---|
| 500 abs paths under /home/z/my-project/corpus-big | directory missing entirely | ls command above |

## 4. Fabrication / inflation indicators
None in the TSV itself (it is a machine artifact). The deeper evidence problem — the corpus backing committed digests is untracked and now absent — is recorded honestly in the diff report's own header (".md", not this file): "19/35 of its inputs are lost". The audit program's recurring pattern: evidence chains anchored to untracked/ephemeral inputs.
