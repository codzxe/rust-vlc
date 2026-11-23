# AUDIT: crates/codecs/tests/opus_corpus_differential.rs
| Field | Value |
|---|---|
| Auditor | W06-R3 (credit W06-A08) |
| Date | 2026-11-23 |
| Lines / functions | 158 / 1 assertion fn + 2 tests + 1 const |
| File verdict | REAL-UNPROVEN (assertions genuine and bite — proven by execution — but the corpus gate keeps them unexecuted in every available environment) |
| Reachable from shipped entry point? | N/A (test). The composition it drives — demux::ogg → `OpusHead ‖ OpusTags` extra_data → `codecs::audio::opus::OpusDecoder` — IS the CLI's opus lane (pipeline_host "opus" arm; engine ruopus 0.1.2 per docs/dependencies/records/ruopus-0.1.2.md) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `./target/debug/deps/opus_corpus_differential-06f3a6620b4b27bb --nocapture` -> 2 passed in 0.01s, both lanes print SKIP (corpus absent); `LUMEN_CORPUS=/tmp/w04c_clean ... corpus_0047 --nocapture` -> FAILED `left: 26 / right: 201`, exit 101 (asserts bite on a truncated corpus copy — real composition executed); sed/rg reconciliation vs the admission record |

## 1. One-paragraph verdict
A genuine corpus differential with a well-designed assertion stack: hard geometry (201 packets, pre-skip 312, raw 192,960 = 201×960, last granule 192,312, valid output 192,000 = granule − pre_skip, first pts 0), a live ffmpeg f32 differential with a lossy-honest tolerance (zero samples beyond one s16 LSB + 1e-3 sanity bound), and a channel-order check that rides the tolerance (no reference-side permutation — a wrong Vorbis→house order would blow the 5.1 differential catastrophically). All pinned constants reconcile exactly with the admission record (ruopus-0.1.2.md:122-124: 1.378e-06 / 4.381e-06 rows). I proved both paths by execution: the in-tree run is the SKIP path (2 "passed" in 0.01s asserting nothing — corpus-v4 absent), and pointing LUMEN_CORPUS at a truncated W04 copy made the lane run the real composition and fail LOUDLY at the packet-count assert (26 ≠ 201). So the suite is real and would hold the decoder to account — but in this tree, in the baseline, and in every environment W04 saw, it executes zero assertions, and the module docs never disclose the skip.

