# AUDIT: crates/demux/tests/ogg_fixtures.rs
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-23 |
| Lines / functions | 640 / 7 (3 #[test]], 2 helpers, 2 structs + 2 consts) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — integration test (tests/ogg_fixtures.rs), compiled only under `cargo test -p demux`. Correct wiring. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | python3 byte-surgery verification of all 14 fixture files (chain==parentA||parentB True; trunc0..4 exact prefixes; flip1/4/16 single-bit XOR 0x01 at 68/2810/30; garbage0 = parent+4096×0xEE — all True); live `ffprobe -count_packets` on the chain fixture → 31 (exit 0); baseline cargo-test-workspace.txt:2379-2385 (4/4 tests ok) |

## 1. One-paragraph verdict
A genuinely strong acceptance suite. The three valid fixtures are
differential-verified against ffmpeg (codec, rate, granule, packet count, and
bit-exact payload MD5), the chained fixture is verified as the exact byte
concatenation of its two parents with the boundary, fresh track id, and
per-link clocks all pinned, and the 9 malformed derivatives pin exact typed
outcomes (bytes, packets, reasons, terminal, damage, granule discontinuities).
I re-verified every provenance claim in the module docs by byte surgery with
python3: all true. Only hygiene defects: dead bookkeeping and a silently
swallowed second Discontinuity.

## 2. What actually works (verified)
- python3 over tests/fixtures/generated: `chain==a+b: True`; trunc0-4 are
  exact parent prefixes at 12/40/1500/2803/3233; flip1/4/16 differ from the
  parent at exactly one byte (offset 68/2810/30, XOR 0x01); garbage0 is the
  parent + 4096 `0xEE` bytes. The doc's "deterministic byte surgery" claim is
  byte-true.
- Baseline cargo-test-workspace.txt:2382-2385: all four tests "ok" — the
  ffmpeg-truth differential, chain AC, derivative pins, and fuzz-lite sweep
  all executed (not skipped).

## 3. Findings
F-001 [S4]
Where: crates/demux/tests/ogg_fixtures.rs:95-112, 596
Code:
```
    let mut tracks = Vec::new();
...
                tracks.push(p.track.handle());
...
    let _ = &tracks;
```
What is wrong: `drain_ogg` collects per-packet track handles and then
discards them (`let _ = &tracks;`); the derivative test does the same with
the `tracks` from `start()` (line 596). Dead bookkeeping — if the intent was
to assert one-track-per-derivative, the assert is missing.
Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/demux/tests/ogg_fixtures.rs:310-314
Code:
```
            Ok(DemuxEvent::Discontinuity) if !boundary_seen => {
                boundary_seen = true;
                boundary_at_packet = u64::try_from(link0.len()).must();
            }
            Ok(DemuxEvent::Discontinuity) => {}
```
What is wrong: a second or third `Discontinuity` event is silently ignored —
if the demuxer spuriously emitted extra boundary events the test would still
pass. The chain contract implies exactly one boundary; the assert that would
pin that is absent. Blast radius: test honesty only.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| drain_ogg | 87-123 | REAL | S4 | F-001 dead tracks vec; 8-tuple return |
| Truth / TRUTHS | 126-172 | REAL | — | ffmpeg-measured, md5-pinned |
| every_ogg_fixture_matches_the_ffmpeg_truth | 174-271 | REAL | — | codec/rate/granule/counts/md5/damage all exact |
| ogg_chain_two_links_is_two_links_with_a_fresh_track | 278-391 | REAL | S4 | F-002 second boundary swallowed; otherwise exact |
| demuxed_payload | 395-414 | REAL | — | |
| Derivative / DERIVATIVES | 423-560 | REAL | — | 9 pinned outcomes, sticky-error replay checked |
| ogg_derivatives_stay_inside_the_typed_vocabulary | 562-606 | REAL | — | exact pins incl. damage footprint |
| ogg_fixtures_and_derivatives_survive_the_fuzz_harness | 611-640 | REAL | — | 14 fixtures + 11 trunc cuts + 1024-stride flips |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| every_ogg_fixture_matches_the_ffmpeg_truth | YES | md5 payload parity + counts vs ffmpeg truth |
| ogg_chain_two_links_is_two_links_with_a_fresh_track | YES (except extra boundaries) | byte-concat parity + boundary position + fresh track |
| ogg_derivatives_stay_inside_the_typed_vocabulary | YES | exact per-derivative pins + sticky replay |
| ogg_fixtures_and_derivatives_survive_the_fuzz_harness | NO (by design) | absence-of-panic contract; the pins live above |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| fixtures are ffmpeg products, chain = cat of parents | ogg_fixtures.rs:23-26 | REAL | python3: chain==a+b True |
| "truncations are exact parent prefixes ... flips are single-bit XORs 0x01" | ogg_fixtures.rs:70-72 | REAL | python3 byte-diff verified |
| "garbage0 appends 4096 0xEE bytes" | ogg_fixtures.rs:72-73 | REAL | python3 tail check True |
| ffmpeg leaks link-2 headers (31 vs our 28 packets) | ogg_fixtures.rs:54-58 | REAL | re-measured live: `ffprobe -count_packets ... ogg_chain_two_links.ogg` → `31` (exit 0); our side pinned at 28 + md5 |

## 7. Dead / duplicated / unreachable code
F-001's `tracks` collections (both sites). Nothing else.

## 8. Dependencies & flags
Integration test; uses demux::ogg public API, testkit fixture access,
tests/common (md5_hex, open_via_input). No feature gates. External ffmpeg not
required at run time (truth is recorded).

## 9. Open questions for the next agent
- The `flip16` pin (headless passthrough, 16 packets) is the only opaque-
  passthrough coverage in ogg — is that behavior asserted anywhere at the
  codec layer, or does an opus/vorbis-less build silently accept it?
