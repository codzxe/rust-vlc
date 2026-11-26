# STALE TRANSCRIPTS — the committed evidence is stale (declared, not denied)

**Status: STALE** — every artifact under this directory
(`transcripts/*.json`, `final-frame.png`, `ledger.json` built from them)
was last regenerated 2026-11-15 and **no longer describes current
engine behavior**. Nothing in this directory should be cited as proof
of current decode counts, states, or wire shapes until the cure below
lands. This sidecar (FX8-A08, FIX-PLAN §4 F2.2) exists so the staleness
is a declared fact instead of a silent one — and so
`scripts/quality/refresh_transcripts.sh --check` fails loudly while it
is true.

## What the audit baseline proved (2026-11-23, HEAD of the audit branch)

From `audit/runs/baseline/cargo-test-workspace.txt`:

- `:2764` — `committed_transcripts_match_fresh_runs ... FAILED` — the
  committed `open` transcript drifted from a fresh run: the fresh run
  records `frames_accepted` 94 vs 30 committed (the audit's semantic
  diff: `.events` 8→9, `scheduler_pushed` 15→16). The committed
  transcripts understate current behavior by ~3×.
- `:2767` — `final_frame_is_current ... FAILED` — the committed
  `final-frame.png` drifted from a fresh run too.
- `:2793-2794` — the package then exited 101: cargo's default
  fail-fast **aborted the run before `tests/journey.rs`,
  `tests/recovery_matrix.rs` and `tests/wire_mode.rs` ever executed**
  (the baseline's "4744 passed / 2 failed" headline undercounts —
  three whole binaries never ran). Every cargo test invocation in the
  refresh procedure now carries `--no-fail-fast` for exactly this
  reason.

## Why the refresh has not landed yet

Fix agents in this wave are forbidden from running cargo
(`audit/fix/RULES.md` — one orchestrator compiles/tests centrally on
the shared 2-core box), so the writer — which needs cargo — is handed
to the central run unexecuted, with this sidecar as the tripwire.

Known state (the diagnostic central run of the three skipped binaries,
2026-11-26): `journey_pause`, `journey_stop`, `journey_seek` and
`matrix_mp4_trunc4` are currently RED (observed "Ended" vs expected
"Paused"/"Stopped"; 90 vs 154 pre-seek deliveries; "Playing" vs
"Error"); everything else in the package is green. Those four reds are
remedial FX8-R1's scope, not the transcript refresh's. **The writer's
refusal-on-red is working as designed** — it will keep refusing (as it
must: evidence only commits green runs) until those scenarios are
fixed.

## The cure (the central run)

```bash
bash scripts/quality/refresh_transcripts.sh          # the writer
bash scripts/quality/refresh_transcripts.sh --check   # then the gate
```

The writer: builds `lumen-cli` (the wire scenario spawns
`target/debug/lumen`), pre-flights the package's behavior binaries
(refusing to write on any non-green run), regenerates via the package's
own ignored test
(`cargo test -p first-vertical --test evidence -- --ignored regen`),
then re-runs the full package as the post-flight proof. **A green
post-flight retires this file** (the writer deletes it — "stale"
would stop being true the moment fresh, green artifacts land). The
ledger's `started_at`/`commit` fields will then name the refresh run.

Until then: `--check` exits 1 while this file exists, and any run of
`cargo test -p first-vertical --test evidence` reproduces the two
currency failures above.
