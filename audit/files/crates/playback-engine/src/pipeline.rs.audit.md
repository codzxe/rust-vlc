# AUDIT: crates/playback-engine/src/pipeline.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 3338 / 59 production functions + 21 test items |
| File verdict | REAL (library tick engine) — but see F-001: no shipped CLI/GUI driver ever spins it in wall-clock mode |
| Reachable from shipped entry point? | YES (call chain below) for the tick itself; the *continuous* driving loop is NO in the CLI and NO in the GUI (compile break) |
| Findings | S0:1 S1:0 S2:0 S3:1 S4:4 |
| Commands run | Read/Grep/sed only (no cargo available — run-dependent facts cite W00 baseline or UNVERIFIED) |

## 1. One-paragraph verdict

This file is the demux→queue→decode→sink tick engine: budget-bounded
(`TickBudget`: 64 demux events, 16 packets/limb, 8 flush calls),
non-blocking, with generation-stamped queues, a seek pre-roll hold (M10),
per-frame damage concealment/skip, a starve debounce, and a decoder
cache. As *library code* it is real, dense, and internally consistent —
the state-machine facts it reports (`primed`/`starved`/`eof_reached`/
`drained`) line up with the transition table in `state.rs`
(`translate_pipeline_signal`, state.rs:931-959). The wave-3 domain
question is answered by what is **missing around** this file: the tick
runs only when a host calls `Engine::poll`. The CLI REPL never pumps
between commands (run.rs:9-19 admits it; run.rs:226 blocks on
`read_line`), and the GUI's 10 ms pacer thread — the only production
caller of the wall-clock pump — is dead code because `app.rs:301` calls
`spawn_session_pump`, a function that does not exist anywhere in the
repo (only `spawn_event_pump` exists, app.rs:190). Result, matching the
W00 baseline exactly: `lumen open` reaches `Playing`, media time stays
0.000000 s until a human types `advance N`.

## 2. What actually works (verified)

— Call chain CLI (static, verified by Read/Grep):
`apps/cli/src/main.rs` → `run::repl_mode` (run.rs:180) →
`lumen_dispatch` → `CliHost::pump_pipeline` (host.rs:447-451:
`self.engine.poll()`) → `Engine::poll` (session.rs:1157-1177) →
`Session::tick_step` (session.rs:2428-2431:
`pipeline.tick(state, &self.runtime.budget)`) → **`Pipeline::tick`
(pipeline.rs:1904)**. The tick executes `demux_step` (2112), gated
`decode_step` (2269), drained check (1957-1959), priming (1967-2009),
buffer_fill (2014-2028), starve/debounce (2029-2081).
— Verified by W00 baseline (interactive `lumen open` reaches Playing;
`advance N` drives it to Ended/100%) that the tick machinery really
moves the session when pumped; 1751/1753 workspace tests pass (W00).
— Bounded-work proof (read): every loop in `tick`/`demux_step`/
`decode_step` is `for _ in 0..budget.*`; full queues park via the
single `pending` slot (2116-2134, 2193-2231); no blocking, no sleeps,
no threads in this crate.

## 3. Findings

### F-001 [S0] THE wave-3 question: nobody drives the clock in either shipped entry point
Where: cross-file — pipeline.rs:30-38 (design), apps/cli/src/run.rs:9-19
+ 220-287, apps/cli/src/host.rs:523-536, apps/desktop/src/shell.rs:279-282,
apps/desktop-tauri/src/app.rs:190/204/301, Cargo.toml:51

