# AUDIT: crates/playback-engine/src/pipeline.rs#part5 (lines 2669–3338)
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-21 |
| Lines / functions | 670 (chunk L2669–3338 of 3339) / 24 rows |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `apps/cli` (`lumen`) → `apps/cli/src/host.rs` uses `playback_engine` → `Session` `session.rs:1426` `Pipeline::new` → `session.rs:2431` `pipeline.tick(state,&budget)` → `decode_step` `pipeline.rs:2362` `limb.decoder.decode(packet,&mut sink)` → `impl FrameSink for LimbSink::accept` `pipeline.rs:2675`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "fn tick|low_streak|episode_open|…"` (mapped starvation state machine L1990–2107 that these tests drive); `rg -n "preroll_discarded|fold_preroll_discards"` (accounting, session.rs:931–934); `rg -n "stale_frames_dropped|preroll_frames_discarded" crates/playback-engine/tests/` (integration coverage); `rg -n "Pipeline::new|\.tick\("` (reachability); `sed -n` on every quoted range; baseline grep (below) |

Chunk boundary note: part5 = L2669–3338: tail of `PrerollHoldSlot::push` (2665–2672, cap body), `impl FrameSink for LimbSink<'_>` (2674–2724), and the entire `#[cfg(test)] mod tests` (2726–3338). Production code in this chunk is ~56 lines; the rest is the Card-3.B3 policy test suite.

## 1. One-paragraph verdict
This chunk is real, executed, and mostly honest. The production half is the frame exit path (`LimbSink::accept`): generation check, pre-roll hold capture, concealment latch, consumer hand-off — traced end to end and driven by integration suites (`seek_flow.rs:1371` asserts `stale_frames_dropped >= 1`; `session_flow.rs:165` asserts 0 on live frames). The test half drives the REAL `Pipeline::tick` starvation machine (L2029–2081) against the REAL `BoundedQueue` with minimal fakes; all 10 tests executed green in the no-fail-fast baseline (`audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3789–3798`, playback_engine binary at :3740). One real defect: the hold-capacity refusal path double-books a dropped frame — counted as a "stale" drop AND as a delivered frame AND eligible as the first-frame anchor — contradicting `TickReport::stale_frames`' own doc and inflating `EngineStats::frames_delivered` with frames never presented to anyone. The old fail-fast baseline hid nothing here (all these tests passed in both runs); nothing in this chunk explains any of the 7 no-fail-fast failures.

## 2. What actually works (verified)
— `FrameSink::accept` generation gate, hold capture, consumer hand-off, concealment latch: verified by tracing `decode_step` (L2302–2316 sink construction; L2362 decode call) plus baseline `test pipeline::tests::* … ok` (10/10, nofailfast log :3789–3798) and integration asserts `seek_flow.rs:1371` / `session_flow.rs:165`.
— Starve-debounce machine (fresh-episode 1-tick fire, re-entry debounce, full-recovery close, seek-mint reset): tests executed green in baseline (nofailfast :3789–3798) and their arithmetic re-derived by hand against `tick` L2050–2072 (fill 8, 1 pop/tick, high=6, low=2 per `thresholds` L400–415: 8−8/4=6, 8/4=2 — matches the `wired()` doc comment L2819–2821).
— Recycle path uses the parked codec: `seek_flush` L1344–1351 reconfigures cached decoders with `cached.codec.clone()`, exactly what `parked_codec_rides_the_recycle_path` asserts (L3328–3336); a mutation to `CodecDescriptor::new("unknown")` fails the assert.

## 3. Findings
### F-001 [S3]
Where: crates/playback-engine/src/pipeline.rs:2696-2699
Code:
```
            if !hold.push(frame) {
                self.stale_dropped = self.stale_dropped.saturating_add(1);
            }
            self.frames = self.frames.saturating_add(1);
```
What is wrong: A frame refused by the full pre-roll hold is triple-booked: (a) counted into `stale_dropped` → `report.stale_frames` → `EngineStats::stale_frames_dropped`, a counter documented as "Frames dropped at the sink for stale generations … Always zero in v0's single-writer ticks" (L536–540) — a capacity refusal is not a stale-generation drop, and it is NOT always zero in v0; (b) counted into `self.frames` → `report.frames` → `EngineStats::frames_delivered` ("the consumer-delivered truth", L522–528) — the only correction path, `fold_preroll_discards` (session.rs:931–934, fed by `preroll_discarded` L1582/1618/1155), never counts burst refusals, so the inflation is permanent; (c) `self.first` can latch the refused frame's pts (L2700–2702), making the seek re-anchor / first-frame fact the pts of a frame that never presented.
Why it matters: Stats lie about both drop-cause and delivery count exactly in the pathological case (a single `decode` call emitting more than `hold_cap = budget.packets_per_limb` frames, session.rs:2040) an operator would be debugging. No test in the repo drives the burst-refusal path (rg over tests/ shows only `preroll_frames_discarded` coverage for hold discards at end_seek/release).
What correct looks like: A separate `hold_refused` counter (or fold into `preroll_discarded`), excluded from `frames`/`first`, and a doc fix on `TickReport::stale_frames`; plus one test that bursts a decoder past `hold_cap`.
Confidence: HIGH — Verification: read (L2688–2704, L522–545, session.rs:861/931–934) + rg coverage search.

