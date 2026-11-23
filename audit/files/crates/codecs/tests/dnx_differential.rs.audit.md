# AUDIT: crates/codecs/tests/dnx_differential.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-23 |
| Lines / functions | 379 / 8 (6 tests + assert_8bit_lane, balance_gate, raw_es_packet, dnx_decoder, NullSink) |
| File verdict | REAL (live-oracle differential, executed) |
| Reachable from shipped entry point? | N/A (test); the decoder under test IS shipped: pipeline_host.rs:326-330 `"dnxhd" => DnxDecoder::new()` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | executed prebuilt `dnx_differential-96c1fdafa06b7fc6 --nocapture --test-threads=1` -> 6 passed / 0 failed, 9.09s, full per-plane stats printed; `ffmpeg -version` -> 7.1.5-0+deb13u1 (the doc's exact oracle version); baseline cited |

## 1. One-paragraph verdict
A real differential suite with a live oracle: it regenerates the admission record's five DNxHD lanes with the sandbox's ffmpeg 7.1.5 (the exact version the doc names), MD5-pins each lane to the committed record hash (so lane drift fails hard), decodes through the wired `DnxDecoder`, and diffs against ffmpeg's own rawvideo decode under the ratified gates (max|Δ|≤2 LSB, ±1 sign balance within 3:1, |Δ|≥2 outliers <1%). I executed it live: 6/6 pass in 9.09s, and my per-plane histograms reproduce the record's committed §D numbers exactly (L1 frame 0: 54258 differing = −1:24848 +1:29408 +2:2 — matches the doc's cited histogram digit-for-digit). The 10-bit lane pins the trait surface's typed refusal AND runs the full-depth seam differential; the truncation gate pins the exact `Corrupt { Truncated { expected: 917504 } }` refusal plus next-frame usability. Weaknesses are environmental brittleness only.

## 2. What actually works (verified)
— verified by executing the prebuilt binary: all six lanes with real stats, e.g. `L1_1080p_175M frame 0: Y: maxΔ=2 differing=26038/2073600 ... [−1:12058 +1:13978 |Δ|≥2:2]`, and the whole-lane totals reproduce the record's {−1: 24848, +1: 29408, +2: 2} exactly (sum across planes).
— verified `which ffmpeg` -> /usr/bin/ffmpeg, version 7.1.5-0+deb13u1 — the doc's named oracle version, so the MD5 lane pins are meaningful.
— verified baseline audit/runs/baseline/cargo-test-workspace.txt (dnx_differential section): 6 passed in 6.08s.

## 3. Findings
F-001 [S4]
Where: crates/codecs/tests/dnx_differential.rs:59-62
Code: (verbatim)
```
    if !ffmpeg_available() {
        eprintln!("SKIP {lane}: no ffmpeg binary (the live oracle is the lane generator)");
        return;
    }
```
What is wrong: without an ffmpeg binary every test in this file self-skips (returns Ok) — the harness reports 6/6 "ok" while proving nothing. The skip is loud (eprintln) but not distinguishable in CI summaries. In THIS sandbox and in the baseline, ffmpeg 7.1.5 is present, so the recorded evidence is real. Confidence: HIGH — Verification: read + ffmpeg present here.
F-002 [S4]
Where: crates/codecs/tests/dnx_differential.rs:325-327
Code: (verbatim)
```
                expected: 917_504,
                found: (frame.len() / 2) as u64
```
What is wrong: the hard-coded CBR frame size is pinned to the L1 175 Mbps lane's geometry; regenerate-with-different-bitrate and the assert breaks — acceptable because the lane itself is MD5-pinned (any drift fails earlier at assert_lane_md5), but the coupling is implicit. Confidence: HIGH — Verification: read + executed (assert passed live).
F-003 [S4]
Where: crates/codecs/tests/dnx_differential.rs:173,189,205,219,241 (frames=3 per lane)
Code: (excerpt, L173)
```
        3,
```
What is wrong: 3 frames per lane — the record's lanes are this small by design, but the differential sample is tiny; content-variety risk (interlaced DNxHD lanes not covered here — `interlaced=false` in every printed facts line). Confidence: HIGH — Verification: ran (all lanes interlaced=false).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| assert_8bit_lane | 49-134 | REAL | - | MD5 pin + geometry + per-plane tolerance + byte parity; executed |
| balance_gate | 140-161 | REAL | - | sign-balance 3:1 + outlier 1% gates; bit-exact lane trivially passes (documented) |
| lane_l1..l4 | 164-223 | REAL | - | executed; histograms match the record |
| lane_l5_1080p10_185m | 230-292 | REAL | - | typed refusal `codecs.dnx.output.bit_depth` per frame + full-depth seam differential; executed |
| truncated_real_frame_refuses_typed | 297-337 | REAL | - | exact Corrupt::Truncated pin + next-frame usability; executed |
| raw_es_packet / dnx_decoder / NullSink | 341-379 | REAL | - | plumbing |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 5 lanes | YES (when ffmpeg present) | MD5 lane pin + maxΔ/assert chain; a decoder regression breaks tolerance or byte parity; executed live |
| truncated gate | YES | exact kind+code+detail assert, unreachable! on silent Ok |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "regenerated with ffmpeg 7.1.5's own dnxhd encoder" | L3 | REAL | ffmpeg 7.1.5-0+deb13u1 in sandbox; lanes regenerated + MD5-matched live |
| record histogram "L1 frame 0 = {−1: 24848, +1: 29408, +2: 2}" | L23, L138 | REAL | my live run sums to exactly those counts (see §2) |
| "the trait surface's typed refusal (the 8-bit pixel vocabulary gap)" | L28-29 | REAL | executed: refusal code `codecs.dnx.output.bit_depth` asserted per frame |
| "Every number the suite prints is measured in this run" | L33-34 | REAL | stats printed match record cross-check |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Test-only; external dependency: ffmpeg binary at test time (skip when absent, F-001). Decoder under test is shipped (pipeline_host.rs dnxhd lane).

## 9. Open questions for the next agent
- Interlaced DNxHD (1080i variants) not covered by any lane — the decoder's interlaced path (if any) is unpinned here.
- tests/dnx/mod.rs (the helper: ffmpeg_available/generate_lane/compare_plane) is not in my assignment — its correctness underpins the oracle; audit if assigned.
