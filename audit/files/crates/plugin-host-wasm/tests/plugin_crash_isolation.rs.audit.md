# AUDIT: crates/plugin-host-wasm/tests/plugin_crash_isolation.rs
| Field | Value |
|---|---|
| Auditor | W12-A10 |
| Date | 2026-11-24 |
| Lines / functions | 737 / 15 (6 tests, 9 harness items) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (test-only artifact; runs in `cargo test --workspace`. The machinery under test — plugin-host-wasm — is absent from the lumen-cli/desktop dep graphs) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 1 |
| Commands run | `sed -n` quote verification (all quotes below); baseline cite `audit/runs/baseline/cargo-test-workspace.txt:4959-4967` (6/6 ok, 0.02s); `rg -n 'plugin-host-wasm' --glob **/Cargo.toml` (no dependent crate); `rg -n 'ExtensionSupervisor'` (test+lib.rs only) |

## 1. One-paragraph verdict
The suite is genuine and executed: the wasm fixtures really trap (`unreachable` opcode 0x00 emitted by the in-test builder, common/mod.rs:640-643), wasmi really contains them (a crossing panic fails `storm_dispatch`'s `catch_unwind` guard), and the real `playback_engine::Engine` really completes a 96-packet session interleaved with hostile dispatches (baseline: 6/6 passed, 0.02s). The supervisor's decisions (0/5s/30s/Disabled, 15-min re-enable) are computed from the committed schema and asserted exactly. What the suite does NOT prove: the backoff delays are enforced by anyone — the test itself is the "embedder" and merely *simulates* waiting — and none of this machinery is reachable in a shipped build (plugin-host-wasm is in no shipped dep graph; `plugins.crashed` is admitted unwired by its own ignored contract test).

## 2. What actually works (verified)
— verified by `audit/runs/baseline/cargo-test-workspace.txt:4959-4967` -> `running 6 tests … 6 passed; 0 failed; 0 ignored` (crash_injection_trap_on_entry/mid_call/host_fault_from_import/resource_exhaustion, plugin_restart_backoff, plugin_crash_isolation all `ok`). Fixtures verified real traps: `sed -n '640,643p' tests/common/mod.rs` (every export of crash_trap_on_entry_wasm is `unreachablev()`).

## 3. Findings
F-001 [S2]
Where: crates/plugin-host-wasm/tests/plugin_crash_isolation.rs:76-86
Code:
```
    match supervisor.state(EXT_ID, EXT_VERSION) {
        RunState::Disabled { .. } => None, // policy refuses the respawn
        RunState::Enabled { .. } => {
            let mut instance = ok(spawn(wasm, manifest.clone(), HostServices::empty()));
            let outcome = instance.invoke(entry, &[]);
```
What is wrong: The "restart policy" is never enforced. `supervised_dispatch` consults only Enabled/Disabled — an Enabled extension can be respawned 1 ms after a crash that demanded a 5 s or 30 s wait. The supervisor itself tracks no deadline (`RunState::Enabled { next_restart_ms: None }` always, supervisor.rs:414-418), so nothing in the repo — supervisor or embedder — enforces backoff timing; the test fakes waiting with `now_ms += delay_ms` (line 303-305). Why it matters: the doc-06 §5.6 sequence this suite advertises ("immediate, 5 s, 30 s, Disabled") is data on an event, not behavior; a real embedder respawning on every tick would hot-loop a crashing extension at restart #2/#3 speed. What correct looks like: supervisor records `last_crash_ms` and `state()` reports `next_restart_ms: Some(deadline)` the embedder must respect (the field already exists and is dead). Confidence: HIGH — Verification: read.
F-002 [S2]
Where: crates/plugin-host-wasm/tests/plugin_crash_isolation.rs:1-7 (header claim) vs shipped reality
Code:
```
//! §9.5 crash isolation & lifecycle (card 7.F4, issue #46): the doc
//! 06 §7.5 crash-injection suite driven end to end through the REAL
//! host and the REAL supervisor — and the **playback-unaffected
//! proof** (§9.5 `plugin_crash_isolation`), which runs the real
//! playback engine (the scenario-harness pattern from
```
What is wrong: "end to end through the REAL host" ends at the test harness. No shipped entry point loads plugins at all: `plugin-host-wasm` appears in no other crate's Cargo.toml (rg over all Cargo.toml: only its own), the lumen-cli release build compiles `plugin-api` but never `plugin-host-wasm` (baseline cargo-build-release-lumen-cli.txt:140 — `Compiling plugin-api`, no plugin-host-wasm line), and the `plugins.crashed` event the supervisor emits is admitted unwired by the repo's own ignored test (`crates/playback-engine/tests/events_contract.rs:1727` gap_plugins_crashed, baseline line 4234: "ignored, gap: plugins.crashed not wired — handoff H-10"). Why it matters: docs/gates/phase7-exit.md:233 markets this suite as the product's crash-isolation proof ("LANDED … playback-unaffected proof"); in shipped builds the entire property is vacuous — no host, no supervisor, no events. What correct looks like: gate doc scoped to "crate-level suite; host not yet wired into any app" until an embedder exists. Confidence: HIGH — Verification: read + rg + baseline cite.
F-003 [S3]
Where: crates/plugin-host-wasm/tests/plugin_crash_isolation.rs:414-429 (and 441-491)
Code:
```
struct FakeDemuxer {
    tracks: Vec<Track>,
    script: Mutex<VecDeque<DemuxEvent>>,
}

impl Demuxer for FakeDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        Ok(self.tracks.clone())
    }
```
What is wrong: The "playback-unaffected" proof drives the engine with FakeDemuxer/FakeDecoder/FakeProvider — one frame per packet, no decode work, no demux parsing. Real demux/decode code (the code that actually could be disturbed by host state corruption) is out of the loop, and the storm is serialized on the test thread (doc 06 §5.8 specifies an extension thread pool). What is proven is: the engine's state machine + queues survive in-process trap dispatches. That is materially narrower than "a plugin crash cannot take down playback". Disclosed in comments ("scripted stages, never a mocked engine") but the gate-doc phrasing is broader. Confidence: HIGH — Verification: read.
F-004 [S3]
Where: crates/plugin-host-wasm/tests/plugin_crash_isolation.rs:617-644
Code:
```
    for step in 0..2_000u64 {
        playback.engine.poll();

        if step % 8 == 0
            && matches!(
                supervisor.state(EXT_ID, EXT_VERSION),
                RunState::Enabled { .. }
```
What is wrong: The "extension storm" is one dispatch every 8 polls, single-threaded, same-thread as playback — the maximum-pressure claim ("trap-on-every-call extension storm") is a light drizzle: 4 dispatches total before the policy disables the extension (asserted `dispatches >= 1`, `>= 4`). No concurrency, no reentrancy from inside engine callbacks, no dispatch during a poll's interior. A bug where a trap corrupts shared state only under actual interleaving inside `poll()` would be invisible. Confidence: MEDIUM — Verification: read.
F-005 [S4]
Where: crates/plugin-host-wasm/tests/plugin_crash_isolation.rs:536-543, 565-568
Code:
```
    let clock = Arc::new(
        media_clock::MediaClock::system_monotonic(
            Arc::new(SharedNow(AtomicU64::new(0))),
```
What is wrong: The media clock and throttle policy are frozen at t=0 for the whole session (the `now_ms` counter advances only the supervisor's virtual clock). The proof therefore exercises only the degenerate clock path; it passes only because delivery is poll/budget-driven, not clock-paced — an observation about the engine worth recording, since it means this suite cannot catch pacing regressions. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| supervised_dispatch | 69-87 | REAL | — | embedder stand-in; ignores delay_ms (F-001) |
| crash_injection_trap_on_entry_is_contained | 93-137 | REAL | — | would fail if wasmi leaked the trap (process abort) or supervisor miscounted |
| crash_injection_trap_mid_call_is_contained | 139-183 | REAL | — | asserts import effect survives, Faulted status, event shape Isolated{1,0} |
| crash_injection_host_fault_from_import_is_contained | 185-222 | REAL | — | 3rd allowlist violation → ExtensionDisabled; escalation_count==3 |
| crash_injection_resource_exhaustion_is_contained | 224-284 | REAL | — | CpuLimit <5s, MemoryLimit at 1 MiB cap; honest |
| plugin_restart_backoff | 290-382 | REAL | — | exact 0/5000/30000/Disabled + reenable_at=+900_000; waiting simulated (F-001) |
| SharedNow::now_ns | 390-396 | REAL | — | frozen clock (F-005) |
| Collector::accept | 404-409 | REAL | — | counts frames; poison-tolerant lock recovery |
| FakeDemuxer (start/next_event/seek) | 419-438 | REAL | — | scripted; seek errors (F-003) |
| FakeDecoder (configure/decode/flush) | 445-476 | REAL | — | 1 frame/packet; keeps stale-packet guard |
| FakeProvider::decoder_for | 481-491 | REAL | — | always succeeds |
| open_audio | 501-572 | REAL | — | real Engine::new/open with QueuePlan/TickBudget/EventPolicy |
| storm_dispatch | 581-593 | REAL | — | catch_unwind guard makes trap-escape fatal to the test — honest |
| plugin_crash_isolation | 595-728 | REAL | — | the headline proof; see F-002/F-003/F-004 |
| frames_of | 730-737 | REAL | — | trivial accessor |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| crash_injection_trap_on_entry_is_contained | YES | wasmi leak → process abort; miscounted crashes → assert fails |
| crash_injection_trap_mid_call_is_contained | YES | asserts Faulted, InstanceFaulted refusal, exact event tuple |
| crash_injection_host_fault_from_import_is_contained | YES | asserts typed ExtensionDisabled + escalation_count==3 |
| crash_injection_resource_exhaustion_is_contained | YES | asserts CpuLimit/MemoryLimit variants + <5s wall bound |
| plugin_restart_backoff | YES | asserts exact 4-action sequence + 900_000 re-enable + post-disable refusal |
| plugin_crash_isolation | YES | asserts Ended + all 96 frames + ≥4 dispatches + 4 events + Disabled |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "6/0 LANDED, suite green incl. playback-unaffected proof" | docs/gates/phase7-exit.md:233 | REAL (suite) / misleading (product scope) | baseline 6/6 ok; but host not in any shipped binary (F-002) |
| "runs the real playback engine … never a mocked engine" | file header + doc 06 §9.5 | REAL | Engine::new/open real; stages are scripted fakes (disclosed) |
| supervisor policy "values come from the settings schema, never re-stated here" | file header | REAL | schemas/settings.toml:1409-1469 defaults 5000/6/300000/3/900000 match assertions |

## 7. Dead / duplicated / unreachable code
None in-file. The *machinery under test* is unreachable from shipped entry points (F-002): no crate depends on plugin-host-wasm; `ExtensionSupervisor` has zero non-test constructors outside its own crate.

## 8. Dependencies & flags
Test-only crate deps: playback-engine, plugin-api, media-*, codec-api, testkit, plugin-host-wasm (self). No feature gates. No #[ignore] in this file. Common fixtures (tests/common/mod.rs) are a std-only wasm binary emitter — real, validating through wasmi `Module::new`.

## 9. Open questions for the next agent
- Who is the intended production embedder (handoff H-10/H-8/H-9)? Until one exists, every supervisor/`plugins.crashed` claim is crate-local.
- Does `apps/web` (lumen-web) ever load plugin-host-wasm via wasm32? (If the web build embeds the host, reachability changes.) — assigned files below will check.
