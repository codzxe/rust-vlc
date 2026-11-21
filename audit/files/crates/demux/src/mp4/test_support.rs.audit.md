# AUDIT: crates/demux/src/mp4/test_support.rs (lines 1-570, whole file)
| Field | Value |
|---|---|
| Auditor | W05-A09 |
| Date | 2026-11-21 |
| Lines / functions | 570 / 24 items (2 spec structs, 17 builder fns, 3 pub entry builders + 3 more, `mem_stream`) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO by design (test-support; verified: zero imports outside `mp4/tests.rs` ×42 and `mp4/fuzz.rs` ×1 — mem_stream only; the integration suite `mp4_fixtures.rs` uses committed ffmpeg fixture FILES, no builder import). The shipped MP4 open path itself is exercised by baseline smoke: `mp4-smoke-open-session-420.txt` (H.264 4:2:0+AAC: tracks 2 ready → Playing → Stopped, EXIT=0) and `mp4-smoke-open-session.txt` (yuv444p: demux OK, `error [decode.unsupported]` — decoder-stage refusal, demux not the failure) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 4 |
| Commands run | full Read; byte-offset verification of `stsd_entry_with` against the repo's own ffmpeg fixture `tests/fixtures/generated/valid/mp4_h264_aac.mp4` (python struct dump: mp4a entry payload `[16:18]`=1 channelcount, `[18:20]`=16 samplesize, `[24:28]`=0xac440000 samplerate) and against the shipped reader `audio_geometry` (apps/cli/src/containers/mp4.rs:386-390, reads offsets 16/24); trun flag dump of `tests/fixtures/generated/valid/mp4_frag.mp4` (trun flags=0x305, first_sample_flags=0x2000000, no per-sample flags); `rg "test_support"` consumer hunt; baseline log greps (38 `mp4::tests` ok, test-workspace:1805-1842; corpus_mp4 + mp4_fixtures integration ok, :2039-2040, :2201-2203) |

## 1. One-paragraph verdict
The box-machinery is genuinely correct: two-pass stco offset patching with stable byte lengths, run-length stts, one-chunk-per-sample stsc/stsz, the fragmented moof/trun data-offset arithmetic (rebuilt moof identical in length, per-track mdat offsets right), the iTunes udta/meta/ilst/covr ladder, and the elst i64→i32 low-half slice are all right, and 38 unit tests plus three integration suites consume this surface green in the baseline. The two real defects are fidelity lies at the exact seams the module doc brags about: the audio sample entry writes channels/samplesize/rate at byte offsets that match neither ISO 14496-12 §12.4.3 nor the repo's own ffmpeg fixtures nor the shipped CLI geometry reader (which therefore reads 0 channels from every builder fixture), and the fragmented builder's non-sync sample flag `0x8000` is a private encoding shared with the demuxer's equally non-standard `SAMPLE_IS_NON_SYNC` constant — the unit fixtures pin the parser's wrong bit instead of the ISO bit-16/ffmpeg encodings real files carry (verified against the committed mp4_frag.mp4). Both defects are invisible to the green tests because parser and builder were written by the same hand — the textbook golden-generation lock-in.

