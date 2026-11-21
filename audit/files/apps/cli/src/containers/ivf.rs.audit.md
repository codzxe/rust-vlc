# AUDIT: apps/cli/src/containers/ivf.rs
| Field | Value |
|---|---|
| Auditor | W13-A04 |
| Date | 2026-11-21 |
| Lines / functions | 687 / 17 |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open` → apps/cli/src/run.rs → dispatch → `demux_host::open_source` (demux_host.rs:362) → `open_source_with` probe (demux_host.rs:442-446, `ProbeRegistry::with_builtins`) → `containers::open("ivf")` (containers/mod.rs:55) → `ivf::open` ivf.rs:77) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | rg -n "ivf" containers/mod.rs (arm at :55); rg demux_host.rs `open_source`/`wire_error` (:362, :378); sed demux_host.rs 355-600 (probe→registry chain, SelectDemuxer::new sig :529, before_skip); rg "\"ivf\"" crates/media-probe/src/report.rs:84 (probe id mints "ivf"); rg ivf tests in baseline: orchrun-test-workspace-nofailfast.log.txt:2796-2812 all 8 CLI ivf tests `ok`; sed crates/demux/src/ivf/mod.rs 230-260, 340-360 (`tick_nanos`, `stream_info`, `header`); rg "start_time_us" across containers/*.rs (clamp-vs-max consistency); sed ivf.rs quotes 89-91, 145-154, 156-159 |

## 1. One-paragraph verdict
This is the cleanest container host audited so far in my set: 91 lines of real wiring (construct `IvfDemuxer`, host pre-pass `start()`, harvest facts, pure-seek validation of `--start-time`, wrap in `SelectDemuxer`) followed by 505 lines of tests that open REAL committed fixtures through the REAL `open_source` path and drive them through the REAL `HostDecoderProvider` (AV1 8-bit decodes 1:1, VP9 decodes 1:1, 10-bit refuses typed `av1.bitdepth`, past-the-end `--start-time` admits zero packets). All 8 tests ran green in the recorded baseline (`orchrun-test-workspace-nofailfast.log.txt:2796-2812`). No S0–S3 defects found; three S4-level notes (debug-only invariants, an untested flag pass-through site, a dead `Option` arm inherited from the demuxer). The MP4 hostile question does not touch this file — it is the AV1/VP9 lane.

## 2. What actually works (verified)
- Reachability: probe mints `ivf` (crates/media-probe/src/report.rs:84) and containers/mod.rs:55 dispatches to this `open` — verified by rg; the shipped `lumen open <x.ivf>` path reaches every line of the production half.
- All 8 tests green in baseline run: `test containers::ivf::tests::{crafted_shapes_refuse_at_the_demux_stage, generated_av1_lane_decodes_the_full_stream, lanes_demux_the_full_frame_count, lanes_open_with_expected_facts, non_unit_scale_timebase_is_the_containers_own_ratio, start_time_skips_packets, vp9_lane_decodes_the_full_stream, av1_lanes_decode_the_full_stream} ... ok` (audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:2796-2812).
- Decode claims in the module docs ("AV1 decodes… VP9 decodes too") are backed by those tests (30/30 AV1 frames, 25/25 VP9 frames through the real provider).
- NTSC non-unit timebase math: `tick_nanos` = scale·1e9/rate floored = 33,366,666 ns for 1001/30000 — asserted by `non_unit_scale_timebase_is_the_containers_own_ratio` and cross-checked against `crates/demux/src/ivf/mod.rs:239-241`.

## 3. Findings
### F-001 [S4]
Where: apps/cli/src/containers/ivf.rs:145-154
Code:
```rust
        debug_assert_eq!(
            track.timing.base.as_parts(),
            (info.scale, info.rate),
            "the tick base is the header's scale/rate ratio"
        );
```
What is wrong: The comment block above it (lines 138-140) calls this the "harvest's invariant (divergence would mean the track and the accessor disagree about one header — a demuxer bug, never a wiring guess)", but `debug_assert_eq!` is compiled out of release builds — and every recorded run in `audit/runs/baseline/` was executed `--release` (INDEX.md §test-suite footnote). The "pinned" invariant never executed in any recorded evidence.
Why it matters: The doc comment overstates enforcement; a future demuxer change that breaks the accessor/track agreement would ship silently in release. Impact today is nil because `lanes_open_with_expected_facts` independently asserts base and codec against ffprobe-recorded truth per lane.
What correct looks like: Either a real `assert_eq!` (cost: one comparison per open), or a comment that says "checked by the lane tests, debug builds only".
Confidence: HIGH — Verification: read + baseline profile footnote (INDEX.md line ~38-40).

### F-002 [S4]
Where: apps/cli/src/containers/ivf.rs:114-115
Code:
```rust
        opts.no_video,
        opts.no_audio,
    );
