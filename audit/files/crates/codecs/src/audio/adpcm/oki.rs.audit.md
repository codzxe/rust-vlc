# AUDIT: crates/codecs/src/audio/adpcm/oki.rs
| Field | Value |
|---|---|
| Auditor | W07-A06 |
| Date | 2026-11-23 |
| Lines / functions | 225 / 6 (incl. 6 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs:299-302 `codec if codec.starts_with("adpcm_")` → `self.seeded(track, config, AdpcmDecoder::new)`; adpcm module is unconditionally compiled (crates/codecs/src/audio/mod.rs:37, no feature gate per Cargo.toml:26 "in-house ... zero external deps") |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | prebuilt `./target/debug/deps/codecs-a0d7a93b7072ff58 adpcm::oki` -> 6 passed; prebuilt `./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b` -> 4 passed (incl. the OKI ffmpeg-MD5 rows); python3 fixture inspection (ramp covers all 16 nibble values) |

## 1. One-paragraph verdict
This is a real, exact port of ffmpeg's `adpcm_ima_oki_expand_nibble`: the 49-entry OKI step table (16..1552), the 16-entry index table addressed by the full nibble, `diff = ((2·(n&7)+1)·step)>>3`, sign from bit 3, the 12-bit accumulator clamp (`av_clip_intp2(p, 11)` = `clamp(-2048, 2047)`), index clamp 0..=48, and ×16 output scaling — every element matches the reference law, and the pinned differential (hand-crafted 512-byte ramp fixtures decoded by ffmpeg's forced decoder) passes bit-exactly. I executed both the 6 unit tests and the breadth differential. The lettered `adpcm_oki` (32k/24k) gap is honestly typed-refused upstream (`adpcm.oki.no_reference`, mod.rs:481-489) with the no-oracle rationale documented — an honest ledger entry, not a hidden stub. The only defects are two hygiene items: a stats struct whose doc misstates mono semantics and whose value production code discards, and an implicit ≤2-channel contract enforced only by a distant caller.

