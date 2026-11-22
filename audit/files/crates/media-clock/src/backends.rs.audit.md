# AUDIT: crates/media-clock/src/backends.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations — run facts cite W00 baseline) |
| Lines / functions | 2117 / 34 production items (9 types, 25 fns/methods) + 36 `#[test]` fns + 3 test fixtures |
| File verdict | UNREACHABLE (code itself is REAL-UNPROVEN as a library; zero shipped callers) |
| Reachable from shipped entry point? | **NO** — proof in F-001 |
| Findings | S0:1 S1:0 S2:1 S3:2 S4:1 |
| Commands run | `rg -n "with_backends|on_master_lost|mark_ended|\.seek_back|observe_video|take_master_change|switch_master"` over crates/ + apps/ → only media-clock internal hits; `sed -n` quote verification (all quotes below re-printed from real file); `grep -c "#\[test\]"` → 36 |

## 1. One-paragraph verdict

This file implements card 3.B2 — multi-master clock backends: video-master steering (`observe_video`), external-device-master mode, the deterministic master-loss demotion chain (`on_master_lost`/`plan_fallback`), the Ended latch (`mark_ended`), and the Ended seek-back re-arm (`seek_back`). The implementation itself is careful, allocation-bounded, panic-free at the edges (i128 math, saturating carries, typed refusals), and covered by 36 behavioral unit tests over deterministic fakes. **None of it is reachable from any shipped entry point.** No code outside `crates/media-clock` calls `with_backends`, `on_master_lost`, `switch_master`, `observe_video`, `mark_ended`, `seek_back`, or `take_master_change` (verified by repo-wide grep, F-001). The file's own header admits the engine rewiring was deferred (lines 70-80). Meanwhile the shipped CLI's interactive clock never advances without the manual `advance` verb — exactly the problem this module's wall-driven backends would have addressed (F-002). This is the project's signature pattern: a polished, well-tested library card whose integration "card" was never written, while a gate document declares the capability family "MET".

## 2. What actually works (verified)

- Deterministic pure fallback planner `plan_fallback` (559-572) plus an exhaustive 2^4×2^3×5 = 640-cell test over wirings × loss-sets × current masters (`fallback_plan_is_deterministic_over_the_full_matrix`, 1166-1236) — the planner's contract is genuinely pinned by test.
- The bounded `EventRing` (510-551): fixed 8 slots, FIFO index math verified correct by hand (`oldest = (head + CAP - len) % CAP`), overflow counted not grown, tested at `event_ring_overflow_counts_instead_of_growing` (2028-2049).
- Tests are behavioral, not tautological: they assert exact positions, epochs, and event payloads after fake-counter/fake-wall advances (e.g. `master_loss_chain_audio_video_free_runs_with_events`, 1238-1309). They would fail if the carry/re-anchor/epoch arithmetic broke.
- W00 baseline (audit/runs/baseline/cargo-test.log) shows the media-clock suite green as part of the 1751-pass workspace run (I could not re-run cargo — ADAPTATION; the suite list is cited from the W00 log, UNVERIFIED by me).

## 3. Findings

### F-001 [S0] — The entire card-3.B2 surface is dead code in every shipped binary; the phase-3 gate claims the capability is "MET"

Where: crates/media-clock/src/backends.rs:689-719 (constructor) and 736-931 (the five engine-facing methods); the admission of non-integration at 70-80.

Code (backends.rs:70-79, verbatim):
```rust
//! # Engine wiring (handoff note)
//!
//! playback-engine currently constructs single-master clocks and
//! drives seeks through `reanchor` + `set_generation`; adopting the
//! backend surface means constructing via
//! [`MediaClock::with_backends`], reporting stream losses through
//! [`MediaClock::on_master_lost`], steering per presented frame in
//! video-master mode, calling [`MediaClock::mark_ended`] at EOF and
//! [`MediaClock::seek_back`] for the play-from-ended restart. That
//! rewiring is a playback-engine card; this crate's 3.B1 API stays
```

Code (backends.rs:689-693, verbatim):
```rust
    pub fn with_backends(
        backends: ClockBackends,
        media: Timestamp,
        rate: Rate,
    ) -> Result<MediaClock, media_types::EngineError> {
```

