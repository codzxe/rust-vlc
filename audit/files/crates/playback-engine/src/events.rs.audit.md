# AUDIT: crates/playback-engine/src/events.rs

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-22 |
| Lines / functions | 2568 / ~66 production items + 20 unit tests (~1025 test lines = 40% of file) |
| File verdict | **REAL-UNPROVEN** (policy core, wired into session.rs) — the guard/catalog/coalescer/queue half is **UNREACHABLE** (test-only) |
| Reachable from shipped entry point? | YES for TimeTickPolicy/ProgressWindow/DamageCoalescer/StalenessStamp/DrainStaleness/EventPolicy (chain below). NO for OrderingGuard, debug_assert_wire_ordered, LatestCoalescer, BoundedEventQueue, wire_event_id, emission_status, is_engine_emitted, CATALOG_EMISSIONS. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 4 S4: 2 |
| Commands run | rg searches (callers of every public item); sed -n quote verification; W00 baseline: cargo test --workspace 1751/2 fail (both failures first-vertical), MP4 smoke (`state-changed Idle → Opening`, `tracks 2 ready`, `TimingDurationKnown` fired from the real binary) |

## 1. One-paragraph verdict

This is the event-emission *policy* library (throttling, coalescing, staleness, ordering), not dispatch — the emission points live in `session.rs`, and the wiring is real: `session.rs:74` imports `DamageCoalescer, DrainStaleness, EventPolicy, ProgressWindow, TimeTickPolicy`; `session.rs:2651` calls `tick.is_due`, `session.rs:2489` calls `buffering_window.permits`, `session.rs:2459/2499/1740` record/flush the damage coalescer, `session.rs:748` calls `is_stale_at_drain` on the drain path, and the W00 CLI smoke proves state-changed/tracks/duration events actually fire from the shipped binary. Of 2568 lines, roughly half is production policy and half is (a) a 26-row catalog completeness table plus lookups used **only by tests**, and (b) an ordering guard the module doc claims "hosts … run" — false, no shipped host references it. The catalog's "15 of 26 rows engine-emitted today" overstates by 3: the hotplug trio (`audio.device-lost/-changed`, `video.backend-changed`) is emitted only via `Engine::observe_audio_device_lost`/`observe_audio_reopen`/`poll_hotplug`, which **no shipped app calls** (grep of apps/: zero hits; only `tests/hotplug.rs` calls them), and the only audio backend is null (W00), which never loses a device.

## 2. What actually works (verified)

- Policy core wired and reachable: `lumen` CLI → `apps/cli/src/host.rs:976-980` (`config.throttle = playback_engine::EventPolicy { tick: playback_engine::TimeTickPolicy::default_policy(), throttle: source }`) → `session.rs:1433-1434` (SessionRuntime retains `tick`/`throttle`) → emission sites `session.rs:2651`, `2441`, `2489`, `2459`, `748` (verified by rg + sed; no cargo available to this agent).
- W00 baseline smoke (audit/runs/baseline/mp4-smoke.txt, BASELINE-SUMMARY.md): the real `lumen` binary emitted `state-changed Idle → Opening`, `tracks 2 ready`, `TimingDurationKnown { duration: Timestamp(3000000) }`, `state-changed Buffering → Playing` — the engine event stream is not vaporware.
- W00 baseline: `cargo test --workspace` 1751 passed / 2 failed (both in `first-vertical`, transcript+png golden drift) / 25 ignored — this file's 20 unit tests pass in that run (UNVERIFIED individually; log is trimmed before the playback_engine section).
- The completeness map is honest about 11 of 26 rows (`WiredLater { owner }` names the owning phase; empty-owner case is asserted away at lines 1977-1983).

## 3. Findings

