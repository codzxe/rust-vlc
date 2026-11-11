# W24-G fixes — r2 review findings (gate-findings fixer lane)

**Task:** W24-R5 r2 (review scout 5, round 2 — resume/SHIP).
**Audited diff:** `git diff origin/main...origin/w24/g` @ `8121cbf`
(origin/main `c339ee8` + 7 commits: `1c48902` OF-2, `64757b1` OF-3, `ba9659f` OF-4,
`de9b341` OF-5, `9b15003` fmt, `56b59c1` gitignore chore, `8121cbf` clippy style).
**Context:** `docs/gates/phase7-exit.md` on `origin/w24/f` (§3 OF-1…OF-5).
**Date:** 2026-11-10. **Agent:** w24-r5. **Branch:** `w24/r5` (== `origin/w24/g`, no
source changes by this lane; this report + worklog only).

## 1. Verdict

**ALL FOUR IN-SCOPE FINDINGS (OF-2…OF-5) FIXED PROPERLY — none weakened. SHIP.**
Every previously-failing test now passes from worktree-proven binaries; fmt/clippy
on all three touched crates are clean under `-D warnings`; lint-literals green;
`npm run settings:check` green. Scope discipline holds (7 files: 6 finding-related
+ 1 benign gitignore chore that is itself finding-related infrastructure, see §6).
OF-1 (App.tsx markers) is lane A's finding, out of this lane's scope, still present
on main (2 markers) — unchanged expectation, to close via the Lead's lane-A merge.

## 2. Per-finding verdicts

### OF-2 — stale seed-count pins (settings-schema lib, integration, cli help) — **FIXED PROPERLY**

* **Pins are DERIVED, not re-pinned.** All three sites now read
  `schemas/settings.toml` with an independent raw-`toml::Table` parse (the crate
  under test is not asked what its own input contains) and derive every count:
  * `crates/settings-schema/src/lib.rs:155-185` — `schema.len()` vs seed
    `[[setting]]` row count; `schema.areas.len()` vs `[areas].known` length
    (32 / 14 today). No literals; the old `27`/`14` pins are gone.
  * `crates/settings-schema/tests/integration.rs:117-141` — `committed_outputs_have_no_drift`
    asserts the schema-gen output contains `"{derived} entries"` instead of `"27 entries"`.
  * `apps/cli/src/help.rs:303-375` — `areas_block_lists_the_six_areas` renamed to
    `areas_block_lists_every_screen_the_seed_declares`; it now rebuilds the ENTIRE
    expected `Areas:` block from the seed (screens sorted+deduped, per-screen
    counts, `engine-internal` annotation rule mirroring `emit.rs:291-302`) and
    asserts full string equality + `known_areas() == screens`. Strictly STRONGER
    than the old test (old: name-vector equality + two `contains` spot-checks).
  * Seed growth can no longer strand a pin: a c71cd5a-class commit (27→32)
    moves both sides of every assertion together.
* **The shape guard is real and fails on drift** — traced against the actual
  artifacts: `seed_shape_is_what_every_committed_artifact_self_reports`
  (`integration.rs:150-214`) cross-checks the seed's own shape (32 entries /
  7 screens / 14 areas) against (a) the G1 header
  `// 32 entries across 7 screen(s) (14 known areas).` in
  `schemas/generated/settings.d.ts` and (b) every per-screen row
  (`  engine         6 options`, … `  extensions     5 options`, sums = 32) in
  `schemas/generated/settings-help.txt`. A 27/6-stale artifact fails both checks
  with a message naming the two regen commands (`cargo xtask schema-gen`,
  `npm run settings:generate` in ui/). Negative paths are additionally covered by
  the pre-existing `stale_output_drift_fails_check_with_exit_one` (fixture-based)
  and `committed_outputs_have_no_drift` (runs `schema-gen --check` itself).
