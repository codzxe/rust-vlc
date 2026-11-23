# AUDIT: crates/codecs/tests/mp3_bench.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-23 |
| Lines / functions | 67 / 2 (bench, mp3_decode_throughput) |
| File verdict | REAL (measurement tool, executed) |
| Reachable from shipped entry point? | N/A (#[ignore] bench by documented convention; number goes to the worklog) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | executed prebuilt `mp3_bench-f3a5109ec9b5a742 --ignored --nocapture` -> 1 passed, 0.53s, real output captured (below) |

## 1. One-paragraph verdict
A real, small throughput measurement in the repo's bench convention (#[ignore], median of 7, realtime factor): I executed it — `fixture cbr128.mp3: 176400 inter-channel samples, median 2448111 samples/s = 28x realtime (2.000s of audio)` in debug profile, corpus row SKIP (absent). The math is self-consistent (seconds = len/ch/rate; realtime = median/(ch·rate) both correct for a total-sample count), but the "inter-channel samples" LABEL is wrong: `decoded.samples` is interleaved f32 (one per individual sample, mp3/decoder.rs:89), so 176400 is the TOTAL sample count (88200 frames × 2ch); the module docstring repeats the mislabel while docs/formats/audio-mp3-mp2.md correctly lists 88200 inter-channel for the same file.

## 2. What actually works (verified)
— verified by execution: the bench runs, decodes the committed fixture (`.must()` — a decode regression fails the bench), prints the median and realtime factor; corpus row self-skips loudly.
— verified `DecodedMp3.samples: Vec<f32>` (crates/codecs/src/audio/mp3/decoder.rs:88-89) — basis of the label finding.
— verified baseline: mp3_bench listed as ignored in the gate run (convention documented at L1-4).

## 3. Findings
F-001 [S4]
Where: crates/codecs/tests/mp3_bench.rs:9-10, 29
Code: (verbatim, L29-30)
```
    let (decoded, _) = decode_s16le(data).must();
    let inter_channel: u64 = decoded.samples.len() as u64;
```
What is wrong: `samples` is interleaved f32 — its len() counts INDIVIDUAL samples (frames × channels), not inter-channel samples. The printed line and the docstring ("176 400 inter-channel samples", L10) mislabel the quantity; the format doc's row for the same file says 88200 (the true inter-channel count). Seconds and realtime x are computed consistently so no reported NUMBER is wrong — only the unit name. Confidence: HIGH — Verification: ran + type check.
F-002 [S4]
Where: crates/codecs/tests/mp3_bench.rs:54-66
Code: (verbatim, L63-65)
```
        None => eprintln!(
            "corpus: LUMEN_CORPUS unset and /tmp/my-project/corpus-v4 absent — SKIP corpus row"
        ),
```
What is wrong: the corpus row (0139-audio-only.mp3, MPEG-2.5 lane) self-skips in this environment — the MPEG-2.5 throughput row is unmeasured here. Confidence: HIGH — Verification: ran (SKIP printed).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| bench | 26-48 | REAL | S4 | warm-up decode + 7 reps + median + realtime; label bug F-001 |
| mp3_decode_throughput | 50-67 | REAL | S4 | #[ignore] convention; corpus skip F-002 |

## 5. Test honesty
Not an assertion test — a measurement. Its `.must()` calls make a decode regression fail it (executed green).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "176 400 inter-channel samples" (cbr128) | L10 | FALSE label (176400 = total samples; inter-channel = 88200) | decoder.rs:89 Vec<f32> + format doc row 3 |
| "never the normal gate run" | L3-4 | REAL | #[ignore]; baseline lists it ignored |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Test-only; helpers from tests/mp3/mod.rs (decode_s16le). REPS=7 median.

## 9. Open questions for the next agent
- The worklog's recorded 5.A3 throughput number: verify it was taken in release profile and with which label (if it says "samples/s inter-channel", it inherits F-001's mislabel).
