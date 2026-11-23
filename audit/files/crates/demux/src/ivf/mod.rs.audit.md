# AUDIT: crates/demux/src/ivf/mod.rs
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-23 |
| Lines / functions | 703 / 25 (incl. `Demuxer` impl) |
| File verdict | REAL (cleanest file of my assignment; perf/observability nits only) |
| Reachable from shipped entry point? | YES: `lumen` CLI → `apps/cli/src/containers/mod.rs:54 "ivf" => ivf::open` → `apps/cli/src/containers/ivf.rs:82 IvfDemuxer::new(stream)` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 4 |
| Commands run | `cargo test -p demux --lib ivf` → 23 passed; `cargo test -p demux --test ivf_fixtures` → 5 passed (ffmpeg-differential corpus) |

## 1. One-paragraph verdict
The IVF demuxer is genuinely implemented and the hostile focus ("frame header trust") is handled correctly: the 32-byte header is validated field-by-field (magic, exact header length, nonzero dims/rate/scale), every declared frame size is ceiling-checked (16 MiB) *before* any allocation, an overrun past EOS is a typed refusal, and a pts ≥ 2^63 is a typed `pts-overflow` rather than a reinterpretation — all five refusal paths have dedicated tests, plus fuzz-lite bit-flip/truncation/wild-size suites and a bit-exact ffmpeg 7.1.5 differential over committed fixtures (payload MD5 + pts ladder). The walk trusts exactly what the format guarantees (positional frames, no resync) and nothing more; the declared frame count is advisory; keyframe=false and dts=Missing are honest container-truth decisions with recorded ffprobe divergences. Remaining defects are performance-shaped: every seek/reanchor is a from-scratch linear scan doing one 12-byte heap-allocating read per frame (seek-then-reanchor scans twice), and frames above the 4 MiB pool cap allocate unpooled.

## 2. What actually works (verified)
- `cargo test -p demux --lib ivf` → `test result: ok. 23 passed; 0 failed` — header round-trip/refusals, clean walk container-truth, VFR gaps + zero-size frames passed through, advisory frame count, trailing fragment recovery, sticky truncation, ceiling-before-alloc, seek/reanchor landings, fuzz-lite.
- `cargo test -p demux --test ivf_fixtures` → `5 passed` — stream params vs ffprobe, packet-count truth, pts-ladder + payload digest vs `ffmpeg -c copy -f md5 -`, truncated derivatives typed, fuzz over the real ffmpeg bytes.
- Header field offsets audited against the IVF/DKIF layout (version@4 u16, hdrlen@6, fourcc@8, w@12, h@14, rate@16 u32, scale@20, nframes@24): correct; little-endian throughout, correct.

## 3. Findings
F-001 [S3]
Where: crates/demux/src/ivf/mod.rs:506-527 and 420-434
Code:
```
        while at + FRAME_HEADER_BYTES <= len {
            let (size, pts) = self.frame_header_at(at)?;
```
with `frame_header_at` doing `self.reader.read_prefix_at(12, at)` (a fresh 12-byte `Vec` per frame, riff.rs:307-311) and `seek` (645-667) + `reanchor` (677-702) each calling `scan_landing`.
What is wrong: every seek is a full from-position-0 linear scan, and every frame in that scan costs one heap allocation + one seek + one 12-byte read on the source. For a 100k-frame IVF (~55 min at 30 fps) that is ~100k allocations and ~200k source calls per seek query — and the engine's seek-then-reanchor flow pays it twice. `SlowLinear` is the documented honesty class, but the *implementation* is needlessly linear-in-syscalls: the AVI idx1 parser in the same crate batches 256 entries (4 KiB) per read into a reused buffer (index.rs:352-363); the same batching would cut this by ~300×. Why it matters: interactive seeking on long IVF files is O(frames) small-IO per query; users feel it. What correct looks like: read the scan in 4-64 KiB blocks into a reused buffer (the AVI/ADTS pattern), or cache the frame-offset table after the first walk. Confidence: HIGH — Verification: read (call sites cited; per-call allocation cited at riff.rs:308).

