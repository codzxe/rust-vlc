# AUDIT: apps/cli/src/containers/ts.rs#part1 (lines 1–551, all production code)
| Field | Value |
|---|---|
| Auditor | W13-A06 |
| Date | 2026-11-21 |
| Lines / functions | 551 (of 1021; part2 = 557–1021 tests, A04) / 22 items |
| File verdict | REAL (with one S1 hostile-input defect) |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI open → demux_host::open_source → probe "mpegts" → containers/mod.rs:72 `"mpegts" => ts::open(stream, opts)`; tests at ts.rs:598+ construct it directly) |
| Findings | S0: 0 S1: 1 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "^(pub\|)...ts.rs` (inventory); `rg ts.rs audit/waves/w13-a*.tsv` (part2 owner = A04); `rg -n "fn parse\|frame_length" crates/demux/src/adts/mod.rs` + Read 176-250, 525-545 (AdtsHeader::parse performs NO field validation; the crate's own walker validates `frame_length < header_len` at :529-532 — the host loop does not); Read crates/demux/src/ts/mod.rs:826-935, 1106-1152 (event model: `pending` queue, one event per call, consumer fires at END of process_packet) — basis of the ordering analysis; `rg -n "pes_payload" crates/demux/src/ts/mod.rs` (:933-935 call site); `sed -n` quote verification |

Chunk boundary note: part1 = lines 1–551 — the module docs, `open`, `harvest_facts`, the whole `PesBridge`/`BridgeConsumer` PES assembly seam, `mint_packet`, `au_stamp`, `bridge_poisoned`, `TsHostDemuxer`. Lines 557–1021 are `mod tests` (A04). Cross-boundary reads (test content) were used only to assess test coverage of part1's code.

## 1. One-paragraph verdict
The TS host is genuinely wired and its architecture claims check out against the demuxer crate: the PES seam split, the pid→track routing from `programs()`/`track_of_pid()`, the limb-less aac opening (geometry in-band), the honest `SeekSupport::None` refusal for `--start-time` (pinned by a passing test), and the AU-split stamp arithmetic (`128 000 + round(1024×90000/44100) = 130 090` — verified by hand and pinned by the synthetic-lane test). But the host re-implemented the demux crate's ADTS frame walk WITHOUT the crate's own `frame_length < header_len` validation: a PES payload on an aac-typed pid whose first frame declares `frame_length == 0` (13-bit field all zeros behind a valid syncword) sends the AU-split loop into an infinite iteration that pushes an unbounded queue of empty packets — a process hang with unbounded memory growth from file-controlled input (S1). Second, the bridge-first event interleaving preserves ordering only within a single transport packet; across packets it inverts minted-packet vs demuxer-event order while the comment claims exactness. Two minor debts follow. No fuzz target reaches this code (fuzz targets are crates-side; the AU-split walk lives only in apps/cli).

## 2. What actually works (verified)
- Both TS container tests passed in the orchestrator's no-fail-fast rerun: `containers::ts::tests::corpus_ts_lanes_through_the_open_path ... ok` and the lib target's "129 passed; 0 failed" (orchrun-test-workspace-nofailfast.log.txt:2769+ relative lines 67, 134). — verified by reading the cited log.
- AU-split stamp arithmetic: `(1024 × 90 000 + 22 050) / 44 100 = 2090` — hand-checked against the test's pinned `vec![126_000, 128_000, 130_090]` (ts.rs:776-780). — verified by arithmetic + passing test.
- `--start-time` typed refusal: `SeekSupport::None` forwarded; `start_time_is_refused_typed_at_open` pins stage/code/`demux.ts.seek.unsupported` (ts.rs:807-822, passed). — verified by reading + passing result.
- Consumer-call ordering premise (within one transport packet): demuxer pushes `pending` events (ts/mod.rs:863, 883) BEFORE the PES consumer fires at the end of `process_packet` (ts/mod.rs:933-935) — the same-packet half of the wrapper's ordering comment is TRUE. — verified by reading both sites.

