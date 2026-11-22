# AUDIT: apps/cli/tests/pipeline_accounting.rs

| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-22 |
| Lines / functions | 201 / 3 helpers + 3 tests |
| File verdict | REAL-UNPROVEN (real in-process integration; all pins runtime-UNPROVEN — the binary never ran in the W00 baseline) |
| Reachable from shipped entry point? | N/A (test). Drives the real `lumen_dispatch` + `CliHost` + engine + renderer in-process. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | `rg` verification of every host API used (`screenshot_png` host.rs:430, `HeadlessAccounting` fields host.rs:114-130); fixture existence (verified in earlier audits). NO cargo (adaptation). |

## 1. One-paragraph verdict

Three strong in-process acceptance tests over the committed AVI fixture: whole-media accounting closed to the exact frame (120 deliveries, 60 scheduled, zero refusals, staging bytes = frames × 320×240×4), an in-process determinism proof that byte-compares the FINAL FRAME PNG across two full runs, and the native-seek accounting variant (90 accepted / 45 pushed / 15 stale — the not-reopened arithmetic). Every host API it calls exists (verified). The only systemic defect is the inherited one: this binary never ran in the W00 baseline, so the determinism proof and the exact accounting pins — precisely the evidence a matrix row would cite for "engine-driven first vertical" — have no recorded green run. Secondary: the accounting pins are coupled to the settle pump's decode-ahead constants (same class as native_seek.rs F-003).

## 2. What actually works (verified)

- All host APIs used exist: `screenshot_png` (host.rs:430), `HeadlessAccounting` fields `video_presented`/`scheduler_presented`/`scheduler_repeats`/`staging_frames`/`staging_bytes`/`staging_copies` (host.rs:114-130), plus `settle`/`advance`/`state_history`/`position_us`/`session_id` (verified in the native_seek audit).
- Fixture: `tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` (committed, verified).
- The staging-bytes assertion (pipeline_accounting.rs:109-113) is a real memory-behavior pin: exactly one full-frame RGBA copy per present — would catch accidental double-copies or dropped presents.
- The determinism test (134-142) compares four aggregates including the PNG bytes — the strongest form of determinism evidence available in-process.

## 3. Findings

### F-001 [S2] — No recorded run: the accounting/determinism evidence is unexecuted

Where: whole file; evidence: audit/runs/baseline/cargo-test.log (116 binaries, `pipeline_accounting` absent)

Code (pipeline_accounting.rs:135-141, verbatim):
```rust
fn whole_media_runs_are_deterministic() {
    let first = run_to_end();
    let second = run_to_end();
    assert_eq!(first.0, second.0, "state trails");
    assert_eq!(first.1, second.1, "positions");
    assert_eq!(first.2, second.2, "accounting");
    assert_eq!(first.3, second.3, "final frames (PNG bytes)");
```

What is wrong: the W00 baseline stopped fail-fast at `first-vertical` (cargo-test.log:2499) and never reached any lumen-cli test binary. This suite — the "pipeline-accounting AC" its module doc names as the acceptance criterion for the engine-driven first vertical — has no recorded execution against the current tree; its exact pins (120/60/90/45/15, PNG bytes) are numbers nobody has seen a harness produce since the tree changed.
Why it matters: §3.4 — acceptance-shaped tests that have not run are not acceptance evidence.
What correct looks like: baseline re-run with `--no-fail-fast`.
Confidence: HIGH — Verification: baseline binary inventory.

### F-002 [S4] — Accounting pins coupled to decode-ahead constants

Where: pipeline_accounting.rs:187-195

Code (pipeline_accounting.rs:187-190, verbatim):
```rust
    assert_eq!(accounting.frames_accepted, 90, "{accounting:?}");
    // The scheduler saw the pre-seek window (16 — the window cap)
    // plus the landing's 30.
    assert_eq!(accounting.scheduler_pushed, 45, "{accounting:?}");
```

What is wrong: same class as native_seek.rs F-003 — the numbers encode the settle pump's decode-ahead (30) and the scheduler window cap (16→15 after purge); any tuning of those constants breaks the test without a user-visible regression. The comments derive each number honestly, but the brittleness is real.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parent` | 15-23 | trivial-correct | — | Committed fixture. |
| `dispatch_open` | 25-41 | trivial-correct | — | Real dispatch + settle. |
| `run_to_end` | 46-69 | REAL-UNPROVEN | — | Whole-media pump to `Ended` + screenshot. |
| `whole_media_accounting_closes` | 74-130 | REAL-UNPROVEN | S2 (F-001) | Exact frame/scheduler/staging pins; PNG magic check. |
| `whole_media_runs_are_deterministic` | 134-142 | REAL-UNPROVEN | S2 (F-001) | Byte-identical PNGs across runs. |
| `native_seek_accounting_closes` | 151-201 | REAL-UNPROVEN | S2 (F-001)/S4 (F-002) | The not-reopened arithmetic (90 ≠ 60). |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 3 | YES | Exact counters, staging-byte math, byte-identical PNGs, epoch survival. (All runtime-UNPROVEN — F-001.) |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the whole first vertical — access → probe → AVI demux → … → software renderer — driven by the engine through the ONE command layer" | :1-5 | REAL-UNPROVEN | The test structure does drive all of it in-process; unexecuted (F-001). |

## 7. Dead / duplicated / unreachable code

- None. No corpus gating.

## 8. Dependencies & flags

- Library-level integration (`lumen_cli::dispatch`/`host`); no feature gates; committed fixture only.

## 9. Open questions for the next agent

- The renderer's `screenshot_png` path (renderer-wgpu, another wave) is the evidence source for the PNG assertions — if that crate's audit finds nondeterminism in GPU-less rendering, this suite's determinism test is the canary.
