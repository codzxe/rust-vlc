# AUDIT: crates/demux/src/mp4/tests.rs#part1 (lines 1–564)
| Field | Value |
|---|---|
| Auditor | W05-A04 |
| Date | 2026-11-21 |
| Lines / functions | 564 (chunk 1–564 of 1112) / 8 helpers + 19 tests |
| File verdict | REAL (coverage gap ships a live S2 in the language lane) |
| Reachable from shipped entry point? | YES for the code under test: `lumen open x.mp4` → containers/mod.rs:67 `"mp4" => mp4::open(stream, opts)` → `Mp4Demuxer`; executed end-to-end in baseline (mp4-smoke-open-session-420.txt: tracks → Playing → Stopped, EXIT=0) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 3 |
| Commands run | `rg -n "mp4::tests::" audit/runs/baseline/test-workspace.log.txt` → 38 ok incl. all 19 part1 tests (log lines 1805–1833). `rg -n "language\|"und"\|voe" crates/demux/src/mp4/tests.rs crates/demux/src/mp4/test_support.rs` → only test_support.rs:124 (writes 0x55C4, comment `// "und"`); ZERO language assertions in tests.rs. `sed -n '533,540p' crates/demux/src/mp4/mod.rs` (off-by-one verified). Python box-walk of committed baseline samples (below) → mdhd bytes on disk are `55 c4`. `sed -n` on every quoted range. |

Chunk boundary: part1 = lines 1–564 (through the end of `size_one_reads_64bit_largesize`); part2 (W05-A05) = 565–1112 (starts `child_beyond_parent_is_typed`). Boundary chosen at the test boundary nearest the 555/556 manifest split.

## 1. One-paragraph verdict
These are real, executed, mostly strong unit tests: 19 tests in this half (38 in the whole file) ran green in the baseline (`test-workspace.log.txt:1805-1842`), they pin the DTS/PTS ladder (stts/ctts/elst/tfdt), keyframe flags, seek landings, chapter/cover surfacing, and five typed error codes, and the demuxer they exercise is proven on real files by the baseline smoke session (`mp4-smoke-open-session-420.txt`). But the suite has one blind spot that is shipping a live bug: **not one test asserts on `Track.language`**, and the mdhd language decoder has an off-by-one that turns every standard `und` packing (0x55C4) into the garbage tag `"voe"` — visible in the shipped binary's own baseline output on both MP4 samples. The helper `drain` also swallows typed errors and never asserts EndOfStream, and the "interleave" test's fixture is not interleaved.

## 2. What actually works (verified)
— `rg -n "mp4::tests::" audit/runs/baseline/test-workspace.log.txt` → all 19 part1 tests `... ok` (lines 1805–1833): `probe_accepts_known_brands_and_qt_heads`, `progressive_single_track_demuxes_samples_in_order`, `progressive_two_tracks_interleave_by_file_offset`, `ctts_shifts_pts_off_dts`, `elst_empty_edit_delays_and_media_time_trims`, `absent_stss_means_all_sync`, `fragmented_demuxes_moofs_with_tfdt_ladder`, `fragmented_two_tracks_per_moof`, `chap_tref_chapters_surface_with_titles`, `cover_art_surfaces_as_a_still_image_video_track`, `png_cover_and_unknown_cover_types_map_or_note`, `cover_and_chapters_keep_packet_handles_aligned_with_tracks`, `progressive_seek_lands_on_keyframe`, `fragmented_seek_scans_moofs`, `not_mp4_is_typed_refusal`, `missing_moov_is_typed`, `truncated_box_header_is_typed_truncation`, `size_zero_convention_extends_to_stream_end`, `size_one_reads_64bit_largesize`.
— Baseline end-to-end on real ffmpeg files: `mp4-smoke-open-session-420.txt` (tracks 2 ready → duration 3s → Playing → Stopped, EXIT=0) — the demux math this suite pins is corroborated outside the suite.
— Seek math hand-checked: 6×1000 ticks @9000 Hz; target 580 ms → sample 5, previous keyframe sample 4 → landed ticks 4000 (test 415-445 asserts exactly this). ✓

## 3. Findings

