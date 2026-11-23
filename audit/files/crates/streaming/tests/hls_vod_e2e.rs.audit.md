# AUDIT: crates/streaming/tests/hls_vod_e2e.rs
| Field | Value |
|---|---|
| Auditor | W10-A09 |
| Date | 2026-11-23 |
| Lines / functions | 325 / 6 helpers + 2 tests |
| File verdict | REAL — genuine end-to-end over a real socket with real ffmpeg-generated fixtures and a real AAC decode; I ran both tests (pass). Weakness: the TS lane asserts only `pushes > 0` (no count oracle), and the code under test is unreachable from shipped entry points (see media.rs F-002). |
| Reachable from shipped entry point? | Test file (test-only by nature). The `open_vod` under test has no production caller — the shipped `lumen open` never routes an .m3u8 URL here (media.rs audit F-002, verified by rg). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/hls_vod_e2e-ebe3dbbd4d81a154 --test-threads=2 --nocapture` → 2 passed / 0 failed, exit 0, `ts lane: 375 PES pushes, 67437 payload bytes`; fixture ls (8 committed files); sed/cat quote + fixture verification |

## 1. One-paragraph verdict
This is a real e2e: committed ffmpeg 7.1.5-generated HLS fixtures (verified on disk: 4 TS segments + 4 ADTS segments + playlists) served by a real-socket mock HTTP server, consumed through `open_vod`, compared byte-exact against the on-disk concatenation, then demuxed (TsDemuxer with the PSI-scan rewind exercising the session's retained-prefix seek; AdtsDemuxer) and — in the ADTS lane — decoded through the real `codecs::audio::aac::decode_stream` to hard oracle numbers (346 packets, 354 304 samples/channel) that are arithmetically self-consistent (346 × 1024 = 354 304; ×2 bytes = 708 608 = the documented ffmpeg PCM measure). I ran it: 2/2 pass in 0.10 s. The gaps: the TS lane's only PES assertion is `pushes > 0` (any flow passes — the count oracle for TS is deferred to the CLI host, disclosed), and the whole proof exercises a session no shipped binary can open.

## 2. What actually works (verified)
— verified by `./target/debug/deps/hls_vod_e2e-ebe3dbbd4d81a154 --nocapture` → `test result: ok. 2 passed` (exit 0): byte parity `streamed == cat(segments)` both lanes, `size() == Some(total)` probe-fixed, TsDemuxer start over the session (1 aac track), 375 PES pushes measured, AdtsDemuxer 346 packets == oracle == independent in-test ADTS frame walk, decode 354 304 samples/channel, and the exact wire shape both sessions (playlist → 4× `bytes=0-0` probes → 4 ordered whole-body GETs, 18 requests total).

## 3. Findings
F-001 [S3]
Where: crates/streaming/tests/hls_vod_e2e.rs:120 (open_vod call) / repo-wide
Code:
```
    match open_vod(fetch, url, HlsVodConfig::default()) {
        Ok(stream) => stream,
        Err(e) => unreachable!("session must open: {e}"),
    }
```
What is wrong: the e2e proves a session the shipped binary can never create — `lumen open http://…/pl.m3u8` goes through ranged-HTTP + media-probe and dies at the probe stage (no `#EXTM3U` signature). A passing e2e here says nothing about user-visible HLS playback; the module header's "played through the existing demux + decode machinery" is true of the test harness only. Disclosed in the media.rs audit (F-002 there); repeated here because this file is the evidence string a reader would cite for "HLS VOD e2e works".
Confidence: HIGH — Verification: read + rg

F-002 [S4]
Where: crates/streaming/tests/hls_vod_e2e.rs:213-217
Code:
```
    assert!(
        pushes > 0,
        "PES payloads flowed: {pushes} pushes, {bytes} bytes"
    );
    eprintln!("ts lane: {pushes} PES pushes, {bytes} payload bytes (measured)");
```
What is wrong: the TS lane's demux-stage oracle is existence-only (`pushes > 0`). A regression that demuxed 3 of 346 AAC packets, or mangled every PES payload length, still passes this lane (byte-parity and start assertions aside — they do not inspect PES content). The comment at L157-162 honestly defers the TS AU-split differential to "the CLI host's pinned territory", so this is a disclosed weak assertion, not a hidden one — but the lane's name (`ts_lane_streams_byte_exact_and_demuxes_over_the_session`) promises more demux proof than `pushes > 0` delivers.
Confidence: HIGH — Verification: read + ran

