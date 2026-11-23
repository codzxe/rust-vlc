# AUDIT: crates/subtitles/tests/cc_acceptance.rs
| Field | Value |
|---|---|
| Auditor | W09-A03 |
| Date | 2026-11-15 |
| Lines / functions | 491 / 4 helpers + 11 tests |
| File verdict | REAL (acceptance tests execute real decode paths; honest about no reference corpus) — but the crate it accepts is UNREACHABLE from shipped hosts |
| Reachable from shipped entry point? | N/A (test file); the tested code (`subtitles::cc`) is compiled by NO shipped host (fuzz/ only) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | prebuilt `target/debug/deps/cc_acceptance-28b72722d6989f9e` -> 11 passed / 0 failed, 3.48s (exit 0); same for `-4dabb7ab8134629d` -> 11 passed; sed quote verification |

## 1. One-paragraph verdict
This is a genuine acceptance suite, not a tautology farm: 11 tests drive hand-assembled ATSC
A/53 / CTA-708 byte vectors through the real decoder and assert observable state (grid text
per row, cell colors, cue spans, bitmap pixels, anchor rationals, damage variants). The
header is honest that no reference corpus exists ("No reference corpus exists in the restored
sandbox (the N2 honesty-gap precedent: no ffmpeg text→CC transcode path)") and compensates
with grammar round-trips + fuzz-lite + systematic truncation. All 11 pass (executed). The
structural caveat: it accepts a library no shipped product compiles, and every expected value
is authored by the decoder's own authors (self-pinned goldens) — the best available here, but
it is not independent evidence.

## 2. What actually works (verified)
— verified by `./target/debug/deps/cc_acceptance-28b72722d6989f9e` ->
`test result: ok. 11 passed; 0 failed; ... finished in 3.48s` — includes roll-up scrolling
across 3 frames, PAC red reaching actual bitmap pixels `(255, 0, 0, 255)` at (22,5), pop-on
double-buffering until `End Of Caption`, parity-drop + state-intact recovery, XDS skip,
CC2 channel latch, 708 service-1 routing to 608, SCTE-20 bit-packing, 200KB fuzz-lite, and
a full truncation sweep.

## 3. Findings
F-001 [S4]
Where: crates/subtitles/tests/cc_acceptance.rs:87-99 (and all `.must()` uses)
Code:
```
    let builder = CcCueBuilder::new();
    let c1 = builder.cues(&u1, Some(pts[1]), 0).must();
```
What is wrong: `CcCueBuilder::cues` is only ever exercised on the happy path (`.must()`
unwraps); its typed error branches have zero acceptance coverage in this file. Why it
matters: an acceptance suite that never sees the builder fail proves half the contract.
What correct looks like: one vector that makes `cues` return Err and asserts the variant.
Confidence: HIGH — Verification: read (rg `.must()` / `cues(` in this file).

F-002 [S4]
Where: crates/subtitles/tests/cc_acceptance.rs:4-12
Code:
```
//! The vectors are hand-assembled per the ATSC A/53 Part 4 / CTA-708
//! grammars (the parity bits are computed by the crate's own
//! [`subtitles::cc::eia608::with_odd_parity`], which is itself
//! pinned by the unit suite). No reference corpus exists in the
```
What is wrong: nothing false — but the suite's expected outputs (22x32 px cells, anchor
1/15 and 1/32, row numbering) are all constants of the implementation under test, i.e.
self-pinned. Combined with zero shipped consumers of `subtitles`, this "acceptance" accepts
an island. Recorded so the matrix does not cite this file as user-facing CC support.
Confidence: HIGH — Verification: read + Cargo.toml scan (only fuzz/ depends on subtitles).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| p / t608 / ga94 / rollup_frames (helpers) | 23-56 | REAL | — | build grammar-correct vectors; parity via crate fn (self-pinned, admitted) |
| classic_rollup_scrolls_through_the_full_stack | 58-99 | REAL | — | 3-frame scroll + half-open cue spans + open-ended end |
| ga94_sei_carriage_end_to_end | 101-110 | REAL | — | SEI type-4 -> decode_ga94 |
| pac_colors_reach_the_bitmap | 112-152 | REAL | — | pixel-level red + anchor rationals |
| pop_on_double_buffers_until_end_caption | 154-175 | REAL | — | hidden buffer, EOC swap, erase -> no cue |
| parity_failure_is_typed_and_dropped | 177-199 | REAL | — | Parity damage + state-intact follow-up |
| xds_and_channel_latch | 201-220 | REAL | — | XdsSkipped + CC2 latch |
| cea708_service1_routes_to_608_and_native_is_recorded | 222-298 | REAL | — | svc1->608, svc5 recorded, DtvContinuationWithoutStart, DefineWindow op |
| unknown_service_and_truncated_carriage_are_typed | 300-341 | REAL | — | UnknownService708{0}, Truncated, BadMarker{0x7E} |
| scte20_carriage_end_to_end | 343-383 | REAL | — | hand bit-packed SCTE-20 -> "ok" |
| Lcg (xorshift64) | 385-400 | REAL | — | seeded, deterministic |
| fuzz_lite_survival_seeded_mutations | 402-466 | REAL | — | 200KB budget, 4 mutation kinds, damage cap asserted |
| systematic_truncation_sweep_survives | 468-491 | REAL | — | every cut of every seed through fuzz_sub_cc |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 11 | YES (mutation-reasoned) | each asserts concrete decoder state (row text, colors, pixels, damage variants), not just "no panic" — except the two survival tests, which assert only survival + damage cap (that is their stated purpose) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no ffmpeg text→CC transcode path" (no corpus) | file header :7-9 | REAL (honest limit) | consistent with baseline; no CC fixtures in repo (fixtures dir has only ass/srt/fuzz) |
| "seeded-LCG fuzz-lite survival sweep (< 200 KB total input)" | :11-12 | REAL | L385 says xorshift64 (naming nit), budget asserted `total > 100 * 1024` at :465, executed pass |

## 7. Dead / duplicated / unreachable code
None in-file. The file's entire value is gated on the island-crate problem (see F-002).

## 8. Dependencies & flags
Dev-consumer of `subtitles` + `testkit::Must`. No features. No unsafe.

## 9. Open questions for the next agent
- Does `cc/mod.rs` `CcCueBuilder::cues` have reachable Err branches at all? (If not, F-001
  is moot and the builder is a bigger problem.)
- Is there any plan artifact wiring CC into lumen-cli (e.g., an MP4/MPEG-TS SEI -> cc_data
  consumer)? If none exists, CC decoding shares the ASS verdict: library-only.
