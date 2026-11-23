# AUDIT: crates/codecs/tests/mp3_corpus_differential.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| Lines / functions | 210 / 2 helpers + 2 tests |
| File verdict | SHALLOW (test 1's fixture half is structurally skip-gated behind the absent corpus — its live-oracle assertions never run here; test 2 is real and runs) |
| Reachable from shipped entry point? | N/A (test binary; decode path `codecs::audio::mp3::decode_stream` is library/product code) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | sed reads; baseline cite; corpus-absence proven earlier; ffmpeg presence proven earlier |

## 1. One-paragraph verdict
Two tests, opposite honesty. `vbr_toc_seek_accuracy_is_within_one_frame_and_refines_exactly` is real and runs on the committed vbr_q4.mp3 fixture (baseline ok; TOC length/monotonicity, midpoint-±600-byte landing, refined header validation — genuine assertions). `corpus_and_fixture_differential_vs_ffmpeg` is structurally broken in this environment: the corpus-absent early return at lines 83-86 aborts the WHOLE test — including the six committed-fixture live-oracle rows (lines 114-124) that have no corpus dependency whatsoever — so despite ffmpeg being installed and the fixtures being committed, the ±1-LSB live differential never executed (baseline: the binary finished in 0.06s; a real six-file ffmpeg differential costs ~1s+ by comparison with mp3_bitexact's 1.01s). The test's own name promises "corpus AND fixture"; the fixture half never runs anywhere the /tmp corpus is absent. Recorded-digest coverage of the fixtures exists separately (mp3_bitexact, baseline ok) — against authoring-time constants, not the live oracle.

## 2. What actually works (verified)
- Baseline :1271: `vbr_toc_seek_accuracy_is_within_one_frame_and_refines_exactly ... ok`; :1270 `corpus_and_fixture_differential_vs_ffmpeg ... ok` (binary total 0.06s — the skip path).
- Test 2's chain verified by read: `codecs::audio::mp3::decode_stream` → Xing/TOC extraction → `vbr_toc_seek` → `refine_to_frame` → `parse_frame_header` — all product functions.
- Support helpers (tests/mp3/mod.rs): `ffmpeg_decode_s16le` runs the real oracle command; `decode_s16le` is the real product decode.

## 3. Findings
F-001 [S3]
Where: crates/codecs/tests/mp3_corpus_differential.rs:83-86 vs 114-124
Code:
```
    let Some(corpus) = corpus_dir() else {
        eprintln!("SKIP mp3 corpus differential: corpus root absent");
        return;
    };
```
vs
```
    // The committed fixture set through the same live oracle.
    for name in [
        "cbr128.mp3",
```
What is wrong: the early return fires before the committed-fixture loop, so the fixture half of the differential (six files, live ffmpeg, ±1 LSB bar at lines 145-152) is unreachable without the /tmp corpus — even though neither ffmpeg (present: /usr/bin/ffmpeg) nor the fixtures (committed under tests/fixtures/) depend on it. In the baseline run this test asserted nothing (0.06s binary total). The header's "SKIP-with-message when the corpus root or ffmpeg is absent" (:21) is accurate for the corpus ROWS but the gate over-skips the fixture rows. Why it matters: the only live-oracle MP3 window-locked comparison in the repo never runs in this environment; fixture coverage exists only as recorded digests (mp3_bitexact). What correct looks like: move the `corpus_dir()` check inside the corpus loop (`if let Some(corpus) = ...` per row, like the file-absent per-row skip at :94-97 already does). Confidence: HIGH — Verification: read + baseline timing + corpus absence proven.

F-002 [S4]
Where: crates/codecs/tests/mp3_corpus_differential.rs:100-110
Code:
```
        if let Some(expect) = corpus_reference(name.trim_end_matches(".mp3")) {
            assert_eq!(
                row.ffmpeg_md5, expect["ffmpeg_pcm_md5"],
```
What is wrong: the drift check is conditional on the reference section existing. A corpus file present with NO matching `[corpus.…]` section (reference data lag) silently skips both the MD5 pin and the sample-count pin — contradicting the header's "if the corpus was re-imaged, the row FAILS loudly instead of silently testing different bytes" (:17-19). A missing section should be an explicit choice (assert or eprintln), not a quiet `if let`. Confidence: HIGH — Verification: read + corpus_reference's Option return (tests/mp3/mod.rs:65-67).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Row struct + run_row | 40-75 | REAL | — | length-locked window assert before stats; unreachable! on decode failure |
| corpus_and_fixture_differential_vs_ffmpeg | 78-153 | SHALLOW (in this env) | S3 | F-001 over-gating; per-row file-absent skip exists but the corpus root gate aborts all |
| vbr_toc_seek_accuracy_is_within_one_frame_and_refines_exactly | 163-210 | REAL | — | committed fixture; real assertions incl. refine-to-header |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| corpus_and_fixture_differential_vs_ffmpeg | NO in this environment (early return); YES with corpus present | fixture half gated off (F-001) |
| vbr_toc_seek_accuracy... | YES | runs on committed fixture; TOC shape/monotone/landing/refine all asserted (baseline ok) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "decoded BOTH by Lumen and by the external ffmpeg ... compared on the SAME gapless window" | :1-4 | REAL-UNPROVEN | mechanism verified by read; execution requires the corpus (never ran here) — worklog numbers not re-executed |
| "if the corpus was re-imaged, the row FAILS loudly" | :17-19 | PARTLY FALSE | only when the reference section exists (F-002) |
| "SKIP-with-message when the corpus root or ffmpeg is absent" | :21 | TRUE but over-broad | the skip also kills the fixture half (F-001) |

## 7. Dead / duplicated / unreachable code
The six-fixture live-oracle loop (:114-124) is unreachable in every corpus-less environment — dead code in the current lane.

## 8. Dependencies & flags
Test-target; shares tests/mp3/mod.rs helpers (another file). No features. Depends on external ffmpeg + /tmp corpus-v4 for the corpus rows.

## 9. Open questions for the next agent
- tests/mp3/mod.rs and mp3_bitexact.rs auditors: confirm the recorded-digest fixture coverage (bitexact) is generated from ffmpeg at authoring (the reference file's header should say so).
- Same over-gating pattern check for mp2_corpus_differential.rs (my next file) and the vorbis corpus tests.
