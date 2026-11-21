# AUDIT: crates/video-engine/src/policy.rs
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-21 |
| Lines / functions | 748 / 28 (14 prod incl. 4 consts + 2 trait impls, 14 test fns) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` bin (apps/cli, pkg `lumen-cli`) → `pipeline_host::accept_video` → `SchedulerCfg::for_frame_duration` (apps/cli/src/pipeline_host.rs:1300) → `PresentationScheduler::new` (apps/cli/src/pipeline_host.rs:1334) → scheduler tick reads `cfg.decide()`/`cfg.sync.*`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `rg -n "SchedulerCfg|SyncPolicy|FrameDecision|DropPolicy" crates/video-engine/src/` → 3 files (policy/scheduler/lib); `rg ... -l` across crates/+apps/ → scheduler.rs, sink.rs, pipeline_host.rs, renderer-wgpu & desktop tests; `rg -n "with_sync"` → no production caller; `rg -n "DropPolicy::Never|DropPolicy::Auto"` → only policy.rs/scheduler.rs tests; `sed -n` verification of every quote (byte-checked via `od -c` on :312 — terminal mangled `#[` as escape); baseline test-run check `rg -n "policy::tests" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` → 13 hits, all `ok` |

## 1. One-paragraph verdict
Pure arithmetic module (zero I/O, zero state) implementing the doc 03 §4.10
presentation-policy vocabulary; genuinely reachable from the `lumen` CLI via
pipeline_host, genuinely consumed by `PresentationScheduler` (every
`SyncPolicy` field read at scheduler.rs:488-977), and backed by 13 real
boundary/grid tests that all executed and passed in the no-fail-fast baseline
(orchrun log :5929-5940). The classifier math is correct (i128 widening,
strict/inclusive boundaries per doc, saturating `due_at`). The defects are
peripheral but real: `with_sync` is a documented "wiring knob" no wiring ever
turns, `DropPolicy::Auto`/`Never` are never selected by any shipped
construction path, and two doc comments promise contracts the function cannot
honor.

## 2. What actually works (verified)
— All 13 `policy::tests` executed and passed: baseline
`audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5929-5940`
(`grid_late_policy_decisions_are_exact ... ok` through
`zero_threshold_drops_anything_not_yet_on_time ... ok`).
— Runtime consumption verified: `rg -n "cfg.sync|cfg.decide|late_threshold"
crates/video-engine/src/scheduler.rs` → decide/late_threshold/sync fields read
at :488-:977 (incl. `bands()` at :491/:510, `pts_gap_threshold` at :636,
`widened_threshold` at :958, `drop_budget` :952, `decide_every` :877,
`repeat_extension` :922/:977).
— Boundary semantics mutation-checked by reading: flipping `decide`'s strict
`pts_ns < late_bound` to `<=` would break test case `(960, Present)` at
policy.rs:505 → tests are load-bearing.

## 3. Findings
### F-001 [S3]
Where: crates/video-engine/src/policy.rs:309-316
Code:
```rust
    /// Replace the sync policy (the wiring's knob — e.g. a session
    /// that knows the queue caps sets the PTS-gap snap threshold from
    /// them, doc 03 §4.7 e's "half the queue duration").
    #[must_use]
    pub fn with_sync(mut self, sync: SyncPolicy) -> SchedulerCfg {
        self.sync = sync;
        self
    }
```
What is wrong: The doc comment presents `with_sync` as the production wiring
point ("the wiring's knob", sessions overriding the snap threshold from their
QueuePlan per doc 03 §4.7 e). `rg -n "with_sync"` across `crates/` and
`apps/` finds exactly two call sites: the method definition and its own unit
test (policy.rs:720). No session, no pipeline wiring ever calls it.
Why it matters: The 500 ms default `pts_gap_threshold` is therefore
hard-wired in every shipped path; the documented QueuePlan-derived snap
threshold does not exist. A reader auditing A/V-sync-gap behavior per doc 03
§4.7 e would trace to a knob nobody turns.
What correct looks like: Either the session wiring calls `with_sync` with
queue-derived thresholds (per doc 03 §4.7 e), or the doc comment states this
is future-phase API with no current callers.
Confidence: HIGH — Verification: read + `rg -n "with_sync" crates/ apps/` →
only policy.rs:313 (def) and policy.rs:720 (test).

