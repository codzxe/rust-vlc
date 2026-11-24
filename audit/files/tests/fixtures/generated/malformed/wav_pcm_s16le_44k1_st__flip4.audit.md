# AUDIT: tests/fixtures/generated/malformed/wav_pcm_s16le_44k1_st__flip4
| Field | Value |
|---|---|
| Auditor | W16-A10 |
| Date | 2026-11-24 |
| Lines / functions | binary; 35358 bytes; N/A (Class-B malformed fixture) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test-only input; consumed by crates/demux/tests/wav_aiff_fixtures.rs (2 tests, dynamic path) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |

## 1. One-paragraph verdict
Genuine Class-B derivative: 4 seeded bit flips within the RIFF header extent of the valid WAV parent; the decisive flip lands on the fmt tag (byte 20: tag 1 -> 0x41), producing the pinned typed refusal `demux.wav.unsupported-format` at `start()` — exactly what the consuming test asserts, and that test passes 8/8 in the committed baseline. Two of the four flips land in the audio payload (offsets 146, 236) and are never read because the demuxer refuses at the header.

## 2. What actually works (verified)
- `python3 audit/scratch/W16-A10/batch.py` -> size=35358, sha256=d1695cbd25f4b88c0c69617b74a5c3fce5bb2ac134629eb06d90020e0abf6b75; wav fmt tag=65 (0x41) vs parent tag=1; PAIRS: 3 diff bytes at offsets [20,146,236], xor values 0x40/0x21/0x02 = exactly 4 distinct bits — matches generator contract "Flip exactly `k` distinct bits" (tools/xtask/src/cmd/fixtures.rs:2462).
- Consumer: crates/demux/tests/wav_aiff_fixtures.rs:453 `let rel = format!("generated/malformed/wav_pcm_s16le_44k1_st__{}", deriv.name);` and :432-439 pin `terminal: "demux.wav.unsupported-format", fails_at_start: true` for flip4; plus the fuzz-harness totality test :707 includes flip4 bytes through `demux::wav::fuzz::fuzz_demux_wav`.
- Execution evidence: audit/runs/baseline/cargo-test-workspace.txt:2436-2452 — `wav_mutated_family_outcomes_are_pinned_exactly ... ok`, `every_wav_and_aiff_fixture_survives_the_fuzz_harnesses ... ok` (8 passed).

## 3. Findings
F-001 [S3]
Where: tests/fixtures/generated/MANIFEST.toml (28 entries; wav family absent)
What is wrong: this fixture (and the whole wav/aiff/ogg/mkv/mov Class-B families) is NOT covered by the manifest drift gate — only the avi/mp4 mjpeg_pcm family is pinned. Silent drift in this file would not be caught by `cargo xtask fixtures --check`. Why it matters: the generator claims determinism ("the same parent always yields the same derivative, byte for byte", fixtures.rs:2423-2424) but there is no committed pin proving the committed bytes still equal a regeneration for this family. What correct looks like: manifest covers all generated files. Confidence: HIGH — Verification: ran (python cross-check: file NOT IN MANIFEST).
F-002 [S4]
Where: bytes at offsets 146, 236 (inside the audio payload)
What is wrong: 2 of the 4 flips are dead mutation — the demuxer refuses at the fmt tag before ever reading the payload, so those flips add nothing this consumer can observe. Confidence: HIGH — Verification: ran (byte-diff + pinned terminal is a start-failure).

## 4. Per-function table
N/A (binary data file).

## 5. Test honesty
Honest: `wav_mutated_family_outcomes_are_pinned_exactly` would FAIL if the demuxer stopped refusing at the fmt tag (terminal + fails_at_start are asserted per derivative), and `every_wav_and_aiff_fixture_survives_the_fuzz_harnesses` feeds the raw bytes to the fuzz harness. Baseline: both ok.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Flips hit the fmt tag (byte 20: tag 1 → 0x41): typed refusal." | crates/demux/tests/wav_aiff_fixtures.rs:432 | REAL | byte 20: 0x01->0x41 verified; fmt tag parsed = 65 |
| "same parent always yields the same derivative, byte for byte" | tools/xtask/src/cmd/fixtures.rs:2423-2424 | REAL-UNPROVEN (no committed pin for wav family; regeneration not run here — cargo absent) | — |

## 7. Dead / duplicated / unreachable code
Extension-less name (no `.wav`) is deliberate: tests address it exactly so. Two dead flipped bytes (F-002).

## 8. Dependencies & flags
None.

## 9. Open questions for the next agent
None for this file.
