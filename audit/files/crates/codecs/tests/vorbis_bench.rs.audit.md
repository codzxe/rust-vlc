# AUDIT: crates/codecs/tests/vorbis_bench.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| Lines / functions | 89 / 1 test (#[ignore]d measurement) |
| File verdict | FALSE (the documented fallback lane does not work: I executed the prebuilt test binary and the bench PANICS on its own sanity assertion; the failure is invisible to every gate because the test is ignored) |
| Reachable from shipped entry point? | N/A (ignored measurement test) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | DIRECT EXECUTION of the prebuilt binary `./target/debug/deps/vorbis_bench-cb6cd7ebd64e5905 --ignored --nocapture --test-threads=1` → FAILED (assert at :67, 66 vs 67); binary-freshness stat check (binary 18:07 > source 17:35 — current); python Ogg packet census; baseline cite |

## 1. One-paragraph verdict
The ignored-bench convention hides a broken fallback path. The module doc promises "the committed 44.1 kHz stereo sine fixture as the fallback lane when the /tmp corpus is absent" — the corpus IS absent here, and when I ran the test directly (prebuilt debug binary, no cargo build needed; binary is newer than the source, so it reflects current code) it PANICKED at its own sanity assert: `assertion 'left == right' failed: one packet = one block, left: 66, right: 67`. My python census of the fixture shows 67 real data packets and zero zero-length packets, so the count mismatch is the decoder's documented head-hold behavior (tests/vorbis/mod.rs:236-241: "the per-packet frame count [is] stream-shaped: 0 while the head is withheld, `released + 1` on the release packet … any count is the contract's `Emitted`") — the bench author asserted an invariant the pipeline explicitly does not guarantee. Nobody ever ran the fallback lane: the test is `#[ignore]`d, the baseline counts it as "ignored, ok" (0 tests run), and the 5.B2 worklog number was presumably taken on the corpus lane. The measurement methodology itself (median of 7, deterministic reps, warm-up) is sound.

## 2. What actually works (verified)
- DIRECT RUN (executed by me): corpus-absent fallback selected with the honest eprintln; decode ran (0.12s); the failure is the sanity assert, not the decode.
- Binary freshness: `stat` — target binary 2026-11-23 18:07:52 vs source 2026-11-23 17:35:58 → the executed binary is current.
- Python Ogg census of vorbis_sine_44100_stereo.ogg: 4 pages, 70 packets (3 headers + 67 data), 0 zero-length — the 66/67 mismatch is not a degenerate-packet artifact.
- Baseline :1366-1369 area: `vorbis_decode_throughput ... ignored` (0 run) — confirming the gate never executes this path.
- Methodology read: REPS=7 median (`samples_per_sec[REPS/2]`), deterministic-rep packet-count assert, warm-up outside timing.

## 3. Findings
F-001 [S3]
Where: crates/codecs/tests/vorbis_bench.rs:5-11 vs 67-71
Code:
```
//! packets, 32 000 granule samples) — with the committed 44.1 kHz
//! stereo sine fixture as the fallback lane when the /tmp corpus is
//! absent. Median of 7 repetitions, reported as inter-channel
```
vs
```
    assert_eq!(
        warm.sink.buffers.len(),
        warm.packets,
        "one packet = one block"
    );
```
What is wrong: the fallback lane cannot complete — executed proof: the assert fails 66 vs 67 on the committed fixture (67 data packets, no zero-length ones). The invariant is false for the head-trimmed Vorbis pipeline (the driver's own comment at tests/vorbis/mod.rs:236-241 documents stream-shaped per-packet counts), so the bench's sanity gate contradicts the composition's documented contract. Anyone following the ignore message's own instructions (`cargo test -p codecs --release --ignored`) in a corpus-less environment gets a panic instead of a number. Why it matters: the recorded 5.B2 worklog throughput is unreproducible in this environment, and the fallback-lane claim in the module doc is false. What correct looks like: assert `buffers.len() <= packets` plus a nonzero-sample check (already present at :63-66), or compare against the fixture's recorded frame count. Confidence: HIGH — Verification: EXECUTED (direct binary run, FAILED) + python packet census.

F-002 [S3]
Where: crates/codecs/tests/vorbis_bench.rs:25 (ignore attribute) + the failure above
Code:
```
#[ignore = "5.B2 throughput measurement: cargo test -p codecs --release --ignored (number in the 5.B2 worklog entry)"]
```
What is wrong: the ignore-by-design convention let a broken code path ship invisible — the baseline's "4744 passed / 51 ignored" counts this test as healthy while it cannot run to completion on any committed input. The same convention question applies to every ignored bench in this crate (flac_bench corpus_0046, mp3/mp2/av1 benches): none of their lanes are exercised by any gate. This instance is proven broken; the siblings are UNVERIFIED (release build outside my budget). What correct looks like: a cheap non-ignored smoke variant that runs the fallback lane's sanity asserts (without timing) in the normal gate. Confidence: HIGH for this file — Verification: executed.

F-003 [S4]
Where: crates/codecs/tests/vorbis_bench.rs:57-62
Code:
```
    let total_samples: u64 = warm
        .sink
        .buffers
        .iter()
        .map(|b| (b.len() / (channels * 4)) as u64)
        .sum();
```
What is wrong: `total_samples` counts emitted-frame samples — with the head-hold withholding the first block and the count mismatch above, the "inter-channel samples" denominator is the EMITTED window, not the stream's valid sample count, so the reported samples/sec is measured over a slightly different basis than the module doc's "32 000 granule samples" corpus-lane framing (the two lanes' numbers are not comparable even once F-001 is fixed). Confidence: MEDIUM — Verification: read + the 66/67 evidence.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| vorbis_decode_throughput | 26-89 | FALSE (fallback lane) / REAL (corpus lane, UNVERIFIED) | S3 | F-001/F-003; methodology (median-of-7, warm-up, deterministic reps) sound |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| vorbis_decode_throughput | It cannot even pass itself today (F-001); as a measurement it is ignored by every gate — zero evidence value in the baseline |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "with the committed 44.1 kHz stereo sine fixture as the fallback lane when the /tmp corpus is absent" | :8-10 | FALSE | executed: fallback lane panics at :67 |
| "an ignored measurement test whose number goes into the card's worklog entry, never the normal gate run" | :1-3 | TRUE (convention) | baseline: ignored |
| corpus lane "(8 kHz stereo, 126 data packets, 32 000 granule samples)" | :7-8 | UNVERIFIED | corpus absent; worklog number not re-executed (cargo-build-budget for release) |

## 7. Dead / duplicated / unreachable code
The fallback-lane half of the lane-selection match (:33-50) is effectively dead — it selects a lane that cannot pass its own sanity gate.

## 8. Dependencies & flags
Test-target, `#[ignore]` by convention; shares tests/vorbis/mod.rs (decode_through_ogg — another agent's file, whose flush handler requires exactly `Drained`, corroborating that no final frame is emitted at flush on this fixture).

## 9. Open questions for the next agent
- tests/vorbis/mod.rs auditor: is the head block's withheld frame correctly discarded pre-skip, and is the final block's valid tail emitted on the LAST packet (not flush)? vorbis_bitexact passing suggests the window is right — confirm its tail coverage.
- Run the sibling ignored benches' fallback lanes the same way I did (prebuilt debug binaries, direct exec) — if flac/mp3/mp2/av1 benches have fallback lanes, they may hide the same class of breakage.
