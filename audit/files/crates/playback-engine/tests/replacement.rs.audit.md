# AUDIT: crates/playback-engine/tests/replacement.rs
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-23 |
| Lines / functions | 1254 / 40 (6 `#[test]`) |
| File verdict | REAL (tests are genuine and pass) — but the FEATURE they prove (in-place source replacement) is UNREACHABLE from every shipped entry point |
| Reachable from shipped entry point? | Test file: dev lane. Code under test: `Engine::open`'s replacement branch (crates/playback-engine/src/session.rs:1061-1064 `if self.session.is_some() { self.replace_live_session()?; }`) is NEVER taken from shipped hosts: apps/cli/src/host.rs:897-903 and apps/web/src/host.rs:1446-1452 both refuse a live session ("close it first (source replacement is card 3.A2)") before calling `engine.open`; apps/desktop routes through `lumen_cli::host::CliHost` (apps/desktop/src/dispatch.rs:67, shell.rs:139). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `cargo test -p playback-engine --test replacement` -> 6 passed, 0 failed (exit 0); baseline cargo-test-workspace.txt:4370-4376 (6 passed) |

## 1. One-paragraph verdict
A genuinely adversarial suite: every packet carries its source marker in its payload and the fake decoders copy that marker into frame bytes and (video) frame width, so every delivered frame is attributed to its epoch BY CONTENT — stale-frame leaks cannot hide behind plumbing. The AC test does 51 opens / 50 live replacements with per-switch exact assertions (fresh session id, dead old handle, zero stale events, exact 3-edge state trail, suppressed-events arithmetic of exactly 2, marker-only frames, per-limb progress, totals cross-check). I ran it: 6/6 pass. The problem is not the tests — it is that the capability they prove (live-session replacement, card 3.A2) is implemented in the engine but wired to NOTHING shipped: all three hosts refuse a live-session open, so this entire flow is dead code in the release lane.

## 2. What actually works (verified)
— verified by `~/.cargo/bin/cargo test -p playback-engine --test replacement` -> `test result: ok. 6 passed; 0 failed` (exit 0). Matches baseline cargo-test-workspace.txt:4370-4376.

## 3. Findings
F-001 [S3]
Where: apps/cli/src/host.rs:897-903 (feature reachability; cross-file)
Code:
```
        if self.engine.session_id().is_some() {
            return Err(CommandError::new(
                Stage::Session,
                ErrorCode::NotReady,
                "a session is already open — close it first (source replacement is card 3.A2)",
            ));
        }
```
What is wrong: the engine HAS implemented card 3.A2 (session.rs:1061-1064 `replace_live_session`, proven by this 6-test suite), but the CLI host refuses to route it and claims replacement is a future card. Same refusal in apps/web/src/host.rs:1446-1452; the desktop shares the CLI host. Result: the entire replacement machinery — implicit stop+close composition, epoch drain, marker-clean teardown, totals merge — is UNREACHABLE from lumen-cli, desktop, and web. Users must close-then-open, paying the full device/decoder teardown this engine path was built to avoid. Why it matters: ~1250 lines of proven engine capability are dead in the shipped product, and the error text misleads ("card 3.A2" implies unimplemented). What correct looks like: wire `open_session` to the engine path (drop the refusal) or delete/mark the message as a deliberate v0 posture. Confidence: HIGH — Verification: read all three hosts + ran the engine suite proving the path works. UNVERIFIED: whether some matrix/doc row claims shipped "source switching" (next agent: check feature-matrix.toml).

