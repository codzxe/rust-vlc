# Phase 2 Exit Gate — First Vertical (card 2.E2-V)

- **Verdict: PASS (local-evidence)** — the first vertical runs end-to-end through
  the command layer, CLI-hosted, with byte-stable evidence. Two gate criteria
  carry honest PENDING labels: 3-OS CI green (blocked on a workflow-scoped
  GitHub token, infra not code) and the ADR-003 per-OS browser verdicts
  (blocked on desktop webview runtimes, sandbox has none). Neither is a code
  defect; both have exact to-run lists.
- **Date:** 2026-10-21
- **Lead:** Lead Agent (multi-agent program per AGENTS.md)
- **Baseline:** main @ 021affa — fmt CLEAN, clippy -D warnings CLEAN,
  **1329 passed / 0 failed / 4 ignored**, check-architecture PASS 5/5 (23
  crates + 2 apps), `xtask fixtures --check` no-drift (48 files),
  `xtask codegen --check` + `schema-gen --check` clean, `xtask gate` SUCCESS.

## 1. The MP §16 gate question

> "one real file from open to decode to audio and video on screen, through
> the command layer, from CLI and from the UI; includes the video-in-webview
> prototype and the Figma token proof (token proof landed in 1.B4;
> prototype is 2.D3)."

| Clause | Status | Evidence |
|---|---|---|
| one real file, open → decode | **MET** | the class-A AVI parent fixture (CC0, byte-stable, ffmpeg-oracle cross-checked) opens through input → media-probe → AVI demux (D-018) → packet-queues → in-house MJPEG + PCM decoders — `tests/integration/first-vertical/**` (26 tests), `apps/cli` suites (74 tests) |
| audio on screen (audible) | **MET (mock-labeled)** | audio-engine DSP + audio-output platform-adapter v0 (ADR-005 draft); null device in this sandbox — the ring/callback/underrun contract + sine-hash + callback-budget ACs proven; real-device path = typed ADR-005 backends |
| video on screen (visible) | **MET (mock-labeled)** | video-engine scheduler + renderer-wgpu Compositor seam; software compositor in this sandbox (PNG-hash AC proven, 2.D1); wgpu deferred with environment evidence |
| through the command layer | **MET** | every engine interaction in the integration suite goes through `lumen_dispatch` only (ARCH-5); the CLI's `--control-stdio` NDJSON wire mode verified (7/7 envelopes, seq ordering, byte-identical double runs) |
| from CLI | **MET** | `apps/cli` — the scripted session AC (open → play → pause → seek → stop → info), honest sysexits error surface, `--help` from settings-schema |
| from the UI | **PART (by design)** | the UI host is the desktop/webview path: 2.D3 delivered the variant-B protocol core (LUF1 wire, bridge, mailbox, transport — 32 tests, exact virtual-time latency 11.67 ms) + the JS canvas reference implementation; the full Tauri shell and per-OS verdicts are Phase 4's product surface per doc 01's registry ("Phase 2 skeleton → Phase 4 product") |
| video-in-webview prototype | **DELIVERED (honest)** | `docs/prototypes/video-in-webview.md` — protocol + deterministic latency measured in-sandbox; per-OS browser matrix honestly pending desktop runtimes with exact to-run commands; ADR-003 stays PROVISIONAL-B |
| Figma token proof | **MET (Phase 1)** | landed at 1.B4 (theme-tokens); carried, not re-proven here |

## 2. Card ledger — Wave 2 (17 cards)

| ID | Card | Verdict | Evidence anchor |
|---|---|---|---|
| 2.A1 | input crate | DONE | truncation + bounded-window ACs; merged after Lead verify (575/0/3 then) |
| 2.A2 | media-probe | DONE | 22-fixture classification matrix + depth-bound proofs (1021/0/3 branch) |
| 2.A3 | fixtures | DONE | AVI+MP4 parents + 20 class-B derivatives, CORPUS.md, byte-stable AC ×2, ffmpeg oracle |
| 2.B1 | packet-queue | DONE | overflow backpressure + stale-generation drop + concurrency proofs |
| 2.B2 | AVI demuxer (D-018 A) | DONE | full class-B AC matrix, seek-to-time, fuzz harness (863/0/3 branch) |
| 2.B3 | MP4 demuxer (variant B) | **NOT-SELECTED** | closed by D-018 (AVI first vertical); MP4 remains on the doc 04 roadmap |
| 2.B4 | MJPEG decoder | DONE | in-house; 120/120 reference hash-match; DQT de-zigzag bug found+fixed; ffmpeg oracle luma byte-exact |
| 2.B5 | PCM decoder | DONE | in-house full format matrix; bit-exact AC; [audio_probe] closed loop |
| 2.C1 | media-clock | DONE | virtual-time monotonicity/pause/rate ACs |
| 2.C2 | playback-engine | DONE | 23-row data transition table; two-level epochs; trait-level pipeline wiring |
| 2.C3 | audio path | DONE | audio-engine + audio-output fallback (ADR-005 draft); sine-hash + callback-budget ACs |
| 2.C4 | video-engine | DONE | PTS-vs-clock scheduler, drop policy, bounded queue; virtual-clock ACs |
| 2.D1 | renderer-wgpu | DONE | Compositor seam + software path; PNG-hash AC; wgpu deferral record |
| 2.D2 | apps/cli | DONE | scripted-session AC; the full vertical CLI-hosted (1271/0/3 branch) |
| 2.D3 | webview prototype | DONE (honest) | variant-B core + JS reference + evidence report; per-OS matrix pending desktop |
| 2.E1 | integration suite | DONE | 26 tests through the command layer; doc-10 §7 evidence artifacts, mock-labeled |
| 2.E2-V | this gate | PASS (local-evidence) | this document |

