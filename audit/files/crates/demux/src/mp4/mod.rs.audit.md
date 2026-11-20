# AUDIT: crates/demux/src/mp4/mod.rs
| Field | Value |
|---|---|
| Auditor | W05-A01 (Devil-Auditor, hostile evidence-only) |
| Date | 2026-11-16 |
| Lines / functions | 2225 / 51 (18 free functions + 33 methods, incl. 2 `StickyShape` methods) |
| File verdict | **REAL** — a genuine, substantial ISO-BMFF demuxer with real defects at the edges (no panics found; several silent-wrong-output and DoS-class holes) |
| Reachable from shipped entry point? | **YES** — `lumen open <file.mp4>` → apps/cli `main.rs` → `run.rs` → `host.rs::open_session` → `demux_host.rs:458 crates::containers::open(&container, stream, &opts)` → `containers/mod.rs:67 "mp4" => mp4::open(...)` → `containers/mp4.rs:100 Mp4Demuxer::new(stream)` → `Demuxer::start/next_event/seek` → this file. Probe side: `media_probe` FormatId "mp4" names the lane. |
| Findings | S0:0 S1:3 S2:4 S3:12 S4:5 (24 total) |
| Commands run | `wc -l`; `rg "fn "` (function census); `rg "order_cursor|discontinuity|MAX_BOX_DEPTH|depth_exceeded"` (usage proofs); `rg -n "unwrap\(\)|expect\(|panic!|unreachable!|todo!|dbg!|unsafe"` on mod.rs (zero hits outside safe fixed-index reads); `rg -c "unwrap_or"` = 67; `rg " as "` count = 86; `rg "let _ ="` = 2; `sed -n` verbatim verification of every quote below; `rg` over `apps/cli/src`, `crates/demux/src/lib.rs`, `crates/demux/src/avi/riff.rs`, `crates/media-probe`, `docs/feature-matrix.toml`; `od -t x1z` on `tests/fixtures/generated/valid/mp4_h264_aac.mp4` (box order: ftyp@0, moov@32 — moov FIRST). No cargo run (per orders). |

## 1. One-paragraph verdict

This is a real, working ISO-BMFF demuxer, not a stub: the executed `lumen open` trace (2026-11-16, orchestrator) demuxed a real 3s H.264+AAC MP4 (`tracks 2 ready`, duration `Timestamp(3000000)`, seekable=true, clean `PlaybackEndOfTrack`) through exactly this file's `open()→parse_moov()→parse_trak()→step_progressive()` path, and the 51 functions here carry real box-walking, sample-table expansion, elst/edts application, chapters, cover art, and a genuine moof/trun fragment engine. The happy path is honest. The hostile edges are not: three S1 defects — (1) packets are stamped with `exposed[]` ordinals while the `start()` track list is stamped with raw enumerate ordinals, so any file whose chapter trak precedes a media trak delivers packets on TrackIds that do not exist in the track list; (2) a backward progressive seek after playback has begun returns a landing TimeSpan and then delivers samples from the *old* cursor (`order_cursor` is never reset by seek); (3) `stts`/`ctts` parse loops iterate the full file-declared entry count with no body-size or ceiling clamp, turning a 12-byte box into minutes-to-days of CPU burn. Four S2 defects (quadratic stsc mapping, silent dropping of progressive samples in mixed progressive+fragmented files, an elst empty-edit units bug mixing movie and media timescales, and fragment-seek that cannot scan backwards). The remaining findings are doc-vs-code lies (a depth guard that does not exist, a "one sample at a time" claim violated by whole-moof materialization), stub seams (`discontinuity` never true), and swallowed errors.

## 2. What actually works (verified)

- **Happy-path demux (execution-proven).** The orchestrator's `lumen open` on `tests/fixtures/generated/valid/mp4_h264_aac.mp4` (13703 bytes; `od` shows `ftyp` at 0 with brands isom/iso2/avc1/mp41, `moov` at 32) printed `tracks 2 ready`, h264/aac track info, `duration Timestamp(3000000)`, `seekable=true`, then `PlaybackEndOfTrack` with zero frames decoded (no decoder in build). That exercises: `probe()` (154), `open()`'s top-level scan (697), `parse_moov()` (914), `parse_trak()` (1045) with tkhd/mdhd/hdlr/stsd/stbl tables, `track_record()` (869), and `step_progressive()` (1569) — the parser side works end-to-end on a real file.
- **Box walker hardening is real.** `iter_boxes` (345) and `read_top` (390) reject header truncation, `largesize < 16`, box-overruns-parent, and non-advancing extents with typed errors; `checked_add` everywhere on box arithmetic; no `unwrap/expect/panic/unreachable/unsafe` in the 2225 lines (verified by rg; the only fixed-index reads at 320/326/402/728/1397 are all guarded by preceding length checks).
- **Ceilings are honored where they exist**: `MAX_MOOV_BYTES`/`MAX_MOOF_BYTES`/`MAX_SAMPLE_BYTES` checked before allocation (768, 1790, 1584, 1442); `stsz/stz2/stco/co64/stsc/stss/trun` loops clamped by `MAX_SAMPLES` with damage notes; fragment scan bounded by `MAX_FRAGMENT_SCAN_BOXES` (717, 1778, 2167).
- **moov-at-end IS handled by code** (not execution-proven): `open()`'s scan (704–759) skips `mdat/free/skip/wide/pdin/sidx` until it finds `moov` ("moov may trail mdat", line 703) — but the executed corpus file has moov FIRST (od-verified), and no unit test in mp4/tests.rs builds a trailing-moov fixture (`test_support.rs:310` always builds `ftyp + moov + mdat`). Code-read confidence only.
- **Fragmented MP4 IS implemented** (moof/traf/tfhd/tfdt/trun/trex defaults, default-base-is-moof with carry-forward damage note, tfdt ladder, sync from sample flags, bounded moof-scan seek) and unit-tested (`fragmented_demuxes_moofs_with_tfdt_ladder`, `fragmented_two_tracks_per_moof`, `fragmented_seek_scans_moofs`, fuzz-lite fragment mutations in mp4/tests.rs). Caveats: F-005/F-007 below.
- **Damage accounting is real and mostly honest**: 24 distinct damage reasons surfaced as `Recovered` events; typed refusals (`no_moov`, `no_sample_table`, `oversized_box`, `truncated`, `scan_budget_exhausted`) for the structural lies; sticky-fail state machine (690) replays errors forever after — EOF/flush state machine is coherent (`eos` latch, `pending` queue, seek re-arms `eos=false`).
- **Pool safety verified against the actual implementation**: `PayloadSlots::fill` (avi/riff.rs:588) only recycles arenas via `Arc::get_mut` (line 617) — pinned arenas are never reused, and overflow spills to a fresh buffer; the 2×16 MiB bound is respected lazily. No aliasing corruption is possible through this file's pool usage.

