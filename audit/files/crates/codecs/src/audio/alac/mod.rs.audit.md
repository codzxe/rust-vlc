# AUDIT: crates/codecs/src/audio/alac/mod.rs
| Field | Value |
|---|---|
| Auditor | W07-A02 |
| Date | 2026-11-21 |
| Lines / functions | 83 / 7 items (module docs 1-47; 3 module decls + re-export 49-55; 1 test) |
| File verdict | REAL (clean glue; claims verified against code, engine source, record, and baseline runs) |
| Reachable from shipped entry point? | YES for `AlacDecoder` — call chain: `lumen open` (MP4 `alac` sample entry / MKV `A_ALAC` lane) → apps/cli/src/pipeline_host.rs:293-297 `"alac"` arm → `AlacDecoder::new` (requires the magic cookie in `extra_data`, seeded from the facts). `decode_frame`/`DecodedAlac`/`output_format` free surface: fuzz/tests only (the mod doc itself labels `decode_frame` "the fuzz entry" — disclosed tooling). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read mod.rs 1-83 · hand verification of the test's ALACSpecificConfig field offsets against the published cookie layout (frameLength BE [0..4], compatibleVersion [4], bitDepth [5], pb/mb/kb [6..9], numChannels [9], maxRun [10..12], maxFrameBytes [12..16], avgBitRate [16..20], sampleRate BE [20..24] — 24 bytes total: the test's writes at [0..4]=4096, [5]=16, [9]=2, [20..24]=44100 all land on the right fields) · `rg -n "alac" audit/runs/baseline/test-workspace.log.txt` (10 unit/fuzz tests green :348-357; `alac_bitexact::fixtures_bitexact_vs_ffmpeg` green :862) · `head` of the pinned engine `symphonia-codec-alac-0.6.1/src/lib.rs` (`#![forbid(unsafe_code)]` at line 9 — doc claim verified) · `ls docs/dependencies/records/` (symphonia-alac-0.6.1.md exists) · Read `crates/codecs/tests/alac_bitexact.rs` header (5 fixtures: stereo/mono × 16/24-bit + stereo24_noise; MD5 digests pinned; the suspected-engine-bug tripwire documented) · `rg -n "suspect\|truncated" crates/codecs/src/audio/alac/decoder.rs` (the referenced bit-level audit exists at decoder.rs:63-87) · `rg -n "alac" audit/waves/w07-a*.tsv` (module fully assigned: decoder part1=a05, part2=a08, error=a10, fuzz=a07) |

## 1. One-paragraph verdict
This is honest glue: every claim in the module docs checks out. The engine pin (`symphonia-codec-alac` 0.6.1) is real, exact-pinned in Cargo.toml, its `#![forbid(unsafe_code)]` verified in the actual pinned crate source, and the dependency record exists; the ALACSpecificConfig field offsets used by the module's one test are correct against the published 24-byte cookie layout (I re-derived all eleven fields); the bit-exactness claim (byte equality vs ffmpeg s16le/s24le dumps) is backed by a baseline-green test with five MD5-pinned fixtures covering stereo/mono × 16/24-bit plus a white-noise maximum-entropy stressor explicitly added as the tripwire for a prior session's suspected engine bug (`alac.frame.truncated` on multi-frame stereo — NOT reproduced, with the bit-level audit living in decoder.rs:63-87 exactly where the mod doc says); and the free-surface (`decode_frame`) is disclosed as the fuzz entry rather than passed off as shipped functionality. The only finding is that the test's damage assert is code-blind (`is_err()` without pinning the typed code), which is below the crate's own taxonomy-pinning discipline. The wave question's ALAC answer: the decoder is dispatched and reachable in the shipped build; no baseline shipped-binary ALAC smoke exists, but unit + fuzz + byte-exact differential all ran green through the same plan path.

