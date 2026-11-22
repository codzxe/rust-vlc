# AUDIT: crates/playback-engine/tests/track_switch.rs

| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-22 |
| Lines / functions | 1891 / 12 `#[test]` + 10 fakes/helpers (SharedNow, Collector, Media, ScriptDemuxer, ScriptDecoder, CountingProvider, builders, harness fns) |
| File verdict | REAL (as a behavioral suite over the real Engine) — but every API it proves (`set_audio_track`/`set_video_track`/`set_subtitle_track`) is UNREACHABLE from both shipped binaries |
| Reachable from shipped entry point? | NO — the *engine* code under test is real and live, but no shipped host calls the audited API (proof below). The suite itself ran in the W00 baseline (no `#[ignore]`, no cfg/env gates — verified by grep, 0 hits). |
| Findings | S0:0 S1:0 S2:0 S3:4 S4:1 |
| Commands run | Read (full file, 3 chunks); Grep: `set_audio_track\|set_video_track\|set_subtitle_track` over apps/ (0 hits), repo-wide callers (8 files: session.rs + 7 test files only), `tracks.set-` in apps/cli/src/dispatch.rs (hit :199-200), `Subtitle` in apps/cli/src/pipeline_host.rs (hit :1237-1238), `#[ignore\|#[cfg(` in this file (0 hits), `#[test]` count (12); sed verification of every quoted range. No cargo in sandbox — run claims cite W00 baseline or UNVERIFIED. |

## 1. One-paragraph verdict

The card 3.A3 acceptance suite: 12 behavioral tests driving the real
`playback_engine::Engine` through scripted demux/decode fakes (the
honest seam pattern — fakes at the plugin boundaries, the engine,
queues, clock, state machine and session logic all real), with strong
oracles: exact single-terminal event equality, receipt↔generation
linkage, supersession silence, paused carve-out, decoder-cache counts
cross-checked against `EngineStats`, clock re-anchor equality. It ran
in the W00 baseline (plain `#[test]`, no gates). Two of its advertised
assertions are fake (a self-comparing count and a discarded index),
the paused/video-switch cases assert the terminal generation against
itself, and — the structural fact — the entire surface it proves has
zero shipped callers: the CLI refuses every `tracks.set-*` command
citing the very card this suite accepts, and neither `apps/desktop`
nor `apps/desktop-tauri` mentions the API. 1891 LOC proving a library
capability no shipped binary can reach.

## 2. What actually works (verified)

— 12 `#[test]` fns, none gated (grep `#[ignore]|#[cfg(` = 0 hits) → part of W00's 1751 passing (`cargo test --workspace`, baseline run log).
— Wrong-track observability is real: tracks 2/3 carry distinct audio shapes (`STEREO_48K`/`MONO_44K1`, :195-205), and case 1 (:732-738) asserts the new track's first frame carries track 3's 44.1 kHz mono u8 shape — a mis-decode or stale re-push would fail it.
— Case 1 (a)/(c) (:679-716): receipt `Accepted{generation}` must equal the predicted post-bump generation AND the terminal tuple is compared for exact equality (kind, track id, origin request) — a dropped, duplicated, or mis-origined terminal fails.
— Case 6 (:1421-1454): both mints proven via the two receipts' generations, exactly one terminal, it belongs to the newest mint, superseded track 3 never delivered a frame — a real supersession oracle.
— Case 9 (:1799-1819): per-track provider build counts pinned at 1 across A→B→A AND cross-checked against the engine's own `stats.decoders_reused == 1` / `decoders_built` — two independent oracles for the cache.
— Case 4 (:1189-1275): the F5 play-out gate — no `Ended` with the clock frozen even after the script is fully drained (`media_cursor == total_packets`, :1192-1196), then `Ended` only after the virtual clock advances past the last video pts — a genuine regression test both directions.
— Mutation reasoning (UNVERIFIED by execution, no cargo): flush the retired limb's queue incorrectly → case 10's `stale_old` (:1882-1890) fails; mint no generation on switch → case 1 (a) fails; emit a terminal for the superseded switch → case 6's `len == 1` fails; re-anchor the clock on the freeze position → case 1 (g)'s `position_ns == first_new_pts` (:779-782) fails.

