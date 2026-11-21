# AUDIT: crates/codecs/src/audio/adpcm/swf.rs
| Field | Value |
|---|---|
| Auditor | W07-A06 |
| Date | 2026-11-21 |
| Lines / functions | 431 / 15 (7 prod items + 8 tests) |
| File verdict | UNREACHABLE (real, tested code; no shipped entry point can deliver an `adpcm_swf` track) |
| Reachable from shipped entry point? | NO — `AdpcmDecoder` accepts `"adpcm_swf"` (adpcm/mod.rs:296) and both hosts route `adpcm_*` to it (apps/cli/src/pipeline_host.rs:298, apps/web/src/host.rs:614), but NO demuxer mints the descriptor: rg over crates/demux/src finds only `adpcm_ms`/`adpcm_ima_wav` (wav/format.rs, asf/header.rs); there is no FLV or SWF demuxer in the repo (demux crate dirs: adts aiff asf avi es ivf mkv mp4 ogg ps ts wav). Only tests construct the config by hand. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg `decode_swf_packet\|Plan::Swf` (callers: adpcm/mod.rs:616 only + tests); rg `"adpcm_swf"` repo-wide (codecs + docs only, zero demux hits); rg `"adpcm_[a-z0-9_]*"` crates/demux/src (descriptor inventory); rg `adpcm` apps/web/src/host.rs (wasm host also routes but same dead descriptor); sed -n adpcm/mod.rs:280-300,460-537,698-730,753-761 (accepts/plan/parse_raw_facts/bound); grep baseline log (8/8 swf tests + breadth differential ok); hand-recomputation of both decode vectors (see §2) |

