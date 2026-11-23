# AUDIT: crates/demux/src/es/mp3.rs
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-13 |
| Lines / functions | 691 / 21 (incl. Demuxer impl) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — lumen-cli `containers/mod.rs:65-66` routes probe ids "mp3"/"mp2" → `es::open_mpeg_audio` (apps/cli/src/containers/es.rs:88-95) → `RawMp3Demuxer::new/start`; in-crate: es/tests.rs, es/fuzz.rs:100, tests/es_corpus.rs:179,207 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 5 S4: 4 |
| Commands run | `cargo test -p demux --lib es::tests::mp3` → 4 passed (exit 0); `cargo test -p demux --lib es::tests::fuzz_lite` → 1 passed (exit 0); sed quote verification (exit 0) |

## 1. One-paragraph verdict
This is a real, working raw-MPEG-audio demuxer: the ISO bitrate/samplerate tables are
byte-accurate (I checked all five tables and the sample-rate rows against 11172-3/
13818-3), the frame-size arithmetic (`coeff × bitrate / rate + padding`, largest frame
2881 B ≤ 4096 pool cap) is right, Xing/Info/VBRI first-frame detection uses the correct
side-info offsets (MPEG1 17/32, MPEG2 9/17, VBRI @36), and the 4 mp3 unit tests plus
the fuzz-lite sweep execute and pass. The defects are in the recovery/ordering edges:
the resync scan forgot the 3-byte window overlap that open_scan has (straddled headers
are missed), Recovered events queued by a failing step are emitted only AFTER
EndOfStream (invisible to any consumer that stops at EOS), seek is a hard typed
refusal, and a mid-stream sample-rate change leaves PTS running in the old timebase.
No panics reachable: every arithmetic path is bounds-checked, parse_head is total.

## 2. What actually works (verified)
- `cargo test -p demux --lib es::tests::mp3` -> 4 passed (walk/sample-clock/pts, sticky
  truncation, no-sync typed, mid-stream resync). 
- `cargo test -p demux --lib es::tests::fuzz_lite` -> 512 LCG-mutation rounds, 0 panics.
- Table verification by hand: BITRATES_V1_L1/L2/L3, BITRATES_V2_L1/L2_23 and the three
  sample-rate rows match ISO 11172-3 §3.1.3 (mp3.rs:126-169). Frame math
  `144×128000/44100 = 417` matches test constant MP3_HEAD [FF FB 90 00].
- Bounds: skip_id3 trusts ≤16 MiB and errors on overrun past EOF (mp3.rs:365-374);
  frame reads are pre-checked `end <= len` before the pool fill (mp3.rs:537-542,560);
  MAX_FRAME_BYTES ceiling enforced in parse_head (mp3.rs:229-231).

## 3. Findings
F-001 [S3]
Where: crates/demux/src/es/mp3.rs:404-410 vs 444-445
Code:
```
                None => at += win.len() as u64 - 3,
```
vs resync's
```
                None => at = win_end,
```
What is wrong: open_scan advances with a 3-byte overlap so a header straddling a
window boundary is found; resync advances to `win_end` with no overlap, so a valid
frame header starting in the last 3 bytes of a 4 KiB window is missed by both the
current and the next window. Why it matters: on corrupt input the recovery path skips
one more frame (~26 ms audio at 128 kbps/44.1 kHz) than necessary, or can burn budget
toward the 4 MiB ResourceLimit error. What correct looks like: the same
`at += win.len() - 3` overlap in resync. Confidence: HIGH — Verification: read (both
loops traced byte-by-byte; no test covers a straddling resync — UNVERIFIED at runtime).

F-002 [S3]
Where: crates/demux/src/es/mp3.rs:513-521 and 457-468 vs 665-670
Code:
```
            self.pending.push_back(DemuxEvent::Recovered {
                skipped: left,
                reason: "mp3: trailing fragment",
            });
            self.walk = len;
            return Ok(false);
```
next_event:
```
        while self.pending.is_empty() {
            if self.eos || !self.step()? {
                self.eos = true;
                return Ok(DemuxEvent::EndOfStream);
            }
        }
```
What is wrong: step() pushes a final Recovered event and then returns Ok(false);
next_event returns EndOfStream first, so the Recovered event only surfaces on a call
AFTER EndOfStream. Any consumer that stops at EOS (including the crate's own drain()
helper, es/tests.rs:55-67, which breaks on EndOfStream) never sees the
trailing-fragment / lost-sync-at-EOF damage report. What correct looks like: drain
pending before returning EndOfStream. Confidence: HIGH — Verification: read.

