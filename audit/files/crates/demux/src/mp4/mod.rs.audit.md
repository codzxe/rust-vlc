# AUDIT: crates/demux/src/mp4/mod.rs

| Field | Value |
|---|---|
| Auditor | W05-A01 |
| Date | 2026-11-22 |
| Lines / functions | 2225 / 51 functions + 8 structs + 9 constants |
| File verdict | REAL (defects listed below) |
| Reachable from shipped entry point? | YES — call chain in §2 |
| Findings | S0: 0 S1: 2 S2: 7 S3: 6 S4: 2 |
| Commands run | static analysis only (sandbox: no cargo/git/node). `Read` (whole file, 3 chunks), `Grep`/`rg` caller sweeps, `sed -n 'a,bp'` quote verification (5 batches), `cat -A` line check. Run-dependent claims cite the orchestrator's W00 baseline or are marked UNVERIFIED. |

## 1. One-paragraph verdict

This is a genuine, non-trivial ISO-BMFF demuxer — a self-built box walker (progressive `moov`/`stbl` tables, fragmented `moof`/`traf`/`trun`, `edts`/`elst`, chapters, cover art, 3GP brands, keyframe seek) with real typed-error taxonomy, real allocation ceilings, and zero unwrap/expect/panic/unsafe. It is reachable from the `lumen` CLI and was executed for real in the W00 baseline (h264+aac MP4 opens: 2 tracks, 3 s duration, seekable, reaches Playing; mjpeg+pcm in MOV decodes 75/75 video + 130/130 audio frames through it). But it carries two S1s (track-ID aliasing that orphans packets in chapter-bearing files; an O(chunks × stsc-entries) expansion that turns one 16 MiB `moov` into ~10¹² loop iterations) and seven S2s, including a `ctts` v1 misparse that reads 8 bytes where the entry is 4, an `elst` movie-vs-media-timescale units error baked into the unit test as the expected value, an ISO-639 language unpack off-by-one that turns every "und" into the "voe" the W00 smoke actually observed, and backward seek that is broken on both the progressive and fragmented paths. The module documentation overclaims in at least three places (depth guard that does not exist, "one sample at a time" for a fragment path that materializes a whole `moof`, "clamped" extents where the walk refuses). The demuxer is real; its timeline math and ID plumbing are not trustworthy.

## 2. What actually works (verified)

Reachability chain (rg-verified):
`lumen` CLI → `apps/cli/src/host.rs:904` `open_source(&source, …)` → `apps/cli/src/demux_host.rs:362` `open_source` → `:446` `media_probe::probe_stream` → `:458` `crate::containers::open(&container, …)` → `apps/cli/src/containers/mod.rs:67` `"mp4" => mp4::open(stream, opts)` → `apps/cli/src/containers/mp4.rs:100` `Mp4Demuxer::new(stream)` + `:101` `demuxer.start()` → **this file** `mod.rs:697` `fn open()`. The mp4 probe is mirrored in `crates/media-probe/src/signature/mp4.rs` (no crate edge, by layering).

- MP4 open/track-enum/seekability real — verified by W00 baseline (real run, 2026-11-22): h264+aac MP4 opens, 2 tracks identified (h264 video, aac audio), duration 3 s parsed, seekable, reaches Playing.
- Packet delivery real — verified by W00 baseline: mjpeg+pcm in a MOV container through this demuxer: decode-bench decodes 75/75 video + 130/130 audio frames. (h264+aac: 0 frames decoded — the release binary ships no h264/aac decoder: "no decoder in this build (first vertical: mjpeg + pcm)". That failure is downstream of this file, not in it.)
- No panic paths in the file: rg for `unwrap()|expect(|panic!|todo!|unimplemented!|unsafe` over mod.rs → 0 hits; every integer read is `u32be/u16be/u64be … .unwrap_or(0)` (bounds-checked `get`), every arithmetic site on file data is `checked_`/`saturating_`. Verified by reading every `as usize`/indexing site (list in per-function notes).
- `cargo test --workspace` 1751 pass / 2 fail (unrelated golden drift) / 25 ignored — W00 baseline. I could not run tests myself (no cargo in sandbox): UNVERIFIED locally, cited from W00.

## 3. Findings

### F-001 [S1] Track-ID aliasing: `start()` list and minted packets use different ID spaces when a chapter trak precedes an exposed trak

Where: mod.rs:833-839 (with 301-304, 669-672, 1882)

Code:
```rust
        let out: Vec<Track> = self
            .tracks
            .iter()
            .enumerate()
            .filter(|(_, t)| !t.is_chapter)
            .map(|(ordinal, t)| self.track_record(ordinal, t))
```

What is wrong: `enumerate()` runs **before** `filter()`, so `track_record` receives the RAW trak index and stamps it (`TrackId::new(ordinal as u32)`, mod.rs:885). But packets are stamped with the *filtered* ordinal: mod.rs:1882 `track: TrackId::new(self.exposed[ti]),` where `exposed` (mod.rs:816-829) renumbers non-chapter traks sequentially and maps chapter traks to `u32::MAX`. The doc comment on `exposed` (mod.rs:301-304) states the intent: "the handle `mint_packet` stamps, so packets match the `start()` list even when hidden traks leave gaps" — the implementation breaks exactly that invariant. Whenever a chapter trak sits before the last media trak (valid ISO input; e.g. `[chapter, video]` or `[video, chapter, audio]`), the Track list and the packets disagree: e.g. traks `[audio, chapter, cover]` → Track list ids `{0, 2}`, packet handles `{0, 1}` — handle 1 has no Track, Track 2 gets no packets. Second face of the same bug: `sample_entry_bytes` (mod.rs:669-672) indexes `self.tracks.get(track.handle() as usize)` — the caller `apps/cli/src/containers/mp4.rs:190` passes `track.id` (raw index) into a vector that must be indexed by raw index; under the same condition the harvest returns the WRONG track's `stsd` entry (e.g. the chapter trak's `text` entry as the video codec config).

Why it matters: on valid chapter-bearing input, packets are minted for track IDs the engine was never told about — silent track data loss — and/or decoders are configured from another track's sample entry. This is data loss on valid input (S1 per §4).

What correct looks like: build the Track list from the `exposed` mapping (`filter` before `enumerate`, or pass `self.exposed[raw]`), and make `sample_entry_bytes` map exposed handle → raw index (store the inverse).

Confidence: HIGH — Verification: read (control flow traced through `open`, `mint_packet`, `exposed` construction, and the CLI caller; fixture-order analysis in tests.rs:377-401 confirms the test never compares `tracks[i].id` to packet handles). Not executed (no cargo): UNVERIFIED at runtime.

