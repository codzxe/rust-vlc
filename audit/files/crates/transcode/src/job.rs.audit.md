# AUDIT: crates/transcode/src/job.rs
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2025-06-14 |
| Lines / functions | 534 / 22 prod fns + 4 tests |
| File verdict | REAL (executed: 4 unit + 6 lifecycle integration tests, all pass) — UNREACHABLE from shipped entry points (crate-level, F-001) |
| Reachable from shipped entry point? | NO (crate-level). In-crate chain: `record.rs Recorder::submit` -> `runner.rs:231 JobHandle::new` -> this file. Cross-crate: zero (apps/ never references `transcode`). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/transcode-f17733efc4fd5aa3 job::` -> 4 passed, exit 0; `./target/debug/deps/job_lifecycle-6124d913c7793298` -> 6 passed, exit 0 |

## 1. One-paragraph verdict
The job lifecycle cell is real and correctly engineered: one mutex + OnceLock first-set-wins terminal, saturating stats with max-merge for high-water marks, a bounded FIFO event hub that drop-and-counts (never blocks the worker), a condvar for pollers, a join-based `wait()` that maps worker panics to a typed failure and is idempotent. I executed the 4 unit tests and the 6-test `job_lifecycle.rs` integration suite (full-hub drop-and-count under load, prompt cancellation with truthful prefix, typed stage failures, bounded resources) — all green. The defects are: the crate ships nowhere (same as mp4mux.rs F-001); `wait()` is unbounded with no timeout variant — a stage wedged in uninterruptible blocking I/O parks the host forever (the docs admit "bounded by the pipeline joining"); and `cancel()` fires the same shared `Arc<AtomicBool>` flag twice through two aliases, which is harmless but undocumented-as-redundant.

## 2. What actually works (verified)
— `./target/debug/deps/transcode-f17733efc4fd5aa3 job::` -> "test result: ok. 4 passed" (hub FIFO + drop counts, saturating absorb + max peaks, first-set-wins terminate with final progress, state machine).
— `./target/debug/deps/job_lifecycle-6124d913c7793298` -> 6 passed, incl. `a_full_event_hub_drops_and_counts_but_never_blocks_the_pipeline`, `cancellation_is_prompt_terminal_and_leaves_a_truthful_prefix`, `completed_lifecycle_runs_progresses_completes_and_stays_bounded`. This is the end-to-end proof of this file's contracts.
Baseline cross-check: audit/runs/baseline/cargo-test-workspace.txt (same suites).

## 3. Findings
F-001 [S2]
Where: crates/transcode/src/job.rs:346-350 (crate-level; same defect as mp4mux.rs F-001)
Code: `pub struct JobHandle {\n    shared: Arc<JobShared>,\n    cancel: CancelToken,\n    threads: Mutex<Option<crate::runner::PipelineThreads>>,`
What is wrong: the entire transcode crate — this job API included — is not linked by any shipped entry point (grep over apps/: `transcode|Remuxer|JobHandle` = no matches). Feature matrix honestly `not_started` (feature-matrix.toml:1631-1639, 2914-2922). Listed here so this file's audit stands alone; not double-counted as a new defect. Confidence: HIGH — Verification: ran (grep).
F-002 [S3]
Where: crates/transcode/src/job.rs:402-424
Code: `/// Wait for the pipeline to finish and read the terminal outcome.\n    ///\n    /// Joins both pipeline threads (so the sink is finalized before\n    /// this answers), maps a host-stage panic to the typed\n    /// `transcode.job.worker-panic` failure, and is idempotent:`
What is wrong: `wait()` has no bound and no timeout variant. The interrupt path only wakes queue parks and track handoffs (runner.rs:243-253, cancel.rs:46-48 "Firing does **not** by itself wake parked queue waiters"); a pipeline stage blocked inside an uninterruptible OS call (a network read on a wedged source, a stalled disk) never reaches a stage boundary, so both threads never join and `wait()` hangs the host forever. Polling `state()` is available but equally never terminates for a stuck job — there is no escape hatch at all (no timeout, no force-detach-and-report). Why it matters: this is the public API a UI would call on a Record button; a wedged network source freezes the caller. What correct looks like: `wait_timeout(Duration)` or a documented detach that reports `Running` honestly. Confidence: HIGH — Verification: read (interrupt coverage cross-checked in runner.rs/cancel.rs).
F-003 [S4]
Where: crates/transcode/src/job.rs:369-371
Code: `pub fn cancel(&self) {\n        self.cancel.cancel();\n        self.shared.cancel_token().cancel();`
What is wrong: `self.cancel` and `self.shared.cancel_token()` are the SAME underlying `Arc<AtomicBool>` (runner.rs:139 creates one token, :145 clones it into `JobShared::new`, :231-233 passes the original to `JobHandle::new`; cancel.rs:26-28 "Clone shares the same underlying flag"). The double fire is a redundant no-op — harmless and arguably defensive, but nothing in the code or docs says the two fields alias, so a future refactor that makes them distinct tokens would silently change semantics here. What correct looks like: one call plus a comment, or distinct roles documented. Confidence: HIGH — Verification: read + traced construction.
F-004 [S4]
Where: crates/transcode/src/job.rs:301-304
Code: `pub(crate) fn terminate(&self, outcome: JobOutcome) {\n        if self.outcome.get().is_none() {\n            self.hub.emit(JobEvent::Progress(self.snapshot()));\n        }`
What is wrong: two racing terminators can both pass the `is_none()` check and both emit a "final" progress (check-then-act on separate locks — the outcome OnceLock is only the second gate). The comment at :297-300 pre-admits this ("A duplicate final progress from the loser is harmless") — it is harmless, but the `if` suggests atomicity it does not have. Confidence: HIGH — Verification: read (race window is between :302 check and :305 set).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| JobStats::absorb | 102-127 | REAL | — | saturating adds, max peaks — pinned by test |
| JobEventHub::new/emit | 164-183 | REAL | — | FIFO drop-and-count; no nested locks; tested under load in job_lifecycle |
| JobEventHub::try_recv/dropped | 186-197 | REAL | — | |
| JobShared::new | 226-237 | REAL | — | |
| set_running | 240-247 | REAL | — | idempotent Queued→Running |
| update_stats / snapshot | 250-257 | REAL | — | Copy snapshot, lock-scoped |
| observe_queue | 261-265 | REAL | — | max-merge at termination |
| is_stopping / cancel_token | 269-276 | REAL | — | |
| try_recv_event / dropped_events / emit_progress | 280-293 | REAL | — | |
| terminate | 301-315 | REAL | S4 | F-004 duplicate-final-progress race (pre-admitted) |
| state / outcome / wait_settled / lock | 318-339 | REAL | — | poison-tolerant |
| JobHandle::new | 353-363 | REAL | — | |
| JobHandle::cancel | 369-380 | REAL | S4 | F-003 double-fire of one flag |
| state / stats / try_next_event / dropped_events | 383-400 | REAL | — | polling surface |
| JobHandle::wait | 409-436 | REAL | S3 | F-002 unbounded; panic→typed-failure mapping correct; idempotent via OnceLock |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| hub_drops_and_counts_past_capacity_never_growing | YES | exact counts (4 kept/6 dropped) + FIFO oldest-first re-check |
| stats_absorb_saturates_and_maxes_the_high_water_marks | YES | u64::MAX-1 + 10 saturates; max-merge both directions |
| terminate_is_first_set_wins_and_emits_a_final_progress | YES | racing second terminal changes nothing; final progress carries the folded stats |
| state_starts_queued_and_stopping_needs_a_reason | YES | both stop reasons checked |
| (integration job_lifecycle.rs, 6 — executed) | YES | full-hub pressure, prompt cancel, typed failures, bounded peaks |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a full hub drops and counts — the worker never blocks on observability" | job.rs:22-27 | REAL | unit test + `a_full_event_hub_drops_and_counts_but_never_blocks_the_pipeline` (ran) |
| "every terminal is durable … first-set-wins … wait joins both pipeline threads" | job.rs:13-18 | REAL | terminate test + lifecycle suite (ran) |
| "Dropping a JobHandle detaches the job: the pipeline runs to its source's end" | job.rs:40-45 | REAL-UNPROVEN (design verified by read; no explicit detach test in this file's suite — job_lifecycle's cancel tests cover the related stop path) | |
| "cancellation is cooperative and prompt at stage boundaries" | job.rs:44-45 | REAL | `cancellation_is_prompt_terminal_and_leaves_a_truthful_prefix` (ran); promptness bounded by F-002's uninterruptible-window caveat |
| Accounting "real bytes and samples, never estimates", saturating | job.rs:29-36 | REAL | absorb test + bounded-resource lifecycle test |

## 7. Dead / duplicated / unreachable code
No dead functions in-file. The redundant double-cancel (F-003) is the only duplication. Crate-level unreachability: F-001.

## 8. Dependencies & flags
std only (VecDeque, Condvar, Mutex, OnceLock) + media-types + crate::cancel. No unsafe. No feature gates.

## 9. Open questions for the next agent
- runner.rs (not my file): confirm every blocking park in the pump/worker loops is genuinely interrupt-covered (queue parks + handoff), since F-002's severity depends on how wide the uninterruptible windows are.
- limits.rs: does `event_slots()` have a floor? A configured 0 makes the hub drop everything (legal per contract but the Running event would never be observable — worth a typed minimum).
