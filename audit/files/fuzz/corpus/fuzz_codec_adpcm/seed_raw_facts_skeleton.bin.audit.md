# AUDIT: fuzz/corpus/fuzz_codec_adpcm/seed_raw_facts_skeleton.bin
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 144 B / 3a42b7797e8d5c48… (full digest in audit/runs/W16-A09-batchmeta.txt) |
| File verdict | REAL (synthetic codec-facts skeleton seed) |
| Reachable from shipped entry point? | N/A (fuzz corpus by convention) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

What it is: synthetic "raw facts" skeleton for the ADPCM codec fuzz target: a
16-byte header containing 44 100 Hz twice (`44 ac 00 00` LE, at bytes 4-7 and
8-11), sample/bit fields (00 02, 20 00), followed by a deterministic arithmetic
byte pattern (0x00, 0x0d, 0x1a, 0x27 … step 13 — verified by od dump above).
Machine-generated in-tree skeleton, not real audio.

Consumer: corpus-dir convention (fuzz/corpus/fuzz_codec_adpcm/ + Cargo.toml
target fuzz_codec_adpcm); the in-crate twin is the baseline test
`audio::adpcm::fuzz::tests::raw_facts_skeleton_stays_legal` (passes in
audit/runs/baseline/cargo-test-workspace.txt:391), which embeds the same
skeleton shape — so the *shape* is exercised by committed tests even though
this corpus file is only convention-consumed.
