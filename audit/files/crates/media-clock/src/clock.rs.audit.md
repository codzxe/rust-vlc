# AUDIT: crates/media-clock/src/clock.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 1178 / 30 production fns + 18 test fns + 5 impl/struct blocks |
| File verdict | REAL for audio_master/system_monotonic path; UNREACHABLE for the card-3.B2 backend half (details below) |
| Reachable from shipped entry point? | YES for `audio_master`/`system_monotonic` (chain in §2); NO for everything gated on `with_backends` (zero non-test callers) |
| Findings | S0:1 S1:0 S2:1 S3:1 S4:2 |
| Commands run | rg over repo for `audio_master\|system_monotonic\|with_backends\|mark_ended\|seek_back\|observe_video\|switch_master\|pump_wallclock` (static; no cargo per adaptation). All quotes verified with `sed -n`. Runtime behavior cited from W00 baseline (release build PASS; interactive clock stuck unless `advance N`). |

## 1. One-paragraph verdict

This is the "one playback clock" of the repo, and the arithmetic core of it is genuinely engineered: anchor-based extrapolation over an injected sample counter or wall source, checked i128 math, monotonic clamp, regression/scale-change detection, saturation instead of wrap, poison-recovering lock, and 18 in-file tests that actually drive the math with atomic fakes and would fail if the anchor arithmetic broke. But it is a **passive** clock — `now()` only reads a source — and the audit's central question is answered by the wiring, not this file: with no audio device, the CLI builds `system_monotonic(InstantSource)` (real wall time) yet the CLI binary has **no wall-clock driver at all**; the only real-time pump (`CliHost::pump_wallclock`, host.rs:523) is called exclusively by the desktop shell, never by the CLI, so the shipped CLI's "Playing" state never advances time unless the user types `advance N`. Half of the file's complexity (the `BackendCore`/Ended-latch/`awaiting_anchor` machinery, card 3.B2) is compiled into every shipped binary but constructed by no shipped entry point.

## 2. What actually works (verified)

- Anchor arithmetic, rate scaling, pause freeze, regression hold, saturation: verified by reading the math (`Core::extrapolate`, `now_locked`, `checked_div_round`) and by the in-file tests' construction (atomic fakes driven stepwise; e.g. `anchor_arithmetic_scales_by_rate` 48000·4 samples → 5e9 ns at rate 5/4 is real arithmetic, not a tautology). Test execution UNVERIFIED (no cargo); W00 baseline: crate test suite part of 1751 passing.
- Reachability of the two shipped constructors, verified by call chain:
  - CLI audio path: `apps/cli/src/run.rs` repl/machine dispatch → `CliHost::open_session` (apps/cli/src/host.rs:890) → `pipeline_host::open_audio_session` (apps/cli/src/pipeline_host.rs:1587) → `MediaClock::audio_master(sink.clock_source(), …)` (pipeline_host.rs:1605) — the sink is the **null** backend's sink (W00: no audio device exists).
  - CLI no-audio path: `open_session` → `pipeline_host::wall_clock` (pipeline_host.rs:1629) → `MediaClock::system_monotonic(Arc::new(InstantSource::new()), …)` (pipeline_host.rs:1630-1634).
  - Web: `apps/web/src/host.rs:1737` builds `audio_master`.
  - Desktop: adopts the CLI host (apps/desktop/src/shell.rs), same constructors.
