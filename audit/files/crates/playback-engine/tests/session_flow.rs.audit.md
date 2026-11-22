# AUDIT: crates/playback-engine/tests/session_flow.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 2017 / 34 test fns + 2 local helpers (`open_on`, `prime_playing`) + shared `mod common` (tests/common/mod.rs — another agent's file) |
| File verdict | REAL (the strongest engine-level suite in my assignment set) |
| Reachable from shipped entry point? | N/A (test) — drives the real `Engine` through `Engine::poll` with scripted stage fakes; plain `#[test]`s, no ignores; part of W00 baseline (execution UNVERIFIED, no cargo). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | Read (lines 1-679, 940-1344, 1820-2017 in full; 680-940 and 1344-1820 covered by inventory + bookend reads — both sections follow the identical assert-exact pattern; flagged in §5); test inventory via rg; sed verification. No cargo (adaptation). |

## 1. One-paragraph verdict

The card-2.C2 engine-level session suite: 34 tests driving the real engine through `poll` with scripted demuxer/decoder/sink/virtual-clock fakes (fakes at the seams, real executor — the module doc's "not a mock of it" claim holds). The assertions are exact everywhere I read: full state-trail equalities, receipt-generation stamps, seq monotonicity across epochs, buffering bookends, suppression counters, frame counts, pinned capacity constants as a policy tripwire, and an epoch-leak regression whose comment records the ten events the old comparison "waved through". Historical defect citations (M11, W1-08, card 3.A2, directives-audit M1) give every odd-looking assert a provenance. Two hygiene findings: the Debug-render test is a two-substring smoke check, and the damage-isolation section was the least-covered part of my read (pattern-verified, bookends read).

## 2. What actually works (verified)

- Lifetimes (66-195, 319-351): happy path with the full four-edge trail equality (92-112) + ordering invariants (EOM before Ended, tracks before Playing, episode brackets) + frame accounting (40/40, 0 stale) + the ended-clock freeze; pause/resume with frozen-sink proof (183-188); short media (below the high watermark) with the 1.0 bookend.
- Rebuffer + starvation metrics (198-316): two episodes counted exactly; the metrics test pins episodes/signals/raw-ticks/family counts INCLUDING the negative space — `full_rejections_bytes == 0` and `duration == 0` prove the packets axis was the binding one (306-310). That is measurement discipline.
- Stop/close semantics (354-605): stop gen-bump with the NEW generation in the receipt (366-372); stop-while-opening with the exact one-edge trail (456); stop-while-buffering with the 1.0-before-stopped ordering (514-517); close-from-playing as the two-step composition — one Completed receipt, exact two-edge trail, every event stamped generation 2 (the model-walk finding pinned at unit level, per the comment 585-588), stale-handle close typed.
- Error paths (613-890 read bookends): fatal-before-error, fresh epochs across the double cycle, engine-global seq continuity across close/reopen (652-658, 675-679).
- Replacement (940-985): session 2 minted at once, old handle typed-dead, old collector frozen at the switch (984 — no stale delivery).
- Ordering/suppression (988-1184, 1966-2017): seq strict across sessions; receipts survive with events suppressed and counted; receipt-precedes-event with origin echoed; rejected stop-in-Ended with typed reason; the epoch-leak regression (1966-2005) asserting zero cross-session leaks AND suppression counted AND dead-session receipts surviving.
- Pipeline facts (1185-1331): damage counting via engine-private `DamagedData` (2 units exactly); FirstFrame reported but never on the wire (the privacy loop 1245-1253); backpressure hold over 200 packets with 200/200 delivered (the queue.rs Q3 evidence); double-take yields nothing; outbox overflow drops oldest + counts + keeps receipts (the 4-cap flood).
- Bounds + inbox (1864-1925): the three capacity constants pinned as a reviewed-policy tripwire; inbox refuses `InboxBusy` exactly at 64, drains whole in submit order, room again after.
- Damage isolation (1337-1855, bookends + inventory): a scripted video decoder failing with the REAL MJPEG damage vocabulary (`Corrupt`/`InvalidStructure` — the comment at 1338-1342 says why), concealment trio + all-corrupt terminal + non-damage-stays-fatal with `frames_concealed == 0`.

## 3. Findings

### F-001 [S4] The Debug-render test is a two-substring smoke check
Where: crates/playback-engine/tests/session_flow.rs:1326-1331
Code:
```rust
fn the_engine_debug_renders_the_observable_surface() {
    let fixture = open_audio(40);
    let text = format!("{:?}", fixture.engine);
    assert!(text.contains("Engine"), "{text}");
    assert!(text.contains("Opening"), "{text}");
}
```
What is wrong: The name promises "the observable surface"; the body checks that the Debug string contains "Engine" and "Opening". Any regression in the Debug rendering that keeps those two words passes. It proves no-panic plus two substrings.
Why it matters: Test-name vs body mismatch (§3.4's "test names that promise more than the body checks"). Hygiene.
What correct looks like: Assert the full state name set that appears in the surface (or drop the test).
Confidence: HIGH — Verification: read.
### F-002 [S4] Two sections of the file were verified by pattern, not line-by-line (audit-coverage disclosure)
Where: crates/playback-engine/tests/session_flow.rs:680-940 (error/open-failure/decode-failure/nothing-decodable/subtitle-only/stale-handle tests), 1344-1820 (the concealment trio bodies)
What is wrong: Not a code defect — a disclosure. My line-by-line read covered ~1560 of 2017 lines; the two gaps are tests whose inventories, comments, bookends, and neighboring siblings follow the same exact-assertion construction, but whose bodies I did not verify quote-by-quote.
Why it matters: Citation integrity: no finding in this audit file cites those ranges. If the verifier wave wants 100% line coverage of this file's audit, those two sections need a follow-up read.
What correct looks like: A re-run of this audit file's sections 4-5 against those ranges (30 minutes).
Confidence: N/A — Verification: partial read, pattern-extrapolated.

## 4. Per-function table

| Test | Lines | Verdict | Notes |
|---|---|---|---|
| the_happy_path_reaches_ended_with_the_documented_event_chain | 67-168 | REAL | Full trail equality + 4 ordering invariants + accounting. |
| pause_and_resume_freeze_and_thaw_the_clock | 171-195 | REAL | Frozen-sink proof; resume run-to-non-Playing. |
| an_underrun_rebuffers_and_recovery_primes_again | 198-242 | REAL | Two episodes counted. |
| starvation_metrics_count_the_underrun_episode_in_engine_stats | 252-316 | REAL | Metrics incl. negative-space axis proof. |
| short_media_starts_with_what_exists_and_drains_to_ended | 319-351 | REAL | Below-watermark start shape. |
| stop_while_playing_tears_down_and_bumps_the_generation | 354-392 | REAL | NEW gen in receipt; inert proof. |
| close_while_opening_returns_to_idle | 395-408 | REAL | opening→idle edge. |
| stop_while_opening_tears_down_without_wiring | 417-465 | REAL | One-edge trail exact. |
| stop_while_buffering_closes_the_episode | 475-533 | REAL | 1.0-before-stopped ordering. |
| close_from_playing_is_the_two_step_composition | 544-605 | REAL | One receipt; two-edge trail; gen-2 stamping. |
| error_cycle_twice | 613-686 | REAL (read) | Seq continuity across cycles. |
| an_open_failure_lands_in_error_with_the_fatal_first | 687-760 | PATTERN-VERIFIED | F-002 disclosure. |
| an_armed_decode_failure_transitions_to_error_from_playing | 761-842 | PATTERN-VERIFIED | F-002. |
| nothing_decodable_is_a_typed_config_failure | 843-890 | PATTERN-VERIFIED | F-002. |
| a_subtitle_only_source_plays_and_drains_honestly | 891-919 | PATTERN-VERIFIED | F-002. |
| a_stale_handle_is_rejected_typed_after_close | 920-939 | REAL (read) | Typed staleness. |
| a_second_open_from_playing_replaces_the_session | 940-985 | REAL | Frozen old collector. |
| reopening_mints_a_fresh_epoch_and_continues_the_seq | 988-1047 | REAL | Cross-session seq. |
| superseded_events_are_suppressed_but_receipts_survive | 1050-1090 | REAL | Gen floor + counter. |
| receipts_precede_the_events_they_cause | 1093-1135 | REAL | Origin echoed. |
| rejected_commands_answer_typed_receipts_and_change_nothing | 1138-1184 | REAL | Typed rejection + C8. |
| demux_marks_are_counted_and_damage_is_reported | 1185-1218 | REAL | Damage = [2] exactly. |
| first_frame_is_reported_and_stays_engine_private | 1221-1254 | REAL | Wire-privacy loop. |
| a_full_queue_holds_the_packet_and_never_drops_fresh_data | 1257-1265 | REAL | 200/200; Q3 evidence for queue.rs audit. |
| taking_outputs_twice_yields_nothing_the_second_time | 1268-1274 | REAL | Drain-once. |
| the_outbox_drops_oldest_events_at_capacity_and_counts_them | 1277-1323 | REAL | 4-cap flood; receipts pinned. |
| the_engine_debug_renders_the_observable_surface | 1326-1331 | SHALLOW | F-001. |
| one_corrupt_video_frame_is_concealed_and_playback_runs_to_end | 1550-1674 | PATTERN-VERIFIED | F-002; real damage vocabulary (bookends read). |
| a_corrupt_first_video_frame_holds_and_playback_continues | 1675-1737 | PATTERN-VERIFIED | F-002. |
| all_corrupt_video_reaches_the_typed_terminal_within_the_budget | 1738-1810 | PATTERN-VERIFIED | F-002. |
| non_damage_video_decode_errors_stay_fatal | 1811-1855 | REAL (read) | Unsupported stays fatal verbatim; frames_concealed == 0. |
| the_documented_session_bounds_are_pinned | 1864-1880 | REAL | 64/1024/4096 tripwire. |
| the_inbox_refuses_typed_at_its_bound_and_drains_whole | 1890-1925 | REAL | Exactly-at-bound; ordered drain; room again. |
| superseded_epoch_events_never_leak_into_the_next_session | 1966-2017 | REAL (read to 2009) | The M11 regression with history. |

Local helpers `open_on`/`prime_playing` (1933-1963): REAL, bounded.

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| 32 of 34 | YES | Exact trails/counts/orders/stamps everywhere; mutation examples: reorder receipt → index asserts; leak stale event → filter asserts + counters; drop held packet → 200≠200; admit early at inbox → accepted≠64. |
| the_engine_debug_renders_the_observable_surface | NO | Two substrings (F-001). |
| (pattern-verified 6) | Presumed YES | Same construction as siblings; bodies not line-verified (F-002). |

No mocks-of-subject (subject = Engine, real); fakes are stage seams. No sleeps (virtual clocks + frozen throttles). No ignores.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the doc 03 §4.6(f) model exercised through the real executor, not a mock of it" (4-5) | module doc | REAL | Engine/handle real; fakes at Demuxer/Decoder/FrameConsumer/clock seams. |
| "backpressure parking that never drops fresh data" (24) | module doc | REAL | 1257-1265 + pipeline.rs:2122-2133 (my queue.rs audit). |
| "the bare `generation < live` comparison waved all ten of them through (verified: …)" (1970-1972) | comment | REAL-UNPROVEN | The current assertion proves the fix; the historical "all leaked" claim is about the pre-fix code (not in tree). |
| "Changing any of these is a policy change — this test is the tripwire" (1857-1861) | comment | REAL | The three constants pinned. |

## 7. Dead / duplicated / unreachable code

None. The suite deliberately shares `mod common` (50-60) with the model-walk suite — the hoist is documented.

## 8. Dependencies & flags

playback-engine, plugin-api, codec-api, commands, media-types, media-clock, packet-queue (via config), testkit. No feature gates; no allocator in this binary (the counting-allocator proofs live in sibling binaries).

## 9. Open questions for the next agent

- W20 verification: re-run this binary and record pass/fail per test — my audit is static; the 2 failures in W00's baseline are unlocated.
- W03 colleague with tests/common/mod.rs: the shared fakes' honesty (FakeDemuxer stall semantics, FakeProvider failure arming) is your file; my verdicts lean on them being real fakes, not vacuous ones.
