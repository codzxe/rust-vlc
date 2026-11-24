# DATA AUDIT: pls_sample.pls (6 loc)

Synthetic PLS playlist fixture for VLC probe work: [playlist]
File1=../media/wav_pcm_py.wav, Title1="Lumen probe tone", Length1=1,
NumberOfEntries=1, Version=2. Clean-room (own-generated; run_probes.sh
family). Verdict: REAL fixture. Findings: none. NOTE (S4 informational):
the playlist is never parsed by any shipped code — playlists crate
unreachable (W10) — the file proves the ORACLE side only.

| Auditor | W19-A04 (orchestrator completion, W19-S2) |
