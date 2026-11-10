# Soak & stress completion — the bounded measurement, freeze decision, and audit (8.C1, issue #54)

Status: **the five-scenario bounded soak RAN for real, in-sandbox, all
green; the 5 MB/h design target is FROZEN on the measured numbers; the
nightly 4 h tier and the LSAN job are wired and billing-blocked.** Every
number below is from a real run whose verbatim report is committed under
`docs/soak/raw/`. What did NOT run is labeled in §8 (blocked).

| Field | Value |
|---|---|
| Card | 8.C1 (P8-3, issue #54; branch `w24/b`) |
| Captured | 2026-11-10, single sandbox, UTC |
| Environment | **Linux x86_64** container (`uname -m` = `x86_64`), 2 vCPU, 4,041 MB RAM, release build, rustc 1.99.1 (the repo pin). Honest label: the wave briefing called this sandbox "aarch64" — the machine is x86_64; every report here records `linux/x86_64`. No aarch64 measurement exists (§8). |
| Harness | `crates/playback-engine/tests/soak_scenarios/` — the config-driven scenario suite (config plane: `tests/soak/*.toml`; contract: `tests/soak/scenarios.schema.json`) |
| Profile | `tests/soak/scenarios-sandbox.toml` — the bounded tier: 3 × 8 min duration-driven + 2 count-driven scenarios calibrated to ~5 min (§5) ≈ 29.4 min of real playback/churn |
| Invocation | `scripts/soak-scenarios.sh --only <id> --release` per scenario (the sandbox's tool-call wall caps force one process per scenario, each a clean allocator baseline — the `SOAK_SCENARIOS_ONLY` chunking knob; CI and the nightly never set it) |
| Heritage | Supersedes the issue-#18 single-loop plumbing report (`docs/perf/soak-nightly-20261109.md`, whose only real number was the CI-short 2 min debug run) with five scenarios of measured release evidence |
| Raw evidence | `docs/soak/raw/scenarios-sandbox__{play-loop,seek-storm,playlist-cycle,open-close-churn,reopen-stress}.md` (one verbatim suite report per scenario), `…__reopen-stress__calibration.md`, `scenarios-forced-threshold.md` |

## 1. Method

Every op is a catalog `WireRequest` through the soak dispatch seam onto
the real engine stack — access → AVI demux → MJPEG/PCM decode →
counting sink — against the committed `tests/fixtures/` media. Fixtures'
seek targets and track facts are read from their recorded expectation
TOMLs, never transcribed. Malformed fixtures (`trunc3`, `noidx0`) rotate
in on purpose (the dirty-world discipline, D-2/D-4).

Instruments (all cross-platform-honest, `cfg(target_os)` sampled):

- `live_bytes` — the process-global counting allocator: the precise
  leak signal (allocs − frees, live at sample time).
- `VmRSS` — `/proc/self/status` (Linux; honestly `n/a` elsewhere).
- fd count — `/proc/self/fd` (count-driven scenarios, Linux only).

Warmup: the first 15% of each scenario is excluded from the fits (pool
sizing, allocator steady state); the series keeps the samples. Slopes:
least squares over the measured window. **Every MB/h figure below is an
EXTRAPOLATION from a ≤407 s window, not a 4 h measurement** — the 4 h
nightly tier (§7) is where the frozen target gets re-asserted at scale.

## 2. The measured numbers (release, linux/x86_64, 2026-11-10)

| scenario | wall | ops | live slope (MB/h) | RSS slope (MB/h) | verdict |
|---|---|---|---|---|---|
| play-loop | 480.0 s | 73,834 open→play→close cycles | **−0.058** (−16.1 B/s) | −0.130 | PASS |
| seek-storm | 480.0 s | 15,395 cycles, **246,309 seeks** (0 failed) | **+0.060** (+16.7 B/s) | +0.162 | PASS |
| playlist-cycle | 480.0 s | 66,886 cycles, 22,295 playlist passes | **+0.038** (+10.5 B/s) | +0.403 | PASS |
| open-close-churn | 216.4 s | **2,000,000** open→close cycles | growth-budgeted (below) | growth-budgeted | PASS |
| reopen-stress | 200.1 s | **150,000** full open→play→decode→close | growth-budgeted (below) | growth-budgeted | PASS |

Duration scenarios: 482 samples each, 73 warmup-excluded, measured
window 407.0 s. Liveness (D-6): play-loop decoded 5,439,151 frames /
337.2 GB payload; seek-storm 6,214,216 frames; playlist-cycle 6,242,720
frames / 383.9 GB; reopen-stress 1,650,000 frames / 103.9 GB. Zero
error-lane opens/closes across all five scenarios; zero failed seeks.

Count scenarios (growth budgets, post-warmup baselines):

| scenario | cycles measured | live bytes | VmRSS | open fds |
|---|---|---|---|---|
| open-close-churn | 849,815 post-warmup (of 2.0M) | 295,117 → 295,117 B (**+0.0 B/cycle**) | 1,720 → 1,972 kB (+252 kB; budget 8 MB) | 4 → 4 (max 4; budget 4) |
| reopen-stress | 127,500 post-warmup (of 150k) | 292,783 → 292,783 B (**+0.0 B/cycle**) | 2,296 → 2,500 kB (+204 kB; budget 8 MB) | 4 → 4 (max 4; budget 4) |

The counting allocator sees **zero net live-byte drift per cycle** at
both scales — the strongest available no-leak signal for the
session-lifecycle path. VmRSS moves ~¼ MB per scenario (kernel-level
arena behavior, monotone-ish but bounded), 30× under its budget.

## 3. Memory slopes vs the 5 MB/h target — FREEZE

The design target (doc 10 §6.5: slope "< 5 MB/h [PROPOSAL — number to
be justified by first measured baseline, then frozen]") comes from
config (`target_mb_per_h = 5.0`), never code. The bounded run is that
first measured baseline:

- Worst live-bytes slope across the duration scenarios:
  **+0.060 MB/h** (seek-storm) — **83× below** the 5 MB/h target.
- Worst RSS slope: **+0.403 MB/h** (playlist-cycle) — 12× below.
- Count scenarios: 0.0 B/cycle live drift; RSS growth sublinear in
  cycles (252 kB at 2M cycles; the 150k→2M cycle scaling did not
  scale the growth).

**Decision: FREEZE.** 5 MB/h stands as the asserted nightly target
(`tests/soak/scenarios-nightly.toml` already sets `assert_target =
true`); the generous 50 MB/h stays as the CI-short asserted ceiling
(catches gross leaks only). Honest caveats: these are ≤407 s windows
extrapolated ×350 to an hour; a slow leak below ~0.06 MB/h would be
invisible to this run and is exactly what the 4 h nightly tier exists
to catch; RSS slopes include allocator/kernel arena effects that do
not linearly extrapolate. If the first real 4 h nightly lands in the
5–50 MB/h band, that is a finding → triage, then renegotiate the
number in config (the issue-#18 protocol, restated in the workflow
header) — never delete the job.

## 4. Enforcement is real (forced-threshold proof)

A green-only runner is a ritual (D-6). `tests/soak/
scenarios-forced-threshold.toml` is a shipped profile that
INTENTIONALLY FAILS, run for real (release, x86_64,
`docs/soak/raw/scenarios-forced-threshold.md`):

- open-close-churn, 1M cycles, budgets pinched from config
  (`rss_growth_budget_mb = 0.01`, `live_growth_budget_bytes = 0`):
  **FAIL, 2 violations** — live bytes 294,183 → 294,265 B (**+82 B
  over a 0 B budget across 849,815 cycles**, ≈ 0.0001 B/cycle
  sensitivity) and VmRSS 1,712 → 1,972 kB (0.25 MB over a 0.01 MB
  budget). Suite asserted; **cargo exit 101**.
- seek-storm with `leak_ceiling_mb_per_h = 0.0001`: its window slope
  happened to be negative (−6.951 MB/h) so the slope path passed this
  time; the slope-verdict arithmetic is pinned by the unit test
  `slope_verdict_flags_budget_violations`, and the deterministic trip
  in the profile is the growth path. The profile is on the
  shipped-manifests contract list, so it can never silently rot.

## 5. Cycle calibration (the config axis doing its job)

| scenario | calibration | bounded run | measured rate | nightly count |
|---|---|---|---|---|
| open-close-churn | 4,000 cycles in 0.6 s (first probe, report-to-stdout) | 2.0M cycles in 216.4 s | **9,242 cycles/s** | 24M ≈ 43.3 min |
| reopen-stress | 1,500 cycles in 3.0 s (`…__calibration.md`, committed) | 150k cycles in 200.1 s | **749.6 cycles/s** | 1.95M ≈ 43.4 min |

The first probes were too thin for a handle-growth signal (sub-second
walls); the counts were raised in `tests/soak/scenarios-sandbox.toml`
until the walls were minutes-scale — config edits only, the runner code
identical across the CI-short/nightly/sandbox tiers. The nightly counts
above fill ~43 min each at the measured rates and are bounded by the
48-min wall caps on slower runners.

**Instrument regression found and fixed during this measurement:** at
`cycles = 2,000,000` the original per-cycle sample series
(3 × n × 8 B, page-faulted in as the pushes advance) grew VmRSS by
~48 MB — the INSTRUMENT's own storage tripping the 8 MB budget with a
flat engine. Fix (commit 09ad85a): count-driven scenarios sample on a
bounded cadence — every `ceil(n / 4096)` cycles, ≤ 4,096+1 samples
(~100 kB of series) whatever n is, each sample tagged with the cycle it
was taken at so baseline/final spans stay honest. Pinned by the unit
test `count_sample_stride_bounds_the_instrument_memory` (stride 1 at
the original design points n ≤ 4,096; stride 489 at 2M).

## 6. Audits (honest)

### 6.1 Flaky-test quarantine — EMPTY; mechanism scheduled

- **Quarantined tests: zero.** `tests/quarantine/` does not exist; no
  test carries a flake/quarantine marker. CI has never executed
  (billing-blocked, §8) so no CI retry marks exist; local full-gate
  runs have recorded no intermittent failures (worklog history).
- All **55** `#[ignore]` markers in `crates/`+`apps/` were audited and
  fall into three documented classes, none a flake quarantine:
  1. explicit-run harnesses — throughput benches, fuzz-survival deep
     runs, randomized-corpus drivers, live-network tests — each with
     its run command recorded in the ignore reason;
  2. feature-gap contract tests — the `gap: … not wired` family with
     handoff ids (H-1..H-11), parked until their owning card lands;
  3. pending-investigation markers (3.B1 timing follow-ups).
- **Scheduled (the mechanism, doc 10 §8.3, still [PROPOSAL]):** owner
  **Lead** (CI policy is Lead territory); next step — when Actions
  billing unblocks, adopt the `tests/quarantine/` convention (header:
  issue, 14-day deadline, owner) and add the quarantined-test re-run
  job to the nightly. Nothing to graduate today: the count is zero.

### 6.2 Regression-guard (G13) — NOT IMPLEMENTED; gaps listed

G13 (doc 10 §8.2: CI step diffing the test inventory — `cargo test
--list` + scenario TOML ids + expectation-file hashes — base vs PR,
failing on net decreases / new `#[ignore]` / re-blessed hashes without
`test-change:` approval) **does not exist as tooling**. What stands in
its place today:

- the Lead's accept/reject review checklist (§8.2, human);
- per-worklog test-count reporting (manual discipline, e.g. "65/65",
  "111 passed" entries — real but not machine-enforced);
- the xtask gate family that DOES exist: check-architecture,
  check-panics, lint-literals, check-identity, sbom --check (drift),
  plus this suite's own contract test (all four shipped manifests
  validate against the loader + schema; forced-threshold profile on
  the list so the enforcement proof cannot rot).

Honest gaps: (1) no automated net-test-decrease tripwire; (2) the 55
`#[ignore]`s are justified but not machine-checked against a baseline;
(3) expectation-file hash re-blessing is not noisy yet (doc 10 §5.4/§8.2
expect it to be); (4) scenario-id deletion across the four shipped
profiles is unguarded. Next step (owner: Lead + wave-9 tooling card;
not technically blocked): an `xtask regression-guard` subcommand in
inventory-diff mode, wired next to lint-literals in `ci.yml`.

## 7. What is wired for CI (and what that wiring is worth)

`.github/workflows/soak-nightly.yml` (extended, existing schedule +
non-blocking posture, action pins verbatim) now has three jobs:

1. `soak` — the issue-#18 single-loop 4 h tier, unchanged.
2. `soak-scenarios` — the 8.C1 "4 h × scenarios" tier via
   `scripts/soak-scenarios.sh --profile tests/soak/scenarios-nightly.toml
   --release` (5 MB/h ASSERTED at this tier), report artifact uploaded
   always.
3. `soak-lsan` — doc 10 §6.5's LeakSanitizer nightly: parser + engine
   crate suites (`demux codecs playlists streaming media-probe input
   playback-engine --lib`) then the CI-short scenario profile, under
   `RUSTFLAGS=-Z sanitizer=leak` on a NIGHTLY toolchain (unstable-only
   flag — the one job deliberately off the stable pin, documented
   in-job). Build-script false-positive mitigation (suppressions file)
   is documented in the job header; a red run is a triage item, never
   a job deletion.

All three are machine-checked for YAML validity only. **None has ever
executed** (§8). Cost note recorded in the workflow header: ~9
runner-hours/night across the family; the Lead decides which 4 h tier
survives when billing unblocks.

## 8. Blocked / not measured (the honest list)

- **CI, all jobs** — billing-blocked repo-wide (every Actions run dies
  at 0 steps; docs/KNOWN-LIMITS.md R-16). The 4 h nightly profile, the
  LSAN run, and every workflow step above are wired-but-never-run
  `[VERIFY on first run]`.
- **LSAN locally** — the sandbox has no nightly toolchain and the
  shared target dir cannot fit a second, sanitizer-flagged rebuild of
  the dependency graph (disk pressure during this session bottomed out
  at 2.6 MB free; a 282 MB incremental-cache clean was needed mid-run).
- **4 h windows** — sandbox tool-call walls cap single commands below
  the tier; the bounded profile (§2) is the in-sandbox ceiling. The
  slope numbers are extrapolations (§3 caveats).
- **aarch64** — the briefing's environment label; the actual sandbox is
  x86_64 and is what the reports record. No aarch64 measurement exists.
- **Non-Linux RSS/fd sampling** — `/proc` instruments are Linux-only;
  the runner labels them `n/a` on other platforms (honest labels, not
  zeros). macOS/Windows soak evidence: none.
- **Real audio output** — scenarios decode into a counting sink (the
  headless presenter/null-device posture); device-timed playback is the
  browser/device matrix cards' territory.

## 9. D-10 pre-submission answers

- **MEASURE** — the five scenarios above (release, real engine, real
  fixtures, committed verbatim reports); forced-threshold exit 101;
  28+1 unit/contract tests green in the suite binary; the verification
  battery in §10.
- **NOT tested** — 4 h windows, real CI runners, LSAN-flagged builds,
  aarch64, non-Linux RSS, real audio devices (§8).
- **What breaks** — instrument memory at unbounded cycle counts (found,
  fixed, unit-pinned, §5); extrapolated slopes hide sub-0.06 MB/h leaks
  (mitigated by the asserted 4 h nightly when billing unblocks); the
  shared-CARGO_TARGET_DIR cross-worktree clobber remains a standing
  hazard for concurrent agents (worklog 7.w23d; hit again as transient
  disk pressure this session).
- **Who hits it first** — the first real nightly run (billing unblock):
  slope re-assertion at 4 h and the count recalibrations on real runner
  hardware; the next agent adding a soak scenario or touching the
  manifest schema (the contract test + schema pin force the loud path).

## 10. Verification battery (this branch, 2026-11-10)

Recorded verbatim in the W24-B worklog entry (repo-root `worklog.md`,
appended with this round): `cargo fmt --check`; `cargo clippy
--workspace --all-targets -- -D warnings`; `cargo test -p
playback-engine` (scoped); `cargo test --workspace --all-features`
(full, once, end of session); `cargo run -p xtask -- lint-literals`;
`cargo run -p xtask -- check-architecture`. The battery runs after
this report is written; any red blocks the push (actual results in
the worklog entry).
