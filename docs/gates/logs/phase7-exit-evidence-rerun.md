# TOMBSTONE — phase7-exit-evidence-rerun.log (deleted 2026-11-26, Devil-Fix FX8-A05)

The 23-line first-vertical evidence-suite log that lived at this path
(`docs/gates/logs/phase7-exit-evidence-rerun.log` — header dated 2026-11-10,
worktree `/home/z/my-project/wt/w24f`, `a4e35e7`; body: 3 passed / 0 failed
/ 1 ignored `regen`, with `final_frame_is_current ... ok` and
`committed_transcripts_match_fresh_runs ... ok`) was deleted as stale-green
gate evidence:

- both "ok" tests FAILED on the audited tree
  (`audit/runs/baseline/cargo-test-workspace.txt:2764-2767`: the same four
  test names, `final_frame_is_current` and
  `committed_transcripts_match_fresh_runs` FAILED) — the committed
  evidence transcripts rotted (baseline stdout: committed `open`
  transcript missing event seq 8 `progress "0.00"`, event seq 10,
  `frames_accepted` 30 vs 94);
- the log's own header honestly admitted its exit code was never captured
  ("an inference from the body, not a recorded value") — but nothing in
  the citation chain (`docs/gates/phase7-exit.md` §2 row 5 "**PASS**")
  carried a validity date, so the exit-gate table presented 2026-11-10
  greens as current truth while the suite is red today.

Audit evidence: `audit/files/docs/gates/logs/phase7-exit-evidence-rerun.log.audit.md`
(F-001 S2 stale green for a gate that is red today, F-002 S4
inferred-not-recorded exit); `audit/final/DEVIL-AUDIT-REPORT.md` §3
finding 20 (transcripts stale at HEAD). The transcript refresh itself is
owned by FX8-A08 (same wave: `refresh_transcripts.sh` writer refuses-on-red
+ STALE-TRANSCRIPTS.md sidecar).

## Regeneration (requires cargo — NOT run by FX8-A05; cargo runs are
orchestrator-central in Devil-Fix per `audit/fix/RULES.md`)

    cargo test -p first-vertical --test evidence 2>&1 | tee docs/gates/logs/phase7-exit-evidence-rerun.log

(the exact `# Cmd:` the deleted log recorded in its own header). A
regenerated log must be captured from the current tree with a
capture-time header recording the exit code, and committed green-or-red as
run.

Citation of the deleted path that still stands in the out-of-scope gate
doc (`docs/gates/phase7-exit.md` §2 row 5 "PASS (3 passed / 1
ignored-by-design `regen`)") is a historical reference to the deleted
snapshot (FX8-A05 Risks & followups).
