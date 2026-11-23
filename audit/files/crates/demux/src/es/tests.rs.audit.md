# AUDIT: crates/demux/src/es/tests.rs
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-13 |
| Lines / functions | 366 / 15 (11 tests + 4 helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — #[cfg(test)] module (es/mod.rs:49-50); tests the shipped es demuxers |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `cargo test -p demux --lib es::tests` → 31 matched (incl. ts::pes::tests substring), 0 failed; earlier targeted runs: 4 mp3 tests pass, fuzz_lite passes |

## 1. One-paragraph verdict
Real and mostly strong unit coverage of the three ES lanes: the H.264/HEVC tests pin
exact access-unit counts, exact packet byte lengths, keyframe flags, contiguous
byte coverage and the no-stamp timing posture; the MP3 tests pin packet geometry,
sample-clock pts (i×1152), the sticky typed truncation, no-sync refusal and the
resync Recovered event — all executed and passing. One test line is a lie by
labelling: fuzz_lite's "The mutation must bite." guards a vacuous
`assert!(round < 512)` on the loop counter — mutation efficacy is never actually
checked. The drain() helper's break-on-EndOfStream mirrors (and hides) the
production post-EOS event-swallowing defect I filed against mp3.rs. The aac lane
test exercises a synthetic demo stream only (wiring smoke, not a real differential).

## 2. What actually works (verified)
- `cargo test -p demux --lib es::tests` → 0 failed (all 11 in-module tests pass;
  filter also substring-matches 20 ts::pes::tests, all passing).
- Mutation reasoning: h264_aus… would fail on any AU-boundary regression (asserts
  4 AUs, per-packet exact lengths 4×4+2+2+2+4 etc., sum == stream.len()).
- mp3_walks… asserts pts ticks (i as i64)×1152 and id3_skipped()==16 — fails on any
  clock or tag-skip regression (verified passing by execution).
- Empty-source and not-started contract pins pass by execution.

## 3. Findings
F-001 [S3]
Where: crates/demux/src/es/tests.rs:347-348
Code:
```
        // The mutation must bite.
        assert!(round < 512);
```
What is wrong: `round` is the loop induction variable of `for round in 0..512`
(tests.rs:320), so `round < 512` is a tautology — it can never fail. The comment
promises a check that the mutation actually changed the corpus; no such check
exists (no `bytes != base` comparison). A mutation loop that silently became a
no-op (e.g., a future refactor breaking `next()`) would still "pass". The no-panic
contract itself is real (drains run), but this line is fabricated evidence by
labelling. What correct looks like: `assert_ne!(bytes, base)` (after accounting for
the truncate-to-same-length mutation), or delete the pretense. Confidence: HIGH —
Verification: read.
F-002 [S4]
Where: crates/demux/src/es/tests.rs:55-58
Code:
```
    loop {
        match demux.next_event() {
            Ok(DemuxEvent::Packet(p)) => packets.push(p),
            Ok(DemuxEvent::EndOfStream) => break,
```
What is wrong: the shared drain() helper stops at EndOfStream, so any event a
demuxer queues after EOS (mp3.rs pushes final Recovered events that only surface
post-EOS — see my mp3.rs F-002) is unobservable by every test using drain(). The
harness blind spot matches the production defect exactly. Confidence: HIGH —
Verification: read.
F-003 [S4]
Where: crates/demux/src/es/tests.rs:176-184
What is wrong: the not-started contract (`demux.es.not-started`) is pinned only for
AnnexbVideoDemuxer; RawMp3Demuxer has the identical guard (mp3.rs:662-664) with no
in-crate test. Confidence: HIGH — Verification: read + rg (no mp3 not-started test).
F-004 [S4]
Where: crates/demux/src/es/tests.rs:283-296
Code:
```
fn aac_lane_is_the_landed_adts_demuxer() {
    let bytes = crate::adts::test_support::demo_stream();
```
What is wrong: the sole in-crate AAC lane test runs on test_support's self-authored
demo stream (3 frames) — a wiring smoke asserting the lane opens and counts its own
fixture. Real-world ADTS truth lives only in tests/es_corpus.rs (external, not this
file). The test is honest about what it checks ("the landed demo stream") but the
coverage name promises a lane, the body proves a smoke. Confidence: HIGH —
Verification: read; es_corpus.rs exists (rg).

## 4. Per-function table (file is 366 LOC; tests + helpers)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mem | 13-19 | REAL | — | shared in-memory seam |
| push_nal | 22-25 | REAL | — | 4-byte start codes |
| h264_slice | 29-33 | REAL | — | header + first_mb bit |
| Walk/drain | 36-70 | REAL | S4 | EOS break hides post-EOS events (F-002) |
| h264_aus_follow_the_nal_type_rule | 72-105 | REAL | — | exact lengths/flags/coverage; executed |
| hevc_aus_group_on_first_slice_flag_and_vps | 107-133 | REAL | — | executed |
| h264_no_sync_is_typed_unsupported_after_scan | 135-155 | REAL | — | sticky refusal pinned |
| empty_source_refuses_at_construction | 157-173 | REAL | — | both demuxers |
| next_event_before_start_is_typed | 176-184 | REAL | S4 | annexb only (F-003) |
| mp3_frames/MP3_HEAD | 186-198 | REAL | — | 417 B no-padding synthetic frames |
| mp3_walks_frames_with_exact_sample_clock | 200-231 | REAL | — | executed; strong pins |
| mp3_truncated_final_frame_is_typed_sticky | 233-254 | REAL | — | executed; kind+code+sticky |
| mp3_no_sync_is_typed_unsupported_after_scan | 256-269 | REAL | — | executed |
| mp3_midstream_garbage_resyncs_with_recovered_event | 271-281 | REAL | — | executed |
| aac_lane_is_the_landed_adts_demuxer | 283-296 | REAL | S4 | demo-stream smoke (F-004) |
| fuzz_lite_mutations_never_panic_or_hang | 302-365 | REAL | S3 | tautology assert (F-001); no-panic contract real |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| h264_aus_follow_the_nal_type_rule | YES | exact AU count/lengths/flags/coverage pins |
| hevc_aus_group_on_first_slice_flag_and_vps | YES | same-strength pins |
| h264_no_sync_is_typed_unsupported_after_scan | YES | error kind + code + stickiness |
| empty_source_refuses_at_construction | YES | construction refusal, both lanes |
| next_event_before_start_is_typed | YES (annexb only) | code pin; mp3 lane uncovered |
| mp3_walks_frames_with_exact_sample_clock | YES | count/size/pts/id3/codec pins |
| mp3_truncated_final_frame_is_typed_sticky | YES | error taxonomy + sticky replay |
| mp3_no_sync_is_typed_unsupported_after_scan | YES | code pin |
| mp3_midstream_garbage_resyncs_with_recovered_event | YES | recovered:7 pin |
| aac_lane_is_the_landed_adts_demuxer | WEAK | counts its own demo fixture; wiring only |
| fuzz_lite_mutations_never_panic_or_hang | PARTIAL | catches panics/hangs; "must bite" line proves nothing (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every outcome must be Ok-or-typed — no panic, no hang" | tests.rs:298-300 | REAL | drains run without unwrap on outcomes; executed |
| "The mutation must bite." | tests.rs:347 | FALSE | tautological assert (F-001) |
| "The mutated corpus stays under 200 KB by construction" | tests.rs:300-301 | REAL | assert at :311 (base_len < 200_000) — real bound |
| "synthetic framing truth, the typed error paths, and the fuzz-lite survival sweep" | tests.rs:1-3 | REAL | verified by execution |

## 7. Dead / duplicated / unreachable code
None dead. Note: drain()'s Walk tuple type alias (36-40) is used; events format
strings are consumed by asserts.

## 8. Dependencies & flags
#[cfg(test)] gated (es/mod.rs:49-50); uses testkit::Must, plugin_api, in-crate
test_support modules. No external deps.

## 9. Open questions for the next agent
- Does tests/es_corpus.rs cover mp3 not-started and the 0139 Xing-skip claim? (It
  is outside my assignment list; whoever owns es_corpus.rs should confirm.)
