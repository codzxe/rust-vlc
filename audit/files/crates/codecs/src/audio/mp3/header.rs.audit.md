# AUDIT: crates/codecs/src/audio/mp3/header.rs
| Field | Value |
|---|---|
| Auditor | W07-A09 |
| Date | 2026-11-30 |
| Lines / functions | 653 / 9 (+7 test fns) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — CLI `decoder_for` "mp3"/"mp1"/"mp2" (apps/cli/src/pipeline_host.rs:261-270) -> `Mp3Decoder::decode` -> `decode_one` -> `parse_frame_header` (mp3/decoder.rs:131); ES container mints `mp3`/`mp2` descriptors (apps/cli/src/containers/es.rs:365-368) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `./target/debug/deps/codecs-a0d7a93b7072ff58 audio::mp3::header` -> `test result: ok. 7 passed; 0 failed` (exit 0); table/formula checks recomputed by hand (see §2); sed/rg quote verification (exit 0) |

## 1. One-paragraph verdict
This is a real, spec-correct MPEG-audio header parser and frame walker. I hand-verified every ISO/IEC 11172-3/13818-3 table and formula against the spec: the five bitrate tables, the three sample-rate rows, version/layer/mode/emphasis bit mappings, all three frame-size formulas (incl. the MPEG-2/2.5 Layer III 72-coefficient), and samples-per-frame values — all correct, and the tests assert exact byte lengths (417/1152/192/484/418) computed from real header bytes, so any table or arithmetic mutation fails a test. Every error path is typed with a distinct context code and pinned by tests. The only defects are trivia: `samples_per_frame` returns 0 instead of an unreachable for Reserved (dead defensive arm), and the free-format refusal is a real but honestly-documented product gap.

## 2. What actually works (verified)
- `./target/debug/deps/codecs-a0d7a93b7072ff58 audio::mp3::header` -> `test result: ok. 7 passed; 0 failed; 0 ignored` — includes `error_paths_are_each_typed` (8 distinct typed rejections) and `walk_frames_resyncs_and_drops_a_truncated_tail`.
- Hand-recomputed (calculator) against the code: `[0xFF,0xFB,0x90,0x00]` → V1/L3/idx9 → 128 kbps, sr idx0 → 44100, `144*128000/44100 = 417` ✓; `[0xFF,0xFC,0xE4,0x00]` → V1/L2/idx14 → 384 kbps, sr idx1 → 48000, `144*384000/48000 = 1152` ✓; `[0xFF,0xF3,0x84,0x00]` → V2/L3/idx8 → 64 kbps @ 24000, `72*64000/24000 = 192` ✓; `[0xFF,0xFE,0xE0,0x00]` → V1/L1/idx14 → 448 kbps @ 44100, `(12*448000/44100)*4 = 484` ✓; padded variant 418 ✓. Bitrate tables match ISO 11172-3 §2.4.2.3 / 13818-3 exactly (MPEG-1 L1/L2/L3, LSF L1, LSF L2-3 rows).
- Walker termination proof (read): every loop iteration either advances `pos` by `frame_size >= 4`, jumps to a strictly greater sync offset, or breaks — no livelock on any input.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/audio/mp3/header.rs:303-311
Code:
```
        None if bitrate_index == 0 => {
            // Free-format: doc 04 §3.2's known gap — refuse, never guess.
            return Err(unsupported(
                "mp3.free_format",
```
What is wrong: free-format (index 0000) MP3 files — legal per ISO 11172-3, produced by some encoders (e.g. LAME `-t --freeformat`) — cannot be played: the walker cannot frame them, so they are rejected wholesale. Why it matters: a spec-legal input class is unplayable; the frame size is measurable (constant across a stream) and other decoders support it. The code is at least honest: the refusal is typed and the module doc calls it a "known `[VERIFY-P0]` gap". What correct looks like: measure the constant frame length from two consecutive syncs and validate across the stream. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/codecs/src/audio/mp3/header.rs:195
Code:
```
        (MpegVersion::Reserved, _) | (_, Layer::Reserved) => 0,
```
What is wrong: `samples_per_frame` answers 0 for Reserved inputs that `parse_frame_header` structurally rejects before this is ever reachable with them; 0 would silently produce zero-duration arithmetic if a future caller skipped the gate. Why it matters: defensive arm that fabricates a plausible value instead of being unrepresentable. What correct looks like: `unreachable!()`-by-construction or Option. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| table_bitrate | 143-160 | REAL | — | tables hand-verified vs ISO; index 0/15 rejected |
| table_sample_rate | 164-175 | REAL | — | idx3 reserved rejected |
| channel_count | 179-184 | REAL | — | dual=2 ✓ |
| samples_per_frame | 188-197 | REAL | — | 384/1152/1152/576 ✓; Reserved arm = F-002 |
| compute_frame_size | 202-234 | REAL | — | u64 math, try_from; three formulas ✓ |
| parse_frame_header | 243-381 | REAL | — | 8 typed error paths, all test-pinned |
| scan_sync | 392-404 | REAL | — | byte-advance scan, validate-before-accept |
| walk_frames | 424-453 | REAL | — | resync + truncated-tail drop; termination proven |
| side_info_size | 458-477 | REAL | — | 17/32/9/17 matches ISO 11172-3 §3.1/13818-3 |
| 7 tests | 486-652 | REAL tests | — | exact-value asserts, mutation-sensitive |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| parses_the_canonical_header | YES | exact bitrate/rate/frame_size/mode equality |
| layer_ii_and_mpeg2_lengths_match_the_spec_formulas | YES | three exact frame sizes from real header bytes |
| mode_extension_emphasis_and_crc_bits_round_trip | YES | every bit-field round-tripped |
| error_paths_are_each_typed | YES | one assert per distinct context code |
| scan_sync_recovers_past_garbage / walk_frames_resyncs_and_drops_a_truncated_tail | YES | exact offsets/counts incl. fake-sync garbage |
| padding_bit_adds_one_slot | YES | 417 vs 418 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the engine (symphonia-bundle-mp3) hard-requires one packet to be exactly one frame" | header.rs:5-9 | REAL | mp3/decoder.rs:131 parse + packet framing; engine dep in Cargo.toml:90 |
| Frame-size formulas L1/L2/L3 | header.rs:11-18 | REAL | recomputed by hand (§2) |
| "free-format index 0000 is a typed refusal" | header.rs:21-23 | REAL | code path + test `mp3.free_format` |
| "The walk is bounded by data.len() — no loops on any input" | header.rs:421-422 | REAL | termination argument (§2) |

## 7. Dead / duplicated / unreachable code
- `SAMPLE_RATES[1]` row `[0,0,0]` (line 136) is unreachable (Reserved version rejected first) — documented as such, harmless.
- `samples_per_frame` Reserved arm (F-002), `side_info_size` Reserved arm (0) — same class.

## 8. Dependencies & flags
No external deps (pure std + media-types). Consumed by mp3/decoder.rs, mp3/seek.rs, mp3/xing.rs, mp3/fuzz.rs, and mp2 (feature codec-mp2 = ["codec-mp3"], both in default set — release lane includes them).

## 9. Open questions for the next agent
- Does any matrix row claim free-format MP3 support that F-001 contradicts?
- mp3/xing.rs and mp3/seek.rs (Xing/VBRI/CBR seek) are not in my assignment — their `side_info_size` use at xing.rs:125 is consistent with the values here, but their own math is unaudited.
