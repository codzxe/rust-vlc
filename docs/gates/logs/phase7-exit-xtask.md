# TOMBSTONE — phase7-exit-xtask.log (deleted 2026-11-26, Devil-Fix FX8-A05)

The 959-line xtask gate-battery log that lived at this path
(`docs/gates/logs/phase7-exit-xtask.log` — header dated 2026-11-10,
worktree `/home/z/my-project/wt/w24f`, branch `w24/f` @ `a4e35e7`; all 7
gates `XTASK_EXIT[...]=0`) was deleted as stale-green gate evidence:

- the run was captured in the lane worktree `w24/f`, not the tree it was
  cited against: on the audited tree the orchestrator baseline re-runs the
  same battery RED (`audit/runs/baseline/full_gate.txt:42-75` —
  lint-literals FAIL 1 violation, check-identity FAIL 1 violation, sbom
  DRIFT, arch edges FAIL);
- the audited facts the log's numbers rested on no longer hold on HEAD
  (sbom 157 components vs the current 158; edge counts, 0-violation
  headlines — all w24f-era).

Audit evidence: `audit/files/docs/gates/logs/phase7-exit-xtask.log.audit.md`
(F-001 S2 stale vs HEAD, F-002 S3 allowlist-size headline, F-003 S4);
citing doc `docs/gates/phase7-exit.md` §2 rows 8-14.

## Regeneration (requires cargo — NOT run by FX8-A05; cargo runs are
orchestrator-central in Devil-Fix per `audit/fix/RULES.md`)

The exact seven commands the deleted log recorded in its own
`## Cmd:` blocks, in order:

    cargo run -q -p xtask -- check-architecture
    cargo run -q -p xtask -- codegen --check
    cargo run -q -p xtask -- schema-gen --check
    cargo run -q -p xtask -- fixtures --check
    cargo run -q -p xtask -- lint-literals
    cargo run -q -p xtask -- check-identity
    cargo run -q -p xtask -- sbom --check

A regenerated log must be captured from the current tree (not a lane
worktree) with a capture-time header, and committed green-or-red as run.

Citations of the deleted path that still stand in out-of-scope docs
(`docs/gates/phase7-exit.md` §2 rows 8-14; `docs/reviews/w24-s-chain-findings.md:240`;
`docs/reviews/w24-f-gate-findings.md:190`) are historical references to
the deleted snapshot (FX8-A05 Risks & followups).
