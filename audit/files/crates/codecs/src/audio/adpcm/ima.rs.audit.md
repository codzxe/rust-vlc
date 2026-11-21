# AUDIT: crates/codecs/src/audio/adpcm/ima.rs
| Field | Value |
|---|---|
| Auditor | W07-A05 |
| Date | 2026-11-21 |
| Lines / functions | 300 / 15 (2 tables + 5 fns + 2 structs + 5 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `lumen open x.wav` (IMA WAV) → WAV demux `crates/demux/src/wav/format.rs:207` `WavCodec::AdpcmImaWav => "adpcm_ima_wav"` (also ASF `crates/demux/src/asf/header.rs:409` tag 0x0011) → CLI `apps/cli/src/pipeline_host.rs:298-303` `adpcm_*` → seeded `AdpcmDecoder` → `Plan::ImaWav` → `adpcm/mod.rs:595` `ima::decode_ima_blocks` (only production caller). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "decode_ima_blocks"` (single production caller mod.rs:595 + 5 test call sites); `rg -n "ImaFacts\|channels"` in `adpcm/mod.rs` (channels validated 1..=64 at :317-325 before `ImaFacts{channels}` at :457); `rg -n "adpcm_ima_wav"` in demux (WAV+ASF lanes); baseline orchrun log:258-262 (all 5 ima tests PASS), corpus differential PASS (log: see `adpcm` suite). Hand-recomputed all five pinned samples of `mono_block_decodes_the_pinned_ladder` independently (see §5). |

## 1. One-paragraph verdict
This is the best-audited kind of file in the tree: a small, complete algorithm with the ffmpeg-rounding divergence documented, the 89-entry step table matching the published IMA ladder value-for-value, and tests whose pinned numbers I re-derived by hand and confirmed exactly (2314 → 2301 → 2331 → 2268 → 2132: the multiply-before-shift diff form is real). Two latent traps survive because the only caller happens to shield them: `decode_ima_blocks` HANGS forever if `facts.channels == 0` (the group-advance `g += group` becomes `g += 0` with a non-empty nibble section — an infinite loop, not a panic, so the "never panics" doc technically holds while the process spins), and a block whose declared `block_align` does not split into whole channel groups silently drops group bytes with no `ImaStats` flag, contradicting the module's own damage-ledger honesty. Both are corruption-only and blocked upstream today.

## 2. What actually works (verified)
- Baseline `orchrun-test-workspace-nofailfast.log.txt:258-262`: `step_table_is_the_published_89_entry_ladder`, `mono_block_decodes_the_pinned_ladder`, `header_index_above_88_clamps_and_flags`, `block_smaller_than_header_is_skipped_not_panicked`, `stereo_group_interleave_keeps_channels_separate` — all `ok`.
- `tests/adpcm_corpus_differential.rs` (corpus 0042, `adpcm_ima_wav`, ffmpeg 7.1.5) ran in baseline — the `adpcm` test suite results appear in the same log (adpcm rows at log:255-301, all `ok`).
- Step table checked against the published IMA ladder: all 89 entries match (values read line-by-line, lines 57-61).
- Index-safety of the interleave loop re-derived: max index `4·(channels−1) + (per_group−1)/2` ≤ `group_bytes.len()−1` because `per_group ≤ 2·min(avail_last_ch, 4)` — in-bounds for every damaged-tail shape.

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/audio/adpcm/ima.rs:175-196
Code:
```rust
        let group = 4 * channels; // bytes per 8-sample group, all channels
        let mut g = 0;
        while g < nibble_bytes.len() {
            let group_end = (g + group).min(nibble_bytes.len());
```
What is wrong: with `facts.channels == 0`, `group == 0`: `group_end == g`, `group_bytes` is empty, `per_group` is `unwrap_or(0)`, the inner loops do nothing, and `g += group` (line 195) never advances — the `while g < nibble_bytes.len()` loop spins forever on any block with a non-empty nibble section. A hang, not a panic: the doc's "Never panics; every index is bounds-checked" (lines 126-127) is true and useless.
Why it matters: `decode_ima_blocks` is `pub(crate)` and takes arbitrary `ImaFacts` — its own contract does not demand channels ≥ 1. Today the only production caller validates (`adpcm/mod.rs:320-325` refuses 0 and >64 before building `ImaFacts` at :457), so the shipped path cannot reach it; but one future caller (a new flavor, a fuzz harness, a refactor that moves the gate) turns this into a process hang on attacker-shaped input.
What correct looks like: an early `if channels == 0 { return stats; }` (or debug_assert + typed stats flag) inside the function — the same self-defense `block_align.max(1)` at line 137 already practices for the chunk size.
Confidence: HIGH — Verification: read + hand-executed the loop with channels=0, nibble_bytes=[0x00]; caller chain rg-verified as the shield.

### F-002 [S3]
Where: crates/codecs/src/audio/adpcm/ima.rs:183-194
Code:
```rust
            let per_group = (0..channels)
                .map(|ch| 2 * group_bytes.len().saturating_sub(4 * ch).min(4))
                .min()
                .unwrap_or(0);
            for j in 0..per_group {
```
What is wrong: when the nibble section does not split into whole `4·channels` groups (a WAVEFORMATEX whose `nBlockAlign` lies, e.g. stereo with a 4-byte nibble section), `per_group` is 0 and the group's bytes are skipped with **no** `ImaStats` flag; in partially-covered groups the unread remainder is likewise silently discarded. `concealed_tail` is only set when `block.len() < block_align` (line 172) — a full-size block with a misaligning declared align produces zero samples past the seed and a stats ledger that says "nothing happened".
Why it matters: the module's whole damage model (lines 44-50) is a concealment LEDGER — skipped blocks and clamped indices are counted, tails flagged. Misaligned-align corruption bypasses the ledger entirely: the codec-api layer maps `ImaStats` onto `DecodeOutcome`, so this damage decodes as a clean short block instead of `Recovered`. Wrong duration/EOF arithmetic downstream, silently.
What correct looks like: flag `concealed_tail` (or a new `misaligned_groups` counter) whenever `per_group == 0` with `!group_bytes.is_empty()`, or when a group's bytes are not fully consumed.
Confidence: HIGH — Verification: read + hand-execution (stereo, nibble_bytes=4 → ch1 avail 0 → per_group 0 → 4 bytes unread, stats unchanged).

### F-003 [S4]
Where: crates/codecs/src/audio/adpcm/ima.rs:16
Code:
```rust
//! channel — **low nibble first**, then high. The block decodes
//! `1 + (block_align − 4·channels)·2 / channels` sample instants.
```
What is wrong: the documented sample-count formula holds only for well-formed aligns (`block_align ≡ 4·channels (mod 4·channels·1)`); for the misaligned shapes of F-002 the function decodes strictly fewer instants than the formula promises (stereo, nibble section 5 bytes: formula says 6, code emits 3).
Why it matters: a doc that is exact only on the happy path invites callers to pre-size buffers by the formula.
What correct looks like: state the precondition ("whole groups") or drop the closed form.
Confidence: HIGH — Verification: read + the same hand-execution as F-002.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `STEP_TABLE` | 56-62 | REAL | — | All 89 values match the published IMA ladder (read value-for-value); test pins length/ends/monotone. |
| `INDEX_TABLE` | 65 | REAL | — | `[-1,-1,-1,-1,2,4,6,8]` — the standard IMA WAV table. |
| `MAX_STEP_INDEX` / `TAG_WAVE_FORMAT` | 53,68 | REAL | — | 88 / 0x0011 correct. |
| `header_bytes` | 72-74 | REAL | — | 4·channels; consumed by mod.rs for skip decisions. |
| `ChannelState::expand` | 83-100 | REAL | — | Multiply-before-shift diff (documented ffmpeg divergence); sign bit 3; predictor clamp; index clamp via table + bounds; all indexing in-range by construction; hand-verified values. |
| `ImaStats` / `ImaFacts` | 105-122 | REAL | S3 | Ledger honest for skip/clamp/tail — but not for F-002's misaligned groups. |
| `decode_ima_blocks` | 128-199 | REAL | S3 | channels-0 hang (F-001, caller-shielded); silent misaligned-group drop (F-002); `block_align.max(1)` guards the `chunks(0)` panic — good self-defense precedent that makes F-001's absence glaring; interleave loop bounds re-derived safe. |
| `push_sample` | 203-205 | REAL | — | Native-endian S16 append. |
| test `step_table_is_the_published_89_entry_ladder` | 211-221 | REAL | — | Would fail on table damage (3 pinned values + monotonicity; full-value pin lives in the corpus differential). |
| test `mono_block_decodes_the_pinned_ladder` | 223-246 | REAL | — | Five hand-verifiable pinned samples — I recomputed all five independently and they are exact. |
| test `header_index_above_88_clamps_and_flags` | 248-259 | REAL | — | Pins the concealment flag. |
| test `block_smaller_than_header_is_skipped_not_panicked` | 261-269 | REAL | — | Damage path + empty-input; `block_align=0` exercised through the `.max(1)` guard. |
| test `stereo_group_interleave_keeps_channels_separate` | 271-299 | REAL | — | Channel-separation pin; frames arithmetic (1 seed + 8) verified. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `mono_block_decodes_the_pinned_ladder` | Yes | Five exact i16 values pinned against ffmpeg; any change to m-form, shift, sign, or index update breaks at least one (I recomputed: 2314, 2301=2314−(7·15>>3), 2331=2301+(16·15>>3), 2268=2331−(34·15>>3), 2132=2268−(73·15>>3) — all match the assertion). |
| `stereo_group_interleave_keeps_channels_separate` | Yes | Cross-wired granules would flip even/odd values. |
| `header_index_above_88_clamps_and_flags` / `block_smaller_than_header…` | Yes | Stats flags and skip counts asserted exactly. |
| Gap | — | No test for channels=0 (F-001) or misaligned block_align (F-002) — the two latent holes. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every decision pinned bit-exactly against the ffmpeg 7.1 reference on the real corpus (0042)" (lines 4-6) | module doc | REAL | `tests/adpcm_corpus_differential.rs` in baseline run (adpcm suite rows, orchrun log:255-301 all ok); in-file pinned values hand-verified. |
| "A block shorter than its 4·channels header is block-skipping damage" (46-47) | module doc | REAL | Test + code path (lines 138-143). |
| "Never panics; every index is bounds-checked" (126-127) | fn doc | REAL (technically) — and misleading | True for panics; but channels=0 hangs (F-001) — "never panics" is not "always terminates". |
| Block sample-count formula (line 16) | module doc | SHALLOW | Happy-path exact; misaligned shapes decode fewer (F-003). |

## 7. Dead / duplicated / unreachable code
None. All items consumed (`TAG_WAVE_FORMAT` by mod.rs's tag gate; tables by expand; stats by the codec-api layer).

## 8. Dependencies & flags
In-house, zero deps; `adpcm` unconditional (audio/mod.rs:37) — always shipped. No unsafe; no unwrap/expect; indexing all bounded (re-derived).

## 9. Open questions for the next agent
- `adpcm/mod.rs` (W07-A0x parts): does the codec-api layer treat `stats.frames == seed-only` blocks (F-002 shape) as clean, and does the WAV demuxer bound `nBlockAlign` before it reaches the plan? If the demuxer already refuses misaligned aligns, F-002's reach shrinks to hostile in-band size lies.