### F-002 [S4]
Where: crates/playback-engine/src/pipeline.rs:3063
Code:
```
        let _ = pipeline.tick(SessionState::Playing, &budget); // 7? no: pops 1 → 7 (high)
```
What is wrong: Abandoned self-correction left in the source ("7? no: pops 1 → 7").
Why it matters: Comment noise in the one test whose whole value is exact scripted arithmetic; a reader must re-derive which of "7? no:" or "→ 7" is truth.
What correct looks like: `// 7 (high)`.
Confidence: HIGH — Verification: read (sed -n '3060,3069p').

### F-003 [S4]
Where: crates/playback-engine/src/pipeline.rs:3185-3198
Code:
```
    fn unknown_duration_mix_reaches_low() {
        let mut pipeline = wired(2);
        let budget = budget();
        fill(&mut pipeline, 8);
        for _ in 0..5 {
```
What is wrong: Near-duplicate of `first_stall_of_a_fresh_episode_fires_at_one_tick` (L3059–3071): same `wired(2)`, same `fill` — and `fill` (L2861–2879) ALWAYS pushes `duration: None` packets, so both tests already run the duration-less mix. The only delta is asserting the five non-low ticks are not starved.
Why it matters: Redundant coverage sold as a distinct wedge case ("the `Weigh`-maps-unknown-to-0 wedge case") — the packets-axis-degenerate-low policy is already proven by `packets_axis_is_the_starvation_axis` (L2887–2905).
What correct looks like: Either delete or make the mix actually differ (duration-bearing packets with unknown-duration ones).
Confidence: HIGH — Verification: read (L2861–2879, L3059–3071, L3185–3198).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PrerollHoldSlot::push` (tail; head in part4) | 2665–2672 | REAL | — | Cap-checked push, refusal signaled by `bool`; correct; no panic path. |
| `impl FrameSink for LimbSink::accept` | 2674–2724 | REAL | S3 | F-001 booking defect on hold refusal; stale gate L2677–2682 correct; consumer error propagates via `?` before counters mutate (L2714) — correct ordering. |
| `tests::StalledDemuxer` (impl Demuxer) | 2745–2775 | REAL | — | Fake; honest (produces nothing, seek lands nowhere). |
| `tests::NullDecoder` | 2781–2797 | REAL | — | Fake; swallows packets by design (tests observe watermarks). |
| `tests::NullProvider` | 2799–2809 | REAL | — | Fake provider. |
| `tests::NullConsumer` | 2811–2817 | REAL | — | Fake consumer; default `ready_for` true (trait L167). |
| `tests::wired` | 2822–2847 | REAL | — | White-box: drains the wire-mint discontinuity marker so scripted levels are exact; asserts the marker exists (pins `wire` behavior). |
| `tests::budget` | 2851–2857 | REAL | — | 0 demux events / 1 decode per limb — matches tick arithmetic. |
| `tests::fill` | 2861–2879 | REAL | — | Pushes directly into `limb.queue` (bypasses demuxer) — legitimate for watermark scripting. |
| `tests::packets_axis_is_the_starvation_axis` | 2887–2905 | REAL | — | Would fail if `thresholds` (L400–415) made low bind on bytes/duration (mutation: low bytes mark = 75% cap → first assert fails). |
| `tests::GatedConsumer` | 2910–2937 | REAL | — | Scriptable `ready_for(Audio)` + accepted counter. |
| `tests::EmittingDecoder` | 2943–2972 | REAL | — | 1 audio frame per packet — the parked-slice observable. |
| `tests::not_ready_consumer_parks_audio_decode_and_resumes` | 2982–3054 | REAL | — | Mutation: delete the gate at L2343–2348 → accepts=12≠0 → fails. Honest. |
| `tests::first_stall_of_a_fresh_episode_fires_at_one_tick` | 3059–3071 | REAL | S4 | F-002 comment; mutation: drop `!limb.episode_open` (L2066) → streak 1<2 → assert fails. |
| `tests::single_tick_reentry_dips_do_not_refire` | 3077–3110 | REAL | — | Mutation: fire on every low tick → `dip.starved` true → fails; also pins `starved_ticks`=3 raw counter. |
| `tests::sustained_low_fires_at_the_debounce_count` | 3115–3131 | REAL | — | Mutation: `>` vs `>=` (L2066) → t3 fails; off-by-one debounce caught. |
| `tests::full_recovery_closes_the_episode` | 3136–3152 | REAL | — | Mutation: remove `mark.high → episode_open=false` (L2057–2061) → `fresh.starved` false → fails. |
| `tests::seek_mint_resets_the_debounce_state` | 3157–3167 | REAL | — | State-based (asserts `low_streak`/`episode_open` after `seek_flush` L1264–1265) — fails if the reset lines are deleted; not behavioral but valid. |
| `tests::debounce_one_is_the_undebounced_behavior` | 3172–3180 | REAL | — | Catches `>` vs `>=` and debounce-floor misconfig. |
| `tests::unknown_duration_mix_reaches_low` | 3185–3198 | REAL | S4 | F-003 duplication. |
| `tests::TwoCodecDemuxer` | 3205–3234 | REAL | — | Fake; two audio tracks, distinct codecs. |
| `tests::RecordingDecoder` | 3243–3263 | REAL | — | Records `configure` codecs — the F-4 observable. |
| `tests::RecordingProvider` | 3266–3280 | REAL | — | Shares one log across decoder instances. |
| `tests::parked_codec_rides_the_recycle_path` | 3291–3337 | REAL | — | Pins `seek_flush` L1344–1351 `cached.codec` (not re-derived, not "unknown"); mutation to fabricated descriptor fails both asserts. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| packets_axis_is_the_starvation_axis | YES | Mutating `thresholds` low marks to bind on bytes/duration flips the first assert. |
| not_ready_consumer_parks_audio_decode_and_resumes | YES | Removing the L2343–2348 `ready_for` gate yields accepts=12 vs asserted 0. |
| first_stall_of_a_fresh_episode_fires_at_one_tick | YES | Removing `!episode_open` from L2066 delays fire to streak 2 → assert fails. |
| single_tick_reentry_dips_do_not_refire | YES | Removing the debounce (fire on every low) makes `dip.starved` true. |
| sustained_low_fires_at_the_debounce_count | YES | `>` instead of `>=` at L2066 fails t3; raising/lowering debounce shifts the fire tick. |
| full_recovery_closes_the_episode | YES | Removing the high-watermark episode close (L2057–2061) fails `fresh.starved`. |
| seek_mint_resets_the_debounce_state | YES | Deleting L1264–1265 leaves streak/episode non-reset → asserts fail. |
| debounce_one_is_the_undebounced_behavior | YES | Any strict-inequality or floor bug stops a low tick from firing. |
| unknown_duration_mix_reaches_low | YES (weakly) | Same mechanism as first_stall; redundant but not tautological. |
| parked_codec_rides_the_recycle_path | YES | Fabricating "unknown" or re-deriving the descriptor fails the exact-vector assert L3328–3336. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`stale_frames` … Always zero in v0's single-writer ticks — … the tests pin that it never fires on live-generation frames" | pipeline.rs L536–540 (TickReport doc) | FALSE in the hold-burst case (F-001), TRUE for the live-frame half | Refusal path L2696–2698 writes it; live-frame half pinned by `session_flow.rs:165` `assert_eq!(stats.stale_frames_dropped, 0)`; seek case `seek_flow.rs:1371` `>= 1`. |
| All 10 chunk tests pass | test suite itself | REAL | `orchrun-test-workspace-nofailfast.log.txt:3789–3798` — 10 `pipeline::tests::* … ok`. |
| "a refused frame … counted as a discard" (hold backstop) | comment L2689–2691 | REAL (but mis-bucketed) | Counted into `stale_dropped` L2697 — see F-001. |

## 7. Dead / duplicated / unreachable code
None in this chunk: every helper is used by at least one test (verified by reading each call site); no `#[allow]`, no dead fakes. F-003 documents test-level duplication.

## 8. Dependencies & flags
Test module uses `testkit::Must` and `media_types::packet::SharedBytes` — both workspace crates, no new deps. No feature gates in this chunk. Whole file compiled by the default workspace build (baseline build #1 CLEAN).

## 9. Open questions for the next agent
- To W03-A09 (pipeline.rs#part3): `obtain_decoder` cache-hit path (L1691–1692) reconfigures with `track.codec.clone()` (re-derived from the CURRENT track list), while `seek_flush`'s parked path (L1344–1345) uses `cached.codec`. The `parked_codec_rides_the_recycle_path` doc comment (L3282–3289) claims the recycle path "never" re-derives — true only for the seek-flush half. If a source swap re-reports the same TrackId with a different codec, the cached decoder gets configured for a codec it was never provided for. Audit whether any swap path can hit that.
- To W03-A10 (pipeline.rs#part4): duplicated comment block at L2014–2019 ("The in-episode progress" verbatim twice) — hygiene.
- Baseline cross-ref: none of the 7 no-fail-fast failures (INDEX footer) touches this chunk; `first-vertical` failures are journey/evidence-level, not engine-tick-level.