F-003 [S3]
Where: crates/demux/src/es/mp3.rs:680-689
Code:
```
    fn seek(
        &mut self,
        target: media_types::time::TimeSpan,
    ) -> Result<media_types::time::TimeSpan, EngineError> {
        let _ = target;
        Err(mint(
```
What is wrong: seek is a hard typed refusal on a length-known seekable source where
frame-index seek is trivial (exact sample-clock arithmetic already exists). The CLI
host documents the refusal and covers only open-time start via the skip watermark
(apps/cli/src/containers/es.rs:85-95); in-session seeking of raw MP3 is impossible.
What correct looks like: a walk-to-target using the existing exact pts clock.
Confidence: HIGH — Verification: read + host doc cross-check.

F-004 [S3]
Where: crates/demux/src/es/mp3.rs:571-579
Code:
```
        let param_change =
            h.sample_rate != reference.sample_rate || h.channels != reference.channels;
        if param_change {
            self.damage.note(0, "mp3: mid-stream parameter change");
            self.pending.push_back(DemuxEvent::Discontinuity);
        }
```
What is wrong: after a mid-stream rate change the track TimeBase stays 1/old_rate and
`samples_clock` keeps adding the new frames' samples_per_frame in old-rate ticks, so
wall-clock PTS after the change is wrong (e.g. 44100→22050 halves playback speed of
the remainder). The Discontinuity event is honest but the timestamps are not.
Documented as "the ADTS precedent" (mp3.rs:22-24) — a pinned limitation, not an
accident. What correct looks like: rebase the clock at the change. Confidence: HIGH —
Verification: read (no test pins post-change PTS; UNVERIFIED at runtime).

F-005 [S3]
Where: crates/demux/src/es/mp3.rs:537-542
Code:
```
        let end = self.walk + u64::from(h.frame_size);
        if end > len {
            // A frame claim that overruns the stream: typed sticky
            // truncation (ffmpeg emits a short packet — divergence).
            return Err(self.fail(super::error::truncated(end, len)));
        }
```
What is wrong: 4–9 trailing junk bytes that happen to parse as a header (0xFF Ex …)
turn a clean EOS into a sticky Corrupt error that kills the whole stream; the
trailing-fragment tolerance path at :513 requires the tail to NOT parse. The module
doc pins this as a deliberate ffmpeg divergence (mp3.rs:33-35) and the test
mp3_truncated_final_frame_is_typed_sticky enshrines it, so it is policy — but the
policy errors out on realistically corrupt tails where resync-to-EOF (or a short
packet) would play the file. Confidence: MEDIUM — Verification: read.

F-006 [S4]
Where: crates/demux/src/es/mp3.rs:248-252
Code:
```
    let name = match layer {
        Layer::L3 => "mp3",
        Layer::L1 | Layer::L2 => "mp2",
        Layer::Reserved => "mp3",
    };
```
What is wrong: the `Layer::Reserved` arm is unreachable through parse_head (rejected
at :195) — dead defensive mapping. Confidence: HIGH — Verification: read.

