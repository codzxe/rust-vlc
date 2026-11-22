# AUDIT: apps/cli/src/containers/ts.rs

| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-22 |
| Lines / functions | 1020 / 17 production items + 15 test items |
| File verdict | REAL-UNPROVEN (production + synthetic tests) / SHALLOW (corpus differential) |
| Reachable from shipped entry point? | YES — main.rs:19 `run::run` → `lumen_dispatch` → dispatch.rs:82 `host.open_session` → host.rs:904 `open_source` → demux_host.rs:446-458 probe names `mpegts` → containers/mod.rs:72 `"mpegts" => ts::open(stream, opts)` → ts.rs:105 `open` → `demux::ts::TsDemuxer` + the PES bridge. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `rg`/`sed -n` quote + reachability verification (all quotes below verified verbatim); `ls` corpus absence; demux-crate cross-checks (ts/mod.rs:622, 1162-1167). NO cargo (adaptation) — runtime claims UNVERIFIED unless citing W00 baseline logs. |

## 1. One-paragraph verdict

The best-behaved container host audited so far. Not a second TS parser: the transport/PSI walk is `demux::ts::TsDemuxer`'s; this file implements the PES-assembly *plug point* (`PesBridge` behind the `PesConsumer` seam) — legitimate host wiring, using the demux crate's own published `AdtsHeader::parse` and `PesAssembler` (no local grammar copies). Its module doc is the one that was UPDATED for the 2026-11-15 H.264 software landing (ts.rs:38-45: "Since the 2026-11-15 H.264 software-track landing, `h264` video dispatches the software decoder"), which makes mkv.rs:54-58 and mp4.rs:29-32 (both still claiming the typed refusal) provably stale rather than "the current truth". The two corpus-independent tests are real behavioral pins (synthetic TS written byte-by-byte, exact AU-split stamp arithmetic, typed seek refusal). The defects: the corpus differential self-skips as PASS (same pattern as mkv/mp4, and no lumen_cli test ever ran in the W00 baseline), and a truncated trailing PES permanently masks `EndOfStream` because the flush error is re-raised on every subsequent `next_event`.

## 2. What actually works (verified)

- Reachability chain above (each hop read).
- The ADTS AU-split arithmetic is exact rational math, not floats: `(samples × 90 000 + rate/2) / rate` in u128 with saturating conversion (ts.rs:460-470), pinned by the synthetic test's `128_000 + 2 090 = 130_090` assertion (ts.rs:776-780). `rate.max(1)` guards division by zero.
- `AdtsHeader::parse` is the demux crate's parser (import ts.rs:82) — no second ADTS grammar in the CLI (verified by `rg`; the only `AdtsHeader` users in apps/ are ts.rs and adts.rs hosts, both importing it).
- The demuxer-side contract claims check out: `set_pes_consumer` (crates/demux/src/ts/mod.rs:622), `SeekSupport::None` + `demux.ts.seek.unsupported` (ts/mod.rs:1162-1167, pinned by its own test suite crates/demux/src/ts/tests.rs:671-673 which RAN in the W00 baseline — cargo-test.log contains `ts_corpus`, `ts_pes_corpus` binaries).
- `--start-time` refuses the whole open typed (ts.rs:119-127), asserted by `start_time_is_refused_typed_at_open` (806-822) — honest no-fake-position behavior, no corpus needed.
- All lock failures are typed errors, never `unwrap` (ts.rs:424-427, 505-508).

## 3. Findings

### F-001 [S2] — Corpus differential self-skips as PASS; zero recorded runs

Where: apps/cli/src/containers/ts.rs:835-843 (gate), 1012-1019 (gate helper)

Code (ts.rs:836-842, verbatim):
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

What is wrong: `corpus_ts_lanes_through_the_open_path` — the only test covering real-world TS lanes (5 files, ffprobe-pinned packet counts, PTS series, the 0091 full AAC decode `44_032` frames) — returns Ok without asserting anything when `/tmp/my-project/corpus-v4/media` is absent (it is absent here, verified `ls`). The W00 baseline `cargo test --workspace` never ran any lumen_cli binary (fail-fast stop at `first-vertical`; audit/runs/baseline/cargo-test.log:2499), so this differential has no recorded green run against the current tree. The test's own comment admits the decode half was never verified: "full-stream decode truth is the codecs-crate differential's lane, not this demux-path suite [VERIFY on a corpus-enabled host]" (ts.rs:908-912).
Why it matters: §3.4 — the "user's standing mandate" differential is evidence-shaped but unexecuted; the 44_032/43-AU/304_030 pins are numbers nobody has seen a test produce.
What correct looks like: `#[ignore]` + corpus CI lane, or committed mini-fixtures (the synthetic lane proves self-written fixtures are feasible).
Confidence: HIGH — Verification: read + `ls` + baseline-log binary inventory.

### F-002 [S3] — A truncated trailing PES permanently masks EndOfStream

Where: apps/cli/src/containers/ts.rs:528-538 (flush site), 259-267 (`finish`), module doc 21-24

