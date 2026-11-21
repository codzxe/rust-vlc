# AUDIT: crates/codecs/src/audio/vorbis/seek.rs
| Field | Value |
|---|---|
| Auditor | W07-A04 |
| Date | 2026-11-21 |
| Lines / functions | 176 / 9 units: `sample_to_nanos`, `nanos_to_sample`, `measure_shift`, `landing_first_sample`, `cold_block_emission`, `join_emission`, tests (3) |
| File verdict | REAL for `measure_shift` (the only live function — executed end-to-end via the ogg/vorbis seek + head-trim lanes, green in baseline); the other five functions are dead exports (verdict for them: UNREACHABLE/test-only) |
| Reachable from shipped entry point? | YES via `measure_shift` only. `lumen open x.ogg` → `demux/src/ogg` mints `vorbis` → `apps/cli/src/pipeline_host.rs:272-275` `VorbisDecoder::new` → decoder.rs:867/922/949 call `measure_shift` (the head-trim and re-anchoring core). Executed: `vorbis_bitexact::stream_head_encoder_delay_is_trimmed_decoder_side ... ok` (baseline :1261-1267), `ogg_vorbis_in_session_seek_replays_the_codec_headers_and_lands ... ok` (orchrun-test-workspace-nofailfast.log.txt:2966, apps/cli/tests/native_seek.rs:543-600 — exact 66,150-sample/1.5 s derivation), `ogg::tests::seek_and_reanchor_land_on_the_last_page_before_the_target ... ok` (:1867). `landing_first_sample`/`cold_block_emission`/`join_emission`/`sample_to_nanos`/`nanos_to_sample`: ZERO callers outside this file's tests (grep over crates+apps; only doc mentions + mod.rs re-export). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | Read 176 lines (full); grep each function name over crates/demux, crates/codecs, apps (caller map → 5 dead); Read decoder.rs:860-960 (the measure_shift call sites + the inline duration math duplicating `sample_to_nanos`); Read demux/src/ogg/mod.rs:239 (demuxer's own `granule_of_nanos`); baseline greps (:516-518, :1261-1267, :1867, orchrun :2960-2978); Read native_seek.rs:543-600 (the seek lane's exact sample-count derivation); sed verification of quoted ranges. |

## 1. One-paragraph verdict
One sixth of this file is load-bearing and excellent: `measure_shift` is the head-trim/re-anchor primitive the Vorbis decoder calls at three sites, and its behavior is pinned end-to-end by real execution — the bitexact suite proves the encoder delay is trimmed decoder-side, and the CLI in-session seek lane derives its landing and 1.5 s play-out from this arithmetic with hand-recorded sample counts (66,150 = final granule; 128-sample delay head-trim; 986-sample padding tail-clamp). The other five sixths is a museum: `landing_first_sample`, `cold_block_emission`, `join_emission`, `sample_to_nanos`, `nanos_to_sample` have no callers anywhere — the decoder computes emissions from the engine's decoded buffer inline and re-implements the ns math at decoder.rs:940-946, the demuxer has its own `granule_of_nanos` — yet the module doc sells all six as "the three arithmetic surfaces the codec needs" and claims "the seek roundtrip suite pins" `landing_first_sample`, which the suite never calls.

## 2. What actually works (verified)
— `measure_shift` (77-84): `raw_end − granule` on matching-base `Present` stamps, `None` on `Missing`/foreign base — exactly the re-anchor contract the decoder uses (`pts = raw_first − shift`, decoder.rs:906-912); head/mid/final-page shapes and both `None` arms pinned by unit test (baseline :517 ok) and by the executed lanes above.
— Time-conversion math (`sample_to_nanos`/`nanos_to_sample`, 44-59): correct floor semantics, u128 intermediates, rate-0 guard, u64::MAX clamp — verified by the roundtrip test (:518 ok; `nanos_to_sample(999_999_999, rate) == rate−1` floors).
— Emission law constants (`cold_block_emission` = n/2, `join_emission` = (prev+n)/4, 99-108): match the Vorbis I overlap-add shape the module doc states; correct arithmetic (2048→1024, (512+2048)/4=640) — pinned by unit test (:516 ok) even though nothing in production calls them.
— The live integration: seek lands at the page start, replays codec headers, closes the rebuffer, and plays to exactly 1,500,000 µs (native_seek.rs:573-580) — the shift arithmetic's end-to-end proof.

## 3. Findings
### F-001 [S3]
Where: crates/codecs/src/audio/vorbis/seek.rs:1-38 (module doc) + 44-59, 86-108 (five dead functions)
Code:
```rust
//! This module owns the three arithmetic surfaces the codec needs:
```
and (the false pin claim, lines 26-28):
```rust
//!    decoder's overlap reset pairs with this to make post-seek
//!    output identical to a cold start at that page (the seek
//!    roundtrip suite pins it).
```
What is wrong: of the six production functions, only `measure_shift` is called (decoder.rs:867/922/949). `landing_first_sample`, `cold_block_emission`, `join_emission`, `sample_to_nanos`, `nanos_to_sample` have zero callers outside this file's own tests (verified by name-grep over crates/demux, crates/codecs, apps — only the mod.rs:74-75 re-export and doc mentions). The decoder re-implements the emission bound inline (`decoder.rs:344` `ident.blocksize_long as usize / 2 * usize::from(ident.channels) * 4`, `:352` `u64::from(ident.blocksize_long) / 2`) and the ns duration inline (`:940-946`, byte-for-byte the `sample_to_nanos` formula); the demuxer carries its own `granule_of_nanos` (ogg/mod.rs:239).
Why it matters: the module doc misrepresents which surfaces are load-bearing — a maintainer "fixing" seek via `landing_first_sample` would be editing dead code, and the doc's "the seek roundtrip suite pins it" is false for that function (the suite pins the decoder, which never calls it). Five dead exports + three duplicated formula copies is drift bait.
What correct looks like: either wire the decoder's inline math through these functions (one source of truth) or delete the dead five and rewrite the module doc around `measure_shift`; drop the roundtrip-suite pin claim for anything not in the lane's call path.
Confidence: HIGH — Verification: read + exhaustive caller grep + decoder.rs/demux source reads; the executed lanes cited above prove which path is live.

### F-002 [S4]
Where: crates/codecs/src/audio/vorbis/seek.rs:118-122
Code:
```rust
            let base = match TimeBase::new(1, rate) {
                Ok(base) => base,
                Err(err) => unreachable!("{err:?}"),
            };
            let _ = base;
```
What is wrong: dead `base` construction in the loop (leftover from a removed assertion); the conversions under test don't take a base.
Why it matters: test noise that implies the conversions are base-checked when they are not.
What correct looks like: delete the three lines.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `sample_to_nanos` | 43-49 | REAL (dead) | S3 (F-001) | correct math; duplicated inline at decoder.rs:940-946 |
| `nanos_to_sample` | 51-59 | REAL (dead) | S3 (F-001) | correct floor; demuxer's `granule_of_nanos` is the live twin |
| `measure_shift` | 76-84 | REAL | — | the one live export; 3 call sites; `i64::try_from(raw_end).unwrap_or(i64::MAX)` clamp unreachable for real counters |
| `landing_first_sample` | 92-95 | REAL (dead) | S3 (F-001) | saturating_sub correct; NOT the decoder's landing rule |
| `cold_block_emission` | 97-101 | REAL (dead) | S3 (F-001) | correct law; decoder computes emissions from decoded buffer instead |
| `join_emission` | 103-108 | REAL (dead) | S3 (F-001) | correct law; same |
| tests: `time_conversions_roundtrip_at_common_rates` | 115-132 | REAL | S4 (F-002) | floors + degenerate rate; dead `base` leftover |
| tests: `shift_measurement_covers_head_seek_and_tail_shapes` | 134-159 | REAL | — | head/mid/tail + both None arms; hand-checked (10496−7936=2560, 34816−32000=2816) |
| tests: `landing_and_emission_arithmetic_pin_the_block_math` | 161-175 | REAL | — | hand-checked (640 joins, clamped landing); pins dead code — would survive production deletions |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 3 unit tests | YES for the functions they call | exact hand-computed values |
| but: `landing_and_emission…` | NO for production behavior | it pins functions nothing in production calls — deleting them from the decoder's perspective changes nothing the suite observes (F-001's blind spot) |
| the real seek pin lives in `native_seek.rs:543+` (CLI) and `vorbis_bitexact` (codecs) | YES | they exercise `measure_shift` through the decoder with exact sample-count/µs assertions |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "This module owns the three arithmetic surfaces the codec needs" | seek.rs:12-13 | FALSE (5 of 6 functions uncalled) | F-001 caller map |
| "post-seek output identical to a cold start at that page (the seek roundtrip suite pins it)" — about `landing_first_sample` | seek.rs:22-28 | FALSE as attributed | the roundtrip lane (native_seek.rs:543) never calls it; it pins decoder.rs's shift/hold machinery |
| "The decoder applies `pts = raw_first − shift` … re-measures at every stamp" | seek.rs:70-75 | REAL | decoder.rs:906-912, 867/922/949; executed lanes green (orchrun :2966) |
| Emission laws (n/2 cold, (prev+n)/4 join) | seek.rs:14-17, 97-108 | REAL (as math) / UNREACHABLE (as code) | unit-pinned; decoder derives emissions from the engine buffer instead |