## 3. Findings

### F-001 [S3] Case 1's "(e) zero old-track frames" assertion compares a number to itself — it can never fail

Where: crates/playback-engine/tests/track_switch.rs:755-763, 785-796
Code:
```rust
    let frames_after = frames
        .iter()
        .filter(|(track, _)| track == &TrackId::new(2))
        .count();
    let frames_before_receipt = frames_before_switch_len(&mut fixture);
    assert!(
        frames_after <= frames_before_receipt,
        "no fresh old-track frames after the switch (had {frames_before_receipt}, total {frames_after})"
    );
```
```rust
fn frames_before_switch_len(fixture: &mut Fixture) -> usize {
    let _ = fixture;
    // The old track's total across the whole run is compared against
    // itself (an upper bound captured post-hoc is not possible); the
    // real assertion is below via generation stamps.
```
What is wrong: `frames` is snapshotted at :720 and `frames_before_switch_len` re-snapshots the same collector at :759 with **zero polls in between** — both count track-2 frames in the identical post-switch collector, so the assert is `x <= x`, vacuously true for any engine behavior. The helper's comment admits it ("compared against itself") and points to "the real assertion ... below via generation stamps" — but no generation-stamp assertion on **old-track** frames exists anywhere in case 1; the only real stale-frame oracle is case 10's pts-based check (:1882-1890). The header (:22-24) nevertheless advertises "zero stale old-track frames after the receipt" as proven by case 1.
Why it matters: a named acceptance criterion of card 3.A3 is "proven" by a comparison that cannot fail; anyone reading case 1's asserts believes the property is pinned there. It is pinned only in case 10, by a different mechanism (pts, not the receipt boundary).
What correct looks like: snapshot the collector count at the poll that applies the receipt, then assert the track-2 count does not grow afterwards (the exact shape case 2 uses for video at :884-888); or assert every track-2 frame's pts ≤ the switch boundary, as case 10 does.
Confidence: HIGH — Verification: read (both snapshots verbatim above; no `engine.poll()` between :720 and :759).

### F-002 [S3] The ordered-story claim "frame precedes terminal" is computed and then thrown away

Where: crates/playback-engine/tests/track_switch.rs:740-751
Code:
```rust
    let terminal_index = index_of(&outputs, |output| {
        matches!(
            output,
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::TrackSelectionChanged { .. },
                ..
            })
        )
    })
    .must();
    let _ = terminal_index;
```
What is wrong: `terminal_index` is computed with `.must()` (so its *existence* is asserted) and then explicitly discarded. The doc header (:6-9) sells this suite on the **ordered story** "receipt → state-changed confirmation → first new-track frame at the sink → the `tracks.changed` terminal", and the (d) comment (:717-719) says the new track's first frame "precedes the terminal (the frame's delivery IS the completion fact)" — but the frame-before-terminal leg is never checked; the index that would anchor any such ordering discussion is dead code.
Why it matters: the ordering claim is the suite's headline property; the leg asserting it is a no-op. A regression that emits the terminal before the first new-track frame would still pass case 1.
What correct looks like: either delete the index and the claim, or gate the terminal's emission on the frame (e.g. assert the terminal only appears in an outputs batch taken after the frame was already in the collector).
Confidence: HIGH — Verification: read.

### F-003 [S3] Cases 2, 4 and 7 assert the terminal's generation against itself

