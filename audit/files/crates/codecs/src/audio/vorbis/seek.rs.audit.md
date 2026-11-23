# AUDIT: crates/codecs/src/audio/vorbis/seek.rs
| Field | Value |
|---|---|
| Auditor | W07-A01 |
| Date | 2026-11-27 |
| Lines / functions | 177 / 6 fns (+3 tests) |
| File verdict | SPLIT: `measure_shift` REAL + shipped-reachable; the other 5 functions UNREACHABLE (test-only exports) |
| Reachable from shipped entry point? | YES for measure_shift only: vorbis/decoder.rs:101 `use super::seek::measure_shift;` used at decoder.rs:867,922,949 inside `VorbisDecoder::decode`, dispatched by apps/cli/src/pipeline_host.rs:271-274 (`"vorbis"`) / apps/web host. The other five: no caller in apps/ or crates/ outside this file's tests (census below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p codecs --lib audio::vorbis::seek` → 3 passed / 0 failed (exit 0); rg tree-wide census of all six export names (exit 0) |

## 1. One-paragraph verdict
The arithmetic in this file is correct — u128-intermediate time conversions with zero-rate guards and u64 clamping, a saturating landing clamp, and the standard Vorbis `(prev_n + n) / 4` join emission — and its 3 tests pass (I ran them). But five of its six functions are a museum: only `measure_shift` is wired into the shipped decoder; `landing_first_sample`, `cold_block_emission`, `join_emission`, `sample_to_nanos`, `nanos_to_sample` have zero callers anywhere in the repository outside their own tests, and the decoder *duplicates* the cold-emission math inline (`blocksize_long / 2` at decoder.rs:344,352) instead of calling the exported helper. The module doc goes further and claims the decoder's overlap reset "pairs with" `landing_first_sample` — it does not; the actual landing mechanism is shift re-derivation via `measure_shift` at the landing page's first stamp (decoder.rs:867-880).

## 2. What actually works (verified)
— verified by `cargo test -p codecs --lib audio::vorbis::seek` → `test result: ok. 3 passed; 0 failed` (rate round-trips at 8k/44.1k/48k incl. floor behavior and zero-rate guards; shift measurement across head/mid/tail/missing/foreign-base shapes; block math pins incl. mixed 512↔2048 joins).
— verified by census: rg `cold_block_emission|join_emission|landing_first_sample|sample_to_nanos|nanos_to_sample` over apps/ + crates/ (excluding seek.rs itself) → only decoder.rs:56, a doc comment. `measure_shift` → decoder.rs:101,867,922,949 (live).

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/audio/vorbis/seek.rs:86-108 (and 42-59) + vorbis/mod.rs:74-75 (re-export)
Code:
```rust
pub fn landing_first_sample(granule: u64, page_emitted: usize) -> u64 {
    granule.saturating_sub(page_emitted as u64)
}
```
What is wrong: `landing_first_sample`, `cold_block_emission`, `join_emission`, `sample_to_nanos`, `nanos_to_sample` are dead exports — no production caller anywhere. The decoder implements its landing via `measure_shift` re-derivation and computes the cold emission inline: decoder.rs:352 `u64::from(ident.blocksize_long) / 2` duplicates `cold_block_emission`. The module doc compounds it: "the decoder's overlap reset pairs with this to make post-seek output identical to a cold start at that page (the seek roundtrip suite pins it)" (seek.rs:25-28) — the overlap reset is real (decoder.rs:761-763 discontinuity drops overlap state), but nothing pairs with `landing_first_sample`; the roundtrip suite pins the decoder's own path, not this function. Claim-level verdict: FALSE for the "pairs with this" sentence. Why it matters: mod.rs:27-28 advertises all six as "the codec's seek surface"; five of them are unreachable math with a doc narrative that misdescribes the wiring. Confidence: HIGH — Verification: read + rg census.
F-002 [S4]
Where: crates/codecs/src/audio/vorbis/seek.rs:77-84
Code:
```rust
pub fn measure_shift(raw_end: u64, stamp: Stamp, base: &TimeBase) -> Option<i64> {
    match stamp {
        Stamp::Present(ts) if ts.base == *base => {
            Some(i64::try_from(raw_end).unwrap_or(i64::MAX) - ts.ticks)
        }
```
What is wrong: unchecked i64 subtraction. `i64::try_from(raw_end).unwrap_or(i64::MAX) - ts.ticks` overflows (debug panic, release wrap) when raw_end > i64::MAX and ts.ticks < 0, or with very negative ticks. Magnitudes required are physically absurd (raw_end > 9.2e18 samples ≈ millions of years of audio; negative granule stamps would have to come from a demuxer that mints them), so this is hygiene, not a live hole. Everything else in the file uses u128/checked/saturating arithmetic correctly. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| sample_to_nanos | 44-49 | REAL (tests) / UNREACHABLE shipped | F-001 | u128 intermediate, u64 clamp, rate-0 guard |
| nanos_to_sample | 54-59 | REAL (tests) / UNREACHABLE shipped | F-001 | same shape, floor semantics |
| measure_shift | 77-84 | REAL + shipped-reachable | F-002 | the only wired function (decoder.rs:867,922,949) |
| landing_first_sample | 93-95 | REAL (tests) / UNREACHABLE shipped | F-001 | saturating clamp correct; zero callers |
| cold_block_emission | 99-101 | REAL (tests) / UNREACHABLE shipped | F-001 | duplicated inline in decoder.rs:352 |
| join_emission | 106-108 | REAL (tests) / UNREACHABLE shipped | F-001 | correct Vorbis join formula; zero callers |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| time_conversions_roundtrip_at_common_rates | YES | exact 1-second identities + floor boundary + zero-rate guards |
| shift_measurement_covers_head_seek_and_tail_shapes | YES | numeric shifts for head/mid/tail + Missing/foreign-base → None |
| landing_and_emission_arithmetic_pin_the_block_math | YES | mixed-block joins (640) and clamped landing pinned |
All three prove the math; none can prove reachability, and F-001 shows five of six functions have none.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the decoder's overlap reset pairs with this [landing_first_sample]" | seek.rs:25-28 | FALSE | decoder.rs never calls it (census); landing is shift re-derivation via measure_shift (decoder.rs:867-880) |
| "`page_emitted` is what the demuxer's landing page hands the decoder" | seek.rs:90-91 | UNVERIFIED | no demuxer→decoder hand-off of this kind exists in-tree (demux cannot depend on codecs); could not find any producer of `page_emitted` outside tests |
| mod map lists all six as the codec's seek surface | vorbis/mod.rs:27-28 | PARTLY FALSE | only measure_shift is on a shipped path |

## 7. Dead / duplicated / unreachable code
Five dead exports (F-001); `cold_block_emission`'s math duplicated inline at decoder.rs:344,352. Same family pattern as mp2/seek.rs (correct, tested, unwired seek planners) — this looks systemic across the codec cards' "+seek" surfaces.

## 8. Dependencies & flags
`media_types::time` only; std. Behind default feature `codec-vorbis` (enabled in shipped default build) — compiled, mostly uncalled.

## 9. Open questions for the next agent
- vorbis/decoder.rs (another agent's file): does its seek-roundtrip test suite reference `landing_first_sample` semantics, and is the "seek roundtrip suite pins it" claim (seek.rs:27-28) backed by any test that would fail if the landing arithmetic changed? If not, that sentence is doubly false.
- Same dead-export census should be run for flac/seek.rs and mp3/seek.rs (vbr_toc_seek also appeared test-only in my mp2 census).
