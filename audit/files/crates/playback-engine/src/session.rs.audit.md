# AUDIT: crates/playback-engine/src/session.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 3058 / 62 (incl. 5 unit tests) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | YES — `lumen` bin: apps/cli/src/main.rs → run.rs `repl_mode`/`machine_mode` → `lumen_dispatch` → apps/cli/src/host.rs:231 `Engine::new()` → host.rs:448 `self.engine.poll()` → session.rs:1157 `Engine::poll`. Desktop: apps/desktop/src/shell.rs:280 `pump_session` → `CliHost::pump_wallclock` → same `Engine::poll`. Web: apps/web/src/host.rs:828. |
| Findings | S0:1 S1:0 S2:1 S3:4 S4:3 |
| Commands run | `sed -n` verbatim-quote checks (session.rs 8-17, 1157-1170, 1580-1586, 1889-1899, 2630-2640, 2641-2663, 2817-2822; run.rs 226-244; host.rs 439-447, 522-536, 946-952; state.rs 155-860) — all exit 0; `rg` caller searches. NO cargo (adaptation) — run-dependent claims cite W00 baseline or UNVERIFIED. |

## 1. One-paragraph verdict

This is the engine core and it is real code, not a stub: a single-writer, host-polled executor over a pure 30-row transition table (`state.rs` TRANSITIONS), with a bounded inbox (64), a bounded tombstone-compacting output hub (1024 events / 4096 receipts), generation minting/supersession, a seven-step seek flow with pre-roll hold, per-kind track switches, and a buffering-episode discipline — all internally coherent and heavily documented. The catastrophic defect is architectural and shipped: the engine spawns **no thread and has no internal clock driver**; it moves only when the host calls `Engine::poll`, and the shipped CLI's REPL **never polls between commands** — it blocks on `read_line` and the only full pump is the manual `advance N` verb. W00 proved the consequence: `time` stays 0.000000s while Playing unless `advance` pumps it. The engine itself is honest about this (module doc: "Until then the host's poll loop *is* the session thread"), but the product shipped a player whose playback does not progress in real time through its primary CLI. Verification of runtime behavior: UNVERIFIED (no cargo permitted); W00 baseline facts cited where used.

## 2. What actually works (verified)

- The state machine is real data: 30 rows in `crates/playback-engine/src/state.rs:371-665` (TRANSITIONS), applied only through `apply_input` (state.rs:755-772 region) called from `Session::apply` (session.rs:1544); `Engine::poll` is the sole mutation entry — verified by reading all mutation paths (`apply`, `open`, `replace_live_session`) and confirming they run only under `&mut` from `poll`/`open`.
- OutputHub overflow/tick-replacement/compaction logic verified by reading (session.rs:599-761) — receipts pinned from event overflow, resident-tick replacement, amortized compaction; internal unit tests (session.rs:2921-3058) assert exact counters and survivor identity.
- Reachability chain to the shipped `lumen` binary verified via `rg`: host.rs:231 `Engine::new()`, host.rs:448 `engine.poll()`.
- W00 baseline (cited, not rerun): release build PASS; tests 1751 pass / 2 fail / 25 ignored; session clock stuck unless advance-pumped.

## 3. Findings

