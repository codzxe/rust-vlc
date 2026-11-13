# Phase 3 Exit Gate — Engine Core (card 3.F2-V)

- **Verdict: PENDING LEAD SIGNATURE** — this is the evidence record produced by
  gate agent 7-GATE (card 3.F2-V) for the Lead's review. The Lead signs the
  final verdict after reviewing the evidence below. Nothing in this document
  is invented: every number is from a run executed this session (logs
  committed under `docs/gates/logs/`) or from a log/commit already on disk
  (cited per row). Items this session could not verify are labeled
  `[VERIFY]` or PENDING (D-4).
- **Date:** 2026-10-24 · **Agent:** 7-GATE (gate evidence; worktree
  `worktrees/w7-gate`, branch `wip/w7-gate`, based on `main @ b8cb904` + this
  branch's evidence commits) · **Final verdict owner:** Lead.
- **Baseline:** `main @ b8cb904` — the post-Wave-7-partial tree: 3.E1
  scenarios 1-8 landed + 7-a red closure (scenarios_b 23/23 green), F-1..F-5
  error-surface fixes (merge c239e57), V-1 arena pool (2df8047), V-3 in-place
  huffman (db7d151), lumen-sweep skeleton (b8cb904).
- **Structure:** modeled on `docs/gates/phase2-exit.md` (gate-question table,
  card ledger, decisions, honest pending, launch list) + the command-evidence
  table pattern of `docs/gates/phase1-exit.md` §1, filling the W5-20 skeleton
  `docs/gates/phase3-exit-draft.md`.

## 1. The doc-13 §6 gate question (MP §16), mapped to status

Doc 13 §6: the Phase-3 gate covers *"clocks, queues, generations, seeking,
source replacement, A/V sync, recovery"*, and card 3.F2-V states: **"Phase 3
gate: 16/16 scenarios green on 3 OS; soak slope within target; differential
harness operational. AC: evidence record + ledger + Phase 4 launch list."**

| Clause | Status | Evidence |
|---|---|---|
| clocks, queues, generations, seeking, source replacement, sync, recovery (7 capability families) | **MET (merged)** | 8 cards complete through wave 3 (worklog 3.WAVE3-CLOSE) + wave-5 merges dce1b64 (3.B2), a181b81 (3.C3), df75bf5 (3.D3), 6f30ac9 (3.B4 hardening); W5R-03's harder 3.A2 re-proof (`tests/source_swap.rs`, 4/4 green, cherry-picked per 6.HOUSEKEEPING). Residuals: none gate-blocking; 3.B1's two PENDING-ACs remain registered (s7 stage-time seam, s8 10 ms margin) — carried, not forgotten |
| 16/16 scenarios green | **MET (Linux, this sandbox)** | 16 scenario files `crates/playback-engine/tests/scenarios_b/scenario_01..16_*.rs` (+ `host.rs`/`main.rs`/`probe.rs` harness), doc-10 §6.4 closed 16-row list. 7-a closure: **23 passed / 0 failed, 3 consecutive runs** of the final code (5.59 s / 5.67 s / 5.54 s; merge 8d039bc; worklog 7-a). **Re-verified on THIS branch this session: 23 passed / 0 failed in 5.53 s** inside the full workspace run (§2 row 1b) |
| …green **on 3 OS** | **PENDING (infra-blocked, honest)** | This sandbox is Linux-only; CI is deliberately ONE lean ubuntu job (`.github/workflows/ci.yml`); the 3-OS matrix stays parked under `.github/workflows-disabled/` pending Lead reconciliation. Same posture as draft §5 gap 1 — recorded, never fabricated. To-run list in §4 gap 1 |
| soak slope within target | **PART (harness operational; 4 h run PENDING)** | Harness `crates/playback-engine/tests/soak/{main,memory,dispatch,scenario,report}.rs` (merge 6f792d6, 2,749 lines): open→play→seek→track-switch→reopen loop **through the command layer** (ARCH-5), two instruments (counting-allocator `live_bytes` = precise leak signal; `/proc` `VmRSS`), least-squares slope reported in MB/h with warmup exclusion. Assertion = generous 50 MB/h CI ceiling + liveness (D-6); the 5 MB/h design target is `[PROPOSAL]` — **reported, never asserted** (soak/main.rs:32-36). The CI-short 2-min soak runs in the default workspace suite — **green this session (11/11, 120.06 s, `soak_loops_stay_memory_bounded` ok = live-bytes slope under the 50 MB/h CI ceiling)**. The 4 h nightly run (`SOAK_MINUTES=240 … --release`) has **not been executed in this sandbox** (wall-clock + release-build cost); no 4 h report exists on disk → the "within target" comparison against 5 MB/h remains PENDING the first real 4 h baseline (§4 gap 3) |
| differential harness operational | **MET (operational; oracle cells PENDING-VLC)** | `tools/bench-vs-vlc/` (merge ad3ae53, +14,571 lines): Rust digest lib + `run.py` driver + `mock_vlc.py`, mock/real separation **mechanically asserted in code and labels**. Per-fixture report `docs/corpus/vlc-diff-report-20261023.{md,json}` over the 500-file regenerated corpus-big: **35 comparable rows with real Lumen decoded-PCM MD5 digests** (each cross-verified by an independent Python RIFF walk + ffprobe byte cross-check 35/35), 369 non-comparable with explicit reasons, 96 excluded by class. Oracle side = **PENDING-VLC**: no VLC 3.0.23 binary exists in this sandbox (install attempt recorded; nothing mocked — mock runs went to /tmp only). Suites green: 12 Rust + 30 Python. Comparison set per doc 10 §6.6: duration, track inventory, decoded-PCM MD5 (lossless), frame hashes at sample points, seek accuracy; non-comparables documented, not hidden |
| evidence record | **THIS DOCUMENT** | §2 command table + committed logs (`docs/gates/logs/phase3-exit-*.log`) |
| ledger | §3 | all 19 Phase-3 rows with status + branch/merge refs |
| Phase 4 launch list | §7 | concrete launch list from doc 13 §7 (waves 4.A-4.I, 25 tasks) |

## 2. Evidence record — verification commands run by 7-GATE on `wip/w7-gate`

Fresh-eyes session note: the build target dir is the config-pinned SHARED
`/home/z/my-project/target` (not worktree-local — corrected from the first
draft of this note); the lumen bin provenance was verified by a forced
rebuild from this worktree (§2 environment note).

| # | Command (as run) | Result | Log |
|---|---|---|---|
| 1 | `cargo test --workspace` (debug; first invocation) | **ABORTED (fail-fast) at `-p first-vertical --test evidence`**: `committed_transcripts_match_fresh_runs` panicked — the `lumen` bin was not at the test's relative path `<worktree>/target/debug/lumen` (worktree-path artifact: builds go to the config-pinned shared dir; see note below) | `logs/phase3-exit-workspace-test.log` |
| 1b | env fix, then `cargo test --workspace --no-fail-fast` | **1939 passed / 0 failed / 36 ignored across 163 test binaries** — incl. scenarios_b **23/23 green (5.53 s)** and soak **11/11 green (120.06 s, `soak_loops_stay_memory_bounded` ok)**. Exit 101 from exactly ONE target: `-p first-vertical --doc` doctest **compile** failure (E0463 `can't find crate for lumen_cli`, harness.rs:40) — which **PASSES on immediate targeted retry** (`running 0 tests … ok`), consistent with a transient shared-target-dir artifact under concurrent sibling builds, not a deterministic code failure [VERIFY: Lead re-run on main] | `logs/phase3-exit-workspace-test.log` (RERUN section) |
| 2 | `cargo fmt --check` (pre-fix) | **FAIL — 32 diffs across 15 files** (drift accumulated from wave-6/7 partial merges; scenario/queue_policy/sweep/tool files) | `logs/phase3-exit-fmt-initial-drift.log` |
| 3 | `cargo fmt --all` + `cargo fmt --check` (post-fix) | **CLEAN (exit 0)** — drift fixed on this branch (commit 56f135a), re-check clean. The Lead should fold this hygiene fix into the merge | `logs/phase3-exit-fmt.log` |
| 4 | `cargo clippy --workspace --all-targets -- -D warnings` (final form: `--keep-going` to enumerate all) | **NOT CLEAN — 3 targets / 16 errors, all PRE-EXISTING on main b8cb904** (none from this branch; this session's huffman fix REMOVED a 17th, see row 4b): (a) `codecs` example `stage_bench` 8× dead_code (decoder.rs:136/157/676/840/854/1014, error.rs:73/131 — the example `#[path]`-includes lib sources, so `pub(crate)` items read as dead; from the 7-b partial merge db7d151); (b) `lumen-sweep` 1× `assert!(false)` worker.rs:553 (from the 7-d skeleton merge b8cb904; note: replacing with `unreachable!`/`panic!` interacts with the F-5 check-panics gate — owner's call); (c) `playback-engine` test `scenarios_b` 6× (5× `expect()` on Option: scenario_02_play.rs:44/78/84, scenario_03_pause_resume.rs:45/92 + host.rs:535 — the known pre-existing base issue 7-a's worklog registered with a "whoever owns scenarios 2/3" handoff) | `logs/phase3-exit-clippy.log` |
| 4b | `codecs` lint fix (this session) | 7-GATE fixed the `needless_range_loop` at `crates/codecs/src/video/mjpeg/huffman.rs:94` (loop rewritten to `bits.iter().enumerate().skip(1)` — behavior-identical; **codecs suite re-verified 170 passed / 0 failed after the edit**) | commit on this branch; log §2.1 |

Environment note (recorded for reproducibility): the repo-committed `.cargo/config.toml` pins `target-dir = /home/z/my-project/target` (SHARED across all worktrees/agents — the 7-a stale-rlib caution applies). This worktree has no local `target/`; the first-vertical evidence test resolves the `lumen` binary relatively, so 7-GATE (a) force-rebuilt `-p lumen-cli` from THIS worktree (provenance line `Compiling lumen-cli v0.1.0 (/home/z/my-project/worktrees/w7-gate/apps/cli)` in the session) and (b) symlinked `<worktree>/target → /home/z/my-project/target` so the relative path resolves. Both runs' raw output is in the log. **Disk incident (ENOSPC) mid-session**: the shared target dir filled the 9.9 G rootfs to 100% during the clippy sweep (4 agents building concurrently); 7-GATE freed 2.7 G by deleting `target/debug/incremental` (regenerable cache; the DISK-INCIDENT-1 precedent's capped+cleaned pattern) — the first clippy log entries showing "No space left on device" are that transient, not code findings; the committed clippy log is the post-cleanup definitive run.

Notes on deltas vs the draft §4 command table: (a) row 1 was run without
`--all-features` and in debug, per the gate-agent brief — the draft's
`--all-features` variant was NOT re-run this session; (b) `cargo xtask gate`,
`check-architecture`, `fixtures --check`, `codegen/schema-gen --check` were NOT
re-run this session (time budget; the workspace test + clippy + fmt were the
brief's verification set) — last committed evidence remains worklog
3.WAVE3-CLOSE / 4.WAVE4-CLOSE; the Lead may re-run the aggregate gate at
sign-off. (c) CI was not re-run (no push of this branch; CI runs on main).

### 2.1 Workspace test totals

- **Rerun (this session, `wip/w7-gate` @ 56f135a + fmt fix, sources = main b8cb904
  + fmt/whitespace only): 1939 passed / 0 failed / 36 ignored across 163 test
  binaries** — `test result:` lines tally in
  `logs/phase3-exit-workspace-test.log` (RERUN section). Every test binary
  green, including:
  - `scenarios_b`: **23 passed / 0 failed (5.53 s)** — the 16/16 gate clause
    re-verified on this branch this session.
  - `soak`: **11 passed / 0 failed (120.06 s)** incl.
    `soak_loops_stay_memory_bounded` — the 2-min CI-short soak's live-bytes
    slope stayed under the asserted 50 MB/h CI ceiling. The numeric slope
    value is not printed to the log (`SOAK_REPORT` unset); the 5 MB/h target
    comparison stays PENDING the 4 h run (§4 gap 3).
  - The single non-green target is the `-p first-vertical --doc` doctest
    compile failure (E0463) — passes on immediate retry; see §2 row 1b.
- Trajectory: 1329 (Phase-2 exit) → 1718/0/20 (wave-3 merge log) → 1914/0
  (worklog 7.LEAD-RESET @ 9393b53) → **1939/0/36 (this gate run)**.
- Clippy: **NOT clean — 3 targets / 16 errors, all pre-existing on main b8cb904**
  (full file:line map in §2 row 4; owners: 7-b stage_bench example, 7-d
  lumen-sweep, 3.E1 scenarios 2/3 expect()s). This session REMOVED one error
  (huffman `needless_range_loop`, codecs re-verified 170/0). The workspace
  last verifiably clippy-clean commit is 95f4031 (pre-dates the wave-7 partial
  merges). Disposition is a Lead decision at sign-off: fold small fixes at
  merge, or dispatch to the owning in-flight cards.

## 3. Card ledger — Phase 3 (doc 13 §6, 19 rows)

Count reconciliation (draft §3 note, resolved here): the "16 Phase-3 cards"
phrasing in the dispatch counts implementation cards; doc 13 §6 lists **19
tasks** 3.A1..3.F2-V. This ledger carries all 19. All wave-5 branches were
merged zero-conflicts by the Lead (worklog 5.WAVE5-MERGE).

| ID | Card | Status | Evidence anchor |
|---|---|---|---|
| 3.A1 | seek (7-step flow, contract D4) | **COMPLETE** | worklog 3.WAVE3-CLOSE; D4 pinned at `crates/playback-engine/src/seek.rs:2,17,82-87` |
| 3.A2 | source replacement | **COMPLETE (+hard re-proof)** | worklog W3-02; W5R-03 `tests/source_swap.rs` 4/4 green (50-switch no-stale-frames, bounded memory creep ≤ 0, teardown-order journal) — cherry-picked to main per 6.HOUSEKEEPING |
| 3.A3 | track switching | **COMPLETE** | merge c0f0fe2 (33-row table + 104-pair expected_table.toml) |
| 3.B1 | A/V sync hardening | **COMPLETE (2 PENDING-ACs registered)** | merge 945e392; s7 stage-time seam + s8 10 ms margin carried |
| 3.B2 | clock backends | **COMPLETE (merged)** | branch wip/w5-01 @ 3276c7f → merge dce1b64 (5.WAVE5-MERGE) |
| 3.B3 | queue backpressure | **COMPLETE** | merge 167f6e9 |
| 3.B4 | event completeness | **COMPLETE (+wave-5 hardening merged)** | worklog 3.WAVE3-CLOSE; 12/23 emission map + bijective CATALOG_EMISSIONS → merge 6f30ac9 |
| 3.C1 | state-machine completeness | **COMPLETE** | merge 6faff0e (33-row transition table, 500×64 walks) |
| 3.C2 | media-level recovery | **COMPLETE** | merge 6556164 (tier-1 matrix 22 fixtures) |
| 3.C3 | environment recovery | **COMPLETE (merged)** | branch wip/w5-02 → merge a181b81 |
| 3.D1 | testkit v1 | **COMPLETE (folded)** | folded into W5-02 + W5-04 local fakes per worklog 5.WAVE5-MERGE (recorded decision, not a silent drop) |
| 3.D2 | fault injection | **COMPLETE (merged)** | branch wip/w5-04 → merge d3d69cf (`testkit/src/faults.rs`) |
| 3.D3 | property suites | **COMPLETE (merged)** | branch wip/w5-06 → merge df75bf5 |
| 3.E1 | scenarios 1-8 | **COMPLETE (merged)** | commit 8866aa0 (hard-AC suites, +1,308 lines) + 7-a red closure merge 8d039bc: scenarios_b 23/23 green, 3× consecutive runs |
| 3.E2 | scenarios 9-16 | **COMPLETE (merged)** | branch wip/w5-08 → merge 47d54c4 |
| 3.E3 | differential harness v1 | **COMPLETE (operational; oracle PENDING-VLC)** | branch wip/w5-09 @ c76e8fb → merge ad3ae53; report `docs/corpus/vlc-diff-report-20261023.md` (§1 row above) |
| 3.E4 | soak harness + first run | **COMPLETE (harness; 4 h nightly run PENDING)** | branch wip/w5-10 → merge 6f792d6; §1 soak row + §4 gap 3 |
| 3.F1 | perf baseline capture | **COMPLETE (merged)** | branch wip/w5-11 @ 71ef66e → merge 0e2ca87; `docs/perf/baseline.md` + `docs/perf/raw/` committed — baseline only, **no targets claimed** (doc 12 §1.1) |
| 3.F2-V | this gate | **EVIDENCE COMPLETE — verdict PENDING LEAD SIGNATURE** | this document (draft was wip/w5-20 → merge 5a411ba) |

## 4. Honest gaps — what this sandbox cannot prove at 3.F2-V

1. **3-OS scenario green (infra-blocked).** Linux-only sandbox; live CI is one
   lean ubuntu job by design; 3-OS matrix parked under
   `.github/workflows-disabled/`. To-run (the phase-2 PENDING-with-to-run-list
   pattern): (a) Lead reconciles the parked matrix with the lean job (or adds
   `os: [ubuntu-latest, macos-latest, windows-latest]`); (b) add scenarios_b
   as a required check; (c) three green runs — one per OS — on the merge
   commit; record run links here.
2. **Real-hardware tiers.** Carried unchanged from `phase2-exit.md` §4.3: no
   `real` evidence tier exists in this sandbox; audio-device and
   surface-loss scenarios run on testkit synthetic devices.
3. **4 h soak run + frozen target.** The 5 MB/h figure is `[PROPOSAL]` until
   the first measured 4 h baseline justifies and freezes it (doc 10 §6.5).
   What exists: the operational harness, a green 2-min CI-short soak in the
   workspace suite (50 MB/h CI ceiling + liveness asserted), and W5-15's
   memory audit (`docs/perf/memory-audit-20261023.md`, V-1..V-4 findings —
   V-1/V-3 already fixed by 7-c/7-b partial merges). To-run:
   `SOAK_MINUTES=240 cargo test -p playback-engine --test soak --release`
   with `SOAK_REPORT=` committed, then freeze the number.
4. **VLC 3.0.23 oracle availability.** No oracle binary in this sandbox
   (nothing mocked; separation mechanically enforced). The 35 comparable rows
   carry exact to-run commands; verdicts fill in on the first real-oracle run
   on a machine with the pinned binary.
5. **Aggregate gate re-run.** `cargo xtask gate` / `check-architecture` /
   `fixtures --check` / `codegen --check` / `schema-gen --check` not re-run
   this session (§2 note b) — Lead re-runs at sign-off or accepts the
   workspace test + clippy + fmt evidence here plus last-committed evidence.

## 5. In-flight Wave-7 optimizations (NOT gate-blocking)

The draft's stance (§2 "supporting wave-5 work … not gate-blocking on their
own") carries to wave 7: these feed **Phase-4 performance targets** (3.F1
baseline numbers exist; targets are claimed only in Phase 4 per doc 12):

| Card | Scope | Status on main @ b8cb904 |
|---|---|---|
| 7-b | MJPEG SIMD honest push (the 0.16× parity target) + V-3 in-place huffman | **PARTIAL MERGED** (db7d151): V-3 zero-alloc DHT rebuilds + ScanHooks seam + stage_bench landed; `idct_simd` stub (05f8fb3) — intrinsics work continues in-session |
| 7-c | V-1 demux arena pool + input-crate large-read path | **PARTIAL MERGED** (2df8047): V-1 arena pool landed (164→26 seam calls, PAYLOADS×2 256→0 allocs, demux 140/0); part 2 (input large-read) explicitly deferred — successor scope in `docs/perf/v1-input-readpath-20261024.md` |
| 7-d | corpus-v4 parity sweep + randomized engine sweep | **SKELETON MERGED** (b8cb904/e9febea): lumen-sweep tool (subprocess isolation, 15/15 unit tests); NEXT-STEPS breadcrumb commit 1b57222 for the relaunch session |

Also closed in wave 7 (already in main, gate-relevant): 7-a 3.E1 red closure
(8d039bc) and 7-e F-1..F-5 error-surface fixes (c239e57, closing doc
`docs/audits/error-surface-fixes-20261024.md`; `xtask check-panics` gate
catches seeded violations).

## 6. Decisions for the Lead at sign-off

- Sign the verdict (or reject with named blockers) on this evidence record.
- **Clippy debt disposition** (§2 row 4): 16 pre-existing errors on 3 targets
  from the wave-7 partial merges — fold fixes at merge or assign to the owning
  in-flight cards (7-b/7-d/scenarios 2-3).
- The 16-vs-19 card-count reconciliation (resolved in §3 — confirm).
- 3-OS matrix shape + scenarios_b as required CI check (gap 1).
- Freeze the soak target after the first 4 h run (gap 3).
- Fold this branch's fixes into main: fmt drift (56f135a) + huffman lint fix
  (§2 row 4b).
- Phase 4 launch (§7) — issue the dispatch.

## 7. Phase 4 launch list (doc 13 §7 — waves 4.A-4.I, 25 tasks)

Gate dependency: **every Phase-4 card chains from 3.F2-V** (doc 13: 4.A1
"Deps: 3.F2-V"). On gate pass, the unblocked launch set and suggested
parallelism (doc 13 prescribes 3-5 for Phase 4; UI cards own disjoint
component/screen zones, shared primitives registry Lead-merged per R3):

| Wave | Cards (deps) | Launch shape |
|---|---|---|
| 4.A UI foundation | 4.A1 ui scaffold (deps: 3.F2-V ✅), 4.A2 token pipeline (deps: 4.A1) | **Launch immediately on gate pass**: 4.A1 first (single agent — the scaffold everything chains from); 4.A2 follows 4.A1 |
| 4.B Primitive library | 4.B1 button/form (4.A2), 4.B2 slider/seekbar (4.B1), 4.B3 menus (4.B1), 4.B4 dialogs/lists/icons (4.B1) | 4.B1 first, then **4.B2+4.B3+4.B4 in parallel** (disjoint zones) — the first full 3-5 parallel wave |
| 4.C Player surface | 4.C1 shell (4.B4), 4.C2 control bar (4.C1), 4.C3 UX states (4.C1) | 4.C1, then 4.C2+4.C3 parallel |
| 4.D Playlist + info | 4.D1 playlists crate v0 + panel (4.C2), 4.D2 media-info screen (4.C2) | parallel pair |
| 4.E Settings | 4.E1 framework (4.B4), 4.E2/4.E3/4.E4 groups (4.E1) | 4.E1, then the three groups in parallel |
| 4.F Secondary + a11y | 4.F1 onboarding (4.E2), 4.F2 variants (4.C2), 4.F3 customize (4.C2), 4.F4 a11y pass (4.F1-4.F3) | 4.F1-4.F3 parallel, 4.F4 last (touches all screens) |
| 4.G Fidelity harness | 4.G1 screenshot-diff harness (4.F4), 4.G2 baseline matrix (4.G1) | sequential pair; 4.G2's 3-OS matrix hits gap 1 — same infra blocker, plan early |
| 4.H Shell/rebrand/l10n | 4.H1 Tauri shell (4.G2), 4.H2 rebrand test (4.H1), 4.H3 l10n (4.F4) | 4.H3 can run alongside 4.H1; 4.H2 after |
| 4.I Gate | 4.I1-V `[LEAD]` Phase-4 gate (4.H2, 4.H3) | Lead-owned; needs `docs/gates/phase4-exit.md` |

Wave-7 in-flight optimizations (§5) feed Phase-4 targets, not its launch —
7-b/7-c/7-d completions can proceed alongside 4.A/4.B without blocking them.

## 8. Log index (committed on this branch)

- `docs/gates/logs/phase3-exit-workspace-test.log` — `cargo test --workspace`
  (debug) full output (or in-progress; see NEXT-STEPS at file end if
  truncated).
- `docs/gates/logs/phase3-exit-clippy.log` — `cargo clippy --workspace
  --all-targets -- -D warnings` (chained after the test run).
- `docs/gates/logs/phase3-exit-fmt-initial-drift.log` — pre-fix fmt drift
  (32 diffs).
- `docs/gates/logs/phase3-exit-fmt.log` — post-fix re-check (clean).
- Prior-phase logs in the same directory (wave1/2/3-merge, merge-3a1).

## 9. LEAD SIGN-OFF (card 3.F2-V — final verdict)

- **Verdict: PASS WITH REGISTERED CAVEATS — PHASE 4 IS AUTHORIZED TO LAUNCH.**
- Signed: 2026-10-24, Lead, on `main` post-merge of all wave-7 branches
  (scenarios closure 8d039bc, F-fixes c239e57, V-1/V-3/input b8cb904+,
  gate evidence + fmt hygiene cddf0db-side merges, SIMD 3278344-side merge).
- **Lead review performed:** (a) scenarios_b 23/23 re-run by the Lead on
  merged main pre-signature (independent of 7-a/7-GATE runs); (b) the single
  workspace exit-101 (first-vertical --doc E0463, recorded §2 row 1b as
  transient) **re-verified CLEAN on main by the Lead post-merge**
  (`running 0 tests … ok`) — the shared-target-dir concurrency artifact is
  confirmed non-deterministic-infra, not code; (c) the stray worktree
  `target` symlink incident (gate agent's ENOSPC workaround, committed by
  accident) was caught at merge and reverted with the .gitignore pattern
  hardened to `/target` — no build state affects any verdict here.
- **The caveats (each already registered in this doc, restated for the
  record):** (1) 3-OS scenario matrix infra-blocked — Linux evidence only in
  this sandbox; CI stays one lean ubuntu job (§1 row 3, §4 gap 1);
  (2) 4 h soak baseline PENDING — the operational harness is green (11/11,
  2-min, live-bytes slope under the 50 MB/h CI ceiling) but no 4 h run
  exists on disk; the 5 MB/h design target stays a [PROPOSAL] until the
  first real 4 h baseline freezes it (§4 gap 3) — a merged-tree 4 h re-run
  is registered as a Phase-4-parallel follow-up; (3) differential oracle
  cells PENDING-VLC — 35 real Lumen MD5 rows stand, no VLC binary exists in
  this sandbox, nothing was mocked (§1 row 5, §4 gap 2); (4) workspace
  clippy carries 16 pre-existing errors enumerated §2 row 4 (stage_bench
  dead_code ×8, sweep assert!(false) ×1, scenarios_b expect() ×6 + host)
  — dispatch card 7-f owns their cleanup; they are lint debt on partial-
  merge scaffolding, not engine defects, and the 1939/0 test evidence
  stands independent of them.
- **Why PASS is honest at this bar:** every capability claim in the doc-13
  §6 question is backed by merged, re-runnable code with committed logs;
  every unmet clause is named, owned, and infra- or wall-clock-bound rather
  than capability-bound; nothing in this record was mocked or fabricated
  (mock/real separation is mechanically asserted in the differential
  harness). The Phase-4 launch list (§7) is the operative output.

---

## Lead sign-off (the owed square-up, 2026-11-13)

**Verdict: PASS (Lead-signed 2026-11-13).** The signature line above stood
PENDING from 2026-10-24 while the program proceeded on exactly this
evidence — Phase 4 launched on it the same day, and Phase 5's Lead
sign-off (2026-10-24, `docs/gates/phase5-exit.md`) reviewed the chain and
authorized Phase 6 with this gate's record in scope. The ROADMAP's status
note ("the signature line is owed a Lead square-up, not a re-run")
prescribes precisely this administrative ratification. Nothing is re-run
here; the evidence ledger above stands as captured. The gate's own named
debts (oracle PENDING-VLC, 4 h soak, the 16 then-open clippy errors) were
subsequently discharged by the phases that own them — soak reports now
live under `docs/perf/` (soak-nightly-20261109), the clippy debt is zero
(workspace `-D warnings` green across waves 23–26), and the VLC-oracle
differential harness has run against VLC 3.0.23 corpora in every later
phase gate. Signed: Lead (wave-26 orchestration), issue #59 square-up.
