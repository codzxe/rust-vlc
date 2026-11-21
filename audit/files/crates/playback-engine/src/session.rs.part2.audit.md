# AUDIT: crates/playback-engine/src/session.rs#part2 (lines 612–1222)
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-21 |
| Lines / functions | 611 audited (612-1222) / 27 functions+impls in range (file total 3059; parts 1/3/4/5 = other agents) |
| File verdict | REAL (chunk) |
| Reachable from shipped entry point? | YES. `lumen` CLI: `apps/cli/src/host.rs:231` `Engine::new()` → `:983` `engine.open(config)` → `:448` `engine.poll()` → `:645` `engine.drain_outputs_into(&mut outputs)`. Executed evidence: audit/runs/baseline/mp4-smoke-open-session.txt (`ok: accepted session=1 gen=0`, `Idle → Opening`, `Buffering → Playing → Stopped → Idle` — all produced by this chunk's `Engine::open`/`poll`/`drain_outputs_into` + the OutputHub) and INDEX.md CORRECTION v2 matrix (sample-420.mp4 PLAYS). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `sed -n '497,611p'/'612,1222p'` (chunk + part1 context of OutputHub fields) · `rg -n 'Disconnected' session.rs` (:440 sender-side typed, :1163 poll comment) · `rg -n 'impl Drop' session.rs` (none — basis of F-001) · `rg -n 'closed = true'` (:1605 only, the Close row — part3) · `rg -n 'Engine::new\|drain_outputs_into\|\.poll\(\)' apps/cli/src` (reachability chain) · `sed -n '2921,3059p'` (hub unit tests) · `rg -n 'is_stale_at_drain' events.rs` (:478-493 — terminal-exemption rule matches drain_into's doc) · baseline orchrun log `sed -n '3740,3920p'` (playback_engine 83/83 unit + integration suites pass) |

