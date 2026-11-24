# AUDIT: fuzz/corpus/fuzz_demux_mkv/seed_mkv_h264_aac.mkv
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A08) |
| Date | 2026-06-14 |
| File verdict | REAL bytes — STUB-consumption (no committed run) |
| Reachable from shipped entry point? | NO — fuzz corpus seed; consumed only by `cargo +nightly fuzz run fuzz_demux_mkv` (fuzz/fuzz_targets/fuzz_demux_mkv.rs:16), a campaign the tree itself documents as NOT WIRED |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3: 26,105 bytes, sha256 42ece741f5d2986c1fde4066b8a3fe3d5c83b4e435ba843edc77ffa237dbcbdc, EBML magic 1a45dfa3; `file -b` -> Matroska data; content walk: doc-type `matroska` x1, track codec `V_MPEG4/ISO/AVC` x1, `A_AAC` x1 — the seed is what its name claims |

Generated-vs-handmade: authored Matroska seed (EBML header + segment, one H.264 + one AAC track entry) — provenance not in any manifest (fuzz corpus has no sha ledger); W16-A02's batch covered sibling seeds the same way. Name matches content (verified bytes above).
Consumer: fuzz_demux_mkv.rs:16-18 `fuzz_target!(|data: &[u8]| { demux::mkv::fuzz::fuzz_demux_mkv(data); })` — real in-crate panic-free harness. BUT the campaign is explicitly "SCAFFOLDED, NOT WIRED" (fuzz/README.md "Status:" block; fuzz/Cargo.toml header "NOT wired into any gate") — the Lead nightly-toolchain decision is outstanding, so NO committed run ever executes these seeds.
F-001 [S3]: zero evidenced executions — the seed exercises nothing in any gate (cargo-test baseline has no fuzz campaign runs; CI has no nightly fuzz lane). The corpus is decoration until the LEAD-DECISION lands. Where: fuzz/README.md status block. Confidence: HIGH — Verification: read (README + Cargo.toml) + rg (no consumer beyond the target shim).
