# AUDIT: crates/codecs/src/audio/adpcm/ms.rs
| Field | Value |
|---|---|
| Auditor | W07-A02 |
| Date | 2026-11-21 |
| Lines / functions | 325 / 13 (2 consts + 1 const-table + 5 fns/structs/impl + 5 tests + 1 test helper) |
| File verdict | REAL (spec-correct, differentially bit-exact vs ffmpeg on corpus+fixtures; defensive gaps only) |
| Reachable from shipped entry point? | YES — call chain: `lumen open` (WAV/ADPCM lane) → apps/cli/src/pipeline_host.rs:298-302 `codec.starts_with("adpcm_")` → `AdpcmDecoder::new` seeded → adpcm/mod.rs:586 `ms::decode_ms_blocks` (via `Plan::Ms`, built by `configure_wav` mod.rs:346-431). No shipped-binary ADPCM smoke exists in baseline (AVI first-vertical is PCM), but the codec's own plan path is executed green in baseline tests including the corpus differential. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | Read ms.rs 1-325 · hand re-derivation of the pinned ladder (239/−145/863/3278 from delta 16→48→144→345, ADAPTATION trunc math) · Read mod.rs 280-431 (gates: channels 0 and >64 refused at 320-325; block_align 0 refused at 334-338; MS channels > 2 refused at 358-368; coef table must equal STANDARD at 407) · Read fuzz.rs 40-95 (fuzz synthesizes 1-2 channels, goes through configure — gates apply) · `rg -n "decode_ms_blocks\|MsFacts"` (single caller mod.rs:586) · baseline test-workspace.log.txt:311-315 (5 ms unit tests green), 822-840 (`adpcm_breadth_differential` 4/4, `adpcm_corpus_differential` 2/2, `adpcm_fuzz_survival` green) · `sed -n` quote verification below · `rg -n "S16\|native" crates/media-types/src/frame.rs` (S16 contract = "native endian") |

## 1. One-paragraph verdict
The MS-ADPCM core is correct: I hand-re-derived the pinned mono ladder (nibble 8 with delta 16: 367·256/256 − 8·16 = 239; delta 16→48→144→345 by truncated ×ADAPTATION/256 — every asserted sample in `mono_block_decodes_the_pinned_ladder` reproduces), the block-header field layout (predictors, then deltas, then samp1, then samp2, per channel — 7·channels bytes) matches the spec/ffmpeg layout the module documents, the output order (s2, s1, then nibble instants, channel-interleaved) is right, and the C-truncation semantics (`/256` toward zero, delta floor 16) are faithfully reproduced with i64 intermediates that refuse to mirror ffmpeg's wrapping `int`. The damage model (predictor > 6 or short header → block skipped and counted; short tail → prefix decoded and flagged) is concealment-with-ledger, never a panic, never silent. External proof is strong: `adpcm_corpus_differential::real_corpus_decodes_bit_exact` (corpus 0102, stereo 32 kHz, pinned MD5 `a1de073a…`) and `committed_fixtures_decode_bit_exact` (mono fixture MD5 `a95d2cbb…`) both pass in baseline — so the one path with no in-file unit test (the stereo hi=ch0/lo=ch1 nibble interleave) IS differentially pinned. Three S4 findings: an unreachable `states[0]` panic if `facts.channels == 0` (every in-tree caller gates it), a module-doc geometry sentence that undercounts mono output by 2×, and a dead emptiness check plus a theoretical `u32` skip-counter overflow.