## 7. Dead / duplicated / unreachable code
- Dead exports: `sample_to_nanos`, `nanos_to_sample`, `landing_first_sample`, `cold_block_emission`, `join_emission` (F-001) — plus their re-export at vorbis/mod.rs:74-75.
- Duplication: ns↔sample math exists in ≥4 copies across the audio tree (this file, decoder.rs:940-946 inline, adpcm mod.rs `duration_nanos`, pcm `PcmPlan::duration_nanos`, mp3 inline 592-595) — hygiene-level duplication, one canonical helper would serve all.

## 8. Dependencies & flags
- `media_types::time::{Stamp, TimeBase, Timestamp}` only; `codec-vorbis` feature gates the module (codecs/Cargo.toml:80, default-on :57) — the shipped CLI builds it. No unsafe, no panics (all `match`/`try_from` guarded).

## 9. Open questions for the next agent
- vorbis/decoder.rs owners (W07 parts): confirm the head-hold release path (`resolved = fresh.max(0)`) cannot mis-trim when the FIRST stamp arrives on the final page (the doc at decoder.rs:877-881 says flush answers that case un-trimmed — is there a test for a stream whose only stamp is the last page?).
- If the dead five get wired in as the single source of truth (F-001's fix), the `native_seek` lane's exact µs pins are the regression bar.
