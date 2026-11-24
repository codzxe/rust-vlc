# AUDIT: tests/fixtures/generated/malformed/mp4_h264_aac__flip4.mp4
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A05) |
| Date | 2026-11-24 |
| File verdict | REAL (data + executed) |
| Reachable from shipped entry point? | As fixture: YES — feeds the demux crate's malformed-mp4 robustness suites |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 byte-diff vs parent; rg consumers/generator; baseline citation |

## 1. Verdict (data audit)
Seeded bit-flip derivative of the real parent: 13,703 bytes (== parent
tests/fixtures/generated/valid/mp4_h264_aac.mp4), sha256
3a1c79bb114f1de5f522612752220e3ed4cdeb961aedff84db110b7655257ada. Byte-diff vs parent:
exactly **4 bit flips, all inside `ftyp`** — offsets 8..11 `isom`→`awml` (0x08,0x04,0x02,0x01
single-bit masks) — matching the generator's contract (k seeded bits within the 512-byte MP4
header extent, tools/xtask/src/cmd/fixtures.rs:2430,2464-2465). Generated, not handmade.

Consumer + execution proven: crates/demux/tests/mp4_fixtures.rs:226 lists it in MALFORMED,
feeding `malformed_mp4_fixtures_survive_the_fuzz_harness` (:244 — no panic/hang/unbounded
alloc) and `malformed_mp4_fixtures_only_ever_refuse_or_demux_typed` (:255 — typed refusal
`demux.mp4.*` or clean walk). Both green in the orchestrator baseline:
audit/runs/baseline/cargo-test-workspace.txt:2373-2374 (`... ok` ×2). Also cited as a corpus
row in docs/corpus/compat-matrix-20261110.raw-rows.jsonl.

F-001 [S3] Not pinned by the drift gate: tests/fixtures/generated/MANIFEST.toml pins 28 files
(avi/mp4_mjpeg_pcm families) — the whole mp4_h264_aac family (parent + 7 derivatives) is
unpinned, so byte drift in this file would not fail `cargo xtask fixtures --check`.
Confidence: HIGH — Verification: read (rg over MANIFEST.toml).
