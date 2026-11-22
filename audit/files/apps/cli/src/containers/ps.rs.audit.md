# AUDIT: apps/cli/src/containers/ps.rs

| Field | Value |
|---|---|
| Auditor | W13-A02 |
| Date | 2026-11-22 |
| Lines / functions | 1056 / 25 (19 production + 6 test helpers/tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `main.rs:19` → `run::run` → `repl_mode`/`one_shot` → `lumen_dispatch` → `CliHost::open_session` (host.rs:904) → `demux_host::open_source` → `containers::open` (containers/mod.rs:71: `"mpegps" => ps::open(stream, opts)`) → this file's `open`. |
| Findings | S0:0 S1:0 S2:1 S3:2 S4:5 |

Commands run:
- `sed -n '109,116p;519,526p' apps/cli/src/containers/ps.rs` (quote verification, exit 0)
- `rg -n "ps::open|mpegps" apps/cli/src` → wired at containers/mod.rs:71
- `rg -n "fn ticks_for|au_stamp" apps/cli/src/containers` → duplicate arithmetic in ts.rs:460 and ps.rs:438
- `rg -n -A25 "fn seek" crates/demux/src/ps/mod.rs:1116` → seek is a pure query (no position move)
- `rg -n "MAX_FRAME_BYTES" crates/demux/src/es/mp3.rs` → 4096; `parse_head` rejects frame_size < 4 (mp3.rs:229) — the zero-frame infinite-mint hazard does not exist
- `rg -n -B3 -A6 "DemuxEvent::EndOfStream" crates/playback-engine/src/pipeline.rs` → engine stops calling `next_event` after EOS (pipeline.rs:2135-2139)
- `ls crates/demux/fixtures/ps/` → all 4 cited fixtures exist (75-241 KB)

## 1. One-paragraph verdict

This is NOT a second PS demuxer — it is a host adapter over the real `demux::ps::PsDemuxer` (crates/demux), adding three things: a facts harvest, `--start-time` validation via the demuxer's pure seek query plus a skip watermark, and a streaming MPEG-audio frame assembler that re-packetizes PES payloads into one-frame-per-packet AUs for the mp1/mp2/mp3 decoder contract. The assembler is real work with measured pins (39 AUs, stamp ladder ending 136 957, 44 928 decoded sample frames) and bounded damage handling. The one genuine defect: the end-of-stream flush pushes "tail note" damage events into a queue that the engine will never read, because the wrapper returns `EndOfStream` immediately and the engine stops polling — the module's own "never silent" promise is broken for truncated files. Second-tier: the per-AU stamp arithmetic and frame-walk pattern are duplicated from the TS sibling, and demuxer-domain re-packetization lives in the app layer.

## 2. What actually works (verified)

- Wiring: `"mpegps" => ps::open(...)` at containers/mod.rs:71; the four committed fixtures exist (`ls crates/demux/fixtures/ps/` → ps-mp2-video-audio.mpg, ps-vob-ac3/dts/lpcm.vob).
- The assembler's invariants hold by construction I could check: `parse_head` (demux/es/mp3.rs:180-243) rejects free-format/reserved indices and enforces `4 <= frame_size <= 4096`, so the incomplete-frame hold is bounded (≤4096 + one PES ≤ 64 KiB) and a zero-size infinite-mint loop is impossible; `resync_from` only returns `Found` at a position where `parse_head` succeeds, so the resync loop advances monotonically (no O(n²) re-scan).
- `PsDemuxer::seek` is verified pure (crates/demux/src/ps/mod.rs:1116-1137 computes and returns a clamped TimeSpan, moves nothing) — so `open`'s discarded seek result at line 164 is validation-only, and the skip watermark is the only positioning mechanism (no double-skip).
- Not executed (no cargo): the in-file tests' pinned numbers are taken on trust of the code's assertions, corroborated by W00 baseline (1751 pass / 2 fail / 25 ignored workspace-wide).

## 3. Findings

### F-001 [S2]
Where: apps/cli/src/containers/ps.rs:519-526
Code:
```rust
                DemuxEvent::EndOfStream => {
                    // 2. The flush: leftovers become tail notes
                    //    before the honest end.
                    for (_, assembler) in &mut self.assemblers {
                        assembler.flush(&mut self.queue);
                    }
                    return Ok(DemuxEvent::EndOfStream);
                }
```
What is wrong: the flush pushes `Recovered` tail notes into `self.queue` and then `EndOfStream` is returned IMMEDIATELY — bypassing the queue-first check at the top of the loop (ps.rs:504-506). The engine stops calling `next_event` once it has seen `EndOfStream` (crates/playback-engine/src/pipeline.rs:2135-2139: `if self.eof { return; }`), so the tail notes are stranded in `self.queue` forever. The loop should `continue` after flushing so the queue drains before the terminal is returned.
Why it matters: the module's own contract (ps.rs:44-46: "end-of-stream leftovers surface as a tail note — counted, never silently dropped") is false on the engine path. A truncated PS file (audio PES cut mid-frame — the realistic damage case) loses its damage accounting: the engine's `damaged_demux`/`damaged` counters (pipeline.rs:2166-2167) never hear about the stranded bytes. Observable-data loss by omission, reachable from `lumen open file.mpg` on any truncated file.
What correct looks like: after flushing, `continue` the loop; return `EndOfStream` only when the queue is empty (the inner demuxer re-delivers EOS idempotently).
Confidence: HIGH — Verification: read both sites + engine EOS handling.

### F-002 [S3]
Where: apps/cli/src/containers/ps.rs:438-442 (vs ts.rs:460-470)
Code:
```rust
fn ticks_for(samples: u64, rate: u32) -> u64 {
    let rate = u64::from(rate.max(1));
    u64::try_from((u128::from(samples) * 90_000 + u128::from(rate / 2)) / u128::from(rate))
        .unwrap_or(u64::MAX)
}
```
What is wrong: this is a line-for-line duplicate of the arithmetic in `au_stamp` (apps/cli/src/containers/ts.rs:460-470: `(u128::from(samples_before) * 90_000 + u128::from(rate / 2)) / u128::from(rate)`), and both files implement an MPEG-audio frame walk with the same anchor+samples ladder concept. Two implementations of the same per-AU stamp law in sibling files — a divergence bug waiting to happen (fix the rounding in one, not the other).
Why it matters: §3.1 copy-paste / §3.5 — the "one implementation of the truth" standard the rest of this codebase loudly holds (host.rs:1431-1435 reuses the engine's track mapping "not re-implemented").
What correct looks like: one shared helper (e.g., in demux_host.rs or crates/demux) used by both container hosts.
Confidence: HIGH — Verification: read both.

### F-003 [S3]
Where: apps/cli/src/containers/ps.rs:270-472 (the whole assembler)
Code:
```rust
struct MpegAudioAssembler {
    /// Unconsumed payload bytes (front = the next frame boundary).
    buffer: Vec<u8>,
```
What is wrong: a streaming elementary-stream re-packetizer (frame walk, resync, hold cap, stamp synthesis) — demuxer-domain logic — lives in the application layer (apps/cli), while the PS demuxer itself and even the TS sibling's `PesAssembler` live in crates/demux (ts.rs:16, 83). Any second host (web, desktop-tauri) that wants PS audio must re-implement or depend on the CLI crate.
Why it matters: §3.5 layering violation; the module docs' justification ("the host adapts container truth to the decoder contract") is a defensible seam for the SELECTION wrapper, but a frame re-packetizer is container/codec knowledge, not host knowledge.
What correct looks like: the assembler in crates/demux (ps module), the host calling it.
Confidence: HIGH — Verification: read; crate boundary verified by module paths.
Severity: S3 (works, tested, but misplaced).

### F-004 [S4]
Where: apps/cli/src/containers/ps.rs:109-111
Code:
```rust
/// The MPEG-PS payload ceiling (the demuxer's own `MAX_PACKET_BYTES`
/// — a PES length is a u16, so no legal packet exceeds it). The
```
What is wrong: `MAX_PACKET_BYTES = 64 * 1024 = 65536` but `u16::MAX = 65535` — "no legal packet exceeds it" is off by one in the prose; also unbounded-length PES (length field 0, legal for video PS) is not addressed by the comment.
Confidence: HIGH — Verification: read + `rg MAX_PACKET_BYTES crates/demux/src/ps/mod.rs` → `64 * 1024`.

### F-005 [S4]
Where: apps/cli/src/containers/ps.rs:219-227 (and 235-239)
Code:
```rust
    debug_assert_eq!(
        descriptors.len(),
        tracks.len(),
        "stream_descriptors parallels the track list"
    );
```
What is wrong: the descriptor-parity and 90 kHz-clock invariants are pinned with `debug_assert!` only — compiled out of release builds. A demuxer regression that breaks parity would pass silently in the shipped binary.
What correct looks like: a typed error (or at least a runtime check) on the open path, since this is the wiring's own truth pin.
Confidence: HIGH — Verification: read.

### F-006 [S4]
Where: apps/cli/src/containers/ps.rs:321, 393
Code:
```rust
        self.buffer.extend_from_slice(data.as_slice());
...
                data: SharedBytes::from_vec(self.buffer[at..end].to_vec()),
```
What is wrong: every PES payload is copied into the assembler buffer and every minted AU is copied out again — two full copies per audio frame on the audio cadence path (SharedBytes exists precisely to avoid this; the copy breaks its zero-copy purpose for this family).
Why it matters: §3.3 needless copying; bounded (≤4 KiB per AU) so S4, not higher.
Confidence: HIGH — Verification: read.

### F-007 [S4]
Where: apps/cli/src/containers/ps.rs:392 (vs ts.rs:367)
Code:
```rust
                duration: Some(Duration::Known(Nanoseconds(duration_ns))),
```
What is wrong: PS minted AUs carry a computed `duration`, the TS sibling's minted AUs carry `duration: None` — the same wire object, two conventions, for the same kind of synthesized AU. Downstream consumers that key on duration will behave differently per family.
Confidence: HIGH — Verification: read both.

### F-008 [S4]
Where: apps/cli/src/containers/ps.rs:164
Code:
```rust
        demuxer.seek(target).map_err(|err| wire_error(&err))?;
```
What is wrong: the returned (clamped) landing TimeSpan is discarded. It is harmless today (verified: the seek is pure and the watermark uses the pre-clamped-but-duration-clamped `start_us`), but the call's return value being dead invites the reader to think the landing is used; a `let _ =`-style comment or use of the clamp result would be honest.
Confidence: HIGH — Verification: read + PsDemuxer::seek purity verified (mod.rs:1116-1137).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open` | 139-198 | REAL | — | constructs demuxer, pre-pass, harvest, start-time clamp + pure-seek validation, assembler set, wrapper. `duration_us` fallback `i64::MAX/4` when no extent (documented) |
| `is_mpeg_audio` | 203-205 | REAL | — | mp1/mp2/mp3 split set |
| `harvest_facts` | 214-264 | REAL | S4 | debug_assert-only pins (F-005); harvests only container-declared truth (no pcm/blob/cadence/dimensions — honest) |
| `MpegAudioAssembler::push` | 296-412 | REAL | S4 | crossing rule + anchor re-anchor correct per docs; bounded holds (≤4096+PES); alloc copies (F-006); `pending_discontinuity` latched to first minted AU |
| `MpegAudioAssembler::flush` | 416-424 | REAL | S2 | correct in isolation; stranded by the caller (F-001); does not reset anchors (irrelevant post-EOS) |
| `timestamp_of` | 428-433 | REAL | — | |
| `ticks_for` | 438-442 | REAL | S3 | duplicate of ts.rs `au_stamp` arithmetic (F-002); u128, saturating, round-half-up |
| `Resync` enum | 447-454 | REAL | — | |
| `resync_from` | 456-472 | REAL | — | linear, Found only at parse_head-OK positions (no re-scan loops); MaybeTail for <4-byte candidates |
| `PsHostDemuxer::start` | 494-498 | REAL | — | forwards (idempotent inner) |
| `PsHostDemuxer::next_event` | 500-530 | REAL | S2 | queue-first interleave correct; EOS flush strands notes (F-001) |
| `PsHostDemuxer::seek` | 532-534 | REAL | — | forwards pure query |
| `PsHostDemuxer::seekability` | 536-538 | REAL | — | forwards SlowLinear |
| `PsHostDemuxer::reanchor` | 540-549 | REAL | — | forwards; assembler state intentionally survives (documented: mid-frame cut holds like a boundary cut) |
| test `CountingSink::accept` | 571-580 | REAL | — | counts sample frames |
| test `fixture` | 584-588 | REAL | — | paths verified to exist |
| test `open` | 592-597 | REAL | — | through the REAL open path (good) |
| test `decoder_for` | 601-612 | REAL | — | typed refusals returned |
| test `mp2_frame_208` | 619-626 | REAL | — | framing-only synthetic (nothing decodes it — honest comment) |
| test `TempDir::new/write/Drop` | 631-648 | REAL | — | self-cleaning |
| test `synthetic_lane_splits_spanning_frames_and_drains_typed` | 657-753 | REAL | — | strong: spanning frame, anchor arithmetic (9000→11351), bounded drain note, SlowLinear clamp. Would fail if the walk or arithmetic broke. Does NOT cover the REASON_TAIL path (F-001's blind spot) |
| test `mpg_lane_demuxes_splits_and_decodes_mp2_exactly` | 763-890 | REAL | — | 39-AU ladder pinned to the tick, decode to 44 928 frames; would catch assembler regressions |
| test `vob_lanes_demux_and_refuse_undecodable_codecs_typed` | 903-1002 | REAL | — | pins the typed codec refusals (cli.codec_unavailable / cli.pcm_facts_absent) |
| test `start_time_skips_packets_with_slow_linear_seekability` | 1012-1054 | REAL | — | watermark skip measured (30/36 kept, first survivors pinned) |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| synthetic_lane… | YES | exact event-sequence assertion incl. stamp arithmetic and drain note |
| mpg_lane… | YES | 39-value stamp ladder + packet counts + decode count |
| vob_lanes… | YES | per-file counts, span pins, refusal codes |
| start_time_skips… | YES | exact kept-counts and first-stamp pins |
| (gap) | — | NO test exercises `flush`'s tail note DELIVERY through the engine — exactly where F-001 hides. The synthetic lane's garbage is drained at push time (REASON_DRAIN), never the EOS tail (REASON_TAIL). |
Not executed (no cargo): honesty judged by reading assertions; W00 baseline (1751 pass / 2 fail) corroborates the suite runs.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "end-of-stream leftovers surface as a tail note — counted, never silently dropped" | ps.rs:44-46 | FALSE on the engine path | F-001: notes stranded in the queue after EOS; engine stops polling (pipeline.rs:2135-2139) |
| "the registry's FINAL wired family" | ps.rs:1-2 | REAL | containers/mod.rs:71 wiring verified |
| "damage is bounded and never silent" | ps.rs:41-44 | FALSE for the EOS-tail case; REAL for resync/drain | F-001 vs the tested Drain path |
| "an unparseable walk position re-syncs … (a `Recovered` note for the skipped prefix)" | ps.rs:42-43 | REAL | push 330-353 + synthetic test |

## 7. Dead / duplicated / unreachable code

- Duplicated: `ticks_for` ≡ ts.rs `au_stamp` core (F-002).
- No dead code found in this file; every fn has a caller or is a test helper.

## 8. Dependencies & flags

No feature gates; unconditional in `lumen_cli`. Depends on `demux::ps`, `demux::es::mp3::parse_head` (no local grammar copy — verified), `plugin_api` trait surface, and `crate::demux_host::{SelectDemuxer, TrackFact, OpenOptions}`.

## 9. Open questions for the next agent

- Does the TS sibling (`ts.rs`, not in my assignment) have the same stranded-flush-at-EOS shape as F-001? Its `mint_adts_aus` tail notes go through the same `self.queue` pattern (ts.rs:373, 382).
- `SelectDemuxer` (demux_host.rs): does it forward `EndOfStream` verbatim, or does it queue-and-delay (which would change F-001's blast radius)? Audited in demux_host.rs below.
- The engine's demux slice bound (`events` per tick, pipeline.rs:2140): confirm the wrapper's multi-event bursts (1 PES → N AUs) cannot starve the video limb across ticks.