F-002 [S4]
Where: crates/playback-engine/tests/replacement.rs:202-210
Code:
```
    fn reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        // Compute-then-commit (D4): every landing here succeeds and
        // restarts the stream from its head — the tests that seek get
        // superseded by a replacement before any post-seek packet
        // matters, so the simplest honest commit is a full restart.
```
What is wrong: the fake `reanchor` returns `Ok(target)` (the request) rather than a computed landing — acceptable here because every seek in this file is killed by a replacement before its pre-roll completes (the tests assert exactly that), but a reader could mistake it for landing-truth semantics; contrast seek_flow.rs:153-178 which computes the real landing. No test can currently observe the difference. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/playback-engine/tests/replacement.rs:1155-1168
Code:
```
    // The exact total is policy-shaped (the pause-entry ticks follow the
    // hub's replace-resident-tick rule, counted separately), so the
    // lower bound pins what must hold for ANY policy: the six
    // implicit-close trail pairs (12 `state-changed` events, never
```
What is wrong: the undrained-hub suppression assertion is a lower bound (`>= 12 + reference`), not an exact pin — weaker than the exact-arithmetic pins used in tests 1-3 and 6. Justified in-comment (tick-replacement policy shape), but it means a regression that suppresses too much (e.g. swallowing live-epoch events beyond the cap) would still pass this specific check; the companion `outbox_dropped == 0` assertions partially compensate. Confidence: MEDIUM — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SharedNow | 108-114 | REAL | — | frozen virtual clock |
| Collector + take | 120-144 | REAL | — | poison-safe; drained per epoch window |
| MarkerDemuxer::start/next_event | 162-186 | REAL | — | synthesizes on demand; stall → Discontinuity; fail_start armed |
| MarkerDemuxer::seek | 188-196 | REAL | — | same not-the-reanchor-path guard as seek_flow.rs |
| MarkerDemuxer::seekability/reanchor | 198-210 | REAL | S4 | reanchor = full restart, returns request (F-002) |
| MarkerAudioDecoder / MarkerVideoDecoder | 217-274 | REAL | — | marker copied from packet payload into frame bytes; video width = 32+marker (2nd attribution) |
| MarkerProvider | 277-303 | REAL | — | typed refusal for non-A/V kinds |
| base/at_millis/track | 309-332 | REAL | — | |
| frame_marker | 337-350 | REAL | — | content extraction; debug_assert cross-checks width |
| source_tracks/source_kinds | 352-372 | REAL | — | ladder: k%10==0 video-only, odd audio-only, else A+V — exercises limb-count and kind flips |
| marker_packet/audio_frame/video_frame | 374-412 | REAL | — | |
| switch_queues/switch_budget | 414-433 | REAL | — | 64-packet caps keep every switch Playing-departed |
| Harness::new/source_config/corrupt_config/config_over | 439-515 | REAL | — | retained seams (clock/sink/throttle/stall) across epochs |
| open_source | 519-522 | REAL | — | the replacement entry when live |
| pump_to_playing / two_extra_pumps | 527-546 | REAL | — | hang = fail (80-poll bound) |
| event_facts | 550-558 | REAL | — | |
| fifty_rapid_switches_playing_to_playing | 564-706 | REAL | — | THE AC: 50 switches, all per-switch pins + final totals cross-check (frames_delivered == sink total, outbox_dropped == 0) |
| replacement_from_buffering_composes_and_replaces_from_opening | 719-815 | REAL | — | stall-frozen mid-episode replacement; exact suppressed counts 3 (Buffering) and 1 (Opening) |
| in_flight_seek_dies_silently_at_replacement | 821-993 | REAL | — | both arms (Paused- and Playing-departed); no Seeked/SeekFailed ever delivered; accept receipt survives with post-bump gen |
| replacement_preserves_engine_totals_and_seq | 999-1056 | REAL | — | seq strictly increasing across epochs; totals merge exactly |
| undrained_hub_suppresses_old_epoch_events | 1062-1173 | REAL | S4 | 6 undrained windows; 12 receipts in request order; suppression lower-bound (F-003) |
| replacement_then_failed_open_lands_in_error_closeably | 1179-1254 | REAL | — | fatal precedes state-changed→error; close→Idle; third open OK |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 6 | YES | Content-marker attribution means any cross-epoch frame leak fails `frame_marker(frame) == Some(k)` (659-665, 772-778, 924-928, 1050-1055). If the epoch drain leaked an old event, `event_session == session` fails (607-612). If the implicit close minted the wrong number of trail events, the exact `suppressed == 2/3/1` deltas fail. If receipts were suppressed with events, test 5's `receipts.len() == 12` fails. If totals regressed at the switch instant, test 1/4 fail. If an in-flight seek completed across a replacement, tests 3 both arms fail. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a session is already open — close it first (source replacement is card 3.A2)" | apps/cli/src/host.rs:901 | MISLEADING — the engine implements 3.A2; hosts simply refuse to wire it | session.rs:1061-1064 + this suite (executed, 6 passed); see F-001 |
| module doc: "THE AC (`fifty_rapid_switches_playing_to_playing`): 50 play-new-while-playing switches" | lines 13-23 | REAL | test executed; every listed pin present in the body |
| "the bounded-memory half of the AC ... lives in `tests/replacement_alloc.rs`" | lines 65-67 | REAL-UNPROVEN (not my file) | baseline cargo-test-workspace.txt:4377-4382 shows `memory_stays_bounded_across_fifty_switches ... ok` |

## 7. Dead / duplicated / unreachable code
The tested feature (live replacement) is UNREACHABLE from shipped hosts (F-001) — engine code is live only under `cargo test`. No dead helpers in this file; no duplication beyond the two marker decoders (near-identical bodies, justified by distinct frame constructors).

## 8. Dependencies & flags
Same dependency set as seek_flow.rs (testkit, packet_queue, media_clock, plugin_api, codec_api). No feature gates, no unsafe, no #[ignore]. Single-threaded engine — no races by construction.

## 9. Open questions for the next agent
- Does any matrix row (feature-matrix.toml) claim "open-while-playing / source switching" for a shipped platform? If yes, that row is FALSE given F-001.
- apps/web/src/host.rs:1086-1092 has a close-then-open auto-settle path — confirm it never calls engine.open live (it submits Close first; verified by read, LOW effort).
- Who owns the CLI/desktop refusal message (F-001) — is wiring replacement deliberately deferred (product posture) or an oversight?