* **Residual (accepted, by design):** the Rust guard does not read the UI artifact
  `ui/src/screens/settings/generated/schema.ts` — that stays guarded by
  `npm run settings:check` + the UI vitest suite (battery §2 #16/#17), the
  pre-existing division of labor. `cargo test --workspace` alone would not catch
  UI-artifact drift; the full gate battery does.

### OF-3 — stale ui generated/schema.ts — **FIXED PROPERLY (regenerated)**

* Diff is pure addition: 5 `SETTING_EXTENSIONS_CRASH_*` constants (exactly the
  5 `extensions.*` `[[setting]]` rows c71cd5a added — crash-backoff-initial-ms /
  multiplier / max-ms / restart-limit / disabled-reenable-ms) + the 5 matching
  `SETTINGS_MSGIDS`. Correctly NOT added to `SETTINGS_ENTRIES` ("every ui=true
  entry"): the extensions rows are `ui = false` (engine-internal), matching the
  G2 annotation `extensions … (engine-internal: ui = false)` and the 21 ui=true
  count. `Screens.tsx` untouched — correct, no ui-visible shape changed.
* Verified: `cd ui && npm run settings:check` → **PASS**, exit 0 —
  "OK (5 ui areas, 21 ui entries, 32 total)". The check byte-compares BOTH
  `schema.ts` and `Screens.tsx` against a fresh regeneration from
  `schemas/settings.toml` + `product.toml` (`gen-settings-screens.mjs:581-597`),
  so the artifact is byte-exact vs the current seed, not just count-matched.
  (Run without the node_modules symlink — the script is zero-dep node; the npm
  wrapper invokes exactly this script.)

### OF-4 — rebrand reverse-control test mis-rooting — **FIXED PROPERLY, NOT WEAKENED**

* `tests/rebrand/rebrand_skeleton.rs:126-143` — `repo_root()` now derives from
  compile-time `env!("CARGO_MANIFEST_DIR")/../..` (this file is compiled into the
  xtask package's `cli` integration test, so that is always THIS worktree) instead
  of walking up from `CARGO_BIN_EXE_xtask`, which under the shared
  `CARGO_TARGET_DIR` resolved the orchestrator's stale checkout. Exactly the
  gate doc's [PROPOSAL].
* `run_rebrand` (`:80-95`) additionally pins `.current_dir(repo_root())` — the
  tool discovers the workspace from its cwd (`tools/xtask/src/cmd/rebrand.rs:718-719`),
  so tool and test now provably operate on the same tree: hermetic w.r.t. both
  the harness cwd and the binary location.
* **The proof is intact and stronger:** `reverse_control_real_product_toml_is_a_noop`
  (`:470-519`) still runs the REAL `product.toml` through the real binary into a
  temp dest, asserts exit 0 + `result: no-op`, and BYTE-compares all 5 surfaces
  (product.toml, tauri.conf.json, README.md, settings-docs.md, settings-help.txt)
  repo-vs-dest — now against the same tree the tool read, which is what makes the
  no-op claim meaningful. No assertion deleted; the 3 `#[ignore]`d boot-tier
  placeholders are pre-existing (verified identical on origin/main) and unrelated.
* **Field-proven:** the suite passes under the shared target dir (the original
  failure condition) — see §4, and this audit itself hit the same clobber disease
  twice while verifying (§5), which is the strongest possible confirmation the
  fix addresses a real, live mechanism.

### OF-5 — feature-matrix MF over-claim — **FIXED PROPERLY (matches code reality)**

* `docs/feature-matrix.toml:416` note now states: VT landed (aarch64
  compile-verified, runtime [VERIFY], pure halves host-tested); VA-API landed
  (compile-verified + failure-path runtime-tested on the no-GPU host, activation
  [PROPOSAL] pending ADR-013); **"Media Foundation (Windows) NOT IMPLEMENTED —
  no MF lane exists"**, retraction attributed to the hw/mod.rs truth table, plus
  an explicit honesty-fix provenance sentence naming gate OF-5. Status cells
  unchanged (`partial`), which the gate doc itself already assessed as honest.
* Code reality re-verified this session: `crates/codecs/src/video/h264/hw/`
  contains only `vaapi/` + `videotoolbox/` (no MF module/directory). The
  remaining `MediaFoundation` hits in Rust are (a) the `HwBackend::MediaFoundation`
  capability-tag enum (the settings/negotiation vocabulary — Windows's nominal
  label that answers the typed `h264.hw.required`/`no-bridge` truth) and (b)
  truth-table/doc comments saying NOT IMPLEMENTED. No MF FFI exists. Note ==
  code. ✓

## 3. Violation scan (diff-wide)

* **No `#[ignore]` added** (`git diff | grep -c '^\+.*#\[ignore]'` → 0); the 3
  ignores in rebrand_skeleton.rs are byte-identical to origin/main.
* **Removed assertions** = exactly the 4 stale pins
  (`assert_eq!(schema.len(), 27 …)`, `assert_eq!(schema.areas.len(), 14 …)`,
  `contains("27 entries")`, and the old 6-area vec + 2 spot-checks) — every one
  replaced by a derived, equal-or-stronger assertion (full-block string equality
  in help.rs). No test was weakened; one was renamed with its semantics upgraded.
* **Strict Rust:** no `unwrap()` in new code paths (failures route through
  `unreachable!` with context — acceptable, test-only, loud); no `unsafe`; no
  new dependencies (`toml` was already a dep of both crates; no Cargo.toml in
  the diff).
* **fmt / clippy:** `cargo fmt --check` PASS; `cargo clippy -p settings-schema
  -p lumen-cli -p xtask --all-targets -- -D warnings` PASS (provenance-verified,
  see §5) — CLI crate name is `lumen-cli`.
* **lint-literals:** PASS — 0 violations, 452 allowlisted (matches gate-log
  baseline). **check-architecture:** PASS 5/5 (extra, unrequested but cheap).

## 4. Previously-failing tests — re-run on w24/r5

From `git show origin/w24/f:docs/gates/logs/phase7-exit-test.log` (failures:
`help::tests::areas_block_lists_the_six_areas`, `tests::committed_seed_validates_against_committed_product`,
`committed_outputs_have_no_drift`, `rebrand_skeleton::reverse_control_real_product_toml_is_a_noop`).

| Suite (gate-log target) | Result on w24/r5 | Notes |
|---|---|---|
| `-p settings-schema --lib` | **43 passed / 0 failed** | old: 42/1; pin now derived |
| `-p settings-schema --test integration` | **8 passed / 0 failed** | old: 6/1; incl. NEW `seed_shape_is_what_every_committed_artifact_self_reports` + fixed `committed_outputs_have_no_drift` |
| `-p lumen-cli --lib` | **125 passed / 0 failed** | old: 124/1; renamed+strengthened `areas_block_lists_every_screen_the_seed_declares` passes |
| `-p xtask --test cli` | **12 passed / 0 failed / 3 ignored** | old: 11/1/3; `reverse_control_real_product_toml_is_a_noop` + `nova_fixture…` both pass under the shared target dir |

Plus: `npm run settings:check` PASS; `lint-literals` PASS; `check-architecture` PASS.
All runs executed from binaries whose `.d` dep-info proves compilation from
`/home/z/my-project/wt/w24r5` (see §5 for why that proof was necessary).

## 5. Operational incident during this audit — shared-target clobber (P2, infra)

The shared `CARGO_TARGET_DIR` binary-name collision (documented by w23e; the same
disease family OF-4 recorded) hit THIS audit twice, live:

1. First `cargo test -p lumen-cli --lib` run FAILED with the OLD test name
   (`areas_block_lists_the_six_areas`, pin 27 vs 32) — the executed binary's
   dep-info showed `CARGO_MANIFEST_DIR=/home/z/my-project/wt/w24d/apps/cli`
   (parallel lane w24d, at pre-fix source). Remedied with `cargo clean -p lumen-cli`
   + immediate re-run → 125/0, dep-info = my worktree.
2. `cargo test -p xtask --test cli` ran a binary compiled from
   `/home/z/my-project/wt/w24l/tools/xtask` (pre-OF-4 code: panic "workspace root
   not found from /tmp/my-project/target/debug/xtask"), failing 2-3 tests
   including the fixed one. Remedied by re-running in a lane-isolated
   `CARGO_TARGET_DIR=target-xtask` (the pattern g itself used and gitignored in
   `56b59c1`) → 12/0/3, provenance clean.

Lesson for every reviewer lane: **a shared-dir test result without binary
provenance is not evidence.** Verify the executed binary's `.d` file
(`CARGO_MANIFEST_DIR=...`) or run in an isolated target dir before recording
a FAIL. [PROPOSAL] (Lead): per-lane `CARGO_TARGET_DIR` suffixing or fingerprint
namespacing in `scripts/cargo-env.sh` to kill the collision class; the flock
wrapper serializes builds but does not isolate output filenames (metadata hash
is source-path-independent, so lanes overwrite each other's
`deps/<name>-<hash>` binaries).

## 6. Scope discipline

`git diff --stat origin/main...origin/w24/g` = 7 files:
`.gitignore` (+3), `apps/cli/src/help.rs`, `crates/settings-schema/src/lib.rs`,
`crates/settings-schema/tests/integration.rs`, `docs/feature-matrix.toml`,
`tests/rebrand/rebrand_skeleton.rs`, `ui/src/screens/settings/generated/schema.ts`.
Six map 1:1 onto OF-2/3/4/5. The `.gitignore` chore (`target-xtask/`) is
finding-related infrastructure: g needed an isolated build dir to test OF-4
hermetically under the shared-target regime (§5 proves the need is real). No
product code, no CI, no schemas touched. **Gap (P3): the g lane never appended
its AGENTS.md §4 worklog entry** — its 7 commits carry good messages but the
mandatory root `worklog.md` is untouched on w24/g. This lane's entry (below)
records the r2 half; the g lane's own entry remains owed to the Lead's merge
discipline.

## 7. Findings register (this audit)

| ID | Severity | Finding | Owner |
|---|---|---|---|
| R5-1 | P2 (infra) | Shared-target-dir binary clobber still live; hit this audit twice (w24d, w24l collisions); flock serializes but does not isolate; unprovenanced shared-dir test results are not evidence | Lead / tooling |
| R5-2 | P3 (process) | g lane's mandatory worklog.md entry missing (AGENTS.md §4) for all 7 commits | Lead (merge gate) |
| R5-3 | P3 (nit) | `integration.rs:176-180` — the "every [[setting]] row must name a placement.screen" assert is tautological as written (screens is collected 1:1 from entries; a missing screen yields `""` and passes THIS assert) — the drift IS caught one step later by the G1-header/G2-row asserts, only the message quality suffers | settings-schema (next touch) |
| R5-4 | note | Rust-side shape guard intentionally excludes the UI artifact (ui tooling owns it — `settings:check` + vitest); `cargo test --workspace` green ≠ UI artifact fresh | accepted design |

## 8. Conclusion

The gate-findings fixer lane did what the gate doc asked, without shortcuts:
pins became derivations, the artifact was regenerated (byte-exact), the test's
rooting now matches the tool's, the matrix tells the truth the code tells.
**Recommendation: merge `w24/g` (this branch, `w24/r5` == `8121cbf`) toward
wave-24 integration; re-run the §8 phase-7 battery at merge time (OF-1 closes
via lane A).**