F-003 [S4]
Where: crates/streaming/tests/hls_vod_e2e.rs:47-49
Code:
```
/// The ffmpeg-measured oracle (this session; see the module docs).
const ORACLE_SAMPLES_PER_CHANNEL: usize = 354_304;
const ORACLE_AAC_PACKETS: usize = 346;
```
What is wrong: the oracle constants are from a one-time ffmpeg run ("this session") and are not re-derivable by the test (no ffmpeg invocation); they pin the committed fixtures, which is their real function. Mitigations that keep this honest: the packet count is cross-checked by an independent in-test ADTS frame walker (L297), and the numbers are arithmetically consistent (346 × 1024 = 354 304; 354 304 × 2 = 708 608 = the documented `s16le` byte measure). The sample-count oracle still ultimately trusts the original measurement session.
Confidence: HIGH — Verification: read + arithmetic + ran

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| lane_bytes | 54-56 | REAL | — | committed fixtures via testkit |
| serve_lane | 59-74 | REAL | — | real-socket MockServer |
| drain_audio | 78-93 | REAL | — | collects packets + payload |
| count_adts_frames | 98-115 | REAL | — | independent in-test truth; loose sync check (b[1]&0xF0) acceptable for fixtures |
| open_session | 118-124 | REAL | — | MockClient over real socket |
| PesCounter (PesConsumer) | 131-155 | REAL | — | poisoned-lock → typed error, no unwrap |
| ts_lane_streams_byte_exact_and_demuxes_over_the_session | 164-249 | REAL | S4 | F-002 weak PES oracle; wire shape exact |
| adts_lane_is_byte_exact_and_decodes_to_the_ffmpeg_oracle | 255-325 | REAL | — | full decode differential; strongest lane |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ts_lane_streams_byte_exact_and_demuxes_over_the_session | PARTIAL | byte parity, size, track set, wire order/headers: YES; PES flow: only `> 0` (F-002) |
| adts_lane_is_byte_exact_and_decodes_to_the_ffmpeg_oracle | YES | exact packet count (two ways), exact decoded sample count, exact request shape |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "real ffmpeg-generated HLS fixtures served over a real socket" | hls_vod_e2e.rs:1-2 | REAL | fixtures on disk; ran, pass |
| "played through the existing demux + decode machinery" | hls_vod_e2e.rs:3-4 | REAL for the harness; UNREACHABLE in the shipped binary | F-001; media.rs F-002 |
| "708 608 bytes = 354 304 samples/channel (346 frames × 1024, mono)" | hls_vod_e2e.rs:18-19 | REAL (arithmetically consistent, decode hits it) | ran: samples/channel == 354 304 |
| "108 + 108 + 107 + 23 = 346 AAC packets" | hls_vod_e2e.rs:20-21 | REAL | in-test ADTS walk == 346 == demuxer count |

## 7. Dead / duplicated / unreachable code
None in-file. The `PesCounter` bridge is test-local by design (the CLI host owns the real bridge).

## 8. Dependencies & flags
streaming (open_vod, MockServer/MockClient), demux (ts, adts), codecs (aac decode_stream), plugin_api, testkit. No features. Runs in the default test lane (baseline cargo-test-workspace.txt shows hls_vod_e2e ok).

## 9. Open questions for the next agent
- The TS AU-split decode differential "pinned in the CLI host (6.OPW-ts)" — does that CLI-host test exist and run, or is the TS lane's decode proof actually missing repo-wide?
- If fixtures are ever regenerated, the constants fail loudly (good) — but the regeneration procedure (ffmpeg command lines at L9-13) is not a checked-in script.