## 2. What actually works (verified)
— Hand re-derivation of the ladder test: expand(nibble=8) = (367·256 + (−226)·0)/256 + (−8)·16 = 239; delta = 16·768/256 = 48; expand(8) = 239 + (−8)·48 = −145; delta = 48·768/256 = 144; expand(7, signed +7) = 239… next predictor = (−145·256 + 367·0)/256 + 7·144 = −145 + 1008 = 863; delta = 144·614/256 = 345 (trunc); expand(7) = (863·256 − 145·0)/256 + 7·345 = 863 + 2415 = 3278. Asserted `[-226, 367, 239, -145, 863, 3278]` — exact match, including the seed order s2-then-s1.
— Baseline `audit/runs/baseline/test-workspace.log.txt:311-315` — all 5 ms unit tests green; `:822-840` — `breadth_corpus_is_bit_exact_against_ffmpeg_reference`, `real_corpus_decodes_bit_exact` (0102 stereo + 0042 IMA), `committed_fixtures_decode_bit_exact`, adpcm fuzz survival all green.
— Gates verified in the caller (mod.rs): channels 0/>64 refused (320-325), block_align 0 refused (334-338), MS >2 channels refused with the exact nibble-packing rationale (358-368), wNumCoef must be 7 and the file's table must byte-equal `STANDARD_COEFFICIENTS` (382-408) — so `MsFacts` reaching `decode_ms_blocks` always has 1..=2 channels, a nonzero block_align, and the standard table.
— `push_sample`'s `to_ne_bytes` is the DOCUMENTED contract (`media-types/src/frame.rs:18` — "Signed 16-bit, native endian"); the producer side is correct (see F-003 note for the consumer side).

