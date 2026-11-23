# AUDIT: crates/demux/src/ivf/tests.rs
| Field | Value |
|---|---|
| Auditor | W04-A08 |
| Date | 2026-11-23 |
| Lines / functions | 553 / 24 (drain, open, Lcg×2 + 20 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — tests exercise `IvfDemuxer`; the IVF lane is wired: apps/cli/src/containers/mod.rs `("ivf") => ivf::open(stream, opts)` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p demux ivf` -> exit 0, 23 passed / 0 failed (20 in ivf::tests + 3 ivf::error::tests), 0.00s; sed quote verification |

## 1. One-paragraph verdict
This is what an adversarial unit suite should look like. It attacks the parser with real malformed bytes at every layer: cut/short headers, corrupted magic, structural lies (header_len ≠ 32, zero dims, zero rate/scale — each field-offset targeted), declared-but-absent payloads (frame-beyond-file, sticky replay checked), a 16 MiB+1 ceiling-sized frame claim with the allocation discipline pinned (`pool_bytes() == 4` proves nothing was allocated), a u64::MAX pts (i64 overflow refusal), zero-size frames, VFR gaps, trailing fragments with damage accounting, seek clamping at both ends, and three deterministic fuzz-lite sweeps (512 seeded bit-flip rounds, a full truncation sweep 0..len, 256 wild-declared-size rounds with the pool bound asserted every round). Unlike ASF (see asf/mod.rs audit), IVF's allocation ceiling is both enforced in code and pinned by tests. All 20 tests pass under my run. Only trivial hygiene nits.

## 2. What actually works (verified)
— verified by `cargo test -p demux ivf` -> `test result: ok. 23 passed; 0 failed; 0 ignored` (my run, exit 0).
— verified by read: `FRAME_CEILING_BYTES = 16 MiB` ceiling and `MAX_FRAME_BYTES = 4 MiB` pool exist in ivf/mod.rs:118-122, 457-460; the ceiling test and wild-size test assert the pool never grows toward the claim.

## 3. Findings
F-001 [S4]
Where: crates/demux/src/ivf/tests.rs:407
Code:
```
    assert_eq!(ts.ticks, 0);
    let _ = bytes;
}
```
What is wrong: Dead `let _ = bytes;` binding — `bytes` was cloned at line 366 and never used. Cosmetic. Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/demux/src/ivf/tests.rs:485-487
Code:
```
    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound.max(1)
    }
```
What is wrong: Modulo bias in the LCG bound — irrelevant for a seeded regression sweep (determinism is the goal, not uniformity), noted for completeness. Confidence: HIGH — Verification: read.

Zero S0-S3 findings. Checked and clean: header field-offset lies (12/14 dims, 16/20 rate/scale — correct IVF header layout offsets), sticky error replays for every refusal lane, seek-as-pure-query (walk position never moved, 5 packets after 4 seeks), reanchor discontinuity semantics (1 Discontinuity, tail [3,4,5]), negative-target clamping, `demux.source.empty` staged at Open not Demux (line 354-355 — the shared access seam, consistent with the AVI/WAV/Ogg/ADTS precedent), determinism double-run (round % 64 == 0 dual-demuxer code comparison).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| drain | 24-44 | REAL | — | full event vocabulary incl. Recovered/Discontinuity/terminal code |
| open | 46-50 | REAL | — | start-must helper |
| header_parse_roundtrips_the_field_set | 54-82 | REAL | — | exact ffmpeg 7.1.5 header shape; AV01/VP90/VP80 lanes |
| probe_detects_dkif_and_rejects_the_rest | 84-90 | REAL | — | |
| header_refuses_short_buffers_typed | 92-98 | REAL | — | 0/4/16/31 lengths |
| header_refuses_bad_magic_typed | 100-106 | REAL | — | |
| header_refuses_structural_lies_typed | 108-129 | REAL | — | header_len, zero dims ×2, zero rate/scale ×2 |
| clean_walk_passes_the_container_truth | 133-187 | REAL | — | pts/duration/base/data pinned; honesty pair (no keyframe, no dts) |
| walk_passes_vfr_gaps_and_zero_size_frames_through | 189-213 | REAL | — | zero-size emitted, VFR passed through, ms tick shape |
| declared_frame_count_is_advisory_the_walk_is_truth | 215-227 | REAL | — | 99 declared vs 2 real |
| trailing_fragment_is_recovered_then_clean_end | 229-242 | REAL | — | damage_total == 5 pinned |
| start_refuses_every_malformed_shape_typed | 246-276 | REAL | — | 5 shapes × sticky × next_event |
| walk_refuses_frame_beyond_file_sticky | 278-301 | REAL | — | documents ffmpeg divergence honestly (ffmpeg emits short packet; we refuse) |
| walk_refuses_ceiling_sized_frames_before_allocating | 303-318 | REAL | — | the allocation pin: pool_bytes() == 4 |
| walk_refuses_i64_impossible_pts | 320-332 | REAL | — | u64::MAX pts → pts-overflow |
| trait_calls_before_start_refuse_typed | 334-356 | REAL | — | not-started ×3 + empty-source staging |
| seek_lands_on_the_last_frame_at_or_before_the_target | 360-408 | REAL | — | 4 targets + past-end clamp + negative clamp + pure-query proof |
| reanchor_resumes_the_walk_from_the_landing | 410-449 | REAL | — | discontinuity + tail + negative reanchor |
| seek_out_of_range_only_when_no_frame_exists | 451-468 | REAL | — | degenerate container refusal |
| Lcg::next / below | 476-488 | REAL | S4 | biased bound, deterministic (fine) |
| fuzz_lite_bit_flips_never_panic_or_hang | 490-520 | REAL | — | 512 rounds × 1-8 flips + periodic determinism double-run |
| fuzz_lite_truncations_never_panic_or_hang | 522-533 | REAL | — | full 0..len truncation sweep + 4 wild edges |
| fuzz_lite_wild_declared_sizes_are_refused_not_allocated | 535-553 | REAL | — | 256 rounds, 0xFF size bytes, u64::MAX pts, pool bound asserted each round |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 20 | YES | Every refusal asserts its exact typed code; sticky replay asserted; the allocation tests would fail if the ceiling check were removed (pool_bytes would grow); the seek tests pin exact tick landings; the fuzz tests would fail on any panic/hang (debug build, overflow checks on). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "header round-trips, the frame walk, every typed error path, seek/reanchor, and the +FB fuzz-lite sweeps" | file header lines 1-3 | REAL | 20/20 pass (my run); inventory matches the claim |

## 7. Dead / duplicated / unreachable code
None (F-001's dead binding aside).

## 8. Dependencies & flags
`#[cfg(test)] mod tests` inside ivf/mod.rs:104-105 area — unit tests run only under cargo test. Uses `super::test_support` (pub mod, library-compiled — same shipped-seam pattern as every demux family). No feature gates.

## 9. Open questions for the next agent
- ivf/mod.rs owner: `FUZZ_EVENT_BUDGET` in fuzz.rs bounds the drain loop — confirm no real-file walk can exceed it silently in production paths (it is fuzz-only, so low stakes).