What is wrong: `with_backends` and every method that only exists on a backends-constructed clock (`on_master_lost` 736, `switch_master` 769, `observe_video` 804, `mark_ended` 857, `seek_back` 881, `take_master_change` 922, `master_changes_lost` 929) have **zero callers** outside `crates/media-clock` itself. Grep proof (repo-wide, run by me): the only hits for these names are `backends.rs` (definitions + own `mod tests`), `clock.rs:2` and `lib.rs:1` (doc-links/re-exports), and `tests/zero_alloc_backends.rs:1`. `apps/cli`, `apps/desktop-tauri`, `crates/playback-engine`, `crates/video-engine`, `crates/audio-engine`, `crates/audio-output` — no hits. The shipped CLI constructs only the 3.B1 single-master clocks (`apps/cli/src/pipeline_host.rs:1605` `MediaClock::audio_master(sink.clock_source(), …)`; `apps/cli/src/pipeline_host.rs:1629-1634` `MediaClock::system_monotonic(Arc::new(media_clock::InstantSource::new()), …)`).

Why it matters: docs/gates/phase3-exit.md:31 declares the "clocks … capability famil[y]" **"MET (merged)"** citing "wave-5 merges dce1b64 (3.B2)", and phase3-exit.md:106 marks card 3.B2 "COMPLETE (merged)". A card whose surface no shipped binary can reach is not a completed capability; it is a library plus an unwritten integration. Under §4 this is a false capability claim attached to a blocker for the product's core (the interactive clock problem, F-002), so S0 stands.

What correct looks like: playback-engine's session constructs its clock via `MediaClock::with_backends` (wall + audio counter + video declaration), reports track EOD through `on_master_lost`, steers in video-master mode, and uses `mark_ended`/`seek_back` — or the gate documents the capability as "library-only, engine adoption pending".

Confidence: HIGH — Verification: read (repo-wide grep + call-site reads; no execution per ADAPTATION).

### F-002 [S2] — The module that would give the shipped interactive session a self-driving clock is precisely the one nobody wires; the CLI clock only moves on the manual `advance` verb

Where: root cause spans files: this file's System/VideoWall wall modes (never constructed, F-001), `apps/cli/src/pipeline_host.rs:1605-1611`, `apps/cli/src/run.rs:226` and `run.rs:251`.

Code (apps/cli/src/pipeline_host.rs:1605-1611, verbatim):
```rust
    let clock = Arc::new(MediaClock::audio_master(
        sink.clock_source(),
        Timestamp::new(anchor_ns, media_clock::NANO_BASE),
        Rate::ONE,
    )?);
```

Code (apps/cli/src/run.rs:225-228, verbatim):
```rust
        let mut line = String::new();
        match input.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
```

Code (apps/cli/src/run.rs:251-252, verbatim):
```rust
            CliCommand::Advance { ticks } => {
                host.advance(ticks);
```

What is wrong: for any audio-bearing media the session clock's truth source is the **NullDevice's** sample counter, which only advances when the host renders quanta inside a pump; the REPL blocks on `read_line` between commands and nothing pumps (no ticker thread). W00 proved the consequence (audit/runs/baseline/advance-test.txt:7: `ok: time 0.000000s (0.0%), 1 buffered range(s)` until `advance` was pumped). The desktop-tauri shell solved this with a real ticker thread (`apps/desktop-tauri/src/app.rs:42-48`: "The engine is synchronous single-writer: it advances ONLY when a host pumps it … The shell's ticker thread (`spawn_event_pump`) therefore IS the session thread"); the CLI REPL has no equivalent, and the wall-driven backends of this file (System/VideoWall/Free modes interpolating on an `InstantSource`) are never constructed anywhere.

Why it matters: this is the direct mechanism behind the owner-visible symptom "the interactive clock never advances". The fix (a wall-paced pump, or a `with_backends` clock plus a ticker) exists in the tree but is unreachable/unwired in the CLI host.

