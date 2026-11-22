# AUDIT: crates/playback-engine/tests/replacement.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 1254 / 6 tests + marker-carrying harness |
| File verdict | REAL — the source-replacement acceptance suite: marker-by-content frame attribution, 50 rapid play-new-while-playing switches with per-switch epoch/staleness/trail/no-stale-frames assertions, plus Buffering/Opening replacement, in-flight-seek supersession, totals/seq survival, undrained-hub suppression, and failed-open-after-replacement |
| Reachable from shipped entry point? | N/A (test binary). The subject (`Engine::replace_live_session` / open-while-open) is the shipped replacement path (session.rs:1245+) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read/Grep (quotes verified with unfiltered tools) |

## 1. One-paragraph verdict

The strongest acceptance suite in my file list. The harness's key
design decision kills the classic replacement-test cheat: every packet
of source k carries marker k in its payload and the fake decoders copy
the marker into every frame, so "no stale frames" is proven BY CONTENT
(`frame_marker`, :337-356) — no epoch plumbing that could agree with
itself. The AC test (:564-718) runs 51 opens = 50 replacements, each
departing from Playing, and asserts per switch: fresh session id
(epoch arithmetic `k+1`), the previous handle's typed
`SessionError::SessionClosed` death, zero delivered events of any
superseded session (every event's session id checked, :612-618), the
exact 3-row state trail of the new epoch (:620-643), the implicit
close's two-event suppressed trail by arithmetic, every delivered
frame carrying marker k, ≥1 frame per wired limb, and the
`frames_delivered` counter cross-checked against the sink. The
supporting tests cover the Buffering typed refusal with the pipeline
provably alive afterwards, the Opening direct-close differential, the
in-flight seek's silent supersession (receipt survives, terminal
owed nothing — doc 05 §4.2), engine totals/seq surviving across
epochs, the six-switch undrained-hub suppression with order
preserved, and the failed-open → Error → close → Idle → third-open
chain.

## 2. What actually works (verified)

— Marker attribution: `MarkerDemuxer`/`MarkerAudioDecoder`/
`MarkerVideoDecoder` (:153-306) stamp content; `frame_marker`
(:337-356) extracts it — a stale-frame leak from epoch k into k+1
fails the content check regardless of generation bookkeeping.
— The per-switch assertions listed above are exact-value (session ids,
trail vecs, counts) — mutation reasoning (UNVERIFIED): a stale event
surviving drain fails :612-618; a stale frame fails the marker check;
an epoch reuse fails `k+1`; a dropped receipt fails the undrained-hub
test's pinned set.
- The suite's own module doc carries a "deviation note" about the
  suppressed implicit-close trail — the honesty marker that the
  pinned arithmetic documents an implementation choice, not a spec
  quote.

## 3. Findings

### F-001 [S4] The marker alphabet is u8 (256 sources max) — fine for 51, silent wrap beyond
Where: replacement.rs:337-356, :374-386

Code:
```
fn frame_marker(frame: &Frame) -> Option<u8> {
```
What is wrong: markers are single bytes; a future longer run (the
AC says 50, the constant-free loop says `0..=50`) wrapping at 256
would attribute source 256's frames to source 0 and the no-stale
check would vacuously pass on collisions. Not reachable in this
suite (51 < 256) — a latent harness constraint worth one comment.
What correct looks like: a `const MAX_SOURCES: u32 = 256` next to the
marker type, or assert `k < 256` in `open_source`.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Test/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| marker harness (SharedNow, Collector, MarkerDemuxer/Decoders/Provider, builders) | 108-563 | REAL | — | content-level attribution; loud failures |
| `fifty_rapid_switches_playing_to_playing` | 564-718 | REAL | — | THE AC; per-switch epoch/staleness/trail/marker/counter assertions |
| `replacement_from_buffering_composes_and_replaces_from_opening` | 719-820 | REAL | — | typed refusal + pipeline-alive proof + Opening differential |
| `in_flight_seek_dies_silently_at_replacement` | 821-998 | REAL | — | supersession contract; receipt survives |
| `replacement_preserves_engine_totals_and_seq` | 999-1061 | REAL | — | R4 totals merge; global seq monotone |
| `undrained_hub_suppresses_old_epoch_events` | 1062-1149 | REAL | — | six switches undrained; pinned sets; order |
| failed-open-after-replacement test | 1150-1254 | REAL | — | Error ordering, close→Idle, third open |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 6 | YES | exact trails, session-id arithmetic, content markers, pinned event sets, counter cross-checks |

Not executed by me (no cargo); W00 ran the suite green.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every delivered frame carrying marker k (**no stale frames**)" | :17-19 | REAL | the content check in the AC loop |
| "a generation that never completed owes no terminal" | :26-27 | REAL | the in-flight-seek test |
| "zero delivered events of any superseded session" | :13-14 | REAL | per-event session-id check |

## 7. Dead / duplicated / unreachable code

Local fakes again (marker-armed variants — justified here: the common
harness has no marker support; the duplication is purposeful and
documented).

## 8. Dependencies & flags

`playback_engine`, `commands`, `codec_api`, `media_clock`,
`media_types`, `packet_queue`, `plugin_api`, `testkit`. No feature
gates.

## 9. Open questions for the next agent

- Wave 3 A02 (session.rs): the "deviation note" the module docs
  mention for the implicit-close trail — confirm the session's
  close_plan arithmetic matches the pinned two-event suppression
  exactly (the pipeline audit traced close_plan at session.rs:1245).
