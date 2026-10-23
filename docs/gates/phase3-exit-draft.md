# Phase 3 Exit Gate — DRAFT (preparation for card 3.F2-V)

- **Verdict: DRAFT — not a gate verdict.** This skeleton is the wave-5
  preparation for the Lead's `docs/gates/phase3-exit.md` (card 3.F2-V). Every
  wave-5 slot is **PENDING** and labeled with its owning branch; no result in
  this document was measured or invented here (D-4). The Lead fills each slot
  from the owning branch's evidence at the merge session, then issues the
  verdict.
- **Date:** 2026-10-23 · **Author:** W5-20 (audit + gate-prep; branch `wip/w5-20`)
- **Baseline:** `main @ a63fe22` (wave-5 dispatch base).
- **Structure:** modeled on `docs/gates/phase2-exit.md` (gate-question table,
  card ledger, decisions, honest pending, launch list) with the command-evidence
  table pattern of `docs/gates/phase1-exit.md` §1.

## 1. The doc-13 §6 gate question (MP §16), mapped to status

Doc 13 §6 (lines 357-361): the Phase-3 gate covers *"clocks, queues,
generations, seeking, source replacement, A/V sync, recovery"*, and card 3.F2-V
(doc 13 line 411) states: **"Phase 3 gate: 16/16 scenarios green on 3 OS; soak
slope within target; differential harness operational. AC: evidence record +
ledger + Phase 4 launch list."**

| Clause | Status | Evidence / owning branch |
|---|---|---|
| clocks, queues, generations, seeking, source replacement, sync, recovery (the seven capability families) | **MET (merged)** | 8 Phase-3 cards complete through wave 3 — worklog 3.WAVE3-CLOSE ("3.A1-A3, 3.B1 (2 pending-ACs registered), 3.B3, 3.B4, 3.C1, 3.C2 COMPLETE"); merges c0f0fe2 (3.A3), 945e392 (3.B1), 167f6e9 (3.B3), 6faff0e (3.C1), 6556164 (3.C2) + worklog W3-02 (3.A2). Residuals: 3.B2/3.C3 below. |
| 16/16 scenarios green | **PENDING** | scenarios 1-8 (3.E1) = `wip/w5-07`; scenarios 9-16 (3.E2) = `wip/w5-08`. Scenario authority: doc 10 §6.4's closed 16-row list (open, play, pause/resume, seek, stop, repeated seeks, A/V sync, track switching, corrupt, missing, interrupted network, device loss, long, short, reopen cycles, exit during playback) — adding one is a doc change, removing one needs Lead sign-off (doc 10 §6.4, "closed" note). |
| …green **on 3 OS** | **PENDING (infra-blocked)** | CI is live but deliberately ONE lean ubuntu job (`.github/workflows/ci.yml:22-26, 68`); the 3-OS matrix remains parked under `.github/workflows-disabled/` "pending Lead reconciliation" (ci.yml:23-26). See §5 gap 1. |
| soak slope within target | **PENDING** | soak harness + first 4 h run (3.E4) = `wip/w5-10`. Target < 5 MB/h is `[PROPOSAL — number to be justified by first measured baseline, then frozen]` (doc 10 §6.5). |
| differential harness operational | **PENDING** | differential-vs-VLC harness v1 (3.E3) = `wip/w5-09`; comparison set per doc 10 §6.6 (duration, track inventory, decoded-PCM MD5 on lossless paths, frame hashes at sample points, seek accuracy; non-comparables recorded, not hidden). |
| evidence record | **THIS DRAFT →** Lead's `phase3-exit.md` | gate command table §4; supporting wave-5 measurement inputs §2. |
| ledger | §3 | all Phase-3 cards with status + branch refs. |
| Phase 4 launch list | **PENDING (Lead)** | doc 13 §7 already defines waves 4.A-4.I (25 tasks, first cards gated on 3.F2-V); the Lead issues the concrete launch list at finalize. |

## 2. Wave-5 measurement inputs (every slot PENDING, each with its owner)