```
What is wrong: The `--no-video`/`--no-audio` flag pass-through site for the IVF lane has no test: every test helper in this file calls `open_source(..., false, false)` (lines 231, 579, 596). A mutation swapping these two bools (or passing `true, true`) inside `ivf::open` survives the entire committed ivf test set; only the shared `SelectDemuxer` internals are covered elsewhere (demux_host tests construct the wrapper directly, not through this file).
Why it matters: Wiring sites are exactly where argument-order bugs live; the file's own philosophy ("pinned by lane tests") has a hole at this site.
What correct looks like: One lane test opening an ivf fixture with `no_video: true` and asserting the track list is empty (mirrors what other container suites do for their lanes).
Confidence: HIGH — Verification: read (all `open_source`/`open` calls in the test module pass `false, false`).

### F-003 [S4]
Where: apps/cli/src/containers/ivf.rs:158
Code:
```rust
    let frame_ns = demuxer.header().and_then(|head| head.tick_nanos());
```
What is wrong: The `None` arm is dead. `tick_nanos()` (crates/demux/src/ivf/mod.rs:239-241) returns `u64::try_from(u128::from(scale) * 1_000_000_000 / u128::from(rate)).ok()` and its doc claims "`None` if the ratio overflows u64 (a hostile rate/scale pair)" — but u32::MAX · 1e9 ≈ 4.29e18 < u64::MAX ≈ 1.84e19, and rate = 0 is refused typed at parse (`tick_base` L0, demux/src/ivf/mod.rs:231-234). No u32 pair can produce `None`; after a successful `start()` on this single-video-track format, `frame_ns` is always `Some`.
Why it matters: The demuxer-side doc claim is mathematically false (flagged for the demux owner below); here it means the `Option` plumbing suggests a hostile-header path that cannot occur — a reader auditing hostile-input behavior is misled.
What correct looks like: `tick_nanos()` returning plain `u64` (demux side), or this site documenting that `None` is unreachable for validated headers.
Confidence: HIGH — Verification: read of both files + arithmetic (u32::MAX×1e9 = 4,294,967,295,000,000,000 < 18,446,744,073,709,551,615).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open` (production) | 77-123 | REAL | — | Correct wiring: µs→ns via `saturating_mul(1000)` (no overflow: input is CLI-clamped µs; saturation consistent between seek target and watermark); negative start clamped to 0 — the documented no-duration-container pattern (identical in ts.rs:117, adts.rs:73, es.rs:102/193; duration-declaring containers clamp instead). Seek pre-pass runs the SlowLinear scan once at open; skip watermark = same value as `start_position_ns`. |
| `harvest_facts` | 133-180 | REAL | S4 (F-001, F-003) | Facts taken from the demuxer's own accessors, never re-derived; `dimensions` from `stream_info()`; single-track format so the per-track loop is honest. debug_assert invariants (F-001). |
| `tests::av1_fixture` | 200-204 | REAL | — | Compile-time path join to committed codecs fixtures. |
| `tests::generated_fixture` | 209-212 | REAL | — | `testkit::fixture_path`; unreachable! on missing fixture is test-appropriate. |
| `tests::CountingSink::accept` | 217-226 | REAL | — | Counts Video frames only; audio silently ignored — correct for this video-only family. |
| `tests::open` | 230-235 | REAL | — | Drives the REAL open path (`open_source`), not a mock. |
| `tests::lanes` | 258-295 | REAL | — | 7 lanes (5 AV1 codecs-crate + AV1/VP9 generated); "measured this session, never transcribed" is unfalsifiable prose, but the numbers are asserted against real bytes. |
| `tests::lanes_open_with_expected_facts` | 309-351 | REAL | — | Would fail on wrong codec/base/dims/frame_ns/seekability/span — mutation-hard. |
| `tests::lanes_demux_the_full_frame_count` | 359-399 | REAL | — | Full packet walk; asserts pts ladder 0..N-1, per-packet duration present, keyframe never invented; fails if wrapper drops/reorders. |
| `tests::av1_lanes_decode_the_full_stream` | 410-464 | REAL | — | Real provider; tenbit lane asserts 5 typed `av1.bitdepth` refusals, 0 fabricated frames — the honest-refusal shape. |
| `tests::generated_av1_lane_decodes_the_full_stream` | 471-478 | REAL | — | 30 packets → 30 frames. |
| `tests::vp9_lane_decodes_the_full_stream` | 487-513 | REAL | — | Verifies the module doc's "VP9 decodes" claim: 25/25 frames. |
| `tests::start_time_skips_packets` | 521-563 | REAL | — | 1 s target keeps exactly 30/60; past-the-end (i64::MAX/8 µs → saturated i64::MAX ns) admits 0 — fails if watermark or seek mis-wired. |
| `tests::crafted_shapes_refuse_at_the_demux_stage` | 574-607 | REAL | — | Header-only → `demux.ivf.no-frames`; VP80 → `demux.ivf.unsupported-fourcc`; temp files cleaned (leak only on mid-test panic). |
| `tests::non_unit_scale_timebase_is_the_containers_own_ratio` | 617-651 | REAL | — | 1001/30000 pass-through verbatim; tick floor 33,366,666 ns asserted. |
| `tests::decode_all` | 657-686 | REAL | — | `let _ = decoder.decode(packet, &mut sink)?` — `let _ =` discards the Ok unit, error propagates: not a swallow. Flush before counting. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| lanes_open_with_expected_facts | YES | Asserts codec/base/dims/frame_ns/seekability against per-lane ffprobe-recorded truth; any harvest lie breaks it. |
| lanes_demux_the_full_frame_count | YES | Exact count + pts ladder + duration presence; a wrapper that drops or renumbers packets breaks it. |
| av1_lanes_decode_the_full_stream | YES | Frame counts 1:1 and typed refusal code `av1.bitdepth` asserted by string. |
| generated_av1_lane_decodes_the_full_stream | YES | 30/30 frames, coded size asserted. |
| vp9_lane_decodes_the_full_stream | YES | 25 packets then 25 frames through the real provider — an unseeded decoder that fails would panic via `decode_all`'s unreachable. |
| start_time_skips_packets | YES | Count exactly 30 and min-pts 1e9 ns; watermark off-by-one breaks either. |
| crafted_shapes_refuse_at_the_demux_stage | YES | Typed stage+code asserted for both refusals. |
| non_unit_scale_timebase_is_the_containers_own_ratio | YES | Exact base pair and floored tick ns. |
| Mutation NOT caught (F-002) | NO | Swapping `opts.no_video`/`opts.no_audio` at ivf.rs:114-115 survives every test in this file. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "AV1 decodes (rav1d-backed Av1Decoder… no blob needed)" | module docs ivf.rs:18-23 | REAL | av1_lanes_decode_the_full_stream green in baseline orchrun log:2812. |
| "VP9 decodes too (rusty_vp9-backed Vp9Decoder, card 6.M-vp9)" | module docs ivf.rs:20-23 | REAL | vp9_lane_decodes_the_full_stream green, baseline orchrun log:2801. |
| "Seek is SlowLinear (OPW-0 §5 shorthand said frame-index Fast — its own §6 unverified note)" | module docs ivf.rs:50-53 | REAL | Test asserts `SeekSupport::SlowLinear` (line 344-348); demuxer seek at ivf/mod.rs:645 is the bounded scan. |
| "No declared duration… a past-the-end target… admits no packet" | module docs ivf.rs:43-49 | REAL | start_time_skips_packets past-lane asserts 0 packets. |
| "10-bit lane refuses typed av1.bitdepth (v0 8-bit boundary)" | test docs ivf.rs:405-408 | REAL | Baseline orchrun log:2812; assertion at line 461-462. |

