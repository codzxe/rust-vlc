# AUDIT: apps/cli/src/containers/ts.rs#part2 (lines 511–1020)
| Field | Value |
|---|---|
| Auditor | W13-A04 |
| Date | 2026-11-21 |
| Lines / functions | 510 (of 1020; part1 = 1–510 is A06's) / 17 |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open <x.ts>` → run.rs → `demux_host::open_source` → probe mints `mpegts` → containers/mod.rs:72 `"mpegts" => ts::open` → ts.rs:105 `open` → `TsHostDemuxer` wrapped in `SelectDemuxer` (ts.rs:156-169, part1) → the `impl Demuxer for TsHostDemuxer` in this part is the engine's event source) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 0 |
| Commands run | sed ts.rs 1-510 (part1 context: open/harvest/PesBridge/mint/au_stamp); Read ts.rs 511-1021 (this part); sed demux/src/ts/mod.rs 834-975, 1000-1035, 1106-1160, 1028-1055 (process_packet event-push order :863/:883 before consumer :930; next_event multi-packet loop :1113-1152; start idempotence :1036-1038); rg demux ts/tests.rs discontinuity (:577 — no ordering-vs-PES test anywhere); rg ts tests in baseline: orchrun-test-workspace-nofailfast.log.txt:2835-2837 all `ok`; ls /tmp/my-project/corpus-v4/media → ABSENT; rg LUMEN_CORPUS in baseline logs → not set; sed quotes 520-542, 529-538, 835-843, 1011-1019 |

## 1. One-paragraph verdict
Part2 is the host-demuxer impl (start/next_event/seek/seekability) plus the test suite. The wiring is real and two of the three tests drive genuine lanes through the real open path (synthetic TS with real PAT/PMT/SDT + PES payloads; typed seek refusal pinned by code). But the event-interleaving policy in `next_event` has a systematic ordering defect: the bridge queue is drained before each inner event, while the inner demuxer processes MANY transport packets per call (pushing its own events to `pending` and minting PES packets into the bridge along the way) — so every PES completed inside a processing window is emitted AFTER the event that terminated that window, inverting chronology whenever a discontinuity/damage event follows a PES completion. The comment justifying the policy only covers same-transport-packet pairs and is falsified by the cross-packet case. Additionally, the strongest test (the corpus differential with ffprobe-measured pins and a full AAC decode) silently no-ops on any machine without the Lead's private corpus and passed vacuously in the recorded baseline.

## 2. What actually works (verified)
- All three tests green in baseline: `test containers::ts::tests::{corpus_ts_lanes_through_the_open_path, start_time_is_refused_typed_at_open, synthetic_lane_opens_demuxes_and_refuses_seek_typed} ... ok` (orchrun-test-workspace-nofailfast.log.txt:2835-2837) — with the corpus caveat of F-002.
- `synthetic_lane_opens_demuxes_and_refuses_seek_typed` is a genuine end-to-end pin: real open path, PMT-minted h264/aac tracks, 90 kHz base, AU-split arithmetic (128 000 + round(1024×90000/44100) = 130 090 asserted at :776-780), typed `demux.ts.seek.unsupported` refusal (:789-792), and real `HostDecoderProvider` dispatch for both h264 and aac (:796-799).
- `--start-time` refusal is real and typed (stage Demux, code `demux.ts.seek.unsupported`) — verified by the baseline test at :2836.
- `start()` idempotence claim verified: TsDemuxer::start returns the cached `tracks_out` on second call (demux/src/ts/mod.rs:1036-1038), so the engine's re-`start` through the wrapper is safe.
- Clean-stream ordering is correct (traced by hand): with no inner events, all mints accumulate, EOS triggers `finish()`, then minted packets drain in order before EndOfStream — the synthetic test's `[126 000, 128 000, 130 090]` pins exactly this.