## 2. What actually works (verified)
- `./target/debug/deps/codecs-a0d7a93b7072ff58 adpcm::oki` -> 6 passed: step-table pin (49 entries, 16 first, 1552 last, monotone), hand-computed first-nibble ladder (0→32, 7→512), 12-bit rails (−2048→−32768, −2048+max-step→13792 — arithmetic reproduced by hand: ((2·7+1)·1552)>>3 = 2910), index rails, high-nibble-first channel order (0x18 → [96, −32]), cross-call state persistence.
- `./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b` -> 4 passed, incl. `breadth_corpus_is_bit_exact_against_ffmpeg_reference` — oki_mono.vox (MD5 60714fbf...) and oki_st.vox (MD5 bad7f93b...) against ffmpeg 7.1.5's forced decoder.
- python3 fixture check: oki_mono.vox is a 512-byte step-7 ramp whose nibble set is exactly {0..15} — the "every nibble value" coverage claim is true.
- Integer-safety audit: predictor ∈ [−2048, 2047] ⇒ ×16 ∈ [−32768, 32752] fits i16 with no truncation; all arithmetic in i32 with no overflow possible (max (15)·1552 = 23280).

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/audio/adpcm/oki.rs:90-116
Code:
```
pub(crate) struct OkiStats {
    /// Sample instants appended (all channels interleaved).
    pub frames: u64,
}
...
    let mut frames = 0u64;
    for &byte in data {
```
What is wrong: the doc says "sample instants appended (all channels interleaved)", but `frames` counts one per BYTE. For stereo that is one instant (correct); for mono a byte is TWO instants (high nibble then low on the same channel), so mono undercounts by 2×. The production caller discards the return entirely (mod.rs:609 `oki::decode_ima_oki(data, channels, &mut oki_state, &mut buf);` — no binding), so the field is consumed only by the in-file test (which uses stereo-shaped counting on a mono byte at L204-205). What correct looks like: `frames += u64::from(channels == 1 ? 2 : 1)`-style honesty, or rename the field `byte_pairs`/delete the struct. Confidence: HIGH — Verification: read + ran.
F-002 [S4]
Where: crates/codecs/src/audio/adpcm/oki.rs:107-111
Code:
```
    let st = channels.saturating_sub(1);
    let mut frames = 0u64;
    for &byte in data {
        let high = expand(&mut state[0], byte >> 4);
        let low = expand(&mut state[st], byte & 0x0F);
```
What is wrong: for channels ≥ 3 this silently decodes a two-channel stream onto state[0] and state[channels−1] and interleaves only two planes — wrong output, no error. The ≤2 contract is enforced only by `parse_raw_facts` (mod.rs:721-728: `channels == 0 || channels > 2` → typed `adpcm.raw.facts.channels`) at configure time, two modules away; the function itself neither asserts nor documents its channel ceiling (the doc at L99-100 says "channel 0 = v >> 4, channel 1 = v & 0x0F" without stating "channels must be ≤ 2"). A future caller (e.g., a WAV-carried OKI flavor) bypassing parse_raw_facts gets silent corruption. What correct looks like: `debug_assert!(channels <= 2)` or a typed error. Confidence: HIGH — Verification: read + caller traced.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| OKI_STEP_TABLE (const) | 45-51 | REAL | — | ffmpeg's oki_step_table verbatim; pinned by test (49/16/1552/monotone) + MD5 differential |
| INDEX_TABLE (const) | 55-58 | REAL | — | ffmpeg's ff_adpcm_index_table (±, 2/4/6/8 halves) |
| OkiChannel (struct) | 64-70 | REAL | — | 12-bit predictor + step index; Default = fresh stream |
| expand | 73-88 | REAL | — | Exact ffmpeg law (see §1); clamps before state commit; ×16 fits i16 |
| OkiStats (struct) | 91-95 | REAL | S4 | Misdocumented for mono, discarded in production (F-001) |
| decode_ima_oki | 101-117 | REAL | S4 | High-nibble-first packing correct for 1-2 ch; mono uses state[0] twice (matches ffmpeg); implicit ≤2 contract (F-002); per-sample extend_from_slice on a caller-cleared buf — amortized, no per-call alloc after growth |
| tests ×6 | 119-225 | REAL | — | Hand-verified arithmetic + rails + channel order + persistence |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| step_table_is_the_pinned_49_entry_ladder | YES | Length/first/mid-slot/last pins catch table drift (memory-guessed tables fail the 107@20 pin) |
| first_nibble_ladder_is_hand_verified | YES | Exact i16 outputs + step indices from hand arithmetic |
| accumulator_is_twelve_bit_and_output_scaled | YES | Rail saturation at −2048/−32768 and the 13792 recovery pin the clamp AND the ×16 |
| index_clamps_at_both_rails | YES | 46+8→48 and 48×(−1)→0 |
| channel_order_is_high_nibble_first | YES | 0x18 → [96, −32] — packing inversion caught |
| state_persists_across_calls | YES | Second call's last sample equals scaled predictor |
| tests/adpcm_breadth_differential.rs (external, this module's real proof) | YES | ffmpeg forced-decoder MD5s on both fixtures — any law drift fails; EXECUTED by me |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "diff = ((2·(nibble & 7) + 1) · step) >> 3" / 12-bit clamp / ×16 output | oki.rs:11-17 | REAL | Code L77-87 matches; unit tests + ffmpeg MD5 differential |
| "ffmpeg 7.1.5 ships neither a decoder nor an encoder for them [lettered adpcm_oki]" | oki.rs:27-29 | REAL | EXECUTED: `ffmpeg -decoders | rg -i oki` on this sandbox's ffmpeg 7.1.5-0+deb13u1 lists ONLY `adpcm_ima_oki` (+unrelated smjpeg) — no lettered 32k/24k `adpcm_oki` decoder or encoder exists |
| "hand-crafted vectors (a 512-byte deterministic ramp — every nibble value, both channels)" | oki.rs:38-40 | REAL | python3: 512 bytes, nibble set {0..15}, both fixtures present |
| "this decoder is bit-exact on both paths — tests/adpcm_breadth_differential.rs" | oki.rs:41-42 | REAL | EXECUTED: 4/4 passed incl. both OKI MD5 rows |

## 7. Dead / duplicated / unreachable code
`OkiStats` is production-dead (return value discarded at the only production call site, mod.rs:609) — kept alive by the in-file test only. No duplication: the step table's relationship to the IMA table is documented as a window (L136-137) but the tables are correctly separate constants.

## 8. Dependencies & flags
None. Unconditional `pub(crate) mod oki` (adpcm/mod.rs:99) — no feature gate, always in the shipped build. No unsafe. Zero external deps (in-house card 5.J2b).

## 9. Open questions for the next agent
- adpcm/mod.rs owner: confirm `parse_raw_facts`'s ≤2 gate also covers any future WAV-flavor route to decode_ima_oki (F-002).
- If ffmpeg lands in the sandbox, re-verify the "no lettered-oki decoder" claim (`ffmpeg -decoders | grep oki`) to close the REAL-UNPROVEN row above.
