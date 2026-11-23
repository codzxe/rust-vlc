# AUDIT: crates/playback-engine/tests/events_contract.rs
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-23 |
| Lines / functions | 1741 / 43 (12 tests, 11 of them `#[ignore]` gap markers) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test binary). Reached via `cargo test -p playback-engine --test events_contract` — ran live, exit 0. Exercises the real `Engine` + `events.rs` policy layer shipped in the release lane. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `cargo test -p playback-engine --test events_contract` -> 11 passed; 0 failed; 11 ignored (exit 0); `rg`/`sed`/`od` verifications listed per finding |

## 1. One-paragraph verdict
This is a genuine, hostile contract suite, not a mock-of-the-thing-tested theater. It drives the real `Engine` (playback-engine/src/lib.rs) through scripted demuxers/decoders on a shared virtual clock and pins exact wire event id sequences, payloads, generations, and ordering through the real `OrderingGuard`, `BoundedEventQueue`, `CATALOG_EMISSIONS`, and `wire_event_id` machinery from `playback-engine/src/events.rs`. The 11 gap tests are honest `#[ignore]` markers with named owners, paired with a live negative test (`the_wired_later_rows_stay_silent_today`) that would fail if a gap row leaked. Defects found are test-strength/doc-level only: the module header is stale (23 rows / "twelve emitted" vs the code's 26 / 15), the "every engine kind" test has no exhaustiveness mechanism, and the "zero-drop accounting" claim is a trivially-true tripwire.

## 2. What actually works (verified)
— verified by `cargo test -p playback-engine --test events_contract` -> `test result: ok. 11 passed; 0 failed; 11 ignored` (matches baseline cargo-test-workspace.txt:4223-4247). The happy-path test pins a 14-row exact id sequence with per-row payload asserts; the stop test plants a synthetic tick and proves `OrderingGuard` names `TickOutsidePlaying { at: Stopped }`; the bijection test cross-checks `CATALOG_EMISSIONS` (26 rows, 15 emitted — confirmed `ENGINE_EMITTED_ROWS: usize = 15` at src/events.rs:638) against the generated `EventId::ALL`.

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/tests/events_contract.rs:849-963
Code:
```
/// Every engine event kind either maps onto an **emitted** catalog
/// row or is one of the two documented engine-private kinds — which
/// must answer `None` on the wire conversion (they never leak).
...
    assert_eq!(kinds.len(), 19, "one sample per engine kind");
```
What is wrong: The test's exhaustiveness is hand-maintained. `kinds` is a hand-built `Vec`; `assert_eq!(kinds.len(), 19)` pins only the list's own length. There is no generated registry of `EngineEventKind` variants (rg for `ALL_ENGINE|engine_kinds|EngineEventKind::ALL` in src/ -> no hits), and `kind.as_wire()` uses no exhaustiveness-forcing hook into this test. Adding a 20th variant (e.g. a new engine-private kind that DOES leak a wire row) leaves this test green. The catalog-side twin does not share this hole because `EventId::ALL` is generated and zipped (line 827).
Why it matters: The header sells "every engine `EngineEventKind` variant either maps onto an emitted row or …" as a proven invariant; the mechanism cannot detect new variants, only deletions from the sample list.
What correct looks like: A generated variant list (macro or `const ALL: &[EngineEventKind]`) zipped with the samples, mirroring the `EventId::ALL` pattern.
Confidence: HIGH — Verification: read + rg (no generated list exists).

F-002 [S4]
Where: crates/playback-engine/tests/events_contract.rs:10-16 and 41-42
Code:
```
//! - **the completeness map, proven both ways**: the 23 catalog rows
//!   (D-017-frozen; `EventId::ALL` — the generated catalog — is the
//!   source of truth) are classified by
...
//! `media.info-ready` — the map's twelve emitted rows. The eleven
```
What is wrong: The module doc is stale on both counts it advertises. The code asserts 26 rows (`assert_eq!(samples.len(), 26, "D-017 23 + the issue #42 trio")`, line 823) and 15 emitted (`assert_eq!(emitted, 15)`, line 841; `ENGINE_EMITTED_ROWS = 15` at src/events.rs:638). The header says 23 rows and "twelve emitted rows". A reader auditing coverage from the header gets both numbers wrong.
Why it matters: Doc-vs-code disagreement inside the very file whose job is contract truth.
What correct looks like: Header updated to 26 rows / 15 emitted (or made count-free).
Confidence: HIGH — Verification: read; quotes verified with sed; counts verified against src/events.rs:638 and the passing test.

F-003 [S4]
Where: crates/playback-engine/tests/events_contract.rs:26-34 and 565-573
Code:
```
//!   (the D-2 bounded-collector policy twin — and its zero-drop
//!   accounting proves no scenario storms past the bound).
...
    fn finish_clean(&self) {
        self.guard.finish().must();
        assert_eq!(
            self.collector.dropped_total(),
            0,
            "no scenario may storm past the collector bound"
        );
```
What is wrong: The collector is built with capacity 4096 (line 627) while the largest scenario stream is ~30 items (the happy path asserts a 14-event sequence). `dropped_total() == 0` and `len() <= capacity()` cannot fail unless the engine emits >4096 events in one scripted session — the assertion is a tripwire, not a proof of burst behavior, and the drop path of `BoundedEventQueue` is never exercised here (it is exercised in src/events.rs unit tests, `assert_eq!(queue.dropped_total(), 10_000 - 256)` at events.rs:2552). The header phrase "proves no scenario storms past the bound" oversells a vacuous-by-construction check.
Why it matters: Evidence-string inflation — the suite's claimed policy coverage is weaker than stated.
What correct looks like: One scenario sized to actually hit the bound, or the header saying "tripwire".
Confidence: HIGH — Verification: read; capacity at line 627; drop-path coverage confirmed in src/events.rs:2537-2557.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SharedNow / impl MonotonicSource | 93-99 | REAL | — | Atomic virtual clock, feeds both media clock and throttle (line 616) — honest shared time. |
| advance_ms | 101-103 | REAL | — | ns arithmetic correct (ms * 1_000_000). |
| CounterSink | 108-119 | REAL | — | Counts frames, deliberately not inspected (events are the subject) — documented, fine. |
| Media / lock_media | 123-134 | REAL | — | Poison-tolerant lock recovery, fine for a test harness. |
| ScriptDemuxer::start/next_event/seek/seekability/reanchor | 143-198 | REAL | — | seek() always refuses (`Unsupported`); reanchor does real landing math (rposition ≤ target, line 183-187). |
| reanchor_refusal | 202-210 | REAL | — | Typed D4 refusal constructor. |
| MarkDemuxer | 214-244 | REAL | — | Fixed script incl. `Recovered` marks; `seekability` claims Fast but seek() refuses — unused path in these scenarios, cosmetic. |
| stamp_ns | 246-252 | REAL | — | Saturating i64::try_from, correct. |
| FakeDecoder | 255-280 | REAL | — | One frame per packet, generation-stale check honored (line 270). |
| StallFlushDecoder | 285-315 | REAL | — | 9 flush calls vs budget 8 — actually forces the two-tick drain the test claims. |
| LimbProvider | 319-342 | REAL | — | Steers stall decoder to the video limb only. |
| base/at_millis/track/packet/audio_frame | 348-403 | REAL | — | Builders; 1ms-stride packets, µs timestamps consistent with WireTimestamp doc (commands/src/catalog/types.rs:29). |
| tiny_queues / budget / audio_media | 405-431 | REAL | — | Budget 12/4/8 matches the "flush budget is 8" comment at line 332-333. |
| WireItem / Session::record/poll/poll_until/kinds/ids/state_trail/receipt_for/event_index/finish_clean | 439-573 | REAL | — | Records receipts+events in channel order, both guard forms observe; finish_clean enforces guard invariants. F-003 covers finish_clean's collector half. |
| open_session / open_standard / open_script | 578-676 | REAL | — | Real Engine::open with real MediaClock on the virtual source; duplicated builder body between open_session/open_script (cosmetic dup, not flagged separately). |
| every_catalog_row_sample | 685-811 | REAL | — | 26 samples; zip against EventId::ALL enforced. |
| the_completeness_map_and_bridge_are_catalog_bijections | 819-847 | REAL | — | Bijection + map arithmetic; self-enforcing via generated ALL. |
| every_engine_kind_lands_on_an_emitted_row_or_stays_private | 852-963 | SHALLOW | S3 | F-001: no exhaustiveness mechanism for new variants. |
| happy_path_session_emits_the_exact_wire_sequence_and_payloads | 973-1097 | REAL | — | 14-row exact sequence, payload-per-row, receipt-before-event causal spot-check (1083-1096). |
| stop_ends_the_stream_and_a_planted_tick_after_stop_is_caught | 1103-1190 | REAL | — | Negative space + planted violation named; ran green. |
| seek_completed_carries_the_gen_and_payload_then_the_immediate_tick | 1198-1265 | REAL | — | Gen bump, µs landing, terminal+1 tick adjacency. |
| an_unseekable_source_answers_the_typed_seek_failed | 1271-1324 | REAL | — | Typed refusal payload incl. old position "0", no fake success. |
| end_of_track_precedes_end_of_media_with_the_track_payload | 1331-1380 | REAL | — | Asymmetric-limb ordering eot < eom < ended. |
| non_fatal_damage_coalesces_into_one_counted_error | 1386-1447 | REAL | — | Count 9 = 2+3+4 coalesced into one non-fatal error. |
| fatal_open_failure_emits_error_then_state_changed_error | 1452-1508 | REAL | — | Error-before-state order, stage/code bridging. |
| track_selection_completes_with_the_changed_terminal | 1514-1591 | REAL | — | Gen-carrying terminal under the switch's own gen. |
| full_session_ids | 1601-1614 | REAL | — | Shared scenario for gap tests. |
| the_wired_later_rows_stay_silent_today | 1623-1635 | REAL | — | Live negative half; would fail on a phantom emission. |
| gap_* (11 tests) | 1644-1741 | REAL (as markers) | — | All `#[ignore]` with named owner handoffs H-1..H-11; honest negative-space bookkeeping, never deleted. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| the_completeness_map_and_bridge_are_catalog_bijections | YES | Bijection vs generated EventId::ALL; any map/bridge drift breaks the zip. |
| every_engine_kind_lands_on_an_emitted_row_or_stays_private | PARTIALLY | Catches drift in the 19 existing kinds; blind to NEW kinds (F-001). |
| happy_path... | YES | Exact 14-id sequence + per-row payload + state trail; any emission change fails it. |
| stop_..._caught | YES | Both the silent-tail asserts and the planted-violation guard verdict are behavioral. |
| seek_completed... / an_unseekable... | YES | Terminal count, payload fields, generation, adjacency all pinned. |
| end_of_track... | YES | Exactly one EOT, payload, triple ordering. |
| non_fatal_damage... | YES | Exactly one error, count string "9". |
| fatal_open_failure... | YES | Order + stage/code bridge + trail. |
| track_selection... | YES | Receipt gen, terminal count/payload/gen. |
| the_wired_later_rows_stay_silent_today | YES | Any non-emitted id in a real stream fails it. |
| gap_* (11) | N/A (ignored) | Positive-contract placeholders with owners; paired with the live negative guard above. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the 23 catalog rows … are classified" | module doc, lines 10-11 | FALSE (stale) | Test line 823 asserts 26; EventId::ALL len 26 (test passes). |
| "the map's twelve emitted rows" | module doc, line 41 | FALSE (stale) | Test line 841 asserts 15; events.rs:638 `ENGINE_EMITTED_ROWS: usize = 15`. |
| "planted violations (… error-after-state-change, bookend-less episodes, …) are caught" | module doc, lines 27-30 | PARTLY FALSE | Only the tick-after-stop class is planted HERE (lines 1161-1189); the other classes are tested in src/events.rs's own unit matrix (events.rs:2184-2237), not "against the real stream shape" in this suite. |
| "zero-drop accounting proves no scenario storms past the bound" | module doc, lines 33-34 | OVERSOLD | F-003: 4096 capacity vs ~30-event scenarios; drop path never taken here. |
| "Rows proven here: … (both halves)" | module doc, lines 36-43 | REAL | fatal (1452) and non-fatal (1386) playback.error tests exist and pass. |

## 7. Dead / duplicated / unreachable code
- `open_session` vs `open_script` duplicate the whole builder body (lines 578-629 vs 638-676) modulo demuxer choice — duplication, cosmetic (S4-level, folded into F-002 file hygiene, not separately counted).
- MarkDemuxer::seekability advertises `SeekSupport::Fast` while its seek() always refuses (lines 241-238) — inconsistent fake, never exercised in these scenarios.
- No dead test helpers found; every builder is used (rg-verified `StallFlushDecoder`, `MarkDemuxer`, `open_script` each referenced by ≥1 test).

## 8. Dependencies & flags
- Uses `testkit::Must` (panic-on-None helper), `plugin_api::Demuxer`, `codec_api::Decoder`, `commands` wire types, `media_clock::MonotonicSource`, `packet_queue::QueueLimits` — all dev/test-graph crates of playback-engine; no feature gating; runs in the plain `cargo test` lane (verified live).

## 9. Open questions for the next agent
- src/events.rs (the policy layer this suite leans on) is assigned elsewhere; its own unit matrix should be cross-checked to confirm the guard's other violation classes have negative tests (I saw `the_guard_catches_the_planted_violations` at events.rs:2188 — appears covered).
- The `OrderingGuard` replay in the stop test uses `seq: u64::MAX` on the planted event; whether `SeqRegression` masks `TickOutsidePlaying` ordering in other shapes is a property of events.rs:1008-1155 — not this file.
