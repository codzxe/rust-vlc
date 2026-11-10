# W24-B soak lane — review findings (audit W24-R4 r2)

- **Auditor**: W24-R4 (wave 24 review scout 4, round 2), read-only audit of `origin/w24/b` (issue #54, card 8.C1 / P8-3 — soak & stress completion).
- **Audited diff**: `git diff origin/main...origin/w24/b` — 19 files, +4,529/−0: `tests/soak/**` (4 scenario manifests + JSON schema), `crates/playback-engine/tests/soak_scenarios/**` (main/manifest/report/runner), `docs/soak/**` (7 raw evidence reports + synthesis with the 5 MB/h FREEZE), `.github/workflows/soak-nightly.yml` (2 new jobs), `scripts/soak-scenarios.sh`. The shared 3.E4 harness (`tests/soak/{dispatch,memory,scenario}.rs`) is pre-existing on main and was read as the runner's dependency, not re-audited as diff.
- **Verdict**: **no P0 — the evidence is real and it reproduces.** Every raw report is internally consistent, the instruments are real (counting allocator + `/proc`, no fake sampling anywhere), and I re-ran both the CI-short suite (green) and the forced-threshold enforcement proof (red, exit ≠ 0) locally. The FREEZE decision follows from the recorded numbers. But **1 P1** (the report cites a verification-battery worklog entry that does not exist on the branch), **3 P2** (a dead report line from a wrong fold initializer, a wrong post-warmup span copied into the synthesis table, a dangling doc reference in the shipped schema) and **5 P3**.

**Severity counts: P0=0, P1=1, P2=3, P3=5 (total 9).**

---

## What I re-ran and re-computed (the audit's own evidence)

- `cargo test -p playback-engine --test soak_scenarios` (CI-short default profile): **29/29 PASS in 39.4 s**, all five scenarios inside budgets — matches the merge-gate claim (~1 min, `ci.yml` gate step 3 `cargo test --workspace --all-features`).
- Forced-threshold profile re-run (`--release`, verbatim env contract): **FAIL — 2 violations, non-zero exit**, live bytes `294,178 → 294,262` (+84 B vs the recorded +82 B — run-to-run variance, i.e. real measurement), VmRSS +0.33 MB (recorded 0.25 MB), measured span **849,815 cycles — reproduced exactly** (1,000,000 − 150,185). The enforcement path bites; the raw evidence is authentic.
- `cargo test -p playback-engine` (whole crate): green, incl. the pre-existing soak loop (12/12, 120.05 s).
- `cargo clippy -p playback-engine --all-targets --locked -- -D warnings`: CLEAN (workspace lints deny `unwrap_used`/`expect_used`/`panic`/`unsafe_code`; grep of the four new files finds zero unwrap/expect/panic/unsafe — only the test-binary idioms `unreachable!` + `testkit::Must`).
- `cargo fmt --check` (workspace): CLEAN. `cargo run -p xtask --locked -- lint-literals`: PASS, 0 violations / 452 allowlisted.
- `soak-nightly.yml`: PyYAML `safe_load` OK (3 jobs: `soak`, `soak-scenarios`, `soak-lsan`); diff vs main has **0 deleted lines** (purely additive — existing job, schedule `0 3 * * *`, `workflow_dispatch`, permissions, concurrency untouched); action pins uniform across jobs (checkout `3d3c42e`, dtolnay `d124f86`, rust-cache `6323deb`, upload-artifact `cf430e0`); all 7 `-p` package names in the LSAN job exist in the workspace; timeout arithmetic sound (3×48 min + 2×48 min caps = 240 min + build < 300 min cap).
- Sample-count arithmetic recomputed from the configs and matched against the raw reports: churn 2M → stride `ceil(2e6/4096)=489`, 4,089 multiples + final = **4,090 samples**, warmup index **613** (first tag ≥ 299,999 is 614×489 = 300,246) ✓; reopen 150k → stride 37, **4,055 samples**, warmup **608** ✓; forced 1M → stride 245, warmup **612** ✓; duration scenarios 482 samples / 73 warmup / 407.0 s window (8 min × 15% warmup) ✓×4.
- Slope unit conversions recomputed: +16.7 B/s ↔ 0.060 MB/h, −16.1 ↔ −0.058, +10.5 ↔ 0.038, −1,930.9 ↔ −6.951; RSS kB/s ↔ MB/h (×3600/1024) all match.
- Rate calibration recomputed: 2,000,000/216.4 s = **9,242 c/s**; 150,000/200.1 s = **749.6 c/s**; calibration 1,500/3.0 s = 500 c/s; nightly counts 24M → 43.3 min, 1.95M → 43.4 min at those rates ✓ (matches `scenarios-nightly.toml` comments).
- Cross-checks of authenticity: raw-report capture times run 17:54→19:23 UTC sequentially, each committed after its capture with no time travel (reopen report captured 19:14:10, committed 19:14:21); manifest paths point at the `w24b` worktree; env labels honest (`release, linux/x86_64`, with the briefing's "aarch64" corrected in §Environment); structural counts line up (seek-storm exactly 16 seeks/reopen epoch ↔ `SEEKS_PER_REOPEN`; reopen-stress exactly 11 frames/cycle at both 1.5k and 150k scales; ~62 KB/frame payload consistent across play-loop/playlist/reopen).
- FREEZE decision: worst duration-scenario live slope **0.060 MB/h = 83× under** the 5 MB/h target (5/0.060 = 83.3 ✓), worst RSS slope 0.403 MB/h = 12× under ✓; count scenarios 0.0 B/cycle live drift; RSS growth sublinear (204 kB @150k vs 252 kB @2M cycles). The decision follows, with the extrapolation caveats stated honestly (§3).
- Quarantine/regression-guard audit claims verified: `tests/quarantine/` absent; **55 `#[ignore]`** in `crates/`+`apps/` (exact); the `gap: … handoff H-1..H-11` family exists verbatim (11 markers); the named unit tests (`slope_verdict_flags_budget_violations`, `count_sample_stride_bounds_the_instrument_memory`) exist and pass. Soak-suite test count **29 = "28+1"** claim exact.
- Config-driven: all budget/target/duration literals in the runner/report live only in the `#[cfg(test)]` unit-test module; production paths read `Defaults`/`ScenarioSpec` from the TOML loader; unknown keys/types/ranges refuse loudly (11 loader tests); loader↔schema drift test present.

---

## P1 — fix before merge

### P1-1. The synthesis report cites a verification-battery worklog entry that does not exist on the branch

`docs/soak/soak-report-20261110.md:253-262` (§10) states:

> "Recorded verbatim in the W24-B worklog entry (repo-root `worklog.md`, appended with this round): `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test -p playback-engine` (scoped); `cargo test --workspace --all-features` (full, once, end of session); … **actual results in the worklog entry**."

Verified: `git diff origin/main...origin/w24/b -- worklog.md docs/worklog/` is **empty** — none of the 9 branch commits touched the log, and `worklog.md` on the branch contains no W24-B entry at all (it ends at the `7.w23f` entry; the w24b worktree itself is clean, so this is not an uncommitted leftover). AGENTS.md §4 ("Single shared log: `worklog.md` at the repository root (append-only)") makes that entry the mandated record, and the report explicitly points readers at it for the battery's *actual results* — which therefore exist nowhere. The last commit (`0acd8c4`, "synthesis report … W24-B r6") was the natural place and skipped it.

Impact is bounded — I re-ran the substantive battery in a fresh worktree and it is green (fmt/clippy/scoped tests/lint-literals/CI-short soak suite; see above) — but as shipped, the branch's central verification claim is a dangling citation, and the full-workspace `--all-features` run the report claims is not reproducible from any committed record.

**Fix**: append the W24-B entry to repo-root `worklog.md` (chronological work log + the battery's actual results, per the established per-task format), and repoint §10's "recorded verbatim" sentence at it truthfully. Do not rewrite the report's numbers — they check out.

---

## P2 — real, bounded

### P2-2. `report.rs:145-148` — wrong fold initializer makes the "Worst measured live-bytes slope" summary line dead code

```rust
let worst = results
    .iter()
    .filter_map(|result| result.verdict.live_mb_per_h)
    .fold(f64::INFINITY, f64::max);
if worst.is_finite() { /* print the worst-slope headline */ }
```

`f64::max(f64::INFINITY, x)` is `INFINITY` for every `x`, so `worst` is always infinite and `is_finite()` is always false: the "Worst measured live-bytes slope across scenarios: …" paragraph **never renders, for any input**. Empirically confirmed: a fresh single-scenario run with a defined slope (−0.058 MB/h) produces a report with zero occurrences of "Worst measured"; none of the seven committed raw reports contain the line either (consistent — the bug was present at run time, so the evidence is honest). Consequence at the tier that matters: the nightly 5-scenario report — the artifact the freeze-or-revise decision is re-made from — will never carry its own headline worst-slope sentence; the reader must re-derive it from the table.

**Fix**: `fold(f64::NEG_INFINITY, f64::max)`. One character-class change; the line then prints exactly what its doc comment promises (and would have printed "worst **0.060 MB/h**" for the sandbox evidence, matching §3's hand-computed claim).

### P2-3. `docs/soak/soak-report-20261110.md:62` — the 2M churn row carries the 1M forced-threshold run's post-warmup span

§2's count-scenario table row: `| open-close-churn | 849,815 post-warmup (of 2.0M) | …`. 849,815 is the **forced-threshold run's** measured span (1M cycles, baseline tag 150,185 — reproduced exactly in my re-run and printed in that run's own violation sentences, `scenarios-forced-threshold.md:34-35`). The **2M sandbox run's** post-warmup span is **1,699,754** (baseline sample tag 614×489 = 300,246 → final 2,000,000), or 1.7M nominal (2M − 15% warmup). The raw 2M report states no span (it passed, so no violation sentence carried it), and the synthesis filled the cell from the wrong file. The row's conclusion (+0.0 B/cycle live drift) is unaffected — 295,117 → 295,117 is exact — but a reviewer recomputing "849,815" from the cited raw evidence cannot, because it isn't there.

**Fix**: correct the cell to `1,699,754 post-warmup (of 2.0M)` (or `~1.70M`), and consider having the runner always record the measured span in the report (it already computes `measured_cycles_span`; it only surfaces in violation sentences and the drift mean).

### P2-4. `tests/soak/scenarios.schema.json:5` — $comment cites `docs/soak/regression-guards.md`, which does not exist

The schema's $comment (the human/tool-facing contract statement) says "…for review and future CI wiring (**see docs/soak/regression-guards.md for the wiring gap**)." No such file exists anywhere in the tree; the wiring-gap content actually lives in `docs/soak/soak-report-20261110.md` §6.2. The shipped-manifests drift test checks key/kind containment in the schema, so it will never catch a dangling prose reference. This is the lane's contract file pointing reviewers at a phantom document.

**Fix**: repoint the $comment at `docs/soak/soak-report-20261110.md §6.2` (or commit the referenced file — but a repoint is the honest one-line fix, since §6.2 already owns the content).

---

## P3 — nits, fix opportunistically

### P3-1. `runner.rs:771` — seek-storm never folds sink bytes: "6,214,216 frames, 0 payload bytes at the sinks (the D-6 liveness axis)"

`run_seek_storm` sets `frames_decoded` from the engine's lifetime counter but, unlike `play_loop_cycle` and the count scenarios, never folds `host.sink().bytes_total()` into `decoded_bytes` — so every seek-storm report (raw evidence, my re-run, and the nightly's) will print millions of frames next to "0 payload bytes … (the D-6 liveness axis)", which reads like a liveness failure to anyone skimming the D-6 line. `report.rs:240-244` prints both numbers side by side. **Fix**: fold the sink bytes at each close (or drop the payload clause for this kind); one `if let Some(sink)` mirror of `runner.rs:611-613`.

### P3-2. `docs/soak/soak-report-20261110.md:15` — Profile row "≈ 29.4 min" matches neither plan nor measurement

The raw walls sum to 3×480.0 s + 216.4 s + 200.1 s = 1,856.5 s = **30.9 min**; the sandbox manifest's own header plans "~35 minutes"; the synthesis says "≈ 29.4 min of real playback/churn". No derivation reproduces 29.4. Cosmetic (the per-scenario walls are all recorded correctly), but this lane's currency is exactly this kind of number. **Fix**: 30.9 min (measured) with the warmup-excluded windows noted if a smaller figure is wanted.

### P3-3. `docs/soak/raw/scenarios-sandbox__reopen-stress__calibration.md:22` — committed calibration report predates the 09ad85a sampling rework, unlabeled

The calibration report prints "225 warmup-excluded" at n=1500. The committed runner computes `warmup_samples = index of first sample tag ≥ warmup_cycles` = floor(1500×0.15) → tag 225 at index **224**; the pre-09ad85a code (index-based baseline, per-cycle untagged series) prints **225**. So this one evidence file was generated by the superseded runner — verbatim and authentic (its rate, 1,500 cycles / 3.0 s = 500 c/s, is version-independent and stays valid), but a reviewer recomputing from committed code gets an off-by-one and no explanation. **Fix**: add a one-line provenance note ("generated by the pre-09ad85a per-cycle sampling code") or regenerate.

### P3-4. `runner.rs:993-1020` — non-Linux count scenarios skip the VmRSS growth check silently while the fd check gets an honest-absence note

`read_vm_rss_kb()` returns `None` off-Linux; the count scenarios' RSS growth block is a silent `if let (Some, Some)`, and — unlike the fd instrument (which appends "the fd-count instrument is not available on this platform … NOT evaluated", `runner.rs:1012-1020`) — the absent VmRSS instrument produces no note. The duration scenarios do print "VmRSS slope: n/a". Minor honesty asymmetry on the platform the suite is explicitly designed to degrade gracefully on. **Fix**: mirror the fd note for `rss_baseline_kb.is_none()`.

### P3-5. Op-shape constants sit in code beside genuinely-configurable neighbors (`runner.rs:70-87`, `235-253`)

`SEEKS_PER_REOPEN = 16`, the relative-seek `delta_ms: 250`, `Percent { fraction: 0.4 }`, and `DECODE_EVIDENCE_POLLS = 64` are hardcoded while the adjacent `advance_polls` is a manifest key. Each carries a doc comment arguing it is a structural bound of the scenario's shape, not a duration/target/threshold — defensible, and `lint-literals` agrees — but the seek epoch length is exactly the knob a future scenario author will want per-profile (e.g. a stormier nightly). Same-file nit: the schema's `$comment` manifest list omits `scenarios-forced-threshold.toml`, which the loader and the drift test do cover. **Fix** (optional): promote `seeks_per_reopen` to an optional scenario key with default 16; add the forced-threshold profile to the `$comment` list.

---

## Honest-sampling statement (required)

Everything I report as *measured* was measured by me in the `wt/w24r4` worktree on this sandbox (linux/x86_64, rustc 1.99.1 per the repo pin): the CI-short suite run (29/29, 39.4 s), the forced-threshold re-run (release, 2 violations, exit ≠ 0, 137 s), the full playback-engine crate run, fmt/clippy/lint-literals, and the YAML parse. Everything else is *recomputation from committed artifacts* (slopes, strides, spans, rates, counts — shown above with the arithmetic) or *code reading*. I did **not** run: the 4 h nightly profile (sandbox wall caps — same blocker the lane records), any LSAN build (no nightly toolchain here), the GitHub workflow itself (billing-blocked repo-wide), or a non-Linux lane. The ~48 MB instrument-regression run described in §5 is narrated in the synthesis but its failing report is not committed (acknowledged in-commit as fixed in 09ad85a; the fix and its unit pin are verified).
