# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_08_track_switching.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 250 / 1 test fn (two session arms) |
| File verdict | REAL with one unasserted pass criterion |
| Reachable from shipped entry point? | N/A (test) — plain `#[test]` + `serialize_tests`; part of W00 baseline (execution UNVERIFIED, no cargo). Rides the scenarios_b harness (my host.rs audit: real engine/AVI/MJPEG/PCM under a local dispatch copy). |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:1 |
| Commands run | Read; cross-check of the ring tuple shape (host.rs:492) and the CLI track_snapshot claim (comment 94-100) against apps/cli/src/host.rs `track_snapshot`; sed verification. No cargo (adaptation). |

## 1. One-paragraph verdict

Doc 10 §6.4 row 8 (track switching) over the committed multi-track parent: opens, asserts the honest 3-track list with exactly one selected audio, plays into the default track, switches audio mid-play, asserts the `tracks.changed` terminal carries the NEW selection, drives to `Ended`, then proves the strongest property — after the first new-track audio frame in the sink's ring, zero old-track audio frames follow (with the documented in-flight grace), while video decodes on both sides of the boundary — and a second session arm proves deselect stops audio while video continues with no error. Two defects: the row's headline pass criterion ("the new track's first frame carries the switch generation") is never asserted — the ring records no generation and the captured receipt generation is discarded with `let _ =` — and the test's own comment documents a shipped product bug (tracks.list never refreshes selection after a switch) without flagging it as a defect.

## 2. What actually works (verified)

- Determinism discipline: `serialize_tests` (33), virtual-time harness, bounded pumps everywhere (`run_until_terminal(512)` at 138 answers pumps spent).
- Boundary assertion (150-162): position of first TO_TRACK audio frame via `.must()` (fails if the new track never flowed), then a zero-count filter for FROM_TRACK audio after it — exact, and the in-flight grace is documented at 145-149 per card 3.A3.
- Video continuity both sides of the switch (164-175) — both counts required > 0.
- The `tracks.changed` terminal's payload asserted as the authoritative selection truth (107-128), including kind (`TrackChangeKind::Audio`).
- Deselect arm (186-249): fresh session (documented as the v0-honest shape — the media already ended), audio count frozen across 8 pumps after deselect (226-234), video still flowing (235-242), zero `playback.error` (243-247).

## 3. Findings

