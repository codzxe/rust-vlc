# AUDIT: apps/cli/src/containers/mkv.rs#part1 (lines 1–651)
| Field | Value |
|---|---|
| Auditor | W13-A09 |
| Date | 2026-11-21 |
| Lines / functions | 651 (of 1304; chunk boundary pinned from audit/waves/wave-00/assignments.tsv:808 — part2=652–1304 is A08's) / 17 items |
| File verdict | REAL (one S2 spec misreading in the Vorbis lane, proven end-to-end with the shipped binary) |
| Reachable from shipped entry point? | YES. Chain: `lumen` main.rs → host.rs:904 `open_source` → demux_host.rs:362 `open_source` → demux_host.rs:458 `crate::containers::open` → containers/mod.rs:57 `"matroska" => mkv::open(stream, opts)`. EXECUTED: `printf 'quit\n' | ./target/release/lumen open tests/fixtures/generated/valid/mkv_h264_aac.mkv` → `state-changed Buffering → Playing → Stopped → Idle`, EXIT=0 (full run below). |
| Findings | S0:0 S1:0 S2:1 S3:3 S4:1 |
| Commands run | Read mkv.rs 1–119/120–359/359–658/659–748/1075–1104/1243–1304; rg `mkv::open|matroska` apps/cli/src (callers → containers/mod.rs:57); rg `open_source|containers::open` (chain); sed quotes 268–296/52–58/340–347/635–639; rg `pub struct TrackPrivate|codec_private` crates/demux/src/mkv/header.rs:155-160 (raw private bytes, lacing NOT pre-stripped); rg `audio_geometry|\.pcm` apps/cli/src (consumers → demux_host.rs:202, host.rs:760/778/918/1457, pipeline_host.rs:1587); sed pipeline_host.rs 1587–1609 + rg `spec.rate == 0` crates/audio-engine/src/pipeline.rs:114 (zero-rate typed guard); rg `lumen_cli` audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt (CLI unit tests ran: lines 2769–2809, 2900); ls /tmp/my-project/corpus-v4/media → ABSENT. LIVE experiments (allowed tools only, no cargo): ffmpeg 7.1.5 generated vorbis MKVs; hand-crafted EBML files with three lacing shapes; `lumen open` + ffprobe on each (outputs pasted in F-001/F-002). |

## 1. One-paragraph verdict
This is real, reachable, mostly disciplined container-host code: the open path genuinely constructs the demuxer, walks EBML, harvests TrackEntry facts, and the committed-fixture tests decode real bytes through the real provider (all green in baseline `orchrun-test-workspace-nofailfast.log.txt:2804-2810,2900`). But the Vorbis CodecPrivate parser `vorbis_chain` misreads the Matroska lacing count byte — it treats byte 0 as "number of length bytes" when the spec and every reference implementation (ffprobe/ffmpeg `put_xiph_size`) define it as "number of headers − 1" with 0xFF-continued variable-length sizes. I built two byte-identical-except-lacing MKVs: the ffmpeg-shaped one plays (EXIT=0); the spec-valid one whose comment header is 300 bytes — accepted by ffprobe — is refused by the shipped binary (`vorbis.headers.missing`, EXIT=65). The unit test pins the WRONG semantics with a spec-invalid vector. The module doc compounds this with a corpus-fitted "measured shapes" truth table and a stale claim that no software H.264 decoder is dispatched (it is, and an h264 MKV plays).

## 2. What actually works (verified)
- **Full open→play on a real MKV**: `printf 'quit\n' | timeout -k 5 40 ./target/release/lumen open tests/fixtures/generated/valid/mkv_h264_aac.mkv` → `event: tracks 2 ready`, `MediaInfoReady … tracks: [Video h264, Audio aac]`, `event: PlaybackEndOfTrack { track_kind: Video }`, `Buffering → Playing → Stopped → Idle`, EXIT=0. h264 4:2:0 in Matroska decodes in the shipped build.
- **Vorbis lacing `02 1e 5d` (all corpus instances)**: my crafted `mkv_ctrl2.mkv` with exactly ffmpeg's own lacing opens and plays end-to-end (EXIT=0) — the happy path is real.
- **Fixture decode tests ran green at baseline**: `orchrun-test-workspace-nofailfast.log.txt:2804-2810` (`flac_fixture_opens_and_decodes_the_full_stream`, `h264_aac_fixture_opens_with_expected_facts`, `multitrack_fixture_deselects_subtitles_at_open`, `six_channel_aac_geometry_flows_but_the_decoder_refuses`, `start_time_skips…` "ok") and `:2900` (`webm_vp9_and_opus_decode_from_the_private_head`). Fixtures exist under tests/fixtures/generated/valid/ (ls verified).
- **Subtitle deselect at open** (module doc line 108-111): asserted by `multitrack_fixture_deselects_subtitles_at_open` (facts carry 4 tracks, wrapper `start()` answers 3, subrip packets still demux — counts `[20, 88, 20, 2]`), baseline green.
- **`flac_streaminfo` extraction**: fixture test asserts the 34-byte STREAMINFO out of the `fLaC`-prefixed private + full-stream decode 88,200 sample frames (baseline green); hand-built negative cases (wrong block type, lying length) refuse.
- **Duration honesty**: `duration: Unknown` in my live MediaInfoReady output; `h264_aac` test asserts `opened.facts.duration_us() == None` (line 473) — matches the doc's "no per-track length" claim.

## 3. Findings

### F-001 [S2] — `vorbis_chain` decodes a lacing format the Matroska spec (and ffmpeg) does not define; valid A_VORBIS files with ≥255-byte headers fail to open
Where: apps/cli/src/containers/mkv.rs:268-295 (bug), 714-722 (test pinning the wrong shape — part2's range but it is this function's contract), 20-27 (module doc)
Code:
```
    let count = *private.first()? as usize;
    if count == 0 || count > 8 {
        return None; // a count outside any sane packing
    }
    // The 0xff-padded raw lengths (each group ends at a byte < 0xFF).
    let mut lens: Vec<usize> = Vec::new();
    let mut at = 1usize;
    let mut acc = 0usize;
    let mut read = 0usize;
    while read < count {
```
What is wrong: The code reads exactly `count = private[0]` bytes as length bytes. The Matroska CodecPrivate for A_VORBIS is `[nheaders-1][xiph-laced size of header 1][xiph-laced size of header 2][headers…]` — byte 0 is the header count minus one (always 2 for Vorbis), and each size is a variable-length 0xFF-continued integer, so a ≥255-byte identification or comment header occupies MULTIPLE length bytes while byte 0 stays 2. `while read < count` then consumes only 2 bytes, `lens.len() + 1 != 3` (or `acc != 0`) fires, and the blob is refused.
Why it matters: I proved it end-to-end. Two MKVs differing ONLY in the lacing bytes (same 30-byte id header, same setup header; comment 93 vs a structurally valid 300-byte comment `03 76 6f 72 62 69 73 …`):
- `mkv_ctrl2.mkv` (private `02 1e 5d …`, ffmpeg's own output shape): `lumen open` → `Buffering → Playing → Stopped → Idle`, EXIT=0. ffprobe: `codec_name=vorbis`.
- `mkv_big2.mkv` (private `02 1e FF 2D …` — exactly what ffmpeg's `put_xiph_size` writes for sizes ≥255, ffprobe reads it fine): `lumen open` → `error: [decode.unsupported]: unsupported in vorbis (vorbis.headers.missing): no extra_data: the decoder requires the stream's three header packets …`, `Opening → Error`, **EXIT=65**.
- The inverse: `mkv_code3.mkv` with private `03 FF 01 08 …` (the code's own 0xFF unit-test vector, line 716) — `vorbis_chain` ACCEPTS it while ffprobe answers `[vorbis @ …] Extradata corrupt.` — the code accepts shapes the reference rejects and rejects shapes the reference accepts.
Any real-world Vorbis file whose comment header exceeds 254 bytes (long metadata, `METADATA_BLOCK_PICTURE` cover art — ubiquitous in tagged music) cannot be opened, solely because of this misreading. The refusal is typed and honest — but the lane is dead for a class of valid files, and the module doc's claim "Vorbis `extra_data` = the three-header chain joined VERBATIM after stripping the Matroska Xiph lacing" is false for that class.
What correct looks like: parse byte 0 as `nheaders - 1` (require exactly 2), then read exactly two 0xFF-continued sizes (loop until a byte < 0xFF per size), then the headers. The repo's own block-level `xiph_sizes` (crates/demux/src/mkv/cluster.rs:283-295) already implements continuation-aware reading — the CodecPrivate parser must match it.
Confidence: HIGH — Verification: read + ran (shipped `lumen` binary vs ffprobe 7.1.5 on hand-crafted files, outputs pasted above; fixture files kept at /tmp/w13a09 for the wave gate).

### F-002 [S3] — Module doc claims no software h264 decoder is dispatched; the shipped binary decodes h264-in-MKV
Where: apps/cli/src/containers/mkv.rs:55-58
Code:
```
//! avcC `configurationVersion` 1 — ffmpeg's extradata shape). No
//! software h264/hevc decoder is dispatched in this build: the
//! blob is the forward-honest config, and `decoder_for` refuses
//! the codec typed (`cli.codec_unavailable`) — the hw-path cards
```
What is wrong: `apps/cli/src/pipeline_host.rs:346` dispatches `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new)` (rg verified; `use codecs::video::h264::H264SwDecoder` at :65), and the shipped binary played `tests/fixtures/generated/valid/mkv_h264_aac.mkv` (yuv420p per ffprobe) to `PlaybackEndOfTrack` with EXIT=0 (my run, §2). The hevc half of the sentence remains true (no `"hevc"` arm in pipeline_host).
Why it matters: the doc tells the next maintainer the h264 blob is dead weight consumed by nobody — false; it is the live configure path of the working h264 lane. Baseline INDEX CORRECTION v2 already established the sw decoder exists; this comment predates it and was never updated.
What correct looks like: delete the stale sentence, or state the truth: h264 configures `H264SwDecoder` (4:2:0 8-bit; 4:4:4 refuses typed), hevc refuses typed.
Confidence: HIGH — Verification: read + ran (live `lumen open` on the h264 fixture, EXIT=0).

### F-003 [S3] — `audio_pair` mints a 0 Hz rate for 0 < SamplingFrequency < 0.5
Where: apps/cli/src/containers/mkv.rs:340-347
Code:
```
fn audio_pair(private: &TrackPrivate) -> Option<(u32, u32)> {
    let channels = private.channels?;
    let rate = private.sampling_frequency?;
    if channels == 0 || channels > 64 || !rate.is_finite() || rate <= 0.0 || rate > 4_096_000.0 {
        return None;
    }
    Some((channels as u32, rate.round() as u32))
}
```
What is wrong: the guard excludes only `rate <= 0.0`; a hostile/absurd EBML float like `0.4` passes and `rate.round() as u32` = 0, so `TrackFact.pcm = Some((channels, 0))`. The doc contract at line 83 says "a non-finite or implausible value yields no pair, never a guess" — 0 Hz is the implausible guess.
Why it matters: `audio_geometry()` (demux_host.rs:206) answers the pair verbatim; `host.rs:918` feeds it to `pipeline_host::open_audio_session(channels, rate, …)`. Downstream is saved only by a typed guard in another crate — `crates/audio-engine/src/pipeline.rs:114` `if spec.rate == 0 … return Err(… "audio.pipeline_shape_invalid")` (rg + sed verified) — so the session open fails with an audio-stage error instead of the intended limb-less open. No panic (verified: the guard returns Err, not a divide), hence S3 not S1.
What correct looks like: reject `rate < 0.5` (or round first and check the rounded value > 0) in `audio_pair` itself.
Confidence: HIGH — Verification: read (guard chain traced to audio-engine/src/pipeline.rs:114-116); not executed (would need a crafted MKV with a 0.4 Hz Audio master — low value, the downstream refusal is proven by code).

### F-004 [S3] — Corpus-gated test silently PASSES when its fixture is absent; baseline green rows prove nothing for the seek lane
Where: apps/cli/src/containers/mkv.rs:635-639 (test body 635-664 straddles the part boundary at 651; assertions 640-664 — flagging to A08 so the straddle is not double-counted)
Code:
```
    fn start_time_skips_packets_and_reports_indexed_seek() {
        let Some(dir) = corpus_dir() else {
            skip_loudly();
            return;
        };
```
What is wrong: when `/tmp/my-project/corpus-v4/media` is absent and `LUMEN_CORPUS` unset (true in THIS sandbox — `ls /tmp/my-project/corpus-v4` → No such file or directory), the test prints to stderr and returns Ok. Cargo counts it "ok" and swallows the stderr of passing tests, so the baseline row `containers::mkv::tests::start_time_skips_packets_and_reports_indexed_seek ... ok` (orchrun-test-workspace-nofailfast.log.txt:2809) is indistinguishable from a real pass. The "loud skip" is loud only to a human running --nocapture.
Why it matters: this is the only test covering `--start-time` seek-query validation + `Fast { indexed: true }` for the MKV lane; the module doc (lines 102-107) leans on it. Evidence inflation: green CI cannot detect the corpus being missing.
What correct looks like: return `Err` / mark ignored dynamically, or assert at least one corpus file was consumed (e.g., fail if the gate skipped inside a CI-flagged env), so absence is visible in the test result line.
Confidence: HIGH — Verification: read + ran (`ls /tmp/my-project/corpus-v4/media` → absent here; baseline log line 2809 shows "ok" either way).

### F-005 [S4] — Module doc's "every shape below MEASURED on this corpus instance this session" bakes single-corpus overfitting into the contract
Where: apps/cli/src/containers/mkv.rs:16-17
Code:
```
//! source — the CodecPrivate bytes plus the Audio/Video masters'
//! own declarations, every shape below MEASURED on this corpus
//! instance this session):
```
What is wrong: the truth table is normed on one corpus instance (143 files, per docs/worklog/archive-2026-10.md:10029). F-001 is the direct consequence: the one corpus shape (`02 1e 5d`, comment 93 bytes) was generalized into a parser that rejects the spec's general form.
Why it matters: future maintainers treat "measured on this corpus" as the spec. The Matroska spec, not the corpus instance, is the contract.
What correct looks like: cite the spec semantics for each shape and mark corpus measurements as examples, not definitions.
Confidence: HIGH — Verification: read (consequence demonstrated live in F-001).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc (truth table) | 1-119 | SHALLOW-as-doc | S3/S4 | vorbis bullet FALSE for ≥255-byte headers (F-001); h264 bullet FALSE (F-002); flac/aac/opus/vp9/subtitle/duration bullets verified live or by baseline tests |
| `open` | 142-187 | REAL | — | clamps `--start-time` (MAX/4 fallback when no duration — honest), pure `seek` query gate, `saturating_mul` µs→ns (no overflow), SelectDemuxer wrap with id→base map; executed live twice (§2) |
| `harvest_facts` | 195-233 | REAL | — | ordinal↔handle lookup O(n²) trivial; missing entry → limb-less facts (honest); span end Missing per demuxer design |
| `config_blob` | 239-257 | REAL | — | per-codec arms match verified shapes; empty private → None (decoder refuses honestly) |
| `vorbis_chain` | 268-313 | REAL but WRONG | **S2** | F-001; bounds via `.get`/`checked_add` are safe; magic gates good; the count semantics is the defect |
| `flac_streaminfo` | 320-334 | REAL | — | bare-34 fallback + verified block header; negative cases unit-tested (part2 range) |
| `audio_pair` | 340-347 | REAL | S3 | F-003 (0 < rate < 0.5 → 0 Hz pair); channels 1..=64 good |
| `dimensions_of` | 350-357 | REAL | — | zero/u32-overflow guarded |
| `tests::fixture` | 373-377 | REAL | — | committed fixtures, exist (ls) |
| `tests::open` | 380-385 | REAL | — | drives the REAL `open_source` path (not a mock) |
| `tests::CountingSink::accept` | 390-407 | REAL | — | counts sample frames via `frames_in` (correct: samples are bytes) |
| `tests::decoder_for` | 412-426 | REAL | — | real `HostDecoderProvider`; refusal test (six-channel) uses it too |
| `h264_aac_fixture_opens_with_expected_facts` | 435-475 | REAL | — | asserts concrete blob bytes (avcC[0]==1, 5-byte ASC, AOT LC), dims, cadence — would fail if harvest broke; baseline :2806 ok |
| `flac_fixture_opens_and_decodes_the_full_stream` | 483-525 | REAL | — | 20 packets → 88,200 sample frames through the real provider; baseline :2804 ok |
| `multitrack_fixture_deselects_subtitles_at_open` | 532-560 | REAL | — | subtitle deselect + packet counts [20,88,20,2]; baseline :2807 ok |
| `webm_vp9_and_opus_decode_from_the_private_head` | 570-628 | REAL | — | vp9 20 frames + opus 101 packets → 96,648 samples; baseline :2900 ok; geometry via OpusHead sniff asserted |
| `start_time_skips_packets_and_reports_indexed_seek` | 635-664 (straddles 651) | REAL-UNPROVEN | S3 | F-004: corpus-gated silent pass; assertions themselves strong (watermark ≥ 500 ms, Fast{indexed:true}, `seen > 0`) |

## 5. Test honesty (tests starting in lines 1–651)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| h264_aac_fixture_opens_with_expected_facts | YES | asserts exact blob bytes/dims/cadence/pair against the real open path; mutation of `config_blob`'s h264 arm breaks `avcc[0] == 1` |
| flac_fixture_opens_and_decodes_the_full_stream | YES | 88,200-frame count comes from the real decoder fed by the harvested STREAMINFO; breaking `flac_streaminfo` fails the unwrap at :491 |
| multitrack_fixture_deselects_subtitles_at_open | YES | deselect is asserted against the wrapper's `start()`; counts are ffprobe-pinned |
| webm_vp9_and_opus_decode_from_the_private_head | YES | full decode counts for both tracks through the real provider |
| start_time_skips_packets_and_reports_indexed_seek | ONLY IF THE CORPUS EXISTS | absent corpus → unconditional "ok" (F-004); in this sandbox the corpus is absent, so the baseline pass is ambiguous |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Vorbis extra_data = three-header chain joined VERBATIM after stripping the Matroska Xiph lacing" | mkv.rs:19-27 | **FALSE** for headers ≥255 bytes | live: `mkv_big2.mkv` (ffprobe-valid) → `vorbis.headers.missing`, EXIT=65 (F-001) |
| "No software h264/hevc decoder is dispatched in this build" | mkv.rs:55-58 | **FALSE** (h264 half) | pipeline_host.rs:346 + live h264 MKV play, EXIT=0 (F-002) |
| FLAC extra_data = 34-byte STREAMINFO | mkv.rs:28-32 | REAL | fixture test baseline :2804 ok |
| AAC extra_data = bare 5-byte ASC | mkv.rs:33-40 | REAL | h264_aac test asserts `asc.len()==5`, `asc[0]>>3 == 2` (:465-466), baseline :2806 |
| av1 configures from in-band OBUs alone ("0034 decodes 101 frames") | mkv.rs:63-67 | REAL-UNPROVEN | corpus-gated (0003/0034 corpus files absent here); code arm (`_ => None` for av1) consistent |
| Duration NOT per-track; MAX/4 clamp | mkv.rs:98-104 | REAL | test :473 `duration_us() == None`; my live run shows `duration: Unknown` |
| Subtitles deselect at open | mkv.rs:108-111 | REAL | multitrack test baseline :2807 ok |
| Seek: Cues → Fast{indexed:true} (all corpus lanes but 0083) | mkv.rs:105-107 | REAL-UNPROVEN | only the corpus-gated start_time test asserts it (F-004); my no-Cues crafted file still reports `seekable: true` (weaker claim, demuxer-domain) |

## 7. Dead / duplicated / unreachable code
None in lines 1-651. All helpers are used; `open` is the single entry from the registry. (`corpus_dir`/`skip_loudly` at 1083-1098 are part2's range but are the shared gate for the straddling test — noted, not audited.)

## 8. Dependencies & flags
No feature gates in this file; compiled unconditionally in `lumen-cli` lib (no cfg walls). External deps: none beyond workspace crates. The `lumen-allow-literal` markers appear only in part2's corpus helpers (1084-1096) — none in part1.

## 9. Open questions for the next agent
- A08 (mkv.rs#part2): the 0xFF unit vector at 714-722 pins the wrong lacing semantics — fix belongs with F-001; the corpus differential (1101+) would MISS this class (all corpus comments ≤ 254 bytes).
- A08 (host.rs#part2): big2 exited 65 while the MP4 decode-refusal smoke exits 0 — where is the exit-code divergence decided? (my two live runs differ only in container/format).
- Demux wave: does the A_VORBIS id→descriptor map (header.rs:751-808 per open-path-wiring.md:329) also gate on private shape? If so, F-001's blast radius starts even earlier.
- Test fixtures for F-001 are preserved at /tmp/w13a09/{mkv_ctrl2,mkv_big2,mkv_code3}.mkv for the wave gate.