### F-002 [S1] Chunk expansion is O(chunks × stsc-entries): one 16 MiB `moov` ⇒ up to ~10¹² inner iterations

Where: mod.rs:1351-1359

Code:
```rust
            for (ci, chunk_offset) in offsets.iter().copied().enumerate() {
                // stsc run for this chunk (1-based).
                let chunk_1 = ci as u64 + 1;
                let mut per = stsc.last().map_or(0, |(_, p)| *p);
                for (first, p) in &stsc {
                    if *first <= chunk_1 {
                        per = *p;
                    }
                }
```

What is wrong: for every chunk the code linearly rescans the entire `stsc` vector. Both are file-controlled and each capped only by `MAX_SAMPLES` (1 048 576): `stco`/`co64` (mod.rs:1268-1287) and `stsc` (mod.rs:1288-1299). A 16 MiB `moov` (the `MAX_MOOV_BYTES` ceiling, mod.rs:113/767) holds ~1 M `stco` entries (4 B each) and ~1 M `stsc` entries (12 B each) simultaneously for one track. 10⁶ × 10⁶ = 10¹² comparisons at open — hours of CPU on one core from a 16 MB file.

Why it matters: algorithmic DoS at open from a malformed-but-accepted file; §3.3 O(n²) in a hot path. Not a hang-forever loop (it terminates), so the `scan_budget_exhausted` guard never fires — this loop has no budget at all.

What correct looks like: `stsc` first-chunk values are sorted by definition; keep a moving cursor (the run active at chunk k is found in O(1) amortized), or binary-search per chunk.

Confidence: HIGH — Verification: read; bounds arithmetic from the constants at mod.rs:109-126. Runtime hang UNVERIFIED (no cargo).

### F-003 [S2] `ctts` version 1 reads an 8-byte i64 where the entry field is a 4-byte i32 — composition offsets become garbage

Where: mod.rs:1319-1321

Code:
```rust
                        let off = if version == 1 {
                            i64be(body, 12 + i * 8).unwrap_or(0)
                        } else {
```

What is wrong: a `ctts` entry is 8 bytes: `sample_count`(u32)@8+8i, `sample_offset`(i32 — signed in BOTH v0 and v1 per ISO 14496-12 §8.6.1.2)@12+8i. The v0 arm correctly reads a 4-byte `u32be … as i32` (mod.rs:1322-1323); the v1 arm reads `i64be` — 8 bytes at 12+8i, i.e. the real 4-byte offset fused with the NEXT entry's sample count: `off = (real_offset as u64) << 32 | next_count`. ffmpeg writes `ctts` v1 exactly when a stream has negative composition offsets (B-pyramid x264 is the canonical case).

Why it matters: every v1-ctts file gets PTS values off by ~2³² ticks — wrong display order, wrong A/V sync, wrong seek landing. The v0 test path (`test_support.rs:246` always writes `fullbox(b"ctts", 0, …)`) never exercises v1, and the ffprobe corpus differential compares only packet counts/keyframe counts, not timestamps — the bug is invisible to every existing test.

What correct looks like: `i64::from(u32be(body, 12 + i * 8).unwrap_or(0) as i32)` in both arms (v1 only changes the run semantics, not the field width).

Confidence: HIGH — Verification: read + spec offsets re-derived; fixture confirmed v0-only (test_support.rs:240-246). Runtime UNVERIFIED.

### F-004 [S2] `elst` empty-edit duration is movie-timescale, applied as media-timescale ticks — units error, and the unit test asserts the wrong value

Where: mod.rs:1090-1093 (parse), 1866-1869 (apply; also 870-875, 1908-1912, 2099-2103)

Code:
```rust
                            if media == -1 {
                                empty_edit_delay = seg_dur;
                            } else if media > 0 {
                                media_time = media as u64;
                            }
```
```rust
        let dts_ticks = (s.dts as i64).saturating_sub(t.media_time as i64);
        let pts_ticks = dts_ticks
            .saturating_add(s.cts)
            .saturating_add(t.empty_edit_delay as i64);
```

What is wrong: per ISO 14496-12 §8.6.6, an `elst` entry's `segment_duration` is in **movie** timescale (`mvhd`), while `media_time` is in **media** timescale (`mdhd`). `media_time` is correctly subtracted in media ticks, but `empty_edit_delay` (the empty edit's `segment_duration`) is added as if it were media ticks. With `movie_timescale` 1000 and an audio track at 48 000 Hz, a 500 ms empty edit is applied as 500/48 000 s ≈ 10.4 ms — a 48× error. Same mixing in `track_record` (start stamp), `extent_nanos`, and both `pts_of` closures. The unit test bakes the bug in: the fixture's `mvhd(1000, 0)` (test_support.rs:294) vs track timescale 9000 (tests.rs:19), `edit = Some((1_000, -1))`, and `assert_eq!(pts(&packets[0]), 1_000);` (tests.rs:196) asserts 1000 media ticks (111 ms) where the spec says 1000 movie ticks = 1 s = 9000 media ticks.

Why it matters: wrong timeline start for every gapless/priming audio file (the main real-world user of empty edits) — S2 wrong behavior in realistic cases.

What correct looks like: convert `segment_duration` from movie to media ticks at parse time (`nanos`-bridge or rational multiply) before storing; test with movie ≠ media timescale asserting the converted value.

Confidence: HIGH — Verification: read; spec units re-derived; fixture values confirmed by Read of test_support.rs:294 and tests.rs:19/187/196.

### F-005 [S2] `unpack_language` is off by one: every language string is shifted one letter; "und" surfaces as "voe" — the exact W00 smoke observation

Where: mod.rs:535-539

Code:
```rust
fn unpack_language(packed: u16) -> Option<String> {
    let c = |shift: u32| -> char { (b'a' + ((packed >> shift) & 0x1F) as u8) as char };
    let tag: String = [c(10), c(5), c(0)].iter().collect();
    if tag == "und" { None } else { Some(tag) }
}
```

What is wrong: ISO 14496-12 §12.4.2 packs each ISO-639-2/T char as `char - 0x60` (so 'a' = 1); unpacking must add `0x60`. This code adds `b'a'` (= 0x61), one too high, so every decoded letter is one past the real one. "und" is packed as 0x55C4 (21,14,4); this function decodes it as 'v','o','e' → `"voe"`. The `tag == "und"` filter can therefore NEVER fire (the only value it is meant to catch decodes as "voe") — it is dead logic. The W00 baseline (real run) observed exactly `"voe"` as the track language on the ffmpeg-authored h264+aac sample — this line is the origin. The project's own fixture writer knows the correct semantics: `test_support.rs:124` writes `b.extend_from_slice(&u32b(0x55C4)); // "und"` — and no test asserts the decoded language, so the off-by-one survived.

