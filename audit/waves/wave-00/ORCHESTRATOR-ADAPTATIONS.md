# Orchestrator adaptations (recorded per honesty rules)

Constraints discovered at Wave 0: 2 vCPU, 3.9 GB RAM, 9.9 GB disk (94% full at
one point), background processes do not survive between orchestrator shell calls.

Adaptations, all recorded here and in each wave SUMMARY:
1. Agent counts per wave consolidated (3-10 instead of fixed 10) — see
   assignments.tsv. All 20 wave domains preserved; every text file assigned
   exactly once (coverage proof in assignments.tsv + manifest-stats.md).
2. Sub-agents do NOT run git (no commit/push) and do NOT run cargo/npm:
   on this host, concurrent cargo would thrash the 2-vCPU box and contend on
   target/ locks. The orchestrator commits/pushes after gating each wave.
   Run-dependent verification comes from the Wave 0 baseline (real outputs in
   audit/runs/baseline/) or is marked UNVERIFIED (tool unavailable in sandbox).
3. Files >800 LOC (199 of them) are chunked at function boundaries by their
   assigned agent inside one audit file, per §7.1.
4. Binary/media assets (798 files: png/svg/mp4/avi/mkv/...) are inventoried in
   manifest.tsv (tier X) but not line-audited; agents verify their existence,
   size, and — where they are test fixtures — whether tests actually consume
   them.
5. clippy --workspace --all-targets and UI npm suites not yet run at Wave 0
   (disk/time); scheduled between waves. Recorded as gaps, not skipped silently.