### F-001 [S0] The engine has no clock/poll driver, and the shipped CLI REPL never pumps — playback time does not advance while Playing
Where: crates/playback-engine/src/session.rs:8-17 (design admission), 2647-2651 (frozen clock ⇒ no ticks), with cross-file chain apps/cli/src/run.rs:226, apps/cli/src/host.rs:439-447
Code (session.rs:8-17, verbatim):
```rust
//! discipline without spawning anything: every mutation happens
//! behind `&mut self` on [`Engine::poll`], so invariant S1 ("state
//! transitions occur only on the session thread") holds *by
//! construction* — there is no second writer to exclude. Commands
//! arrive through a std mpsc inbox ([`SessionHandle`]), which is the
//! only cross-thread edge; the thread-per-session spawn, the demux
//! and decode workers, and the event broadcast bus land with cards
//! 2.E1/3.B3. Until then the host's poll loop *is* the session
//! thread.
```
Code (session.rs:2647-2651, verbatim):
```rust
        if self.state == SessionState::Playing {
            let position = clock_position_ns(&self.runtime.clock);
            match self.emissions.last_tick_ns {
                None => self.emissions.last_tick_ns = Some(position),
                Some(last) if self.runtime.tick.is_due(Some(last), position) => {
```
What is wrong: The engine core requires an external driver to (a) call `Engine::poll` (decode, presentation, signal translation all happen only inside `poll` → `tick_step` → `pipeline.tick`) and (b) advance the media clock (audio-master sessions advance only when the host renders null-device quanta). The shipped CLI REPL loop does neither: apps/cli/src/run.rs:226 `match input.read_line(&mut line) { Ok(0) | Err(_) => break,` — the thread blocks on stdin; the only full pump is the explicit `advance` verb (run.rs `CliCommand::Advance` → `host.advance(ticks)`), and `settle()` after commands is pipeline-only (host.rs:439-447: "One **pipeline-only** pump … NO time advance … Media time moves ONLY via [`CliHost::advance`]"). `pump_wallclock` (host.rs:522) exists but its only production caller is the desktop shell (apps/desktop/src/shell.rs:280), driven by a 10 ms ticker thread in apps/desktop-tauri/src/app.rs:109/197 — not the `lumen` bin.
Why it matters: The user-visible core is broken in the shipped CLI: open a media, state reaches Playing, and nothing decodes forward, no time-ticks fire (frozen clock ⇒ `is_due` never true), position reads 0.000000s forever until the user types `advance N`. This is exactly the W00 baseline proof ("`time` stays 0.000000s while Playing unless `advance N` ticks pump it"). The engine is honest internally; the product is a player that does not play.
What correct looks like: Either the engine owns a session thread (the deferred cards 2.E1/3.B3) or the CLI REPL runs a wall-clock pacer loop (`pump_wallclock` on a ~10-30 ms ticker, like the desktop shell) instead of blocking on `read_line`.
Confidence: HIGH — Verification: read + W00 baseline run output (cited).

### F-002 [S2] `video_played_out` Drained gate depends on a clock the CLI never advances — video-only EOF sessions can sit in Playing forever
Where: crates/playback-engine/src/session.rs:2740-2749, 2608-2613
Code (session.rs:2740-2749, verbatim):
```rust
    fn video_played_out(&self) -> bool {
        match self.pipeline.as_ref() {
            None => true,
            Some(pipeline) if pipeline.has_audio_limb() => true,
            Some(pipeline) => match pipeline.last_video_end_ns() {
                None => true,
                Some(end) => clock_position_ns(&self.runtime.clock) >= end,
            },
        }
    }
```
Code (session.rs:2608-2613, verbatim):
```rust
        let signal = if report.fatal.is_some() {
            Some(PipelineSignal::Fatal)
        } else if report.drained && self.video_played_out() {
            Some(PipelineSignal::Drained)
        } else if report.eof_reached {
```
What is wrong: For a video-only session the `Drained` signal (→ `Ended`) is gated on the media clock reaching the last delivered video pts. In the shipped CLI the clock is `system_monotonic` for no-audio sessions (apps/cli/src/pipeline_host.rs:1622-1631 "wall_clock"), so wall time does advance it — but `poll` is never called between commands (F-001), so `report.drained` is never even computed; and for audio sessions the gate is bypassed (`has_audio_limb() → true`) while the audio-master clock is frozen because the null device renders nothing without `advance`. Net: end-of-media cannot fire between commands in the shipped CLI; state stays Playing with fully drained queues.
Why it matters: Wrong behavior in the realistic shipped case — a session that finished its data never reaches `Ended` on its own; only `advance N` (or the desktop ticker) lets the machine settle.
What correct looks like: A host-independent drain deadline (e.g., pipeline-driven end detection with a bounded grace), or the F-001 pacer fix.
Confidence: MEDIUM — Verification: read (cross-file); the EOF-stall behavior itself UNVERIFIED by run.

