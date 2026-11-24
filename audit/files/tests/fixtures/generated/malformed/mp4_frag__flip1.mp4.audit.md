# AUDIT: tests/fixtures/generated/malformed/mp4_frag__flip1.mp4
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A08) |
| Date | 2026-11-24 |
| File verdict | REAL (data + executed) |
| Reachable from shipped entry point? | As fixture: YES — demux crate malformed-mp4 robustness suites |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 byte-diff vs parent; rg consumers/generator; baseline citation |

## 1. Verdict (data audit)
Seeded 1-bit derivative of the real parent: 14,045 bytes (== tests/fixtures/generated/valid/
mp4_frag.mp4), sha256 33ee37cdfade5cd4509167dc6680ef478d0b987d44b99f9646fff10bc9b7997f.
Byte-diff vs parent: **exactly 1 bit** at offset 8 (0x69→0x61, mask 0b1000) — the first major-
brand byte of `ftyp` (`iso…`→`aso5` per the head bytes), inside the generator's 512-byte MP4
header extent (tools/xtask/src/cmd/fixtures.rs:2430). Generated, not handmade.

Consumer + execution proven: crates/demux/tests/mp4_fixtures.rs:237 lists it in MALFORMED →
`malformed_mp4_fixtures_survive_the_fuzz_harness` (:244) and
`malformed_mp4_fixtures_only_ever_refuse_or_demux_typed` (:255), both green in the
orchestrator baseline (audit/runs/baseline/cargo-test-workspace.txt:2373-2374). Cited in
docs/corpus/compat-matrix-20261110.raw-rows.jsonl.

F-001 [S3] Unpinned by the drift gate: tests/fixtures/generated/MANIFEST.toml's 28 entries do
not include the mp4_frag family (parent + derivatives) — byte drift here escapes
`cargo xtask fixtures --check`. Confidence: HIGH — Verification: read (rg over MANIFEST.toml).
