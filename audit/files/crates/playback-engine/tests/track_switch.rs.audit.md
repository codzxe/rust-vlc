# AUDIT: crates/playback-engine/tests/track_switch.rs
| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2025-06-27 |
| Lines / functions | 1891 / 39 (27 helpers/fakes + 12 tests) |
| File verdict | REAL-UNPROVEN (real engine exercised under faked demux+decode stages; assertions largely genuine but 2 claimed proofs are vacuous) |
| Reachable from shipped entry point? | Engine under test YES: apps/cli/src/host.rs:231 `engine: Engine::new()`, :983 `.open(config)` — the same `playback_engine::Engine` lumen-cli ships. The test binary itself is CI-only. |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 2 |
| Commands run | `sed -n` (quotes verified, exit 0); `rg` reachability (exit 0); baseline citation: audit/runs/baseline/cargo-test-workspace.txt:4579-4595 — "running 12 tests ... 12 passed; 0 failed; 0 ignored" |

## 1. One-paragraph verdict
This is a real engine-orchestration test suite, not a mock-of-the-thing-tested: the 12 tests drive the shipped `playback_engine::Engine` (the one lumen-cli instantiates) through command mint, generation bump, queue flush/fill, decoder cache, and clock re-anchoring — but the demuxer (`ScriptDemuxer`) and decoder (`ScriptDecoder`) are scripted fakes, so **nothing about real demux or real decode is proven here** (hostile question 2: yes, the decoder layer is faked — 1 frame per packet, audio shape baked in at line 232-242). The suite's headline claim is "event-sequence assertions"; two of the claimed sequence facts are NOT enforced: the no-stale-old-frames proof of case 1 is a numeric tautology (`X <= X`), and the frame-precedes-terminal ordering is computed then thrown away (`let _ = terminal_index;`). The remaining assertions (typed rejections, supersession silence, cache counts, clock re-anchor equality, F5 play-out gating) are genuine and mutation-sensitive.

## 2. What actually works (verified)
— All 12 tests pass: audit/runs/baseline/cargo-test-workspace.txt:4579-4595 ("running 12 tests ... test result: ok. 12 passed; 0 failed"). Not re-run here (cargo-build-budget); baseline cited per spec §2.
— Decoder-cache proof is real: `decoder_cache_recycles_instances_across_switches` (lines 1746-1820) asserts per-track build counts stay at 1 AND `stats.decoders_reused == 1` (line 1818) — a mutation that rebuilds decoders per switch fails it.
— Clock re-anchor equality (lines 779-782) and video-switch non-re-anchor (lines 911-914) are exact-value assertions; a broken anchor fails them.

## 3. Findings
F-001 [S2]
Where: crates/playback-engine/tests/track_switch.rs:751-763 and 785-796
Code:
```
    let frames_before_receipt = frames_before_switch_len(&mut fixture);
    assert!(
        frames_after <= frames_before_receipt,
        "no fresh old-track frames after the switch (had {frames_before_receipt}, total {frames_after})"
    );
...
    // The old track's total across the whole run is compared against
    // itself (an upper bound captured post-hoc is not possible); the
    // real assertion is below via generation stamps.
```
What is wrong: `frames_before_switch_len` is called at line 759, AFTER the `frames` snapshot (line 720) with zero `engine.poll()` calls in between, so it returns the identical track-2 count: the assertion is `X <= X`, true for any engine behavior including one that leaks stale old-track frames. The comment's escape hatch ("the real assertion is below via generation stamps") is false — no generation-stamp check of old-track frames exists in this test. The file header (lines 22-24) lists "zero stale old-track frames after the receipt" as proven by case 1. Why it matters: the doc's per-case proof map lies for case 1(e); a stale-frame regression in exactly the audio-switch path would not be caught here. (The property does have a real oracle in test 12, lines 1882-1890, which is why this is S2 not S0.) What correct looks like: snapshot the collector length before submitting the command, or assert no track-2 frame carries the post-switch generation. Confidence: HIGH — Verification: read (snapshots taken at 720 and 759 with no poll between; verified by sed).

