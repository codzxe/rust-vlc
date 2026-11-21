# AUDIT: crates/codecs/src/audio/adpcm/yamaha.rs
| Field | Value |
|---|---|
| Auditor | W07-A04 |
| Date | 2026-11-21 |
| Lines / functions | 199 / 10 units: `DIFF_LOOKUP`, `INDEX_SCALE`, `YamahaChannel`, `expand`, `YamahaStats`, `decode_yamaha`, tests (`samples` helper + 6 tests) |
| File verdict | UNREACHABLE (real, bit-exact code; no shipped entry point mints the `adpcm_yamaha` descriptor — executed only by unit/differential tests) |
| Reachable from shipped entry point? | NO. `grep -rn "adpcm_yamaha" crates/demux/src apps/` → zero hits: no demuxer mints the descriptor, and the raw tag-0 facts skeleton has no producer outside tests. The dispatch arm exists (`apps/cli/src/pipeline_host.rs:296-300` `codec.starts_with("adpcm_")`), but nothing feeds it `adpcm_yamaha`. Module docs are honest about it (adpcm/mod.rs:48-52: "none of which have demuxers yet (future cards…)"). Executed in TEST lane: baseline `test-workspace.log.txt:342-347` (6 `yamaha::tests` ok) + `:341` (`mod.rs` wrapper state test) + `adpcm_breadth_differential` yamaha mono+stereo rows vs ffmpeg with recorded MD5s (:822-830 ok). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | Read 199 lines (full, twice for the nibble-law check); nibble arithmetic hand-simulated against the documented law and ffmpeg semantics (trunc-vs-floor cases recomputed); grep `adpcm_yamaha\|adpcm_ima_oki\|adpcm_swf\|adpcm_ima_qt\|adpcm_ima_ws` over crates/demux + apps (zero → unreachable); grep `decode_yamaha\|YamahaStats` callers (single: mod.rs:602, return discarded); baseline log greps (:341-347, :822-830); sed verification of quoted ranges. |

## 1. One-paragraph verdict
A clean, tiny, exactly-pinned port of ffmpeg's Yamaha ADPCM nibble law: the two lookup tables match the 7.1.5 source, the truncating-`/8` (not `>>3`) subtlety is both implemented correctly and pinned by a dedicated test that constructs the −241-vs−−242 discriminator by hand, the `step==0` fresh-state sentinel reproduces ffmpeg's first-nibble behavior, and the mono/stereo nibble packing (low→ch0, high→ch1) is value-asserted. Bit-exactness against the ffmpeg reference on real NUT-carried mono and stereo payloads is recorded and passed in baseline. The two defects are hygiene: `YamahaStats` is returned by `decode_yamaha` and thrown away by its only caller (and its doc mislabels byte count as "sample instants" for the mono shape), and `expand`'s reset branch zeroes `predictor` where the ffmpeg original does not — an unobservable divergence (no reachable state distinguishes them) that still contradicts the doc's "exact" claim.

## 2. What actually works (verified)
— Nibble law hand-verified: fresh state, nibble 1 → `(127·3)/8 = 47` (trunc), step `(127·230)>>8 = 114` clamped back to 127 (test :142-143 asserts exactly this); nibble 8 → `(127·−1)/8 = −15` trunc-toward-zero → 47−15 = 32 (test :147); step-129/nibble-15 → `(129·−15)/8 = −241` where a flooring shift gives −242 (test :159) — the C-semantics discriminator is real and would fail under `div_euclid`/`>>3`.
— Tables match ffmpeg 7.1.5's `ff_adpcm_yamaha_difflookup`/`ff_adpcm_yamaha_indexscale` (values cross-checked against the published source shape; the symmetric second half of INDEX_SCALE is asserted at :129-131).
— Bit-exact vs the reference: `adpcm_breadth_differential` rows `adpcm_yamaha` mono/stereo 22050 Hz with recorded PCM MD5s (`aeddc262…`, `fcf6aeca…`) — `breadth_corpus_is_bit_exact_against_ffmpeg_reference ... ok` in baseline (:822-830).
— Stereo packing pinned by value: byte `0x18` → low nibble 8 → ch0 −15, high nibble 1 → ch1 +47 (:183-184) — matches the doc's low=ch0/high=ch1 law and ffmpeg's `status[0]`/`status[st]`.
— Cross-packet state: `state_persists_across_calls` asserts the second call's last sample equals the carried predictor (:197).
— Bounds: `state[st]` indexing is safe under the mod.rs invariant (`state.len() == plan.channels()`, allocated at configure, mod.rs:530); output is exactly 4 bytes per input byte, within `raw_out_bound`'s 8× (mod.rs:760).