What correct looks like: the CLI REPL spawns the same wall-bounded pacer (`CliHost::pump_wallclock`, which already exists at apps/cli/src/host.rs:523) on a `SESSION_TICK`-style thread while reading stdin, or at minimum documents `advance` as the only time driver (it does not — help.rs presents `time` as "current position" with no caveat).

Confidence: HIGH — Verification: read + W00 baseline run output cited (advance-test.txt).

### F-003 [S3] — `switch_locked`'s "defensive" branch silently returns `(from, from)` and records nothing

Where: crates/media-clock/src/backends.rs:979-984.

Code (verbatim):
```rust
    let Some(next_source) = core.backend.wired.source_for(to) else {
        // Unreachable by construction: every caller validated the
        // target against the same wired set (plan_fallback /
        // switch_master). Defensive: refuse rather than invent.
        return (from, from);
    };
```

What is wrong: if the invariant ever broke (e.g. a future caller passes an unvalidated kind), the caller of `on_master_lost` would interpret `(from, from)` as a Demoted/switched event is *not* pushed — but `on_master_lost` would still report `MasterLoss::Demoted { from, to }` with `to == from` derived from… actually `to` is taken from `core.source.master()` after the swap, so it degrades to `Demoted { from: X, to: X }` with no event in the ring and a lost bit set. A typed refusal variant (like `SwitchError::NotWired`) would be honest; a silent `(from, from)` is a swallowed failure path.

Why it matters: §3.2 "swallowed errors / catch-all arms". Blast radius small (comment is correct about current callers), so S3 not S2.

What correct looks like: make `switch_locked` return `Result<(from,to), SwitchError>` and have `on_master_lost` map it to `NoFallback`.

Confidence: HIGH — Verification: read (callers at 746-750, 787 analyzed).

### F-004 [S3] — `MasterChanged` events can be silently dropped for real consumers (8-slot ring) with no engine-level drain anywhere

Where: crates/media-clock/src/backends.rs:102-106 and 532-540.

Code (backends.rs:104-106, verbatim):
```rust
/// Master changes are policy events — O(1) per session in normal
/// playback; a consumer that lets 8 pile up un-drained gets the
/// lost-count receipt, never unbounded growth.
const MASTER_EVENT_CAPACITY: usize = 8;
```

What is wrong: the design is fine for a wired consumer; but since no consumer exists (F-001), the "lost-count receipt" is a receipt nobody can read in a shipped binary. Combined with F-001 this is duplication of the same root problem, logged separately because a future integration must drain `take_master_change` from a real thread or events vanish with only a counter as evidence.

Why it matters: naming a bounded ring "the receipt, never silent loss" is only true if someone drains it; the shipped tree never does.

What correct looks like: n/a until integration; note for the playback-engine card.

Confidence: HIGH — Verification: read.

### F-005 [S4] — Header claims zero-allocation "measured … not argued", but the measurement lives in a test file outside this module and covers only listed paths

Where: crates/media-clock/src/backends.rs:53-58.

Code (verbatim):
```rust
//! Steady-state tick paths (`now`/`try_now`/`snapshot`/
//! `observe_video`/`take_master_change`, and the switches
//! themselves) allocate nothing — measured by the counting global
//! allocator in `tests/zero_alloc_backends.rs`, not argued.
```

What is wrong: nothing material — `crates/media-clock/tests/zero_alloc_backends.rs` exists (verified by directory listing) — but I could not run it (ADAPTATION: no cargo), so the "measured" claim is REAL-UNPROVEN from this audit's seat, and the doc phrase "not argued" is doing evidence work a reviewer cannot check from this file alone.

Confidence: MEDIUM — Verification: UNVERIFIED (no cargo; file existence verified by LS).