## 2. What actually works (verified)
- Skip path executed: `--nocapture` -> `SKIP corpus.0047: no corpus directory (LUMEN_CORPUS unset, /tmp default absent)` + same for 0077; `2 passed; 0 failed ... finished in 0.01s`, exit 0. Baseline :1290-1296 is this same path (2 passed, 0.01s).
- Asserts bite: `LUMEN_CORPUS=/tmp/w04c_clean ./target/debug/deps/opus_corpus_differential-06f3a6620b4b27bb --test-threads=1 corpus_0047 --nocapture` -> real ogg→OpusDecoder decode of the truncated copy, then `assert_eq!(run.packets, packets)` FAILED (`left: 26, right: 201`), exit 101. The composition driver, packet counting, and geometry asserts are live code, not decoration.
- Constant reconciliation vs the admission record: record "ruopus samples/ch (post-pre-skip) 192,648" = test raw 192,960 − pre-skip 312; record "ffmpeg samples/ch 192,000" = test valid; end-trim 648 = raw − granule (the eprintln's own arithmetic :111). The doc-cited max |Δ| values (1.378e-06, 4.381e-06) match ruopus-0.1.2.md:123-124 verbatim.
- Helper honesty (tests/opus/mod.rs, audited REAL by W06-A09): f32_max_delta/samples_beyond compare equal-length streams only after the count asserts; ffmpeg_decode_f32le pins `-map 0:a -c:a pcm_f32le` at native channel count.

## 3. Findings
F-001 [S3]
Where: crates/codecs/tests/opus_corpus_differential.rs:49-52 (and the module docs :1-25 that never mention it)
Code:
```
    let Some(root) = corpus_dir() else {
        eprintln!("SKIP {lane}: no corpus directory (LUMEN_CORPUS unset, /tmp default absent)");
        return;
    };
```
What is wrong: the corpus gate makes both tests assert NOTHING in-tree: corpus-v4 is absent from this sandbox and from every environment W04 agents saw (audit/runs/W04-A01.log.md:107, W04-A02.log.md:68, W04-A04.log.md:107 — "the entire real-media conformance layer asserts nothing in-tree"). The baseline's green "2 passed (0.01s)" is the skip path — I verified by running it: both lanes print SKIP and no assertion executes. The module header presents three assertion classes as the suite's content without disclosing that none of them run without an external corpus. The admission-record numbers (1.378e-06 etc.) are past-session records, unverifiable here. Why it matters: a reader of the baseline counts this suite as live differential evidence for the opus lane; it is currently evidence of nothing but the skip. What correct looks like: a module-doc "Corpus-gated: skips without corpus-v4" line (the mp3/aac corpora committed fixtures in-tree as the always-on lane), or one committed fixture lane per family (W04-A04's recommendation). Confidence: HIGH — Verification: ran (both paths).
F-002 [S4]
Where: crates/codecs/tests/opus_corpus_differential.rs:10-12
Code:
```
//!    OpusHead pre-skip (312 on both lanes), the raw per-channel
//!    decode count (192,960 = 201 × 960 — pre-skip included), and the OUTPUT
//!    count = the last granule (192,000) — i.e. **pre-skip 312 +
//!    end-trim 648 exactness**, binding condition 2 measured.
```
What is wrong: "the OUTPUT count = the last granule (192,000)" — the last granule is 192,312; the output count is granule − pre-skip (192,312 − 312 = 192,000), as the code itself correctly documents at :69-70 ("valid = granule − pre-skip") and asserts at :71-80. Wording slip only; the arithmetic everywhere else is exact. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `const ONE_S16_LSB` | 36-37 | REAL | — | 1/32768; the tolerance class matches the admission record's "s16-audible equivalence" |
| `fn assert_lane` | 40-124 | REAL (executed on a truncated corpus: asserts bite) | S3-adjacent | full stack: 7 geometry asserts + live differential + order-rides-tolerance; corpus-gated (F-001); `unreachable!` on missing lane file (dir is the gate, files are contract — documented) |
| `tests::corpus_0047_mono` | 129-140 | REAL-UNPROVEN | — | constants reconcile with record :123; skip-gated in-tree |
| `tests::corpus_0077_surround_51` | 147-158 | REAL-UNPROVEN | — | constants reconcile with record :124; the direct-native-layout comparison is a REAL regression check on the decoder's Vorbis→house order map |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| corpus_0047_mono | YES in a corpus environment (geometry asserts + per-sample differential); NO in-tree (skip returns before any assert) | Proven both ways by execution: skip path green in 0.01s; truncated-corpus run failed loudly at packets assert (26 ≠ 201, exit 101) |
| corpus_0077_surround_51 | Same shape; additionally the 5.1 lane would catch a wrong channel permutation via the tolerance asserts (channels carry different signals) | Same skip caveat; permutation-catch reasoning is sound (read) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "decoded through the real composition (ogg demux → OpusHead ‖ OpusTags as extra_data → OpusDecoder)" | :1-4 | REAL | decode_through_ogg (tests/opus/mod.rs, W06-A09 audit); composition executed in my truncated-corpus run |
| "checked against the ffmpeg 7.1.5 float oracle" | :4 | REAL-UNPROVEN | oracle path exists (ffmpeg_decode_f32le); never ran here — corpus absent (F-001); ffmpeg 7.1.5 present in sandbox, only the media is missing |
| Geometry numbers (201 / 312 / 192,960 / 192,312 / 192,000 / 648) | :8-12, :126-158 | REAL (constants) / UNVERIFIED (against real corpus) | internally consistent (201×960; granule−pre_skip; raw−granule) and reconcile with ruopus-0.1.2.md:122-124 |
| "0/192,000 beyond the LSB" (0047) and "4.381e-06 ... 0/1,152,000" (0077) | :126-146 | REAL-UNPROVEN | recorded values in the admission record; not re-derivable without the corpus |
| "OUTPUT count = the last granule (192,000)" | :10-12 | SHALLOW (wording) | last granule is 192,312; output = granule − pre_skip (F-002) |

## 7. Dead / duplicated / unreachable code
No dead code. The suite's assertions are corpus-unreachable in-tree (F-001) — the same posture W04-A04 rolled up for every corpus differential.

## 8. Dependencies & flags
Test-only: `mod opus` helpers, std. External: corpus-v4 media files (LUMEN_CORPUS or /tmp/my-project/corpus-v4), ffmpeg binary for the differential leg (present here; skipped only for corpus absence). Engine under test: ruopus 0.1.2 (admission record).

## 9. Open questions for the next agent
- Same as W04-A04's rollup: commit one small opus fixture lane in-tree so the differential asserts something on every machine. The 26-packet truncated copy in /tmp/w04c_clean is NOT a substitute (wrong geometry, mutant provenance).
- The admission record's 5.1 row used reference-side permutation; this test asserts the decoder's own order map — when the corpus next runs, confirm the direct comparison actually yields the recorded 4.381e-06 class (it should, but nobody has run it since the order map landed).