## 2. What actually works (verified)
— Cookie field-offset verification (hand-derived): the test's `cookie[0..4]=4096u32 BE` (frameLength), `cookie[5]=16` (bitDepth), `cookie[9]=2` (numChannels), `cookie[20..24]=44_100u32 BE` (sampleRate) all match the published ALACSpecificConfig layout — so `output_format_matches_the_decode_mapping` genuinely verifies the parse mapping, not just self-consistency.
— Baseline evidence: `test-workspace.log.txt:348-357` — 10 alac tests green (decoder 4 incl. `cookie_damage_is_typed_per_rule` and `truncation_maps_to_the_typed_tripwire`, error 3, fuzz 2, mod 1); `:862` — `fixtures_bitexact_vs_ffmpeg` **ok** (byte equality vs ffmpeg, 5 fixtures, MD5-pinned digests in the test table at alac_bitexact.rs:23-27).
— Engine purity claim: `symphonia-codec-alac-0.6.1/src/lib.rs:9` — `#![forbid(unsafe_code)]` (read from the actual pinned registry source, not the record's say-so); record `docs/dependencies/records/symphonia-alac-0.6.1.md` exists.
— Doc-pointer integrity: the module map's "suspected-engine-bug audit documented there [decoder]" — verified: decoder.rs:63-87 carries the full NOT-reproduced bit-level audit; the bitexact header (29-42) records the tripwire role of `alac_stereo24_noise.m4a` and the honest 32-bit-gap note (ffmpeg's ALAC encoder quantizes s32p to 24 bits, so no 32-bit differential row is possible with that oracle).
- Reachability of the shipped lane: pipeline_host.rs:293-297 dispatches `"alac"`; `codec-alac` is in the codecs crate's `default` feature set (Cargo.toml) — the decoder ships.

