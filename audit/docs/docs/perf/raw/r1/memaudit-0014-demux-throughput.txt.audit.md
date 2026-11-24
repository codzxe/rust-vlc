# DATA AUDIT: docs/perf/raw/r1/memaudit-0014-demux-throughput.txt (8 loc)

Committed MEMAUDIT raw record (card 5.R1 step 5): mkv demux throughput
on 0014-muxed-av.mkv — rep1 0.0068s, rep6 0.0306s, derived per-walk
4.77ms -> 53,494 pkt/s, 773.2 MiB/s demux; walk_alloc_B=3,930,502 over
28 calls (B_per_pkt=15,413.73); v_dec=0/a_dec=0 (demux-only walk, honest
— no decode claimed). Verdict: REAL raw evidence; internally consistent
arithmetic (0.0306-0.0068)/5 = 0.00476s. Findings: none. Consumers:
docs/perf/ reports cite the r1 family.

| Auditor | W19-A09 (orchestrator completion, W19-S2) |