### F-001 [S2] — 3 of the "15 emitted" catalog rows cannot fire from any shipped entry point
Where: crates/playback-engine/src/events.rs:706-727 (and 108-115 module-doc claim)
Code:
```rust
    // audio (§3.2) — emitted since card 7.C1: the engine's hotplug
    // hooks drive the recovery guard's suspend/reopen policy onto
    // the wire (tests/hotplug.rs proves the payloads live).
    CatalogEmission {
        id: EventId::AudioDeviceLost,
        status: EmissionStatus::Emitted,
    },
```
and:
```rust
//! 15 of 26 rows are engine-emitted today — the ids above
```
What is wrong: `AudioDeviceLost`, `AudioDeviceChanged`, `VideoBackendChanged` are marked `Emitted`, but their only triggers are `Engine::observe_audio_device_lost` / `observe_audio_reopen` / `poll_hotplug` (hotplug.rs:182/225/265) — rg over `apps/` returns **zero** callers; the only caller in the tree is `crates/playback-engine/tests/hotplug.rs` (lines 256, 292, 384, 394, 476, 508, 514, 724, 731). The shipped `lumen` CLI, web host, and desktop hosts never call them, and the only audio device is the null backend (W00 baseline: "null backend only"), which cannot report a lost device.
Why it matters: `CATALOG_EMISSIONS` is the machine-checked honesty instrument this module advertises ("the map below is the machine-checked count"). It conflates "an engine API can emit this" with "the engine emits this"; the honest count from a shipped binary is 12/26, not 15/26. Downstream claim ledgers (wave 18/19) will inherit the overstatement.
What correct looks like: either a fourth status (`EmittedApiOnly`) or `WiredLater { owner: "host hotplug wiring (no shipped host calls the hooks; null backend cannot lose a device)" }`.
Confidence: HIGH — Verification: read (rg callers of `observe_audio_device_lost|poll_hotplug` across the whole tree; only tests/hotplug.rs hits).

### F-002 [S3] — `LatestCoalescer` (≈105 lines + tests) is dead production code, admitted in its own doc
Where: crates/playback-engine/src/events.rs:1355-1460, doc admission at 1348-1351
Code:
```rust
/// the engine's in-episode `session.buffering` path
/// ([`crate::session`], the `EmissionState` fields) can adopt it as
/// a one-structure swap — recorded as a handoff, not wired here
/// (scope discipline: the emission site is sibling-owned).
```
What is wrong: rg for `LatestCoalescer` outside this file returns zero hits (the earlier full-tree grep found matches only in events.rs itself and a worklog mention). session.rs implements the same policy inline instead (`session.rs:2483-2490`: `if fill > self.emissions.buffering_last && self.emissions.buffering_window.permits(now)`), with `buffering_last` playing `last_emitted` and no pending-retention at all.
Why it matters: two parallel implementations of the ≤5 Hz monotonic rule that can drift (the inline version has no latest-wins pending retention — a denied sample is simply lost, the exact "lossier reading" this type says it exists to replace). The richer version is the one nobody uses.
What correct looks like: either wire `LatestCoalescer` into `EmissionState` or delete it.
Confidence: HIGH — Verification: read (rg `LatestCoalescer` tree-wide; only self-references).

### F-003 [S3] — `OrderingGuard`/`debug_assert_wire_ordered` have no production caller; module doc claims hosts run the guard
Where: crates/playback-engine/src/events.rs:936-944 (claim), 949-1321 (implementation)
Code:
```rust
/// This is the policy layer's *guard* half: the emission sites
/// ( [`crate::session`] ) are the production carriers; hosts and the
/// contract suites run the guard over what actually crossed the wire
```
What is wrong: rg for `OrderingGuard|debug_assert_wire_ordered` outside events.rs hits only `crates/playback-engine/tests/events_contract.rs` (lines 71-72, 454-460, 625-627, 672-674, 1161). Neither shipped host (`apps/cli/src/host.rs`, `apps/web/src/host.rs`) runs it, and `debug_assert_wire_ordered`'s advertised users ("emission sites and hosts can sprinkle") include **zero** emission sites — session.rs never calls it. The guard's doc-05 invariants are therefore enforced only in a test harness, never on the wire the user's host actually consumes.
Why it matters: the doc sentence is false as written, and the "differential check that keeps the policy and the engine honest" (D-5) does not exist in production. A real ordering regression would reach users unflagged.
What correct looks like: either the CLI/web host wraps its drain loop with the guard in debug builds, or the doc says "the contract suites run the guard; hosts do not (yet)".
Confidence: HIGH — Verification: read (rg over apps/ and crates/).

