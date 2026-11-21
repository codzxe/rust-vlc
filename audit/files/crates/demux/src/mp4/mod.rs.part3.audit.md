# AUDIT: crates/demux/src/mp4/mod.rs#part3 (lines 1113-1668)
| Field | Value |
|---|---|
| Auditor | W05-A06 |
| Date | 2026-11-21 |
| Lines / functions | 556 (chunk 1113-1668 of 2225; part1 = 1-556 → W05-A08, part2 = 557-1112 → W05-A07, part4 = 1669-2225 → W05-A10; boundaries cut through `parse_trak` (1045-1426 — head 1045-1112 is part2's, tail 1427+ is mine) and `parse_moof` (1621-1774 — head through tfhd/tfdt is mine, trun handling is part4's)) / 10 items |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open x.{mp4,mov,3gp,…}` → `crates/media-probe/src/signature/mp4.rs:39` `Mp4Signature` (ftyp brand) → `apps/cli/src/containers/mod.rs:67` `"mp4" => mp4::open(stream, opts)` → `apps/cli/src/containers/mp4.rs:100` `Mp4Demuxer::new` → `Demuxer` impl. EXECUTED in baseline: `mp4-smoke-open-session-420.txt` — tracks 2 ready → Buffering → Playing → Stopped, EXIT=0; 60 `mp4::tests::` ok (orchrun-test-workspace-nofailfast.log.txt:1764-1801), `tests/mp4_fixtures.rs` 3/3 ok (:2160-2162), `tests/corpus_mp4.rs` 2/2 differential vs ffmpeg truth ok (:1998-1999), CLI `containers::mp4::` incl. `corpus_iso_bmff_lanes_match_the_ffmpeg_truth` ok (:2813-2814)) |
| Findings | S0: 0 S1: 0 S2: 3 S3: 2 S4: 1 |
| Commands run | full Read of 1045-1674 (my range + parse_trak head + parse_moof head) + context reads 436-600 (codec_of/kind_of/unpack_language/esds), 869-1044 (track_record/parse_moov); `sed -n` on every quoted range; **byte-level proof for F-001: python3 box-walk of `audit/runs/baseline/samples/sample-420.mp4` → mdhd@76778 (video) and mdhd@77962 (audio) both ver=0, `lang_raw=0x55C4`**; `rg "voe"` (zero source hits — value is computed); `rg "ctts|stz2"` over demux tests (one ctts test, v0-only; zero stz2 tests); test_support.rs:155-266 read (ctts builder always `fullbox(b"ctts", 0, …)`); h264/sw.rs 50-60/360-375/520-535 read (4:2:0-only output gate, typed `h264.sw.unsupported`); baseline log greps above; feature-matrix.toml:142-154 |

## 1. One-paragraph verdict
This chunk is the heart of the MP4 demuxer that the owner's bug report is about — the trak sample-table build, cover/chapter handling, and the progressive packet walk — and it is real, reachable, and proven end-to-end by the corrected baseline (4:2:0 H.264+AAC plays to Stopped; the original "cannot open" was a 4:4:4 sample refused codec-side, INDEX.md CORRECTION v2). But it carries the root cause of the shipped `language: Some("voe")` garbage that the baseline INDEX flagged "demux wave to trace": `unpack_language` decodes the mdhd 5-bit fields with the wrong base (`'a'+v` instead of `0x60+v`), so every real language tag is shifted one letter ("und"→"voe", "eng"→"foh") — proven byte-exact against the sample's own mdhd. The ctts v1 lane reads an 8-byte i64 where the spec (and ffmpeg) use a 4-byte signed i32, corrupting every composition offset in negative-CTS files; stts/ctts iterate a raw file-controlled run count without the clamp their sibling boxes apply, stalling open for seconds on malformed tables; and stz2's 4-bit packing is misparsed as byte-per-sample. The truncation policy is internally inconsistent (over-ceiling sample → note+skip; sample 1 byte past EOF → fatal error mid-playback), though test-pinned as deliberate.

## 2. What actually works (verified)
- mdhd/hdlr/minf field geometry (1105-1131): v1 mdhd timescale@20/duration@24/language@28 and v0 @12/@16/@20 match ISO 14496-12 §12.4.2; hdlr handler@8-12 correct; `minf→stbl` located by box walk.
- elst semantics consumed correctly from part2's parse (1071-1096 read for context): empty edit (media_time −1) → `empty_edit_delay`, media_time > 0 → trim — both pinned by `elst_empty_edit_delays_and_media_time_trims` (baseline :1771 ok).
- stsd entry extraction (1148-1165): size-sanity `entry_size < 8 || 8 + entry_size > body.len()` → typed `bad_box`; entry payload sliced `16..8+entry_size`.
- Codec vocabulary + mislabel quirks (1166-1180): `mp4a`+OTI 0x6b → mp3, `mp4v`+OTI 0x6c → mjpeg (corpus-pinned), ipcm/fpcm via `pcmC` child — all four pinned by tests (`ipcm_fpcm_pcmc_mapping_table`, `ipcm_fpcm_with_pcmc_demux_as_pcm_codecs`, baseline :1777-1780 ok).
- Unknown-codec traks do NOT kill the file: `parse_moov` catches `parse_trak` errors with a `trak-unparseable` damage note (972-979, read cross-chunk) — the codec_of doc comment "the trak is skipped with a damage note" (436-437) is TRUE at the caller level; a tmcd/ProRes track degrades to a note, not an open failure.
- stss handling (1300-1313): loop clamped to MAX_SAMPLES, sort+dedup then binary search — pinned by `stss_unsorted_with_duplicates_matches_sorted_flags` (:1796 ok); absent stss ⇒ all sync per §8.6.2 — pinned by `absent_stss_means_all_sync` (:1764 ok).
- Chunk walk (1343-1423): stsc run resolution (last run whose `first ≤ chunk_1`, `stsc.last()` extends to infinity — the spec's open-ended final run), saturating dts/offset accumulation, `stsc-stsz-mismatch-short` damage note; PCM chunk collapse to ~1024-sample packets (1386-1417) with summed size/dur — the ffmpeg mov.c packetization, corpus-pinned (0051).
- Cover art (1433-1515): wkt 13/14 → mjpeg/png, unknown type and over-ceiling → note+skip; offset bounds checked against stream length before the pool read; still-image `TrackFlags` set — pinned by `cover_art_surfaces_as_a_still_image_video_track`, `png_cover_and_unknown_cover_types_map_or_note`, `cover_and_chapters_keep_packet_handles_aligned_with_tracks` (:1768, :1785, :1768 ok).
- `step_progressive` order-cursor machine (1569-1614): pre-seek sample skip, zero-size note, ceiling note, EOS with surfaced damage — pinned by `progressive_single_track_demuxes_samples_in_order`, `progressive_two_tracks_interleave_by_file_offset`, `truncated_sample_read_is_typed_after_start` (:1788-1789, :1799 ok).
- `parse_moof` head (1621-1668): tfhd flag-ordered field walk matches §8.8.7 (base-data-offset u64, sample-description-index skip, default dur/size/flags); default-base-is-moof anchor = moof_start; the carry-forward case damage-noted (1661-1666); tfdt v1 u64/v0 u32 correct — pinned by `fragmented_demuxes_moofs_with_tfdt_ladder`, `fragmented_two_tracks_per_moof` (:1772, :1774 ok).
- End-to-end demux of the owner's own problem file: mp4-smoke-open-session-420.txt — 2 tracks, duration 3.0 s, seekable, Playing → Stopped, EXIT=0.

## 3. Findings

### F-001 [S2]
Where: crates/demux/src/mp4/mod.rs:535-539 (root, part1's range) — called at 1116-1118 (this chunk)
Code:
```rust
fn unpack_language(packed: u16) -> Option<String> {
    let c = |shift: u32| -> char { (b'a' + ((packed >> shift) & 0x1F) as u8) as char };
    let tag: String = [c(10), c(5), c(0)].iter().collect();
    if tag == "und" { None } else { Some(tag) }
}
```
(call site, 1116-1118:)
```rust
                                if let Some(packed) = u16be(body, lang_at) {
                                    language = unpack_language(packed);
                                }
