# AUDIT: crates/video-engine/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-17 |
| Lines / functions | 199 / 6 modules declared + re-export block + 1 doctest |
| File verdict | REAL (crate root wiring is genuine and doctested) with one stale-doc contradiction and one dead-in-production module |
| Reachable from shipped entry point? | YES for policy/scheduler/queue/sink: the `PresentationScheduler` is constructed in the shipped CLI (apps/cli/src/pipeline_host.rs:704-769). NO for `adjust`: `AdjustParams` has ZERO consumers outside video-engine (rg over crates/*/src apps/*/src: only adjust.rs + lib.rs) — the renderer-wgpu shader wiring is `[PROPOSAL]` (adjust.rs:14-16). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | cat Cargo.toml (deps verified); rg unsafe (only 2 doc-comment hits); rg AdjustParams repo-src (self-only); baseline cargo-test-workspace.txt:6554-6562 (doctest passes). |

## 1. One-paragraph verdict
A well-documented crate root: the module map, the honest "What this crate is not (yet)" section, recorded deviations from doc sketches, and a runnable end-to-end doctest (scheduler → queue → presenter on virtual time) that passes in baseline. The crate's structural claims check out: std-only with zero crates.io dependencies (Cargo.toml: media-types + media-clock only, testkit dev-only) and no unsafe (rg: only doc-comment mentions). Two real problems: (1) the "not yet" section says the video filter chain (color adjust) is "a later card; the pipeline seam is reserved" — but the 355-line `adjust` module EXISTS, is publicly re-exported, and is simply missing from the Module map — stale docs contradicting the crate's own code; (2) that `adjust` module has no consumer anywhere in the shipped product, so a public API surface is exported that nothing reaches.

## 2. What actually works (verified)
— verified by baseline `audit/runs/baseline/cargo-test-workspace.txt:6554-6562` -> "Doc-tests video_engine … test crates/video-engine/src/lib.rs - (line 95) ... ok; test result: ok. 1 passed".
— verified by cat crates/video-engine/Cargo.toml -> `[dependencies] media-types, media-clock` only; testkit under dev-dependencies — the "Std-only by C5: zero external dependencies" claim holds (no crates.io deps).
— verified by rg `unsafe` over crates/video-engine/src -> 2 hits, both inside doc comments (adjust.rs:36, lib.rs:173) — "no unsafe" claim holds.

