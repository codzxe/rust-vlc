# AUDIT: crates/demux/src/mp4/tests.rs
| Field | Value |
|---|---|
| Auditor | W05-A05 |
| Date | 2026-11-23 |
| Lines / functions | 1111 / 50 (38 #[test] + 12 helpers) |
| File verdict | REAL (mutation-tight in-module suite; two S2 blind spots it actively maintains) |
| Reachable from shipped entry point? | Test-only by design: `#[cfg(test)] mod tests;` (mod.rs:103-104) — runs under cargo test (baseline cargo-test-workspace.txt:1977-2014, 38/38 ok). Code under test reachable: `Mp4Demuxer::new` <- apps/cli/src/containers/mp4.rs:100 <- lumen CLI open path (rg verified). |
| Findings | S0: 0 S1: 0 S2: 2 S3: 4 S4: 3 |
| Commands run | sed quote checks (all quotes below verified byte-verbatim); rg mod.rs/tests.rs/test_support.rs; rg apps for Mp4Demuxer; ffmpeg/ffprobe ground-truth generation under audit/runs/tmp/ (exit 0) + python3 box parser (exit 0). No cargo runs (baseline cited). |

## 1. One-paragraph verdict
This is a genuinely hostile in-module suite, far stronger than the Wave-4 integration twin (tests/mp4_fixtures.rs, whose valid-path check is `total > 0` per W04-A01 F-001): it pins exact DTS ladders, ctts-shifted PTS, keyframe flags per sample, payload bytes verbatim, seek landing ticks, typed error codes for 10+ damage shapes, and exact damage-note magnitudes (`Some(50)`, `Some(0x0800_0000 - MAX_SAMPLES)`) — break the demuxer's arithmetic anywhere on those lanes and a test fails. All 38 tests run (no #[ignore], no cfg gate) and pass. BUT it has two S2 blind spots it actively maintains: (1) the `elst` empty-edit test enshrines a movie-timescale/media-timescale units bug as its expected value — ffmpeg ground truth (generated `-itsoffset` file: `segment_duration=1500` @ movie_ts 1000 ⇒ ffprobe pts shift exactly 1.5 s; video mdhd ts 10240) proves the value is movie ticks, while the test's fixture diverges the timescales (mvhd 1000 vs track 9000) and asserts the raw copy (`pts == 1_000` track ticks = 0.111 s for a 1.0 s dwell); a units-correct demuxer (9000 ticks) would FAIL this test. (2) Zero version-1 box coverage: the only negative-composition-time test writes negative offsets into a v0 ctts — a shape ffmpeg never emits (verified: default B-frame file ⇒ ctts v0 all-positive + elst media_time; `-movflags +negative_cts_offsets` ⇒ ctts **v1** with run offsets `(1,0),(1,2048),(2,-1024)`) — while the real v1 ctts lane (mod.rs:1319-1320), tfdt v1, elst v1, tkhd v1, mdhd v1 have no fixture at all. Hostile-focus answers: fragmented MP4 YES (4 tests incl. seek + fuzz), ctts YES-but-wrong-shape, edts YES-but-wrong-units, negative compression times YES-in-v0-only, oversized stco YES (stco/co64/stsc ceilings, exact magnitudes — the strongest lane in the file).

## 2. What actually works (verified)
— verified by baseline cargo-test-workspace.txt:1977-2014: all 38 `mp4::tests::*` pass (`... ok` on every line; demux unittest binary `target/debug/deps/demux-f7fd0b08183ad874`).
— verified by ffmpeg/ffprobe (audit/runs/tmp/): `ffmpeg -itsoffset 1.5 -i base.mp4 -c copy delayed.mp4` → ffprobe first packet `packet,1.500000,1.300000`; box parse: `elst v=0 entries=2 | dur=1500 media_time=-1 | dur=500 media_time=2048`, `mvhd movie_ts=1000`, `mdhd media_ts=10240` — ground truth for F-001/F-003. `ffmpeg -movflags +negative_cts_offsets` → `ctts v=1 runs=4 [(1, 0), (1, 2048), (2, -1024)]` — ground truth for F-002.
— read + mutation reasoning: payload content IS pinned in-module (answering W04-A01's open question): tests.rs:147-148 (`0x10`/`0x13`), 324 (cover JPEG verbatim), 869-870 (ipcm `0x50,0x50`/`0x51,0x51`); fragmented lanes pin counts/DTS/syncs but not payload bytes.

## 3. Findings
F-001 [S2]
Where: crates/demux/src/mp4/tests.rs:186-197 (+ test_support.rs:168-175, 294; mod.rs:1090-1091, 1866-1869)
Code (verbatim, tests.rs:186-197):
```
    let mut track = video_track(2, 3000, &[true, true], &[0]);
    track.edit = Some((1_000, -1)); // empty edit: 1000-tick dwell
    let bytes = progressive(&[track], b"isom");
    let mut mp4 = open(&bytes);
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    let (_, packets) = drain(&mut mp4);
    let pts = |p: &media_types::Packet| match &p.pts {
        Stamp::Present(ts) => ts.ticks,
        Stamp::Missing => unreachable!("pts missing"),
    };
    assert_eq!(pts(&packets[0]), 1_000);
```
What is wrong: the fixture's moov hard-codes `m.extend_from_slice(&mvhd(1000, 0));` (test_support.rs:294) while the video track runs timescale 9000 — and the expected value `1_000` is asserted in TRACK ticks. ISO 14496-12 elst `segment_duration` is in the MOVIE timescale; ffmpeg ground truth (delayed.mp4: movie_ts 1000, seg_dur 1500 ⇒ ffprobe pts shift exactly 1.5 s, and the media track is ts 10240) confirms the unit. The demuxer copies the movie-tick value raw into a media-tick timeline (`empty_edit_delay = seg_dur;` mod.rs:1091, then `pts_ticks = dts_ticks + s.cts + empty_edit_delay` mod.rs:1866-1869) — it reads the mvhd timescale (mod.rs:918-923) but never converts. The test deliberately diverges the two timescales and then pins the un-converted value: 1000/9000 s ≈ 111 ms is asserted where the movie timeline asked for 1.0 s (9000 track ticks). Why it matters: this is a golden generated by the code under test — the one test positioned to catch the units bug certifies it; on real files (ffmpeg `-itsoffset`, or any encoder-delayed file where mvhd_ts ≠ track_ts) the empty-edit delay is wrong by the timescale ratio (10.24× on my ground-truth file) and no test in the repo fails (the integration `mp4_edts` fixture test only asserts `pts != 0`, W04-A01 F-004). What correct looks like: fixture keeps the divergent timescales and the assertion becomes `pts(packets[0]) == 9_000` after converting `seg_dur` movie-ticks → media ticks. Confidence: HIGH — Verification: read + ffmpeg/ffprobe ground truth (audit/runs/tmp/delayed.mp4); repo-binary differential NOT run (no prebuilt per-packet probe; cargo-build-budget) — the demuxer's output on delayed.mp4 is UNVERIFIED, its units arithmetic is verified from source.
F-002 [S2]
Where: crates/demux/src/mp4/tests.rs:169-182 (+ test_support.rs:239-247, 466; mod.rs:1314-1332, 1668-1748)
Code (verbatim, test_support.rs:241-246):
```
            ctts.extend_from_slice(&u32b(spec.samples.len() as u32));
            for s in &spec.samples {
                ctts.extend_from_slice(&u32b(1));
                ctts.extend_from_slice(&u32b(s.cts as u32));
            }
            stbl.extend_from_slice(&fullbox(b"ctts", 0, &ctts));
```
What is wrong: every ctts/trun/tfdt/elst fixture is hard-wired version 0 (`fullbox(b"ctts", 0, ...)`, `fullbox_f(b"trun", 0, ...)`, `fullbox(b"tfdt", 0, ...)`, `fullbox(b"elst", 0, ...)`), so the demuxer's v1 branches — ctts v1 `i64be` (mod.rs:1319-1320), tfdt v1 (mod.rs:1671), elst v1 (mod.rs:1081-1082), tkhd/mdhd v1 — execute in ZERO tests. The only negative-composition-time coverage wraps `-3000` into a v0 unsigned field (`s.cts as u32`), a shape no conforming muxer emits: my ffmpeg ground truth shows default B-frame muxing writes ctts v0 with all-positive offsets (base.mp4: `ctts v=0 runs=4 [(1, 2048), (1, 4096), (2, 1024)]`) and uses `-movflags +negative_cts_offsets` to write `ctts v=1` with negative run offsets (negctts.mp4). Why it matters: the hostile-focus question "negative compression times?" is answered YES only for a synthetic shape; the real-world signed form (ctts v1, trun v1) is untested anywhere in the repo — if the v1 lane regressed (wrong offset, wrong sign), every test still passes. What correct looks like: a `ctts_v1` fixture lane (negctts.mp4's exact table) asserting the same PTS ladder the v0 lane asserts, plus a trun-v1 lane in `fragmented`. Confidence: HIGH — Verification: read + ffmpeg-generated ground truth (audit/runs/tmp/{base,negctts}.mp4).
F-003 [S3]
Where: crates/demux/src/mp4/tests.rs:184-211 (whole elst test) + mod.rs:1077-1079, 1207-1210
Code (verbatim, mod.rs:1207-1210):
```
        }
        if multi_edit {
            track.damage.push((0, "elst-multi-entry"));
        }
```
What is wrong: multi-entry elst is the NORMAL ffmpeg shape — my `-itsoffset` ground-truth file carries `elst entries=2` (empty edit + media edit), as does any encoder-delayed AAC/B-frame mux — and the demuxer deliberately applies only the first entry and damage-notes the rest, yet NO test in the crate asserts the `elst-multi-entry` note (rg over crates/demux: produced at mod.rs:1209, asserted nowhere) or the resulting timeline. The empty-edit lane of this test uses a single-entry elst; the multi-entry behavior (apply entry 1, drop entry 2's media_time trim) is unpinned. Confidence: HIGH — Verification: read + rg + ffmpeg ground truth.
F-004 [S3]
Where: crates/demux/src/mp4/test_support.rs:205-210 (fixture writer; consumed by every progressive test)
Code (verbatim):
```
        let mut stsc = Vec::new();
        stsc.extend_from_slice(&u32b(1));
        stsc.extend_from_slice(&u32b(1));
        stsc.extend_from_slice(&u32b(1));
        stsc.extend_from_slice(&u32b(1));
```
What is wrong: every fixture is one-chunk-per-sample (`samples_per_chunk=1`, one stco offset per sample — the ffmpeg-movenc layout), so the demuxer's multi-sample chunk expansion (mod.rs:1351-1364) and the PCM chunk-collapse path it documents ("corpus 0051: 93 chunks × 1024 frames = 93 packets", mod.rs:1337-1347) have zero unit coverage. QuickTime-authored files and most non-ffmpeg muxers pack samples per chunk; a regression in the per-chunk walk (off-by-one in `si`, wrong `per` lookup) fails no test. What correct looks like: a `chunked_track(n_samples, per_chunk)` builder lane asserting packet count/bytes for per>1, plus a PCM lane with per=1024. Confidence: HIGH — Verification: read.
F-005 [S3]
Where: crates/demux/src/mp4/mod.rs:1300-1312 vs tests.rs:966-1035
Code (verbatim, mod.rs:1301-1304):
```
                    let count = u32be(body, 4).unwrap_or(0) as usize;
                    let mut set = Vec::with_capacity(count.min(MAX_SAMPLES));
                    for i in 0..count.min(MAX_SAMPLES) {
                        set.push(u32be(body, 8 + i * 4).unwrap_or(0));
                    }
```
What is wrong: the ceiling trio (stco/co64/stsc) is tested with exact damage-note magnitudes, but the stss twin is neither tested NOR noted — unlike its siblings (`self.note((count - take), "stco-count-over-ceiling")` mod.rs:1275), the stss take silently truncates with no `note()` call, so an oversized stss table in the wild loses keyframes with zero damage visibility and zero test coverage. Cross-ref for the mod.rs auditor: this is an implementation gap the suite's own pattern says should have a fourth test. Confidence: HIGH — Verification: read.
F-006 [S3]
Where: crates/demux/src/mp4/test_support.rs:466 + 457-465 (every trun carries full fields)
Code (verbatim, test_support.rs:466):
```
            traf.extend_from_slice(&fullbox_f(b"trun", 0, 0x000_0F01, &trun));
```
What is wrong: trun flags `0x0F01` = data-offset + duration + size + flags + cts — every per-sample field explicit in every fixture, so the trex/default fallbacks (mod.rs:1692-1706: default_duration/size/flags from tfhd or trex) never execute in a test; the builder's filler lane (`map_or(1024/0xAA...)`, test_support.rs:437/460) that would exercise defaults only triggers when frags×per_frag exceeds the sample list, which no test does. A regression in trex defaulting fails nothing. Confidence: HIGH — Verification: read.
F-007 [S4]
Where: crates/demux/src/mp4/tests.rs:57-70 (drain), consumed at 294-299 and 213-221
Code (verbatim, tests.rs:60-64):
```
    while let Ok(event) = demuxer.next_event() {
        events += 1;
        match event {
            DemuxEvent::Packet(p) => packets.push(p),
            DemuxEvent::EndOfStream => break,
```
What is wrong: `drain` exits silently on the first `Err` — the walk's failure mode is invisible unless the caller asserts an exact packet count. Most callers do (mitigating), but `chap_tref_chapters_surface_with_titles` (tests.rs:295 `assert!(!packets.is_empty())`) and `absent_stss_means_all_sync` (tests.rs:220 `assert!(packets.iter().all(|p| p.keyframe))`, no count) pass if the demuxer errors out after the first packet. Confidence: HIGH — Verification: read.
F-008 [S4]
Where: crates/demux/src/mp4/tests.rs:1077-1111
Code (verbatim, tests.rs:1082-1090):
```
    // Truncation prefixes.
    for cut in [0usize, 7, 8, 32, 64, 128, bytes.len() / 2] {
        super::fuzz::fuzz_demux_mp4(&bytes[..cut.min(bytes.len())]);
    }
    // Single-bit flips at stride.
    let mut mutated = bytes.to_vec();
    for i in (0..mutated.len()).step_by(37) {
```
What is wrong: the two fuzz-lite tests assert only absence of panic — a gutted `fuzz_demux_mp4` body passes them (same defect class as W04-A01 F-003 for the integration twin). Also the fragmented lane never pins payload bytes (progressive/cover/ipcm lanes do), so a fragmented payload corruption that survives without panic goes unmeasured. Confidence: HIGH — Verification: read.
F-009 [S4]
Where: crates/demux/src/mp4/tests.rs:588-594, 645-649
Code (verbatim, tests.rs:589-594):
```
    assert!(
        code == "demux.mp4.box-overruns-parent"
            || code == "demux.mp4.truncated"
            || code == "demux.mp4.no-tracks",
        "got {code}"
    );
```
What is wrong: two damage tests accept disjunctions (3 codes here; 2 at 645-649) — the `is_typed` naming promises a pin, the body accepts whichever of three typed outcomes arrives. The exact-shape second lane (tests.rs:615-618) compensates for child-beyond-parent; zero-sample-tables has no exact lane. Confidence: HIGH — Verification: read.

## 4. Per-function table (all 50)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| video_track | 15-31 | REAL | — | per-sample cts/sync/dur/bytes spec |
| audio_track | 33-49 | REAL | — | mp4a/44100 lane |
| open | 51-55 | REAL | — | mem_stream + new, unreachable! on error |
| drain | 57-70 | REAL | S4 | swallows Err (F-007) |
| ErrOf trait + impl | 72-83 | REAL | — | Ok ⇒ unreachable, Err ⇒ typed |
| code_of | 85-87 | REAL | — | err.context.code |
| probe_accepts_known_brands_and_qt_heads | 91-114 | REAL | — | brands, compat-brand, moov head, 2 negatives — mutation-tight |
| progressive_single_track_demuxes_samples_in_order | 118-149 | REAL | — | DTS ladder, stss flags, payload bytes — tight |
| progressive_two_tracks_interleave_by_file_offset | 151-166 | REAL | — | file-order interleave pinned at packets[0]/[3] |
| ctts_shifts_pts_off_dts | 168-182 | REAL | S2 | v0-signed synthetic shape only (F-002) |
| elst_empty_edit_delays_and_media_time_trims | 184-211 | REAL/FALSE-expectation | S2 | pins wrong units (F-001); media_time trim lane correct |
| absent_stss_means_all_sync | 213-221 | REAL | S4 | no count assert (F-007) |
| fragmented_demuxes_moofs_with_tfdt_ladder | 225-254 | REAL | — | tfdt ladder + sync pattern pinned; no payload bytes |
| fragmented_two_tracks_per_moof | 256-271 | REAL | — | per-moof track order pinned |
| chap_tref_chapters_surface_with_titles | 275-300 | REAL | S4 | titles + ns starts pinned; !is_empty weak (F-007) |
| cover_art_surfaces_as_a_still_image_video_track | 304-332 | REAL | — | JPEG bytes verbatim, still_image flag |
| png_cover_and_unknown_cover_types_map_or_note | 334-374 | REAL | — | png lane + unknown-type Recovered reason |
| cover_and_chapters_keep_packet_handles_aligned_with_tracks | 376-401 | REAL | — | hidden-trak handle alignment |
| span_at | 405-413 | REAL | — | ns seek target helper |
| progressive_seek_lands_on_keyframe | 415-445 | REAL | — | two landings + drain-resume + out-of-range code |
| fragmented_seek_scans_moofs | 447-470 | REAL | — | moof-scan landing pinned |
| not_mp4_is_typed_refusal | 474-479 | REAL | — | RIFF ⇒ demux.mp4.not-mp4 |
| missing_moov_is_typed | 481-491 | REAL | — | no-moov code |
| ftyp_bytes | 493-502 | REAL | — | 32-byte isom ftyp |
| truncated_box_header_is_typed_truncation | 504-513 | REAL | — | half-cut ⇒ demux.mp4.truncated |
| size_zero_convention_extends_to_stream_end | 515-534 | REAL | — | mdat size=0 lane |
| size_one_reads_64bit_largesize | 536-564 | REAL | — | 64-bit largesize rebuild lane |
| child_beyond_parent_is_typed | 566-619 | REAL | S4 | 3-code disjunction (F-009); exact lane 2 |
| zero_sample_tables_in_progressive_is_typed | 621-650 | REAL | S4 | 2-code disjunction (F-009) |
| unknown_boxes_between_ftyp_and_moov_are_tolerated | 652-670 | REAL | — | junk top-level box tolerated |
| truncated_sample_read_is_typed_after_start | 672-688 | REAL | — | mid-walk truncation + sticky error |
| not_started_calls_are_typed | 690-699 | REAL | — | not-started for next_event + seek |
| mov_quirks_wide_and_qt_brand_open | 701-717 | REAL | — | wide-box prefix + major_brand() |
| sowt_entry_maps_to_pcm_s16le | 719-727 | REAL | — | sowt → pcm_s16le |
| threegp_brand_and_samr_entry_open | 729-739 | REAL | — | 3gp5 + samr → amr_nb |
| seek_failure_leaves_state_usable_d4 | 741-763 | REAL | — | failed seek keeps the walk resumable |
| pcmc_box | 770-778 | REAL | — | 23003-5 pcmC builder |
| pcm_track | 782-798 | REAL | — | ipcm/fpcm track builder |
| ipcm_fpcm_pcmc_mapping_table | 800-854 | REAL | — | 10 positive + 4 negative + 2 default lanes on production fn |
| ipcm_fpcm_with_pcmc_demux_as_pcm_codecs | 856-880 | REAL | — | end-to-end pcmC demux + payload bytes |
| ipcm_without_pcmc_defaults_to_entry_name_convention | 882-894 | REAL | — | i=be/f=le default |
| ipcm_pcmc_unmapped_width_notes_unknown_entry | 896-930 | REAL | — | damage note magnitude Some(50) pinned |
| find_box | 937-941 | REAL | — | first-match box locator |
| drain_noting | 945-964 | REAL | — | packet + damage-note capture |
| stco_count_over_ceiling_is_bounded_and_damage_noted | 966-985 | REAL | — | AUD-01 regression pin, exact magnitude |
| co64_count_over_ceiling_is_bounded_and_damage_noted | 987-1016 | REAL | — | co64 twin, stsz trimmed to 1 |
| stsc_count_over_ceiling_is_bounded_and_damage_noted | 1018-1035 | REAL | — | healthy sibling still demuxes |
| stss_unsorted_with_duplicates_matches_sorted_flags | 1037-1073 | REAL | — | unsorted/dup multiset ≡ canonical set — strong |
| fuzz_lite_mutations_never_panic | 1077-1096 | REAL | S4 | panic-only contract (F-008) |
| fuzz_lite_fragment_mutations_never_panic | 1098-1111 | REAL | S4 | panic-only contract (F-008) |

## 5. Test honesty (would it fail if the demuxer broke?)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| probe_accepts_known_brands | YES | brand list + negatives pinned |
| progressive_single_track | YES | DTS ladder, keyframes, payload bytes |
| progressive_two_tracks | YES | interleave order at packets[0]/[3] |
| ctts_shifts_pts | YES (for v0-signed) | pts 0/0/9000 pinned — but only for the synthetic v0 shape; v1 regression invisible (F-002) |
| elst_empty_edit | YES — but it certifies the BUG | pins un-converted movie ticks; units-correct code FAILS it (F-001) |
| absent_stss | PARTIAL | no count assert (F-007) |
| fragmented_demuxes_moofs | YES | 8 packets, dts 0/1024/2048/4096, syncs |
| fragmented_two_tracks | YES | 8 packets, per-moof handle order |
| chap_tref | PARTIAL | titles/ns pinned; !is_empty weak (F-007) |
| cover_art_surfaces | YES | bytes verbatim + handles |
| png_cover_and_unknown | YES | png lane + Recovered reason + packet count |
| cover_and_chapters_handles | YES | counts + handle ceiling + bytes |
| progressive_seek_lands_on_keyframe | YES | landing ticks 0/4000, resume count, oor code |
| fragmented_seek_scans_moofs | YES | landing 4000 + 4 packets |
| not_mp4 / missing_moov / truncated_header | YES | exact typed codes |
| size_zero / size_one | YES | 2 packets each after box-surgery |
| child_beyond_parent | PARTIAL | 3-code disjunction; lane 2 exact (F-009) |
| zero_sample_tables | PARTIAL | 2-code disjunction (F-009) |
| unknown_boxes_tolerated | YES | 1 packet |
| truncated_sample_read | YES | packet then truncated ×2 sticky |
| not_started_calls | YES | codes pinned |
| mov_quirks / sowt / threegp | YES | brand + codec strings + counts |
| seek_failure_leaves_state_usable | YES | 2 packets survive a failed seek |
| ipcm_fpcm_pcmc_mapping_table | YES | 16 lanes against production fn |
| ipcm end-to-end ×3 | YES | codec strings + payload bytes + note magnitude |
| stco/co64/stsc ceilings | YES | exact `0x0800_0000 - MAX_SAMPLES` magnitudes + surviving packets |
| stss_unsorted | YES | unsorted ≡ sorted flags equality |
| fuzz_lite ×2 | NO (panic-only) | gutted harness passes (F-008) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every error path pinned to its typed code (no panics, no unwrap-driven assertions on demuxer output)" | tests.rs:1-4 | MOSTLY REAL | 10+ exact codes pinned; two disjunctions loosen it (F-009); open() does use `unwrap_or_else(|e| unreachable!...)` — panics on construction failure, acceptable in tests but the header overstates |
| "DTS ladder from stts deltas" | tests.rs:130 | REAL | 0/3000/6000/9000 pinned (136-140) |
| "empty edit: 1000-tick dwell" | tests.rs:187 | FALSE-UNITS | 1000 MOVIE ticks at mvhd_ts 1000 = 1.0 s; asserted as 1000 TRACK ticks @ 9000 = 0.111 s (F-001, ffmpeg ground truth) |
| "v0 composition offsets are signed i32" | mod.rs:1322 | REAL-AS-CODEC-CHOICE / UNVERIFIED-vs-ffmpeg | ffmpeg wrote v0-positive for default B-frames and v1 for negative (audit/runs/tmp); whether any real muxer emits v0-negative is UNVERIFIED |
| "AUD-01 … take is now capped at MAX_SAMPLES with the surplus damage-noted" | tests.rs:968-970 | REAL | exact magnitude assertions 983/1014/1033; stss twin missing (F-005) |

## 7. Dead / duplicated / unreachable code
None dead. Duplication: the mdat/stbl/trak find-and-patch loops (519-525, 541-547, 573-577, 600-605) repeat the same scan four times before the shared `find_box` helper appears at 937 — the earlier tests predate the helper (S4 hygiene, not worth a separate finding).

## 8. Dependencies & flags
media_types (time types), plugin_api (Demuxer/DemuxEvent/SeekSupport), super::test_support (fixture builders), super::{Mp4Demuxer, probe}, super::fuzz (fuzz_demux_mp4), super::{pcm_codec_of_ipcm, MAX_SAMPLES} — production symbols, no test-only seams in the demuxer path. No feature gates (rg: none). `#[cfg(test)] mod tests` (mod.rs:103-104) — never in the shipped binary.

## 9. Open questions for the next agent
- mod.rs auditor: (a) empty-edit units — mod.rs:1091 stores movie ticks, mod.rs:1866-1869 adds them to media ticks; mvhd timescale is read (918-923) but unused on this path (my F-001, needs an S2 on mod.rs); (b) stss ceiling has no damage note (F-005); (c) multi-entry elst applies only entry 1 (1080-1095) — normal ffmpeg files carry 2 entries (delayed.mp4 ground truth).
- test_support.rs auditor: fixtures hard-code mvhd_ts=1000 (294) — keep, it is exactly what exposes F-001; add v1-box and per-chunk>1 builders per F-002/F-004/F-006.
- corpus_mp4.rs auditor: does the corpus differential pin per-packet PTS on any elst/ctts-v1 file? If not, F-001/F-002 are total in-repo.