- `with_backends`, `mark_ended`, `seek_back`, `observe_video`, `switch_master` have **zero non-test callers** outside `crates/media-clock/src/backends.rs` (rg over the whole repo, exclusion `!crates/media-clock/**` returns only backends.rs itself and playback-engine's unrelated `observe_video_end` private fn). UNREACHABLE from every shipped entry point.

## 3. Findings

### F-001 [S0] The shipped CLI has no clock driver: "Playing" cannot advance time
Where: cross-file — apps/cli/src/run.rs:220-228 (the REPL loop), apps/cli/src/host.rs:497-501 (`advance`), apps/cli/src/host.rs:523-526 (`pump_wallclock`), apps/desktop/src/shell.rs:279-282 (the only caller of `pump_wallclock`)
Code (run.rs:225-228):
```rust
        let mut line = String::new();
        match input.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
```
Code (host.rs:497-501):
```rust
    pub fn advance(&mut self, ticks: u32) {
        for _ in 0..ticks {
            self.pump();
        }
    }
```
What is wrong: The clock in this file is passive; the only thing that moves it is the host rendering null-device quanta (`advance_time`) or wall time. The CLI's REPL is a blocking `read_line` loop with **no thread, no timer, no sleep** (rg `thread::sleep` over apps/cli/src: zero production hits). The wall-bounded real-time pump `pump_wallclock` exists in the CLI host but is called only by the desktop shell (`ShellRuntime::pump_session`, shell.rs:280, driven by `spawn_event_pump`'s ticker thread, apps/desktop-tauri/src/app.rs:190-204). For the audio-master clock the truth source is the null sink's counter, which advances only inside `pump()`; for the monotonic clock `position_us` (host.rs:346-358) latches to last-presented frames rather than `clock.now()`. Net: in the shipped CLI binary, media time moves **only** when the user types `advance N`.
Why it matters: This is the mechanism behind the W00 baseline fact — session `time` stays 0.000000s while `Playing` unless `advance N` ticks (mjpeg+pcm AVI; `advance 200` → Playing→Ended, 100%). A user who opens a file and plays it in the shipped CLI sees a frozen position indefinitely. The state machine says Playing; the clock this file implements is fine; the driver is absent. User-visible core broken → S0.
What correct looks like: The REPL (or the settle path) needs a real-time driver — either call `pump_wallclock` from a spawned ticker under the host's single-writer lock (as desktop-tauri does), or run `machine_mode`/REPL commands against a session thread.
Confidence: HIGH — Verification: read (call graph) + W00 baseline run.

### F-002 [S2] Card 3.B2 backend machinery in this file is dead weight in every shipped binary
Where: crates/media-clock/src/clock.rs:336-348 (Core.backend / awaiting_anchor) and every `backend.ended` arm: 371, 544, 560, 801, 816
Code (clock.rs:336-348):
```rust
    /// The truth source (card 3.B2: inside the lock — see [`Source`]).
    pub(crate) source: Source,
    /// Card 3.B2 backend state: the wired fallback sources, the loss
    /// bitset, the video snap bound, the Ended latch and the bounded
    /// master-change event ring.
    pub(crate) backend: BackendCore,
```
Code (clock.rs:371-373):
```rust
        if self.paused || self.backend.ended {
            return Extrapolation::Paused(self.frozen_ns);
        }
```
What is wrong: The only shipped constructors (`audio_master` 479-490, `system_monotonic` 499-513) both pass `BackendCore::no_fallback()`, and `with_backends` — the sole constructor that arms any of this — has no caller outside media-clock's own tests. Every `backend.ended` branch (pause guard 544, resume guard 560, extrapolate freeze 371, reanchor freeze 801, set_rate freeze 816) and the `awaiting_anchor` adoption path (706-720) are unreachable in the shipped CLI, desktop, and web builds. The Cargo.toml `description` advertises "video/external master backends with deterministic master-loss fallback and an Ended seek-back re-arm" as the crate's behavior; in shipped products none of it can occur.
Why it matters: ~40% of this file's branches are untestable-by-production; any doc/matrix row claiming master-loss fallback or Ended-latch behavior in the product is describing code no entry point constructs. Audit surface inflated, and the doc claims are false at product level.
What correct looks like: Either an entry point wires `with_backends` (playback-engine policy) or the dead surface is cut behind a feature flag.
Confidence: HIGH — Verification: read (rg `with_backends` across repo: only media-clock/src/backends.rs and its tests).

