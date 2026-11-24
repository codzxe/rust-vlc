# AUDIT: fuzz/corpus/fuzz_sub_ass/seed_ass_plain.ass
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 656 B / 2eb66399373cf2ed… (full digest in audit/runs/W16-A09-batchmeta.txt) |
| File verdict | REAL (valid-shape text seed, hand-authored in-tree) |
| Reachable from shipped entry point? | N/A (fuzz corpus by convention) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

What it is: well-formed ASS subtitle: [Script Info] (ScriptType v4.00+), full
[V4+ Styles] Format line + one Default style, [Events] Format + 2 Dialogue lines
including tag overrides `{\b1}bold{\b0}` and `{\i1}italic`. Baseline seed for
the ASS subtitle parser fuzz target. Content verified by full read (above).
Consumer: corpus-dir convention (fuzz/corpus/fuzz_sub_ass/ + Cargo.toml target
fuzz_sub_ass); no in-repo test reads it by name.
