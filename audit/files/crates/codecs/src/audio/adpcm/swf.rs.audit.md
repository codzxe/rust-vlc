# AUDIT: crates/codecs/src/audio/adpcm/swf.rs
| Field | Value |
|---|---|
| Auditor | W07-A02 |
| Date | 2026-11-23 |
| Lines / functions | 431 / 5 production fns (BitReader::new/u/s, decode_swf_packet, push_sample) + 8 tests |
| File verdict | REAL (bit-exact against ffmpeg 7.1.5 on every code width, boundary, and block-chaining case I threw at it) |
| Reachable from shipped entry point? | Code path: via `AdpcmDecoder` Plan::Swf (mod.rs:615-621) — but no shipped demuxer emits `adpcm_swf` (see mod.rs audit F-002); plugin-seam only. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/codecs-a0d7a93b7072ff58 audio::adpcm::swf` → 8 passed (exit 0); ffmpeg 7.1.5 oracle: committed payload FLV re-wrap → `MD5=9aac930510b6ef482b7303db62a7b200` (= pin); fresh `adpcm_swf` FLV encode → port MD5 `de9cdd08c27a6406192975430419db53` = ffmpeg; crafted exact-fit/width-2/3/5/header-only/4095-cap packets → all MATCH; python3 port scripts in /tmp |

## 1. One-paragraph verdict
This is the strongest kind of implementation: a from-scratch MSB-first bit reader plus the SWF ADPCM block loop, and I verified it bit-exactly against the real ffmpeg 7.1.5 decoder far beyond the repo's own pinned corpus — code widths 2, 3, 4 and 5 (the committed corpus only pins width 4), header-only packets, exact-field-boundary tails, and the 4095-field block-cap chaining with a second in-band block header all produce sample-for-sample identical output to ffmpeg. The index tables for widths 2/3/5 — which have NO differential pin in the repo — are correct. The only defects are hygiene: a latent infinite loop if `channels == 0` (unreachable through the public gate) and the usual little-endian host assumption on native-endian output.

## 2. What actually works (verified)
- `./target/debug/deps/codecs-a0d7a93b7072ff58 audio::adpcm::swf` → `test result: ok. 8 passed` (exit 0).
- Committed `swf_payload.bin` (9 × 4101 stereo) re-wrapped into FLV audio tags: `ffmpeg ... -f md5 -` → `MD5=9aac930510b6ef482b7303db62a7b200` — identical to the pin in adpcm_breadth_differential.rs:77; my python port of this file decodes the same bytes to the same MD5.
- Fresh encode round-trip: `ffmpeg -c:a adpcm_swf` 6-packet stereo FLV; ffmpeg PCM MD5 `de9cdd08c27a6406192975430419db53` == port MD5, 49152 samples.
- Adversarial crafted packets (mono, FLV-wrapped, decoded by ffmpeg vs port): header-only `(100,)` MATCH; exact 32-bit boundary packet `(100,108,102)` MATCH; width-2 12-field `(100,115,121,...,93)` MATCH; width-3 16-field MATCH; width-5 16-field incl. negative/clamped excursion `(...,863,-958,-178)` MATCH; 4095-cap + second block `(4100 samples, seed -1 at [4096], tail 9,17,25)` MATCH.
- The partial-sum vpdiff (vs the IMA multiply form) is directly confirmed at an odd step: field 0x5 at step 7 gives +8 (partial) not +9 (multiply); ffmpeg agrees with the partial form — exactly what the module doc claims at swf.rs:29-31.

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/audio/adpcm/swf.rs:164-221 (loops keyed on `header_bits`/`field_bits` with `channels` unvalidated)
Code:
```
    let header_bits = 22i64 * channels as i64;
    let field_bits = nb_bits as i64 * channels as i64;
    if (size_bits as i64 - reader.pos as i64) < header_bits {
```
What is wrong: `decode_swf_packet(data, 0, out)` never advances `reader.pos` (both loops iterate zero channels) yet both guards stay true, so the outer/inner loop pair spins forever (count resets to 0 each outer pass). Reachability: NOT reachable — the only caller is mod.rs:615-621 with channels from `parse_raw_facts` (mod.rs:721-728 refuses 0), and fuzz enters through the same gate. Still, the function is `pub(crate)` with no debug_assert on the precondition.
What correct looks like: an early `if channels == 0 { return stats; }` or a debug_assert.
Confidence: HIGH — Verification: read (loop math traced; not executed — refuse to run an infinite loop).

F-002 [S4]
Where: crates/codecs/src/audio/adpcm/swf.rs:226-229 (and push_sample twins in ima.rs/ms.rs)
Code:
```
fn push_sample(out: &mut Vec<u8>, sample: i16) {
    out.extend_from_slice(&sample.to_ne_bytes());
}
```
What is wrong: native-endian output equals ffmpeg's s16le only on little-endian hosts; on a big-endian host every differential MD5 would mismatch. The breadth test documents this ("identical bytes on the little-endian build host — asserted by the match itself"), so it is a known, documented assumption rather than a hidden lie — but it is an assumption, not a property.
Confidence: HIGH — Verification: read (no BE host available to execute).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| BitReader::new | 87-89 | REAL | — | trivial |
| BitReader::u | 92-105 | REAL | — | MSB-first, checked; round-trip test executed; exact-boundary reads verified vs ffmpeg behavior |
| BitReader::s | 109-117 | REAL | — | get_sbits sign-extend; test asserts 0xFF00 → −256 |
| decode_swf_packet | 143-224 | REAL | S4 (F-001) | loop guards empirically identical to ffmpeg (inclusive boundary — proven by the exact-fit packet); 4095 cap chaining verified; channels=0 latent spin |
| push_sample | 227-229 | REAL | S4 (F-002) | native-endian assumption |
| SWF_INDEX_TABLES | 52-65 | REAL | — | rows verified against ffmpeg for widths 2/3/4/5 (crafted-packet runs above) |
| STEP_TABLE | 69-75 | REAL | — | byte-identical to the canonical 89-entry IMA ladder (python diff vs reference: equal) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| bit_writer_round_trips_the_reader | YES | exact values both directions |
| bit_reader_is_msb_first_and_checked | YES | exact bit splits + None at EOF + sign extension |
| mono_block_seed_then_partial_sum_fields | YES | exact ladder [100, 89, 91] |
| odd_step_rounding_is_the_partial_sum_form | YES | pins 11 (partial) vs 13 (multiply) — the discriminating case, and ffmpeg agrees (verified) |
| stereo_pair_layout_interleaves_fields | YES | exact 4-sample pair layout incl. 7-byte packet with 2 pad bits |
| packet_short_of_a_header_is_skipped | YES | skip + zero output |
| empty_and_tiny_packets_are_typed | YES | default stats / skip |
| count_cap_chains_a_second_block_in_one_packet | YES | 4098 samples, seed restart at [4096] — and I re-verified the same shape against ffmpeg with a 4100-sample crafted packet |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "pinned against the ffmpeg 7.1.5 reference" (packet grammar) | swf.rs:5-15 | REAL | my crafted-packet oracle runs above |
| "Block capacity is 4095 coded samples plus the seed (ffmpeg's count < 4095)" | swf.rs:19-20 | REAL | 4095+block-2 crafted packet: port == ffmpeg, 4100 samples |
| partial-sum form "equals the classic ... the odd-step rounding that the multiply form would hide" | swf.rs:22-31 | REAL | field 0x5 @ step 7: +8 (partial) — ffmpeg agrees; IMA (multiply) disagrees |
| "a packet too short for even the 2-bit code + one header is skipped whole" | swf.rs:45-46 | REAL | executed unit tests + fuzz-lite (breadth suite) |
| "bit-exact against the reference decoder's PCM — tests/adpcm_breadth_differential.rs" | swf.rs:35-39 | REAL | breadth suite executed (4 passed) + my independent re-measure of the pin |

## 7. Dead / duplicated / unreachable code
- STEP_TABLE duplicated between ima.rs and swf.rs (89 identical entries, byte-checked equal by me). A shared const would be cleaner; the duplication is deliberate per-flavor isolation — S4-adjacent, noted here, not counted.
- No dead functions.

## 8. Dependencies & flags
- std-only. No feature gates on this file.

## 9. Open questions for the next agent
- None blocking. If anyone ever adds a SWF/FLV demuxer, F-001's channels=0 guard should land first.
