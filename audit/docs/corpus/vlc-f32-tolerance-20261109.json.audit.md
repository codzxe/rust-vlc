# CLAIM AUDIT: docs/corpus/vlc-f32-tolerance-20261109.json
| Field | Value |
|---|---|
| Auditor | W19-A02 |
| Claims extracted | 6 row verdicts + policy + summary |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 1 REAL-UNPROVEN (record as a whole) / 0 / 0 / 0 / 0 / 0 / 1 (measurement re-run) |

## 1. Blunt verdict
Honest failure record: 6/6 rows FAIL against a ratified f32 tolerance policy, and the
verdict arithmetic is internally consistent (re-checked by script). The measuring tool
exists in-tree; the media it measured does not (external worktree path), so the numbers
themselves are UNVERIFIED, but the record reports failure rather than hiding it — the
opposite of inflation.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | policy: "PASS iff per-row RMS deviation <= 1e-4 … AND peak deviation <= 1e-3" | L6 | REAL-UNPROVEN (policy text) | decision ratified per record; no doc-10 §6.6 check performed here (doc 10 not in my set) |
| 2 | six rows, each "verdict": "FAIL" | rows[] | REAL (arithmetic) | I re-ran the policy comparison in python3 over all 6 rows: expected FAIL vs recorded FAIL — 6/6 match; summary "pass": 0, "fail": 6 consistent with rows |
| 3 | comparator backing (RMS/peak over f32le) | implied by schema | REAL | tools/corpus/fidelity.py:15,94-143 implements interleaved-PCM RMS/peak incl. f32le branch (rg-verified) |
| 4 | row 0330 edge case: peak 0.0011998 vs peak_max 0.001 → FAIL | L76-86 | REAL | arithmetic re-check: 0.0011998 > 0.001 → FAIL correct (no rounding-up to PASS) |

## 3. Contradictions
None found. Note the honest detail: `lumen_bytes` exceeds `vlc_bytes` on every row
(e.g. 0022: 557336 vs 556976) — sample-count drift is visible in the raw fields, not
normalized away.

## 4. Fabrication / inflation indicators
None: a fabricated record would show PASSes. All six lanes fail, including one (0297)
whose RMS passes but peak fails — the AND-condition is applied strictly. Measurement
re-execution UNVERIFIED (media/ files absent from repo; corpus-v4 manifest paths point
to a nonexistent worktree).
