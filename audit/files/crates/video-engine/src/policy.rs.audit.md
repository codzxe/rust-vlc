# AUDIT: crates/video-engine/src/policy.rs
| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2025-06-27 |
| Lines / functions | 747 / 13 (+12 unit tests) |
| File verdict | REAL for the config/sync surface (live-wired into the shipped CLI scheduler); the headline pure classifier `SchedulerCfg::decide` is UNREACHABLE from production — the scheduler reimplements its semantics inline |
| Reachable from shipped entry point? | YES: apps/cli/src/pipeline_host.rs:1300 `SchedulerCfg::for_frame_duration(...)` → :1324 `PresentationScheduler::new(cfg, clock)` → per-pump `tick_once` (:768-775) → scheduler consumes `cfg.target_lead` (scheduler.rs:796), `cfg.late_threshold()` (scheduler.rs:909-913), `cfg.due_at` (scheduler.rs:898), `cfg.sync.bands()/widened_threshold()/pts_gap_threshold/drop_budget` (scheduler.rs:481-491, 636, 932-969). `decide()` itself: NO production caller (see F-001). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `sed -n` quotes (exit 0); `rg '\.decide\('` repo-wide excluding audit/ (exit 0 — only policy.rs's own tests call it; streaming's `abr.decide` is unrelated); baseline cargo-test-workspace.txt:6455-6477 — all 12 policy::tests pass |

## 1. One-paragraph verdict
Hostile question 1, answered precisely: policy.rs is NOT dead scaffolding — its `SchedulerCfg` (drop policy, lead, frame duration, pending bound) and `SyncPolicy` (band machine params, widening cap, drop budget, snap threshold) are constructed by the shipped lumen-cli pipeline host and consumed every scheduler tick, including the 3.B1 sync ladder's widening/extension actuators. BUT the module's headline artifact — the "pure per-frame classifier" `decide()` returning `FrameDecision`, whose exhaustive grid tests are the file's proudest content — has zero production callers: `PresentationScheduler` inlines its own copies of the late test (`is_late`, scheduler.rs:1078-1083) and the due test (due_bound, scheduler.rs:796-800), and the equivalence between the two implementations is maintained by hand-written comments, not by code. The grid tests therefore prove a function the engine never calls; a semantic drift between `decide()` and the inline logic would keep every test green while production misbehaves. Remaining debt: a wrapping (not saturating, despite the comment) i128→u64 fallback in `widened_threshold`, an unwired "wiring's knob" (`with_sync`), and two disagreeing 30 fps fallback constants across the crate boundary.

## 2. What actually works (verified)
— All 12 unit tests pass: baseline cargo-test-workspace.txt:6455-6477 ("running 48 tests" for the package, policy::tests all ok).
— The grid tests are exact and boundary-complete: `grid_late_policy_decisions_are_exact` (lines 494-523) pins pts == now−threshold (presents), pts == now+lead (presents), one-past-either-bound flips, and i64::MIN/MAX extremes — real mutation-sensitive tests of the classifier.
— Live wiring verified by read: pipeline_host.rs:1300→1324→1354 (`scheduler.push`), scheduler.rs:796/898/909/958 consume cfg fields every tick.

## 3. Findings
F-001 [S3]
Where: crates/video-engine/src/policy.rs:271-288 (and header lines 1-9, 259-269)
Code:
```
    pub fn decide(&self, pts: Timestamp, now: Timestamp) -> FrameDecision {
        let pts_ns = i128::from(pts.ticks);
        let now_ns = i128::from(now.ticks);
        if let Some(late_bound) = self
            .late_threshold()
```
What is wrong: `decide()` has no production caller. rg over the repo (excluding audit/) shows `.decide(` only inside policy.rs's own `#[cfg(test)]` mod (lines 521-606) — the streaming-crate `abr.decide` hits are a different function. The runtime decision path is the scheduler's inline logic: `is_late` (scheduler.rs:1078, doc: "the strict comparison and i128 widening of `SchedulerCfg::decide`") and the due_bound scan (scheduler.rs:796-800). The module header sells `FrameDecision` as "the pure per-frame classifier" and doc 03 §4.10(f)'s grid-testability is implemented against it — but the classifier the grids validate is not the code that runs. Why it matters: the two implementations can drift silently; the grid suite would stay green. What correct looks like: the scheduler calls `cfg.decide(pts, now)` (or `is_late`/due_bound are factored out of `decide` and both call the shared primitives). Confidence: HIGH — Verification: read + rg repo-wide.