## 3. Findings

Severity scale: S0 = memory-unsafety/crash/corruption; S1 = wrong output or unbounded resource burn reachable from shipped entry point; S2 = silent data loss / wrong units / bounded-but-severe DoS / broken advertised operation; S3 = doc-vs-code lies, rare-shape misparses, stub seams, perf deformities; S4 = naming/cosmetic lies.

---

### F-001 [S1] — `start()` track list and minted packets live in two different TrackId spaces; hidden chapter traks desynchronize them
- **Where:** crates/demux/src/mp4/mod.rs:833-839 (list side), :1882 (packet side), :301-304 (the doc that states the violated invariant)
- **Code (verified `sed -n '833,840p'`):**
```rust
        let out: Vec<Track> = self
            .tracks
            .iter()
            .enumerate()
            .filter(|(_, t)| !t.is_chapter)
            .map(|(ordinal, t)| self.track_record(ordinal, t))
```
and (`sed -n '1882p'`):
```rust
            track: TrackId::new(self.exposed[ti]),
```
- **What's wrong:** `enumerate()` runs *before* `filter`, so `ordinal` is the **raw track index** (chapter traks included); `track_record` (869) mints `TrackId::new(ordinal as u32)` (885). Packets are minted with `self.exposed[ti]` — the ordinal *excluding* chapter traks. The struct doc at 301-304 says `exposed` exists precisely so "packets match the `start()` list even when hidden traks leave gaps" — the start list never consults it. Whenever a chapter trak precedes or interleaves media traks (e.g. tracks = `[chapter, audio, cover]`: list ids `{1,2}`, packet ids `{0,1}`), packets carry TrackIds that do not exist in the `start()` list, and list ids that no packet will ever carry. The engine/consumer that routes packets by TrackId cannot match them to tracks; `sample_entry_bytes` (669) indexes `tracks` by the *list* id — the two lookups disagree.
- **Why it matters:** every packet after a chapter-prefixed trak is unroutable (or routed to the wrong track); the cover-packet lane in apps/cli/containers/mp4.rs `harvest_facts` builds facts on the list ids, so the mismatch lands in shipped code.
- **What correct looks like:** `track_record` should stamp `self.exposed[ordinal]` (or the enumerate should count only exposed tracks); `sample_entry_bytes` should use the same space.
- **Confidence/Verification:** read + line-verified + logic-traced. The existing test `cover_and_chapters_keep_packet_handles_aligned_with_tracks` (tests.rs:377) builds exactly this shape (chapter trak between audio and cover) and passes **only because it asserts `packets.iter().all(|p| p.track.handle() <= 1)` and codec names, never `tracks[i].id == packets[j].track`** — the test asserts one side of the invariant, not the match. UNVERIFIED at runtime (no cargo per orders) but the arithmetic is mechanical.

### F-002 [S1] — Backward progressive seek returns a landing it does not deliver: `order_cursor` is never reset
- **Where:** crates/demux/src/mp4/mod.rs:1571-1577 (emission), :2129-2141 (seek commit), field at :310 (grep proof: `order_cursor` appears only at 310, 627, 1571, 1572, 1573 — no write in `seek_progressive`)
- **Code (verified `sed -n '1569,1580p'`):**
```rust
    fn step_progressive(&mut self) -> Result<DemuxEvent, EngineError> {
        let len = self.reader.length();
        while self.order_cursor < self.order.len() {
            let (_, ti, si) = self.order[self.order_cursor];
            self.order_cursor += 1;
            let track = &self.tracks[ti];
            if si < track.next {
                continue; // pre-seek sample — skipped by the cursor
```
- **What's wrong:** `seek_progressive` (2092) re-arms per-track `t.next` cursors (2129-2140) but leaves `order_cursor` where playback left it. `order` is sorted by file offset and consumed monotonically. After consuming N entries and seeking *backward* to sample k < N, `step_progressive` resumes at `order[N]` — entries `[k..N]` are permanently unreachable; the `si < track.next` skip clause only filters *forward*. The seek call nevertheless returns `TimeSpan{ start: land_pts }` (2144-2147) — the caller is told it landed on the keyframe while delivery resumes from the old position.
- **Why it matters:** seek-backward-after-playback is the resume use case; the returned TimeSpan is a lie and the media silently continues from the pre-seek position. Forward seeks work (the skip clause handles them), which is why `progressive_seek_lands_on_keyframe` (tests.rs:416, two forward seeks before any drain) never catches it.
- **What correct looks like:** reset `order_cursor = 0` in the seek commit (the `si < track.next` clause then does exactly what its comment says), or binary-search `order` for the first entry whose (ti,si) satisfies the landing.
- **Confidence/Verification:** read + grep proof of missing reset + control-flow trace. UNVERIFIED at runtime (no cargo).

