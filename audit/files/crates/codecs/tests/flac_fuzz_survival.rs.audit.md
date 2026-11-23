# AUDIT: crates/codecs/tests/flac_fuzz_survival.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-23 |
| Lines / functions | 575 / 14 (3 tests, 11 helpers) |
| File verdict | REAL (gate lane) + FALSE deep-run claim (deep test FAILS on execution) |
| Reachable from shipped entry point? | N/A (integration test of shipped flac decoder) |
| Findings | S0: 1 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | executed prebuilt binary `--include-ignored --nocapture --test-threads=1` -> gate test ok (784 mutants, 3451 isolation byte-matches), corpus test SKIP, **deep test FAILED** at flac_fuzz_survival.rs:335 in 13.70s total; re-ran deep test alone to capture panic; python3 analysis of failure arrays + fixture metadata chain |

## 1. One-paragraph verdict
The gate lane is real and honest: three surfaces per mutant (registered `fuzz_codec_flac` harness on a stride, `parse_stream` layout typing, per-frame `decode_frame` with byte-length honesty and a byte-identical isolation oracle), executed by me — 784 accounted mutants, 8224 frames ok, 728 typed frame errors, 96 walk rejections, 3451 isolation byte-matches, matching the baseline (11.76s, 2 passed / 1 ignored). But the `#[ignore]` deep run — the "exhaustive single-byte flip sweep" — FAILS when executed: the isolation oracle fires at STREAMINFO damage because the sweep flips metadata bytes while the oracle's own doc comment says metadata flips must be excluded. Root cause proven from the failure arrays: mutant decode = clean decode × 256 — the documented left-justify of an 8-bit decode after a one-bit bps lie in STREAMINFO (decoder CORRECT per its contract; oracle WRONG). Because the sweep hits the STREAMINFO bps field at offset ≈22 of 12243, the deep test can never complete, which makes the header's "measured ~3 min debug-profile for the whole deep test" claim impossible for the committed code — a false evidence claim.

## 2. What actually works (verified)
— verified by `../../target/debug/deps/flac_fuzz_survival-f895e1faa862c45f --include-ignored --nocapture --test-threads=1` -> `fixtures gate run: 784 accounted mutants, 98 harness calls -> 688 layouts, 8224 frames ok, 728 frame errors (all typed), 96 walk rejections, 3451 isolation byte-matches` ok; corpus test printed SKIP (corpus absent).
— verified by baseline audit/runs/baseline/cargo-test-workspace.txt:1076-1083 -> 2 passed / 1 ignored, 11.76s.
— verified by python3 parse of the mono fixture: 12243 bytes, metadata chain STREAMINFO+VORBIS_COMMENT+PADDING(8192), frames_start=8286, 3 frame syncs, STREAMINFO facts 4608/4608/44100/1ch/16bps.
— verified FLAC_CODES taxonomy: all 21 asserted codes exist in the flac module (rg spot-checks: metadata.rs, decoder.rs, error.rs).

