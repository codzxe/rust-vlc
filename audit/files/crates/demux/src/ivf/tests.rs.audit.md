# AUDIT: crates/demux/src/ivf/tests.rs (553 lines, whole file)
| Field | Value |
|---|---|
| Auditor | W04-A09 |
| Date | 2026-11-21 |
| Lines / functions | 553 / 24 items (20 tests + Drain/drain/open helpers + Lcg) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test code); subject `IvfDemuxer` reachable: `apps/cli/src/containers/mod.rs:55` `"ivf" => ivf::open(...)` → `containers/ivf.rs` (uses `stream_info()` at 141); external truth exists via the committed ffmpeg-generated fixtures (`crates/demux/tests/ivf_fixtures.rs`, fixtures under tests/fixtures/generated/ivf/) |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | read whole file; grep of `orchrun-test-workspace-nofailfast.log.txt` → all 20 `ivf::tests::*` ok; traced `IvfDemuxer::step` (ivf/mod.rs:440-490), `PayloadSlots::fill` (avi/riff.rs:588-673), `pool_bytes` (riff.rs:567-569, ivf/mod.rs:385), `scan_landing` (ivf/mod.rs:506-527), constants (ivf/mod.rs:111-122); sed -n on all quotes |

## 1. One-paragraph verdict
A genuinely strong unit suite: every behavior is pinned with hardcoded absolute values
(pts ladders, tick bases, durations in ns, damage totals, pool bytes, sticky error codes),
field offsets in the malformed-header tests match the real IVF layout (header_length at 6,
width/height at 12/14, rate/scale at 16/20), and the fuzz-lite sweeps are deterministic and
seeded. All 20 tests executed ok in the nofailfast baseline, and the crate's separate
fixture suite pins the demuxer bit-exactly against ffmpeg-measured truth (payload MD5, pts
ladders, packet counts), so — unlike the ADTS/ADIF lane — this format's evidence chain
reaches an external oracle. The one substantive defect is an over-promising allocation
test: `fuzz_lite_wild_declared_sizes_are_refused_not_allocated` asserts the *pool* bound as
proof of bounded allocation, but the pool observable excludes the spill path, and the
sweep's byte mutations never produce a size in the 4–16 MiB band where frames are accepted
and allocated unpooled (up to a 16 MiB fresh Vec per frame). The rest is dead scaffolding
and three coverage gaps.

## 2. What actually works (verified)
— All 20 tests ok in audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt
(grep "test ivf::tests::" → 20 hits, every one `ok`, incl. the three fuzz-lite sweeps).
— `clean_walk_passes_the_container_truth` (133-187) pins: stream_info tuple
("ivf","av1",64,48,30,1), pts [0,1,2] on base (1,30), duration 1e9/30 ns per packet,
`!keyframe` + `dts == Missing` (the IVF honesty pair), payload lengths [8,12,4], first
payload bytes `&[0..8]`, frames_demuxed 3, damage_total 0, track duration 3×tick —
absolute, mutation-sensitive.
— `walk_refuses_ceiling_sized_frames_before_allocating` (303-318): FRAME_CEILING_BYTES+1
→ `demux.ivf.frame-too-large` with pool_bytes()==4 — the OOM-refusal is observable, not
just asserted-by-error-code.
— Header-field offsets verified against the real IVF header layout: mutations at 6
(header_length), 12/14 (width/height u16), 16/20 (rate/scale u32) — all correct.
— External truth: `crates/demux/tests/ivf_fixtures.rs` documents ffmpeg 7.1.5 fixture
provenance, ffprobe-measured truth (never hand-transcribed), payload-MD5 differential, and
honest divergences (flags decode-domain, short-packet vs typed refusal, header-only
refusal, VP80 scope refusal) — the strongest evidence chain in the demux crate.

## 3. Findings

