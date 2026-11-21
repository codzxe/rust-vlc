# AUDIT: apps/cli/src/containers/mp4.rs#part1 (lines 1–603)
| Field | Value |
|---|---|
| Auditor | W13-A07 |
| Date | 2026-11-21 |
| Lines / functions | 603 (of 1176) / 23 items (15 production + 8 test) |
| File verdict | REAL (open path baseline-proven; one S2 byte-order bug in the opus synthesis, one FALSE capability claim in the module docs) |
| Reachable from shipped entry point? | YES (call chain: `lumen open <mp4>` → run/dispatch → demux_host::open_source → containers/mod.rs:67 `"mp4" => mp4::open(stream, opts)`; executed by baseline mp4-smoke-open-session*.txt — `tracks 2 ready`, `MediaInfoReady { Video h264, Audio aac }`) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 0 S4: 4 |
| Commands run | `rg -n "mp4::open|mod mp4"` (containers/mod.rs:28,67 — probe dispatch); read crates/demux/src/mp4/mod.rs:1145–1191 (`sample_entry_bytes` payload semantics) and :669–672; read apps/cli/src/pipeline_host.rs:235–381 (decoder dispatch: h264 at 346, pcm_ gate at 347, refusal list at 372–379); `sed -n` on every quoted range; baseline INDEX.md corrected MP4 matrix |

**Chunk boundary note (for A05):** **part1 (this audit) = lines 1–603** (all
production code 1–428 + test helpers 431–496 + the first four tests through
`start_time_skips_packets_and_reports_indexed_seek`); part2 = lines 604–1176
(chapter/cover-art/corpus tests, `dops_synthesizes_the_rfc7845_opus_head`,
`esds_dsi_walk_extracts_the_asc`). Split at the end of the start-time test —
the natural item boundary nearest 1176/2 = 588.

## 1. One-paragraph verdict
This is the real MP4/MOV/M4A host pre-pass and it is the layer the baseline
smoke actually exercised: construct `Mp4Demuxer`, `start()`, harvest per-track
facts from the verbatim `stsd` entry (avcC/hvcC/av1C payloads, esds ASC walk,
ALAC cookie, dOps→OpusHead), clamp `--start-time`, wrap in `SelectDemuxer`.
The harvest offsets are correct (verified against ISO 14496-12 payload
semantics — `sample_entry_bytes` returns the entry payload after the 8-byte
box header, demux/src/mp4/mod.rs:1160–1165) and pinned by strong fixture
tests. Two S2s: `opus_head_of_dops` copies the big-endian dOps multi-byte
fields **verbatim** into the little-endian OpusHead — its own doc says
"re-packed little-endian … ffmpeg's `ff_mov_rewrite_dops` mapping" and the
implementation does no repacking (the part2 test even pins the unswapped
bytes); and the module docs flatly claim "No software h264/hevc decoder is
dispatched this build" — false for h264 (pipeline_host.rs:346 dispatches
`H264SwDecoder`; baseline 4:2:0 MP4 plays).

