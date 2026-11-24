# AUDIT: apps/cli/src/containers/ts.rs
| Field | Value |
|---|---|
| Auditor | W13-A08 |
| Date | 2026-11-24 |
| Lines / functions | 1020 / 18 production + 18 test fns/helpers (3 #[test]) |
| File verdict | REAL (with one S1 hostile-input hole in the ADTS AU-split walk) |
| Reachable from shipped entry point? | YES: `lumen` main → open cmd → `open_source` (apps/cli/src/demux_host.rs:446 probe, :458 `containers::open`) → containers/mod.rs:72 `"mpegts" => ts::open` → this file |
| Findings | S0: 0 S1: 2 S2: 0 S3: 3 S4: 4 |
| Commands run | `./target/debug/deps/lumen_cli-5f13e10be716a8b0 containers::ts` → 3 passed (exit 0); same `--nocapture corpus_ts` → SKIP line captured; `./target/debug/lumen open audit/runs/W13-A08-scratch/ts188.bin` → **panic exit 101**; ts187.bin → exit 65; ts189.bin → exit 65; `rustc -O` replica probe adts_zero_len_probe.rs → NON-PROGRESS output (all exit 0 except the intended 101/65s); sed/rg verifications |

## 1. One-paragraph verdict
The TS container host is genuinely implemented and genuinely wired: the open path runs the
demuxer's bounded PSI pre-pass, harvests one fact per declared ES, routes PES through a real
shared assembler bridge, AU-splits ADTS aac, and forwards the typed seek refusal — the two
corpus-independent tests exercise the REAL open path and pass under my own run. But the AU-split
walk has NO guard against an ADTS header declaring `frame_length == 0` (a shape the demux crate's
own ADTS lanes explicitly refuse in three places): `at` never advances, so one crafted PES payload
in a hostile .ts file (local or HTTP) mints `DemuxEvent::Packet`s forever into an unbounded queue —
hang + OOM, executed by replica probe. Second, the shipped entry path still crashes (exit 101, my
own reproduction) on a 188-byte 0x47 file in `TsSignature::probe` — media-probe's code, but it sits
in front of every `ts::open` call (the Wave-4/5 S1, still unfixed in this tree, byte-verified).
Third, the `next_event` "queue-first preserves the internal sequence" claim only holds per
transport packet, not across the demuxer's multi-packet processing batch.

## 2. What actually works (verified)
- `./target/debug/deps/lumen_cli-5f13e10be716a8b0 containers::ts` → "3 passed; 0 failed" (binary
  mtime newer than ts.rs). `synthetic_lane_opens_demuxes_and_refuses_seek_typed` walks
  `open_source` (canonicalize→access→probe→this host) end to end: container "mpegts", 2 tracks,
  h264+aac codecs, 90 kHz base, PTS series `[126_000, 128_000, 130_090]` (AU arithmetic
  128 000 + round(1024×90 000/44 100) = 2 090), track routing order, `SeekSupport::None`, typed
  `demux.ts.seek.unsupported`, h264+aac decoder dispatch. One broken link in open/harvest/route/
  split/stamp fails it.
- `start_time_is_refused_typed_at_open` → stage `Demux`, code `Unsupported`, details
  `demux.ts.seek.unsupported` — executed pass.
- `--start-time` negative clamp `.max(0)` and `saturating_mul(1000)`: no overflow path (read).
- 188/187/189-byte probe controls executed (see F-002): the 188-byte panic is still live; the
  187B `demux.ts.no-pat` and 189B `probe.no-demuxer` typed refusals prove the guard works when
  a byte exists at [188].

## 3. Findings
F-001 [S1]
Where: apps/cli/src/containers/ts.rs:354-377 (walk), :349 (rate anchor); missing guard the demux
crate applies at crates/demux/src/adts/mod.rs:530, :659, :928
Code:
```
            let mut at = 0usize;
            let mut samples_before = 0u64;
            let mut aus = 0u64;
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
What is wrong: `AdtsHeader::parse` (adts/mod.rs:193-214) validates only syncword+7 bytes; it does
NOT validate `frame_length >= header_len()`. The demux crate's own ADTS demuxer lanes refuse the
shape explicitly (`if u64::from(raw.frame_length) < hlen` — adts/mod.rs:530/659/928); this host's
walk has no such guard. With `frame_length == 0` (header bytes `FF F1 50 00 00 00 00`: sync ok,
sf_idx 4, all 13 length bits zero), `end == at`, an empty `payload[at..at]` packet is minted,
`at = end` does not advance, and the loop re-parses the same header forever — `self.queue`
(`VecDeque<DemuxEvent>`) grows without bound (one heap-allocated `Packet` per iteration), the
`push`→`drain`→`mint` chain never returns, and the walk hangs into OOM. Executed replica probe
(rustc, verbatim copy of `AdtsHeader::parse` + the walk structure, audit/runs/W13-A08-scratch/
adts_zero_len_probe.rs): "parse accepted: frame_length=0 rate=Some(44100) / after 1000000
iterations: at=0 ... zero-byte packets=1000000 / RESULT: NON-PROGRESS". Loop-exit analysis (read):
the only exits are parse `None` (same bytes → deterministic `Some`), `end > len` (false, end==at),
and `au_stamp` Err (needs `samples_before` ≈ 2^63 — ~9e15 iterations past OOM). Why it matters:
one crafted PES on a PMT-declared 0x0F/0x11 pid in a hostile .ts file kills the shipped open path
(local file or the HTTP lane, which serves the same path) — exactly the "unbounded alloc from
header" S1 class. Secondary: `frame_length` 1..=6 (< its own header) mints sub-header garbage
"AU" packets instead of refusing, diverging from the crate's own strict policy. What correct
looks like: `if usize::from(header.frame_length) < usize::from(header.header_len()) { break/`
`Recovered "ts.pes.adts-short-frame" }` — i.e., adopt the crate's own `frame_length < hlen`
refusal before minting. Confidence: HIGH — Verification: ran (replica probe; parse semantics
copied verbatim and sed-verified) + read (non-progress proof).

F-002 [S1 — cross-file, entry path of this host; still unfixed]
Where: crates/media-probe/src/signature/phase5.rs:160-161 (live site), reached via
apps/cli/src/demux_host.rs:446 before containers/mod.rs:72 dispatches to this file
Code:
```
        if prefix.len() >= TS_PACKET_BYTES {
            if prefix[TS_PACKET_BYTES] != 0x47 {
```
What is wrong: on a probe window of EXACTLY 188 bytes starting 0x47 (fill_window stops at EOF,
lib.rs:224-234), `prefix[188]` is out of bounds → panic. EXECUTED BY ME NOW (not just cited from
Wave 4/5): `./target/debug/lumen open audit/runs/W13-A08-scratch/ts188.bin` →
"thread 'main' panicked at crates/media-probe/src/signature/phase5.rs:161:16: index out of
bounds: the len is 188 but the index is 188", **exit 101**; controls 187 B → exit 65
(`demux.ts.no-pat`), 189 B → exit 65 (`probe.no-demuxer`). The demux-side twin (ts/mod.rs:321
`header.len() < PACKET_BYTES || header[PACKET_BYTES] == 0x47`) carries the identical off-by-one
but has no production caller. A 188-byte single-packet TS is valid input by the demuxer's own
comment ("a whole 188-byte file is also accepted — the walk will tell"). Filed against ts.rs
because every `lumen open <x>.ts` traverses this chain immediately before `ts::open`.
What correct looks like: `prefix.len() <= TS_PACKET_BYTES || prefix[TS_PACKET_BYTES] == 0x47`
(or `.get(188)`) in BOTH copies. Confidence: HIGH — Verification: ran (3 executions, exit codes
captured; bytes sed-verified today).

F-003 [S3]
Where: apps/cli/src/containers/ts.rs:520-527 (claim), :528-541; demuxer batch loop
crates/demux/src/ts/mod.rs:1113-1151
Code:
```
        // 1. The bridge's minted packets first: the transport walk
        //    queued its own events before the seam minted packets
        //    for the same transport packet, so this order preserves
        //    the demuxer's internal sequence.
        if let Some(event) = self.with_bridge(|bridge| Ok(bridge.next_event()))? {
            return Ok(event);
        }
```
What is wrong: the claim holds per transport packet (demuxer pushes Discontinuity into `pending`
at ts/mod.rs:863/883 BEFORE `pes_payload` at :933-935, and `inner.next_event` pops `pending`
before the wrapper ever re-examines the bridge). It does NOT hold across one `inner.next_event`
call, whose loop processes many transport packets: packets minted into the bridge by EARLIER
packets of the batch are still queued when a LATER packet's Discontinuity is popped and returned
first. Wire order P1(pkt)…P2(cc-jump→Discontinuity, pkt) is delivered as Discontinuity, P1's
packet, P2's packet — the discontinuity signal overtakes pre-discontinuity packets. Realistic
(damaged streams raise cc-jump Discontinuities mid-burst); impact bounded because the module doc
itself defers clock-reset arithmetic to the engine ("arithmetic across them without a clock reset
is the engine's own rule to enforce", :282-285), but the "preserves the internal sequence"
statement is stronger than the code delivers. What correct looks like: drain the bridge BEFORE
returning any demuxer event minted later in the same batch (e.g., have `inner` expose the number
of packets walked, or drain bridge-then-pending per transport packet), or weaken the doc.
Confidence: HIGH — Verification: read + trace of both loops (no run; mutation reasoning).

F-004 [S3]
Where: apps/cli/src/containers/ts.rs:146-148
Code:
```
                routing.insert(stream.pid, track);
                if stream.descriptor().as_str() == "aac" {
                    au_split.push(stream.pid);
                }
```
What is wrong: AU-split membership is keyed on string equality with the demuxer's registry
vocabulary. If the demuxer's descriptor for 0x0F/0x11 ever changes (or a new ADTS-framed codec id
appears), the split silently stops while routing continues: multi-frame ADTS PES reach the AAC
decoder whole, and by this file's own analysis (:318-320 "a multi-frame buffer would decode only
its first frame") audio silently truncates to ~1 frame per PES — no error, no damage event.
Stringly-typed seam whose failure mode is silent media corruption. Today correct (descriptor()
maps 0x0F|0x11 → "aac", ts/mod.rs:356, pinned by the synthetic test), so S3 not S2.
What correct looks like: key on `stream_type` (0x0F/0x11) or a typed predicate from the demux
crate (`matches!(stream.stream_type, 0x0F | 0x11)`). Confidence: HIGH — Verification: read.

F-005 [S3]
Where: apps/cli/src/containers/ts.rs:296-305
Code:
```
        let Some(track) = self.routing.get(&pes.pid).copied() else {
            // A completed PES on a pid the PSI declared but the
            // routing missed: a demuxer/host disagreement —
            // counted as recovered damage, never silently dropped.
            self.queue.push_back(DemuxEvent::Recovered {
                skipped: pes.pes_len as u64,
                reason: "ts.pes.unrouted-pid",
```
What is wrong: `skipped` reports the DECLARED `pes_len`, which is 0 for every unbounded video PES
and never equals the actual dropped payload length (`pes.payload.len()`); the "counted, never
silently dropped" ledger is numerically wrong exactly when it fires on video. Also note the arm
is dead today by construction (routing is built from the same `programs()` surface the demuxer
routes with), so it is untested by any executed test. What correct looks like:
`skipped: pes.payload.len() as u64` (and a test that forces the disagreement). Confidence: HIGH —
Verification: read; dead-path status by construction analysis.

F-006 [S4]
Where: apps/cli/src/containers/ts.rs:255, :370, :404, :452
Code:
```
        self.assembler.push_packet(pid, pusi, false, payload)?;
```
What is wrong: the third argument (discontinuity) is hard-coded `false`, so `PesPacket.
discontinuity` is constant false in the shipped lane: `mint_packet`'s `discontinuity:
pes.discontinuity` (:452) and `discontinuity && aus == 0` (:370) never fire. Cross-file
confirmation from W05-A02 F-001 (audit/files/crates/demux/src/ts/pes.rs.audit.md:38-65): the
`PesConsumer` seam carries only `(pid, pusi, payload)`. This file's own doc (:248-253) is honest
about it ("the seam passes payload only ... no duplicate event is minted here"), so this is
dead-plumbing hygiene, not a false claim here — but the per-packet discontinuity plumbing
end-to-end is dead in production, as the pes.rs auditor asked this file's owner to confirm.
Confidence: HIGH — Verification: read + rg (W05-A02 callers enumerated).

F-007 [S4]
Where: apps/cli/src/containers/ts.rs:834-843 (gate), :846-851/:954-1008 (heavy pins)
Code:
```
        let Some(dir) = corpus_dir() else {
            eprintln!(
                // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
                "corpus(ts-cli): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
                 — SKIP (the differential table is in the worklog)"
```
What is wrong: the corpus differential lane (PES counts 81/234/160/3, PTS pins, 0091's 43 AUs and
44 032 == ffmpeg sample frames) SKIPS on this host — executed: `--nocapture` prints the SKIP line
and the test reports "ok" in 0.00 s. The skip is loud (honest), but "3 passed" in any run without
the corpus overstates coverage: the strongest pins (ffprobe-measured counts, full-stream AAC
decode == ffmpeg) are UNVERIFIED by execution here; they rest on the worklog's recorded numbers,
which I cannot check against a live corpus. Confidence: HIGH — Verification: ran (SKIP captured).

F-008 [S4]
Where: apps/cli/src/containers/ts.rs:117
Code:
```
    let start_us = opts.start_time_us.unwrap_or(0).max(0);
```
What is wrong: a negative `--start-time` is silently clamped to 0 (start of stream) rather than
refused typed — a value the user typed is accepted with different semantics. Whether a negative
value can reach here depends on parse.rs/argv validation (checked in the parse.rs audit; see
cross-ref there). Blast radius: one flag, benign landing. What correct looks like: refuse
negative `--start-time` at parse (typed `InvalidArgument`), delete the clamp. Confidence: HIGH —
Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open | 105-176 | REAL | — | executed via synthetic test; seek-refusal lane pinned |
| harvest_facts | 184-221 | REAL | — | executed (facts asserted); debug_assert 90 kHz is debug-only, documented |
| PesBridge::new | 239-246 | REAL | — | trivial |
| PesBridge::push | 254-257 | REAL | S1 | forwards to assembler; discontinuity hard-false (F-006); entry to F-001 loop |
| PesBridge::finish | 264-267 | REAL | — | flush + drain; sticky truncation is pes.rs's (W05-A02 F-005 mid-stream gap noted) |
| PesBridge::drain | 272-290 | REAL | — | packets-then-damage order; timing events → Discontinuity |
| PesBridge::mint | 295-313 | REAL | S3 | unrouted-pid ledger wrong number (F-005); au_split linear scan ok |
| PesBridge::mint_adts_aus | 334-408 | REAL (defective) | S1 | F-001 zero-length non-progress; whole-PES fallback honest; tail counted |
| PesBridge::next_event | 411-413 | REAL | — | trivial |
| BridgeConsumer::pes_payload | 423-429 | REAL | — | typed poisoning refusal, no unwrap |
| mint_packet | 436-455 | REAL | S4 | discontinuity field constant false in production (F-006) |
| au_stamp | 460-470 | REAL | — | u128 round-half-up, saturating; rate.max(1) guards 0 |
| bridge_poisoned | 475-482 | REAL | — | honest unreachable-arm shape |
| TsHostDemuxer::with_bridge | 501-510 | REAL | — | |
| TsHostDemuxer::start | 514-518 | REAL | — | idempotence is the demuxer's (ts/mod.rs:1036-1038, read-verified) |
| TsHostDemuxer::next_event | 520-542 | REAL | S3 | batch reordering (F-003); EOS flush + re-drain correct |
| TsHostDemuxer::seek | 544-546 | REAL | — | forwarded refusal |
| TsHostDemuxer::seekability | 548-550 | REAL | — | forwarded None |
| tests: CountingSink/open/decoder_for/TempDir/encode_ts5/pes_bytes/adts_frame_85/synthetic_ts/corpus_dir | 572-717, 1011-1019 | REAL | — | exercised by the two executed tests |
| test synthetic_lane_opens_demuxes_and_refuses_seek_typed | 725-800 | REAL | — | honest, strong (see §5) |
| test start_time_is_refused_typed_at_open | 806-822 | REAL | — | honest |
| test corpus_ts_lanes_through_the_open_path | 834-1009 | REAL-UNPROVEN | S4 | skips on corpus-less hosts (F-007) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| synthetic_lane_opens_demuxes_and_refuses_seek_typed | YES | one wrong fact field, stamp, routing entry, split count, or seek answer fails it; walks the REAL open path |
| start_time_is_refused_typed_at_open | YES | pins stage+code+details of the refusal |
| corpus_ts_lanes_through_the_open_path | YES on a corpus host / NO here | UNVERIFIED: skips loudly on this host (executed); would fail on wrong PES counts/PTS/decode totals where the corpus exists |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "its elementary payloads flow through the PesConsumer seam ... minting engine Packets per completed PES into a queue" | ts.rs:11-21 | REAL | synthetic test executed; pes.rs audited REAL by W05-A02 |
| "queue-first preserves the internal sequence" | ts.rs:18-21, :521-524 | PARTLY FALSE | holds per transport packet, not across one next_event batch (F-003 trace) |
| "a bounded PES short of its declared length surfaces the assembler's typed truncation — never a silent drop" | ts.rs:22-24 | FALSE mid-stream (cross-file) | W05-A02 F-005: mid-stream underrun completes silently; only EOS is typed |
| "the AU-split set is the ADTS-framed aac family (PMT 0x0F/0x11)" | ts.rs:136-139 | REAL (stringly) | descriptor() maps 0x0F/0x11→"aac" (ts/mod.rs:356); fragility F-004 |
| "unknown types pass through as ts-stream-0xNN, honest unknowns" | ts.rs:34-38 | REAL | ts/mod.rs:366-369 read |
| "`SeekSupport::None` ... the typed refusal ... propagated verbatim" | ts.rs:73-76 | REAL | executed: seekability None + `demux.ts.seek.unsupported` |
| "no committed .ts fixture exists in the tree, so the corpus-independent lane writes its own" | ts.rs:609-611 | REAL | no .ts under crates/demux fixtures (ls; corpus only under /tmp) |

## 7. Dead / duplicated / unreachable code
- `mint_packet`'s / `mint_adts_aus`'s `discontinuity` plumbing: constant false in production (F-006).
- The unrouted-pid arm (F-005): unreachable by construction today.
- `skip_until_ns` set at :131: dead today (seek refuses first) — self-documented as future-proofing.
- Duplicate PES-parse policy vs crates/demux/src/ps (marker-bit fork) is pes.rs/ps's issue (W05-A02 F-009b), not duplicated here.

## 8. Dependencies & flags
- Uses demux (ts, adts), media_types, plugin_api, commands; no feature gates, no unsafe, no new
  deps. Release lane builds (baseline cargo-build-release-lumen-cli.txt exit 0). The
  `lumen-allow-literal` annotations at :616, :838-839, :1013, :1017 are cfg(test)-only paths.

## 9. Open questions for the next agent
- F-001 fix must also decide the `frame_length < header_len()` (1..=6) policy — crate lanes
  refuse; the whole-PES fallback here would be a third policy. Unify.
- pipeline_host owner: does the engine tolerate a Discontinuity delivered before earlier packets
  (F-003), i.e., is the reorder observable? If the engine resets clock arithmetic on the event,
  pre-discontinuity packets get post-reset treatment.
- Whoever owns phase5.rs: the 188-byte panic (F-002) is STILL LIVE as of today's execution — the
  Wave-4 finding was never fixed; both copies (phase5.rs:160-161, ts/mod.rs:321) need the one-char
  fix (`>=` → `>`).
