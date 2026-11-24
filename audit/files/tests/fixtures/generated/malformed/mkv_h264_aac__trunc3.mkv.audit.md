# AUDIT: tests/fixtures/generated/malformed/mkv_h264_aac__trunc3.mkv
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A05) |
| Date | 2026-06-14 |
| File verdict | REAL — load-bearing malformed derivative (typed-error survival loop, baseline-passing) |
| Reachable from shipped entry point? | NO — test fixture; consumed by crates/demux/tests/mkv_fixtures.rs:351 (`malformed_derivatives_are_typed_or_recover_never_panics`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3: 13,412 bytes, sha256 899be655369cdecd3a7ddff6a0827fca393348a0909229ec8e3edb4e09cf6bd3; content: `matroska` doctype, V_MPEG4/ISO/AVC x1, A_AAC x1; derivation verified: byte-prefix of the valid parent (parent 26,105 B; `p[:len(c)]==c` -> True, cut at ~51% — the trunc3 boundary); baseline cargo-test-workspace.txt:2358 (consuming test ok, suite 9/9 :2366) |

Generated-vs-handmade: genuine class-B-style derivative — a clean byte-prefix truncation of the committed parent tests/fixtures/generated/valid/mkv_h264_aac.mkv (python-verified prefix property this run). The consumer feeds it to the real MkvDemuxer asserting typed refusal (stage Demux, `demux.*` code) or bounded recovery with an event-storm cap, no panic (mkv_fixtures.rs:370-397) — GREEN in the orchestrator baseline.
F-001 [S3]: unregistered — no row in tests/fixtures/generated/MANIFEST.toml (28 registered families; the mkv family has none) and no expectation TOML (`ls tests/fixtures/expectations/malformed | grep -c mkv` -> 0). CORPUS.md's class-B contract (derivation record + error contract TOML) does not exist for this family; only the panic-survival loop consumes it. Where: MANIFEST.toml (absence) + expectations/malformed/ (absence). Confidence: HIGH — Verification: ran (grep/python).
Note: the fuzz seed fuzz/corpus/fuzz_demux_mkv/seed_mkv_h264_aac.mkv is 26,105 bytes — same length as this file's parent; byte-identity UNVERIFIED (not needed for this audit's verdict).