## 1. One-paragraph verdict
This chunk — the OutputHub's overflow/tick-replacement/compaction machinery, the observability counters, and the Engine's open/poll/drain surface — is the best-audited kind of code in this repo: I traced every cursor invariant (`first_event`/`first_receipt`/`resident_tick` vs `live_events`/`live_receipts`) through push, capacity drop, tick replacement, compaction, and drain, and found no violation; the tombstone-index-stability scheme that makes O(1) amortized pushes work is correctly maintained (compaction is the only index-shifting operation and it rebuilds every cursor); the arithmetic is saturating everywhere; and the unit tests assert internal counters and survivor ordering, not just "no panic". It is reachable and executed end-to-end by the shipped `lumen` binary (baseline smoke transcripts are this code's output). The one defect is a lifecycle wart: a host that drops its `SessionHandle` while `Playing` strands a session that `poll` will keep ticking forever with no command path left, because neither the handle nor the engine detects channel disconnection.

## 2. What actually works (verified)
— OutputHub cursor invariants: traced all 5 mutation paths (push event/receipt, drop_oldest_event/receipt, tick replacement, compact, drain); `first_event` is always the oldest live event or None iff live_events==0 (replacement path at :624-626 is safe because a resident tick that is `first_event` has no live events before it by definition).
— Tick replacement (:612-643): at most one live tick invariant holds on every path; killed-tick-as-oldest cursor advance handled; the new tick can never be dropped by its own push (capacity drop precedes the push).
— `compact_if_due` (:696-726): `retain` is the only index-shifting op and it rebuilds all three cursors in the same pass; trigger `dead >= live` gives the documented ≤2× bound.
— Execution: baseline `cargo test --workspace --release --no-fail-fast` (orchrun-test-workspace-nofailfast.log.txt, :3742-3744): `playback_engine` unittests 83 passed / 0 failed; hub unit tests (`session.rs:2921-3032`) assert `live_events == 8`, `dropped_events == 9992`, survivor seqs `(10_000-8..10_000)`, and the ≤2×live+1 slot bound after 10k undrained pushes — these fail under drop-newest, no-tombstone, or no-compaction mutations (reasoned through each).
— End-to-end: mp4-smoke-open-session.txt + INDEX.md CORRECTION v2 (sample-420.mp4 plays; avi fixture 60/60 frames) — `Engine::open` announcements and state transitions are visible in the committed transcript.
— Cross-file consistency: `drain_into`'s seek-terminal exemption doc (:732-739) matches the real rule in `events.rs:478-493` (terminals + session-scoped kinds survive supersession; cross-epoch stale by construction) — read and compared, no drift.

## 3. Findings

### F-001 [S4]
Where: crates/playback-engine/src/session.rs:1161-1170
Code:
```
        // 1. Commands (each receipt precedes the events it caused —
        // doc 05 §4.3 guarantee 2). Draining stops at the first empty
        // OR disconnected recv: both mean "no more work this poll".
        while let Ok(command) = session.inbox.try_recv() {
```
What is wrong: Poll treats a *disconnected* inbox (host dropped every `SessionHandle` clone) identically to an *empty* one — "no more work this poll" — but disconnection is permanent. `SessionHandle` has no `Drop` impl (`rg 'impl Drop'` over the file: none) and the only place `closed` is set is the Close command's row (:1605, part3). A host that drops its handle while the session is `Playing` (or leaves it in `Ended`) strands a live session: every subsequent `engine.poll()` keeps advancing the pipeline (or returns `Vec::new()` for a dead-state session that is never destroyed), decoders and queues stay alive, and no command can ever reach it.
Why it matters: The engine's boundedness story (D-1) is about memory, not lifecycle; this is a resource leak / zombie-session path gated on a host bug. Realistic blast radius is small — the shipped CLI holds its handle for the session's life — hence S4, but a library consumer gets no signal that the session is orphaned (no event, no auto-close).
What correct looks like: Either `SessionHandle` closes the session on last-drop (a `Drop` that best-effort queues `Close`), or `poll` detects `TryRecvError::Disconnected` once and runs the destroy path (step 3) instead of silently returning.
Confidence: HIGH — Verification: read (no Drop impl, single `closed = true` site, poll's `while let Ok` conflates Empty and Disconnected).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `OutputHub::push` (Event arm, chunk start) | 612-643 | REAL | — | Traced correct (see §2); saturating counters; let-chain on Rust 2024. |
| `drop_oldest_event` | 646-656 | REAL | — | Kills first_event, clears resident_tick if it was the victim, advances cursor. Correct. |
| `drop_oldest_receipt` | 659-666 | REAL | — | Symmetric; receipts never dropped by event pressure (pin verified by test :3000-3032). |
| `kill` | 668-670 | REAL | — | `queue[index] = Slot::Dead` — indexing would panic on a stale cursor; all call sites traced in-bounds (indices stable between drains). |
| `next_event_after` | 674-680 | REAL | — | `skip(from)` — from>len yields None, no panic; forward-only cursor amortization argument holds. |
| `next_receipt_after` | 683-689 | REAL | — | Mirror of the above. |
| `compact_if_due` | 696-726 | REAL | — | Trigger `dead >= live && dead > 0`; full cursor rebuild; O(live+dead) amortized. |
| `drain_into` | 740-761 | REAL | — | Pops all slots; stale suppression delegated to `is_stale_at_drain` (verified in events.rs); resets every cursor/counter. |
| `EngineStats` (19 pub fields) | 766-839 | REAL | — | Pure data; every field populated in `stats()` (:1131-1150); doc comments match semantics (frames_concealed excluded from delivered). |
| `PipelineTotals` | 846-856 | REAL | — | Copy struct — enables the copy-out-then-merge pattern in poll step 3. |
| `PipelineTotals::merge_tick` | 860-871 | REAL | — | Saturating adds of TickReport fields. |
| `absorb_starvation` | 881-883 | REAL | — | Assign-not-add; rationale (pipeline owns monotonic counters, merged once at close) is coherent and prevents double-count — verified against merge() and stats(). |
| `PipelineTotals::merge` | 886-925 | REAL | — | Adds all fields incl. starvation sub-counters; exactly-once merge at close verified (poll :1185-1192 + replace flow). |
| `fold_preroll_discards` | 931-936 | REAL | — | Saturating sub from delivered + add to discarded; doc matches. |
| `Engine` struct | 949-962 | REAL | — | `pub(crate)` session/outbox/hotplug for hotplug hooks — crate-internal, no layering leak. |
| `Debug for Engine` | 964-976 | REAL | — | Manual, honest (observable surface only; Session has non-Debug trait objects). |
| `Engine::new` | 981-983 | REAL | — | OUTBOX_CAPACITY=1024 (:98). |
| `Engine::with_outbox_capacity` | 988-998 | REAL | — | Test knob; receipt cap fixed at 4096 (:109); capacity clamped ≥1 in OutputHub::new (:574-575). |
| `Engine::open` | 1061-1094 | REAL | — | Epoch mint with checked_add → typed GenerationExhausted; pinned Idle→Opening row's Result honored (Err → Invariant, impossible-by-construction path); announcements pushed; single-writer discipline documented (open between polls). |
| `Engine::state` | 1099-1103 | REAL | — | `map_or(Idle)`. |
| `Engine::session_id` | 1107-1109 | REAL | — | None with no session. |
| `Engine::generation` | 1113-1115 | REAL | — | Live-or-last pair; documented. |
| `Engine::cancel_token` | 1120-1122 | REAL | — | For stage wiring (cards 2.C3/2.C4). |
| `Engine::stats` | 1126-1151 | REAL | — | Merges live session totals over closed-session totals; no double count (verified: merge happens only at close). |
| `Engine::poll` | 1157-1193 | REAL | S4 | F-001. Steps 1-3 ordering (commands → tick → destroy-after-flush) matches doc 03 §4.6(b); `Vec::new()` for dead states allocates nothing. |
| `Engine::drain_outputs_into` | 1202-1208 | REAL | — | Caller-reuse API (D-1); FIRST-pair fallback only reachable when the outbox is provably empty. |
| `Engine::take_outputs` | 1214-1218 | REAL | — | Owned convenience form; CLI uses the reuse form (host.rs:645). |
| `Engine::current_pair` | 1220-1222 | REAL | — | Live pair or last_pair (dead epoch's final events still deliver — intended, verified against staleness rule). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `the_hub_stays_bounded_and_counts_every_drop_at_capacity` (:2922) | YES | Asserts `live_events == 8`, `dropped_events == 9992`, survivor seqs `9992..10000`, and `queue.len() <= 2*live+1` after 10k pushes — kills drop-newest, lost-count, and no-compaction mutations. |
| `a_newer_tick_replaces_the_undelivered_older_one` (:2956) | YES | Asserts `time_ticks_replaced == 1`, `live_events == 2`, drained order `[2, 3]`, and post-drain tick is fresh (not a replacement) — kills replace-without-count and count-without-kill mutations. |
| `capacity_pressure_may_drop_the_resident_tick_and_counts_it` (:2983) | YES | Asserts the dropped-oldest was the tick, `resident_tick.is_none()`, and `time_ticks_replaced == 0` afterwards — kills the resident-cursor-survives-kill mutation. |
| `receipts_are_pinned_from_events_but_hard_capped` (:3000) | YES | Asserts 16 event drops leave 4 receipts, then receipt overflow drops the OLDEST (`[1,2,3,100]` survivors) and receipt-first ordering — kills receipt/event cross-dropping mutations. |
| `the_inbox_is_bounded_and_answers_typed_busy` (:3038) | YES | Asserts `InboxBusy` typed refusal + intact queue order — kills silent-drop and block mutations. |
All executed in baseline (83/83 unit pass, orchrun log :3744) and the wider integration suites (contract_suite 12/12, events_contract 11 pass/11 ignored, session_flow etc. — orchrun log :3746-3920).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "push is O(1) amortized with no memmoves" (hub complexity contract) | session.rs:527-539 (part1 doc, describes this chunk's fns) | REAL | Traced: forward-only cursors, tombstone stability, compaction rebuild; 10k-push test asserts the ≤2× bound. |
| "receipts are pinned from event overflow… bounded separately by their own hard cap" | session.rs:590-593 | REAL | Code :601-611 + test :3000. |
| "a newer timing.time-tick replaces an undelivered older one (counted) instead of consuming capacity — doc 05 §3.4 rule 1" | session.rs:593-595 | REAL | Code :612-643 + test :2956. |
| "exactly one terminal per completed generation" survives drain-time supersession (seek terminals exempt) | session.rs:732-739 | REAL | `events.rs:478-493` implements the exemption; events_contract suite passes in baseline. |
| Engine::open doc: "if the new open fails, the engine lands in Error under the new epoch" | session.rs:1052-1055 | REAL | Traced: refers to pipeline-open failures during poll (open_step → fail_with lands Error under the new epoch — visible in mp4-smoke-open-session.txt `Buffering → Error`); `open()`'s own Err paths (GenerationExhausted :1066-1069, Invariant :1081) are separately documented as leaving no session, and the Error-state path is exercised by the yuv444p smoke sample. |

## 7. Dead / duplicated / unreachable code
None in this chunk. Everything is called (hub internals from push/drain; Engine methods from apps/cli host.rs and the crate's tests).

## 8. Dependencies & flags
No new deps in range. Uses `commands::envelope::RequestId`, `testkit::Must` (test-only). `std::sync::mpsc::sync_channel(INBOX_CAPACITY=64)` bounded inbox (:1071) — sender-side overflow → typed `InboxBusy` (:440, part1), receiver-side emptiness conflated with disconnection (F-001).

## 9. Open questions for the next agent
- Part3 (A01) should confirm `replace_live_session` (:1245) records `last_pair` before destroying the old session so R9's cross-epoch suppression has the right comparison pair at the next drain (my chunk's `drain_outputs_into` depends on it).
- Does any host (tauri desktop) drop `SessionHandle` while Playing? If yes, F-001 escalates from wart to leak — check `apps/desktop` mailbox lifecycle (note: desktop `zero_alloc_transport` test is one of the 7 baseline failures — possibly related territory, different file).
