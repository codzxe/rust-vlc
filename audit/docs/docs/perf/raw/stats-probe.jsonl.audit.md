# DATA AUDIT: docs/perf/raw/stats-probe.jsonl (2 records)

Captured control-stdio probes on avi_mjpeg_pcm.avi (live session
records with full stdout event traces):
Record 1: requested system.get-stats -> exit 64, error "unknown variant
`system.get-stats`" — the command is NOT in the 65-command catalog
(confirms Wave 2's catalog inventory; get-stats simply does not exist).
Record 2: requested system.get-state-snapshot -> typed
unsupported-for-host ("not in the first vertical's served set") —
CONFIRMS W02-A07's finding: no host serves get-state-snapshot.
Both records include the full 9-event state walk (idle->opening->
buffering->stopped->idle) and tracks.list — genuine wire captures.
Verdict: REAL evidence. Findings: S3:1 (informational) — file has no
consuming doc header; its significance (command-surface proof) is
unclaimed anywhere.

| Auditor | W19-A02 (orchestrator completion, W19-S2) |
