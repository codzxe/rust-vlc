# CLAIM AUDIT: docs/corpus/vlc-diff-report-20261023.json (parts 1-13, single file)
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 8 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 4 / 0 / 0 / 0 / 0 / 0 / 4-unverifiable |
| Internal contradictions found | 0 (file self-consistent; 1 external contradiction vs manifest) |

## 1. Blunt verdict
This 9,815-line "diff report" contains ZERO comparisons: every oracle field is
null because no VLC binary exists in the environment, and the file says so
upfront. It is honest about that. What it does contain — 500 Lumen-only
decode/digest rows, classification reasons, and a companion lumen.json — is
internally consistent to a degree I verified mechanically (counts, statuses,
md5s, rels all cross-checked; two files agree 500/500). The real defects are
outside the file: the 500-file corpus it describes matches NO committed
manifest (485-file manifest overlaps only 423/500), and the 61 PCM MD5
digests cannot be re-executed in this sandbox (corpus and binary absent).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "counts": {"total": 500, "comparable": 35, "non_comparable": 369, "excluded": 96} | json:14-19 | REAL | `python3` Counter over rows: 500 = 35+369+96 exact; no dup rels |
| 2 | oracle "kind": "pending" … "no oracle binary in this environment — nothing mocked" | json:4-10 | REAL | all 500 rows have oracle_md5/oracle_bytes/oracle_error = null (checked programmatically; 0 non-null) |
| 3 | 35 comparable rows "with real Lumen decoded-PCM MD5 digests" (phase3-exit.md:35) | rows (all) | REAL-STRUCTURAL / UNVERIFIED-DIGESTS | 35 comparable rows all lumen.status=ok with md5+pcm_bytes; digests not re-executable: corpus-big/media absent (`ls /home/z/my-project/corpus-big/media` → 0), no built lumen binary |
| 4 | "each cross-verified by an independent Python RIFF walk + ffprobe byte cross-check 35/35" (docs/gates/phase3-exit.md:35) | cited doc | UNVERIFIED | ffprobe cross-check cannot be re-run here (no corpus files in sandbox); report JSON itself carries no ffprobe output (ffprobe_payload_bytes null everywhere) |
| 5 | lumen digests file schema "bench-vs-vlc/lumen-pcm-digests/1" | lumen.json:2 | REAL | cross-checked 500/500 rels: md5/status/error fields identical between .json rows[].lumen and .lumen.json results (python3; 0 mismatches) |
| 6 | lumen side ok/error split 61/439 | rows | REAL | Counter: ok=61, error=439; 61 = 35 comparable + 26 lossless-undumpable (15 pcm_s24le + 6 pcm_s32le + 5 garbage-decoded) |
| 7 | "corpus-big" 500-file regenerated corpus (phase3-exit.md:35) | external | FALSE (mismatch) | committed manifest corpus-big-manifest-20261022.json count=485; overlap with report rels only 423/500 (62 manifest files absent from report, 77 report files absent from manifest) |
| 8 | report covers "the 500-file regenerated corpus-big" | report header/counts | SHALLOW-name | no manifest documents a 500-file corpus (only 485-file and 126-file manifests exist in docs/corpus/) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| report rows = 500 files of corpus-big | corpus-big-manifest-20261022.json: count 485, files[] 485 | both files exist; they describe different corpora — the 500-row corpus has no committed manifest (evidence-chain gap, S3) |

## 4. Fabrication / inflation indicators
None: the file is the anti-fraud case study — oracle columns null, PENDING
labels explicit, "nothing mocked" stated, error rows retained (439 errors
committed verbatim, not hidden). Inflation risk lives in the GATE doc that
cites it (phase3-exit.md) as "differential harness operational … MET" while
zero differential comparisons exist; the gate doc's own parenthetical
"(operational; oracle cells PENDING-VLC)" discloses it.

## 5. Matrix rows
Not a matrix chunk. For the record: docs/feature-matrix.toml does not cite
this file as row evidence (`rg 'vlc-diff-report' docs/feature-matrix.toml`
→ no hits); it is cited by worklog.md, tests/compat, tools/bench-vs-vlc,
docs/PARITY-REPORT.md:157, docs/gates/phase3-exit.md:35,117.

Findings: F-001 [S3] evidence-chain gap — 500-row corpus undocumented by any
committed manifest (423/500 overlap with the 485-file manifest).
F-002 [S3] the "diff report" name promises a differential that contains zero
diffs (all oracle cells null) — honest inside, overselling outside.
F-003 [S4] ffprobe_payload_bytes field exists but is null in all 500 rows —
dead schema field.

Commands run: python3 json analysis (counts/status/md5 cross-check between
the two files; exit 0), sed -n spot checks, ls corpus-big (absent), rg
citation sweep. All digest VALUES: UNVERIFIED (inputs not in sandbox;
re-run needs corpus-big + built lumen binary — cargo-build-budget).
