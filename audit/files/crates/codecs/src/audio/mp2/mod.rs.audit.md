# AUDIT: crates/codecs/src/audio/mp2/mod.rs
| Field | Value |
|---|---|
| Auditor | W07-A08 |
| Date | 2026-11-23 |
| Lines / functions | 105 / 1 (module surface: 5 submodule decls, 3 re-export groups) + 1 test |
| File verdict | REAL (module doc + surface; every doc claim I could execute or trace verified true) |
| Reachable from shipped entry point? | YES — `lumen open x.mp2` → apps/cli/src/containers/mod.rs:65 `"mp2" => es::open_mpeg_audio` → crates/demux/src/es/mp3.rs `RawMp3Demuxer` (shared family demuxer) → apps/cli/src/pipeline_host.rs:266-270 `"mp2" => Mp2Decoder::new()` → this module's `decoder` (the doc's R3-routing claim, verified in code, not just the worklog snippet). Web lane mirrors it (apps/web/src/host.rs:633-638). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `./target/debug/deps/codecs-a0d7a93b7072ff58 --test-threads=2 "mp2::tests"` → 1 passed, exit 0; `./target/debug/deps/mp2_corpus_differential-295120e37059f05c --test-threads=2 --nocapture` → 3 passed (fixture rows vs live ffmpeg: max_delta=1 both fixtures; corpus rows SKIP — corpus absent); `./target/debug/deps/mp2_fuzz_survival-a59ac51317fe6421 --test-threads=2` → 3 passed, 4.7 s |

## 1. One-paragraph verdict
A module-surface file whose documentation is unusually honest and, on every point I traced, true: the `mp2` descriptor really is routed to `Mp2Decoder` (not the family gate), the engine really is the already-admitted `symphonia-bundle-mp3` configured with `CODEC_ID_MP2` (decoder.rs:44,164) with no new dependency (`codec-mp2 = ["codec-mp3"]`, Cargo.toml:77), the stricter Layer II gate (`mp2.header.not_layer_ii`, header.rs:158) and CBR gate (`mp2.stream.cbr_mismatch`, header.rs:272) exist as documented, and the differential evidence is real — I ran it against live ffmpeg: ±1 LSB on both committed fixtures. The one structural caveat is inherited from the family: the module advertises seek arithmetic ("both directions") that no shipped path can reach — the shared raw-MPEG-audio demuxer refuses all seeks.

## 2. What actually works (verified)
— `./target/debug/deps/mp2_corpus_differential-295120e37059f05c --test-threads=2 --nocapture` → 3 passed; fixture rows executed with the live ffmpeg oracle: `mp2_48k.mp2 rate=48000 ch=2 samples=72576 differing=35526 (24.5%) max_delta=1`, `mp2_lsf_22k.mp2 rate=22050 ch=2 samples=35712 max_delta=1` — the "identical frames × 1152 window" claim holds (72576 = 63 × 1152, 35712 = 31 × 1152, matching my independent frame walk of both fixtures).
— `./target/debug/deps/codecs-a0d7a93b7072ff58 --test-threads=2 "mp2::tests"` → 1 passed (re-export pins: `layer_ii_bitrate(V1,14) == Some(384)`, `LAYER_II_BITRATES_MPEG2_LSF[13] == 160` — both cross-checked against the family tables in mp3/header.rs:121-131, identical rows).
— `./target/debug/deps/mp2_fuzz_survival-a59ac51317fe6421 --test-threads=2` → 3 passed (mutated fixtures settle typed, harness survives the same mutations, dirty pools survive).