## 3. Findings

### F-001 [S1]
Where: apps/cli/src/containers/ts.rs:354-377
Code:
```rust
            while at + 7 <= payload.len() {
                let Some(header) = AdtsHeader::parse(&payload[at..]) else {
                    break;
                };
                let end = at + usize::from(header.frame_length);
                if end > payload.len() {
                    // A trailing partial AU: the ffmpeg-honest stop.
                    break;
                }
```
What is wrong: The AU-split walk advances by `header.frame_length` with no check that `frame_length >= header_len` (7 or 9). `AdtsHeader::parse` performs NO field validation — it is a field extractor ("field validation is the caller's policy", crates/demux/src/adts/mod.rs:188-191) — and `frame_length` is the raw 13-bit value (0..8191, adts/mod.rs:208-210). A payload whose first 7 bytes are a valid syncword with an all-zero length field (e.g. `FF F1 50 00 00 00 00`: syncword ✓, sampling_index 4 → 44 100 Hz ✓, frame_length = 0) makes `end == at`, so `at = end` never advances: the loop re-parses the same bytes forever, each iteration pushing one `Packet` with an empty payload into the unbounded `queue: VecDeque<DemuxEvent>` (ts.rs:235) and adding 1024 to `samples_before`. The demux crate's own ADTS walker validates exactly this (`if u64::from(raw.frame_length) < hlen { return Err(error::bad_header(at)); }`, crates/demux/src/adts/mod.rs:529-532) — the host copy of the walk dropped the check.
Why it matters: A crafted (or damaged) MPEG-TS file — PMT entry typed aac (0x0F/0x11), one PES payload starting with the bytes above — hangs `lumen open` forever with unbounded memory growth: a file-controlled denial of service / OOM on the shipped CLI entry point. This is the same "process hangs forever" family the wave's hostile question highlights (the 4:4:4 H.264 desktop hang is a different site — see cross-refs). No test covers short frames (part2's builders mint only well-formed 85-byte frames, ts.rs:665-675) and no fuzz target reaches apps/cli host code.
What correct looks like: Validate before use, as the demux crate does: reject `usize::from(header.frame_length) < usize::from(header.header_len())` (break to the whole-PES fallback or count as `ts.pes.adts-tail` damage). One added line closes the hole.
Confidence: HIGH — Verification: read + trace (loop arithmetic) + demux-crate comparison; not executed (REQUEST-ORCH-RUN candidate in log).

### F-002 [S3]
Where: apps/cli/src/containers/ts.rs:520-527
Code:
```rust
    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        // 1. The bridge's minted packets first: the transport walk
        //    queued its own events before the seam minted packets
        //    for the same transport packet, so this order preserves
        //    the demuxer's internal sequence.
        if let Some(event) = self.with_bridge(|bridge| Ok(bridge.next_event()))? {
            return Ok(event);
```
What is wrong: The ordering argument holds only for events and packets minted by the SAME transport packet. Across packets it inverts: trace — call N finds the bridge empty, `inner.next_event` walks T1 (bridge mints packet P1 during `process_packet(T1)`, demuxer queues nothing), continues to T2 (demuxer queues Discontinuity E2, bridge mints P2), returns E2. Call N+1 returns P1, call N+2 returns P2. Delivered order: E2, P1, P2 — but P1's elementary data physically preceded T2's discontinuity. The comment asserts the interleaving "preserves the demuxer's internal sequence" as a general invariant; it does not.
Why it matters: A packet that predates a discontinuity marker is delivered after it. The engine treats `Discontinuity` as an observe-and-continue marker (playback-engine pipeline.rs:2354), so the blast radius today is limited to consumers that key ordering-sensitive behavior (anchor resets, damage ledgers) off the marker — but the wrapper's contract is silently wrong and untested (part2's synthetic lane asserts a clean stream where `Recovered`/`Discontinuity` are `unreachable!`, ts.rs:770-772).
What correct looks like: Either sequence the bridge queue and the demuxer's pending events by a shared monotonic mint counter (merge on pop), or weaken the comment to the same-transport-packet guarantee and register the cross-packet inversion as a known defect.
Confidence: HIGH — Verification: read (event model at ts/mod.rs:1106-1152: pending-queue, one event per call, consumer at end of process_packet) + constructed trace; not executed.

