# AUDIT: crates/transcode/src/job.rs (whole file, 534 lines)
| Field | Value |
|---|---|
| Auditor | W10-A07 |
| Date | 2026-11-21 |
| Lines / functions | 534 / 24 fns/methods + 5 types + 4 tests |
| File verdict | UNREACHABLE (real, well-tested code — the transcode crate has zero consumers) |
| Reachable from shipped entry point? | NO — no workspace member depends on `transcode` (see §7) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -l '(streaming\|transcode\|device-discovery)' --glob Cargo.toml crates apps fuzz` → transcode's only "dependent" is its own manifest; `rg -n "JobHandle" crates/transcode` → record.rs/runner.rs/lib.rs (all in-crate); `rg -in transcode docs/feature-matrix.toml product.toml tests/rebrand/nova.toml`; `sed -n` on all quoted ranges; baseline INDEX #3 (transcode tests green in workspace run). |

## 1. One-paragraph verdict
This is the best-engineered file audited so far in W10-A07: a clean job lifecycle (Queued→Running→{Completed,Failed,Cancelled}), a genuinely bounded drop-and-count event hub, saturating accounting, a first-set-wins `OnceLock` terminal, and honest unit tests that pin every invariant they claim. The problems are peripheral: the `JobHandle` doc promises a `Clone` the type does not have, `cancel()` fires the same token twice, `terminate()` notifies a Condvar without holding its mutex (correct today only by statement-ordering accident), and — decisively — **nothing in the shipped product can reach any of it**: the transcode crate is an orphan (no dependent crate, no CLI/tauri wiring, `product.toml`'s `transcode = false` key read by nothing, `ui-dialog-convert` matrix row `not_started` with empty evidence).

## 2. What actually works (verified)
— `JobEventHub` boundedness: capacity enforced at :176-182, drops counted, FIFO kept — pinned by test `hub_drops_and_counts_past_capacity_never_growing` (:444-472, asserts seen==4, dropped==6, oldest-first). Passed in baseline `cargo test --workspace` (INDEX #3, 2026-11-21).
— First-set-wins terminal + final-progress ordering pinned by `terminate_is_first_set_wins_and_emits_a_final_progress` (:492-520); saturating absorb pinned by :474-490; `wait()`/cancel integration covered by `crates/transcode/tests/job_lifecycle.rs` (:285-363: completed lifecycle, prompt cancellation, truthful prefix) — all green in baseline #3.
— Detach-on-drop semantics verified by read: `JobHandle` has no `Drop`, `PipelineThreads` (runner.rs:245-271) has no `Drop`, std `JoinHandle` drop detaches, and the thread closures own their own `Arc` clones (runner.rs:189-204) — the module doc's detach claim holds.

## 3. Findings

### F-001 [S3] `JobHandle` doc claims `Clone` — the type is not `Clone`
Where: crates/transcode/src/job.rs:342-350
Code:
```
/// The host's handle to a submitted job (module docs for the full
/// lifecycle story). Cheap to hold and `Send`; `Clone` is deliberate
/// — every clone sees the same shared cell, and `wait` runs its join
/// protocol once (later callers get the stored outcome).
pub struct JobHandle {
    shared: Arc<JobShared>,
    cancel: CancelToken,
    threads: Mutex<Option<crate::runner::PipelineThreads>>,
}
```
What is wrong: there is no `#[derive(Clone)]` and no `impl Clone for JobHandle` anywhere (the only derives in the file are on `JobState`/`JobStats`/`JobEvent` at :56/:72/:132). The doc asserts a deliberate API property that does not compile — a host that wants to hand the handle to a polling thread and a joining thread cannot.
Why it matters: the doc's concurrency story ("every clone sees the same shared cell, wait runs its join protocol once") is exactly the design a consumer would rely on; it is fiction. Misleading public-API documentation.
What correct looks like: implement `Clone` (all fields are `Arc`/`CancelToken`/`Mutex<Option<…>>` — the `threads` slot's `take` semantics already make cloned `wait` idempotent via the `else` branch at :423-425), or delete the claim.
Confidence: HIGH — Verification: read (whole file + `rg "impl Clone for JobHandle"` → none).

### F-002 [S4] `cancel()` fires the same `CancelToken` twice
Where: crates/transcode/src/job.rs:369-371
Code:
```
    pub fn cancel(&self) {
        self.cancel.cancel();
        self.shared.cancel_token().cancel();
```
What is wrong: `submit` (runner.rs:139-145, :231-233) clones ONE token into `JobShared`, the queue, the hand-off, and the `JobHandle` — `self.cancel` and `self.shared.cancel_token()` are the same token by construction, so the second call is a redundant no-op that obscures the single-token invariant.
Why it matters: suggests two distinct cancellation paths exist; a future "fix" that makes them diverge would silently break one path's wakeups.
What correct looks like: one call plus a debug_assert or comment that both fields alias the same token.
Confidence: HIGH — Verification: read (runner.rs:137-241 traced).

### F-003 [S4] `terminate()` notifies the Condvar without holding its mutex — correct only by ordering accident
Where: crates/transcode/src/job.rs:305-314
Code:
```
        if self.outcome.set(outcome).is_ok() {
            let state = match self.outcome.get() {
                Some(JobOutcome::Completed(_)) => JobState::Completed,
                Some(JobOutcome::Failed(_)) => JobState::Failed,
                Some(JobOutcome::Cancelled(_)) => JobState::Cancelled,
                None => JobState::Failed,
            };
            self.lock().state = state;
            self.settled.notify_all();
        }
```
What is wrong: `settled.notify_all()` runs after the `MutexGuard` temporary from `self.lock().state = state;` is dropped. `wait_settled` (:330-335) checks `outcome.get()` under the core lock and then parks; the only thing preventing a lost wakeup is that the state write must acquire the core lock the waiter holds until it parks, forcing the notify to happen after the park. Reordering the two statements (or deleting the state write) introduces an unfixable-by-test hang.
Why it matters: classic fragile concurrency — the invariant is real but undocumented and unaudited by any test (no lost-wakeup stress test exists).
What correct looks like: hold the guard across `notify_all` (the standard pattern), i.e. `let mut core = self.lock(); core.state = state; drop(core); self.settled.notify_all();` is what exists — make it explicit with a comment, or notify while holding the guard.
Confidence: MEDIUM-HIGH — Verification: read (interleaving traced by hand; no execution).

### F-004 [S4] Transient window where `outcome()` is terminal but `state()` still says `Running`
Where: crates/transcode/src/job.rs:305-312 with :318-320
Code:
```
        if self.outcome.set(outcome).is_ok() {
            …
            self.lock().state = state;
```
What is wrong: the terminal lands in the `OnceLock` before the derived `JobState` is written; a concurrent `state()` poller can observe `outcome() == Some(Failed)` and `state() == Running` in the same instant.
Why it matters: cosmetic inconsistency in the poll API (`state` docs call it "the lifecycle state"); `wait()` is unaffected because it joins first.
What correct looks like: derive `state()` from `outcome` (`outcome.get().map_or(core.state, …)`) or write state under the same lock before setting the outcome.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `JobState` (enum) | 57-69 | REAL | — | matches doc machine |
| `JobStats` + `absorb` | 73-128 | REAL | — | saturating adds, max for high-water marks; test-pinned |
| `JobEvent` (enum) | 133-140 | REAL | — | terminals excluded by design ✓ |
| `JobOutcome` (enum) | 144-153 | REAL | — | carries stats/error |
| `JobEventHub::new` | 164-170 | REAL | — | capacity stored; 0 capacity = all-drop (see limits.rs audit for validation) |
| `JobEventHub::emit` | 174-183 | REAL | — | non-blocking, bounded; slots lock released before dropped lock (no nesting) ✓ |
| `JobEventHub::try_recv`/`dropped` | 186-197 | REAL | — | trivial, correct |
| `JobShared::new` | 226-237 | REAL | — | Queued initial ✓ |
| `set_running` | 240-247 | REAL | — | Queued-guard prevents Running-after-terminal ✓ |
| `update_stats`/`snapshot` | 250-257 | REAL | — | lock+absorb/copy ✓ |
| `observe_queue` | 261-265 | REAL | — | max-merge ✓ |
| `is_stopping` | 269-271 | REAL | — | cancel ∨ terminal ✓ (test :527-532) |
| `cancel_token`/`try_recv_event`/`dropped_events`/`emit_progress` | 274-293 | REAL | — | delegates ✓ |
| `terminate` | 301-315 | REAL | S4 | F-003/F-004; first-set-wins via OnceLock ✓; final-progress-only-if-first ✓ |
| `state`/`outcome` | 318-326 | REAL | S4 | F-004 transient |
| `wait_settled` | 330-335 | REAL | S4 | poisoned-lock recovery ✓; parked-forever risk only if pipeline never terminates (protocol) |
| `lock` | 337-339 | REAL | — | poison-recovery doctrine ✓ |
| `JobHandle::new` | 353-363 | REAL | — | — |
| `JobHandle::cancel` | 369-380 | REAL | S4 | F-002; token + interrupt (queue + handoff wake) ✓ |
| `JobHandle::state`/`stats`/`try_next_event`/`dropped_events` | 383-400 | REAL | — | delegates ✓ |
| `JobHandle::wait` | 409-436 | REAL | — | join-once + idempotent replay ✓; panic→typed `transcode.job.worker-panic` ✓; defensive impossible-branch :426-431 ✓ |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `hub_drops_and_counts_past_capacity_never_growing` (:444) | YES | asserts exact kept/dropped counts and FIFO order; any capacity breach or miscount fails |
| `stats_absorb_saturates_and_maxes_the_high_water_marks` (:475) | YES | u64::MAX saturation + max-merge asserted exactly |
| `terminate_is_first_set_wins_and_emits_a_final_progress` (:493) | YES | second terminal must not change state; final progress must precede terminal; Running must be first |
| `state_starts_queued_and_stopping_needs_a_reason` (:523) | YES | initial state + both stop reasons asserted |
Gaps: no lost-wakeup/`wait_settled` concurrency stress (F-003), no `JobHandle::wait` idempotence test in-file (covered in tests/job_lifecycle.rs :296/:320 — two waits, same outcome), no Clone-usage test (impossible — F-001).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Clone is deliberate — every clone sees the same shared cell" | job.rs:343-345 | FALSE | F-001: no Clone impl |
| "a full hub drops and counts — the worker never blocks" | job.rs:22-25 | REAL | :174-183 non-blocking; test :444 |
| "terminals are not events: they are the outcome cell, which cannot overflow" | job.rs:26-27 | REAL | OnceLock :215, :305 |
| "Every counter is a saturating u64" | job.rs:31-32 | REAL | :115-126; test :475 |
| "Dropping a JobHandle detaches the job" | job.rs:40-43 | REAL | no Drop impls; threads own Arcs (runner.rs:189-204) |
| "wait joins both pipeline threads before answering" | job.rs:16-18 | REAL | :409-425 + PipelineThreads::join (runner.rs:266-270) |
| 7.E1 "job lifecycle (progress/cancel), bounded resources; AC: job model tests; no unbounded queues" | docs/13-agent-wave-program.md:733 | REAL-UNPROVEN + UNREACHABLE | tests exist & green (baseline #3); zero shipped consumers (§7) |

## 7. Dead / duplicated / unreachable code
The **entire transcode crate is dead code in the shipped product**: `rg` over every `Cargo.toml` in the workspace shows no member depends on `transcode` (only its own manifest self-reference); `apps/cli/Cargo.toml` and `apps/desktop-tauri/Cargo.toml` do not list it; `rg -l "transcode::"` matches only files inside `crates/transcode/**`. The user-facing surfaces that would consume it are absent: feature-matrix `ui-dialog-convert` (docs/feature-matrix.toml:2913-2922) is `not_started` with `evidence = ""`, and `product.toml:165` declares `transcode = false # Phase 7` — a config key no code reads (checked `crates/product-config/src`, `apps/cli/src` — zero readers; the rebrand test `tests/rebrand/nova.toml:98` toggles the unread key).

## 8. Dependencies & flags
`media_types` (EngineError), `crate::cancel::CancelToken` (AtomicBool token — cancel.rs:31-55), `crate::runner::PipelineThreads`, `crate::error::worker_panic`. No feature gates; part of workspace build (baseline #1 clean, #3 tests green). No unsafe.

## 9. Open questions for the next agent
- A03/A09 (record.rs / runner.rs parts): confirm the pump/worker actually call `update_stats` with deltas (not snapshots — double-absorb would inflate counters) and that `emit_progress`'s byte cadence exists as documented.
- Whoever owns product-config: the unread `transcode` (and `recording`) keys in product.toml — declared-but-unread config is a checklist defect; file it under product.toml's audit if not already there.