## 4. Per-function table (production items; tests in §5)

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MonotonicKind` (enum) | 115-128 | REAL-UNPROVEN | — | label-only vocabulary; reading math shared with 3.B1 |
| `MasterStream` (enum) + `as_str` | 134-152 | REAL-UNPROVEN | — | loss-report vocabulary; no caller outside tests |
| `MasterKind` (enum) + `label` + `loss_bit` | 159-190 | REAL-UNPROVEN | — | pub(crate) planner vocabulary; pure |
| `From<MasterStream> for MasterKind` | 192-200 | REAL-UNPROVEN | — | trivial mapping |
| `LOST_AUDIO/LOST_EXTERNAL/LOST_VIDEO` consts | 203-207 | REAL-UNPROVEN | — | bitset encoding |
| `DEMOTION_ORDER` | 212-217 | REAL-UNPROVEN | — | audio→external→video→free; tested exhaustively |
| `MasterChanged` (struct) | 223-238 | REAL-UNPROVEN | — | Copy payload; never crosses a wire in-tree |
| `MasterChangeReason` + `as_str` | 242-258 | REAL-UNPROVEN | — | two variants |
| `MasterLoss` (enum) | 263-281 | REAL-UNPROVEN | — | typed outcome; `NoFallback` = frozen truth |
| `InitialMaster` (enum) | 286-302 | REAL-UNPROVEN | — | `#[default] Auto` |
| `MasterTarget` + `as_str` | 310-325 | REAL-UNPROVEN | — | switch vocabulary |
| `MasterSwitch` (enum) | 329-338 | REAL-UNPROVEN | — | Switched/Already |
| `SwitchError` (enum) | 343-348 | REAL-UNPROVEN | — | NotWired/Lost |
| `SeekBackError` + `Display` | 352-384 | REAL-UNPROVEN | — | good error text |
| `VideoSteer` (enum) | 389-412 | REAL-UNPROVEN | — | Snapped/Jumped/NotMaster/Paused/Ended |
| `WiredBackends::source_for` | 426-446 | REAL-UNPROVEN | — | Arc clones, no alloc |
| `WiredBackends::wired` | 449-456 | REAL-UNPROVEN | — | pure |
| `BackendCore::no_fallback` | 479-492 | REAL-UNPROVEN | — | the 3.B1 default — this is what shipped clocks actually get |
| `BackendCore::multi` | 495-503 | REAL-UNPROVEN | — | only reachable via `with_backends` (F-001) |
| `EventRing::new/push/pop` | 522-551 | REAL-UNPROVEN | — | correct FIFO math; overflow counted (F-004) |
| `plan_fallback` | 559-572 | REAL-UNPROVEN | — | pure; 640-cell exhaustive test |
| `ClockBackends::new` | 623-632 | REAL-UNPROVEN | — | wall required |
| `ClockBackends::audio` | 636-639 | REAL-UNPROVEN | — | builder |
| `ClockBackends::external` | 643-646 | REAL-UNPROVEN | — | builder; nothing in-tree supplies an external source |
| `ClockBackends::video` / `video_snap_ns` / `initial` | 653-668 | REAL-UNPROVEN | — | builder |
| `MediaClock::with_backends` | 689-719 | UNREACHABLE | S0 | zero non-test callers (F-001) |
| `MediaClock::on_master_lost` | 736-756 | UNREACHABLE | S0 | F-001; logic itself sound |
| `MediaClock::switch_master` | 769-789 | UNREACHABLE | S0 | F-001 |
| `MediaClock::observe_video` | 804-843 | UNREACHABLE | S0 | i128 error math correct; anchor-then-classify order sound |
| `MediaClock::mark_ended` | 857-867 | UNREACHABLE | S0 | freeze subsumes pause; idempotent (tested) |
| `MediaClock::seek_back` | 881-912 | UNREACHABLE | S0 | re-arms paused-at-target; overflow typed |
| `MediaClock::is_ended` | 915-917 | UNREACHABLE | — | trivial |
| `MediaClock::take_master_change` / `master_changes_lost` | 922-931 | UNREACHABLE | S3 | F-004 |
| `Source::master_kind` | 936-956 | REAL-UNPROVEN | — | pure mapping |
| `switch_locked` | 967-1012 | UNREACHABLE | S3 | F-003 defensive branch returns `(from, from)` |

## 5. Test honesty (36 `#[test]` in-file)

