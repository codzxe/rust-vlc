# WAVE 3 — SUMMARY

Date: 2026-11-21 · Agents W03-A01…A10 · Domain: engine/playback pipeline —
playback-engine, media-clock, packet-queue, video-engine, input, platform-*,
renderer-wgpu.
Gate: **PASS — 110/110 assignments audited, 0 missing, 0 thin.**
(Infra note: A08/A09 first spawns hit a platform-level timeout before any
result returned; A08's partial work had persisted in its worktree and was
completed by the retry — the worktree design absorbed the failure.)

## Verdicts (file-level)
REAL 62 · REAL-UNPROVEN 3 · UNREACHABLE 36 · SPLIT 1

## Findings
**S0:0 S1:1 S2:17 S3:99 S4:164 — total 281**

## Systemic findings (multi-agent)
1. **The entire platform-adapter layer is orphaned.** All three `platform-*`
   crates (windows, linux, macos) have zero dependents — no shipped binary
   wires them in. Their "compile-verified"/"runtime-tested" claims had no
   committed execution evidence (W03-A01/A02/A03/A04/A08/A09/A10).
2. **media-clock's card-3.B2 surface (~530–1000 LOC, 30+ green tests) has
   zero production callers** — master-loss fallback, video steering,
   Ended-latch, seek_back/mark_ended: inert capability, doc 03 §4.7
   ClockMasterChanged emission is paper-only (W03-A02/A04/A05).
3. **The recovery/hotplug layer is driven only by tests** — audio-device
   loss, source-retry, surface-loss, renderer-backend report; catalog rows
   AudioDeviceLost/Changed/VideoBackendChanged "Emitted" can never fire in
   the product (W03-A10).

## Worst findings (citations verified at gate)
1. **[S1, EXECUTED] shipped `lumen` panics (exit 101) on non-UTF-8 argv** —
   `std::env::args()` at apps/cli/src/main.rs:14 unwraps before any typed
   refusal; voids the crate's "recorded v0 limitation" path and main.rs's
   own "Std-only, no panics" header. Execution pasted in
   audit/files/crates/input/src/lib.rs.audit.md F-001. (W03-A09)
2. **[S2] audio-track switch wedges playback** — clock paused, nothing
   resumes it (playback-engine tracks.rs). (W03-A08)
3. **[S2] `MetalDevice` documented "released on Drop" in three places, no
   Drop impl codebase-wide** — every `retain_device_at` leaks a retained
   MTLDevice; the in-code SAFETY comment is false. (W03-A03, metal.rs)
4. **[S2×3] pulse.rs `pump_once(...)?` paths skip all documented FFI
   cleanup** — two leave callback userdata dangling. (W03-A03)
5. **[S2] wasapi.rs `enumerate()` joins its COM thread with no deadline on
   the Windows startup path** — a hung audio driver blocks app launch
   forever; violates the crate's own D-2 no-unbounded-waits directive.
   (W03-A01)
6. **[S2] FIFO open hangs the CLI's synchronous open path** —
   `std::fs::File::open` blocks on an unwritten FIFO before the metadata
   check that was supposed to be the typed rejection. (W03-A06,
   input/src/file.rs)
7. **[S2] media-clock sync.rs `for_shift_and_window` accepts unbounded
   shift** — shift-overflow panic (debug) / silent mask (release) inside the
   A/V-sync EMA. (W03-A05)
8. **[S2] journey_seek fails on the real pipeline** — "the decode
   accounting keeps the pre-seek deliveries — observed 90, expected 154";
   one of the 7 hidden baseline failures; orchestrator re-ran with
   `--nocapture`, confirmed verbatim. (W03-A07;
   orchrun-test-journey-seek.log.txt)
9. **[S2] take-screenshot / step-frame are unwired component sets** —
   catalog-visible capabilities with no host wiring. (W03-A04)

## REQUEST-ORCH-RUN outcomes
- `cargo check -p platform-macos --target aarch64-apple-darwin` and
  `--target x86_64-apple-darwin` → **both EXIT=0**
  (orchrun-platform-macos-crosscheck.log.txt). The "compile-verified" claims
  are now verifiably true — but they had no committed evidence when made;
  the evidence-gap findings stand, the capability claim is repaired.
- `cargo test -p input --features contract --release --locked` → **6 passed
  / 0 failed** (orchrun-test-input-contract.log.txt) — first committed
  execution of the truncation AC suite.
- `cargo test -p first-vertical --test journey journey_seek -- --nocapture`
  → **FAILED as suspected** (observed 90 vs expected 154)
  (orchrun-test-journey-seek.log.txt).

## Citation spot-check (§7.3 step 4)
12 audit files sampled: **PASS 25 · FAIL 0. Fabrications: 0.**

## Blocked / UNVERIFIED carried forward
- Windows WASAPI / PulseAudio runtime behavior (no server/host in sandbox).
- macOS runtime behavior (cross-compile check only).
- Real-GPU DRM connector impact (renderer-wgpu) — no GPU in sandbox.
- Debug-profile test lane (dev-profile build needs ~6 GB; sandbox disk).

## Orchestrator machine note
`CARGO_TARGET_DIR=/home/z/my-project/target` (shared, outside the repo) is
the convention; the baseline `lumen` release binary is preserved there for
smoke probes. Cargo artifacts are trimmed between waves to keep 10 parallel
worktrees within the 9.9 GB sandbox disk.
