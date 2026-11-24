# DATA AUDIT: docs/baselines/raw/linux/probe-work/pls/xspf_sample.xspf (credit: W19-A07, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Content | XSPF 1.0 playlist, 1 track, 303 bytes, ASCII |
| Cited by | corpus-plan.md:357 (sha256 + "absolute file:// entry"), run_probes.sh:32, 17-format-probes.txt:1048 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## Verdict
A working probe fixture with full provenance chain, all verified:
1. sha256 recomputed: `d281ad9f56d8a4bf6f4473d15b2d6da3a7051525d8306d6085cf65d15a2e6e49`
   == corpus-plan.md:357's recorded hash EXACTLY; size 303 B == manifest.tsv entry ✓.
2. Generator: docs/baselines/raw/linux/probe-work/gen_py.py:25 writes this exact content
   (title "Lumen probe", track "probe tone", location file://{M}/media/wav_pcm_py.wav) ✓.
3. Use: run_probes.sh:32 loops it through the pinned VLC; the recorded result
   (17-format-probes.txt:1048-1058) shows exit code 0 and "playlist stream debug: using
   XSPF playlist reader" — the fixture demonstrably exercised VLC's XSPF reader on the
   pinned binary ✓.
4. XSPF grammar: `version="1" xmlns="http://xspf.org/ns/0/"` — spec-conformant.
S4 only: the track location is an absolute `file:///home/z/my-project/tmp/probe/media/
wav_pcm_py.wav` — a machine-specific path outside the repo (media no longer present), so
the fixture is inert as committed evidence; corpus-plan.md:357 discloses this ("absolute
file:// entry"). Quotes byte-verbatim from Read output + sed of gen_py.py:25-29.
