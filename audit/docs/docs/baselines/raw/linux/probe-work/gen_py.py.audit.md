# CLAIM AUDIT: docs/baselines/raw/linux/probe-work/gen_py.py (108 lines — probe generator)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 2 (pure-python fixture generation; spec-valid outputs) |
| REAL / ... / FALSE | 2 REAL |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Real deterministic fixture generator: WAV (RIFF/WAVE PCM s16le 8 kHz, 440 Hz sine),
M3U/PLS/XSPF/B4S playlists — all built with struct.pack, no external tools. The committed
artifacts it generated are present in probe-work (tone.wav verified as exactly the
declared format: RIFF WAVE PCM 16-bit mono 8000 Hz; b4s_sample.b4s is well-formed
WinampXML). Cited by docs/inventory/{security-phase0,formats,corpus-plan}.md as probe
evidence.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Generate pure-python synthetic probe fixtures (no external tools)" | :2-3 | REAL | Source is stdlib-only (math, struct, os); no subprocess. |
| 2 | WAV spec: "canonical RIFF WAVE, PCM s16le, 0.5 s @ 8 kHz mono" | :5-10 | REAL — VERIFIED | file dir/tone.wav -> "RIFF (little-endian) data, WAVE audio, Microsoft PCM, 16 bit, mono 8000 Hz". |

## 3-4. Contradictions / fabrication indicators
None; none — generator + committed outputs match.