## 7. Dead / duplicated / unreachable code
- F-003's dead `None` arm (demuxer-side, inherited here).
- No duplication found: the harvest reads demuxer accessors instead of re-parsing the header — the anti-copy discipline the module docs promise (verified: no local fourcc→codec mapping table exists in this file; codec comes from `track.codec`/`info.codec`).

## 8. Dependencies & flags
- Uses: `demux::ivf` (crate), `media_types`, `plugin_api` (Demuxer/SourceStream), `commands::CommandError` — all workspace crates, no new deps. No feature gates in this file; nothing cfg'd out of shipped builds. Tests use `testkit`, `codec_api`, `playback_engine`, `commands::catalog` — dev-dep only.

## 9. Open questions for the next agent
- DEMUX OWNER (ivf/mod.rs): the `tick_nanos` doc's "None if the ratio overflows u64 (a hostile rate/scale pair)" is mathematically unreachable for u32 scale/rate (max product 4.29e18 < u64::MAX) — false doc claim, and the `Option` is dead API surface.
- PIPELINE HOST owner: what does the engine do with `TrackFact.frame_ns: None` (the dead arm) and `dimensions: None` — is there a downstream `unwrap` waiting for a demuxer that CAN return None?
- No test anywhere opens ivf with `--no-video` (F-002) — confirm whether other container suites have the same hole or whether one shared test covers the pattern.