F-002 [S4]
Where: crates/demux/src/ivf/mod.rs:457-461 vs 118 (pool cap)
Code:
```
        if u64::from(size) > FRAME_CEILING_BYTES {
            // Declared size above every sane delivery frame: the MP
            // §12 ceiling refuses it before an allocation is trusted.
            return Err(error::frame_too_large(FRAME_CEILING_BYTES, u64::from(size)));
        }
```
What is wrong: nothing in the refusal itself (correct and tested); but `MAX_FRAME_BYTES` (the pool slot cap, line 118) is 4 MiB while the accepted ceiling is 16 MiB — legal frames in (4 MiB, 16 MiB] (big 4K/8K keyframes, VP9 lossless) take the unpooled spill path: one exact-size heap allocation per packet, never pooled. Unlike ADTS, the module doc is honest about the boundary ("Payload sizes up to this are served from the packet pool"), so this is a perf note, not a doc lie. Confidence: HIGH — Verification: read (PayloadSlots::new(2, MAX_FRAME_BYTES) at 338; spill path riff.rs:654-670 from the ADTS audit).

F-003 [S4]
Where: crates/demux/src/ivf/mod.rs:473-476
Code:
```
        let track = self
            .track
            .clone()
            .ok_or_else(|| self.fail(error::not_started()))?;
```
What is wrong: per-frame clone of the whole `Track`, whose `codec` is `CodecDescriptor(String)` (media-types/src/ids.rs:573) — a heap allocation per frame in the hot path. Same pattern as the ADTS demuxer; the cached `TrackId` + `TimeBase` would do. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/demux/src/ivf/mod.rs:483 and 697
Code:
```
        self.last_pts = self.last_pts.max(pts);
```
```
        self.last_pts = pts;
```
What is wrong: `reanchor` assigns the landing pts directly, so after a backward reanchor the `track_duration()` observable regresses (reports a smaller duration) until the walk re-emits past the old max — `last_pts` is "max emitted" in the walk but "landing pts" after reanchor; two meanings, one field. Also `seek()`'s `self.reader.seek_to(save)?` (line 664) propagates before `result?` (line 665), discarding the original scan error if the restore fails — same ordering nit as ADTS. Both observability-level. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/demux/src/ivf/mod.rs:460-468
Code:
```
            return Err(error::frame_too_large(FRAME_CEILING_BYTES, u64::from(size)));
        }
        let payload_at = self.walk + FRAME_HEADER_BYTES;
        let payload_end = payload_at + u64::from(size);
        if payload_end > len {
            // Declared frame overruns the stream: typed sticky
```
What is wrong: the doc calls these "sticky" (module docs 64-65), but neither error is recorded through `fail()`/`StickyShape` in `next_event`'s `self.step()?` path — stickiness is only behavioral (the deterministic walk re-derives the same error each call, which the test `walk_refuses_frame_beyond_file_sticky` pins). Functionally equivalent, but the invariant lives in walk determinism rather than the state the type system was built for; a future step that advances `walk` before failing would silently un-stick it. Confidence: MEDIUM — Verification: read + test assertion in tests.rs:279.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| probe | 133-135 | REAL | — | DKIF magic |
| IvfHeader::parse | 175-203 | REAL | — | field offsets verified; typed refusals all tested |
| IvfHeader::codec / descriptor | 209-224 | REAL | — | AV01/VP90 only, refusal carries fourcc |
| IvfHeader::tick_base | 232-234 | REAL | — | scale/rate semantics correct (ffprobe-verified in fixtures test) |
| IvfHeader::tick_nanos | 239-241 | REAL | — | u128 math, overflow → None |
| StickyShape of/mint | 278-289 | REAL | — | |
| IvfDemuxer::new | 325-341 | REAL | — | |
| header/stream_info/frames_demuxed/damage_total/pool_bytes/track_duration | 344-408 | REAL | S4 | track_duration reanchor regression (F-004) |
| fail | 412-415 | REAL | — | |
| frame_header_at | 420-434 | REAL | — | pts ≥ 2^63 typed refusal; callers guarantee the 12 bytes |
| step | 438-498 | REAL | S4 | ceiling-before-alloc correct; per-frame Track clone (F-003); spill >4 MiB (F-002) |
| scan_landing | 506-527 | REAL | S3 | per-frame alloc/read scan (F-001); trusts size for advance (documented, consistent with walk) |
| target_ticks | 531-534 | REAL | — | |
| landing_span | 538-552 | REAL | — | |
| Demuxer::start | 556-616 | REAL | — | header-only file → typed no_frames (documented divergence) |
| next_event | 618-643 | REAL | — | |
| seek | 645-667 | REAL | S4 | restore-before-result (F-004) |
| seekability | 669-675 | REAL | — | SlowLinear, honest |
| reanchor | 677-702 | REAL | S4 | commit-after-success correct (D4); last_pts regression (F-004) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| header_parse_roundtrips_the_field_set | YES | every field asserted |
| start_refuses_every_malformed_shape_typed | YES | exact codes per malformation |
| clean_walk_passes_the_container_truth | YES | pts ladder + payload bytes |
| walk_passes_vfr_gaps_and_zero_size_frames_through | YES | gaps not smoothed |
| walk_refuses_ceiling_sized_frames_before_allocating | YES | 16 MiB+1 refused, no alloc |
| walk_refuses_frame_beyond_file_sticky | YES | error + replay asserted |
| seek_lands_on_the_last_frame_at_or_before_the_target | YES | at-or-before + clamp-to-first |
| reanchor_resumes_the_walk_from_the_landing | YES | pts continuity post-reanchor |
| fuzz_lite (bit-flips/truncations/wild sizes) | NO (by design) | absence-of-panic/hang, documented harness contract |
| ivf_fixtures (5) | YES | bit-exact vs recorded ffmpeg 7.1.5 truth (MD5 + pts + counts) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ffprobe reads the generated 30 fps fixture back as time_base = 1/30" | mod.rs:22-23 | REAL | fixtures test stream_parameters_match_ffprobe passed (recorded truth; I did not re-run ffprobe — UNVERIFIED live, evidence is the committed differential) |
| "ffmpeg reports duration = 1 on every packet" | mod.rs:29-30 | REAL-UNPROVEN | same recorded-differential basis |
| "a size above the payload ceiling is the typed ResourceLimit refusal before any allocation is trusted" | mod.rs:66-68 | REAL | code 457-461 + test walk_refuses_ceiling_sized_frames |
| "the scan never reads payload bytes" | mod.rs:74-75 | REAL | scan_landing reads 12-byte headers only |
| "every packet carries keyframe = false — never invented" | mod.rs:49-51 | REAL | line 491 + recorded ffprobe divergence note (fixtures.rs:47-49) |
| "the declared frame count is advisory … never trusted to bound reads" | mod.rs:86-89 | REAL | frame_count only exposed in IvfStreamInfo; walk unbounded by it |

## 7. Dead / duplicated / unreachable code
- `StickyShape` is a byte-for-byte copy of the ADTS/WAV/Ogg pattern (third duplicate across my assignment alone) — the promised R3 consolidation (per adts/error.rs:5-7) is owed.
- `pub mod fuzz` / `pub mod test_support` un-gated — shipped in release like ADTS (same S4 class, F-009 there).

## 8. Dependencies & flags
- No external deps; reuses `avi::riff::{PayloadSlots, SourceReader}` and `avi::header::DamageLog` (same cross-family coupling as ADTS). No feature gates.

## 9. Open questions for the next agent
- F-001: is any engine-side seek cache planned (playback-engine session) that would amortize the per-seek scan, or should the demuxer cache the offset table after first walk?
- The vp9/av1 differential tests in codecs (vp9_differential.rs, av1_differential.rs) consume this demuxer — a codec-lane agent should confirm the payload passthrough is still bit-exact there.
