# TOMBSTONE — phase3-exit-fmt.log (deleted 2026-11-26, Devil-Fix FX8-A05)

The one-line exit marker that lived at this path
(`docs/gates/logs/phase3-exit-fmt.log` — its entire content was
`FMT_RECHECK_EXIT:0`) was deleted as stale-green gate evidence:

- content-free: no command line, no timestamp, no worktree, no check
  output — a single exit token is indistinguishable from a typed line
  (contrast the honest sibling `phase3-exit-fmt-initial-drift.log`, which
  carries the full 32-diff drift record and `FMT_EXIT_CODE:1` — KEPT);
- stale vs the current tree: the orchestrator baseline
  (`audit/runs/baseline/cargo-fmt.txt`) records `cargo fmt --check`
  FAILED (exit 1, real diffs) on the audited tree, so the "post-fix
  re-check clean" claim this log certified did not survive to HEAD.

Audit evidence: `audit/files/docs/gates/logs/phase3-exit-fmt.log.audit.md`
(F-001 S4 content-free, F-002 S4 stale-vs-HEAD); citing doc
`docs/gates/phase3-exit.md` §2 row 3 + §8 — §8 now carries the tombstone
disclosure (FX8-A05).

## Regeneration (requires cargo — NOT run by FX8-A05; cargo runs are
orchestrator-central in Devil-Fix per `audit/fix/RULES.md`)

    cargo fmt --all && cargo fmt --check; echo "FMT_RECHECK_EXIT:$?"

(the post-fix re-check form the citing `docs/gates/phase3-exit.md` §2 row 3
records for this log). A regenerated log must carry the full check output
plus provenance (worktree/commit, date, command, exit code) and be
committed green-or-red as run.
