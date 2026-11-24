# CLAIM AUDIT: docs/baselines/raw/linux/probe-work/dir/tone.wav (binary fixture)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 1 |
| REAL / ... / FALSE | 1 REAL |
| Internal contradictions found | 0 |

## 1. Blunt verdict
`file` -> RIFF WAVE audio, Microsoft PCM, 16 bit, mono 8000 Hz — exactly the format
gen_py.py declares for its synthetic 440 Hz probe tone. Generated CC0 test asset (not
acquired media, not VLC-derived); cited by docs/inventory/corpus-plan.md. Audio data
present (4 KB header+frames for 0.5 s). No doc claims this asset executes anything.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Valid minimal WAV probe fixture | binary | REAL | file(1) output above; generator provenance gen_py.py:5-10. |

## 3-4. Contradictions / fabrication indicators
None; none.
