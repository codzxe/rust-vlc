# CLAIM AUDIT: docs/baselines/raw/linux/probe-work/pls/cue_sample.cue
| Field | Value |
|---|---|
| Auditor | W19-A08 |
| Claims extracted | 1 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 1 / 0 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
89-byte CUE fixture backing the pl_cue_sample probe in dump 17. Valid minimal CUE sheet.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Fixture is a real CUE sheet probing VLC's cue handling | whole file | REAL | `cat`: FILE "wav_pcm_py.wav" WAVE / TRACK 01 AUDIO / TITLE "Probe Tone" / INDEX 01 00:00:00 — structurally valid CUE; references the gitignored wav fixture (consistent with dump 17's "fixtures NOT committed" note) |

## 3-5. Contradictions / fabrication / matrix rows
None / none / N/A. Commands: wc -c (89); cat.
