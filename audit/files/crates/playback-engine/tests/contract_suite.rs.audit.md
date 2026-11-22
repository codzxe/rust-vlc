# AUDIT: crates/playback-engine/tests/contract_suite.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 1499 / 12 test fns + 20 helpers/fakes/builders |
| File verdict | REAL (strong engine-level integration suite) |
| Reachable from shipped entry point? | N/A (test) — plain `#[test]`s, no ignores, no cfg gates; part of W00's baseline (1751 pass / 2 fail / 25 ignored; whether either failure is in this file UNVERIFIED — no cargo). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | Read in 4 chunks; cross-check of engine API names against playback-engine/src (Engine/SessionHandle/TickBudget/EventPolicy all exported, lib.rs:105-112 area); sed verification. No cargo (adaptation). |

## 1. One-paragraph verdict

The doc 05 §7 contract suite driven against the **real** playback `Engine` through its public API: scripted demuxers/decoders implement the real `plugin_api::Demuxer`/`codec_api::Decoder` seams, one shared virtual monotonic source drives both the media clock and the event throttle (no sleeps), and every assertion is an exact behavioral fact — state trails, seq monotonicity, receipt-before-caused-event ordering, terminal invariants, generation-stamped seek terminals, suppression counts, µs payloads, coalesced damage counts. The fakes sit at the I/O/codec boundaries, which is the legitimate place for fakes; the subject under test is the engine itself. The two hygiene findings: the frame collector accumulates frames that no test ever asserts (the frame-delivery path is exercised but unobserved), and a preemptive lint allow.

## 2. What actually works (verified)

- Real engine, real seams: `Engine::open(SessionConfig{…})` with `ScriptDemuxer`/`MarkDemuxer` (Demuxer impls 129-222), `LimbProvider` (DecoderProvider 316-335), `Collector` (FrameConsumer 92-100), `MediaClock::system_monotonic` over `SharedNow` (454-461) — the same clock construction the shipped CLI uses for audio-less media (pipeline_host.rs:1630).
- Test 1 (692-885) proves seven ordering invariants in one walk with exact assertions: seq strictly increasing (720-731), receipt-before-event causality (735-750), list-ready/duration-known/info-ready before first → playing (755-779), duration before every tick (782-788), payload values (791-817), end-of-media before → ended (820-831), buffering bookends + monotonic interior (837-844), exact state trail (847-857), and the single pause-entry immediate tick at the frozen position (862-884). Each is a `==`/`<` on positioned indices — no tautology.
- The "never queued" tick proof (1029-1052) asserts the hub kept only the newest tick AND the replacement count (3) — the counted part is what makes it non-vacuous.
- Supersession and drain-exemption tests (1108-1255) drive the repeated-seek storm shape (both seeks submitted before one poll) and assert exactly one terminal for the newest generation, receipts surviving, and `suppressed_stale_events >= 2/3` counts.
- The damage test (1425-1499) asserts the coalesced count "9" arrives in one non-fatal event with the fatal half never firing.
- `poll_until` (542-560) is bounded and fails loudly with context; no unbounded loops, no sleeps anywhere in the file.

## 3. Findings

### F-001 [S4] `Collector` accumulates frames no test asserts — the frame path is exercised blind
Where: crates/playback-engine/tests/contract_suite.rs:86-90
Code:
```rust
#[derive(Default)]
struct Collector {
    #[allow(dead_code)]
    frames: Mutex<Vec<(TrackId, Frame)>>,
}
```
What is wrong: Every session pushes its decoded frames into `Collector::frames`, and no test in the file reads the vector (the `#[allow(dead_code)]` admits it). The suite proves the event contract while ignoring whether/what/in-what-order frames were delivered to the consumer.
Why it matters: A regression that dropped 50% of frames or reordered them would pass this suite untouched. The suite's module doc scopes itself to the wire contract (accurate), but the unused collector creates the appearance of frame-path coverage. Hygiene/scope.
What correct looks like: Either assert per-scenario frame counts/PTS order from the collector (cheap, high value), or remove the field and let `FrameConsumer` be a no-op.
Confidence: HIGH — Verification: read (rg `frames` in file: only the struct + push).
### F-002 [S4] Preemptive `#[allow(clippy::too_many_arguments)]` on a 6-parameter function
Where: crates/playback-engine/tests/contract_suite.rs:437-438
Code:
```rust
#[allow(clippy::too_many_arguments)]
fn open_with(
```
What is wrong: clippy's `too_many_arguments` default threshold is 7; `open_with` takes 6 parameters. The allow suppresses nothing today and would silently suppress a real trigger if an 8th argument were added.
Why it matters: A lint allow that is dead now and broad later — the `#[allow]` hygiene the workspace otherwise polices.
What correct looks like: Drop the allow; clippy passes at 6 args (or a builder struct if it grows).
Confidence: HIGH — Verification: read (parameter count).