### F-003 [S1] — `stts`/`ctts` loops iterate the full file-declared count with no body-length or ceiling clamp: CPU burn decoupled from input size
- **Where:** crates/demux/src/mp4/mod.rs:1253-1266 (stts), :1314-1332 (ctts)
- **Code (verified `sed -n '1253,1266p'`):**
```rust
                b"stts" => {
                    let count = u32be(body, 4).unwrap_or(0) as usize;
                    for i in 0..count {
                        let run = u32be(body, 8 + i * 8).unwrap_or(0) as u64;
```
and (`sed -n '1317p'` inside ctts, same shape):
```rust
                    for i in 0..count {
```
- **What's wrong:** every sibling table (`stsz/stz2/stco/co64/stsc/stss/trun`) clamps its loop with `count.min(MAX_SAMPLES)`; `stts` and `ctts` do not — only the *pushes* are clamped via `take`. `count` is a raw u32 from a 12-byte box: a truncated/lying body makes `u32be` return `None → 0`, so each iteration is cheap — but the loop still runs `count` times. One 12-byte `stts` box declaring `count = 0xFFFF_FFFF` is ~4.3·10⁹ iterations; a 16 MiB `moov` fits ~1.4 M such boxes (`8 + 16` bytes each are not required — the arm runs per box regardless of body), i.e. effectively unbounded CPU with bounded input. `ctts` repeats the same defect in the same `moov`.
- **Why it matters:** hostile (or merely corrupt) file → minutes-to-days of un-interruptible CPU inside `start()`, before a single typed error. The house rule ("declared sizes are ceiling-checked before they are trusted") is honored by the sibling tables and violated by these two.
- **What correct looks like:** `for i in 0..count.min(MAX_SAMPLES)` plus a damage note (the `stts-count-over-ceiling` note already exists for the run-expansion path); ideally also clamp to `body.len()/8`.
- **Confidence/Verification:** read + line-verified; iteration count arithmetic is mechanical. UNVERIFIED at runtime.

### F-004 [S2] — Per-chunk stsc lookup is O(chunks × stsc runs): quadratic burn on a hostile-but-legal pair of tables
- **Where:** crates/demux/src/mp4/mod.rs:1351-1359
- **Code (verified `sed -n '1351,1360p'`):**
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
- **What's wrong:** for each of up to `MAX_SAMPLES` (1 Mi) chunk offsets, the inner loop re-scans the whole `stsc` run list (also up to 1 Mi entries within the 16 MiB moov). Worst case ~3.5·10¹¹ iterations ≈ tens of minutes of CPU inside `start()`. A binary search (or precomputed run boundaries at parse time) is the standard fix.
- **Why it matters:** legal-file shapes reach it; the ceiling machinery that bounds everything else does not bound this loop's *product*.
- **Confidence/Verification:** read + line-verified. UNVERIFIED at runtime.

### F-005 [S2] — Mixed progressive+fragmented file silently drops every progressive sample (contradicts the module's own damage-note promise)
- **Where:** crates/demux/src/mp4/mod.rs:779-796 (decision), :798-813 (order build gated on `!self.fragmented`), :2032-2036 (dispatch never calls `step_progressive` when fragmented)
- **Code (verified `sed -n '779,796p'`):**
```rust
        let any_samples = self.tracks.iter().any(|t| !t.samples.is_empty());
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
            return Err(error::no_sample_table(moov_start));
        }
```
- **What's wrong:** a moov carrying *both* filled `stbl` sample tables *and* `mvex/trex` (progressive-then-fragmented files, a real ffmpeg-produceable shape) sets `fragmented = true` with `any_samples = true`; the mixed-tracks damage note only fires when `!any_samples`. The `order` table is never built (798: `if !self.fragmented`) and `next_event` (2032) dispatches exclusively to `step_fragment` — the progressive samples of those tracks are never emitted and never damage-noted. The module doc (lines 20-44) explicitly promises "a damage-noted gap, not a silent lie"; this path is the silent lie.
- **Why it matters:** silent data loss on a legal shape; contradicts the file's central honesty claim.
- **What correct looks like:** note `mixed-progressive-fragment-tracks` whenever `any_samples && fragmented` (or walk both engines).
- **Confidence/Verification:** read + line-verified control flow. UNVERIFIED at runtime.

