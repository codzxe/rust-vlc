# AUDIT: crates/codecs/tests/flac_seek.rs
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-23 |
| Lines / functions | 258 / 3 helpers + 4 tests (support module tests/flac/mod.rs verified: real FlacDecoder, no mocks) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test: NO. `codecs::audio::flac::{SeekPlanner, parse_stream, frame_header_at}` is production seek code; FlacDecoder is wired in the shipped CLI (pipeline_host.rs:253 `"flac" =>`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/flac_seek-f28325ba6212a26f --nocapture` → 4 passed / 0 failed (0.96s; corpus lane SKIP). Then generated a matching corpus lane myself (`ffmpeg -f lavfi -i sine=500Hz:3s:88200 -c:a flac -sample_fmt s32 -bits_per_raw_sample 24` → no SEEKTABLE, block walk verified: types 0/4/1) and re-ran with `LUMEN_CORPUS` set → **4 passed (1.22s)** including `corpus_stream_seeks_by_bisection_alone` executing for real. |

## 1. One-paragraph verdict
A strong sample-accurate seek proof: for a bracketing set of targets (stream head, every SEEKTABLE point and +1, gap midpoints, the short tail block, the final sample) BOTH resolvers — the advisory SEEKTABLE path and the tableless bisection — must agree, land in the containing frame (verified against the frame's own parsed header, not the planner's claim), and the bytes from the plan's offset must decode bit-exactly to the full decode's suffix. The damaged-table test corrupts a real seek-point byte offset and requires the same answers via fallback; past-end and metadata-only streams are typed errors with exact context codes. I additionally executed the corpus lane (normally skipped) with a self-generated 88.2 kHz/24-bit tableless FLAC — bisection-only seeking verified bit-exact including tail-block targets. Only blemish: the corpus row's vacuous skip when the /tmp corpus is absent (default state).

## 2. What actually works (verified)
- 4/4 tests pass, and the corpus lane re-run with a real (self-generated) tableless FLAC also passes — bisection seeking at 6 targets with decode-suffix equality at 24-bit width.
- The carrier fixture's full target sweep (both resolvers, containment, suffix equality) executes in the default run.
- Typed-error paths verified: `flac.seek.past_end` (3 hostile targets incl. u64::MAX/2), `flac.seek.no_frame` (truncated metadata-only carrier).
- SEEKTABLE corruption: byte offset of point 2 shifted by +13 — parse still yields the lying table; resolution still correct at every target (verify-and-fallback is real, not decorative).

## 3. Findings
F-001 [S4]
Where: crates/codecs/tests/flac_seek.rs:219-223
Code:
```rust
    let Some(root) = corpus_dir() else {
        eprintln!("corpus: LUMEN_CORPUS unset and /tmp/my-project/corpus-v4 absent — SKIP");
        return;
    };
```
What is wrong: the corpus row reports `ok` without testing anything when the /tmp-resident corpus is absent — the default state (verified: default run skips it). 3 of 4 tests fully execute regardless, so the impact is one lane (the only bisection-only 24-bit coverage). Confidence: HIGH — Verification: ran (both modes).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| carrier | 23-25 | REAL | — | the SEEKTABLE fixture (44 frames, 6 points, short tail) |
| targets | 30-45 | REAL | — | bracketing sweep incl. tail-specific magic (198_144+100 — fixture-coupled but real) |
| assert_resolve | 49-94 | REAL | — | dual-resolver agreement + containment vs the frame's OWN header + bit-exact decode-suffix equality |
| seektable_and_binary_search_agree_across_the_stream (test) | 96-120 | REAL | — | executed, passing |
| corrupted_seektable_degrades_to_bisection (test) | 124-174 | REAL | — | real in-place byte corruption of point 2's offset; same answers required |
| past_end_and_frameless_streams_are_typed_errors (test) | 176-212 | REAL | — | exact codes asserted |
| corpus_stream_seeks_by_bisection_alone (test) | 218-257 | REAL | S4 | F-001; verified live with a generated corpus file |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| seektable_and_binary_search_agree... | YES | any resolver disagreement, off-by-one containment, or one wrong sample in the decoded suffix fails the byte-equality |
| corrupted_seektable_degrades_to_bisection | YES | a planner that trusted the lying row would resolve a wrong frame and fail containment/suffix |
| past_end_and_frameless... | YES | exact error codes and kinds |
| corpus_stream_seeks_by_bisection_alone | YES (with corpus) / N/A (absent) | same assert_resolve machinery at 24-bit width |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "both resolvers ... return the containing frame ... and the bytes from that frame onward decode to exactly the full decode's suffix" | header L4-10 | REAL | executed (both resolvers, all targets, byte-equality asserts) |
| "A damaged seek table must degrade to the same answer via bisection, never a wrong seek" | header L11-13 | REAL | corruption test passes |
| "a target past the stream's declared sample range and a metadata-only stream are typed errors" | header L12-13 | REAL | exact codes verified |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Workspace crates (codecs::audio::flac, testkit) + tests/flac/mod.rs helpers (real decoder drives; verified no mocks). Env knob LUMEN_CORPUS. No feature gates.

## 9. Open questions for the next agent
- crates/codecs/src/audio/flac/seek.rs (the SeekPlanner itself) is not in my assignment — the bisection's iteration bound and the verify rule's exact predicate deserve their own function-level audit; this suite pins their observable behavior only on two fixture shapes.