Code (ts.rs:528-537, verbatim):
```rust
        match self.inner.next_event()? {
            DemuxEvent::EndOfStream => {
                // 2. The flush: mint the trailing PES, then the
                // honest end. A short bounded PES raises the
                // typed truncation here (re-raised verbatim on
                // every later call — see `finish`).
                self.with_bridge(PesBridge::finish)?;
```

What is wrong: if `finish()` returns the assembler's typed truncation, `?` propagates the error and the `EndOfStream` branch never completes. On every subsequent `next_event` the demuxer answers `EndOfStream` again and `finish()` re-raises the same error (the doc for `finish` says so explicitly: "calling again re-raises it identically while the short buffer stays put"). The stream's terminal event is therefore unreachable for any file whose final bounded PES is short — the engine either errors out (session dies on a truncated tail that other players treat as end-of-file) or, if it polls again, loops on the same error forever. All packets minted before the damage DO flow (the bridge queue is drained first), so the loss is the terminal event, not the payload.
Why it matters: realistic damage case (truncated download) turns into a permanent error loop or a failed session instead of "play to end, report truncation"; the correct design is one `Recovered` for the truncation followed by `EndOfStream`.
What correct looks like: on flush error, push `DemuxEvent::Recovered { reason: truncation }`, clear the assembler state, then emit `EndOfStream`.
Confidence: MEDIUM — Verification: read; engine's repeated-error behavior UNVERIFIED (no cargo), but the re-raise-forever property is stated by the code's own doc.
Severity: S3 not S2 because the payload is preserved and the failure mode is typed, not silent.

### F-003 [S4] — `keyframe: false` hardcoded for every TS packet

Where: apps/cli/src/containers/ts.rs:445-452 (mint_packet), 363-372 (mint_adts_aus)

Code (ts.rs:445-453, verbatim):
```rust
    Ok(Packet {
        track,
        pts,
        dts,
        duration: None,
        data: SharedBytes::from_vec(pes.payload),
        keyframe: false,
        discontinuity: pes.discontinuity,
        generation: media_types::GenerationId::FIRST,
    })
}
```
What is wrong: the TS adaptation field's `random_access_indicator` (a real keyframe signal the demuxer parses but does not expose) is never consumed; every packet is minted non-keyframe (also `keyframe: false` in `mint_adts_aus`, ts.rs:369). Today that is inert (`SeekSupport::None` — no seek ever lands), but the constant is a latent lie for any future seek/priming consumer, and the comment "The container declares no keyframe bit (IVF's precedent)" (ts.rs:434-435) is false as a statement about MPEG-TS — the container DOES carry one.
What correct looks like: surface `random_access_indicator` through the seam (or delete the misleading comment).
Confidence: HIGH — Verification: read.

### F-004 [S4] — Single-threaded bridge behind an unnecessary `Arc<Mutex>`

Where: apps/cli/src/containers/ts.rs:152-155, 231-236, 418-430

Code (ts.rs:153-155, verbatim):
```rust
    demuxer.set_pes_consumer(Box::new(BridgeConsumer {
        bridge: Arc::clone(&bridge),
    }));
```

