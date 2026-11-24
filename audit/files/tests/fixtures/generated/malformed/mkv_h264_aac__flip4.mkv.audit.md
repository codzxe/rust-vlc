# AUDIT: tests/fixtures/generated/malformed/mkv_h264_aac__flip4.mkv
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary, 26105 bytes |
| File verdict | REAL (derivation verified byte-level: exactly 4 single-bit flips) |
| Reachable from shipped entry point? | NO (test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 XOR diff vs parent (exit 0); rg consumer (exit 0) |

## 1. Verdict
Genuine class-B malformed MKV derivative. VERIFIED by byte-diff against parent
generated/valid/mkv_h264_aac.mkv: exactly 4 differing bytes (offsets 118, 219, 474, 497),
each a single-bit XOR (popcount 1) — the "k=4" flip contract holds exactly, unlike the OGG
family's misnamed flipN files. All flips land in the header/EBML region. sha256
8e2b653bd6c5a2b9... (recomputed). Parent is byte-identical to the fuzz corpus seed
(see seed_crossformat_mkv audit).

## 2. Consumers
crates/demux/tests/mkv_fixtures.rs:356 — the malformed-family walk asserts every derivative
opens through MkvDemuxer with typed refusals or bounded recovery, no panic
(`assert_eq!(e.stage, media_types::PipelineStage::Demux, "{name}: {e}")` — verified sed).
docs/corpus/compat-matrix-20261110.raw-rows.jsonl records its outcome (rg matched).

## 3. Findings
F-001 [S3] — same systemic gap: not in MANIFEST.toml's 28-file drift gate (manifest covers
only the AVI/MP4 first-vertical family); no generator row in tools/xtask/src/cmd/fixtures.rs
(rg `mkv` -> no writer). CORPUS.md:10-11's "Every file under generated/" authorship/drift
claim FALSE here. Confidence HIGH — ran (manifest parse + byte verification).

## 4. Open questions
- mkv_fixtures.rs's truth table pins per-file outcomes? The walk at 352-385 pins stage/code
  per file — W-next assignee of mkv_fixtures.rs should verify flip4's expected code row.