F-002 [S4]
Where: crates/video-engine/src/policy.rs:449-456
Code:
```
        // |ê| scaled by 2, capped at widen_cap × duration — i128
        // throughout, saturating into u64 (a negative-width threshold
        // is unrepresentable; extreme inputs clamp to the cap).
        let doubled = i128::from(error_ema_ns).saturating_mul(2).saturating_abs();
        let cap = i128::from(duration.0).saturating_mul(i128::from(self.widen_cap_frames));
        let width = doubled.clamp(i128::from(duration.0), cap);
        let ns = u64::try_from(width).unwrap_or(cap.max(0) as u64);
```
What is wrong: the comment says "saturating into u64", but `cap.max(0) as u64` is a WRAPPING i128→u64 cast. The fallback triggers only when width > u64::MAX, which needs duration × widen_cap > 2^64 (a frame duration > ~97 years at cap 3). Worked example: duration = 2^63 ns, cap = 3·2^63 → `as u64` truncates to 2^63 = one frame duration — the "clamped at the cap" threshold silently becomes the FLOOR. No panic, unreachable with real media (and `SchedulerCfg::new` doesn't range-check frame_duration), but the code does not do what the comment claims. What correct looks like: `u64::try_from(width.min(u64::MAX as i128)).unwrap_or(u64::MAX)`. Confidence: HIGH — Verification: read + arithmetic.

F-003 [S4]
Where: crates/video-engine/src/policy.rs:309-316
Code:
```
    /// Replace the sync policy (the wiring's knob — e.g. a session
    /// that knows the queue caps sets the PTS-gap snap threshold from
    /// them, doc 03 §4.7 e's "half the queue duration").
    #[must_use]
    pub fn with_sync(mut self, sync: SyncPolicy) -> SchedulerCfg {
```
What is wrong: `with_sync` (and the `sync()` getter at 304-307) have no production caller — the only callers are policy.rs's own test (line 720). The documented wiring ("a session that knows the queue caps sets the snap threshold") exists in no shipped session; every production `SchedulerCfg` carries the default `SyncPolicy` derived from frame duration. The knob is dead API surface. Confidence: HIGH — Verification: rg repo-wide.

F-004 [S4]
Where: crates/video-engine/src/policy.rs:404 vs apps/cli/src/pipeline_host.rs:1299
Code:
```
const DEFAULT_SYNC_FRAME_NS: u64 = 33_333_334;
```
What is wrong: two different 30 fps fallbacks in the same shipped path — policy.rs uses 33_333_334 for `SyncPolicy::default`, pipeline_host.rs:1299 `unwrap_or(33_333_333)` for the scheduler's frame duration. A 1 ns disagreement, harmless in effect, but the "one canonical fallback" discipline is broken across the crate boundary. Confidence: HIGH — Verification: read both.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DropPolicy::late_threshold | 73-79 | REAL | — | consumed by scheduler.rs:909 |
| SchedulerCfgError Display/Error | 120-134 | REAL | — | typed config errors, tested |
| SchedulerCfg::new | 206-231 | REAL | — | rejects Auto-without-duration, zero duration |
| SchedulerCfg::for_frame_duration | 241-251 | REAL | — | THE production constructor (pipeline_host.rs:1300) |
| SchedulerCfg::late_threshold | 255-257 | REAL | — | consumed every tick |
| SchedulerCfg::decide | 271-288 | UNREACHABLE (prod) | S3 | test-only parallel classifier (F-001) |
| SchedulerCfg::due_at | 296-300 | REAL | — | scheduler.rs:898 next_due hint |
| SchedulerCfg::sync / with_sync | 304-316 | UNREACHABLE (prod) | S4 | no wiring turns the knob (F-003) |
| SyncPolicy::for_frame_duration | 415-428 | REAL | — | defaults pinned by test |
| SyncPolicy::bands | 433-436 | REAL | — | scheduler.rs:491 |
| SyncPolicy::widened_threshold | 443-457 | REAL | S4 | wrapping fallback vs "saturating" comment (F-002) |
| SyncPolicy::default | 460-464 | REAL | — | 30 fps fallback const (F-004) |
| 12 tests | 494-746 | REAL | — | exact grids + property sweeps; all pass in baseline |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| grid_late_policy_decisions_are_exact | YES (of decide) | 12-case boundary-exact grid incl. i64 extremes |
| grid_never_policy_never_skips | YES | assert_ne on SkipLate across extremes |
| zero_threshold_drops_anything_not_yet_on_time | YES | 3-case exact |
| drops_are_monotone_in_lateness / waits_are_monotone_in_earliness | YES | property sweeps over stepped grids |
| extreme_positions_never_overflow | YES | u64::MAX lead/threshold × i64 extremes |
| policy_threshold_resolution | YES | exact Some/None per variant |
| auto_without_duration / zero_duration typed errors | YES | typed equality + Display content |
| for_frame_duration_applies_the_doc_defaults | YES | exact constants |
| sync_policy_defaults_follow_the_frame_duration | YES | all 9 fields pinned + with_sync replace |
| widened_threshold_clamps_between_one_and_three_frames | YES | 1.4-frame / cap / i64::MIN / None cases |
| CAVEAT | — | the grids validate `decide`, which production does not call (F-001); the scheduler's inline twins have their own coverage in scheduler.rs tests |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Everything here is pure arithmetic ... testable directly against these types" (doc 03 §4.10 f) | header:5-9 | REAL | pure functions, exact grid tests pass |
| "Decision order (doc 03 §4.10 e, binding)" implemented by decide | header:19-28 | SHALLOW | implemented TWICE — decide (test-only) + scheduler inline (F-001); currently equivalent |
| "the ladder is dormant while the scheduler presents within its own lead" | lines:163-167, 347-353 | REAL | Hold band = ½ frame = default lead; ladder applies only via apply_sync_ladder at decide_every cadence (scheduler.rs:874-880) |
| "saturating into u64" in widened_threshold | lines:450-451 | FALSE | `as u64` wraps (F-002); unreachable-with-real-media but the comment misstates the code |
| "the wiring's knob" for with_sync | lines:309-311 | FALSE | no shipped wiring calls it (F-003) |

## 7. Dead / duplicated / unreachable code
- `SchedulerCfg::decide` + `FrameDecision` — production-unreachable; twin of scheduler.rs's `is_late`/due_bound (F-001).
- `with_sync`/`sync()` getter — production-unreachable (F-003).
- `DEFAULT_MAX_PENDING_NZ`'s `None => loop {}` const arm (lines 186-190) — legitimate compile-time-dead trick with a scoped, justified `#[allow(clippy::empty_loop)]`; the same pattern is copied at pipeline_host.rs:142.

## 8. Dependencies & flags
Std-only crate surface: media-types + media-clock only (Cargo.toml deps), testkit dev-only. No feature gates; everything compiled in the default and release lanes. No unsafe.

## 9. Open questions for the next agent
- scheduler.rs is owned by W03-A03 (audit exists: audit/files/crates/video-engine/src/scheduler.rs.audit.md — they found `decide_every = 0` unvalidated); my F-001 (decide unwired) and their finding should be merged into the video-engine chapter.
- pipeline_host.rs:1362-1382 registers a KNOWN pacing DEFECT: the WindowFull hold stashes the UNCONVERTED frame (renderer refuses it, surface freezes) and `advance_time` drains the whole ring per pump (one tick plays the entire 2 s fixture) — i.e. in the shipped CLI the wall-clock PACING above this policy is masked by a renderer refusal. Whoever owns pipeline_host.rs must reconcile its comment at :840-841 ("the hold stores already-converted frames") with :1383 (`frame.clone()` of the original) — those two comments contradict each other today.
- apps/web and apps/desktop also reference video-engine (bridge/wire/frame_tap) — desktop-tauri unbuildable in sandbox (environment limit, per baseline); its use of this policy is UNVERIFIED.