### F-004 [S3] — the advertised config key `playback.time-tick-interval-ms` is read by nothing
Where: crates/playback-engine/src/events.rs:25-27, 144-146
Code:
```rust
//! 1. **`timing.time-tick` is the only continuous stream** — its
//!    interval is config-driven (`playback.time-tick-interval-ms`,
//!    default 250 ms, floor 50 ms — [`TimeTickPolicy`]) and it is
```
What is wrong: rg for `time-tick-interval` tree-wide finds only this file's comments, docs/05-command-layer.md:382, a worklog note, and a test *fixture string* (`events_contract.rs:778` builds a `ConfigSettingChanged` sample with that key — it does not apply any config). `TimeTickPolicy::from_config_ms` is never called with a config-derived value anywhere (callers: `default_policy()` internally at line 182, and unit tests). The CLI host hardcodes `TimeTickPolicy::default_policy()` (apps/cli/src/host.rs:977), discarding the configured interval even when `event_throttle` is set.
Why it matters: the floor contract (50 ms) is enforced against an input that is always the constant 250. The "config-driven" claim is doc-only; a user setting `playback.time-tick-interval-ms` to anything gets 250 ms regardless.
What correct looks like: host reads the settings key and calls `from_config_ms`, or the doc drops the "config-driven" claim.
Confidence: HIGH — Verification: read (rg `from_config_ms|time-tick-interval` tree-wide).

### F-005 [S3] — `BoundedEventQueue` is the "policy twin for hosts" that the shipped host ignores
Where: crates/playback-engine/src/events.rs:1466-1483 (claim), 1484-1542 (impl)
Code:
```rust
/// This is the policy twin for the engine's *edges*: hosts, test
/// collectors and future bridges that accumulate wire events must be
/// as bounded as the engine feeding them — an unbounded collector on
/// a bounded channel is a leak with extra steps.
```
What is wrong: the shipped CLI host accumulates every wire event for the host's lifetime in an unbounded `Vec` — `apps/cli/src/host.rs:172` `events: Vec<WireEvent>` ("The wire events observed since construction (tests + printing)"), pushed at host.rs:678 with no cap and no clearing. `BoundedEventQueue` itself is used only by `tests/events_contract.rs:460,627,674`.
Why it matters: the engine's outbox is capped at 1024 (OUTBOX_CAPACITY) but the host that drains it grows without bound — exactly the "leak with extra steps" the module names. In a multi-hour session (or an event storm), host memory grows monotonically.
What correct looks like: the host collects through `BoundedEventQueue` (or drains-and-discards past a print window).
Confidence: HIGH — Verification: read (host.rs:160-200, 642-685; rg `BoundedEventQueue`).

### F-006 [S4] — stale counts: gate doc says 12/23, code says 15/26; module doc's "Lead-merge follow-up" already landed
Where: crates/playback-engine/src/events.rs:16-17 vs session.rs:748; docs/gates/phase3-exit.md:108
Code:
```rust
//! exact `session.rs` wiring line is registered as a Lead-merge
//! follow-up in the Wave-1 worklog. The bare-counter comparison it
```
What is wrong: the wiring line exists today (`session.rs:748` `if event.is_stale_at_drain(current)`), so the "registered as a follow-up" comment describes a past state. Separately, docs/gates/phase3-exit.md:108 records "12/23 emission map" while `ENGINE_EMITTED_ROWS = 15` over 26 rows (asserted at lines 1962-1974).
Why it matters: drift between the honesty instruments (gate docs, module docs, machine-checked map) is the disease this module claims to cure.
What correct looks like: update the comments/gate doc when counts move.
Confidence: HIGH — Verification: read.

### F-007 [S4] — `OrderingGuard::observe` commits state before the verdict can fail
Where: crates/playback-engine/src/events.rs:1011-1019 vs 1026-1065
Code:
```rust
        if let Some(last) = self.last_seq
            && event.seq <= last
        {
            return Err(OrderingViolation::SeqRegression {
```
What is wrong: `self.last_seq = Some(event.seq)` (line 1019) is written before the kind-level checks run; if `observe_state_change` returns Err, the guard's `state` stays old but `last_seq` and (line 1064) `prev_was_seek_completion` already advanced. Benign for the documented use (fail-fast: callers stop at first Err, tests assert `.err()` on a fresh guard), but a caller that logs-and-continues now has a corrupted trail.
Why it matters: latent misuse trap in a public validator.
What correct looks like: compute the verdict first, commit state only on Ok.
Confidence: MEDIUM — Verification: read.

