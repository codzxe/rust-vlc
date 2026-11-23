# AUDIT: crates/demux/src/wav/tests.rs
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-23 |
| Lines / functions | 919 / 26 (4 helpers + 22 #[test]) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — `#[cfg(test)] mod tests;` (wav/mod.rs:95-96); compiled+run by `cargo test -p demux`, all 26 executed ok in baseline |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | python3 byte-accurate reads (display channel mangles `[h`/`[m`); rg; sed quote verification; baseline citation audit/runs/baseline/cargo-test-workspace.txt:2140-2165 |

## 1. One-paragraph verdict
A genuinely hostile unit-test suite for the WAV demuxer: byte-level
hand-built fixtures (no mocks), typed error-code assertions for every
refusal path, damage-event ordering pinned (`Recovered` before the prefix
packet), pad-byte/odd-chunk arithmetic, RF64/ds64 truth, ADPCM block math
(2041-samples-per-block landing), sentinel handling, and seek/reanchor
commit semantics. All 26 tests pass (baseline :2140-2165). The one real
defect: the PTS-ladder continuation — the property two of these test names
advertise — is asserted nowhere (only first-packet pts and track span are
checked; no test pins a second packet's pts in this file, in
wav_aiff_fixtures.rs, in corpus_wav.rs, or in the mod.rs doctest — `rg -c
pts` returns 0 for both fixture files). Plus two vestigial warning-silencing
lines. This file also PINS the "misuse is sticky" convention
(`trait_misuse_is_typed_not_a_panic`, 633-664) that my AVI audit flagged as
inconsistent there (AVI's `locate()` returns not-started *non-sticky*).

## 2. What actually works (verified)
- All 26 tests executed ok: audit/runs/baseline/cargo-test-workspace.txt:2140-2165
  (`wav::tests::adpcm_packets_are_block_aligned_with_frame_accounting ... ok`
  through `wav::tests::zero_data_size_is_typed_no_data ... ok`), package
  demux unittests, 0 failures.
- Assertions are behavior-bearing (verified by reading each body): payload
  byte-equality, event-sequence equality (839-853 asserts the exact
  `[riff-size-sentinel, data-sentinel-without-ds64, EndOfStream]` vector),
  damage counters (513), seek landing ticks (743), post-reanchor pts (751),
  remaining-byte counts (712, 754).

## 3. Findings
F-001 [S3]
Where: crates/demux/src/wav/tests.rs:91-109 (and 111-140)
Code:
```
fn packets_are_bounded_and_laddered() {
    // 64 KiB + 1 frames of mono s16 → two packets, pts in frames.
    let frames = (MAX_PACKET_BYTES / 2 + 1) as usize;
    ...
    let (bytes, events) = drain(&mut wav);
    assert_eq!(bytes, samples);
```
What is wrong: the name and comment promise "two packets, pts in frames";
the body never counts packets and never looks at a single pts value — it
asserts payload equality, absence of damage, and pool size only. The
companion `pts_ladder_counts_sample_frames` (112-140) drives 300×4 = 1200
bytes — ONE packet under the 64 KiB quantum — and asserts only
`seen.first() == Some(0)` plus the track span. The ladder implementation
lives at wav/mod.rs:903-904 (`let pts_ticks = self.frames_emitted...`);
mutation reasoning: if `frames_emitted` reset to 0 per packet (or pts were
constant 0), every WAV test in the repo still passes — `rg -c pts` over
tests/wav_aiff_fixtures.rs and tests/corpus_wav.rs returns 0 matches, and
the mod.rs doctest asserts only `p.keyframe`. Mid-stream PTS for WAV is
unpinned by tests. Why it matters: A/V-sync regressions in the packet
ladder ship silently. What correct looks like: assert the second packet's
pts ticks == MAX_PACKET_BYTES/2 (32768) in the 32769-frame file.
Confidence: HIGH — Verification: read + rg (not executed; cargo-build-budget).

F-002 [S4]
Where: crates/demux/src/wav/tests.rs:407-408
Code:
```
    let mut wav2 = started_wav(minimal_pcm_wav(1, 2, 8_000, 16, &[0u8; 4]));
    let _ = &mut wav2;
```
What is wrong: dead scaffolding — a fully constructed second demuxer that
asserts nothing; `let _ = &mut wav2;` exists only to silence the unused-mut
lint. Leftover debris inside `flac_in_wav_enumerates...`. Confidence: HIGH —
Verification: read.

F-003 [S4]
Where: crates/demux/src/wav/tests.rs:493,514
Code:
```
    let mut wav = started_wav(std::sync::Arc::from(b.clone()));
```
```
    let _ = &mut b;
```
What is wrong: needless clone plus warning-silencing line — `b` is never
used after 493; both lines are lint-dodge debris. Confidence: HIGH —
Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| drain (helper) | 19-33 | REAL | — | collects bytes+events; panics on error (correct for tests) |
| started_wav (helper) | 35-40 | REAL | — | new+start with unreachable! on failure |
| wav_over (helper) | 43-60 | REAL | — | fmt+data builder, odd-fmt pad handled |
| pcm16_fmt (helper) | 62-72 | REAL | — | correct WAVEFORMATEX layout (align=ch*2, byte rate) |
| drains_plain_pcm16_whole_frames | 74-89 | REAL | — | payload, fmt facts, fmt-chunk len, fact None, damage 0 |
| packets_are_bounded_and_laddered | 91-109 | SHALLOW | S3 | F-001: no packet count, no pts |
| pts_ladder_counts_sample_frames | 111-140 | SHALLOW | S3 | F-001: single packet, first pts only |
| odd_sized_chunks_skip_their_pad_byte | 142-176 | REAL | — | JUNK 3+pad; data lands past pad |
| odd_data_payload_strips_its_pad | 178-199 | REAL | — | 5-byte payload, pad excluded |
| fact_and_info_metadata_are_parsed | 201-246 | REAL | — | fact=7, INAM→title, 28 payload bytes |
| rf64_ds64_sizes_are_the_truth | 248-286 | REAL | — | ds64 dataSize=8 honored; no >4GiB case (unit scope) |
| adpcm_packets_are_block_aligned_with_frame_accounting | 288-351 | REAL | — | strict: packets==1, bytes==2048, sliver damage==512, span 2034 |
| flac_in_wav_enumerates_with_declared_span_and_unstamped_packets | 353-409 | REAL | S4 | honest assertions; F-002 debris at 407-408 |
| flac_packets_carry_no_stamps | 411-440 | REAL | — | pts/dts Missing, duration None, verbatim payload |
| truncated_head_is_typed_not_wav_or_truncated | 442-465 | REAL | — | RIFF-prefix→truncated; wrong form→not-wav |
| truncated_mid_header_is_typed_truncation | 467-486 | REAL | — | cut at 24; sticky replay verified |
| declared_size_past_actual_is_typed_truncation_after_packets | 488-515 | REAL | S4 | damage-first ordering pinned; F-003 debris |
| zero_data_size_is_typed_no_data | 517-544 | REAL | — | no-data |
| data_sliver_below_one_frame_is_no_data | 546-577 | REAL | — | 1 byte of s16 → no-data |
| trailing_partial_frame_is_damaged_and_dropped | 579-600 | REAL | — | whole frames only; damage reason matched |
| missing_fmt_and_data_are_typed | 602-630 | REAL | — | no-fmt / no-data |
| trait_misuse_is_typed_not_a_panic | 632-664 | REAL | — | PINS sticky-misuse convention incl. start-after-misuse replay (cross-ref avi/mod.rs F-002: AVI locate() is non-sticky — convention violated there) |
| seek_maps_frames_and_reanchors | 666-713 | REAL | — | landing 500, out-of-range typed, reanchor pts 500, 2000 bytes total |
| adpcm_seek_lands_on_block_boundaries | 715-758 | REAL | — | landing 2041, packet 2048 bytes |
| seek_errors_stay_sticky_free_and_demuxer_unmoved | 760-773 | REAL | — | D4: failed seek leaves full drain |
| rf64_sentinel_without_ds64_is_a_declared_lie | 775-804 | REAL | — | bad-structure |
| plain_riff_data_sentinel_means_to_stream_end | 806-854 | REAL | — | exact event vector asserted |
| probe_recognizes_the_three_magics_and_rejects_other_forms | 856-865 | REAL | — | RIFF/RF64/BW64 accept; AVI/RIFX/short/FORM reject |
| oversized_info_value_is_damage_not_fatal | 867-908 | REAL | — | 1025-byte value skipped, damage>0, data still delivers |
| error_constructors_keep_their_codes | 910-919 | REAL | — | code-string pinning |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all except the two ladder tests | YES | byte-exact payloads, exact event vectors, error codes, counters |
| packets_are_bounded_and_laddered | NO for the ladder claim | body never inspects pts or packet count (F-001) |
| pts_ladder_counts_sample_frames | NO for continuation | single packet; only first pts (F-001) |
| error_constructors_keep_their_codes | YES weakly | pins strings only (tautology-adjacent but intentional: stable-code contract) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the AC error paths — truncated header/chunk, declared-size > actual, zero data size, unaligned chunk skip, unknown-format tag (0xFFFE without extensible body), unsupported bit depth" | tests.rs:1-7 (module doc) | PARTIAL | truncated/declared-size/zero-data/unaligned all tested here; "unknown-format tag 0xFFFE without extensible body" and "unsupported bit depth" are NOT in this file — they live in wav_aiff_fixtures.rs/corpus_wav.rs (verified by rg over this file: no 0xFFFE-refusal test here; 0xFFFE appears only as the *valid* extensible path at 358/414). Module doc overstates this file's coverage |
| "Operational misuse is sticky (the landed convention…)" | tests.rs:657-658 | REAL | 659-663 asserts start() replays not-started after misuse |

## 7. Dead / duplicated / unreachable code
F-002 (wav2), F-003 (`let _ = &mut b;`) — no other dead code; every other
helper/test is used.

## 8. Dependencies & flags
cfg(test)-gated (wav/mod.rs:95-96); uses super::{error, format,
test_support::{mem_stream, minimal_pcm_wav}}, media_types time types,
plugin_api Demuxer trait. No feature flags. Not in the shipped binary.

## 9. Open questions for the next agent
- wav_aiff_fixtures.rs / corpus_wav.rs (my next files): confirm whether the
  ffmpeg-parity suites pin per-packet pts (rg says no `pts` matches —
  likely md5/payload parity only, leaving F-001 standing repo-wide).
- wav/mod.rs (another agent): verify `frames_emitted` advance arithmetic
  (the untested ladder).