Why it matters: user-visible wrong metadata on every MP4 ever opened by this demuxer (every language is one letter off: "eng" → "foh"), and the "unset language" case leaks a bogus tag into `Track.language` (mod.rs:880-883) for downstream filtering/UI.

What correct looks like: `(0x60 + ((packed >> shift) & 0x1F) as u8) as char`, plus a test asserting `None` for 0x55C4 and `"eng"` for 0x15C7.

Confidence: HIGH — Verification: read + arithmetic re-derived (21→'v', 14→'o', 4→'e'); matches W00 baseline observation verbatim.

### F-006 [S2] Progressive backward seek is broken: `order_cursor` is never rewound

Where: mod.rs:1575-1577 (skip logic), 2129-2142 (seek commit — no cursor reset)

Code:
```rust
            if si < track.next {
                continue; // pre-seek sample — skipped by the cursor
            }
```
```rust
        for (ti, t) in self.tracks.iter_mut().enumerate() {
            if ti == ref_idx {
                t.next = land_idx;
```
(and mod.rs:2141-2142 `self.pending.clear(); self.eos = false;` — nothing touches `self.order_cursor`)

What is wrong: the progressive walk is a one-way cursor over the merged `order` table. `seek_progressive` rewinds each track's `next` index but not `order_cursor`. After playback has advanced (cursor at position k), a seek to an earlier time: the loop resumes at k, and the `si < track.next` skip only drops samples *before* the (rewound) `next` — it cannot re-emit samples before k. After a full drain (cursor == `order.len()`), a backward seek returns a landing `TimeSpan` claiming success, then `step_progressive` immediately hits `while self.order_cursor < self.order.len()` = false → `EndOfStream`. The returned landing is a lie. The skip comment ("pre-seek sample") shows the author only modeled forward seeks; `progressive_seek_lands_on_keyframe` (tests.rs:416-445) only seeks forward, pre-drain.

Why it matters: any user-initiated seek backwards during playback (the normal meaning of "seekable: Fast { indexed: true }", mod.rs:2046) either resumes from the furthest position reached or delivers nothing. The CLI's pre-play `--start-time` path (containers/mp4.rs:109-122) is unaffected (cursor still 0) — the shipped smoke lane cannot catch this.

What correct looks like: on seek commit, binary-search `order` for the first entry with `(ti, si)` at/after each track's `next` and set `order_cursor` to the minimum; or drop the merged cursor and emit per-track by `next`.

Confidence: HIGH — Verification: read (all writes to `order_cursor` are mod.rs:310 init and 1573 increment — rg-verified: only `order_cursor` assignments are those two sites). Runtime UNVERIFIED.

### F-007 [S2] Fragmented backward seek is broken: the moof scan starts at the current `frag_pos`

Where: mod.rs:2164 (with 2197-2203)

Code:
```rust
        let mut pos = self.frag_pos.unwrap_or(len);
```
```rust
                } else if frag_samples
                    .iter()
                    .any(|(ti, s)| *ti == ref_idx && pts_of(s) > target_ticks)
                {
                    // Passed the target without a sync landing.
                    break;
                }
```

What is wrong: `seek_fragment` scans forward from `self.frag_pos`, which `step_fragment` advances past every consumed `moof` (mod.rs:1840). Once playback has moved past fragment N, a seek to any earlier time finds only fragments whose samples are past the target → the `break` at 2202 fires on the first moof → `best` stays `None` → `error::seek_out_of_range` (mod.rs:2212-2213). Same one-way-cursor root cause as F-006, fragment face. First seek before any playback works (frag_pos = moov_end from open, mod.rs:786) — which is the only case the test exercises (`fragmented_seek_scans_moofs`, tests.rs:448-470, seeks once immediately after start).

Why it matters: `SeekSupport::SlowLinear` (mod.rs:2044) promises seek; backward seek in any fMP4 after playback started is a typed refusal. Realistic (DASH/local fMP4 + a position slider), S2.

What correct looks like: scan from the first moof (moov_end) when target < current position, or cache moof→dts landmarks.

Confidence: HIGH — Verification: read; `frag_pos` writes are mod.rs:786 (open), 1840 (step), 2216 (seek commit) — rg-verified.

### F-008 [S2] Hybrid progressive+fragmented files silently drop all `stbl` samples; the "mixed" damage note can never fire for them

Where: mod.rs:781-793 (with 798-813)

Code:
```rust
        self.fragmented = self.tracks.iter().any(|t| t.trex.is_some())
            || (!any_samples && self.any_top_box(b"moof"));
        if self.fragmented {
            // Fragment scan starts after the moov (moofs never precede
            // the moov they extend).
            self.frag_pos = Some(moov_end);
            if !any_samples {
                // Pure fMP4: fine. Mixed (some tracks progressive, some
                // not) is damage-noted.
                if self.tracks.iter().any(|t| !t.trex.is_some()) {
                    self.note(0, "mixed-progressive-fragment-tracks");
                }
            }
        } else if !any_samples {
```

What is wrong: a `moov` with BOTH a real `stbl` on track A and an `mvex/trex` for track B sets `fragmented = true` (first disjunct). Then the progressive emission `order` table is never built (`if !self.fragmented`, mod.rs:798) and `step_fragment` only walks `moof`s — track A's progressive samples are never emitted at all. The "mixed … is damage-noted" comment is false for this case: the note is gated on `!any_samples`, which is by definition false in a hybrid file. So the exact file the comment claims to flag gets silent data loss with no `Recovered` event. (Hybrid files are valid ISO-BMFF: ffmpeg plays them, mixing stbl tracks and fragmented tracks.)

Why it matters: an entire track's payload vanishes with zero diagnostic — S2 silent wrong behavior on valid input.

What correct looks like: build the `order` table for the tracks that have samples AND walk moofs for trex tracks (both paths), or damage-note `mixed-progressive-fragment-tracks` whenever `any_samples && any trex`, refusing or merging deliberately.

Confidence: HIGH — Verification: read; both branches traced (`order` build condition and `step_fragment` selection in `next_event`, mod.rs:2032-2036).

### F-009 [S2] Fragment path materializes a whole `moof`'s payloads into the pending queue — defeats the documented 2 × 16 MiB pool; memory bounded only by file size

Where: mod.rs:1806-1838 (with module doc 33-35)

