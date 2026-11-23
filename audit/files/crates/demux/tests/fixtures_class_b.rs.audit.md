# AUDIT: crates/demux/tests/fixtures_class_b.rs
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-02-13 |
| Lines / functions | 488 / 11 tests + 6 helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (integration test) — the code under test (AviDemuxer + avi walk) IS reachable via apps/cli/src/containers/avi.rs (see riff.rs audit) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p demux --test fixtures_class_b` → 11 passed / 0 failed (exit 0); rg cross-checks on fixture references; sed quote verification |

## 1. One-paragraph verdict
A genuine differential suite: ten malformed AVI derivatives (5 truncations, 4
bit-flip/garbage, 1 index-less) are opened through the REAL input access stack
(`open_via_input` → plugin-api seam), drained through the real demuxer, and their
packet payloads are SHA-256-diffed against hashes recorded by the independent xtask
fixture WRITER (tools/xtask/src/cmd/fixtures.rs:3039 hashes the frames it writes — not
demuxer output, so no self-fulfilling goldens). The hostile RIFF-size cases are exactly
here and they pass: lying RIFF/hdrl sizes (flip4) recover with counted damage, mid-payload
cuts (trunc3) deliver the readable prefix with three named containment reasons and EOS,
the last-byte cut (trunc4) salvages the index with a 17-byte damage accounting, and
noidx0 demuxes 120 packets + linear-scan seeks purely through the movi walk. Two pins
are looser than the file's own "flavors are PINNED here" claim.

## 2. What actually works (verified)
— `cargo test -p demux --test fixtures_class_b` -> 11 passed, 0 failed.
— Ground-truth independence verified: expectation hashes are writer-side
  (xtask fixtures.rs:3039 `sha256_hex(frame)` over the frames being written).
— absidx1/negheight0 (the two AVI derivatives NOT in this suite) are covered elsewhere:
  crates/playback-engine/tests/scenarios_b/scenario_09_corrupt.rs:131,135 and
  tests/integration/first-vertical/tests/recovery_matrix.rs:650,668 — no dead fixtures.

## 3. Findings
F-001 [S4]
Where: crates/demux/tests/fixtures_class_b.rs:385-388
Code:
```
    assert!(
        recovered >= 2,
        "riff-extends-past-stream + hdrl-size lies are counted (got {recovered})"
    );
```
What is wrong: The module doc (:7-8) claims "Per-file flavors are PINNED here"; trunc3 and
trunc4 pin their containment reasons BY NAME (exact arrays), but flip4 pins only a count
lower bound. A regression that swaps a flip4 recovery flavor (e.g. a resync where a clamp
belonged) passes this test. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/demux/tests/fixtures_class_b.rs:244-249
Code:
```
    let (last_track, last_len) = match packets.last() {
        Some((track, data)) => (*track, data.len()),
        None => unreachable!("61 packets were just counted"),
    };
    assert_eq!(last_track, 0, "the partial is the video frame-30 prefix");
    assert!(last_len > 0, "a non-empty readable prefix was delivered");
```
What is wrong: The trunc3 delivered-partial length is asserted only `> 0`, not the exact
byte count (cut_offset 212_512 vs the frame-30 chunk start is a deterministic number the
expectation file could pin). Any partial length would pass. Confidence: HIGH —
Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| derivative / derivative_expectation | 24-38 | REAL | — | loads committed fixtures via testkit |
| assert_truncated | 41-50 | REAL | — | exact expected/found + code demux.avi.truncated |
| drain / drain_with_reasons | 54-96 | REAL | — | terminal-preserving collectors |
| assert_video_hashes_match_parent / _prefix_ | 98-144 | REAL | — | SHA-256 differential vs writer-recorded hashes |
| trunc0_after_magic... | 150-171 | REAL | — | (425_516, 12) exact + sticky replay |
| trunc1_mid_avih... | 173-184 | REAL | — | (88, 52) exact |
| trunc2_at_first_movi_chunk... | 186-209 | REAL | — | tracks survive, 0 packets, (423_588, 324) |
| trunc3_mid_payload... | 211-267 | REAL | S4 | 61 packets, 3 reasons by name, EOS; F-002 |
| trunc4_last_byte... | 269-311 | REAL | — | 120 packets, full hash match, damage_total==17 exact |
| garbage0... | 317-336 | REAL | — | past-extent bytes invisible, recovered==0 |
| flip1... | 338-357 | REAL | — | advisory flip, identical output, 0 damage |
| flip4... | 359-389 | REAL | S4 | F-001: count-only pin |
| flip16... | 391-407 | REAL | — | demux.avi.not-avi + Unsupported{avi} |
| noidx0... | 413-468 | REAL | — | 120 packets via movi walk + tolerance/monotonic seeks |
| every_avi_fixture_survives_the_fuzz_harness | 474-487 | REAL | — | 11 files through fuzz_demux_avi, no panic |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| trunc0/1/2 | YES | exact (expected, found) pairs + sticky code |
| trunc3 | YES (mostly) | packet count, prefix hashes, exact reason array; partial length loose (F-002) |
| trunc4 | YES | exact damage 17 + full-stream hash + reason array |
| garbage0/flip1/flip16/noidx0 | YES | counts, hashes, typed error shape, seek grid |
| flip4 | PARTIAL | hashes exact; recovery pin is a lower bound (F-001) |
| fuzz-harness sweep | YES (panic only) | absence-of-panic contract, no output asserts |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "all ten generated malformed AVIs behave per their expectation/1 files" | :1-3 | REAL | 11/11 pass (I ran them); 10 derivatives + parent all asserted |
| "Per-file flavors are PINNED here" | :7-8 | PARTIAL | flip4 count-only (F-001), trunc3 partial-length loose (F-002) |

## 7. Dead / duplicated / unreachable code
None in this file. Cross-check: all 12 malformed AVI derivatives in
tests/fixtures/generated/malformed/ are referenced by some suite (class-B 10 here;
absidx1/negheight0 by playback-engine scenario_09 + first-vertical recovery_matrix).

## 8. Dependencies & flags
Integration test target; demux, media-types, plugin-api, testkit. No feature gates.
Uses the real input crate's FileAccess (`open_via_input`) — full-stack composition.

## 9. Open questions for the next agent
- avi/fuzz.rs's `fuzz_demux_avi` is the panic-contract backstop here — audit its mutation
  coverage (does it also truncate/mutate headers, or only whole-file drains?).
- Pin the trunc3 partial length and flip4 reason names (fixes F-001/F-002).