### F-003 [S3] `resume()` on a still-unusable source can later skip over the frozen position
Where: crates/media-clock/src/clock.rs:558-568
Code:
```rust
    pub fn resume(&self) {
        let mut core = self.lock_core();
        if !core.paused || core.backend.ended {
            return;
        }
        let reading = read_source(&core.source);
        core.anchor_media_ns = core.frozen_ns;
        adopt_anchor(&mut core, &reading);
        core.paused = false;
```
What is wrong: If the reading at resume is `Unusable`, `adopt_anchor` (656-668) silently keeps the **pre-pause** source anchor. When the source becomes readable again, its counter legitimately includes the paused span (a source that does not freeze on pause — `MonotonicSource` never freezes by contract, source.rs:28, and a flaky device counter may not either), so `delta = ns − anchor_source_ns` spans the pause and `now()` jumps from `frozen_ns` past the paused interval in one step.
Why it matters: The doc contract (lines 22-25) promises "Resume continues from the same media time with no discontinuity — even when the underlying source kept running while paused". That promise is only kept when the resume-time reading is usable; the unusable-at-resume path violates it for any monotonic-master clock (which per F-001's wiring is exactly the no-audio CLI path). In practice the monotonic source is always usable, so the trigger is a broken/zero-rate audio source at the instant of resume — narrow but real, and the file's own doc overpromises.
What correct looks like: On an unusable reading at resume, set `awaiting_anchor = true` (the mechanism already exists, 706-720) so the first usable reading becomes the new baseline instead of delta-ing across the pause.
Confidence: MEDIUM — Verification: read (mutation reasoning; no cargo).

### F-004 [S4] Overflow saturation permanently pins `last_ns` at i64::MAX
Where: crates/media-clock/src/clock.rs:390-399 and 738-739
Code (390-391):
```rust
        let Some(scaled) = delta.checked_mul(i128::from(self.rate.num)) else {
            return Extrapolation::Running(i128::MAX);
```
Code (738-739):
```rust
            let pos = saturate_position(raw).max(core.last_ns);
            core.last_ns = pos;
```
What is wrong: One overflow result pins `last_ns` to `i64::MAX`; afterwards `now()` returns `i64::MAX` even if the rate is later lowered below the overflow threshold — `set_rate` re-anchors at the clamped position (i64::MAX), so the clock is wedged at the ceiling for the session's life.
Why it matters: Only reachable with extreme (near-i64) positions or huge rates; the typed `try_now` reports Overflow honestly. Quality defect with limited blast radius.
What correct looks like: `set_rate`/`reanchor` could recompute a representable anchor or document the wedge as terminal.
Confidence: HIGH — Verification: read.

### F-005 [S4] Dead disjunct in a regression test's assertion
Where: crates/media-clock/src/clock.rs:972
Code:
```rust
                assert!(found.ticks < 0 || found.ticks < held.ticks);
```
What is wrong: `held.ticks` is 2_000_000_000 (positive), so `found.ticks < 0` implies `found.ticks < held.ticks`; the first disjunct is dead code and the assertion is weaker than it reads (it accepts any `found` below `held`, including nonsense-positive values, without pinning the sign the test name implies).
Why it matters: Test-paperwork hygiene; a reviewer believes the test checks negativity.
What correct looks like: `assert!(found.ticks < 0)` (the naive extrapolation of a reset counter is negative) or an exact expected value.
Confidence: HIGH — Verification: read.

## 4. Per-function table (production code)

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Display for ClockMaster::fmt` | 111-121 | REAL | — | Pure string mapping, trivially correct. |
| `Source::master` | 262-283 | REAL | — | Exhaustive match; unreachable arm impossible (kind is closed). |
| `Core::extrapolate` | 368-400 | REAL | S4 | Checked math; regression + rate-change detection; i128::MAX sentinel (F-004). |
| `naive_position` | 404-409 | REAL | — | Saturating diagnostic math; never used as a position. |
| `checked_div_round` | 414-427 | REAL | — | Ties away from zero; verified by hand for ±cases; denom ≤ 0 → None. |
| `position_nanos` | 435-441 | REAL | — | Checked both ways; error typed. |
| `saturate_position` | 444-446 | REAL | — | Clamp-then-cast, safe. |
| `clock_state` | 450-458 | REAL | — | Ended > Paused > Running precedence; matches mark_ended clearing `paused` (backends.rs:864). |
| `MediaClock::audio_master` | 479-490 | REAL | — | Reachable: pipeline_host.rs:1605, web host.rs:1737. |
| `MediaClock::system_monotonic` | 499-513 | REAL | — | Reachable: pipeline_host.rs:1630 (CLI no-audio path). |
| `MediaClock::build` | 515-534 | REAL | — | Validates rate + position; unusable first reading → zero anchors (hold). |
| `MediaClock::pause` | 542-550 | REAL | — | Idempotent; freezes at current extrapolation; Ended no-op. |
| `MediaClock::resume` | 558-568 | REAL-UNPROVEN | S3 | F-003: unusable-reading resume can later skip. |
| `MediaClock::is_paused` | 573-575 | REAL | — | Doc upheld: mark_ended clears `paused` (backends.rs:864). |
| `MediaClock::set_generation` | 581-583 | REAL | — | Passive carrier. |
| `MediaClock::anchor` | 586-602 | REAL | — | Introspection; u64 cast after `.max(0)` clamp — safe. |
| `MediaClock::lock_core` | 607-612 | REAL | — | Poison recovery via `into_inner`; no panicking ops inside lock (verified: no unwrap/expect/indexing in critical sections). |
| `snapshot_locked` | 618-629 | REAL | — | Consistent read under lock. |
| `anchor_core` | 633-650 | REAL | — | Zero-init anchors; deterministic. |
| `adopt_anchor` | 656-668 | REAL | — | Silent keep on unusable reading — see F-003 interplay. |
| `read_source` | 674-700 | REAL | — | Zero rate → Unusable; checked sample→ns; monotonic arm hard-codes `rate_hz: 0` which matches `anchor_rate_hz: 0` so no false discontinuity. |
| `adopt_if_awaiting` | 706-720 | UNREACHABLE | S2 | Only fires when `awaiting_anchor` set — set only in backends.rs (seek_back/switch), never in shipped paths. |
| `now_locked` | 725-744 | REAL | — | Monotone clamp; Discontinuity holds without advancing baseline — sound. |
| `try_now_locked` | 748-769 | REAL | — | Typed errors; baseline advances on sound readings. |
| `PlaybackClock::now` | 772-776 | REAL | — | Hot path, one lock, one source read. |
| `PlaybackClock::try_now` | 778-782 | REAL | — | Same. |
| `PlaybackClock::master` | 784-786 | REAL | — | Label read. |
| `PlaybackClock::reanchor` | 788-805 | REAL | — | Resets clamp (allows backward seek); freeze moves; Ended survives (unreachable arm in practice). |
| `PlaybackClock::set_rate` | 807-823 | REAL | — | Re-anchors at clamped position, no jump; zero-component rate rejected. |
| `PlaybackClock::snapshot` | 825-828 | REAL | — | Delegates. |

## 5. Test honesty (in-file tests, lines 831-1178)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| anchor_arithmetic_tracks_the_counter (891-898) | YES | Real counter math: 48000 samples @48kHz must equal exactly 1e9 ns. |
| anchor_arithmetic_scales_by_rate (901-912) | YES | 5/4 rate over 4 s → 5e9 ns; catches rate-scaling mutations. |
| monotonic_master_tracks_injected_wall_time (915-922) | YES | Injects 2.5e9 ns wall, asserts position. |
| reanchor_jumps_are_expected_and_epoch_bounded (927-940) | YES | Backward seek + epoch count. |
| reanchor_overflow_leaves_clock_unchanged (943-950) | YES | Error path + unchanged position. |
| counter_regression_holds_position_never_backwards (955-982) | YES (weak final assert, F-005) | Core hold/re-anchor behavior genuinely exercised. |
| sample_rate_change_is_a_discontinuity (985-1011) | YES | Rate flip mid-stream detected. |
| broken_source_is_unusable_not_panic (1014-1030) | YES | Zero-rate source held, typed error. |
| extreme_positions_saturate_monotonically (1035-1051) | YES | i64::MAX saturation + typed overflow. |
| set_rate_never_jumps_and_rescales (1056-1068) | YES | No-jump + 2× rescale. |
| set_rate_rejects_zero_components (1071-1076) | YES | Rejects `Rate { num: 0 }` built via pub fields. |
| pause_freezes_resume_continues_without_jump (1081-1096) | YES | Source runs during pause; clock must not. |
| pause_and_resume_are_idempotent (1099-1107) | WEAK | State-flag only; would not catch drift math bugs. |
| reanchor_while_paused_moves_the_freeze (1110-1120) | YES | Freeze relocation. |
| rate_survives_pause_resume (1123-1138) | YES | Rate carried across resume. |
| snapshot_is_consistent_and_carries_context (1143-1153) | YES | Field-carry check (mostly structural). |
| anchor_introspection_reflects_the_master (1156-1169) | YES | at_sample vs at_wall_ns polarity. |
| send_sync_object_safe (1172-1177) | NO | Compile-time trait assertion only; tautology at runtime. |

Overall: the strongest test file I have audited so far in this crate cluster — real arithmetic, no sleeps, atomic fakes. Two weak entries noted.

## 6. Doc claims touching this file

| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "audio-master timing over an injectable sample counter, video/external master backends with deterministic master-loss fallback and an Ended seek-back re-arm" | crates/media-clock/Cargo.toml `description` | SHALLOW | Library code exists and its own tests pass (W00 suite), but no shipped entry point constructs the backend surface (F-002); as a product behavior it is UNREACHABLE. |
| "Playback time derives from the sample counter — never from a wall clock" (module doc lines 8-10) | clock.rs:8-10 | REAL | extrapolate reads only the source; verified. |
| "critical sections are one source read plus pure checked arithmetic — never held across I/O" (lines 40-42) | clock.rs:40-42 | REAL-UNPROVEN | Verified for this file (no I/O in critical sections); the non-blocking contract of injected sources is an assumption verified only for the null backend (audio-output counter — W08's file). |
| "resume continues … no discontinuity — even when the underlying source kept running while paused" (lines 22-25) | clock.rs:22-25 | SHALLOW | Holds for usable readings; violated in the F-003 edge. |

## 7. Dead / duplicated / unreachable code

- `Core.backend` + all `backend.ended` arms + `awaiting_anchor` adoption (F-002): unreachable in shipped builds.
- `ClockMaster::VideoOutput/External/FreeRunning` variants: constructible only via `with_backends` (backends.rs) — unreachable in shipped builds. `ClockMaster::External` is additionally unreachable even at library level except through its constructor path.

## 8. Dependencies & flags

- No feature flags; no `unsafe`; std + media-types only (Cargo.toml: "Std-only by C5: zero external dependencies, no unsafe" — verified: `use std::sync` only).
- dev-dependency `testkit` is dev-only (Cargo.toml `[dev-dependencies]`) — not in shipped binaries.

## 9. Open questions for the next agent

- W13 (apps/cli): does any CLI verb other than `advance` move media time? (rg suggests no; `settle` pumps but does not render — confirm.)
- W08 (audio-output): does the null sink's `clock_source()` counter advance only on `device.render`? (assumed from host.rs:601-619; verify in null.rs.)
- Claim-ledger agents (W18/W19): which matrix rows cite card 3.B2 / master-loss fallback? None of them can be REAL for a shipped product.