### F-003 [S4]
Where: apps/cli/src/containers/ts.rs:254-256
Code:
```rust
    fn push(&mut self, pid: u16, pusi: bool, payload: &[u8]) -> Result<(), EngineError> {
        self.assembler.push_packet(pid, pusi, false, payload)?;
        self.drain()
```
What is wrong: The assembler's `discontinuity` parameter is hard-coded `false`, so the assembler's per-PES discontinuity state (pes.rs:531-539: `state.disc = discontinuity`) is never set and every minted `Packet.discontinuity` is structurally `false` (ts.rs:370, 404, 452). The transport-level signal rides only the demuxer's `Discontinuity` events — which F-002 shows can be delivered mis-ordered relative to the packets they describe. The choice is documented (no duplicate event) but leaves a dead packet-level flag.
Why it matters: Downstream consumers reading `Packet.discontinuity` get a constant; the informational path that replaces it has the F-002 ordering defect.
What correct looks like: Forward the adaptation-field discontinuity bit through the seam (the consumer would need the bit — the seam currently passes payload only), or delete the dead flag plumbing.
Confidence: HIGH — Verification: read (pes.rs:524-542 confirms the parameter is the only source).

### F-004 [S4]
Where: apps/cli/src/containers/ts.rs:391-407
Code:
```rust
        // No ADTS framing found: the whole-PES shape, honestly.
        self.queue.push_back(DemuxEvent::Packet(Packet {
            track,
            pts: match pts {
                Some(ticks) => pts_stamp(ticks)?,
```
What is wrong: The whole-PES fallback re-implements `mint_packet` (436-455) inline (pts/dts stamping, payload move, keyframe false, discontinuity) instead of calling it — copy-paste divergence risk (the two bodies already differ subtly: this one moves `payload`, `mint_packet` receives the struct by value).
Why it matters: Duplication in a file whose whole doctrine is "the demux crate's published grammar, no local copy".
What correct looks like: Reconstruct the `PesPacket` (or refactor `mint_packet` to take fields) and call it.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-76 | REAL | — | Accurate family truths; h264-since-2026-11-15 dispatch claim verified against pipeline_host.rs:346 + passing test (ts.rs:796-799) |
| `open` | 94-176 | REAL | — | seek refusal path verified by test; routing from programs()/track_of_pid; unrouted pids counted not dropped (see `mint`) |
| `harvest_facts` | 178-221 | REAL | — | debug_assert 90 kHz pin (release no-op); all-None facts honest per docs; test pins every None |
| `PesBridge` (struct) | 227-236 | REAL | S1 | unbounded `queue` is the F-001 amplifier |
| `PesBridge::new` | 238-246 | REAL | — | trivial |
| `PesBridge::push` | 248-257 | REAL | S4 | F-003: hard-coded `false` discontinuity |
| `PesBridge::finish` | 259-267 | REAL | — | sticky re-raise documented and consistent with next_event EOS path |
| `PesBridge::drain` | 269-290 | REAL | — | packets before ledgers per drain; timing events → bare Discontinuity each |
| `PesBridge::mint` | 292-313 | REAL | — | unrouted-pid damage counted (`pes_len as u64` — 0 for unbounded PES, mild undercount) |
| `PesBridge::mint_adts_aus` | 315-408 | REAL | S1 | F-001 missing frame_length validation; also: rate taken from FIRST frame only (:349) — mid-PES rate change would mis-stamp subsequent AUs (malformed-input edge); tail bytes counted when aus>0 |
| `PesBridge::next_event` | 410-413 | REAL | — | trivial |
| `BridgeConsumer` (struct) | 416-420 | REAL | — | — |
| `PesConsumer::pes_payload` | 422-430 | REAL | — | lock poisoning → typed refusal, never unwrap |
| `mint_packet` | 432-455 | REAL | — | keyframe always false (container declares none — documented; TS random_access_indicator exists but seam passes payload only) |
| `au_stamp` | 457-470 | REAL | — | u128 intermediate, saturating, rate.max(1) guards 0; round-half-up via `+ rate/2` — hand-verified 2090 |
| `bridge_poisoned` | 472-482 | REAL | — | typed, house shape |
| `TsHostDemuxer` (struct) | 488-497 | REAL | — | — |
| `TsHostDemuxer::with_bridge` | 499-511 | REAL | — | — |
| `Demuxer::start` | 513-518 | REAL | — | idempotent cached list (ts/mod.rs:1036-1038 confirms) |
| `Demuxer::next_event` | 520-542 | REAL | S3 | F-002 ordering inversion; EOS→finish→bridge-drain→EOS sequence correct |
| `Demuxer::seek` | 544-546 | REAL | — | forwarded refusal |
| `Demuxer::seekability` | 548-550 | REAL | — | forwarded |

