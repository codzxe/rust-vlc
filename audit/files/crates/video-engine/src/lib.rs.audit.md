# AUDIT: crates/video-engine/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-21 |
| Lines / functions | 199 / 10 rows (crate root: docs, doctest, module decls, re-exports) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → `apps/cli/src/pipeline_host.rs:86` `use video_engine::{EnqueueOutcome, PresentationQueue, SchedulerCfg}` → scheduler/queue/policy types; renderer-wgpu `Cargo.toml` depends on video-engine and implements its `Presenter`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | read whole file + `adjust.rs` header (module framing); `cat Cargo.toml` (deps: media-types + media-clock only — "std-only, zero external deps" claim verified at manifest level); `rg -c "unsafe"` over src/*.rs (2 hits = both are doc-comment mentions of "no unsafe"; zero unsafe code); `rg "AdjustParams"` repo-wide (0 consumers outside the crate — consistent with matrix row `vfeat-colors-adjust` partial, per prior Cargo.toml audit); `rg -n "Doc-tests video_engine"` in nofailfast baseline (doctest line 95 GREEN :6564–6569); `rg -n "adjust::tests"` (13/13 green :5918–5928). |

## 1. One-paragraph verdict
A crate root that is almost entirely documentation, and the documentation is unusually honest: the "What this crate is not (yet)" section records real gaps (no filter chain, no scheduler thread, `DropPolicy::Auto` resolved to `Late`), the deviations section lists four concrete divergences from the doc sketches with their locations, and the flagship doctest — a full scheduler→queue→presenter wiring on virtual time — actually executes and passes in the no-fail-fast baseline (:6567). The manifest claims ("std-only, zero external dependencies, no unsafe") check out: deps are media-types + media-clock, and the only `unsafe` strings in the crate are doc-comment promises. Two hygiene defects: the module map omits the crate's fifth public module (`adjust` — declared, exported, 13 green tests, and invisible in the crate-root docs), and the doctest teaches the push seam by discarding the very `PushOutcome` the module docs call "observable … never silent".

## 2. What actually works (verified)
— The crate's advertised core (scheduler + queue + presenter seam) is real and executed: video_engine lib 48/48 (:5915–5955), doctest green (:6564–6569 — 3 frames scheduled, delivered in order `[0, 33333334, 66666668]` on injected virtual time), presenter_seam/backend_fallback/desktop-pipeline integration suites green.
— "Std-only by C5: zero external dependencies, no unsafe" (L172–173): Cargo.toml deps = media-types, media-clock (workspace crates; "external" = none from crates.io); `rg unsafe` over src/ → only doc mentions. Verified.
— Cancellation discipline description (L44–52) matches the implementations I audited: `PresentationQueue::publish` wholesale replacement (queue.rs:150–156), scheduler generation purge (`generation_change_purges_the_window_and_kills_repeats`, green :5936).
— Deviations ledger (L67–87): each named deviation verified present in code (`FrameDecision` payload-free in policy.rs — grid tests green :5928–5937; `SchedulerCfg.frame_duration`/`max_pending` exist per queue.rs/policy.rs tests).

## 3. Findings
### F-001 [S4]
Where: crates/video-engine/src/lib.rs:54-65
Code:
```
//! # Module map
//!
//! - [`policy`]    — [`DropPolicy`], [`SchedulerCfg`] and the pure
//!   per-frame classifier [`FrameDecision`] (grid-testable, doc 03
//!   §4.10 f).
//! - [`scheduler`] — the [`PresentationScheduler`] window state
```
What is wrong: The module map lists `policy`, `scheduler`, `queue`, `sink` — four modules — but the crate declares and exports five (`pub mod adjust;` L175, `pub use adjust::{AdjustError, AdjustParams};` L184). `adjust` (the color-adjust parameter model + CPU reference, issue #33/D-012, 13 green tests) is absent from the crate-root documentation entirely; a reader of lib.rs cannot discover it exists.
Why it matters: The crate-root docs are the contract surface for boundary reviewers (boundaries.toml consumers); an undocumented public module is how seams drift. The "What this crate is not (yet)" section even says the video filter chain is "a later card; the pipeline seam is reserved" — while the crate quietly ships the adjust filter's engine-side model.
What correct looks like: Add the `adjust` row to the module map with its honest placement line ("parameter model + CPU reference; GPU wiring [PROPOSAL]; 0 in-repo consumers — matrix `vfeat-colors-adjust` partial").
Confidence: HIGH — Verification: read (L54–65, L175–199); adjust.rs header read; prior-wave Cargo.toml audit's 0-consumer rg cited.
### F-002 [S4]
Where: crates/video-engine/src/lib.rs:152
Code:
```
//!     let _ = scheduler.push(frame);
```
What is wrong: The flagship example discards `PushOutcome` — the very observable the module docs lean on four paragraphs earlier: "skips are observable events, never silent" (L11–12). A copy-paste of this example into production wiring would silently drop held/refused/stale pushes.
Why it matters: Doc examples are the de-facto API contract for new callers; teaching `let _ =` on a result type whose variants include `Stale`/`WindowFull`/`UnschedulablePts` contradicts the crate's own never-silent doctrine. (The example's final `delivered == 3` assert does catch total breakage, so the example is not dishonest about its outcome — only about the seam's contract.)
What correct looks like: `match scheduler.push(frame) { PushOutcome::Scheduled | PushOutcome::Inferred { .. } => {} other => return Err(format!("{other:?}").into()) }` or a comment naming the outcomes.
Confidence: HIGH — Verification: read (L141–166); PushOutcome variants enumerated in scheduler.rs exports (L191).

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs "What this crate is/is not" | 1–30 | REAL | — | Honest gaps ledger; verified against code (no filter chain, no thread, stepped core). |
| module docs "Time and clock vocabulary" | 32–42 | REAL | — | NANO_BASE usage verified in queue tests; TimeError passthrough is scheduler's territory (green tests). |
| module docs "Cancellation" | 44–52 | REAL | — | Matches queue.rs (audited) + scheduler tests (green). |
| module docs "Module map" | 54–65 | REAL | S4 | F-001: omits `adjust`. |
| module docs "Deviations from the doc sketches" | 67–87 | REAL | — | Four named deviations, each verified present. |
| doctest (full wiring example) | 95–167 | REAL | S4 | Executed GREEN in baseline (:6567); F-002 outcome discard. |
| module docs "Never" | 169–173 | REAL | — | Verified: no GPU APIs, std-only, no unsafe (rg). |
| module declarations | 175–182 | REAL | — | 5 pub mods + cfg(test) test_support. |
| re-exports | 184–194 | REAL | — | Complete: every pub item of the 5 modules except test_support. |
| `pub use media_clock::NANO_BASE` | 196–199 | REAL | — | Convenience re-export, documented rationale; used by CLI/tests. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| lib.rs doctest (line 95) | YES | Drives real scheduler + queue + presenter on virtual time; asserts 3 delivered in exact PTS order. Green in baseline :6567. |
| (indirect) the 48 lib tests + integration suites | YES | Covered per-module in my queue.rs audit and the baseline lines cited. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "This crate contains no GPU, window or surface code" | lib.rs L15–16 | REAL | Cargo.toml deps; renderer-wgpu owns surfaces (my surface.rs audit). |
| "skips are observable events, never silent" | lib.rs L11–12 | REAL in scheduler/queue (drop counters, typed outcomes) / contradicted by its own example | F-002. |
| "Std-only by C5: zero external dependencies, no unsafe" | lib.rs L172–173 | REAL | Cargo.toml (media-types, media-clock only); rg unsafe = doc mentions only. |
| "DropPolicy::Auto … v0 resolves it to Late" | lib.rs L69–73 | REAL | policy tests `sync_policy_defaults_follow_the_frame_duration` green :5934. |
| Module map completeness | lib.rs L54–65 | FALSE (incomplete) | F-001: `adjust` missing. |

## 7. Dead / duplicated / unreachable code
- `AdjustParams`/`apply_rgba8`: 0 in-repo consumers (matrix row honestly "partial" — prior-wave rg cited). Not dead by the matrix's own accounting, but unwired; the crate-root docs' silence (F-001) makes it look accidental rather than recorded.
- No duplication; re-export surface is complete.

## 8. Dependencies & flags
media-types + media-clock (workspace) — matches the registry row quoted in Cargo.toml and boundaries.toml (per prior audit, exact match). testkit dev-only. No cfg feature gates. 48/48 lib tests + 1/1 doctest green in nofailfast (:5915–5955, :6564–6569).

## 9. Open questions for the next agent
- To the settings/commands wave: `vfeat-colors-adjust` matrix row says UI sliders render "inert-with-reason until the command lands" — verify the inertness is actually wired in `ui/src/screens/settings/areas/effects.tsx` (the file exists; the comment there cites the same seam) and that no command path accepts adjust values yet.
- To scheduler/policy auditors (W03-A02/A03): the deviations ledger promises `max_pending` bounds "by construction" — confirm the window bound is a NonZero/checked type, not a doc-only promise.