### F-006 [S2] — `elst` empty-edit delay stored in movie timescale, applied as media timescale ticks
- **Where:** crates/demux/src/mp4/mod.rs:1090-1094 (capture), :224 (field doc claiming "media ticks"), :866-869 (mint), :2099-2103 (seek), :1907-1912 (extent)
- **Code (verified `sed -n '1080,1095p'`, the elst arm):**
```rust
                            if media == -1 {
                                empty_edit_delay = seg_dur;
                            } else if media > 0 {
                                media_time = media as u64;
                            }
```
and (`sed -n '1866,1869p'`):
```rust
        let dts_ticks = (s.dts as i64).saturating_sub(t.media_time as i64);
        let pts_ticks = dts_ticks
            .saturating_add(s.cts)
            .saturating_add(t.empty_edit_delay as i64);
```
- **What's wrong:** per ISO 14496-12, `elst` `segment_duration` is in **movie** timescale, `media_time` in **media** timescale. `TrackState.empty_edit_delay`'s own doc (224) says "media ticks", but the captured value is raw movie ticks. `mint_packet` adds it to a media-tick ladder under `t.base` (media timescale). With movie_ts = 1000 and media_ts = 48000, a 100 ms empty edit becomes 100/48000 s ≈ 2.08 ms — wrong by a factor of 48. It also pollutes `track_record`'s span start (870), `extent_nanos` (1907-1912: `empty_edit_delay.saturating_sub(media_time).saturating_add(media_duration)` mixes the two scales in one expression) and thus the seek range check (2071-2076). `media_time` itself is correctly media ticks — the bug is one-sided.
- **Why it matters:** every gapless-AAC / delayed-start file with movie_ts ≠ media_ts gets wrong PTS, wrong track span, wrong extent, and potentially false `seek_out_of_range` refusals.
- **What correct looks like:** `delay_media_ticks = seg_dur * timescale / movie_timescale` at capture (ffmpeg's mov.c does exactly this scaling).
- **Confidence/Verification:** read + line-verified + spec reasoning. UNVERIFIED at runtime (the `elst_empty_edit_delays_and_media_time_trims` test uses matching timescales, so it cannot catch the scaling).

### F-007 [S2] — Fragmented seek scans from the *current* `frag_pos`: backward fragment seek is structurally impossible
- **Where:** crates/demux/src/mp4/mod.rs:2164-2165 (also 1777 for playback)
- **Code (verified `sed -n '2163,2166p'`):**
```rust
        let target_ticks = nanos_to_ticks(target_nanos, timescale);
        let mut pos = self.frag_pos.unwrap_or(len);
        let mut best: Option<(u64, u64)> = None; // (moof offset, land dts)
```
- **What's wrong:** the moof scan starts at the live playback position. A backward target is never matched against earlier moofs — `best` stays `None` → `seek_out_of_range` (2212-2214), or, worse, a later moof's sync ≤ target is impossible so it refuses; a forward seek after a backward one is fine but the "seek to 0" case after playing fragment N is a refusal. The doc's own claim (55-60) is moof-scan = bounded linear; the implementation makes the linear window forward-only.
- **What correct looks like:** scan from `moov_end` (fragment start) every seek, as `step_fragment`'s initial position does — or from the computed landing via sidx (the recorded v0 gap).
- **Confidence/Verification:** read + line-verified. UNVERIFIED at runtime.

### F-008 [S3] — `stz2` field_size=4 packing misread as one byte per sample
- **Where:** crates/demux/src/mp4/mod.rs:1238-1251
- **Code (verified `sed -n '1238,1252p'`):**
```rust
                    for i in 0..count.min(MAX_SAMPLES) {
                        let v = if field == 16 {
                            u64::from(u16be(body, 12 + i * 2).unwrap_or(0))
                        } else {
                            u64::from(body.get(12 + i).copied().unwrap_or(0))
                        };
```
- **What's wrong:** ISO 14496-12 §8.5.2 packs two 4-bit samples per byte; this reads one byte per sample for every `field != 16`. A legal `stz2` with `field_size = 4` yields wrong sizes and half-consumed table. Also: field_size values outside {4,8,16} are silently treated as 8 instead of refused (ffmpeg validates the trinity).
- **Confidence/Verification:** read + spec knowledge. UNVERIFIED at runtime.

### F-009 [S3] — Seek's `pts_of` omits the composition offset while claiming "exactly what mint_packet stamps"
- **Where:** crates/demux/src/mp4/mod.rs:2087-2091 (doc claim), :2099-2103 (implementation), :1866-1869 (the real PTS)
- **Code (verified `sed -n '2087,2103p'`):**
```rust
    /// Progressive seek: reference-track keyframe walk-back, then
    /// every track's first sample ≥ the landing. The landing is in
    /// the emitted (movie-timeline PTS) domain — exactly what
    /// `mint_packet` stamps, so a caller re-targeting the landing
    /// never chases a stamp the demuxer will not produce.
```
vs. the same span's `pts_of = |t: &TrackState, s: &Sample| s.dts.saturating_sub(t.media_time).saturating_add(t.empty_edit_delay)` — no `+ s.cts`, which `mint_packet` adds. For B-frame streams (ctts > 0) the landing and the first delivered PTS differ by the max composition offset; a caller re-targeting the returned stamp chases a stamp the demuxer will not produce — the exact failure the doc paragraph forbids.
- **Confidence/Verification:** read + line-verified.

### F-010 [S3] — "one sample at a time" is false for fragments: `step_fragment` materializes every sample of a moof eagerly
- **Where:** crates/demux/src/mp4/mod.rs:33-35 (doc), :1808-1838 (implementation)
- **Code (verified `sed -n '33,35p'`):**
```rust
//!   read whole: `next_event` pulls exactly one sample's bytes
//!   through the pooled [`PayloadSlots`] arena (2 × 16 MiB ceiling,
//!   the house zero-copy scheme).
```
vs. the moof loop that fills and mints *every* sample of the fragment into `pending` before returning the first. With 2 pool slots pinned by the queue, sample 3..N of every moof spills to a fresh `vec![0u8; want]` (riff.rs:660) — one allocation + full copy per sample, and transient memory equal to the moof's payload. Correct behavior, dishonest doc.
- **Confidence/Verification:** read + line-verified + PayloadSlots read (riff.rs:588-671).

### F-011 [S3] — `discontinuity` parameter is a stub seam: never `true` at any call site
- **Where:** crates/demux/src/mp4/mod.rs:1864 (signature), :1888 (pass-through), call sites :1511, :1607, :1831 (all `false`)
- **Code (verified `rg "mint_packet\("`):**
```rust
                    let packet = self.mint_packet(ti, &sample, payload, false);
```
(all three call sites; grep found no `true`).
- **What's wrong:** post-seek packets — exactly where a decoder needs a flush/discontinuity signal — are minted with `discontinuity: false`. The parameter exists to satisfy the Packet shape, not to carry truth.
- **Confidence/Verification:** read + grep proof.

### F-012 [S3] — `MAX_BOX_DEPTH` and `error::depth_exceeded` are dead; the module doc claims a depth guard that does not exist
- **Where:** crates/demux/src/mp4/mod.rs:126 (const), :29-31 (doc claim); crates/demux/src/mp4/error.rs:81 (`pub fn depth_exceeded`, zero callers — grep over all crates returns only its definition)
- **Code (verified `sed -n '29,31p'` of the doc):**
```rust
//!   walked in memory with a depth guard ([`MAX_BOX_DEPTH`]) and a
//!   child>parent check on every level.
```
- **What's wrong:** the walk is flat per level (iter_boxes is non-recursive; callers descend manually), so no depth guard is *needed* — but the doc promises one and the const + typed error vocabulary ship unused. A hostile reader cannot find the guard because there is none; a reviewer trusting the doc will believe recursion is bounded.
- **Confidence/Verification:** read + rg proof (const referenced only by its own doc-link; `depth_exceeded` has zero call sites in the workspace).

### F-013 [S3] — Unsorted `stsc` silently mis-maps every chunk after the inversion point
- **Where:** crates/demux/src/mp4/mod.rs:1354-1359 (the last-`first<=chunk_1`-wins loop)
- **What's wrong:** the loop takes the LAST run whose `first_chunk <= chunk`, which is only correct for ascending `first_chunk`. An unsorted table (no monotonicity validation, no damage note) makes later smaller-`first` entries override earlier ones: silent wrong `samples_per_chunk`, wrong offsets downstream. Contrast with `stss`, which *is* validated (sorted+deduped, 1306-1311) with an explanatory comment.
- **Confidence/Verification:** read + line-verified.

### F-014 [S3] — A track with stco-but-empty stts/stsz (or missing stsc) parses "successfully" with zero samples and zero damage notes
- **Where:** crates/demux/src/mp4/mod.rs:1343-1344
- **Code (verified read):**
```rust
        let count = sizes.len().min(deltas.len());
        if !offsets.is_empty() && count > 0 {
```
- **What's wrong:** `stsc` absent → `per = 0` (1354) → zero samples pushed, no note; `stts`/`stsz` truncated-to-empty → `count = 0` → skipped, no note. In a multi-track progressive file that track is a silent black hole of zero packets (single-track files hit the `no_sample_table` refusal at 795, which is why tests pass). The "stsc-stsz-mismatch-short" note (1419-1421) only covers the partial case.
- **Confidence/Verification:** read + line-verified.

### F-015 [S3] — `8 + entry_size` wraps on 32-bit targets, bypassing the stsd entry bound check
- **Where:** crates/demux/src/mp4/mod.rs:1162-1165
- **Code (verified `sed -n '1160,1165p'`):**
```rust
        if entry_size < 8 || 8 + entry_size > body.len() {
            return Err(error::bad_box(stsd.body.start as u64));
        }
        let entry = body.get(16..8 + entry_size).unwrap_or(&[]);
```
- **What's wrong:** `entry_size` is a u32 cast to usize; on 32-bit targets `8 + entry_size` overflows (panic in debug, wrap in release) — a wrapped small value passes the guard and `body.get(16..small)` yields an empty/short entry silently. 64-bit builds are unaffected. Correct form: `entry_size.checked_add(8).map_or(true, |e| e > body.len())`.
- **Confidence/Verification:** read + arithmetic; platform-dependent. UNVERIFIED on a 32-bit target.

### F-016 [S3] — The dual non-sync bit heuristic collides with `sample_degradation_priority` bits
- **Where:** crates/demux/src/mp4/mod.rs:142-146 (the admitted heuristic), :1760 (the check)
- **Code (verified `sed -n '1760p'`):**
```rust
                                sync: sflags & SAMPLE_IS_NON_SYNC == 0 && sflags & 0x1_0000 == 0,
```
- **What's wrong:** bits 0-15 of trun sample flags are degradation priority; a priority value with bit 15 set (0x8000) marks a sync sample as non-sync. The file's own comment concedes "degradation-priority high bits are effectively unused" — an assumption, not a guarantee; the cost is mis-keyed seeks (over-walking back), not corruption.
- **Confidence/Verification:** read + line-verified.

### F-017 [S3] — `any_top_box` swallows every error as `false`: fragment detection lies about the file
- **Where:** crates/demux/src/mp4/mod.rs:861-862
- **Code (verified `sed -n '858,866p'`):**
```rust
                Ok(None) => return false,
                Err(_) => return false,
```
- **What's wrong:** a moof-less-track file whose post-moov top level contains one unreadable box header (source error or mid-header cut) reports `any_top_box("moof") == false` → `no_sample_table` refusal naming the moov, not the real damage. A probe-shaped check used as a structural oracle.
- **Confidence/Verification:** read + line-verified.

### F-018 [S3] — `order` vector memory is capped per track, not per file
- **Where:** crates/demux/src/mp4/mod.rs:798-813
- **What's wrong:** each track contributes up to `MAX_SAMPLES` (1 Mi) entries × 24 B; a 10-track hostile file materializes ~240 MiB of `(u64,usize,usize)` tuples on top of the per-track `Sample` tables (48 B × 1 Mi × 10 = another ~480 MiB — that one is the `MAX_SAMPLES` doc's own arithmetic). The ceiling doc (119-122) computes the per-track 48 MiB figure but never the cross-track sum.
- **Confidence/Verification:** read + arithmetic.