## 2. What actually works (verified)
- `progressive_with_entry_children` (323-356): pass-1 zero-offset build and pass-2 real-offset rebuild produce identical byte lengths (only stco field values change, fixed-width u32), `mdat_payload_start = file.len() + 8` correctly anchors sample offsets after the mdat header; per-sample sequential offsets match the mdat payload order. — verified by read.
- `fragmented` (408-515): trun flags `0x000_0F01` = data-offset+duration+size+flags+cts, field order matches the demuxer's parser (mp4/mod.rs:1735-1757: dur, size, flags, cts); `data_offset = moof_box.len() + 8 + track_start[ti]` is the correct default-base-is-moof arithmetic (tfhd `0x020_000`), with the rebuild trick's length-stability argument holding (fixed-width fields only). — verified by read + parser cross-check.
- `elst` media_time: `media_time.to_be_bytes()[4..8]` takes the i64's low 4 bytes = the v0 i32 value. — verified by read.
- `stss` omission rule (231): all-sync tracks emit no stss (absence = all sync, asserted by `mp4::tests::absent_stss_means_all_sync ... ok`, test-workspace:1805); partial-sync emits 1-based indices. — verified by read + baseline.
- `udta_cover` (266-283): `udta/meta(FullBox prefix)/hdlr(mdir)/ilst/covr/data(wkt+locale+payload)` is the iTunes cover shape; consumed green by `cover_art_surfaces_as_a_still_image_video_track ... ok` (test-workspace:1810). — verified by read + baseline.
- `chapter_track` (519-541): u16-BE length-prefixed text samples = QuickTime text sample; green via `chap_tref_chapters_surface_with_titles ... ok` (test-workspace:1806).
- `mem_stream` (566-570): delegates to the single AVI seam (the repo-wide one-SourceStream pattern, same as wav/ogg/aiff twins). — verified by read.
- Executed: 38 `mp4::tests` ok (test-workspace:1805-1842), `corpus_mp4_family_matches_the_ffmpeg_truth` ok (:2040), `valid_mp4_fixtures_demux_fully` ok (:2203).

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/mp4/test_support.rs:143-150
Code:
```rust
fn stsd_entry_with(entry: &[u8; 4], children: &[u8]) -> Vec<u8> {
    let mut b = vec![0u8; 28];
    b[8..10].copy_from_slice(&2u16.to_be_bytes()); // channels
    b[10..12].copy_from_slice(&16u16.to_be_bytes()); // samplesize
    b[16..20].copy_from_slice(&44_100u32.to_be_bytes()); // 16.16 rate
    b.extend_from_slice(children);
    boxb(entry, &b)
}
```
What is wrong: all three geometry fields sit at the wrong payload offsets with the wrong encoding. ISO/IEC 14496-12 §12.4.3 AudioSampleEntry: `[8:10]`=version, `[10:12]`=revision, `[12:16]`=vendor, `[16:18]`=channelcount, `[18:20]`=samplesize, `[24:28]`=samplerate as 16.16 (44100 = 0xac440000). The builder writes 2 at the version field, 16 at the revision field, and raw 44_100 across the channelcount+samplesize pair (yielding channels=0, samplesize=44100), leaving the real samplerate field zero. Verified against the repo's own ffmpeg-authored fixture `tests/fixtures/generated/valid/mp4_h264_aac.mp4` (mp4a entry payload: `[16:18]`=1, `[18:20]`=16, `[24:28]`=0xac440000) and against the shipped reader: `audio_geometry` (apps/cli/src/containers/mp4.rs:386-390) reads `u16be(entry, 16)` channels and `u16be(entry, 24)` rate — from a builder-built entry it reads channels=0 → `(channels > 0 && rate > 0)` fails → geometry `None`. Writing version=2 is itself a landmine: QT v2 sound entries carry an extended layout, so any future version-honoring parser misreads the whole entry.
Why it matters: the module doc (8-10) claims "The builders emit the layouts ffmpeg's muxers produce … so the fixtures exercise the same paths the corpus does" — for audio sample entries this is false at every geometry byte. No builder fixture can ever exercise the shipped CLI's pcm/aac geometry lane (`containers/mp4.rs:200-203`); the "geometry rides the classic v0 fields … which ffmpeg fills" handoff (mp4/mod.rs:477-479) is fed zeros by every unit fixture. The tests stay green only because the demuxer itself never parses these fields — the wrong bytes ride verbatim into `DecoderConfig::extra_data`.
What correct looks like: `b[16..18] = channels`, `b[18..20] = samplesize`, `b[24..28] = 44_100 << 16` (and keep `[8:12]` zero); plus one test byte-comparing a builder `mp4a` entry's geometry fields against the committed ffmpeg fixture's — that test fails today and would have caught this.
Confidence: HIGH — Verification: read + python struct dump of the repo's own ffmpeg fixture + read of the shipped consumer's offsets.

