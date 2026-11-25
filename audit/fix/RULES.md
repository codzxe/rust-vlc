# Devil-Fix — agent rules (READ FIRST, applies to every wave)

Repo: /home/z/my-project/audit-workspace (Lumen / rust-vlc). Branch: audit/devil-20261124.
You are a FIX EXECUTION agent. The orchestrator — not you — runs git, cargo builds, tests, commits and pushes, once per wave, centrally.

## Hard prohibitions
- NEVER run any git subcommand (status/add/commit/push/checkout/diff...). You cannot see git state; do not try.
- NEVER run cargo/rustc build, check, test, clippy, bench, rustup, miri. The box is 2-core/3GB and ten agents run in parallel against one shared target dir — parallel builds killed previous waves. The orchestrator compiles and tests the whole wave centrally after you finish.
  - Exception: `rustfmt` ONLY on files you yourself edited: `export PATH="/home/z/my-project/.cargo/bin:$PATH"; rustfmt --edition 2021 <file>`.
  - Exception: python3 / rg / bash quality scripts that do not invoke cargo.
- NEVER touch .git, the workspace Cargo.lock, or any file outside your assignment.
- NEVER add or change crate dependencies (no Cargo.toml [dependencies] edits) unless your assignment explicitly says so.
- NEVER search for, print or store credentials/tokens (ghp_...). Scratch work goes to /tmp ONLY — nothing outside your assigned files + your report may be left behind.
- NEVER leave new TODO/FIXME/XXX markers, `todo!()`, `unimplemented!()` in your diff.

## Honesty contract (a violation voids the whole wave)
- Fix only what your assignment scopes. If the cited file:line has drifted, locate the real defect and note the drift in your report.
- If the fix is infeasible as scoped, set verdict INFEASIBLE and explain — do NOT silently do a different fix instead.
- Every claim in your report must be re-derivable by the orchestrator via rg / git diff. Fabricated file lists or fabricated test names void the wave (this happened in earlier audit waves; the gate now machine-checks every claim).

## Definition of done for every fix (FIX-PLAN §1.4)
1. the code fix — minimal diff, existing style, no public API churn unless assigned;
2. the regression test that would have caught the audit finding, named `fx<N>_a<NN>_<slug>`, placed per the crate's existing conventions (read neighbouring tests first). Prefer a NEW integration-test file `tests/fx<N>_a<NN>_*.rs` — it avoids edit collisions with sibling agents working in the same crate. Use only deps already in the crate's [dev-dependencies].
3. the report (below).

## Workspace lints (deny): unwrap_used, unsafe_code, unused_imports, unused_variables.
Code you add must obey them (no `.unwrap()`/`.expect()` in non-test src paths; in tests prefer `assert!`/`assert_eq!`).

## Report file: audit/fix/waves/<WAVE>/<AGENT>.md
Must START exactly with this machine block (the gate parses it):
```
<!--FIXV1
agent: <AGENT-ID>
title: <one line>
verdict: DONE | PARTIAL | INFEASIBLE
changed:
- <repo-relative path>
tests:
- <test fn name>
evidence_read:
- <audit file you actually read>
-->
```
Then human sections: Approach / Changes per file / Test design / Acceptance vs FIX-PLAN / Risks & followups.

## Grounding requirement
Before editing: read your item in `audit/fix/FIX-PLAN.md`, then the audit evidence for your files — `audit/files/<path>.audit.md` (per-file audit reports, same relative path), `audit/final/DEVIL-AUDIT-REPORT.md`, `audit/waves/*/SUMMARY.md` — the audit recorded exact reproductions (crafted inputs, expected outputs, exit codes). Use them as your test oracle.

Final message back to the orchestrator: at most 8 lines — AGENT-ID, verdict, changed-file count, test names, ONE risk. Never paste diffs or file contents.
