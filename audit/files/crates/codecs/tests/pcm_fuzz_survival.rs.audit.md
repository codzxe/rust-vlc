# AUDIT: crates/codecs/tests/pcm_fuzz_survival.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-23 |
| Lines / functions | 1403 / 36 (9 #[test], ~27 helpers/mutators) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (integration test). Tests crates/codecs PCM decode (pcm_s16le is a shipped first-vertical decoder, per baseline engine-info) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | sed-verified quotes; rg for codes/unwrap; executed prebuilt binary: `./target/debug/deps/pcm_fuzz_survival-a020d5883ef11742 --include-ignored --nocapture --test-threads=1` -> 9 passed; 0 failed, 4.30s, exit 0 |

## 1. One-paragraph verdict
This is a real, executed, exact-accounting fuzz-survival suite for PCM decode, not a tautology. Three surfaces per mutant (registered `fuzz_codec_pcm` harness, free `decode_pcm`, contract `PcmDecoder` path) with assertions that recompute the damage model independently and demand byte-exact agreement (`dropped_tail_bytes`, whole-frame prefix length, `DecodeOutcome` shape, frame duration from decoded instants). I ran the entire suite INCLUDING the `#[ignore]` deep run: 9/9 pass in 4.30s, and the deep-run ledger (17163 mutants -> 9732 emitted / 6712 concealed / 192 skipped / 64 empty / 5828 typed errors / 1242 accepted lies) reproduces the committed worklog numbers exactly (docs/worklog/archive-2026-10.md). Verdict REAL.

## 2. What actually works (verified)
— verified by `./target/debug/deps/pcm_fuzz_survival-a020d5883ef11742 --include-ignored --nocapture --test-threads=1` -> `test result: ok. 9 passed; 0 failed; ... finished in 4.30s`, deep ledger lines printed match docs/worklog/archive-2026-10.md verbatim.
— verified by `rg -n "pcm\.(channels\.zero|rate\.zero|waveformat|stream_format_conflict|codec_descriptor)" crates/codecs/src/audio/pcm/decoder.rs` -> every code asserted by the test exists in the decoder's taxonomy (L114-685).
— verified by baseline audit/runs/baseline/cargo-test-workspace.txt:1330-1341 -> 8 passed / 1 ignored in 0.56s (gate lane), consistent with my run.

## 3. Findings
F-001 [S3]
Where: crates/codecs/tests/pcm_fuzz_survival.rs:63-68
Code: (verbatim)
```
//! The valid payloads are the committed class-A parents' real audio
//! chunks (60 per container, CC0, `tests/fixtures/CORPUS.md`) —
//! generator-authored, extraction-pinned by the 2.B5 AC suite
//! (`pcm_fixtures.rs`). The adversarial value here is the mutation
//! engine, not external media; the external-oracle corpus is the
```
What is wrong: all "valid" parents are two generator-authored fixtures whose audio streams are byte-identical; payload-space coverage is bounded by mutations of one 2-second s16le stereo signal. Why it matters: the suite proves survival under structural damage of known-good PCM, not under real-world PCM variety (other widths/layouts come only via the header-lie class). The doc admits this, so it is a scope limitation, not a lie. What correct looks like: an external-media corpus lane (exists separately per doc: `corpus.rs`, harness-gated — not audited here). Confidence: HIGH — Verification: read + executed.

F-002 [S4]
Where: crates/codecs/tests/pcm_fuzz_survival.rs:101-103, 1368-1370
Code: (verbatim)
```
/// Deep-run volume per class (≥ 2000): `#[ignore]`-marked, executed on
/// demand; the gate never pays for it.
const DEEP_SEEDS: usize = 2048;
```
and
```
#[test]
#[ignore = "deep run: N=2048 per class, executed on demand for the audit report"]
fn deep_seeded_mutation_run() {
```
What is wrong: the deep volume never runs in the gate; its numbers are only as good as the last on-demand run. Why it matters: evidence risk if nobody re-runs it after decoder changes. Mitigated: I executed it today and it matches the committed record exactly. Confidence: HIGH — Verification: ran.

F-003 [S4]
Where: crates/codecs/tests/pcm_fuzz_survival.rs (whole file; 28 occurrences, e.g. 250, 276, 303)
Code: (verbatim, L250)
```
            Err(e) => unreachable!("{fixture}: {e}"),
```
What is wrong: `unreachable!` as the Err arm of every fallible call (repo-wide precedent, 4126 in tree per baseline loc-patterns.txt). In a test this is loud failure, which is correct behavior, but the pattern is debt flagged repo-wide. Confidence: HIGH — Verification: `rg -c "unreachable!"` -> 28.

F-004 [S4]
Where: crates/codecs/tests/pcm_fuzz_survival.rs:1195-1360
Code: (excerpt, L1196)
```
fn seeded_format_header_lies_are_typed_or_honestly_decoded() {
```
What is wrong: one 165-line test covering five lie strata, 8 edge pins, 3 GUID pins and conflict pins — a god-test; a failure in the middle strata hides which pin broke (assert messages mitigate). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| corpus_chunks | 236-262 | REAL | - | asserts 60 chunks/parent, whole-frame hygiene; parents verified present (repo-root tests/fixtures/generated/valid/) |
| SplitMix64::{new,next_u64,below,byte} | 160-186 | REAL | - | correct SplitMix64 constants; modulo bias immaterial for bounds used |
| Ledger::{absorb,report} | 192-227 | REAL | - | plain counters |
| CaptureSink::accept | 273-281 | REAL | - | audio-only sink, unreachable on video |
| packet | 284-295 | REAL | - | pts/dts Missing, keyframe true |
| true_config_decoder | 300-323 | REAL | - | with_format seam + configure |
| assert_typed_pcm_error | 329-344 | REAL | - | stage/code/kind triple-checked; codes verified to exist in decoder |
| duration_nanos | 350-355 | REAL | - | saturating mul + checked div, mirrors decoder formula |
| free_surface | 363-397 | REAL | - | exact tail/prefix accounting; would fail if concealment lied |
| contract_surface | 403-472 | REAL | - | recomputes expected DecodeOutcome from damage; asserts frame honesty + duration |
| mutate_bit_flips | 483-491 | REAL | - | 1..=8 single-bit XORs; below(0) impossible (chunks non-empty, asserted) |
| mutate_truncation | 500-519 | REAL | - | stratified index%64 / index%4 pins degenerate cuts; assert frames>=2 |
| mutate_garbage_splice | 525-535 | REAL | - | 1..=96 bytes inserted anywhere incl. ends |
| run_mutation_class | 542-577 | REAL | - | harness on EVERY mutant; asserts typed_errors==0 for payload damage |
| random_source | 584-592 | REAL | - | includes zero channels/rate lies |
| contract_with_format_lie | 597-635 | REAL | - | with_format seam lies typed; coherent lies accounted |
| waveformat_lie | 643-698 | REAL | - | length/byte lies; accepted lies read geometry from blob[2..8] (len>=16 guaranteed by configure success) |
| descriptor_lie | 708-784 | REAL | - | 4 strata incl. layout (_planar) lie |
| self_consistent_waveformat | 789-798 | REAL | - | coherent-lying-encoder blob builder |
| extensible_lie | 818-869 | REAL | - | 40-byte WAVEFORMATEXTENSIBLE corruption stratum |
| expect_configure_code | 874-900 | REAL | - | exact code pins, both directions |
| both_parents_full_audio_tracks_survive... | 908-973 | REAL | - | passed (executed); pins 2.000s duration |
| systematic_prefix_truncations... | 979-1025 | REAL | - | passed; exhaustive 0..=5880, exact class counts |
| systematic_single_byte_flips... | 1035-1071 | REAL | - | passed; MSB of every byte |
| every_real_chunk_survives_boundary... | 1078-1116 | REAL | - | passed |
| seeded_bit_flips... | 1120-1139 | REAL | - | passed |
| seeded_truncations... | 1141-1168 | REAL | - | passed; count pins match stratification |
| seeded_garbage_splices... | 1170-1189 | REAL | - | passed |
| seeded_format_header_lies... | 1195-1360 | REAL | - | passed (see F-004) |
| deep_seeded_mutation_run | 1368-1403 | REAL | - | executed by me: 4.30s, ledger matches worklog |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 8 gate tests | YES | exact assert_eq on dropped_tail_bytes/samples.len/outcome/duration recomputed independently of decoder; a mis-concealing or mis-counting decoder fails; a panicking decoder fails the run |
| deep_seeded_mutation_run | YES | same assertions at 8x volume; additionally verifies no OOM/hang at 2048/class (ran in 4.30s) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "this file was cited by pcm/fuzz.rs and fuzz/fuzz_targets/fuzz_codec_pcm.rs before it existed; it now exists and delivers exactly what those citations claim" | file header L1-5 | REAL (current state) / UNVERIFIED (the historical "cited before it existed" — needs git history, forbidden) | citations exist: crates/codecs/src/audio/pcm/fuzz.rs:24, fuzz/fuzz_targets/fuzz_codec_pcm.rs:14; suite executed green incl. deep run |
| "measured well inside the 10 s cargo test budget" (gate run) | L52-53 | REAL | baseline: 0.56s for whole suite (cargo-test-workspace.txt:1341) |
| deep run numbers | docs/worklog/archive-2026-10.md | REAL | my execution reproduces 17163/9732/6712/192/64/5828/1242 exactly |

## 7. Dead / duplicated / unreachable code
None. All helpers are used; the #[ignore] deep run is reachable on demand and was executed.

## 8. Dependencies & flags
Test-only (dev-dependency on codec_api, codecs, media_types via crate's Cargo.toml). No feature gates observed in this file. Uses repo-root tests/fixtures via common/mod.rs `fixtures_root()` = CARGO_MANIFEST_DIR/../../tests/fixtures — verified parents exist there with .oracle.sha256 sidecars and CC0 CORPUS.md.

## 9. Open questions for the next agent
- corpus.rs (external-oracle lane) is claimed harness-gated; not in my assignment — verify it exists and runs.
- pcm_zero_alloc.rs passed in baseline (0.03s) — separate file, not audited here.
