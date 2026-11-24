# AUDIT: apps/cli/src/containers/aiff.rs
| Field | Value |
|---|---|
| Auditor | W13-A05 |
| Date | 2025-06-27 |
| Lines / functions | 451 / 2 prod fns + 5 tests + 4 test helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: `lumen` session.open → host.rs:889 `open_session` → host.rs:904 `open_source` → demux_host.rs:458 `containers::open` → containers/mod.rs:49 `"aiff" => aiff::open(stream, opts)` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `cargo test -p lumen-cli --lib containers::aiff` → 5 passed / 0 failed (own execution) |

## 1. One-paragraph verdict
78 lines of production (open + harvest_facts) doing the same honest shape as the WAV/IVF hosts: construct `AiffDemuxer`, pre-pass `start()` (FORM/COMM/SSND gate, compression vocabulary refused typed in the demuxer), harvest channels+rate as the PCM pair (every demuxer-exposed form is a `pcm_` descriptor; `extra_data: None`), clamp `--start-time` against the declared sample-frame span, validate via the demuxer's Fast seek query, wrap in `SelectDemuxer`. I ran all 5 tests — pass. They open the committed AIFF/AIFC fixtures through the REAL CLI open path and decode all four fixture flavors (s16be/s24be/sowt-s16le/raw-u8) through the real decoder provider with ffmpeg-measured frame counts (8,820/9,600/8,820/8,820), and pin the typed refusals (`demux.aiff.unsupported-compression` for hand-built `ima4` and classic 8-bit; `demux.aiff.truncated` for the committed trunc1 derivative). No fakeness, no local compression tables, no guessed lanes. Four hygiene-level findings only.

