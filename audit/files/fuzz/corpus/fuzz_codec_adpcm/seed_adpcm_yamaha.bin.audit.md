# AUDIT: fuzz/corpus/fuzz_codec_adpcm/seed_adpcm_yamaha.bin
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A08) |
| Date | 2026-06-14 |
| File verdict | REAL bytes — STUB-consumption (no committed run) |
| Reachable from shipped entry point? | NO — fuzz corpus seed; consumed only by `cargo +nightly fuzz run fuzz_codec_adpcm` (fuzz/fuzz_targets/fuzz_codec_adpcm.rs -> crates/codecs/src/audio/adpcm/fuzz.rs harness) — campaign SCAFFOLDED, NOT WIRED (fuzz/README.md LEAD-DECISION) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3: 35,840 bytes, sha256 78d387af21d0aa56fe20871c4fb643880e25754940d7bb4132fef947b4741bbe; `file -b` -> data (raw payload, correct for a codec-level seed); head hex 00777777 77750100 01818080 9098a8a8 — nibble-packed ADPCM-style substrate; rg consumers -> no in-tree test references `seed_adpcm_yamaha` (only the fuzz shim family) |

Generated-vs-handmade: authored codec-substrate seed (no container structure — correct for a codec target; the sibling seeds ima_qt/ima_ws/swf/raw_facts_skeleton complete the four ADPCM variants the decoder supports). The Yamaha decoder itself is real and tested (crates/codecs/src/audio/adpcm/yamaha.rs; baseline adpcm_fuzz_survival 4/4 + adpcm_corpus_differential 2/2 pass, cargo-test-workspace.txt:940-945/:932-935) — but those tests do NOT consume this seed file.
F-001 [S3]: zero evidenced executions — no committed run references this seed (rg over crates/tests found no consumer; the campaign requires the outstanding nightly LEAD-DECISION per fuzz/README.md). The seed is decoration until the campaign is wired. Confidence: HIGH — Verification: read + rg (narrowed to crates/codecs/tests and fuzz/). Confidence for "no other consumer": HIGH within the searched hot paths; a whole-tree rg was avoided for load reasons.