Code:
```rust
                    let mut first = true;
                    let mut first_packet: Option<Packet> = None;
                    for (ti, sample) in frag_samples {
                        if sample.size == 0 || sample.size > MAX_SAMPLE_BYTES {
```
```rust
                        } else {
                            self.pending.push_back(DemuxEvent::Packet(packet));
                        }
```
Module doc (mod.rs:32-35) claims:
```rust
//!   read whole: `next_event` pulls exactly one sample's bytes
//!   through the pooled [`PayloadSlots`] arena (2 × 16 MiB ceiling,
//!   the house zero-copy scheme).
```

What is wrong: `step_fragment` reads **every** sample payload of the parsed `moof` and queues the packets in `pending` before returning the first event. Each `pool.fill` hands out a `SharedBytes` view; because the queued packets keep the arena `Arc` alive, the next `fill` finds both arenas pinned and spills to a fresh exact-size allocation per sample (`crates/demux/src/avi/riff.rs:597-670`, spill at 660-670). Total queued bytes are bounded only by the sum of sample sizes within the stream (`sample.offset + sample.size <= len`, mod.rs:1813): a crafted multi-GiB fMP4 with one 1 MiB `moof` whose trun entries scatter 16 MiB samples across the file queues ~file-size bytes in RAM before the caller sees one packet. The "one sample at a time / 2 × 16 MiB" doc claim is true only for the progressive path; for the fragment path it is false.

Why it matters: unbounded-from-input allocation class (§3.2) — bounded by file length rather than by a constant, contradicting the crate header's "Bounded memory (MP §12)" pledge; also a doc-vs-code falsehood.

What correct looks like: return the first packet and keep the fragment's parsed (offset,size) sample list — not the payloads — in `pending`, pulling bytes lazily per event; or cap queued bytes per moof.

Confidence: HIGH — Verification: read of this file + read of `PayloadSlots::fill` (riff.rs:597-670: pinned arenas → spill). Runtime memory profile UNVERIFIED.

### F-010 [S3] Seek landing `pts_of` omits `cts` while `mint_packet` includes it; the doc comment claims exact equality

Where: mod.rs:2087-2103 (doc 2087-2091 + closure 2099-2103), vs 1867-1869

Code:
```rust
    /// Progressive seek: reference-track keyframe walk-back, then
    /// every track's first sample ≥ the landing. The landing is in
    /// the emitted (movie-timeline PTS) domain — exactly what
    /// `mint_packet` stamps, so a caller re-targeting the landing
    /// never chases a stamp the demuxer will not produce.
```
```rust
        let pts_of = |t: &TrackState, s: &Sample| {
            s.dts
                .saturating_sub(t.media_time)
                .saturating_add(t.empty_edit_delay)
        };
```

