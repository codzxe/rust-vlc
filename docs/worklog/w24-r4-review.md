# W24-R4 r2 — soak-lane review worklog (read-only audit of w24/b)

Task ID: W24-R4 r2 (review scout 4, round 2 — RESUME). Branch: `w24/r4`
(worktree `wt/w24r4`, based on `origin/w24/b` @ 0acd8c4; round 1 had made
no commits — this round performed the full audit and shipped the report).
Report: `docs/reviews/w24-b-soak-findings.md`. Read-only on the audited
code; my only writes are that report and this worklog.

## What I did (chronological)

1. Confirmed worktree state (`w24/r4` == `origin/w24/b` @ 0acd8c4, clean),
   fetched `origin/w24/b` + `origin/main`, mapped the diff: 19 files,
   +4,529/−0 — the whole 8.C1 soak lane (issue #54).
2. Read every audited file end to end: the 4 `tests/soak/*.toml`
   manifests + `scenarios.schema.json`; the 4 `soak_scenarios/*.rs`
   modules (main 606 / manifest 929 / report 448 / runner 1316 lines);
   `scripts/soak-scenarios.sh`; the 7 `docs/soak/raw/*.md` verbatim
   reports; `docs/soak/soak-report-20261110.md`; the `soak-nightly.yml`
   diff. Read the pre-existing shared harness (`tests/soak/dispatch.rs`,
   `memory.rs`, `scenario.rs`) as the runner's dependency — verified real
   instruments (counting allocator, `/proc/self/status`, `/proc/self/fd`),
   correct OLS slope math, real `playback_engine::Engine` dispatch, no
   mocks on the measured path.
3. Evidence-authenticity recomputation (all green, details in the report):
   sample counts/strides/warmup indices for every count run (4090/613,
   4055/608, 4082/612, 1500/225), duration windows (482/73/407 s ×4),
   slope unit conversions (B/s↔MB/h, kB/s↔MB/h), cycle rates (9,242 c/s,
   749.6 c/s, 500 c/s), nightly fill times (43.3/43.4 min), 83×/12×
   FREEZE margins, commit-vs-capture timestamps (no time travel), env
   labels (release/linux-x86_64, aarch64 corrected honestly), structural
   counters (16 seeks/epoch, 11 frames/cycle, ~62 kB/frame).
4. Executed the verification battery in the worktree:
   - CI-short soak suite: **29/29 PASS, 39.4 s** (five scenarios, all
     budgets held).
   - Forced-threshold profile, release: **FAIL — 2 violations, cargo
     exit ≠ 0**; span 849,815 reproduced exactly; +84 B live vs recorded
     +82 B (real run variance).
   - Full `cargo test -p playback-engine`: green (incl. old soak loop
     12/12 @ 120 s — no sibling breakage).
   - `cargo clippy -p playback-engine --all-targets -- -D warnings`
     CLEAN; workspace `cargo fmt --check` CLEAN; `cargo run -p xtask
     --locked -- lint-literals` PASS (0 violations / 452 allowlisted).
   - `python3 yaml.safe_load` on `soak-nightly.yml` OK; diff vs main has
     0 deleted lines (purely additive; schedule/permissions/concurrency
     untouched; pins uniform; LSAN `-p` packages all exist).
   - Claim recount: 29 = "28+1" tests exact; 55 `#[ignore]` exact;
     `tests/quarantine/` absent; `gap:` H-1..H-11 family present.
5. Found the findings (9 total): P1-1 missing W24-B worklog entry vs the
   report's §10 "recorded verbatim … actual results in the worklog entry"
   claim (worklog.md untouched by all 9 commits); P2 dead
   "Worst measured" fold (`f64::INFINITY` initializer — empirically
   confirmed on a fresh run); P2 wrong post-warmup span in the synthesis
   table (849,815 is the 1M forced run's, the 2M run's is 1,699,754);
   P2 dangling `docs/soak/regression-guards.md` reference in the schema
   $comment; P3 seek-storm "0 payload bytes" artifact, 29.4-vs-30.9 min
   drift, stale calibration report (pre-09ad85a warmup arithmetic),
   silent non-Linux VmRSS skip, hardcoded op-shape knobs. Wrote
   `docs/reviews/w24-b-soak-findings.md` with severity/file:line/
   evidence/fix and the required honest-sampling statement.
6. Committed both files on `w24/r4` and pushed to `origin`.

## D-10

- MEASURE — everything listed in steps 3-4 above, in-worktree, on the
  recorded sandbox (linux/x86_64, rustc 1.99.1).
- NOT TESTED — the 4 h nightly profile, LSAN builds, the GitHub
  workflow itself (billing-blocked), non-Linux lanes; the ~48 MB
  instrument-regression run is narrated-only in the synthesis (not
  committed as a raw report).
- WHAT BREAKS — the nightly report will never print its worst-slope
  headline until P2-2's one-token fix lands; a reviewer recomputing the
  synthesis table's churn span from raw evidence hits P2-3's wrong
  number.
- WHO HITS IT FIRST — the first real nightly run (billing unblock) for
  P2-2/P3-1; the next reviewer of the soak lane for P2-3/P2-4; the
  W24-B author (worklog append) for P1-1.
