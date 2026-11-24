# AUDIT: apps/cli/src/containers/ps.rs
| Field | Value |
|---|---|
| Auditor | W13-A06 |
| Date | 2026-11-24 |
| Lines / functions | 1056 / 13 prod (+9 test helpers, 4 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: main.rs -> commands -> demux_host::open_source (demux_host.rs:362) -> open_source_with -> probe (demux_host.rs:445-447) -> containers::open (demux_host.rs:458; containers/mod.rs:48) -> "mpegps" arm (mod.rs:64-65) -> ps::open (ps.rs:139) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 3 |
| Commands run | `./target/debug/deps/lumen_cli-5f13e10be716a8b0 containers::ps --test-threads=1` -> 4 passed, 0 failed, exit 0; rg/sed/Read for callers, fixtures, sibling code |

## 1. One-paragraph verdict
Real, wired, tested MPEG-PS host. The demux crate owns all container validation
(pack/PES parse, 33-bit stamps, codec peek, SCR-clamped seek, compute-then-commit
reanchor); this host owns start-time clamping, the skip watermark, facts harvest
(no invented blob/pcm/cadence — verified in tests), and the MPEG-audio streaming
frame assembler. I executed all 4 tests from the prebuilt binary: pass. The one
real defect: the EOS flush queues tail-damage notes AFTER returning EndOfStream,
and the engine never polls past EOS — the "never silent" tail promise is dead in
the shipped path, and the TS sibling (ts.rs:529-538) proves the correct pattern
was one file away.

## 2. What actually works (verified)
— verified by `./target/debug/deps/lumen_cli-5f13e10be716a8b0 containers::ps --test-threads=1` ->
`test result: ok. 4 passed; 0 failed` (synthetic spanning-frame lane with exact
stamp ladder 9000/11351 and a 30-byte drain note; MPG lane: 32 video PES + 39
mp2 AUs + full 39-value stamp ladder + 44 928 decoded sample frames; 3 VOB
lanes with typed codec refusals; --start-time 0.6s keeps 30/36 packets, first
stamps 55 800 / 54 671). Also baseline cargo-test-workspace.txt:3044-3047 (ok x4).

## 3. Findings
F-001 [S2]
Where: apps/cli/src/containers/ps.rs:519-525
Code:
```
                DemuxEvent::EndOfStream => {
                    // 2. The flush: leftovers become tail notes
                    //    before the honest end.
                    for (_, assembler) in &mut self.assemblers {
                        assembler.flush(&mut self.queue);
                    }
                    return Ok(DemuxEvent::EndOfStream);
```
What is wrong: flush() pushes tail `Recovered` notes into self.queue, then
EndOfStream is returned while the notes are still queued. The queue is only
served at the TOP of the next call — i.e. after EOS. The shipped engine stops
polling at the first EOS (crates/playback-engine/src/pipeline.rs:2135-2139:
`if self.eof { ... return; }`, set at 2175-2181), so the tail notes are never
observed by any shipped consumer. Why it matters: the module doc promises the
opposite (ps.rs:44-47: "end-of-stream leftovers surface as a tail note — ...
never silent"); damaged_demux accounting silently undercounts exactly in the
damaged-stream case the notes exist for, and REASON_TAIL ("ps.pes.mpeg-audio-tail")
has zero test coverage (rg: only defined ps.rs:125, used ps.rs:420 — no test
asserts it). The TS sibling does it right (ts.rs:529-538: flush, then
`if let Some(event) = ... { return Ok(event); }` BEFORE `Ok(EndOfStream)`) —
this file deviates from its own stated family convention. What correct looks
like: return the first queued note first, defer EndOfStream until the queue
drains (ts.rs pattern). Confidence: HIGH — Verification: read + executed engine
gate path.

F-002 [S3]
Where: apps/cli/src/containers/ps.rs:330-339, 402-409
Code:
```
                    Resync::Found(next) => {
                        queue.push_back(DemuxEvent::Recovered {
                            skipped: next as u64,
                            reason: REASON_RESYNC,
                        });
                        at += next;
                        continue;
```
and
```
            if !crossed && at >= old_len {
                // The walk entered this PES's own bytes: it anchors
                // everything from here.
                self.anchor_pts = timestamp_of(pts);
```
What is wrong: the anchor switch only executes after a MINT. If a garbage
leftover forces a resync that jumps `at` past `old_len` (into the current PES's
own bytes), the first recovered frame is minted with the PREVIOUS PES's anchor
and sample count, violating the documented crossing rule (ps.rs:307-310: "a
frame belongs to the PES whose bytes carry its FIRST byte"). The drain/truncate
paths (L345-352) likewise never switch anchors. Why it matters: on damaged
streams the first post-resync AU is stamped from a stale anchor (error bounded
by inter-PES PTS drift, damage is noted, subsequent frames self-correct). What
correct looks like: switch anchors when the WALK POSITION crosses old_len
(prelude to mint), not only post-mint. Confidence: HIGH (traced both paths) —
Verification: read.

F-003 [S4]
Where: apps/cli/src/containers/ps.rs:416-424
Code:
```
    fn flush(&mut self, queue: &mut VecDeque<DemuxEvent>) {
        if !self.buffer.is_empty() {
            queue.push_back(DemuxEvent::Recovered {
                skipped: self.buffer.len() as u64,
                reason: REASON_TAIL,
```
What is wrong: a pending discontinuity (packet flag consumed at L318-320, held
at L289) is silently dropped if no AU mints before EOS — the flush never
surfaces it. Bounded observability loss on the same dead path as F-001.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: apps/cli/src/containers/ps.rs:393
Code:
```
                data: SharedBytes::from_vec(self.buffer[at..end].to_vec()),
```
What is wrong: per-AU heap allocation + copy, and `self.buffer.drain(..at)`
(L411) memmoves the remainder on every push — the demux crate fought a
zero-alloc war (crates/demux/tests/zero_alloc_read_chain.rs) that this host
layer re-opens for every MPEG-audio AU. Blast radius: audio path only, ~1 alloc
per ~26 ms of audio. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: apps/cli/src/containers/ps.rs:153-154
Code:
```
    let duration_us = facts.duration_us().unwrap_or(i64::MAX / 4);
    let start_us = opts.start_time_us.unwrap_or(0).clamp(0, duration_us);
```
What is wrong: if duration_us() ever returned a negative value, clamp(0, neg)
panics ("min > max"). For PS it cannot (parse_stamp returns 33-bit unsigned,
demux/src/ps/mod.rs:274-282), but the pattern is a latent panic seam copied
across hosts; harmless here. Confidence: HIGH — Verification: read + parse_stamp
checked.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open | 139-198 | REAL | — | start-time clamp + seek query + wrapper; see F-005 |
| is_mpeg_audio | 203-205 | REAL | — | mp1/mp2/mp3 split set, matches provider decoders |
| harvest_facts | 214-264 | REAL | — | no invented facts; debug_asserts only (stripped in release) |
| MpegAudioAssembler::push | 296-412 | REAL | S3 | spanning-frame + stamp arithmetic correct; F-002 stale anchor on resync; hold bounded (frame_size<=4096, mp3.rs:229) |
| MpegAudioAssembler::flush | 416-424 | REAL | S2 | dead in shipped path (F-001), drops pending discontinuity (F-003) |
| timestamp_of | 428-433 | REAL | — | trivial |
| ticks_for | 438-442 | REAL | — | u128 round-half-up, saturating; correct |
| resync_from | 456-472 | REAL | — | bounded scan; MaybeTail for last-3-byte sync |
| PsHostDemuxer::start | 494-498 | REAL | — | forwards cached list |
| PsHostDemuxer::next_event | 500-530 | REAL | S2 | interleave order correct; EOS flush ordering wrong (F-001) |
| PsHostDemuxer::seek | 532-534 | REAL | — | forwarded |
| PsHostDemuxer::seekability | 536-538 | REAL | — | forwards SlowLinear |
| PsHostDemuxer::reanchor | 540-549 | REAL | — | forwarded; demuxer marks next packet discontinuous (ps/mod.rs:1212) so post-seek AUs are flagged |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| synthetic_lane_splits_spanning_frames_and_drains_typed | YES | exact AU sizes, stamps 9000/11351, 30B drain note, seek clamp 90_000 — any assembler regression moves them |
| mpg_lane_demuxes_splits_and_decodes_mp2_exactly | YES | 32+39 packet counts, full 39-value ladder, 44_928 decoded frames; the "== ffmpeg" pins are external measurements (UNVERIFIED in sandbox, internal consistency verified by my run) |
| vob_lanes_demux_and_refuse_undecodable_codecs_typed | YES | pinned PES counts + typed refusal codes per lane |
| start_time_skips_packets_with_slow_linear_seekability | YES | kept counts 30/36 + first stamps pinned |
Missing: the flush/tail path (REASON_TAIL) — no test reaches it (F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Damage is bounded and never silent" / "leftovers surface as a tail note" | ps.rs:41 / ps.rs:45 | FALSE in shipped path (the tail-note half) | F-001: pipeline.rs:2135-2139 stops polling at first EOS; queue served only before EOS return |
| "partial frame + one PES payload (a legal hold can never exceed" | ps.rs:46 | REAL | parse_head bounds frame_size <= 4096 (mp3.rs:229); spanning hold completes at 4096+payload |
| "one PES = one" + "demuxed packet" (one PES = one demuxed packet, wrapped) | ps.rs:18-19 | REAL | next_event None arm returns PES whole (ps.rs:516); VOB lanes pin whole-PES counts |
| "The split set is exactly the decodable MPEG-audio descriptors" | ps.rs:49-50 | REAL | is_mpeg_audio ps.rs:203-205; ac3/dts/pcm_dvd refused typed in vob lane test |

## 7. Dead / duplicated / unreachable code
REASON_TAIL path (flush notes) is unreachable-in-effect from the shipped engine
(F-001). No other dead code. HOLD_CAP_BYTES (128 KiB) binds only the MaybeTail
backstop — legal streams never reach it (correct as documented).

## 8. Dependencies & flags
No feature gates. Uses demux::ps (MAX_PACKET_BYTES=64 KiB, mod.rs:102),
demux::es::mp3::parse_head (strict: free-format/reserved refused), commands,
media_types, plugin_api. All reachable in the release lane (no cfg beyond
#[cfg(test)]).

## 9. Open questions for the next agent
- demux/src/ps parse_stamp (mod.rs:270-283) does NOT validate PTS marker bits
  (0x0F pattern) — lenient stamp accepts garbage markers; demux-crate owner
  should judge.
- Who observes Recovered notes for damage_total observability besides the
  engine report? bench.rs polls `EndOfStream => break` too (same loss as F-001).