### F-003 [S3] Tick-path transition rejections are silently dropped (`Err(_) => {}`)
Where: crates/playback-engine/src/session.rs:2631-2639
Code (verbatim):
```rust
            match self.apply(input, context) {
                Ok(mut transition_outputs) => outputs.append(&mut transition_outputs),
                Err(_) => {
                    // Unreachable: the exhaustive tests prove the
                    // translation only yields table-legal inputs.
                    // Honored honestly (dropped) rather than
                    // panicked on.
                }
            }
```
What is wrong: A rejected pipeline-signal translation is swallowed with no counter, no event, no log. The "unreachable" claim rests on the exhaustive table tests (UNVERIFIED here — no cargo; W00: 1751 pass / 2 fail / 25 ignored, failing tests unidentified). If the invariant ever breaks, a fatal/EOF/underrun signal is lost invisibly. Same pattern: `open_step`'s `.unwrap_or_default()` (session.rs:2383-2385) and `fail_with`'s `.unwrap_or_default()` (session.rs:2807-2808).
Why it matters: Silent drop of the machine's only progress inputs is the worst failure mode for a state machine; a one-line counter would make it observable.
What correct looks like: Count dropped rejections in `EngineStats` (there is precedent: `suppressed_stale_events`).
Confidence: HIGH — Verification: read.

### F-004 [S3] `apply_command`'s Close path rewrites event generations post-hoc
Where: crates/playback-engine/src/session.rs:1889-1898
Code (verbatim):
```rust
            if rejection.is_none() {
                // The composition completed: stamp its whole trail
                // with the final generation (the doc 05 §4.2 `gen` of
                // the close — the session's last).
                let final_generation = self.pair().wire_generation();
                for output in &mut outputs {
                    if let EngineOutput::Event(event) = output {
                        event.generation = final_generation;
                    }
                }
            }
```
What is wrong: The `state-changed → stopped` event, emitted under the Stop row's mint, is re-stamped with the Close row's final generation so the drain-time staleness rule does not swallow it. The wire's `generation` field stops meaning "the generation under which this event was produced" for composed closes. Rationale is documented (session.rs:1844-1870) and self-consistent with the drain exemption, but any host correlating generations to receipts/events sees a value that never existed at emission time.
Why it matters: Wire-semantics distortion in the one place hosts do correlation; a spec-driven host cannot distinguish a close trail from a same-generation burst.
What correct looks like: A drain-time exemption for composed-close events (as already exists for seek terminals) instead of retroactive rewriting.
Confidence: MEDIUM — Verification: read.
Note: session.rs:1518-1533 `emit_terminal` shows the exemption mechanism exists and works for seek/switch terminals — the asymmetry is a choice, not necessity.

### F-005 [S3] `clock_position_ns` converts clock errors to position 0 presented as truth
Where: crates/playback-engine/src/session.rs:2817-2822
Code (verbatim):
```rust
pub(crate) fn clock_position_ns(clock: &MediaClock) -> i64 {
    match clock.now().as_nanos() {
        Ok(ns) => nanos_i64(ns),
        Err(_) => 0,
    }
}
```
What is wrong: On the i64-exhaustion corner the position silently becomes 0 — this value feeds `time-tick` events, the pause-entry tick (session.rs:1571), seek resolution base (session.rs:1998), and the failed-seek `position` report (session.rs:2008). A host would see a jump to 0.0 with no error marker. The comment (2812-2816) calls 0 "the honest floor", but 0 is indistinguishable from the media start.
Why it matters: Time displays and seek rollback positions lie on the corner; no counter exists for how often.
What correct looks like: Clamp to i64::MAX (or the last known position) and count occurrences.
Confidence: HIGH — Verification: read.

### F-006 [S4] Fabricated numbers in typed stand-in errors
Where: crates/playback-engine/src/session.rs:1286-1297, 2826-2836, 2840-2850
Code (1286-1297, verbatim):
```rust
    fn generation_exhausted_boxed() -> SessionError {
        SessionError::GenerationExhausted(EngineError::new(
            media_types::PipelineStage::Control,
            media_types::ErrorKind::ResourceLimit {
                limit: media_types::error::LimitKind::SessionGenerations,
                budget: u64::MAX,
                attempted: u64::MAX,
            },
```
What is wrong: `budget: u64::MAX, attempted: u64::MAX` (and `Memory, 0, 0` in `internal_missing_fatal`/`internal_missing_pipeline`) are placeholder numbers in host-visible typed errors — no real budget/attempt was measured. These paths are caller-bug corners, but the error payload misinforms.
What correct looks like: A dedicated ErrorKind for invariant breaches instead of ResourceLimit with dummy values.
Confidence: HIGH — Verification: read.