## 5. Test honesty
Tests live in part2 (A04). Coverage assessment from part1's perspective:
| Test (part2) | Would it fail if part1's code were broken? | Reason |
|---|---|---|
| synthetic_lane_opens_demuxes_and_refuses_seek_typed | YES for routing/AU-split/stamps/seek | pins pts vec + track vec + all-None facts + typed seek refusal |
| start_time_is_refused_typed_at_open | YES | pins stage/code/details |
| corpus_ts_lanes_through_the_open_path | ONLY WITH CORPUS (same gate as wav.rs F-001 — corpus dir absent in sandbox) | video lanes demux-only (decode refuses typed), 0091 aac AU-split |
| (no test) frame_length 0/short ADTS frame | NO TEST EXISTS | F-001 would hang any test that tried — none does |
| (no test) demuxer-event vs bridge-packet interleaving | NO TEST EXISTS | synthetic lane asserts a clean stream (`unreachable!` on events) — F-002 unpinned |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "h264 … dispatches the software decoder (Annex-B framing …)" | module docs 38-40 | REAL | pipeline_host.rs:346 + passing test ts.rs:796-797 |
| "a bounded PES short of its declared length surfaces the assembler's typed truncation — never a silent drop" | module docs 22-24 | REAL-UNPROVEN | code path read (finish → ? at 534); no baseline run exercises a truncated bounded PES through the CLI |
| "queue-first preserves the demuxer's internal sequence" | module docs 18-21 / comment 521-524 | FALSE (as general invariant) | F-002 constructed trace |
| "The skip watermark is never set on a family that cannot position" | module docs 69-72 | REAL | test start_time_is_refused_typed_at_open |

## 7. Dead / duplicated / unreachable code
- `Packet.discontinuity` plumbing (always false) — F-003.
- Whole-PES fallback duplicates `mint_packet` — F-004.

## 8. Dependencies & flags
Depends on demux (ts, adts header grammar), media-types, plugin-api, commands — all default-on, no feature gates. The ADTS header grammar is the demux crate's published API (no local copy) — except the walk's validation, which was NOT copied (F-001).

## 9. Open questions for the next agent
1. (A04, part2) Add the missing hostile-input tests: frame_length 0/6 ADTS frame in an aac PES; a demuxer event interleaved between two bridge packets. Both currently unexercised.
2. (demux ts wave) Does the PES assembler bound a single unbounded PES payload? (F-001's payload must merely be ≥ 7 bytes, so boundedness of the assembler is not at issue, but a huge PES plus the F-001 loop multiplies memory.)
3. (engine wave) Which engine behaviors key off `Packet.discontinuity` vs the `Discontinuity` event? Determines F-002/F-003's real blast radius.
