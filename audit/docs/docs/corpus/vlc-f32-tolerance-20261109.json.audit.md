# DATA AUDIT: docs/corpus/vlc-f32-tolerance-20261109.json (credit: W19-A02, recovered by W19-R5)
| Field | Value |
|---|---|
| Auditor | W19-A02 (recovered by W19-R5) |
| Date | 2026-11-24 |
| Kind | Tolerance decision record + 6 f32 measurement rows (schema corpus/vlc-f32-tolerance/1, issue 19) |
| Internal consistency | python3 re-validation: all 6 verdicts consistent with the stated policy; byte arithmetic exact |

## 1. Blunt verdict
Internally exact and externally corroborated. Every verdict recomputes under the stated
policy (PASS iff rms≤1e-4 AND peak≤1e-3): 0 pass / 6 fail, matching the summary block. The
byte arithmetic is exact per row (samples_compared × 4 == vlc_bytes; lumen_bytes exceeds it
by 84–424 samples — the disclosed lumen-overrun). 3 of 6 magnitude pairs (0022, 0297, 0330)
reproduce digit-exact in the later compat-matrix-20261110.json; the other 3 (0247, 0280,
0305) differ — and the companion vlc-diff-report discloses precisely why (they are the
runtime-random `anoisesrc` fixtures that cannot regenerate 2026-11-09 bytes). The corpus
media itself is external, so payload_md5s are not re-checkable here.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Policy: PASS iff RMS ≤ 1e-4 AND peak ≤ 1e-3 | :6 | REAL | python3: re-evaluated all 6 rows against the policy — every FAIL row violates at least one bound (0297 and 0330 pass RMS but fail peak: 0.0462/0.0012 > 1e-3); zero mismatches. |
| 2 | 6 rows, summary pass 0 / fail 6 | :89-92 | REAL | census: 6 rows; recomputed expected verdicts = 6 FAIL — matches. |
| 3 | Row arithmetic: samples vs bytes | rows | REAL | samples_compared×4 == vlc_bytes EXACT on all 6 rows (e.g. 0022: 139244×4 = 556976); lumen_bytes−vlc_bytes = 360/336/1216/1696/784/1360 bytes (=90/84/304/424/196/340 samples) — lumen emitted extra samples on every fixture, direction disclosed by the companion report. |
| 4 | Magnitudes reproducible | rows | REAL (3 exact) / REAL-UNPROVEN (3 random) | 0022 (0.01252049830383012), 0297 (8.215050255268216e-05), 0330 (6.425152069055904e-06) digit-identical in compat-matrix-20261110.json; 0247/0280/0305 differ there (0.0123417 vs 0.0123756 etc.) — vlc-diff-report-20261023.md discloses these three as unseeded `anoisesrc` runtime-random fixtures. |
| 5 | payload_md5 asserted == Lumen harness digests 6/6 | rows/:12-13 | REAL-UNPROVEN | corpus media lives in an external worktree (absent from repo); md5 re-check impossible — asserted by vlc-diff-report ("RIFF-walk payload MD5s were asserted equal ... 6/6"). |
| 6 | Ratified 2026-11-09 Lead decision; schema referenced | :2-5 | REAL | schema id + file cited in vlc-diff-report-20261023.md as the evidence record; issue 19 field present. |

## 3. Contradictions
None. The apparent 3-row magnitude drift vs the 2026-11-10 compat-matrix is fully explained
by the disclosed random-fixture classification — the deterministic rows reproduce exactly.

Commands run: python3 json validation (policy re-evaluation, byte arithmetic, deltas);
rg "corpus/vlc-f32-tolerance" + "0297-avi-av" across docs/corpus; python3 f32-row extraction
from compat-matrix-20261110.json (6 rows); read of vlc-diff-report-20261023.md measurement
basis paragraph.