### F-002 [S3]
Where: crates/demux/src/mp4/test_support.rs:462-463 (and the twin at 498-499)
Code:
```rust
                let sync = s.is_some_and(|s| s.sync);
                trun.extend_from_slice(&u32b(if sync { 0 } else { 0x8000 }));
```
What is wrong: the non-sync marker `0x8000` is bit 15 — per ISO/IEC 14496-12 §8.8.3.1 that bit is the top bit of `sample_degradation_priority`; `sample_is_non_sync_sample` is bit 16 (0x0001_0000). ffmpeg marks keyframes with `sample_depends_on=2` (0x02000000) in `first_sample_flags` and non-sync with the bit-16 encoding — verified by dumping the repo's own `tests/fixtures/generated/valid/mp4_frag.mp4`: every trun uses flags 0x305 (no per-sample flags lane at all) with first_sample_flags 0x2000000. The builder's 0x8000 matches exactly one consumer: the demuxer's own `SAMPLE_IS_NON_SYNC: u32 = 0x0000_8000` (mp4/mod.rs:147), whose doc comment (142-146) misattributes bit 15 to ISO and calls bit 16 "ffmpeg's … in practice" — the inverse of the standard's assignment.
Why it matters: parser and builder share a private encoding, so every fragmented unit fixture (e.g. `fragmented_demuxes_moofs_with_tfdt_ladder`, `fragmented_two_tracks_per_moof`) validates the non-standard bit-15 lane and never the ISO bit-16 / ffmpeg first_sample_flags/default_flags lanes real files use (those are covered only by the committed-fixture suites). If anyone "fixes" either side to the standard, the other side silently breaks sync detection; if a real-world file ever sets degradation_priority ≥ 0x8000 on a keyframe, the parser misclassifies it as non-sync. This is the golden-generation lock-in the audit checklist names: tests asserting shapes the code under test invented.
What correct looks like: builder emits 0x01010000 (depends_on=1 + non_sync bit 16) for non-sync and 0x02000000 for sync — the ffmpeg encodings; parser doc corrected to name bit 16 as the ISO bit (cross-file, mp4/mod.rs owners A06-A08/A10).
Confidence: HIGH — Verification: read of builder + parser constant; python dump of mp4_frag.mp4 trun/tfhd flags.

### F-003 [S4]
Where: crates/demux/src/mp4/test_support.rs:3-5
Code:
```rust
//! Public like [`crate::wav::test_support`] on purpose: the
//! integration suites + fuzz harness need deterministic shapes. This
//! is test-support surface, not player surface — nothing in the
//! demuxer itself uses it.
```
What is wrong: the consumer list overstates. Repo-wide consumer hunt: `mp4/tests.rs` (42 builder references), `mp4/fuzz.rs` (1 — `mem_stream` only). Zero integration suites import the builders: `tests/mp4_fixtures.rs` demuxes committed ffmpeg-authored files (`mp4_h264_aac.mp4`, `mp4_frag.mp4`, …), `tests/corpus_mp4.rs` pins ffprobe truth — neither touches this module's builders. Same false-consumer pattern W04-A10 convicted in `wav/test_support.rs:3-5` (its F-001).
Why it matters: the pub-ness of a compiled-into-the-lib surface (not cfg(test)) is an API-surface decision justified by a consumer list; the integration half of the justification is false.
What correct looks like: "rustdoc examples cannot see #[cfg(test)] items, and the fuzz harness needs a memory stream" — the true consumers.
Confidence: HIGH — Verification: repo-wide rg for every builder name + `test_support` import.

### F-004 [S4]
Where: crates/demux/src/mp4/test_support.rs:161-162 (and `mdhd` at 177-180)
Code:
```rust
    let mut t = Vec::new();
    t.extend_from_slice(&tkhd(track_id, spec.samples.iter().map(|s| s.dur).sum()));
```
What is wrong: `tkhd` duration is written as the sum of sample durations — media-ticks — while tkhd duration is defined in MOVIE timescale (the moov uses `mvhd(1000, 0)`, 294). A track with timescale 12800 and 12800 ticks of media (1 s) declares a 12.8 s tkhd duration. ffmpeg converts. Latent only: the demuxer reads exactly one tkhd field (flags, for the enabled bit — mp4/mod.rs:1064-1067, 1205-1206) and takes duration from mdhd, so no fixture outcome changes today.
Why it matters: the module doc's "the layouts ffmpeg's muxers produce" claim again; any future consumer of tkhd duration gets wrong values from every fixture.
What correct looks like: `sum * movie_timescale / media_timescale` (or document the field as intentionally dummy).
Confidence: HIGH — Verification: read of builder + parser's tkhd usage.