Code (pipeline.rs:32-35 — the honest design statement):
```
//! v0 is synchronous and thread-free: the pipeline advances only
//! when the session polls it, in **bounded** slices — at most
//! `budget.demux_events` demux events, `budget.packets_per_limb`
//! decode calls and `budget.flush_calls_per_limb` flush calls per
//! tick.
```
Code (apps/cli/src/run.rs:14-19 — the CLI admission):
```
//! nothing pumps; the explicit `advance [ticks]` host verb is the
//! run-to-end driver (deterministic, no wall-clock sleeps anywhere).
//! Continuous wall-time pumping joins with the session-thread card
//! (3.B3). `[PROPOSAL: the settle-after-command policy is this
//! host's; a `--wait` flag or event-driven pumping is a later card]`
```
Code (apps/desktop-tauri/src/app.rs:190 vs 301):
```
fn spawn_event_pump(app: AppHandle) {
...
            spawn_session_pump(app.handle().clone());
```
What is wrong: The wall-clock pump exists and is correct-looking —
`CliHost::pump_wallclock` (host.rs:523-536: anchor refresh, wall-bounded
render of only the quanta the wall owes, video scheduler ticks) — but:
(a) In the CLI its only callers are `#[cfg(test)]` tests
(host.rs:1892, 1904, 1912, 1933). The REPL loop (run.rs:220-287) blocks
at `input.read_line(&mut line)` (run.rs:226) and pumps only on commands
and on the `advance` verb (run.rs:251-252). Between commands, media time
is frozen — exactly W00's "`time` stays 0.000000s forever; only manual
`advance N` ticks drive it to Ended/100%".
(b) In the GUI, the only production path to the pump is the pacer thread
`spawn_event_pump` (app.rs:190-232, 10 ms `SESSION_TICK`, calls
`runtime.pump_session()` at app.rs:204 → `pump_wallclock` at
apps/desktop/src/shell.rs:279-282). But the spawn site calls
`spawn_session_pump` (app.rs:301), which is defined **nowhere** in the
repo (repo-wide grep: 1 hit, the call site; the defined function is
`spawn_event_pump`). That is E0425 — the desktop-tauri crate cannot
compile. It is invisible to the release build because the crate is
excluded from the workspace: root Cargo.toml:51 `exclude =
["apps/desktop-tauri"]` (members list at Cargo.toml:41 does not include
it). CI does have a dedicated lane that builds this crate
(.github/workflows/ci.yml:731 `cargo test --manifest-path
apps/desktop-tauri/Cargo.toml --locked --all-targets`, clippy at :733),
so this tree's CI must be red on that job — the rename regression
either postdates the last green run or the lane is being skipped;
UNVERIFIED which (no CI access in sandbox). So the "session thread
that DRIVES" documented at app.rs:38-55 is
never spawned in the only binary that could spawn it.
Why it matters: this is the user-visible core of a player — press play,
time advances. In the CLI it never does without manual pumping; in the
GUI the driving crate does not build. The engine-side loop is real; both
shipped drivers are absent. The docs' own module comment (app.rs:42 "the
shell's ticker thread … therefore IS the session thread") describes code
that is never reached.
What correct looks like: (1) fix app.rs:301 to call `spawn_event_pump`
and add desktop-tauri to CI compile checks (or the workspace); (2) give
the CLI a `--wait`/real-time mode that calls `pump_wallclock` on a
ticker between REPL lines, as the PROPOSAL note already sketches.
Confidence: HIGH — Verification: read (compile failure is static: no
definition/import exists; UNVERIFIED only insofar as I could not run
`cargo check -p` — no cargo in sandbox).

### F-002 [S3] Hold-capacity loss is counted as "stale frames dropped"
Where: pipeline.rs:2696-2697 (and the field doc 536-540)

Code:
```
            if !hold.push(frame) {
                self.stale_dropped = self.stale_dropped.saturating_add(1);
            }
```
What is wrong: a frame refused because the pre-roll **hold is full**
(capacity, D-1 backstop) is folded into `stale_dropped`, which
`TickReport.stale_frames` documents as "Frames dropped at the sink for
stale generations (doc 03 §5 checking point 5). Always zero in v0's
single-writer ticks" (pipeline.rs:536-540). The hold-full path is
reachable in v0 (a single decode call bursting past `hold_cap`,
documented at 2661-2664), and when it fires the engine reports it as
generation staleness — a fabricated diagnostic category.
Why it matters: observability lies; a debugger seeing `stale_frames > 0`
in a single-writer session would hunt for a race that cannot exist.
What correct looks like: a separate `hold_refused` counter, or at
minimum a comment on `stale_frames` admitting the conflation.
Confidence: HIGH — Verification: read.

### F-003 [S4] Inconsistent ns conversion: unchecked `as i64` next to the safe `nanos_i64`
Where: pipeline.rs:2592 vs 2538-2544

Code (2591-2593):
```
    fn observe_video_end(&mut self, pts: Stamp) {
        let Some(end) = pts.map_timestamp(|ts| ts.as_nanos().ok().map(|ns| ns.0 as i64)) else {
```
Code (2538-2544):
```
fn stamp_ns(stamp: Stamp) -> Option<i64> {
    let timestamp = stamp.timestamp().ok()?;
    match timestamp.as_nanos() {
        Ok(ns) => Some(nanos_i64(ns)),
        Err(_) => None,
    }
}
```
What is wrong: `ns.0 as i64` wraps on overflow while the sibling helper
routes through `nanos_i64` (crate::seek) for a checked conversion. Same
file, two disciplines for the same unit conversion.
Why it matters: F5's end-of-media gate consumes `last_end`; a wrapped
negative high-water would let `session.rs:2746`
(`clock_position_ns(...) >= end`) fire instantly — wrong early `Ended`.
Requires a pts near i64::MAX ns (~292 years), so not exploitable by any
real media; it is hygiene, not a crash.
What correct looks like: use `stamp_ns`-style checked conversion in
`observe_video_end`.
Confidence: HIGH — Verification: read.

### F-004 [S4] Subtitle lane byte cap contradicts its own "far below the A/V defaults" claim
Where: pipeline.rs:267-271 vs 418-425

Code (268-271):
```
    /// §4.4(b): "small and
    /// duration-only"; card 3.A3): subtitle packets are tiny, sparse
    /// and latency-tolerant, so the caps sit far below the A/V
    /// defaults
```
Code (420-425):
```
const fn default_subtitle_limits() -> QueueLimits {
    QueueLimits {
        max_packets: nonzero(128),
        max_bytes: nonzero(16 << 20),
        max_duration: media_types::Nanoseconds(10_000_000_000),
    }
}
```
What is wrong: subtitle `max_bytes` = 16 MiB is 16× the audio lane
(1 MiB) and 4× the video lane (4 MiB). Packets (128) and duration
(10 s) are indeed lower, but the byte cap is the largest in the plan,
and the doc `[PROPOSAL figures]` marker shows it was never ratified.
What correct looks like: either a byte cap below 1 MiB or a corrected
comment.
Confidence: HIGH — Verification: read.

### F-005 [S4] "ONE budget for both limbs" is actually a per-limb budget
Where: pipeline.rs:629-647 (doc) vs 626/2377-2390 (code)

Code (643-645):
```
/// is not playing. ONE budget for both limbs by design (design D-2:
/// the audio lane mirrors the video lane's shape) — the historical
```
Code (2379-2381):
```
                                        limb.conceal_failures =
                                            limb.conceal_failures.saturating_add(1);
                                        if limb.conceal_failures > VIDEO_CONCEALMENT_BUDGET {
```
What is wrong: `conceal_failures` is a field of `Limb` (line 626), so
each limb independently rides out 1000 consecutive damaged packets —
the documented "one shared budget" is two budgets. Behaviorally benign
(the per-limb form is arguably better), but the doc is false.
What correct looks like: fix the comment or hoist the counter to
`Pipeline`.
Confidence: HIGH — Verification: read.

### F-006 [S4] Starve debounce silently clamped, against the crate's own "reject, never clamp" doctrine
Where: pipeline.rs:1061 vs 191-193

Code (1061):
```
        self.starve_debounce = plan.starve_debounce_ticks.max(1);
```
Code (192-193):
```
/// out-of-range packet caps are **rejected, never clamped** (MP §5 —
/// the same posture as product.toml's own schema ranges).
```
What is wrong: a host that sets `starve_debounce_ticks: 0` (asking for
"re-entry fires immediately on every low tick") silently gets 1, which
is observably different only in the `episode_open` first-tick path —
minor, but the module's own stated posture for invalid plans is typed
rejection, and this is a quiet clamp with no typed error and no log.
What correct looks like: validate in `QueuePlan` (typed error) or
document the floor on the field.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Display for QueuePlanKind` | 182-189 | REAL | — | match, no panic |
| `Display for QueuePlanError` | 191-236 | REAL | — | typed, informative |
| `impl Error for QueuePlanError` | 238 | REAL | — | empty impl, fine |
| `QUEUE_*` consts | 244-250 | REAL | — | duplicated from schema "by design"; `tests/queue_policy.rs` pins it |
| `DEFAULT_STARVE_DEBOUNCE_TICKS` | 257 | REAL | — | PROPOSAL-marked |
| `QueuePlan::new` | 283-291 | REAL | — | subtitle + debounce defaults |
| `QueuePlan::with_subtitle` | 295-307 | REAL | — | |
| `QueuePlan::from_product_limits` | 325-370 | REAL | — | reject-never-clamp on packet caps; `audio_packets as usize` cast safe under the 64/8192 bounds |
| `QueuePlan::for_kind` | 381-388 | REAL | — | total over WiredKind |
| `QueuePlan::thresholds` | 399-415 | REAL | — | low bytes/duration set to caps (never-binding by construction, packets-only low axis) |
| `default_subtitle_limits` | 420-426 | REAL | S4 | 16 MiB byte cap contradicts doc (F-004) |
| `Default for QueuePlan` | 428-443 | REAL | — | 200/300 packets via packet-queue ctors |
| `nonzero` | 448-453 | REAL | — | literal-only use; 0→MIN fallback unreachable for current call sites |
| `Default for TickBudget` | 469-477 | REAL | — | 64/16/8, PROPOSAL-marked figures |
| `TickReport` struct | 484-545 | REAL | S3 | `stale_frames` doc lies about the hold-full fold-in (F-002) |
| `WindowOwner::*` | 822-850 | REAL | — | bitmask ops, correct |
| `ObserveWindow::observe` | 903-923 | REAL | — | first-frame latch; no allocation |
| `ObserveWindow::first_pts_ns` / `saw` | 926-939 | REAL | — | linear find over ≤3 entries |
| `ObserveWindow::held_count` | 942-946 | REAL | — | saturating add |
| `ObserveWindow::hold_slot` | 953-962 | REAL | — | borrow split for the sink |
| `Pipeline::new` | 999-1018 | REAL | — | no limbs yet |
| `Pipeline::tracks` | 1022-1024 | REAL | — | accessor |
| `Pipeline::wire` | 1051-1092 | REAL | S4 | one limb per kind; subtitle failure deselected not fatal; debounce clamp (F-006) |
| `Pipeline::build_limb` | 1104-1133 | REAL | — | fresh queue published at generation; `let _ = queue.flush` is a publish no-op |
| `Pipeline::start_demuxer` | 1141-1143 | REAL | — | delegates |
| `Pipeline::teardown` | 1153-1164 | REAL | — | flush→close→drop; held preroll counted into discards |
| `Pipeline::has_audio_limb` | 1170-1174 | REAL | — | F5 gate input |
| `Pipeline::last_video_end_ns` | 1180-1185 | REAL | — | max over limbs |
| `Pipeline::seekability` / `duration_ns` | 1189-1203 | REAL | — | delegate/track extents |
| `Pipeline::seek_flush` | 1239-1355 | REAL | — | mint semantics, decoder+cache flush+reconfigure, error paths propagate; 116 lines of careful ordering |
| `Pipeline::window_open_seek` | 1365-1403 | REAL | — | pre-allocated hold (D-1) |
| `Pipeline::window_arm_switch` | 1413-1455 | REAL | — | hold only if a seek also in flight |
| `Pipeline::window_end` / `window_live` | 1459-1473 | REAL | — | |
| `take_seek_flush_stale` / `take_preroll_discarded` | 1478-1488 | REAL | — | std::mem::take observability |
| `Pipeline::demuxer_reanchor` | 1498-1500 | REAL | — | delegate (D4 lives in demuxer) |
| `Pipeline::seek_preroll` | 1512-1533 | REAL | — | Ready requires every limb saw-or-never-deliver |
| `Pipeline::release_preroll_hold` | 1552-1604 | REAL | — | pts-ordered release, stale re-check, consumer refusal counted; O(n log n) once per seek |
| `Pipeline::end_seek` | 1615-1625 | REAL | — | discard+count held frames |
| `Pipeline::switch_preroll` | 1634-1660 | REAL | — | no-limb = Ready (honest completion) |
| `Pipeline::end_switch` | 1664-1666 | REAL | — | |
| `Pipeline::obtain_decoder` | 1679-1710 | REAL | — | cache-first, configure on hit; swap_remove reorders cache (fine, keyed lookup) |
| `Pipeline::switch_limbs` | 1750-1858 | REAL | — | prepare-before-commit retire/install; drain refusal still parks (comment 1806-1810) |
| `Pipeline::take_ended_limbs` | 1865-1867 | REAL | — | Drain keeps capacity |
| `Pipeline::can_never_deliver` | 1874-1876 | REAL | — | eof && !pending && drained && empty |
| `Pipeline::paces` | 1895-1900 | REAL | — | subtitle + switch-installed exempt |
| `Pipeline::tick` | 1904-2083 | REAL | S0(context) | the bounded tick — correct in isolation; never continuously driven by a shipped host (F-001) |
| `Pipeline::starvation_counters` | 2090-2092 | REAL | — | snapshot |
| `Pipeline::is_drained` | 2100-2107 | REAL | — | empty-limb set counts as drained (documented) |
| `Pipeline::demux_step` | 2112-2188 | REAL | — | pending re-offer FIFO; re-offer refusal ends slice (data-loss fix 2124-2133); EOF once; all loops bounded |
| `Pipeline::push_packet` | 2193-2231 | REAL | — | unwired tracks dropped by policy; Full→pending; Stale/Closed silently ignored (2229, justified: unreachable single-writer) |
| `Pipeline::decode_step` | 2269-2533 | REAL | S4 | damage isolation, conceal/skip, budget exhaustion terminal, EOF drain gated on queue empty + audio ready; per-limb budget vs doc (F-005) |
| `stamp_ns` | 2538-2544 | REAL | — | checked conversion via nanos_i64 |
| `per_frame_damage` | 668-676 | REAL | — | Corrupt + Decoder{Bitstream} only |
| `concealment_budget_exhausted` | 683-694 | REAL | — | typed ResourceLimit with source chain |
| `LimbSink::observe_video_end` | 2591-2597 | REAL | S4 | `as i64` wrap (F-003) |
| `LimbSink::conceal_last` | 2615-2637 | REAL | — | stale latch never repeated; hold ⇒ no presentation |
| `LimbSink::hold_is_full` | 2641-2643 | REAL | — | |
| `PrerollHoldSlot::is_full`/`push` | 2657-2671 | REAL | — | hard cap, refusal bool |
| `FrameSink::accept for LimbSink` | 2675-2723 | REAL | S3 | generation check first, F5 fold, hold capture (F-002 mislabeled counter), latch-after-accept |
| `FrameConsumer::ready_for` (default) | 167-170 | REAL | — | default `true`; the audio gate is the real seam (host side) |

## 5. Test honesty (in-file `mod tests`, 2726-3338)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `packets_axis_is_the_starvation_axis` | YES | asserts real `Watermark::evaluate` on the plan's thresholds; breaks if thresholds change |
| `not_ready_consumer_parks_audio_decode_and_resumes` | YES | counts real accepts through a gated consumer + real tick; 0 accepts while parked, 4 after unpark |
| `first_stall_of_a_fresh_episode_fires_at_one_tick` | YES | scripted queue levels through real `tick`; `starved` flag + counter pinned |
| `single_tick_reentry_dips_do_not_refire` | YES | the debounce contract, levels scripted by hand |
| `sustained_low_fires_at_the_debounce_count` | YES | exact streak counts |
| `full_recovery_closes_the_episode` | YES | post-prime dip re-fires |
| `seek_mint_resets_the_debounce_state` | PARTIAL | asserts limb fields after `seek_flush`, not behavior through a tick |
| `debounce_one_is_the_undebounced_behavior` | YES | three consecutive lows, three signals |
| `unknown_duration_mix_reaches_low` | YES | duration-less packets still starve |
| `parked_codec_rides_the_recycle_path` | YES | records real configure calls; pins codec order and no "unknown" fabrication |

Honest overall: the fakes (`StalledDemuxer`, `NullDecoder`) are
demuxer/decoder *seams*, not mocks of the code under test — the code
under test is `Pipeline` itself, driven for real. Not executed by me
(no cargo); W00 ran the workspace suite: 1751 pass / 2 fail (golden
drift) / 25 ignored. UNVERIFIED individually.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the pipeline advances only when the session polls it" | pipeline.rs:32-33 | REAL | session.rs:2428-2431 `pipeline.tick(...)`; poll session.rs:1157 |
| "Nothing blocks, nothing spins" | pipeline.rs:36-38 | REAL | all loops budget-bounded (2117, 2317, 2452); single `pending` park slot |
| "the audio pump (2.C3) and the video scheduler (2.C4) implement this [FrameConsumer]" | pipeline.rs:22-24 | REAL-UNPROVEN | CLI passes `CliFrameConsumer` (apps/cli/src/pipeline_host.rs, host.rs:97-99); the audio *device* is null-only (W00 baseline) |
| "v0 is synchronous and thread-free" | pipeline.rs:32 | REAL | no `thread::`/`spawn` anywhere in the crate's pipeline path |
| "ONE budget for both limbs" | pipeline.rs:643-645 | FALSE | per-limb `conceal_failures` (F-005) |
| "subtitle … caps sit far below the A/V defaults" | pipeline.rs:268-271 | FALSE | 16 MiB > 4 MiB > 1 MiB (F-004) |

## 7. Dead / duplicated / unreachable code

- Nothing dead in this file. The unreachable-in-production item is the
  *driver*, not this code: `pump_wallclock` (apps/cli/host.rs:523) has
  no CLI production caller (F-001).
- `Pipeline::tick`'s Stopped/Error states are unreachable through
  `Engine::poll` (session.rs:1172-1177 only routes
  Opening/Buffering/Playing/Paused) — consistent, no defect.

## 8. Dependencies & flags

- Depends on: `codec_api` (Decoder/FrameSink/DecodeOutcome),
  `packet_queue` (BoundedQueue), `plugin_api` (Demuxer/DemuxEvent),
  `media_types`. All workspace members (crates/*), no feature gates on
  this file's code path. The playback-engine crate is built in the
  release binary (W00 baseline: release build PASS covers it).

## 9. Open questions for the next agent

- Wave 13/14 (apps/cli, apps/desktop-tauri): confirm `spawn_session_pump`
  (app.rs:301) is a compile error under `cargo check` — I could not run
  cargo (UNVERIFIED); check whether any CI job builds the excluded
  desktop-tauri crate (a "tauri-check" run id 43384519598 is cited at
  app.rs:253-254, but that predates nothing — the call site is broken
  *now*).
- Wave 3 A02 (session.rs/state.rs): my F-001 chain terminates at
  `translate_pipeline_signal`; the Buffering+EOF exit relies on
  `EofReached` firing *while* Buffering (state.rs:944) — if EOF already
  fired in Playing and an underrun followed, `starved` is suppressed at
  EOF (pipeline.rs:2077 `report.starved = !self.eof && starved`), so the
  wedge I hunted for appears closed; please verify with the contract
  suite.
- Wave 8 (audio-output): who implements `ready_for(Audio)` in
  production — the null device always-ready, or the CLI host gate?
