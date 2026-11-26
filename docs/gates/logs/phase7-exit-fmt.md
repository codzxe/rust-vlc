# TOMBSTONE — phase7-exit-fmt.log (deleted 2026-11-26, Devil-Fix FX8-A05)

The five-line fmt gate log that lived at this path
(`docs/gates/logs/phase7-exit-fmt.log` — header dated 2026-11-10, worktree
`/home/z/my-project/wt/w24f`, branch `w24/f` @ `a4e35e7`, then
`FMT_EXIT=0`) was deleted as stale-green gate evidence:

- fmt was clean at `a4e35e7` (2026-11-10), but formatting drift landed
  after the gate was cut and no committed fmt log reflected it: the
  orchestrator baseline on the audited tree (`audit/runs/baseline/cargo-fmt.txt`)
  records `cargo fmt --check` FAILED (exit 1, 2 real `Diff in` sections,
  incl. `crates/plugin-api/tests/testkit_wiring.rs`);
- the gate doc's own instruction `cargo fmt --check  # expect 0`
  (`docs/gates/phase7-exit.md` §8) now yields exit 1 — the committed
  evidence lane, not the live CI lane, is what was stale.

Audit evidence: `audit/files/docs/gates/logs/phase7-exit-fmt.log.audit.md`
(F-001 S3 — stale-snapshot class, contradicts current-tree reality);
`audit/final/CONTRADICTIONS.md` (row A: the committed `FMT_EXIT=0` vs
current-tree fmt failure); citing doc `docs/gates/phase7-exit.md` §2 row 1.

## Regeneration (requires cargo — NOT run by FX8-A05; cargo runs are
orchestrator-central in Devil-Fix per `audit/fix/RULES.md`)

    cargo fmt --check; echo "FMT_EXIT:$?"

(the exact `# Cmd:` the deleted log recorded in its own header). A
regenerated log must be captured from the current tree, not a lane
worktree, and committed green-or-red as run.

Citation of the deleted path that still stands in the out-of-scope gate
doc (`docs/gates/phase7-exit.md` §2 row 1 "PASS (exit 0, no diffs)") is a
historical reference to the deleted snapshot (FX8-A05 Risks & followups).
