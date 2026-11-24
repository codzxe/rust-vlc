# AUDIT: apps/cli/src/containers/ivf.rs
| Field | Value |
|---|---|
| Auditor | W13-A05 |
| Date | 2025-06-27 |
| Lines / functions | 687 / 2 prod fns + 8 tests + 6 test helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: `lumen` session.open → host.rs:889 `open_session` → host.rs:904 `open_source` → demux_host.rs:458 `containers::open` → containers/mod.rs:50 `"ivf" => ivf::open(stream, opts)` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p lumen-cli --lib containers::ivf` → 8 passed / 0 failed (own execution) |

## 1. One-paragraph verdict
The production surface is 104 lines (open + harvest_facts) and does exactly what the 480-line test module proves: construct `IvfDemuxer`, run the pre-pass `start()` (DKIF gate, fourcc AV01/VP90 scope, dims, rational tick rate), harvest the fourcc-honest codec descriptor (`av1`/`vp9`), tick base and coded dims as `TrackFact`s, validate `--start-time` via the demuxer's pure seek query, wrap in `SelectDemuxer`. No local fourcc table, no invented config (`extra_data: None` — IVF is in-band), no PCM pair, span end honestly `Missing`. I ran all 8 tests: they open real committed ffmpeg fixtures through the REAL CLI open path (`open_source`), assert ffprobe-recorded truths (codec, time_base, dims, nb_frames), decode the full AV1 (4 lanes 1:1 frame-per-TU) and VP9 (25 frames) streams through the real decoder provider, pin the 10-bit typed refusal (`av1.bitdepth`, 5/5 refused, 0 fabricated frames), the header-only and VP80 refusals, the non-unit 1001/30000 timebase, and `--start-time` skip semantics (30 of 60 kept; past-the-end admits 0). Nothing here is fake. Two hygiene notes only.

## 2. What actually works (verified)
— `cargo test -p lumen-cli --lib containers::ivf` → `test result: ok. 8 passed; 0 failed` (6.46s; includes full rav1d AV1 and rusty_vp9 VP9 decodes through `HostDecoderProvider`).
— Reachability chain read + sed-verified (host.rs:889/904, demux_host.rs:458, containers/mod.rs:50).
— The AV1/VP9 decode claims in the module docs are not vapor: `av1_lanes_decode_the_full_stream` asserts 6/18/60/18 decoded frames at declared coded sizes; `vp9_lane_decodes_the_full_stream` asserts 25.

## 3. Findings
F-001 [S4]
Where: apps/cli/src/containers/ivf.rs:145-154
Code: (verbatim)
```
        debug_assert_eq!(
            track.timing.base.as_parts(),
            (info.scale, info.rate),
            "the tick base is the header's scale/rate ratio"
        );
```
What is wrong: the harvest's only internal-consistency checks (track timing base == header scale/rate; descriptor == fourcc mapping) are `debug_assert!` — compiled out of every release build, so a future demuxer/accessor divergence ships silently. Defensible for an internal invariant (both sides mint from one header) but the comment calls it "the harvest's invariant" as if enforced. What correct looks like: a release-checked `if … { return Err(...) }` or drop the enforced-invariant language. Confidence: HIGH — Verification: read.

F-002 [S4]
Where: apps/cli/src/containers/ivf.rs:160-173
Code: (verbatim)
```
    let mut facts = Vec::with_capacity(tracks.len());
    for track in tracks {
        let span_end_ticks = track.timing.span.end.timestamp().ok().map(|ts| ts.ticks);
        facts.push(TrackFact {
```
What is wrong: the loop stamps EVERY track with the single video `dimensions`/`frame_ns` harvested from `stream_info()`/header — correct only because IVF is a single-video-track format (the demuxer enforces it). Dead generality that would mislabel a hypothetical multi-track file; harmless today. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open | 77-123 | REAL | — | pre-pass, seek-query validation (µs→ns saturating), skip watermark, SelectDemuxer wrap; negative start clamped to 0 |
| harvest_facts | 133-180 | REAL | S4 | F-001 debug-only invariants; F-002 per-track stamping |
| tests: lanes_open_with_expected_facts | 308-351 | REAL | — | 7 fixtures, ffprobe truths incl. SlowLinear seekability |
| tests: lanes_demux_the_full_frame_count | 358-399 | REAL | — | pts ladder 0..N−1, one-tick duration, no invented keyframes |
| tests: av1_lanes_decode_the_full_stream | 409-464 | REAL | — | 1:1 decode + tenbit typed refusal pinned by error code |
| tests: generated_av1_lane_decodes_the_full_stream | 470-478 | REAL | — | 30 frames end-to-end |
| tests: vp9_lane_decodes_the_full_stream | 486-513 | REAL | — | 25 packets → 25 frames |
| tests: start_time_skips_packets | 520-563 | REAL | — | mid-target keeps 30; i64::MAX/8 admits 0 |
| tests: crafted_shapes_refuse_at_the_demux_stage | 573-607 | REAL | — | header-only → demux.ivf.no-frames; VP80 → unsupported-fourcc |
| tests: non_unit_scale_timebase_is_the_containers_own_ratio | 616-651 | REAL | — | 1001/30000 verbatim; frame_ns 33,366,666 floored |
| helpers (av1_fixture, generated_fixture, CountingSink, open, Truth/lanes, decode_all) | 200-295, 657-686 | REAL | — | real-path helpers; temp files cleaned up |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 8 | YES | executed by me; each asserts exact counts/codes against real committed bytes through the real open path; mutations (tenbit, VP80, header-only, NTSC ratio) pin typed refusals and passthrough arithmetic — none are tautologies |
Weakest link: `lanes_open_with_expected_facts` trusts the committed lane table's ffprobe numbers (recorded "this session" by the author, not re-derived by me) — UNVERIFIED against fresh ffprobe (no ffprobe run in sandbox).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "AV1 decodes (rav1d-backed Av1Decoder)" | module docs L18-19 | REAL | test run: 6/18/60/18+30 frames decoded |
| "VP9 decodes too (rusty_vp9-backed Vp9Decoder)" | L20-23 | REAL | test run: 25 frames |
| "No extra_data ... in-band config ... None is the wiring" | L24-31 | REAL | lanes assert `extra_data.is_none()` |
| "Seek is SlowLinear (OPW-0 §5 shorthand said Fast)" | L50-53 | REAL | `seekability() == SeekSupport::SlowLinear` asserted per lane |
| "a past-the-end target lands on the last frame and the skip watermark then admits no packet" | L47-49 | REAL | start_time_skips_packets: 0 packets at i64::MAX/8 |
| "header-only refuses demux.ivf.no-frames (ffmpeg opens it — the recorded divergence)" | L567-571 | REAL-UNPROVEN | refusal asserted by me; the ffmpeg-divergence half is the author's recording, not re-measured |

## 7. Dead / duplicated / unreachable code
None. Production path is live via the container registry.

## 8. Dependencies & flags
demux::ivf, media-types, plugin-api, commands, crate::demux_host; tests add codecs/playback-engine/codec-api/testkit/demux test_support. No feature gates, no unsafe, no allow-attr hiding.

## 9. Open questions for the next agent
- `SelectDemuxer` (demux_host.rs) owns the skip watermark semantics this file trusts — its unit handling (ns) vs the track bases (ticks) deserves its own audit.
- The lane table's ffprobe numbers were recorded by the author; a fresh ffprobe diff over `crates/codecs/tests/fixtures/av1` + `generated/ivf` would close the one UNVERIFIED gap.
