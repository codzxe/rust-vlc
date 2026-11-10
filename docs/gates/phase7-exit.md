# Phase 7 Exit Gate — Platform & Advanced (issue #51, card 7.L1-V)

**Task:** W24-F r2 (verification lane; round 1 = gate-log collection, round 2 = this report).
**Audited tree:** worktree `/home/z/my-project/wt/w24f`, branch `w24/f` @ `a4e35e7`
(== `origin/main` `c339ee8` + gate-log commits only; no source changes by this lane).
**Date:** 2026-11-10. **Agent:** w24-f (Lead-delegated verification).

## 1. Verdict

**NOT PASSED at `c339ee8` — 5 open findings (OF-1…OF-5), all with single-commit
root causes in already-merged wave-23 commits, all with identified owners and
bounded fixes.** 18 of 22 battery checks green (§2). The per-card substance of
Phase 7 (§4) is in-tree and tested: platform adapters, hw-decode lanes with
honest compile/runtime labels, hotplug+discovery, recording/remux, crash
isolation + example extensions, web pillars, control server first-pass
security — with the browser matrix (#49) honestly in-flight on `w24/a`.

Per repo law this is a **valid gate outcome**: failures are recorded, not
papered over. Recommended closure path: land OF-1's in-flight fix
(`w24/a` `70ed69c`), apply the OF-2/OF-3 pin bumps + regeneration, correct the
OF-5 matrix note, then re-run the §8 battery at wave-24 merge. No finding
requires design work.

## 2. Gate battery — evidence ledger

All commands run in `/home/z/my-project/wt/w24f` after
`source /home/z/my-project/scripts/cargo-env.sh` (shared `CARGO_TARGET_DIR`,
`RUSTFLAGS=-Cdebuginfo=0`, `CARGO_INCREMENTAL=0`). Logs carry per-run headers
with worktree, commit, date and exit code.

| # | Check | Status | Evidence |
|---|---|---|---|
| 1 | `cargo fmt --check` | **PASS** (exit 0, no diffs) | `logs/phase7-exit-fmt.log` (`FMT_EXIT=0`) |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** (exit 0; r2 in 21.8 s warm, r1 cold 1 m 14 s) | `logs/phase7-exit-clippy.log` (r2) + `logs/phase7-exit-clippy-r1.log` (round-1 body, `CLIPPY_EXIT=0`) |
| 3 | `cargo test --workspace` (round 1, all 306 targets + doctests) | **FAIL** (exit 101): **4418 passed / 4 failed / 49 ignored** | `logs/phase7-exit-test.log` (`TEST_EXIT=101`; failing targets listed at end) |
| 4 | 4 failing targets, scoped re-confirmation (r2) | **FAIL** (reproduced; exit 101) | `logs/phase7-exit-test-rerun.log` |
| 5 | first-vertical evidence suite (`cargo test -p first-vertical --test evidence`) | **PASS** (3 passed / 1 ignored-by-design `regen`) | `logs/phase7-exit-evidence-rerun.log` |
| 6 | wasm32 build, web composition root: `cargo build -p lumen-web --target wasm32-unknown-unknown` (+ `--locked`) | **PASS** (exit 0 both; artifact `lumen_web.wasm` 5,998,961 B debug) | `logs/phase7-exit-wasm.log` |
| 7 | wasm32 canary (mirrors CI G12): `cargo build -p theme-tokens --target wasm32-unknown-unknown` (+ `--locked`) | **PASS** (exit 0 both) | `logs/phase7-exit-wasm.log` |
| 8 | `cargo run -q -p xtask -- check-architecture` | **PASS** (5/5: edges, cycles 0, unsafe-lints 39/39, static-mut 0/627 files, testkit-direction; 39 crates, 97 internal edges) | `logs/phase7-exit-xtask.log` |
| 9 | `cargo run -q -p xtask -- codegen --check` | **PASS** (no drift; 64 commands / 26 events) | `logs/phase7-exit-xtask.log` |
| 10 | `cargo run -q -p xtask -- schema-gen --check` | **PASS** (no drift; 32 entries / 7 screens / 14 areas) | `logs/phase7-exit-xtask.log` |
| 11 | `cargo run -q -p xtask -- fixtures --check` | **PASS** (no drift; 55 files) | `logs/phase7-exit-xtask.log` |
| 12 | `cargo run -q -p xtask -- lint-literals` | **PASS** (0 violations, 452 allowlisted; 18,602 sites / 558 files) | `logs/phase7-exit-xtask.log` |
| 13 | `cargo run -q -p xtask -- check-identity` | **PASS** (0 violations, 453 allowlisted) | `logs/phase7-exit-xtask.log` |
| 14 | `cargo run -q -p xtask -- sbom --check` (extra: #52 drift gate) | **PASS** (no drift; 121,900 B) | `logs/phase7-exit-xtask.log` |
| 15 | cargo-deny 0.20.2, CI command `--workspace check licenses sources bans advisories` | **PASS** (exit 0: "advisories ok, bans ok, licenses ok, sources ok"; 32 non-blocking `wildcard` path-dep warnings) | `logs/phase7-exit-deny.log` — binary = pinned musl tarball, downloaded this session, SHA-256 `9f12ed4c…c8d5f` == `.github/workflows/ci.yml:469` pin |
| 16 | UI `npm run typecheck` / `lint` | **FAIL** (TS1185/TS1005 parse errors; eslint 1 error) — OF-1 | `logs/phase7-exit-ui.log` |
| 17 | UI `npm run test` (vitest) | **FAIL**: **705/706 tests passed**; 2 suites fail to *load* (App.tsx markers), 1 failure = settings-drift (OF-3) | `logs/phase7-exit-ui.log` |
| 18 | UI `npm run l10n:check` | **PASS** (611 keys, missing 0, duplicates 0, orphans 0) | `logs/phase7-exit-ui.log` |
| 19 | `cargo run -q -p xtask -- parity-report` | **PASS** (runs clean; numbers in §5) | `logs/phase7-exit-parity.log` |
| 20 | `cargo run -q -p xtask -- feature-audit` | **PASS** (252 rows, structure + evidence resolution OK) | `logs/phase7-exit-parity.log` |
| 21 | `cargo deny` + `sbom --check` supply-chain pair (doc 11 §3.5 order) | **PASS** (see #14, #15) | same logs |
| 22 | `xtask rebrand` reverse control (manual, gate-diagnosis run) | **PASS** (`result: no-op`, exit 0, dest README byte-identical to worktree) — the *committed test* variant fails, see OF-4 | shell run 2026-11-10, reproduced in §3 OF-4 |

Round-1 provenance: logs `phase7-exit-{fmt,clippy,test,evidence-rerun}.log` were
collected in round 1 (committed `a4e35e7` before any other work, per
context-kill discipline); round 1's fmt log was empty with no exit marker, so
fmt was re-run in r2 with a recorded header.

## 3. Open findings (file · line · severity · root cause · owner · next step)

### OF-1 — committed merge-conflict markers in `ui/src/App.tsx` [P0, product]

* **Where:** `ui/src/App.tsx:15-24` (`<<<<<<< HEAD` … `>>>>>>> w23/c`) and
  `ui/src/App.tsx:100-104` (route block). Confirmed in `origin/main`
  (`git show origin/main:ui/src/App.tsx | grep -c '^<<<<<<<'` → 2) and at HEAD.
* **Symptom:** UI typecheck exit 2 (21 TS errors), eslint exit 1 (parse error),
  vitest 2 suites fail to load (`App.test.tsx`, `i18n/pseudo.render.test.tsx`).
  Evidence: `logs/phase7-exit-ui.log`.
* **Root cause:** wave-23 merge `45c5c6f` (issue #44, `w23/c`): the commit
  *message* says "i18n/App keep discovery+convert" but the committed file kept
  the raw conflict region (both screens' imports/routes are present, wrapped in
  markers).
* **Fix in flight:** `w24/a` `70ed69c` "fix(ui): resolve committed merge
  markers in App.tsx — keep BOTH discovery+convert" (verified: its App.tsx has
  both `/discovery` and `/convert` routes, no markers). Owner: lane A / Lead
  merge.
* **Next step:** merge lane A; re-run UI battery (§8).

### OF-2 — stale Phase-1 pins after the settings seed grew 27 → 32 entries [P1, tests]

* **Where / symptom** (all three re-confirmed in r2, `logs/phase7-exit-test-rerun.log`):
  * `crates/settings-schema/src/lib.rs:155` — `assert_eq!(schema.len(), 27, "Phase-1 seed entry count")`; actual 32 (`tests::committed_seed_validates_against_committed_product`).
  * `crates/settings-schema/tests/integration.rs:117` — `contains("27 entries")`; actual output "32 entries" (`committed_outputs_have_no_drift`).
  * `apps/cli/src/help.rs:302-321` — `areas_block_lists_the_six_areas` pins 6 help areas; generated help now lists 7 (`extensions` added): `["engine","extensions","global","interface","performance","playback","privacy"]`.
* **Root cause:** single commit `c71cd5a` (wave 23, issue #46 — plugin crash
  backoff settings) grew `schemas/settings.toml` 27→32 `[[setting]]` entries
  (verified: 27 at `3116aec`, 32 at `c71cd5a`) and regenerated the *Rust-side*
  artifacts (`schema-gen --check` is green, 32 entries) but did not bump these
  three pins (nor the UI artifact, OF-3).
* **Owner/next step:** not owned by any wave-24 lane (settings-schema crate);
  2-line pin bumps ×3 by the Lead or a granted lane. These pins exist to force
  conscious updates — the failure is the mechanism working as designed.

### OF-3 — UI generated settings schema stale vs seed [P1, generated artifact]

* **Where:** `ui/src/screens/settings/generated/schema.ts`.
* **Symptom:** `npm run settings:check` → "schema.ts is stale vs
  schemas/settings.toml — run `npm run settings:generate`", exit 1 (captured
  2026-11-10); vitest `src/screens/settings/generated.test.ts:80`
  "passes settings:check against the committed generated files" fails.
* **Root cause:** same commit `c71cd5a` (OF-2) — UI-side generator output not
  regenerated.
* **Owner/next step:** `ui/` is lane A this wave; fix = `npm run
  settings:generate` + commit. Fold into lane A's existing UI pass (OF-1).

### OF-4 — rebrand reverse-control test breaks under the shared target dir [P2, test infra]

* **Where:** `tests/rebrand/rebrand_skeleton.rs:492` (assert) via
  `repo_root()` at `:120-134`.
* **Symptom:** `reverse_control_real_product_toml_is_a_noop` fails: "`README.md`
  differs in the no-op dest copy"; left 3569 B vs right 8032 B.
* **Root cause (verified this session):** `repo_root()` walks up from
  `env!("CARGO_BIN_EXE_xtask")`. Under the orchestrator's shared
  `CARGO_TARGET_DIR=/home/z/my-project/rust-vlc/target`, that resolves to the
  **stale rust-vlc checkout** (its README.md = 3569 B, still "Phase 1 —
  Foundation planning (in progress)"), while the rebrand *tool* roots itself
  via cwd at the current worktree (README 8032 B, Phase-6 status + Install
  section). Manual proof: `cargo run -q -p xtask -- rebrand --config
  product.toml --dest /tmp/…` → `result: no-op`, exit 0, **dest README
  byte-identical to the worktree README** (`diff -q` clean). I.e. the tool is a
  true no-op; only the test's root discovery is wrong under a shared target
  dir. Not a product regression; the test passes with per-worktree target dirs.
* **Owner/next step:** Lead (test infra). Proposed fix: derive the root from
  `env!("CARGO_MANIFEST_DIR")` (compile-time, worktree-correct) or pass an
  explicit `--root`. Recorded as `[PROPOSAL]`.

### OF-5 — feature-matrix H.264 note over-claims the Media Foundation lane [P1, honesty]

* **Where:** `docs/feature-matrix.toml:416` (H.264 row `notes`): "adapter lanes
  landed (VT/MF/VA-API under `crates/codecs/src/video/h264/hw/`) —
  compile-verified via `cargo check --target aarch64-apple-darwin` +
  `x86_64-pc-windows-msvc`".
* **Contradicted in-tree by** `crates/codecs/src/video/h264/hw/mod.rs:53-58`:
  "**Media Foundation** (Windows, 7.B2): **NOT IMPLEMENTED** — no MF lane
  exists in this crate … The bullet this one replaces claimed a
  compile-verified MF FFI that does not exist — **retracted**." Directory
  truth: `hw/` contains only `vaapi/` and `videotoolbox/` (no
  `mediafoundation.rs`; `git ls-tree 67a139c` confirms it never existed).
* **Also over-claiming:** commit messages `67a139c` ("land VideoToolbox + MF
  hw lanes — all three 7.B1-7.B3 lanes complete") and merge `9cff3c4`
  ("VT/MF/VA-API lanes … compile-verified"). History is not rewritten; recorded
  here for the ledger.
* **Status of the row itself:** honest — `status = { windows/linux/macos =
  "partial", web = "not_started" }`; only the note text over-claims.
* **Owner/next step:** `feature-matrix.toml` is lane A this wave; correct the
  note to "VT + VA-API lanes landed (MF retracted — not implemented)".
  The module-level truth table (`hw/mod.rs`) is the authoritative honest
  statement and is what §4 7.B cites.

**Minor (recorded, no action demanded):** `crates/codecs/src/video/hevc/hw.rs:7`
numbers the 7.B lanes (7.B1 VA-API, 7.B2 VT, 7.B3 MF) differently from doc 13
§10 (7.B1 VT, 7.B2 MF, 7.B3 VA-API). Cosmetic labeling drift only; the HEVC
module is otherwise explicit that its adapters are "*not built here*" and its
matrix row claims nothing false.

## 4. Per-card Phase-7 verification (cards → issues → evidence)

All test counts below are from `logs/phase7-exit-test.log` (full workspace
run, round 1) unless noted; the named tests were individually confirmed `ok`.

| Card | Issue (commits) | Status | Evidence |
|---|---|---|---|
| 7.A1–7.A3 platform adapters | #40 (`556ae5f` wave 22) | **LANDED, host-verified; real-OS device-enum BLOCKED (hardware)** | Crates exist (`crates/platform-{windows,linux,macos}`). This host (Linux): `platform-linux` 28/0 green — `tests/linux_integrations.rs` 13/0 (incl. `drm_scanner_reads_the_real_host_sysfs_honestly`, `pulse_probe_reports_this_hosts_truth`, typed no-server refusal) + `tests/seam_behaviour.rs` 15/0. macOS/Windows on Linux host: `seam_behaviour` 16/0 each, incl. `discover_on_a_foreign_host_reports_wrong_os_everywhere`. Unsafe audit: module-scoped `#![allow(unsafe_code)]` with "SAFETY-SCOPED MODULE" justification + per-block `// SAFETY:` invariants (e.g. `platform-linux/src/linux/dl.rs`, `pulse.rs`); structural gate: check-architecture `unsafe-lints` PASS 39/39. **Blocked:** mac/Windows device-enum tests need 3-OS hardware (MP §17; §7). FFI compile for darwin/windows: cited from lane evidence (`cargo check --target …` labels in crate docs), not re-runnable in this sandbox (targets not installed, 1.1 GB free disk) — the Linux-target half is re-verified by this gate's clippy (exit 0). |
| 7.B1–7.B3 hw-decode truth table | #41 (`9cff3c4`, `67a139c`) | **LANDED with honest labels; truth table = `crates/codecs/src/video/h264/hw/mod.rs:38-65`** | VT (macOS): compile-verified via `cargo check --target aarch64-apple-darwin` (lane evidence, cited), runtime hardware **NOT exercised — `[VERIFY]`**; pure halves host-tested (`hw/feed.rs`). VA-API (Linux): compile-verified + **failure-path runtime-tested on this host** (no GPU → typed `Unavailable`, typed-refusal chain engages); driver-mediated decode compile-verified only; activation `[PROPOSAL]` pending ADR-013 ratification (Lead action). **MF (Windows): NOT IMPLEMENTED — claim retracted in-tree** (see OF-5: matrix note contradicts). Settings-driven, default OFF (`installed_bridges`/`negotiate_lane`, `playback.hw-decode-policy` = `software-only`, `features.hw-decode = false`). codecs suite green: 506 unit + full integration battery (incl. `h264_hw_path.rs`, `hevc_hw_path.rs`). |
| 7.C1 hotplug + output fallback | #42 (`d5611d1`) | **LANDED, tests green** | `playback-engine` 81/0, `audio-output` 33/0, `renderer-wgpu` 60/0 (backend fallback chain), simulated-device-event hotplug tests in suite. |
| 7.D1 device discovery | #42 (`4b8c808`, `9e77cc1`, `3e852fc`) | **LANDED, tests green; UI exists (route blocked by OF-1, fix in flight)** | `device-discovery` 37/0 green (bounded volume watcher, SSDP lane + honest verdict); `/discovery` UI: `ui/src/screens/discovery/DiscoveryScreen.tsx` + `DiscoveryScreen.test.tsx` 8/8 green (vitest, `logs/phase7-exit-ui.log`); route wiring on main is inside OF-1's conflict region, resolved in `w24/a` `70ed69c`. |
| 7.E1 transcode core | #43 (`3fd09ca` wave 22) | **LANDED, tests green** | `transcode` 50/0 (job model, progress/cancel, bounded resources; WAV PCM + stream-copy only). |
| 7.E2/7.E3 recording, remux, convert UI | #44 (`45c5c6f`, `f037ba6`, `b3f8445`, `62dfade`) | **LANDED, tests green; UI exists (route blocked by OF-1, fix in flight)** | record-to-WAV + MP4 remux (H.264/AAC stream copy, no transcode claim) with outputs verified by own demux+decode (acceptance suites in `crates/transcode`); `/convert` UI: `ConvertScreen.test.tsx` 8/8 green; target picker limited to real capability. |
| 7.F1–7.F3 plugin host v1 | #45 (`d33886d` wave 22) | **LANDED, tests green** | `plugin-host-wasm` 44/0 (wasmi per ADR-002, capability enforcement, `.lumenext` packaging). |
| 7.F4 crash isolation | #46 (`95c0743`, `c71cd5a`, `b5e2560`) | **LANDED, suite green incl. playback-unaffected proof** | `tests/plugin_crash_isolation.rs` 6/0: `crash_injection_trap_on_entry_is_contained`, `…_mid_call_…`, `…_host_fault_from_import_…`, `…_resource_exhaustion_…`, `plugin_restart_backoff`, and `plugin_crash_isolation` (trap-loop/OOM cannot kill playback — `PlaybackFixture` continues). |
| 7.G1 example extensions | #46 (`3a77606`, `3de83e5`) | **LANDED, kit green, docs walkthrough in-tree** | `plugins/extensions/{metadata-provider,ui-panel}`; `plugin_examples_conformance` 5/0; walkthrough docs (commit `3de83e5`). |
| 7.H1 apps/web v1 | #47 (`c97a3d6` wave 22) | **LANDED, tests green, wasm build green** | `lumen-web` 111/0; wasm32 build exit 0 (§2 #6). |
| 7.H2–7.H4 web pillars | #48 (`67d567d`, `5dbd7ec`) | **LANDED, tests green** | render/audio/io pillars in `apps/web/src/{render,audio,io}`; 111/0 covers the pillar suites (per w23/e closeout: render 20, audio 15, io 35); WebGL2 fallback ladder, AudioWorklet sample-counter clock, fetch+Range seekability + typed blocked capabilities. Browser-halves remain `[VERIFY]` → 7.I1. |
| 7.I1 browser matrix | **#49 — IN-FLIGHT on `w24/a` this wave** | **IN-FLIGHT — no pass claimed; Lead re-verifies at wave merge** | Lane A active (branch tip `771259c`; contains OF-1's fix `70ed69c`). Delivers `docs/web/browser-matrix.md` + web feature-matrix honesty pass. This gate's web snapshot: §5. |
| 7.K1 web packaging + budgets | #49 (same issue) | **IN-FLIGHT** (sequenced after 7.I1 per plan) | — |
| 7.J1 control-protocol + control-server | #50 (`8e8c70c`, `30ca309` wave 22) | **LANDED, first-pass secured** | `control-protocol` 47/0 (lib) + conformance 25/0: `non_loopback_bind_is_refused_at_construction_conformance` (`tests/conformance.rs:727`), `loopback_binds_succeed_on_both_families` (`src/server.rs:918`), `handshake_success_full_session_round_trip`, `wrong_token_closes_4401_and_counts`, `five_failures_lock_out_even_the_right_token`. `control-server` 11/0. First-pass-secured statement: `docs/security/sweep-control.md` (w23-f sweep; loopback/token/deny-by-default verified, config grants closed list). Deep security review deferred to 8.A2 per plan — recorded, not hidden. |

## 5. Web parity column honesty

`cargo run -q -p xtask -- parity-report` (this session, `logs/phase7-exit-parity.log`):

* implemented-or-verified: **windows 64/252 · linux 64/252 · macos 64/252 ·
  web 0/252 (0.0 %)**; verified-only (MP §1 strict): **0/252 on every
  platform** — the strict-verification debt remains the honest headline,
  unchanged since the Phase-6 draft.
* The web column is **honest by conservatism**: no web row claims
  `implemented`/`verified`; web cells are `not_started`/`blocked`/`unsupported`
  (e.g. `docs/feature-matrix.toml` MKV/MP4/Ogg rows: `web = "not_started"`
  despite the demuxers compiling into the wasm core). The flip-to-implemented
  pass is exactly in-flight issue #49's deliverable (browser-matrix evidence
  first). Hardware decode on web: `blocked` pending a WebCodecs ADR
  (`docs/KNOWN-LIMITS.md:63`).
* `feature-audit` PASS (252 rows; per-phase rows include phase 7 = 25;
  per-platform-status cells: blocked=133, implemented=192, not_started=505,
  partial=142, unsupported=36).
* One honesty defect in a *desktop* note: OF-5.

## 6. Phase 8 launch list

Wave-24 lanes already running (this wave, per assignment map): B → #54
(soak), C → #56 (packaging), D → #53 (corpus/compat matrix), E → #58
(docs/release checklist); A carries #49 (browser matrix + web packaging) —
**Lead re-verifies #49 at wave merge, incl. OF-1/OF-3/OF-5 which sit in lane
A's owned files**. Next after wave 24: #59 (P8-8, `[LEAD]` Phase-8 gate +
program exit). #52 (security sweeps + SBOM) closed in wave 23
(`14496c7`/`7f3443d`; this gate re-verified `sbom --check` + cargo-deny
green). #55 (perf-vs-VLC) and #57 (parity tool finalization) queue behind
their deps per `docs/plans/phase8-cards.md` ordering.

**Carried into Phase 8 from this gate:** OF-2/OF-3/OF-4 fix + full battery
re-run at wave-24 merge (§8); ADR-013 ratification (VA-API activation,
Lead); strict-verified 0/252 debt (CI-dependent); 6.H1 Medium security
finding (inherited by 8.A2 as first triage item, per phase8-cards §0.4).

**External blockers (owner = user, not agents; MP §17 — blocked with reason +
next step, never silently descoped):**

| Blocker | Blocks | Next step |
|---|---|---|
| CI Actions billing (every run dies at 0 steps) | all CI gates; strict-verified parity; browser matrix CI leg | user enables billing; until then local runs are the evidence vehicle (this report's mode) |
| 3-OS runner machines | 7.A mac/windows device-enum runtime tests; 8.B1 matrix; 8.D1 perf; 8.G2 RC smoke | user provides runners (R-20) |
| Apple Developer account / key custody (R-35) | 8.E2 macOS notarization, cascades to SBOM-all-platforms + RC | user decision at first signed-build attempt |
| Windows code-signing certificate | 8.E1 signing + upgrade-path test | user procures cert (`[VERIFY] — escalate if absent`) |

## 7. Environment honesty register

* All evidence is **local** (Linux sandbox, 2-core, `RUSTFLAGS=-Cdebuginfo=0`);
  CI is billing-blocked (0 steps execute) — standing caveat, doc 10 §10.4.
* Shared `CARGO_TARGET_DIR` across 6 agents: build logs legitimately show
  `/home/z/my-project/rust-vlc/target/...` paths (the orchestrator-maintained
  cache); sources compiled are always this worktree's (cargo keys units by
  workspace path). One test's root discovery breaks under it (OF-4).
* macOS/Windows hardware absent: FFI lanes compile-verified only (labels
  cited, §4 7.B/7.A); runtime verification blocked (§6).
* cargo-deny binary: pinned 0.20.2 musl tarball downloaded + SHA-256-verified
  against the CI pin this session (`/tmp/cargo-deny-0.20.2-x86_64-unknown-linux-musl/`).
* Disk budget (~1.1 GB free) is why the darwin/windows cross-target
  re-checks were not re-run at gate time (recorded, not hidden).

## 8. Re-verification battery for the Lead (at wave-24 merge)

```bash
source /home/z/my-project/scripts/cargo-env.sh && cd <worktree>
cargo fmt --check                                            # expect 0
cargo clippy --workspace --all-targets -- -D warnings        # expect 0
cargo test --workspace --no-fail-fast                        # expect 0 failures after OF-2/OF-4 fixes
cargo build -p lumen-web --target wasm32-unknown-unknown --locked   # expect 0
cargo build -p theme-tokens --target wasm32-unknown-unknown --locked # expect 0
for g in check-architecture "codegen --check" "schema-gen --check" \
         "fixtures --check" lint-literals check-identity "sbom --check"; do
  cargo run -q -p xtask -- $g || echo "FAIL: $g"
done
/home/z/…/cargo-deny --workspace check licenses sources bans advisories  # expect 0
cd ui && npm run typecheck && npm run lint && npm run test && npm run l10n:check  # expect 0×4 after OF-1/OF-3
cargo run -q -p xtask -- parity-report && cargo run -q -p xtask -- feature-audit
```

Pass criteria: all of the above green, OF-1…OF-5 closed or consciously
accepted by the Lead, and #49's browser matrix merged with its honesty pass.
