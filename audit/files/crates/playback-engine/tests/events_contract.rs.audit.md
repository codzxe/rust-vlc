# AUDIT: crates/playback-engine/tests/events_contract.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 1741 / 15 tests + ~30 harness items |
| File verdict | REAL — the doc 05 §7 events contract on virtual time: exact wire sequences, per-row payloads, bijection proofs, ordering guard with planted violations; 11 honest `#[ignore]` gap tests with a live anti-leak guard |
| Reachable from shipped entry point? | N/A (test binary). The tested subject is the shipped engine's event policy (`events.rs`) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | Read/Grep (quotes verified with unfiltered tools; not executed — no cargo) |

## 1. One-paragraph verdict

The strongest contract suite audited in this wave. The happy-path test
pins the exact 14-row wire sequence of an open→prime→play→pause→
resume→end session (:996-1015), then asserts payloads row by row (state
trail, buffering progress `[0.0, 0.75, 1.0]`, track list with selection
truth, duration, info snapshot, tick time+fraction, end-of-media
ordering, :1017-1096) and the receipts-precede-events rule explicitly
(:1083-1096). The completeness map is proven both ways — the
`EventKind`↔`EventId` bridge is a bijection over all 23 catalog rows
and every engine kind is either emitted or one of two private kinds
(:819-971). The `OrderingGuard` observes every full stream and planted
violations are caught (the stop test, :1103+). The 11 `WiredLater` gap
tests are `#[ignore]`d with owner-card reasons — and, unusually
honestly, a live companion test (`the_wired_later_rows_stay_silent_
today`, :1623-1635) asserts those rows do NOT leak today, so the
ignore/un-ignore pair is forced to travel with the emission map.

## 2. What actually works (verified)

— The suite runs the real `Engine` through seam fakes
(`ScriptDemuxer`/`MarkDemuxer`/`FakeDecoder`/`StallFlushDecoder`) on
one shared virtual clock that drives BOTH the media clock and the
throttle (`advance_ms`, :101-103) — time-tick cadence assertions are
therefore real behavior, not sleeps.
— The happy-path sequence and payloads are exact-value assertions; any
emission-order or payload regression fails the pinned vec.
— The bijection tests consume the generated catalog
(`EventId::ALL` :685) — a new catalog row without a map entry fails
`the_completeness_map_and_bridge_are_catalog_bijections`.
— Mutation reasoning (UNVERIFIED, no cargo): swapping two rows in the
pinned sequence fails; emitting a private kind fails the
engine-kind test; a receipts-after-events ordering fails both the
guard and the explicit spot-check.

## 3. Findings

### F-001 [S4] Eleven catalog rows have no engine behavior — 11 of the 25 workspace-ignored tests live here
Where: events_contract.rs:1644-1741 (the `gap_*` tests)

Code:
```
#[test]
#[ignore = "gap: session.opening-progress not wired — handoff H-1 (streaming opens are Phase 6; local opens are synchronous)"]
fn gap_session_opening_progress() {
```
What is wrong: nothing dishonest — every ignore names its owner card
(H-1..H-11: rate-changed, device lost/changed, screenshot, playlist,
settings, plugins loaded/failed/crashed, system log), the module doc
admits "no engine behavior to test yet" (:41-43), and the live guard
(:1623-1635) prevents phantom emissions. But the §3.4 ledger fact
stands: these are 11 never-running tests asserting features that do
not exist (rate change, device observation, playlists, settings,
plugins, logging). They are part of W00's "25 ignored" count and map
directly to the engine's missing feature surface.
Confidence: HIGH — Verification: read.

### F-002 [S4] The buffering-progress climb `[0.0, 0.75, 1.0]` is asserted against a frozen clock's single-sample window
Where: events_contract.rs:1030-1041

