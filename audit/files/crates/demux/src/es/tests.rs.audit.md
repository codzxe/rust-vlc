# AUDIT: crates/demux/src/es/tests.rs
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-21 |
| Lines / functions | 365 / 16 (5 helpers/builders, 11 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (cfg(test)); subjects under test ARE shipped-reachable: AnnexbVideoDemuxer via containers "h264"/"hevc" arms, RawMp3Demuxer via "mp3"/"mp2", aac::open via "adts" (apps/cli/src/containers/mod.rs:63-64,70-71,51) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |

## 1. One-paragraph verdict
A genuinely strong test file: the AU-boundary tests assert exact packet counts, exact
byte math (`4*4 + 2 + 2 + 2 + 4`), contiguous coverage, keyframe flags, and
stamps-Missing honesty; the MP3 lane pins the sample clock tick-by-tick
(`i * 1152`), the ID3 skip byte count, the truncated-frame sticky error, and the
resync `Recovered` event; all 11 tests plus the 4 ffprobe-differential corpus tests
(es_corpus.rs) pass in the baseline. The one real defect is inside the fuzz-lite test:
a tautological `assert!(round < 512)` wearing the comment "The mutation must bite" —
it verifies nothing (round is the loop induction variable), so the test's implicit
claim that mutations actually changed the corpus is unchecked. Two smaller notes: the
test name promises "never … hang" but no timeout enforces it, and `mp3_frames` builds
only valid frames (fine), leaving the no-sync lanes to the dedicated garbage tests
(also present).

## 2. What actually works (verified)
- All 11 module tests pass: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt`
  lines 1680-1690 (`es::tests::*` … ok), and the ffprobe differential
  `tests/es_corpus.rs` → "4 passed; 0 failed" (same log, lines 2019-2027) — the
  module doc's claim that the corpus differential "lives in tests/es_corpus.rs"
  (tests.rs:3-4) is REAL: the file exists and its tests ran green.
- Mutation-sensitivity of the core tests verified by reasoning (see §5) — e.g.
  `h264_aus_follow_the_nal_type_rule` fails if the AU boundary rule regresses
  (packet count 4 → 5 or 3) or if bytes are dropped (`total == stream.len()`).
- House no-unwrap rule kept via `unwrap_or_else(|e| unreachable!(...))` and
  `testkit::Must` throughout.

## 3. Findings

### F-001 [S3] Tautological assert disguised as a mutation-effectiveness check
Where: crates/demux/src/es/tests.rs:347-348
Code:
```rust
        // The mutation must bite.
        assert!(round < 512);
```
What is wrong: `round` is the induction variable of `for round in 0..512`
(tests.rs:320) — `round < 512` is true by construction on every iteration. The assert
can never fire; it is dead. The comment claims it verifies "the mutation must bite",
i.e., that the LCG mutations actually changed the corpus — nothing of the sort is
checked. A regression that made `next()` return a constant (e.g., a bad xorshift
refactor) would leave every round mutating zero meaningful bytes and the fuzz-lite
test would still pass, draining the SAME unmutated base 512 times.
Why it matters: This is exactly the "test asserting nothing" class the audit hunts:
the one line that claims to guard fuzz effectiveness guards nothing, and its comment
actively misleads. The fuzz-lite lane is the only in-crate hostility coverage for the
raw-ES demuxers.
What correct looks like: `assert_ne!(bytes, base, "round {round}: mutation must bite")`
after the mutation loop (cheap — 512 comparisons of a ~1.3 KB vector), or drop the
comment.
Confidence: HIGH — Verification: sed -n '320,348p' (quote verified); loop variable
range read at :320.

### F-002 [S4] Test name promises "never … hang" with no timeout enforcing it
Where: crates/demux/src/es/tests.rs:302-303
Code:
```rust
/// Fuzz-lite (`+FB`, doc 04 §8.5): a seeded LCG mutates a small
/// real-shaped stream (flips, truncations, wild headers, bursts);
```
(the name at :303: `fn fuzz_lite_mutations_never_panic_or_hang()`)
What is wrong: `drain` loops until EndOfStream or Err (tests.rs:55-68). If a mutated
stream ever made a demuxer loop without consuming bytes, the test would hang forever
— `cargo test` has no per-test timeout, and none is configured here. "Never hang" is
asserted by nothing; only a panic or a green run is observable.
Why it matters: The name claims a property the harness cannot detect; a future
non-progress bug in an ES demuxer would surface as a CI stall, not a typed failure.
What correct looks like: A drain-iteration cap inside `drain` (e.g., fail after 10_000
events) — one line, converts hangs into failures.
Confidence: HIGH — Verification: read of drain (43-70) and the test body (303-365);
no timeout mechanism present.

### F-003 [S4] `mp3_frames` builder hard-codes the 417-byte frame size with no derivation check
Where: crates/demux/src/es/tests.rs:191-196
Code:
```rust
fn mp3_frames(n: usize, payload: u8) -> Vec<u8> {
    let mut out = Vec::new();
    for _ in 0..n {
        out.extend_from_slice(&MP3_HEAD);
        out.resize(out.len() + 417 - 4, payload);
```
What is wrong: 417 is the MPEG-1 L3 128 kbps 44.1 kHz frame size
(⌊144·128000/44100⌋ = 417) — correct, but magic. If the header constant `MP3_HEAD`
were ever edited to another bitrate/sampling (say 0xFF 0xFB 0x90 → 0xE0), the frames
would silently mis-frame and the tests would fail for the WRONG reason (parser
resyncs, not builder correctness), or worse, pass by accident against a parser that
trusts declared sizes.
Why it matters: Test-builder correctness is the ground truth these tests assert
against; a builder that can drift from its header unnoticed undermines that.
What correct looks like: A debug_assert or comment tying 417 to MP3_HEAD's
bitrate/sampling bits, or computing the size from the header bits.
Confidence: MEDIUM — Verification: sed -n '191,198p' (verified, quote is verbatim);
frame-size math checked against the MPEG-1 L3 tables (128 kbps, 44.1 kHz → 417).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `mem` | 13-19 | REAL | — | Shared in-memory seam (avi test_support), no mock of the subject. |
| `push_nal` | 22-25 | REAL | — | 4-byte start-code builder. |
| `h264_slice` | 29-33 | REAL | — | Type-1/5 header + ue(0) first-MB bit; honest shape. |
| `Walk` type | 36-40 | REAL | — | Data shape. |
| `drain` | 43-70 | REAL | S4 | Collects packets/events/terminal error; unbounded until EOS/Err — the hang blindness of F-002 lives here. |
| `h264_aus_follow_the_nal_type_rule` | 72-105 | REAL | — | Strongest test: exact AU count, byte math, coverage, keyframe flags, stamps-Missing, aus_demuxed counter. |
| `hevc_aus_group_on_first_slice_flag_and_vps` | 107-133 | REAL | — | First-slice-flag + mid-VCL param-set boundary lanes; coverage assert. |
| `h264_no_sync_is_typed_unsupported_after_scan` | 135-155 | REAL | — | demux.es.no-sync + stickiness (second start replays). |
| `empty_source_refuses_at_construction` | 157-173 | REAL | — | Both annexb and mp3 constructors. |
| `next_event_before_start_is_typed` | 175-184 | REAL | — | demux.es.not-started. |
| `MP3_HEAD` / `mp3_frames` | 189-198 | REAL | S4 | F-003 magic 417. |
| `mp3_walks_frames_with_exact_sample_clock` | 200-231 | REAL | — | ID3 skip = 16, pts ladder i×1152, 44_100/1152 head facts, descriptor "mp3". Strong. |
| `mp3_truncated_final_frame_is_typed_sticky` | 233-254 | REAL | — | 1 packet + demux.es.truncated Corrupt/Truncated kind + sticky replay. |
| `mp3_no_sync_is_typed_unsupported_after_scan` | 256-269 | REAL | — | 300 garbage bytes → no-sync Unsupported. |
| `mp3_midstream_garbage_resyncs_with_recovered_event` | 271-281 | REAL | — | 7-byte garbage → recovered:7 event, both frames emitted. |
| `aac_lane_is_the_landed_adts_demuxer` | 283-296 | REAL | — | aac::open over the adts demo stream; 3 frames; probe smoke. |
| `fuzz_lite_mutations_never_panic_or_hang` | 302-365 | REAL | S3/S4 | Real mutation loop (flip/wild/truncate/zero-burst, 512 rounds, xorshift64); F-001 tautology, F-002 hang blindness. Drain results intentionally discarded (`let _ =`) — correct for a survival sweep. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| h264_aus_follow_the_nal_type_rule | YES | AU miscount, dropped bytes, wrong keyframe flag, or a minted pts each break a concrete assert. |
| hevc_aus_group_on_first_slice_flag_and_vps | YES | Same class: 3 AUs + coverage assert. |
| h264_no_sync_is_typed_unsupported_after_scan | YES | Code + kind + stickiness all asserted. |
| empty_source_refuses_at_construction | YES | Both constructors. |
| next_event_before_start_is_typed | YES | Code pinned. |
| mp3_walks_frames_with_exact_sample_clock | YES | Tick-exact ladder; any clock drift (1151/1153) fails. |
| mp3_truncated_final_frame_is_typed_sticky | YES | Packet count 1 + typed error + sticky. |
| mp3_no_sync_is_typed_unsupported_after_scan | YES | Code + kind. |
| mp3_midstream_garbage_resyncs_with_recovered_event | YES | recovered:7 prefix assert. |
| aac_lane_is_the_landed_adts_demuxer | YES | 3-frame count + frames_demuxed + probe. |
| fuzz_lite_mutations_never_panic_or_hang | PARTIAL | Catches panics only; the "must bite" effectiveness check is the F-001 tautology; hangs invisible (F-002). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The real-corpus differential against ffprobe lives in tests/es_corpus.rs" | tests.rs:3-4 | REAL | File exists; 4 tests "ok" in baseline (nofailfast log lines 2019-2027). |
| "every outcome must be Ok-or-typed — no panic, no hang" | tests.rs:300-301 | PARTIAL | Panics: enforced by the run. Hangs: not enforced (F-002). Ok-or-typed: enforced only by absence of panic; drain results are discarded by design. |
| "The mutated corpus stays under 200 KB by construction" | tests.rs:301-302 | REAL | assert at :311 (`base_len < 200_000`); mutations only shrink or flip bytes. |

## 7. Dead / duplicated / unreachable code
- F-001's assert is dead code.
- No duplication; helpers all used.

## 8. Dependencies & flags
- cfg(test); uses `testkit::Must` (workspace test kit), crate-internal test_support
  seams (avi::test_support::mem_stream, adts::test_support). No feature gates. The
  subjects (es::annexb, es::mp3, es::aac → adts) are unconditional demux modules
  wired into the shipped registry.

## 9. Open questions for the next agent
- es/annexb.rs and es/mp3.rs (not my files): the tests here assert `Stamp::Missing`
  for annexb — confirm the container host (containers/es.rs) synthesizes timing for
  the raw lanes rather than relying on the demuxer's missing stamps.
- fuzz_loop_stable.rs (tests/) vs this in-crate fuzz-lite: is the corpus-based fuzz
  run in the baseline? (`tests/fuzz_loop_stable.rs` exists; check its baseline lines
  if assigned to you.)
