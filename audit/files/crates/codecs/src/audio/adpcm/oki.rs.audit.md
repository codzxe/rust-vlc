# AUDIT: crates/codecs/src/audio/adpcm/oki.rs
| Field | Value |
|---|---|
| Auditor | W07-A08 |
| Date | 2026-11-21 |
| Lines / functions | 225 / 11 (2 tables, 1 const, 1 struct pair, 1 expand fn, 1 decode fn, 6 test fns) |
| File verdict | REAL (bit-exact nibble law verified against ffmpeg's `adpcm_ima_oki_expand_nibble`; two S4 hygiene defects) |
| Reachable from shipped entry point? | YES (call chain: `lumen` open WAV/ADPCM → `AdpcmDecoder::decode` adpcm/mod.rs:608-609 `oki::decode_ima_oki(...)` ← descriptor dispatch apps/cli/src/pipeline_host.rs:298-303 `codec if codec.starts_with("adpcm_")`). Execution: breadth differential + unit suites green in baseline #3 (INDEX.md). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | read adpcm/mod.rs configure/decode/parse_raw_facts (gates: channels ∈ {1,2} at mod.rs:721-728; state sized at 531; return value discarded at 609) · `rg -n "oki" tests/adpcm_breadth_differential.rs` (oracle coverage: mono+stereo MD5-pinned) · nibble law re-derived by hand vs ffmpeg 7.1.5 `adpcm_ima_oki_expand_nibble` · `sed -n` quote verification |

## 1. One-paragraph verdict
This is what a bit-exactness-disciplined codec module should look like: the 49-entry step table, the ±1/2/4/6/8 index table, the `((2·(n&7)+1)·step)>>3` diff, the 12-bit `[-2048,2047]` accumulator clamp, and the ×16 output scaling all match ffmpeg's OKI expander operation-for-operation (order of step-read/index-update/predictor-clamp included), and both mono and stereo are pinned by ffmpeg-measured MD5s on hand-crafted ramps that walk every nibble value. The `adpcm_oki` lettered-variant gap is an honestly-typed refusal with the no-oracle rationale documented. The two defects are hygiene: a stats return value whose doc lies for mono and which the only caller throws away, and a decode entry point whose `channels`/`state` contract lives entirely in a gate 500 lines away in another file, enforced by neither signature nor assert.

## 2. What actually works (verified)
— Nibble law verified by hand against ffmpeg 7.1.5 `adpcm_ima_oki_expand_nibble`: step table (49 entries, max 1552), index table (16 entries, ±), diff formula, sign bit (nibble&8 → subtract), `av_clip_intp2(p,11)` clamp (85), `predictor*16` output (87) — all exact. `predictor*16` ∈ [-32768, 32752] cannot truncate in i16.
— Bit-exactness: `tests/adpcm_breadth_differential.rs` cases `oki_mono.vox`/`oki_st.vox` (1×512-byte ramps, MD5s `6071…`/`bad7…`) ran green in baseline #3 (INDEX.md: 1681 passed). Hand-verification tests (147-158, 161-181, 184-195, 198-211, 214-224) pin the ladder, rails, index clamps, channel order, and cross-packet state.
— Channel-order claim (high nibble = channel 0) pinned by `channel_order_is_high_nibble_first` (198-211) with exact expected samples (96 / −32).
— `raw_out_bound(data.len())` = 8×payload+64 (mod.rs:759-761) bounds the 4×payload worst case — the pooled slot can never be short.

## 3. Findings
### F-001 [S4] `OkiStats.frames` doc is false for mono, and the value is dead
Where: crates/codecs/src/audio/adpcm/oki.rs:90-95 and 107-116
Code:
```
pub(crate) struct OkiStats {
    /// Sample instants appended (all channels interleaved).
    pub frames: u64,
}
```
(with the loop at 114: `frames += 1;` once per **byte**)
What is wrong: a byte carries 2 mono samples or 1 stereo frame. For stereo, frames == sample instants; for mono, the code counts bytes (half the true sample instants) — the doc "Sample instants appended" is wrong for the mono lane. The only caller discards the return (adpcm/mod.rs:609 `oki::decode_ima_oki(data, channels, &mut oki_state, &mut buf);` — no binding), computing the true frame count from byte length instead (mod.rs:650).
Why it matters: dead API whose documentation invites a future consumer into a 2× timestamp drift on mono VOX (the dominant OKI shape — telephony is mono).
What correct looks like: delete `OkiStats`, or count `frames += 1` per output sample-instant (2 per byte mono, 1 per byte stereo) and fix the doc.
Confidence: HIGH — Verification: read; caller traced (single call site, return discarded).
### F-002 [S4] `decode_ima_oki`'s channels/state contract is implicit and unenforced
Where: crates/codecs/src/audio/adpcm/oki.rs:101-115
Code:
```
    let st = channels.saturating_sub(1);
    let mut frames = 0u64;
    for &byte in data {
        let high = expand(&mut state[0], byte >> 4);
        let low = expand(&mut state[st], byte & 0x0F);
```
What is wrong: the function requires `channels ∈ {1,2}` and `state.len() == channels` to be correct, but nothing in the signature or body enforces it: `channels == 0` with an empty `state` panics at `state[0]`; `channels ≥ 3` silently decodes only channels 0 and N−1, appends 2 samples per byte (the caller's `filled = len/2/channels` then computes a wrong duration and the format's declared channel count misinterprets the interleave). The invariant is upheld today only by `parse_raw_facts` (mod.rs:721-728, "channel count {channels} … at most stereo") and the state sizing at mod.rs:531 — both in another file.
Why it matters: `pub(crate)` surface with a hidden contract; the next flavor added to the ADPCM family that reuses this entry point with a wider channel fact gets silent wrong audio, not a typed error.
What correct looks like: `debug_assert!((1..=2).contains(&channels) && state.len() == channels)` at the top, or a typed error return like the sibling block flavors.
Confidence: HIGH — Verification: read; gates traced (mod.rs:721-728, 531); no other caller (rg `decode_ima_oki` → mod.rs:609 and tests only).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| const OKI_STEP_TABLE | 45-51 | REAL | — | 49 entries; ladder monotonicity pinned by test 132-144 |
| const INDEX_TABLE | 55-58 | REAL | — | ffmpeg's 16-entry ± table ✓ |
| const MAX_STEP_INDEX | 61 | REAL | — | 48 = table len − 1 |
| struct OkiChannel | 64-70 | REAL | — | persists across packets (pinned by test 214-224) |
| expand | 73-88 | REAL | — | nibble law exact vs ffmpeg (see §2); index update clamped 0..=48; no panic path (nibble ≤ 15 indexes 16-entry table) |
| struct OkiStats | 91-95 | REAL / dead | F-001 | discarded by the only caller |
| decode_ima_oki | 101-117 | REAL | F-002 | mono: both nibbles through state[0] sequential ✓; stereo: high→state[0], low→state[1] ✓; 4 output bytes per input byte within the 8× bound |
| test step_table_is_the_pinned_49_entry_ladder | 131-144 | REAL | — | would fail on a memory-guessed table (the doc's own words) |
| test first_nibble_ladder_is_hand_verified | 146-158 | REAL | — | exact hand values 32/512 |
| test accumulator_is_twelve_bit_and_output_scaled | 160-181 | REAL | — | rail −2048 → −32768, no wrap; recovery +13792 exact |
| test index_clamps_at_both_rails | 183-195 | REAL | — | 48 ceiling, 0 floor |
| test channel_order_is_high_nibble_first | 197-211 | REAL | — | exact 96/−32 |
| test state_persists_across_calls | 213-224 | REAL | — | 2 calls, 4 samples, scaled-predictor pin |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 6 in-file tests | YES | exact-value assertions on hand-derived numbers (not golden-from-self) |
| breadth differential oki_mono/oki_st | YES | ffmpeg-measured MD5 of s16le decode; any table/formula/order regression flips the digest; the ramp fixture walks every nibble value on both channels |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| nibble law block ("pinned against the ffmpeg 7.1.5 reference … exact") | oki.rs:5-22 | REAL | re-derived by hand vs `adpcm_ima_oki_expand_nibble`; MD5-pinned |
| "this decoder is bit-exact on both paths — tests/adpcm_breadth_differential.rs" | oki.rs:36-42 | REAL | baseline #3 green; the `-f s16le` shorthand is a lossy paraphrase of the test's `pcm_s16le -f md5` oracle (breadth header lines 9-11) — substance correct |
| "`adpcm_oki` lettered variants: no ffmpeg 7.1.5 decoder or encoder — typed Unsupported" | oki.rs:24-34 | REAL | mod.rs:481-492 gate + test `lettered_oki_is_the_honest_typed_gap` (mod.rs:1020-1026) |
| "OkiStats.frames: Sample instants appended" | oki.rs:93-94 | FALSE (mono) | F-001 |

## 7. Dead / duplicated / unreachable code
`OkiStats` + the `frames` counter (F-001). Nothing else: both tables, both structs, and both functions have live consumers.

## 8. Dependencies & flags
No deps (pure std, integer-only). Part of the adpcm module (no separate feature). No unsafe, no panics reachable through the current gates (the F-002 panic window is closed upstream by mod.rs:721-728).

## 9. Open questions for the next agent
- adpcm/mod.rs (its agent): the Yamaha path (`yamaha::decode_yamaha`, mod.rs:602) has the same shape (channels, per-channel state, discarded stats?) — does it share F-001/F-002, and does its raw-facts gate also clamp to stereo?
- The mono-`frames` semantics would matter if any future caller binds it — if mod.rs ever switches to stats-based duration (like the ms/ima block flavors' `Stats`), re-audit this file's counter first.
