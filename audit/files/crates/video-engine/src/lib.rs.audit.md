# AUDIT: crates/video-engine/src/lib.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 199 / 0 functions — crate root: module docs (1-173), 5 mod decls, 1 cfg(test) mod, 7 re-export groups, 1 doctest |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — the crate it roots is on the shipped video path: apps/cli/src/pipeline_host.rs:1300 (`SchedulerCfg::for_frame_duration`), :1319 (`PresentationQueue::with_generation`), :1324 (`PresentationScheduler::new`), apps/desktop/src/bridge.rs:41 (`Presenter`). This file itself is compile glue + docs. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 (zero-findings disclosure in §2) |
| Commands run | rg `unsafe|todo!|unimplemented!` and `VideoFilter` over crates/video-engine/src; rg re-export names against their defining modules; sed verification of quotes. No cargo (adaptation). |

## 1. One-paragraph verdict

A crate root whose value is its documentation: 173 lines of unusually honest scope statements ("What this crate is not (yet)", the filter chain and scheduler-thread absences disclosed by name) plus a real, executable doctest that wires scheduler + queue + presenter over virtual time and asserts in-order delivery. No functions, no logic, no unsafe, no stubs — every re-export resolves to a real item in its module. The one soft spot is a doc sentence that assigns the pacing duty to "the playback engine" with a "(vsync callback or timer)" — true in desktop-tauri, absent in the shipped CLI, which is the same integration gap recorded as S0 in my clock.rs audit.

## 2. What actually works (verified — zero-findings disclosure per §13)

Checked, with method:
- **No unsafe, no todo!/unimplemented! anywhere in the crate** (rg over src/: only doc-comment mentions at adjust.rs:36 and lib.rs:173). Matches the "Std-only by C5: zero external dependencies, no unsafe" claim (172-173).
- **Dependency claim**: Cargo.toml `[dependencies]` = media-types + media-clock only; no GPU/window/surface crates (doc 15-16 claim holds at the manifest level).
- **All re-exports resolve**: policy items (policy.rs: DropPolicy 41, FrameDecision 92, SchedulerCfg 143, SchedulerCfgError 111, SyncPolicy 355, DEFAULT_MAX_PENDING 174, DEFAULT_MAX_REPEAT 196), queue items (queue.rs: EnqueueOutcome/PresentationQueue/QueueStats/DrainError), scheduler items (scheduler.rs: PresentAction/PresentationScheduler/PushOutcome/SchedulerStats/SyncSample/TickDrops/TickOutcome/DropTrail/MAX_DROP_TRAIL — all seen in scheduler.rs grep), sink items (sink.rs: PresentInfo/PresentRequest/Presenter — imported as such at pipeline_host.rs:84-86, bridge.rs:41), adjust items (adjust.rs). No phantom exports.
- **`#[cfg(test)] mod test_support`** (181-182) is test-only — not compiled into shipped binaries.
- **Doctest** (95-167): logic walked by hand — virtual monotonic source advanced to each frame's slot before tick; three frames → three in-order presents; the assert values (0, 33_333_334, 66_666_668) match the arithmetic. Execution UNVERIFIED (no cargo); part of W00's 1751-passing baseline.

## 3. Findings

None. The file contains no executable logic beyond the doctest; everything checkable was checked (§2). The integration gap noted below is filed against the integration (see clock.rs audit F-001), not against this file's content.

## 4. Per-function table

Not applicable — no functions. Declaration inventory:

| Item | Lines | Verdict | Notes |
|---|---|---|---|
| Module docs | 1-173 | REAL | Honest absences; see §6 for claim ledger. |
| `pub mod adjust/policy/queue/scheduler/sink` | 175-179 | REAL | All five modules exist with real content (LOC: adjust 355, policy 747, queue 388, scheduler 1711, sink 103). |
| `#[cfg(test)] mod test_support` | 181-182 | REAL | Test-only gate correct. |
| `pub use adjust::{AdjustError, AdjustParams}` | 184 | REAL | Resolves. |
| `pub use policy::{…7 items}` | 185-188 | REAL | Resolves. |
| `pub use queue::{DrainError, EnqueueOutcome, PresentationQueue, QueueStats}` | 189 | REAL | Resolves. |
| `pub use scheduler::{…9 items}` | 190-193 | REAL | Resolves. |
| `pub use sink::{PresentInfo, PresentRequest, Presenter}` | 194 | REAL | Resolves; consumed by desktop bridge. |
| `pub use media_clock::NANO_BASE` | 199 | REAL | Convenience re-export; used as `video_engine::NANO_BASE` at bridge.rs:315/320 and tests. |
| Doctest | 95-167 | REAL-UNPROVEN | Logic verified by hand; execution UNVERIFIED (no cargo). |

## 5. Test honesty

The doctest is the only test in this file. It asserts real observable behavior (3 delivered, exact in-order PTS values), not a tautology. Would fail if the scheduler mis-ordered or the queue dropped.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no GPU, window or surface code" (15-16) | crate doc | REAL | Cargo.toml deps; no such APIs in src (rg). |
| "The video filter chain … a later card; the pipeline seam is reserved" (20-21) | crate doc | REAL | No `VideoFilter` type exists anywhere in the crate (rg: only the doc mention) — the absence is disclosed, not papered over. |
| "The scheduler thread …: production wiring — the playback engine hosts this state machine and paces it (vsync callback or timer). v0 is the deterministic, explicitly stepped core" (26-30) | crate doc | SHALLOW | True of desktop-tauri (app.rs:190-204 ticker → pump_session → pump_wallclock). False of the shipped CLI, where nothing paces the scheduler between REPL commands (see clock.rs audit F-001; W00: interactive clock stuck unless `advance N`). The doc names the duty-holder but does not say only one of the two shipped apps performs it. |
| "skips are observable events, never silent" (11-12) | crate doc | UNVERIFIED | scheduler.rs internals (another agent's file); the exported `SchedulerStats`/`TickDrops`/`DropTrail` vocabulary supports the claim structurally. |
| "every acceptance test runs on injected virtual time with zero sleeps" (29-30) | crate doc | REAL-UNPROVEN | Consistent with policy.rs tests (no sleeps — verified) and the doctest's virtual source; full scheduler.rs test scan is another agent's task. |

## 7. Dead / duplicated / unreachable code

None at this level. (`AdjustParams`/`AdjustError` reachability is adjust.rs's concern — adjust.rs is another agent's file.)

## 8. Dependencies & flags

No feature flags; no unsafe; deps media-types + media-clock (Cargo.toml verified). dev-dep testkit dev-only.

## 9. Open questions for the next agent

- W03 colleague with scheduler.rs/queue.rs/adjust.rs: confirm `AdjustParams` has a production consumer (the CLI converts I420→RGBA in pipeline_host.rs — is `adjust` reached at all from shipped apps?).
- W14: confirm the desktop-tauri ticker's `SESSION_TICK` interval vs the scheduler's `next_due` hint — is the wait hint consumed or ignored?