## 1. One-paragraph verdict
The codec itself is high-quality: the SWF index tables and 89-entry step table match ffmpeg's `swf_index_tables`/`ff_adpcm_step_table` entry-for-entry, the partial-sum vpdiff loop is the exact do-while shape of ffmpeg's decoder (hand-verified below), the bit reader is MSB-first and bounds-checked, and the block-capacity/budget guards implement the documented `count < 4095` law. All 8 unit tests plus the ffmpeg-referenced breadth differential (`breadth_corpus_is_bit_exact_against_ffmpeg_reference ... ok`) executed in the baseline run. But the module is dead weight in the shipped binary: no FLV/SWF demuxer exists and no demuxer emits `adpcm_swf`, so the only way this code runs outside `cargo test` is a hand-built `DecoderConfig` — the file gets an UNREACHABLE verdict on reachability despite REAL-quality internals. Two latent nits: a `channels == 0` infinite loop (unreachable behind `parse_raw_facts`'s 1..=2 gate) and a dead `!data.is_empty()` expression.

## 2. What actually works (verified)
— Table fidelity: `SWF_INDEX_TABLES` (swf.rs:52-65) matches ffmpeg's 4×16 table exactly; `STEP_TABLE` (swf.rs:69-75) matches `ff_adpcm_step_table`'s 89 entries exactly — verified by read against the known tables.
— Algorithm hand-check (independent of the code's own tests): predictor 100, index 0 (step 7), field 0xF → vpdiff = 7+3+1+(7>>3)=11, sign set → 89, index += table[7]=+8; field 0x0 → vpdiff = 16>>3=2 → 91. Code path (swf.rs:187-219) reproduces this exactly; test `mono_block_seed_then_partial_sum_fields` asserts [100, 89, 91].
— k-loop equivalence to ffmpeg's do-while traced for nb_bits=2 and 4 (swf.rs:197-207): magnitude bits step, step>>1, step>>2 … then final `+= scale` (= step>>3 at width 4) — identical to the classic form.
— Execution evidence: `audio::adpcm::swf::tests::*` 8/8 ok (test-workspace.log.txt:322-329), `audio::adpcm::tests::swf_and_qt_packet_damage_recovers_typed ... ok` (:336), `breadth_corpus_is_bit_exact_against_ffmpeg_reference ... ok` (:826).

## 3. Findings
### F-001 [S3] Whole module unreachable in the shipped build — no container lane can mint `adpcm_swf`
Where: crates/codecs/src/audio/adpcm/swf.rs:1-3 (module purpose) vs crates/demux/src (no FLV/SWF demuxer)
Code:
```rust
//! SWF / Shockwave Flash ADPCM (card 5.J2b): the 4-bit codec of SWF
//! `SoundStreamBlock`s and FLV audio tags — the `adpcm_swf` row of
//! doc 04 §3.1's ADPCM block.
```
What is wrong: The decoder, its provider lanes (CLI `pipeline_host.rs:298` and web `host.rs:614`, both `starts_with("adpcm_")`), and its 431 lines of tested code are all compiled into shipped builds, but the descriptor `adpcm_swf` is minted by nothing. The demux crate contains no FLV or SWF demuxer at all, and the only ADPCM descriptors any demuxer emits are `adpcm_ms` and `adpcm_ima_wav` (wav/format.rs, asf/header.rs). The doc's "FLV-carried fixture" evidence is a test-time payload extraction (`swf_payload.bin` + `swf_sizes.json`), not a runtime demux lane.
Why it matters: Dead weight shipped in the binary and a parity claim that cannot be exercised: a user cannot open any SWF/FLV file (no demuxer) nor any file whose audio resolves to `adpcm_swf`. The feature-matrix row `codec-audio-adpcm` (status `implemented`, win/linux/macos) covers this flavor in its evidence string ("7 descriptors").
What correct looks like: Either an FLV/SWF demux lane that mints the descriptor, or the module kept test-only and the matrix row's evidence scoped to what a shipped binary can open.
Confidence: HIGH — Verification: read (rg over every demuxer's descriptor minting; no `adpcm_swf` producer exists).

### F-002 [S4] `channels == 0` hangs `decode_swf_packet` in an infinite loop (latent — blocked upstream)
Where: crates/codecs/src/audio/adpcm/swf.rs:143-222
Code:
```rust
    let header_bits = 22i64 * channels as i64;
    let field_bits = nb_bits as i64 * channels as i64;
```
What is wrong: With `channels == 0`, both budgets are 0: the header loop seeds nothing (0 states), the field loop consumes no bits (guard `left >= 0` always true, 4095 no-op iterations), and the outer loop re-enters forever — `pos` never advances, the function never returns. A hang, not a panic.
Why it matters: Latent only: `decode_swf_packet` is `pub(crate)` with one production caller that can never pass 0 — `parse_raw_facts` (adpcm/mod.rs:721-728) refuses `channels == 0 || channels > 2` at configure — and the fuzz harness asserts `1..=2` (adpcm/fuzz.rs:153-155). But any future caller (e.g., an FLV demuxer wiring raw facts directly) that skips the gate gets a player hang on malformed input.
What correct looks like: An early `if channels == 0 { return stats; }` (or debug_assert) at function entry; the budgets make no sense for zero channels.
Confidence: HIGH — Verification: read (loop trace with channels=0: no bit consumption, guard `>= 0` always true).

### F-003 [S4] Dead expression `!data.is_empty()` in the code-bits early return
Where: crates/codecs/src/audio/adpcm/swf.rs:146-149
Code:
```rust
    let Some(code) = reader.u(2) else {
        stats.skipped_packet = !data.is_empty();
        return stats;
    };
```
What is wrong: `reader.u(2)` at position 0 fails only when `2 > data.len() * 8`, i.e. only when `data` is empty — so inside the `else` branch `!data.is_empty()` is provably always `false`. The expression suggests a non-empty case that cannot occur.
Why it matters: Misleading defensive code; a reader assumes non-empty packets can take this path.
What correct looks like: `stats.skipped_packet = false;` (or a comment that only empty packets reach here), or drop the assignment.
Confidence: HIGH — Verification: read (u() guard arithmetic at swf.rs:92-96).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SWF_INDEX_TABLES` | 52-65 | REAL | — | matches ffmpeg `swf_index_tables` entry-for-entry (verified by read) |
| `STEP_TABLE` | 69-75 | REAL | — | matches `ff_adpcm_step_table` (89 entries, verified by read) |
| `BitReader::new` | 86-88 | REAL | — | trivial |
| `BitReader::u` | 92-105 | REAL | — | MSB-first, `pos+bits > len*8` checked; bit-per-bit loop (not word-batched — cold path, fine); `bits==0` → None |
| `BitReader::s` | 109-117 | REAL | — | correct sign extension; test pins 0xFF00 → −256 |
| `SwfChannel` | 122-126 | REAL | — | state re-seeded per block header; nothing persists across packets (correct: SWF blocks are self-contained) |
| `SwfStats` | 129-137 | REAL | — | damage observability |
| `decode_swf_packet` | 143-224 | REAL | — | hand-verified vs ffmpeg semantics (vpdiff partial-sum, sign convention `delta & signmask → -=`, magnitude-masked table index, step clamp 0..=88, predictor clamp i16); guards `count < 4095` + bit budgets; F-002 zero-channel hang latent; outer-guard form `left >= 22*ch` mirrors the swf.rs:163 comment's citation of ffmpeg `get_bits_count <= size - 22*ch` — ffmpeg's exact form NOT verifiable in-sandbox (see §9) |
| `push_sample` | 227-229 | REAL | — | native-endian i16 append |
| test `bit_writer_round_trips_the_reader` | 295-306 | REAL | — | mutation: reader bit order change fails ✓ |
| test `bit_reader_is_msb_first_and_checked` | 308-319 | REAL | — | pins MSB-first + exhaustion + sign extension ✓ |
| test `mono_block_seed_then_partial_sum_fields` | 321-332 | REAL | — | hand-computed vector [100, 89, 91] ✓ |
| test `odd_step_rounding_is_the_partial_sum_form` | 334-346 | REAL | — | pins partial-sum vs multiply-form divergence (11 vs 13) — genuinely discriminates the algorithm ✓ |
| test `stereo_pair_layout_interleaves_fields` | 348-376 | REAL | — | hand-packed 54-bit packet; pins seed-pair + interleaved fields ✓ |
| test `packet_short_of_a_header_is_skipped` | 378-393 | REAL | — | pins typed skip ✓ |
| test `empty_and_tiny_packets_are_typed` | 395-403 | REAL | — | empty + sub-header ✓ |
| test `count_cap_chains_a_second_block_in_one_packet` | 405-430 | REAL | — | 2054-byte packet, 4098 samples, re-seed at index 4096 — pins the block-capacity law ✓ |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 8 tests | YES | Every expectation is hand-computed from the bit grammar and the partial-sum formula (e.g. [100, 89, 91], [0, 11, 13], got[4096]=−1); none derived from the decoder. The odd-step test specifically discriminates the partial-sum form from the multiply form. All executed in baseline (test-workspace.log.txt:322-329). |
| breadth differential (`tests/adpcm_breadth_differential.rs`) | YES (for the fixture) | MD5 vs ffmpeg 7.1.5 reference output, committed hash `9aac930510b6ef482b7303db62a7b200`; ran ok in baseline (:826). Scope limit: the 9 fixture packets all end with 2 pad bits — the fixture cannot discriminate the outer-guard boundary case (see §9). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "adpcm_swf … ffmpeg enc+dec, FLV-carried fixture, bit-exact" | adpcm/mod.rs:25 table; docs/formats/audio-adpcm.md Evidence § | REAL (as a decoder-vs-oracle claim) | breadth differential ok in baseline (:826) |
| feature-matrix row `codec-audio-adpcm` status `implemented` (win/linux/macos), evidence "7 descriptors" | docs/feature-matrix.toml:793-804 | SHALLOW for the swf flavor | decoder exists + is bit-exact, but no shipped demux path mints `adpcm_swf` (F-001); only adpcm_ms/adpcm_ima_wav are openable end-to-end |
| "Never panics" (module doc swf.rs:142) | swf.rs:139-142 | REAL | bounds-checked bit reader, clamped tables/index; fuzz + survival suites green in baseline; the channels=0 hang (F-002) is a hang, not a panic, and is gated off |

## 7. Dead / duplicated / unreachable code
- The entire module is unreachable from shipped entry points (F-001) — kept alive by tests/differential only.
- `STEP_TABLE` is a second copy of the IMA 89-entry ladder (comment at swf.rs:67-68 acknowledges the shared-shape with `ima`/`oki` modules — deliberate per-flavor duplication; cross-flavor divergence not checked here, other agents' files).
- Dead expression F-003.

## 8. Dependencies & flags
- std-only, no external deps, no `unsafe`, no feature gate (in-house decoder, unconditional like pcm). The whole adpcm module ships in every default build — compiled dead weight for the swf flavor.

## 9. Open questions for the next agent
- ffmpeg guard-form citation: swf.rs:163 comments `ffmpeg: while (get_bits_count(&gb) <= size - 22 * channels)` and the code implements `left >= 22*ch` (equivalent). I could NOT verify ffmpeg 7.1.5's actual source form in this sandbox (no ffmpeg checkout; UNVERIFIED). If ffmpeg's real form is `get_bits_left > 22*ch` (strict), this decoder accepts one extra bare-header block (leftover bits exactly 22×ch) that ffmpeg would drop — one seed sample per channel of divergence. The committed fixture (9 × 4101-byte packets, all ending with 2 pad bits) cannot discriminate the two forms. REQUEST-ORCH-RUN (evidence-gathering, not a build): `ffmpeg -version` + extract adpcm.c SWF loop from the pinned 7.1.5 source — or accept as UNVERIFIED.
- adpcm/mod.rs owner: the pooled-slot `raw_out_bound` (mod.rs:753-761) is payload×8+64 — my arithmetic says SWF width-2 is exactly 8×; confirm the other breadth flavors' maxima (QT 34-byte BE chunks) stay under it.