Code:
```
    // `session.buffering` — opens 0.0, one in-episode climb
    // (0.75: the single sample the ≤ 5 Hz window permits under the
    // frozen virtual clock — monotonic, §3.4.2), closes 1.0.
```
What is wrong: the mid-episode progress value (0.75) is a
frozen-throttle artifact (exactly one window sample permitted), not a
sampled climb — the comment says so. The assertion is correct for this
harness, but it pins a harness-specific number: a host with a moving
clock would emit more samples, and someone re-deriving the contract
from this test could mistake 0.75 for a policy constant.
What correct looks like: assert the sub-sequence is monotonic and
bounded `[0.0, 1.0]` with the endpoints exact, rather than the exact
middle sample.
Confidence: HIGH — Verification: read.

## 4. Per-function table (tests; harness items grouped)

| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `the_completeness_map_and_bridge_are_catalog_bijections` | 819-851 | REAL | — | both directions over the generated catalog |
| `every_engine_kind_lands_on_an_emitted_row_or_stays_private` | 852-972 | REAL | — | the two private kinds must never leak |
| `happy_path_session_emits_the_exact_wire_sequence_and_payloads` | 973-1097 | REAL | S4 | exact 14-row pin; 0.75 climb is harness-specific (F-002) |
| `stop_ends_the_stream_and_a_planted_tick_after_stop_is_caught` | 1103-1197 | REAL | — | planted-violation proof of the guard |
| `seek_completed_carries_the_gen_and_payload_then_the_immediate_tick` | 1198-1270 | REAL | — | |
| `an_unseekable_source_answers_the_typed_seek_failed` | 1271-1330 | REAL | — | |
| `end_of_track_precedes_end_of_media_with_the_track_payload` | 1386-1451 | REAL | — | |
| `non_fatal_damage_coalesces_into_one_counted_error` | 1387-1451 | REAL | — | |
| `fatal_open_failure_emits_error_then_state_changed_error` | 1452-1513 | REAL | — | |
| `track_selection_completes_with_the_changed_terminal` | 1514-1600 | REAL | — | |
| `the_wired_later_rows_stay_silent_today` | 1623-1635 | REAL | — | the live anti-leak guard |
| `gap_*` (11 tests) | 1644-1741 | REAL (contract) / not run | S4 | honest ignored positives naming owners (F-001) |
| harness (SharedNow, CounterSink, Media, ScriptDemuxer, MarkDemuxer, FakeDecoder, StallFlushDecoder, LimbProvider, Session, builders) | 93-682 | REAL | — | seam fakes; shared virtual time; `Session::finish_clean` runs the OrderingGuard |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 15 non-ignored | YES | exact sequences/payloads, bijections, planted violations, guard-clean finishes |
| the 11 gap_* | N/A (ignored) | they assert unwired features; un-ignoring is gated on the owning card |

Not executed by me (no cargo); W00: workspace suite green, 25 ignored
(11 of them here).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the 23 catalog rows … classified … bridge is a bijection" | :10-15 | REAL | test :819-851 |
| "planted violations … are caught and named against the real stream shape" | :28-30 | REAL | stop test :1103+ |
| "The eleven WiredLater rows … have no engine behavior to test yet" | :41-43 | REAL | the gap_* ignores (F-001) |

## 7. Dead / duplicated / unreachable code

The harness duplicates the common/mod.rs fakes in adapted form
(different session shape with wire items) — acceptable; the sibling
`contract_suite.rs` is referenced as separate and merge-blocking
(:7-8), not audited here.

## 8. Dependencies & flags

`playback_engine` (events policy surface is `pub` for tests),
`commands`, `codec_api`, `media_clock`, `media_types`, `packet_queue`,
`plugin_api`, `testkit`. No feature gates on this file.

## 9. Open questions for the next agent

- Wave 16: `OrderingGuard`, `BoundedEventQueue`, `CATALOG_EMISSIONS`
  live in `playback-engine/src/events.rs` (another agent's file) — the
  guard's own correctness (does it really catch all §4.3 violations?)
  is decided there, not here.
- Wave 19 (claim ledger): rows citing the events contract as evidence
  should be verdicted on the 12 emitted rows only; the 11 WiredLater
  rows are confessed gaps.