### F-019 [S3] — trex-present-but-no-moof file yields a silent empty stream
- **Where:** crates/demux/src/mp4/mod.rs:781-793 (fragmented set, no error), :1777-1860 (`step_fragment` finds nothing → `eos`)
- **What's wrong:** `fragmented` is set by trex presence alone; if the file then contains no moof, `next_event` walks the scan loop, finds nothing, and returns `EndOfStream` with zero packets and zero damage notes — an empty "successful" playback for a structurally-lying file. Single-track zero-table files get the typed refusal; this shape gets silence.
- **Confidence/Verification:** read + line-verified.

### F-020 [S4] — `error::not_started` is used for two opposite conditions
- **Where:** crates/demux/src/mp4/mod.rs:2009-2017 (`if self.started` → `not_started`), :2020-2022 (`if !self.started` → `not_started`); error.rs:142-149 (single code `demux.mp4.not-started`)
- **Code (verified `sed -n '2009,2017p'`):**
```rust
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        if self.started {
            return Err(error::not_started());
        }
```
- **What's wrong:** calling `start()` twice returns an error whose code says "not started" — a naming lie; the CLI wrapper (containers/mp4.rs `StartedMp4`) works around it by caching and its comment calls it "the deliberate second-call refusal (`demux.mp4.not-started`)" — the workaround documents the confusion rather than fixing the name.
- **Confidence/Verification:** read + line-verified.