## 3. Findings
### F-001 [S4]
Where: crates/codecs/src/audio/alac/mod.rs:79-81
Code:
```
        // A bare frame-less packet is structure damage on the free
        // surface, not a decode.
        assert!(decode_frame(&[], &cookie).is_err());
```
What is wrong: the assert checks only that SOME error occurs — not WHICH typed code. The crate's own discipline (and every sibling module's tests — e.g. `cookie_damage_is_typed_per_rule`, `truncation_maps_to_the_typed_tripwire`, the mp2/aac taxonomy tests) pins exact dotted codes; this one would pass if `decode_frame` regressed to returning the wrong class (e.g. a generic `alac.engine` instead of the documented empty-packet rule) or even a BadState misuse error.
Why it matters: the assert exists precisely to pin the damage model ("structure damage … not a decode") but cannot distinguish the documented error from any other; a taxonomy drift here fails silently.
What correct looks like: `let e = decode_frame(&[], &cookie).err().unwrap(); assert_eq!(e.context.code, "<the documented empty-packet code>");` (the exact code lives in decoder.rs — a05/a08's range).
Confidence: HIGH — Verification: read; the code-blindness is structural (no code string appears).

## 4. Per-function table (items — the file is docs + glue + one test)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs "Format slice" | 1-16 | REAL | — | owns exactly the raw-block + cookie; container framing disclaimed to the demux cards ✓ (matches the crate's layering) |
| module docs "Module map" | 18-28 | REAL | — | all four named items exist and are re-exported; `AlacConfig`/`DecodedAlac` are re-exported (53-55) but not map-documented — trivial doc gap, not numbered |
| module docs "Conventions" | 30-36 | REAL | — | "no unsafe on this module's own paths" — the mod itself has zero code paths; the engine-side `forbid(unsafe_code)` verified in the pinned source; "no panic on any input" is decoder.rs's claim — REAL-UNPROVEN from this file (fuzz tests `hostile_wide_inputs_survive`, `mutated_cookies_are_typed_not_panics` green support it) |
| module docs "Bit-exactness" | 38-47 | REAL | — | verified: the named test exists, ran green, 5 fixtures with MD5s; "the engine outputs planar, so the interleaved dump is the honest oracle" — decoder-side, a05/a08 to verify the interleave |
| `pub(crate) mod decoder; pub(crate) mod error;` | 49-50 | REAL | — | internal, fully assigned to wave-7 agents |
| `pub mod fuzz;` + re-exports | 52-55 | REAL | — | `pub mod fuzz` per the crate's registered-target convention (same as aac); free surface disclosed as fuzz entry |
| `tests::output_format_matches_the_decode_mapping` | 61-82 | REAL | F-001 | cookie offsets verified against the published layout; mapping asserts exact (S16/2ch/44.1k/interleaved); the damage assert is code-blind (F-001) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| output_format_matches_the_decode_mapping | YES for the mapping, NO for the damage code | hand-verified cookie offsets mean a parse-field regression (e.g. bitDepth read from [4]) fails the mapping asserts; but the empty-packet assert accepts any error class (F-001) |
| (external) fixtures_bitexact_vs_ffmpeg | YES | byte equality against MD5-recorded ffmpeg dumps across 5 fixtures × both bit widths — the strongest oracle class in this wave; any engine or wrapper drift fails |
| (external) alac fuzz survival ×2 + decoder/error unit ×7 | YES | exact-code taxonomy tests (per baseline names) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "frame decode through the pinned `symphonia-codec-alac` 0.6.1 engine … purity record … documented there" | mod.rs 9-12 | REAL | `=0.6.1` pin (codecs/Cargo.toml); `#![forbid(unsafe_code)]` verified in the pinned source lib.rs:9; record file exists |
| "the prior session's suspected-engine-bug audit are documented there [decoder]" | mod.rs 11-12 | REAL | decoder.rs:63-87 "The prior session's suspected engine bug (NOT reproduced)" — the audit exists where pointed |
| "differential tests … require byte equality against ffmpeg's own decoded PCM … oracle digests recorded next to the fixtures" | mod.rs 40-47 | REAL | alac_bitexact.rs:23-27 MD5 table; baseline :862 green |
| "no panic on any input — every decode path is bounds-checked" | mod.rs 33-36 | REAL-UNPROVEN (this file; decoder.rs is a05/a08's) | fuzz survival green (baseline :355-356, :866+); full path audit pending in decoder parts |
| doc 04 §3.1 row "ALAC" (lossless Apple codec, MP4/MKV payload) | mod.rs 1-2 citing doc 04 | REAL-UNPROVEN at shipped-binary level | dispatch + engine + bitexact all real; no shipped-binary ALAC smoke in baseline runs (the MP4 smoke samples are h264/aac/hevc) |

## 7. Dead / duplicated / unreachable code
- `decode_frame`/`DecodedAlac`/`output_format` free surface: unreachable from shipped entry points (fuzz/tests) — DISCLOSED by the module map ("the fuzz entry"), same honest-tooling pattern as AAC's `decode_stream`. Not fraud.
- No dead modules; error.rs is the 8-way-duplicated taxonomy skeleton (see my mp2/error.rs F-001 — alac/error.rs is one of the eight).

## 8. Dependencies & flags
- `symphonia-codec-alac` + `symphonia-core` `=0.6.1`, optional behind `codec-alac`, in `default` (codecs/Cargo.toml:61,67) — shipped builds carry it; checksum-pinned in Cargo.lock.
- No `unsafe` in this file; no feature-gated code in mod.rs itself.

## 9. Open questions for the next agent
- **W07-A05 / W07-A08 (decoder.rs parts)**: (a) the exact typed code for the empty-packet damage (F-001's fix needs it); (b) verify the "engine outputs planar, decoder interleaves" claim and the `alac.frame.truncated` mapping at decoder.rs:326-339 against the pinned engine's EOF behavior; (c) the NOT-reproduced engine-bug audit (63-87) — confirm the wrapper's truncation detection actually distinguishes engine EOF from short payloads.
- **W07-A07 (fuzz.rs)**: does the fuzz construct cookies through `parse_alac_config` (gated) or raw (ungated)? — same defensive-gate question I filed for ms.rs and aac.
