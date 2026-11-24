# CLAIM AUDIT: docs/corpus/compat-matrix-transitions-20261110.gap-triage.json
| Field | Value |
|---|---|
| Auditor | W19-A02 |
| Claims extracted | 1 transition + its numeric evidence |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 1 REAL-UNPROVEN (transition applied) / 0 / 0 / 0 / 0 / 1 FALSE number / 0 |

## 1. Blunt verdict
The transition itself is real and was actually applied (matrix row shows linux partial with
this text pasted into notes), but the hand-authored addendum — its own `origin` field admits
"NOT emitted by the runner — authored by the triage round" — transcribes its worst-row peak
wrong: it says 4.94e-1 where the cited measurement file says 5.42e-1. The error then
propagated verbatim into feature-matrix.toml's row notes. A wrong number copied into the
master ledger by a "rule-enforcing writer" that clearly does not check arithmetic.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | `"row": "codec-audio-pcm", "platform": "linux", "from": "implemented", "to": "partial"` | L9-12 | REAL-UNPROVEN (applied; run not re-executed) | feature-matrix.toml:634 `status = { windows = "implemented", linux = "partial", … }` + notes L636 carry the transition log verbatim |
| 2 | "ALL 6 pcm_f32le corpus rows FAIL the ratified f32 tolerance" | L13 | REAL | docs/corpus/vlc-f32-tolerance-20261109.json: 6 rows all FAIL (python3 re-check in that ledger); compat-matrix-20261110.md:29 "0 f32 tolerance-pass, 6 f32 tolerance-FAIL" |
| 3 | "every integer-PCM row (u8/s16, 29 rows) is bit-exact" | L13 | REAL (as recorded) | compat-matrix-20261110.md:29 "29 bit-exact (MD5-prefix)" (rg-verified); not re-measured (media absent) |
| 4 | "worst row 0305-avi-av.avi measures rms 1.22e-1 / peak 4.94e-1" | L13 | FALSE (peak) | tolerance JSON row 0305: `rms_dev 0.1209369799206826 peak_dev 0.5424160254187882` → peak is 5.42e-1, not 4.94e-1 (python3 printed both). rms 1.21e-1 vs claimed 1.22e-1 also rounds wrong. Same wrong pair propagates into feature-matrix.toml:636 notes |

## 3. Findings
F-001 [S3] — transcribed number contradicts its cited source.
Where: gap-triage.json L13 vs vlc-f32-tolerance-20261109.json row 0305; propagated to feature-matrix.toml:636.
What is wrong: peak 4.94e-1 vs actual 5.42e-1 (understates the worst deviation by ~9%);
rms rounding 1.22e-1 vs 1.21e-1. Why it matters: hand-authored "addendum" numbers enter
the master ledger unverified; the "same rule-enforcing writer" does no arithmetic check.
What correct looks like: emit-from-runner or recompute from the measurement file.
Confidence: HIGH — Verification: ran python3 on both files, printed both values.

## 4. Contradictions
| This file says | vlc-f32-tolerance json says | True |
|---|---|---|
| 0305 peak 4.94e-1 | 0305 peak_dev 0.5424160254187882 | measurement file |

## 5. Fabrication / inflation indicators
The transition direction is honest (a demotion implemented→partial backed by a failure
record — anti-inflation). The defect is sloppy transcription in a self-declared hand-authored
addendum, not status inflation.
