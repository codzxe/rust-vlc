# AUDIT: crates/playback-engine/tests/eom_playout.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 316 / 2 test fns + 10 helpers/fakes |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test) — plain `#[test]`s, no ignores; part of W00 baseline (execution UNVERIFIED, no cargo). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read; cross-check F5 history against pipeline.rs `is_drained` (2100-2107) and the CLI drain-gate comment (pipeline_host.rs:949-951); sed verification. No cargo (adaptation). |

## 1. One-paragraph verdict

The F5 regression suite (issue #16): a video-only session's end-of-media must wait for the presentation clock to pass the last frame's PTS, not fire at drain-completion. This is the exact mechanism class my clock.rs S0 finding describes — with no audio device, nothing moves the clock, so the engine gates EOM on a clock that only a host pump can advance — and this file proves the engine-side half of that fix deterministically: real Engine, video-only script media, frozen virtual clock, exact assertions both ways (frozen clock ⇒ stays Playing after full drain; clock advanced past last PTS ⇒ Ended + EndOfMedia event; already-presented short media ⇒ ends at drain). Honest history: the header cites the measured defect (mp4 corpus 0115, EOM at 0.111 s of a 4.0 s media, 2.8%) and the root cause.

## 2. What actually works (verified)

- Test 1 (239-295) reproduces the defect's exact state (script EOF + queues empty + decoder flushed, clock frozen at 0, last PTS 99 ms) and asserts `Playing` + no EndOfMedia — the pre-fix engine would fail this at line 265 (state would be `Ended`). The drain window (250-258) is bounded at 400 polls and asserts it reached Playing first (259-262), so the test cannot pass vacuously by never priming.
- The complement test 2 (298-316) guards the opposite failure: the gate must not wedge an already-presented media (one frame at PTS 0, clock at 0 ⇒ ends at drain).
- Real seams: `ScriptDemuxer`/`VideoDecoder`/`VideoProvider` implement the real plugin-api/codec-api traits; the clock is the real `MediaClock::system_monotonic` over a hand-advanced `SharedNow` — the same construction the shipped CLI uses for audio-less media.
- No sleeps; fully deterministic; both tests use bounded poll loops with loud failure.
- The F5 root-cause narrative (5-15) matches the engine code I audited: `is_drained` (pipeline.rs:2100-2107) is demux-EOF + pending-none + queues-empty, and the fix layers the clock gate on top (the file proves its presence).

## 3. Findings

### F-001 [S4] `has_end_of_media` consumes engine outputs as a side effect of a predicate
Where: crates/playback-engine/tests/eom_playout.rs:228-236
Code:
```rust
fn has_end_of_media(engine: &mut Engine) -> bool {
    engine.take_outputs().iter().any(|output| {
        matches!(
            output,
            EngineOutput::Event(event)
                if matches!(event.kind, EngineEventKind::EndOfMedia)
        )
    })
}
```
What is wrong: The "query" drains the output channel. In this file no receipts are in flight when it is called (no commands are issued after open), so nothing is lost today — but a future edit that issues a command before calling it would silently eat receipts/events, and the second call at 291-294 answers over a fresh drain (fine only because the event persists in the new batch).
Why it matters: Test-hygiene: predicates with destructive side effects are traps. Hygiene.
What correct looks like: Take outputs once into a local `Vec`, assert over it, and reuse.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Notes |
|---|---|---|---|
| `micros_base`/`at_micros` | 40-46 | REAL | µs time base. |
| `SharedNow` | 50-56 | REAL | Hand-advanced virtual time. |
| `video_track` | 60-75 | REAL | UNKNOWN span — honest (fps-extremes corpus shape). |
| `video_script` | 79-96 | REAL | 1 ms-spaced packets + EndOfStream. |
| `ScriptDemuxer` | 98-128 | REAL | Real Demuxer seam; seek refused typed. |
| `VideoDecoder`/`VideoProvider` | 132-176 | REAL | One frame per packet at packet PTS; immediate flush (mjpeg shape). |
| `Collector` | 179-193 | REAL | Frame counter (asserted nowhere — scope: state/EOM only; acceptable here, the file's subject is the EOM gate, and unlike contract_suite's dead Vec this is a used-typed counter... note: the count itself is never read; same scope note as contract_suite F-001, not re-filed at this severity since no `#[allow(dead_code)]` lie is present). |
| `open_video_frozen` | 197-226 | REAL | Throttle source separate and eternally frozen (219-222) — intentional; ticks never fire, isolating the EOM gate. |
| `has_end_of_media` | 228-236 | REAL | S4 F-001. |
| video_only_end_of_media_waits_for_the_clock_to_present_the_last_frame | 239-295 | REAL | The F5 regression; exact both-phase assertions. |
| a_short_video_media_whose_first_frames_cover_the_clock_ends_at_drain | 298-316 | REAL | The anti-wedge complement. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| video_only_end_of_media_waits… | YES | Pre-fix engine: state becomes Ended during the drain window → `assert_eq!(Playing)` at 265 fails. Over-fixed engine (never ends): the 40-poll loop after clock advance fails at 287. |
| a_short_video_media_whose_first_frames_cover… | YES | A gate that ignores already-presented media never reaches Ended in 400 polls → assert at 312 fails. |

No mocks-of-subject, no sleeps, no ignores.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "mp4 corpus 0115 … reported end-of-media at 0.111043 s — 2.8% of the declared extent" (4-7) | module doc (cites worklog 2026-10) | UNVERIFIED | The corpus file and its worklog measurement are W16/W19 territory; the mechanism claim is REAL (is_drained read at pipeline.rs:2100-2107). |
| "Audio limbs are exempt: the render-driven clock and the device-ring drain gate keep them honest" (13-15) | module doc | REAL-UNPROVEN | DrainGatedDecoder referenced at pipeline_host.rs:949-951; audio path verification is W07/W13. |
| "The regression here is engine-level and deterministic" (17) | module doc | REAL | The fixture is fully deterministic (virtual clock, no I/O). |

## 7. Dead / duplicated / unreachable code

`Collector`'s frame count is written, never read (scope note in §4; not filed — no misleading `#[allow]`).

## 8. Dependencies & flags

dev-deps: playback-engine, plugin-api, codec-api, media-types, media-clock, testkit. No feature gates.

## 9. Open questions for the next agent

- W03 colleague with pipeline.rs's EOM gate: does the gate read `clock.now()` directly or the consumer's last-presented latch? In the shipped CLI the two diverge for monotonic-master sessions (host.rs:346-358 latches to presented frames) — the gate's choice determines whether `advance`-less sessions can EVER end (W00: `advance 200` → Ended 100% suggests the pump path works).
- W13: the interactive CLI's EOM behavior for video-only media — does anything besides `advance` move the clock? (My clock.rs F-001 says no.)