## 3. Findings
### F-001 [S2]
Where: apps/cli/src/containers/ts.rs:520-542
Code:
```rust
    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        // 1. The bridge's minted packets first: the transport walk
        //    queued its own events before the seam minted packets
        //    for the same transport packet, so this order preserves
        //    the demuxer's internal sequence.
        if let Some(event) = self.with_bridge(|bridge| Ok(bridge.next_event()))? {
            return Ok(event);
        }
        match self.inner.next_event()? {
```
What is wrong: The bridge-first drain inverts event/packet order across transport packets. The inner `TsDemuxer::next_event` loops over transport packets until its `pending` queue is non-empty (demux/src/ts/mod.rs:1113-1152), and `process_packet` pushes its own events (Discontinuity at :863/:883, Recovered at :1019) BEFORE calling the PES consumer (:930). Trace: packets 1..k complete PES packets (minted into the bridge during call N's inner processing), packet k+1 raises a Discontinuity → inner returns the Discontinuity at call N; the minted packets only become visible at call N+1's bridge drain. Emitted order: Discontinuity, then PES packets whose payloads chronologically precede it. The comment's justification ("queued its events before the seam minted packets for the same transport packet") is true only for same-packet pairs — for those the host does preserve order (event returned by inner at call N, same-packet mint drained at call N+1) — but the common cross-packet case is inverted, and the comment's conclusion ("this order preserves the internal sequence") is false: internally the mint at packet 1 precedes the event queued at packet 2; the host emits the event first.
Why it matters: A real broadcast stream (discontinuity indicators and CC jumps are why the demuxer implements the continuity policy at all) hands the engine a Discontinuity BEFORE the pre-discontinuity packets, so the engine's cross-discontinuity rules ("arithmetic across them without a clock reset is the engine's own rule to enforce" — part1 drain comment, ts.rs:282-285) operate on a misordered stream; pre-jump media is treated as post-jump. No test anywhere pins event-vs-packet ordering: the demux-crate discontinuity test (crates/demux/src/ts/tests.rs:577 `af_discontinuity_indicator_raises_the_event`) drives the demuxer with NO PES consumer and only counts events; the CLI lanes are clean by construction.
What correct looks like: Interleave at transport-packet granularity — either the demuxer exposes the seam's mints through its own `pending` queue (events and mints in one FIFO), or the host drains the bridge after EACH inner processing step (an inner API that returns per-packet), so a minted packet is never deferred behind a later event.
Confidence: HIGH — Verification: read of both files + hand-traced call sequence; UNVERIFIED only in the sense that no executable run demonstrates it (no fixture with discontinuity+PES exists — see F-002).

### F-002 [S3]
Where: apps/cli/src/containers/ts.rs:836-843, 1017
Code:
```rust
        let Some(dir) = corpus_dir() else {
            eprintln!(
                // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
                "corpus(ts-cli): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
                 — SKIP (the differential table is in the worklog)"
            );
            return;
        };
```
and `let default = PathBuf::from("/tmp/my-project/corpus-v4/media");` (:1017)
What is wrong: The corpus differential — the only test that pins real-file PES counts (81/234/160/3), ffprobe-measured PTS ends, uniform-cadence midchange behavior, the hevc `cli.codec_unavailable` refusal (:966-968), and the full 44 032-frame AAC decode equality with ffmpeg (:1007) — silently returns (counted PASS by cargo) whenever the corpus is absent, and the corpus is a Lead-sandbox path (`/tmp/my-project/corpus-v4/media`) that exists neither in the repo nor at the default location in the audit sandbox (`ls` → No such file or directory), nor was `LUMEN_CORPUS` set in any baseline run (rg over baseline logs). The baseline's `ok` at orchrun log:2835 is therefore a vacuous pass: none of the test's pins executed in the recorded evidence, and the code's own inline marker concedes the video-decode half is "[VERIFY on a corpus-enabled host]" (:912).
Why it matters: The strongest truth-claims in this file (ffprobe parity numbers "measured this session", the hevc-refusal claim, the 44 032 == ffmpeg decode equality) rest on a test that has never run in any recorded evidence; a regression in the TS lane's real-file behavior would not fail any committed run.
What correct looks like: Commit small .ts fixtures under tests/fixtures (the synthetic builder pattern already proves it is possible), or make the corpus-gated lane `#[ignore]`-style explicit instead of a silent green, and drop the hard-coded `/tmp` Lead path in favor of a workspace-relative default.
Confidence: HIGH — Verification: read + ls + rg over baseline logs.

### F-003 [S3]
Where: apps/cli/src/containers/ts.rs:529-538
Code:
```rust
            DemuxEvent::EndOfStream => {
                // 2. The flush: mint the trailing PES, then the
                //    honest end. A short bounded PES raises the
                //    typed truncation here (re-raised verbatim on
                //    every later call — see `finish`).
                self.with_bridge(PesBridge::finish)?;
```
What is wrong: A file truncated mid-bounded-PES (interrupted recording/download — the last audio PES declares `pes_len` it never fulfills) makes `finish()` raise the assembler's typed truncation; `?` propagates it, so `next_event` returns `Err` on EVERY subsequent call and `EndOfStream` is never delivered — the session ends in a fatal demux error. This diverges from the demuxer's own tail policy: a partial trailing transport PACKET is a damage note, not an error (`ts: partial tail packet skipped` → `Recovered`, demux/src/ts/mod.rs:1124-1133), and from the engine's per-frame-damage concealment philosophy (a truncated final AU is exactly the class `per_frame_damage` conceals). It also abandons every other trailing mint `finish()` would have produced (the `drain()` after `finish()` never runs when finish errors).
Why it matters: Realistic truncated files flip a recoverable end-of-stream into a permanent error and discard the flush's other packets; the "naturally-sticky damage shape" is sticky-fatal, not damage.
What correct looks like: Treat a short bounded PES at flush as `Recovered { skipped, reason: "ts.pes.truncated-at-eos" }` + deliver the partial payload (or drop it, counted), then return EndOfStream — matching the tail-packet policy one layer down.
Confidence: MEDIUM — Verification: read of this part + part1's `finish` doc (ts.rs:259-263) + demux tail policy; the sticky-re-raise behavior is documented but no committed test drives a truncated-bounded-PES file through the CLI host (UNVERIFIED at runtime; the demux suite may pin the assembler half — A06/demux wave to confirm).

## 4. Per-function table (this part only)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Demuxer::start` (TsHostDemuxer) | 514-518 | REAL | — | Delegates to inner's idempotent cached list (verified mod.rs:1036-1038). |
| `Demuxer::next_event` | 520-542 | REAL | S2 (F-001), S3 (F-003) | Bridge-first drain inverts cross-packet order; EOS/finish sticky-fatal on truncated bounded PES; after a clean EOS, repeated calls loop drain→inner→finish→drain→EndOfStream harmlessly (finish is idempotent when nothing is pending). |
| `Demuxer::seek` | 544-546 | REAL | — | Forwards; inner refuses typed (verified by test :789-792). |
| `Demuxer::seekability` | 548-550 | REAL | — | Forwards inner's `SeekSupport::None` (mod.rs:1156-1158); asserted :788. |
| `tests::CountingSink::accept` | 573-583 | REAL | — | Counts sample frames via `format.frames_in`; audio-only by design. |
| `tests::open` | 587-592 | REAL | — | Real open path; panics (unreachable!) on refusal — test-appropriate. |
| `tests::decoder_for` | 596-607 | REAL | — | Real `HostDecoderProvider`; typed refusals returned not panicked. |
| `tests::TempDir::{new,write,drop}` | 613-630 | REAL | — | pid-suffixed temp dir, self-removing — clean. |
| `tests::encode_ts5` | 635-644 | REAL | — | Test-local inverse of the assembler's decode; marker bits present; proven by the 126 000/128 000 round-trip assertions. |
| `tests::pes_bytes` | 648-660 | REAL | — | Bounded PTS-only PES; `u16::try_from(after).unwrap_or(u16::MAX)` clamps hostile sizes (test-only). |
| `tests::adts_frame_85` | 665-675 | REAL | — | 85-byte LC/44.1k/stereo frame; fields exercised via the real `AdtsHeader::parse` in the AU-split path. |
| `tests::synthetic_ts` | 684-717 | REAL | — | Real demux-crate builders for PSI; two-ADTS PES exercises the AU split. |
| `tests::synthetic_lane_opens_demuxes_and_refuses_seek_typed` | 726-800 | REAL | — | Mutation-hard: codec/base/facts, walk order+stamps, seek refusal, provider dispatch all asserted; ran green in baseline (:2837). |
| `tests::start_time_is_refused_typed_at_open` | 807-822 | REAL | — | Stage/code/details asserted; ran green (:2836). |
| `tests::corpus_ts_lanes_through_the_open_path` | 835-1009 | REAL-UNPROVEN | S3 (F-002) | Vacuous in all recorded evidence; pins are real-looking and internally consistent (AU math, ffprobe-claimed ends) but unexecuted. |
| `tests::corpus_dir` | 1012-1019 | REAL | S3 (F-002) | Lead-sandbox hard-coded default. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| synthetic_lane_opens_demuxes_and_refuses_seek_typed | YES | Exact facts, exact pts series incl. the AU-split arithmetic, typed seek refusal, provider dispatch — any wiring lie breaks an assertion. Ran in baseline. |
| start_time_is_refused_typed_at_open | YES | Stage+code+details of the refusal asserted. Ran in baseline. |
| corpus_ts_lanes_through_the_open_path | NO (in recorded evidence) | Returned before any assertion — corpus absent on every recorded machine (F-002). On a corpus-enabled host it would be mutation-hard (counts, PTS ends, cadence, decode equality). |
| No test pins event-vs-PACKET ordering | NO | F-001's inversion survives every committed test in both the CLI suite and the demux suite (the demux discontinuity test runs consumer-less). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "h264 dispatches the software decoder… aac dispatches the AAC-LC decoder" (2026-11-15 landing) | module docs ts.rs:38-45 (part1) + test :793-799 | REAL | Baseline :2837 (synthetic lane asserts both dispatches through the real provider). |
| "hevc/mpeg2 video still refuses decode typed at the provider (`cli.codec_unavailable`)" | module docs ts.rs:40-42 | REAL-UNPROVEN | Only asserted inside the corpus-gated test (:966-968) which never ran in recorded evidence (F-002). |
| "0091's aac track decodes the FULL stream exactly… == ffmpeg" | test docs :828-831, :1003-1008 | REAL-UNPROVEN | Same corpus gate; no recorded execution. |
| "queue-first preserves the internal sequence" | next_event comment :521-524 | FALSE | F-001: holds only for same-transport-packet pairs; cross-packet pairs are inverted. |
| "A short bounded PES raises the typed truncation here (re-raised verbatim on every later call)" | comment :531-533 | REAL | Matches part1 `finish` doc + assembler contract; consequence is F-003. |

## 7. Dead / duplicated / unreachable code
- Nothing dead in this part. The `Arc<Mutex<PesBridge>>` (part1 construct, used here) is single-threaded in practice (consumer is called synchronously inside `inner.next_event`) but is the Send-safe choice for a `Box<dyn Demuxer>` that the engine may move — justified, noted for completeness.
- `encode_ts5`/`pes_bytes`/`adts_frame_85` are test-local mirrors of demux-side grammars — acceptable test scaffolding, exercised through the real parsers.

## 8. Dependencies & flags
- No new deps in this part; uses `demux::ts` test_support (cfg(test)), `HostDecoderProvider`, `testkit`. No feature gates. Whole file ships (baseline build #1).

## 9. Open questions for the next agent
- A06 (ts.rs part1): does `PesBridge::drain`'s "packets first, then damage, then timing" queue order interact with F-001 (a double inversion layer)? And confirm the demux suite's pin of the sticky truncation in `PesAssembler::finish`.
- DEMUX WAVE: the `pending`-vs-seam ordering inside `process_packet` (events pushed at mod.rs:863/883 before the consumer call at :930) is the root of F-001 — worth a demux-side ordering test with a REAL consumer.
- ENGINE WAVE: what does the engine actually do when `Discontinuity` precedes packets that predate it (clock-reset misapplication)? F-001's blast radius is defined there.
- F-002's corpus convention (`corpus_ogg.rs`) — if the ogg suite has the same vacuous gate, this is a workspace-wide evidence hole worth one consolidated finding.