## 3. Findings
F-001 [S0]
Where: crates/codecs/tests/flac_fuzz_survival.rs:497-517 (deep test), 81-83 (false measurement claim), 333-339 (failing assert)
Code: (verbatim, L81-83 and L502-506)
```
/// Deep-run volume per class: `#[ignore]`-marked, executed on demand
/// (measured ~3 min debug-profile for the whole deep test).
const DEEP_SEEDS: usize = 512;
```
```
    for at in 0..data.len() {
        let mut mutant = data.clone();
        mutant[at] ^= 0x80;
        let damage = Damage::Flips(vec![at]);
```
What is wrong: I executed the deep test: it PANICS at line 335 (`assertion left == right failed: an untouched frame must decode identically after neighbouring damage`) within the first ~25 sweep offsets. Root cause: the exhaustive sweep flips EVERY byte including STREAMINFO; a one-bit flip in the STREAMINFO bps field (bps 16→8) lies the facts, the frame header says "bps from STREAMINFO", the decoder (correctly, per its documented left-justify contract) emits every sample ×256 in the same 2-byte container — 9216 bytes both sides, 9178 differing, left[n] == right[n]<<8 exactly (python3-verified). The oracle demands byte-identical output for a frame whose FACTS were doctored. Since the sweep reaches the STREAMINFO bps field at offset ≈22 of 12243, the test can never complete; the "12 243 + 4*4*(512+4) > 20 000 accounted mutants" volume (L489-491) and the "~3 min measured" claim are impossible for the committed code. The doc claim is FALSE for the committed state. (Historical possibility that an earlier version passed: UNVERIFIABLE without git — but the committed doc+code pair is contradictory.) Why it matters: a deep lane offered as on-demand audit evidence does not run; anyone executing it gets a failure that looks like a decoder bug but is a test bug. What correct looks like: pass `Damage::NoBaseline` for offsets < frames_start (as the Damage::Flips doc already specifies) and re-measure. Confidence: HIGH — Verification: ran (FAILED reproduced twice) + python3 numeric proof.
F-002 [S3]
Where: crates/codecs/tests/flac_fuzz_survival.rs:299-311
Code: (verbatim)
```
            Damage::Flips(offsets) => {
                let start = frame.offset as usize;
                let end = start + frame.packet.len();
                let clean_bytes = clean
                    .1
                    .iter()
                    .find(|(offset, _)| *offset == frame.offset)
                    .map(|(_, bytes)| bytes);
                clean_bytes.filter(|_| {
                    frame.offset >= clean.0 && offsets.iter().all(|at| *at < start || *at >= end)
                })
```
What is wrong: the oracle's documented precondition ("*if* the flips never touched the metadata chain", L235-237) is enforced NOWHERE — the filter only checks the frame's own byte range avoided the flips. The gate run's whole-file flip stratum (region=None) can also draw metadata offsets; it passes only because 48 fixed random seeds happened not to hit a facts-lie + matching-offset + Ok-decode combination. Latent false-failure in the gate lane too (fixed seeds make it deterministic-today, but any seed/fixture change can trip it). Confidence: HIGH — Verification: read + failure reproduced in deep lane.
F-003 [S4]
Where: crates/codecs/tests/flac_fuzz_survival.rs:134-163
Code: (verbatim, L134)
```
// ---- the seeded PRNG (the pcm suite's SplitMix64, verbatim) -------------------
```
What is wrong: third verbatim copy of SplitMix64 across test files (pcm_fuzz_survival.rs:160, h264_hw_path.rs:772, here). Confidence: HIGH — Verification: read.
F-004 [S4]
Where: crates/codecs/tests/flac_fuzz_survival.rs:524-528
Code: (verbatim)
```
    let Some(root) = corpus_dir() else {
        eprintln!("corpus: LUMEN_CORPUS unset and /tmp/my-project/corpus-v4 absent — SKIP");
        return;
    };
```
What is wrong: corpus lane self-skips in this sandbox (my run printed the SKIP); its 20-file dirty+garbage assertions never ran here. Confidence: HIGH — Verification: ran.
F-005 [S4]
Where: crates/codecs/tests/flac_fuzz_survival.rs:502-511
Code: (excerpt)
```
    for at in 0..data.len() {
```
What is wrong: ~8188 of the 12243 sweep offsets land in the 8192-byte PADDING block — provably no-op flips that can never change any surface; two-thirds of the "exhaustive" budget is spent on padding. Confidence: HIGH — Verification: python3 metadata-chain parse.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| output_width | 126-132 | REAL | - | matches documented left-justify buckets; the failure proof relies on it |
| SplitMix64 | 139-163 | REAL | S4 | verbatim dup (F-003) |
| Ledger + report | 168-202 | REAL | - | counts printed on run |
| assert_typed_flac | 209-227 | REAL | - | stage+prefix+taxonomy+kind quadruple check; taxonomy verified to exist |
| Damage enum | 231-244 | REAL | S3 | doc'd precondition unenforced (F-002) |
| clean_frames | 249-266 | REAL | - | isolation baseline, unreachable! on clean-fixture failure (correct) |
| account | 271-347 | REAL | S3 | length honesty + isolation oracle; oracle precondition bug (F-001/F-002) |
| truncations | 352-364 | REAL | - | pinned degenerate cuts + random |
| flips | 369-395 | REAL | - | returns offset list for oracle |
| splices | 398-413 | REAL | - | 8..=64 bytes |
| gate_fixture | 419-451 | REAL | - | 4 classes incl. metadata-confined stratum (correctly NoBaseline there) |
| fuzz_on_stride | 454-459 | REAL | - | |
| fixtures_survive_truncation_flips_splices_and_fact_lies | 461-487 | REAL | - | executed ok; volume/coverage asserts real |
| deep_exhaustive_flip_sweep_survives | 496-517 | FALSE-CLAIM/BROKEN | S0 | fails on execution (F-001) |
| corpus_dirty_and_garbage_survive | 523-575 | REAL-UNPROVEN (here) | S4 | self-skips without corpus (F-004); assertions strong when present |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fixtures_survive... | YES | exact typing + length + byte-identity asserts; executed |
| deep_exhaustive... | NO — it fails even with the code CORRECT | false-positive oracle (F-001); today it fails, so it proves nothing except its own bug |
| corpus_dirty... | NOT IN THIS ENV | self-skip |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "measured ~3 min debug-profile for the whole deep test" | L81-83 | FALSE | deep test panics at offset ≈22 of the sweep; cannot complete (my execution, twice) |
| "12 243 + 4 * 4 * (512 + 4) > 20 000 accounted mutants" | L489-491 | FALSE | sweep dies in the STREAMINFO region; volume unreachable |
| "the isolation oracle applies ... *if* the flips never touched the metadata chain" | L235-237 | TRUE as doc, NOT ENFORCED as code | F-002: filter checks only frame-range avoidance |
| worklog "fuzz survival suite (bit-flip sweeps, 512-seed classes, corpus dirty/mutants ...) passes with typed errors, no panics/hangs" | docs/worklog/archive-2026-10.md:8540 | CONTRADICTED for the committed deep lane | my execution of the deep test FAILS (may describe an earlier suite shape — UNVERIFIABLE without git) |

## 7. Dead / duplicated / unreachable code
SplitMix64 triplicated across test files. The deep test is effectively dead as evidence (cannot pass).

## 8. Dependencies & flags
Test-only; uses mod flac (tests/flac/mod.rs — my file 9) for corpus_dir/fixture_bytes/walk_frames. No feature gates.

## 9. Open questions for the next agent
- Fix shape: exclude `at < frames_start` from the Damage::Flips oracle (one-line filter) and re-measure the deep run; then the volume claims can be re-anchored.
- The decoder itself was vindicated on this input (left-justify ×256 is the documented contract) — but the flac decoder file (not in my assignment) should get its own audit of the left-justify mapping's callers.