## 3. Findings
F-001 [S3]
Where: crates/video-engine/src/lib.rs:18-25 vs 175,184
Code:
```
//! - The **video filter chain** (`VideoFilter` — color adjust, crop,
//!   deinterlace per doc 01 §2.2): a later card; the pipeline seam is
//!   reserved.
```
against
```
pub mod adjust;
…
pub use adjust::{AdjustError, AdjustParams};
```
What is wrong: The "What this crate is not (yet)" section tells the reader color adjust is NOT here ("a later card"), while `adjust` (355 lines: AdjustParams validation + byte-exact CPU reference `apply_rgba8`, issue #33/D-012) is compiled, public, and re-exported. The Module map (lines 54-65) lists policy/scheduler/queue/sink and omits `adjust` entirely. The docs contradict the file's own declarations 160 lines below. Why it matters: crate-level docs are the map auditors and integrators trust; a missing module is exactly how dead surface goes unnoticed (see F-002). What correct looks like: move `adjust` into the Module map with its "parameter model + CPU reference, GPU wiring [PROPOSAL]" status and delete the stale "not yet" bullet. Confidence: HIGH — Verification: read.
F-002 [S3]
Where: crates/video-engine/src/lib.rs:175,184 (export) + crates/video-engine/src/adjust.rs:14-16
Code (adjust.rs:13-16):
```
//! This is the **parameter model + CPU reference** for the adjust
//! stage. The production path is a renderer-wgpu shader consuming
//! these as uniforms (doc 03 §2.3 rule 3 — one pipeline, zero extra
```
What is wrong: `AdjustParams`/`AdjustError` are exported public API with zero consumers outside the crate (rg over crates/*/src and apps/*/src: only adjust.rs and lib.rs). The wgpu uniform wiring is an unlanded `[PROPOSAL]`. Until it lands, the module is UNREACHABLE from every shipped entry point — exported dead surface. Why it matters: the UI ships a Colors tool (ui/src/screens/player/tools/video/ColorsTool.tsx exists); if its settings route toward these params, the chain is broken at the renderer; if they don't, the params model nothing in production either way. Confidence: HIGH — Verification: read + rg.
F-003 [S4]
Where: crates/video-engine/src/lib.rs:152,159
Code:
```
//!     let _ = scheduler.push(frame);
```
and
```
//!         if let EnqueueOutcome::Enqueued = queue.try_push(request) {}
```
What is wrong: The canonical wiring example discards both typed outcomes (`let _ =`, empty if-let) while the crate docs insist "skips are observable events, never silent" (line 11-12). An example that pattern-matches success and drops the rest teaches exactly the swallowing the codebase forbids. Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC — module-level inventory)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Module docs (is/is-not/time/cancellation/map/deviations) | 1-93 | REAL | S3 | Deviations honestly recorded (Auto→Late, PresentAction split, SchedulerCfg extensions); F-001 stale "not yet". |
| Doctest (end-to-end wiring) | 95-167 | REAL | S4 | Passes in baseline; F-003 swallowed outcomes. |
| `pub mod adjust` | 175 | REAL (code) / UNREACHABLE (product) | S3 | F-002: no consumer; CPU reference awaiting renderer wiring. |
| `pub mod policy / queue / scheduler / sink` | 176-179 | REAL | — | Consumers: scheduler→pipeline_host.rs (shipped CLI); sink::Presenter→renderer-wgpu. |
| `mod test_support` (cfg(test)) | 181-182 | REAL | — | Test-only. |
| Re-export block | 184-194 | REAL | — | Coherent; adjust re-export flagged by F-002. |
| NANO_BASE re-export | 196-199 | REAL | — | Documented convenience. |

## 5. Test honesty
No #[test] in this file; the doctest is the executable artifact and it RUNS (baseline :6557) — it would fail if scheduler/queue/drain semantics broke (asserts 3 delivered, in-order stamps). It would NOT fail on push/enqueue rejection paths it silences (F-003).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "This crate contains no GPU, window or surface code" | lines 15-16 | REAL | Only 5 src modules, none touch wgpu; Cargo.toml has no renderer deps. |
| "The video filter chain … a later card; the pipeline seam is reserved" | lines 20-25 | FALSE (stale) | `adjust` module exists and is re-exported (F-001). |
| "Std-only by C5: zero external dependencies, no unsafe" | lines 172-173 | REAL | Cargo.toml (path deps only) + rg unsafe (doc comments only). |
| "skips are observable events, never silent" | lines 11-12 | REAL (in the modules) | scheduler.rs drop accounting (other agent's file); the doctest itself violates the spirit (F-003). |

## 7. Dead / duplicated / unreachable code
`adjust` module: unreachable from shipped entry points (F-002) — the one genuinely dead public surface in this crate root.

## 8. Dependencies & flags
media-types + media-clock (path); testkit dev-only. No feature gates, no unsafe, no external crates — matches boundaries.toml's allowlist per the Cargo.toml comment.

## 9. Open questions for the next agent
- scheduler.rs / policy.rs / queue.rs / sink.rs auditors: confirm `SyncPolicy` wiring (the SyncBands actuators) and whether `DropPolicy::Auto`'s v0→Late resolution is surfaced to users as a deviation.
- Whoever audits the feature matrix: is there a `vfeat-colors-adjust` row, and what evidence does it cite — adjust.rs's CPU reference cannot evidence a working product feature while F-002 stands.
