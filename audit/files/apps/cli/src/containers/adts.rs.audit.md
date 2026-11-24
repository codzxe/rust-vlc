# AUDIT: apps/cli/src/containers/adts.rs
| Field | Value |
|---|---|
| Auditor | W13-A04 |
| Date | 2026-11-24 |
| Lines / functions | 648 (incl. ~494 test lines) / 2 non-test fns + 4 test helpers + 7 test fns |
| File verdict | REAL (open path verified) — but the ADTS family cannot PLAY audio through the shipped CLI (F-001, composition-level) |
| Reachable from shipped entry point? | YES: containers/mod.rs:62 `"adts" => adts::open` ← demux_host::open_source:458 ← open_source:362 ← commands open. Executed via bin/lumen on crates/codecs/tests/fixtures/sine48_lc_mono.aac and sine441_lc_stereo.aac (both exit 0 at the shell, session dies in-engine — §3 F-001). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | ./target/debug/deps/lumen_cli-5f13e10be716a8b0 containers::adts (7 passed, 0.08s); same --nocapture (2 loud corpus SKIPs); bin/lumen open sine48_lc_mono.aac → `event: error [audio-output.unsupported] unsupported (cli.audio_limb_absent)`, `state-changed Buffering → Error`; same on sine441_lc_stereo.aac; bin/lumen open samples/sample-aac-only.mp4 (contrast: reaches Playing). |

## 1. One-paragraph verdict
The ADTS/ADIF open path is real and its facts are honest (profile-honest descriptor, sample-clock base, in-band config, no fake duration), and the demux+decode halves are genuinely tested against committed ffmpeg-authored fixtures (7/7 unit tests pass, packet counts 142/131 and exact packets×1024 sample frames). But the family is UNPLAYABLE end-to-end: the session's audio limb is created only when the harvest names channels+rate (host.rs:913-927 via `TrackFact::audio_geometry()`), ADTS deliberately carries no geometry pair (pcm: None, extra_data: None), and the demuxer's `stream_info()` — which HAS the rate and channels at open time — is consulted only inside a `debug_assert` and thrown away (:124-132). Result, executed: every raw-AAC session dies at its first decoded frame with `cli.audio_limb_absent` (`Buffering → Error → Idle`). The module docs never disclose this; the only in-repo recording of the gap is a corpus-gated conformance test that SKIPS in this tree. The same file's sibling mp4.rs solves the identical problem by routing the aac geometry through the `pcm` field — the fix is three lines away and intentionally not taken.

## 2. What actually works (verified)
— `./target/debug/deps/lumen_cli-5f13e10be716a8b0 containers::adts` → 7 passed / 0 failed: fixtures open with `aac-lc`, base (1,48000)/(1,44100), 142/131 packets, exact `packets × 1024` decoded sample frames, channels 1/2; reserved sampling index refuses `demux.adts.bad-sampling-index`; aac-main refuses `cli.codec_unavailable`; `--start-time` 1s skips early packets, past-end admits 0.
— `bin/lumen open …/sine48_lc_mono.aac`: probe+open+facts+MediaInfoReady all work (`codec aac-lc`, seekable, duration Unknown) — the open path itself is REAL up to the composition seam.