### F-001 [S3] The row-8 pass criterion "the new track's first frame carries the switch generation" is never asserted
Where: crates/playback-engine/tests/scenarios_b/scenario_08_track_switching.rs:1-3 (criteria), 82-85 (capture), 180 (discard); host.rs:492 (ring shape)
Code (1-3):
```rust
//! Scenario 8 — **track switching** (doc 10 §6.4 row 8): multi-track
//! synthetic container — the new track's first frame carries the
//! switch generation; no old-track frames after.
```
Code (82-85 and 180):
```rust
    let generation_at_switch = match &switch.result {
        Ok(CommandOutcome::Accepted { generation, .. }) => *generation,
        other => unreachable!("the switch accepts: {other:?}"),
    };
```
```rust
    let _ = generation_at_switch;
```
What is wrong: The doc-10 row-8 criterion has two clauses. The second (no old frames after) is proven exactly. The first (the new track's first frame carries the switch generation) is structurally unprovable in this harness: the sink's frame ring stores `(track, is_audio, pts_ns)` (host.rs:492) with no generation, and the receipt's generation — captured at 82-85 — is discarded at 180 with `let _ =`. If the switch minted the wrong generation, or the new frames carried the old one, this test still passes.
Why it matters: The scenario is the acceptance evidence for row 8; half its stated criterion is unchecked. The generation discipline is card 3.A3's core mechanism (the "generation floor" that makes stale frames die at the boundary) and this is the scenario that was supposed to prove it end-to-end.
What correct looks like: Extend the ring tuple with the frame's generation (the sink sees `Frame::Audio(...generation)`), assert `first_new_frame.generation == generation_at_switch` and that no post-boundary FROM_TRACK frame carries an older generation than the floor.
Confidence: HIGH — Verification: read (ring shape + assertion inventory).
### F-002 [S3] The test documents a shipped product bug as a known limitation instead of a defect
Where: crates/playback-engine/tests/scenarios_b/scenario_08_track_switching.rs:94-100
Code:
```rust
    // NOT asserted via `tracks.list`: the host's list snapshot is
    // the open-time `tracks.list-ready` mirror (engine `TracksChanged`
    // fires only at open — state.rs's `Opening` row), and neither
    // this local host nor the real CLI (apps/cli/src/host.rs
    // `track_snapshot`) refreshes it on the switch terminal in v0.
    // [PROPOSAL] CLI follow-up: refresh the snapshot from
    // `tracks.changed`; until then the terminal is the truth.
```
What is wrong: After a mid-play `tracks.set-audio-track`, the product's `tracks.list` answer keeps reporting the OPEN-time selection — stale `is_selected` for the rest of the session, in both this harness and the shipped CLI (the comment names the production file). The test works around it (asserts the terminal instead) and the comment files a [PROPOSAL], but the user-visible defect — a UI query that lies after a switch — is treated as acceptable v0 behavior, not as a bug with an owner.
Why it matters: A user who switches audio tracks and then asks for the track list gets wrong data. The scenario suite's honest disclosure is good; the product state it discloses is a defect. Filed here because this is where the tree documents it.
What correct looks like: Product fix in apps/cli host (refresh snapshot on `tracks.changed`); this test then asserts the refreshed list too.
Confidence: HIGH — Verification: read (comment names the production site; consistent with the CLI host's open-time-only snapshot I saw in my queue/clock cross-reads).
### F-003 [S4] Deselect-arm audio-stop window is a fixed 8 pumps
Where: crates/playback-engine/tests/scenarios_b/scenario_08_track_switching.rs:227-229
Code:
```rust
    for _ in 0..8 {
        host.pump_pipeline();
    }
```
What is wrong: The "no audio after deselect" proof covers exactly 8 pipeline pumps. With the multitrack budget (1 packet per limb per poll), 8 pumps is a reasonable window, but a late in-flight audio frame landing at pump 9 would evade the assertion.
Why it matters: Bounded-window proofs are the harness's own standard; the rest of the file drives to terminal states. Hygiene.
What correct looks like: Pump until Ended (or a fixed larger bound) before comparing audio counts.
Confidence: MEDIUM — Verification: read (pump arithmetic reasoning).

## 4. Per-function table

| Test | Lines | Verdict | Notes |
|---|---|---|---|
| audio_track_switch_mints_generation_with_no_old_frames_after | 32-250 | REAL | Two arms (switch + deselect); F-001 generation gap, F-002 disclosure, F-003 window. Everything else exact. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| audio_track_switch… | YES for the asserted halves | Switch never completes → `changed` assert; old frames after boundary → count assert; video stops → both-sides asserts; deselect leaks audio → count freeze assert; any playback.error → count. NO for the generation clause (F-001). |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the new track's first frame carries the switch generation" (2-3) | module doc / doc 10 row 8 | NOT TESTED | F-001. |
| "no old-track frames after" (3) | module doc / row 8 | REAL | 150-162. |
| "the video limb kept decoding through the audio switch (the unchanged kind's generation floor)" (13-14) | module doc | REAL | 164-175. |
| "A deselect (`None`) stops the audio limb while video continues; the media ends honestly" (15-16) | module doc | PARTIAL | Audio-stop + video-continue proven; "the media ends honestly" is asserted only in arm 1 (138-143), not in the deselect arm. |

## 7. Dead / duplicated / unreachable code

`generation_at_switch` (F-001's discard). Nothing else.

## 8. Dependencies & flags

Harness + commands + testkit. No feature gates. `serialize_tests` per the binary's counting allocator.

## 9. Open questions for the next agent

- W03 colleague with playback-engine session.rs: does the switch receipt's generation actually equal the post-bump pipeline generation (the unasserted claim)? A one-line assert here would close F-001.
- W13: owns F-002's product fix (track_snapshot refresh).