## 4. Per-function table

| Test / helper | Lines | Verdict | Notes |
|---|---|---|---|
| `SharedNow` + `advance_ms` | 72-83 | REAL | Virtual time, no sleeps. |
| `Collector` | 86-100 | SHALLOW | F-001. |
| `Media`/`lock_media` | 105-117 | REAL | Scripted state; poison-recovering lock. |
| `ScriptDemuxer` (Demuxer impl) | 123-187 | REAL | Ordered packets, stall, fail-start, compute-then-commit reanchor (D4). |
| `MarkDemuxer` | 192-222 | REAL | Fixed script incl. `Recovered` marks. |
| `reanchor_refusal`/`stamp_ns` | 224-240 | REAL | Typed errors; saturating ns conversion. |
| `FakeDecoder`/`StallFlushDecoder`/`LimbProvider` | 243-335 | REAL | Real Decoder seam; stale-packet guard; 9-vs-8 flush-budget overrun drives the multi-tick drain. |
| builders (`base`/`at_millis`/`track`/`packet`/`audio_frame`/`tiny_queues`/`budget`) | 341-415 | REAL | Deterministic fixtures; tiny caps make watermark/queue behavior observable. |
| `Fixture`/`open`/`open_with`/`open_script`/`audio_media` | 417-537 | REAL | F-002 on open_with. |
| `poll_until` | 542-560 | REAL | Bounded, loud failure. |
| wire helpers (`WireItem`…`seek_terminals`) | 566-674 | REAL | The observation surface; engine-private kinds filtered by `as_wire_event`. |
| the_wire_stream_honors_every_doc_05_ordering_invariant | 692-885 | REAL | The flagship walk; exact everywhere. |
| c8_idempotent_commands_confirm_with_self_transition_echoes | 892-946 | REAL | Echo + no second pause tick. |
| a_fatal_error_precedes_its_state_changed_to_error | 951-985 | REAL | Stage/code payload asserted. |
| time_ticks_fire_on_the_interval_and_carry_the_wire_payload | 996-1023 | REAL | Exact tick times + fraction math. |
| undelivered_time_ticks_are_never_queued | 1029-1052 | REAL | Newest-only + replacement count. |
| a_seek_completion_emits_the_immediate_tick_at_the_landing | 1058-1096 | REAL | terminal+1 adjacency asserted. |
| a_completed_seek_terminal_survives_a_later_generation_mint | 1108-1172 | REAL | Drain exemption + suppression counts. |
| superseded_seeks_are_silent_and_stale_events_are_suppressed | 1185-1255 | REAL | Storm shape; one terminal, gen-stamped. |
| in_episode_buffering_progress_is_monotonic_and_window_capped | 1267-1314 | REAL | 0.0 + one sample + 1.0 exactly. |
| an_elapsed_window_emits_the_monotonic_climb | 1320-1369 | REAL | ≥2 strictly monotonic samples. |
| end_of_track_fires_for_a_limb_that_ends_before_the_media | 1382-1418 | REAL | Asymmetric decoder ends; exactly one end-of-track. |
| non_fatal_damage_coalesces_into_one_counted_playback_error | 1425-1499 | REAL | count "9" in one event; clean end. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 12 | YES | Exact values/indices/counts everywhere; mutation examples: reorder receipt after event → guarantee-2 loop fails; queue ticks → `tick_times` equality fails; deliver stale events → suppression asserts + `is_none` checks fail; lose the damage count → "9" assert fails; fire a second end-of-track → `ends.len()==1` fails. |

No mocks-of-subject (subject = Engine, driven for real). No sleeps. No ignored tests. Caveat: the frame path (F-001) is unobserved.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every observable is a WireEvent/EventKind … never an engine-internal type" (3-6) | module doc | REAL | `as_wire_event` filter at 580. |
| The AC bullet list (8-37) | module doc | REAL | Each bullet maps to a named test (§4 table). |
| "no sleeps" (69-70, doc 03 §8) | module doc | REAL | rg: zero `thread::sleep` in file. |
| (Implied) frame delivery correctness | — | NOT TESTED | F-001. |

## 7. Dead / duplicated / unreachable code

`Collector.frames` (F-001). Minor duplication: `open`/`open_script` repeat the config assembly (~15 lines) — acceptable builder duplication, not filed.

## 8. Dependencies & flags

dev-deps: playback-engine, plugin-api, codec-api, commands, media-types, media-clock, packet-queue, testkit. No feature gates. Runs in the default profile.

## 9. Open questions for the next agent

- W00/W20: which 2 of the 1751 tests fail, and are either in this file? (UNVERIFIED here.)
- W03 colleagues with playback-engine/src/session.rs + events.rs: this suite is your behavioral oracle — its assertions (e.g., `suppressed_stale_events`, `time_ticks_replaced`) depend on `Engine::stats()` being honest counters, not derived from the outbox.