### F-005 [S4]
Where: crates/demux/src/mp4/test_support.rs:255-257
Code:
```rust
    let mut minf = Vec::new();
    minf.extend_from_slice(&fullbox(b"vmhd", 0, &[0u8; 8]));
    minf.extend_from_slice(&boxb(b"stbl", &stbl));
```
What is wrong: every track — audio included — gets a `vmhd` (video media header); ffmpeg writes `smhd` for sound tracks. The demuxer reads no media-header box at all (grep: `vmhd`/`smhd` absent from mp4/mod.rs), so both the wrong-kind path and the smhd path are unexercised by every unit fixture.
Why it matters: third instance of the "ffmpeg layouts" claim being false; the fixtures cannot catch a future smhd-requirement regression because they never emit one.
What correct looks like: pick `vmhd`/`smhd` (or `nmhd`) by `spec.handler`.
Confidence: HIGH — Verification: read + grep of parser.

### F-006 [S4]
Where: crates/demux/src/mp4/test_support.rs:189-198
Code:
```rust
        let mut stts = Vec::new();
        stts.extend_from_slice(&u32b(1)); // patched below per-run
        let mut runs: Vec<(u64, u64)> = Vec::new();
```
What is wrong: the `1` pushed at 190 ("patched below per-run") is dead — `stts.clear()` at 198 discards it before the real entry count is written at 199. The comment describes a patching step that does not exist (the code rebuilds instead).
Why it matters: dead write + a comment narrating a phantom operation; small, but it is exactly the kind of leftover that misleads the next reader into thinking the buffer is patched in place.
What correct looks like: drop line 190 and the comment.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-10 | REAL (2 claims FALSE — F-001/F-003) | S3/S4 | "layouts ffmpeg's muxers produce" false for audio entries, non-sync flags, tkhd duration, vmhd |
| `SampleSpec` / `TrackSpec` | 17-39 | REAL | — | plain data |
| `boxb` | 42-48 | REAL | — | `(8 + payload.len()) as u32` truncates >4 GiB−8 payloads — unreachable for test shapes |
| `fullbox` / `fullbox_f` | 51-65 | REAL | — | 24-bit flags split correct |
| `u32b` | 67-69 | REAL | — | — |
| `ftyp` | 71-80 | REAL | — | major + 0 + 5 compatible brands; matches ffmpeg's isom family |
| `mvhd` | 82-95 | REAL | — | v0 field order verified (rate/volume/matrix/next_track_id) |
| `tkhd` | 97-116 | REAL | S4 | flags=3 (enabled|in_movie) correct; duration units wrong (F-004) |
| `mdhd` | 118-127 | REAL | — | 0x55C4 "und" language correct |
| `hdlr` | 129-136 | REAL | — | — |
| `stsd_entry_with` | 143-150 | DEFECTIVE | S3 | F-001 — geometry offsets/encoding wrong vs ISO, ffmpeg fixture, and shipped reader |
| `trak` | 155-261 | REAL | S4 | F-004 (tkhd), F-005 (vmhd), F-006 (stts dead write); stsc/stsz/stco/stss/ctts builders verified correct; `*o as u32` stco — no co64 shapes buildable (undocumented limit) |
| `udta_cover` | 266-283 | REAL | — | iTunes ladder correct; executed green (cover-art test ok) |
| `moov_body` | 288-301 | REAL | — | mvhd timescale 1000 hardcoded |
| `moov_body_with_udta` | 304-308 | REAL | — | — |
| `progressive` | 313-315 | REAL | — | delegate |
| `progressive_with_entry_children` | 323-356 | REAL | — | two-pass patching verified (see §2) |
| `progressive_with_cover` | 361-402 | REAL | — | same two-pass pattern; ~40 lines duplicated with 323-356 (S4-level duplication, see §7) |
| `fragmented` | 408-515 | REAL | S3 | F-002 (non-standard non-sync bit); tfdt/trex/moof arithmetic verified; `let _ = t;` at 421 — enumerated param discarded (style) |
| `chapter_track` | 519-541 | REAL | — | QT text sample shape correct |
| `chap_ref` | 546-548 | REAL | — | — |
| `progressive_with_chapters` | 554-560 | REAL | — | hard-codes chapter as track id 2 — consistent with moov_body's i+1 ids |
| `mem_stream` | 566-570 | REAL | — | delegate to the single AVI seam |