F-002 [S2]
Where: crates/playback-engine/tests/track_switch.rs:717-750
Code:
```
    // (d) The new track's first frame at the sink is stamped with the
    // switch generation and precedes the terminal (the frame's
    // delivery IS the completion fact).
...
    let terminal_index = index_of(&outputs, |output| {
...
    .must();
    let _ = terminal_index;
```
What is wrong: the docstring promises the first-new-track-frame *precedes* the `tracks.changed` terminal; `terminal_index` is computed and then discarded with `let _ = terminal_index;` — the ordering is never asserted anywhere in the file. The suite's entire reason to exist (header lines 4-10: "event-sequence assertions: the ordered story") is missing one of its four orderings. Mutation: an engine that emits the terminal before delivering the first frame passes every test here. What correct looks like: assert the collector received the first track-3 frame before the poll that produced the terminal, e.g. capture frame count at terminal emission. Confidence: HIGH — Verification: read; `rg 'terminal_index'` shows only the dead binding.

F-003 [S3]
Where: crates/playback-engine/tests/track_switch.rs:654-669 (same pattern at 849-864, 1405-1420, 1583-1608, 1680-1705)
Code:
```
    let mut outputs = Vec::new();
    for _ in 0..10 {
        outputs = fixture.engine.take_outputs();
        if outputs.iter().any(|output| {
```
What is wrong: `outputs` is overwritten each iteration, so every "exactly one terminal" assertion (e.g. `assert_eq!(seeked.len(), 1)` line 1620, `selection_terms` equality line 707) inspects only the detection batch — a duplicate `Seeked`/`tracks.changed` emitted in an earlier poll batch escapes detection and the test still passes. The "exactly one" claims are actually "exactly one in the final batch". What correct looks like: accumulate `outputs.extend(...)` across iterations and assert on the union. Confidence: HIGH — Verification: read.

F-004 [S3]
Where: crates/playback-engine/tests/track_switch.rs:865-874, 1173-1177 (`terms0`), 1513-1523
Code:
```
    let terms = selection_terms(&outputs);
    assert_eq!(
        terms,
        vec![(
            terms[0].0,
            MediaKind::Video,
            Some(TrackId::new(4)),
```
What is wrong: the terminal's generation is compared against itself (`terms[0].0`), so any generation value passes; only kind/track/origin are checked. Cases 2, 4 and 7 never validate that the terminal rides the switch's own generation — unlike case 1 (line 682) and case 6 (line 1451) which do. If terms is empty the expression panics (loud failure), so this is a weakened oracle, not a false pass. What correct looks like: capture the receipt's `Accepted { generation }` and compare, as case 6 does. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/playback-engine/tests/track_switch.rs:787-789
Code:
```
fn frames_before_switch_len(fixture: &mut Fixture) -> usize {
    let _ = fixture;
```
What is wrong: dead `let _ = fixture;` immediately followed by real use of `fixture` (line 792) — leftover from an abandoned implementation, misleading about intent. Confidence: HIGH — Verification: read.