### F-001 [S2] Zero language coverage in this suite while the mdhd decoder ships an off-by-one: every `und` MP4 reports `language: Some("voe")` in the shipped binary
Where: crates/demux/src/mp4/tests.rs:1-564 (the gap), crates/demux/src/mp4/mod.rs:535-539 (the bug, cross-file — file owned by W05-A06/A07/A08/A10), crates/demux/src/mp4/test_support.rs:124 (the fixture that could have caught it)
Code (mod.rs:535-539, verified `sed -n '533,540p'`):
```rust
fn unpack_language(packed: u16) -> Option<String> {
    let c = |shift: u32| -> char { (b'a' + ((packed >> shift) & 0x1F) as u8) as char };
    let tag: String = [c(10), c(5), c(0)].iter().collect();
    if tag == "und" { None } else { Some(tag) }
}
```
Code (test_support.rs:124):
```rust
    b.extend_from_slice(&u32b(0x55C4)); // "und"
```
What is wrong: ISO 14496-12 packs each mdhd language letter as `letter - 0x60` (values 1..26; `chr(0x60 + v)`), so the standard "und" packing is 0x55C4 (21,14,4 → 'u','n','d'). The decoder adds `b'a'` (0x61) instead of 0x60: (21,14,4) decodes to 'v','o','e'. I verified the on-disk bytes of both committed baseline samples with a Python box-walk: sample-420.mp4 and sample.mp4 each carry two `mdhd` boxes with language bytes `55 c4` — i.e. the files say "und"; the shipped binary answers `language: Some("voe")` (mp4-smoke-open-session-420.txt:9, mp4-smoke-open-session.txt:9, both tracks). The `if tag == "und"` filter is dead for real files: no standard muxer writes the 0x5163 packing that would be needed to produce "und" under this arithmetic. This suite builds the exact fixture bytes (test_support.rs:124, comment `// "und"`) and then never asserts what the demuxer reports for language — `rg "language" crates/demux/src/mp4/tests.rs` returns nothing.
Why it matters: user-visible wrong metadata on every MP4 without explicit language tags (the common case — both baseline samples exhibit it); the intended "und → None" cleanup never fires; and 38 green tests certify a demuxer that is wrong on this lane. This is exactly the baseline INDEX "Standing defects" item `language: Some("voe")` — root cause found.
What correct looks like: `(b'a' + (v - 1))` / `char::from(0x60 + v)` in `unpack_language`, plus a part1 test asserting `tracks[x].language == None` for the 0x55C4 fixture and `Some("eng")` for a packed `eng` fixture.
Confidence: HIGH — Verification: ran (Python parse of committed samples: `mdhd v0 lang_packed=0x55C4` ×4) + read (mod.rs/test_support quotes) + baseline cite (smoke session lines 9). The off-by-one itself is arithmetic, reproducible from the quote.

### F-002 [S4] `drain` silently swallows typed errors and never asserts EndOfStream; every part1 caller discards the events count
Where: crates/demux/src/mp4/tests.rs:57-70
Code:
```rust
fn drain(demuxer: &mut Mp4Demuxer) -> (usize, Vec<media_types::Packet>) {
    let mut packets = Vec::new();
    let mut events = 0;
    while let Ok(event) = demuxer.next_event() {
        events += 1;
        match event {
            DemuxEvent::Packet(p) => packets.push(p),
            DemuxEvent::EndOfStream => break,
            _ => {}
        }
        assert!(events < 10_000, "event budget");
    }
    (events, packets)
}
```
What is wrong: `while let Ok(event)` ends the loop silently on any `Err` — indistinguishable from a clean EOS. All 14 pass-path callers in part1 destructure `let (_, packets) = drain(...)`, discarding the events count, so no test asserts that `EndOfStream` was actually reached. A demuxer that raises a typed error after emitting the expected packets but before EOS passes every pass-path test here. The `_ => {}` arm also drops `Recovered`/`Discontinuity` events on the floor (only `png_cover_and_unknown_cover_types_map_or_note` checks a Recovered reason, via its own loop at 356-372).
Why it matters: the module doc (lines 1-4) promises "every error path pinned"; mid-walk errors after the last expected packet are unpinned and unobserved.
What correct looks like: `drain` should return whether EOS was seen and panic if the loop exited via `Err`, or return the `Result`; at minimum pass-path tests should assert `events` includes an `EndOfStream`.
Confidence: HIGH — Verification: read (all call sites scanned: every part1 caller is `let (_, packets) =`).