## 3. Findings
### F-001 [S4]
Where: crates/codecs/src/audio/adpcm/ms.rs:209-212 (with 144-176)
Code:
```
            _ => {
                for byte in nibble_bytes {
                    push_sample(out, states[0].expand(byte >> 4));
                    push_sample(out, states[1].expand(byte & 0x0F));
```
What is wrong: `decode_ms_blocks` trusts `facts.channels ∈ 1..=2`. With `channels == 0`: `header == 0`, the length check at 148 passes for any block, the state loop builds zero states, `states.len() != channels` (0 != 0) passes, `stats.frames += 2` miscounts, and the `_` arm indexes `states[0]` of an EMPTY vec — panic. With `channels ≥ 3` the `_` arm silently decodes only two of the channels (corrupt interleave, no panic).
Why it matters: it is a pub(crate) function whose safety rests entirely on caller discipline; the fuzz harness synthesizes 1-2 channels and the plan gates refuse 0 and >2 (verified), so it is unreachable today — but a new flavor wired to `decode_ms_blocks` without the MS gate inherits a panic or silent channel loss on hostile bytes.
What correct looks like: an up-front `if facts.channels == 0 || facts.channels > 2 { return stats }` (or a debug_assert + documented precondition), making the local invariant local — same fix shape as the AAC `adts_engine` finding (W07-A02 aac/decoder.rs#part1 F-001).
Confidence: HIGH — Verification: read; all in-tree callers gated (mod.rs:320,358; fuzz.rs raw_facts_skeleton 1+a%2).

### F-002 [S4]
Where: crates/codecs/src/audio/adpcm/ms.rs:9-10 (module doc)
Code:
```
//! One `nBlockAlign`-byte block decodes `(block_align − 7·channels)`
//! nibble bytes → that many extra sample instants, plus 2 header
```
What is wrong: the geometry sentence is stereo-only. For mono the code emits **two** instants per nibble byte (hi then lo, lines 202-207), i.e. `2·(block_align − 7)` extra instants, not `block_align − 7`. The code is spec-correct (wSamplesPerBlock = 2 + 2·(ba−7) mono, 2 + (ba−14) stereo — both match the code's frame accounting at 193/207/214); only the doc undercounts mono by 2×.
Why it matters: the module doc is the block-grammar reference for the next maintainer; a reader computing expected output sizes from it gets mono wrong by a factor of 2.
What correct looks like: "`(block_align − 7·channels)` nibble bytes → one instant per byte per channel pair (mono packs two instants per byte, stereo one)".
Confidence: HIGH — Verification: read + arithmetic above; mono path confirmed by the mono ladder test (2 nibble bytes → 4 samples).

### F-003 [S4]
Where: crates/codecs/src/audio/adpcm/ms.rs:148-152 (dead branch), 118 (u32 counter)
Code:
```
        if block.len() < header {
            if !block.is_empty() {
                stats.skipped_blocks += 1;
            }
            continue;
        }
```
What is wrong: (a) `!block.is_empty()` is dead — `slice::chunks` never yields an empty chunk, so the guard is always true; it implies an emptiness case that cannot occur. (b) `skipped_blocks: u32` counts one skip per `block_align`-or-smaller chunk; with `block_align == 0` (gated at configure, but not here — `chunks(block_align.max(1))` then yields one byte-chunk per input byte) a ≥4 GiB hostile buffer would overflow the u32 (panic in debug, wrap in release) — the `.max(1)` silently converts a nonsense block size into maximal skip counting instead of refusing.
Why it matters: dead conditions mislead readers about the damage model; the counter is the concealment ledger that the codec-api layer maps to `DecodeOutcome::Recovered`, so a wrapped count under-reports damage.
What correct looks like: drop the dead branch; make the counter u64 (frames already is) or refuse `block_align == 0` locally.
Confidence: HIGH (dead branch — `chunks` semantics), MEDIUM (overflow — requires a 4 GiB buffer past every gate; theoretical).
— Verification: read; `block_align == 0` reachability ruled out by mod.rs:334-338.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-40 | REAL | F-002 | grammar/damage-model docs otherwise accurate (field layout verified against code; damage model matches code) |
| `TAG_WAVE_FORMAT` | 43 | REAL | — | 0x0002 ✓ MS-ADPCM |
| `STANDARD_COEFFICIENTS` | 48-56 | REAL | — | the 7 published pairs; caller enforces file tables byte-equal (mod.rs:407) |
| `ADAPTATION` | 59-61 | REAL | — | 16-entry table matches ffmpeg's `ff_adpcm_adaptation_table` (230…768…230) |
| `MAX_PREDICTOR_INDEX` | 64 | REAL | — | 6 (7 pairs) ✓ |
| `header_bytes` | 68-70 | REAL | — | 7·channels ✓ |
| `ChannelState` struct | 73-78 | REAL | — | block-local state; i32 fields hold clamped-i16 values (invariant maintained in expand) |
| `ChannelState::expand` | 81-107 | REAL | — | hand-verified ladder; i64 intermediates (no wrap); predictor clamp ±32768/32767 before store = ffmpeg's `av_clip_int16`; delta update reads OLD delta then assigns (order matches reference); delta clamp [16, 32767] — the upper bound is a documented divergence from ffmpeg's wrapping int, unreachable for clean streams (corpus stays 4 orders below); `/` trunc-toward-zero = C semantics ✓ |
| `MsStats` | 112-121 | REAL | F-003 | frames = instants (u64 ✓); skipped_blocks u32 (see F-003); concealed_tail bool |
| `MsFacts` | 124-131 | REAL | — | channels + coefficients; always standard table at runtime (gate) — the per-file intake is a full cross-check, making the field redundant-but-harmless |
| `decode_ms_blocks` | 138-219 | REAL | F-001, F-003 | chunks walk; short-header skip; predictor gate; field-interleaved header parse (offsets verified: delta at c+2ch, s1 at 3c+2ch, s2 at 5c+2ch); seed order s2-then-s1; mono 2 instants/byte, stereo 1/byte (hi=ch0/lo=ch1); `block[header..]` safe (length check above); no panics on any gated input |
| `push_sample` | 224-226 | REAL | — | `to_ne_bytes` = documented S16 native-endian contract (frame.rs:18); consumer-side LE hard-code is convert.rs's defect (W07-A10's file — see §9) |
| `tests::facts` (helper) | 232-237 | REAL | — | mono standard facts |
| `standard_table_matches_the_published_pairs` | 239-254 | REAL | — | change-detector vs literal copy — would catch a typo'd edit, proves nothing about the spec (the corpus fmt gate at mod.rs:407 does the real verification) |
| `mono_block_decodes_the_pinned_ladder` | 256-279 | REAL | — | hand-verified exact ladder (I reproduced every sample); strong |
| `predictor_out_of_table_skips_the_block_typed` | 281-293 | REAL | — | pin of the skip path |
| `block_smaller_than_header_is_skipped_not_panicked` | 295-304 | REAL | — | short block + zero-input no-hang |
| `negative_header_delta_heals_to_the_floor` | 306-324 | REAL | — | delta −300 → floor 16 on first expand; arithmetic verified ((−300·230)/256 = −269 → clamp 16) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| mono_block_decodes_the_pinned_ladder | YES | exact 6-sample ladder incl. seeds and the ×3 delta growth; mutation: swap seed order (s1 before s2) flips it; mutation: `/255` or `>>8`-with-different-rounding flips sample values |
| predictor_out_of_table / block_smaller_than_header | YES | exact stats asserts (skipped=1, frames=0, empty out) |
| negative_header_delta_heals | YES | exact 6×1000 output; a wrong clamp bound (e.g. 0 instead of 16) changes sample 3+ |
| standard_table_matches | WEAK | tautology vs its own constant — change-detector only; the real table verification is mod.rs:407 + corpus differential |
| MISSING: stereo in-file test | — | the `_` arm (hi=ch0/lo=ch1) has no unit test; covered externally by `real_corpus_decodes_bit_exact` (0102 stereo MD5 pin, baseline green) — acceptable but in-file pinning would localize regressions |
| MISSING: concealed_tail unit test | — | no test constructs a short trailing block and asserts `concealed_tail == true` (the flag is set at 197-199 but never asserted anywhere in this file); the corpus files are well-formed so the differential cannot cover it either — the flag's correctness is UNVERIFIED by tests |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "every packing decision pinned bit-exactly against the ffmpeg 7.1 reference on the real corpus (0102)" | module docs 4-5 | REAL | baseline `adpcm_corpus_differential::real_corpus_decodes_bit_exact` ok (test-workspace.log.txt:837), corpus 0102 stereo MD5 `a1de073a…` pinned in the test table |
| "(block_align − 7·channels) nibble bytes → that many extra sample instants" | module docs 9-10 | FALSE for mono | code emits 2 instants/byte mono (202-207); F-002 |
| "Damage model: … block-skipping … concealed_tail … never panics and never silent output" | module docs 35-40 | REAL (gated inputs) / panic exists ungated | skip+count paths tested; channels=0 panic is F-001 (unreachable through gates); `concealed_tail` set-but-never-asserted (§5) |
| ADPCM family rows (`adpcm_ms` tag 0x0002 mapping) | codecs/src/audio/adpcm/mod.rs:16 table | REAL | mod.rs:351 tag cross-check + ms tests green |

## 7. Dead / duplicated / unreachable code
- `!block.is_empty()` (149) — dead condition (F-003a).
- `MsFacts::coefficients` — always equal to `STANDARD_COEFFICIENTS` at runtime (configure gate at mod.rs:407 refuses any other table); the field exists so the intake is a cross-check rather than a trust — deliberate, documented (45-47), harmless.
- No unreachable functions; single caller chain verified.

## 8. Dependencies & flags
- No external deps (in-house decoder per the crate's D-018 doctrine); std only.
- No feature gate for adpcm (in-house decoders are unconditional per codecs/Cargo.toml's feature comment) — always compiled in shipped builds.
- No `unsafe` (verified by read).

## 9. Open questions for the next agent
- **W07-A10 (audio-engine/src/convert.rs)**: `s16_at`/`f32_at`/`s24_at`/`s32_at`/`f64_at` hard-code `from_le_bytes` while the `SampleFormat` contract in media-types/src/frame.rs:18-22 says **native endian** and every producer (ms.rs:225, aac decoder.rs:559, opus decoder.rs:450, adpcm ima.rs:204) writes `to_ne_bytes`. On big-endian hosts every audio path produces garbage; on the shipped LE targets it is latent. One side must change (or the contract must say LE). This is the third file where I've hit the same seam — the fix belongs in convert.rs or the contract doc, not in the four producers.
- **W07-A03/A04 (adpcm/mod.rs)**: confirm the `MsStats.concealed_tail` flag is actually mapped into a `DecodeOutcome::Recovered` shape somewhere in the plan layer (no test asserts the flag — see §5) and that `Stats.skipped` propagation survives.
- Corpus owner: corpus 0102's fact-chunk sample count (128_000) disagrees with the decoded 128_524 (noted in adpcm_corpus_differential.rs:11-14) — the demuxer suite owns that discrepancy; flagged so it is not rediscovered as a decode bug.