What is wrong: `mint_packet` computes `pts = dts − media_time + cts + empty_edit_delay` (mod.rs:1867-1869); the seek `pts_of` omits `s.cts`. For any B-frame stream the landing stamp returned to the engine differs from the PTS of the first packet actually delivered (by the keyframe's composition offset), and the "last sample with PTS ≤ target" selection is really DTS-based. The doc sentence "exactly what `mint_packet` stamps" is false.

Why it matters: clock/position error of up to a reorder delay on every seek in B-frame video; doc-vs-code falsehood. Bounded magnitude → S3.

What correct looks like: `.saturating_add(s.cts as …)` in `pts_of` (handling negative cts), or fix the doc to say DTS-domain.

Confidence: HIGH — Verification: read, both closures compared.

### F-011 [S3] `mdhd` version 1 language field read at offset 28 — inside the 64-bit duration; correct offset is 32

Where: mod.rs:1105-1109

Code:
```rust
                                let (ts_at, dur_at, lang_at) = if version == 1 {
                                    (20, 24, 28)
                                } else {
                                    (12, 16, 20)
                                };
```

What is wrong: `mdhd` v1 layout is version/flags(4), creation(8)@4, modification(8)@12, timescale(4)@20, duration(8)@24, language(2)**@32**, pre_defined(2)@34. The v0 arm (12/16/20) is correct; the v1 arm reads the language from bytes 28..30 — the high half of the duration field. Every v1-`mdhd` track gets a garbage language (compounding F-005's shift). The fixture writer only ever emits v0 (`fullbox(b"mdhd", 0, …)`, test_support.rs:126), so this is untested.

Why it matters: wrong metadata for v1-mdhd files (64-bit durations — long recordings, some modern muxers); S3.

What correct looks like: `(20, 24, 32)`.

Confidence: HIGH — Verification: read + ISO 14496-12 §12.4.2 field offsets re-derived. Runtime UNVERIFIED.

### F-012 [S3] Second `start()` returns the "not-started" error — semantically inverted; forced a CLI workaround adapter

Where: mod.rs:2009-2012

Code:
```rust
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        if self.started {
            return Err(error::not_started());
        }
```

What is wrong: `error::not_started()` is documented (error.rs:141-147) as "A trait method was called **before** `start` opened the container" and carries code `demux.mp4.not-started`. Here it is returned for the opposite condition — a second `start()`. The naming lie has already cost cross-layer code: `apps/cli/src/containers/mp4.rs:145-158` adds the `StartedMp4` adapter whose whole reason to exist is this inverted refusal ("the engine's own `start()` would hit the demuxer's deliberate second-call refusal (`demux.mp4.not-started` …)"). The worklog (docs/worklog/archive-2026-10.md:10508) records the same workaround.

Why it matters: misleading error taxonomy + a permanent workaround seam in the shipped CLI caused by this file; S3.

What correct looks like: a distinct `already-started` code (or answer the cached list like ADTS/AVI do, per the adapter comment).

Confidence: HIGH — Verification: read of mod.rs + error.rs:141-147 + apps/cli/src/containers/mp4.rs:145-158 (rg-verified callers).

### F-013 [S3] The documented box-depth guard does not exist; `MAX_BOX_DEPTH` and `error::depth_exceeded` are dead code; "clamped" extents are actually refusals

Where: mod.rs:31 + 126 (doc/constant), 342-344 (iter_boxes doc); error.rs:78-90

Code:
```rust
//!   walked in memory with a depth guard ([`MAX_BOX_DEPTH`]) and a
```
```rust
pub const MAX_BOX_DEPTH: u32 = 8;
```
```rust
/// Walk boxes in a structural buffer (`data[range]`), enforcing
/// header truncation, 64-bit sizes, size==0 (to end), non-advancing
/// extents and child>parent.
```

What is wrong: rg over the whole mp4 module shows `MAX_BOX_DEPTH` occurs only at its definition (mod.rs:126) and this doc line; `depth_exceeded` occurs only in error.rs. No depth parameter exists anywhere in the walk. The real walk is structurally bounded (fixed nesting: `parse_moov` → `parse_trak` → mdia/minf/stbl, each an explicit `iter_boxes` call), so there is no actual depth risk — but the documented guard is fiction, the constant and constructor are dead code, and the module doc's "Declared sizes are hints; the stream is authority … every box extent is clamped to parent ∩ stream" (mod.rs:22-24) is also not what `iter_boxes` does: a child whose declared end exceeds the parent is a typed **refusal** (`box_overruns_parent`, mod.rs:371-373), not a clamp (only the top-level scan clamps, mod.rs:720).

Why it matters: dead code + two doc-vs-code falsehoods in the security-relevant "what can this parser be trusted to bound" story; S3.

What correct looks like: delete `MAX_BOX_DEPTH` + `depth_exceeded`, or implement the guard; reword the doc to "refused, not clamped" for in-memory walks.

Confidence: HIGH — Verification: rg `MAX_BOX_DEPTH|depth_exceeded` over crates/demux/src/mp4 → only the three definition/doc sites listed above.

### F-014 [S3] A present-but-empty `stss` is treated as all-samples-sync

Where: mod.rs:1368-1372

Code:
```rust
                    let sync = match &sync_set {
                        Some(set) => set.binary_search(&sample_no).is_ok() || set.is_empty(),
                        // ISO 14496-12 §8.6.2: absent stss ⇒ all sync.
                        None => true,
                    };
```

What is wrong: the ISO citation is correct for an **absent** `stss` (all samples sync). For a **present** `stss` with entry_count 0, the code also declares every sample a keyframe (`|| set.is_empty()`), while a writer that emitted an empty `stss` is asserting the opposite (no sync samples; ffmpeg's `mov_read_stss` marks keyframes only from the table and treats the empty-present case as non-key). Every packet is then flagged `keyframe: true` (mod.rs:1887), so downstream seek/decode-refresh logic is lied to.

Why it matters: wrong keyframe flags for a pathological-but-parseable table; narrow blast radius → S3.

What correct looks like: `Some(set) => set.binary_search(&sample_no).is_ok()` and a damage note for the empty-present case.

Confidence: MEDIUM — Verification: read; ffmpeg's exact empty-present behavior not re-derived from source in this sandbox (UNVERIFIED), the ISO-absent case is verified.

### F-015 [S4] Probe/doc mismatches: `skip` accepted but undocumented; brand scan limited to the 32-byte prefix; `CodecDescriptor` failure mapped to `bad_box(0)`

Where: mod.rs:150-152 + 173 (probe), 699 (32-byte prefix), 879 (error mapping)

Code:
```rust
    // QuickTime legacy: files may open with wide/free before moov.
    matches!(&kind, b"moov" | b"wide" | b"free" | b"skip")
```
```rust
        let codec = CodecDescriptor::new(&t.codec).map_err(|_| error::bad_box(0))?;
```

What is wrong: (a) the probe doc (and `error::not_mp4`'s doc, error.rs:25-26) enumerate `moov`/`wide`/`free` but the code also claims `skip` — underdocumented acceptance surface. (b) `open` probes only the first 32 bytes (`read_prefix_at(32, 0)`, mod.rs:699), so an `ftyp` whose known brand sits beyond byte 28 (long compatible-brand lists) is refused despite being claimable — a false-negative probe class. (c) if a `codec_of` name were ever absent from the registry, open fails with `bad_box(0)` — an "invalid structure at offset 0" error that names neither the codec nor the real cause.

Why it matters: hygiene/doc drift and a misleading error; no observed user-visible failure (all current names are registry-backed — the 30-lane corpus passes, W00 baseline). S4.

What correct looks like: fix the two doc sentences; probe a longer prefix (or re-read the ftyp body for brands); mint a dedicated `unknown-codec-descriptor` error.

Confidence: HIGH for (a)/(c) (read); (b) is arithmetic on the 32-byte read + probe loop bounds (read).

### F-016 [S4] `stz2` 4-bit field width is misread as 8-bit

Where: mod.rs:1242-1246

Code:
```rust
                        let v = if field == 16 {
                            u64::from(u16be(body, 12 + i * 2).unwrap_or(0))
                        } else {
                            u64::from(body.get(12 + i).copied().unwrap_or(0))
                        };
```

What is wrong: `stz2` allows `field_size` of 4, 8, or 16 (ISO 14496-12 §8.7.3.2). The code handles 16 explicitly, and treats **everything else** as 8-bit — a 4-bit table is read as bytes, halving the sample count actually consumed and pairing every sample with its neighbor's size. No damage note distinguishes the unsupported width.

Why it matters: rare box (QuickTime-era, compact tables); wrong sizes → wrong packet boundaries if ever hit; S4 given rarity, but silent.

What correct looks like: `match field { 4 => pack two nibbles per byte, 8 => …, 16 => …, _ => damage note }`.

Confidence: HIGH — Verification: read; field layout re-derived from the box definition.

### F-017 [S3] Sample-table memory amplification: a 16 MiB `moov` can materialize ~200-300 MiB of tables

Where: mod.rs:770 (moov buffer), 1346-1347 (samples Vec), 807-812 (order Vec), caps at 109-122

Code:
```rust
        let mut data = vec![0u8; moov_len as usize];
```
```rust
            let mut samples: Vec<Sample> =
                Vec::with_capacity(if pcm { offsets.len() } else { count });
```
```rust
                for (si, s) in t.samples.iter().enumerate() {
                    order.push((s.offset, ti, si));
                }
```

What is wrong: per sample the file spends as little as 4 bytes (`stsz` fixed-size runs cost 8 bytes per `stts` **run**, and a single run can cover 1 M samples), while the demux materializes `Sample` (48 B) + an `order` tuple (24 B) + the transient `sizes`/`deltas` vectors (8 B each) — an amplification of ~12-15× per track, each track capped at `MAX_SAMPLES` (1 M) with several tracks fitting in one 16 MiB moov. 4 tracks × (48 MB samples + 8 MB sizes + 8 MB deltas) + 96 MB order ≈ 300 MB resident from a 16 MB header. Bounded by the caps, so not S1, but the crate header's "Bounded memory (MP §12)" story is only as good as this multiplier.

Why it matters: memory-pressure DoS multiplier on a small crafted file; S3.

What correct looks like: a global (cross-track) sample budget rather than per-track ceilings, or store `order` implicitly (offsets are already per-sample).

Confidence: MEDIUM — Verification: read + arithmetic on struct sizes (`Sample` = 5×u64/i64 + bool ⇒ 48 B padded; tuple (u64,usize,usize) = 24 B); actual residency not measured (no cargo): UNVERIFIED at runtime.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `probe` | 154-174 | REAL | S4 | No panics; brand loop bounds-checked. Doc omits `skip` (F-015a); 32-byte window limits visible brands (F-015b). |
| `u32be` | 318-321 | REAL | — | `data.get` guarded; correct BE. |
| `u16be` | 324-327 | REAL | — | Guarded; correct BE. |
| `u64be` | 330-335 | REAL | — | Guarded; correct BE. |
| `i64be` | 338-340 | REAL | — | Cast of u64be; fine. Misused by ctts v1 (F-003). |
| `iter_boxes` | 345-384 | REAL | S3 | 64-bit sizes, size==0, non-advancing, child>parent all enforced; `checked_add` on extents; no depth param despite doc (F-013); refuses where doc says clamps. |
| `read_top` | 390-433 | REAL | — | 16-byte prefix read; `checked_add`; typed truncation; clamps `pos` to `len-8`. Correct. |
| `codec_of` | 438-470 | REAL | — | 26 fourcc→name rows + None (damage). Answer to the domain question — see §6. |
| `pcm_codec_of_ipcm` | 481-522 | REAL | — | pcmC walk after 28-byte v0 base; widths/endianness mapped; unmapped → None (no guess). Bounds via `iter_boxes`/`get`. |
| `kind_of` | 525-532 | REAL | — | vide/soun/sbtl/text/subt/clcp→Subtitle, else Data. `meta`/`hint` traks become Data kind — harmless (no hdlr for them). |
| `unpack_language` | 535-539 | SHALLOW | S2 | Off-by-one decode; "und" filter dead; "voe" origin (F-005). |
| `oti_of_esds` | 542-570 | REAL | — | Expandable lengths (≤4), tag checks; all `get`-guarded; returns None on odd shapes. |
| `esds_oti` | 576-599 | REAL | — | Tries 3 anchor offsets (28/76/78); no panic; heuristic documented. |
| `StickyShape::of` / `::mint` | 252-258 / 260-263 | REAL | — | Error-shape replay; clone only of small error parts. |
| `Mp4Demuxer::new` | 610-631 | REAL | — | No I/O; pool `PayloadSlots::new(2, 16 MiB)`; defaults sane. |
| `major_brand` | 635-637 | REAL | — | Observability; callers: tests only (rg). |
| `compatible_brands` | 641-643 | REAL | — | Observability; tests only. |
| `is_fragmented` | 647-649 | REAL | — | Observability; tests only. |
| `chapters` | 654-656 | REAL | — | Used by apps/cli containers/mp4.rs:640 (real caller). |
| `pool_bytes` | 660-662 | REAL | — | Observability (D-1); tests only in mp4. |
| `sample_entry_bytes` | 669-672 | REAL | S1 | Indexes raw vec by exposed handle — aliases under chapter-first (F-001). Real caller: apps/cli mp4.rs:190. |
| `note` | 675-677 | REAL | — | Damage queue only. |
| `surface_damage` | 679-687 | REAL | — | Drains damage into Recovered events; take-and-push, no panic. |
| `fail` | 690-693 | REAL | — | Sticky set; correct. |
| `open` | 697-842 | REAL | S1/S2 | moov-at-end OK (header-only scan, mdat skipped, 65536-box budget); moov ≤16 MiB ceiling; **enumerate-before-filter track IDs (F-001)**; **hybrid drop (F-008)**; order build amplification (F-017); unknown top boxes damage-noted. |
| `any_top_box` | 845-866 | REAL | — | Bounded scan; swallows errors to false (acceptable for its single use). |
| `track_record` | 869-910 | REAL | S2 | `start` stamp mixes empty_edit_delay units (F-004); `CodecDescriptor` failure → `bad_box(0)` (F-015c). |
| `parse_moov` | 914-999 | REAL | — | mvhd/mvex/trex/udta/meta/trak dispatch; per-trak failure swallowed to damage note (`let _ = err`, 976-977) — deliberate, loses error detail (S4-level, noted). |
| `parse_udta_chpl` | 1003-1041 | REAL | — | Nero chpl; count ≤ 64; all reads `get`-guarded; lossy UTF-8 by design. |
| `parse_trak` | 1045-1426 | REAL | S1/S2/S3 | tkhd/edts/mdia/hdlr/minf/stbl; mdhd v0 offsets correct, **v1 lang_at=28 wrong (F-011)**; elst first-entry only (documented + damage note for multi); **elst units (F-004)**; stsz/stz2/stts/stco/co64/stsc/stss/ctts tables capped at MAX_SAMPLES with damage notes; **ctts v1 8-byte read (F-003)**; **stsc rescan per chunk (F-002)**; empty-stss all-sync (F-014); stz2 4-bit (F-016); expansion + PCM 1024-split correct per corpus comment; `sizes[si]`/`deltas` indexing guarded by `si < count`. |
| `push_cover_track` | 1433-1480 | REAL | — | wkt 13/14 only, others damage-noted; size ≤ 16 MiB; synthetic id u32::MAX. Correct. |
| `queue_cover_packets` | 1485-1515 | REAL | — | Bounds-checked offsets; short read → damage skip; pool fill. Correct. |
| `read_chapter_titles` | 1519-1565 | REAL | — | size ≤ 4096 cap; short read damage-noted; u16 length prefix; `?` propagates source errors (hard open failure on source error — acceptable). |
| `step_progressive` | 1569-1614 | REAL | S2 | One sample per event; zero-size/over-ceiling skip; past-stream → typed truncation + sticky; **cursor never rewound by seek (F-006)**. |
| `parse_moof` | 1621-1773 | REAL | — | tfhd flags/base-offset/tfhd-defaults, tfdt v0/v1, trun flags 0x001-0x800 all correct; `unsigned_abs().min(u64::MAX/2)` tames negative offsets; sync bits 0x8000/0x10000 dual reading documented; unknown traf damage-noted; `count.min(MAX_SAMPLES)` cap. Correct except it feeds F-009's eager queue. |
| `step_fragment` | 1775-1861 | REAL | S2 | Budget 65536; moof ≤ 1 MiB; truncated moof typed; **whole-moof eager queue (F-009)**; `frag_pos` advance (F-007 root). |
| `mint_packet` | 1864-1891 | REAL | S1/S2 | pts = dts − media_time + cts + empty_edit_delay: **units mix (F-004)**; **`exposed[ti]` handle vs raw-index Track list (F-001)**; duration via nanos; `GenerationId::FIRST` always (fine for v0). |
| `seek_reference` | 1894-1899 | REAL | S3 | First video else first non-chapter — a cover track (Video kind) wins the reference for audio+cover files; seek then targets a 1-sample still. Edge case, unnoted. |
| `extent_nanos` | 1903-1915 | REAL | S2 | Same empty_edit_delay units mix (F-004); saturating; timescale ≠ 0 guaranteed by parse_trak. |
| `Demuxer::start` | 2009-2017 | REAL | S3 | **Second call → not_started (F-012)**; open errors made sticky. |
| `Demuxer::next_event` | 2019-2037 | REAL | — | Sticky → pending → step; eos sticky. Correct ordering. |
| `Demuxer::seekability` | 2039-2048 | REAL | — | Fast{indexed} / SlowLinear as documented. |
| `Demuxer::seek` | 2050-2083 | REAL | — | Negative/out-of-extent refused pre-compute (D4); dispatch to the two seek fns. Correct glue; the bodies are F-006/F-007/F-010. |
| `seek_progressive` | 2092-2148 | REAL | S2/S3 | Keyframe walk-back correct; **no order_cursor reset (F-006)**; **pts_of omits cts + false doc (F-010)**; compute-then-commit honored for cursors it does set. |
| `seek_fragment` | 2152-2224 | REAL | S2 | **Scans from current frag_pos → backward seek refusal (F-007)**; first-sync-per-moof landing; budget guard; truncated moof silently breaks (progressive path errors — inconsistent). |
| `cover_in_meta` | 1923-1959 | REAL | — | FullBox/plain meta both tried; all `get`/`iter_boxes` guarded; wkt read. Correct. |
| `cover_in_udta` | 1963-1969 | REAL | — | Thin wrapper. Correct. |
| `trak_chap_refs` | 1972-1992 | REAL | — | ids capped at 64; guarded. Correct. |
| `ticks_to_nanos` | 1995-2001 | REAL | — | u128 intermediate, saturating, timescale-0 guarded. Correct. |
| `nanos_to_ticks` | 2004-2006 | REAL | — | u128, floor, saturating. Correct. |
| (structs) `Chapter`/`Sample`/`TrexDefaults`/`TrackState`/`StickyShape`/`RawBox`/`TopBox`/`Mp4Demuxer` | 178-315 | REAL | — | Data carriers; no logic, no invariants beyond field docs. |
| (constants) `MAX_SAMPLE_BYTES`/`MAX_MOOV_BYTES`/`MAX_MOOF_BYTES`/`MAX_SAMPLES`/`MAX_BOX_DEPTH`/`MAX_FRAGMENT_SCAN_BOXES`/`KNOWN_BRANDS`/`EXTENSIONS`/`SAMPLE_IS_NON_SYNC` | 109-146 | REAL | S3 | Ceilings real and enforced at their sites; **`MAX_BOX_DEPTH` dead (F-013)**. |

## 5. Test honesty (tests live in sibling files — cross-file judgment)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `progressive_seek_lands_on_keyframe` (tests.rs:416) | Partially | Catches broken walk-back/landing stamps for forward seeks; never seeks backward after drain → F-006 escapes. |
| `fragmented_seek_scans_moofs` (tests.rs:448) | Partially | One forward seek pre-drain only → F-007 escapes. |
| `ctts_shifts_pts_off_dts` (tests.rs:169) | Only for v0 | Fixture writes ctts v0 only (test_support.rs:246) → F-003 (v1) escapes. |
| `elst_empty_edit_delays_and_media_time_trims` (tests.rs:185) | **No — it entrenches the bug** | mvhd=1000 vs track=9000; asserts 1000 media ticks for a 1000-movie-tick edit → F-004's wrong value is the pinned expectation. |
| `cover_and_chapters_keep_packet_handles_aligned_with_tracks` (tests.rs:377) | **No** | Asserts `p.track.handle() <= 1` and cover payload, but never compares `tracks[i].id` to packet handles — F-001 is live in its own fixture (cover TrackId 2 vs packet handle 1) and passes. |
| `unpack_language` coverage | No test exists | No test asserts language; fixture writes 0x55C4 labeled `// "und"` (test_support.rs:124) → F-005 escapes (and W00 observed "voe" live). |
| `fuzz_lite_mutations_never_panic` / `..._fragment_mutations` (tests.rs:1078/1099) | For panics only | Real mutation loop over fixtures (panic hunt); no semantic assertions — correct for its stated goal, blind to all value-level findings here. |
| `corpus_mp4.rs` (30-lane ffprobe differential) | For counts/codecs/keyframes | Real measured truth, real access stack — but compares packet COUNTS and K-flags only, never PTS/DTS/language → F-003/F-004/F-005/F-010 all escape the "30-lane corpus" evidence. Skips loudly when corpus absent (W00 run counts UNVERIFIED for this suite). |
| `stco/co64/stsc_count_over_ceiling…` (tests.rs:967-1036) | Yes | Genuinely pin the MAX_SAMPLES caps + damage reasons. |

Mutation check (§10 step 5, no cargo — reasoned, UNVERIFIED at runtime): e.g. deleting the `saturating_add(s.cts)` in `mint_packet` would fail `ctts_shifts_pts_off_dts`; deleting the `stsc` run-selection body would fail `progressive_two_tracks_interleave_by_file_offset`; deleting the `si < track.next` skip would fail forward-seek resumes. So the happy-path tests are real; the gaps are exactly the ones listed above.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Demux ISO-BMFF family: track boxes, stco/stsc sample tables, edts/elst, 3gp quirks, fragmented MP4" | feature-matrix.toml:142-152 (row container-mp4-mov-3gp, status windows/linux/macos = "implemented") | REAL | W00 baseline: h264+aac MP4 opens, 2 tracks, 3 s, seekable, Playing; mjpeg+pcm MOV 75/75 + 130/130 decoded. Code read in full this audit. |
| "2026-11-15: end-to-end VIDEO PLAYBACK proven on the h264+aac lane — mp4_h264_aac.mp4 plays to Ended (exit 0 …) through the … software decoder" | same row, notes (feature-matrix.toml:152) | **FALSE for the shipped release binary** | W00 baseline (real run of the release build): 0 frames decoded, binary says "no decoder in this build (first vertical: mjpeg + pcm)". The lane either used a non-default feature set or the claim drifted. Demux itself is not the failure point — belongs to codec rows (W06/W18 to adjudicate the feature-flag question). |
| "No MP4 demuxer code exists; the MP4 box walker … is TEST-SIDE" | same row, notes (stale D-018 sentence in the same `notes` string as "implemented") | Stale self-contradiction | Contradicted by this 2225-line file + W00 runs. Internal contradiction inside one matrix row — for the claim ledger (W18/W19). |
| "walked in memory with a depth guard (MAX_BOX_DEPTH)" | mod.rs:31 | FALSE | F-013: no depth guard exists; constant dead. |
| "next_event pulls exactly one sample's bytes through the pooled PayloadSlots arena (2 × 16 MiB ceiling)" | mod.rs:33-35 | FALSE for fMP4 | F-009: step_fragment queues a whole moof's payloads; pool spills per sample (riff.rs:660-670). |
| "every box extent is clamped to parent ∩ stream" | mod.rs:22-24 | FALSE as stated | F-013: in-memory walks REFUSE (box_overruns_parent, mod.rs:371-373); only the top-level scan clamps (mod.rs:720). Refusal is safer, but the doc lies. |
| "stss absence means all-samples-sync — the same reading ffmpeg and mp4parse apply" | mod.rs:48-49 | REAL for absence | Correct for absence (mod.rs:1371); the empty-PRESENT divergence is F-014. |
| "v0 applies the first elst entry per track … multi-edit splices are a damage-noted gap, not a silent first-entry-only lie" | mod.rs:41-44 | REAL | count>1 → `elst-multi-entry` damage note (mod.rs:1077-1079, 1208-1210). Verified. |
| "cmov (compressed MOV) is refused typed (bad-box vocabulary)" | mod.rs:68 | REAL | `cmov` is a top-level box: the unknown-arm counts it (mod.rs:751-753), the scan never finds a `moov` → typed `demux.mp4.no-moov` (mod.rs:760-762), with an `unknown-top-box-skipped` damage note (mod.rs:764). Refused — but as no-moov, not a cmov-specific code. Partially accurate (S4-level wording). |
| Crate header: "Bounded memory (MP §12): every read is chunk-bounded and payload sizes are refused above a typed ResourceLimit ceiling" | crates/demux/Cargo.toml:13-14 | PARTLY FALSE | Ceilings exist (moov/moof/sample) but F-009 (file-size-bounded pending queue) and F-017 (≈15× table amplification) break the spirit; F-002 breaks the CPU analogue. |
| Codec fourcc recognition (domain question) | codec_of mod.rs:438-470 + pcm 481-522 | Answer | Video: avc1/avc2/avc3/avc4/avcp→h264, hvc1/hev1→hevc, mp4v→mpeg4video (esds OTI 0x6c remap→mjpeg), mjpa/mjpb/jpeg→mjpeg, vp08, vp09, av01, s263/h263→h263. Audio: mp4a→aac (esds OTI 0x6b remap→mp3), alac, ac-3, ec-3, opus, fLaC, sowt→pcm_s16le, twos/lpcm/raw→pcm_s16be, in24/in32/fl32/fl64, ulaw/alaw, .mp3→mp3, samr/sawb→amr_nb/wb, ipcm/fpcm via pcmC. Text: tx3g/text→mov_text. Unknown → damage note "unknown-stsd-entry" + trak skipped (mod.rs:1181-1184). |
| H.264/AAC demuxed into packets? (domain question) | codec_of + W00 | Answer | YES at demux level: avc1→h264 and mp4a→aac are recognized and packetized (W00: 2 tracks, Playing, packets flowing). The stsd entry payload (incl. avcC/esds) rides `extra_data` verbatim (mod.rs:1185) for the codec layer; the shipped release binary contains no h264/aac decoder → 0 frames decoded there (W00). No refusal happens in this file. |

## 7. Dead / duplicated / unreachable code

- `MAX_BOX_DEPTH` (mod.rs:126) and `error::depth_exceeded` (error.rs:78-90): dead — no caller (rg over the mp4 module: 3 hits, all definitional/doc). F-013.
- The `tag == "und"` filter inside `unpack_language` (mod.rs:538): dead in practice — its only trigger value decodes as "voe" (F-005).
- Observability accessors `major_brand`/`compatible_brands`/`is_fragmented`/`pool_bytes` (mod.rs:635-662): production callers none; test-only (rg). Fine as designed ("observability" is the documented purpose) — listed for the dead-code ledger.
- No duplicated logic found within the file; the `moof` read-and-parse block is duplicated between `step_fragment` (mod.rs:1785-1802) and `seek_fragment` (mod.rs:2172-2188) — near-identical 15-line bodies (S4-level duplication, one behavioral divergence: truncated moof errors in one, silently breaks in the other).

## 8. Dependencies & flags

- `crates/demux/Cargo.toml`: no `[features]` section — the mp4 module is compiled unconditionally into every build of the `demux` crate; no feature flag can turn it off. Dependencies: `media-types`, `plugin-api` (path-only, std-only crate per header). No unsafe, no external crates.
- Uses `crate::avi::riff::{PayloadSlots, SourceReader}` (mod.rs:97) — cross-module reuse from the AVI demuxer (intra-crate, no layering violation).
- The CLI wiring (`containers/mod.rs:67`) is unconditional within apps/cli; the release workflow's feature set was not re-verified here (no cargo/git in sandbox) — the W00 baseline release build demonstrably includes and reaches this file (h264+aac MP4 opened, mjpeg+pcm MOV decoded).

## 9. Open questions for the next agents

- W13 (CLI/engine): does the engine ever issue an in-session backward seek through `SelectDemuxer`/`reanchor`? F-006/F-007 make the shipped `--start-time` path safe but any slider/seek command broken. Also confirm whether `StartedMp4`'s cached-track path masks F-001 for the CLI (it does not — the Track ids come from `start()` itself).
- W06/W18 (codecs/matrix): resolve the feature-flag question behind the matrix row's "end-to-end VIDEO PLAYBACK … plays to Ended" vs the W00 release binary's "no decoder in this build". The demuxer is exonerated either way.
- W16 (tests): the corpus differential compares counts, not timestamps — PTS/DTS lanes would have caught F-003/F-004/F-010. Worth a lane.
- avi/riff.rs owner: the `PayloadSlots` spill policy (riff.rs:654-670) means any demuxer that queues multiple packets per event defeats the pool bound — same pattern likely in ts/ps/mkv `next_event` implementations (not audited here).
- media-types owner: confirm every `codec_of` name is a registered `CodecDescriptor` (the `bad_box(0)` fallback at mod.rs:879 would otherwise turn an add/remove into a confusing open failure).