### F-021 [S4] — `oversized_box` ignores its `_kind` parameter
- **Where:** crates/demux/src/mp4/error.rs:95 (`pub fn oversized_box(_kind: &[u8; 4], ...)`) called at mod.rs:768 with `b"moov"`.
- **What's wrong:** the caller dutifully passes which box was oversized; the constructor drops it — the error cannot name the offender (moof over-ceiling at 1790-1791 doesn't even route here; it damage-notes instead, a second inconsistency).
- **Confidence/Verification:** read.

### F-022 [S4] — `ftyp` brand list read capped at 64 bytes without a note; probe doc omits the `skip` arm it implements
- **Where:** crates/demux/src/mp4/mod.rs:722-739 (`.min(64)`), :148-152 (doc listing "moov/wide/free" but the code at 173 also matches `b"skip"`).
- **What's wrong:** compatible brands beyond 15 entries are silently truncated (observability only); the doc/probe mismatch is cosmetic but is a doc-vs-code divergence in the very first function.
- **Confidence/Verification:** read + line-verified.

### F-023 [S4] — Language plumbing swallows at two layers
- **Where:** crates/demux/src/mp4/mod.rs:535-539 (`unpack_language` emits chars beyond 'z' for 5-bit groups > 26), :880-883 (`LanguageTag::parse(tag).ok()` discards the parse error → `language: None`).
- **What's wrong:** a hostile mdhd language field becomes garbage chars, then a silent `None`. Cosmetic metadata path only.
- **Confidence/Verification:** read.

### F-024 [S4] — Damage-note byte counts carry box counts
- **Where:** crates/demux/src/mp4/mod.rs:707, 763-764 (`unknown_top` counts *boxes* but is noted through the `(bytes, reason)` channel surfaced as `Recovered { skipped: bytes }`).
- **What's wrong:** the `Recovered` event's `skipped` field semantically counts bytes; this path reports box counts — the observability consumer reads a lie of up to ~8×.
- **Confidence/Verification:** read.

## 4. Per-function table

