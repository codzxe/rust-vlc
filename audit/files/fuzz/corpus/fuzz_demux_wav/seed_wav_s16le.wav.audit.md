# AUDIT: fuzz/corpus/fuzz_demux_wav/seed_wav_s16le.wav
| Field | Value |
|---|---|
| Auditor | W16-A10 |
| Date | 2026-11-24 |
| Lines / functions | binary; 35358 bytes; N/A (corpus seed) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — fuzz corpus seed; consumed only by the unwired nightly campaign + registry existence gate |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |

## 1. One-paragraph verdict
Real, valid WAV seed: RIFF/WAVE, PCM s16le, 2ch, 44100 Hz, `data` chunk 35280 bytes (= 0.2s stereo), RIFF size field consistent (decl 35358 == file). It is byte-identical (sha256) to the committed class-A fixture `tests/fixtures/generated/valid/wav_pcm_s16le_44k1_st.wav`, whose bytes ARE exercised on stable by demux tests — so the seed content is genuine, but the corpus FILE itself is never read by any stable gate: the cargo-fuzz campaign that would consume it is explicitly not wired (nightly Lead decision outstanding).

## 2. What actually works (verified)
- `python3 audit/scratch/W16-A10/batch.py` -> size=35358, sha256=e24b939a33fa51050adab81411cccedd80a91e8d81a5e36c6dbecbb7ccecbbd8; wav chunks fmt(16)/LIST(26)/data(35280); fmt tag=1 ch=2 rate=44100 bits=16; `riff_decl=35358` matches file size.
- `sha256sum` both files -> identical hash `e24b939a...` (corpus seed == tests fixture twin).
- `bash scripts/fuzz_registry_check.sh` -> exit 0, "fuzz-registry: CLEAN — 35 targets, diff empty, corpora present" (I ran it).

## 3. Findings
F-001 [S3]
Where: fuzz/Cargo.toml:5-9
Code: (verbatim)
```
# This manifest is a *scaffold*: it makes the three previously-dead
# target files under fuzz_targets/ buildable by cargo-fuzz. It is NOT
# wired into any gate: building it requires a nightly toolchain
```
What is wrong: no stable gate reads `fuzz/corpus/**` — the seed bytes only run via the tests-fixture twin; the corpus file's only gate consumer is the non-empty existence check in scripts/fuzz_registry_check.sh. Why it matters: committed "fuzz corpus" suggests executed fuzzing evidence; nothing executes it (parity report verified-only is 0/252). What correct looks like: either wire the campaign (nightly CI lane) or run corpus seeds through the stable sweeps directly from fuzz/corpus.
Confidence: HIGH — Verification: ran (rg for "fuzz/corpus" consumers: only scripts + netfuzz comment).
F-002 [S4]
Where: fuzz/Cargo.toml:1-3
Code: (verbatim)
```
# fuzz/ — the cargo-fuzz campaign for Lumen's three registered fuzz
# targets (doc 04 §8.5; audit row M14, fix card ED-FIX-5, delivered by
# card W1-12).
```
What is wrong: comment says "three registered fuzz targets" while the manifest registers 35 `[[bin]]` targets (counted: 35) — stale scaffold-era header. Why it matters: readers under-estimate scope; the registry README says 35. Confidence: HIGH — Verification: ran (`rg -c '^\[\[bin\]\]' fuzz/Cargo.toml` -> 35).

## 4. Per-function table
N/A (binary data file).

## 5. Test honesty
Corpus input, not a test. The twin bytes are exercised by `crates/demux/tests/wav_aiff_fixtures.rs` (baseline: demux suite passed — audit/runs/baseline/cargo-test-workspace.txt).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "same files are the committed fuzz/corpus/ seeds" | crates/streaming/tests/netfuzz_sweep.rs:25 | REAL (for HLS/headers; pattern holds here too) | sha256 equality verified for this seed |
| Registry corpora present (35) | scripts/fuzz_registry_check.sh output | REAL | I ran it: exit 0 |

## 7. Dead / duplicated / unreachable code
Exact-duplicate bytes of tests/fixtures/generated/valid/wav_pcm_s16le_44k1_st.wav (sha-identical) — duplication is deliberate (corpus mirror of fixture); cost: two copies can drift independently since the corpus copy is pinned by no manifest.

## 8. Dependencies & flags
None.

## 9. Open questions for the next agent
Do fuzz/corpus/fuzz_demux_wav/seed_wav_rf64.wav and seed_wav_u8.wav have fixture twins too? (rf64 -> tests/fixtures/generated/valid/wav_rf64_44k1_st.wav; u8 -> wav_pcm_u8_44k1_mono.wav — same sha check.)