## 5. Test honesty
No tests live in this file. Its output is consumed by 38 `mp4::tests` (all ok, test-workspace:1805-1842) and shaped the fixtures for `tests/mp4_fixtures.rs` (ok, :2201-2203).
| Consumer suite | Would it fail if the builder were broken? | Reason |
|---|---|---|
| mp4 unit tests (builder-fed) | YES for structure (box sizes, stco patching, trun arithmetic) — NO for geometry/flag fidelity | the demuxer never parses the fields F-001/F-002 get wrong; the tests assert demuxer outcomes on shapes the builder and parser co-designed — mutation test: byte-compare a builder mp4a entry against the committed ffmpeg fixture's WOULD fail today (channels 0 vs 1, rate 0 vs 0xac440000) |
| tests/mp4_fixtures.rs + tests/corpus_mp4.rs | N/A (do not use the builder) | committed ffmpeg files — the real-layout coverage the builder's doc claims for itself |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The builders emit the layouts ffmpeg's muxers produce (two-pass offset patching, one-chunk-per-sample stsc, mvex/trex defaults)" | test_support.rs:8-10 | **SHALLOW** | the three named mechanics are real and verified; the unnamed rest is not: audio entry geometry (F-001), non-sync flags (F-002), tkhd duration units (F-004), vmhd-vs-smhd (F-005) all diverge from ffmpeg — proven byte-wise against the repo's own ffmpeg fixtures |
| "the integration suites + fuzz harness need deterministic shapes" | test_support.rs:4-5 | SHALLOW | integration suites import nothing (F-003); fuzz uses mem_stream only |
| "nothing in the demuxer itself uses it" | test_support.rs:5-6 | REAL | verified: no production import |
| "the v0 `samplesize` field is written 16 for every width by ffmpeg and is not trusted by the parser" (comment citing this builder's precedent) | test_support.rs:320-321 + mp4/mod.rs:476-477 | REAL | ffmpeg writes 16 for all widths; the parser routes width through pcmC (mod.rs:481-510) |

## 7. Dead / duplicated / unreachable code
- `progressive_with_cover` (361-402) duplicates `progressive_with_entry_children` (323-356) except for the udta splice — ~40 lines copy-paste; a `udta: &[u8]` parameter on the one builder would collapse them.
- Line 190's dead `stts` push (F-006).
- No unreachable production surface (the module is test-support by design and grep-verified unconsumed by production).

## 8. Dependencies & flags
- `pub mod` compiled into the lib unconditionally (not cfg(test)) — the deliberate doctest/fuzz seam; `plugin_api::SourceStream` + `std::sync::Arc` only. No feature gates. No unsafe. All box arithmetic is allocation-proportional to caller-chosen sample counts (test tool; the demuxer's own ceilings are the parser's concern, not this file's).

## 9. Open questions for the next agent
- A04 (mp4/fuzz.rs): the fuzz harness feeds raw bytes through `mem_stream` only — confirm it never exercises the builders (consistent with my consumer hunt) and whether `fuzz_demux_mp4` has any #[cfg(test)] sweep at all (I found none in the baseline log — no `mp4::fuzz::tests` line exists, unlike ts/ogg/wav whose fuzz sweeps ran as tests).
- A06-A08/A10 (mp4/mod.rs parts): the `SAMPLE_IS_NON_SYNC = 0x0000_8000` doc comment (142-146) misstates the ISO bit (bit 16 is `sample_is_non_sync_sample`); my F-002 proves the pairing with this builder. File it on your side; also confirm the bit-16 lane (`& 0x1_0000`) is exercised by corpus fixtures (mp4_frag.mp4 uses first_sample_flags 0x2000000 — sync lane only; which committed file, if any, carries a bit-16 non-sync sample?).
- A04/A05 (mp4/tests.rs): the byte-compare-vs-ffmpeg-fixture test proposed in F-001 would pin the geometry offsets both the builder and any future parser must honor.
