# AUDIT: crates/streaming/tests/hls_master_e2e.rs
| Field | Value |
|---|---|
| Auditor | W10-A01 |
| Date | 2026-11-15 |
| Lines / functions | 272 / 3 tests + 5 helpers |
| File verdict | REAL — honest wire-oracle ABR e2e; one under-asserted doc claim |
| Reachable from shipped entry point? | NO — `streaming::hls::master::open_master` unreachable from apps (same wave-level gap as net_interrupted) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `cargo test -p streaming --test hls_master_e2e` → 3 passed/0 failed (exit 0, 0.07s); sed quote checks |

## 1. One-paragraph verdict
Real-socket e2e for master-playlist selection and ABR switching: a 3-rung ladder
(2M/800k/200k) served by mockhttp, opened through the real `open_master`
adaptive session, bandwidth scripted through the production `note_throughput`
seam, and the SERVER'S REQUEST LOG used as the oracle for which variant fed each
segment. Tests 1 and 3 are exactly pinned (selection order, zero switches inside
the hysteresis band with the wire proving lo-only throughout); test 2 pins
timeline continuity (media_sequence +1 across every seam, discontinuity flags,
size accounting) but under-asserts its headline "one rung per boundary" claim —
a hi→lo skip on the way down would still pass. I ran it: 3/3 pass.

## 2. What actually works (verified)
— verified by `cargo test -p streaming --test hls_master_e2e` → 3 passed; 0
failed (exit 0). Throughput math cross-checked by hand: note_throughput(75_000
bytes, 1e9 ns) = 0.6 Mbps, (50_000, 1e9) = 0.4 Mbps — both inside the stated
band (0.16–1.0 Mbps).

## 3. Findings
F-001 [S3]
Where: crates/streaming/tests/hls_master_e2e.rs:197-205
Code:
```
    assert!(runs.len() >= 3, "at least hi/mid/lo runs: {runs:?}");
    assert!(runs[0].0 == "hi", "starts on the selected rung");
```
What is wrong: The test's doc claim (L13-15) is "a bandwidth crash steps the
ladder down at segment boundaries (one rung per boundary)" — i.e. hi→mid→lo,
never skipping a rung. The assertions only require ≥3 runs, first run "hi", and
"lo" somewhere. A buggy ABR that drops hi→lo directly (skipping mid), then
recovers lo→mid, yields runs [hi, lo, mid] = 3 runs and PASSES. The
one-rung-per-boundary property is promised, not proven. What correct looks like:
assert the exact expected run sequence (or at least that every adjacent run pair
differs by exactly one rung index). Confidence: HIGH — Verification: read
(mutation reasoning; not executed).
F-002 [S3]
Where: crates/streaming/tests/hls_master_e2e.rs:32 (whole file)
Code:
```
use streaming::hls::master::{AbrConfig, HlsMasterConfig, open_master};
```
What is wrong: Library-only coverage: no shipped entry point reaches
`streaming::hls::master` (rg over apps/: zero `hls::` hits). The ABR
watermark/hysteresis machinery is proven for a lane the product never opens
(same wave-level reachability gap as mpd.rs F-001 / net_interrupted F-001).
Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/streaming/tests/hls_master_e2e.rs:173 and 218
Code:
```
    let switches_at_bottom = stream.stats().variant_switches;
```
and later `let _ = switches_at_bottom;`
What is wrong: Dead observation — captured, never asserted (silenced with
`let _`). Either assert it or delete it. Same for `let _ = all;` at L271 (the
byte accumulator in test 3 is written but never checked — unlike test 2, where
`all.len()` feeds the size assertion). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| master_text / variant_playlist | 47-70 | REAL | — | 3-rung ladder + EXT-X-MEDIA audio group (alternate rendition present but unselected — matches matrix note) |
| serve_ladder | 74-93 | REAL | — | distinct segment sizes per rung (H500/M200/L50) so the wire oracle is legible |
| segment_fetches | 97-104 | REAL | — | request-log → variant prefix per segment fetch |
| config | 108-116 | REAL | — | test owns the cap, not the [PROPOSAL] default |
| master_selects_highest_variant_under_cap | 121-140 | REAL | — | exact first-4 request paths; no-lo invariant |
| scripted_drop_and_rise_fetch_the_right_variants | 147-219 | REAL | S3 (F-001) | guard-bounded starvation loop; run-encoding too loose |
| in_band_oscillation_never_switches_again | 226-272 | REAL | S4 (F-003) | 12 alternations in-band; switches-before == switches-after + all-lo wire slice |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| master_selects_highest_variant_under_cap | YES | exact request order + selection index + bandwidth stat |
| scripted_drop_and_rise… | PARTIALLY | timeline/continuity/size: yes; "one rung per boundary": no (F-001) |
| in_band_oscillation_never_switches_again | YES | switch-count equality + every post-crash segment fetch is lo (wire-proven) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the server's request log is the oracle for which variant actually fed each segment" | hls_master_e2e.rs:15-17 | REAL | segment_fetches over server.requests() |
| "one rung per boundary" | hls_master_e2e.rs:13-14 | SHALLOW | F-001: not asserted |
| "a link oscillating inside the hysteresis band produces zero further switches" | hls_master_e2e.rs:19-22 | REAL | ran; switch count + wire slice pinned |
| matrix network-hls evidence "highest-BW-under-cap selection… ABR hysteresis band/dwell" | docs/feature-matrix.toml:1440 | REAL at library level | ran; but lane unreachable in product (F-002) |

## 7. Dead / duplicated / unreachable code
- F-002 (lane unreachable), F-003 (dead observations). No other dead code.

## 8. Dependencies & flags
streaming (fetch, hls::master, mockhttp), plugin_api (SourceStream), testkit.
No features.

## 9. Open questions for the next agent
- What are AbrConfig's default margins/dwell (verify the 0.16/1.0 Mbps band
  numbers against crates/streaming/src/hls/master.rs or abr code)?
- EXT-X-MEDIA audio rendition is never selected — is that the honest matrix
  gap ("alternate renditions parsed+recorded but not selected")?
