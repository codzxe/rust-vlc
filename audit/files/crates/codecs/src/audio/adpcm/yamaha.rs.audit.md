# AUDIT: crates/codecs/src/audio/adpcm/yamaha.rs
| Field | Value |
|---|---|
| Auditor | W07-A09 |
| Date | 2026-11-30 |
| Lines / functions | 199 / 3 (+6 test fns) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — dispatch arm exists (apps/cli/src/pipeline_host.rs:298 `codec.starts_with("adpcm_")` -> AdpcmDecoder; configure accepts `adpcm_yamaha` at adpcm/mod.rs:293,515) but NO shipped container mints the `adpcm_yamaha` descriptor: WAV accepts only tags 1/3/0x0002/0x0011 (demux/src/wav/format.rs:14,44,50), ASF mints only `adpcm_ms`/`adpcm_ima_wav` (asf/header.rs:406,409), ES is h264/hevc only. rg for `"adpcm_yamaha"` outside crates/codecs: zero hits. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `./target/debug/deps/codecs-a0d7a93b7072ff58 audio::adpcm::yamaha` -> 6 passed (exit 0); `./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b` -> 4 passed incl. `breadth_corpus_is_bit_exact_against_ffmpeg_reference` (exit 0); sed/rg verification (exit 0) |

## 1. One-paragraph verdict
A faithful, bit-exact port of ffmpeg 7.1.5's `adpcm_yamaha_expand_nibble`: both tables match the reference (`ff_adpcm_yamaha_difflookup`/`ff_adpcm_yamaha_indexscale`), the C-truncation `/8` semantics are correctly kept as Rust `/` (and the negative-case test pins trunc-vs-floor: −241 not −242), the `step==0` first-nibble reset, both clamps (i16 predictor, step 127..=24576), and the low-nibble→status[0] / high-nibble→status[st] packing — proven by MD5 equality against ffmpeg's own encoder+decoder on committed mono and stereo corpora. The defects are reachability and one dead statistic: no shipped demuxer can ever hand this decoder a packet (the Dreamcast `.adp`/raw lane was never built), and `YamahaStats::frames` miscounts mono instants (counts bytes, not samples) — harmless only because the production caller discards it.