Where: crates/playback-engine/tests/track_switch.rs:865-874, 1173-1177, 1513-1523
Code:
```rust
    let terms = selection_terms(&outputs);
    assert_eq!(
        terms,
        vec![(
            terms[0].0,
            MediaKind::Video,
            Some(TrackId::new(4)),
            Some(request)
        )]
    );
```
What is wrong: the expected tuple is built from `terms[0].0` (case 2), `terms0(&outputs)` (case 4, helper :1278-1280) and `terms[0].0` (case 7) — the actual terminal's own generation, so the generation component of these `assert_eq!`s cannot fail. Kind/track/origin and the exactly-one length are asserted concretely, but case 7 (paused switch) nowhere pins the terminal to its receipt's generation — unlike case 1 (:681-685) and case 8a (:1623-1632), which do it properly. Case 2's `terms[0]` would also panic-index instead of assert-fail if the terminal were missing (still a failure, but a less diagnostic one).
Why it matters: the "under the switch's own generation" property (header :8-10) is unverified in 3 of the 12 cases; a generation bookkeeping bug in the paused or video path would slip through.
What correct looks like: capture the receipt's `Accepted{generation}` first, assert against that (the case-1/case-8a pattern).
Confidence: HIGH — Verification: read.

### F-004 [S3] 1891 LOC of acceptance proof for an API no shipped binary calls

