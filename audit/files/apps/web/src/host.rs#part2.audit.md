# AUDIT: apps/web/src/host.rs#part2 (lines 780-1705)
| Field | Value |
|---|---|
| Auditor | W12-A05 |
| Date | 2026-11-21 |
| Chunk boundary | part2 = 780-1705 (`pub struct WebHost` at 780 through the end of `impl WebHost` at 1705). Part1 = 1-779 (types/consumer/decoder table, W12-A04); part3 = 1706-1954 (free helper fns, W12-A06); part4 = 1955-2651 (`mod tests`, W12-A03). Boundary picked at the struct/impl seam so no other agent re-audits this range. |
| Lines / functions | 926 / 45 (1 struct with 26 fields + 44 methods) |
| File verdict | REAL (core pump/dispatch engine is genuine and test-executed; two S2 behavioral defects in the reopen/timeout paths) |
| Reachable from shipped entry point? | NO — the whole lumen-web crate is UNREACHABLE from shipped entry points (W01-A10 F-001: browser UI uses DemoTransport, ui/src has no Worker; only rc-smoke check 6 compiles the module). Executed headless-native by this crate's own tests via `cargo test` (baseline repair run, below). |
| Findings | S0: 0 S1: 0 S2: 3 S3: 2 S4: 2 |
| Commands run | Read host.rs 1-1709 (full context through part3 start); `rg -n "last_fatal" apps/web/src/host.rs` (set at 1308, only consumer 1546 — no teardown clear); `sed -n '1860,1940p' crates/playback-engine/src/session.rs` + `rg -n "fn close_plan" -A 28 crates/playback-engine/src/state.rs` (Close IS accepted from Error state: `S::Error => CLOSE_ONLY`); `rg -n "fn session_id" -A 3 crates/playback-engine/src/session.rs` (session stays Some until Close); `rg -n "MAX_PENDING_MESSAGES" apps/web/src/abi.rs` (4096); `rg -n "NANO_BASE|audio_master" crates/media-clock/src/mapping.rs` + `sed -n '1717,1740p' apps/web/src/host.rs` (clock is ns-based → `position_us` = ticks/1000 correct); `rg -n "TracksChanged" -A 5 crates/playback-engine/src/event.rs` + `rg -n "fn tracks\(&self\)" -A 3 crates/playback-engine/src/pipeline.rs` (tracks/selected are parallel slices → `zip` cannot truncate); `rg -n "seekable" apps/cli/src/host.rs apps/web/src/host.rs` (both hard-code true; CLI serves seek, web refuses it); `rg -n "SourceSpec|blob|http" apps/web/src/open.rs` (web serves ONLY `blob:` URLs — http/file are typed refusals); baseline `orchrun-test-workspace-nofailfast.log.txt:3115-3128` (host::tests 15/15 ok) — note baseline run #3 (`test-workspace.log.txt`) ABORTED at the desktop zero-alloc failure BEFORE apps/web ran; only the no-fail-fast repair run executed these tests. NOTE: quotes below were verified with the Read tool (this sandbox's `sed` mangles some byte sequences). |

## 1. One-paragraph verdict
The pump architecture is real, carefully bounded, and honestly documented — count-form budgets, latest-wins mailbox, autoplay park, staleness filter, backpressure cap — and all 15 host tests ran green in the no-fail-fast repair run. But the session-lifecycle half has a genuine wedge: `last_fatal` is set by any fatal observed during pump slices and is cleared ONLY by the next `open_session` (1546), never by `teardown_session_scoped` (1567-1577) — so a fatal that lands after the 8-poll settle window (the COMMON case: open answers Accepted while Buffering, the decode fatal arrives on later pump slices, exactly the baseline MP4 h264 pattern) poisons every subsequent open with the dead session's error, and because the wrongly-failed new session is in Opening/Buffering (not Error), `teardown_after_fatal` skips the Close submit and orphans a live engine session the host can never close again (`handle` is None, `engine.session_id()` stays Some) — the worker is permanently unable to open anything. Add a snapshot that hard-codes `seekable: true` while `playback.seek` is refused unserved, and a receipt-leak that pins `busy()=true` forever after one 64-poll timeout, and the "nothing blocks, nothing spins / host is reusable" claims are contradicted by the code's own edge paths.

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3115-3128` → `host::tests::{audio_fill_is_bounded_per_tick, commands_before_boot_refuse_typed, frames_to_ns_is_exact_and_saturating, pause_freezes_the_counter_clock, pump_slices_play_the_fixture_to_its_end, shutdown_closes_and_acknowledges, pump_ticks_are_capped_and_flushes_bounded, snapshots_answer_over_the_open_fixture, the_autoplay_gate_parks_freezes_and_resumes, the_boot_snapshot_reports_the_active_backend, surface_loss_falls_then_degrades_to_audio_only, unserved_surface_refuses_typed, wasm32_profile_decoder_seam_refuses_typed, the_present_path_ships_one_frame_per_pump}` all `ok` (orchestrator repair run, `cargo test --workspace --release --locked --no-fail-fast`).
— Verified by call-chain read: `pump` → `fill_audio` → `NullDevice::render` (crates/audio-output/src/null.rs:292) advances the sample counter; the session clock is `MediaClock::audio_master(sink.clock_source(), …, NANO_BASE)` (host.rs:1726-1735, part3) — so `position_us()` = ns-ticks/1000 is unit-correct (media-clock mapping.rs:50 `NANO_BASE` is 1/1e9).
— Verified by read of playback-engine: `close_plan(SessionState::Error) => CLOSE_ONLY` (state.rs:886-891) — Close works from Error, which is what makes the F-001 brick sequence reachable (the user CAN close the failed session; the poison is the stale `last_fatal`, not a stuck engine).

## 3. Findings

### F-001 [S2]
Where: apps/web/src/host.rs:1307-1309 (set), 1546-1548 (only clear), 1558-1563 (wrong-state teardown), 1567-1577 (teardown that never clears it)
Code (set during any pump poll):
```
                        if let EngineEventKind::Fatal { error } = &event.kind {
                            self.last_fatal = Some(open::wire_error(error));
                        }
