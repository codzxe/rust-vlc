# AUDIT: apps/cli/src/containers/es.rs
| Field | Value |
|---|---|
| Auditor | W13-A02 |
| Date | 2026-11-21 |
| Lines / functions | 702 / 23 (5 production + 18 test items) |
| File verdict | REAL (production wiring verified; h264 lane's doc narrative FALSE — see F-001) |
| Reachable from shipped entry point? | YES (call chain: `lumen` main.rs:9 → run.rs:126 `Some(CliCommand::Open{..})` → parse.rs:197 `"open" => parse_open` → dispatch.rs:82 `host.open_session` → host.rs:904 `open_source(...)` → demux_host.rs:458 `crate::containers::open(&container, stream, &opts)` → containers/mod.rs:62-66 arms `flac`/`hevc`/`h264`/`mp2`/`mp3` → this file. Also bench.rs:270 `demux_host::open_source`.) |
| Findings | S0:0 S1:0 S2:2 S3:2 S4:3 |
| Commands run | sed -n 1-240/241-500/501-702 (full read); rg `open_mpeg_audio|open_annexb|open_flac` apps/cli/src (callers: containers/mod.rs:62-66 only — registry is sole consumer); sed -n 1-120 containers/mod.rs (dispatch + unwired_refusal); sed -n 170-380 pipeline_host.rs (decoder_for arms); sed -n 40-80 + 420-446 + 182-184 crates/codecs/src/video/h264/sw.rs (configure/accepts); rg `fn accepts` sw.rs; LS crates/demux/src/es (no flac demuxer — aac/mp3/hevc/h264/annexb only); LS crates/codecs/tests/fixtures (mono48.mp3, mpeg2_24k.mp3, mp2_48k.mp2, mp2_lsf_22k.mp2 committed); ls /tmp/my-project (corpus-v4 ABSENT in this sandbox); grep baseline test-workspace.log.txt (`containers::es` absent — lumen-cli tests never ran in fail-fast baseline); grep orchrun-test-workspace-nofailfast.log.txt:2787-2795 (all 9 es tests "ok"); sed -n 15-24/420-424/454-458/605-609/625-631/655-672 es.rs (quote verification). |

## 1. One-paragraph verdict
The five production functions are a small, correct wiring layer: construct the demuxer, run `start()`, harvest per-track facts with no local arithmetic (codec/base/span come from the track itself, geometry from the demuxer's validated first frame header), wrap in `SelectDemuxer`, and pass `--start-time` through as a ns watermark with saturating µs→ns conversion. The mp3/mp2 lanes are proven end-to-end by committed-fixture tests that really ran (baseline no-failfast log lines 2792/2795). The module's centerpiece narrative — that the h264/hevc raw-ES lanes "refuse decode honestly" because "H.264/HEVC have no software decoder in this tree" — is FALSE for h264: `pipeline_host.rs:346` dispatches `H264SwDecoder`, `sw.rs:182-184` accepts `"h264"`, and `configure` with `extra_data: None` returns `Ok` (lazy Annex-B), so the provider returns a working decoder and the committed test that asserts `cli.codec_unavailable` would PANIC if its corpus fixture were present — it only records "ok" because the corpus is absent and the test silently skips. 7 of 9 tests in this file are gated on an untracked sandbox path and contribute vacuous "ok" lines to the recorded evidence.

## 2. What actually works (verified)
- mp3/mp2 open + facts: `mpeg_audio_fixture_opens_with_expected_facts` (es.rs:363) ran on COMMITTED fixtures (present in crates/codecs/tests/fixtures, `assert!(path.is_file())` guards) and passed — orchrun-test-workspace-nofailfast.log.txt:2792 `containers::es::tests::mpeg_audio_fixture_opens_with_expected_facts ... ok`.
- mp3/mp2 decode end-to-end through the REAL open path + `HostDecoderProvider` (mp3 arm pipeline_host.rs:261-265, mp2 arm 266-270): `mpeg_audio_fixtures_decode_the_full_stream ... ok` (log:2795). Packet/decoded counts are hard numbers (64/63/31 packets; 73,199/72,576/35,712 frames), not tautologies.
- `open_flac`'s gap claim is accurate: `crates/demux/src/es/` contains aac/mp3/hevc/h264/annexb only — no raw-FLAC demuxer (LS verified).
- Watermark arithmetic: `start_us.saturating_mul(1000)` µs→ns is unit-correct; negative `start_time_us` clamps to 0 via `.max(0)` (es.rs:102, 193).

## 3. Findings

### F-001 [S2] — Module doc's h264 claim is FALSE: the provider does NOT refuse h264
Where: apps/cli/src/containers/es.rs:15-24 (claim) vs apps/cli/src/pipeline_host.rs:346 and crates/codecs/src/video/h264/sw.rs:182-184,442-445 (contradicting code)
Code:
```
//!   refuse decode honestly** — H.264/HEVC have no software decoder
//!   in this tree (`codecs/src/video/h264/mod.rs`: parsers +
//!   hardware-path seam only), so the provider answers the typed
//!   `cli.codec_unavailable` and an all-video session lands in the
//!   engine's `NoDecodableTrack` naming the container.
```
```
            "h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new),
```
```
        if let Some(extra) = cfg.extra_data.as_ref() {
            self.seed_extradata(extra.as_slice())?;
        }
        Ok(())
```
What is wrong: The doc asserts the h264 raw-ES lane answers `cli.codec_unavailable` because no software H.264 decoder exists. The decoder exists and is wired: `decoder_for` matches `"h264"` at pipeline_host.rs:346, `seeded()` seeds `extra_data: None` (the annexb harvest sets `extra_data: None`, es.rs:237), and `H264SwDecoder::configure` returns `Ok(())` with no extradata (lazy Annex-B framing hypothesis — sw.rs:431-445). `cli.codec_unavailable` is only minted by the `other =>` arm (pipeline_host.rs:372-379), which `"h264"` never reaches. Baseline CORRECTION v2 (audit/runs/baseline/INDEX.md:104-118) already established sw.rs exists and 4:2:0 H.264 MP4 plays. The claim is stale for h264 (still true for hevc — no hevc row exists).
Why it matters: The module's stated "value" of the h264 lane ("that honest refusal IS these lanes' value") is fiction; a raw Annex-B h264 file actually obtains a software decoder and decodes. Anyone reading this doc to decide behavior (or writing tests/evidence from it) is misled — which is exactly what happened in F-002.
What correct looks like: Doc rewritten to: h264 lane → `H264SwDecoder` (Annex-B lazy configure, 4:2:0 8-bit surface, typed `h264.sw.*` refusals for out-of-scope profiles); hevc lane → typed `cli.codec_unavailable`.
Confidence: HIGH — Verification: read (full call chain traced: es.rs:186-212 → harvest sets extra_data None → pipeline_host.rs:346 → sw.rs:420-445); baseline cite: INDEX.md CORRECTION v2.

### F-002 [S2] — h264 corpus test asserts a refusal the code cannot produce; it "passes" only by silently skipping
Where: apps/cli/src/containers/es.rs:425-458 and 655-672
Code:
```
        let err = decode_refusal(&opened);
        assert_eq!(err.context.code, "cli.codec_unavailable", "{err}");
```
```
        match provider.decoder_for(&track, &config) {
            Ok(_) => unreachable!("h264/hevc refuse decode"),
            Err(err) => err,
        }
```
What is wrong: For an h264 track, `provider.decoder_for` returns `Ok` (F-001 chain), so `decode_refusal` hits `unreachable!("h264/hevc refuse decode")` and the test PANICS — if `0019-raw-video.h264` exists under the corpus root. The recorded evidence shows `containers::es::tests::h264_corpus_lane_opens_demuxes_and_refuses_decode ... ok` (orchrun-test-workspace-nofailfast.log.txt:2789), which is only possible because `corpus_dir()` returned None (corpus absent) and the test returned early at es.rs:427-430 after `eprintln!("SKIP es corpus: corpus dir absent")`. In this sandbox `/tmp/my-project` contains no `corpus-v4` (ls verified). The test's assertion is not merely unproven — it is contrary to the shipped code.
Why it matters: A test that fails (panics) whenever its fixture is present and silently passes when it is not is worse than no test: the baseline logs record "ok" for a h264 behavioral claim that is false, and any developer who mounts the corpus gets a red suite that will be "fixed" by deleting the assertion rather than understanding the lane. This is test-evidence inflation on a decode-path claim.
What correct looks like: The test should either assert the real current behavior (decoder obtained; e.g. decode the 4:2:0 corpus file end-to-end like the mp3 lane does) or be deleted along with the stale doc. Silent `return`-on-missing-corpus should be an explicit `#[ignore = "corpus-v4 not mounted"]` so it shows as ignored, not ok.
Confidence: HIGH — Verification: read (trace) + baseline log line 2789 + ls /tmp/my-project (corpus absent).

### F-003 [S3] — 7 of 9 tests silently skip (and report "ok") when the untracked corpus path is absent
Where: apps/cli/src/containers/es.rs:297-304 (gate) and 427-430, 464-467, 495-498, 529-532, 553-556, 612-615, 638-641 (skip sites)
Code:
```
        let default = PathBuf::from("/tmp/my-project/corpus-v4/media"); // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
        default.is_dir().then_some(default)
```
```
        let Some(dir) = corpus_dir() else {
            eprintln!("SKIP es corpus: corpus dir absent");
            return;
        };
```
What is wrong: Every corpus-gated test in this file (h264/hevc lanes, flac refusals, oversized-ID3, the mpeg-audio differential, both `--start-time` tests) ends in a silent `return` when `/tmp/my-project/corpus-v4/media` is missing — a Lead-sandbox path that is not in the repo (docs/worklog/archive-2026-10.md:8516 confirms the corpus "lives untracked"). All seven recorded "ok" lines in the baseline no-failfast run are therefore indistinguishable from skipped. Only the two committed-fixture tests assert fixture presence.
Why it matters: The file's most detailed assertions (ffprobe packet counts, sample-exact mp2 windows, watermark arithmetic) have never produced a failing result in any recorded environment — they are evidence-shaped, not evidence. Regression protection for the ES lanes is effectively zero outside one machine.
What correct looks like: Corpus tests marked `#[ignore]` with the mount requirement named (visible in `cargo test -- --ignored` tallies), or the corpus fixtures committed like the codec-crate fixtures already are (`crates/codecs/tests/fixtures` proves small real-media fixtures are committable policy).
Confidence: HIGH — Verification: read + ls /tmp/my-project + baseline log 2787-2795.

### F-004 [S3] — Vacuous assertion: `.unwrap_or(true)` swallows the very failure it claims to check
Where: apps/cli/src/containers/es.rs:629
Code:
```
                let ns = pts.convert(&media_clock::NANO_BASE);
                assert!(ns.map(|ns| ns.ticks >= 1_000_000_000).unwrap_or(true));
```
What is wrong: If `pts.convert` returns None (the exact case where the invariant cannot be shown), the assert passes. The loop body's only purpose — "every surviving packet's pts is at-or-after the target" — is unverified precisely when conversion fails.
Why it matters: A watermark bug that mangled pts into an unconvertible stamp would pass this test; the test would still "verify" the watermark.
What correct looks like: `let Some(ns) = pts.convert(&media_clock::NANO_BASE) else { panic!("unconvertible pts after watermark") }; assert!(ns.ticks >= 1_000_000_000);`
Confidence: HIGH — Verification: read.
(Note: this test is itself corpus-gated and skipped in all recorded runs — F-003 — so the weakness compounds.)

### F-005 [S4] — Doc comment off-by-one prose: "19 frames" vs its own `k ≤ 19` (= 20 frames)
Where: apps/cli/src/containers/es.rs:605-609
Code:
```
    /// `--start-time` on the mp3 lane: the watermark is exact —
    /// frames whose sample-clock pts predates 1.000 s are dropped
    /// (19 frames of 576 samples @ 11025 Hz: k×576 < 11025 ⇒
    /// k ≤ 19), the walk resumes at the first frame whose pts is
```
What is wrong: `k×576 < 11025 ⇒ k ≤ 19` describes k ∈ {0,…,19} = 20 frames; the doc says "19 frames". The test's own assertion (`assert_eq!(count, 59, "79 − 20 pre-target frames (k=0..=19)")`, es.rs:623) counts 20.
Why it matters: Prose contradicts the assertion it explains; a reader reconciling the two wastes time or "fixes" the correct assert.
What correct looks like: "20 frames (k = 0..=19)".
Confidence: HIGH — Verification: read.

### F-006 [S4] — Pinned invariant is debug-only, so the recorded release-profile runs never execute it; geometry would mislabel on a multi-track future
Where: apps/cli/src/containers/es.rs:135-152
Code:
```
    if let Some(head) = demuxer.head()
        && let Some(track) = tracks.first()
    {
        debug_assert_eq!(
            track.timing.base.as_parts().1,
            head.sample_rate,
```
```
        let geometry = demuxer
            .head()
            .map(|head| (u32::from(head.channels), head.sample_rate));
```
What is wrong: (a) The "pinned invariant" is a `debug_assert_eq!` — compiled out in the release profile the baseline tests ran with (INDEX.md:38-40), so the recorded evidence never checked it. (b) `geometry` is computed once from the demuxer's first frame header and stamped onto EVERY track in the loop; correct today only because `RawMp3Demuxer` mints exactly one track. Nothing in this function enforces or documents that coupling at the type level.
Why it matters: A future demuxer change (multi-track, or header re-validation) turns this into silent wrong geometry on facts that downstream `audio_geometry()` sizing trusts.
What correct looks like: Assert single-track at the top of `harvest_mpeg_audio` (`assert!(tracks.len() == 1)` or a typed refusal), and/or compute geometry per track.
Confidence: HIGH — Verification: read.

### F-007 [S4] — `let _ = decoder.decode(...)` / `let _ = decoder.flush(...)` discard DecodeOutcome in the decode driver
Where: apps/cli/src/containers/es.rs:692 and 699
Code:
```
                Ok(DemuxEvent::Packet(packet)) => {
                    let _ = decoder.decode(packet, &mut sink)?;
                }
```
```
        let _ = decoder.flush(&mut sink)?;
```
What is wrong: The `DecodeOutcome` (e.g. `Recovered { CorruptInput }` concealment counts) is dropped; the tests assert only total sample-frame counts, so a decoder that recovered through corruption (or emitted concealment) passes identically to a clean decode.
Why it matters: The lane's claimed value is "packet-exact demux … no silent wrong decode" — the driver cannot see silent anything.
What correct looks like: Match the outcome and fail on `Recovered`/`CorruptInput` for these clean-fixture lanes (or at least count and assert zero recoveries).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open_mpeg_audio` | 88-121 | REAL | — | Wired via containers/mod.rs:65-66. No local arithmetic; µs→ns saturating; negative start clamped. Errors typed via `wire_error`. Reachable (chain in header). |
| `harvest_mpeg_audio` | 128-170 | REAL | S4 (F-006) | debug-only invariant; single `head()` geometry stamped on all tracks (safe only while demuxer is single-track). |
| `open_annexb` | 181-212 | REAL | — | Same wrapper shape; skip watermark honest no-op for Missing pts (wrapper rule lives in demux_host part2 — cross-agent). |
| `harvest_annexb` | 222-247 | REAL | — | Codec descriptor mirrored locally (demuxer's `descriptor()` crate-private — noted in doc); base passed through from track; nothing derived. |
| `open_flac` | 258-271 | REAL | — | Constant typed refusal; gap claim verified accurate (no flac demuxer in crates/demux/src/es). Stage::Probe for a post-probe demuxer gap is a documented house choice (matches `unwired_refusal`). |
| `tests::codec_fixture` | 289-293 | REAL | — | Points at committed fixtures (LS-verified present). |
| `tests::corpus_dir` | 297-304 | REAL | S3 (F-003) | Untracked Lead-sandbox default path; the vacuous-evidence root. |
| `tests::corpus_file` | 306-308 | REAL | — | Trivial join. |
| `tests::open` | 312-317 | REAL | — | Real open path; `unreachable!` on error is acceptable in test helper. |
| `tests::open_refused` | 320-325 | REAL | — | Twin for refusals. |
| `tests::walk` | 329-340 | REAL | — | Collects packets; `unreachable!` on demux error hides recovery paths but is scoped to clean lanes. |
| `tests::CountingSink::accept` | 345-355 | REAL | — | Counts frames via `format.frames_in`; remembers channels. |
| `mpeg_audio_fixture_opens_with_expected_facts` | 362-385 | REAL | — | Ran for real (committed fixtures; baseline no-failfast:2792). Hard facts assertions. |
| `mpeg_audio_fixtures_decode_the_full_stream` | 398-418 | REAL | S4 (F-007) | Ran for real (log:2795); outcome discarded by driver. |
| `h264_corpus_lane_opens_demuxes_and_refuses_decode` | 425-458 | FALSE | S2 (F-001/F-002) | Asserts `cli.codec_unavailable` — code returns a decoder; would panic if run; skipped in all recorded runs. |
| `hevc_corpus_lane_opens_demuxes_and_refuses_decode` | 462-488 | REAL-UNPROVEN | S3 (F-003) | Assertion matches code (`hevc` → `other` arm), but corpus-gated/skipped; never executed in recorded evidence. |
| `flac_lanes_refuse_with_the_demuxer_gap_named` | 493-518 | REAL-UNPROVEN | S3 (F-003) | Same — correct-by-trace, skipped in recorded runs. |
| `oversized_id3_lane_is_a_typed_probe_miss` | 527-542 | REAL-UNPROVEN | S3 (F-003) | Corpus-gated/skipped. |
| `corpus_mpeg_audio_differential` | 551-603 | REAL-UNPROVEN | S3 (F-003) | Detailed numbers, all skipped in recorded runs; mp2-vs-ffmpeg equality only asserted for .mp2 names. |
| `mp3_start_time_watermark_is_exact` | 610-632 | REAL-UNPROVEN | S3 (F-003) + S3 (F-004) | Skipped + vacuous unwrap_or(true) assertion. |
| `h264_start_time_is_the_honest_noop` | 636-650 | REAL-UNPROVEN | S3 (F-003) | Skipped in recorded runs. |
| `tests::decode_refusal` | 655-672 | FALSE | S2 (F-002) | Panics via `unreachable!` for h264 when fixture present. |
| `tests::decode_sink` | 676-701 | REAL | S4 (F-007) | Real dispatch driver; outcomes discarded. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| mpeg_audio_fixture_opens_with_expected_facts | YES | Committed fixtures + presence assert + exact facts (base (1,48000), geometry, no extra_data). Mutation: swap base denominator → fails. |
| mpeg_audio_fixtures_decode_the_full_stream | YES (partially) | Exact packet/decoded-frame counts fail on demux or decoder regression; but `Recovered` outcomes are invisible (F-007). |
| h264_corpus_lane_opens_demuxes_and_refuses_decode | NO — worse: it fails if the code is CORRECT | Current code returns a decoder → helper panics. Passes today only via silent skip (F-002). |
| hevc_corpus_lane_… | NO (in recorded envs) | Corpus-gated silent skip; never executed in baseline (log shows ok = skip). With corpus mounted it would genuinely exercise the lane. |
| flac_lanes_refuse_with_the_demuxer_gap_named | NO (in recorded envs) | Same skip gate. |
| oversized_id3_lane_is_a_typed_probe_miss | NO (in recorded envs) | Same skip gate. |
| corpus_mpeg_audio_differential | NO (in recorded envs) | Same skip gate. |
| mp3_start_time_watermark_is_exact | NO (in recorded envs); and weak even when run | Skip gate + vacuous `unwrap_or(true)` (F-004). |
| h264_start_time_is_the_honest_noop | NO (in recorded envs) | Skip gate. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "mp3/mp2 … lanes decode end-to-end — decoders wired (pipeline_host mp3/mp2 arms)" | es.rs:9-14 | REAL | pipeline_host.rs:261-270; committed-fixture test ran & passed (baseline no-failfast:2792,2795). |
| "h264/hevc … refuse decode honestly — no software decoder … provider answers cli.codec_unavailable" | es.rs:15-24 | FALSE (h264) / REAL (hevc) | pipeline_host.rs:346 + sw.rs:182-184,442-445 vs `other` arm 372-379; baseline INDEX.md CORRECTION v2:104-118. |
| "no raw-FLAC demuxer exists — crates/demux/src/es covers h264/hevc/mp3/aac only" | es.rs:25-27, 265-268 | REAL | LS crates/demux/src/es: aac, mp3, hevc, h264, annexb, fuzz, tests, mod, error — no flac. |
| "the `annexb` family id never reaches the registry — probe mints it sub-threshold" | es.rs:31-34 | UNVERIFIED | Threshold logic lives in media-probe/src/signature/es.rs (another wave's assignment); registry side (containers/mod.rs:60-61) has no `annexb` arm — consistent, but probe behavior not traced here. |
| "MPEG-audio watermark is exact (drops precisely the frames that predate the target)" | es.rs:60-64 | REAL-UNPROVEN | Drop rule lives in `SelectDemuxer` (demux_host.rs part2, W13-A03); es side only passes the watermark; test skipped in recorded runs. |
| "Annex-B watermark judges nothing … open still succeeds with the honest no-op" | es.rs:64-67 | REAL-UNPROVEN | Same — wrapper rule cross-agent; test skipped. |

## 7. Dead / duplicated / unreachable code
None. All five production functions have exactly one consumer (containers/mod.rs:62-66, rg-verified). No duplicated harvest logic between the two lanes (they differ honestly: geometry vs none).

## 8. Dependencies & flags
Depends on: `commands` (CommandError/Stage/ErrorCode), `demux::es::{annexb, mp3}`, `media_types`, `plugin_api` (Demuxer/SourceStream), `crate::demux_host` types. Test deps: `codec_api`, `playback_engine::DecoderProvider`, `media_clock`. No feature gates on this file's code paths (registry arms unconditional). No unsafe. `lumen-allow-literal` annotations at 298/302 are test-scope and honest about what they excuse.

## 9. Open questions for the next agent
- W13-A03 (demux_host part2): confirm `SelectDemuxer`'s skip rule actually drops on `Present(pts) < watermark` and passes `Missing` — the es.rs "exact watermark"/"honest no-op" claims rest on it.
- Whoever owns media-probe: verify the "annexb mints sub-threshold" claim (es.rs:31-34) and that `mp3`/`mp2`/`h264`/`hevc`/`flac` probe ids are reachable for real files — this file's registry arms are dead without them.
- The engine's `NoDecodableTrack` claim (es.rs:21-22): does an all-h264 session actually land there now that the provider succeeds? Trace in playback-engine if assigned.
