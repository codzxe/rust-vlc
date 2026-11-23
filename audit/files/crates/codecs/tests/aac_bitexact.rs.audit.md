# AUDIT: crates/codecs/tests/aac_bitexact.rs
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2025-06-27 |
| Lines / functions | 158 / 4 fns + 2 tests |
| File verdict | REAL (executed: 2/2 pass, direct binary run, exit 0, live ffmpeg tolerance arm ran; reference data independently re-verified against ffmpeg) |
| Reachable from shipped entry point? | N/A (integration test; baseline cargo-test-workspace.txt:889; I re-ran the built binary: 2/2 in 1.11s) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | ./target/debug/deps/aac_bitexact-1e99e0008b8875c2 --nocapture -> 2/2 ok EXIT=0; ffmpeg decode of both fixtures -> md5sum (== recorded ffmpeg_pcm_md5, both); cat tests/data/aac_bitexact_reference.txt; sed/rg quote checks |

## 1. One-paragraph verdict
A clean bit-exact/regression suite with an honest tolerance model, and I verified every layer externally: the two committed AAC-LC fixtures decode under my own ffmpeg to EXACTLY the recorded `ffmpeg_pcm_md5` values (`ebb4ae6e…` / `0b5b973f…`), the reference file's provenance section is candid (fixtures authored by the same ffmpeg's encoder; ISO 14496-4 conformance vectors not redistributed for license; AAC is lossy so the tolerance is ±1 LSB, recorded per row), and the executed test ran all four assertion layers — ffprobe-facts agreement, walked frame/sample counts, the s16le regression-MD5 pin, and the live-ffmpeg ±1-LSB delta — passing 2/2. The `lumen_pcm_md5` pin is a self-golden by design (drift guard), but it is never the only check: the live tolerance arm bounds correctness against an external decoder whenever ffmpeg exists, and steps 1-3 still assert when it does not. No findings.

## 2. What actually works (verified)
- Direct run `./target/debug/deps/aac_bitexact-1e99e0008b8875c2 --nocapture` (binary 18:07 newer than source 17:35) → `test result: ok. 2 passed; 0 failed ... finished in 1.11s`, EXIT=0. No SKIP line printed → ffmpeg was present → the live tolerance arm EXECUTED (per aac/mod.rs's ffmpeg_decode_s16le, the exact command the reference file records).
- Independent oracle: `ffmpeg -i fixtures/sine441_lc_stereo.aac -map 0:a -c:a pcm_s16le -f s16le - | md5sum` → `ebb4ae6edb452a22fe301f3f0efb58a4` == reference `ffmpeg_pcm_md5`; same for sine48_lc_mono → `0b5b973fe9f32823deee93130bab892b` == reference. The recorded expectations are reproducible outside the repo's own tooling.
- Arithmetic: 131 × 1024 = 134144 and 142 × 1024 = 145408 — the reference's `samples_per_channel` are exact multiples (also pinned by test 2, `lc_frames_are_exactly_1024_samples`, which passed).
- Baseline corroborates: cargo-test-workspace.txt:889 (suite ran; my direct run is the fresh evidence).

## 3. Findings
None. What was checked and how: reference-file parser (hand-rolled INI, typed errors, section/key shapes — read); MD5 pin vs live-oracle division of labor (documented at aac_bitexact.rs:13-18, verified by execution — both arms ran); ffmpeg-absent fallback still asserts steps 1-3 (read, :140-142 — unlike av1_conformance's full skip); frame-count/sample-count constants (executed); 1024-samples-per-frame invariant (executed, test 2); fixture existence (decode ran); no `.ok()` swallows on asserted paths (`unreachable!` on decode failure is correct for committed fixtures).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| reference / fixture_reference | 42-55 | REAL | — | OnceLock-cached parse of tests/data/aac_bitexact_reference.txt |
| parse_reference | 57-81 | REAL | — | INI-subset parser; unterminated section / key-outside-section are typed errors |
| fixtures_decode_the_recorded_window_exactly (test) | 83-144 | REAL | — | EXECUTED: facts, counts, MD5 pin, live ±1 tolerance (all four layers) |
| lc_frames_are_exactly_1024_samples (test) | 148-158 | REAL | — | EXECUTED: per_channel == frames × 1024 on both fixtures |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fixtures_decode_the_recorded_window_exactly | YES | MD5 pin catches any drift; live tolerance catches divergence from ffmpeg beyond ±1 LSB; counts/facts catch structural breakage — all executed here |
| lc_frames_are_exactly_1024_samples | YES | invariant check independent of the reference file |
The self-golden `lumen_pcm_md5` alone would be circular, but the suite's design (documented :13-18) never relies on it alone — verified by the executed live arm.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the two committed fixtures decode against the recorded ffmpeg 7.x reference" | aac_bitexact.rs:1-5 | REAL | my independent ffmpeg decode reproduces both ffmpeg_pcm_md5 values |
| "a lossy decoder's MD5 is not ffmpeg's, but it must never drift" (pin semantics) | :14-15 | REAL | lumen_pcm_md5 (85836a9b… / e8f06389…) differs from ffmpeg's, as documented; pin passed live |
| "±1 LSB" tolerance | :17-18 + reference file `max_sample_delta = 1` | REAL | live arm executed with tolerance 1, passed |
| "ISO 14496-4 conformance vectors: not redistributed (license + size)" | reference file header | REAL-UNPROVEN | license claim not independently checked; consistent with the av1 lane's equivalent claim |

## 7. Dead / duplicated / unreachable code
None. All helpers consumed; `ffmpeg_decode_s16le_adts_bytes` (aac/mod.rs:162) belongs to the corpus suite, not this file.

## 8. Dependencies & flags
Test-only; external: ffmpeg/ffprobe (present). Reference data at tests/data/aac_bitexact_reference.txt (committed, self-describing provenance). Fixtures: sine441_lc_stereo.aac, sine48_lc_mono.aac.

## 9. Open questions for the next agent
- aac_corpus_differential.rs auditor: the corpus rows' `ffmpeg_pcm_md5` "pin the corpus bytes — a re-imaged corpus FAILS the row loudly" (reference header) — confirm that mechanism exists there.
- Whether the ±1 LSB class holds for non-sine content (speech/music) is a corpus question, not this fixture set's — the reference honestly scopes itself to sine fixtures authored by ffmpeg.