## 3. Findings
### F-001 [S2] ADTS sessions cannot play: open-time geometry exists but is discarded; every raw-AAC session errors at its first decoded frame
Where: apps/cli/src/containers/adts.rs:124-132 and :142-143; composition at apps/cli/src/host.rs:913-927; failure at apps/cli/src/pipeline_host.rs:1258-1263
Code (sed-verified, adts.rs:124-132):
```rust
    if let Some(info) = demuxer.stream_info()
        && let Some(track) = tracks.first()
    {
        debug_assert_eq!(
            track.timing.base.as_parts().1,
            info.rate,
            "the sample clock is the header's declared rate"
        );
    }
```
and adts.rs:142-143 (`pcm: None,` / `extra_data: None,`), and host.rs:913-917:
```rust
        let audio_facts: Option<(u32, u32)> = facts
            .facts
            .iter()
            .find(|fact| fact.kind == MediaKind::Audio)
            .and_then(|fact| fact.audio_geometry());
```
What is wrong: `AdtsStreamInfo` carries `pub rate: u32` and `pub channels: u16` (crates/demux/src/adts/mod.rs:317-319, exposed by `stream_info()` :408) — the geometry is available at open time. The harvest reads it only inside a `debug_assert` (compiled out in release) and emits no `pcm` pair, so `audio_geometry()` (demux_host.rs:202-208) returns None (no pcm, no extra_data to sniff), host.rs wires NO audio limb, and the first decoded frame hits `cli.audio_limb_absent` (pipeline_host.rs:1258-1263, "an audio frame arrived with no audio limb wired"). Executed twice via the shipped binary: `event: error [audio-output.unsupported] unsupported (cli.audio_limb_absent)`, `state-changed Buffering → Error`, `Error → Idle` — on sine48_lc_mono.aac AND sine441_lc_stereo.aac. Contrast, executed: samples/sample-aac-only.mp4 (same AAC decoder, geometry routed through mp4.rs's `pcm` limb) reaches Playing. Why it matters: `lumen open any.aac` never plays audio — a whole registered family is dead at the session level while its unit tests pass (they call `HostDecoderProvider` directly, bypassing limb wiring). The only in-repo recording is container_conformance.rs:209-243 ("the recorded limb-less gap"), corpus-gated → SKIP in this tree (verified). What correct looks like: mp4.rs:200-203's own precedent — route `(info.channels, info.rate)` through `TrackFact.pcm` for the aac descriptor (the geometry-only carrier; consumers gate on the codec descriptor). Severity note: this is S0-adjacent (user-visible core broken for the whole family); it stays S2 only because the conformance suite records the gap honestly as a known limitation and the failure is a typed event, not silent garbage. Confidence: HIGH — Verification: ran (both fixtures, both contrast files).

### F-002 [S3] Module doc sells a working LC decode lane and never discloses the session-level death
Where: apps/cli/src/containers/adts.rs:16-18 and :28-30
Code (sed-verified):
```rust
//!   `descriptor()`, never a local copy). The AAC-LC decoder matches
//!   `aac`/`aac-lc` only; the other profiles open honestly and the
//!   provider refuses them typed at decode (LC-only by card 5.C2a).
```
and
```rust
//! * **No PCM pair**: the track is not PCM — `channels`/`rate` live
//!   in the ADTS header the decoder parses itself; the facts carry
```
What is wrong: the doc narrative ("LC opens and decodes; other profiles refuse at decode") describes the provider seam, and §"No PCM pair" justifies limb-lessness as format truth — but the session-level consequence (every ADTS session errors at first frame, F-001) appears nowhere in this file's 37-line module doc, while the 300-line test suite below it demonstrates decoding that the shipped player cannot reach. The honest sentence lives in a different, corpus-gated file. Confidence: HIGH — Verification: read + ran (bin/lumen error events).

### F-003 [S4] The only release-build use of `stream_info()` is none: the invariant pin vanishes in release
Where: apps/cli/src/containers/adts.rs:127-131 — the `debug_assert_eq!` is the file's sole consumer of `stream_info()`; in release builds the accessor is never called at all. Cosmetic (the accessor is also used by other crates' tests), but the "demuxer-side confirmation" the comment promises does not exist in shipped builds. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open | 61-107 | REAL | — | executed via bin/lumen (2 fixtures): probe→start→harvest→seek-query→wrapper all run; past-end start admits 0 packets (test ran) |
| harvest_facts | 116-153 | SHALLOW | S2 | honest facts but discards the geometry that would make the family playable (F-001); debug_assert-only confirmation (F-003) |
| tests::codec_fixture | 176-180 | REAL | — | committed fixtures verified present |
| tests::CountingSink::accept | 188-198 | REAL | — | counts sample frames via frames_in |
| tests::open | 202-207 | REAL | — | real CLI open path |
| tests::decode_frames | 214-243 | REAL | — | real provider; bypasses limb wiring (see F-001 — honest for what it tests, insufficient as family evidence) |
| fixture_opens_with_expected_facts | 252-269 | REAL | — | ran, passed |
| fixtures_decode_the_full_stream | 277-316 | REAL | — | ran, passed (142/131 packets, ×1024 frames) |
| start_time_skips_packets | 323-362 | REAL | — | ran, passed (incl. past-end → 0 packets) |
| reserved_sampling_index_refuses_at_the_demux_stage | 368-384 | REAL | — | ran, passed (typed demux.adts.bad-sampling-index) |
| main_profile_opens_but_refuses_the_decoder | 391-420 | REAL | — | ran, passed (cli.codec_unavailable) |
| corpus_adts_lanes_decode_through_the_open_path | 475-525 | vacuous here | — | corpus-gated → SKIP (verified loud skip) |
| corpus_dirty_tail_lane_is_honest_through_the_open_path | 538-647 | vacuous here | — | corpus-gated → SKIP (verified loud skip) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fixture_opens_with_expected_facts | YES | real fixture through real open path; asserts descriptor/base/no-pair/no-blob/no-extent; ran ok |
| fixtures_decode_the_full_stream | YES | exact ffprobe packet counts + ×1024 frames + channels; ran ok — but proves the PROVIDER lane, not session playback (F-001) |
| start_time_skips_packets | YES | per-packet pts floor + past-end zero-tail; ran ok |
| reserved_sampling_index_refuses_at_the_demux_stage | YES | builds the bad frame, asserts the typed code; ran ok |
| main_profile_opens_but_refuses_the_decoder | YES | asserts aac-main opens + refusal code; ran ok |
| corpus_adts_lanes… / corpus_dirty_tail… | untested here | SKIP (corpus absent, verified) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The AAC-LC decoder matches `aac`/`aac-lc` only; the other profiles open honestly and the provider refuses them typed at decode" | adts.rs:16-18 | REAL at the provider seam, FALSE as a user-facing story | provider lane verified by test; session dies: bin/lumen → `cli.audio_limb_absent` on LC files |
| "**No PCM pair**: the track is not PCM — `channels`/`rate` live in the ADTS header the decoder parses itself" | adts.rs:28-29 | REAL as written / misleading by omission | the SESSION needs the pair pre-decode (host.rs:913); family unplayable (F-001) |
| "`None` is the correct wiring, not an omission" (extra_data) | adts.rs:23-25 | REAL | seeding a synthetic ASC would misframe (decoder ADTS mode builds from the first header); unit tests pass in-band |
| "a past-the-end target lands at the last frame, and the skip watermark then admits no packet" | adts.rs:34-37 | REAL | test :347-361 ran, packets == 0 |

## 7. Dead / duplicated / unreachable code
None dead; no duplication. The `stream_info()` read is release-dead (F-003).

## 8. Dependencies & flags
No feature gates; `#[cfg(test)]` only. Depends on `demux::adts` (`AdtsDemuxer`, `SAMPLES_PER_BLOCK`, `test_support::adts_frame`) and `testkit::Must`. All reachable via the `adts` registry arm.

## 9. Open questions for the next agent
- host.rs / pipeline_host.rs owners: is the limb-less wall-clock session supposed to be a transitional state (card 6.OPW-ogg's "seam fix" comment suggests the generic geometry lookup was added for ogg) — was ADTS simply never revisited after that fix?
- The dirty-tail lane records a 351-vs-350 packet divergence vs ffmpeg — corpus-gated, unverifiable here.
