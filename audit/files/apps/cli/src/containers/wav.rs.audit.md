# AUDIT: apps/cli/src/containers/wav.rs

| Field | Value |
|---|---|
| Auditor | W13-A02 |
| Date | 2026-11-22 |
| Lines / functions | 591 / 14 (2 production + 12 test fns/tests/helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `main.rs` → `run::run` → `lumen_dispatch` → `CliHost::open_session` → `open_source` → `containers::open` (mod.rs:73: `"wav" => wav::open(stream, opts)`). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:4 |

Commands run:
- `sed -n '116,118p' apps/cli/src/containers/wav.rs` (quote verification, exit 0)
- `ls tests/fixtures/generated/valid/ | rg "^wav_"` → all 6 cited fixtures exist (extensible, f32le, s16le, s24le, u8, rf64)
- `rg -n '"wav" =>' apps/cli/src/containers/mod.rs` → line 73
- `[ -d /tmp/my-project/corpus-v4/media ]` → ABSENT (the corpus-gated test skips here)

## 1. One-paragraph verdict

The cleanest container host in the set: a thin adapter over the real `demux::wav::WavDemuxer` with a per-payload-family harvest that is coherent end to end (PCM → the `pcm` pair legitimately, ADPCM → the verbatim WAVEFORMATEX blob, FLAC-in-WAV → the 34-byte STREAMINFO blob) — this is the one family where stuffing `pcm` is actually truthful. Five of six tests are unconditional over committed or synthesized bytes, including a hand-built IMA-ADPCM file that proves the blob→decoder-configure path and a fmt-chunk-destruction damage test. The one structural weakness: the 12-lane corpus differential — the ONLY decode-count evidence for s24le/s32le/u8/adpcm_ms/sr-chaos and the only pin of the flac-in-wav gap — silently skips when the corpus dir is absent, and it is absent here. The flac-in-wav lane also inherits demux_host's `audio_geometry` gap (STREAMINFO is a geometry-bearing blob this build declines to parse), so those sessions open limb-less.

## 2. What actually works (verified)

- Wiring verified (mod.rs:73). All six committed fixtures exist on disk.
- The synthesized ADPCM test builds a real minimal WAV (fmt tag 0x0011, fact chunk, one 256-byte block) and asserts the 20-byte fmt body lands verbatim in `extra_data` AND decodes to exactly 505 frames — this proves the blob-seeding path end to end without any fixture dependency.
- The fmt-chunk-destruction test proves the probe-passes-but-demux-refuses stage split.
- Not executed (no cargo): assertions read and found specific; W00 baseline corroborates the suite runs.

## 3. Findings

### F-001 [S3]
Where: apps/cli/src/containers/wav.rs:509-528 (test gate), 419-504 (the 12-lane truth table)
Code:
```rust
        let default = PathBuf::from("/tmp/my-project/corpus-v4/media"); // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
        default.is_dir().then_some(default)
```
What is wrong: `corpus_wav_lanes_decode_through_the_open_path` — the ONLY source of decode-count evidence for pcm_s24le, pcm_s32le, pcm_u8, adpcm_ms, the sr-chaos lanes, and the only assertion of the pinned flac-in-wav gap (`flac.frame.structure` refusal) — silently `return`s when the corpus is absent. Verified absent in this workspace. Same class as es.rs F-001 / adts.rs F-002; smaller ratio here (1/6 tests) but that one test carries most of the decode-differential evidence.
What correct looks like: commit the lanes (small WAVs) or `#[ignore]` visibly.
Confidence: HIGH — Verification: ls of default path (absent) + read.

### F-002 [S4]
Where: apps/cli/src/containers/wav.rs:117
Code:
```rust
            None => ("unknown".to_string(), None, None),
```
What is wrong: if `demuxer.format()` answers `None` after a successful `start()`, the fact row is minted with the codec string `"unknown"` and no truth — a fabricated descriptor instead of a typed refusal at the open boundary. Reachability: a started WavDemuxer without a parsed format is presumably impossible (the demuxer refuses earlier) — defensive branch, but if ever hit, the failure surfaces later as a decode-stage `cli.codec_unavailable` instead of an open-stage truth.
What correct looks like: return the open error, not a fake fact.
Confidence: MEDIUM — Verification: read; the None case's reachability is a demux-crate question (Wave 4).
Severity: S4.

### F-003 [S4]
Where: apps/cli/src/containers/wav.rs:68
Code:
```rust
        demuxer.seek(target).map_err(|err| wire_error(&err))?;
```
What is wrong: the pure-seek landing is discarded (validation only) — same harmless-but-misleading pattern as ps.rs:164 / adts.rs:87. The clamp against `duration_us` (line 53-55) is what bounds the target here, and it is real (the WAV declares an extent — the clamped case is asserted at 330-331, unlike ADTS's unclamped ~292-year anchor).
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: apps/cli/src/containers/wav.rs:111-115 (with demux_host.rs:202-220)
Code:
```rust
                WavCodec::Flac { streaminfo } => (
                    fmt.codec_descriptor().to_string(),
                    None,
                    streaminfo.map(|bytes| SharedBytes::from_vec(bytes.to_vec())),
                ),
```
What is wrong: the FLAC-in-WAV harvest hands over a 34-byte STREAMINFO — a geometry-bearing blob (sample rate at bits 80-99, channels at bits 83-85) — that `TrackFact::audio_geometry` does not parse (it knows only the pcm pair, Vorbis ident, OpusHead). FLAC-in-WAV sessions therefore open limb-less despite the container declaring full geometry. Fourth instance of the demux_host F-001 geometry-gap family (AAC ASC, and by design ADTS).
Confidence: HIGH — Verification: read both files.
Severity: S4 here (the lane cannot decode anyway — the pinned byte-quantum gap), cross-ref demux_host F-001 for the root fix.

### F-005 [S4]
Where: apps/cli/src/containers/wav.rs:539-552
Code:
```rust
            match truth.codec {
                "flac" => {
```
What is wrong: the corpus test's per-codec expected-shape logic (`flac` blob / `adpcm_` blob / pcm pair) re-encodes the harvest's own branch structure — a test that would pass if the harvest and the test made the SAME mistake together (circular shape check), except that the decode-count assertion (582-586) independently anchors the pcm lanes. Partial mitigation; noted for the test-honesty table.
Confidence: MEDIUM — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open` | 45-88 | REAL | S4 | clamp-then-validate order; watermark positions; discarded seek landing (F-003) |
| `harvest_facts` | 94-137 | REAL | S4 | per-family truth coherent; "unknown" fallback (F-002); flac blob not geometry-parsed (F-004) |
| test `fixture` | 153-157 | REAL | — | |
| test `CountingSink::accept` | 163-172 | REAL | — | fourth copy of this helper in containers/ tests |
| test `open` | 176-181 | REAL | — | real open path |
| test `decode_frames` | 188-215 | REAL | — | typed refusals returned |
| test `pcm_fixture_opens_with_expected_facts` | 221-235 | REAL | — | unconditional; facts + declared extent |
| test `all_wav_fixture_flavors_open` | 242-260 | REAL | — | 6 committed fixtures, loud presence assert |
| test `adpcm_extra_data_flows_into_the_decoder` | 266-321 | REAL | — | synthesized end-to-end blob proof (best test in the file) |
| test `start_time_clamps_and_skips` | 328-375 | REAL | — | clamped past-end (0 packets) + in-media watermark |
| test `broken_fmt_chunk_refuses_at_the_demux_stage` | 381-398 | REAL | — | synthesized damage; stage pin |
| const `TRUTHS` | 419-504 | REAL | — | 12-lane ffmpeg truth table (corpus-gated use) |
| test `corpus_wav_lanes_decode_through_the_open_path` | 518-589 | REAL-UNPROVEN (env-gated) | S3 | F-001; flac gap pinned by refusal code, not smoothed |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| pcm_fixture… / all_flavors… | YES | exact facts; loud fixture asserts |
| adpcm_extra_data… | YES | synthesized bytes; blob content + 505-frame decode pinned |
| start_time_clamps_and_skips | YES | both watermark ends asserted |
| broken_fmt_chunk… | YES | stage assertion on synthesized damage |
| corpus_wav_lanes… | NO-OP in this environment | corpus absent → early return; carries the only s24le/s32le/u8/adpcm_ms decode counts and the flac gap pin |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "PCM … the facts pair channels + rate — the AVI strf precedent" | module docs 13-15 | REAL | harvest 99-103; the one family where `pcm` is truthful |
| "ADPCM … the verbatim fmt chunk body as extra_data" | module docs 16-19 | REAL | harvest 104-110; proven by the synthesized test |
| "FLAC-in-WAV: the 34-byte STREAMINFO … as extra_data" | module docs 20-22 | REAL | harvest 111-115; blob verified by (corpus-gated) test only |
| flac-in-wav decodes | — | STUB-by-refusal | the pinned gap: byte-quantum packets, `flac.frame.structure` refusal — honestly recorded, deferred to a future card |

## 7. Dead / duplicated / unreachable code

- Test-helper duplication: `CountingSink` + decode driver, fourth near-copy in this directory (ps.rs, es.rs, adts.rs, wav.rs).
- The `"unknown"` codec fallback (F-002) is believed-unreachable defensive code.

## 8. Dependencies & flags

No feature gates. Depends on `demux::wav` (format enum, codec_descriptor accessor — no local tag-table copy), shared `SelectDemuxer`, `pipeline_host::HostDecoderProvider` (tests).

## 9. Open questions for the next agent

- WavDemuxer: can `format()` be `None` after a successful `start()`? If yes, F-002 is reachable and the open path mints a fake `"unknown"` track fact.
- The flac-in-wav byte-quantum packetization gap (deferred [PROPOSAL]): does any matrix row claim FLAC-in-WAV playback? (Wave 18 claim ledger.)
