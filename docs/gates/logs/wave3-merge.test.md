# TOMBSTONE — wave3-merge.test.log (deleted 2026-11-26, Devil-Fix FX8-A05)

The cargo-test snapshot that lived at this path
(`docs/gates/logs/wave3-merge.test.log`, 2,626 lines) was deleted as
stale-green gate evidence. Its tally — **1718 passed / 0 failed / 20
ignored** over 121 test binaries — was internally consistent, but it
describes the tree at wave-3 merge time, and on the audited tree the same
suites are RED:

- the two first-vertical evidence tests this log records as `ok`
  (`final_frame_is_current`, `committed_transcripts_match_fresh_runs` —
  log lines 803-808) FAILED in the orchestrator baseline
  (`audit/runs/baseline/cargo-test-workspace.txt:2764,2767`), which
  records **4744 passed / 2 failed / 51 ignored** for the same workspace
  command;
- the log carried **no provenance header** — no command line, no date, no
  commit, no toolchain version — so nothing in it could date or invalidate
  itself (exactly how the drift went unnoticed).

Audit evidence: `audit/files/docs/gates/logs/wave3-merge.test.log.audit.md`
(F-001 S3 stale evidence, F-002 S4 no provenance);
`audit/final/DEVIL-AUDIT-REPORT.md` §3 finding 15 and §4 #2
("Stale-green evidence"); `audit/final/CONTRADICTIONS.md` (stale-green
class).

## Regeneration (requires cargo — NOT run by FX8-A05; cargo runs are
orchestrator-central in Devil-Fix per `audit/fix/RULES.md`)

    cargo test --workspace --all-features 2>&1 | tee docs/gates/logs/wave3-merge.test.log

Command per the citing `docs/gates/phase3-exit-draft.md` §4 row 4 — the
deleted log did not record its own command line (F-002). A regenerated log
must carry a provenance header (worktree/commit, date, command, exit code)
and be committed green-or-red as run: a red result is committed as-is (the
honest-FAIL standard of the kept sibling logs, e.g.
`phase3-exit-fmt-initial-drift.log`, `phase3-exit-workspace-test.log`).

Citations of the deleted path that still stand in out-of-scope docs
(`docs/gates/phase3-exit-draft.md:110`, `docs/gates/phase3-exit.md:83-84`
trajectory + §8, `docs/audits/error-surface-20261023.md`,
`docs/worklog/archive-2026-10.md:7694`) are historical references to the
deleted snapshot; `docs/gates/phase3-exit.md` §8 discloses this tombstone
(FX8-A05).