```
Code (the only consumer, at the NEXT open):
```
        if let Some(error) = self.last_fatal.take() {
            self.teardown_after_fatal();
            return Err(error);
        }
```
Code (teardown clears everything EXCEPT last_fatal):
```
    fn teardown_session_scoped(&mut self) {
        self.handle = None;
        self.consumer = None;
```
What is wrong: `last_fatal` is session-scoped state that is never reset when the session dies. A fatal that arrives after `open_session`'s settle window (≤8 polls, which typically ends at the first Buffering transition — the decode fatal arrives later on pump slices, exactly the baseline MP4 h264 sequence `tracks ready → Buffering → decode.unsupported → Error`) survives `SessionClose` → `session_closed` → `teardown_session_scoped`. The next `session.open` of a perfectly valid media runs its settle polls, then `take()`s the PREVIOUS session's fatal and fails. Worse: the new session is in Opening/Buffering, so `teardown_after_fatal`'s guard `if self.state() == PlaybackState::Error` is false — no Close is submitted — yet `teardown_session_scoped` drops `self.handle` while `engine.session_id()` remains Some. Every future open then hits `open_session`'s guard ("a session is already open — close it first") and every play/pause/close hits `submit_and_await`'s "no session open" (handle is None). `shutdown()` cannot fix it either (it too needs `self.handle`). The worker is permanently wedged.
Why it matters: open-failed-media → close → open-good-media is the most basic error-recovery flow. The in-code comment "close it so the host is reusable" (1556-1557) is false on this path. No test in part4 covers reopen after a mid-session fatal (test names audited: 2024-2651 cover happy lifecycle, boot, surface loss, autoplay only).
What correct looks like: clear `last_fatal` in `teardown_session_scoped` (and capture the fatal only within the open settle window, e.g. a generation/epoch check), and make `teardown_after_fatal` submit Close whenever `self.handle.is_some() && self.engine.session_id().is_some()` — not only in the Error state.
Confidence: HIGH — Verification: read (all four code sites traced; engine semantics verified: `close_plan` accepts Error→CLOSE_ONLY at state.rs:886-891; `session_id()` stays Some until Close at session.rs:1107-1109). Runtime execution: UNVERIFIED (rule 8 — no test exercises it; a REQUEST-ORCH-RUN reproduction is possible but the static chain is complete).

### F-002 [S2]
Where: apps/web/src/host.rs:1660 (with 762-775 SERVED and part3's 1836-1842)
Code:
```
                seekable: true,
```
What is wrong: `media.get-info` hard-codes `seekable: true`, but `playback.seek` is not in `SERVED` (762-775) and `route`'s fall-through answers it `unsupported-for-host` via `unserved`/`owner_of` (part3, 1810/1836-1842: "the seek boundary's host duties (ring discard + generation publish) land with the …"). The snapshot advertises a capability the host refuses. The CLI twin also hard-codes `true` (apps/cli/src/host.rs:1307) but the CLI actually serves seek; here the field is a lie.
Why it matters: a web UI consuming `media.get-info` renders a scrubber; every seek command then fails. It also contradicts the module's own "honest posture" doc (host.rs:50-61). No test pins `seekable` at all (`snapshots_answer_over_the_open_fixture`, 2156-2210, asserts duration/time/tracks/engine-info only) — the dishonesty is unpinned.
What correct looks like: report the truth the host can serve (`seekable: false` until the seek boundary lands), or derive it from the served command table.
Confidence: HIGH — Verification: read (SERVED list + unserved path + CLI contrast all verified by grep).

### F-003 [S2]
Where: apps/web/src/host.rs:1360-1367 (busy), 1605-1625 (await_receipt timeout leaks the receipt)
Code (busy):
```
        let engine_busy = match self.state() {
            PlaybackState::Opening | PlaybackState::Buffering | PlaybackState::Playing => true,
            PlaybackState::Ended => self.ring_backlog() > 0 || !self.event_backlog.is_empty(),
            _ => !self.event_backlog.is_empty() || !self.pending_receipts.is_empty(),
        };
```
Code (the leak):
```
        for _ in 0..AWAIT_POLLS {
            self.poll_engine();
            if let Some(position) = self
                .pending_receipts
                .iter()
                .position(|receipt| receipt.request == request)
```
What is wrong: when `await_receipt`'s 64-poll bound expires it returns the typed "engine never answered" error, but the submitted command's receipt typically arrives on poll 65+ (during later pump slices) and is pushed into `pending_receipts`, where nothing ever removes it: `await_receipt` removes only receipts matching ITS request id (unique per `allocate()`), and only `teardown_session_scoped` (session close/fatal) clears the vector. In Paused/Stopped/Idle/Error the `_` arm of `busy()` then returns true forever → the JS glue re-arms `Pump` from `setTimeout(0)` indefinitely (module doc 17-20: "the JS glue re-arms the pump … while the PumpReport says busy").
Why it matters: the module's headline claim "Nothing blocks, nothing spins" (host.rs:17) is violated by a permanent busy-spin (battery/CPU burn) that persists until the user closes the session. The timeout path exists precisely because the designers consider an engine stall reachable — the leak converts that anticipated edge into an unbounded loop.
What correct looks like: on timeout, remember the orphaned request id and discard its receipt when it lands (or age receipts out of `pending_receipts`), and/or stop counting stale receipts in `busy()`.
Confidence: HIGH — Verification: read (removal paths traced: only 1616 `remove(position)` and 1575 `clear()`; no other consumer of `pending_receipts`).

### F-004 [S3]
Where: apps/web/src/host.rs:1227-1229
Code:
```
        // One scratch render buffer per fill (D-1): reused across the
        // slice's quanta; the tail render uses a prefix of it.
        let mut scratch = vec![0.0_f32; RENDER_QUANTUM * channels];
```
What is wrong: the buffer labeled with the D-1 (zero-alloc steady state) discipline is a fresh heap allocation per `fill_audio` call — i.e. per pump TICK, up to 32 per pump message (SliceBudgets::default, 164-174), ≈11.7 KB × channels each. At a 60 Hz re-arm that is megabytes/second of allocation churn on the wasm GC heap. The struct's own `outbox_scratch` (818-820) is the real D-1 pattern (take → clear → drain → move back); this one is not.
Why it matters: the comment borrows the zero-alloc label for a per-tick allocation — a misleading claim in a file whose discipline is otherwise explicit — and the churn is real on the target (wasm32) heap.
What correct looks like: hoist the scratch into `WebHost` next to `outbox_scratch`, or size it once per session (channels are fixed at open).
Confidence: HIGH — Verification: read (call frequency: `pump` → per-tick `fill_audio`, 1123-1129).

### F-005 [S3]
Where: apps/web/src/host.rs:1243-1248 (with 1669-1694)
Code:
```
            } else {
                // No audio lane: the silence clock (see the module
                // docs) — a whole quantum of silence advances the
                // counter, nothing else does.
                RENDER_QUANTUM
            };
```
What is wrong: for video-only sessions the silence render is NOT gated on `Ended` — after end-of-media each further pump tick still advances the sample-counter clock by up to 16 quanta past the media end (the lane case correctly drains only the residue, 1233-1237). `current_time` clamps only the `fraction` (1678); the reported `time: Timestamp(position_us.max(0))` (1690) is unclamped, so position can overshoot duration until the JS stops re-arming.
Why it matters: position/duration overshoot in the snapshot API for every video-only (`--no-audio`) media; a UI progress bar can transiently exceed 100% by more than the clamped fraction admits.
What correct looks like: gate the no-lane silence render on `!ended`, or clamp `time` to `duration` like `fraction` is.
Confidence: MEDIUM — Verification: read (overshoot magnitude bounded by re-arm latency; not executed).
### F-006 [S4]
Where: apps/web/src/host.rs:1012-1024, 1066-1068
Code:
```
            HostToWorker::Boot { renderer } => {
                self.booted = true;
```
What is wrong: `Boot` is accepted at any time — a second Boot mid-session re-runs `render_lane.boot(caps)` while a mailbox present may be staged — and `Shutdown` never resets `booted`, so a host that answered `ShutdownComplete` still dispatches non-Boot messages afterwards. Both lifecycle edges are unguarded and unpinned by tests.
Why it matters: protocol robustness debt on the worker's state machine; a page bug (double boot, post-shutdown message) gets silently tolerated instead of a typed control error — inconsistent with the file's own "typed, never silent" posture.
What correct looks like: refuse re-Boot and post-Shutdown messages with `ControlError` (the `not_booted` twin, 1002-1010).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| struct WebHost (26 fields) | 780-821 | REAL | — | all fields consumed; `last_fatal` lifecycle is F-001 |
| WebHost::new | 826-855 | REAL | — | audio_rate default 48_000 matches open_null_lane fallback (part3 1723) |
| profile | 859-861 | REAL | — | trivial accessor |
| set_budgets / budgets | 864-872 | REAL | — | test seam; tests tighten (part4 2070) |
| events | 877-888 | REAL | — | flushed(outbound) + backlog in order; test-evidence helper |
| state_history | 892-894 | REAL | — | grows with StateChanged events; bounded by state machine |
| counters | 898-902 | REAL | — | delegates to consumer; default when none |
| state | 906-908 | REAL | — | `engine.state().as_wire()` |
| session_id / generation | 912-920 | REAL | — | delegates |
| position_us | 926-931 | REAL | — | ns→µs correct (NANO_BASE verified); 0 with no clock |
| take_outbound | 935-937 | REAL | — | drain(..) — the ABI pull |
| take_pending_frame | 942-944 | REAL | — | one in flight; displaced untaken present is latest-wins (documented 1158-1163) |
| booted_capabilities | 951-958 | REAL | — | patches renderer row with lane truth or honest "unprobed" |
| render_snapshot / render_backend | 964-972 | REAL | — | delegates to RenderLane (backend.rs, W12-A08) |
| present_stats | 977-981 | REAL | — | delegates; default when no consumer |
| handle | 984-1070 | REAL | S4 | backpressure cap 4096 (abi.rs:97) clears+drops — documented; Boot/Shutdown edges = F-006; AudioContext discards `GateChange::Resumed` intentionally (resume unfreezes via is_parked, fill_audio 1213) |
| handle_control_error | 1075-1080 | REAL | — | control-lane twin; no cap (outbound cap enforced at handle entry) |
| shutdown | 1085-1107 | REAL | S4 | bounded close (AWAIT_POLLS), flush, ack; cannot close an orphaned session (F-001); does not reset booted (F-006) |
| pump | 1120-1138 | REAL | — | caps ticks at max_ticks_per_pump; roles in documented priority order; executed green (part4 2024, 2070) |
| present_one | 1145-1166 | REAL | — | one present per pump message; only while lane active; executed green (part4 2407) |
| observe_autoplay | 1171-1176 | REAL | — | exactly-once GestureRequired; executed green (part4 2591) |
| wants_audio | 1181-1188 | REAL | — | join condition matches autoplay.rs docs |
| fill_audio | 1199-1255 | REAL | S3/S4 | pause/park/ended gates correct; per-tick scratch alloc = F-004; no-lane Ended overshoot = F-005; bounded by audio_quanta_per_tick (part4 2087 green) |
| lane_exists | 1260-1267 | REAL | — | facts-driven; matches open path rule |
| audio_channels | 1270-1281 | REAL | — | first audio fact; fallback 2 |
| poll_engine | 1288-1291 | REAL | — | poll + drain |
| drain_outputs | 1295-1333 | REAL | S2 | sets last_fatal (F-001); TracksChanged zip safe (parallel vectors, pipeline.rs:1022-1023); staleness filter applied; scratch reused (real D-1) |
| flush_events | 1339-1353 | REAL | — | bounded by events_per_flush; remainder carried; busy stays true |
| busy | 1360-1367 | REAL | S2 | re-arm truth; stale-receipt leak = F-003 |
| present_pending | 1372-1376 | REAL | — | mailbox tail |
| ring_backlog | 1379-1381 | REAL | — | device ring read |
| dispatch | 1389-1398 | REAL | — | echoes request_id; session/generation at answer time |
| route | 1402-1437 | REAL | — | served table + typed refusals; Close `.inspect` teardown only on Ok — correct (rejected close keeps session); executed green (part4 2115) |
| open_session | 1441-1554 | REAL | S2 | option refusals typed; blob-only source (open.rs 129-186); settle polls bounded; stale-fatal consume = F-001; on engine.open error no host state leaks (locals dropped) |
| teardown_after_fatal | 1558-1563 | REAL | S2 | Error-only Close guard orphans non-Error sessions (F-001); `let _ =` swallows the close result — acceptable only because teardown follows regardless |
| teardown_session_scoped | 1567-1577 | REAL | S2 | clears 9 halves; NOT last_fatal/event_backlog/state_history/audio_rate (F-001) |
| session_closed | 1580-1582 | REAL | — | thin wrapper |
| submit_and_await | 1586-1602 | REAL | — | NotReady when handle None (the F-001 brick surfaces here) |
| await_receipt | 1605-1625 | REAL | S2 | 64-poll bound; linear scan; timeout leaks the receipt (F-003) |
| receipt_outcome | 1628-1645 | REAL | — | Accepted/Completed/Rejected mapped; rejection_wire is part3 |
| media_info | 1650-1667 | REAL | S2 | seekable hard-coded true = F-002; title/metadata/chapters empty by v1 scope |
| current_time | 1670-1694 | REAL | S3 | buffered=[0..duration] honest for blob-only sources (verified open.rs); fraction clamps, time does not (F-005) |
| duration | 1697-1704 | REAL | — | delegates to facts |

## 5. Test honesty (tests live in part4, W12-A03's range; assessed for cross-references only)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| pump_slices_play_the_fixture_to_its_end (2024) | YES | full lifecycle to Ended over pump slices; any role regression breaks it (executed ok, repair-run 3119) |
| the_autoplay_gate_parks_freezes_and_resumes (2591) | YES | position freeze + GestureRequired once (3124 ok) |
| shutdown_closes_and_acknowledges (2281) | YES | happy shutdown only — does NOT cover reopen-after-fatal (F-001 gap) |
| snapshots_answer_over_the_open_fixture (2156) | YES for duration/time/tracks/engine-info; NO for seekable/buffered | never asserts `seekable` — F-002 unpinned |
| NO TEST | — | reopen-after-mid-session-fatal (F-001) and receipt-timeout (F-003) have zero coverage |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Nothing blocks, nothing spins" | host.rs:17 (module doc) | FALSE on an edge | F-003: leaked receipt pins busy()=true → permanent setTimeout(0) re-arm |
| "close it so the host is reusable" (after fatal) | host.rs:1556-1557 | FALSE for mid-session fatals | F-001: stale last_fatal kills the next open AND orphans its engine session |
| "the served command surface is the first vertical's playing subset … playback.seek answers the typed refusal" | host.rs:52-56 | REAL | SERVED 762-775 excludes seek; unserved/owner_of (part3 1810/1837) — but contradicted by seekable:true (F-002) |
| "audio-critical first … bounded by SliceBudgets" | host.rs:12-20 | REAL | pump 1120-1138 order + caps; test 2070/2087 green |
| "steady state allocates nothing" (D-1) | host.rs:818-820 | REAL for outbox_scratch; MISLEADING for fill_audio | F-004: per-tick scratch allocation |

## 7. Dead / duplicated / unreachable code
None dead in this range; every method has a caller (handle→route→…, abi/worker drive handle/pump; tests drive the rest). The whole crate is unreachable from shipped entry points (see header). `state_history`/`events` are test-evidence accessors that production JS never reads — acceptable, documented as such.

## 8. Dependencies & flags
Consumes from part1 types (WebFrameConsumer, WebDecoderProvider, SliceBudgets, PumpClock, engine_info, command_listings), part3 helpers (open_null_lane, not_ready, wire_duration, enrich_track_info, unserved, rejection_wire, session_error_wire), crate::{abi::MAX_PENDING_MESSAGES, audio::autoplay, capabilities, open, protocol, render, source}. `builtin-codecs` feature gates only the decoder table (part1); no cfg in this range except none. No new deps.

## 9. Open questions for the next agent
- W12-A03 (part4 tests): does any test sequence open→fatal-mid-pump→close→reopen? If not, F-001 is untested by design — consider a REQUEST-ORCH-RUN test.
- W12-A06 (part3): `unserved`/`owner_of` strings — confirm the seek refusal message matches F-002's contradiction analysis.
- W12-A08 (abi.rs): confirm the JS glue actually honors `PumpReport.busy` for re-arming (F-003's spin depends on it).