```
What is wrong: ISO 14496-12 §12.4.2 packs the mdhd language as three 5-bit values where each value = **letter − 0x60** ('a'=1, 'z'=26, 0=undefined). This decoder adds `'a'` (0x61) to each 5-bit value instead of 0x60 — every decoded letter is one past the intended one, and the zero value (undefined) decodes as 'a'.
Why it matters: This is the traced root of the shipped garbage the baseline INDEX lists as a standing defect: "TrackInfo `language: Some(\"voe\")` on tracks with no language metadata — garbage default (demux wave to trace)". Byte-level proof: python3 box-walk of `audit/runs/baseline/samples/sample-420.mp4` shows both mdhd boxes carry `lang_raw=0x55C4`; under ffmpeg's packing (movenc `mov_mdhd_language_code`, char−0x60) 0x55C4 = values 21,14,4 = **"und"**; this code decodes the same bytes to "voe" (0x61+21='v', 0x61+14='o', 0x61+4='e'), which `LanguageTag::parse` (media-types/src/ids.rs:509-530 — shape-only validation, any 2-8 ASCII letters pass) accepts and the engine ships as `language: Some("voe")` in MediaInfoReady (mp4-smoke-open-session-420.txt). Every MP4 with a real language is equally wrong ("eng"→"foh", "jpn"→"kqo"); the `tag == "und"` escape can never fire for a correctly-packed "und", and packed 0 (unspecified) surfaces as "aaa".
What correct looks like: `let c = |shift| (0x60u8 + ((packed >> shift) & 0x1F) as u8) as char`, with any zero 5-bit field ⇒ `None` (undefined), then the "und" ⇒ `None` check works as written.
Confidence: HIGH — Verification: read + byte-level hexdump of the baseline sample (command in header) + ffmpeg packing arithmetic + shipped output cited. Root function sits in part1 (W05-A08's range) — flagged there in §9.

### F-002 [S2]
Where: crates/demux/src/mp4/mod.rs:1317-1324
Code:
```rust
                    for i in 0..count {
                        let run = u32be(body, 8 + i * 8).unwrap_or(0) as u64;
                        let off = if version == 1 {
                            i64be(body, 12 + i * 8).unwrap_or(0)
                        } else {
                            // v0 composition offsets are signed i32.
                            i64::from(u32be(body, 12 + i * 8).unwrap_or(0) as i32)
                        };
```
What is wrong: A ctts entry is 8 bytes — `sample_count` u32 at 8+i*8 and `sample_offset` **i32** at 12+i*8 — in BOTH versions (ISO 14496-12 §8.6.1.3: v0 offset unsigned, v1 offset signed; the 2015 amendment changed the sign, never the width). The v1 arm reads `i64be` — 8 bytes starting at the offset field, swallowing the next entry's `sample_count` into the high 32 bits.
Why it matters: Every v1 ctts composition offset is garbage: e.g. offset −1000 with next-run count 50 reads as 0xFFFFFC18_00000032 → pts = dts + a −4.29-billion-tick offset. v1 ctts is what ffmpeg writes for `-movflags +negative_cts_offsets` and what DASH packagers emit for negative CTS — the standard B-frame H.264 shape. A/V timestamps become nonsense (seek, sync, and the mp4mux cts pass-through all poisoned). Zero coverage: the only ctts test (`ctts_shifts_pts_off_dts`, baseline :1770) runs through `test_support.rs`'s builder, which unconditionally writes `fullbox(b"ctts", 0, …)` (test_support.rs:246) — the v1 lane has never executed in any committed test, fixture, or corpus lane.
What correct looks like: `i64::from(u32be(body, 12 + i * 8).unwrap_or(0) as i32)` for v1 (signed i32, same width as v0 — only the signedness differs), plus a v1 fixture.
Confidence: HIGH — Verification: read + ISO §8.6.1.3 entry geometry; `rg "ctts"` over the demux test tree → one test, v0-only.

### F-003 [S2]
Where: crates/demux/src/mp4/mod.rs:1253-1266 (stts) and 1314-1332 (ctts) — contrast the clamped siblings at 1268-1277 (stco)
Code:
```rust
                b"stts" => {
                    let count = u32be(body, 4).unwrap_or(0) as usize;
                    for i in 0..count {
                        let run = u32be(body, 8 + i * 8).unwrap_or(0) as u64;
                        let delta = u64::from(u32be(body, 12 + i * 8).unwrap_or(0));
```
What is wrong: stco/co64/stsc/stsz all clamp their parse loop (`let take = count.min(MAX_SAMPLES); for i in 0..take`). stts and ctts loop over the **raw file-controlled count** (`for i in 0..count`); only the per-run expansion is clamped. A malformed stts/ctts declaring `entry_count = 0xFFFFFFFF` on a short body makes every `u32be` return `None→0` (cheap) but the loop still spins ~4.29×10⁹ times.
Why it matters: This runs inside `parse_trak` at **open time**: a corrupt or hostile MP4 stalls the open for seconds (release) to minutes (debug/test profiles) — a bounded but user-visible denial of service, inconsistent with the module's own hardening pattern 10 lines below. The existing ceiling tests (`stco/co64/stsc_count_over_ceiling_is_bounded_and_damage_noted`, baseline :1794-1795) pin the clamped boxes; no stts/ctts ceiling test exists, and `fuzz_lite_mutations_never_panic` (:1776) would hang, not fail, if a mutation hit a count field.
What correct looks like: `for i in 0..count.min(MAX_SAMPLES)` (the deltas/cts expansion already self-limits), and a `stts_count_over_ceiling` damage note like the siblings.
Confidence: HIGH — Verification: read + arithmetic; sibling clamp at 1270 quoted by contrast.

### F-004 [S3]
Where: crates/demux/src/mp4/mod.rs:1240-1248
Code:
```rust
                    let field = u32be(body, 4).unwrap_or(0) & 0xFF;
                    for i in 0..count.min(MAX_SAMPLES) {
                        let v = if field == 16 {
                            u64::from(u16be(body, 12 + i * 2).unwrap_or(0))
                        } else {
                            u64::from(body.get(12 + i).copied().unwrap_or(0))
                        };
```
What is wrong: ISO 14496-12 §8.5.2.2 (CompactSampleSizeBox) allows `field_size` 4, 8, or 16. The 16-bit lane is handled; everything else is read as **one byte per sample** — correct only for `field_size == 8`. For `field_size == 4` two samples pack per byte, so sample i's size lives in a nibble of byte `12 + i/2`; reading byte `12+i` as sample i's size yields a wrong size for every sample (and reads twice the table's byte extent). Non-4/8/16 field sizes (malformed) are silently treated as 8-bit with no damage note.
Why it matters: A stz2-bearing file (rare but legal — some MOV muxers emit it) with 4-bit fields gets every sample size wrong → wrong offsets → garbage packets or spurious truncation errors from `step_progressive`. No test or fixture covers stz2 at all (`rg "stz2"` over demux tests: zero hits).
What correct looks like: A `field == 4` arm reading `(byte >> 4)` / `(byte & 0xF)` for even/odd i, and a damage note for field sizes outside {4,8,16}.
Confidence: HIGH — Verification: read + §8.5.2.2 packing rule.

### F-005 [S3]
Where: crates/demux/src/mp4/mod.rs:1580-1594
Code:
```rust
            if sample.size > MAX_SAMPLE_BYTES {
                self.note(sample.size, "sample-over-ceiling");
                continue;
            }
            if sample.offset >= len || sample.offset + sample.size > len {
                // Declared sample past the stream: deliver nothing
                // fake — typed truncation (the WAV precedent: never a
                // silent EOS with lost tail).
                self.surface_damage();
                return Err(self.fail(error::truncated(sample.offset + sample.size, len)));
```
What is wrong: Two malformed-sample conditions, same function, opposite policies: an over-ceiling size is damage-noted and skipped, but a sample whose offset/extent passes the stream end is a **fatal typed error** that kills the whole playback mid-stream (the pipeline maps any demux Err to `report.fatal`, crates/playback-engine/src/pipeline.rs:2182-2185).
Why it matters: A truncated download or a file whose mdat is shorter than the tables promise (both common in the wild) plays fine until the first out-of-range sample, then aborts to an Error state instead of ending with a damage note — VLC-class players deliver the readable prefix and finish. The comment shows this is a deliberate WAV-precedent choice, and `truncated_sample_read_is_typed_after_start` (baseline :1799) pins it — but the zero-size arm directly above shows the damage-note-and-continue policy was available for the same class of defect; the split is a policy inconsistency, not a forced one.
What correct looks like: Note the sample as `sample-past-stream` damage, skip it, and let the walk continue to EOS (the ps demuxer's positional-skip posture); or at minimum make the policy uniform and document why one malformed-sample class is fatal.
Confidence: HIGH — Verification: read; behavior pinned by baseline test :1799.

### F-006 [S4]
Where: crates/demux/src/mp4/mod.rs:1155-1159, 1368-1371, 1343-1344, 1544-1547
Code:
```rust
        let entry_size = u32be(body, 8).unwrap_or(0) as usize;
        let entry_kind = [body[12], body[13], body[14], body[15]];
```
(1368-1371:)
```rust
                    let sync = match &sync_set {
                        Some(set) => set.binary_search(&sample_no).is_ok() || set.is_empty(),
```
What is wrong: Four silent-lenience residues: (a) the stsd `entry_count` field is never read — a multi-entry stsd silently uses only the first entry, and `entry_count == 0` with padded body parses garbage as an entry; (b) an stss box that is present but empty (`count == 0`) makes `set.is_empty()` true ⇒ **every** sample sync — per §8.6.2 a present stss lists the only sync samples, so the degenerate box should mean none (or a damage note), not all; (c) a progressive track missing stco/co64 or stts builds zero samples with **no damage note** (`!offsets.is_empty() && count > 0` guard falls through silently); (d) a chapter sample whose 16-bit length prefix exceeds the sample body yields an empty title via `unwrap_or_default()` — no note.
Why it matters: All four swallow malformed input without the observability the module otherwise guarantees ("damage, not a dead file" — the parse_moov comment at 974-975); (b) additionally mis-flags every sample as a keyframe, which `progressive_seek_lands_on_keyframe`-style consumers trust.
What correct looks like: Read `entry_count`, note `entry_count != 1`; treat empty-present stss as no-sync (or note); note `no-chunk-offsets` / `no-stts`; note `chapter-title-overrun`.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parse_trak` (mid: mdhd tail/hdlr/minf) | 1113-1137 | REAL | S2 | Field geometry correct; F-001 call site at 1116-1118 (root in part1) |
| `parse_trak` (guards + stsd) | 1139-1184 | REAL | S4 | typed guards correct; F-006(a) entry_count ignored |
| `parse_trak` (TrackState assembly) | 1185-1210 | REAL | — | tkhd-disabled and multi-elst damage notes honest; trex attach by track_id |
| `parse_trak` (progressive tables) | 1211-1336 | REAL | S2/S3/S4 | stsz/stco/co64/stsc clamped+noted; stss sorted/deduped; F-002 ctts v1 width, F-003 unclamped stts/ctts loops, F-004 stz2 4-bit, F-006(b) |
| `parse_trak` (sample expansion + PCM collapse) | 1337-1423 | REAL | S4 | stsc run resolution correct incl. open-ended last run; saturating arithmetic; mismatch-noted; F-006(c) silent empty on missing tables |
| `push_cover_track` | 1433-1480 | REAL | — | wkt gate, ceilings, still-image flags; synthetic `track_id: u32::MAX` could collide only with a crafted tkhd (negligible) |
| `queue_cover_packets` | 1485-1515 | REAL | — | bounds-checked pool read, unreadable→note; packets queued ahead of media (ffmpeg attached-pic order) |
| `read_chapter_titles` | 1519-1564 | REAL | S4 | 4096-byte cap, UTF-8 lossy, nanos sort; F-006(d) silent empty titles |
| `step_progressive` | 1569-1614 | REAL | S3 | order-cursor + pre-seek skip correct; F-005 fatal-vs-note policy split; zero-size/ceiling notes honest |
| `parse_moof` (head: traf/tfhd/tfdt) | 1621-1668 | REAL | — | tfhd flag-order walk spec-correct; default-base-is-moof anchor; carry-forward damage-noted; tfdt v0/v1 correct (trun is part4's) |

## 5. Test honesty (mp4 tests touching this range, all baseline-ok at :1764-1801)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `ctts_shifts_pts_off_dts` (:1770) | YES for v0, NO for F-002 | Builder writes ctts v0 unconditionally (test_support.rs:246) — the v1 lane has zero coverage anywhere |
| `stco/co64/stsc_count_over_ceiling_is_bounded_and_damage_noted` (:1794-1795) | YES | Pins the clamp+note for those three boxes — and by absence demonstrates stts/ctts were never given the same pin (F-003) |
| `stss_unsorted_with_duplicates_matches_sorted_flags` (:1796) | YES | Feeds an unsorted/dup stss and pins per-sample sync flags via the sort/dedup path |
| `absent_stss_means_all_sync` (:1764) | YES | Pins the `None` arm; the empty-present-stss arm (F-006b) is unpinned |
| `truncated_sample_read_is_typed_after_start` (:1799) | YES | Pins F-005's fatal truncation — the test enshrines the policy rather than challenging it |
| `progressive_seek_lands_on_keyframe` (:1787) | YES | Consumes the sync flags this chunk computes |
| `elst_empty_edit_delays_and_media_time_trims` (:1771) | YES | Pins empty-edit delay and media-time trim arithmetic consumed here |
| `fuzz_lite_mutations_never_panic` / fragment variant (:1775-1776) | PARTIAL | Panic-only contract on synthetic fixtures; would HANG (not fail) on an F-003 count mutation, and never generated one |
| `valid_mp4_fixtures_demux_fully` + `malformed_mp4_fixtures_*` (:2160-2162) | YES | Real ffmpeg fixtures, full-drain + typed-refusal-or-demux contract |
| `corpus_mp4_family_matches_the_ffmpeg_truth` (:1999) | YES | Differential packet/truth table vs ffprobe across the corpus family |
| `containers::mp4::corpus_iso_bmff_lanes_match_the_ffmpeg_truth` (:2814) | YES | End-to-end through the wired CLI container |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Demux ISO-BMFF family: track boxes, stco/stsc sample tables, edts/elst, 3gp quirks, fragmented MP4 — implemented (win/linux/macos)" | docs/feature-matrix.toml:142-151 (`container-mp4-mov-3gp`) | REAL | Baseline execution: mp4-smoke-open-session-420.txt (Playing→Stopped, EXIT=0), 60 unit tests, mp4_fixtures 3/3, corpus_mp4 2/2, CLI corpus lane — all ok |
| "2026-11-15: end-to-end VIDEO PLAYBACK proven on the h264+aac lane" | feature-matrix.toml:154 | REAL | mp4-smoke-open-session-420.txt + mp4-smoke-open-session-aac.txt (audio-only lane also plays) |
| "No MP4 demuxer code exists; the MP4 box walker … is TEST-SIDE" | feature-matrix.toml:153 (stale note inside the same row) | FALSE (stale) | Contradicted by the row's own evidence string (:150), the 2026-11-15 note (:154), and 2225 lines of shipped demux wired at containers/mod.rs:67 — internal contradiction, hygiene-level |
| TrackInfo language surfaces real mdhd metadata | implied by TrackInfo shape; INDEX.md:141 flags `Some("voe")` as garbage | FALSE | F-001: off-by-one decode, proven byte-exact on the baseline sample (0x55C4 = "und" → "voe") |
| "MP4 cannot be opened properly" (owner's core bug) | INDEX.md CORRECTION v2 | REAL (demux side exonerated) | Demux delivers 2 tracks + packets; the 4:4:4 refusal is codec-side by documented design (crates/codecs/src/video/h264/sw.rs:52-56, 367-371, 527-535: 8-bit 4:2:0 output vocabulary, typed `h264.sw.unsupported`); `lumen decode-bench`'s "no decoder in this build" is a hard-coded CLI lane (INDEX standing defects; mp4-smoke-bench-420.txt), not this file |

## 7. Dead / duplicated / unreachable code
- `unpack_language`'s `tag == "und"` check is effectively dead for conforming files (F-001: a correctly-packed "und" decodes "voe" and never matches).
- No dead code otherwise in my range; every table arm maps to a real box type.
- The `stz2` arm's 4-bit shape is reachable-in-spec but unreachable-in-suite (zero tests/fixtures).

## 8. Dependencies & flags
- No feature gates on this code path; compiled into the default build the baseline ran (build-release-workspace.log.txt exit 0).
- Reuses `crate::avi::riff::{PayloadSlots, SourceReader}` — same cross-format I/O layer as ps/ogg; consistent.
- `MAX_SAMPLES` = 1M bounds per-table vectors (sizes/deltas/offsets/cts ≈ 8-40 MB per track worst case — documented ceiling, mp4/mod.rs:122).

## 9. Open questions for the next agent
- **For W05-A08 (part1, owns 535-539):** F-001 is yours to file formally — my range holds only the call site; the byte-level proof and fix shape are in F-001 above.
- For W05-A10 (part4): does `parse_moof`'s trun arm apply ctts-style composition offsets from the `SAMPLE_IS_NON_SYNC`/first-sample flags, and does `seek_progressive`'s keyframe landing consume the `sync` field this chunk computes (F-006b's mis-flag would then corrupt seek landings)?
- For the media-types owner: `LanguageTag::parse` accepts any 2-8-letter alpha primary ("voe", "foh") — shape-only validation let the F-001 garbage ship; a registry check (or at least ISO-639-2/b/t list) would have caught it at the seam.
- REQUEST-ORCH-RUN (optional, settles F-002 empirically): `printf 'quit\n' | timeout 40 ./target/release/lumen open <v1-ctts h264 b-frame mp4>` — a file muxed with `ffmpeg -movflags +negative_cts_offsets`; expected per code: wildly negative pts. No such sample exists in baseline.
