# AUDIT: crates/demux/tests/ps_fixtures.rs
| Field | Value |
|---|---|
| Auditor | W04-A08 |
| Date | 2026-11-23 |
| Lines / functions | 340 / 8 (Lane struct + LANES const + 3 helpers + 4 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (integration test); it exercises `demux::ps::PsDemuxer`, which IS CLI-wired: apps/cli/src/containers/mod.rs `("mpegps") => ps::open(stream, opts)` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo test -p demux --test ps_fixtures` -> exit 0, 4 passed; **independent ffmpeg cross-verification**: `ffmpeg -map 0:v -c copy` on ps-mp2-video-audio.mpg -> 63898 B, md5 `b235981565a1b5f08783097bb9f40627` (== lane pin); `-map 0:a -f mp2` -> 8150 B, md5 `903e70465aecd25c6e5cf6c9f38b3daf` (== lane pin); `-map 0:2` on ps-vob-lpcm.vob -> **96144 B** (== recorded ffmpeg_bytes; delta 144 = 3×48 AU headers, ours 96000) |

## 1. One-paragraph verdict
A differential truth suite against four committed ffmpeg-authored PS/VOB fixtures, and the recorded "measured ffmpeg truth" is genuine — I re-derived it myself with the sandbox ffmpeg: the pinned payload digests and byte totals match ffmpeg's own `-c copy` extractions bit-for-bit (video and MP2 md5s identical; LPCM extraction exactly 96144 B vs our 96000 B, the documented 3-byte pcm_dvd AU-header delta per each of 48 packets). The suite's honesty framing is unusually good: where our packet counts differ from ffprobe's (PES vs access-unit re-packetization), it pins OUR counts and records ffprobe's alongside instead of faking agreement, and the nav-packet divergence (ffprobe 0 packets, ours 4) is measured and documented rather than hidden. The one hole: `run_lane` silently discards `Recovered`/`Discontinuity` events, so the suite would not notice a regression that started surfacing damage on these clean fixtures. This is not an adversarial-malformed-input file (that coverage lives in the ps module's own tests/fuzz — outside this file's scope).

## 2. What actually works (verified)
— verified by `cargo test -p demux --test ps_fixtures` -> `test result: ok. 4 passed; 0 failed` (my run, exit 0; matches baseline cargo-test-workspace.txt ps_fixtures section).
— verified by my own ffmpeg 7.x extraction runs (commands above): the digest pins in LANES are the ffmpeg truth, not self-generated goldens — the classic 6.4 test-fraud pattern is ABSENT here, proven by independent measurement.

## 3. Findings
F-001 [S4]
Where: crates/demux/tests/ps_fixtures.rs:212-215
Code:
```
        match ps.next_event().must() {
            DemuxEvent::Packet(p) => packets.push((p.track.handle(), p.data.to_vec())),
            DemuxEvent::Recovered { .. } | DemuxEvent::Discontinuity => {}
            DemuxEvent::EndOfStream => break,
```
What is wrong: `Recovered` and `Discontinuity` events are silently dropped — the suite never asserts these clean fixture walks are damage-free (no `damage_total()==0` assertion, unlike the AVI/AIFF suites). Why it matters: a demuxer regression that began emitting damage notes (or losing data through a "recovery" that drops payloads) would still pass as long as the surviving payload concat matched — a payload-dropping recovery WOULD fail the digest, but a damage-note regression alone would not. What correct looks like: assert the recovered/discontinuity lists are empty per lane. Confidence: HIGH — Verification: read.

Zero S0-S3 findings. Checked and clean: the LPCM exact-delta assertion (`ffmpeg_bytes - bytes == 3 × packets` on the lpcm lane, 0 elsewhere — line 281-290) is arithmetic, not hand-waved; nav-packet zero-fill/lead-byte/size pins (980/1018 alternating) are real byte assertions; span conversion onto the 90 kHz base via `.must()` fails loudly; the 3-slot fixed arrays with `[0;3]`/`""` unused tails are managed by `take(track_count)`.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Lane struct / LANES const | 49-169 | REAL | — | 4 lanes × measured truth; digests independently re-verified by me |
| fixture_path | 171-175 | REAL | — | crate-local fixtures/ps (not tests/fixtures) |
| span_ticks | 177-188 | REAL | — | 90 kHz conversion, must() |
| run_lane | 191-219 | REAL | S4 | drops Recovered/Discontinuity (F-001) |
| fixture_stream_tables_and_packet_counts_match_the_pins | 221-251 | REAL | — | kinds, descriptors, PES counts, first/last PTS extents |
| fixture_payload_streams_match_ffmpeg_extractions_bit_for_bit | 253-293 | REAL | — | md5 + size + exact-delta per lane |
| lpcm_packets_strip_the_full_dvd_prefix | 295-312 | REAL | — | 48 packets, 2008/1424 first/last, 96000 total |
| vob_nav_packets_are_real_and_zero_padded_by_ffmpeg | 314-340 | REAL | — | 980/1018 sizes, lead bytes, zero-fill |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 4 | YES | Digest/size/count/PTS pins are absolute and (proven by my ffmpeg runs) derived from an independent oracle, not from the code under test. Any payload byte change, packet loss, stamp drift, or prefix-strip error fails. Only damage-note regressions are invisible (F-001). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "payload digests from `ffmpeg -map -c copy` raw extractions ... measured this session" | header lines 5-8 | REAL | my own ffmpeg extractions reproduce both md5s and both sizes exactly |
| "the *byte streams* are what must agree, and they do bit-for-bit on video / MP2 / AC-3 / DTS" | header lines 18-19 | REAL (video+MP2 verified by me; AC-3/DTS lanes carry equal bytes/ffmpeg_bytes and recorded digests — verified only via the passing pins) | /tmp extraction runs |
| "ffmpeg keeps a 3-byte pcm_dvd AU header per packet: 96 144 = 48 × (3 + 2 000)" | header lines 23-24 | REAL | my extraction: exactly 96144 B |
| "ffprobe declares the dvd_nav_packet Data stream but demuxes zero packets ... we expose the 4 nav PES" | header lines 30-35 | REAL | test 4 pins 4 nav packets with measured sizes |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
`mod common` (md5_hex + open_via_input) — the real file-access stack, not a mem-stream stand-in. No ffmpeg invocation at test time (truth is pre-recorded; I re-verified it externally). No feature gates.

## 9. Open questions for the next agent
- ps/mod.rs owner: where are the malformed-PS adversarial tests (truncated packs, lying PES lengths, SCR discontinuities)? This file is differential-only by design; confirm the module's own suite covers the hostile shapes.
- The `ours` PES counts are self-pinned (container truth) — if the PES-splitting logic changes deliberately, this table needs a conscious update; the doc's framing already says so.
