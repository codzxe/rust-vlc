# AUDIT: apps/cli/src/containers/es.rs
| Field | Value |
|---|---|
| Auditor | W13-A07 |
| Date | 2026-11-23 |
| Lines / functions | 702 / 5 prod fns (+8 test fns, 9 tests) |
| File verdict | REAL (with one FALSE doc claim and one broken-if-run test, both hidden) |
| Reachable from shipped entry point? | YES: main.rs → run::run → dispatch::lumen_dispatch → host (CliHost) → demux_host::open_source → containers/mod.rs:62-66 match arms `"flac"/"hevc"/"h264"/"mp2"/"mp3"` → es::{open_flac, open_annexb, open_mpeg_audio} |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 3 |
| Commands run | `./target/debug/deps/lumen_cli-9887c2f6df9cd333 containers::es` → 9 passed / 0 failed (exit 0); same + `--nocapture` → 7 "SKIP es corpus: corpus dir absent" lines (7 of 9 tests vacuous); `test -d /tmp/my-project/corpus-v4/media` → absent; `nm -C … | rg rust_h264` → symbols present; sed/rg cross-reads of pipeline_host.rs, codecs/Cargo.toml, h264/sw.rs. |

## 1. One-paragraph verdict
The wiring is real and careful: raw MPEG audio opens through the real probe/open path with layer-honest descriptors and exact sample-clock bases, the two committed-fixture tests genuinely execute (open + full-stream decode through the REAL provider), and the raw-FLAC typed refusal is current and honest (no flac.rs in crates/demux/src/es — verified by ls). But the module's centerpiece claim — that the `h264` lane "refuse[s] decode honestly" because "H.264/HEVC have no software decoder in this tree" — is FALSE for H.264 since ADR-001: pipeline_host.rs:344 wires `H264SwDecoder` behind default-on feature `codec-h264-sw`, the rust_h264 symbols are in the very test binary I ran, and `H264SwDecoder::configure` accepts `h264` with `extra_data: None` (Annex-B lazy framing). The test that pins the stale claim (`h264_corpus_lane_opens_demuxes_and_refuses_decode`) would PANIC at `unreachable!("h264/hevc refuse decode")` if the corpus were present; it "passes" today only because 7 of 9 tests silently skip (corpus absent, skip messages swallowed by libtest capture).

## 2. What actually works (verified)
— `./target/debug/deps/lumen_cli-9887c2f6df9cd333 containers::es` → `test result: ok. 9 passed; 0 failed`, and with `--nocapture` the two fixture tests (mpeg_audio_fixture_opens_with_expected_facts, mpeg_audio_fixtures_decode_the_full_stream) run for real against committed bytes in crates/codecs/tests/fixtures (facts + ffprobe-pinned packet counts + measured decode counts through the real provider).
— Raw-FLAC refusal still accurate: `ls crates/demux/src/es/` → aac.rs annexb.rs error.rs fuzz.rs h264.rs hevc.rs mod.rs mp3.rs tests.rs (no flac demuxer); the refusal text names exactly that gap.