Where: crates/playback-engine/tests/track_switch.rs:1-4 (header) vs apps/cli/src/dispatch.rs:199-200
Code (this file):
```rust
//! The card 3.A3 acceptance suite: track switching mid-play (design:
//! worklog W1-17 S7) — A/V/S selection via commands, decoder instance
```
Code (apps/cli/src/dispatch.rs:199-200):
```rust
    } else if id.starts_with("tracks.set-") {
        "track switching is card 3.A3"
```
What is wrong: the suite proves card 3.A3 landed in the engine, while the shipped CLI refuses every `tracks.set-*` command citing "track switching is card 3.A3" — the refusal is a fossil (W03-A03 already flagged it on tracks.rs). Repo-wide grep: the only callers of `set_audio_track`/`set_video_track`/`set_subtitle_track` are `crates/playback-engine/src/session.rs` (definition) and 7 test files; `apps/desktop`, `apps/desktop-tauri` have zero hits. Additionally, even if a host selected subtitles, the CLI's consumer drops them (`pipeline_host.rs:1237-1238`: "Subtitle/data limbs are not wired in the first vertical. `Frame::Subtitle(_) => Ok(())`"), so case 3b's synthetic subtitle delivery is unreachable with real media from the CLI.
Why it matters: this file is the acceptance evidence for a card whose user-visible surface does not exist; the capability is library-and-tests only. Any matrix row citing track switching as shipped functionality is FALSE (claim ledger's call).
What correct looks like: either wire `tracks.set-*` in the CLI (the engine work is done — this suite proves it) or mark the rows engine-only/unshipped.
Confidence: HIGH — Verification: read + repo-wide grep (callers listed in Commands run).
Conflicts: none — consistent with W03-A03 (tracks.rs) and W13-A01 (CLI fossil refusals); not re-derived beyond the caller census.

### F-005 [S4] Decorative predicates and a second frozen clock

Where: crates/playback-engine/tests/track_switch.rs:508-511, 648-653, 843-848
Code:
```rust
    config.throttle = playback_engine::EventPolicy {
        tick: playback_engine::TimeTickPolicy::default_policy(),
        throttle: Arc::new(SharedNow(AtomicU64::new(0))),
    };
```
What is wrong: (1) the `throttle` source is a *second* `SharedNow` pinned at 0 forever, unrelated to the fixture's `now` — tick-throttling logic runs with a frozen clock across the whole suite and is never exercised meaningfully. (2) `poll_until(..., |s| s == SessionState::Playing, ..., "stay playing through the switch")` (:648-653, :843-848) is satisfied on the first check because the session was already `Playing` before the command — the "stay playing" wording suggests it verifies the switch window; it verifies nothing. The real driving is the `take_outputs` loops that follow.
Why it matters: misleading test narration; the throttle seam (a real production path, per the events.rs audit) has zero meaningful coverage here.
What correct looks like: advance the throttle clock with `now`, and either drop the decorative `poll_until` or make its predicate detect the switch window (e.g. wait for the terminal event).
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SharedNow`/`impl MonotonicSource` | 81-87 | REAL | — | virtual clock, hand-advanced; poison-free |
| `Collector`/`impl FrameConsumer` | 93-106 | REAL | — | sink seam fake; delivery-order log; poison-tolerant |
| `Media` | 112-116 | REAL | — | script + shared cursor |
| `ScriptDemuxer` (start/next_event/seek/seekability/reanchor) | 121-183 | REAL | — | honest fake: seek always refuses (`not-the-reanchor-path`), `reanchor` implements compute-then-commit keyframe landing (:161-182); all-keyframe simplification noted in comment |
| `AudioShape` consts | 188-205 | REAL | — | the wrong-track observability seam |
| `ScriptDecoder` (configure/decode/flush) | 210-273 | REAL | — | 1 frame per packet, carries packet pts+generation, honors `is_stale` (:227-229); audio shape / video / subtitle baked in |
| `CountingProvider` (+impl) | 279-351 | REAL | — | per-track build counter = recycling oracle; shapes map; no panics |
| builders (`base/at_millis/stamp_ns/track/packet/tiny_queues/tiny_budget`) | 357-430 | REAL | — | `tiny_budget`'s comment documents the salvage deadlock honestly (18 vs 12 demux events) |
| `Fixture`/`open_multi_track`/`open` | 433-523 | REAL | — | see F-005 for the second frozen clock; URI is a fake string (demuxer injected — fine at this seam) |
| `poll_until` | 529-547 | REAL | — | polls ≥1, loud timeout; predicate often pre-satisfied (F-005) |
| `frames_snapshot`/`media_cursor`/`media_packet_at` | 549-573 | REAL | — | accessors |
| `receipt_for`/`selection_terms`/`index_of` | 575-611 | REAL | — | oracles; `index_of` result discarded once (F-002) |
| test 1 `audio_switch_happy_path_is_the_full_ordered_sequence` | 626-783 | REAL (with two dead asserts) | S3 | F-001, F-002; (a)(c)(d)(f)(g) are strong oracles |
| `frames_before_switch_len` | 787-796 | STUB-as-oracle | S3 | self-comparison helper (F-001); `let _ = fixture` dead param |
| `first_packet_pts_at_or_after` | 800-809 | REAL | — | forward walk; returns 0 (vacuous) if no track-3 packet remains — unreachable with 240 steps |
| test 2 `video_switch_keeps_the_clock_and_changes_the_video_track` | 818-915 | REAL | S3 | F-003 generation self-reference; clock-never-re-anchors assert (:911-914) is real |
| test 3a `subtitle_selection_is_class_s_over_parsed_streams` | 927-1015 | REAL | — | typed refusal, unbumped generation, no state-changed — all concrete |
| test 3b `subtitle_swap_delivers_frames_without_av_interruption` | 1024-1136 | REAL | — | synthetic subtitle limb; see F-004 for unreachability with real media |
| test 4 `audio_deselect_lets_video_run_to_ended` | 1154-1276 | REAL | — | F5 both directions; `terms0` self-reference (F-003) is its only weak leg |
| `terms0` | 1278-1280 | REAL | — | helper feeding F-003 |
| test 5 `not_found_rejects_before_the_mint_with_no_state_change` | 1291-1363 | REAL | — | dead id AND wrong-kind id; pipeline-untouched check |
| test 6 `rapid_double_switch_emits_one_terminal_for_the_newest_selection` | 1374-1463 | REAL | — | strongest case; two-mint proof + silence + no intermediate frames |
| test 7 `switch_while_paused_completes_without_leaving_paused` | 1474-1545 | REAL | S3 | F-003 (generation unpinned); wire-state and carve-out asserts real |
| test 8a `switch_during_seek_...` | 1556-1650 | REAL | — | both terminals once, distinct generations, post-seek audio stamped with switch gen |
| test 8b `seek_during_switch_...` | 1655-1734 | REAL | — | both terminals once; last audio belongs to new track |
| test 9 `decoder_cache_recycles_instances_across_switches` | 1746-1820 | REAL | — | dual oracle (provider counts + engine stats) |
| test 10 `the_new_limb_fills_from_the_demuxers_position` | 1833-1891 | REAL | — | the real stale-frame oracle the file's header credits to case 1 |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| 1 audio_switch_happy_path | YES (partially) | receipt-gen linkage, exact terminal tuple, shape, drain floor, clock re-anchor all concrete — but (e) is vacuous (F-001) and the frame→terminal order unasserted (F-002) |
| 2 video_switch | YES | new-track delivery, old-track frozen count, audio continuity, clock ≠ new video pts |
| 3a subtitle_selection | YES | typed rejection result, generation unchanged, zero state-changed, exact terminal tuple |
| 3b subtitle_swap | YES | subtitle frames at sink per track, A/V count strictly grows across swap |
| 4 audio_deselect | YES | no Ended while frozen despite full drain; Ended after pump; no EndOfTrack(audio); audio count frozen; play-out gate |
| 5 not_found | YES | rejection result enum, no mint, no state-changed, frames still growing |
| 6 rapid_double_switch | YES | len==1, newest wins, origin, both mints proven, superseded never presented |
| 7 switch_while_paused | YES | state stays Paused, terminal exists, landing frame exists, no state-changed (generation unpinned — F-003) |
| 8a switch_during_seek | YES | exactly one of each terminal, distinct gens, post-seek audio = new track + switch gen |
| 8b seek_during_switch | YES | exactly one of each terminal, last audio = track 3 |
| 9 decoder_cache | YES | build counts pinned at 1, engine stats must agree |
| 10 new_limb_fills | YES | first-new pts ≥ boundary; zero old-track frames past boundary |

All 12 are ungated `#[test]` → ran in W00 baseline (1751 pass). Mutation conclusions are reasoned, not executed: UNVERIFIED (no cargo in sandbox).

## 6. Doc claims touching this file

| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "card 3.A3 acceptance suite … proven end-to-end through the real executor" | header :1-10 | REAL-UNPROVEN (engine-level) / UNREACHABLE (shipped) | real Engine + honest fakes; but zero shipped callers (F-004); W00 baseline ran the tests |
| "the real-container proof lives in tests/track_switch_fixture.rs" | header :15-17 | REAL (file exists, audited separately) | LS: tests/track_switch_fixture.rs present; its audit exists |
| "the counting-allocator recycling proof in tests/track_switch_alloc.rs" | header :17-18 | REAL-UNPROVEN | file exists; audited as my file #8 |
| "no demuxer emits subtitle tracks today" (the "honest stub") | :30-31 comment | FALSE per W03-A03 | MP4/MKV/PS/TS demuxers DO emit subtitle tracks (tracks.rs audit); case 3a's "no subtitle track in the list" is a property of the fake, not of reality |

## 7. Dead / duplicated / unreachable code

- `frames_before_switch_len` (:787-796): exists only to feed a vacuous assert (F-001); its `fixture` parameter is dead (`let _ = fixture;`).
- `terminal_index` (:740-750): computed, `.must()`-guarded, discarded (F-002).
- The audited API surface itself: unreachable from every shipped binary (F-004).

## 8. Dependencies & flags

No feature flags in this file; test-only crate deps (`codec_api`, `commands`, `media_clock`, `media_types`, `packet_queue`, `playback_engine`, `plugin_api`, `testkit`) per crates/playback-engine/Cargo.toml dev-dependencies. Runs under the default `cargo test --workspace` (W00 baseline) — no env/corpus/nightly gate.

## 9. Open questions for the next agent

- Claim ledger: which feature-matrix rows cite this suite as evidence for "track switching"? Given F-004, those rows are engine-only at best (files: feature-matrix.toml, worklog).
- Whether any host layer (wave 13/14: apps/desktop bridge, tauri dispatch) was *supposed* to expose `tracks.set-*` — the dispatch.rs fossil suggests a wiring card that never landed.