What is wrong: the engine is a synchronous single-writer poll model (lib.rs:57-62); the consumer is invoked from inside `inner.next_event()` on the same thread that drains the queue. The Mutex adds a poisoned-lock error path (defensively typed, ts.rs:475-482) that cannot occur, and the `debug_assert` clock pin (ts.rs:192-196) compiles out of release builds. Harmless, but it is defensive scaffolding presented as concurrency handling.
Confidence: HIGH — Verification: read (single-thread claim from lib.rs docs + call structure).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open` | 105-176 | REAL-UNPROVEN | — | No duration → no clamp; `--start-time` > 0 rides the pure seek query which refuses typed (tested 806-822). Routing table built from `programs()`/`track_of_pid` — a pid the PSI declares but `track_of_pid` misses silently gets no routing (counted later as `ts.pes.unrouted-pid` damage at mint, 296-305 — honest). |
| `harvest_facts` | 184-221 | REAL-UNPROVEN | — | Facts are all-None by design (no pcm/blob/extent/cadence/dims) — the honest minimal harvest; `debug_assert` clock pin release-stripped (F-004 note). |
| `PesBridge::new` | 239-246 | trivial-correct | — | |
| `PesBridge::push` | 254-257 | REAL-UNPROVEN | — | Forwards to assembler + drains. |
| `PesBridge::finish` | 264-267 | REAL-UNPROVEN | S3 (F-002) | Sticky truncation re-raise; see finding. |
| `PesBridge::drain` | 272-290 | REAL-UNPROVEN | — | Packets → Recovered → Discontinuity ordering; assembler ledgers forwarded, none swallowed. |
| `PesBridge::mint` | 295-313 | REAL-UNPROVEN | — | Unrouted pid → counted damage, never dropped. |
| `PesBridge::mint_adts_aus` | 334-408 | REAL-UNPROVEN | — | Uses demux crate's `AdtsHeader::parse` (no copy); syncword-gated; partial trailing AU → counted `ts.pes.adts-tail`; non-ADTS PES falls back to whole-packet shape. `keyframe: false` (F-003). `discontinuity && aus == 0` — only first AU carries the flag: correct. |
| `PesBridge::next_event` | 411-413 | trivial-correct | — | |
| `BridgeConsumer::pes_payload` | 422-429 | REAL-UNPROVEN | — | Typed poison handling; passes `false` for discontinuity (demuxer owns that event — documented, consistent). |
| `mint_packet` | 436-455 | REAL-UNPROVEN | — | `pts_stamp` typed; `keyframe: false` (F-003). |
| `au_stamp` | 460-470 | REAL-UNPROVEN | — | Exact rational u128 arithmetic, saturating; `rate.max(1)` guard. |
| `bridge_poisoned` | 475-482 | trivial-correct | — | Typed, never panics. |
| `TsHostDemuxer` + `with_bridge` + `Demuxer` impl | 494-551 | REAL-UNPROVEN | S3 (F-002) | Queue-first ordering preserves demuxer sequence (argued in comment; plausible); `start()` re-serves demuxer's cached list; seek/seekability forwarded. |
| tests: `CountingSink`/`open`/`decoder_for` | 572-607 | REAL-UNPROVEN | — | Real open path + real provider. |
| `TempDir` | 612-630 | trivial-correct | — | Self-cleaning scratch. |
| `encode_ts5`/`pes_bytes`/`adts_frame_85`/`synthetic_ts` | 635-717 | REAL-UNPROVEN | — | Test-local TS *builder* (inverse of the parser) — not circular: it encodes known stamps and asserts the demuxer decodes them back. |
| `synthetic_lane_opens_demuxes_and_refuses_seek_typed` | 725-800 | REAL-UNPROVEN | — | Strong: real open path, AU-split arithmetic pin (126_000/128_000/130_090), typed seek refusal, h264+aac dispatch assertions CONSISTENT with pipeline_host.rs:346/288. |
| `start_time_is_refused_typed_at_open` | 806-822 | REAL-UNPROVEN | — | Typed refusal pin, no corpus needed. |
| `corpus_ts_lanes_through_the_open_path` | 834-1009 | SHALLOW | S2 (F-001) | Skip-as-pass; ffprobe pins unexecuted in any recorded run. |
| `corpus_dir` | 1012-1019 | SHALLOW | S2 (F-001) | Gate. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| synthetic_lane_opens_demuxes_and_refuses_seek_typed | YES | Exact stamps/counts/routing/seek-refusal on a self-built stream; would catch bridge routing, AU arithmetic, and clock-base regressions. (Runtime-UNPROVEN — never ran in baseline.) |
| start_time_is_refused_typed_at_open | YES | Pins stage/code/detail of the refusal. |
| corpus_ts_lanes_through_the_open_path | NO (corpus-less env) | Returns before asserting (F-001). |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Since the 2026-11-15 H.264 software-track landing, `h264` video dispatches the software decoder" | ts.rs:38-40 | REAL-UNPROVEN | pipeline_host.rs:346 dispatches `H264SwDecoder`; ts.rs:796-797 asserts dispatch. This doc CONTRADICTS mkv.rs:54-58 and mp4.rs:29-32 (same repo, same build) — those are the stale ones. |
| "hevc/mpeg2 video still refuses decode typed (`cli.codec_unavailable`)" | ts.rs:40-42 | REAL | No hevc/mpeg2 arm in pipeline_host's match (read); refusal arm at :372-379. |
| "a bounded PES short of its declared length surfaces the assembler's typed truncation — never a silent drop" | ts.rs:22-24 | REAL-UNPROVEN | Code path verified (F-002 documents the side effect the doc does not: EOS masked). |
| "0091: last AU 304 030 == ffprobe exactly" | ts.rs:330-333, 961-965 | UNVERIFIED | Requires corpus + ffmpeg; no recorded run (F-001). |

## 7. Dead / duplicated / unreachable code

- No dead code found; no duplicated parsers (the ADTS grammar is imported, the PES assembler is imported).
- Contrast with mp4.rs F-003: this host is the model the MP4 host should follow (consume the demux crate's parsers, don't re-implement).

## 8. Dependencies & flags

- No feature gates: `mod ts;` unconditional (mod.rs:31), arm at mod.rs:72. Deps: demux (ts, adts, pes), media-types, plugin-api, commands. No cfg(test) branches in production code.

## 9. Open questions for the next agent

- Does the engine treat a repeated demux error as fatal (making F-002 a session-kill) or does it poll again (infinite re-raise)? Needs a run or the engine's error policy docs (playback-engine crate — outside my files).
- mkv.rs/mp4.rs h264-doc staleness vs ts.rs's updated doc: confirm which landing updated ts.rs but not its siblings (git history unavailable — adaptation).