## 3. Findings
F-001 [S2]
Where: apps/cli/src/containers/es.rs:15-24 (doc), 426-433+456-458 (test), 669 (helper panic)
Code (doc):
```
//! * **`h264` / `hevc`** → [`AnnexbVideoDemuxer`](demux::es::annexb::
//!   AnnexbVideoDemuxer) with the codec the probe's NAL-set
//!   discrimination named. These lanes **open and demux, then
//!   refuse decode honestly** — H.264/HEVC have no software decoder
//!   in this tree (`codecs/src/video/h264/mod.rs`: parsers +
//!   hardware-path seam only), so the provider answers the typed
//!   `cli.codec_unavailable` and an all-video session lands in the
```
Code (test helper, 668-670):
```
        match provider.decoder_for(&track, &config) {
            Ok(_) => unreachable!("h264/hevc refuse decode"),
            Err(err) => err,
```
What is wrong: the doc is FALSE for h264 (still true for hevc). pipeline_host.rs:344 is `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new)`; codecs/Cargo.toml `default = [… "codec-h264-sw" …]`; apps/cli depends on codecs with default features; `H264SwDecoder::configure` (crates/codecs/src/video/h264/sw.rs:419-446) accepts `"h264"` (accepts at sw.rs:182-184) with `extra_data: None` ("Annex-B until an avcC says otherwise"). So a raw .h264 file now DECODES in the shipped CLI; the "honest refusal" the module advertises is gone, `codecs/src/video/h264/mod.rs: parsers + hardware-path seam only` is stale (sw.rs exists, ~700 lines), and the corpus test asserting `err.context.code == "cli.codec_unavailable"` would panic at the unreachable! with a corpus present. Why it matters: the module's stated value proposition ("That honest refusal IS these lanes' value") inverts reality; the regression test for that value is broken-if-run and vacuously green today (corpus absent, skips hidden by libtest capture — I had to pass --nocapture to see them). What correct looks like: update the doc to "h264 decodes (rust_h264), hevc refuses", and change the h264 lane test to assert a real decode (or split hevc-only for the refusal). Confidence: HIGH — Verification: read + executed (binary symbols + 9-test run + --nocapture skip evidence); the panic itself UNVERIFIED by execution (corpus absent; cargo-build-budget forbids synthesizing a corpus run) — but the call chain is fully read-verified.
F-002 [S3]
Where: apps/cli/src/containers/es.rs:426-435 (and the 6 other corpus-gated tests at 463-471, 494-503, 529-536, 553-555, 613-619, 638-645)
Code:
```
    fn h264_corpus_lane_opens_demuxes_and_refuses_decode() {
        let Some(dir) = corpus_dir() else {
            eprintln!("SKIP es corpus: corpus dir absent");
            return;
        };
```
What is wrong: 7 of 9 tests in this file assert NOTHING in this environment (corpus dir absent — verified `test -d /tmp/my-project/corpus-v4/media` → no). They report `ok` identically to real tests; the skip is an eprintln that libtest captures by default. The baseline "4744 passed" includes these as passes. Why it matters: evidence inflation — a reader cannot distinguish executed green from skipped green without --nocapture. What correct looks like: `#[ignore]` + explicit corpus feature, or assert the corpus dir exists and fail loudly when a corpus-running lane is requested. Confidence: HIGH — Verification: executed (--nocapture shows 7 SKIP lines).
F-003 [S4]
Where: apps/cli/src/containers/es.rs:134-143
Code:
```
        debug_assert_eq!(
            track.timing.base.as_parts().1,
            head.sample_rate,
            "the sample clock is the first frame's declared rate"
        );
```
What is wrong: the harvest's "pinned invariant" is debug-build-only; in release (the shipped lane) a track/accessor divergence passes silently. The comment calls it "Pinned as the harvest's invariant" — overstated for release. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: apps/cli/src/containers/es.rs:300-304
Code:
```
        let default = PathBuf::from("/tmp/my-project/corpus-v4/media"); // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
```
What is wrong: hard-coded machine-specific corpus path as the default; on any other checkout the corpus tests skip silently (compounding F-002). cfg(test)-only, so hygiene not product. Confidence: HIGH — Verification: read + test -d.
F-005 [S4]
Where: apps/cli/src/containers/es.rs:262-286 (open_flac)
Code:
```
pub(crate) fn open_flac(
    _stream: Box<dyn SourceStream>,
    _opts: &OpenOptions,
) -> Result<OpenedMedia, CommandError> {
    Err(CommandError::new(
```
What is wrong: nothing functionally — an honest typed refusal (not a fake); noted because both parameters are ignored, which is the stub shape. The refusal text is current (no flac demuxer exists) and names the harvest contract. Confidence: HIGH — Verification: read + ls of crates/demux/src/es.