### F-002 [S3]
Where: crates/video-engine/src/policy.rs:54-78
Code:
```rust
    /// Automatic. v0 resolves to [`DropPolicy::Late`] with the doc's
    /// default threshold — one nominal frame duration — and therefore
    /// requires [`SchedulerCfg::frame_duration`]
    ...
    Auto,
}

impl DropPolicy {
    pub fn late_threshold(self, frame_duration: Option<Nanoseconds>) -> Option<Nanoseconds> {
```
What is wrong: `DropPolicy::Auto` and `DropPolicy::Never` are dead in every
shipped configuration. The sole production construction site,
`SchedulerCfg::for_frame_duration` (policy.rs:241-251), hard-codes
`DropPolicy::Late`, and `pipeline_host.rs:1300` calls only that. `Auto`/`Never`
appear solely in test files (policy.rs tests, scheduler.rs:1342/:1537).
Why it matters: A public policy vocabulary ships variants no shipped binary can
select — `Auto`'s "resolve to never when duration unknown" arm
(policy.rs:74-78) is unreachable through the shipped path, and its typed
rejection in `new` (:213-215) is exercised only by tests.
What correct looks like: Wire `Auto` (or drop the variants) at the
pipeline_host construction site — the whole point of `Auto` per the doc
comment is that v0 uses it — or mark the variants as test-only/unwired.
Confidence: HIGH — Verification: read + `rg -n "DropPolicy::Never|DropPolicy::Auto"
crates/ apps/` → only tests; `sed -n '1297,1301p' apps/cli/src/pipeline_host.rs`
→ sole production site uses `for_frame_duration`.