## 4. Per-function table (production items)

| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| consts TIME_TICK_DEFAULT_MS/FLOOR_MS/BUFFERING_WINDOW_MS/DAMAGE_WINDOW_MS | 142-157 | REAL-UNPROVEN | — | doc 05 §3.2 numbers; floor logic exercised by tests |
| TimeTickPolicy::from_config_ms | 173-177 | REAL-UNPROVEN | S3 | never called with a real config value (F-004) |
| TimeTickPolicy::default_policy | 181-183 | REAL | — | called by CLI host (host.rs:977) |
| TimeTickPolicy::is_due | 192-199 | REAL-UNPROVEN | — | wired at session.rs:2651; saturating_sub makes backward position not-due; session resets baseline at seek/pause (session.rs:1572, 2545) so no stall |
| TimeTickPolicy::Default | 202-206 | REAL-UNPROVEN | — | delegates |
| ProgressWindow::new / for_buffering | 221-232 | REAL-UNPROVEN | — | used by session.rs:1452 |
| ProgressWindow::permits | 239-254 | REAL-UNPROVEN | — | wired session.rs:2489; opens window on first call (documented); frozen source ⇒ exactly one emission |
| ProgressWindow::open_if_closed | 260-264 | REAL-UNPROVEN | — | pub(crate), used by DamageCoalescer::record |
| ProgressWindow::window_ms | 268-270 | REAL-UNPROVEN | — | accessor |
| DamageCoalescer::new/Default | 290-296, 350-354 | REAL-UNPROVEN | — | wired session.rs:1455 |
| DamageCoalescer::record | 304-311 | REAL-UNPROVEN | — | wired session.rs:2459; opens window on first damage (fixes the documented first-flush bug) |
| DamageCoalescer::is_pending | 315-317 | REAL-UNPROVEN | — | trivial, correct |
| DamageCoalescer::flush_if_due | 323-328 | REAL-UNPROVEN | — | wired session.rs:2499 |
| DamageCoalescer::flush_all | 333-338 | REAL-UNPROVEN | — | wired session.rs:1740 (terminal flush) |
| DamageCoalescer::take_pending | 340-347 | REAL-UNPROVEN | — | resets window; per-stage counts preserved |
| StalenessStamp::new/from_pair/epoch/counter | 389-413 | REAL-UNPROVEN | — | two-word Copy stamp |
| StalenessStamp::is_stale_against | 427-429 | REAL-UNPROVEN | — | epoch-first compare; `!=` fail-safe form documented; differential test vs CancelToken oracle (line 1901) |
| DrainStaleness trait | 444-465 | REAL-UNPROVEN | — | seam trait, single impl |
| DrainStaleness for EngineEvent (staleness_stamp) | 467-476 | REAL-UNPROVEN | — | envelope session↔epoch mapping |
| DrainStaleness for EngineEvent (is_stale_at_drain) | 478-507 | REAL-UNPROVEN | — | wired session.rs:748; intra-epoch terminal exemption (Seeked/SeekFailed/TrackSelectionChanged + non-pipeline-scoped kinds) |
| EventPolicy + Debug | 515-530 | REAL-UNPROVEN | — | manual Debug (Arc<dyn MonotonicSource> not Debug) |
| EventPolicy::default_policy | 554-559 | REAL-UNPROVEN | — | wasm32-safe zero source (issue #49 regression note) |
| default_throttle_source (both cfg arms) + ZeroMonotonic | 566-589 | REAL-UNPROVEN | — | wasm arm returns frozen 0 clock — documented placeholder |
| EmissionStatus::is_emitted | 620-622 | UNREACHABLE | — | test-only consumer |
| ENGINE_EMITTED_ROWS / CATALOG_EMISSIONS | 638, 645-787 | UNREACHABLE | S2 | data is honest for 23 rows; 3 "Emitted" rows unreachable from shipped hosts (F-001); consumed only by tests |
| emission_status / is_engine_emitted | 791-802 | UNREACHABLE | — | linear scan over 26; test-only callers |
| wire_event_id | 813-842 | UNREACHABLE | — | compiler-exhaustive bridge; test-only callers; correct per-sample |
| OrderingViolation (18 variants) | 852-918 | UNREACHABLE | — | names cite doc sections; all 18 variants exercised by unit test at 2188-2480 |
| SEEK_TERMINAL_WINDOW / ANSWERED_REQUEST_WINDOW | 924-928 | UNREACHABLE | — | bounded sliding windows; >128 receipts answered ⇒ late old-origin event false-flags EventBeforeItsReceipt (documented tradeoff, bounded memory) |
| OrderingGuard::new/Default | 984-1000, 1213-1217 | UNREACHABLE | — | test-only |
| OrderingGuard::observe | 1008-1066 | UNREACHABLE | S3/S4 | seq + receipt + kind dispatch; state committed before verdict (F-007); no production caller (F-003) |
| OrderingGuard::observe_receipt | 1071-1077 | UNREACHABLE | — | drain(0..drop) O(n) shift, fine at 128 |
| OrderingGuard::finish | 1083-1091 | UNREACHABLE | — | dangling-invariant check |
| OrderingGuard::observe_state_change | 1093-1143 | UNREACHABLE | — | legal-edge + bookend + first-playing ordering checks; matches doc 05 §3.3/§4.3.4 |
| OrderingGuard::observe_tick | 1145-1162 | UNREACHABLE | — | Playing-or-post-seek rule; pause-entry tick passes because engine emits it before the state change (session.rs:1572-1579) |
| OrderingGuard::observe_buffering | 1164-1192 | UNREACHABLE | — | NaN banned; bookends; monotonic |
| OrderingGuard::observe_seek_terminal | 1194-1210 | UNREACHABLE | — | duplicate-terminal window |
| Display for OrderingViolation | 1221-1300 | UNREACHABLE | — | spec-citation strings; pure |
| debug_assert_wire_ordered | 1311-1321 | UNREACHABLE | S3 | zero call sites tree-wide (F-003) |
| LatestCoalescer (all methods) | 1355-1460 | UNREACHABLE | S3 | dead in production by own admission (F-002); logic itself correct per its unit test |
| BoundedEventQueue (all methods) | 1484-1542 | UNREACHABLE | S3 | test-infra only; the shipped host violates its doctrine (F-005); logic correct |

## 5. Test honesty (20 unit tests, lines 1544-2568)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| the_default_policy_is_constructible_and_readable (1549) | YES | two reads must be monotonic; catches a panicking/regressing default source |
| the_interval_is_floored_at_50ms (1566) | YES | max() clamp mutated to min() fails all four asserts |
| a_fresh_policy_only_sets_the_baseline (1583) | YES | None-arm must return false |
| ticks_are_due_only_at_the_interval (1592) | YES | boundary 249.999999/250000000; backward position not due |
| a_frozen_source_permits_exactly_one_progress_emission (1603) | YES | window semantics incl. 200 ms boundary |
| damage_coalesces_then_flushes_on_the_window (1612) | YES | window-from-first-damage semantics |
| terminal_flush_answers_pending_counts_and_reopens (1630) | YES | unconditional flush + window reopen |
| cross_epoch_is_stale_by_construction_whatever_the_counter (1686) | YES | the masquerade matrix incl. higher-counter leak |
| fresh_epoch_fresh_counter_passes (1736) | YES | near-miss negatives |
| supersede_then_push_storm_all_dropped_counted_bounded (1755) | YES | 10k-event storm; receipts pinned; counts exact |
| seek_terminal_exemption_is_intra_epoch_only (1814) | YES | exemption boundaries both directions |
| track_selection_terminal_joins_the_intra_epoch_exemption (1885) | YES | 3.A3 terminal added to exemption |
| the_stamp_rule_agrees_with_the_cancel_token_oracle (1901) | YES | differential check vs epoch.rs oracle (D-5) — strongest test in file |
| stamps_round_trip_through_pairs_and_envelopes (1928) | YES | u64::MAX boundaries |
| the_completeness_map_partitions_the_catalog_exactly (1956) | YES | bijection onto EventId::ALL, 15+11=26, owner non-empty |
| the_wire_id_bridge_agrees_with_the_map (1999) | YES | 8 samples incl. non-emitted rows |
| the_guard_accepts_a_legal_hand_built_stream (2133) | YES | 10-event legal prefix incl. bookends and pause-entry tick |
| the_guard_catches_the_planted_violations (2188) | YES | all 18 violation variants planted and caught |
| the_latest_coalescer_keeps_the_newest_value (2487) | YES | but tests dead code (F-002) |
| the_bounded_queue_stays_bounded_under_any_push_count (2540) | YES | 10k pushes, oldest-first drop order verified by seq |

All conclusions UNVERIFIED by execution (no cargo in this environment); reasoned mutation analysis only. W00 baseline ran the full suite green except 2 first-vertical failures, which is consistent with these passing.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "15 of 26 rows are engine-emitted today" | events.rs:109 | FALSE (as a shipped-product statement) | 3 of the 15 are hotplug-API-only, no shipped caller (F-001); 12/26 is the shipped truth |
| "hosts and the contract suites run the guard" | events.rs:939-940 | FALSE | rg: only tests/events_contract.rs runs OrderingGuard; apps/ has zero references (F-003) |
| tick interval "config-driven (`playback.time-tick-interval-ms`)" | events.rs:26-27, docs/05 §3.2 | PAPER-ONLY | no reader of the key exists (F-004) |
| "≤ 5 Hz" buffering cap | events.rs:33 | REAL-UNPROVEN | BUFFERING_WINDOW_MS=200 wired at session.rs:2489 |
| non-fatal error coalesced "per 1 s window with a count" | events.rs:38-40 | REAL-UNPROVEN | DAMAGE_WINDOW_MS=1000, record/flush wired (session.rs:2459/2499/1740) |
| outbox capacity 1024 / receipts 4096 | events.rs:44-48 | REAL-UNPROVEN | session.rs:98/109 define OUTBOX_CAPACITY/RECEIPT_CAPACITY (cross-file, session.rs is W03-A0x's) |
| "12/23 emission map" | docs/gates/phase3-exit.md:108 | FALSE (stale) | code asserts 15/26 (events.rs:1962-1974) |
| emission table rows "emitted (2.C2)" for state-changed/buffering/end-of-media/error/list-ready | events.rs:82-93 | REAL | W00 smoke: state-changed, tracks-ready, duration-known, Buffering→Playing all fired from the shipped binary |

## 7. Dead / duplicated / unreachable code

- `LatestCoalescer` — dead (F-002). Duplicates session.rs inline policy with different (richer) semantics.
- `OrderingGuard` + `debug_assert_wire_ordered` — no production caller (F-003).
- `BoundedEventQueue` — test-only; the host that needs it doesn't use it (F-005).
- `wire_event_id` / `emission_status` / `is_engine_emitted` / `CATALOG_EMISSIONS` — test-only bookkeeping (not dead: the contract suite genuinely asserts against it).

## 8. Dependencies & flags

- No feature flags gate this file (playback-engine Cargo.toml exposes no cfg toggles on events). The only cfg is `target_arch = "wasm32"` for `default_throttle_source`/`ZeroMonotonic` (566-589) — the wasm arm is honest about being a frozen clock.
- Depends on: `commands` (EventId/EventKind/WireEvent/PlaybackState), `media-clock` (MonotonicSource), `crate::epoch`, `crate::event`. All workspace-internal.

## 9. Open questions for the next agent

- session.rs (another W03 agent): confirm the outbox tick-replacement path (`time_ticks_replaced`, session.rs:623) and the `EventBeforeItsReceipt` guarantee on the real channel.
- apps/cli/src/host.rs (W13): the unbounded `events: Vec<WireEvent>` (F-005) and the hardcoded default tick interval (F-004) live there.
- wave 18/19 claim-ledger agents: use 12/26 (not 15/26) as the shipped-emitted count unless a host wiring lands.
- hotplug.rs owner: the trio's reachability verdict depends on whether any audio-output backend ever reports loss; W00 says null-only.