## 2. What actually works (verified)
— `cargo test -p lumen-cli --lib containers::aiff` → `test result: ok. 5 passed; 0 failed`.
— Reachability chain sed-verified (host.rs:889/904, demux_host.rs:458, containers/mod.rs:49).
— `AiffDemuxer::start()` idempotency (the test's second `start()` call is safe) verified by reading crates/demux/src/aiff/mod.rs:641-644 (`if let Some(tracks) = &self.tracks { return Ok(tracks.clone()); }`).

## 3. Findings
F-001 [S4]
Where: apps/cli/src/containers/aiff.rs:431-449
Code: (verbatim)
```
        // An in-media target: every surviving packet lands at/after it.
        let mut mid = open(path, Some(1_000));
        assert_eq!(mid.start_position_ns, 1_000_000);
```
What is wrong: the in-media half of `start_time_clamps_and_skips` asserts only "no packet before 1 ms" — never HOW MANY survive. An over-dropping skip watermark (dropping everything, or half the correct set) passes this test; contrast ivf.rs:546's exact `assert_eq!(seen, 30, "the 1 s target keeps the second half")`. Likewise the past-the-end half asserts `> 0` (L406) instead of the exact clamped value (200_000_000 ns for this fixture). What correct looks like: exact count + exact clamp value. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: apps/cli/src/containers/aiff.rs:105-121
Code: (verbatim)
```
    for track in tracks {
        // The pair the provider's `with_format` seam consumes; the
        // codec descriptor is the demuxer's own track vocabulary
```
What is wrong: same dead generality as ivf.rs F-002 — the single COMM `pcm` pair is stamped on every track in the loop; correct only because AIFF is single-track by format. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: apps/cli/src/containers/aiff.rs:407
Code: (verbatim)
```
        let _ = past.demuxer.start().unwrap_or_else(|e| unreachable!("{e}"));
```
What is wrong: the result of the second (wrapper) `start()` is discarded with `let _ =` — a filtered-empty track list would go unnoticed here (failure would only surface downstream). Harmless today (inner start is cached-idempotent), but the discard is pointless in a test whose purpose is assertion. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: apps/cli/src/containers/aiff.rs:14-16
Code: (verbatim)
```
//!   AIFC `sowt` (little-endian `pcm_s16le`/`pcm_s24le`/`pcm_s32le`),
//!   `fl32`/`fl64` (big-endian float) and `raw ` (unsigned 8-bit,
```
What is wrong: the module doc promises the `fl32`/`fl64` (and `s32`, `pcm_s32le`) lanes, but the decode differential in this file covers only 4 of the vocabulary's forms; the float/32-bit lanes have no fixture and no provider-side decode anywhere in this file (demux-side vocabulary only: crates/demux/src/aiff/tests.rs:223-226 hand-built bytes assert descriptors, not decodes). Untested ≠ broken (codecs/src/audio/pcm/mod.rs:54-55,173-175 do define F32Be/F64Be), but the family's end-to-end claim rests on 4 lanes. What correct looks like: generated float fixtures in the differential, or the doc scoped to what is pinned. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open | 51-94 | REAL | — | clamp(0, duration_us); µs→ns saturating; seek-query validation; skip watermark |
| harvest_facts | 102-128 | REAL | S4 | F-002 per-track stamping; pcm from `comm()` only |
| tests: pcm_be_fixture_opens_with_expected_facts | 211-227 | REAL | — | exact facts incl. 8,820-frame extent |
| tests: all_aiff_fixture_flavors_open_and_decode | 237-310 | REAL | — | 4 flavors, ffmpeg frame-count truths, real provider |
| tests: unsupported_aifc_compression_refuses_typed | 319-377 | REAL | — | hand-built ima4 + classic s8 → exact typed codes |
| tests: truncated_aiff_refuses_at_the_demux_stage | 382-396 | REAL | — | committed malformed fixture, exact code |
| tests: start_time_clamps_and_skips | 402-450 | REAL | S4 | F-001/F-003 (weaker asserts than the ivf twin) |
| helpers (fixture, CountingSink, open, decode_frames) | 144-204 | REAL | — | real-path helpers; sample-frame counting via `frames_in` |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| pcm_be_fixture_opens_with_expected_facts | YES | exact codec/pcm/extent asserts |
| all_aiff_fixture_flavors_open_and_decode | YES | exact decoded-frame counts vs ffmpeg truths, executed by me |
| unsupported_aifc_compression_refuses_typed | YES | exact stage+code on hand-built bytes |
| truncated_aiff_refuses_at_the_demux_stage | YES | exact code on committed bytes |
| start_time_clamps_and_skips | PARTIAL | F-001: over-dropping / wrong clamp value would pass |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every form the demuxer exposes is a PCM descriptor" | L13-17 | REAL | demuxer vocabulary test (crates/demux/src/aiff/tests.rs:223-226) + 4-lane decode differential |
| "Everything else refuses before the harvest exists ... typed `demux.aiff.unsupported-compression`" | L23-29 | REAL | ima4 + classic-s8 test, exact codes |
| "clamp `--start-time` against the declared sample-frame span" | L4-5 | REAL | clamp + i64::MAX/8 test (0 packets past end) |
| "Fast, non-indexed — the SSND extent arithmetic" (seek) | L6 | REAL-UNPROVEN | demuxer-owned; not directly asserted in this file (seekability asserted in the demux crate) |

## 7. Dead / duplicated / unreachable code
None. The structurally parallel shape with ivf.rs/wav.rs is deliberate registry uniformity, not copy-paste drift (each file's facts differ).

## 8. Dependencies & flags
demux::aiff, media-types, plugin-api, commands, crate::demux_host; tests add codec-api/playback-engine/testkit + demux::aiff::test_support. No feature gates, no unsafe, no hidden allows.

## 9. Open questions for the next agent
- Are `pcm_f32be`/`pcm_f64be`/`pcm_s32be` decodable through `HostDecoderProvider` end-to-end? Generate one float fixture and run the differential — this file's doc implies yes, nothing pins it.
- `MediaFacts::duration_us` (demux_host.rs:239-241) is the clamp's truth source for ALL container hosts — audit its span→µs conversion once, centrally.