### F-003 [S4] "interleave_by_file_offset" test is built from a sequential, not interleaved, mdat
Where: crates/demux/src/mp4/tests.rs:151-166
Code:
```rust
    let (_, packets) = drain(&mut mp4);
    // The builder writes video samples before audio: file order.
    assert_eq!(packets.len(), 6);
    assert_eq!(packets[0].track.handle(), 0);
    assert_eq!(packets[3].track.handle(), 1);
```
What is wrong: the test name promises interleave; the fixture places all three video samples before all three audio samples (the comment admits it). What is actually pinned is only "the walk follows file offsets across two sequential runs". Sample-level interleaving inside one mdat (v,a,v,a — what every real muxer writes and what the merge-by-offset logic must reorder correctly for smooth playback) is never constructed in part1.
Why it matters: the one behavior the name advertises is the one behavior not tested; a regression in the offset-merge (e.g., per-track buffering collapse) would not be caught by this test.
What correct looks like: a `progressive_interleaved` fixture (or a test_support option) writing alternating sample chunks and asserting the emitted order.
Confidence: HIGH — Verification: read (fixture construction is `progressive(&[v, a], b"isom")`, one mdat, video run then audio run).

### F-004 [S4] Cover-type 27 framed as "unknown" — it is a defined ilst cover type; the skip may be scope, the framing is wrong
Where: crates/demux/src/mp4/tests.rs:350-352
Code:
```rust
    // An unknown well-known type: damage-noted and skipped — the
    // audio still demuxes, no fake codec is guessed.
    let bytes = progressive_with_cover(&[audio], b"M4A ", 27, &[1, 2, 3, 4]);
```
What is wrong: 27 is not an unknown value — the Apple ilst well-known-type namespace defines cover types beyond 13 (JPEG) and 14 (PNG), and ffmpeg's mov demuxer maps covr type 27 to a bitmap codec. The test comment calls it "unknown well-known type", which mischaracterizes a known-but-unsupported type as unknowable. The runtime behavior asserted (`Recovered { reason: "cover-type-unsupported" }`, audio still demuxes) is honest for a first vertical with no BMP decoder, but the test cements the "unknown" framing.
Why it matters: the next agent reading this test believes 27 carries no meaning; a future BMP support task starts from a wrong premise.
What correct looks like: rename the comment to "a known cover type this build does not decode (27 = BMP per ffmpeg's mov.c)" — and verify that mapping against ffmpeg source before relying on it.
Confidence: MEDIUM — Verification: read for the test framing; the ffmpeg 27→BMP mapping is UNVERIFIED in this sandbox (no ffmpeg source available) — flagged, not asserted as fact.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `video_track` (helper) | 15-31 | REAL | — | Defaults `sync=true`/`cts=0` for missing entries — generous but explicit at call sites. |
| `audio_track` (helper) | 33-49 | REAL | — | 44_100/mp4a, all-sync. |
| `open` (helper) | 51-55 | REAL | — | `unreachable!` on construct failure — precondition, acceptable in tests. |
| `drain` (helper) | 57-70 | REAL (flawed) | S4 | Swallows Err; EOS never asserted (F-002); 10k budget guard prevents hangs. |
| `ErrOf::err_of` | 72-83 | REAL | — | Panics if Ok — correct polarity for error-path tests. |
| `code_of` | 85-87 | REAL | — | Trivial. |
| `span_at` | 405-413 | REAL | — | ns→Timespan; correct. |
| `ftyp_bytes` | 493-502 | REAL | — | 32-byte isom ftyp; correct. |
| `probe_accepts_known_brands_and_qt_heads` | 91-114 | REAL | — | 5 accept + 3 reject cases incl. compat-brand-only and pre-ftyp QT; would fail on probe regression. |
| `progressive_single_track_demuxes_samples_in_order` | 118-149 | REAL | — | avc1→"h264", Fast{indexed}, DTS ladder, stss keyframes, payload bytes — strongest lane. |
| `progressive_two_tracks_interleave_by_file_offset` | 151-166 | REAL (misnamed) | S4 | F-003: sequential fixture, interleave untested. |
| `ctts_shifts_pts_off_dts` | 168-182 | REAL | — | B-frame shift math (0, 0, 9000) hand-verified ✓. |
| `elst_empty_edit_delays_and_media_time_trims` | 184-211 | REAL | — | Empty-edit dwell and media_time trim; trim emits negative PTS (-1000) — pinned design, see §9. |
| `absent_stss_means_all_sync` | 213-221 | REAL | — | Correct default. |
| `fragmented_demuxes_moofs_with_tfdt_ladder` | 225-254 | REAL | — | tfdt ladder (0,1024,2048,4096) + frag sync pattern hand-verified ✓. |
| `fragmented_two_tracks_per_moof` | 256-271 | REAL | — | Per-moof track alternation. |
| `chap_tref_chapters_surface_with_titles` | 275-300 | REAL | — | Hidden chapter trak, ns start times, no packet leak. |
| `cover_art_surfaces_as_a_still_image_video_track` | 304-332 | REAL | — | covr 13→mjpeg still-image track, verbatim bytes, 1 packet. |
| `png_cover_and_unknown_cover_types_map_or_note` | 334-374 | REAL | — | 14→png; 27→`cover-type-unsupported` Recovered (F-004 framing); only test checking a Recovered reason. |
| `cover_and_chapters_keep_packet_handles_aligned_with_tracks` | 376-401 | REAL | — | Handle alignment with hidden trak between media and cover. |
| `progressive_seek_lands_on_keyframe` | 415-445 | REAL | — | Two landings + resume-at-keyframe + out-of-range typed refusal; math hand-verified ✓. |
| `fragmented_seek_scans_moofs` | 447-470 | REAL | — | Linear frag scan lands 4000 ticks, 4 packets. |
| `not_mp4_is_typed_refusal` | 474-479 | REAL | — | RIFF→`demux.mp4.not-mp4`. |
| `missing_moov_is_typed` | 481-491 | REAL | — | ftyp+garbage→`demux.mp4.no-moov`. |
| `truncated_box_header_is_typed_truncation` | 504-513 | REAL | — | Half-file cut→`demux.mp4.truncated`. |
| `size_zero_convention_extends_to_stream_end` | 515-534 | REAL | — | size=0 to-EOF convention ✓. |
| `size_one_reads_64bit_largesize` | 536-564 | REAL | — | 64-bit largesize rebuild ✓. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 19 except `drain`-blindness | YES | Concrete value assertions (tick ladders, codec strings, payload bytes, typed codes); mutation reasoning: break stts delta accumulation → `progressive_single_track…` fails at ticks; break keyframe lookup → `progressive_seek…` fails at landing; break box-size arithmetic → `size_zero`/`size_one` fail at packet count. |
| Pass-path tests vs post-packet errors | NO (gap) | F-002: `drain` exits silently on `Err`; an error raised after the last expected packet but before EOS passes. |
| Language lane | NO (gap) | F-001: no assertion touches `Track.language` anywhere in the file; the live "voe" bug passes the whole suite. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every error path pinned to its typed code" | tests.rs:2-3 (module doc) | SHALLOW | 5 codes pinned in part1 (not-mp4, no-moov, truncated, seek.out-of-range, cover-type-unsupported) but `drain` swallows mid-walk errors (F-002) and no error-path test exists for the language lane (F-001). |
| (implied) track facts (codec/kind/language) are surfaced correctly | tests.rs track assertions | FALSE for language | `language: Some("voe")` in shipped binary on und-tagged files (mp4-smoke-open-session-420.txt:9); suite asserts codec/kind but never language. |
| MP4 demux works end-to-end | wave evidence | REAL | mp4-smoke-open-session-420.txt: 2 tracks, duration, Playing, EXIT=0. |

