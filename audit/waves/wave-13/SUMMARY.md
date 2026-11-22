# Wave 13 SUMMARY

Agent logs: 3 — W13-A01.log.md, W13-A02.log.md, W13-A03.log.md

## Severity counts (from audit files)
S0: 3 | S1: 0 | S2: 25 | S3: 46 | S4: 90

## Worst 10 findings (by severity, then file)
- [S0] audit/files/apps/cli/src/bench.rs.audit.md: F-001 [S0] — "no decoder in this build" is a false claim about the build; the bench's private allowlist refuses codecs that have decoders
- [S0] audit/files/apps/cli/src/run.rs.audit.md: F-001 [S0] — the interactive player never advances time: no wall-clock pump, `advance` is the only time mover
- [S0] audit/files/apps/cli/tests/sync_scenarios.rs.audit.md: F-001 [S0] — module doc claims the A/V tolerance "holds at every present"; the only test that checks it measured a breach and is ignored
- [S2] audit/files/apps/cli/src/bench.rs.audit.md: F-002 [S2] — The runner measures 2 codecs but presents as the corpus speed-parity benchmark
- [S2] audit/files/apps/cli/src/containers/mkv.rs.audit.md: F-001 [S2] — Module doc and corpus test claim h264 typed-refusal; the shipped provider dispatches an H264 software decoder
- [S2] audit/files/apps/cli/src/containers/mkv.rs.audit.md: F-002 [S2] — The corpus differential self-skips as PASS when the corpus is absent, and no baseline run ever executed it
- [S2] audit/files/apps/cli/src/containers/mp4.rs.audit.md: F-001 [S2] — Stale "no software h264 decoder" doc + corpus expectations contradicted by the shipped provider
- [S2] audit/files/apps/cli/src/containers/mp4.rs.audit.md: F-002 [S2] — Corpus differential self-skips as PASS; no baseline run ever executed it
- [S2] audit/files/apps/cli/src/containers/ogg.rs.audit.md: F-003 [S2] — Corpus differential skip-as-pass, no recorded run
- [S2] audit/files/apps/cli/src/containers/ps.rs.audit.md: F-001 [S2]

Gate: PASS. Key S0s: dead pump_wallclock (host.rs:523), pacing flag discarded (run.rs:219), bench.rs:334 false 'no decoder in this build' while pipeline_host dispatches 14 codecs.