## 3. Findings
F-001 [S3]
Where: mp2/mod.rs:11-14 and 31 (module doc advertising the seek surface) vs crates/demux/src/es/mp3.rs:683-689
Code (the doc):
```
//! extension ([`header`]) — the CBR geometry gate (Layer II is CBR by
//! construction; there is no Xing/Info/VBRI construction in Layer
//! II) and the padding-aware seek arithmetic, both directions
//! ([`seek`]).
```
Code (the shipped demuxer):
```
    ) -> Result<media_types::time::TimeSpan, EngineError> {
        let _ = target;
        Err(mint(
            media_types::ErrorKind::Unsupported { format: None },
            "demux.es.mp3.seek.unsupported",
```
What is wrong: the module owns a full CBR seek planner (`cbr_seek`, `sample_at_byte`, `duration_nanos`, `refine_to_frame`, re-exported at mod.rs:79) with zero shipped callers — raw MP2 streams demux through the family `RawMp3Demuxer`, whose `seek` is a typed refusal; callers exist only in tests (mp2_corpus_differential's corpus-gated seek pin SKIPs in this environment) and fuzz. Same unreachable-surface pattern as the FLAC and MP3 seek modules. Why it matters: three parallel seek implementations (flac/mp3/mp2) now exist and none is reachable from a shipped seek. What correct looks like: land demuxer-side seek support or mark the surfaces unwired. Confidence: HIGH — Verification: read (exhaustive caller search) + executed the SKIPping corpus pin.

F-002 [S4]
Where: mp2/mod.rs:55-60
Code:
```
//! MP2 is lossy, but the engine's Layer II synthesis is the
//! reference DCT/quantization cascade: the differential against
//! ffmpeg compares whole-decode PCM MD5s on the identical
//! `frames × 1152` window (no gapless arithmetic exists in Layer II)
```
What is wrong: "the reference DCT/quantization cascade" is puffery — symphonia is one of several Layer II implementations, not "the reference" (that would be ISO 11172-3 reference code / libmpeg2 lineage). The operative claim (identical-window differential vs ffmpeg, max delta recorded) is real and I executed it. Wording only. Confidence: HIGH — Verification: read + ran the differential.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Module doc | 1-62 | REAL | S3/S4 | all traceable claims verified true; seek-surface reachability (F-001) + wording (F-002) |
| Submodule decls (decoder/error/fuzz/header/seek) | 64-71 | REAL | — | fuzz pub for the registered target (fuzz_codec_mp2.rs exists, fuzz-registry CLEAN) |
| Re-exports | 73-79 | REAL | — | pinned by the mod test |
| tests::the_public_surface_reexports_resolve | 85-104 | REAL | — | compile-level pin + 4 behavior asserts (would fail if a re-export or table row broke; does not cover decode behavior — the submodules' suites do, and I ran them) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| the_public_surface_reexports_resolve | Yes (compile) + Yes (4 value pins) | mode-extension bound 12, V1 idx14=384, LSF idx13=160, mono has no intensity bound — all spec-correct values I cross-checked against mp3/header.rs tables and ISO 11172-3 mode-extension semantics |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The R3 registration card routes the `mp2` descriptor HERE" | mod.rs:42-43 | REAL | pipeline_host.rs:266-270, apps/web/src/host.rs:633-638 |
| "`MpaDecoder` configured with `CODEC_ID_MP2` … no new dependency" | mod.rs:14-19 | REAL | mp2/decoder.rs:44,164; Cargo.toml:77 `codec-mp2 = ["codec-mp3"]` |
| "the stricter header gate (a non-Layer II frame is typed damage)" | mod.rs:40-41 | REAL | header.rs:148-158 `mp2.header.not_layer_ii` |
| "a mid-stream layer or bitrate switch is refused" | mod.rs:52-53 | REAL | header.rs:257-272 `mp2.stream.cbr_mismatch` (read; exercised by mp2 fuzz survival — 3 passed) |
| "the differential against ffmpeg compares whole-decode PCM MD5s on the identical `frames × 1152` window" | mod.rs:58-62 | REAL | executed: samples 72576/35712 = 63/31 × 1152, max_delta=1 vs live ffmpeg |
| seek surface "both directions" | mod.rs:13-14, 31 | REAL code / UNREACHABLE shipped | demuxer seek refusal (F-001) |
| "Layer II is CBR by construction; there is no Xing/Info/VBRI construction in Layer II" | mod.rs:11-13 | REAL (format fact) | Xing/Info/LAME are Layer III ancillary constructions |

## 7. Dead / duplicated / unreachable code
- `seek` module surface: unreachable from shipped entry points (tests/fuzz only) — F-001.
- No dead re-exports (each pinned by the mod test).

## 8. Dependencies & flags
`codec-mp2 = ["codec-mp3"]` — Layer II rides the family admission; both in `default`, so the release lane ships them. No new dependency, as claimed.

## 9. Open questions for the next agent
- mp2/decoder.rs and mp2/header.rs (not my files): confirm the CBR gate's mid-stream refusal semantics match the demuxer's Discontinuity flagging (the family decoder ignores `packet.discontinuity` — see my mp3/decoder.rs audit F-005; check whether Mp2Decoder does the same on channel-mode switch).
- The family `Mp3Decoder` also accepts `mp2` (unconditional 529-sample trim — my mp3 audit F-003); a raw `mp2` stream can therefore be decoded by EITHER decoder depending on descriptor routing — only the registry row ordering decides. Worth one worklog note.