| Deliverable | Owning branch | Card | Feeds gate clause |
|---|---|---|---|
| Scenario suite part 1 (scenarios 1-8) | `wip/w5-07` | 3.E1 | 16/16 scenarios green |
| Scenario suite part 2 (scenarios 9-16) | `wip/w5-08` | 3.E2 | 16/16 scenarios green |
| Differential-vs-VLC harness v1 + per-fixture report | `wip/w5-09` | 3.E3 | differential harness operational |
| Soak harness + first 4 h run + memory slope | `wip/w5-10` | 3.E4 | soak slope within target |
| Perf baseline capture (`docs/perf/baseline.md` per doc 13 3.F1 row: startup, probe latency, time-to-first-frame/sound, seek latency, CPU/GPU, peak memory, dropped frames, underruns + hardware manifest; metrics M-01… per doc 12 §2) | `wip/w5-11` | 3.F1 | gate input (baseline numbers committed, no targets claimed) |
| Memory audit | `wip/w5-15` | — (wave-5 audit) | soak-slope corroboration + leak triage list (doc 10 §6.5) |
| Parity audit (Lumen-vs-ffmpeg; denominator = `docs/corpus/speed-baseline-20261022.md`, first numbers 1.18×/0.75×/0.16× per worklog 4.WAVE4-CLOSE) | `wip/w5-17` | — (wave-5 audit) | honest-optimization record feeding Phase-4 targets |
| Corpus sweep | `wip/w5-18` | — (wave-5 measurement) | randomized real-media evidence (D-11) across corpus-big (485 files, manifest `docs/corpus/corpus-big-manifest-20261022.json`) |