F-007 [S4]
Where: crates/demux/src/es/mp3.rs:55-56
Code:
```
use crate::avi::header::DamageLog;
use crate::avi::riff::{PayloadSlots, SourceReader};
```
What is wrong: the ES (elementary-stream) module reaches into the AVI container
module for shared read/pool/damage infrastructure — inverted layering (a generic ES
demuxer depending on a specific container's guts). The demux lib.rs module map
(crates/demux/src/lib.rs:12-33) does not list `es` (nor adts/mkv/mp4/ogg/ps) at all.
Confidence: HIGH — Verification: read.

F-008 [S4]
Where: crates/demux/src/es/mp3.rs:421-425 vs 447-456
Code:
```
            let win_end = (at + 4096).min(budget_end.max(at + 1));
            let win = self.reader.read_prefix_at((win_end - at) as usize, at)?;
            if win.len() < 4 {
```
What is wrong: when 1–3 bytes remain before the recovery budget the short window
`break`s past the ResourceLimit check at :447, reporting a clean end instead of the
typed budget error; symmetrically open_scan's budget (mp3.rs:385-392) effectively
ends 3 bytes early (a header must start ≤ scan_end-4). Both are edge accounting
inconsistencies, not hangs. Confidence: MEDIUM — Verification: read/trace.

F-009 [S4]
Where: crates/demux/src/es/mp3.rs:523
Code:
```
        let head = self.reader.read_prefix_at(4, self.walk)?;
```
What is wrong: per-frame 4-byte header read allocates a fresh Vec per frame
(read_prefix_at builds a Vec) — avoidable per-frame allocation in the hot loop; the
payload path is pooled but the header path is not. Confidence: HIGH — Verification:
read (read_prefix_at impl at avi/riff.rs:307-312).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| table_bitrate | 140-156 | REAL | — | ISO table lookup, index 0/15 refused; verified vs ISO |
| table_sample_rate | 158-169 | REAL | — | rows match 11172-3; index 3 refused |
| parse_head | 180-244 | REAL | — | strict validation; frame math verified; MAX_FRAME_BYTES gate |
| descriptor | 247-256 | REAL | — | Reserved arm dead (F-006) |
| mint | 279-285 | REAL | — | typed error factory |
| RawMp3Demuxer::new | 293-309 | REAL | — | rejects empty/unknown-length via SourceReader (test passes) |
| frames_demuxed | 313-315 | REAL | — | accessor |
| head | 319-321 | REAL | — | accessor used by CLI harvest |
| id3_skipped | 325-327 | REAL | — | accessor |
| damage_total | 331-333 | REAL | — | accessor |
| fail | 335-338 | REAL | — | sticky setter |
| sticky_err | 340-349 | REAL | — | None arm defensive |
| skip_id3 | 352-378 | REAL | — | syncsafe decode correct; 16 MiB ceiling; EOF-checked |
| open_scan | 383-414 | REAL | — | 3-byte overlap correct; budget edge F-008 |
| resync | 417-469 | REAL | S3 | missing overlap (F-001); tail events swallowed (F-002) |
| mint_track | 472-500 | REAL | — | timebase 1/rate, rate>0 proven |
| step | 504-593 | REAL | S3 | sticky truncation policy (F-005); rate-change PTS (F-004) |
| first_frame_is_metadata | 599-633 | REAL | — | side-info offsets correct; bounds safe |
| Demuxer::start | 637-656 | REAL | — | idempotent; sticky replay |
| Demuxer::next_event | 658-678 | REAL | S3 | post-EOS event ordering (F-002) |
| Demuxer::seek | 680-689 | REAL | S3 | typed refusal only (F-003) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mp3_walks_frames_with_exact_sample_clock | YES | pins packet count/size/pts ticks (i×1152), id3 bytes, codec name — executed, passed |
| mp3_truncated_final_frame_is_typed_sticky | YES | pins error kind Corrupt/Truncated + code + stickiness — executed |
| mp3_no_sync_is_typed_unsupported_after_scan | YES | pins code demux.es.no-sync — executed |
| mp3_midstream_garbage_resyncs_with_recovered_event | YES | pins recovered:7 event — executed |
| empty_source_refuses_at_construction (mp3 half) | YES | construction refusal — executed |
| fuzz_lite_mutations_never_panic_or_hang | partial | absence-of-panic only; a frame-skip regression in resync would NOT fail it |
Gap: no test for header-straddling resync (F-001), post-EOS event order (F-002), or
post-rate-change PTS (F-004) — all three defects are invisible to the current suite.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "tables are the published ISO tables" | mp3.rs:123-125 | REAL | hand-checked all 5 tables + rate rows |
| "largest table-derived frame is 2881 bytes" | mp3.rs:67-69 | REAL | 144×160000/8000+1 = 2881 (V2.5 L2) |
| "pts = Σ samples_per_frame on 1/sample_rate" | mp3.rs:21-22 | REAL | step():577-579 + executed test pin i×1152 |
| "bounded resync … next parseable header" | mp3.rs:31-32 | SHALLOW | bounded yes; "next parseable header" false at window straddles (F-001) |
| "mid-stream (rate) change surfaced as Discontinuity" | mp3.rs:23-24 | REAL | step():571-576 — but PTS drift untested (F-004) |
| Xing/Info/VBRI offsets per version/channels | mp3.rs:595-598 | REAL | 17/32, 9/17, VBRI@36 match ISO side-info sizes |

## 7. Dead / duplicated / unreachable code
- descriptor(Layer::Reserved) arm (F-006).
- step():531-534 second-parse Err arm: unreachable (resync only returns Some at a
  position parse_head already accepted on identical bytes).
- es/tests.rs drain() breaks on EndOfStream, so post-EOS pending events (F-002) are
  unobservable — dead event production.

## 8. Dependencies & flags
No external deps (std + workspace crates media_types/plugin_api). No feature gates.
Cross-module imports from crate::avi (F-007).

## 9. Open questions for the next agent
- Does es_corpus.rs (real fixtures) pin the Xing-skip behavior on 0139 (claimed
  mp3.rs:543-546)? Verify when auditing tests/es_corpus.rs.
- resync straddle (F-001): confirm with a targeted synthetic test if anyone owns a
  sandbox; I could not add tests (read-only repo).