### F-003 [S4]
Where: crates/video-engine/src/policy.rs:438-443
Code:
```rust
    /// The widened effective late threshold for a sustained error
    /// `error_ema_ns` (the W1-18 §3.1 formula): `clamp(2·|ê|, 1, cap)
    /// × frame_duration`. `None` when no frame duration is known
    /// (never a guess) or the policy drops nothing for lateness.
    pub fn widened_threshold(
```
What is wrong: The contract says the function returns `None` "when ... the
policy drops nothing for lateness", but `widened_threshold` is a method on
`SyncPolicy`, which has no knowledge of `DropPolicy`; the only `None` path is
`frame_duration?` (:448). The never-drop guard lives at the call site
(scheduler.rs:950-958), not here.
Why it matters: A caller relying on this doc would skip its own Never-policy
guard and widen a threshold that then silently applies to a policy that
should never drop.
What correct looks like: Doc comment states `None` only for unknown frame
duration, and that the DropPolicy gate is the caller's duty (as scheduler.rs
does).
Confidence: HIGH — Verification: read of :443-457 and scheduler.rs:926-984.

### F-004 [S4]
Where: crates/video-engine/src/policy.rs:142-168
Code:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerCfg {
    /// The late-frame drop policy. See [`DropPolicy`].
    pub policy: DropPolicy,
```
What is wrong: Every field is `pub`, so the checked constructor's invariants
(`Auto` ⇒ frame_duration known; frame_duration ≠ 0, :213-218) are advisory —
any struct literal `SchedulerCfg { policy: DropPolicy::Auto, frame_duration:
None, .. }` bypasses validation. No in-repo caller bypasses today (`rg
"SchedulerCfg \{"` → no hits outside policy.rs itself), so this is a latent
hazard, not an active bug.
Why it matters: The typed-error discipline the module advertises ("failure is
data, never a panic", :108-109) is unenforceable with all-pub fields.
What correct looks like: Private fields + `new`/builders, or a debug_assert
guard, or a doc note that literals bypass validation.
Confidence: HIGH — Verification: read + `rg -n "SchedulerCfg \{|SchedulerCfg{"`
→ no external literal construction.

### F-005 [S4]
Where: crates/video-engine/src/policy.rs:502-503
Code:
```rust
            (0, FrameDecision::SkipLate),                          // far late
            (900, FrameDecision::SkipLate),                        // just past late bound − 1
```
What is wrong: The late bound in this grid is 1000−40 = 960; pts 900 is 60 ns
past it, not "just past late bound − 1". The neighboring case (959) is the
actual "late bound − 1". Misleading test comment. Related hygiene: the 30 fps
fallback here is 33_333_334 ns (:404) while pipeline_host's fallback is
33_333_333 ns (apps/cli/src/pipeline_host.rs:1299) — two different "30 fps"
constants in the same pipeline.
Why it matters: Boundary-test comments are the spec for future maintainers; a
wrong one invites "fixing" the wrong line.
What correct looks like: Comment "60 ns past the late bound"; one shared
30 fps constant.
Confidence: HIGH — Verification: read + sed of both sites.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `DropPolicy::late_threshold` | 73-79 | REAL | — | Pure match; Auto→duration arm production-dead (F-002) |
| `FrameDecision` (enum) | 91-106 | REAL | — | Payload-free by design, documented |
| `SchedulerCfgError` + Display + Error | 110-134 | REAL | — | Typed errors, strings match variants (tested :657-659) |
| `SchedulerCfg::new` | 206-231 | REAL | — | Validates Auto+None, zero duration; bypassable via pub fields (F-004) |
| `SchedulerCfg::for_frame_duration` | 241-251 | REAL | — | The only production constructor (pipeline_host.rs:1300); lead = dur/2 floors for odd dur |
| `SchedulerCfg::late_threshold` | 255-257 | REAL | — | Delegates |
| `SchedulerCfg::decide` | 271-288 | REAL | — | i128 throughout, no overflow; late-strict/due-inclusive per doc §4.10 e; mutation-checked by grid test |
| `SchedulerCfg::due_at` | 296-300 | REAL | — | i128 clamp to i64 range, then `as i64` (safe post-clamp); `Timestamp::new` is a plain ctor (media-types time.rs:147) — no panic path |
| `SchedulerCfg::sync` | 304-307 | REAL | — | Accessor; used by scheduler |
| `SchedulerCfg::with_sync` | 312-316 | REAL | — | 0 production callers (F-001) |
| `DEFAULT_MAX_PENDING` | 174 | REAL | — | PROPOSAL-tagged, tested |
| `DEFAULT_MAX_PENDING_NZ` | 187-190 | REAL | — | `loop {}` const dead-arm, scoped allow documented; correct pattern |
| `DEFAULT_MAX_REPEAT` | 196 | REAL | — | PROPOSAL-tagged, tested |
| `DEFAULT_SYNC_FRAME_NS` | 404 | REAL | — | 33_333_334 vs pipeline_host's 33_333_333 (F-005) |
| `SyncPolicy::for_frame_duration` | 415-428 | REAL | — | All 9 fields consumed by scheduler (verified rg) |
| `SyncPolicy::bands` | 434-436 | REAL | — | Called scheduler.rs:491/:510 |
| `SyncPolicy::widened_threshold` | 443-457 | REAL | — | Math equivalent to documented formula; saturating; doc-contract defect F-003; `cap.max(0)` at :455 dead (cap ≥ 0 by construction) |
| `SyncPolicy::default` | 461-464 | REAL | — | 30 fps fallback |
| test `ns`/`cfg` helpers | 471-490 | REAL | — | Local, fine |
| test `grid_late_policy_decisions_are_exact` | 495-523 | REAL | — | 12-case boundary grid incl. i64 extremes; comment defect F-005; ran ok (baseline :5933) |
| test `grid_never_policy_never_skips` | 526-539 | REAL | — | Ran ok (:5934) |
| test `zero_threshold_drops_anything_not_yet_on_time` | 542-551 | REAL | — | Ran ok (:5940) |
| test `drops_are_monotone_in_lateness` | 554-578 | REAL | — | Property over grid; ran ok (:5930) |
| test `waits_are_monotone_in_earliness` | 581-592 | REAL | — | Name overpromises: asserts due_at stability, not monotonicity itself (implied by math); ran ok (:5937) |
| test `extreme_positions_never_overflow` | 595-621 | REAL | — | u64::MAX threshold+lead vs i64 extremes; ran ok (:5931) |
| test `policy_threshold_resolution` | 626-640 | REAL | — | Ran ok (:5935) |
| test `auto_without_duration...` / `zero_duration...` | 645-678 | REAL | — | Typed-error tests; ran ok (:5929/:5939) |
| test `for_frame_duration_applies_the_doc_defaults` | 681-694 | REAL | — | Ran ok (:5932) |
| test `sync_policy_defaults_follow_the_frame_duration` | 699-723 | REAL | — | Ran ok (:5936) |
| test `widened_threshold_clamps_between_one_and_three_frames` | 726-746 | REAL | — | i64::MIN input clamps to cap, no panic; ran ok (:5938) |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| grid_late_policy_decisions_are_exact | YES | Exact 12-case expected-decision table incl. both boundary flips; mutating `<` to `<=` at :277 flips case (960, Present) to SkipLate |
| grid_never_policy_never_skips | YES | assert_ne on SkipLate + two exact asserts |
| zero_threshold_drops_anything_not_yet_on_time | YES | Three exact decisions at the 0-threshold boundary |
| drops_are_monotone_in_lateness | YES | Grid sweep with `unreachable!` on regression; requires first_drop_now present for pts<100 |
| waits_are_monotone_in_earliness | PARTIAL | Asserts due_at value only; a scheduler presenting early frames (lead ignored) would still pass — the monotonicity itself is math-implied, not asserted |
| extreme_positions_never_overflow | YES | Panics (debug) on any overflow; exact asserts on saturation and subtraction |
| policy_threshold_resolution | YES | Exact Option equality on all four arms |
| auto_without_duration / zero_duration | YES | Exact variant + Display-string asserts |
| for_frame_duration_applies_the_doc_defaults | YES | Exact field equality incl. computed 16_666_667 lead |
| sync_policy_defaults_follow_the_frame_duration | YES | All 9 fields pinned + builder round-trip |
| widened_threshold_clamps_between_one_and_three_frames | YES | Exact values incl. i64::MIN saturation |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "present the frame with the largest `pts ≤ clock + target_lead`", late `pts < clock − threshold` → drop | doc 03 §4.10 e | REAL | policy.rs:271-288 implements exactly; grid test baseline :5933 ok |
| lead default half frame, threshold default one frame [PROPOSAL] | doc 03 §4.10 e | REAL | `for_frame_duration` :241-251; test :681 ok |
| DropPolicy sketch `Never / Late{threshold} / Auto` | doc 03 §4.10 b | REAL (types) but Auto/Never unwired in shipped path | F-002; pipeline_host.rs:1300 only constructs Late |
| §4.10 f grid-testable pure classifier | doc 03 §4.10 f | REAL | 13 tests ran ok, baseline :5929-5940 |
| `with_sync` = "the wiring's knob", sessions set snap threshold from QueuePlan (doc 03 §4.7 e) | policy.rs:309-311 doc comment | FALSE (no such wiring exists) | F-001; rg shows zero production callers |

## 7. Dead / duplicated / unreachable code
- `SchedulerCfg::with_sync`: production-dead (F-001).
- `DropPolicy::Auto`, `DropPolicy::Never`: production-dead variants (F-002).
- `cap.max(0)` in widened_threshold:455: dead expression (cap = u64×u32 ≥ 0 by construction).

## 8. Dependencies & flags
Depends only on `media_clock` (NANO_BASE, SyncBands), `media_types` (time),
`std`. No feature gates on this file (`rg "cfg(" policy.rs` → only
`#[cfg(test)]`). No unsafe. No I/O. Cargo.toml of video-engine lists no
conditional deps for this module.

## 9. Open questions for the next agent
- scheduler.rs auditor (same wave): confirm the widened threshold actually
  feeds `decide` (i.e., the ladder's output replaces `cfg.late_threshold()`
  at the effective threshold, and the Never-policy gate at scheduler.rs:950
  guards it as F-003 assumes).
- Whoever audits apps/cli/src/pipeline_host.rs: the 33_333_333 vs 33_333_334
  30 fps fallback mismatch (F-005) originates there.