## 2. What actually works (verified)
- `./target/debug/deps/codecs-a0d7a93b7072ff58 audio::adpcm::yamaha` -> `test result: ok. 6 passed; 0 failed` — including the truncation-semantics pin `truncating_division_is_the_negative_case` (step 129, nibble 15 → −241; a `>>3` port would answer −242 and fail).
- `./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b` -> `test result: ok. 4 passed; 0 failed` — `breadth_corpus_is_bit_exact_against_ffmpeg_reference` MD5-matches ffmpeg-encoded `yamaha_mono_payload.bin` (18×1024 bytes) and `yamaha_st_payload.bin` (35×1024) against recorded digests (`aeddc262…`, `fcf6aeca…`, header table lines 17-18).
- Overflow analysis (read): max product `24576·15 = 368640` fits i32; predictor clamped every nibble; step `24576·614` fits i32 — no overflow path exists.
- Caller-safety analysis (read): `parse_raw_facts` (adpcm/mod.rs:698-738) bounds channels to 1..=2 and `yamaha_state` is sized `channels` at configure (mod.rs:530) — `state[st]` can never index out of bounds; `raw_out_bound` (payload·8+64) always covers the 4-bytes-per-byte output.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/audio/adpcm/yamaha.rs:1-4 (scope claim) vs the shipped container set
Code:
```
//! Yamaha ADPCM (card 5.J2b): the 4-bit codec of Sega Dreamcast `.adp`
//! and a family of game containers — a **headerless continuous nibble
//! **stream** with per-channel decoder state carried across packets, the
```
(line 2-3 of file: "a **headerless continuous nibble stream** with per-channel decoder state carried across packets")
What is wrong: nothing in the code — the reachability is the defect. The descriptor `adpcm_yamaha` is minted by no demuxer in the shipped CLI/desktop (WAV: tags 1/3/0x0002/0x0011 only; ASF: ms/ima only; MKV/MP4/OGG/TS/AVI/IVF/ES/AIFF/PS: no adpcm_yamaha lane). The card's own module table says "containers that are not WAV" (adpcm/mod.rs:17) — and none of those containers exist. Why it matters: this is real, tested, bit-exact code that no user-openable file can ever reach; the "breadth" card shipped the decoder but not a single one of its carrying containers. What correct looks like: either a raw/`.adp` container lane or an honest UNREACHABLE marker in the registry docs. Confidence: HIGH — Verification: read + rg (zero mint sites). UNVERIFIED: plugin-host-wasm could in principle mint arbitrary descriptors; no evidence it ships one.
F-002 [S4]
Where: crates/codecs/src/audio/adpcm/yamaha.rs:80-85 + 100-107
Code:
```
/// What one `decode_yamaha` call appended.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct YamahaStats {
    /// Sample instants appended (all channels interleaved).
    pub frames: u64,
```
and `frames += 1;` once per byte (line 105).
What is wrong: `frames` counts bytes, not sample instants. For stereo 1 byte = 1 instant (correct); for mono 1 byte = TWO mono instants (both nibbles expand channel 0) but frames increments once — the doc "Sample instants appended" is wrong for mono. Why it matters: today the sole production caller discards the value (adpcm/mod.rs:602 computes duration from `samples.len()/2/channels` instead — correct), so this is a misleading dead statistic that would silently halve mono durations if anyone ever wires it. What correct looks like: count `samples.len()/2/channels` or delete the struct. Confidence: HIGH — Verification: read + tests run.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| expand | 68-78 | REAL | — | bit-exact ffmpeg port; trunc `/8`, clamps, reset — all pinned |
| decode_yamaha | 92-108 | REAL | — | low→ch0, high→ch(st), state in place; stats = F-002 |
| YamahaChannel/YamahaStats types | 59-65, 81-85 | REAL | — | stats misleading (F-002) |
| 6 tests | 111-199 | REAL tests | — | table pins, reset pin, truncation pin, mono/stereo packing, state persistence |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| tables_are_the_pinned_7_1_5_pair | PARTIAL | spot-checks 8 of 32 entries — a mutated untested entry (e.g. INDEX_SCALE[2]) survives; the breadth MD5 is the real table oracle |
| fresh_state_seeds_127_and_zero | YES | exact 47/32/127 values |
| truncating_division_is_the_negative_case | YES | −241 exact; catches a `>>3` regression |
| mono_two_samples_per_byte_low_first / stereo_pair_keeps_channels_separate | YES | exact sample values incl. channel assignment |
| state_persists_across_calls | PARTIAL | asserts len + last-sample equality, not the exact second-byte value |
| breadth_corpus_is_bit_exact_against_ffmpeg_reference (integration) | YES | MD5 vs external ffmpeg reference, mono+stereo |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "bit-exact against the reference decoder's PCM on both the mono and stereo paths" | yamaha.rs:29-33 | REAL | breadth differential MD5s pass (ran, exit 0) |
| "Rust's `/` on integers truncates toward zero too, so the port is exact" | yamaha.rs:22-25 | REAL | test −241 pin + corpus |
| "damage cannot be detected, only decoded" | yamaha.rs:36-42 | REAL | no header grammar exists; every byte is lawful |
| nibble law (low=ch0, high=ch1, mono low-first) | yamaha.rs:8-10 | REAL | stereo_pair test + corpus |
| "the 4-bit codec of Sega Dreamcast .adp and a family of game containers" (implying carriers) | yamaha.rs:1-4 | FALSE (no carrier shipped) | F-001 |

## 7. Dead / duplicated / unreachable code
- The entire module is unreachable from shipped files (F-001) — kept alive by tests only.
- `YamahaStats` production-dead (F-002).

## 8. Dependencies & flags
Std-only, `pub(crate)` module behind no feature gate (in-house decoder, unconditional per Cargo.toml comment block). Callers: adpcm/mod.rs only.

## 9. Open questions for the next agent
- The other breadth flavors (`ima_oki`, `swf`, `ima_qt`, `ima_ws`) share the same no-carrier problem — whoever audits adpcm/mod.rs should treat F-001 as a family-wide reachability finding (mod.rs is a separate assignment).
- Does any matrix row claim ADPCM-yamaha playback on a platform with no container lane? Doc-audit territory.