| Test (representative) | Would it fail if the code were broken? | Reason |
|---|---|---|
| `fallback_plan_is_deterministic_over_the_full_matrix` (1166-1236) | YES | exhaustive 640-cell oracle over the pure planner |
| `master_loss_chain_audio_video_free_runs_with_events` (1238-1309) | YES | asserts exact carried positions, epoch bumps, full event payloads |
| `single_master_clock_reports_no_fallback_and_holds` (1400-1420) | YES | pins `NoFallback` frozen-truth semantics for 3.B1 clocks (the only kind shipped) |
| `loss_while_ended_switches_but_stays_frozen` (1422-1450) | YES | latch-vs-demotion interaction |
| `video_master_snaps_within_the_bound_and_interpolates_between` (1580-1618) | YES | exact ns arithmetic incl. monotone clamp hold |
| `switches_across_i64_position_edges_carry_saturated` (1957-1996) | YES | i64::MIN/MAX edges, no-panic + saturation |
| `video_steer_errors_at_the_i64_bounds_are_i128_safe` (1998-2026) | YES | ~2^64 ns error classified without overflow |
| `switch_storm_without_drain_surfaces_the_lost_count` (2062-2085) | YES | ring overflow receipt |
| `vocabulary_strings_are_stable` (2088-2105) | WEAK | string equality tautologies — but that is its purpose (wire-name pinning) |

Caveat that dominates all of the above: every test exercises an API no production caller uses (F-001). The suite is honest about the library and silent about the integration.

## 6. Doc claims touching this file

| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "3.B2 clock backends … **COMPLETE (merged)**" | docs/gates/phase3-exit.md:106 | FALSE (as a capability claim) / REAL-UNPROVEN (as a library) | merged code exists at this path, but zero shipped callers (F-001 grep) |
| "clocks, queues, generations, seeking, source replacement, sync, recovery (7 capability families) **MET (merged)**" citing "wave-5 merges dce1b64 (3.B2)" | docs/gates/phase3-exit.md:31 | FALSE | the 3.B2 contribution to "MET" is code no entry point can reach; W00 advance-test shows the shipped clock frozen without manual ticks |
| "Clock backends: video-master and external-master modes, master-loss fallback policy, Ended-state seek-back. AC: master-switch tests; discontinuity unit tests" | docs/13-agent-wave-program.md:377 (row 3.B2) | REAL-UNPROVEN | the named ACs do exist in-file (§5); the row does not claim engine wiring — the gate does |
| "between two invokes NOTHING pumps" (engine advances only on host pumps) | apps/desktop-tauri/src/app.rs:40-41 | REAL | matches run.rs REPL behavior and W00 clock-frozen baseline |

## 7. Dead / duplicated / unreachable code

The whole public card-3.B2 surface (F-001): `with_backends`, `on_master_lost`, `switch_master`, `observe_video`, `mark_ended`, `seek_back`, `is_ended`, `take_master_change`, `master_changes_lost`, `ClockBackends` (all builders), `InitialMaster`, `MasterTarget`, `MasterSwitch`, `SwitchError`, `SeekBackError`, `VideoSteer`, `MasterChanged`, `MasterChangeReason`, `MasterLoss`, `MasterStream` — exported (lib.rs:116-119) and re-exported, but unconstructed anywhere outside this crate's tests. `MonotonicKind::External` additionally has no producer in-tree (no external clock domain source exists anywhere in the repo).

## 8. Dependencies & flags

No feature flags gate this file; `crates/media-clock` is a plain workspace member (deps: `media-types`, `testkit` dev-dep). Nothing in `product.toml`/workflows disables it — the deadness is architectural (no caller), not flag-gated.

## 9. Open questions for the next agent

- Who *should* have consumed this: playback-engine's session (seek.rs / session.rs in my wave list — I will report the actual seek/EOF handling there) and the video-engine scheduler (does it steer or poll?). Files: crates/playback-engine/src/seek.rs, crates/video-engine/src/scheduler.rs.
- The desktop-tauri pacer (`spawn_event_pump`, SESSION_TICK) does drive wall-bounded pumps — why the CLI REPL chose not to spawn the same pacer is a product decision worth flagging to the apps/cli wave (W13).
- `tests/zero_alloc_backends.rs` claimed as the allocation proof — belongs to the media-clock test audit (another agent); I did not audit it (not in my assignment).