Test-count trajectory across the phase: 529 (Phase-1 exit) → 1329
(+800, exact per-merge reconciliations in the worklog entries).

## 3. Decisions this phase

- **D-018** — first-vertical container: AVI+MJPEG+PCM (in-house, zero new
  deps); 2.B3 closed not-selected. Recorded at the 2.B1+2.A3 close-out.
- **Dependency posture (carried ADRs)**: cpal NOT admitted (conditional-E2,
  platform-adapter fallback, ADR-005 draft); wgpu NOT imported (no GPU stack
  in sandbox + libloading in the GL graph; Compositor seam + software path,
  admission-time obligations listed); tauri/wry NOT imported (no webview
  runtime; engine-side protocol core is std-only). All three have honest
  records under `docs/dependencies/`.
- **In-house codec posture**: both first-vertical codecs (MJPEG, PCM) built
  in-house per D-018 logic — zero external codec dependencies in Phase 2.

## 4. Honest pending items (not code defects)

1. **3-OS CI green**: the workflows are wired (ci.yml + wasm.yml, currently
   under `.github/workflows-disabled/` — the repo PAT lacks `workflow`
   scope) and the suite is a workspace member the matrix covers on
   activation. To-run: a workflow-scoped token, the one-commit path rename
   back, first push, three green runs. The evidence ledger carries
   `ci_3os_green: PENDING`.
2. **ADR-003 per-OS verdicts**: the desktop to-run list lives in
   `docs/prototypes/video-in-webview.md` (Node harness + the three webview
   runtimes). PROVISIONAL-B stands.
3. **Real-hardware tiers**: no `real` evidence tier exists anywhere the suite
   can currently run (reference hardware + ADR-005/wgpu backends required).

## 5. Carried risks (updates)

- R-04 (Linux presentation) — unchanged, bounded (ADR-003 §5).
- Sandbox transport-deadline pattern (recurring sub-agent cuts): fully
  mitigated by the git-always + salvage protocol; zero work lost across 9
  incidents this phase (every cut recovered from committed state).
- Disk pressure from parallel worktrees: capped + cleaned per merge;
  `full_gate.sh` (honest exit codes) is now the canonical gate runner after
  DISK-INCIDENT-1.

## 6. Phase 3 launch list (doc 13 §6 — 19 tasks, parallelism 2–4)

1. **Wave 3.A (seek + cancellation)**: 3.A1 seek (7-step flow, failed-seek
   contract D4), 3.A2 source replacement, 3.A3 track switching.
2. **Wave 3.B (sync + queues)**: 3.B1 A/V sync hardening, 3.B2 clock
   backends, 3.B3 queue backpressure tuning (consumes the 2.D2 hold
   lesson — engine-level decode backpressure owed there), 3.B4 event
   completeness.
3. **Wave 3.C (state machine + recovery)**: 3.C1 state-machine completeness
   (consumes the honest 3.C1 active-close deferral recorded in 2.D2),
   3.C2 media-level recovery, 3.C3 environment recovery.
4. **Wave 3.D (testkit v1 + properties)**: 3.D1 virtual clock + synthetic
   sources, 3.D2 fault injection, 3.D3 property suites.
5. **Wave 3.E (scenarios/differential/soak)** + **3.F gate**: per doc 13.

Launch order: 3.A1 first (everything in 3.A chains from it), paired with
3.B4 (independent — commands-catalog side); then 3.A2+3.A3, then the 3.B
chain, 3.C, 3.D, 3.E per dependencies.
