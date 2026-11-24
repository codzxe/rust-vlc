# AUDIT: apps/cli/src/containers/wav.rs
| Field | Value |
|---|---|
| Auditor | W13-A08 |
| Date | 2026-11-24 |
| Lines / functions | 590 / 2 production fns + 11 test fns/helpers (6 #[test]) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: `lumen` main → open cmd → `open_source` (demux_host.rs:446 probe, :458) → containers/mod.rs:73 `"wav" => wav::open` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `./target/debug/deps/lumen_cli-5f13e10be716a8b0 containers::wav` → 6 passed (exit 0); same `--nocapture corpus_wav` → SKIP line captured; `ls tests/fixtures/generated/valid | rg '^wav'` → 6 fixtures present; sed/rg verifications |

## 1. One-paragraph verdict
The WAV container host does what its docs claim and its tests execute it: the open path runs the
demuxer pre-pass, harvests per-family decoder-config truth (PCM pair / ADPCM WAVEFORMATEX blob /
FLAC STREAMINFO), clamps `--start-time` against the declared extent, validates through the
demuxer's pure seek query (verified pure: wav/mod.rs:1075-1078 "arithmetic on parsed facts, the
source untouched" — the skip watermark does the filtering), and wraps in the shared selector. All
6 tests pass under my own run, 5 of them exercising real committed fixtures (including
extensible-GUID and RF64) and one hand-built ADPCM file that proves the `extra_data` → decoder
configure → 505-frame decode chain end to end. The defects are coverage-shaped: the 12-lane
ffmpeg-differential corpus test skips on this host (executed SKIP), the FLAC-in-WAV family is a
pinned, honestly-refused decode gap that no committed fixture exercises (corpus-only), and the
two hand-built-file tests use fixed temp filenames without the pid suffix (race under parallel
runs).

## 2. What actually works (verified)
- `./target/debug/deps/lumen_cli-5f13e10be716a8b0 containers::wav` → "6 passed; 0 failed":
  `pcm_fixture_opens_with_expected_facts` (codec/pcm pair/no blob/declared extent through the
  REAL open path), `all_wav_fixture_flavors_open` (s16le/s24le/f32le/u8/extensible-6ch/RF64),
  `adpcm_extra_data_flows_into_the_decoder` (hand-built IMA file: 20-byte fmt body in
  `extra_data`, full decode → exactly 505 = wSamplesPerBlock sample frames), `start_time_clamps_
  and_skips` (past-end i64::MAX/8 clamps to span, 0 packets survive; in-media 1 ms target: every
  surviving packet ≥ 1 ms), `broken_fmt_chunk_refuses_at_the_demux_stage` (destroyed `fmt `
  magic → stage `Demux` refusal), corpus lane (SKIP here).
- Fixtures verified present: tests/fixtures/generated/valid/wav_{pcm_s16le_44k1_st,
  pcm_s24le_48k_st, pcm_f32le_48k_st, pcm_u8_44k1_mono, extensible_6ch_48k, rf64_44k1_st}.wav.
- The demuxer seam the host relies on is real: `format()`/`fmt_chunk_bytes()`
  (wav/mod.rs:304/312), pure `seek` (wav/mod.rs:1075-1078), and the demux crate's own audit is
  REAL with S3/S4-only findings (audit/files/crates/demux/src/wav/mod.rs.audit.md:7-9).
- Arithmetic safety (read): `start_us.saturating_mul(1000)` everywhere; `clamp(0, duration_us)`
  bound is a frame count derived µs (non-negative by construction — the demuxer's span is a
  sample-frame count).

## 3. Findings
F-001 [S3]
Where: apps/cli/src/containers/wav.rs:111-115, :553-575 (the pinned gap), TRUTHS row
:443-447 (0016 flac lane)
Code:
```
                WavCodec::Flac { streaminfo } => (
                    fmt.codec_descriptor().to_string(),
                    None,
                    streaminfo.map(|bytes| SharedBytes::from_vec(bytes.to_vec())),
                ),
```
```
            let frames = match decode_frames(&mut opened) {
                Ok(frames) => frames,
                Err(err) if truth.codec == "flac" => {
                    assert!(
                        err.context.code.contains("flac.frame"),
```
What is wrong: FLAC-in-WAV is a shipped, advertised harvest family (module doc :20-22 promises
the STREAMINFO blob seeds the decoder) whose DECODE is a pinned refusal: the demuxer's
byte-quantum packetization has no FLAC frame alignment, so every real FLAC-in-WAV file opens,
demuxes, and refuses to decode (`flac.frame.structure`). The gap is honestly documented and
asserted-by-name — not a fraud — but it is a user-visible functional hole in a "wired" family:
`lumen open <flac-in-wav>` can never produce audio. On this host the lane is also UNVERIFIED by
execution (the only test row is corpus-gated, F-002), so the STREAMINFO harvest arm itself has
zero non-corpus coverage — no committed fixture carries the flac-in-wav shape (ls of the valid
fixtures: pcm×4/extensible/rf64 only). What correct looks like: either frame-align the WAV
packetization for FLAC (the deferred codec-card work) or demote the module-doc claim from
"seeds the decoder" to "seeds a decoder that today refuses"; add a committed flac-in-wav
fixture so the harvest arm is exercised on every host. Confidence: HIGH — Verification: read +
ls (fixture survey); corpus-lane execution UNVERIFIED (F-002).
F-002 [S4]
Where: apps/cli/src/containers/wav.rs:509-516 (gate), :419-504 (the 12-lane TRUTHS table)
Code:
```
    fn corpus_dir() -> Option<PathBuf> {
        // lumen-allow-literal: cfg(test) corpus-gate env-var name — test harness knob, not product config
        if let Ok(dir) = std::env::var("LUMEN_CORPUS") {
```
What is wrong: the 12-lane ffmpeg-differential (pcm_s24le/s16le/s32le/u8 × rates/channels,
adpcm_ima_wav 168 822 frames, adpcm_ms 128 524 frames, flac gap row) SKIPS on this host —
executed: `--nocapture` prints "corpus(wav-cli): /tmp/my-project/corpus-v4/media absent ...
SKIP" and the test reports ok in 0.01 s. The skip is loud and the truth table cites the demux
crate's own corpus suite, but "6 passed" overstates: the strongest pins (ffmpeg-measured decode
totals) are UNVERIFIED here. Same class as ts.rs F-007. Confidence: HIGH — Verification: ran
(SKIP captured).
F-003 [S4]
Where: apps/cli/src/containers/wav.rs:297-300, :390-396
Code:
```
        let path = std::env::temp_dir().join("lumen-cli-adpcm-ima.wav"); // lumen-allow-literal: cfg(test) scratch filename — test-only, never a shipped surface
        std::fs::write(&path, &wav).must();
        let opened = open(path.clone(), None);
        std::fs::remove_file(&path).ok();
```
What is wrong: both hand-built-file tests write FIXED filenames in the shared temp dir with no
process-id suffix (unlike ts.rs's `TempDir` which embeds `std::process::id()`, ts.rs:616).
Two concurrent instances of the suite (or a stale file from a killed run — the ADPCM test
removes its file only on the success path after open; a panic between write and remove leaks
it, and the next run then reads the stale file, which happens to be byte-identical only by
luck) race on the same path. Test-only blast radius. What correct looks like: the ts.rs TempDir
convention (pid-suffixed scratch dir with Drop cleanup). Confidence: HIGH — Verification: read
+ cross-file comparison.
F-004 [S4]
Where: apps/cli/src/containers/wav.rs:117
Code:
```
            None => ("unknown".to_string(), None, None),
```
What is wrong: the `format() == None` arm mints a track with codec "unknown", no pcm pair, no
blob — a track that can only fail later at the decoder provider. It is unreachable through
`open()` as shipped: `WavDemuxer::start` refuses when the fmt chunk is unparseable (proven by
the executed `broken_fmt_chunk_refuses_at_the_demux_stage`), so a started demuxer always has
`format() == Some`. Dead defensive arm; harmless but unpinned (no test forces it). Confidence:
HIGH — Verification: read + executed broken-fmt refusal.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open | 45-88 | REAL | — | executed via 5 fixture tests; pure-seek validation + watermark consistent |
| harvest_facts | 94-137 | REAL | S3/S4 | PCM/ADPCM arms executed; FLAC arm corpus-only (F-001); None arm dead (F-004) |
| tests: fixture/CountingSink/open/decode_frames | 153-215 | REAL | — | real open path + real provider |
| pcm_fixture_opens_with_expected_facts | 220-235 | REAL | — | honest pin |
| all_wav_fixture_flavors_open | 241-260 | REAL | — | 6 flavors incl. extensible + RF64 |
| adpcm_extra_data_flows_into_the_decoder | 265-321 | REAL | S4 | end-to-end blob → decode; fixed temp filename (F-003) |
| start_time_clamps_and_skips | 327-375 | REAL | — | both directions pinned |
| broken_fmt_chunk_refuses_at_the_demux_stage | 380-398 | REAL | S4 | fixed temp filename (F-003) |
| corpus_dir + corpus_wav_lanes_decode_through_the_open_path | 509-589 | REAL-UNPROVEN | S4 | skips here (F-002); would fail on wrong decode totals where corpus exists |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| pcm_fixture_opens_with_expected_facts | YES | facts fields + extent pinned through the real open path |
| all_wav_fixture_flavors_open | YES (weakly) | pins container/track-count/codec-prefix/pcm-present per flavor — a swapped descriptor family fails |
| adpcm_extra_data_flows_into_the_decoder | YES | 20-byte blob, tag bytes, 505 decoded frames — breaks on harvest or decoder-config regressions |
| start_time_clamps_and_skips | YES | clamp + watermark + packet timing pinned both directions |
| broken_fmt_chunk_refuses_at_the_demux_stage | YES (weakly) | stage pinned; a refusal at probe instead of demux would fail it |
| corpus_wav_lanes_decode_through_the_open_path | YES on a corpus host / NO here | UNVERIFIED: skips loudly (F-002) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "PCM ... the facts pair channels + rate" | wav.rs:13-15 | REAL | pcm_fixture test executed |
| "ADPCM ... the verbatim fmt chunk body as extra_data" | wav.rs:16-19 | REAL | adpcm test: 20 bytes, tag 0x0011, decode 505 frames |
| "FLAC-in-WAV: the 34-byte STREAMINFO ... the FLAC decoder refuses to guess geometry without it" | wav.rs:20-22 | REAL-UNPROVEN (harvest) + pinned decode gap | harvest arm corpus-only (F-001/F-002); decode refusal asserted by name in the same corpus-only test |
| "clamp --start-time against the declared sample-frame span, validate it through the demuxer's pure seek query" | wav.rs:4-6 | REAL | start_time test executed; wav/mod.rs:1075-1078 pure-query verified |
| "clamping is truth" (doc 05 §2.4) | wav.rs:54 | REAL | past-end clamp executed |

## 7. Dead / duplicated / unreachable code
- The `None => ("unknown", ...)` harvest arm (F-004): unreachable through the shipped open path.
- No duplication: the tag tables live in the demux crate (format.rs) and the host cites
  accessors only — verified by the imports (demux::wav::format::WavCodec) and absence of local
  tag parsing.

## 8. Dependencies & flags
- Uses demux::wav, media_types, plugin_api, commands; no features, no unsafe, no new deps.
  Release lane builds (baseline exit 0). `lumen-allow-literal` annotations are cfg(test) paths
  (:297, :390, :510, :514, :522).

## 9. Open questions for the next agent
- The `clamp(0, duration_us)` at :55 would panic if `duration_us()` were ever negative
  (Rust `clamp` panics on min > max); today the demuxer's span is a sample-frame count so it
  cannot be — if a future WAV variant derives span from a SIGNED field, harden with
  `.max(0)` on the bound. Not a live defect (invariant verified by read of the demuxer's
  count-based spans).
- flac-in-wav frame alignment (F-001) is the codec card's deferred work — the next agent on
  codecs/flac should check whether the WAV byte-quantum packets still hit
  `flac.frame.structure` and whether any corpus host ran the 0016 lane since 2026-10-24.
