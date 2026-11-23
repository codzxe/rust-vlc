# WAVE 05 SUMMARY — demuxers II: MP4/MOV (PRIORITY), MKV/WebM, MPEG-TS, MPEG-PS

| Field | Value |
|---|---|
| Agents | A01–A10 all complete |
| Files audited | 27 files (36 rows), 36/36 covered |
| Findings | S0:1 S1:6 S2:21 S3:59 S4:70 (157 total) |
| Citation gate | 118 checked, 111 verbatim, 6 range-offsets, 1 wrong-file (repaired), 0 fabrications |
| Tree purity | clean |

## THE MP4 PRIORITY FINDINGS
1. [S0, EXECUTED] In-session MP4 seek ALWAYS fails: Mp4Demuxer never
   implements `reanchor` — the ONLY seek entry the engine calls
   (pipeline.rs:1499). Shipped `lumen ... seek 1s` -> `TimingSeekFailed
   plugin.demux.reanchor.unsupported` while MediaInfo reports
   `seekable: true`. The fully-tested seek() is engine-unreachable.
   (W05-A01)
2. [S1, executed] `lumen open` on a 188-byte 0x47-led file CRASHES the
   shipped CLI (exit 101, index out of bounds) via TsSignature::probe
   (media-probe phase5.rs:161); triggerable over HTTP too; 187B -> typed
   error, 189B -> typed error, exactly 188 -> panic. (W05-A04 confirmed
   W04-A01)
3. [S1, executed] MKV lacing inversion at the demuxer root: frame-count
   byte read as literal frames (spec: frames-1) in Fixed/Xiph/EBML lanes;
   ffmpeg-legal blocks mis-decode silently (damage=0); 3 in-tree tests
   PIN THE BUG. (W05-A06 adjudicated W04-A06)
4. [S1, executed] MKV window-slide livelock: block size in (4MiB, 32MiB]
   hangs next_event() forever (watchdog kill exit 124). (W05-A06)
5. [S1] MP4 trex defaults read +4-skewed (fragmented MP4 broken: duration
   @16/size@20/flags@24 vs spec 12/16/20 — the crate's own builder writes
   the correct layout); stts/ctts entry-count loops uncapped (~4.3e9
   iterations from a 12-byte box = CPU DoS). (W05-A01)
6. [S1] PS MPEG-1 lane payload_len u64 underflow on crafted length
   (debug panic / release session death). (W05-A03)
7. [S2, proven with ffmpeg ground truth] elst empty-edit movie-timescale
   applied as media ticks — and the TEST enshrines the wrong units
   (1.5s ffmpeg shift asserted as 0.111s track ticks); a units-correct
   demuxer would FAIL the test. Zero version-1 box coverage (ctts/tfdt/
   elst) anywhere. (W05-A05)
8. [S2, executed] MKV BlockGroup keyframe never derived — all BlockGroup
   packets keyframe=false. (W05-A06)
9. [S2] MKV tags value budget never counts TagName/Language/IETF bytes
   (~48 GiB retainable per element); cues >65,536 entries REFUSE TO OPEN
   (doc promises SlowLinear degrade — false, no catch site). (W05-A09/A10)
10. [KEY ROOT CAUSES for the Wave-0 smoke] "voe" language:
    unpack_language (mp4/mod.rs:535-539) adds 'a' to 1-based packed
    letters instead of 'a'-1 — EVERY language is one letter off and the
    und->None check can never fire. container: None: hardcoded at
    playback-engine/src/event.rs:929 ("v0 honest unknowns") — the demuxer
    parses major_brand but nothing consumes it. (W05-A01)

## Domain verdict
MP4/MKV/TS/PS demuxers are REAL and parse the baseline sample correctly
(boxes, tracks, duration, avcC/esds ride the stsd blob; h264+aac wired)
— but the seek story is fiction (S0), fragmented-MP4 is skewed (S1),
version-1 boxes are untested territory, and three panic/livelock paths
are reachable from the shipped binary. MP4 demux verdict: REAL for
progressive open/demux; the shipped USER experience of MP4 = open+play
to null sinks only; seek impossible.

## Carried forward
- Wave 13: why CLI containers/mp4.rs:237 extracts avcC but es/h264.rs
  is unreachable; pipeline reanchor contract.
- Wave 6: h264 decode from avcC (the codec side of the MP4 story).
- Wave 16: 3 in-tree tests pinning the lacing inversion bug; elst units
  test enshrining wrong math — headline test-honesty items.
- Wave 18: matrix rows claiming MP4/MKV/TS seek or fragmented support.