| Function | Lines | Verdict | Sev | Notes |
|---|---|---|---|---|
| `probe` | 154-174 | REAL | — | Brand scan incl. compatible list; qt `moov/wide/free/skip` heads; <8 bytes refuse. Doc omits `skip` arm (F-022). |
| `Chapter` (type) | 178-184 | REAL | — | Plain record. |
| `Sample` (type) | 186-200 | REAL | — | offset/size/dts/cts/dur/sync; i64 cts correct for signed ctts. |
| `TrexDefaults` (type) | 204-209 | REAL | — | duration/size/flags. |
| `TrackState` (type) | 211-241 | REAL | — | `empty_edit_delay` doc says media ticks but holds movie ticks (F-006). |
| `StickyShape::of` / `mint` | 252-262 | REAL | — | Replayable error shape; used by `fail`. |
| `u32be` / `u16be` / `u64be` / `i64be` | 318-340 | REAL | — | Range-checked BE readers; `i64be` via u64→i64 bit pattern (correct for signed fields). |
| `iter_boxes` | 345-384 | REAL | — | Truncation/largesize<16/overrun/non-advance all typed; no depth guard despite doc (F-012); returns Err on child>parent rather than "clamping" as module doc claims (doc-vs-code, folded into F-012). |
| `read_top` | 390-433 | REAL | — | Clamped header read; size==0 → to-end; checked_add; sound. |
| `codec_of` | 438-470 | REAL | — | 30+ fourccs → registry names; unmapped → None (damage path). |
| `pcm_codec_of_ipcm` | 481-522 | REAL | — | pcmC walk via iter_boxes at anchor 28; width/endianness table; unmapped → None. Correct. |
| `kind_of` | 525-532 | REAL | — | hdlr→MediaKind; unknown → Data (edge: no-hdlr audio becomes Data; cosmetic). |
| `unpack_language` | 535-539 | REAL | S4 | 5-bit groups > 26 produce out-of-alphabet chars (F-023). |
| `oti_of_esds` | 542-570 | REAL | — | ES_Descriptor walk with expandable lengths; flag-dependent extra fields make it return None (graceful). |
| `esds_oti` | 576-599 | REAL | — | 28/76/78 anchor triple; drives mp4a→mp3 (0x6b) and mp4v→mjpeg (0x6c) quirks. |
| `Mp4Demuxer::new` | 610-631 | REAL | — | No I/O; pool 2×16MiB lazy. |
| `major_brand` / `compatible_brands` / `is_fragmented` / `chapters` / `pool_bytes` | 635-662 | REAL | — | Observability getters. |
| `sample_entry_bytes` | 669-672 | REAL | S1 | Indexes `tracks` by the *list* id — the wrong-side lookup of the two-id-space bug (F-001). |
| `note` / `surface_damage` | 675-687 | REAL | S4 | F-024 byte-count lie on the unknown-top path. |
| `fail` | 690-693 | REAL | — | Sticky shape capture. |
| `open` | 697-842 | REAL | S1/S2/S3 | Top-level scan (moov-at-end OK); ftyp 64-byte cap (F-022); fragment decision (F-005); order build (F-018); exposed mapping (F-001). Execution-proven on the corpus happy path. |
| `any_top_box` | 845-866 | REAL | S3 | Err swallow (F-017); bounded scan. |
| `track_record` | 869-910 | REAL | S1 | `TrackId::new(ordinal as u32)` with raw index — the F-001 mint side. Span start = delay−media_time (units F-006). |
| `parse_moov` | 914-999 | REAL | — | mvhd v0/v1 offsets correct; trex; udta chpl + cover; chapter marking; cover push after media. `let _ = err` swallows are damage-noted (honest). |
| `parse_udta_chpl` | 1003-1041 | REAL | — | Nero chpl layout; count≤64; u64 100ns→ns; break-on-short. |
| `parse_trak` | 1045-1426 | REAL | S1/S2/S3 | tkhd/mdhd/hdlr offsets correct; elst first-entry capture (F-006); stsz/stz2/stts/stco/co64/stsc/stss/ctts expansion (F-003, F-004, F-008, F-013, F-014, F-016); stsd entry bound (F-015); PCM chunk-collapse to 1024-packets. |
| `push_cover_track` | 1433-1480 | REAL | — | wkt 13/14 map; ceiling guards; synthetic id u32::MAX. |
| `queue_cover_packets` | 1485-1515 | REAL | — | Header-resident cover packet; past-stream → note. |
| `read_chapter_titles` | 1519-1565 | REAL | — | 4096-byte cap; 16-bit BE prefix; sort by nanos; chpl fallback sort. |
| `step_progressive` | 1569-1614 | REAL | S1 | `si < track.next` skip + untouched `order_cursor` = backward-seek lie (F-002); typed truncation on short reads; execution-proven. |
| `parse_moof` | 1621-1773 | REAL | S3 | tfhd flags/base-offset/trex defaults; tfdt v0/v1; trun flags 0x1/0x4/0x100/0x200/0x400/0x800 all present and correct; count clamped; negative offsets clamped (min u64::MAX/2) — damage-noted downstream. Zero-size samples dropped *without* note (minor, sibling of F-014). |
| `step_fragment` | 1775-1861 | REAL | S3 | Bounded moof scan; moof read whole ≤1MiB; eager materialization (F-010); budget → typed refusal. |
| `mint_packet` | 1864-1891 | REAL | S1/S2 | `exposed[ti]` stamp (F-001); delay units (F-006); `discontinuity` never true (F-011); duration degrade via `.ok()`. |
| `seek_reference` | 1894-1899 | REAL | — | First video else first non-chapter. |
| `extent_nanos` | 1903-1915 | REAL | S2 | Units-mixing expression (F-006 consequence). |
| `cover_in_meta` | 1923-1959 | REAL | — | FullBox/plain dual anchor (4/0); ilst/covr/data; wkt read. |
| `cover_in_udta` | 1963-1969 | REAL | — | meta find + delegate. |
| `trak_chap_refs` | 1972-1992 | REAL | — | tref/chap; ≤64 ids. |
| `ticks_to_nanos` | 1995-2001 | REAL | — | u128 intermediate, saturating, zero-guard. |
| `nanos_to_ticks` | 2004-2006 | REAL | — | u128 floor, saturating; timescale 0 → 0 (no div-by-zero possible: multiply). |
| `start` | 2009-2017 | REAL | S4 | not_started for the already-started case (F-020); sticky-fail wrap. |
| `next_event` | 2019-2037 | REAL | — | Sticky → eos → pending → step dispatch; coherent state machine. |
| `seekability` | 2039-2048 | REAL | — | Fast{indexed} / SlowLinear; honest per doc (the "index" is a linear walk — generous wording, folded into F-009 notes). |
| `seek` | 2050-2083 | REAL | — | Negative/missing target guards; extent check (F-006 consequence); dispatch. |
| `seek_progressive` | 2092-2148 | REAL | S1/S3 | Walk-back correct; `order_cursor` never reset (F-002); pts_of omits cts (F-009); O(n) linear landing scan. |
| `seek_fragment` | 2152-2224 | REAL | S2 | Forward-only scan (F-007); whole moof re-delivery from landing moof's first sample (granularity note); typed budget refusal. |

## 5. Chunk notes

**Chunk 1 (lines 1-912, box/tree walk + ftyp/moov/mvhd):** Header/doc block is unusually honest about v0 limits (sidx skipped, first-elst-only, cmov refused — `cmov` is in the typed-refusal vocabulary via bad_box? — actually no cmov arm exists; the doc claims "cmov refused typed" — an unknown top-level box is skipped as `unknown-top-box-skipped` damage, and a cmov trak would fail stsd/codec mapping → damage-noted refusal. The "typed" claim is soft-true via unknown-entry, not a dedicated arm — noted for W05-A02). Readers are range-safe; `iter_boxes`/`read_top` arithmetic is `checked_*` throughout; no panics. `open()`'s scan loop is bounded (65536) and non-spinning. The module-doc "clamped to parent ∩ stream" claim is not what `iter_boxes` does (it refuses on overrun) — refusing is safer, but the doc lies about the mechanism (F-012 family). ftyp parsing truncates brands at 64 bytes. `probe` accepts moov/wide/free/skip heads — QuickTime legacy.

**Chunk 2 (lines 914-1565, track/sample tables):** tkhd/mdhd/hdlr/elst field offsets all verified correct against ISO 14496-12 (v0/v1 both). `stsd` entry-size guard has the 32-bit wrap (F-015). The stbl expansion is where the three S1/S2 algorithmic holes live (F-003 stts/ctts unclamped loops, F-004 stsc quadratic, F-014 zero-table silence). stss handling (absent = all-sync; sorted/deduped + binary search) matches the ffmpeg/mp4parse reading. ctts v0 sign reinterpretation (`u32be as i32`) is correct. The PCM 1024-split collapse mirrors ffmpeg's observed packetization. Chapter/cover plumbing (chpl, ilst/covr/data, tref/chap) is real and bounded. elst capture is first-entry-only with a damage note for multi-entry — documented honest gap.

