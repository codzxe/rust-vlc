# CLAIM AUDIT: docs/baselines/raw/linux/17-format-probes.txt (3 parts, L1-1664)
| Field | Value |
|---|---|
| Auditor | W19-A08 |
| Claims extracted | 7 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 5 / 2 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Genuine consolidated format-probe session log against the pinned VLC 3.0.23 binary, with
preserved harness (probe-work/run_probes.sh + gen_py.py + playlist fixtures) and honest
negative results. Two claims are UNVERIFIED BY ITS OWN ADMISSION (ffmpeg synthesis command
lines not preserved; ac3/dts actual-decode not exercised) — the file flags both itself.
Chunk ranges L1-782, 783-1581, 1582-1664 all inside the real 1664-line file.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Oracle: pinned VLC 3.0.23 Vetinari ... Clean-room" | L2-5 | REAL | header provenance; fixtures in probe-work/ verified present (gen_py.py, run_probes.sh, pls/) |
| 2 | "the exact ffmpeg command lines were NOT preserved by the prior 0.B4 session [VERIFY]" | L10 | REAL (honest admission) | verbatim L10 — self-flagged gap |
| 3 | "probes with module-selection evidence: 58 of 61 logs" | L1660-ish tail | REAL-UNPROVEN | logs live in tmp/ (gitignored) — cannot recount; claim is the file's own reconciliation |
| 4 | "midi_minimal ('Codec `MIDI' is not supported' ...)" | tail | REAL | verbatim negative-result section present in tail (sed-verified) |
| 5 | "pl_cue_sample (no cue reader -> 'nothing to play')" | tail | REAL | probe-work/pls/cue_sample.cue exists (89 bytes, valid CUE sheet: FILE/TRACK/INDEX) |
| 6 | "ac3/eac3/dts raw ... actual sample decode not exercised by these runs [VERIFY]" | tail | REAL (honest admission) | verbatim — self-flagged |
| 7 | "Media fixtures are NOT committed (gitignored tmp/)" | L11 | REAL | tmp/ not present in repo tree (LS spot check) |

## 3-4. Contradictions / fabrication indicators
None. Negative results are preserved, which is the opposite of inflation.
## 5. Matrix rows
N/A — evidence baseline feeding format matrix rows (other agents' scope).
Commands: sed -n '1,14p'; tail -6; ls probe-work/; cat probe-work/pls/cue_sample.cue.