### F-001 [S3] The allocation-safety fuzz test measures a pool observable that cannot see the spill path, and never exercises the accepted 4–16 MiB band
Where: crates/demux/src/ivf/tests.rs:536-551 (assertion at 551); mechanism: crates/demux/src/avi/riff.rs:654-660
Code (tests.rs:538-541 + 549-551):
```
        seed.truncate(32 + 12 + 8); // header + one frame + ramp
        let at = 32 + (round % 4);
        seed[at] = 0xFF;
        if round % 2 == 0 {
```
```
        // Whatever the outcome, no allocation grew toward the 4 GiB
        // claim (pool is bounded by the ceiling refusal).
        assert!(demuxer.pool_bytes() <= 2 * super::MAX_FRAME_BYTES);
```
Code (riff.rs:654-660, the spill path):
```
        // Spill: one exact-size buffer, filled directly, that rides
        // with the packet (every arena pinned, or the payload over the
        // ceiling). It enters the pool for future payloads only while
        // below the arena bound — at the bound every arena is pinned
```
What is wrong: the sweep sets ONE byte of the 4-byte size field to 0xFF. `round % 4` ∈
{0,1,2,3} → the size becomes 0x000000FF (255), 0x0000FF00 (65 280), 0x00FF0000
(16 711 680), or 0xFF000000 (4 278 190 080). Only two of those are above the 16 MiB
ceiling; 255 and 65 280 are pool-served. 0x00FF0000 = 16 711 680 = 15.97 MiB — just
*under* FRAME_CEILING_BYTES (16 777 216): **accepted**, and since it exceeds the 4 MiB
slot cap, `PayloadSlots::fill` skips both pool passes and takes the spill path — a fresh
`vec![0u8; want]` of ~16 MiB per frame (riff.rs:660) that rides with the packet and is
NOT counted by `pool_bytes()` (riff.rs:567-569 sums `self.arenas` only; the spill buffer
is only pooled "while below the arena bound" — false here). So the assertion
`pool_bytes() <= 2 * MAX_FRAME_BYTES` is true by construction of the observable, not
evidence that allocation stayed bounded; the one mutation that lands in the accepted
oversized band (0x00FF0000) allocates ~16 MiB invisibly to the test. The comment "no
allocation grew toward the 4 GiB claim (pool is bounded by the ceiling refusal)"
conflates pool-resident bytes with allocated bytes.
Why it matters: the test name says "wild declared sizes are refused **not allocated**" —
for the 4–16 MiB band they are accepted AND allocated (one fresh Vec per frame, up to
16 MiB each, GC churn in a 30 fps walk = up to ~480 MiB/s of transient allocations). That
is bounded (ceiling) but unpooled and unproven; a future regression that raises the
ceiling or drops the spill cap passes this test unchanged.
What correct looks like: add a case with a deliberate size in (MAX_FRAME_BYTES,
FRAME_CEILING_BYTES] over a file that actually contains that many payload bytes, and
assert the real allocation behavior (either a documented spill with a byte budget
observable, or a typed refusal if the intent is "never accept above the pool cap").
Confidence: HIGH — Verification: read fill/pool_bytes/step + arithmetic above; no run
permitted (baseline only).

### F-002 [S4] Dead scaffolding and shallow determinism check
Where: crates/demux/src/ivf/tests.rs:407 (also 504-517)
Code:
```
    let _ = bytes;
```
What is wrong: `seek_lands_on_the_last_frame_at_or_before_the_target` clones `bytes` into
`open(bytes.clone())` at 366 and then closes with a no-op `let _ = bytes;` — dead tail.
Additionally the fuzz determinism check (504-517) compares only the first event's error
code of two fresh demuxers — a single-event sample, not the full drain it claims to guard
("two runs of the same bytes agree on the terminal code" — the *terminal* code is exactly
what is NOT compared: only start() + one next_event()).
What correct looks like: delete the tail; compare full drains (terminal code after
drain) in the determinism check.
Confidence: HIGH — read.

