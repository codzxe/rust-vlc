# AUDIT: crates/codecs/src/audio/pcm/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W07-A01 |
| Date | 2026-11-27 |
| Lines / functions | 192 / 3 fns (+CountingSink impl, 5 tests) |
| File verdict | REAL (fuzz infrastructure; shipped-entry reachability is N/A by design — but the registered target file + campaign scaffolding + nightly CI tier all exist) |
| Reachable from shipped entry point? | N/A (fuzz harness); reached by fuzz/fuzz_targets/fuzz_codec_pcm.rs:36-38 (`fuzz_target!` → `codecs::audio::pcm::fuzz::fuzz_codec_pcm`), by crates/codecs/tests/pcm_fuzz_survival.rs:73,557, and by .github/workflows/audit.yml:325-349 nightly fuzz tier (shards all 35 targets incl. this one) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p codecs --lib audio::pcm::fuzz` → 5 passed / 0 failed (exit 0); ls fuzz/Cargo.toml, fuzz/corpus/fuzz_codec_pcm (3 seeds), scripts/quality/25_fuzz_campaign.sh (all exist); sed verifications |

## 1. One-paragraph verdict
A small, sound, panic-free-by-oracle fuzz harness: every index is modulo-guarded (`PcmFormat::ALL[(a % 11)]`, `RATES[.. % 3]`), results are deliberately swallowed per the documented absence-of-panic contract (the outcome-pinning lives in pcm_fuzz_survival.rs, a separate already-audited suite), and — correctly — the contract path fuzzes the seam the product actually ships (`PcmDecoder::with_format`, L117). The 5 in-crate smoke tests pass (I ran them), the registered cargo-fuzz target file exists and calls this harness, and a committed 3-seed corpus exists. The one real defect: the module's "Wiring status (honest)" block is stale — it claims the libFuzzer campaign "is owed CI wiring," but the scaffolding (fuzz/Cargo.toml with libfuzzer-sys 0.4.13) and a nightly 5-shard fuzz tier in CI (audit.yml) both exist in the current tree.

## 2. What actually works (verified)
— verified by `cargo test -p codecs --lib audio::pcm::fuzz` → `test result: ok. 5 passed; 0 failed` (empty input, 1..=9-byte sub-frame lengths, fixture-shaped + dangling-tail buffer, all-0xff/all-0x00 4096-byte buffers, determinism/boundedness of the config spread).
— verified by `ls`: fuzz/fuzz_targets/fuzz_codec_pcm.rs (38 lines, `fuzz_target!` wrapper), fuzz/Cargo.toml (libfuzzer-sys "0.4.13", target registered L64), fuzz/corpus/fuzz_codec_pcm/ (3 committed seeds), scripts/quality/25_fuzz_campaign.sh — the CI fuzz tier's runner.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/audio/pcm/fuzz.rs:14-26 (module doc), cross-ref fuzz/fuzz_targets/fuzz_codec_pcm.rs:20-25
Code:
```rust
//! dev-dependency plus the `fuzz/Cargo.toml` scaffolding — both
//! Lead-owned dependency decisions (AGENTS.md §3.2, C5), so until
//! they land the harness logic is proven by the in-crate
//! deterministic smoke tests here and by the committed-corpus +
//! seeded-mutation survival run
//! (`crates/codecs/tests/pcm_fuzz_survival.rs`); the libFuzzer
//! campaign itself is owed CI wiring (same status as
//! `fuzz_demux_avi` / `fuzz_codec_mjpeg`, worklogs 2.B2/2.B4).
```
What is wrong: the claim is FALSE for the current tree. The scaffolding landed (fuzz/Cargo.toml: `libfuzzer-sys = "0.4.13"`, card W1-12/ED-FIX-5 per its own header), a committed seed corpus exists (fuzz/corpus/fuzz_codec_pcm/), and CI wiring exists: .github/workflows/audit.yml:325-349 schedules a nightly (`0 2 * * *`, 240-min, 5-shard) fuzz campaign over every `fuzz/fuzz_targets/*.rs` — fuzz_codec_pcm included — via scripts/quality/25_fuzz_campaign.sh. The same stale text is repeated in the target file ("not yet compiled in CI"). Why it matters: this file's only job is to be the honest evidence chain for PCM fuzzing; a stale "not wired" marker makes auditors undercount the actual coverage — and conversely nobody pruned the marker when wiring landed, so the "honest" doc now lies. Claim-level verdict: FALSE (reality exceeds the doc). Confidence: HIGH — Verification: read (all four artifacts listed in §2). Note UNVERIFIED: whether the nightly tier actually ran green in the wild (no CI run logs in-tree; sandbox cannot execute GitHub Actions).
F-002 [S4]
Where: crates/codecs/src/audio/pcm/fuzz.rs:73-79
Code:
```rust
    let a = u32::from(data.first().copied().unwrap_or(0));
    let b = data.get(1).copied().unwrap_or(0);
    let derived = PcmSource {
        sample: PcmFormat::ALL[(a % PcmFormat::ALL.len() as u32) as usize],
        planar: b & 0x01 == 1,
        channels: 1 + u32::from(b >> 1) % 8,
    };
```
What is wrong (coverage note): the derived channel space is 1..=8 and the rate space is 3 fixed values; the extreme-channel geometry (the >64 / u32::MAX space the `with_format` seam accepts, see decoder.rs F-003) and the WAVEFORMATEX blob path are never fuzzed. Bounded spread is a defensible design (fixed stress shapes cover s24be-planar-3ch and f32be), but the harness does not exercise the shipped seam's validation boundary. Confidence: HIGH — Verification: read.

## 4. Per-function table (file <200 LOC but included for completeness)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| configurations | 64-94 | REAL | F-002 | deterministic, bounded 3-4 configs, dedup vs fixture shape |
| fuzz_codec_pcm | 99-139 | REAL | — | both surfaces; contract path uses the SHIPPED seam (with_format L117); all outcomes swallowed by documented design |
| CountingSink::accept | 54-59 | REAL | — | saturating counter, never fails |
| (tests ×5) | 145-190 | REAL | — | honest smoke; would fail on panic/regression of spread bounds |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| empty_input_survives_both_passes | YES (panic oracle) | executes both passes on the empty edge |
| sub_frame_lengths_survive_both_passes | YES | 1..=9 bytes = every sub-frame length for the widest frame_bytes |
| fixture_shaped_buffer_survives_both_passes | YES | 5880-byte chunk + dangling tail |
| hostile_wide_input_survives_both_passes | YES | 0xff/0x00 4096 |
| configurations_spread_is_deterministic_and_bounded | YES | asserts 3..=4 configs, fixture-first, determinism |
The outcome-level assertions (typed errors, concealment accounting) live in pcm_fuzz_survival.rs — already audited (audit/files/crates/codecs/tests/pcm_fuzz_survival.rs.audit.md exists in-tree).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the libFuzzer campaign itself is owed CI wiring" | fuzz.rs:24-26 | FALSE (stale) | fuzz/Cargo.toml + corpus + audit.yml:325-349 nightly tier exist (F-001) |
| "Memory is bounded by the input … this harness adds no buffers" | fuzz.rs:28-33 | REAL (MEDIUM) | decode_pcm allocates ≤ input (prefix/swap); pass 2 does one `data.to_vec()` per config (≤4 copies, each ≤ input) — bounded, though "adds no buffers" is slightly generous for pass 2's packet copies |
| "not yet compiled in CI" | fuzz/fuzz_targets/fuzz_codec_pcm.rs:23-24 | FALSE (stale, same as F-001) | audit.yml fuzz tier compiles all targets via cargo-fuzz |

## 7. Dead / duplicated / unreachable code
None in this file. The `pub` visibility is required (fuzz target + survival tests call it from outside the crate — mod.rs:91-95 documents this).

## 8. Dependencies & flags
std-only + codec-api/media-types types; no feature gates (PCM is unconditional). The cargo-fuzz side needs nightly (documented in fuzz/Cargo.toml) — not part of the stable release lane.

## 9. Open questions for the next agent
- Has the nightly fuzz tier (audit.yml) ever produced artifacts/crashes for fuzz_codec_pcm? Check docs/audit/reports/fuzz/ and fuzz/artifacts/ history (orchestrator has git access; I do not).
- Stale wiring markers exist across the other 34 fuzz targets' target files and harness modules (same "not yet compiled in CI" text family) — one sweep could fix them all.