## 2. What actually works (verified)
- Open path executed: baseline `audit/runs/baseline/mp4-smoke-open-session-420.txt`
  (tracks ready → Playing) and `mp4-smoke-open-session.txt` (yuv444p: tracks 2
  ready, duration, MediaInfoReady h264+aac — the demux/harvest half of that
  transcript is THIS file's output shape).
- `h264_aac_fixture_opens_with_expected_facts` (504–541) asserts the avcC
  first byte, coded dimensions (128,96), the ASC's AOT (2=LC) and the geometry
  pair (1, 44100) against a committed ffmpeg fixture through the REAL
  `open_source` path — mutation check: shifting `audio_geometry`'s offsets to
  the module-doc's wrong 12/20 (F-003) would read zeros → `pcm: None` → the
  `assert_eq!(af.pcm, Some((1, 44_100)))` fails. The test would catch it.
- `sample_entry_bytes` payload semantics verified (demux/src/mp4/mod.rs:1153–1165:
  `entry = body.get(16..8 + entry_size)`, comment "the payload handed
  downstream starts after [the 8-byte header]") — which makes the code's
  offsets (16/24 audio, 24/26 visual, child candidates 28/76/78) internally
  consistent.

## 3. Findings

### F-001 [S2] — `opus_head_of_dops` copies big-endian dOps fields verbatim into the little-endian OpusHead — no repacking despite the documented ffmpeg mapping
Where: apps/cli/src/containers/mp4.rs:416–421 (doc at 401–407)
Code:
```rust
    head.extend_from_slice(&dops[2..4]); // pre-skip (LE)
    // The INPUT sample rate (advisory — the decoder output is 48 kHz;
    // audio_geometry keeps the 48 kHz rule on the OpusHead sniff).
    head.extend_from_slice(&dops[4..8]); // input rate (LE)
    head.extend_from_slice(&dops[8..10]); // output gain (LE)
```
What is wrong: dOps (ISO/IEC 23003-5) stores pre-skip, input rate and output
gain **big-endian**; RFC 7845's OpusHead stores them **little-endian**. The
function's own doc (404–407) says "all big-endian — re-packed little-endian
into the 19-byte OpusHead … ffmpeg's `ff_mov_rewrite_dops` mapping", but the
implementation is a byte-for-byte copy — no swap. ffmpeg's
`ff_mov_rewrite_dops` reads `AV_RB16/32` and writes `avio_wl16/32` (the
actual mapping the comment cites). The part2 test
`dops_synthesizes_the_rfc7845_opus_head` (1139–1141) asserts the UN-swapped
bytes (`&head[10..12] == &[0x00, 0x38]` "pre-skip LE" — correct LE for BE
0x0038 is `[0x38, 0x00]`), i.e. the golden pins the bug.
Why it matters: opus-in-MP4 is a dispatched lane (module doc: "The opus
decoder IS dispatched since 6.M-opus"). A real ffmpeg-muxed dOps with
pre-skip 312 (0x0138) yields an OpusHead claiming 0x3801 = 14 337 samples
(~0.3 s at 48 kHz) — wrong decoder-side trimming; a nonzero output gain is
byte-swapped garbage. Only palindromic field values (0, 0x0000ac44? no —
44100 is not palindromic) escape: in practice only zero pre-skip/zero gain
files decode correctly.
What correct looks like: `head.extend_from_slice(&u16::from_be_bytes([dops[2], dops[3]]).to_le_bytes())`
(and the u32 twin for the rate), then fix the part2 golden to assert the
swapped bytes.
Confidence: HIGH — Verification: read (dOps BE per ISO 23003-5 / ffmpeg's
`mov_read_dops` `avio_rb16/rb32`; OpusHead LE per RFC 7845 §5.1; the shipped
test's own comments name the mapping the bytes contradict).

### F-002 [S2] — Module docs claim no h264/hevc decoder is dispatched in this build — FALSE for h264
Where: apps/cli/src/containers/mp4.rs:29–32
Code:
```rust
//!   shape). No software h264/hevc decoder is dispatched in this
//!   build: the blob is the forward-honest config, and
//!   `decoder_for` refuses the codec typed (`cli.codec_unavailable`)
//!   — the hw-path cards own the seam.
```
What is wrong: `HostDecoderProvider::decoder_for` dispatches the software
H.264 decoder — apps/cli/src/pipeline_host.rs:346
`"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new)` — and the
baseline proves the lane: `audit/runs/baseline/mp4-smoke-open-session-420.txt`
shows a 4:2:0 H.264+AAC MP4 reaching **Playing** (decode happened; baseline
INDEX.md CORRECTION v2). The refusal fall-through's own message
(pipeline_host.rs:376–377) lists `h264` among the landed decoders. Only hevc
is actually refused.
Why it matters: this module doc is the file's own contract statement for the
harvest ("the blob is the forward-honest config" implying it is never
consumed). A reader auditing the MP4 lane from this doc would conclude the
avcC blob is dead weight and the 4:2:0 MP4 cannot play — the exact opposite
of the shipped truth. Doc-vs-code drift on the core lane.
What correct looks like: "the h264 blob seeds `H264SwDecoder` (ADR-001
software track); hevc has no decoder in this build and `decoder_for` refuses
it typed (`cli.codec_unavailable`)."
Confidence: HIGH — Verification: read + baseline run (mp4-smoke-open-session-420.txt).

### F-003 [S4] — Module doc names wrong AudioSampleEntry offsets (12/20); code and function doc are right (16/24)
Where: apps/cli/src/containers/mp4.rs:43–44 (module doc) vs 386–389 (code)
Code:
```rust
//!   entry's own declaration — `channelcount` at offset 12, the
//!   16.16 `samplerate`'s integer half at offset 20 (the fields
```
vs
```rust
fn audio_geometry(entry: &[u8]) -> Option<(u32, u32)> {
    let channels = u16be(entry, 16)?;
    let rate = u16be(entry, 24)?;
```
What is wrong: `sample_entry_bytes` returns the entry payload after the
8-byte box header (demux/src/mp4/mod.rs:1160–1165), so channelcount sits at
payload offset 16 and the samplerate's integer half at 24 — exactly what the
code and the function's own doc (382–385) say. The module doc's 12/20 is
stale/wrong by 4.
Why it matters: two docs in one file disagree; the wrong one is the one a
reader hits first.
What correct looks like: fix the module doc to 16/24 (or delete the offsets
and point at the function doc).
Confidence: HIGH — Verification: read (ISO 14496-12 §12.4.3 payload layout
recomputed + demux payload semantics verified).

### F-004 [S4] — `find_child_box` / `child_box_full` are copy-paste twins
Where: apps/cli/src/containers/mp4.rs:271–288 and 291–308
Code:
```rust
fn find_child_box<'a>(entry: &'a [u8], magic: &[u8; 4]) -> Option<&'a [u8]> {
    for start in [28usize, 76, 78] {
```
```rust
fn child_box_full(entry: &[u8], magic: &[u8; 4]) -> Option<Vec<u8>> {
    for start in [28usize, 76, 78] {
```
What is wrong: identical candidate-walk loops differing only in the returned
slice (payload vs header+payload). A third variant of the same walk exists in
the demux crate (`esds_oti`). Three copies of a parse walk is the classic
drift vector: a future fix (e.g. a new entry-class offset) must land in all
three.
Why it matters: duplication of security-relevant parsing.
What correct looks like: one `find_child<'a>(entry, magic) -> Option<&'a [u8]>`
returning the full box, with thin payload/header accessors over it.
Confidence: HIGH — Verification: read (both bodies compared line by line).

### F-005 [S4] — `dsi_of_esds` assumes the flags==0 ES_Descriptor shape
Where: apps/cli/src/containers/mp4.rs:357–358
Code:
```rust
    at = expandable_len(body, at + 1)?.0;
    at += 3; // ES_ID + flags
```
What is wrong: the walk hard-codes ES_ID(2)+flags(1) and ignores the optional
ES_Descriptor fields (dependsOn_ES_ID when streamDependencyFlag set, OCR_ES_ID,
URL string) and DecoderConfigDescriptor's upStream flag byte — i.e. it
assumes the minimal flags==0 chain. A legal esds with streamDependencyFlag
set misaligns the walk; the 0x04/0x05 tag checks then fail → `None` → the
AAC/mpeg4video track gets no config blob and decodes only if the codec can
sniff raw (AAC can, per the module doc's ADTS mode).
Why it matters: silent narrowing of the harvest on unusual-but-legal files;
the doc comment (330–335) documents the assumed shape but not the limitation
it imposes.
What correct looks like: honor the ES flags byte (`if flags & 0x40 { at += 2 }`
etc.), or document the refusal envelope as intentional.
Confidence: MEDIUM — Verification: read (MPEG-4 Systems descriptor grammar;
no fixture with a non-zero flags chain exists to prove the walk's behavior —
UNVERIFIED by execution).
Severity S4 (not S3): degradation is a typed-ish None → raw-mode fallback,
not a wrong decode.

### F-006 [S4] — `TrackFact.pcm` carries AAC geometry (field name lies)
Where: apps/cli/src/containers/mp4.rs:200–203
Code:
```rust
        let pcm = entry
            .filter(|_| track.kind == MediaKind::Audio)
            .and_then(audio_geometry)
            .filter(|_| codec.starts_with("pcm_") || codec == "aac");
```
What is wrong: for an `aac` track the field named `pcm` holds
(channels, rate). Documented in the module doc (46–52: "geometry-only, never
a decode route"), and both consumers I could verify gate on the codec
descriptor (pipeline_host.rs:347 `codec.starts_with("pcm_")`; the bench lane
per the module doc — bench.rs is A04's file this wave).
Why it matters: the safety of an AAC track depends on every future consumer
remembering the gate; the field name actively invites the mistake.
What correct looks like: rename to `audio_geometry` on the fact (it already
has `audio_geometry()` as an accessor — the field is the leftover).
Confidence: HIGH — Verification: read (both gates located; provider arm
quoted above).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open` | 95–143 | REAL | — | clamp `unwrap_or(i64::MAX/4)` for fragment-only (documented); `saturating_mul(1000)` us→ns; seek-then-watermark interplay is coherent (seek walks back to keyframe, `skip_until_ns` trims the gap — proven by the 578–603 test); typed `wire_error` mapping |
| `struct StartedMp4` | 155–158 | REAL | — | caches pre-pass track list (demuxer refuses second `start()` — idempotence adapter, documented) |
| `Demuxer::start` (StartedMp4) | 161–163 | REAL | — | clones cached list — cheap, cold path |
| `Demuxer::next_event` | 164–166 | REAL | — | verbatim forward |
| `Demuxer::seek` | 167–169 | REAL | — | verbatim forward |
| `Demuxer::seekability` | 170–172 | REAL | — | verbatim forward (test pins `Fast { indexed: true }`) |
| `Demuxer::reanchor` | 173–178 | REAL | — | forwards demuxer's typed refusal; "never invented here" — honest gap |
| `harvest_facts` | 186–226 | REAL | S4 | F-006 (pcm carries AAC pair); `frame_ns: None` per VFR rule (feeds pipeline_host's 33_333_333 fallback — cross-file note, see that audit's F-004); `span_end_ticks` from track timing |
| `config_blob` | 229–250 | REAL | — | per-codec dispatch matching the module-doc truth table; mpeg4video reuses the ASC walker (VOL start codes — same DSI slot, documented) |
| `u16be` | 253–256 | REAL | — | bounds-checked via `get` |
| `u32be` | 259–262 | REAL | — | bounds-checked |
| `find_child_box` | 271–288 | REAL | S4 | F-004 duplication; `size < 8` break guards structural lies; `entry.get(pos+8..pos+size)` — `get` with start>end returns None, no panic (size==0+magic hit → None, safe) |
| `child_box_full` | 291–308 | REAL | S4 | F-004 duplication |
| `child_header` | 312–326 | REAL | — | `pos + size > entry.len()` containment check; no overflow risk on 64-bit usize (size ≤ u32::MAX) |
| `dsi_of_esds` (+`expandable_len`) | 336–370 | REAL | S4 | F-005 flags==0 assumption; `expandable_len` caps at 4 continuation bytes; `checked_add` on DSI slice end; `!dsi.is_empty()` guard |
| `asc_of_entry` | 375–379 | REAL | — | composition of find_child_box + dsi walk |
| `audio_geometry` | 386–390 | REAL | — | offsets verified correct (F-003 is the module doc, not this); rejects zero channels/rate |
| `visual_dimensions` | 386–399 | REAL | — | offsets 24/26 verified against VisualSampleEntry payload layout; coded box not display matrix (documented rotation stance) |
| `opus_head_of_dops` | 408–428 | REAL | S2 | F-001 (no BE→LE repack); `dops.get(11..)` on an exactly-11-byte family>0 input yields Some(&[]) — empty mapping table copied verbatim (malformed-in → malformed-out; decoder's gate refuses; no panic); version gate refuses nonzero |
| test `fixture` | 445–449 | REAL | — | committed fixtures under tests/fixtures |
| test `open` helper | 452–457 | REAL | — | real `open_source` path; `unreachable!` on error = test fails loudly |
| test `CountingSink` | 461–478 | REAL | — | counts audio frames via `format.frames_in`, video +1; subtitle ignored (consistent with the consumer's swallow — pipeline_host F-003) |
| test `decoder_for` helper | 482–496 | REAL | — | exercises the REAL provider (facts-seeded), config with `extra_data: None` — the seeding path, not a mock |
| test `h264_aac_fixture_opens_with_expected_facts` | 504–541 | REAL | — | strong: real fixture, real path, blob/dims/AOT/geometry asserts (mutation analysis in §2) |
| test `mov_and_edts_fixtures_open` | 545–557 | REAL | — | medium: container/track-count/codec/duration asserts; two fixtures (qt.mov, edts.mp4) |
| test `mjpeg_pcm_fixture_opens_with_pcm_facts` | 561–571 | REAL | — | strong for the PCM pair + no-blob asserts |
| test `start_time_skips_packets_and_reports_indexed_seek` | 577–603 | REAL | — | strong: asserts start ns, indexed seekability, every packet ≥ target, non-empty tail; `Stamp::Present` guard skips Missing-pts packets (honest) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| h264_aac_fixture_opens_with_expected_facts | YES | Offsets mutated to the module-doc's 12/20 → geometry reads zeros → `assert_eq!(af.pcm, Some((1, 44_100)))` fails; avcC/ASC first-byte asserts pin the blob extraction |
| mov_and_edts_fixtures_open | YES (open-path breakage) / NO (harvest offsets) | asserts only container/tracks/codec/duration — a wrong blob harvest passes |
| mjpeg_pcm_fixture_opens_with_pcm_facts | YES | `pcm == Some((2, 44_100))` pins the pair; `extra_data == None` pins the no-blob rule |
| start_time_skips_packets_and_reports_indexed_seek | YES | asserts per-packet `ns.ticks >= 1_000_000` — an unseeded/failed seek leaks early packets and fails |
| (part2, cross-ref) dops_synthesizes_the_rfc7845_opus_head | NO — it certifies the bug | asserts the UN-swapped bytes while its comments claim the ffmpeg mapping (F-001 evidence; the test itself is A05's to audit) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "No software h264/hevc decoder is dispatched in this build" | module doc 29–32 (this file) | FALSE (h264) / TRUE (hevc) | pipeline_host.rs:346 dispatches `H264SwDecoder`; baseline mp4-smoke-open-session-420.txt plays a 4:2:0 H.264 MP4; refusal message lists h264 as landed (pipeline_host.rs:376–377) |
| "opus `extra_data` = OpusHead synthesized from dOps … re-packed little-endian" | module doc 33–37 + fn doc 404–407 | FALSE (the repack is not implemented) | code 416–421 copies verbatim; F-001 |
| "The opus decoder IS dispatched since 6.M-opus" | module doc 36–37 | REAL | pipeline_host.rs:276–282 `"opus" => self.seeded(track, config, OpusDecoder::new)` |
| "channelcount at offset 12, samplerate's integer half at offset 20" | module doc 43–44 | FALSE | code reads 16/24 and is correct (F-003 verification); function doc agrees with code |
| "AAC extra_data = the esds AudioSpecificConfig … the bare ASC" | module doc 17–23 | REAL | `asc_of_entry`→`dsi_of_esds` walk; pinned by test 529–535 (`asc[0] >> 3 == 2`) |
| "Duration IS declared … `--start-time` clamps against the real extent" | module doc 60–62 | REAL | `open` 106–107; test 539–540 |
| "Chapters … hidden from the track list; no chapter vocabulary yet" | module doc 68–72 | REAL (part2 test pins it — 605+ is A05's chunk) | harvest has no chapter vocabulary (verified: `MediaFacts` fields 221–225 carry none) |

## 7. Dead / duplicated / unreachable code
- `find_child_box`/`child_box_full` duplication (F-004).
- No dead code found in 1–603; every production fn is reachable from `open`
  (grep: `config_blob` ← `harvest_facts` ← `open` ← containers/mod.rs:67).

## 8. Dependencies & flags
- Consumes: `demux` (Mp4Demuxer), `commands` (CommandError), `media_types`,
  `plugin_api` (Demuxer/SourceStream/DemuxEvent/SeekSupport), `crate::demux_host`.
- Test-only: `testkit`, `playback_engine`, `codec_api`, `commands::catalog`.
- No feature gates in this file — the MP4 lane is unconditional in the
  shipped `lumen` binary (baseline engine-info + smoke prove it).

## 9. Open questions for the next agent
- A05 (part2 of this file): audit `dops_synthesizes_the_rfc7845_opus_head`
  as a bug-pinning golden (F-001) and the corpus tests' coverage of the
  geometry/harvest lanes; `esds_dsi_walk_extracts_the_asc` uses a flags==0
  hand-built chain — F-005's envelope is untested by construction.
- A04 (bench.rs): the module doc claims the bench lane gates `pcm` on the
  codec descriptor — verify the gate exists (F-006's safety depends on it).
- A02/A03 (demux_host.rs): `SelectDemuxer`'s skip watermark vs the pre-seek —
  confirm no double-trim interaction (test 577–603 suggests none).