F-006 [S4]
Where: crates/playback-engine/tests/track_switch.rs:800-809
Code:
```
    let mut probe = index;
    while let Some(packet) = media_packet_at(fixture, probe) {
        if packet.track == TrackId::new(3) {
            return stamp_ns(&packet.pts).unwrap_or(0);
        }
        probe += 1;
    }
    0
```
What is wrong: the `0` fallback makes assertion (f) (`first_new_pts >= position_ns_at_switch`, line 769) vacuously true if no track-3 packet exists after the cursor (e.g. media truncated by a future edit). Practically unreachable with today's 240-step media, hence S4. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SharedNow::now_ns | 83-87 | REAL | — | virtual clock source, frozen until test advances |
| Collector::accept | 98-106 | REAL | — | collects (track, frame) in delivery order |
| ScriptDemuxer::start/next_event | 128-143 | REAL (fake by design) | — | scripted packet replay; honest seam |
| ScriptDemuxer::seek | 145-151 | REAL | — | deliberately returns Unsupported to force reanchor path |
| ScriptDemuxer::seekability | 153-159 | REAL | — | flag-driven |
| ScriptDemuxer::reanchor | 161-182 | REAL | — | rposition ≤ target; all-keyframe assumption documented |
| ScriptDecoder::configure/decode/flush | 217-272 | REAL (fake by design) | — | 1 frame/packet, baked shape — decode layer is faked (hostile Q2: YES) |
| CountingProvider::new/shape/build_count/total_builds/decoder_for | 284-351 | REAL | — | build-count oracle for the cache proof |
| base/at_millis/stamp_ns/track/packet | 357-401 | REAL | — | builders |
| tiny_queues / tiny_budget | 405-430 | REAL | — | sizing rationale documented |
| open_multi_track / open | 454-523 | REAL | — | wires real Engine with fakes |
| poll_until | 529-547 | REAL | — | always polls once; loud failure on hang |
| frames_snapshot / media_cursor / media_packet_at | 549-573 | REAL | — | accessors |
| receipt_for / selection_terms / index_of | 575-611 | REAL | — | output filtering |
| frames_before_switch_len | 787-796 | FALSE (tautology) | S2 | compares count with itself; see F-001 |
| first_packet_pts_at_or_after | 800-809 | REAL | S4 | 0-fallback weakens (f); F-006 |
| terms0 | 1278-1280 | REAL | S3 | self-referential generation; F-004 |
| test audio_switch_happy_path... | 626-783 | SHALLOW at (d)/(e) | S2 | F-001, F-002 |
| test video_switch_keeps_the_clock... | 818-915 | REAL | S3 | F-004 generation unchecked |
| test subtitle_selection_is_class_s... | 927-1015 | REAL | — | typed refusal + unbumped generation, exact assertions |
| test subtitle_swap_delivers... | 1024-1136 | REAL | — | frame + continuity checks genuine |
| test audio_deselect_lets_video_run_to_ended | 1154-1276 | REAL | S3 | F5 play-out gate pinned exactly |
| test not_found_rejects_before_the_mint... | 1291-1363 | REAL | — | typed rejection, no mint, playback continues |
| test rapid_double_switch... | 1374-1463 | REAL | — | supersession silence genuinely proven |
| test switch_while_paused... | 1474-1545 | REAL | S3 | F-004 |
| test switch_during_seek... | 1556-1650 | REAL | S3 | F-003 batch scoping |
| test seek_during_switch... | 1655-1734 | REAL | S3 | F-003 |
| test decoder_cache_recycles... | 1746-1820 | REAL | — | strongest test in file |
| test the_new_limb_fills... | 1833-1891 | REAL | — | pts-based staleness oracle (rescues case 1's claim) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| audio_switch_happy_path... | PARTIAL | generation/anchor/order-of-receipt yes; no-stale-frames NO (F-001), frame-before-terminal NO (F-002) |
| video_switch_keeps_the_clock... | YES | exact counts + assert_ne on clock |
| subtitle_selection_is_class_s... | YES | typed matches on receipt/kind/generation |
| subtitle_swap_delivers... | YES | saw_subtitle/saw_b/av_after monotone checks |
| audio_deselect_lets_video_run_to_ended | YES | state == Playing after full drain, Ended only after clock advance |
| not_found_rejects... | YES | typed rejection + frames grew |
| rapid_double_switch... | YES | exactly-one-terminal + gen_two > gen_one |
| switch_while_paused... | PARTIAL | terminal generation unchecked (F-004) |
| switch_during_seek / seek_during_switch | PARTIAL | batch-scoped "exactly one" (F-003) |
| decoder_cache_recycles... | YES | independent provider-count + engine-stats double oracle |
| the_new_limb_fills... | YES | pts bounds + stale_old scan |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "proven end-to-end through the real executor on scripted multi-track fakes" | header lines 3-5 | REAL | engine is shipped `Engine`; fakes are at plugin seams only |
| "event-sequence assertions: receipt → confirmation → first frame → terminal" | header lines 6-10 | SHALLOW | 3 of 4 orderings asserted; frame→terminal discarded (F-002) |
| "zero stale old-track frames after the receipt" (case 1) | header lines 22-24 | FALSE for case 1's own assertion; REAL via test 12 | F-001; lines 1882-1890 |
| "decoder instance recycling" | header lines 2-3 | REAL | decoder_cache test, baseline 12 passed |

## 7. Dead / duplicated / unreachable code
- `let _ = terminal_index;` (line 750) — computed, never used (F-002).
- `let _ = fixture;` (line 788) — dead statement (F-005).
- `frames_before_switch_len` as a whole is a vestige of an abandoned pre-switch-snapshot design.

## 8. Dependencies & flags
Test-only crate deps used: codec_api, commands, media_clock, media_types, packet_queue, playback_engine, plugin_api, testkit. No feature gates; runs in the default dev lane (baseline confirms execution). No `#[ignore]` in this file.

## 9. Open questions for the next agent
- tests/track_switch_alloc.rs (referenced lines 17-18) is NOT in my assignment — whoever owns it should confirm the counting allocator actually intercepts the real MJPEG decoder path.
- Does the `Frame::Subtitle` synthetic path (case 3b) have any non-fake producer in shipped code? Baseline mp4-smoke says decoders: mjpeg, pcm_s16le only — subtitle decode is likely unreachable from real containers; flag for tracks.rs owner.
