# AUDIT: fuzz/corpus/fuzz_demux_es/seed_crossformat_mkv.mkv
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary, 26105 bytes |
| File verdict | REAL (seed corpus; verified byte-identical to the committed valid fixture) |
| Reachable from shipped entry point? | NO (fuzz corpus; consumed by fuzz_demux_es target, campaign unwired) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 byte-compare + sha256 (exit 0); ran `bash scripts/fuzz_registry_check.sh` -> "CLEAN — 35 targets, diff empty, corpora present" (exit 0) |

## 1. Verdict
Genuine MKV (EBML head `1a45dfa3`), 26105 bytes, sha256 42ece741f5d2986c... (recomputed).
VERIFIED byte-identical to tests/fixtures/generated/valid/mkv_h264_aac.mkv (same size +
sha; python3 `a == b` -> True) — the corpus seed is a copy of the committed valid fixture,
which is the flip-family parent (I verified the flip4 derivative differs from it by exactly
4 single-bit flips). Seed existence is gate-checked: I RAN scripts/fuzz_registry_check.sh —
"fuzz-registry: CLEAN — 35 targets, diff empty, corpora present" — and baseline
full_gate.txt:56-58 records the same PASS.

## 2. Consumers / campaign status
- Consumed by: fuzz/fuzz_targets/fuzz_demux_es.rs:15 (`demux::es::fuzz::fuzz_demux_es(data)`)
  — the raw-ES target. A whole-container file as a raw-ES seed is intentional warming: the
  Annex-B/AAC bytes inside the MKV exercise the start-code scanner (the "crossformat" name).
- Campaign status (honest, README:13-31): SCAFFOLDED, NOT WIRED — Lead decision outstanding;
  stable lane runs `cargo check` + in-crate deterministic sweeps only. No libFuzzer campaign
  has ever run against this seed. UNVERIFIED-run (cargo absent in sandbox).

## 3. Findings
F-001 [S4]
Where: this file vs its consumer target
What is wrong: the seed is a full container fed to a raw elementary-stream target — the
first 100+ bytes (EBML headers) are structurally irrelevant to the ES parser; effective
seed value is only the embedded ES payload. Naming ("crossformat") is defensible but the
seed could be pre-extracted ES for a sharper starting corpus.
Confidence: HIGH — Verification: read + byte-compare.
Mitigation: the coverage-guided engine would extract value itself once wired; and the
registry gate guarantees presence, not quality.

## 4. Open questions
- When the campaign is wired, does seed_raw_aac.aac (the sibling) plus this file give the
  H.264 lane anything? No pure Annex-B .h264 seed exists in this dir (only .mkv/.aac).
