# AUDIT: crates/codecs/tests/alac_bench.rs
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-02-14 |
| Lines / functions | 119 / 1 test + 3 helpers |
| File verdict | REAL (measurement bench over committed fixtures) — but with no failure mode short of I/O damage (F-001) |
| Reachable from shipped entry point? | NO (test binary; `#[ignore]`d measurement by the flac_bench convention — baseline records 1 ignored) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read of full file; python3 stsz parse of alac_stereo24.m4a (packet-count claim); fixture presence verified in the alac_stereo24.m4a audit; baseline citation audit/runs/baseline/cargo-test-workspace.txt:947-954 |

## 1. One-paragraph verdict
A real throughput bench with the right shape (full open → MP4 demux → configure from the sample-entry cookie → per-packet decode → counting sink, 8 passes with the first discarded as warm-up, median of 7) over COMMITTED fixtures — unlike adpcm_bench there is no private-corpus dependency, so it can actually run from a fresh checkout with `--ignored`. Its doc claims check out: alac_stereo24.m4a carries exactly 9 packets (my stsz parse: sample_count = 9) and is the heaviest committed ALAC input. The defect relative to its sibling benches: it asserts nothing about the decode (no extent check, no per-packet `Emitted` check, no throughput floor) and discards the `DecodeOutcome`, so a decoder regression would silently produce a wrong number in the worklog rather than a failure — acceptable for a purely-ignored measurement, but weaker than the adpcm_bench/flac_bench convention it cites.

## 2. What actually works (verified)
— python3: `stsz ... sample_size 0 count 9` — the "9 packets" claim in the header (line 7) is exact.
— Median arithmetic: REPS=7, warm-up excluded via `if rep > 0`, `timings[7/2]` = index 3 ✓.
— Baseline: `test alac_decode_throughput_fixtures ... ignored, 5.J3a throughput measurement: cargo test -p codecs --release --ignored ...` (audit/runs/baseline/cargo-test-workspace.txt:952-954) — by-design ignore, invocation documented in the ignore reason.
— Fixtures present on disk (verified in the alac_stereo24.m4a audit: 112,791 bytes, valid M4A/alac).

## 3. Findings
F-001 [S4]
Where: alac_bench.rs:95-98 (+ absence of any extent/floor assertion)
Code: (verbatim)
```
                    DemuxEvent::Packet(packet) => {
                        let _ = decoder
                            .decode(packet, &mut sink)
                            .unwrap_or_else(|e| unreachable!("{e}"));
                    }
```
What is wrong: The `DecodeOutcome` is discarded and nothing asserts the decoded extent (total_samples/bytes are only eprintln'd) or a throughput floor. A silent decode regression (e.g., every packet `Recovered`, zero frames emitted) would still "pass" the `--ignored` run and report a meaningless (near-zero) MB/s into the 5.J3a worklog entry. Contrast adpcm_bench.rs, which asserts the exact inter-channel extent on the warm-up pass and every rep plus a 1 M/s release floor, and cites THIS convention. Since the test is ignored by design and the number is human-reviewed, this is hygiene, not a gate hole.
Confidence: HIGH — Verification: read (no assertion exists in the file).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixture_path | 24-28 | REAL | — | committed fixtures dir |
| open_via_input | 30-35 | REAL | — | product access path |
| Sink::accept | 43-56 | REAL | — | counts bytes + inter-channel frames; bytes_per_sample×channels divisor (no .max(1), but ALAC guarantees ≥1 — channels comes from the decoded format) |
| alac_decode_throughput_fixtures | 61-119 | REAL | S4 | warm-up + median; no assertions (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| alac_decode_throughput_fixtures | NO (except I/O damage → unreachable!) | no extent, outcome, or floor assertions; measurement-only (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the heaviest committed input: 9 packets, 2 channels × 24-bit" | alac_bench.rs:6-7 | REAL | stsz count 9 (python3); 2ch/24-bit verified in the fixture audit |
| "through the full open → MP4-demux → decode pipeline, so decode-only throughput is strictly higher than the reported number" | alac_bench.rs:8-10 | REAL | read: timing includes open+demux |
| "the 5.A1 flac_bench.rs convention — an ignored measurement test whose number goes into the card's worklog entry" | alac_bench.rs:1-3 | SHALLOW | convention followed for ignore+median, but flac_bench/adpcm_bench assert extents/floors — this one doesn't (F-001) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
codec_api (default ThreadBudget — consistent with config.rs F-001), demux::mp4, input, testkit. No features. No env knobs (unlike adpcm_bench's LUMEN_CORPUS — not needed, fixtures committed).

## 9. Open questions for the next agent
None.
