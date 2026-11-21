# AUDIT: crates/demux/src/wav/tests.rs#part1 (lines 1–466)
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-21 |
| Lines / functions | 466 audited (1-466: 4 helpers + 11 tests; the nominal 459-line chunk boundary cuts `truncated_head_is_typed_not_wav_or_truncated` (:442-465) mid-body, so I took that whole test — part2/A10 starts at :467 with `truncated_mid_header_is_typed_truncation`) / 15 items |
| File verdict | REAL (chunk) |
| Reachable from shipped entry point? | The tests exercise `WavDemuxer` — reachable: `lumen open` → probe `wav` (media-probe signature) → `apps/cli/src/containers/mod.rs:73` `"wav" => wav::open`. Executed: all 11 part1 tests pass in baseline (orchrun-test-workspace-nofailfast.log.txt:1927-1952 — 26/26 `wav::tests` incl. part2's; plus `containers::wav::tests` 4+ lanes at :2838-2841 incl. `corpus_wav_lanes_decode_through_the_open_path`); the WAV fixture differential (`tests/wav_aiff_fixtures.rs`) and corpus suite (`tests/corpus_wav.rs`) also ran. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Full read :1-466 · pts-assertion sweep over the whole wav family: `rg -n "pts\|ticks" crates/demux/src/wav/tests.rs crates/demux/tests/wav_aiff_fixtures.rs crates/demux/tests/corpus_wav.rs` (every hit is first-packet, post-seek-first-packet, or span-end — no sequential second-packet pts assert anywhere; basis of F-001) · Read `corpus_wav.rs:225-265` (span/payload/frames asserts — confirms the sweep) · `rg -n "wav::tests::" orchrun…log.txt` (26/26 executed) · `rg "wav/tests" audit/waves/wave-00/assignments.tsv` (part1=459/part2=460 — boundary note above). |

## 1. One-paragraph verdict
Part 1 is a strong hand-built unit suite: every test constructs a byte-exact RIFF/RF64 container by hand (including pad bytes, ds64 tables, extensible-format FLAC GUIDs, IMA-ADPCM block tails) and asserts exact payload bytes, exact event sequences, typed error codes, and observables (`pool_bytes`, `damage_total`, `frames()`, span ends) — the pad-byte, RF64-sentinel, ADPCM-block-accounting, and FLAC-unstamped tests would each fail under the specific mutation they target, and all executed in the baseline. Two honesty defects: the suite's one systematic hole is the **sequential multi-packet pts ladder** — every pts assertion in the entire wav family (unit, fixture differential, corpus) checks only a first packet (0, or a post-seek landing) or the track span end, so a walk that stamps the landing/zero value on the first packet and never advances pts per packet survives the whole suite, despite a test literally named `packets_are_bounded_and_laddered` that asserts boundedness (the pool quantum) and no laddering at all; and one test carries a dead two-line leftover (`wav2` constructed and discarded, asserting nothing).

## 2. What actually works (verified)
— All 11 tests executed, 0 failed (orchrun :1927-1952). Mutation reasoning per test in §5.
— Hand-built containers are byte-exact RIFF: `wav_over` even inserts the odd-fmt pad byte (:51-53) and patches RIFF size last; `rf64_ds64_sizes_are_the_truth` builds a real ds64 table with the 0xFFFFFFFF data sentinel and patches `ds64.riffSize` at the right offset (:274).
— The D-1 pool observable is pinned, not just claimed: `packets_are_bounded_and_laddered` asserts `pool_bytes() == MAX_PACKET_BYTES` after a 64 KiB+1-frame drain (:108) — a pool that over- or under-grew fails.
— The ADPCM test is the suite's best: block geometry (1024 B × 2ch, 1017 samples/block), codec enum shape, `frames() == 2×1017`, exactly one 2 048-byte packet, first tick 0, the 512-byte sliver surfaced as `Recovered { skipped: 512, reason: "data-not-frame-aligned" }`, and span end 2 034 — six independent observables that all must agree.
— FLAC-in-WAV honesty pair: `flac_in_wav_enumerates…` (span rides fact: 96 000; seekability None; payload verbatim) + `flac_packets_carry_no_stamps` (pts/dts Missing, duration None) — the "the frame grammar is not ours" posture is asserted, not assumed.

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/wav/tests.rs:128-130 (with :92-94, :106-108)
Code:
```
    // 1200 bytes = 1 packet ≤ 64 KiB — but assert the ladder start
    // and the track span independently.
    assert_eq!(seen.first().copied(), Some(0));
```
What is wrong: The per-packet pts ladder of a multi-packet sequential walk is never asserted anywhere in the wav family. `packets_are_bounded_and_laddered` (:92) builds the one fixture that produces two packets (64 KiB+1 frames) and asserts payload parity, damage-free events, and the pool quantum — zero pts assertions. `pts_ladder_counts_sample_frames` uses 300 frames = 1 200 bytes = a single packet (the comment admits it) and asserts only the first tick and the span end. Every other pts assert in the family is a first post-seek packet (part2: ticks 500/:680, 2 041/:743) or a span end (fixtures :241/:307; corpus :236-243). Verified by exhaustive rg over `wav/tests.rs`, `wav_aiff_fixtures.rs`, `corpus_wav.rs`: no test reads the pts of a second sequentially-delivered packet.
Why it matters: A mutation of the walk's stamp accumulation — e.g. every packet stamped with the session-start/landing ticks instead of cumulative sample frames, or pts advanced by packet count instead of frames-per-packet — passes the entire executed wav suite (unit + differential + corpus) while breaking the demuxer's timing contract for every WAV longer than 64 KiB (the common case). The test named "…and_laddered" actively suggests the property is covered; it is not. (The AVI lane's W5-14 ladders are separately tested — this gap is wav-specific.)
What correct looks like: In `packets_are_bounded_and_laddered`, collect each packet's pts and assert the exact two-element ladder (0, then 32 768 — MAX_PACKET_BYTES/2 frames); one fixture-differential lane asserting the pts sequence for a >64 KiB corpus file closes the rest.
Confidence: HIGH — Verification: read + exhaustive rg sweep of the family's pts/ticks assertions; baseline execution cited.

### F-002 [S4]
Where: crates/demux/src/wav/tests.rs:407-408
Code:
```
    let mut wav2 = started_wav(minimal_pcm_wav(1, 2, 8_000, 16, &[0u8; 4]));
    let _ = &mut wav2;
```
What is wrong: Dead scaffolding at the end of `flac_in_wav_enumerates_with_declared_span_and_unstamped_packets`: a second demuxer is constructed, started, and discarded with no assertion touching it — two lines that look like a control case but verify nothing.
Why it matters: Test corpses mislead the next reader (what was wav2 for? a removed contrast assert?) and inflate the suite's apparent coverage; this file's own standard is that every line earns its assert.
What correct looks like: Delete the two lines, or restore whatever contrast the `wav2` was built to assert (e.g. that a PCM wav's packets DO carry stamps — the natural foil for the FLAC unstamped case).
Confidence: HIGH — Verification: read; no reference to `wav2` after construction.

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (suite scope; fixtures/corpus pointers) | 1-7 | REAL | — | Accurate: the differential/corpus suites exist and ran in baseline. |
| `drain` helper | 19-33 | REAL | — | `unreachable!` on terminal errors — usable only for clean walks; the error-path tests correctly use bespoke loops (part2). No infinite-loop risk (Err panics). |
| `started_wav` / `wav_over` / `pcm16_fmt` helpers | 35-72 | REAL | — | Byte-exact builders; `wav_over` handles the odd-fmt pad; sizes patched last. |
| `drains_plain_pcm16_whole_frames` | 74-89 | REAL | — | Payload equality + events + fmt facts + descriptor + fmt-chunk len + fact None + damage 0. Strong. |
| `packets_are_bounded_and_laddered` | 91-109 | REAL | S3 | F-001 — name promises the ladder, body asserts none; does pin the pool quantum (:108). |
| `pts_ladder_counts_sample_frames` | 111-140 | REAL | S3 | F-001 — single-packet fixture; ladder start + span only (admitted :128-129). Base 1/8000 and span 300 pinned. |
| `odd_sized_chunks_skip_their_pad_byte` | 142-176 | REAL | — | Hand-built JUNK(3)+pad; exact payload; would fail on pad miscounting. |
| `odd_data_payload_strips_its_pad` | 178-199 | REAL | — | Odd data payload; the pad must not leak (ffmpeg MD5 parity note). Strong. |
| `fact_and_info_metadata_are_parsed` | 201-246 | REAL | — | fact=7, one INFO pair, INAM id bytes, track title surfaces, payload delivered. Strong. |
| `rf64_ds64_sizes_are_the_truth` | 248-286 | REAL | — | RF64 + ds64 + sentinel; is_rf64_like, descriptor, exact payload, damage-free. Strong. |
| `adpcm_packets_are_block_aligned_with_frame_accounting` | 288-351 | REAL | — | Six agreeing observables incl. the Recovered sliver — the suite's strongest test. |
| `flac_in_wav_enumerates_with_declared_span_and_unstamped_packets` | 353-409 | REAL | S4 | F-002 (dead wav2 tail). Main asserts strong (span 96 000 from fact, seekability None, verbatim payload). |
| `flac_packets_carry_no_stamps` | 411-440 | REAL | — | pts/dts Missing + duration None + payload verbatim. Strong. |
| `truncated_head_is_typed_not_wav_or_truncated` | 442-465 | REAL | — | 4-byte cut → `demux.wav.truncated`; `RIFF…AVIX` → `demux.wav.not-wav`. Exact-code asserts. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `drains_plain_pcm16_whole_frames` | YES | Exact byte equality kills any framing/pooling mutation; descriptor and fmt facts pinned. |
| `packets_are_bounded_and_laddered` | PARTIALLY | Fails for pooling/boundedness mutations (pool_bytes assert) and payload corruption — but NOT for ladder (pts) mutations: no pts assert (F-001). |
| `pts_ladder_counts_sample_frames` | PARTIALLY | Fails for span-end/base mutations; single packet means no ladder coverage (F-001). |
| `odd_sized_chunks_skip_their_pad_byte` | YES | The data header must be found past the pad — exact payload equality. |
| `odd_data_payload_strips_its_pad` | YES | 5 declared bytes, pad excluded — byte-exact. |
| `fact_and_info_metadata_are_parsed` | YES | fact value, INFO entry id, title surfacing, payload count — four independent asserts. |
| `rf64_ds64_sizes_are_the_truth` | YES | Sentinel replaced by ds64 truth; exact payload; would fail if the sentinel were trusted or ds64 ignored. |
| `adpcm_packets_are_block_aligned_with_frame_accounting` | YES | Geometry + count + bytes + first tick + Recovered(512, exact reason) + span 2034 — a block-misalignment mutation breaks ≥2 asserts. |
| `flac_in_wav_enumerates…` | YES (main asserts) | Span-from-fact, seekability refusal, verbatim payload; the dead wav2 tail asserts nothing (F-002). |
| `flac_packets_carry_no_stamps` | YES | Any stamp/duration invention fails the Missing/None asserts. |
| `truncated_head_is_typed_not_wav_or_truncated` | YES | Exact error-code discrimination (truncated vs not-wav). |
All 11 executed, 0 failed (orchrun :1927-1952).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "framing, oddities, and the AC error paths" (suite scope) | module docs :1-4 | REAL | Part1 covers framing/oddities; the AC error paths are part2's (`truncated_mid_header`, `declared_size_past_actual`, `zero_data_size`…) — all executed. |
| "pts in frames" (packet stamping) | test name/comment :93 | REAL-UNPROVEN (tests) | The property is implemented (span-end math agrees) but no test pins the per-packet ladder beyond packet 1 — F-001. |
| "the ffmpeg MD5 parity depends on this" (pad stripping) | :182-184 | REAL | `odd_data_payload_strips_its_pad` + the fixture differential's md5 lanes (executed in baseline, `wav_aiff_fixtures.rs` ran — see orchrun demux test block). |

## 7. Dead / duplicated / unreachable code
- F-002's `wav2` corpse (:407-408) — the only dead code in the chunk.
- No test-only production leakage: helpers are all `#[cfg(test)]`-scoped by the module's location inside `tests.rs` (the file itself is `#[cfg(test)]`-gated at declaration, wav/mod.rs — verified by the module outline in my riff/ivf reads of sibling structure; wav/test_support.rs is the production-compiled support module, audited separately).

## 8. Dependencies & flags
- `super::{MAX_PACKET_BYTES, WavDemuxer}`, `super::format::WavCodec`, `super::test_support::{mem_stream, minimal_pcm_wav}`, `super::error` (part2 uses it) — no feature gates; the suite compiles and runs in the default test profile (baseline executed it).

## 9. Open questions for the next agent (A10 — part2, :467-919)
- F-001's fix belongs in part1's range but the seek tests you audit (:666-773) are the natural place to confirm the landing-ticks-only pattern — if any part2 test DOES assert a second sequential packet's pts, downgrade F-001 to cover only the fixtures/corpus (my sweep says none exists; `seek_maps_frames_and_reanchors` asserts the post-seek first packet at :703 and the landing span at :680).
- `trait_misuse_is_typed_not_a_panic` (:633) — verify it covers the same pre-start refusals as the ivf/adts siblings (uniformity check).