## 7. Dead / duplicated / unreachable code
- The `und`→`None` filter in mod.rs:538 is dead for real-world files (only 0x5163 would trigger it; standard muxers write 0x55C4) — cross-file, see F-001.
- No dead code inside part1 itself; the two `mdat`-scan loops in `size_zero`/`size_one` (521-525, 541-546) are duplicated but test-local.

## 8. Dependencies & flags
`#[cfg(test)]` file (wired via `mod tests` in mp4/mod.rs presumably at its tail — part2/A05 confirms). No feature gates; runs in the default `cargo test --workspace` baseline (38/38 mp4 tests ok). Fixtures are hand-built in `super::test_support` (W05-A09), independent of the demuxer — no golden-from-subject-under-test, except the shared-source risk that test_support and mod.rs co-misread the spec (exactly what happened with 0x55C4: test_support writes the right bytes, but nothing reads the language back).

## 9. Open questions for the next agent
- **W05-A06/A07/A08/A10 (mp4/mod.rs)**: file the F-001 off-by-one properly — `(b'a' + v)` should be `0x60 + v`; also audit every other 5-bit unpack in the file for the same pattern.
- **W05-A09 (test_support.rs)**: add a language-assertion lane (0x55C4 → None; packed "eng" → Some("eng")).
- Does the elst media_time trim intentionally emit negative PTS (-1000, test line 209)? ffmpeg applies edit lists via start_pad/discard; confirm the pipeline tolerates pre-zero PTS or whether samples before the edit point should be dropped — mp4/mod.rs part owners.
- Verify ffmpeg's covr type-27 mapping (F-004) against real ffmpeg source when available.