Supporting wave-5 work (context for the ledger; not gate-blocking on their own):
3.B2 clock backends = `wip/w5-01` (commit 3276c7f "feat(3.B2): clock backends —
video/external masters, master-loss fallback, Ended seek-back"); 3.C3
environment recovery = `wip/w5-02` (worktree evidence: new
`crates/playback-engine/src/recovery.rs` + `tests/recovery_env.rs`;
[VERIFY] uncommitted at draft time); 3.D1 testkit v1 = `wip/w5-03`
(`testkit/src/{clock,decoders,script}.rs`; [VERIFY] uncommitted); 3.D2 fault
injection = `wip/w5-04` (`testkit/src/faults.rs`; [VERIFY] uncommitted); 3.D3
property suites = `wip/w5-06` (`packet-queue/tests/property.rs`,
`playback-engine/tests/property/`; [VERIFY] uncommitted); event-emission
completeness map (3.B4 hardening: 12/23 emitted rows, `CATALOG_EMISSIONS`
bijective map) = `wip/w5-05` ([VERIFY] uncommitted); MJPEG decoder work =
`wip/w5-12` and `wip/w5-13` (the named 0.16× optimization target, worklog
4.WAVE4-CLOSE); AVI demux optimize = `wip/w5-14` (commit 3b1a9e2; worklog W5-14 records it
as pushed; walk 1.84×-10.8×, idx1 filtered-stream fix, 137/0/4); corpus
extension = `wip/w5-16` [VERIFY]; audio convert/resample/PCM optimization =
`wip/w5-19` [VERIFY]; engine error-surface audit + this gate draft = `wip/w5-20`
(`docs/audits/error-surface-20261023.md`).

## 3. Card ledger — Phase 3 (doc 13 §6)

Eight cards were complete before wave 5 (worklog 3.WAVE3-CLOSE; commit
d9218e8 "Phase 3 8/16 cards complete, audit register fully closed"); the
wave-5 completions are **PENDING-MERGE** with branch refs. Count note for the
Lead [VERIFY]: the dispatch and commit d9218e8 say "16 Phase-3 cards", while
doc 13 §6 lists **19 tasks** (3.A1..3.F2-V); this ledger carries all 19 rows —
reconcile the denominator at finalize.

| ID | Card | Status | Evidence anchor |
|---|---|---|---|
| 3.A1 | seek (7-step flow, contract D4) | **COMPLETE** | worklog 3.WAVE3-CLOSE; D4 "never a fake success" pinned at `crates/playback-engine/src/seek.rs:2, 17, 82-87` |
| 3.A2 | source replacement | **COMPLETE** | worklog W3-02 (salvage continuation of W2-11); merged per 3.WAVE3-CLOSE |
| 3.A3 | track switching | **COMPLETE** | merge c0f0fe2 (worklog 3.WAVE3-CLOSE: 7-step flow, decoder cache, 33-row table + 104-pair expected_table.toml) |
| 3.B1 | A/V sync hardening | **COMPLETE (2 pending-ACs)** | merge 945e392 (W3-04; M12 closed); s7 (stage-time fill observation seam) + s8 (10 ms margin investigation) registered as PENDING-ACs — worklog 3.WAVE3-CLOSE |
| 3.B2 | clock backends | **PENDING-MERGE** | `wip/w5-01` @ 3276c7f (feat commit on branch; unmerged at draft time) |
| 3.B3 | queue backpressure | **COMPLETE** | merge 167f6e9 (W3-05; starvation counters, M4 root killed) |
| 3.B4 | event completeness | **COMPLETE (+wave-5 hardening)** | worklog 3.WAVE3-CLOSE; `wip/w5-05` carries the 12/23 emission map + bijective `CATALOG_EMISSIONS` (PENDING-MERGE) |
| 3.C1 | state-machine completeness | **COMPLETE** | merge 6faff0e (W3-11; 33-row transition table, 500×64 model walks) |
| 3.C2 | media-level recovery | **COMPLETE** | merge 6556164 (W3-12; G1 format info, G3 bounded audio skip, tier-1 matrix 22 fixtures) |
| 3.C3 | environment recovery | **PENDING-MERGE** | `wip/w5-02` (recovery.rs + recovery_env.rs in worktree; [VERIFY] uncommitted at draft time) |
| 3.D1 | testkit v1 (virtual clock, synthetic sources) | **PENDING-MERGE** | `wip/w5-03` ([VERIFY] uncommitted) |
| 3.D2 | fault injection | **PENDING-MERGE** | `wip/w5-04` ([VERIFY] uncommitted) |
| 3.D3 | property suites | **PENDING-MERGE** | `wip/w5-06` ([VERIFY] uncommitted) |
| 3.E1 | scenarios 1-8 | **PENDING** | `wip/w5-07` |
| 3.E2 | scenarios 9-16 | **PENDING** | `wip/w5-08` |
| 3.E3 | differential harness v1 | **PENDING** | `wip/w5-09` |
| 3.E4 | soak harness + first run | **PENDING** | `wip/w5-10` |
| 3.F1 | perf baseline capture | **PENDING** | `wip/w5-11` |
| 3.F2-V | this gate | **IN PREP** | draft = `wip/w5-20` (this file); finalize + verdict = Lead |

## 4. Gate command table (exact commands + what each proves)

Pattern: `docs/gates/phase1-exit.md` §1 (command | result | evidence) and the
`phase2-exit.md` baseline line (fmt/clippy/tests/arch/fixtures/codegen/schema +
`xtask gate` SUCCESS). The Lead runs these on the post-merge `main` commit;
"last committed evidence" anchors are from the wave-3 close because the
wave-4 full-gate log was not committed (§5 gap 5).

| # | Command | What it proves | Last committed evidence | At finalize |
|---|---|---|---|---|
| 1 | `cargo xtask gate` | the aggregate local gate (10-step table, `tools/xtask/src/cmd/gate.rs:10-21`; engine subset while ui/tauri/wasm targets are absent — absent steps SKIP honestly, never pass silently) | SUCCESS (worklog 3.WAVE3-CLOSE) | re-run |
| 2 | `cargo fmt --all -- --check` | style gate | CLEAN (worklog 3.WAVE3-CLOSE) | re-run |
| 3 | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | lint block incl. the panic-surface denies (`Cargo.toml:49-56`); note the verified `unreachable!` blind spot — `docs/audits/error-surface-20261023.md` §0 + fix F-5 | CLEAN (worklog 3.WAVE3-CLOSE) | re-run |
| 4 | `cargo test --workspace --all-features` | unit + integration + doctests (121 `Running` test binaries + doctest sections incl. `Doc-tests demux/codecs/…`, `docs/gates/logs/wave3-merge.test.log`) | 1718 passed / 0 failed / 20 ignored (`docs/gates/logs/wave3-merge.test.log`) | re-run; count moves with wave-4/5 merges (demux 129→133 @ W4-R1 → 137 @ W5-14; CLI decode-bench suites @ W4-R2) |
| 5 | `cargo xtask check-architecture` | boundary/cycle/unsafe/static-mut/testkit-direction checks (5/5) | PASS 5/5 (worklog 3.WAVE3-CLOSE) | re-run |
| 6 | `cargo xtask fixtures --check` | byte-stable fixture set, no drift | no-drift, 55 files (worklog 4.WAVE4-CLOSE) | re-run |
| 7 | `cargo xtask codegen --check` + `cargo xtask schema-gen --check` | commands-catalog + settings-schema drift gates | clean (phase-1/2 gates, `docs/gates/phase1-exit.md` §1 rows 6-7) | re-run |
| 8 | scenario suites 1-16 (3.E1/3.E2 harness entry points) | **the 16/16 gate clause** — per-scenario junit + `transcripts/<scenario>.json` evidence (doc 10 §6.4) | — | **PENDING `wip/w5-07` + `wip/w5-08`** |
| 9 | soak run (4 h loop, RSS + queue-resident-bytes slope) | **the soak-slope gate clause** (doc 10 §6.5; < 5 MB/h `[PROPOSAL]` until first baseline freezes it) | — | **PENDING `wip/w5-10`** |
| 10 | differential run (pinned VLC 3.0.23, external) | **the differential gate clause** — per-fixture report, non-comparable list recorded (doc 10 §6.6) | — | **PENDING `wip/w5-09`** |
| 11 | perf baseline artifact | 3.F1 AC: baseline numbers committed, no targets claimed (doc 12 §1.1) | — | **PENDING `wip/w5-11`** (`docs/perf/baseline.md`) |
| 12 | CI on the merge commit | repo-parity of the local gate (ci.yml replicates `scripts/full_gate.sh`'s five cargo commands verbatim, ci.yml:12-20) | run #1 SUCCESS (da2e830), run #2 in flight at 3.WAVE3-CLOSE; single-OS (§5 gap 1) | re-run; 3-OS = gap |

## 5. Honest gaps — what this sandbox cannot prove at 3.F2-V

1. **3-OS scenario green.** The live workflow is one lean ubuntu job by design
   (`.github/workflows/ci.yml:22-26`: "deliberately ONE lean ubuntu job: no
   --release, no 3-OS matrix, no artifact uploads"); the full matrix design
   stays parked under `.github/workflows-disabled/` "pending Lead
   reconciliation with this file" (ci.yml:23-26). To-run list (the established
   PENDING-with-to-run-lists pattern from `phase2-exit.md` §4): (a) reconcile
   the parked matrix with the lean job (or extend ci.yml with an
   `os: [ubuntu-latest, macos-latest, windows-latest]` strategy); (b) add the
   scenario suites (3.E1/3.E2) as required checks; (c) three green runs — one
   per OS — on the merge commit; record run links in the final gate doc.
2. **Real-hardware tiers.** Carried unchanged from `phase2-exit.md` §4.3: no
   `real` evidence tier exists in this sandbox (reference hardware +
   ADR-005/wgpu backends required); audio-device and surface-loss scenarios run
   on the testkit's synthetic devices, and the 4 h soak needs a desktop-class
   runtime to be more than synthetic.
3. **Soak target is not frozen.** "< 5 MB/h" is `[PROPOSAL]` until the first
   measured baseline justifies and freezes it (doc 10 §6.5) — the gate clause
   "within target" must cite `wip/w5-10`'s measured slope AND the then-frozen
   number.
4. **VLC 3.0.23 oracle availability.** Doc 10 §6.6's clean-room rules pin the
   oracle externally; whether the pinned binary is reachable in the merge
   sandbox is `wip/w5-09`'s to state (mock runs must stay separated from
   real-oracle runs per the 3.E3 card text).
5. **Wave-4 full-gate log uncommitted.** 4.WAVE4-CLOSE ran the full gate in
   background (`logs/gate-w4.log`) but no such file is committed under
   `docs/gates/logs/` (directory listing shows wave1-3 + merge-3a1 logs only);
   the finalize pass re-runs the gate on the merge commit regardless.
6. **This draft is evidence-only.** Every PENDING slot names its owning branch
   (§2, §3); sibling-branch states were recorded read-only on 2026-10-23 and
   marked [VERIFY] where uncommitted. No scenario, soak, differential, or perf
   number appears in this document because none has been merged (D-4: never
   fabricate; AGENTS.md §3.4).

## 6. Decisions this phase should record at finalize (Lead)

- The 16-vs-19 card-count reconciliation (§3 note).
- Whether F-1..F-5 from `docs/audits/error-surface-20261023.md` §3 (the one
  REAL-RISK `expect`, the contract-module gating, the `cap()` const form, the
  `unknown` descriptor const, the gate-side `unreachable!` grep) land as
  Phase-4 hardening cards or are closed in-wave.
- The frozen soak number (gap 3) and the 3-OS matrix shape (gap 1).

## 7. Phase 4 launch list (placeholder — Lead issues at finalize)

Doc 13 §7 defines Phase 4 (waves 4.A-4.I, 25 tasks; parallelism 3-5), first
cards depending on 3.F2-V (e.g. 4.A1 ui scaffold "Deps: 3.F2-V"). The
concrete launch table — card, scope, branch, owner — is the Lead's finalize
output, per the phase-1/2 exit precedent (§6 of both gate docs).