## 3. Findings
### F-001 [S4]
Where: crates/codecs/src/audio/adpcm/yamaha.rs:80-85 (struct) + 92-108 (discarded return)
Code:
```rust
pub(crate) struct YamahaStats {
    /// Sample instants appended (all channels interleaved).
    pub frames: u64,
}
```
What is wrong: `decode_yamaha`'s only caller (`mod.rs:602`) discards the return value outright, so `YamahaStats` is dead code; and its doc is wrong for the mono shape — `frames` counts BYTES (one per iteration, :105), but a mono byte appends TWO channel-0 instants, so "sample instants appended (all channels interleaved)" is false for mono and only accidentally true for stereo.
Why it matters: dead surface plus a mislabeled field invites a future caller to trust `frames` as an instant count and compute durations wrong (mono duration would be halved).
What correct looks like: either delete `YamahaStats`, or return real instants (`frames * channels.min(2)`-style accounting) and have `mod.rs` consume it.
Confidence: HIGH — Verification: read + single-caller grep (mod.rs:602 binds nothing).
### F-002 [S4]
Where: crates/codecs/src/audio/adpcm/yamaha.rs:67-72
Code:
```rust
/// Expand one nibble (ffmpeg's `adpcm_yamaha_expand_nibble`, exact).
pub(crate) fn expand(state: &mut YamahaChannel, nibble: u8) -> i16 {
    if state.step == 0 {
        state.predictor = 0;
        state.step = 127;
    }
```
What is wrong: ffmpeg's branch is `if (!c->step) c->step = 127;` — it does NOT zero the predictor. The Rust port adds `state.predictor = 0`.
Why it matters: the doc claims "exact"; the port is not line-for-line. The divergence is unobservable through every reachable construction (`YamahaChannel` is only ever built `Default` (predictor 0, step 0), and the step floor of 127 means `step` can never return to 0 after an update), so bit-exactness is safe — but the "exact" claim is doing work it shouldn't, and a future caller hand-building a `{predictor: x, step: 0}` state would diverge from the reference.
What correct looks like: drop `state.predictor = 0;` (matching ffmpeg), or weaken the doc to "exact for all reachable states".
Confidence: HIGH — Verification: read + ffmpeg semantics from the module's own pinned description (:11-13) and state-construction grep (mod.rs:530 `vec![YamahaChannel::default(); channels]`).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `DIFF_LOOKUP` | 44-46 | REAL | — | matches the published 7.1.5 table; symmetric signed ladder |
| `INDEX_SCALE` | 48-55 | REAL | — | matches; symmetric halves asserted in test |
| `YamahaChannel` | 57-65 | REAL | — | pub(crate); only constructed Default (invariant for F-002) |
| `expand` | 67-78 | REAL | S4 (F-002) | trunc-`/8` correct; clamp order correct (predictor clamp BEFORE step update, matching ffmpeg); `predictor as i16` lossless (clamped) |
| `YamahaStats` | 81-85 | REAL (dead) | S4 (F-001) | discarded by sole caller; mislabeled for mono |
| `decode_yamaha` | 92-108 | REAL | — | mono/stereo packing per doc; 4 output bytes/input byte; per-sample `extend_from_slice` (feeds mod.rs#part2 F-001's intermediate-Vec finding — the allocation lives here); `state[st]` safe under caller invariant |
| tests: `samples` | 114-120 | REAL | — | NE-bytes → i16 helper |
| tests: `tables_are_the_pinned_7_1_5_pair` | 122-132 | REAL (partial) | — | spot-checks 8 of 32 entries; full coverage via the bit-exact differential |
| tests: `fresh_state_seeds_127_and_zero` | 134-149 | REAL | — | hand-computed values (47/32/127) — would fail on reset or trunc bugs |
| tests: `truncating_division_is_the_negative_case` | 151-160 | REAL | — | the −241-vs−−242 discriminator; kills `div_euclid`/`>>3` mutations |
| tests: `mono_two_samples_per_byte_low_first` | 162-170 | REAL | — | count-only (values pinned by differential) |
| tests: `stereo_pair_keeps_channels_separate` | 173-185 | REAL | — | value assertions both channels; catches nibble-swap and state-bleed |
| tests: `state_persists_across_calls` | 187-198 | REAL | — | carried-state equality by value |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `fresh_state_seeds_127_and_zero`, `truncating_division_is_the_negative_case`, `stereo_pair_keeps_channels_separate` | YES | exact hand-computed sample values incl. the C-truncation edge and the channel split |
| `tables_are_the_pinned_7_1_5_pair` | PARTIALLY | 8/32 entries; a wrong untested entry survives it — but not the bit-exact differential (real ffmpeg-encoded payloads) |
| `mono_two_samples_per_byte_low_first` | YES (weakly) | count only |
| `state_persists_across_calls` | YES | state equality + last-sample==predictor |
| `adpcm_breadth_differential` yamaha rows (integration) | YES | recorded MD5s of ffmpeg reference decode, mono + stereo |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "bit-exact against the reference decoder's PCM on both the mono and stereo paths" | yamaha.rs:31-33 (module doc) | REAL (test lane) | breadth differential MD5s, baseline :822-830 ok |
| "the port is exact (`div_euclid` would NOT be)" | yamaha.rs:22-25 | REAL | `truncating_division_is_the_negative_case` pins −241 |
| "Expand one nibble (ffmpeg's …, exact)" | yamaha.rs:67 | REAL-EXCEPT-RESET-BRANCH | F-002: extra `predictor = 0` (unobservable) |
| doc 04 §3.1 ADPCM breadth rows imply shipped yamaha decode | (via adpcm/mod.rs docs) | UNREACHABLE | no demuxer mints `adpcm_yamaha`; module doc itself says the containers "have [no] demuxers yet" |

## 7. Dead / duplicated / unreachable code
- The ENTIRE FILE is unreachable from the shipped binary (descriptor never minted) — honest gap, documented in mod.rs:48-52; the code itself is real and tested.
- `YamahaStats` dead (F-001). No duplication (single implementation of the law).

## 8. Dependencies & flags
- None: std-only, unconditional compile (in-house ADPCM set). No feature gate — dead weight in every shipped build until a Sega-.adp-style demuxer lands (bounded: 199 lines).

## 9. Open questions for the next agent
- mod.rs#part1 owner: `decode()`'s `filled = samples.len()/2/channels.max(1)` — for mono yamaha, 2 samples/byte are instants of the ONE channel, so `filled` counts them correctly; confirm the same for stereo (1 instant/byte) — both consistent.
- If a .adp demuxer ever lands, it must synthesize the 16-byte tag-0 skeleton (`parse_raw_facts` contract, mod.rs:698-737) — nothing produces it today.
