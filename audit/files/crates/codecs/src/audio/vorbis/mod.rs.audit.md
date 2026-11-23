# AUDIT: crates/codecs/src/audio/vorbis/mod.rs
| Field | Value |
|---|---|
| Auditor | W07-A06 |
| Date | 2026-11-23 |
| Lines / functions | 76 / 0 (module organizer: 6 mod decls + re-exports) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — the module's VorbisDecoder is constructed at apps/cli/src/pipeline_host.rs:271 (`"vorbis" =>` arm); `codec-vorbis` is in codecs' default features |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | prebuilt `./target/debug/deps/codecs-a0d7a93b7072ff58 vorbis::` -> 20 passed; prebuilt `vorbis_bitexact-e8223d5cf8b5f437` -> 5 passed (0.57s); prebuilt `vorbis_vlc_differential-3a9245a4339ccea0` -> 1 passed; rg verification of every doc claim (unsafe, engine forbid, mkv seam, seek-export users) |

## 1. One-paragraph verdict
This is a 76-line module organizer whose value is its documentation contract — and that contract checks out on every claim I could execute or trace. "No `unsafe` on this module's own paths": rg finds zero unsafe tokens in the vorbis module (only the doc lines that mention the word). "the engine's crates are `#![forbid(unsafe_code)]`": verified at symphonia-codec-vorbis-0.6.1/src/lib.rs:9. "Native-Vorbis-in-Matroska reuses the same header chain": the MKV lane exists (A_VORBIS → "vorbis" descriptor, CodecPrivate carries the headers — demux/src/mkv/header.rs:811,158). The differential posture (exact sample count + bounded float delta + silence fixture byte-exact lane): the 5-test bitexact suite executed and passed. The VLC hook is "registered and honestly pending infra": the test exists, runs, and computes the Lumen-side digests against the ffmpeg oracle while skipping the VLC half (no binary in sandbox — verified by the test's own header and its passing run). The 20 submodule unit tests pass. The one defect: `is_vorbis_ident` is exported as a public sniffing surface with no callers outside the module itself.

## 2. What actually works (verified)
- `./target/debug/deps/codecs-a0d7a93b7072ff58 vorbis::` -> `test result: ok. 20 passed` (comments/decoder/error/headers/seek unit suites), matching the baseline count (cargo-test-workspace.txt: 20 `audio::vorbis::` entries).
- `./target/debug/deps/vorbis_bitexact-e8223d5cf8b5f437` -> 5 passed: `fixtures_are_pinned_against_the_ffmpeg_reference`, `mid_stream_corruption_is_typed_and_recovery_scoped` (the "one-block overlap scar" claim), `stream_head_encoder_delay_is_trimmed_decoder_side`, `tagged_fixture_exposes_its_vorbis_comments`, `corpus_0085_differential_or_skip`.
- `./target/debug/deps/vorbis_vlc_differential-3a9245a4339ccea0` -> 1 passed (honest PENDING-VLC skip lane).
- Engine purity: `rg -n forbid` on the registry crate → lib.rs:9 `#![forbid(unsafe_code)]`.

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/audio/vorbis/mod.rs:72-73 (export); headers.rs:76-78 (body)
Code:
```
pub use headers::{Identification, VorbisHeaders, is_vorbis_ident, parse_identification};
```
```
pub fn is_vorbis_ident(data: &[u8]) -> bool {
    data.len() >= 7 && data[0] == TYPE_IDENT && &data[1..7] == MAGIC
}
```
What is wrong: `is_vorbis_ident` is re-exported as a public sniffing surface but has zero callers outside the vorbis module (rg over crates/ + apps/: only headers.rs's own tests and this re-export). The Ogg demuxer — the natural consumer — identifies Vorbis through its OWN `identify()` path (demux/src/ogg/mod.rs:152), not this helper. Dead public API that implies a demuxer integration which does not exist. What correct looks like: delete the export, or wire the Ogg demuxer's codec sniff through it. Confidence: HIGH — Verification: rg callers sweep.

## 4. Module inventory (organizer; <100 LOC so no per-function table required)
| Item | Lines | Verdict | Notes |
|---|---|---|---|
| module docs | 1-57 | REAL | Every concrete claim verified (§2); the `fuzz` wiring-status pointer says "documented there, honestly" — true with the staleness caveat logged in fuzz.rs's audit (F-003 there) |
| `pub(crate) mod` decls | 59-68 | REAL | comments/decoder/error/headers/seek pub(crate); `pub mod fuzz` (L66) correctly pub — the fuzz target + survival tests call it cross-crate |
| re-exports | 70-76 | REAL | VorbisDecoder/decode_block/headers/seek exports all have live users (decoder.rs, opus/decoder.rs uses the seek helpers); `is_vorbis_ident` is the exception (F-001) |

## 5. Test honesty
No tests in this file. The module's claims are pinned by the submodule suites (20 tests, executed) and the three integration binaries (bitexact 5, fuzz-survival 1, vlc-differential 1 — all executed this session). The fuzz-survival suite's three-surface claim is FALSE — see audit/files/crates/codecs/src/audio/vorbis/fuzz.rs.audit.md F-001 (cross-reference; the claim lives in the test file, not here).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "No `unsafe` on this module's own paths (the engine's crates are `#![forbid(unsafe_code)]`)" | mod.rs:37-39 | REAL | rg: zero unsafe tokens in vorbis/*; registry lib.rs:9 forbid attribute |
| "no panic on any input — every decode/parse path is bounds-checked ... keeps decoding after a damaged packet with a one-block overlap scar" | mod.rs:39-43 | REAL | `mid_stream_corruption_is_typed_and_recovery_scoped` executed ok; fuzz survival executed ok (with the coverage caveat in fuzz.rs audit F-001) |
| "a digital **silence** fixture ... gives the true byte-exact MD5 lane" | mod.rs:53-55 | REAL | Fixture present (vorbis_silence_48000_stereo.ogg); bitexact suite pins it (canonicalized −0.0 handling documented in tests/vorbis/mod.rs) |
| "The VLC differential hook ... is registered and honestly pending infra" | mod.rs:56-57 | REAL | vorbis_vlc_differential.rs exists, executes, computes the always-on half, skips the VLC half with a message |
| "Native-Vorbis-in-Matroska reuses the same header chain verbatim" | mod.rs:13-15 | REAL | mkv/header.rs:811 A_VORBIS → "vorbis"; CodecPrivate carries the headers (header.rs:158) |
| "symphonia-codec-vorbis 0.6.1 engine" | mod.rs:10 | REAL | Cargo.toml `=0.6.1` pin (codecs/Cargo.toml:103); registry crate present |

## 7. Dead / duplicated / unreachable code
`is_vorbis_ident` (F-001). No duplication found; the seek helpers are shared with the Opus decoder (opus/decoder.rs) — intentional reuse, not duplication.

## 8. Dependencies & flags
symphonia-codec-vorbis =0.6.1 exact-pinned behind `codec-vorbis` (default-on). No unsafe. No new deps introduced by this file.

## 9. Open questions for the next agent
- vorbis/decoder.rs and headers.rs (not in my file list): the discontinuity-reset coverage gap (fuzz.rs audit F-002) lands in decoder.rs:761-764 — that file's owner should pick it up.
- opus/mod.rs owner: the vorbis seek helpers are reused by opus/decoder.rs — verify the lapping arithmetic fits Opus's fixed 120-sample overlap (my next file audits opus/mod.rs at the organizer level only).