## 4. Per-function table (mandatory, >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open_mpeg_audio | 81-121 | REAL | — | real demuxer + facts + watermark; start_us clamped ≥0, saturating µs→ns |
| harvest_mpeg_audio | 125-169 | REAL | S4 | debug_assert-only invariant (F-003); geometry from first frame header |
| open_annexb | 178-221 | REAL | — | codec from construction; start-time honest no-op (Missing stamps) |
| harvest_annexb | 229-260 | REAL | — | descriptor mirror documented (demuxer's `descriptor()` crate-private) |
| open_flac | 262-286 | REAL | S4 | typed refusal, current and honest (F-005) |
| tests: corpus_dir / codec_fixture / open / open_refused / walk / CountingSink | 289-354, 655-702 | REAL (helpers) | — | open/walk use unreachable! panic-on-error — loud test helpers, acceptable |
| tests: 9 test fns | 356-653 | 2 REAL + 7 VACUOUS-HERE | S2/S3 | fixtures genuinely run; corpus 7 skip silently (F-002); h264 one broken-if-run (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mpeg_audio_fixture_opens_with_expected_facts | YES (when run) | committed fixtures; asserts container/track/codec/base/geometry/extra_data/extent through the REAL open path — ran, passed |
| mpeg_audio_fixtures_decode_the_full_stream | YES (when run) | full-stream decode via real provider with ffprobe-pinned packet counts and measured frame counts — ran, passed |
| h264_corpus_lane_opens_demuxes_and_refuses_decode | NO — it would PANIC if run | asserts `cli.codec_unavailable` for h264, but decoder_for wires H264SwDecoder (F-001); today vacuously skips |
| hevc_corpus_lane_opens_demuxes_and_refuses_decode | YES (when run) | hevc genuinely unrouted (decoder_for has no hevc arm) — but skipped here |
| flac_lanes_refuse_with_the_demuxer_gap_named | YES (when run) | refusal text + details asserted — skipped here |
| oversized_id3_lane_is_a_typed_probe_miss | YES (when run) | probe-miss staged — skipped here |
| corpus_mpeg_audio_differential | YES (when run) | counts vs ffprobe, mp2 sample-exact — skipped here |
| mp3_start_time_watermark_is_exact | YES (when run) | 59 = 79−20 dropped frames — skipped here |
| h264_start_time_is_the_honest_noop | YES (when run) | demux-only walk, still valid with h264 wired — skipped here |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "h264/hevc … refuse decode honestly — H.264/HEVC have no software decoder in this tree" | es.rs:15-24 | FALSE (h264) / REAL (hevc) | pipeline_host.rs:344 h264 arm; sw.rs:182-184 accepts "h264"; codecs default features; rust_h264 symbols in test binary (nm) |
| "the raw-FLAC … demuxer has not landed: crates/demux/src/es covers h264/hevc/mp3/aac" | es.rs:25-30, 262-286 | REAL | ls crates/demux/src/es → aac/annexb/h264/hevc/mp3, no flac |
| "mp3/mp2 … decode end-to-end — the MP3/MP2 decoders are wired (pipeline_host mp3/mp2 arms)" | es.rs:9-14 | REAL | pipeline_host.rs:251-261 mp3/mp1 + mp2 arms; fixture decode test executed |
| "Neither ES demuxer implements the seek API (both refuse typed)" | es.rs:47-56 | REAL-UNPROVEN | consistent with SelectDemuxer watermark design and the no-seek comments; demuxer internals are another agent's file |
| "The `annexb` family id never reaches the registry — the probe mints it sub-threshold" | es.rs:30-33 | UNVERIFIED | probe behavior lives in media-probe; not executed here |

## 7. Dead / duplicated / unreachable code
- No dead production code. Test helpers use 3 `unreachable!` (open/open_refused/decode_refusal) — loud-fail test style, not shipped code.
- `decode_refusal` is now a wrong-premise helper for the h264 lane (F-001).

## 8. Dependencies & flags
- demux::es::{annexb, mp3}, media_types, plugin_api, commands, crate::demux_host. No feature gates in this file; the h264 wiring it mis-docules rides codecs' default `codec-h264-sw` (verified in codecs/Cargo.toml default list).

## 9. Open questions for the next agent
- Whoever owns apps/cli/src/pipeline_host.rs: the "landed:" refusal string (376-377) omits vp9/dnxhd (both wired) — third registry inconsistency beyond my three assigned ones.
- Does any feature-matrix row cite the es h264 "honest refusal" as evidence? If yes, that row is now FALSE (it decodes). Matrix agents should re-check rows citing `0019-raw-video.h264`.