### F-007 [S4] `transition_table()` is a production-surface accessor used only by tests
Where: crates/playback-engine/src/session.rs:2852-2858
Code (verbatim):
```rust
/// The whole transition table is reachable from the pure model —
/// exposed for the exhaustive tests (and for future model checkers
/// per doc 03 §4.6(f)).
#[must_use]
pub fn transition_table() -> &'static [Transition] {
    TRANSITIONS
}
```
What is wrong: Only callers found: crates/playback-engine/tests/table.rs:156-157. No shipped entry point uses it. Documented as test surface, so this is hygiene, not dead-code fraud.
Confidence: HIGH — Verification: `rg "transition_table"` over crates/ and apps/ (only test hits).

### F-008 [S4] `replace_live_session` re-checks `Option` it just matched
Where: crates/playback-engine/src/session.rs:1251-1260
Code (verbatim):
```rust
        let plan = match self.session.as_ref() {
            Some(session) => close_plan(session.state),
            None => return Ok(()),
        };
        for input in plan {
            let outputs = self
                .session
                .as_mut()
                .map(|session| session.apply(*input, InputContext::default()))
                .unwrap_or(Ok(Vec::new()));
```
What is wrong: `.unwrap_or(Ok(Vec::new()))` on a session that was just proven `Some` (and is not removed until line 1272) is a silently-swallowing pattern copied from the impossible-path doctrine; here it is simply redundant. Correct shape: bind `&mut` once.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SessionCommandKind::as_command_id` | 159-170 | REAL | — | 1:1 wire-id mapping, exhaustive |
| `SessionCommandKind::as_input` | 174-188 | REAL | — | subtitle maps to SwitchTrack but never reaches table (pre-table branch) |
| `SessionCommandKind::selection_kind` | 197-204 | REAL | — | lane-typed; audit F-1 fix |
| `ReceiptResult::accepted_generation` | 257-263 | REAL | — | trivial accessor |
| `SessionHandle::session_id` | 307-309 | REAL | — | |
| `SessionHandle::submit` | 320-327 | REAL | — | |
| `SessionHandle::play/pause/stop/close` | 333-359 | REAL | — | thin wrappers |
| `SessionHandle::seek` | 368-375 | REAL | — | payload beside command |
| `SessionHandle::set_video_track`/`set_audio_track`/`set_subtitle_track` | 383-433 | REAL | — | |
| `SessionHandle::queue` | 437-442 | REAL | — | try_send → typed InboxBusy/SessionClosed; never blocks |
| `SessionConfig::new` | 487-505 | REAL | — | doc-default plan/budget/policy |
| `OutputHub::new` | 571-586 | REAL | — | max(1) guards |
| `OutputHub::push` | 599-643 | REAL | — | receipt cap, event cap, tick replacement; O(1) amortized claim consistent with cursor design |
| `OutputHub::drop_oldest_event` | 646-656 | REAL | — | clears resident_tick if victim |
| `OutputHub::drop_oldest_receipt` | 659-666 | REAL | — | |
| `OutputHub::kill` | 668-670 | REAL | — | tombstone, index-stable |
| `OutputHub::next_event_after`/`next_receipt_after` | 674-689 | REAL | — | forward-only scans, amortized |
| `OutputHub::compact_if_due` | 696-726 | REAL | — | retain + cursor recompute; dead ≥ live trigger ⇒ ≤2× bound |
| `OutputHub::drain_into` | 740-761 | REAL | — | staleness suppression with seek-terminal exemption (event.rs `is_stale_at_drain`) |
| `PipelineTotals::merge_tick` | 860-871 | REAL | — | saturating adds |
| `PipelineTotals::absorb_starvation` | 881-883 | REAL | — | assign-not-add, documented monotonic ownership |
| `PipelineTotals::merge` | 886-925 | REAL | — | 14 saturating adds; verbose but correct |
| `PipelineTotals::fold_preroll_discards` | 931-936 | REAL | — | corrects delivered down, discard up |
| `Engine::fmt::Debug` | 968-976 | REAL | — | manual, honest (finish_non_exhaustive) |
| `Engine::new`/`with_outbox_capacity` | 981-998 | REAL | — | |
| `Engine::open` | 1061-1094 | REAL | — | replacement flow R2-R9 via replace_live_session; checked_add epoch; Open rejection → Invariant error (honest) |
| `Engine::state`/`session_id`/`generation`/`cancel_token`/`stats` | 1099-1151 | REAL | — | stats merges live session + lifetime totals per call |
| `Engine::poll` | 1157-1193 | REAL | S0 | the ONLY driver; shipped CLI never calls it between commands (F-001) |
| `Engine::drain_outputs_into`/`take_outputs` | 1202-1218 | REAL | — | reuse API; current_pair fallback for post-close drains |
| `Engine::current_pair` | 1220-1222 | REAL | — | |
| `Engine::replace_live_session` | 1245-1276 | REAL | S4 | redundant Option re-check (F-008); totals merged before drop |
| `Engine::Default` | 1279-1283 | REAL | — | |
| `SessionError::generation_exhausted_boxed` | 1286-1297 | REAL | S4 | dummy u64::MAX values (F-006) |
| `Session::new` | 1420-1464 | REAL | — | state Idle; pipeline constructed |
| `Session::pair` | 1466-1468 | REAL | — | |
| `Session::emit`/`emit_hook`/`emit_raw`/`emit_terminal` | 1470-1533 | REAL | — | seq fetch_add relaxed; terminal under own generation |
| `Session::apply` | 1544-1734 | REAL | S3 | effect interpreter; Underrun episode counting post-`?`; pause-entry tick; buffering bookend; damage flush at settling; swallowed-rejection corners delegated to callers |
| `Session::flush_damage` | 1739-1760 | REAL | — | two-stage flush, zero-count filtered |
| `Session::position_fraction` | 1765-1772 | REAL | — | clamp, 0.0 on unknown duration |
| `Session::apply_command` | 1784-1970 | REAL | S3 | payload guard; subtitle class-S; selector pre-table resolution; Close composition + generation re-stamp (F-004) |
| `Session::seek_pipeline_step` | 1985-2093 | REAL | — | D4 refusal leaves pipeline intact; hold_cap from packets_per_limb; failure path publishes generation (M10) |
| `Session::fold_preroll_discards` | 2098-2106 | REAL | — | |
| `Session::absorb_pipeline_starvation` | 2114-2119 | REAL | — | |
| `Session::switch_pipeline_step` | 2132-2212 | REAL | — | audio freeze except deselect (F5/#16 regression documented); deferred_fatal path |
| `Session::subtitle_select_step` | 2221-2326 | REAL | — | hand-mirrored legality (duplicated from table rejection families — duplication noted); end_switch disarms window |
| `Session::sync_selected_mirror` | 2331-2337 | REAL | — | clones track vec per switch |
| `Session::open_step` | 2342-2419 | REAL | — | one-shot guard; wired==0 → NoDecodableTrack (honest); `unwrap_or_default` swallow (F-003) |
| `Session::tick_step` | 2428-2665 | REAL | S2 | signal priority fatal>drained>eof>primed>starved; seek completion; switch completion; time-tick on media position (frozen clock ⇒ none) (F-001/F-002) |
| `Session::complete_switch_kind` | 2674-2725 | REAL | — | re-anchor failure → Fatal+Failed; end_switch both paths |
| `Session::video_played_out` | 2740-2749 | REAL | S2 | F-002 |
| `Session::inflight_switch_of`/`take_inflight_switch_of` | 2752-2765 | REAL | — | |
| `Session::emit_open_facts` | 2771-2794 | REAL | — | once-per-open flags |
| `Session::fail_with` | 2798-2809 | REAL | S3 | `unwrap_or_default` swallow (F-003) |
| `clock_position_ns` | 2817-2822 | REAL | S3 | F-005 |
| `internal_missing_fatal`/`internal_missing_pipeline` | 2826-2850 | REAL | S4 | F-006 |
| `transition_table` | 2856-2858 | REAL | S4 | test-only surface (F-007) |
| tests: `the_hub_stays_bounded…` | 2921-2949 | REAL | — | asserts live bound, drop count, 2× slot bound, survivor seqs — would fail on mutation |
| tests: `a_newer_tick_replaces…` | 2956-2977 | REAL | — | replacement + drain-frees-resident pin |
| tests: `capacity_pressure_may_drop…` | 2983-2993 | REAL | — | |
| tests: `receipts_are_pinned…` | 3000-3032 | REAL | — | pin + hard-cap pin |
| tests: `the_inbox_is_bounded…` | 3038-3057 | REAL | — | typed busy + order + room-again |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| the_hub_stays_bounded_and_counts_every_drop_at_capacity | YES (reasoned; UNVERIFIED by run) | exact counters (`dropped_events == 10_000-8`), survivor identity `(10_000-8..10_000)`, slot bound — any drop/compaction mutation breaks an assert |
| a_newer_tick_replaces_the_undelivered_older_one | YES (reasoned) | `time_ticks_replaced==1`, live count, drain order, post-drain freshness |
| capacity_pressure_may_drop_the_resident_tick_and_counts_it | YES (reasoned) | tick-as-oldest drop path + cursor death |
| receipts_are_pinned_from_events_but_hard_capped | YES (reasoned) | pinned count then oldest-receipt drop identity `vec![1,2,3,100]` |
| the_inbox_is_bounded_and_answers_typed_busy | YES (reasoned) | InboxBusy on 3rd send into capacity-2 channel; order preserved; room after drain |

All five are white-box (access private fields) — appropriate for policy internals. Mutation verification not run (no cargo); conclusions reasoned from assert specificity.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the poll-driven single-writer executor" | apps/cli/Cargo.toml comment | REAL | session.rs:8-17 + `Engine::poll` — design exists as described |
| Engine mints generations, emits event stream (doc 03 §4.6(a)) | module doc line 1-3 | REAL-UNPROVEN | mint/supersession logic read-verified; runtime behavior not executed |
| "thread-per-session spawn … land with cards 2.E1/3.B3" (i.e., not yet) | session.rs:14-15 | REAL | no thread spawn anywhere in crate (rg: no `thread::spawn` in playback-engine/src) — see F-001 for the shipped consequence |
| Receipt precedes events it caused (doc 05 §4.3) | session.rs:29-40 | REAL | poll pushes receipt first (1166-1169) |
| Superseded pipeline events suppressed at drain; receipts survive | session.rs:36-40 | REAL-UNPROVEN | drain_into + is_stale_at_drain read; exemption asymmetry in F-004 |

## 7. Dead / duplicated / unreachable code

- `transition_table()` — test-only public surface (F-007).
- `subtitle_select_step`'s legality mapping duplicates the table's rejection-reason families by hand (session.rs:2225-2237 vs state.rs `rejection_reason`) — documented as "mirrored by hand because the class-S path has no table row"; a future row addition can desync them (S4, folded into F-004 count? No — separate hygiene note; counted in S4).
- `Engine::with_outbox_capacity` — used by tests; production uses `Engine::new()` (host.rs:231). Bounded hygiene note only.

## 8. Dependencies & flags

- No feature flags gate this file; `playback-engine` is a workspace member pulled by `lumen-cli` (apps/cli/Cargo.toml internal deps list). Release build passes (W00 baseline: `cargo build --release --locked` PASS), so the file compiles in the shipped configuration.

## 9. Open questions for the next agent

- Does `Pipeline::switch_preroll` (pipeline.rs) treat EOF-before-first-frame of a new audio limb as "provably never could"? If not, an audio switch to a track that never delivers freezes the clock (session.rs:2156-2158) with no timeout — playback stalls in Playing. Files: crates/playback-engine/src/pipeline.rs, tracks.rs.
- Which 2 tests fail in the W00 baseline run (`cargo test --workspace`: 1751 pass / 2 fail)? If either is a session/table test, F-003's "unreachable by construction" claim weakens. File: audit/runs/baseline/.
- The desktop-tauri 10 ms ticker (app.rs:109) vs PACER_TICK ≈30.6 ms (apps/desktop/src/shell.rs:80) — two cadences; who wins and does the 10 ms tick starve the webview? Files: apps/desktop-tauri/src/app.rs, apps/desktop/src/shell.rs.
- `apps/web/src/host.rs:828` constructs its own `Engine` — does the web host poll it on a timer at all, or is it the CLI-REPL problem again? File: apps/web/src/host.rs.