### F-003 [S4] Coverage gaps: reanchor-before-start, IVF past-end seek policy, pts-overflow in the scan path
Where: crates/demux/src/ivf/tests.rs:335-356 (test), 384-408 (past-end half)
Code (337-350, the before-start contract):
```
    assert_eq!(
        demuxer.next_event().err().must().context.code,
        "demux.ivf.not-started"
    );
    assert_eq!(
        demuxer
            .seek(media_types::time::TimeSpan::UNKNOWN)
```
What is wrong: (a) `trait_calls_before_start_refuse_typed` covers next_event/seek/
seekability but never `reanchor` before start (the AVI suite likewise lacks it — the
shared Demuxer contract's fourth method is unpinned here). (b) The past-end seek at
386-395 targets tick 5 with 5 frames 0..4 — exactly the end (which clamps to 4, matching
AVI's at-end behavior), but a target strictly past the end (tick 6+) is never tested,
and that is precisely where IVF's `scan_landing` clamps to the last frame
(ivf/mod.rs:520-523: any target ≥ last pts lands on it) while sibling AVI refuses typed
(`demux.avi.seek.out-of-range`, avi/mod.rs:1058-1061) — the cross-demuxer policy
divergence is unpinned on the IVF side. (c) `frame_header_at`'s `pts_overflow` (ivf/mod.rs:
429-432) fires in the seek scan too — untested in the scan path (only the walk's variant
at tests.rs:320-332).
What correct looks like: three one-liner cases in the existing tests.
Confidence: HIGH — read of both demuxers' seek paths.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Drain`/`drain` (harness) | 17-44 | REAL | — | collects packets/recovered/discontinuities/terminal; honest break-on-error |
| `open` (harness) | 46-50 | REAL | — | new+start must; panics (fails test) on bad fixture — correct for a harness |
| `header_parse_roundtrips_the_field_set` | 54-82 | REAL | — | all 8 fields + codec/descriptor/tick_base/tick_nanos hardcoded; VP9 + unsupported-VP8 arms; the "exact ffmpeg header" comment is anchored by ivf_fixtures.rs corpus |
| `probe_detects_dkif_and_rejects_the_rest` | 84-90 | REAL | — | DKIF magic unambiguous; negatives thin but adequate |
| `header_refuses_short_buffers_typed` | 92-98 | REAL | — | lens 0/4/16/31 → truncated |
| `header_refuses_bad_magic_typed` | 100-106 | REAL | — | |
| `header_refuses_structural_lies_typed` | 108-129 | REAL | — | offsets 6/12/14/16/20 all correct per IVF layout |
| `clean_walk_passes_the_container_truth` | 133-187 | REAL | — | the anchor test; absolute everything (see §2) |
| `walk_passes_vfr_gaps_and_zero_size_frames_through` | 189-213 | REAL | — | ticks [0,0,5] hardcoded; 1 ms tick at rate 1000; zero-size frame emitted |
| `declared_frame_count_is_advisory_the_walk_is_truth` | 215-227 | REAL | — | 99 declared vs 2 carried; declared_frames observable still reports 99 |
| `trailing_fragment_is_recovered_then_clean_end` | 229-242 | REAL | — | Recovered(5,"ivf: trailing fragment") + damage 5 + clean end; ffmpeg-divergence note honest |
| `start_refuses_every_malformed_shape_typed` | 246-276 | REAL | — | 5-case table + sticky replay on start and next_event |
| `walk_refuses_frame_beyond_file_sticky` | 278-301 | REAL | — | declares 500 carries 10 → frame-beyond-file sticky; divergence recorded |
| `walk_refuses_ceiling_sized_frames_before_allocating` | 303-318 | REAL | — | pool_bytes()==4 proves pre-allocation refusal |
| `walk_refuses_i64_impossible_pts` | 320-332 | REAL | — | u64::MAX pts → pts-overflow |
| `trait_calls_before_start_refuse_typed` | 334-356 | REAL | S4 | reanchor-before-start missing (F-003a) |
| `seek_lands_on_the_last_frame_at_or_before_the_target` | 360-408 | REAL | S4 | landings hardcoded; pure query leaves walk unmoved (drain 5); at-end clamp tick 5→4; negative→0; past-end policy untested (F-003b); dead `let _ = bytes;` (F-002) |
| `reanchor_resumes_the_walk_from_the_landing` | 410-449 | REAL | — | discontinuity==1 + resumed ticks [3,4,5] + negative reanchor clamps to 0 (6 packets) |
| `seek_out_of_range_only_when_no_frame_exists` | 451-468 | REAL | — | degenerate container (header+tail) → out-of-range |
| `Lcg` (harness) | 474-488 | REAL | — | deterministic seeded LCG; modulo bias irrelevant |
| `fuzz_lite_bit_flips_never_panic_or_hang` | 490-520 | REAL | S4 | 512 rounds × 1-8 flips; determinism check first-event-only (F-002) |
| `fuzz_lite_truncations_never_panic_or_hang` | 522-533 | REAL | — | every prefix + 4 wild edges |
| `fuzz_lite_wild_declared_sizes_are_refused_not_allocated` | 535-553 | REAL | S3 | pool observable blind to spill; band gap (F-001) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| clean_walk / vfr_gaps / declared_count / trailing_fragment / all typed-error tests | YES | absolute hardcoded values incl. payload bytes and damage totals |
| walk_refuses_ceiling_sized_frames… | YES for the refusal; NO for accepted-band allocation | pool_bytes()==4 pins the refusal; band gap F-001 |
| fuzz_lite_wild_declared_sizes… | NO for spill regressions | observable can't see spill (F-001) |
| fuzz_lite_bit_flips… determinism half | only for first-event divergence | single-event sample (F-002) |
| seek/reanchor tests | YES | hardcoded landings and resumed ladders |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the exact header ffmpeg 7.1.5 writes … verified by hexdump on the committed fixture" | tests.rs:56-57 | REAL | the corpus suite (tests/ivf_fixtures.rs) compares demux output to ffprobe/ffmpeg-measured truth on the committed ffmpeg-generated fixtures; unit test itself uses the helper |
| "ffmpeg's demuxer semantics — nothing is invented past the container's truth" (past-end clamp) | tests.rs:384-385 | REAL for at-end; UNPROVEN for past-end | at-end clamp pinned (tick 5→4); strictly-past-end untested (F-003b); IVF implementation clamps (scan_landing 520-523) |
| "no allocation grew toward the 4 GiB claim (pool is bounded…)" | tests.rs:549-550 | FALSE as stated | pool is bounded, but allocation in the accepted 4–16 MiB band is NOT pool — F-001 |
| "+FB fuzz-lite sweeps" (fuzz registry) | baseline INDEX #18 (35 targets, diff empty) | REAL | fuzz_demux_ivf registered (ivf/fuzz.rs:49) and exercised here |

## 7. Dead / duplicated / unreachable code
- `let _ = bytes;` (407) — dead (F-002).
- Nothing unreachable; all tests ran in baseline.

## 8. Dependencies & flags
- Uses `testkit::Must` and crate test_support (mem_stream shared with avi); no feature
  gates, no unsafe, no I/O outside memory streams. Constants imported from
  `super::{FRAME_CEILING_BYTES, HEADER_BYTES, …}`; `super::MAX_FRAME_BYTES` referenced
  directly — the two size constants (4 MiB pool slot vs 16 MiB ceiling) coexist without a
  test explaining their relationship (that gap is F-001's root).

## 9. Open questions for the next agent
1. The IVF spill band (4–16 MiB): is one fresh 16 MiB Vec per frame acceptable for the
   zero-alloc D-1 story, or should the ceiling equal the slot cap? F-001 quantifies it.
2. `scan_landing` (ivf/mod.rs:506-527) reads every frame header per seek call with no
   cache — a UI scrubbing bar calling seek per frame does O(frames) reads each; check
   whether the CLI rate-limits queries (apps/cli/src/containers/ivf.rs).
3. IVF vs AVI past-end seek policy divergence (clamp vs typed refusal) — one of the two
   "ffmpeg semantics" comments is wrong; the avi/tests.rs#part5 audit (this wave) pins
   AVI's refusal, this file leaves IVF's clamp unpinned past the end.
