# AUDIT: apps/cli/src/containers/mp4.rs

| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-22 |
| Lines / functions | 1176 / 14 production + 18 test items |
| File verdict | REAL-UNPROVEN (production half) / SHALLOW (corpus-differential half) |
| Reachable from shipped entry point? | YES — main.rs:19 `run::run` → run.rs:165/198/272 `lumen_dispatch` → dispatch.rs:82 `host.open_session` → host.rs:904 `open_source` → demux_host.rs:444-448 `media_probe::probe_stream` names format `mp4` → demux_host.rs:458 `crate::containers::open` → containers/mod.rs:67 `"mp4" => mp4::open(stream, opts)` → mp4.rs:95 `open` → mp4.rs:100 `demux::mp4::Mp4Demuxer::new` (crates/demux/src/mp4/mod.rs). `apps/desktop` reaches the same code via `lumen-cli = { path = "../cli" }` (apps/desktop/Cargo.toml:40). |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 3 |
| Commands run | `rg`/`sed -n` quote verification (all quotes below verified verbatim); `ls` fixture presence; baseline-log forensics (audit/runs/baseline/*). NO cargo (adaptation) — runtime claims cite W00 baseline outputs or are UNVERIFIED. |

## 1. One-paragraph verdict

Answer to the hostile question first: this is **NOT a second MP4 container parser**. The moov/stbl/sample-table walk is delegated wholesale to `demux::mp4::Mp4Demuxer` (mp4.rs:100-101); crates/demux/src/mp4/mod.rs is the one and only parser that runs on `lumen open file.mp4` (chain above). What mp4.rs DOES duplicate is the **sample-entry/esds child-box walking** (~120 lines, `find_child_box`/`child_box_full`/`child_header`/`dsi_of_esds`) — a second copy of logic `crates/demux/src/mp4/mod.rs:542-599` already implements (`oti_of_esds`/`esds_oti`, same `[28, 76, 78]` candidate offsets), living in the app layer because the demuxer's own seam doc says the parse "belongs to the codec cards" and neither codec nor demuxer implemented it. The production half is careful, bounds-checked code; the test half repeats mkv.rs's two structural defects: a corpus differential that self-skips as PASS when the corpus is absent (it is absent here, and the W00 baseline never ran any lumen-cli test), and h264-refusal expectations contradicted by `pipeline_host.rs:346` (which dispatches an H264 software decoder). The W00 smoke proves the open path itself works on h264+aac MP4 (2 tracks, Playing, duration known) and that 0 frames decode — the failure is downstream of this file, but this file's docs tell the wrong story about why.

## 2. What actually works (verified)

- Reachability chain above, verified site-by-site with `rg`/`sed`.
- W00 baseline smoke (audit/runs/baseline/mp4-smoke2-stderr.txt): `open /…/sample-h264-aac.mp4` → `event: tracks 2 ready` … `state-changed Buffering → Playing`, `duration 3.000000s` — the mp4 open path (probe → registry → this file's `open` + harvest) runs end-to-end in the shipped binary. The same file shows `time 0.000000s` and no frames — downstream failure, not this file's harvest (facts are present; `seekable: true`).
- W00 baseline decode-bench (mp4-smoke-bench.txt): `sample-mjpeg-pcm.mov` (container mp4) → `v_decoded=75 a_decoded=130` — the mjpeg+pcm MP4 path decodes fully through this file's harvest (geometry pair, no blobs).
- All production slicing is bounds-checked (`get` with range, `checked_add`, `size < 8` structural-lie break, overrun → `None`); no `unwrap`/indexing panic path found on hostile input (reading).
- Fixtures used by non-corpus tests exist: `tests/fixtures/generated/valid/{mp4_h264_aac.mp4, mp4_qt.mov, mp4_edts.mp4, mp4_mjpeg_pcm.mp4, mp4_chapters.mp4}` (verified `ls`).

## 3. Findings

### F-001 [S2] — Stale "no software h264 decoder" doc + corpus expectations contradicted by the shipped provider

Where: apps/cli/src/containers/mp4.rs:28-32 (doc), 1001-1016 (assertion); contradicting apps/cli/src/pipeline_host.rs:341-346

Code (mp4.rs:28-32, verbatim):
```rust
//!   (the config record WITHOUT the box header — ffmpeg's extradata
//!   shape). No software h264/hevc decoder is dispatched in this
//!   build: the blob is the forward-honest config, and
//!   `decoder_for` refuses the codec typed (`cli.codec_unavailable`)
//!   — the hw-path cards own the seam.
```

Code (pipeline_host.rs:346, verbatim):
```rust
            "h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new),
```

What is wrong: identical to mkv.rs F-001. The doc's claim is FALSE for h264 (`H264SwDecoder` imported at pipeline_host.rs:65, dispatched at :346, ADR-001 "Lead-admitted 2026-11-15"). The corpus differential's refusal assertion therefore cannot hold for the ~17 h264 lanes (0008, 0012, 0021, 0025, 0027, 0032, 0049, 0058, 0060, 0062, 0069, 0074, 0082, 0105, 0109, 0112, 0122, 0061):

Code (mp4.rs:1008-1011, verbatim):
```rust
                        assert!(
                            matches!(
                                err.context.code.as_str(),
                                "cli.codec_unavailable" | "aac.engine.init"
                            ),
```
If `H264SwDecoder::configure` accepts the harvested avcC, `decoder_for` answers `Ok` and the closing cross-check `assert!(video.is_none(), "{name}: no video decoder expected")` (mp4.rs:1066-1068) fails; if configure refuses, its code matches neither accepted string. Either way the 31-lane differential cannot pass against the current tree with a corpus present.
Why it matters: this file is the recorded truth table for the MP4 open path — the project's litmus question — and its h264 story is wrong in both directions (doc says the provider refuses; the provider dispatches; and the observed runtime behavior per W00 is 0 frames for a third, different reason nobody has recorded here).
What correct looks like: doc + TRUTHS re-measured against the ADR-001 build.
Confidence: HIGH — Verification: read (syntactic contradiction); runtime test failure UNVERIFIED (no cargo, corpus absent).

### F-002 [S2] — Corpus differential self-skips as PASS; no baseline run ever executed it

Where: apps/cli/src/containers/mp4.rs:945-960 (gate), 964-967 (skip-and-return-Ok); 652-655, 1080-1083, 1112-1115 (three more gated tests)

Code (mp4.rs:950-951 + 964-966, verbatim):
```rust
        let default = PathBuf::from("/tmp/my-project/corpus-v4/media"); // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
        default.is_dir().then_some(default)
```
```rust
        let Some(dir) = corpus_dir() else {
            skip_loudly();
            return;
        };
```

What is wrong: 4 tests (`cover_art_m4a_opens_with_still_image_trak`, `corpus_iso_bmff_lanes_match_the_ffmpeg_truth`, `mpeg2_tagged_lane_opens_and_refuses_the_codec_typed`, `six_channel_aac_geometry_flows_but_the_decoder_refuses`) count as pass without asserting anything in a corpus-less environment. `/tmp/my-project/corpus-v4/media` does not exist here (verified `ls`); and the W00 baseline `cargo test --workspace` contains no `lumen_cli` binary at all (fail-fast stop at `first-vertical`, audit/runs/baseline/cargo-test.log:2499) — so this file's differential has NO recorded green run against the current tree, while its comments claim ffmpeg-measured truth per lane.
Why it matters: §3.4 — evidence-shaped tests that may never have run; combined with F-001 the lane table is provably stale, not merely unproven.
What correct looks like: `#[ignore]` + documented invocation, or a committed miniature corpus; and the baseline re-run with `--no-fail-fast`.
Confidence: HIGH — Verification: read + `ls` + baseline-log binary inventory (116 binaries, none lumen_cli).

### F-003 [S3] — Second copy of the demuxer's esds/child-box walk, in the app layer (the real layering answer)

Where: apps/cli/src/containers/mp4.rs:252-326 (readers + child-box walkers), 336-379 (esds DSI walk); duplicate of crates/demux/src/mp4/mod.rs:542-599

Code (mp4.rs:264-270, verbatim):
```rust
/// Find one child box of an `stsd` entry by fourcc and return its
/// PAYLOAD (after the 8-byte box header). Child config boxes ride
/// after the sample entry's fixed prefix, whose length depends on the
/// entry class: 28 for AudioSampleEntry (§12.4.3), 76/78 for
/// VisualSampleEntry (§12.1.3; ffmpeg pads to 78) — the demuxer's own
/// `esds_oti` candidate convention, mirrored (a wrong candidate walks
/// no well-formed child and falls through).
```

Code (crates/demux/src/mp4/mod.rs:582 + 666-667, verbatim):
```rust
    for start in [28usize, 76, 78] {
```
```rust
    /// The verbatim first `stsd` entry payload of a track — the
    /// bytes the engine hands the decoder as
    /// `DecoderConfig::extra_data` (the WAV `fmt ` precedent: the
    /// avcC/hvcC/esds/alac parse belongs to the codec cards).
```

What is wrong: the demuxer already contains an esds/child-box walker with the identical candidate-offset heuristic (`esds_oti`, mod.rs:576-599) and an expandable-length descriptor walk (`oti_of_esds`, mod.rs:542-570). The CLI re-implements the same machinery two levels deeper (`dsi_of_esds` walks the exact same 0x03/0x04 prefix the demuxer walks, then continues to 0x05). The demuxer's seam doc declares the parse "belongs to the codec cards" — but it landed in neither the codecs nor the demuxer; it landed in the app layer. This is a §3.5 layering violation / duplicated logic: container-format knowledge (ISO-BMFF sample-entry layout) now lives in apps/cli, and any other host that opens MP4 without going through lumen-cli must copy it a third time.
Why it matters: two copies of a subtle binary walk = double the drift surface; the F-001 staleness shows drift already happened to this file's docs.
What correct looks like: `Mp4Demuxer::decoder_config(track)` (or a `demux::mp4::stsd` helper) answering the per-codec blob, called by the host.
Confidence: HIGH — Verification: read of both files.

### F-004 [S3] — Module-doc geometry offsets contradict the implementation

Where: apps/cli/src/containers/mp4.rs:43-44 (doc) vs 386-389 (code)

Code (doc, mp4.rs:43-44, verbatim):
```rust
//! entry's own declaration — `channelcount` at offset 12, the
//! 16.16 `samplerate`'s integer half at offset 20 (the fields
```
Code (mp4.rs:387-388, verbatim):
```rust
    let channels = u16be(entry, 16)?;
    let rate = u16be(entry, 24)?;
```

What is wrong: the module doc says channelcount@12 / rate@20; the code (correctly, per ISO 14496-12 §12.4.3 and the function's own comment at mp4.rs:382-385: "channelcount (offset 16) and … integer half (offset 24)") reads 16/24. The fixture test (mp4.rs:536-537: `assert_eq!(af.pcm, Some((1, 44_100))` on the committed mp4_h264_aac.mp4) pins the code, so the module doc is the false half.
Why it matters: a doc that misstates binary offsets in the file whose job is recording those offsets is worse than no doc — the next editor "fixing" per the doc breaks audio geometry for every MP4.
What correct looks like: 16/24 in the module doc (or delete the duplicated spec text and point at the function).
Confidence: HIGH — Verification: read (fixture assertion corroborates the code).

### F-005 [S4] — `find_child_box` vs `child_box_full`: internal copy-paste

Where: apps/cli/src/containers/mp4.rs:271-288 vs 291-308

Code (mp4.rs:278-279 vs 298-299, verbatim):
```rust
            if kind == *magic {
                return entry.get(pos + 8..pos + size);
```
```rust
            if kind == *magic {
                return entry.get(pos..pos + size).map(|bytes| bytes.to_vec());
```

What is wrong: two structurally identical loops differing only in the returned slice bounds; a `with_header: bool` parameter or one walker returning the header position removes the twin. §3.1 copy-paste inside the same file.
Confidence: HIGH — Verification: read.

### F-006 [S4] — Candidate-walk heuristic can false-positive on hostile compressorname bytes

Where: apps/cli/src/containers/mp4.rs:271-288

Code (mp4.rs:272-279, verbatim):
```rust
    for start in [28usize, 76, 78] {
        if entry.len() < start + 8 {
            continue;
        }
        let mut pos = start;
        while let Some((size, kind)) = child_header(entry, pos) {
            if kind == *magic {
                return entry.get(pos + 8..pos + size);
```

What is wrong: for a VisualSampleEntry (real children at 78), the walk first tries 28 — inside the attacker-controlled `compressorname` field (payload 30..62). A crafted entry can plant "avcC"+plausible-size there, and the harvest returns attacker-chosen bytes as the h264 config record (the same exposure exists in the demuxer's `esds_oti`). Impact is bounded: every slice is bounds-checked and the decoder's `configure` gate validates the blob (garbage → typed refusal), so this is misdirection, not corruption or memory unsafety. A zero-size matching magic also short-circuits the remaining candidates (`get(pos+8..pos+0)` → `None` ends the whole search).
What correct looks like: choose the entry class from the fourcc (`mp4a`→28, video→78) instead of shotgunning candidates, and skip size<8 before the magic match.
Confidence: MEDIUM — Verification: read; hostile-input behavior UNVERIFIED (no run).
Severity rationale: bounded by decoder-side validation → S4, not S1.

### F-007 [S4] — `StartedMp4` adapter patches a demuxer API wart at the app layer

Where: apps/cli/src/containers/mp4.rs:145-179

Code (mp4.rs:147-153, verbatim):
```rust
/// A started MP4 demuxer behind the host's start-idempotence seam:
/// the CLI pre-pass calls `start()` once (the composition root's
/// read of the track list, doc §4.2), and the engine's own `start()`
/// would hit the demuxer's deliberate second-call refusal
/// (`demux.mp4.not-started` — unlike ADTS/AVI, which answer the
/// cached list).
```

What is wrong: MP4 is the only wired family whose demuxer refuses a second `start()`, so this host alone carries an adapter that clones the track list (`Ok(self.tracks.clone())` per call). Honest and documented, but the inconsistency is a demuxer-crate API bug being absorbed by the app layer (the "STOP rule" is cited as the reason not to fix it where it lives).
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open` | 95-143 | REAL-UNPROVEN | — | Reachable (chain §top). MAX/4 clamp fallback duplicated 7× (see mkv.rs F-005). `saturating_mul` µs→ns. Seek query gates `--start-time`. |
| `struct StartedMp4` + `Demuxer` impl | 155-179 | REAL-UNPROVEN | S4 (F-007) | Idempotence adapter; `reanchor` forwards the demuxer's typed refusal (in-session seek gap, honestly noted). |
| `harvest_facts` | 186-226 | REAL-UNPROVEN | — | One fact per track; `frame_ns: None` honest (VFR family). Silent `None` when `sample_entry_bytes` misses (same degradation class as mkv F-004). |
| `config_blob` | 229-250 | REAL-UNPROVEN | — | Per-codec dispatch; empty match → None for mp3/mjpeg/pcm/mov_text. |
| `u16be`/`u32be` | 253-262 | trivial-correct | — | Bounds-checked; fixed offsets only. |
| `find_child_box` | 271-288 | REAL-UNPROVEN | S3/S4 (F-003/F-005/F-006) | Candidate heuristic [28,76,78]; duplicated from demux layer. |
| `child_box_full` | 291-308 | REAL-UNPROVEN | S4 (F-005) | Copy of the above with different slice bounds. |
| `child_header` | 312-326 | trivial-correct | — | Overrun → None; u32 size as usize (no overflow on 64-bit; on 32-bit usize cast of >4GiB size impossible from u32). |
| `dsi_of_esds` (+`expandable_len`) | 336-370 | REAL-UNPROVEN | S3 (F-003) | Second esds walker (demuxer stops at OTI, this continues to DSI). All bounds via `get`/`checked_add`; 4-iteration cap on continuation bytes. Unit-tested (1158-1175) incl. truncation refusal. |
| `asc_of_entry` | 375-379 | REAL-UNPROVEN | — | Composition of the two above; empty-DSI refused. |
| `audio_geometry` | 386-390 | REAL-UNPROVEN | — | Correct offsets 16/24 (F-004 is the doc, not the code); v0 sample entries only — QT v1/v2 sound descriptions would mis-parse (untested shape; UNVERIFIED presence in corpus). |
| `visual_dimensions` | 395-399 | REAL-UNPROVEN | — | 24/26 per §12.1.3 — correct. |
| `opus_head_of_dops` | 408-428 | REAL-UNPROVEN | — | Version gate, len≥11, family-0/1 handling, `get(11..)?` for the table; unit-tested (1131-1153). |
| tests: helpers | 445-496 | REAL-UNPROVEN | — | Real open path + real provider; no mocks. |
| `h264_aac_fixture_opens_with_expected_facts` | 504-541 | REAL-UNPROVEN | — | Byte-level pins (avcC v1, ASC AOT 2, 44.1k/1ch, dims 128×96) on a committed fixture. |
| `mov_and_edts_fixtures_open` | 545-557 | REAL-UNPROVEN | — | Family-wide (qt/edts) smoke. |
| `mjpeg_pcm_fixture_opens_with_pcm_facts` | 561-571 | REAL-UNPROVEN | — | PCM pair pins the geometry route (this lane demonstrably works per W00 bench: 75/130). |
| `start_time_skips_packets_and_reports_indexed_seek` | 577-603 | REAL-UNPROVEN | — | Committed fixture; skip watermark + `Fast{indexed}`. |
| `chapter_trak_is_hidden_and_markers_stay_demuxer_side` | 611-643 | REAL-UNPROVEN | — | Dual assertion incl. direct demuxer construction for chapters (honest gap recording). |
| `cover_art_m4a_opens_with_still_image_trak` | 650-703 | SHALLOW | S2 (F-002) | Corpus-gated. |
| `Lane`/`TRUTHS` (31 lanes) | 720-941 | STALE | S2 (F-001) | ~17 h264 lanes expect a refusal that no longer exists. |
| `corpus_dir`/`skip_loudly` | 945-960 | SHALLOW | S2 (F-002) | Skip-as-pass. |
| `corpus_iso_bmff_lanes_match_the_ffmpeg_truth` | 962-1070 | SHALLOW | S2 (F-001/F-002) | Cannot pass with corpus present per F-001; never ran in baseline. |
| `mpeg2_tagged_lane_opens_and_refuses_the_codec_typed` | 1078-1103 | SHALLOW | S2 (F-002) | Corpus-gated; the refusal it pins (`cli.codec_unavailable` for mpeg4video) is still true (no mpeg4video arm). |
| `six_channel_aac_geometry_flows_but_the_decoder_refuses` | 1110-1126 | SHALLOW | S2 (F-002) | Corpus-gated; consistent with pipeline_host's AAC arm. |
| `dops_synthesizes_the_rfc7845_opus_head` | 1131-1153 | REAL-UNPROVEN | — | Pure unit test; runs without corpus. |
| `esds_dsi_walk_extracts_the_asc` | 1158-1175 | REAL-UNPROVEN | — | Pure unit test incl. truncated-chain refusal. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| h264_aac / mov_edts / mjpeg_pcm / start_time / chapter tests | YES | Concrete pins on committed fixtures through the real open path (none ran in the W00 baseline — no lumen_cli binary — so even these are runtime-UNPROVEN). |
| dops / esds unit tests | YES | Hand-built vectors incl. adversarial arms. |
| The 4 corpus tests | NO (corpus-less env) | Return before asserting (F-002); with corpus, the h264 lanes fail for the WRONG reason (F-001). |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "No software h264/hevc decoder is dispatched in this build … refuses typed" | mp4.rs:28-32 | FALSE (h264 half) | pipeline_host.rs:346; import :65. hevc half still true (no arm). |
| "channelcount at offset 12 … samplerate … offset 20" | mp4.rs:43-44 | FALSE | Code reads 16/24 (mp4.rs:387-388); fn comment mp4.rs:382-385 and ISO §12.4.3 agree with the code. |
| "AAC extra_data = the esds AudioSpecificConfig … the extraction is this harvest's own truth" | mp4.rs:17-23 | REAL-UNPROVEN | dsi walk + fixture pin (mp4.rs:529-535); W00 bench shows AAC not decoded only in bench's own allowlist (bench.rs audit), not here. |
| "The opus decoder IS dispatched since 6.M-opus" | mp4.rs:36-37 | REAL | pipeline_host.rs:276-282 ("opus" arm) + W00: bench refusal for aac proves bench's list differs from provider's, and the corpus test expects opus decode via mkv lane. |
| "Chapters … an honestly-recorded gap" | mp4.rs:68-72 | REAL-UNPROVEN | `Mp4Demuxer::chapters` exists (mod.rs:654); TrackFact has no chapter vocabulary (demux_host.rs:144-170). |

## 7. Dead / duplicated / unreachable code

- No dead production code (all functions called; `open` dispatched at mod.rs:67).
- Duplication: F-003 (demux crate), F-005 (internal twin), MAX/4 clamp (7× across hosts).

## 8. Dependencies & flags

- No feature gates: `mod mp4;` unconditional (mod.rs:28), arm unconditional (mod.rs:67). Deps: demux, media-types, plugin-api, commands (path deps). No cfg(test) branches in production code.

## 9. Open questions for the next agent

- Where exactly does h264 MP4 decode die after `Playing`? W00 smoke: 0 frames, clock 0.000000s unless `advance` pumped; pipeline_host.rs:346 dispatches H264SwDecoder; the one-shot argv open TIMES OUT (baseline mp4-smoke.txt SMOKE 1 `exit=124`). Root cause is in host.rs/run.rs/engine/decoder — outside my assignment; Wave-20 A05 should trace it with the h264_sw_differential evidence (crates/codecs/tests/h264_sw_differential.rs).
- bench.rs's refusal string "no decoder in this build (first vertical: mjpeg + pcm)" fired for BOTH h264 and aac in the W00 bench (mp4-smoke-bench.txt) although pipeline_host dispatches both — bench.rs has its own codec allowlist (audited next in this agent's bench.rs report).
- Does any corpus lane use a QT v1+ AudioSampleEntry (children ≠ 28)? If yes, audio_geometry/dsi walk silently miss (UNVERIFIED — corpus absent).