**Chunk 3 (lines 1567-2225, read/seek/EOF + fragmentation):** `step_progressive` is the verified-working emission engine with the backward-seek defect (F-002). `step_fragment`/`parse_moof` implement a real fMP4 engine (tfhd/tfdt/trun/trex all correctly laid out — flag bits, defaults, cts sign, data_offset sign) with the eager-materialization doc lie (F-010). The EOF/sticky state machine is coherent; seek re-arms eos and clears pending (cover packets lost after seek — documented ffmpeg precedent). `seek_progressive` walk-back logic is right for forward seeks; `seek_fragment` is forward-only (F-007). Time conversion helpers are saturating and zero-safe.

## 6. Test honesty (cross-reference only — tests.rs is W05-A02's file)

Unit tests exist in crates/demux/src/mp4/tests.rs (51 test fns, 1111 LOC) covering: probe, single/two-track progressive order, ctts, elst (matching timescales — cannot catch F-006), absent-stss, fragmented moof/tfdt, chapters, covers (incl. the too-weak alignment test at 377 — see F-001), forward progressive seek, forward fragment seek, typed refusals, ceiling damage notes, ipcm/fpcm, stz2? (**no stz2 test found in the fn list**), stss unsorted, fuzz-lite mutations. 

**Functions with NO test coverage in tests.rs (from the fn list + grep):** `stz2` arm (no test), backward seek after drain (no test — F-002 untested), backward fragment seek (no test — F-007 untested), `mixed-progressive-fragment-tracks` path with real samples (F-005 untested), `esds_oti` 0x6b/0x6c quirk lanes (no test fn named for it in tests.rs; may live in corpus tests — cross-ref W05-A02), moov-at-end / trailing-moov fixture (none — test_support.rs:310 always `ftyp+moov+mdat`), `stco`-with-no-`stsc` (no test), `unpack_language` (no test), `extent_nanos`/`seek` range refusals beyond the one out-of-range assert, `pool_bytes` observable, sidx-located-and-skipped (doc claim; no test). The fuzz-lite mutation suites cover panic-freedom only — they would not catch CPU-burn (F-003) or wrong-output (F-001/F-002) classes.

## 7. Doc claims touching this file

- docs/feature-matrix.toml `id = "container-mp4-mov-3gp"` (lines ~142-152): description "Demux ISO-BMFF family: track boxes, stco/stsc sample tables, edts/elst, 3gp quirks, fragmented MP4"; evidence "crates/demux/src/mp4/** (card 5.C1; record docs/formats/container-mp4.md, 5.REC-1: 30-lane corpus_mp4 table)"; notes record the 2026-11-15 end-to-end h264+aac playback. All four description claims are backed by real code in this file (3gp: KNOWN_BRANDS + samr/sawb/s263 arms; fMP4: parse_moof/step_fragment). The matrix's "edts/elst" claim is backed but with the F-006 units defect.
- The file's own module doc (lines 1-87) claims: moof/mvex/trex fragmentation (true), elst timeline application (true w/ F-006), chapter refs + chpl (true), 3GP quirks (true), keyframe-indexed seek (true forward, broken backward — F-002/F-007), "stss absence means all-samples-sync" (true, §8.6.2), fMP4 base-offset DASH convention (true, plus the carry-forward damage note), sidx parsed-and-skipped (located: `sidx` appears in skip lists at 750/1846 — "parsed" is generous: it is *skipped*, never parsed — doc overstates), depth guard (false — F-012), "next_event pulls exactly one sample's bytes" (false for fragments — F-010), multi-edit damage note (true), cmov typed refusal (soft-true via unknown-entry damage path, no dedicated arm).
- docs/KNOWN-LIMITS.md / docs/formats/container-mp4.md: not read in this audit (out of file scope); W05-A02/lead should cross-check the record against F-005/F-006/F-007.

## 8. Dead / duplicated / unreachable code

- `MAX_BOX_DEPTH` (mod.rs:126) — dead constant, zero uses (F-012).
- `error::depth_exceeded` (error.rs:81) — dead constructor, zero callers workspace-wide.
- `StickyShape::stage` field — captured from every error but the `Demuxer` trait surface only replays via `mint()`; used, not dead.
- The `discontinuity` parameter of `mint_packet` — reachable but constant-false (F-011), a dead seam in effect.
- `track_record`'s `title: None` and `gaps: Vec::new()` — constant fields (v0 honest).
- No duplicated logic blocks found; `esds_oti`'s three-anchor loop and `cover_in_meta`'s dual-anchor loop are intentional quirk tolerance, not duplication.

## 9. Open questions for the next agent (W05-A02 — tests.rs)

1. Does any tests.rs or corpus lane build a chapter trak *before* a media trak? If yes, extend `cover_and_chapters_keep_packet_handles_aligned_with_tracks` to assert `tracks[i].id == packets[j].track` — F-001 should fail it immediately.
2. Is there any backward-seek-after-drain test anywhere (workspace)? F-002 predicts a wrong-but-green suite.
3. The `elst` test uses one timescale — add a movie_ts ≠ media_ts empty-edit lane to pin F-006.
4. Corpus `mp4_h264_aac.mp4` is moov-first (od-verified); does corpus_mp4.rs carry a trailing-moov lane (the ffmpeg default mux order)? If not, moov-at-end is untested end-to-end.
5. `stz2` has no unit test at all — the field_size=4 packing defect (F-008) is completely uncovered.
6. Does the 30-lane corpus table (per feature-matrix evidence row) include a mixed progressive+fragmented file? F-005's silent drop would show as zero packets for a table-carrying track.
7. `sidx` — the module doc says "located and skipped"; confirm no corpus lane relies on sidx-based seeking (it cannot — not implemented).
